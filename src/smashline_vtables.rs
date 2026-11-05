use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::clone_vtables::{Space, FIGHTER_SLOTS, HEADER};
use crate::FIRST_CUSTOM_KIND;

const NO_COSTUME: i32 = -1;
const DEFAULT_GROUP: i32 = -1;
const ENTRY_COSTUME: usize = 0x64;
const MAX_COSTUMES: usize = 256;
const LOG_BUDGET: u32 = 48;
const REPLACE_COSTUME: &[u8] = b"smashline_replace_fighter_vtable_costume\0";
const GET_ORIGINAL: &[u8] = b"smashline_get_original_fighter_vtable\0";
const REPLACE_WEAPON: &[u8] = b"smashline_replace_weapon_vtable\0";

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Costume {
    data: *const usize,
    len: usize,
}

impl Costume {
    unsafe fn list(&self) -> Vec<usize> {
        if self.data.is_null() || self.len == 0 || self.len > MAX_COSTUMES {
            return Vec::new();
        }
        let mut list = core::slice::from_raw_parts(self.data, self.len).to_vec();
        list.sort_unstable();
        list.dedup();
        list
    }
}

struct Update {
    kind: i32,
    costumes: Vec<usize>,
    slots: BTreeMap<usize, usize>,
}

struct Variant {
    kind: i32,
    group: i32,
    generation: u32,
    class: usize,
}

type ReplaceFn = unsafe extern "C" fn(i32, Costume, usize, usize);
type ReplaceWeaponFn = unsafe extern "C" fn(i32, usize, usize);

static UPDATES: Mutex<Vec<Update>> = Mutex::new(Vec::new());
static VARIANTS: Mutex<Vec<Variant>> = Mutex::new(Vec::new());
static GENERATION: AtomicU32 = AtomicU32::new(0);
static REPLACE_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static REPLACE_WEAPON_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static INSTALLED: Mutex<bool> = Mutex::new(false);
static LOG: AtomicU32 = AtomicU32::new(0);

thread_local! {
    static CONSTRUCTION_COSTUME: Cell<i32> = Cell::new(NO_COSTUME);
}

fn log(line: String) {
    if LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        crate::dbg_log_public(&line);
    }
}

fn describe(costumes: &[usize]) -> String {
    if costumes.is_empty() {
        return "the costumes no costume list claims".to_string();
    }
    let list: Vec<String> = costumes.iter().map(|costume| costume.to_string()).collect();
    format!("costume(s) {}", list.join(", "))
}

pub(crate) fn invalidate() {
    GENERATION.fetch_add(1, Ordering::AcqRel);
}

pub(crate) unsafe fn entry_costume(entry_id: i32) -> i32 {
    #[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
    if let Some(record) = crate::load_pipeline::resource_record_for_entry(entry_id) {
        return core::ptr::read_volatile((record + ENTRY_COSTUME) as *const i32);
    }
    let _ = entry_id;
    NO_COSTUME
}

pub(crate) fn with_costume<R>(costume: i32, callback: impl FnOnce() -> R) -> R {
    let previous = CONSTRUCTION_COSTUME
        .try_with(|cell| cell.replace(costume))
        .unwrap_or(NO_COSTUME);
    let result = callback();
    let _ = CONSTRUCTION_COSTUME.try_with(|cell| cell.set(previous));
    result
}

fn current_costume() -> i32 {
    CONSTRUCTION_COSTUME.try_with(|cell| cell.get()).unwrap_or(NO_COSTUME)
}

fn record(kind: i32, costumes: Vec<usize>, slot: usize, function: usize) {
    let Ok(mut updates) = UPDATES.lock() else {
        return;
    };
    let note = describe(&costumes);
    match updates
        .iter_mut()
        .find(|update| update.kind == kind && update.costumes == costumes)
    {
        Some(update) => {
            update.slots.insert(slot, function);
        }
        None => updates.push(Update {
            kind,
            costumes,
            slots: BTreeMap::from([(slot, function)]),
        }),
    }
    drop(updates);
    invalidate();
    log(format!(
        "[slvtable] clone kind {kind} slot {slot} -> {function:#x} for {note}: Smashline's replacement goes into the clone's own vtable"
    ));
}

fn selection(kind: i32, costume: i32) -> (i32, BTreeMap<usize, usize>) {
    let mut merged = BTreeMap::new();
    let Ok(updates) = UPDATES.lock() else {
        return (DEFAULT_GROUP, merged);
    };
    let wanted = usize::try_from(costume).ok();
    let claimed = wanted.is_some_and(|costume| {
        updates
            .iter()
            .any(|update| update.kind == kind && update.costumes.contains(&costume))
    });
    for update in updates.iter().filter(|update| update.kind == kind) {
        let applies = match (claimed, wanted) {
            (true, Some(costume)) => update.costumes.contains(&costume),
            _ => update.costumes.is_empty(),
        };
        if applies {
            merged.extend(update.slots.iter().map(|(slot, function)| (*slot, *function)));
        }
    }
    (if claimed { costume } else { DEFAULT_GROUP }, merged)
}

pub(crate) unsafe fn class_for(kind: i32, private: usize) -> usize {
    if private == 0 || kind < FIRST_CUSTOM_KIND {
        return private;
    }
    let costume = current_costume();
    let (group, merged) = selection(kind, costume);
    if merged.is_empty() {
        return private;
    }
    let generation = GENERATION.load(Ordering::Acquire);
    let Ok(mut variants) = VARIANTS.lock() else {
        return private;
    };
    if let Some(variant) = variants
        .iter()
        .find(|variant| variant.kind == kind && variant.group == group && variant.generation == generation)
    {
        return variant.class;
    }
    let words = HEADER + 2 * FIGHTER_SLOTS;
    let blob = Box::leak(vec![0usize; words].into_boxed_slice());
    let base = blob.as_mut_ptr();
    for index in 0..words {
        *base.add(index) = core::ptr::read_volatile((private as *const usize).add(index));
    }
    *base = base.add(HEADER) as usize;
    for (slot, function) in &merged {
        if *slot < FIGHTER_SLOTS {
            *base.add(HEADER + slot) = *function;
        }
    }
    let class = base as usize;
    variants.push(Variant {
        kind,
        group,
        generation,
        class,
    });
    let slots: Vec<String> = merged.keys().map(|slot| slot.to_string()).collect();
    log(format!(
        "[slvtable] clone kind {kind} costume {costume}: own vtable copy {class:#x} with Smashline's slot(s) {} ({})",
        slots.join(", "),
        if group == DEFAULT_GROUP {
            "the entries for unclaimed costumes"
        } else {
            "the entries listing this costume"
        }
    ));
    class
}

pub(crate) fn kind_of_variant(class: usize) -> Option<i32> {
    if class == 0 {
        return None;
    }
    VARIANTS
        .lock()
        .ok()?
        .iter()
        .find(|variant| variant.class == class)
        .map(|variant| variant.kind)
}

unsafe extern "C" fn replace_bridge(kind: i32, costume: Costume, slot: usize, function: usize) {
    if kind >= FIRST_CUSTOM_KIND {
        if slot < FIGHTER_SLOTS && function != 0 {
            record(kind, costume.list(), slot, function);
        } else {
            log(format!(
                "[slvtable] clone kind {kind}: Smashline replacement of slot {slot} with {function:#x} refused (slots 0 to {})",
                FIGHTER_SLOTS - 1
            ));
        }
        return;
    }
    let original = REPLACE_ORIGINAL.load(Ordering::Acquire);
    if original != 0 {
        core::mem::transmute::<usize, ReplaceFn>(original)(kind, costume, slot, function);
    }
}

unsafe extern "C" fn original_bridge(kind: i32, slot: usize) -> usize {
    if slot >= FIGHTER_SLOTS {
        return 0;
    }
    if kind >= FIRST_CUSTOM_KIND {
        return match crate::clone_vtables::clone_table(kind, Space::Fighter) {
            Some(table) => core::ptr::read_volatile((table as *const usize).add(slot)),
            None => 0,
        };
    }
    crate::clone_vtables::vanilla_fighter_entry(kind, slot as u32).map_or(0, |(_, entry)| entry)
}

unsafe extern "C" fn replace_weapon_bridge(kind: i32, slot: usize, function: usize) {
    if crate::custom_articles::custom_weapon_source_kind(kind).is_some() {
        let written = u32::try_from(slot)
            .ok()
            .and_then(|slot| crate::clone_vtables::set_entry(kind, slot, Space::Weapon, function, 0));
        if written.is_none() {
            log(format!(
                "[slvtable] article kind {kind}: Smashline replacement of slot {slot} with {function:#x} refused"
            ));
        }
        return;
    }
    let original = REPLACE_WEAPON_ORIGINAL.load(Ordering::Acquire);
    if original != 0 {
        core::mem::transmute::<usize, ReplaceWeaponFn>(original)(kind, slot, function);
    }
}

unsafe fn symbol(name: &[u8]) -> usize {
    let mut address = 0usize;
    if skyline::nn::ro::LookupSymbol(&mut address, name.as_ptr()) == 0 {
        address
    } else {
        0
    }
}

pub(crate) unsafe fn install() {
    let Ok(mut installed) = INSTALLED.lock() else {
        return;
    };
    if *installed {
        return;
    }
    let replace = symbol(REPLACE_COSTUME);
    if replace == 0 {
        return;
    }
    *installed = true;
    let replace_weapon = symbol(REPLACE_WEAPON);
    if replace_weapon != 0 {
        let mut trampoline: *mut libc::c_void = core::ptr::null_mut();
        skyline::hooks::A64HookFunction(
            replace_weapon as *const libc::c_void,
            replace_weapon_bridge as *const () as *const libc::c_void,
            &mut trampoline,
        );
        if trampoline.is_null() {
            crate::dbg_log_public("[slvtable] hooking smashline_replace_weapon_vtable failed; Smashline's weapon vtable replacements do not reach articles");
        } else {
            REPLACE_WEAPON_ORIGINAL.store(trampoline as usize, Ordering::Release);
        }
    }
    let mut trampoline: *mut libc::c_void = core::ptr::null_mut();
    skyline::hooks::A64HookFunction(
        replace as *const libc::c_void,
        replace_bridge as *const () as *const libc::c_void,
        &mut trampoline,
    );
    if trampoline.is_null() {
        crate::dbg_log_public("[slvtable] hooking smashline_replace_fighter_vtable_costume failed; Smashline's fighter vtable replacements do not reach clones");
        return;
    }
    REPLACE_ORIGINAL.store(trampoline as usize, Ordering::Release);
    let original = symbol(GET_ORIGINAL);
    if original != 0 {
        skyline::hooks::A64HookFunction(
            original as *const libc::c_void,
            original_bridge as *const () as *const libc::c_void,
            core::ptr::null_mut(),
        );
    }
    crate::dbg_log_public(&format!(
        "[slvtable] armed: Smashline 2.0 fighter vtable replacements for clone kinds go into the clone's own vtable, per costume, weapon ones into the article's; get_original_fighter_vtable answers {}",
        if original != 0 {
            "a clone's own entry"
        } else {
            "as Smashline does (its export was not found)"
        }
    ));
}
