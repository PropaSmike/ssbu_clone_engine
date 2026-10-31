#![allow(dead_code)]

pub const OFF_STAGE_MANAGER: usize = 0x53299d8;
pub const MANAGER_STAGES: usize = 0x110;
pub const MANAGER_CURRENT: usize = 0x17c;
pub const MANAGER_SLOTS: u64 = 3;
pub const STAGE_ID: usize = 0x08;
pub const DRAW_MODELS: usize = 0x310;
pub const DRAW_MODEL_VECTOR: usize = 0xd0;
pub const DRAW_MODEL_STRIDE: u64 = 0x10;
pub const DRAW_MODEL_KIND: usize = 0x08;
pub const DRAW_MODEL_KIND_MAX: u32 = 1;
pub const MODEL_NAME_HASH: usize = 0x68;
pub const MODEL_MOTION: usize = 0xd8;
pub const MOTION_FRAME: usize = 0x10;
pub const MOTION_RATE: usize = 0x24;
pub const OFF_MOTION_VTABLE: usize = 0x50c61a0;
pub const RESULT_OK: i32 = 0;
pub const ERROR_NO_STAGE: i32 = -1;
pub const ERROR_NO_MOTION: i32 = -2;
pub const ERROR_VALUE: i32 = -3;
pub const MAX_FRAME: f32 = 1.0e7;
pub const MAX_RATE: f32 = 100.0;

pub fn frame_ok(frame: f32) -> bool {
    frame.is_finite() && (0.0..=MAX_FRAME).contains(&frame)
}

pub fn rate_ok(rate: f32) -> bool {
    rate.is_finite() && (0.0..=MAX_RATE).contains(&rate)
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

    pub(crate) unsafe fn current_stage() -> u64 {
        let slot = word(crate::game::addr(OFF_STAGE_MANAGER) as u64);
        if slot == 0 {
            return 0;
        }
        let manager = word(slot);
        if manager == 0 {
            return 0;
        }
        let index = half(manager + MANAGER_CURRENT as u64) as u64;
        if index >= MANAGER_SLOTS {
            return 0;
        }
        word(manager + MANAGER_STAGES as u64 + index * 8)
    }

    pub(crate) unsafe fn motion_of(stage: u64, set: u64) -> u64 {
        let manager = word(stage + DRAW_MODELS as u64);
        if manager == 0 {
            return 0;
        }
        let begin = word(manager + DRAW_MODEL_VECTOR as u64);
        let end = word(manager + DRAW_MODEL_VECTOR as u64 + 8);
        let Some(count) = crate::stage_move_floors::plausible_vector(begin, end, DRAW_MODEL_STRIDE) else {
            return 0;
        };
        let vtable = crate::game::addr(OFF_MOTION_VTABLE) as u64;
        for index in 0..count {
            let entry = begin + index * DRAW_MODEL_STRIDE;
            if half(entry + DRAW_MODEL_KIND as u64) > DRAW_MODEL_KIND_MAX {
                continue;
            }
            let model = word(entry);
            if model == 0 || word(model + MODEL_NAME_HASH as u64) != set {
                continue;
            }
            let motion = word(model + MODEL_MOTION as u64);
            if motion != 0 && word(motion) == vtable {
                return motion;
            }
        }
        0
    }

    pub(crate) unsafe fn apply_paused(stage: u64) {
        if stage == 0 {
            return;
        }
        let stage_id = half(stage + STAGE_ID as u64);
        let sets: Vec<u64> = match crate::stage_registry::registry().lock() {
            Ok(registry) => registry.paused_motions_for(stage_id).to_vec(),
            Err(_) => return,
        };
        if sets.is_empty() {
            return;
        }
        let mut paused = 0;
        for set in &sets {
            let motion = motion_of(stage, *set);
            if motion == 0 {
                continue;
            }
            core::ptr::write_volatile((motion + MOTION_FRAME as u64) as *mut f32, 0.0);
            core::ptr::write_volatile((motion + MOTION_RATE as u64) as *mut f32, 0.0);
            paused += 1;
        }
        skyline::println!("[motion] StageID {stage_id}: paused {paused} of {} motion(s)", sets.len());
    }

    pub(crate) unsafe fn motion(set: u64) -> Result<u64, i32> {
        let stage = current_stage();
        if stage == 0 {
            return Err(ERROR_NO_STAGE);
        }
        match motion_of(stage, set) {
            0 => Err(ERROR_NO_MOTION),
            motion => Ok(motion),
        }
    }
}

#[cfg(all(not(test), feature = "stage_config_bridge"))]
pub(crate) unsafe fn apply_paused(stage: u64) {
    live::apply_paused(stage);
}

#[cfg(not(all(not(test), feature = "stage_config_bridge")))]
pub(crate) unsafe fn apply_paused(_stage: u64) {}

#[cfg(all(not(test), feature = "stage_config_bridge"))]
unsafe fn motion(set: u64) -> Result<u64, i32> {
    live::motion(set)
}

#[cfg(not(all(not(test), feature = "stage_config_bridge")))]
unsafe fn motion(_set: u64) -> Result<u64, i32> {
    Err(ERROR_NO_STAGE)
}

#[no_mangle]
pub extern "C" fn clone_engine_stage_motion_rate(set: u64, rate: f32) -> i32 {
    if !rate_ok(rate) {
        return ERROR_VALUE;
    }
    match unsafe { motion(set) } {
        Ok(motion) => {
            unsafe { core::ptr::write_volatile((motion + MOTION_RATE as u64) as *mut f32, rate) };
            RESULT_OK
        }
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_stage_motion_frame(set: u64, frame: f32) -> i32 {
    if !frame_ok(frame) {
        return ERROR_VALUE;
    }
    match unsafe { motion(set) } {
        Ok(motion) => {
            unsafe { core::ptr::write_volatile((motion + MOTION_FRAME as u64) as *mut f32, frame) };
            RESULT_OK
        }
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_stage_motion(set: u64, frame: *mut f32, rate: *mut f32) -> i32 {
    if frame.is_null() || rate.is_null() {
        return ERROR_VALUE;
    }
    match unsafe { motion(set) } {
        Ok(motion) => unsafe {
            *frame = core::ptr::read_volatile((motion + MOTION_FRAME as u64) as *const f32);
            *rate = core::ptr::read_volatile((motion + MOTION_RATE as u64) as *const f32);
            RESULT_OK
        },
        Err(code) => code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_and_rates_must_be_finite_and_in_range() {
        assert!(frame_ok(0.0) && frame_ok(4655.0));
        assert!(!frame_ok(-1.0) && !frame_ok(f32::NAN) && !frame_ok(f32::INFINITY));
        assert!(rate_ok(0.0) && rate_ok(1.0) && rate_ok(0.2));
        assert!(!rate_ok(-0.5) && !rate_ok(f32::NAN) && !rate_ok(1000.0));
    }
}
