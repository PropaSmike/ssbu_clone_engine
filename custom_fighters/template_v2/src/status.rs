
use crate::smash;
use smash::{lib::L2CValue, lua2cpp::L2CFighterCommon};

pub unsafe extern "C" fn wait_exec(_fighter: &mut L2CFighterCommon) -> L2CValue {
    0.into()
}
