use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

use clone_engine_core::common_vtable::{
    self, Arena, Entries, Shared, ARENA_WORDS, HEADER_WORDS, OUR_CONTEXT_WORD,
    SMASHLINE_CONTEXT_WORD, SMASHLINE_MODULE_BLOCKS, SLOTS,
};
use clone_engine_core::slots::{clone_row, CLONE_SLOTS};

use crate::custom_articles;
use crate::item_clones::FIRST_SPARSE_ITEM_KIND;

pub(crate) const SPACE_FIGHTER: u32 = common_vtable::SPACE_FIGHTER;
pub(crate) const SPACE_WEAPON: u32 = common_vtable::SPACE_WEAPON;
pub(crate) const SPACE_ITEM: u32 = common_vtable::SPACE_ITEM;

static FIGHTER_TABLES: Mutex<Option<Vec<(i32, Entries)>>> = Mutex::new(None);
static WEAPON_TABLES: Mutex<Option<Vec<(i32, Entries)>>> = Mutex::new(None);
static ITEM_TABLES: Mutex<Option<Vec<(i32, Entries)>>> = Mutex::new(None);
static COPY_TABLES: Mutex<Option<Vec<(i32, Entries)>>> = Mutex::new(None);
static COPY_ARMED: AtomicU32 = AtomicU32::new(0);
static COPY_ROUTED: AtomicU32 = AtomicU32::new(0);
static COPY_PASSED: AtomicU32 = AtomicU32::new(0);
static COPY_CALLS: AtomicU32 = AtomicU32::new(0);
static COPY_ROUTED_BY: [AtomicU32; SLOTS] = [const { AtomicU32::new(0) }; SLOTS];
static COPY_PASSED_BY: [AtomicU32; SLOTS] = [const { AtomicU32::new(0) }; SLOTS];
static REPORTED: AtomicU32 = AtomicU32::new(0);
static DEFERRED: AtomicU32 = AtomicU32::new(0);
static RECONCILED: AtomicU32 = AtomicU32::new(0);

const FIGHTER_KIND_KIRBY: i32 = 6;
const AGENT_BOMA: usize = 0x40;
const FIRST_COPY_SLOT: u32 = common_vtable::SYS_LINE_SYSTEM_INIT;
const SPACE_COPY: u32 = 3;
const ARENA_FILL: usize = 0x4254_5645_4e4f_4c43;

static mut ARENA: [usize; ARENA_WORDS] = [ARENA_FILL; ARENA_WORDS];
static ARENA_BOOK: Mutex<Arena> = Mutex::new(Arena::new());
static PLACEMENT: AtomicU32 = AtomicU32::new(0);
static REFUSALS_SAID: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct MemoryInfo {
    address: u64,
    size: u64,
    kind: u32,
    attribute: u32,
    permission: u32,
    ipc_refcount: u32,
    device_refcount: u32,
    padding: u32,
}

#[cfg(not(target_arch = "aarch64"))]
unsafe fn query_memory(_info: *mut MemoryInfo, _address: u64) -> u32 {
    u32::MAX
}

#[cfg(target_arch = "aarch64")]
unsafe fn query_memory(info: *mut MemoryInfo, address: u64) -> u32 {
    let result: u64;
    core::arch::asm!(
        "svc 0x6",
        inout("x0") info as u64 => result,
        out("x1") _,
        in("x2") address,
        clobber_abi("C"),
        options(nostack),
    );
    result as u32
}

unsafe fn block_at(address: u64) -> Option<(u64, u64)> {
    let mut info = MemoryInfo::default();
    if query_memory(&mut info, address) != 0 || info.size == 0 {
        return None;
    }
    Some((info.address, info.size))
}

unsafe fn module_blocks() -> Option<[(u64, u64); SMASHLINE_MODULE_BLOCKS]> {
    let text = block_at(module_blocks as *const () as u64)?;
    let second = block_at(text.0 + text.1)?;
    let third = block_at(second.0 + second.1)?;
    Some([text, second, third])
}

fn arena_base() -> usize {
    core::ptr::addr_of_mut!(ARENA) as usize
}

unsafe fn arena_in_module() -> bool {
    match PLACEMENT.load(Ordering::Acquire) {
        1 => return true,
        2 => return false,
        _ => {}
    }
    let start = arena_base() as u64;
    let last = start + (ARENA_WORDS * 8) as u64 - 1;
    let blocks = module_blocks();
    let inside = blocks.is_some_and(|blocks| {
        common_vtable::inside_blocks(start, &blocks) && common_vtable::inside_blocks(last, &blocks)
    });
    PLACEMENT.store(if inside { 1 } else { 2 }, Ordering::Release);
    let seen = blocks
        .map(|blocks| {
            blocks
                .iter()
                .map(|(at, size)| format!("{at:#x}+{size:#x}"))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_else(|| "unreadable".to_string());
    crate::dbg_log_public(&format!(
        "[clone_engine] common vtable copies live at {start:#x}..{:#x}, {} the engine module's first three blocks ({seen}): Smashline reads them as {}",
        last + 1,
        if inside { "inside" } else { "OUTSIDE" },
        if inside {
            "a module's own vtables and leaves them alone"
        } else {
            "its own, so no copy is made while Smashline is loaded"
        },
    ));
    inside
}

#[allow(clippy::declare_interior_mutable_const)]
const NO_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static COPY_ORIGINALS: [AtomicUsize; SLOTS] = [NO_ORIGINAL; SLOTS];
static REGISTERED: AtomicU32 = AtomicU32::new(0);
static HOOKED: AtomicU32 = AtomicU32::new(0);
static SMASHLINE: AtomicU32 = AtomicU32::new(0);
static SMASHLINE_SYMBOL: AtomicUsize = AtomicUsize::new(0);
static WROTE_IN_PLACE: AtomicU32 = AtomicU32::new(0);
static COPIED: AtomicU32 = AtomicU32::new(0);
static REFUSED: AtomicU32 = AtomicU32::new(0);

fn table(space: u32) -> Option<&'static Mutex<Option<Vec<(i32, Entries)>>>> {
    match space {
        SPACE_FIGHTER => Some(&FIGHTER_TABLES),
        SPACE_WEAPON => Some(&WEAPON_TABLES),
        SPACE_ITEM => Some(&ITEM_TABLES),
        _ => None,
    }
}

fn known_kind(space: u32, kind: i32) -> bool {
    match space {
        SPACE_FIGHTER => clone_row(kind).is_some(),
        SPACE_WEAPON => custom_articles::custom_weapon_source_kind(kind).is_some(),
        SPACE_ITEM => {
            kind >= FIRST_SPARSE_ITEM_KIND
                && kind < FIRST_SPARSE_ITEM_KIND + crate::item_params::MAX_CLONE_KINDS as i32
        }
        _ => false,
    }
}

pub(crate) fn any_registered() -> bool {
    REGISTERED.load(Ordering::Relaxed) != 0
}

pub(crate) fn note_smashline() {
    let mut address = 0usize;
    let result = unsafe {
        skyline::nn::ro::LookupSymbol(
            &mut address as *mut usize,
            b"smashline_install_status_script\0".as_ptr(),
        )
    };
    let present = result == 0 && address != 0;
    SMASHLINE.store(present as u32 + 1, Ordering::Release);
    SMASHLINE_SYMBOL.store(address, Ordering::Relaxed);
}

fn smashline_present() -> bool {
    SMASHLINE.load(Ordering::Acquire) == 2
}

pub(crate) fn register(
    kind: i32,
    slot: u32,
    space: u32,
    function: usize,
) -> Result<Option<usize>, ()> {
    register_followed(kind, slot, space, function, 0)
}

pub(crate) fn register_followed(
    kind: i32,
    slot: u32,
    space: u32,
    function: usize,
    original_out: usize,
) -> Result<Option<usize>, ()> {
    if !known_kind(space, kind) {
        return Err(());
    }
    let mut guard = table(space).ok_or(())?.lock().map_err(|_| ())?;
    let tables = guard.get_or_insert_with(Vec::new);
    let entry = match tables.iter_mut().find(|(k, _)| *k == kind) {
        Some((_, entries)) => entries,
        None => {
            if tables.len() >= CLONE_SLOTS {
                return Err(());
            }
            tables.push((kind, Entries::default()));
            &mut tables.last_mut().ok_or(())?.1
        }
    };
    let previous = entry.set_followed(space, slot, function, original_out)?;
    REGISTERED.fetch_add(1, Ordering::Relaxed);
    drop(guard);
    Ok(previous)
}

pub(crate) fn register_copy(
    kind: i32,
    slot: u32,
    function: usize,
    original_out: usize,
) -> Result<Option<usize>, ()> {
    if clone_row(kind).is_none() || slot < FIRST_COPY_SLOT {
        return Err(());
    }
    let mut guard = COPY_TABLES.lock().map_err(|_| ())?;
    let tables = guard.get_or_insert_with(Vec::new);
    let entry = match tables.iter_mut().find(|(k, _)| *k == kind) {
        Some((_, entries)) => entries,
        None => {
            if tables.len() >= CLONE_SLOTS {
                return Err(());
            }
            tables.push((kind, Entries::default()));
            &mut tables.last_mut().ok_or(())?.1
        }
    };
    let previous = entry.set_followed(SPACE_FIGHTER, slot, function, original_out)?;
    let known = COPY_ORIGINALS[slot as usize].load(Ordering::Acquire);
    if known != 0 && original_out != 0 {
        unsafe {
            core::ptr::write_volatile(original_out as *mut usize, known);
        }
    }
    COPY_ARMED.store(1, Ordering::Release);
    REGISTERED.fetch_add(1, Ordering::Relaxed);
    drop(guard);
    Ok(previous)
}

unsafe fn copy_holder(agent: u64) -> Option<i32> {
    if agent == 0 {
        return None;
    }
    let boma = core::ptr::read_volatile((agent as usize + AGENT_BOMA) as *const u64);
    let (flag, _base, _entry, target) = crate::kirby_copy::kirby_copy_work_snapshot(boma)?;
    if flag == 0 {
        return None;
    }
    target
}

unsafe fn copy_entry(agent: u64, slot: u32) -> Option<usize> {
    let kind = copy_holder(agent)?;
    let guard = COPY_TABLES.lock().ok()?;
    let tables = guard.as_ref()?;
    tables
        .iter()
        .find(|(k, _)| *k == kind)
        .and_then(|(_, entries)| entries.get(slot))
}

unsafe fn route(agent: u64, slot: u32) -> usize {
    let index = (slot as usize).min(SLOTS - 1);
    let target = match copy_entry(agent, slot) {
        Some(function) => {
            COPY_ROUTED.fetch_add(1, Ordering::Relaxed);
            COPY_ROUTED_BY[index].fetch_add(1, Ordering::Relaxed);
            report_once(3);
            function
        }
        None => {
            COPY_PASSED.fetch_add(1, Ordering::Relaxed);
            COPY_PASSED_BY[index].fetch_add(1, Ordering::Relaxed);
            COPY_ORIGINALS[slot as usize].load(Ordering::Acquire)
        }
    };
    let calls = COPY_CALLS.fetch_add(1, Ordering::Relaxed) + 1;
    if matches!(calls, 16 | 256 | 4096) {
        crate::dbg_log_public(&format!("[clone_engine] {}", report()));
    }
    target
}

unsafe extern "C" fn copy_line_target(agent: u64, slot: u64) -> usize {
    route(agent, slot as u32)
}

#[cfg(target_arch = "aarch64")]
#[unsafe(naked)]
unsafe extern "C" fn copy_line_body() {
    core::arch::naked_asm!(
        "stp x29, x30, [sp, #-0x60]!",
        "mov x29, sp",
        "stp x0, x1, [sp, #0x10]",
        "stp x2, x3, [sp, #0x20]",
        "stp x4, x5, [sp, #0x30]",
        "stp x6, x7, [sp, #0x40]",
        "str x8, [sp, #0x50]",
        "mov x1, x9",
        "bl {target}",
        "mov x16, x0",
        "ldr x8, [sp, #0x50]",
        "ldp x6, x7, [sp, #0x40]",
        "ldp x4, x5, [sp, #0x30]",
        "ldp x2, x3, [sp, #0x20]",
        "ldp x0, x1, [sp, #0x10]",
        "ldp x29, x30, [sp], #0x60",
        "cbz x16, 2f",
        "br x16",
        "2:",
        "ret",
        target = sym copy_line_target,
    )
}

macro_rules! copy_dispatchers {
    ($($name:ident = $slot:expr;)*) => {
        $(
            #[cfg(target_arch = "aarch64")]
            #[unsafe(naked)]
            unsafe extern "C" fn $name() {
                core::arch::naked_asm!(
                    "mov x9, #{slot}",
                    "b {body}",
                    slot = const $slot,
                    body = sym copy_line_body,
                )
            }

            #[cfg(not(target_arch = "aarch64"))]
            unsafe extern "C" fn $name() {}
        )*

        fn dispatcher_for(slot: u32) -> Option<usize> {
            $(
                if slot == $slot {
                    return Some($name as *const () as usize);
                }
            )*
            None
        }
    };
}

copy_dispatchers! {
    copy_sys_line_system_init = common_vtable::SYS_LINE_SYSTEM_INIT;
    copy_sub_begin_added_lines = common_vtable::SUB_BEGIN_ADDED_LINES;
    copy_sys_line_status_end_control = common_vtable::SYS_LINE_STATUS_END_CONTROL;
    copy_sub_end_added_lines = common_vtable::SUB_END_ADDED_LINES;
    copy_reset = common_vtable::RESET;
}

unsafe fn install_copy_dispatch(agent: u64) {
    if COPY_ARMED.load(Ordering::Acquire) == 0 || agent == 0 {
        return;
    }
    let vptr_slot = agent as usize as *mut usize;
    if !readable(vptr_slot as usize) {
        return;
    }
    let vtable = core::ptr::read_volatile(vptr_slot);
    if !readable(vtable) {
        return;
    }
    let vtable = match shared_state(vtable) {
        Shared::SmashlinePrivate | Shared::OursPrivate => vtable,
        Shared::Yes if smashline_present() => {
            defer(SPACE_COPY, FIGHTER_KIND_KIRBY, agent);
            return;
        }
        Shared::Yes => match place_copy(SPACE_COPY, FIGHTER_KIND_KIRBY, vtable, SLOTS) {
            Some(copy) => {
                core::ptr::write_volatile(vptr_slot, copy);
                COPIED.fetch_add(1, Ordering::Relaxed);
                copy
            }
            None => {
                refuse("a KIRBY agent's vtable could not be copied");
                return;
            }
        },
    };

    for slot in FIRST_COPY_SLOT..SLOTS as u32 {
        if !copy_slot_armed(slot) {
            continue;
        }
        let Some(dispatcher) = dispatcher_for(slot) else {
            continue;
        };
        let cell = (vtable + slot as usize * 8) as *mut usize;
        let present = core::ptr::read_volatile(cell);
        if present == dispatcher {
            continue;
        }
        if !is_code(present) {
            refuse("a KIRBY agent's system line entry is not a function; that slot keeps its own entry");
            continue;
        }
        match COPY_ORIGINALS[slot as usize].compare_exchange(
            0,
            present,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => publish_copy_original(slot, present),
            Err(known) if known != present => {
                refuse(
                    "a KIRBY agent's system line entry differs from the one the dispatcher forwards \
                     to, so another mod replaced it; that slot keeps its own entry",
                );
                continue;
            }
            Err(_) => {}
        }
        core::ptr::write_volatile(cell, dispatcher);
    }
}

fn copy_slot_armed(slot: u32) -> bool {
    let Ok(guard) = COPY_TABLES.lock() else {
        return false;
    };
    guard
        .as_ref()
        .is_some_and(|tables| tables.iter().any(|(_, entries)| entries.get(slot).is_some()))
}

fn is_code(address: usize) -> bool {
    address >= 0x10000 && address % 4 == 0
}

fn publish_copy_original(slot: u32, original: usize) {
    let Ok(guard) = COPY_TABLES.lock() else {
        return;
    };
    let Some(tables) = guard.as_ref() else {
        return;
    };
    for (_, entries) in tables.iter() {
        for (at, entry) in entries.iter() {
            if at == slot && entry.original_out != 0 {
                unsafe {
                    core::ptr::write_volatile(entry.original_out as *mut usize, original);
                }
            }
        }
    }
}

const PENDING_MAX: usize = 64;
const PENDING_TRIES: u32 = 8;

static PENDING: Mutex<Option<Vec<(u64, u32, i32, u32)>>> = Mutex::new(None);

fn defer(space: u32, kind: i32, agent: u64) {
    let Ok(mut guard) = PENDING.lock() else {
        return;
    };
    let list = guard.get_or_insert_with(Vec::new);
    if list.iter().any(|(a, s, k, _)| *a == agent && *s == space && *k == kind) {
        return;
    }
    if list.len() >= PENDING_MAX {
        return;
    }
    list.push((agent, space, kind, 0));
    DEFERRED.fetch_add(1, Ordering::Relaxed);
}

unsafe fn reconcile() {
    let Ok(mut guard) = PENDING.lock() else {
        return;
    };
    let Some(list) = guard.as_mut() else {
        return;
    };
    if list.is_empty() {
        return;
    }
    let mut done: Vec<(u32, i32, u64)> = Vec::new();
    list.retain_mut(|(agent, space, kind, tries)| {
        let vptr_slot = *agent as usize as *mut usize;
        if !readable(vptr_slot as usize) {
            return false;
        }
        let vtable = core::ptr::read_volatile(vptr_slot);
        if !readable(vtable) {
            return false;
        }
        match shared_state(vtable) {
            Shared::SmashlinePrivate | Shared::OursPrivate => {
                done.push((*space, *kind, *agent));
                false
            }
            Shared::Yes => {
                *tries += 1;
                if *tries >= PENDING_TRIES {
                    refuse(
                        "an agent's vtable was still shared after every reconcile attempt, so the \
                         entries for it were dropped rather than written to a table the whole \
                         roster reads",
                    );
                    false
                } else {
                    true
                }
            }
        }
    });
    drop(guard);
    for (space, kind, agent) in done {
        RECONCILED.fetch_add(1, Ordering::Relaxed);
        if space == SPACE_COPY {
            install_copy_dispatch(agent);
        } else {
            install(space, kind, agent);
        }
    }
}

fn refuse(reason: &'static str) {
    REFUSED.fetch_add(1, Ordering::Relaxed);
    let first = match REFUSALS_SAID.lock() {
        Ok(mut said) if !said.contains(&reason) => {
            said.push(reason);
            true
        }
        _ => false,
    };
    if first {
        crate::dbg_log_public(&format!("[clone_engine] common vtable refused: {reason}"));
    }
}

fn report_once(which: u32) {
    let bit = 1u32 << which;
    if REPORTED.fetch_or(bit, Ordering::AcqRel) & bit == 0 {
        crate::dbg_log_public(&format!("[clone_engine] {}", report()));
    }
}

fn entries_for(space: u32, kind: i32) -> Option<Entries> {
    let guard = table(space)?.lock().ok()?;
    let tables = guard.as_ref()?;
    tables
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, entries)| *entries)
        .filter(Entries::any)
}

fn readable(address: usize) -> bool {
    address >= 0x10000 && address % 8 == 0
}

unsafe fn context_magic(vtable: usize, word: usize) -> Option<u64> {
    let cell = vtable.checked_sub(word * 8)?;
    if !readable(cell) {
        return None;
    }
    let context = core::ptr::read_volatile(cell as *const usize);
    if !readable(context) {
        return Some(0);
    }
    Some(core::ptr::read_volatile(context as *const u64))
}

unsafe fn shared_state(vtable: usize) -> Shared {
    let smashline = context_magic(vtable, SMASHLINE_CONTEXT_WORD);
    if smashline == Some(common_vtable::SMASHLINE_MAGIC) {
        return Shared::SmashlinePrivate;
    }
    common_vtable::classify(smashline, context_magic(vtable, OUR_CONTEXT_WORD))
}

pub(crate) unsafe fn original_vtable(vtable: usize) -> Option<usize> {
    let header = vtable.checked_sub(OUR_CONTEXT_WORD * 8)?;
    if !readable(header) {
        return None;
    }
    let context = core::ptr::read_volatile(header as *const usize);
    if !readable(context) {
        return None;
    }
    if core::ptr::read_volatile(context as *const u64) != common_vtable::OUR_MAGIC {
        return None;
    }
    let original = core::ptr::read_volatile((context + 8) as *const u64) as usize;
    (original != 0).then_some(original)
}

unsafe fn copy_slots_for(space: u32, vtable: usize) -> Option<usize> {
    if space != SPACE_ITEM {
        return common_vtable::slots_in(space);
    }
    crate::item_clones::agent_vtable_slots(vtable)
}

unsafe fn place_copy(space: u32, kind: i32, vtable: usize, slots: usize) -> Option<usize> {
    let header = vtable.checked_sub(HEADER_WORDS * 8)?;
    if !readable(header) {
        return None;
    }
    let (placed, fresh) = ARENA_BOOK.lock().ok()?.place(space, kind, vtable, slots)?;
    let base = arena_base() as *mut usize;
    if fresh {
        core::ptr::write_volatile(base.add(placed.context()), common_vtable::OUR_MAGIC as usize);
        core::ptr::write_volatile(base.add(placed.context() + 1), vtable);
        core::ptr::write_volatile(
            base.add(placed.vtable() - OUR_CONTEXT_WORD),
            base.add(placed.context()) as usize,
        );
        core::ptr::write_volatile(
            base.add(placed.vtable() - SMASHLINE_CONTEXT_WORD),
            core::ptr::read_volatile((vtable - SMASHLINE_CONTEXT_WORD * 8) as *const usize),
        );
        for slot in 0..slots {
            core::ptr::write_volatile(
                base.add(placed.vtable() + slot),
                core::ptr::read_volatile((vtable + slot * 8) as *const usize),
            );
        }
    }
    Some(base.add(placed.vtable()) as usize)
}

unsafe fn write_entries(vtable: usize, entries: &Entries) {
    for (slot, entry) in entries.iter() {
        let cell = (vtable + slot as usize * 8) as *mut usize;
        let present = core::ptr::read_volatile(cell);
        if entry.original_out != 0 && present != entry.function && is_code(present) {
            core::ptr::write_volatile(entry.original_out as *mut usize, present);
        }
        core::ptr::write_volatile(cell, entry.function);
    }
}

pub(crate) unsafe fn install(space: u32, kind: i32, agent: u64) {
    if agent == 0 || !any_registered() {
        return;
    }
    let entries = match entries_for(space, kind) {
        Some(entries) => entries,
        None => return,
    };
    let vptr_slot = agent as usize as *mut usize;
    if !readable(vptr_slot as usize) {
        return;
    }
    let vtable = core::ptr::read_volatile(vptr_slot);
    if !readable(vtable) {
        return;
    }

    match shared_state(vtable) {
        Shared::SmashlinePrivate | Shared::OursPrivate => {
            write_entries(vtable, &entries);
            WROTE_IN_PLACE.fetch_add(1, Ordering::Relaxed);
        }
        Shared::Yes if smashline_present() && space != SPACE_ITEM => {
            defer(space, kind, agent)
        }
        Shared::Yes if smashline_present() && !arena_in_module() => refuse(
            "Smashline is loaded and the copy arena is outside the engine module, so Smashline \
             would read a copy as its own; the agent keeps its shared vtable",
        ),
        Shared::Yes => match copy_slots_for(space, vtable) {
            None => refuse(
                "this agent's vtable has no known slot count, so a copy would truncate it; \
                 refusing rather than repointing to a short table",
            ),
            Some(slots) => match place_copy(space, kind, vtable, slots) {
                Some(copy) => {
                    write_entries(copy, &entries);
                    core::ptr::write_volatile(vptr_slot, copy);
                    COPIED.fetch_add(1, Ordering::Relaxed);
                }
                None => refuse("the copy arena is full or the agent's vtable was unreadable"),
            },
        },
    }
    report_once(space);
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn on_fighter_agent(object: u64, agent: u64) {
    reconcile();
    if let Some(kind) = crate::clone_kind_of_object(object) {
        install(SPACE_FIGHTER, kind, agent);
    }
    if object != 0 && core::ptr::read_volatile((object + 0xc) as *const i32) == FIGHTER_KIND_KIRBY {
        install_copy_dispatch(agent);
    }
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn on_weapon_agent(object: u64, agent: u64) {
    reconcile();
    if object == 0 {
        return;
    }
    let kind = core::ptr::read_volatile((object + 0xc) as *const i32);
    if known_kind(SPACE_WEAPON, kind) {
        install(SPACE_WEAPON, kind, agent);
    }
}


pub(crate) fn report() -> String {
    let by_slot: String = (FIRST_COPY_SLOT as usize..SLOTS)
        .filter_map(|slot| {
            let routed = COPY_ROUTED_BY[slot].load(Ordering::Relaxed);
            let passed = COPY_PASSED_BY[slot].load(Ordering::Relaxed);
            (routed + passed > 0).then(|| format!(" [slot {slot}: {routed}/{passed}]"))
        })
        .collect();
    format!(
        "common vtables: smashline={} {} registered, {} written in place, {} copied ({} arena words), {} deferred, {} reconciled, {} refused, copy routed {} passed {}{by_slot}",
        smashline_present(),
        REGISTERED.load(Ordering::Relaxed),
        WROTE_IN_PLACE.load(Ordering::Relaxed),
        COPIED.load(Ordering::Relaxed),
        ARENA_BOOK.lock().map(|book| book.used()).unwrap_or(0),
        DEFERRED.load(Ordering::Relaxed),
        RECONCILED.load(Ordering::Relaxed),
        REFUSED.load(Ordering::Relaxed),
        COPY_ROUTED.load(Ordering::Relaxed),
        COPY_PASSED.load(Ordering::Relaxed),
    )
}
