use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use crate::smash;
use clone_engine_api::ArticleHandle;
use smash::{
    app::{lua_bind::*, sv_animcmd::*, GroundCliffCheckKind, SituationKind},
    lib::{lua_const::*, L2CValue},
    lua2cpp::{L2CAgentBase, L2CFighterCommon},
    phx::Hash40,
};
use smash_script::macros;
use smashline::{Agent, End, Exec, ExecStop, Exit, FixCamera, Init, Main, MapCorrection, Pre};

pub const STATUS_FIRST: i32 = 0x520;
pub const STATUS_COUNT: i32 = 1;
const STATUS_SPECIAL_N: i32 = STATUS_FIRST;

const COPY_MOTION_GROUND: &str = "template_fighter_special_n";
const COPY_MOTION_AIR: &str = "template_fighter_special_air_n";
const COPY_MOTION_GROUND_ANIMATION: &str = "template_fighterd00specialn.nuanmb";
const COPY_MOTION_AIR_ANIMATION: &str = "template_fighterd00specialairn.nuanmb";
const COPY_MOTION_GROUND_TEMPLATE: &str = "mario_special_n";
const COPY_MOTION_AIR_TEMPLATE: &str = "mario_special_air_n";

const COPY_GROUND_GAME: &str = "game_templatefighterspecialn";
const COPY_GROUND_SOUND: &str = "sound_templatefighterspecialn";
const COPY_GROUND_EFFECT: &str = "effect_templatefighterspecialn";
const COPY_GROUND_EXPRESSION: &str = "expression_templatefighterspecialn";
const COPY_AIR_GAME: &str = "game_templatefighterspecialairn";
const COPY_AIR_SOUND: &str = "sound_templatefighterspecialairn";
const COPY_AIR_EFFECT: &str = "effect_templatefighterspecialairn";
const COPY_AIR_EXPRESSION: &str = "expression_templatefighterspecialairn";

static COPY_FIREBALL: OnceLock<ArticleHandle> = OnceLock::new();
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
        COPY_MOTION_AIR
    } else {
        COPY_MOTION_GROUND
    };
    MotionModule::change_motion(
        fighter.module_accessor,
        Hash40::new(motion),
        0.0,
        1.0,
        false,
        0.0,
        false,
        false,
    );
    fighter.sub_shift_status_main(L2CValue::Ptr(special_n_loop as *const () as _))
}

unsafe extern "C" fn special_n_loop(fighter: &mut L2CFighterCommon) -> L2CValue {
    let boma = fighter.module_accessor;
    if MotionModule::frame(boma) >= 12.0 && !THROWN.swap(true, Ordering::Relaxed) {
        if let Some(index) = COPY_FIREBALL.get().and_then(ArticleHandle::index) {
            ArticleModule::generate_article(boma, index, false, 0);
        }
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

unsafe extern "C" fn special_n_exit(_fighter: &mut L2CFighterCommon) -> L2CValue {
    THROWN.store(false, Ordering::Relaxed);
    0.into()
}

unsafe extern "C" fn copy_special_game(agent: &mut L2CAgentBase) {
    frame(agent.lua_state_agent, 12.0);
    if macros::is_excute(agent) {
        macros::ATTACK(
            agent,
            0,
            0,
            Hash40::new("top"),
            3.0,
            361,
            30,
            0,
            20,
            4.0,
            0.0,
            8.0,
            8.0,
            None,
            None,
            None,
            1.0,
            1.0,
            *ATTACK_SETOFF_KIND_ON,
            *ATTACK_LR_CHECK_F,
            false,
            0,
            0.0,
            0,
            false,
            false,
            false,
            false,
            true,
            *COLLISION_SITUATION_MASK_GA,
            *COLLISION_CATEGORY_MASK_ALL,
            *COLLISION_PART_MASK_ALL,
            false,
            Hash40::new("collision_attr_fire"),
            *ATTACK_SOUND_LEVEL_S,
            *COLLISION_SOUND_ATTR_FIRE,
            *ATTACK_REGION_ENERGY,
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
            agent,
            Hash40::new("sys_attack_line"),
            Hash40::new("top"),
            0.0,
            7.0,
            2.0,
            0.0,
            0.0,
            0.0,
            0.8,
            true,
        );
    }
}

unsafe extern "C" fn copy_special_expression(agent: &mut L2CAgentBase) {
    frame(agent.lua_state_agent, 12.0);
    if macros::is_excute(agent) {
        ControlModule::set_rumble(
            agent.module_accessor,
            Hash40::new("rbkind_attacks"),
            0,
            false,
            *BATTLE_OBJECT_ID_INVALID as u32,
        );
    }
}

fn register_copy_motions(kind: i32) {
    for (name, animation, template, game, sound, effect, expression) in [
        (
            COPY_MOTION_GROUND,
            COPY_MOTION_GROUND_ANIMATION,
            COPY_MOTION_GROUND_TEMPLATE,
            COPY_GROUND_GAME,
            COPY_GROUND_SOUND,
            COPY_GROUND_EFFECT,
            COPY_GROUND_EXPRESSION,
        ),
        (
            COPY_MOTION_AIR,
            COPY_MOTION_AIR_ANIMATION,
            COPY_MOTION_AIR_TEMPLATE,
            COPY_AIR_GAME,
            COPY_AIR_SOUND,
            COPY_AIR_EFFECT,
            COPY_AIR_EXPRESSION,
        ),
    ] {
        let motion = clone_engine_api::CopyMotion::new(name, animation)
            .template(template)
            .game_script(game)
            .scripts(sound, effect, expression);
        if let Err(error) = clone_engine_api::clone_copy_motion(kind, &motion) {
            clone_engine_api::elog!("[template] copy motion '{name}' refused: {error:?}");
        }
    }
}

pub fn install(kind: i32) {
    register_copy_motions(kind);

    match clone_engine_api::clone_copy_article_handle(
        kind,
        "mario",
        *smash::lib::lua_const::WEAPON_KIND_MARIO_FIREBALL,
        "kirby",
        "template_fireball",
    ) {
        Ok(handle) => {
            let _ = COPY_FIREBALL.set(handle);
        }
        Err(error) => {
            clone_engine_api::elog!("[template] Kirby article registration failed: {error:?}")
        }
    }

    Agent::new("kirby")
        .status(Pre, STATUS_SPECIAL_N, special_n_pre)
        .status(Init, STATUS_SPECIAL_N, special_n_init)
        .status(Main, STATUS_SPECIAL_N, special_n_main)
        .status(End, STATUS_SPECIAL_N, no_op)
        .status(Exec, STATUS_SPECIAL_N, no_op)
        .status(ExecStop, STATUS_SPECIAL_N, no_op)
        .status(Exit, STATUS_SPECIAL_N, special_n_exit)
        .status(MapCorrection, STATUS_SPECIAL_N, no_op)
        .status(FixCamera, STATUS_SPECIAL_N, no_op)
        .game_acmd(COPY_GROUND_GAME, copy_special_game, smashline::Priority::Default)
        .sound_acmd(COPY_GROUND_SOUND, copy_special_sound, smashline::Priority::Default)
        .effect_acmd(COPY_GROUND_EFFECT, copy_special_effect, smashline::Priority::Default)
        .expression_acmd(
            COPY_GROUND_EXPRESSION,
            copy_special_expression,
            smashline::Priority::Default,
        )
        .game_acmd(COPY_AIR_GAME, copy_special_game, smashline::Priority::Default)
        .sound_acmd(COPY_AIR_SOUND, copy_special_sound, smashline::Priority::Default)
        .effect_acmd(COPY_AIR_EFFECT, copy_special_effect, smashline::Priority::Default)
        .expression_acmd(
            COPY_AIR_EXPRESSION,
            copy_special_expression,
            smashline::Priority::Default,
        )
        .install();

    if !clone_engine_api::arm_kirby_copy_status_family(kind, STATUS_FIRST, STATUS_COUNT) {
        clone_engine_api::elog!("[template] Kirby status family could not be armed");
    }
}
