
use crate::smash;
use crate::TEMPLATE;
use smash::{
    app::{lua_bind::*, GroundCliffCheckKind, SituationKind},
    lib::{lua_const::*, L2CValue},
    lua2cpp::L2CFighterCommon,
    phx::Hash40,
};
use smashline::{Agent, End, Exec, ExecStop, Exit, FixCamera, Init, Main, MapCorrection, Pre};

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

unsafe extern "C" fn special_n_main(fighter: &mut L2CFighterCommon) -> L2CValue {
    let motion = if StatusModule::situation_kind(fighter.module_accessor) == *SITUATION_KIND_AIR {
        "mario_special_air_n"
    } else {
        "mario_special_n"
    };
    MotionModule::change_motion(fighter.module_accessor, Hash40::new(motion), 0.0, 1.0, false, 0.0, false, false);
    fighter.sub_shift_status_main(L2CValue::Ptr(special_n_loop as *const () as _))
}

unsafe extern "C" fn special_n_loop(fighter: &mut L2CFighterCommon) -> L2CValue {
    let boma = fighter.module_accessor;
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
        .status(Init, special_n, no_op)
        .status(Main, special_n, special_n_main)
        .status(End, special_n, no_op)
        .status(Exec, special_n, no_op)
        .status(ExecStop, special_n, no_op)
        .status(Exit, special_n, no_op)
        .status(MapCorrection, special_n, no_op)
        .status(FixCamera, special_n, no_op)
        .install();
    if !TEMPLATE.arm_kirby() { // after the statuses above are installed
        clone_engine_api::elog!("[template_v2] the Kirby copy family could not be armed");
    }
}
