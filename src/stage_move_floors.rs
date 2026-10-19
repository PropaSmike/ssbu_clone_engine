#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, Ordering};

pub const STAGE_ID: usize = 0x08;
pub const LEVEL_DATA: usize = 0x300;
pub const DRAW_MODELS: usize = 0x310;
pub const DYNAMIC_OBJECTS: usize = 0x18;
pub const OBJECT_NAME_HASH: usize = 0xa0;
pub const DRAW_MODEL_VECTOR: usize = 0xd0;
pub const DRAW_MODEL_STRIDE: u64 = 0x10;
pub const DRAW_MODEL_KIND: usize = 0x08;
pub const DRAW_MODEL_KIND_MAX: u32 = 1;
pub const MODEL_NAME_HASH: usize = 0x68;
pub const BIND_SLOT: usize = 0x460;
pub const MAX_ENTRIES: u64 = 4096;

static ARMED: AtomicBool = AtomicBool::new(false);

pub fn arm() {
    ARMED.store(true, Ordering::Release);
}

pub fn wanted(name_hash: u64, sets: &[u64], drawn: &[u64]) -> bool {
    sets.contains(&name_hash) && drawn.contains(&name_hash)
}

pub fn plausible_vector(begin: u64, end: u64, stride: u64) -> Option<u64> {
    if begin == 0 || end < begin || begin % 8 != 0 || (end - begin) % stride != 0 {
        return None;
    }
    let count = (end - begin) / stride;
    (count <= MAX_ENTRIES).then_some(count)
}

#[cfg(all(not(test), feature = "stage_config_bridge"))]
mod live {
    use super::*;

    unsafe fn word(at: u64) -> u64 {
        core::ptr::read_volatile(at as *const u64)
    }

    unsafe fn half(at: u64) -> u32 {
        core::ptr::read_volatile(at as *const u32)
    }

    unsafe fn drawn_models(stage: u64) -> Vec<u64> {
        let manager = word(stage + DRAW_MODELS as u64);
        if manager == 0 {
            return Vec::new();
        }
        let begin = word(manager + DRAW_MODEL_VECTOR as u64);
        let end = word(manager + DRAW_MODEL_VECTOR as u64 + 8);
        let Some(count) = plausible_vector(begin, end, DRAW_MODEL_STRIDE) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for index in 0..count {
            let entry = begin + index * DRAW_MODEL_STRIDE;
            if half(entry + DRAW_MODEL_KIND as u64) > DRAW_MODEL_KIND_MAX {
                continue;
            }
            let model = word(entry);
            if model != 0 {
                out.push(word(model + MODEL_NAME_HASH as u64));
            }
        }
        out
    }

    unsafe fn dynamic_objects(stage: u64) -> Vec<u64> {
        let level = word(stage + LEVEL_DATA as u64);
        if level == 0 {
            return Vec::new();
        }
        let collection = word(level + DYNAMIC_OBJECTS as u64);
        if collection == 0 {
            return Vec::new();
        }
        let begin = word(collection);
        let end = word(collection + 8);
        let Some(count) = plausible_vector(begin, end, 8) else {
            return Vec::new();
        };
        (0..count)
            .map(|index| word(begin + index * 8))
            .filter(|object| *object != 0)
            .collect()
    }

    unsafe fn call_binder(binder: u64, stage: u64, object: u64) {
        let mut scratch = [0u64; 8];
        core::arch::asm!(
            "blr {binder}",
            binder = in(reg) binder,
            inout("x0") stage => _,
            inout("x1") 0u64 => _,
            inout("x2") object => _,
            inout("x8") scratch.as_mut_ptr() as u64 => _,
            clobber_abi("C"),
        );
    }

    pub(crate) unsafe fn bind(stage: u64) {
        if !ARMED.load(Ordering::Acquire) || stage == 0 {
            return;
        }
        let stage_id = half(stage + STAGE_ID as u64);
        let sets: Vec<u64> = match crate::stage_registry::registry().lock() {
            Ok(registry) => registry.dynamic_collisions_for(stage_id).to_vec(),
            Err(_) => return,
        };
        if sets.is_empty() {
            return;
        }
        let drawn = drawn_models(stage);
        let objects = dynamic_objects(stage);
        let binder = word(word(stage) + BIND_SLOT as u64);
        let names: Vec<u64> = objects
            .iter()
            .map(|object| word(object + OBJECT_NAME_HASH as u64))
            .collect();
        let mut bound = 0;
        for (object, name) in objects.iter().zip(&names) {
            if wanted(*name, &sets, &drawn) {
                call_binder(binder, stage, *object);
                bound += 1;
            }
        }
        skyline::println!(
            "[movefloor] StageID {stage_id}: bound {bound} of {} dynamic objects \
             (sets {:x?}, drawn {:x?}, names {:x?})",
            objects.len(),
            sets,
            &drawn[..drawn.len().min(12)],
            &names[..names.len().min(12)],
        );
    }
}

#[cfg(all(not(test), feature = "stage_config_bridge"))]
pub(crate) use live::bind;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_offsets_are_the_ones_stage_config_walks() {
        assert_eq!(LEVEL_DATA, 0x300);
        assert_eq!(DYNAMIC_OBJECTS, 0x18);
        assert_eq!(OBJECT_NAME_HASH, 0xa0);
        assert_eq!(DRAW_MODELS, 0x310);
        assert_eq!(DRAW_MODEL_VECTOR, 0xd0);
        assert_eq!(MODEL_NAME_HASH, 0x68);
        assert_eq!(BIND_SLOT, 0x460);
    }

    #[test]
    fn only_a_listed_and_drawn_set_binds() {
        assert!(wanted(7, &[7, 8], &[7]));
        assert!(!wanted(8, &[7, 8], &[7]));
        assert!(!wanted(9, &[7, 8], &[9]));
    }

    #[test]
    fn a_vector_must_be_ordered_aligned_and_whole() {
        assert_eq!(plausible_vector(0x1000, 0x1040, 0x10), Some(4));
        assert_eq!(plausible_vector(0x1000, 0x1000, 8), Some(0));
        assert_eq!(plausible_vector(0, 0x10, 8), None);
        assert_eq!(plausible_vector(0x1040, 0x1000, 8), None);
        assert_eq!(plausible_vector(0x1004, 0x1014, 8), None);
        assert_eq!(plausible_vector(0x1000, 0x1018, 0x10), None);
        assert_eq!(plausible_vector(0x1000, 0x1000 + 8 * (MAX_ENTRIES + 1), 8), None);
    }
}
