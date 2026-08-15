use std::sync::OnceLock;

use crate::smash;
use clone_engine_api::{ArticleHandle, ArticleStatusLine};
use smash::{
    app::lua_bind::*,
    lib::{lua_const::*, L2CValue},
    lua2cpp::L2CWeaponCommon,
};

static EXTRA_FIREBALL: OnceLock<ArticleHandle> = OnceLock::new();

unsafe extern "C" fn fireball_exec(weapon: &mut L2CWeaponCommon) -> L2CValue {
    let owner = clone_engine_api::owner_true_kind(weapon.module_accessor);
    let Some(kind) = crate::kind() else {
        return 0.into();
    };
    if owner != kind {
        return 0.into();
    }
    0.into()
}

pub fn install(kind: i32) {
    let handle = match clone_engine_api::clone_article_handle(
        "mario",
        *smash::lib::lua_const::WEAPON_KIND_MARIO_FIREBALL,
        crate::RESOURCE_NAME,
        "template_fireball",
        crate::BASE_FIGHTER_KIND,
    ) {
        Ok(handle) => handle,
        Err(error) => {
            clone_engine_api::elog!("[template] article registration failed: {error:?}");
            return;
        }
    };

    let weapon_kind = handle.weapon_kind();
    if let Err(error) = clone_engine_api::article_status(
        weapon_kind,
        ArticleStatusLine::Exec,
        *smash::lib::lua_const::WEAPON_MARIO_FIREBALL_STATUS_KIND_REGULAR,
        fireball_exec as *const (),
    ) {
        clone_engine_api::elog!("[template] article status registration failed: {error:?}");
    }

    let mut policies = clone_engine_api::param_villager_pocket_behavior(
        kind,
        weapon_kind,
        clone_engine_api::PARAM_BEHAVIOR_ORIGINAL,
    );
    policies &= clone_engine_api::param_rosetta_pull_behavior(
        kind,
        weapon_kind,
        clone_engine_api::PARAM_BEHAVIOR_ORIGINAL,
    );
    policies &= clone_engine_api::param_kirby_inhale_behavior(
        kind,
        weapon_kind,
        clone_engine_api::PARAM_BEHAVIOR_ORIGINAL,
    );
    if !policies {
        clone_engine_api::elog!("[template] ParamConfig interaction bridge unavailable");
    }

    let _ = EXTRA_FIREBALL.set(handle);
    clone_engine_api::elog!("[template] minted article weapon_kind={weapon_kind}");
}

pub unsafe fn spawn(module_accessor: *mut smash::app::BattleObjectModuleAccessor) {
    let Some(handle) = EXTRA_FIREBALL.get() else {
        return;
    };
    let Some(index) = handle.index() else {
        clone_engine_api::elog!(
            "[template] article kind {} is not in the active table",
            handle.weapon_kind()
        );
        return;
    };
    ArticleModule::generate_article(module_accessor, index, false, 0);
}

#[allow(dead_code)]
fn clone_article_for_example() -> Result<i32, clone_engine_api::Error> {
    clone_engine_api::clone_article_for(
        "mario",
        *smash::lib::lua_const::WEAPON_KIND_MARIO_FIREBALL,
        crate::RESOURCE_NAME,
        crate::RESOURCE_NAME,
        "template_fireball_for",
    )
}
