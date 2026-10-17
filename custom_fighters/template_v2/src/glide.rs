//! Brawl's glide (`clone_engine_api::glide`): hold jump through the top of the
//! midair jump. It starts from the same input as the float, so lib.rs installs
//! float.rs and leaves this one commented out; swap the two to glide instead.
//! The motion list then needs `glide_start`, `glide`, `glide_attack`,
//! `glide_end` and `glide_landing`, and `game_glideattack` scripts the attack.

use crate::smash;
use clone_engine_api::glide::{self, Glide};
use smash::lib::{lua_const::*, L2CValue};
use smash::lua2cpp::L2CFighterCommon;
use smashline::{Agent, Exec, Init, Main};

/// Meta Knight's Brawl glide; `Glide::PIT`, `Glide::CHARIZARD` or
/// `Glide::from_brawl([..])` for another, `.button_frame(n)` for how long
/// jump must be held (8).
const GLIDE: Glide = Glide::META_KNIGHT;

unsafe extern "C" fn glide_entry(fighter: &mut L2CFighterCommon) {
    glide::frame(fighter, &GLIDE);
}

unsafe extern "C" fn glide_start_init(fighter: &mut L2CFighterCommon) -> L2CValue {
    glide::glide_start_init(fighter, &GLIDE)
}

unsafe extern "C" fn glide_start_exec(fighter: &mut L2CFighterCommon) -> L2CValue {
    glide::glide_start_exec(fighter, &GLIDE)
}

unsafe extern "C" fn glide_init(fighter: &mut L2CFighterCommon) -> L2CValue {
    glide::glide_init(fighter, &GLIDE)
}

unsafe extern "C" fn glide_exec(fighter: &mut L2CFighterCommon) -> L2CValue {
    glide::glide_exec(fighter, &GLIDE)
}

#[allow(dead_code)]
pub fn install() {
    Agent::new("template_fighter")
        .on_line(Main, glide_entry) // the way in, checked every frame
        .status(Init, *FIGHTER_STATUS_KIND_GLIDE_START, glide_start_init) // the game's own glide statuses
        .status(Exec, *FIGHTER_STATUS_KIND_GLIDE_START, glide_start_exec)
        .status(Init, *FIGHTER_STATUS_KIND_GLIDE, glide_init)
        .status(Exec, *FIGHTER_STATUS_KIND_GLIDE, glide_exec)
        .install();
}
