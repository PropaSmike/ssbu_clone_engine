pub const OFF_PLACE_HASH_RESOLVER: usize = 0x32b3860;

pub const OFF_STAGE_NAME_RESOLVER: usize = 0x13fa740;

pub const OFF_PLACE_FORM_SCAN: usize = 0x1739df0;

pub const OFF_STAGE_SELECT_TO_ID: usize = 0x33117b0;

pub const OFF_MATCH_START_PATH: usize = 0x2311000;

pub const OFF_PANEL_LIST_COUNT: usize = 0x1B30FB0;
pub const PANEL_LIST_COUNT_OPCODE: u32 = 0x1B097D03;

pub const OFF_PANEL_LIST_SIZED: usize = 0x1B2B4C0;
pub const PANEL_LIST_SIZED_OPCODE: u32 = 0xF940_BF68;

pub const OFF_PANEL_LOOKUP_RESULT: usize = 0x1B2B69C;
pub const PANEL_LOOKUP_RESULT_OPCODE: u32 = 0xF940_0728;

const REPORT_LIMIT: usize = 24;

#[cfg(not(test))]
static RUNTIME_READY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(not(test))]
pub fn ready() -> bool {
    RUNTIME_READY.load(std::sync::atomic::Ordering::Acquire)
}

#[cfg(not(test))]
static REPORTED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub fn untag(value: u64) -> u64 {
    value & 0xFF_FFFF_FFFF
}

#[cfg(not(test))]
fn minted_place(hash: u64) -> Option<(String, u32)> {
    let registry = crate::stage_registry::registry().lock().ok()?;
    let bare = untag(hash);
    let stage = registry.by_place_hash(bare).or_else(|| {
        registry.stages().iter().find(|stage| {
            untag(crate::stage_ledger::hash40(&format!(
                "ui_stage_{}",
                stage.place_name
            ))) == bare
        })
    })?;
    Some((stage.place_name.clone(), stage.place))
}

#[cfg(not(test))]
#[skyline::hook(offset = OFF_PLACE_HASH_RESOLVER)]
unsafe fn place_hash_resolver_probe(x0: u64, x1: u64, x2: u64, x3: u64) -> u64 {
    let lr: usize;
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!("mov {}, x30", out(reg) lr);
    #[cfg(not(target_arch = "aarch64"))]
    {
        lr = 0;
    }
    let caller = lr.wrapping_sub(crate::text_base());

    let result = call_original!(x0, x1, x2, x3);

    match minted_place(x1) {
        Some((name, place)) => skyline::println!(
            "[stageresolve] MINTED {} hash={:#x} -> {} (expected {}) called from {:#x}",
            name,
            untag(x1),
            result as i32,
            place,
            caller,
        ),
        None => {
            if REPORTED.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < REPORT_LIMIT {
                skyline::println!(
                    "[stageresolve] hash={:#x} -> {} called from {:#x}",
                    untag(x1),
                    result as i32,
                    caller,
                );
            }
        }
    }
    result
}

#[cfg(not(test))]
static NAME_REPORTED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[cfg(not(test))]
#[skyline::hook(offset = OFF_STAGE_NAME_RESOLVER)]
unsafe fn stage_name_resolver_probe(x0: u64, x1: u64, x2: u64, x3: u64) -> u64 {
    let result = call_original!(x0, x1, x2, x3);
    if NAME_REPORTED.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < REPORT_LIMIT {
        let peek = if x1 > 0x1000 {
            core::ptr::read_volatile(x1 as *const u64)
        } else {
            0
        };
        skyline::println!(
            "[stagename] resolver(x0={:#x}, x1={:#x} -> [{:#x}], x2={:#x}) = {}",
            x0,
            x1,
            peek,
            x2,
            result as i32,
        );
    }
    result
}

#[cfg(not(test))]
static SCAN_REPORTED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[cfg(not(test))]
#[skyline::hook(offset = OFF_PLACE_FORM_SCAN)]
unsafe fn place_form_scan_probe(x0: u64, x1: u64, x2: u64, x3: u64) -> u64 {
    let result = call_original!(x0, x1, x2, x3);
    if SCAN_REPORTED.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < REPORT_LIMIT {
        skyline::println!(
            "[stagescan] place+form scan(x0={:#x}, x1={:#x}, x2={:#x}, x3={:#x}) = {:#x}",
            x0,
            x1,
            x2,
            x3,
            result,
        );
    }
    result
}

#[cfg(not(test))]
static SELECT_REPORTED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[cfg(not(test))]
#[skyline::hook(offset = OFF_STAGE_SELECT_TO_ID)]
unsafe fn stage_select_to_id_probe(x0: u64, x1: u64, x2: u64, x3: u64) -> u64 {
    let result = call_original!(x0, x1, x2, x3);
    if SELECT_REPORTED.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < REPORT_LIMIT {
        skyline::println!(
            "[stageselect] select->id(x0={:#x}, x1={:#x}, x2={:#x}, x3={:#x}) = {} ({:#x})",
            x0,
            x1,
            x2,
            x3,
            result as i32,
            result,
        );
    }
    result
}

#[cfg(not(test))]
static MATCH_REPORTED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[cfg(not(test))]
#[skyline::hook(offset = OFF_MATCH_START_PATH)]
unsafe fn match_start_path_probe(x0: u64, x1: u64, x2: u64, x3: u64) -> u64 {
    let result = call_original!(x0, x1, x2, x3);
    if MATCH_REPORTED.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < REPORT_LIMIT {
        skyline::println!(
            "[stagematch] match-start(x0={:#x}, x1={:#x}, x2={:#x}, x3={:#x}) = {} ({:#x})",
            x0,
            x1,
            x2,
            x3,
            result as i32,
            result,
        );
    }
    result
}

#[cfg(not(test))]
#[skyline::hook(offset = OFF_PANEL_LIST_COUNT, inline)]
unsafe fn panel_list_count_probe(ctx: &mut skyline::hooks::InlineCtx) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static LAST: AtomicU64 = AtomicU64::new(u64::MAX);
    let count = ctx.registers[8].x() / 3;
    let object = ctx.registers[26].x();
    if LAST.swap(count, Ordering::Relaxed) != count {
        skyline::println!(
            "[stagepanel] publish: STAGE_PANEL_LIST_NUM = {count} from object \
             {object:#x} (+0x168); the UI script pages over this"
        );
    }
}

#[cfg(not(test))]
#[skyline::hook(offset = OFF_PANEL_LIST_SIZED, inline)]
unsafe fn panel_list_sized_probe(ctx: &mut skyline::hooks::InlineCtx) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static LAST: AtomicU64 = AtomicU64::new(u64::MAX);
    let object = ctx.registers[27].x();
    let bound = ctx.registers[24].x();
    let list = ctx.registers[23].x();
    let stages = if list == 0 {
        u64::MAX
    } else {
        let begin = core::ptr::read_volatile(list as *const u64);
        let end = core::ptr::read_volatile((list + 8) as *const u64);
        end.wrapping_sub(begin) / 8
    };
    let held = if object == 0 {
        u64::MAX
    } else {
        let begin = core::ptr::read_volatile((object + 0x168) as *const u64);
        let end = core::ptr::read_volatile((object + 0x170) as *const u64);
        end.wrapping_sub(begin) / 24
    };
    let key = bound << 32 | stages;
    if LAST.swap(key, Ordering::Relaxed) != key {
        skyline::println!(
            "[stagepanel] build: object {object:#x}, stage list has {stages}, \
             bound = {bound}, panel list held {held} on entry"
        );
    }
}

#[cfg(not(test))]
#[skyline::hook(offset = OFF_PANEL_LOOKUP_RESULT, inline)]
unsafe fn panel_lookup_result_probe(ctx: &mut skyline::hooks::InlineCtx) {
    use std::sync::atomic::{AtomicU32, Ordering};
    static CALLS: AtomicU32 = AtomicU32::new(0);
    static IN_BOUND_HITS: AtomicU32 = AtomicU32::new(0);
    static IN_BOUND_MISSES: AtomicU32 = AtomicU32::new(0);
    static OUT_BOUND_HITS: AtomicU32 = AtomicU32::new(0);

    let index = ctx.registers[28].x() as u32;
    let bound = ctx.registers[24].x() as u32;
    let wrapper = ctx.registers[25].x() as usize;
    let pane = if wrapper == 0 {
        0
    } else {
        core::ptr::read_volatile((wrapper + 8) as *const usize)
    };

    if index == 0 {
        CALLS.store(0, Ordering::Relaxed);
        IN_BOUND_HITS.store(0, Ordering::Relaxed);
        IN_BOUND_MISSES.store(0, Ordering::Relaxed);
        OUT_BOUND_HITS.store(0, Ordering::Relaxed);
    }
    CALLS.fetch_add(1, Ordering::Relaxed);
    if index < bound {
        if pane == 0 {
            IN_BOUND_MISSES.fetch_add(1, Ordering::Relaxed);
        } else {
            IN_BOUND_HITS.fetch_add(1, Ordering::Relaxed);
        }
    } else if pane != 0 {
        OUT_BOUND_HITS.fetch_add(1, Ordering::Relaxed);
    }

    if (116..=135).contains(&index) || (index < bound && pane == 0) {
        skyline::println!(
            "[panelookup] index={index} bound={bound} wrapper={wrapper:#x} pane={pane:#x} {}",
            if pane == 0 { "MISS" } else { "HIT" }
        );
    }

    if index + 1 == crate::stage_select_cap::TARGET_CAP {
        skyline::println!(
            "[panelookup] finish calls={} bound={bound} in_hits={} in_misses={} out_hits={}",
            CALLS.load(Ordering::Relaxed),
            IN_BOUND_HITS.load(Ordering::Relaxed),
            IN_BOUND_MISSES.load(Ordering::Relaxed),
            OUT_BOUND_HITS.load(Ordering::Relaxed),
        );
    }
}

#[cfg(not(test))]
pub fn install() {
    let mut preflight_ok = true;
    unsafe {
        let text = skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as *const u8;
        for (site, expected, install) in [
            (OFF_PANEL_LIST_COUNT, PANEL_LIST_COUNT_OPCODE, 0u8),
            (OFF_PANEL_LIST_SIZED, PANEL_LIST_SIZED_OPCODE, 1u8),
            (OFF_PANEL_LOOKUP_RESULT, PANEL_LOOKUP_RESULT_OPCODE, 2u8),
        ] {
            let observed = core::ptr::read_volatile(text.add(site) as *const u32);
            if observed != expected {
                preflight_ok = false;
                skyline::println!(
                    "[stagepanel] REFUSED at {site:#x}: expected {expected:#010x}, \
                     found {observed:#010x}"
                );
                continue;
            }
            match install {
                0 => {
                    skyline::install_hook!(panel_list_count_probe);
                }
                1 => {
                    skyline::install_hook!(panel_list_sized_probe);
                }
                _ => {
                    skyline::install_hook!(panel_lookup_result_probe);
                }
            }
        }
    }
    RUNTIME_READY.store(preflight_ok, std::sync::atomic::Ordering::Release);
    skyline::install_hook!(place_hash_resolver_probe);
    skyline::install_hook!(stage_name_resolver_probe);
    skyline::install_hook!(place_form_scan_probe);
    skyline::install_hook!(stage_select_to_id_probe);
    skyline::install_hook!(match_start_path_probe);
    skyline::println!(
        "[stageresolve] probes armed: {:#x} place-hash, {:#x} stage-name, {:#x} place+form, {:#x} LIVE select->id",
        OFF_PLACE_HASH_RESOLVER,
        OFF_STAGE_NAME_RESOLVER,
        OFF_PLACE_FORM_SCAN,
        OFF_STAGE_SELECT_TO_ID,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tag_is_the_top_byte_and_nothing_else() {
        let bare = 0x00_00_00_0C_2B_9A_50_4Fu64 & 0xFF_FFFF_FFFF;
        assert_eq!(untag(0x69_00_00_0C_2B_9A_50_4F), bare);
        assert_eq!(untag(bare), bare);
    }

    #[test]
    fn untagging_keeps_a_full_length_hash40_intact() {
        let long = 0x00_00_00_FF_FF_FF_FF_FFu64;
        assert_eq!(untag(long), long);
    }

    #[test]
    fn panel_lookup_probe_is_after_the_hidden_result_call() {
        assert_eq!(OFF_PANEL_LOOKUP_RESULT, 0x1B2B69C);
        assert_eq!(PANEL_LOOKUP_RESULT_OPCODE, 0xF940_0728);
        assert!(OFF_PANEL_LOOKUP_RESULT > 0x1B2B694);
        assert!(OFF_PANEL_LOOKUP_RESULT < 0x1B2B6A8);
    }
}

#[cfg(test)]
pub fn ready() -> bool {
    false
}

#[cfg(test)]
pub fn install() {}
