use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};

const EFFECT_BANK_LOAD: usize = 0x3560a00;
const FIGHTER_EFFECT_HANDLE_BASE: u32 = 0x300;
const FIGHTER_EFFECT_HANDLE_END: u32 = 0x400;
const EFFECT_DIRECTORY_PATH_TYPE: i32 = 20;

const MAX_OWNERS: usize = 8;
const RESOURCE_ENTRIES: i32 = 8;
const MAX_LOG_LINES: u32 = 32;
const LOWEST_PLAUSIBLE_POINTER: usize = 0x1_0000;

struct Owner {
    kind: AtomicI32,
    file_index: AtomicI32,
    staged: AtomicBool,
    loaded: AtomicBool,
}

impl Owner {
    const fn new() -> Self {
        Self {
            kind: AtomicI32::new(-1),
            file_index: AtomicI32::new(-1),
            staged: AtomicBool::new(false),
            loaded: AtomicBool::new(false),
        }
    }
}

static OWNERS: [Owner; MAX_OWNERS] = [const { Owner::new() }; MAX_OWNERS];
static MANAGER: AtomicUsize = AtomicUsize::new(0);
static LOG_LINES: AtomicU32 = AtomicU32::new(0);

fn log(message: String) {
    crate::dbg_log_public(&message);
}

fn limited_log(message: String) {
    if LOG_LINES.fetch_add(1, Ordering::Relaxed) < MAX_LOG_LINES {
        log(message);
    }
}

pub(crate) fn note_registered_base_kind(base_kind: i32) {
    let Some(owner) = crate::item_clones::base_item_owner(base_kind) else {
        return;
    };
    if owner < 0 {
        return;
    }
    for slot in &OWNERS {
        match slot
            .kind
            .compare_exchange(-1, owner, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => {
                log(format!(
                    "[ownereffect] base item {base_kind:#x} belongs to fighter kind {owner:#x}; \
                     its effect bank (handle {:#x}) will be staged at load and bound when a \
                     clone item is built",
                    FIGHTER_EFFECT_HANDLE_BASE + owner as u32
                ));
                return;
            }
            Err(found) if found == owner => return,
            Err(_) => continue,
        }
    }
    log(format!(
        "[ownereffect] no free owner slot for fighter kind {owner:#x} (base item {base_kind:#x})"
    ));
}

pub(crate) fn is_fighter_handle(handle: u32) -> bool {
    (FIGHTER_EFFECT_HANDLE_BASE..FIGHTER_EFFECT_HANDLE_END).contains(&handle)
}

pub(crate) fn remember_manager(manager: usize) {
    if manager >= LOWEST_PLAUSIBLE_POINTER {
        MANAGER.store(manager, Ordering::Release);
    }
}

unsafe fn any_live_resource_record() -> Option<usize> {
    (0..RESOURCE_ENTRIES)
        .find_map(|entry| crate::load_pipeline::resource_record_for_entry(entry).filter(|r| *r != 0))
}

unsafe fn stage_one(owner: i32, slot: &Owner) -> bool {
    let Some(name) = crate::custom_articles::fighter_name(owner).and_then(|n| n.to_str().ok())
    else {
        return false;
    };
    let path = format!("effect/fighter/{name}/ef_{name}.eff");
    let hash = crate::hash40::hash40(&path);
    let Some(index) = crate::item_params::scan_file_path_index(hash) else {
        limited_log(format!("[ownereffect] {path} is not in the arc; effects stay unavailable"));
        return false;
    };
    slot.file_index.store(index as i32, Ordering::Release);
    crate::fighter_params::request_resource_file(index as i32);
    limited_log(format!(
        "[ownereffect] staged {path} (file index {index:#x}) for fighter kind {owner:#x}"
    ));
    true
}

pub(crate) unsafe fn stage_owner_banks() {
    for slot in &OWNERS {
        let owner = slot.kind.load(Ordering::Acquire);
        if owner < 0 || slot.staged.swap(true, Ordering::AcqRel) {
            continue;
        }
        if !stage_one(owner, slot) {
            slot.loaded.store(true, Ordering::Release);
        }
    }
}

pub(crate) unsafe fn ensure_owner_bank_bound(owner: i32) {
    if owner < 0 {
        return;
    }
    let Some(slot) = OWNERS
        .iter()
        .find(|slot| slot.kind.load(Ordering::Acquire) == owner)
    else {
        return;
    };
    if !slot.staged.load(Ordering::Acquire) || slot.loaded.load(Ordering::Acquire) {
        return;
    }
    let file_index = slot.file_index.load(Ordering::Acquire);
    if file_index < 0 {
        return;
    }
    crate::fighter_params::request_resource_file(file_index);
    let manager = MANAGER.load(Ordering::Acquire);
    if manager < LOWEST_PLAUSIBLE_POINTER {
        return;
    }
    let Some(record) = any_live_resource_record() else {
        return;
    };
    let Some(index) =
        crate::load_pipeline::resolve_resource_directory(record, owner, EFFECT_DIRECTORY_PATH_TYPE)
    else {
        limited_log(format!(
            "[ownereffect] fighter kind {owner:#x} has no effect directory (path type \
             {EFFECT_DIRECTORY_PATH_TYPE}); its bank cannot be bound"
        ));
        slot.loaded.store(true, Ordering::Release);
        return;
    };
    if slot.loaded.swap(true, Ordering::AcqRel) {
        return;
    }
    let handle = FIGHTER_EFFECT_HANDLE_BASE + owner as u32;
    let index_u32 = index as u32;
    limited_log(format!(
        "[ownereffect] binding effect bank handle={handle:#x} index={index_u32:#x} for fighter \
         kind {owner:#x}"
    ));
    type Load = unsafe extern "C" fn(usize, u32, *const u32) -> u32;
    let load: Load = core::mem::transmute(crate::text_base_public() + EFFECT_BANK_LOAD);
    let result = load(manager, handle, &index_u32 as *const u32);
    limited_log(format!(
        "[ownereffect] effect bank handle={handle:#x} bound, result={result}"
    ));
}
