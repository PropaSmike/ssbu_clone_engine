use super::*;

use clone_engine_api::{
    ItemCategory, ERROR_BACKEND_UNAVAILABLE, ERROR_CUSTOM_KIND, ERROR_UNSUPPORTED, RESULT_OK,
};
use clone_engine_core::item_generate::{self as generate, Entry, Family, Reject, Rule};
use core::sync::atomic::{AtomicU32, Ordering};
use std::sync::RwLock;

const OFF_GENERATOR_SETUP: usize = 0x15d7530;
const OFF_ITEM_LOT: usize = 0x15bae00;
const OFF_GAME_ALLOCATE: usize = 0x392dce0;
const OFF_GAME_FREE: usize = 0x392e590;
const KIND_MASK_GLOBAL: usize = 0x52c34a0;
const ALLOCATION_ALIGN: u32 = 0x10;
const HEAP_FLOOR: usize = 0x10_0000_0000;
const LOG_LIMIT: u32 = 64;
const LOT_LOG_LIMIT: u32 = 16;
const VANILLA_LOT_LOG_LIMIT: u32 = 4;
const LOT_TALLY_EVERY: u32 = 64;
const MAX_RULES: usize = 64;

static RULES: RwLock<Vec<Rule>> = RwLock::new(Vec::new());
static LOG: AtomicU32 = AtomicU32::new(0);
static LOTTED: AtomicU32 = AtomicU32::new(0);
static DRAWS: AtomicU32 = AtomicU32::new(0);
static REBUILDS: AtomicU32 = AtomicU32::new(0);

#[skyline::from_offset(OFF_GAME_ALLOCATE)]
fn game_allocate(align: u32, size: usize) -> *mut u8;

#[skyline::from_offset(OFF_GAME_FREE)]
fn game_free(block: *mut u8);

fn log(message: &str) {
    if LOG.fetch_add(1, Ordering::Relaxed) < LOG_LIMIT {
        dbg_log_public(&format!("[itemgen] {message}"));
    }
}

fn kind_name(kind: i32) -> Option<String> {
    unsafe { item_clones::base_resource_name(kind) }.map(|name| name.to_string_lossy().into_owned())
}

pub(crate) fn register(
    public_kind: i32,
    generator_hash: u64,
    per: i32,
    min: i32,
    max: i32,
    variation: i32,
) -> i32 {
    let Some(base_kind) = item_clones::clone_base_kind(public_kind) else {
        return ERROR_CUSTOM_KIND;
    };
    let Some(generator) = generate::generator_for_hash(generator_hash, kind_name) else {
        log(&format!(
            "generate_add refused public={public_kind:#x} generator={generator_hash:#x}: \
             not item_genid_random or an item_kind_<name> of the 432 vanilla kinds"
        ));
        return ERROR_UNSUPPORTED;
    };
    let family = generate::family_for_generator(generator);
    let category = item_clones::clone_category(public_kind);
    let welcome = match family {
        None => true,
        Some(Family::Assist) => category == Some(ItemCategory::Assist),
        Some(Family::Pokemon) => category == Some(ItemCategory::Pokemon),
    };
    if !welcome {
        log(&format!(
            "generate_add refused public={public_kind:#x} generator={generator:#x}: {} is what a ball summons, \
             only a registered {:?} family member may sit there (this item is {category:?})",
            generate::generator_label(generator, kind_name),
            family.unwrap_or(Family::Assist)
        ));
        return ERROR_UNSUPPORTED;
    }
    let entry = match generate::validate(public_kind, per, min, max, variation) {
        Ok(entry) => entry,
        Err(reject) => {
            log(&format!(
                "generate_add refused public={public_kind:#x} generator={generator:#x}: {}",
                match reject {
                    Reject::Per => format!("per must be 0..={}", generate::MAX_PER),
                    Reject::Count => format!("need 0 <= min <= max <= {}", generate::MAX_COUNT),
                    Reject::Variation => String::from("variation must be -1 (auto) or a variation index"),
                }
            ));
            return ERROR_UNSUPPORTED;
        }
    };
    let Ok(mut rules) = RULES.write() else {
        return ERROR_BACKEND_UNAVAILABLE;
    };
    let label = generate::generator_label(generator, kind_name);
    let existing = rules
        .iter()
        .position(|rule| rule.public_kind == public_kind && rule.generator == generator);
    match existing {
        Some(index) => rules[index].entry = entry,
        None if rules.len() >= MAX_RULES => return ERROR_BACKEND_UNAVAILABLE,
        None => rules.push(Rule { public_kind, base_kind, generator, entry }),
    }
    log(&format!(
        "generate_add public={public_kind:#x} (base {base_kind:#x}) in {label}: per {per} count {min}..{max} \
         variation {variation}; follows the base's item switch"
    ));
    RESULT_OK
}

pub(crate) fn rule_count() -> usize {
    RULES.read().map(|rules| rules.len()).unwrap_or(0)
}

unsafe fn accessor() -> Option<usize> {
    let handle = core::ptr::read_volatile(
        (crate::text_base() + item_common_tables::COMMON_HANDLE_GLOBAL) as *const usize,
    );
    if handle < HEAP_FLOOR {
        return None;
    }
    let accessor = core::ptr::read_volatile(handle as *const usize);
    (accessor >= HEAP_FLOOR).then_some(accessor)
}

unsafe fn kind_mask() -> Option<[u32; generate::KIND_MASK_WORDS]> {
    let mask = core::ptr::read_volatile((crate::text_base() + KIND_MASK_GLOBAL) as *const usize);
    if mask < HEAP_FLOOR {
        return None;
    }
    let mut words = [0u32; generate::KIND_MASK_WORDS];
    for (index, word) in words.iter_mut().enumerate() {
        *word = core::ptr::read_volatile((mask + index * 4) as *const u32);
    }
    Some(words)
}

unsafe fn find_record(accessor: usize, generator: u32) -> Option<(usize, &'static str)> {
    for (table, name) in generate::TABLES_IN_LOOKUP_ORDER.iter().zip(generate::TABLE_NAMES) {
        let begin = core::ptr::read_volatile((accessor + table + generate::TABLE_VECTOR_BEGIN) as *const usize);
        let end = core::ptr::read_volatile((accessor + table + generate::TABLE_VECTOR_END) as *const usize);
        if begin < HEAP_FLOOR || end < begin || (end - begin) % generate::RECORD_SIZE != 0 {
            continue;
        }
        let mut record = begin;
        while record < end {
            if core::ptr::read_volatile((record + generate::RECORD_GENERATOR) as *const u32) == generator {
                return Some((record, name));
            }
            record += generate::RECORD_SIZE;
        }
    }
    None
}

unsafe fn live_entries(record: usize) -> Option<(Vec<Entry>, usize)> {
    let begin = core::ptr::read_volatile((record + generate::RECORD_ENTRIES_BEGIN) as *const usize);
    let end = core::ptr::read_volatile((record + generate::RECORD_ENTRIES_END) as *const usize);
    if begin != 0 && begin < HEAP_FLOOR {
        return None;
    }
    if end < begin || (end - begin) % generate::ENTRY_SIZE != 0 {
        return None;
    }
    let count = (end - begin) / generate::ENTRY_SIZE;
    let mut entries = Vec::with_capacity(count);
    let mut bytes = [0u8; generate::ENTRY_SIZE];
    for index in 0..count {
        core::ptr::copy_nonoverlapping(
            (begin + index * generate::ENTRY_SIZE) as *const u8,
            bytes.as_mut_ptr(),
            generate::ENTRY_SIZE,
        );
        entries.push(Entry::from_bytes(&bytes));
    }
    Some((entries, begin))
}

unsafe fn replace_entries(record: usize, old_begin: usize, entries: &[Entry]) -> bool {
    let size = entries.len() * generate::ENTRY_SIZE;
    let block = game_allocate(ALLOCATION_ALIGN, size.max(generate::ENTRY_SIZE));
    if block.is_null() {
        return false;
    }
    let mut bytes = [0u8; generate::ENTRY_SIZE];
    for (index, entry) in entries.iter().enumerate() {
        entry.write(&mut bytes);
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), block.add(index * generate::ENTRY_SIZE), generate::ENTRY_SIZE);
    }
    let begin = block as usize;
    core::ptr::write_volatile((record + generate::RECORD_ENTRIES_BEGIN) as *mut usize, begin);
    core::ptr::write_volatile((record + generate::RECORD_ENTRIES_END) as *mut usize, begin + size);
    core::ptr::write_volatile((record + generate::RECORD_ENTRIES_CAPACITY) as *mut usize, begin + size);
    if old_begin >= HEAP_FLOOR {
        game_free(old_begin as *mut u8);
    }
    true
}

unsafe fn rebuild(why: &str) {
    let Ok(rules) = RULES.read() else {
        return;
    };
    if rules.is_empty() {
        return;
    }
    let Some(accessor) = accessor() else {
        log(&format!("{why}: no item param accessor, generation tables keep vanilla"));
        return;
    };
    let Some(mask) = kind_mask() else {
        log(&format!("{why}: no item switch mask, generation tables keep vanilla"));
        return;
    };
    let mut generators: Vec<u32> = rules.iter().map(|rule| rule.generator).collect();
    generators.sort_unstable();
    generators.dedup();
    let rebuild = REBUILDS.fetch_add(1, Ordering::Relaxed) + 1;
    for generator in generators {
        let label = generate::generator_label(generator, kind_name);
        let Some((record, table)) = find_record(accessor, generator) else {
            log(&format!("{why} #{rebuild}: no {label} record in any generation table, its rules are idle"));
            continue;
        };
        let Some((live, old_begin)) = live_entries(record) else {
            log(&format!("{why} #{rebuild}: {label} record {record:#x} has an implausible entry vector, left alone"));
            continue;
        };
        let planned = generate::plan(generator, &live, &rules, &mask, |kind| {
            item_clones::clone_base_kind(kind).is_some()
        });
        if planned.entries == live {
            continue;
        }
        if !replace_entries(record, old_begin, &planned.entries) {
            log(&format!("{why} #{rebuild}: allocation failed for {label}, left alone"));
            continue;
        }
        log(&format!(
            "{why} #{rebuild}: {label} ({table} table, record {record:#x}): {} vanilla entr{}, {} clone entr{} appended, {} held back by the item switch",
            planned.vanilla,
            if planned.vanilla == 1 { "y" } else { "ies" },
            planned.appended,
            if planned.appended == 1 { "y" } else { "ies" },
            planned.held_back
        ));
    }
}

#[skyline::hook(offset = OFF_GENERATOR_SETUP)]
unsafe fn generator_setup_bridge(manager: u64) -> u64 {
    rebuild("match setup");
    call_original!(manager)
}

#[skyline::hook(offset = OFF_ITEM_LOT)]
unsafe fn item_lot_bridge(manager: u64, generator: u32, mask: u64, flag: u32) -> u64 {
    let packed = call_original!(manager, generator, mask, flag);
    let (kind, variation) = generate::unpack_lot(packed);
    let draw = DRAWS.fetch_add(1, Ordering::Relaxed) + 1;
    if draw % LOT_TALLY_EVERY == 0 {
        dbg_log_public(&format!(
            "[itemgen] lot tally: {draw} draws so far, {} of them clones",
            LOTTED.load(Ordering::Relaxed)
        ));
    }
    let Some(base_kind) = item_clones::clone_base_kind(kind) else {
        if draw <= VANILLA_LOT_LOG_LIMIT {
            dbg_log_public(&format!(
                "[itemgen] lot #{draw} from {}: vanilla {kind:#x} ({}) variation {variation}",
                generate::generator_label(generator, kind_name),
                kind_name(kind).unwrap_or_else(|| "?".to_string())
            ));
        }
        return packed;
    };
    let ticketed = item_clones::queue_lot_ticket(kind);
    if LOTTED.fetch_add(1, Ordering::Relaxed) < LOT_LOG_LIMIT {
        dbg_log_public(&format!(
            "[itemgen] lot from {}: clone {kind:#x} drawn, handing the game base {base_kind:#x} variation {variation} (ticket {})",
            generate::generator_label(generator, kind_name),
            if ticketed { "queued" } else { "REFUSED, this one spawns as the base" }
        ));
    }
    generate::pack_lot(base_kind, variation)
}

pub(crate) fn install() {
    skyline::install_hooks!(generator_setup_bridge, item_lot_bridge);
    log("generation tables: rules apply at match setup, drawn clones ride a spawn ticket");
}

#[no_mangle]
pub extern "C" fn clone_engine_item_generate_add_v1(
    item_kind: i32,
    generator: u64,
    per: i32,
    min: i32,
    max: i32,
    variation: i32,
) -> i32 {
    register(item_kind, generator, per, min, max, variation)
}

#[no_mangle]
pub extern "C" fn clone_engine_item_generate_count_v1() -> i32 {
    rule_count() as i32
}
