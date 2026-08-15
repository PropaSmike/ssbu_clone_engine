use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};
use std::ffi::{CStr, CString};
use std::sync::{OnceLock, RwLock};

use clone_engine_api::{
    CloneItemUiRegistrationV1, ItemCategory, ItemSpawnSource, API_VERSION_V1, ERROR_DUPLICATE,
    ERROR_ITEM_CATEGORY, ERROR_ITEM_UI_CAPACITY, ERROR_ITEM_UI_METADATA, ERROR_ITEM_UI_UNAVAILABLE,
    ERROR_NULL, ERROR_REGISTRATION_CLOSED, ERROR_STRUCT_SIZE, ERROR_UNSUPPORTED, ERROR_VERSION,
    ITEM_UI_FLAG_MASTERBALL, ITEM_UI_FLAG_POKEBALL, ITEM_UI_FLAG_RULES, ITEM_UI_FLAG_TRAINING,
    RESULT_OK,
};

const OFF_UI_DB_HASH_LOOKUP: usize = 0x3269FD0;
const OFF_UI_DB_NAME_ID: usize = 0x327F680;
const OFF_TRAINING_ORDINARY_LIST_DONE: usize = 0x1BB8308;
const OFF_TRAINING_POKEMON_LIST_DONE: usize = 0x1BB8374;
const OFF_TRAINING_ASSIST_LIST_DONE: usize = 0x1BB83E0;
const OFF_TRAINING_MASTER_LIST_DONE: usize = 0x1BB844C;
const OFF_TRAINING_SELECTION_TOKEN_LOAD: usize = 0x1559B9C;
const OFF_TRAINING_QUEUE_REQUEST: usize = 0x1559EE8;
const OFF_TRAINING_CONFIRM_CALLBACK: usize = 0x13E7610;
const OFF_TRAINING_INPUT_TICK: usize = 0x1BC0F54;
const OFF_TRAINING_CURSOR_COMMIT: usize = 0x1BC1BC8;
const OFF_TRAINING_MODE_RENDERER: usize = 0x1BB9290;

const TRAINING_ORDINARY_VECTOR: usize = 0xA58;
const TRAINING_POKEMON_VECTOR: usize = 0xA70;
const TRAINING_ASSIST_VECTOR: usize = 0xA88;
const TRAINING_MASTER_VECTOR: usize = 0xAA0;
const UI_HASH_MASK: u64 = 0xFF_FFFF_FFFF;
const MAX_UI_ENTRIES: usize = 64;
const MAX_VECTOR_ENTRIES: usize = 4096;
const TRAINING_PANE_COUNT: usize = 88;
const ORDINARY_PAGE_PAYLOAD: usize = TRAINING_PANE_COUNT - 2;
const PAGE_UI_ID: &str = "ui_item_clone_engine_next";
const PAGE_NAME_ID: &[u8] = b"clone_engine_next";
const PAGE_BASE_UI_HASH: u64 = 0x12_8A24_17F9;
const VALID_FLAGS: u32 =
    ITEM_UI_FLAG_TRAINING | ITEM_UI_FLAG_RULES | ITEM_UI_FLAG_POKEBALL | ITEM_UI_FLAG_MASTERBALL;

const PREFLIGHT: &[(usize, &[u32], &str)] = &[
    (
        OFF_UI_DB_HASH_LOOKUP,
        &[0xF9403009, 0xB4000169, 0xD100052A, 0x92409C28],
        "ui-db-hash-lookup",
    ),
    (OFF_UI_DB_NAME_ID, &[0xD10183FF], "ui-db-name-id"),
    (
        OFF_TRAINING_ORDINARY_LIST_DONE,
        &[0xF9408BE0],
        "training-ordinary-list",
    ),
    (
        OFF_TRAINING_POKEMON_LIST_DONE,
        &[0xF94073E0],
        "training-pokemon-list",
    ),
    (
        OFF_TRAINING_ASSIST_LIST_DONE,
        &[0xF9405BE0],
        "training-assist-list",
    ),
    (
        OFF_TRAINING_MASTER_LIST_DONE,
        &[0xF94043E0],
        "training-master-list",
    ),
    (
        OFF_TRAINING_SELECTION_TOKEN_LOAD,
        &[0xF9410D14],
        "training-selected-token",
    ),
    (
        OFF_TRAINING_QUEUE_REQUEST,
        &[0x9402C732],
        "training-queue-request",
    ),
    (
        OFF_TRAINING_CONFIRM_CALLBACK,
        &[0x3940004A, 0xF9400408, 0x7100015F, 0xF9400029],
        "training-confirm-callback",
    ),
    (
        OFF_TRAINING_INPUT_TICK,
        &[0xF9404013],
        "training-input-tick",
    ),
    (
        OFF_TRAINING_CURSOR_COMMIT,
        &[0xB90BD674],
        "training-cursor-commit",
    ),
    (
        OFF_TRAINING_MODE_RENDERER,
        &[0xD106C3FF],
        "training-mode-renderer",
    ),
];

#[derive(Debug)]
struct UiEntry {
    public_kind: i32,
    base_kind: i32,
    category: ItemCategory,
    ui_id: CString,
    ui_hash: u64,
    base_ui_hash: u64,
    flags: u32,
    training_order: i32,
    rules_order: i32,
    base_row: i32,
}

fn entries() -> &'static RwLock<Vec<UiEntry>> {
    static ENTRIES: OnceLock<RwLock<Vec<UiEntry>>> = OnceLock::new();
    ENTRIES.get_or_init(|| RwLock::new(Vec::new()))
}

#[derive(Default)]
struct OrdinaryPager {
    object: usize,
    begin: usize,
    capacity: usize,
    sentinel: u64,
    page_token: u64,
    page_base_row: i32,
    payload: Vec<u64>,
    page: usize,
}

fn ordinary_pager() -> &'static RwLock<OrdinaryPager> {
    static PAGER: OnceLock<RwLock<OrdinaryPager>> = OnceLock::new();
    PAGER.get_or_init(|| RwLock::new(OrdinaryPager::default()))
}

fn page_ui_hash() -> u64 {
    crate::hash40::hash40(PAGE_UI_ID) & UI_HASH_MASK
}

static REGISTRATION_CLOSED: AtomicU32 = AtomicU32::new(0);
static LOG_SEQUENCE: AtomicU32 = AtomicU32::new(0);
static RESET_CURSOR_AFTER_PAGE: AtomicBool = AtomicBool::new(false);
static PAGE_CYCLE_PENDING: AtomicBool = AtomicBool::new(false);

fn log(message: String) {
    if LOG_SEQUENCE.fetch_add(1, Ordering::Relaxed) < 128 {
        crate::dbg_log_public(&message);
    }
}

fn valid_ui_id(value: &str) -> bool {
    value.starts_with("ui_item_")
        && value.len() > "ui_item_".len()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn entry_for_hash(hash: u64) -> Option<(i32, i32, ItemCategory, u32)> {
    let hash = hash & UI_HASH_MASK;
    entries().read().ok()?.iter().find_map(|entry| {
        (entry.ui_hash == hash).then_some((
            entry.public_kind,
            entry.base_kind,
            entry.category,
            entry.flags,
        ))
    })
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_register_item_ui_v1(
    registration: *const CloneItemUiRegistrationV1,
) -> i32 {
    if registration.is_null() {
        return ERROR_NULL;
    }
    if REGISTRATION_CLOSED.load(Ordering::Acquire) != 0 {
        return ERROR_REGISTRATION_CLOSED;
    }
    let registration = &*registration;
    if registration.api_version != API_VERSION_V1 {
        return ERROR_VERSION;
    }
    if registration.struct_size < core::mem::size_of::<CloneItemUiRegistrationV1>() as u32 {
        return ERROR_STRUCT_SIZE;
    }
    if registration.flags == 0
        || registration.flags & !VALID_FLAGS != 0
        || registration.reserved.iter().any(|value| *value != 0)
    {
        return ERROR_UNSUPPORTED;
    }
    if registration.flags & ITEM_UI_FLAG_RULES != 0 {
        return ERROR_ITEM_UI_UNAVAILABLE;
    }
    if registration.ui_id.is_null() {
        return ERROR_NULL;
    }
    let Ok(ui_id) = CStr::from_ptr(registration.ui_id).to_str() else {
        return ERROR_ITEM_UI_METADATA;
    };
    if !valid_ui_id(ui_id) {
        return ERROR_ITEM_UI_METADATA;
    }
    let Some((base_kind, category, base_ui_hash)) =
        crate::item_clones::item_ui_base(registration.item_kind)
    else {
        return ERROR_ITEM_UI_METADATA;
    };
    match category {
        ItemCategory::Item => {
            if registration.flags & (ITEM_UI_FLAG_POKEBALL | ITEM_UI_FLAG_MASTERBALL) != 0 {
                return ERROR_ITEM_CATEGORY;
            }
        }
        ItemCategory::Assist => {
            if registration.flags & (ITEM_UI_FLAG_POKEBALL | ITEM_UI_FLAG_MASTERBALL) != 0 {
                return ERROR_ITEM_CATEGORY;
            }
        }
        ItemCategory::Pokemon => {
            if registration.flags & (ITEM_UI_FLAG_POKEBALL | ITEM_UI_FLAG_MASTERBALL) == 0 {
                return ERROR_ITEM_CATEGORY;
            }
        }
        ItemCategory::Boss | ItemCategory::Unknown => return ERROR_ITEM_CATEGORY,
    }
    let ui_hash = crate::hash40::hash40(ui_id) & UI_HASH_MASK;
    let Ok(mut known) = entries().write() else {
        return ERROR_ITEM_UI_CAPACITY;
    };
    if let Some(existing) = known
        .iter()
        .find(|entry| entry.public_kind == registration.item_kind || entry.ui_hash == ui_hash)
    {
        return if existing.public_kind == registration.item_kind
            && existing.ui_hash == ui_hash
            && existing.flags == registration.flags
        {
            RESULT_OK
        } else {
            ERROR_DUPLICATE
        };
    }
    if known.len() >= MAX_UI_ENTRIES {
        return ERROR_ITEM_UI_CAPACITY;
    }
    known.push(UiEntry {
        public_kind: registration.item_kind,
        base_kind,
        category,
        ui_id: match CString::new(ui_id) {
            Ok(value) => value,
            Err(_) => return ERROR_ITEM_UI_METADATA,
        },
        ui_hash,
        base_ui_hash: base_ui_hash & UI_HASH_MASK,
        flags: registration.flags,
        training_order: registration.training_order,
        rules_order: registration.rules_order,
        base_row: -1,
    });
    log(format!(
        "[itemui] registered public={:#x} base={base_kind:#x} category={category:?} ui={ui_id} flags={:#x}",
        registration.item_kind, registration.flags
    ));
    RESULT_OK
}

unsafe fn vector_bounds(object: usize, offset: usize) -> Option<(*mut u64, *mut u64, *mut u64)> {
    if object == 0 || object & 7 != 0 {
        return None;
    }
    let vector = (object + offset) as *mut usize;
    let begin = vector.read() as *mut u64;
    let end = vector.add(1).read() as *mut u64;
    let capacity = vector.add(2).read() as *mut u64;
    if begin.is_null()
        || end < begin
        || capacity < end
        || end.offset_from(begin) as usize > MAX_VECTOR_ENTRIES
    {
        return None;
    }
    Some((begin, end, capacity))
}

fn belongs_in_mode(entry: &UiEntry, category: ItemCategory, pool_flag: u32) -> bool {
    if entry.flags & ITEM_UI_FLAG_TRAINING == 0 || entry.category != category {
        return false;
    }
    category != ItemCategory::Pokemon || entry.flags & pool_flag != 0
}

unsafe fn append_training_entries(
    object: usize,
    offset: usize,
    category: ItemCategory,
    pool_flag: u32,
    label: &str,
) {
    REGISTRATION_CLOSED.store(1, Ordering::Release);
    let Some((begin, mut end, capacity)) = vector_bounds(object, offset) else {
        log(format!("[itemui] {label}: invalid native vector"));
        return;
    };
    let Ok(mut known) = entries().write() else {
        return;
    };
    let native_tokens = core::slice::from_raw_parts(begin, end.offset_from(begin) as usize);
    let mut appended = 0usize;
    for entry in known.iter_mut() {
        if !belongs_in_mode(entry, category, pool_flag) {
            continue;
        }
        if native_tokens
            .iter()
            .any(|token| token & UI_HASH_MASK == entry.ui_hash)
        {
            continue;
        }
        let Some(base_token) = native_tokens
            .iter()
            .copied()
            .find(|token| token & UI_HASH_MASK == entry.base_ui_hash)
        else {
            log(format!(
                "[itemui] {label}: base token missing for public={:#x} base={:#x}",
                entry.public_kind, entry.base_kind
            ));
            continue;
        };
        if end.offset_from(begin) as usize >= TRAINING_PANE_COUNT {
            log(format!(
                "[itemui] {label}: all {TRAINING_PANE_COUNT} stock panes are occupied; public={:#x} needs paging",
                entry.public_kind
            ));
            break;
        }
        if end >= capacity {
            log(format!(
                "[itemui] {label}: native spare capacity exhausted; public={:#x} not appended",
                entry.public_kind
            ));
            break;
        }
        let token = (base_token & !UI_HASH_MASK) | entry.ui_hash;
        end.write(token);
        end = end.add(1);
        ((object + offset + 8) as *mut usize).write(end as usize);
        entry.base_row = ((base_token >> 40) & 0xFFFF) as i32;
        appended += 1;
        log(format!(
            "[itemui] {label}: appended public={:#x} token={token:#x} base_row={}",
            entry.public_kind, entry.base_row
        ));
    }
    if appended != 0 {
        log(format!("[itemui] {label}: appended={appended}"));
    }
}

unsafe fn write_ordinary_page(pager: &mut OrdinaryPager) -> bool {
    if pager.begin == 0 || pager.capacity == 0 || pager.payload.is_empty() {
        return false;
    }
    let pages = pager.payload.len().div_ceil(ORDINARY_PAGE_PAYLOAD);
    if pages <= 1 || pager.page >= pages {
        return false;
    }
    let begin = pager.begin as *mut u64;
    let capacity = pager.capacity as *mut u64;
    let start = pager.page * ORDINARY_PAGE_PAYLOAD;
    let stop = (start + ORDINARY_PAGE_PAYLOAD).min(pager.payload.len());
    if begin.add(TRAINING_PANE_COUNT) > capacity {
        return false;
    }
    begin.write(pager.sentinel);
    let mut output = 1usize;
    for token in pager.payload[start..stop].iter().copied() {
        begin.add(output).write(token);
        output += 1;
    }
    begin.add(output).write(pager.page_token);
    output += 1;
    ((pager.object + TRAINING_ORDINARY_VECTOR + 8) as *mut usize).write(begin.add(output) as usize);
    true
}

unsafe fn configure_ordinary_pager(object: usize) {
    REGISTRATION_CLOSED.store(1, Ordering::Release);
    let Some((begin, end, capacity)) = vector_bounds(object, TRAINING_ORDINARY_VECTOR) else {
        log("[itemui] training-ordinary: invalid native vector".into());
        return;
    };
    let native_count = end.offset_from(begin) as usize;
    if native_count == 0 || begin.read() & UI_HASH_MASK != 0 {
        log(format!(
            "[itemui] training-ordinary: expected leading sentinel, count={native_count}"
        ));
        return;
    }

    let native_tokens = core::slice::from_raw_parts(begin, native_count).to_vec();
    let mut custom_tokens = Vec::new();
    let Ok(mut known) = entries().write() else {
        return;
    };
    for entry in known.iter_mut() {
        if !belongs_in_mode(entry, ItemCategory::Item, 0) {
            continue;
        }
        let Some(base_token) = native_tokens
            .iter()
            .copied()
            .find(|token| token & UI_HASH_MASK == entry.base_ui_hash)
        else {
            log(format!(
                "[itemui] training-ordinary: base token missing for public={:#x} base={:#x}",
                entry.public_kind, entry.base_kind
            ));
            continue;
        };
        entry.base_row = ((base_token >> 40) & 0xFFFF) as i32;
        custom_tokens.push((base_token & !UI_HASH_MASK) | entry.ui_hash);
    }
    drop(known);

    if custom_tokens.is_empty() {
        if let Ok(mut pager) = ordinary_pager().write() {
            *pager = OrdinaryPager::default();
        }
        return;
    }
    if native_count < 2 {
        log("[itemui] training-ordinary: no native row available for page control".into());
        return;
    }

    let Some(page_base) = native_tokens
        .iter()
        .copied()
        .find(|token| token & UI_HASH_MASK == PAGE_BASE_UI_HASH)
    else {
        log("[itemui] training-ordinary: Green Shell page donor missing".into());
        return;
    };
    let mut payload = native_tokens[1..].to_vec();
    payload.extend(custom_tokens);
    let page_token = (page_base & !UI_HASH_MASK) | page_ui_hash();
    let mut pager = match ordinary_pager().write() {
        Ok(value) => value,
        Err(_) => return,
    };
    *pager = OrdinaryPager {
        object,
        begin: begin as usize,
        capacity: capacity as usize,
        sentinel: native_tokens[0],
        page_token,
        page_base_row: ((page_base >> 40) & 0xFFFF) as i32,
        payload,
        page: 0,
    };
    let pages = pager.payload.len().div_ceil(ORDINARY_PAGE_PAYLOAD);
    if write_ordinary_page(&mut pager) {
        log(format!(
            "[itemui] training-ordinary: paging {} entries over {pages} page(s), showing 1",
            pager.payload.len()
        ));
    } else {
        log("[itemui] training-ordinary: refused unsafe page write".into());
    }
}

unsafe fn cycle_ordinary_page() -> bool {
    let (object, page, pages) = {
        let mut pager = match ordinary_pager().write() {
            Ok(value) => value,
            Err(_) => return false,
        };
        let pages = pager.payload.len().div_ceil(ORDINARY_PAGE_PAYLOAD);
        if pages <= 1 || pager.object == 0 {
            return false;
        }
        pager.page = (pager.page + 1) % pages;
        if !write_ordinary_page(&mut pager) {
            return false;
        }
        (pager.object, pager.page, pages)
    };

    let renderer: unsafe extern "C" fn(usize, i32) =
        core::mem::transmute(crate::text_base() + OFF_TRAINING_MODE_RENDERER);
    renderer(object, 0);
    RESET_CURSOR_AFTER_PAGE.store(true, Ordering::Release);
    log(format!(
        "[itemui] training-ordinary: page {}/{} shown",
        page + 1,
        pages
    ));
    true
}

unsafe fn write_ui_string(output: *mut u8, value: &[u8]) {
    if output.is_null() || value.len() > 63 {
        return;
    }
    let mut hash = 0x811C_9DC5u32;
    for byte in value.iter().copied() {
        hash = hash.wrapping_mul(0x89) ^ byte as u32;
    }
    (output as *mut u32).write(hash);
    (output.add(4) as *mut u32).write(value.len() as u32);
    core::ptr::copy_nonoverlapping(value.as_ptr(), output.add(8), value.len());
    output.add(8 + value.len()).write(0);
}

#[skyline::hook(offset = OFF_UI_DB_HASH_LOOKUP)]
unsafe fn ui_db_hash_lookup(document: *mut u8, hash: u64) -> i32 {
    if hash & UI_HASH_MASK == page_ui_hash() {
        if let Ok(pager) = ordinary_pager().read() {
            if pager.page_base_row >= 0 {
                return pager.page_base_row;
            }
        }
    }
    if let Ok(known) = entries().read() {
        if let Some(entry) = known
            .iter()
            .find(|entry| entry.ui_hash == hash & UI_HASH_MASK && entry.base_row >= 0)
        {
            return entry.base_row;
        }
    }
    call_original!(document, hash)
}

#[skyline::hook(offset = OFF_UI_DB_NAME_ID)]
unsafe fn ui_db_name_id(output: *mut u8, document: *mut u8, token: u64) {
    call_original!(output, document, token);
    let hash = token & UI_HASH_MASK;
    if hash == page_ui_hash() {
        write_ui_string(output, PAGE_NAME_ID);
        return;
    }
    let Ok(known) = entries().read() else {
        return;
    };
    let Some(entry) = known.iter().find(|entry| entry.ui_hash == hash) else {
        return;
    };
    let bytes = entry.ui_id.as_bytes();
    let prefix = b"ui_item_";
    if let Some(name_id) = bytes.strip_prefix(prefix) {
        write_ui_string(output, name_id);
    }
}

#[skyline::hook(offset = OFF_TRAINING_ORDINARY_LIST_DONE, inline)]
unsafe fn training_ordinary_list_done(ctx: &mut skyline::hooks::InlineCtx) {
    configure_ordinary_pager(ctx.registers[19].x() as usize);
}

#[skyline::hook(offset = OFF_TRAINING_POKEMON_LIST_DONE, inline)]
unsafe fn training_pokemon_list_done(ctx: &mut skyline::hooks::InlineCtx) {
    append_training_entries(
        ctx.registers[19].x() as usize,
        TRAINING_POKEMON_VECTOR,
        ItemCategory::Pokemon,
        ITEM_UI_FLAG_POKEBALL,
        "training-pokemon",
    );
}

#[skyline::hook(offset = OFF_TRAINING_ASSIST_LIST_DONE, inline)]
unsafe fn training_assist_list_done(ctx: &mut skyline::hooks::InlineCtx) {
    append_training_entries(
        ctx.registers[19].x() as usize,
        TRAINING_ASSIST_VECTOR,
        ItemCategory::Assist,
        0,
        "training-assist",
    );
}

#[skyline::hook(offset = OFF_TRAINING_MASTER_LIST_DONE, inline)]
unsafe fn training_master_list_done(ctx: &mut skyline::hooks::InlineCtx) {
    append_training_entries(
        ctx.registers[19].x() as usize,
        TRAINING_MASTER_VECTOR,
        ItemCategory::Pokemon,
        ITEM_UI_FLAG_MASTERBALL,
        "training-master",
    );
}

struct SelectionSlot {
    owner_thread: AtomicUsize,
    public_kind: AtomicI32,
    base_kind: AtomicI32,
    spawn_source: AtomicU32,
}

impl SelectionSlot {
    const fn new() -> Self {
        Self {
            owner_thread: AtomicUsize::new(0),
            public_kind: AtomicI32::new(-1),
            base_kind: AtomicI32::new(-1),
            spawn_source: AtomicU32::new(ItemSpawnSource::Unknown as u32),
        }
    }
}

static SELECTIONS: [SelectionSlot; 8] = [const { SelectionSlot::new() }; 8];

unsafe fn current_thread() -> usize {
    skyline::nn::os::GetCurrentThread() as usize
}

fn clear_selection(thread: usize) {
    for slot in &SELECTIONS {
        if slot.owner_thread.load(Ordering::Acquire) != thread {
            continue;
        }
        slot.public_kind.store(-1, Ordering::Relaxed);
        slot.base_kind.store(-1, Ordering::Relaxed);
        slot.spawn_source
            .store(ItemSpawnSource::Unknown as u32, Ordering::Relaxed);
        slot.owner_thread.store(0, Ordering::Release);
    }
}

fn store_selection(thread: usize, public_kind: i32, base_kind: i32, source: ItemSpawnSource) {
    clear_selection(thread);
    for slot in &SELECTIONS {
        if slot
            .owner_thread
            .compare_exchange(0, thread, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            continue;
        }
        slot.public_kind.store(public_kind, Ordering::Relaxed);
        slot.base_kind.store(base_kind, Ordering::Relaxed);
        slot.spawn_source.store(source as u32, Ordering::Release);
        return;
    }
}

fn take_selection(thread: usize) -> Option<(i32, i32, ItemSpawnSource)> {
    for slot in &SELECTIONS {
        if slot.owner_thread.load(Ordering::Acquire) != thread {
            continue;
        }
        let public_kind = slot.public_kind.load(Ordering::Relaxed);
        let base_kind = slot.base_kind.load(Ordering::Relaxed);
        let source = match slot.spawn_source.load(Ordering::Acquire) {
            1 => ItemSpawnSource::Direct,
            2 => ItemSpawnSource::Assist,
            3 => ItemSpawnSource::PokeBall,
            4 => ItemSpawnSource::MasterBall,
            _ => ItemSpawnSource::Unknown,
        };
        clear_selection(thread);
        return Some((public_kind, base_kind, source));
    }
    None
}

#[skyline::hook(offset = OFF_TRAINING_SELECTION_TOKEN_LOAD, inline)]
unsafe fn training_selected_token(ctx: &mut skyline::hooks::InlineCtx) {
    let state = ctx.registers[8].x() as usize;
    if state == 0 || state & 7 != 0 {
        return;
    }
    let token = ((state + 0x218) as *const u64).read();
    let thread = current_thread();
    let Some((public_kind, base_kind, category, _flags)) = entry_for_hash(token) else {
        return;
    };
    let source = match category {
        ItemCategory::Item => ItemSpawnSource::Direct,
        ItemCategory::Assist => ItemSpawnSource::Assist,
        ItemCategory::Pokemon => {
            if ((state + 0x220) as *const u8).read() != 0 {
                ItemSpawnSource::MasterBall
            } else {
                ItemSpawnSource::PokeBall
            }
        }
        _ => return,
    };
    store_selection(thread, public_kind, base_kind, source);
    let armed = crate::item_clones::queue_training_spawn_tickets(public_kind, source);
    log(format!(
        "[itemui] selected token={token:#x} public={public_kind:#x} base={base_kind:#x} source={source:?} ticket={armed}"
    ));
}

#[skyline::hook(offset = OFF_TRAINING_QUEUE_REQUEST, inline)]
unsafe fn training_queue_request(ctx: &mut skyline::hooks::InlineCtx) {
    let thread = current_thread();
    let Some((public_kind, base_kind, source)) = take_selection(thread) else {
        return;
    };
    let carrier_kind = ctx.registers[2].x() as u32 as i32;
    let actor_kind = ctx.registers[3].x() as u32 as i32;
    let native_match = match source {
        ItemSpawnSource::Direct => carrier_kind == base_kind,
        ItemSpawnSource::Assist => carrier_kind == 0 && actor_kind == base_kind,
        ItemSpawnSource::PokeBall => carrier_kind == 6 && actor_kind == base_kind,
        ItemSpawnSource::MasterBall => carrier_kind == 7 && actor_kind == base_kind,
        _ => false,
    };
    let armed =
        native_match && crate::item_clones::queue_training_spawn_tickets(public_kind, source);
    log(format!(
        "[itemui] queue public={public_kind:#x} base={base_kind:#x} source={source:?} carrier={carrier_kind} actor={actor_kind} native_match={native_match} ticket={armed}"
    ));
}

#[skyline::hook(offset = OFF_TRAINING_CONFIRM_CALLBACK)]
unsafe fn training_confirm_callback(callback: usize, token: *const u64, master: *const u8) {
    if !token.is_null() && token.read() & UI_HASH_MASK == page_ui_hash() {
        PAGE_CYCLE_PENDING.store(true, Ordering::Release);
        return;
    }
    call_original!(callback, token, master)
}

#[skyline::hook(offset = OFF_TRAINING_INPUT_TICK, inline)]
unsafe fn training_input_tick(_ctx: &mut skyline::hooks::InlineCtx) {
    if PAGE_CYCLE_PENDING.swap(false, Ordering::AcqRel) && !cycle_ordinary_page() {
        log("[itemui] training-ordinary: deferred page change failed".into());
    }
}

#[skyline::hook(offset = OFF_TRAINING_CURSOR_COMMIT, inline)]
unsafe fn training_cursor_commit(ctx: &mut skyline::hooks::InlineCtx) {
    if !RESET_CURSOR_AFTER_PAGE.swap(false, Ordering::AcqRel) {
        return;
    }
    let inner = ctx.registers[19].x() as usize;
    ctx.registers[20].set_x(1);
    let cursor = ctx.registers[8].x() as *mut u8;
    if !cursor.is_null() {
        (cursor.add(0x2A4) as *mut i32).write(1);
    }
    if inner != 0 && inner & 7 == 0 {
        let set_cursor: unsafe extern "C" fn(*mut usize, i32, i32, bool) =
            core::mem::transmute(crate::text_base() + 0x3783660);
        let cursor_slot = (inner + 0x7C0) as *mut usize;
        set_cursor(cursor_slot, 1, 1, true);

        let cursor_actor = cursor_slot.read();
        if cursor_actor != 0 && cursor_actor & 7 == 0 {
            let vtable = (cursor_actor as *const usize).read();
            if vtable != 0 && vtable & 7 == 0 {
                let enable_address = ((vtable + 0x70) as *const usize).read();
                if enable_address != 0 {
                    let enable: unsafe extern "C" fn(usize, bool) =
                        core::mem::transmute(enable_address);
                    enable(cursor_actor, true);
                    log("[itemui] training-ordinary: cursor rebound and enabled".into());
                }
            }
        }
    }
}

unsafe fn preflight() -> Result<(), (usize, u32, u32, &'static str)> {
    let text = crate::text_base();
    for &(offset, expected_words, label) in PREFLIGHT {
        for (index, expected) in expected_words.iter().copied().enumerate() {
            let address = offset + index * 4;
            let found = ((text + address) as *const u32).read();
            if found != expected {
                return Err((address, found, expected, label));
            }
        }
    }
    Ok(())
}

#[cfg(feature = "item_selftest")]
unsafe fn register_selftest_cells() {
    for (item_kind, ui_id) in [
        (0x36A, b"ui_item_okp\0".as_ptr()),
        (0x36B, b"ui_item_obv\0".as_ptr()),
    ] {
        let registration = CloneItemUiRegistrationV1 {
            api_version: API_VERSION_V1,
            struct_size: core::mem::size_of::<CloneItemUiRegistrationV1>() as u32,
            item_kind,
            flags: ITEM_UI_FLAG_TRAINING,
            ui_id: ui_id.cast(),
            training_order: 0,
            rules_order: 0,
            reserved: [0; 4],
        };
        let result = clone_engine_register_item_ui_v1(&registration);
        log(format!(
            "[itemui] selftest registration public={item_kind:#x} result={result}"
        ));
    }
}

pub(crate) fn install() {
    unsafe {
        if let Err((offset, found, expected, label)) = preflight() {
            skyline::println!(
                "[itemui] REFUSED functional backend: {label} at {offset:#x} found={found:#010x} expected={expected:#010x}"
            );
            return;
        }
        skyline::install_hooks!(
            ui_db_hash_lookup,
            ui_db_name_id,
            training_ordinary_list_done,
            training_pokemon_list_done,
            training_assist_list_done,
            training_master_list_done,
            training_selected_token,
            training_queue_request,
            training_confirm_callback,
            training_input_tick,
            training_cursor_commit
        );
        #[cfg(feature = "item_selftest")]
        register_selftest_cells();
    }
    crate::item_clones::mark_training_ui_ready();
    skyline::println!(
        "[itemui] sparse-safe Training backend installed; Rules paging remains disabled"
    );
}
