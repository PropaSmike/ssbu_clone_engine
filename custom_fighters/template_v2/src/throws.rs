use crate::smash;
use smash::app::{lua_bind::*, sv_battle_object};
use smash::hash40;
use smash::lib::lua_const::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::phx::Hash40;

/// The opponent's half of this fighter's forward throw. lib.rs registers it as a
/// victim global motion, so every fighter (clones included) has it.
pub const THROWN: &str = "thrown_template_fighter_f";

/// The animation the opponent plays. Ship it in this fighter's own motion
/// folders (fighter/template_fighter/motion/body/c00 .. c07) and list it in
/// config.json like the pack's other files. Only this fighter ships it, so only
/// this fighter's motion list gets it; everyone else's THROWN has no animation,
/// and the game plays this fighter's file on them.
pub const THROWN_FILE: &str = "e01throwntemplatefighterf.nuanmb";

/// What a forward throw puts the opponent in; the suffix is the opponent's body
/// type. Vanilla throws ship one animation per body type; register
/// "thrown_template_fighter_f_dx" and friends the same way to do that too.
const VANILLA_THROWN_F: [&str; 4] = ["thrown_f", "thrown_f_dx", "thrown_f_girl", "thrown_f_big"];

/// Called every frame for this fighter. During the forward throw, moves the
/// held opponent from the vanilla thrown motion to THROWN.
pub unsafe fn forward_throw(fighter: &mut L2CFighterCommon) {
    let boma = fighter.module_accessor;
    if StatusModule::status_kind(boma) != *FIGHTER_STATUS_KIND_THROW
        || MotionModule::motion_kind(boma) != hash40("throw_f")
    {
        return;
    }
    // no shipped file means this fighter's THROWN has no animation either: keep the vanilla throw
    if !MotionModule::is_anim_resource(boma, Hash40::new(THROWN)) {
        return;
    }
    let held = LinkModule::get_node_object_id(boma, *LINK_NO_CAPTURE) as u32;
    if held == *BATTLE_OBJECT_ID_INVALID as u32 {
        return;
    }
    let opponent = sv_battle_object::module_accessor(held);
    let motion = MotionModule::motion_kind(opponent);
    if VANILLA_THROWN_F.iter().any(|name| hash40(name) == motion) {
        // only while held: the opponent asks this fighter for the animation
        MotionModule::change_motion(opponent, Hash40::new(THROWN), 0.0, 1.0, false, 0.0, false, false);
    }
}
