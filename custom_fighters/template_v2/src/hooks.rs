use core::sync::atomic::{AtomicBool, Ordering};

use crate::smash;
use clone_engine_api::v2::{hook, slot};
use smash::lib::L2CValue;

static SEEN: AtomicBool = AtomicBool::new(false);
static COMMON_SEEN: AtomicBool = AtomicBool::new(false);
static FIREBALL_SEEN: AtomicBool = AtomicBool::new(false);
static COPY_SEEN: AtomicBool = AtomicBool::new(false);
static OFFSET_SEEN: AtomicBool = AtomicBool::new(false);

// slot: the fighter vtable entry to replace. class: the class object the game
// passes first; object: the fighter; event: the entry's own argument.
// Declare the entry's real return: the game reads it.
#[hook(slot = slot::fighter::ON_LINK_EVENT)]
unsafe fn on_link_event(class: u64, object: *mut smash::app::BattleObject, event: u64) -> u64 {
    if !SEEN.swap(true, Ordering::Relaxed) {
        clone_engine_api::elog!("[template_v2] the fighter's own vtable entry ran");
    }
    call_original!(class, object, event) // the base's entry, same arguments
}

// offset: a game function in main that is not a vtable entry, shared with
// other packs. me: the parameter that must be this fighter's (or one of its
// articles); any other call goes straight to the original.
#[hook(offset = 0x33bd9c0, me = weapon)]
unsafe fn weapon_hit(vtable: u64, weapon: *mut smash::app::Weapon, log: u32) -> u32 {
    if !OFFSET_SEEN.swap(true, Ordering::Relaxed) {
        clone_engine_api::elog!("[template_v2] the shared offset hook ran for this fighter");
    }
    call_original!(vtable, weapon, log)
}

// common_slot: an entry of L2CFighterCommon, the agent vtable every fighter
// shares. The engine writes it into a copy private to this clone's agent, so
// hooking it does not change any other fighter and another pack may hook the
// same entry. agent: the agent object itself, the only argument this one takes.
// Slots 10 and 12 return an L2CValue: declare it and return call_original!'s.
#[hook(common_slot = slot::common::SYS_LINE_SYSTEM_INIT)]
unsafe fn system_init(agent: u64) -> L2CValue {
    if !COMMON_SEEN.swap(true, Ordering::Relaxed) {
        clone_engine_api::elog!("[template_v2] the clone's own common vtable entry ran");
    }
    call_original!(agent) // the entry this replaced, vanilla or another pack's
}

// article: the same, in that article's own copy of L2CWeaponCommon.
#[hook(common_slot = slot::common::SUB_BEGIN_ADDED_LINES, article = "template_fireball")]
unsafe fn fireball_added_lines(agent: u64, lines: L2CValue) -> u64 {
    if !FIREBALL_SEEN.swap(true, Ordering::Relaxed) {
        clone_engine_api::elog!("[template_v2] the article's own common vtable entry ran");
    }
    call_original!(agent, lines)
}

// copy_slot: the entry Kirby runs while he holds this fighter's copy ability.
// The hooks above never reach him, because the object playing the copy is
// Kirby's, a vanilla kind. Slots 10 to 14 only, and it takes no other key.
// Slot 12 runs at every status end; slot 10 only as Kirby's agent starts,
// before he can hold a copy.
#[hook(copy_slot = slot::common::SYS_LINE_STATUS_END_CONTROL)]
unsafe fn kirby_status_end(agent: u64) -> L2CValue {
    if !COPY_SEEN.swap(true, Ordering::Relaxed) {
        clone_engine_api::elog!("[template_v2] the copy entry ran on Kirby");
    }
    call_original!(agent) // Kirby's own entry, also when he holds no copy
}

pub fn install() {
    // every #[hook] in the crate, after registration
    clone_engine_api::install_hooks!(
        on_link_event,
        weapon_hit,
        system_init,
        fireball_added_lines,
        kirby_status_end,
    );
}
