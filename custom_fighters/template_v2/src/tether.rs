//! An air tether and its ledge grab. lib.rs can take them from a donor
//! (`.tether("samus")`, commented out there): the engine then switches the
//! tether on and gives this fighter the donor's air_lasso_data and ledge boxes.
//! What the tether does is the donor's statuses, blank below. To use it:
//!
//! - mint the donor's articles in lib.rs (Samus: `.article("template_gbeam", "samus/gbeam")`)
//!   and ship the donor's `samus_vl.prc` in `fighter/template_fighter/param/`
//! - fill in each status from the donor's; the comments show Samus's, and
//!   docs/wiki/FIGHTERS.md#tethers lists every donor's
//! - port the donor's `aircatch` scripts, which spawn, shoot, rewind and
//!   remove the articles, and list `air_catch`, `air_catch_pose`,
//!   `air_catch_hang`, `air_catch_hit` and `air_catch_landing` in the motion list
//! - uncomment the tether in lib.rs
//!
//! Until then the fighter has no tether and these statuses never run.

use crate::smash;
use smash::lib::{lua_const::*, L2CValue};
use smash::lua2cpp::L2CFighterCommon;
use smashline::{Agent, End, Main};

// In the comments, gbeam is the minted beam's index, read right before use:
// TEMPLATE.article("template_gbeam").index().unwrap_or(*ARTICLE_ID_NONE)

unsafe extern "C" fn air_lasso_main(fighter: &mut L2CFighterCommon) -> L2CValue {
    // the tether's articles and its cliff hang data (1):
    // fighter.status_AirLasso(gbeam.into(), (*ARTICLE_ID_NONE).into(), 1.into(), true.into());
    0.into()
}

unsafe extern "C" fn reach_main(fighter: &mut L2CFighterCommon) -> L2CValue {
    // the article's REACH status (7):
    // fighter.status_AirLassoReach(7.into());
    0.into()
}

unsafe extern "C" fn reach_end(fighter: &mut L2CFighterCommon) -> L2CValue {
    // Samus and Lucas name the article first:
    // WorkModule::set_int(fighter.module_accessor, gbeam, *FIGHTER_STATUS_AIR_LASSO_REACH_WORK_INT_ARTICLE_STATUS);
    // fighter.status_end_AirLassoReach();
    0.into()
}

unsafe extern "C" fn hang_main(fighter: &mut L2CFighterCommon) -> L2CValue {
    // the article joint the fighter hangs from, the hang's cliff hang data (2):
    // fighter.status_AirLassoHang(hash40("gbeam27").into(), 2.into(), hash40("rot").into());
    0.into()
}

unsafe extern "C" fn rewind_main(fighter: &mut L2CFighterCommon) -> L2CValue {
    // the article's HANG_REWIND status (6):
    // fighter.status_AirLassoRewind(6.into());
    0.into()
}

unsafe extern "C" fn failure_main(fighter: &mut L2CFighterCommon) -> L2CValue {
    // the article's REWIND status (3), and a function of yours that calls
    // fighter.sub_air_lasso_failure_uniq(3.into()):
    // fighter.status_AirLassoFailure(L2CValue::Ptr(failure_uniq as *const () as _), hash40("air_catch").into(), 3.into());
    0.into()
}

unsafe extern "C" fn landing_main(fighter: &mut L2CFighterCommon) -> L2CValue {
    // the article's EXHAUST status (2):
    // fighter.status_AirLassoLanding(2.into());
    0.into()
}

pub fn install() {
    Agent::new("template_fighter")
        .status(Main, *FIGHTER_STATUS_KIND_AIR_LASSO, air_lasso_main) // status line, status kind, function
        .status(Main, *FIGHTER_STATUS_KIND_AIR_LASSO_REACH, reach_main)
        .status(End, *FIGHTER_STATUS_KIND_AIR_LASSO_REACH, reach_end)
        .status(Main, *FIGHTER_STATUS_KIND_AIR_LASSO_HANG, hang_main)
        .status(Main, *FIGHTER_STATUS_KIND_AIR_LASSO_REWIND, rewind_main)
        .status(Main, *FIGHTER_STATUS_KIND_AIR_LASSO_FAILURE, failure_main)
        .status(Main, *FIGHTER_STATUS_KIND_AIR_LASSO_LANDING, landing_main)
        .install();
}
