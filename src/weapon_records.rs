use core::sync::atomic::{AtomicUsize, Ordering};

use crate::custom_articles::{FIRST_CUSTOM_WEAPON_KIND, MAX_CUSTOM_ARTICLES};
use crate::dbg_log_public;

const WEAPON_RECORD_SIZE: usize = 0xe8;
const RECORD_GROUND_MODULE_FACTORY: usize = 0x28;
const RECORD_HIT_MODULE_FACTORY: usize = 0x30;
const NO_HIT_MODULE_FACTORY: usize = 0x33b7d40;
const HIT_MODULE_FACTORY: usize = 0x33b88e0;
pub(crate) const PLATFORM_GROUND_MODULE_FACTORY: usize = 0x33ba0d0;

static PRIVATE_RECORDS: [AtomicUsize; MAX_CUSTOM_ARTICLES] =
    [const { AtomicUsize::new(0) }; MAX_CUSTOM_ARTICLES];

fn row(kind: i32) -> Option<usize> {
    let row = usize::try_from(kind.checked_sub(FIRST_CUSTOM_WEAPON_KIND)?).ok()?;
    (row < MAX_CUSTOM_ARTICLES).then_some(row)
}

unsafe fn word(record: usize, offset: usize) -> usize {
    core::ptr::read_volatile((record + offset) as *const usize)
}

pub(crate) unsafe fn private_record(kind: i32, source_record: usize) -> Option<usize> {
    let row = row(kind)?;
    if source_record == 0 {
        return None;
    }
    let held = PRIVATE_RECORDS[row].load(Ordering::Acquire);
    if held != 0 {
        return Some(held);
    }
    let hit_factory = crate::game::addr(HIT_MODULE_FACTORY);
    let ground_factory = crate::game::addr(PLATFORM_GROUND_MODULE_FACTORY);
    let needs_hit = crate::article_hurtboxes::wants_hit_module(kind)
        && word(source_record, RECORD_HIT_MODULE_FACTORY) == crate::game::addr(NO_HIT_MODULE_FACTORY);
    let needs_ground = crate::article_lvds::wants_platform(kind)
        && word(source_record, RECORD_GROUND_MODULE_FACTORY) != ground_factory;
    if !needs_hit && !needs_ground {
        return None;
    }
    let mut words = vec![0u64; WEAPON_RECORD_SIZE / 8];
    for (index, slot) in words.iter_mut().enumerate() {
        *slot = word(source_record, index * 8) as u64;
    }
    let mut changed: Vec<String> = Vec::new();
    if needs_hit {
        words[RECORD_HIT_MODULE_FACTORY / 8] = hit_factory as u64;
        changed.push(format!("HitModule factory {HIT_MODULE_FACTORY:#x} (its source builds none)"));
    }
    if needs_ground {
        words[RECORD_GROUND_MODULE_FACTORY / 8] = ground_factory as u64;
        changed.push(format!(
            "GroundModule factory {PLATFORM_GROUND_MODULE_FACTORY:#x} (the one that can carry a platform, was {:#x})",
            crate::game::offset_of(word(source_record, RECORD_GROUND_MODULE_FACTORY))
        ));
    }
    let copy = Box::leak(words.into_boxed_slice()).as_ptr() as usize;
    match PRIVATE_RECORDS[row].compare_exchange(0, copy, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => {
            dbg_log_public(&format!(
                "[articlerecord] weapon kind {kind} gets a private weapon record {copy:#x}: {}",
                changed.join(", ")
            ));
            Some(copy)
        }
        Err(existing) => Some(existing),
    }
}
