
use core::sync::atomic::{AtomicBool, Ordering};

use crate::smash;
use crate::TEMPLATE;
use smash::{
    app::{lua_bind::*, sv_animcmd::*, GroundCliffCheckKind, SituationKind},
    lib::{lua_const::*, L2CValue},
    lua2cpp::{L2CAgentBase, L2CFighterCommon},
    phx::Hash40,
};
use smash_script::macros;
use smashline::{Agent, End, Exec, ExecStop, Exit, FixCamera, Init, Main, MapCorrection, Pre, Priority};

static THROWN: AtomicBool = AtomicBool::new(false);

unsafe extern "C" fn no_op(_fighter: &mut L2CFighterCommon) -> L2CValue {
    0.into()
}

unsafe extern "C" fn special_n_pre(fighter: &mut L2CFighterCommon) -> L2CValue {
    fighter.sub_status_pre_SpecialNCommon();
    StatusModule::init_settings(
        fighter.module_accessor,
        SituationKind(*SITUATION_KIND_NONE),
        *FIGHTER_KINETIC_TYPE_UNIQ,
        *GROUND_CORRECT_KIND_KEEP as u32,
        GroundCliffCheckKind(*GROUND_CLIFF_CHECK_KIND_NONE),
        true,
        *FIGHTER_STATUS_WORK_KEEP_FLAG_ALL_FLAG,
        *FIGHTER_STATUS_WORK_KEEP_FLAG_ALL_INT,
        *FIGHTER_STATUS_WORK_KEEP_FLAG_ALL_FLOAT,
        0,
    );
    0.into()
}

unsafe extern "C" fn special_n_init(_fighter: &mut L2CFighterCommon) -> L2CValue {
    THROWN.store(false, Ordering::Relaxed);
    0.into()
}

unsafe extern "C" fn special_n_main(fighter: &mut L2CFighterCommon) -> L2CValue {
    let motion = if StatusModule::situation_kind(fighter.module_accessor) == *SITUATION_KIND_AIR {
        "template_fighter_special_air_n" // the manifest's kirby_motion names
    } else {
        "template_fighter_special_n"
    };
    MotionModule::change_motion(fighter.module_accessor, Hash40::new(motion), 0.0, 1.0, false, 0.0, false, false);
    fighter.sub_shift_status_main(L2CValue::Ptr(special_n_loop as *const () as _))
}

unsafe extern "C" fn special_n_loop(fighter: &mut L2CFighterCommon) -> L2CValue {
    let boma = fighter.module_accessor;
    if MotionModule::frame(boma) >= 12.0 && !THROWN.swap(true, Ordering::Relaxed) {
        TEMPLATE.copy_article("template_fighter_fireball").spawn(boma); // the manifest's kirby_article
    }
    if MotionModule::is_end(boma) {
        let next = if StatusModule::situation_kind(boma) == *SITUATION_KIND_AIR {
            *FIGHTER_STATUS_KIND_FALL
        } else {
            *FIGHTER_STATUS_KIND_WAIT
        };
        StatusModule::change_status_request_from_script(boma, next, false);
        return 1.into();
    }
    0.into()
}

unsafe extern "C" fn copy_special_game(agent: &mut L2CAgentBase) {
    frame(agent.lua_state_agent, 12.0);
    if macros::is_excute(agent) {
        macros::ATTACK(
            agent, 0, 0, Hash40::new("top"), 3.0, 361, 30, 0, 20, 4.0, 0.0, 8.0, 8.0,
            None, None, None, 1.0, 1.0, *ATTACK_SETOFF_KIND_ON, *ATTACK_LR_CHECK_F,
            false, 0, 0.0, 0, false, false, false, false, true,
            *COLLISION_SITUATION_MASK_GA, *COLLISION_CATEGORY_MASK_ALL,
            *COLLISION_PART_MASK_ALL, false, Hash40::new("collision_attr_fire"),
            *ATTACK_SOUND_LEVEL_S, *COLLISION_SOUND_ATTR_FIRE, *ATTACK_REGION_ENERGY,
        );
    }
    wait(agent.lua_state_agent, 3.0);
    if macros::is_excute(agent) {
        AttackModule::clear_all(agent.module_accessor);
    }
}

unsafe extern "C" fn copy_special_sound(agent: &mut L2CAgentBase) {
    frame(agent.lua_state_agent, 12.0);
    if macros::is_excute(agent) {
        macros::PLAY_SE(agent, Hash40::new("se_common_swing_02"));
    }
}

unsafe extern "C" fn copy_special_effect(agent: &mut L2CAgentBase) {
    frame(agent.lua_state_agent, 12.0);
    if macros::is_excute(agent) {
        macros::EFFECT_FOLLOW(
            agent, Hash40::new("sys_attack_line"), Hash40::new("top"),
            0.0, 7.0, 2.0, 0.0, 0.0, 0.0, 0.8, true,
        );
    }
}

unsafe extern "C" fn copy_special_expression(agent: &mut L2CAgentBase) {
    frame(agent.lua_state_agent, 12.0);
    if macros::is_excute(agent) {
        ControlModule::set_rumble(
            agent.module_accessor, Hash40::new("rbkind_attacks"), 0, false,
            *BATTLE_OBJECT_ID_INVALID as u32,
        );
    }
}

// The scripts below are the copied MOVE. The vtable entries Kirby runs while
// he holds the copy are copy_slot hooks, in hooks.rs.
pub fn install() {
    let Some((_, count)) = TEMPLATE.kirby_family() else {
        return;
    };
    if count < 1 {
        return;
    }
    let special_n = TEMPLATE.kirby_status(0); // the first status number reserved by .kirby(1)
    Agent::new("kirby")                        // Kirby's copy scripts go on Kirby
        .status(Pre, special_n, special_n_pre) // status line, the reserved status, function
        .status(Init, special_n, special_n_init)
        .status(Main, special_n, special_n_main)
        .status(End, special_n, no_op)
        .status(Exec, special_n, no_op)
        .status(ExecStop, special_n, no_op)
        .status(Exit, special_n, no_op)
        .status(MapCorrection, special_n, no_op)
        .status(FixCamera, special_n, no_op)
        // the kirby_motion scripts: game_ + the .scripts() stem, one set per motion
        .game_acmd("game_templatefighterspecialn", copy_special_game, Priority::Default)
        .sound_acmd("sound_templatefighterspecialn", copy_special_sound, Priority::Default)
        .effect_acmd("effect_templatefighterspecialn", copy_special_effect, Priority::Default)
        .expression_acmd("expression_templatefighterspecialn", copy_special_expression, Priority::Default)
        .game_acmd("game_templatefighterspecialairn", copy_special_game, Priority::Default)
        .sound_acmd("sound_templatefighterspecialairn", copy_special_sound, Priority::Default)
        .effect_acmd("effect_templatefighterspecialairn", copy_special_effect, Priority::Default)
        .expression_acmd("expression_templatefighterspecialairn", copy_special_expression, Priority::Default)
        .install();
    if !TEMPLATE.arm_kirby() { // after the statuses above are installed
        clone_engine_api::elog!("[template_v2] the Kirby copy family could not be armed");
    }
}
