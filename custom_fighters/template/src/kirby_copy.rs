use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use crate::smash;
use clone_engine_api::ArticleHandle;
use smash::{
    app::{lua_bind::*, GroundCliffCheckKind, SituationKind},
    lib::{lua_const::*, L2CValue},
    lua2cpp::L2CFighterCommon,
    phx::Hash40,
};
use smashline::{Agent, End, Exec, ExecStop, Exit, FixCamera, Init, Main, MapCorrection, Pre};

pub const STATUS_FIRST: i32 = 0x520;
pub const STATUS_COUNT: i32 = 1;
const STATUS_SPECIAL_N: i32 = STATUS_FIRST;

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
        "mario_special_air_n"
    } else {
        "mario_special_n"
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

pub fn install(kind: i32) {
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
        .install();

    if !clone_engine_api::arm_kirby_copy_status_family(kind, STATUS_FIRST, STATUS_COUNT) {
        clone_engine_api::elog!("[template] Kirby status family could not be armed");
    }
}
