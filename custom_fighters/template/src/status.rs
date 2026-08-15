use crate::smash;
use smash::{lib::L2CValue, lua2cpp::L2CFighterCommon};

unsafe extern "C" fn wait_exec(_fighter: &mut L2CFighterCommon) -> L2CValue {
    0.into()
}

pub fn install() {
    smashline::Agent::new(crate::RESOURCE_NAME)
        .status(
            smashline::Exec,
            *smash::lib::lua_const::FIGHTER_STATUS_KIND_WAIT,
            wait_exec,
        )
        .install();
}
