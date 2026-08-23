use crate::stage_ledger::hash40;
use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

const ARC_SERVICE_GLOBAL: usize = 0x5331F20;
const ARC_SERVICE_ARC: usize = 0x78;
const ARC_FS_HEADER: usize = 0x40;
const ARC_FILE_PATHS: usize = 0x60;
const ARC_FILE_INFO_INDICES: usize = 0x68;
const ARC_FILE_INFOS: usize = 0x90;
const FS_HEADER_PATH_COUNT: usize = 0x04;
const FILE_PATH_STRIDE: usize = 0x20;
const FILE_INFO_INDEX_STRIDE: usize = 0x08;
const FILE_INFO_STRIDE: usize = 0x10;
const FILE_INFO_FLAGS: usize = 0x0C;
const VANILLA_FILE_COUNT: usize = 590_711;

const UNUSED4_SHIFT: u32 = 22;
pub const STANDALONE_FILE: u32 = 1 << UNUSED4_SHIFT;
pub const UNSHARED_NUS3BANK: u32 = 2 << UNUSED4_SHIFT;

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Disarmed { path_index: u32, was: u32 },
    AlreadyClear { path_index: u32 },
    NotStandalone { path_index: u32 },
    Chained { path_index: u32, owner: u32 },
    NoPath,
    NoArc,
}

pub fn bank_path(place: &str) -> String {
    format!("sound/bank/stage/se_stage_{place}.nus3bank")
}

pub fn disarmed(flags: u32) -> Option<u32> {
    (flags & UNSHARED_NUS3BANK != 0).then(|| flags & !UNSHARED_NUS3BANK)
}

unsafe fn loaded_arc() -> Option<usize> {
    let service =
        core::ptr::read_volatile((crate::text_base_public() + ARC_SERVICE_GLOBAL) as *const usize);
    if service == 0 {
        return None;
    }
    let holder = core::ptr::read_volatile((service + ARC_SERVICE_ARC) as *const usize);
    if holder == 0 {
        return None;
    }
    let arc = core::ptr::read_volatile(holder as *const usize);
    (arc != 0).then_some(arc)
}

unsafe fn file_path_index(arc: usize, hash: u64) -> Option<u32> {
    let header = core::ptr::read_volatile((arc + ARC_FS_HEADER) as *const usize);
    let paths = core::ptr::read_volatile((arc + ARC_FILE_PATHS) as *const usize);
    if header == 0 || paths == 0 {
        return None;
    }
    let count = core::ptr::read_volatile((header + FS_HEADER_PATH_COUNT) as *const u32) as usize;
    if !(VANILLA_FILE_COUNT..=VANILLA_FILE_COUNT * 4).contains(&count) {
        return None;
    }
    for index in (0..count).rev() {
        let entry = paths + index * FILE_PATH_STRIDE;
        let low = core::ptr::read_volatile(entry as *const u32) as u64;
        let length = core::ptr::read_volatile((entry + 4) as *const u8) as u64;
        if (length << 32) | low == hash {
            return Some(index as u32);
        }
    }
    None
}

pub fn disarm_bank_rewrite(place: &str) -> Outcome {
    unsafe {
        let Some(arc) = loaded_arc() else {
            return Outcome::NoArc;
        };
        let Some(path_index) = file_path_index(arc, hash40(&bank_path(place))) else {
            return Outcome::NoPath;
        };
        let paths = core::ptr::read_volatile((arc + ARC_FILE_PATHS) as *const usize);
        let indices = core::ptr::read_volatile((arc + ARC_FILE_INFO_INDICES) as *const usize);
        let infos = core::ptr::read_volatile((arc + ARC_FILE_INFOS) as *const usize);
        if paths == 0 || indices == 0 || infos == 0 {
            return Outcome::NoArc;
        }
        let entry = paths + path_index as usize * FILE_PATH_STRIDE;
        let indice = (core::ptr::read_volatile((entry + 4) as *const u32) >> 8) as usize;
        let info_index = core::ptr::read_volatile(
            (indices + indice * FILE_INFO_INDEX_STRIDE + 4) as *const u32,
        ) as usize;
        let info = infos + info_index * FILE_INFO_STRIDE;
        let owner = core::ptr::read_volatile(info as *const u32);
        if owner != path_index {
            return Outcome::Chained { path_index, owner };
        }
        let flags_at = (info + FILE_INFO_FLAGS) as *mut u32;
        let flags = core::ptr::read_volatile(flags_at);
        if flags & STANDALONE_FILE == 0 {
            return Outcome::NotStandalone { path_index };
        }
        match disarmed(flags) {
            Some(cleared) => {
                core::ptr::write_volatile(flags_at, cleared);
                Outcome::Disarmed {
                    path_index,
                    was: flags,
                }
            }
            None => Outcome::AlreadyClear { path_index },
        }
    }
}

pub fn report(place: &str) {
    let path = bank_path(place);
    match disarm_bank_rewrite(place) {
        Outcome::Disarmed { path_index, was } => skyline::println!(
            "[stagesound] {place}: {path} keeps its own bank id (FilePath {path_index:#x}, \
             flags {was:#010x} -> {:#010x})",
            was & !UNSHARED_NUS3BANK
        ),
        Outcome::AlreadyClear { path_index } => skyline::println!(
            "[stagesound] {place}: {path} was never flagged for a bank id rewrite \
             (FilePath {path_index:#x})"
        ),
        Outcome::NotStandalone { path_index } => skyline::println!(
            "[stagesound] {place}: {path} is not a standalone file (FilePath {path_index:#x}); \
             left alone"
        ),
        Outcome::Chained { path_index, owner } => skyline::println!(
            "[stagesound] {place}: {path} is shared from FilePath {owner:#x} \
             (FilePath {path_index:#x}); it is that stage's own bank, so there is no \
             rewrite to disarm"
        ),
        Outcome::NoPath => skyline::println!(
            "[stagesound] {place}: no {path} in the file table; the stage runs with no bank of \
             its own"
        ),
        Outcome::NoArc => {
            skyline::println!("[stagesound] {place}: the arc is not mounted yet; {path} untouched")
        }
    }
}

const MOD_FILESYSTEM_MOUNTED: u32 = 1;

static PENDING: Mutex<Vec<String>> = Mutex::new(Vec::new());
static MOUNTED: AtomicBool = AtomicBool::new(false);
static SUBSCRIBED: AtomicBool = AtomicBool::new(false);

pub fn schedule(place: &str) {
    if MOUNTED.load(Ordering::Acquire) {
        report(place);
        return;
    }
    if let Ok(mut pending) = PENDING.lock() {
        if !pending.iter().any(|held| held == place) {
            pending.push(place.to_string());
        }
    }
    subscribe();
}

fn drain() {
    let places = match PENDING.lock() {
        Ok(mut pending) => core::mem::take(&mut *pending),
        Err(_) => return,
    };
    for place in places {
        report(&place);
    }
}

extern "C" fn on_mod_filesystem_mounted(_event: u32) {
    MOUNTED.store(true, Ordering::Release);
    drain();
}

fn subscribe() {
    if SUBSCRIBED.swap(true, Ordering::AcqRel) {
        return;
    }
    type Register = unsafe extern "C" fn(u32, extern "C" fn(u32));
    let found = unsafe { crate::css_registration::lookup_symbol(b"arcrop_register_event_callback\0") };
    match found {
        Some(address) => unsafe {
            let register: Register = core::mem::transmute(address);
            register(MOD_FILESYSTEM_MOUNTED, on_mod_filesystem_mounted);
            skyline::println!(
                "[stagesound] waiting on ARCropolis to finish mounting before touching any bank"
            );
        },
        None => {
            skyline::println!(
                "[stagesound] arcrop_register_event_callback is not exported, so nothing waits \
                 for the file table to be patched; every bank keeps whatever id it is given"
            );
            drain();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bank_path_is_the_one_the_stage_id_row_asks_for() {
        assert_eq!(
            hash40(&bank_path("wufd_refresh")),
            crate::stage_ledger::CloneStage::new("wufd_refresh")
                .stage_id_hashes(crate::stage_ledger::Form::Normal)[3]
        );
    }

    #[test]
    fn the_flag_bits_sit_where_modular_bitfield_puts_unused4() {
        let unused4 = |value: u32| (value >> UNUSED4_SHIFT) & 0x3ff;
        assert_eq!(unused4(STANDALONE_FILE), 1);
        assert_eq!(unused4(UNSHARED_NUS3BANK), 2);
        assert_eq!(UNUSED4_SHIFT, 4 + 1 + 7 + 1 + 2 + 1 + 1 + 3 + 1 + 1);
    }

    #[test]
    fn only_a_flagged_file_is_disarmed() {
        assert_eq!(disarmed(0), None);
        assert_eq!(disarmed(STANDALONE_FILE), None);
        assert_eq!(
            disarmed(STANDALONE_FILE | UNSHARED_NUS3BANK),
            Some(STANDALONE_FILE)
        );
    }

    #[test]
    fn disarming_leaves_every_other_flag_alone() {
        let flags = 0xffff_ffffu32;
        assert_eq!(disarmed(flags), Some(flags & !UNSHARED_NUS3BANK));
        assert_eq!(disarmed(flags).unwrap().count_ones(), 31);
    }
}
