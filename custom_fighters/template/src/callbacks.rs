use crate::smash;
use core::sync::atomic::{AtomicBool, Ordering};
use smash::lua2cpp::L2CFighterCommon;
use smashline::{Agent, Main};

static LOGGED_INSTANCE: AtomicBool = AtomicBool::new(false);

unsafe extern "C" fn on_start(fighter: &mut L2CFighterCommon) {
    let Some(kind) = crate::kind() else { return };
    if !clone_engine_api::is_kind(fighter.module_accessor, kind) {
        return;
    }
    let resolved = clone_engine_api::true_kind(fighter.module_accessor);
    clone_engine_api::elog!("[template] fighter start true_kind={resolved}");
}

unsafe extern "C" fn on_line_main(fighter: &mut L2CFighterCommon) {
    let Some(kind) = crate::kind() else { return };
    if !clone_engine_api::is_kind(fighter.module_accessor, kind) {
        return;
    }

    if !LOGGED_INSTANCE.swap(true, Ordering::Relaxed) {
        let entry = smash::app::lua_bind::WorkModule::get_int(
            fighter.module_accessor,
            *smash::lib::lua_const::FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
        );
        clone_engine_api::elog!(
            "[template] OPFF entry={} entry_kind={} true_kind={}",
            entry,
            clone_engine_api::entry_kind(entry),
            clone_engine_api::true_kind(fighter.module_accessor)
        );
    }
}

pub fn install() {
    Agent::new(crate::RESOURCE_NAME)
        .on_line(Main, on_line_main)
        .on_start(on_start)
        .install();
}
