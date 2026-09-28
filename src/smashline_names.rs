use crate::current_thread_key;
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

const FIGHTER_NAME_TABLE: usize = 0x4f83e20;
const WEAPON_NAME_TABLE: usize = 0x5188bd0;
const WEAPON_OWNER_NAME_TABLE: usize = 0x518b240;

const FIGHTER_NAME_COUNT: i32 = 118;
const WEAPON_NAME_COUNT: i32 = 0x267;

static ARMED: AtomicU32 = AtomicU32::new(0);
static REFUSED: AtomicU32 = AtomicU32::new(0);
static WRITE_FAILED: AtomicU32 = AtomicU32::new(0);
static ARM_LOG: AtomicU32 = AtomicU32::new(0);

pub(crate) struct NameGuard {
    slot: usize,
    previous: usize,
}

impl NameGuard {
    pub(crate) const fn inactive() -> Self {
        Self {
            slot: 0,
            previous: 0,
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        self.slot != 0
    }
}

impl Drop for NameGuard {
    fn drop(&mut self) {
        if self.slot == 0 {
            return;
        }
        let restored =
            unsafe { crate::text_patch::write_bytes(self.slot, &self.previous.to_le_bytes()) };
        if !restored {
            WRITE_FAILED.fetch_add(1, Ordering::Relaxed);
            let n = ARM_LOG.fetch_add(1, Ordering::Relaxed);
            if n < 8 {
                crate::dbg_log_public(&format!(
                    "[slname] #{n} RESTORE FAILED slot {:#x}; the table now holds a clone name",
                    self.slot
                ));
            }
        }
    }
}

unsafe fn hold(table: usize, count: i32, index: i32, name: *const u8) -> NameGuard {
    if name.is_null() || !(0..count).contains(&index) {
        REFUSED.fetch_add(1, Ordering::Relaxed);
        return NameGuard::inactive();
    }
    let slot = crate::text_base() + table + (index as usize) * 8;
    let previous = core::ptr::read_volatile(slot as *const usize);
    let wanted = name as usize;
    if previous == wanted {
        return NameGuard::inactive();
    }
    if !crate::text_patch::write_bytes(slot, &wanted.to_le_bytes()) {
        let failed = WRITE_FAILED.fetch_add(1, Ordering::Relaxed);
        if failed < 4 {
            crate::dbg_log_public(&format!(
                "[slname] ARM FAILED slot {index} table {table:#x} at {slot:#x}; sky_memcpy refused this page, so agent names will fall back to the base"
            ));
        }
        return NameGuard::inactive();
    }
    let armed = ARMED.fetch_add(1, Ordering::Relaxed);
    let n = ARM_LOG.fetch_add(1, Ordering::Relaxed);
    if n < 12 {
        crate::dbg_log_public(&format!(
            "[slname] #{n} armed slot {index} in table {table:#x} thread {:#x} (armed {armed})",
            current_thread_key()
        ));
    }
    NameGuard {
        slot,
        previous,
    }
}

pub(crate) unsafe fn hold_fighter_name(base_kind: i32, name: &'static [u8]) -> NameGuard {
    hold(
        FIGHTER_NAME_TABLE,
        FIGHTER_NAME_COUNT,
        base_kind,
        name.as_ptr(),
    )
}

pub(crate) unsafe fn hold_weapon_name(source_kind: i32, name: &'static [u8]) -> NameGuard {
    hold(
        WEAPON_NAME_TABLE,
        WEAPON_NAME_COUNT,
        source_kind,
        name.as_ptr(),
    )
}

pub(crate) unsafe fn hold_weapon_owner_name(source_kind: i32, name: &'static [u8]) -> NameGuard {
    hold(
        WEAPON_OWNER_NAME_TABLE,
        WEAPON_NAME_COUNT,
        source_kind,
        name.as_ptr(),
    )
}

pub(crate) fn stats() -> (u32, u32, u32) {
    (
        ARMED.load(Ordering::Relaxed),
        REFUSED.load(Ordering::Relaxed),
        WRITE_FAILED.load(Ordering::Relaxed),
    )
}

pub(crate) unsafe fn hold_clone_fighter_name(clone_kind: Option<i32>) -> NameGuard {
    let Some(definition) = clone_kind.and_then(crate::clone_definition) else {
        return NameGuard::inactive();
    };
    hold_fighter_name(definition.base_kind, definition.resource_name_cstr)
}

static VANILLA_WEAPON_NAME: [AtomicUsize; WEAPON_NAME_COUNT as usize] =
    [const { AtomicUsize::new(0) }; WEAPON_NAME_COUNT as usize];

static VANILLA_WEAPON_OWNER: [AtomicUsize; WEAPON_NAME_COUNT as usize] =
    [const { AtomicUsize::new(0) }; WEAPON_NAME_COUNT as usize];

fn remember_vanilla(cache: &[AtomicUsize], index: i32, pointer: usize) {
    let Ok(slot) = usize::try_from(index) else {
        return;
    };
    let Some(cell) = cache.get(slot) else {
        return;
    };
    let _ = cell.compare_exchange(0, pointer, Ordering::AcqRel, Ordering::Acquire);
}

fn vanilla(cache: &[AtomicUsize], index: i32) -> Option<usize> {
    let slot = usize::try_from(index).ok()?;
    let pointer = cache.get(slot)?.load(Ordering::Acquire);
    (pointer != 0).then_some(pointer)
}

pub(crate) fn vanilla_weapon_name(index: i32) -> Option<usize> {
    vanilla(&VANILLA_WEAPON_NAME, index)
}

pub(crate) fn vanilla_weapon_owner_name(index: i32) -> Option<usize> {
    vanilla(&VANILLA_WEAPON_OWNER, index)
}

pub(crate) struct WeaponNameHold {
    _name: NameGuard,
    _owner: NameGuard,
}

impl WeaponNameHold {
    pub(crate) const fn inactive() -> Self {
        Self {
            _name: NameGuard::inactive(),
            _owner: NameGuard::inactive(),
        }
    }
}

pub(crate) unsafe fn hold_clone_weapon_names(
    source_kind: i32,
    minted_kind: i32,
) -> WeaponNameHold {
    if crate::custom_articles::is_kirby_copy_weapon(minted_kind) {
        return WeaponNameHold::inactive();
    }
    let name = crate::custom_articles::custom_weapon_name(minted_kind);
    let owner = crate::custom_articles::custom_weapon_owner_name(minted_kind);
    let (Some(name), Some(owner)) = (name, owner) else {
        return WeaponNameHold::inactive();
    };
    let name_guard = hold_weapon_name(source_kind, name);
    if name_guard.is_active() {
        remember_vanilla(&VANILLA_WEAPON_NAME, source_kind, name_guard.previous);
    }
    let owner_guard = hold_weapon_owner_name(source_kind, owner);
    if owner_guard.is_active() {
        remember_vanilla(&VANILLA_WEAPON_OWNER, source_kind, owner_guard.previous);
    }
    WeaponNameHold {
        _name: name_guard,
        _owner: owner_guard,
    }
}
