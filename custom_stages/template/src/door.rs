//! A door of stage.toml at work. The engine only puts the fighter in
//! FIGHTER_STATUS_KIND_GIMMICK_DOOR; what the door then does is the plugin's.
//! Here the door's model set (held still by paused_motions) plays its motion,
//! and the fighter comes back out after a second, as if through a revolving
//! door.

use core::sync::atomic::{AtomicU32, Ordering};

use crate::smash;
use dionysus_api::stage;
use smash::app::lua_bind::*;
use smash::lib::lua_const::*;
use smash::lua2cpp::L2CFighterCommon;

/// The door's model set: listed in paused_motions so it waits for this file.
const DOOR_SET: &str = "gdor_00_set";
/// Frames the fighter stays in the door.
const INSIDE_FRAMES: u32 = 60;

/// Frames each player has spent in the door.
static INSIDE: [AtomicU32; 8] = [const { AtomicU32::new(0) }; 8];

/// Called every frame for every fighter (lib.rs installs it).
pub unsafe extern "C" fn frame(fighter: &mut L2CFighterCommon) {
    let boma = fighter.module_accessor;
    let entry = WorkModule::get_int(boma, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    let Some(inside) = INSIDE.get(entry as usize) else {
        return;
    };
    if StatusModule::status_kind(boma) != *FIGHTER_STATUS_KIND_GIMMICK_DOOR {
        inside.store(0, Ordering::Relaxed);
        return;
    }
    let frames = inside.fetch_add(1, Ordering::Relaxed);
    if frames == 0 {
        // the door opens: its motion from the start, at normal speed
        let _ = stage::set_motion_frame(DOOR_SET, 0.0); // frame
        let _ = stage::set_motion_rate(DOOR_SET, 1.0); // 0 holds it, 0.5 half speed
        if let Ok(motion) = stage::motion(DOOR_SET) {
            println!("[template_stage] door opens at frame {} rate {}", motion.frame, motion.rate);
        }
    }
    if frames == INSIDE_FRAMES {
        // back out; a mode could start its next battle here instead
        StatusModule::change_status_request(boma, *FIGHTER_STATUS_KIND_WAIT, false);
    }
}
