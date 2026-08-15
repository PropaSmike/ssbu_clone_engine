use crate::smash;
use clone_engine_api::{HookCall, SharedHookRegistrationV1, HOOK_DECLINED};
use core::sync::atomic::{AtomicBool, Ordering};

const TARGET: u64 = 0x00aa_6990;
const ENTRY_OPCODES: [u32; 4] = [0xd101_83ff, 0xa903_57f6, 0xa904_4ff4, 0xa905_7bfd];
static LOGGED: AtomicBool = AtomicBool::new(false);

unsafe extern "C" fn observe(call: *mut HookCall) -> u32 {
    if call.is_null() {
        return HOOK_DECLINED;
    }
    let call = &mut *call;
    let boma = call.args[1] as *mut smash::app::BattleObjectModuleAccessor;
    let Some(kind) = crate::kind() else {
        return HOOK_DECLINED;
    };
    if clone_engine_api::true_kind(boma) != kind {
        return HOOK_DECLINED;
    }
    if !LOGGED.swap(true, Ordering::Relaxed) {
        clone_engine_api::elog!("[template] shared hook observed its clone");
    }
    HOOK_DECLINED
}

pub fn install() {
    let registration = SharedHookRegistrationV1::new(TARGET, ENTRY_OPCODES, 3, observe);
    match unsafe { clone_engine_api::shared_hook_checked(&registration) } {
        Ok(()) => clone_engine_api::elog!("[template] shared hook registered"),
        Err(error) => clone_engine_api::elog!("[template] shared hook declined: {error:?}"),
    }
}
