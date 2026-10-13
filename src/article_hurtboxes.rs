use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

use clone_engine_core::article_hurtbox::{
    Hurtbox, ACCESSOR_HIT_MODULE, HIT_MODULE_SET_HIT_DATA, OBJECT_MODULE_ACCESSOR, SETUP_SLOT,
};

use crate::clone_vtables::{self, Space};
use crate::custom_articles::{FIRST_CUSTOM_WEAPON_KIND, MAX_CUSTOM_ARTICLES};
use crate::dbg_log_public;

#[repr(C)]
struct HitDataList {
    entries: usize,
    count: u64,
}

type Setup = unsafe extern "C" fn(usize, usize) -> usize;
type SetHitData = unsafe extern "C" fn(usize, *const HitDataList, u32) -> usize;

static ORIGINAL_SETUP: [AtomicUsize; MAX_CUSTOM_ARTICLES] =
    [const { AtomicUsize::new(0) }; MAX_CUSTOM_ARTICLES];
static ENTRIES: [AtomicUsize; MAX_CUSTOM_ARTICLES] =
    [const { AtomicUsize::new(0) }; MAX_CUSTOM_ARTICLES];
static COUNTS: [AtomicUsize; MAX_CUSTOM_ARTICLES] =
    [const { AtomicUsize::new(0) }; MAX_CUSTOM_ARTICLES];
static INSTALLED: [AtomicBool; MAX_CUSTOM_ARTICLES] =
    [const { AtomicBool::new(false) }; MAX_CUSTOM_ARTICLES];
static APPLIED_LOG: AtomicU32 = AtomicU32::new(0);
static STRAY_LOG: AtomicU32 = AtomicU32::new(0);

pub(crate) fn wants_hit_module(kind: i32) -> bool {
    row(kind).is_some_and(|row| COUNTS[row].load(Ordering::Acquire) != 0)
}

fn row(kind: i32) -> Option<usize> {
    let row = usize::try_from(kind.checked_sub(FIRST_CUSTOM_WEAPON_KIND)?).ok()?;
    (row < MAX_CUSTOM_ARTICLES).then_some(row)
}

pub(crate) fn register(kind: i32, hurtboxes: &[Hurtbox]) -> Result<usize, String> {
    let row = row(kind).ok_or_else(|| format!("weapon kind {kind} is not a minted article"))?;
    for hurtbox in hurtboxes {
        hurtbox.check()?;
    }
    let words: Vec<u64> = hurtboxes
        .iter()
        .flat_map(|hurtbox| hurtbox.entry_words())
        .collect();
    let entries: &'static [u64] = Box::leak(words.into_boxed_slice());
    COUNTS[row].store(0, Ordering::Release);
    ENTRIES[row].store(entries.as_ptr() as usize, Ordering::Release);
    COUNTS[row].store(hurtboxes.len(), Ordering::Release);
    if INSTALLED[row].swap(true, Ordering::AcqRel) {
        return Ok(hurtboxes.len());
    }
    let original_out = &ORIGINAL_SETUP[row] as *const AtomicUsize as usize;
    let installed = unsafe {
        clone_vtables::set_entry(
            kind,
            SETUP_SLOT,
            Space::Weapon,
            setup_with_hurtboxes as *const () as usize,
            original_out,
        )
    };
    if installed.is_none() {
        INSTALLED[row].store(false, Ordering::Release);
        COUNTS[row].store(0, Ordering::Release);
        return Err(format!("weapon kind {kind} has no private vtable to carry hurtboxes"));
    }
    Ok(hurtboxes.len())
}

unsafe fn kind_of(class: usize, weapon: usize) -> Option<i32> {
    clone_vtables::weapon_kind_of_class(class).or_else(|| clone_vtables::minted_kind_of_object(weapon))
}

unsafe extern "C" fn setup_with_hurtboxes(class: usize, weapon: usize) -> usize {
    let Some((kind, row)) = kind_of(class, weapon).and_then(|kind| Some((kind, row(kind)?))) else {
        if STRAY_LOG.fetch_add(1, Ordering::Relaxed) < 4 {
            dbg_log_public(&format!(
                "[articlehurt] setup reached for class {class:#x} object {weapon:#x}, which is no minted article; nothing registered"
            ));
        }
        return 0;
    };
    let original = ORIGINAL_SETUP[row].load(Ordering::Acquire);
    let result = if original != 0 {
        let original: Setup = core::mem::transmute(original);
        original(class, weapon)
    } else {
        0
    };
    apply(kind, row, weapon);
    result
}

unsafe fn apply(kind: i32, row: usize, weapon: usize) {
    let count = COUNTS[row].load(Ordering::Acquire);
    let entries = ENTRIES[row].load(Ordering::Acquire);
    if count == 0 || entries == 0 || weapon == 0 {
        return;
    }
    let accessor = core::ptr::read_volatile((weapon + OBJECT_MODULE_ACCESSOR) as *const usize);
    if accessor == 0 {
        return;
    }
    let hit = core::ptr::read_volatile((accessor + ACCESSOR_HIT_MODULE) as *const usize);
    if hit == 0 {
        return;
    }
    let vtable = core::ptr::read_volatile(hit as *const usize);
    if vtable == 0 {
        return;
    }
    let set_hit_data: SetHitData = core::mem::transmute(core::ptr::read_volatile(
        (vtable + HIT_MODULE_SET_HIT_DATA) as *const usize,
    ));
    let list = HitDataList {
        entries,
        count: count as u64,
    };
    let logged = APPLIED_LOG.fetch_add(1, Ordering::Relaxed);
    let before = (logged < 4).then(|| snapshot(hit));
    set_hit_data(hit, &list, 0);
    if logged >= 16 {
        return;
    }
    let changed = before.map(|before| changed_words(&before, &snapshot(hit)));
    dbg_log_public(&format!(
        "[articlehurt] weapon kind {kind} object {weapon:#x}: {count} hurtbox(es) registered on HitModule {hit:#x} (vtable {:#x}, set_hit_data {:#x}){}",
        crate::game::offset_of(vtable),
        crate::game::offset_of(core::ptr::read_volatile(
            (vtable + HIT_MODULE_SET_HIT_DATA) as *const usize
        )),
        changed.map(|changed| format!("; module words changed: {changed}")).unwrap_or_default()
    ));
}

const SNAPSHOT_WORDS: usize = 0x40;

unsafe fn snapshot(module: usize) -> [usize; SNAPSHOT_WORDS] {
    let mut words = [0usize; SNAPSHOT_WORDS];
    for (index, word) in words.iter_mut().enumerate() {
        *word = core::ptr::read_volatile((module + index * 8) as *const usize);
    }
    words
}

fn changed_words(before: &[usize; SNAPSHOT_WORDS], after: &[usize; SNAPSHOT_WORDS]) -> String {
    let changes: Vec<String> = before
        .iter()
        .zip(after.iter())
        .enumerate()
        .filter(|(_, (old, new))| old != new)
        .take(12)
        .map(|(index, (old, new))| format!("+{:#x} {old:#x}->{new:#x}", index * 8))
        .collect();
    if changes.is_empty() {
        "none".to_string()
    } else {
        changes.join(" ")
    }
}
