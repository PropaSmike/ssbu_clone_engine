
use crate::smash;
use crate::TEMPLATE;
use clone_engine_api::v2::Line;
use smash::{
    app::{lua_bind::*, sv_animcmd::*},
    lib::{lua_const::*, L2CValue},
    lua2cpp::{L2CAgentBase, L2CWeaponCommon},
    phx::Hash40,
};
use smash_script::macros;
use smashline::{Agent, Priority};

unsafe extern "C" fn fireball_exec(_weapon: &mut L2CWeaponCommon) -> L2CValue {
    0.into()
}

unsafe extern "C" fn fireball_game(agent: &mut L2CAgentBase) {
    if macros::is_excute(agent) {
        macros::ATTACK(
            agent, 0, 0, Hash40::new("top"), 6.0, 361, 30, 0, 20, 2.5, 0.0, 0.0, 0.0,
            None, None, None, 1.0, 1.0, *ATTACK_SETOFF_KIND_ON, *ATTACK_LR_CHECK_SPEED,
            false, 0, 0.0, 0, true, false, false, false, false,
            *COLLISION_SITUATION_MASK_GA, *COLLISION_CATEGORY_MASK_ALL,
            *COLLISION_PART_MASK_ALL, false, Hash40::new("collision_attr_fire"),
            *ATTACK_SOUND_LEVEL_S, *COLLISION_SOUND_ATTR_FIRE, *ATTACK_REGION_OBJECT,
        );
    }
}

pub fn install(kind: i32) {
    TEMPLATE
        .article("template_fireball") // the article declared in the manifest
        .status(Line::Exec, *WEAPON_MARIO_FIREBALL_STATUS_KIND_REGULAR, fireball_exec) // status line, a status kind of the source weapon, function
        .install();

    // An article's scripts go on "<fighter>_<article>", with the source weapon's script names.
    Agent::new("template_fighter_template_fireball")
        .game_acmd("game_regular", fireball_game, Priority::Default)
        .install();

    // How other fighters treat it, through ParamConfig: (kind, costumes, weapon kind, behaviour).
    // ORIGINAL keeps what the source weapon does; IGNORE, DELETE and MISFIRE change it.
    let Some(fireball) = TEMPLATE.article("template_fireball").weapon_kind() else {
        clone_engine_api::elog!("[template_v2] template_fireball has no weapon kind");
        return;
    };
    param_config::set_kirby_inhale_behavior(kind, vec![-1], fireball, param_config::POCKET_BEHAVIOR_ORIGINAL); // Kirby's inhale
    param_config::set_villager_pocket_behavior(kind, vec![-1], fireball, param_config::POCKET_BEHAVIOR_ORIGINAL); // Villager and Isabelle's pocket
    param_config::set_rosetta_pull_behavior(kind, vec![-1], fireball, param_config::POCKET_BEHAVIOR_ORIGINAL); // Rosalina's pull
}
