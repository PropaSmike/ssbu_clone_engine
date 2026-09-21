
use crate::smash;
use crate::TEMPLATE;
use clone_engine_api::v2::Line;
use smash::{
    lib::{lua_const::*, L2CValue},
    lua2cpp::L2CWeaponCommon,
};

unsafe extern "C" fn fireball_exec(_weapon: &mut L2CWeaponCommon) -> L2CValue {
    0.into()
}

pub fn install() {
    TEMPLATE
        .article("template_fireball") // the article declared in the manifest
        .status(Line::Exec, *WEAPON_MARIO_FIREBALL_STATUS_KIND_REGULAR, fireball_exec) // status line, a status kind of the source weapon, function
        .install();
    // Its ACMD goes on Agent::new("template_fighter_template_fireball").
}
