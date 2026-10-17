use super::*;

use crate::fighter_param_rows::{enter_for_object, leave};
use core::sync::atomic::{AtomicU32, Ordering};

const OFF_AI_FIGHTER_UPDATE: usize = 0x34b310;

const RECORD_ENTRY: usize = 0x0;
const RECORD_OBJECT: usize = 0x10;
const RECORD_READY: usize = 0x18;
const RECORD_AI: usize = 0x20;
const RECORD_KIND: usize = 0x28;
const LAST_ENTRY: u32 = 0xf;
const UPDATE_RECORD_READERS: [i32; 3] = [51, 65, 83];
const LOWEST_PLAUSIBLE_POINTER: usize = 0x1_0000;
const LOG_LIMIT: u32 = 4;

static UPDATE_LOG: AtomicU32 = AtomicU32::new(0);

fn plausible(value: usize) -> bool {
    value >= LOWEST_PLAUSIBLE_POINTER && value & 7 == 0
}

unsafe fn read_word(address: usize) -> usize {
    core::ptr::read_volatile(address as *const usize)
}

unsafe fn update_object(record: u64) -> usize {
    let record = record as usize;
    if !plausible(record)
        || core::ptr::read_volatile((record + RECORD_ENTRY) as *const u32) > LAST_ENTRY
        || read_word(record + RECORD_AI) == 0
        || read_word(record + RECORD_READY) == 0
    {
        return 0;
    }
    let kind = core::ptr::read_volatile((record + RECORD_KIND) as *const i32);
    if !UPDATE_RECORD_READERS.contains(&kind) {
        return 0;
    }
    read_word(record + RECORD_OBJECT)
}

#[skyline::hook(offset = crate::game::rel(OFF_AI_FIGHTER_UPDATE))]
unsafe fn fighter_update_bridge(record: u64, refresh: u32) {
    let object = update_object(record);
    let guard = enter_for_object(object);
    let own = guard.own_params();
    call_original!(record, refresh);
    leave(guard);
    if own != 0 && UPDATE_LOG.fetch_add(1, Ordering::Relaxed) < LOG_LIMIT {
        dbg_log_public(&format!(
            "[cloneparam] the AI reads clone {}'s own vl.prc while it sizes up that fighter",
            clone_kind_of_object(object as u64).unwrap_or(-1)
        ));
    }
}

pub(crate) fn install_hooks() {
    skyline::install_hooks!(fighter_update_bridge);
}
