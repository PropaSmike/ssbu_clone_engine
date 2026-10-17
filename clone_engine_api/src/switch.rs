use core::sync::atomic::AtomicUsize;

use smash::app::lua_bind::WorkModule;
use smash::app::BattleObjectModuleAccessor;
use smash::lib::lua_const::*;

use crate::resolve;

type NextFn = extern "C" fn(i32) -> bool;

static NEXT_FN: AtomicUsize = AtomicUsize::new(0);

pub unsafe fn next(boma: *mut BattleObjectModuleAccessor) -> bool {
    if boma.is_null() {
        return false;
    }
    let Some(address) = resolve(&NEXT_FN, b"clone_engine_switch_next_v1\0") else {
        return false;
    };
    let function: NextFn = core::mem::transmute(address);
    function(WorkModule::get_int(boma, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID))
}
