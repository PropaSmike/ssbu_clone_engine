//! A borrowed HUD. lib.rs gives this fighter Joker's Rebellion gauge by its
//! portrait (`.hud("jack")`); only the picture is borrowed, so this file
//! decides what it shows. Here it fills with the damage the fighter has
//! taken, the way Joker's does.

use core::sync::atomic::{AtomicU32, Ordering};

use crate::smash;
use clone_engine_api::v2::Hud;
use smash::app::lua_bind::*;
use smash::lib::lua_const::*;
use smash::lua2cpp::L2CFighterCommon;

/// The damage that fills the gauge.
const FULL_AT: f32 = 150.0;

/// What each player's gauge shows, in hundredths, so the HUD is only told
/// when it changes.
static SHOWN: [AtomicU32; 8] = [const { AtomicU32::new(u32::MAX) }; 8];

/// Called every frame for this fighter.
pub unsafe fn update(fighter: &mut L2CFighterCommon) {
    let boma = fighter.module_accessor;
    let entry = WorkModule::get_int(boma, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    let Some(shown) = SHOWN.get(entry as usize) else {
        return;
    };
    let fill = (DamageModule::damage(boma, 0) / FULL_AT).clamp(0.0, 1.0); // 0 is empty, 1 full
    let step = (fill * 100.0) as u32;
    if shown.swap(step, Ordering::Relaxed) != step {
        // other donors take other calls: Hud::Slot(material, count) on Steve's,
        // Hud::State(on) on Terry's, Hud::MenuOpen(..) on Hero's
        crate::TEMPLATE.hud(&*fighter, Hud::Gauge(fill)); // false when the engine has no HUD for it
    }
}
