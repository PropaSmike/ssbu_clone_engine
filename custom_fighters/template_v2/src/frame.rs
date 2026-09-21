
use core::sync::atomic::{AtomicBool, Ordering};

use crate::smash;
use smash::lua2cpp::L2CFighterCommon;

static LOGGED: AtomicBool = AtomicBool::new(false);

pub unsafe extern "C" fn on_start(fighter: &mut L2CFighterCommon) {
    clone_engine_api::elog!(
        "[template_v2] fighter start; is mine = {}, kind {}",
        crate::TEMPLATE.is(&*fighter),
        crate::TEMPLATE.kind()
    );
}

pub unsafe extern "C" fn once_per_frame(fighter: &mut L2CFighterCommon) {
    if !LOGGED.swap(true, Ordering::Relaxed) {
        clone_engine_api::elog!(
            "[template_v2] first frame; entries {:?}",
            crate::TEMPLATE.entries()
        );
    }
}
