#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

const INIT_LOADED_DIR: usize = 0x3541970;
const STAGE_ID_GLOBAL: usize = 0x52c75d0;
const STAGE_ID_NONE: u32 = 0xffff_ffff;

const MUTEX_DATA_OFFSET: usize = 32;
const MUTEX_INITIALIZED: usize = 0;
const MUTEX_RECURSIVE: usize = 1;
const SCAN_WORDS: usize = 24;
const PAIR_REACH: usize = 6;
const RET: u32 = 0xd65f_03c0;

const MEMORY_FREE: u32 = 0x00;
const PERMISSION_READ: u32 = 1;
const PERMISSION_WRITE: u32 = 2;

#[derive(Debug, PartialEq, Eq)]
pub enum Slot {
    Found(usize),
    NoSymbol,
    NoCode(usize),
    NoPair,
    Unmapped(usize),
    NotAMutex(usize),
}

pub fn minted_stage_is_loading(stage_id: u32) -> bool {
    stage_id != STAGE_ID_NONE && stage_id >= crate::stage_ledger::VANILLA_STAGE_IDS
}

fn is_adrp(word: u32) -> bool {
    word & 0x9f00_0000 == 0x9000_0000
}

fn is_add_immediate(word: u32) -> bool {
    word & 0xffc0_0000 == 0x9100_0000
}

fn destination(word: u32) -> u32 {
    word & 0x1f
}

fn source(word: u32) -> u32 {
    (word >> 5) & 0x1f
}

pub fn adrp_add_target(pc: usize, words: &[u32]) -> Option<usize> {
    for (index, first) in words.iter().copied().enumerate() {
        if first == RET {
            return None;
        }
        if !is_adrp(first) {
            continue;
        }
        let last = (index + PAIR_REACH).min(words.len());
        for second in words[index + 1..last].iter().copied() {
            if !is_add_immediate(second) {
                continue;
            }
            if source(second) != destination(first) {
                continue;
            }
            return crate::stage_relocation::decode_pair(pc + index * 4, first, second);
        }
    }
    None
}

pub fn looks_like_mutex(header: &[u8]) -> bool {
    if header.len() < MUTEX_DATA_OFFSET + 16 {
        return false;
    }
    if header[MUTEX_INITIALIZED] != 1 || header[MUTEX_RECURSIVE] != 0 {
        return false;
    }
    let mut tag = [0u8; 4];
    tag.copy_from_slice(&header[MUTEX_DATA_OFFSET..MUTEX_DATA_OFFSET + 4]);
    u32::from_le_bytes(tag) <= 1
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct MemoryInfo {
    address: u64,
    size: u64,
    kind: u32,
    attribute: u32,
    permission: u32,
    ipc_refcount: u32,
    device_refcount: u32,
    padding: u32,
}

#[cfg(not(target_arch = "aarch64"))]
unsafe fn query_memory(_info: *mut MemoryInfo, _address: u64) -> u32 {
    u32::MAX
}

#[cfg(target_arch = "aarch64")]
unsafe fn query_memory(info: *mut MemoryInfo, address: u64) -> u32 {
    let result: u64;
    core::arch::asm!(
        "svc 0x6",
        inout("x0") info as u64 => result,
        out("x1") _,
        in("x2") address,
        clobber_abi("C"),
        options(nostack),
    );
    result as u32
}

unsafe fn region_end(address: usize, permission: u32) -> Option<usize> {
    let mut info = MemoryInfo::default();
    if query_memory(&mut info, address as u64) != 0 {
        return None;
    }
    if info.kind == MEMORY_FREE || info.size == 0 {
        return None;
    }
    if info.permission & permission != permission {
        return None;
    }
    let end = info.address.checked_add(info.size)?;
    ((address as u64) >= info.address && (address as u64) < end).then_some(end as usize)
}

unsafe fn spans(address: usize, length: usize, permission: u32) -> bool {
    match region_end(address, permission) {
        Some(end) => address.checked_add(length).is_some_and(|last| last <= end),
        None => false,
    }
}

static SLOT: AtomicUsize = AtomicUsize::new(0);
static ARMED: AtomicBool = AtomicBool::new(false);
static SUBSCRIBED: AtomicBool = AtomicBool::new(false);
static LAST_REPORTED: AtomicU32 = AtomicU32::new(STAGE_ID_NONE);

#[cfg(not(test))]
unsafe fn resolve() -> Slot {
    let Some(getter) = crate::css_registration::lookup_symbol(b"get_current_stage_alt\0") else {
        return Slot::NoSymbol;
    };
    let Some(end) = region_end(getter, PERMISSION_READ) else {
        return Slot::NoCode(getter);
    };
    let count = SCAN_WORDS.min((end - getter) / 4);
    if count == 0 {
        return Slot::NoCode(getter);
    }
    let words = core::slice::from_raw_parts(getter as *const u32, count);
    let Some(address) = adrp_add_target(getter, words) else {
        return Slot::NoPair;
    };
    if !spans(
        address,
        MUTEX_DATA_OFFSET + 16,
        PERMISSION_READ | PERMISSION_WRITE,
    ) {
        return Slot::Unmapped(address);
    }
    let header = core::slice::from_raw_parts(address as *const u8, MUTEX_DATA_OFFSET + 16);
    if !looks_like_mutex(header) {
        return Slot::NotAMutex(address);
    }
    Slot::Found(address)
}

#[cfg(not(test))]
unsafe fn clear(slot: usize) -> bool {
    let data = (slot + MUTEX_DATA_OFFSET) as *mut u64;
    if core::ptr::read_volatile(data) == 0 && core::ptr::read_volatile(data.add(1)) == 0 {
        return false;
    }
    core::ptr::write_volatile(data.add(1), 0);
    core::ptr::write_volatile(data, 0);
    true
}

#[cfg(not(test))]
unsafe fn disown_this_stage() {
    let slot = SLOT.load(Ordering::Acquire);
    if slot == 0 {
        return;
    }
    let stage_id =
        core::ptr::read_volatile((crate::text_base_public() + STAGE_ID_GLOBAL) as *const u32);
    if !minted_stage_is_loading(stage_id) {
        return;
    }
    if clear(slot) && LAST_REPORTED.swap(stage_id, Ordering::Relaxed) != stage_id {
        skyline::println!(
            "[stagealts] StageID {stage_id} is a minted stage and has no alts of its own; \
             dropped the alt left over from the last match so stage-alts leaves its files alone"
        );
    }
}

#[cfg(not(test))]
#[skyline::hook(offset = INIT_LOADED_DIR)]
unsafe fn init_loaded_dir_hook(filesystem: u64, index: u32) -> u64 {
    disown_this_stage();
    call_original!(filesystem, index)
}

#[cfg(not(test))]
fn install() -> bool {
    match unsafe { resolve() } {
        Slot::Found(address) => {
            SLOT.store(address, Ordering::Release);
            skyline::install_hook!(init_loaded_dir_hook);
            skyline::println!(
                "[stagealts] stage-alts is loaded and its selected alt lives at {address:#x}; \
                 minted stages will load with no alt applied"
            );
            true
        }
        Slot::NoSymbol => false,
        other => {
            skyline::println!(
                "[stagealts] REFUSED to read stage-alts' selected alt ({other:?}); a minted stage \
                 loaded after a stage alt may load the wrong files"
            );
            true
        }
    }
}

#[cfg(not(test))]
extern "C" fn on_mod_filesystem_mounted(_event: u32) {
    install();
}

#[cfg(not(test))]
fn subscribe() {
    if SUBSCRIBED.swap(true, Ordering::AcqRel) {
        return;
    }
    type Register = unsafe extern "C" fn(u32, extern "C" fn(u32));
    let found =
        unsafe { crate::css_registration::lookup_symbol(b"arcrop_register_event_callback\0") };
    if let Some(address) = found {
        unsafe {
            let register: Register = core::mem::transmute(address);
            register(1, on_mod_filesystem_mounted);
        }
    }
}

#[cfg(not(test))]
pub fn arm() {
    if ARMED.swap(true, Ordering::AcqRel) {
        return;
    }
    if !install() {
        subscribe();
    }
}

#[cfg(test)]
pub fn arm() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_stage_id_the_game_never_shipped_is_disowned() {
        assert!(!minted_stage_is_loading(STAGE_ID_NONE));
        assert!(!minted_stage_is_loading(0));
        assert!(!minted_stage_is_loading(
            crate::stage_ledger::VANILLA_STAGE_IDS - 1
        ));
        assert!(minted_stage_is_loading(
            crate::stage_ledger::VANILLA_STAGE_IDS
        ));
    }

    #[test]
    fn the_static_is_read_out_of_the_getter_prologue() {
        let pc = 0x7100_2000usize;
        let words = [
            0xa9bf7bfd, 0x910003fd, 0x90000048, 0x91020100, 0x94000000, 0xa8c17bfd,
        ];
        assert_eq!(adrp_add_target(pc, &words), Some(0x7100_a080));
    }

    #[test]
    fn the_scan_stops_at_the_end_of_the_function() {
        let words = [RET, 0xf0000153, 0x91104273];
        assert_eq!(adrp_add_target(0x11e0, &words), None);
    }

    #[test]
    fn an_add_on_a_register_the_adrp_never_wrote_is_not_the_pair() {
        let pc = 0x7100_2000usize;
        let words = [0x90000048, 0x9102014a, 0x9102016b];
        assert_eq!(adrp_add_target(pc, &words), None);
    }

    #[test]
    fn a_mutex_is_recognised_only_when_its_header_and_option_both_read_back() {
        let mut header = [0u8; 48];
        header[0] = 1;
        assert!(looks_like_mutex(&header));
        header[32] = 1;
        assert!(looks_like_mutex(&header));
        header[32] = 2;
        assert!(!looks_like_mutex(&header));
        header[32] = 0;
        header[1] = 1;
        assert!(!looks_like_mutex(&header));
        header[1] = 0;
        header[0] = 0;
        assert!(!looks_like_mutex(&header));
        assert!(!looks_like_mutex(&header[..40]));
    }

    #[test]
    fn the_shipped_stage_alts_getter_resolves_to_its_own_selected_alt() {
        let words = [
            0xf81e0ffeu32,
            0xa9014ff4,
            0xf0000153,
            0x91104273,
            0xaa1303e0,
            0x940078cf,
        ];
        assert_eq!(adrp_add_target(0x11e0, &words), Some(0x2c410));
    }
}
