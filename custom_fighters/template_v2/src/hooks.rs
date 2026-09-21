use core::sync::atomic::{AtomicBool, Ordering};

use crate::smash;
use clone_engine_api::v2::{hook, slot};

static SEEN: AtomicBool = AtomicBool::new(false);

// slot: the fighter vtable entry to replace. class: the class object the game
// passes first; object: the fighter; event: the entry's own argument.
#[hook(slot = slot::fighter::ON_LINK_EVENT)]
unsafe fn on_link_event(class: u64, object: *mut smash::app::BattleObject, event: u64) {
    if !SEEN.swap(true, Ordering::Relaxed) {
        clone_engine_api::elog!("[template_v2] the fighter's own vtable entry ran");
    }
    call_original!(class, object, event) // the base's entry, same arguments
}

pub fn install() {
    clone_engine_api::install_hooks!(on_link_event); // every #[hook] in the crate, after registration
}
