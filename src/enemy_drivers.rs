use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use crate::enemy_clones::{base_of, KIND_LIMIT, VANILLA_KINDS};

const OFF_FACTORY: usize = 0x57_fa30;
const FACTORY_OPCODE: u32 = 0xd101_83ff;
const HOLDER_INSTANCE: usize = 0x10;
const INSTANCE_ACCESSOR: usize = 8;
const INSTANCE_STATUS: usize = 0x10;
const INSTANCE_KIND: usize = 0x14;
const ACCESSOR_IN_OBJECT: usize = 0x70;
const OBJECT_ID: usize = 8;
const OBJECT_KIND: usize = 0xc;
const VTABLE_SLOTS: usize = 77;
const VTABLE_HEAD: usize = 2;
const SLOT_CHANGE_STATUS: usize = 73;
const SLOT_REQUEST_STATUS: usize = 74;
const SLOT_INSTALL_STATUS: usize = 75;
const SLOT_INSTALL_REQUEST: usize = 76;
const STATUS_BITS: i32 = 32;

pub(crate) type DriverFn = unsafe extern "C" fn(u32, usize, i32);
type ChangeFn = unsafe extern "C" fn(usize, i32);
type InstallFn = unsafe extern "C" fn(usize, i32, usize, usize, usize, usize, usize, usize);

const UNSET: AtomicUsize = AtomicUsize::new(0);
const NONE_NATIVE: AtomicU32 = AtomicU32::new(0);
static DRIVERS: [AtomicUsize; KIND_LIMIT as usize] = [UNSET; KIND_LIMIT as usize];
static NATIVE: [AtomicU32; KIND_LIMIT as usize] = [NONE_NATIVE; KIND_LIMIT as usize];
const UNSET_ROW: [AtomicUsize; VTABLE_SLOTS] = [UNSET; VTABLE_SLOTS];
static HOOKS: [[AtomicUsize; VTABLE_SLOTS]; KIND_LIMIT as usize] = [UNSET_ROW; KIND_LIMIT as usize];
static HOOKED: [AtomicU32; KIND_LIMIT as usize] = [NONE_NATIVE; KIND_LIMIT as usize];
const OBJECT_VTABLE: usize = 0x4f7_5bb0;
const OBJECT_SLOTS: usize = 163;
const OBJECT_ROW: [AtomicUsize; OBJECT_SLOTS] = [UNSET; OBJECT_SLOTS];
static OBJECT_HOOKS: [[AtomicUsize; OBJECT_SLOTS]; KIND_LIMIT as usize] = [OBJECT_ROW; KIND_LIMIT as usize];
static OBJECT_TABLES: [AtomicUsize; KIND_LIMIT as usize] = [UNSET; KIND_LIMIT as usize];
static OBJECT_LOGS: AtomicU32 = AtomicU32::new(0);
static ORIGINALS: [AtomicUsize; VANILLA_KINDS as usize] = [UNSET; VANILLA_KINDS as usize];
const RAW_ROW: [usize; VTABLE_SLOTS] = [0; VTABLE_SLOTS];
static mut ENEMY_DRIVEN_RAW: [[usize; VTABLE_SLOTS]; VANILLA_KINDS as usize] = [RAW_ROW; VANILLA_KINDS as usize];
const SWAP_DEPTH: usize = 8;
static mut SWAPS: [(usize, usize); SWAP_DEPTH] = [(0, 0); SWAP_DEPTH];
static mut SWAP_COUNT: usize = 0;
const HOLDER_PARAMS: usize = 0xb8;
const PARAMS_OBJECTS: usize = 0xd8;
const OFF_PARAM_HOLDER: usize = 0x52b_8418;

extern "C" {
    static ENEMY_DRIVEN_TRAMPOLINES: [usize; VTABLE_SLOTS];
}

core::arch::global_asm!(
    ".pushsection .text.enemy_driven_trampolines, \"ax\"",
    ".irp slot, 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63,64,65,66,67,68,69,70,71,72,73,74,75,76",
    ".p2align 2",
    "enemy_driven_trampoline_\\slot:",
    "stp x29, x30, [sp, #-256]!",
    "mov x29, sp",
    "stp x0, x1, [sp, #16]",
    "stp x2, x3, [sp, #32]",
    "stp x4, x5, [sp, #48]",
    "stp x6, x7, [sp, #64]",
    "str x8, [sp, #80]",
    "stp q0, q1, [sp, #96]",
    "stp q2, q3, [sp, #128]",
    "stp q4, q5, [sp, #160]",
    "stp q6, q7, [sp, #192]",
    "bl {enter}",
    "str x0, [sp, #224]",
    "ldr x0, [sp, #16]",
    "mov x1, #\\slot",
    "bl {target}",
    "mov x16, x0",
    "ldp x0, x1, [sp, #16]",
    "ldp x2, x3, [sp, #32]",
    "ldp x4, x5, [sp, #48]",
    "ldp x6, x7, [sp, #64]",
    "ldr x8, [sp, #80]",
    "ldp q0, q1, [sp, #96]",
    "ldp q2, q3, [sp, #128]",
    "ldp q4, q5, [sp, #160]",
    "ldp q6, q7, [sp, #192]",
    "blr x16",
    "stp x0, x1, [sp, #16]",
    "stp q0, q1, [sp, #96]",
    "stp q2, q3, [sp, #128]",
    "ldr x0, [sp, #224]",
    "bl {exit}",
    "ldp x0, x1, [sp, #16]",
    "ldp q0, q1, [sp, #96]",
    "ldp q2, q3, [sp, #128]",
    "ldp x29, x30, [sp], #256",
    "ret",
    ".endr",
    ".popsection",
    ".pushsection .data.rel.ro.enemy_driven_trampolines, \"aw\"",
    ".p2align 3",
    ".global ENEMY_DRIVEN_TRAMPOLINES",
    ".hidden ENEMY_DRIVEN_TRAMPOLINES",
    "ENEMY_DRIVEN_TRAMPOLINES:",
    ".irp slot, 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63,64,65,66,67,68,69,70,71,72,73,74,75,76",
    ".quad enemy_driven_trampoline_\\slot",
    ".endr",
    ".popsection",
    enter = sym enemy_driven_enter,
    exit = sym enemy_driven_exit,
    target = sym enemy_slot_target,
);

unsafe extern "C" fn enemy_slot_target(instance: usize, slot: usize) -> usize {
    let kind = clone_kind(instance) as usize;
    if kind < KIND_LIMIT as usize && slot < VTABLE_SLOTS {
        let hook = HOOKS[kind][slot].load(Ordering::Acquire);
        if hook != 0 {
            return hook;
        }
    }
    let base = core::ptr::read_volatile((instance + INSTANCE_KIND) as *const u32) as usize;
    ENEMY_DRIVEN_RAW[base][slot]
}

pub(crate) fn hook(kind: u32, slot: u32, function: usize) -> bool {
    if base_of(kind).is_none() || slot as usize >= VTABLE_SLOTS {
        return false;
    }
    HOOKS[kind as usize][slot as usize].store(function, Ordering::Release);
    let hooked = HOOKS[kind as usize].iter().any(|entry| entry.load(Ordering::Acquire) != 0);
    HOOKED[kind as usize].store(u32::from(hooked), Ordering::Release);
    skyline::println!("[enemydrive] kind {kind}: slot {slot} {}", if function == 0 { "back to the base's" } else { "hooked" });
    true
}

fn object_image() -> usize {
    crate::game::addr(OBJECT_VTABLE)
}

fn object_hooked(kind: usize) -> bool {
    OBJECT_HOOKS[kind].iter().any(|entry| entry.load(Ordering::Acquire) != 0)
}

unsafe fn object_table(kind: usize) -> usize {
    let known = OBJECT_TABLES[kind].load(Ordering::Acquire);
    if known != 0 {
        return known;
    }
    let _guard = BUILDING.lock();
    let known = OBJECT_TABLES[kind].load(Ordering::Acquire);
    if known != 0 {
        return known;
    }
    let image = object_image();
    let mut table = vec![0usize; VTABLE_HEAD + OBJECT_SLOTS].into_boxed_slice();
    for (index, entry) in table.iter_mut().enumerate() {
        *entry = read(image - VTABLE_HEAD * 8 + index * 8);
    }
    for slot in 0..OBJECT_SLOTS {
        let hook = OBJECT_HOOKS[kind][slot].load(Ordering::Acquire);
        if hook != 0 {
            table[VTABLE_HEAD + slot] = hook;
        }
    }
    let vtable = Box::leak(table).as_ptr() as usize + VTABLE_HEAD * 8;
    OBJECT_TABLES[kind].store(vtable, Ordering::Release);
    skyline::println!("[enemydrive] kind {kind} owns a private copy of the enemy object's {OBJECT_SLOTS}-slot vtable at {vtable:#x}");
    vtable
}

unsafe fn adopt_object(kind: u32, instance: usize) {
    let accessor = read(instance + INSTANCE_ACCESSOR);
    if accessor == 0 {
        return;
    }
    let object = accessor - ACCESSOR_IN_OBJECT;
    let held = read(object);
    let image = object_image();
    let wanted = if kind < KIND_LIMIT && object_hooked(kind as usize) {
        object_table(kind as usize)
    } else {
        image
    };
    if held == wanted {
        return;
    }
    let ours = held == image || OBJECT_TABLES.iter().any(|table| table.load(Ordering::Acquire) == held);
    let logs = OBJECT_LOGS.fetch_add(1, Ordering::Relaxed);
    if !ours {
        if logs < 8 {
            skyline::println!(
                "[enemydrive] kind {kind}: object {object:#x} holds vtable {held:#x} (image +{:#x}), not the enemy object's; left alone",
                crate::game::offset_of(held)
            );
        }
        return;
    }
    core::ptr::write_volatile(object as *mut usize, wanted);
    if logs < 8 {
        skyline::println!(
            "[enemydrive] kind {kind}: object {object:#x} dispatches through {}",
            if wanted == image { "the game's vtable" } else { "its private vtable" }
        );
    }
}

pub(crate) fn object_hook(kind: u32, slot: u32, function: usize) -> bool {
    if base_of(kind).is_none() || slot as usize >= OBJECT_SLOTS {
        return false;
    }
    OBJECT_HOOKS[kind as usize][slot as usize].store(function, Ordering::Release);
    let table = OBJECT_TABLES[kind as usize].load(Ordering::Acquire);
    if table != 0 {
        let entry = if function == 0 {
            unsafe { read(object_image() + slot as usize * 8) }
        } else {
            function
        };
        unsafe { core::ptr::write_volatile((table + slot as usize * 8) as *mut usize, entry) };
    }
    skyline::println!(
        "[enemydrive] kind {kind}: object slot {slot} {}",
        if function == 0 { "back to the game's" } else { "hooked" }
    );
    true
}

pub(crate) fn object_original(slot: u32) -> usize {
    if slot as usize >= OBJECT_SLOTS {
        return 0;
    }
    unsafe { read(object_image() + slot as usize * 8) }
}

pub(crate) fn original(kind: u32, slot: u32) -> usize {
    let Some(base) = base_of(kind) else {
        return 0;
    };
    if slot as usize >= VTABLE_SLOTS || DRIVEN[base as usize].load(Ordering::Acquire) == 0 {
        return 0;
    }
    unsafe { ENEMY_DRIVEN_RAW[base as usize][slot as usize] }
}

unsafe extern "C" fn enemy_driven_enter(instance: usize) -> usize {
    let own = crate::enemy_params::own_object(clone_kind(instance));
    if own == 0 || SWAP_COUNT >= SWAP_DEPTH {
        return 0;
    }
    let holder = read(crate::game::addr(OFF_PARAM_HOLDER));
    let params = if holder == 0 { 0 } else { read(holder + HOLDER_PARAMS) };
    if params == 0 {
        return 0;
    }
    let base = core::ptr::read_volatile((instance + INSTANCE_KIND) as *const u32) as usize;
    let slot = params + PARAMS_OBJECTS + base * 8;
    let current = read(slot);
    if current == own {
        return 0;
    }
    SWAPS[SWAP_COUNT] = (slot, current);
    SWAP_COUNT += 1;
    core::ptr::write_volatile(slot as *mut usize, own);
    1
}

unsafe extern "C" fn enemy_driven_exit(token: usize) {
    if token == 0 || SWAP_COUNT == 0 {
        return;
    }
    SWAP_COUNT -= 1;
    let (slot, old) = SWAPS[SWAP_COUNT];
    core::ptr::write_volatile(slot as *mut usize, old);
}
static DRIVEN: [AtomicUsize; VANILLA_KINDS as usize] = [UNSET; VANILLA_KINDS as usize];
static BUILDING: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) fn drive(kind: u32, driver: Option<DriverFn>, native: u32) -> bool {
    if base_of(kind).is_none() {
        return false;
    }
    NATIVE[kind as usize].store(native, Ordering::Release);
    DRIVERS[kind as usize].store(driver.map_or(0, |driver| driver as usize), Ordering::Release);
    skyline::println!("[enemydrive] kind {kind}: driven {}, native statuses {native:#x}", driver.is_some());
    true
}

unsafe fn read(address: usize) -> usize {
    core::ptr::read_volatile(address as *const usize)
}

unsafe fn object_of(instance: usize) -> usize {
    read(instance + INSTANCE_ACCESSOR) - ACCESSOR_IN_OBJECT
}

unsafe fn clone_kind(instance: usize) -> u32 {
    core::ptr::read_volatile((object_of(instance) + OBJECT_KIND) as *const u32)
}

unsafe fn original_slot(instance: usize, slot: usize) -> usize {
    let base = core::ptr::read_volatile((instance + INSTANCE_KIND) as *const u32) as usize;
    read(ORIGINALS[base].load(Ordering::Acquire) + slot * 8)
}

unsafe fn driven_vtable(base: usize, original: usize) -> usize {
    let built = DRIVEN[base].load(Ordering::Acquire);
    if built != 0 {
        return built;
    }
    let _guard = BUILDING.lock();
    let built = DRIVEN[base].load(Ordering::Acquire);
    if built != 0 {
        return built;
    }
    let mut table = vec![0usize; VTABLE_HEAD + VTABLE_SLOTS].into_boxed_slice();
    for (index, entry) in table.iter_mut().enumerate() {
        *entry = read(original - VTABLE_HEAD * 8 + index * 8);
    }
    table[VTABLE_HEAD + SLOT_CHANGE_STATUS] = driven_change as usize;
    table[VTABLE_HEAD + SLOT_REQUEST_STATUS] = driven_request as usize;
    for slot in 0..VTABLE_SLOTS {
        ENEMY_DRIVEN_RAW[base][slot] = table[VTABLE_HEAD + slot];
        table[VTABLE_HEAD + slot] = ENEMY_DRIVEN_TRAMPOLINES[slot];
    }
    let vtable = Box::leak(table).as_ptr() as usize + VTABLE_HEAD * 8;
    ORIGINALS[base].store(original, Ordering::Release);
    DRIVEN[base].store(vtable, Ordering::Release);
    vtable
}

fn native(kind: u32, status: i32) -> bool {
    !(0..STATUS_BITS).contains(&status) || NATIVE[kind as usize].load(Ordering::Acquire) >> status & 1 != 0
}

unsafe fn route(instance: usize, status: i32, own: usize, install: usize) {
    let kind = clone_kind(instance);
    if native(kind, status) || DRIVERS[kind as usize].load(Ordering::Acquire) == 0 {
        let change: ChangeFn = core::mem::transmute(original_slot(instance, own));
        change(instance, status);
        return;
    }
    let install: InstallFn = core::mem::transmute(original_slot(instance, install));
    install(instance, status, driven_exec as usize, 0, driven_enter as usize, 0, 0, 0);
}

unsafe extern "C" fn driven_change(instance: usize, status: i32) {
    route(instance, status, SLOT_CHANGE_STATUS, SLOT_INSTALL_STATUS);
}

unsafe extern "C" fn driven_request(instance: usize, status: i32) {
    route(instance, status, SLOT_REQUEST_STATUS, SLOT_INSTALL_REQUEST);
}

unsafe extern "C" fn driven_enter(_instance: usize, _out: usize) {}

unsafe extern "C" fn driven_exec(instance: usize, _out: usize) {
    let object = object_of(instance);
    let kind = core::ptr::read_volatile((object + OBJECT_KIND) as *const u32);
    let driver = DRIVERS[kind as usize].load(Ordering::Acquire);
    if driver == 0 {
        return;
    }
    let driver: DriverFn = core::mem::transmute(driver);
    let id = core::ptr::read_volatile((object + OBJECT_ID) as *const u32);
    let status = core::ptr::read_volatile((instance + INSTANCE_STATUS) as *const i32);
    driver(id, read(instance + INSTANCE_ACCESSOR), status);
}

#[skyline::hook(offset = crate::game::rel(OFF_FACTORY))]
unsafe fn factory(holder: usize, kind: u32, setup: usize, variation: u32) {
    call_original!(holder, kind, setup, variation);
    let created = read(holder + HOLDER_INSTANCE);
    if created != 0 {
        adopt_object(kind, created);
    }
    if kind >= KIND_LIMIT
        || (DRIVERS[kind as usize].load(Ordering::Acquire) == 0 && HOOKED[kind as usize].load(Ordering::Acquire) == 0)
    {
        return;
    }
    let Some(base) = base_of(kind) else {
        return;
    };
    let instance = read(holder + HOLDER_INSTANCE);
    if instance == 0 {
        return;
    }
    let vtable = driven_vtable(base as usize, read(instance));
    core::ptr::write_volatile(instance as *mut usize, vtable);
    skyline::println!("[enemydrive] kind {kind} (on {base}): instance {instance:#x} runs through the clone's entries");
}

pub(crate) fn install() {
    let observed = unsafe { crate::game::text_word(OFF_FACTORY) };
    if observed != FACTORY_OPCODE {
        skyline::println!("[enemydrive] REFUSED {OFF_FACTORY:#x}: expected {FACTORY_OPCODE:#010x}, found {observed:#010x}");
        return;
    }
    skyline::install_hooks!(factory);
}

const OBJECT_HOLDER: usize = 0x158;
const DRIVE_NOW_STATUS: i32 = 1;

pub(crate) unsafe fn drive_now(object: u32) -> bool {
    let address = crate::enemy_spawn::object_address(object);
    if address == 0 {
        return false;
    }
    let holder = read(address + OBJECT_HOLDER);
    let instance = if holder == 0 { 0 } else { read(holder + HOLDER_INSTANCE) };
    if instance == 0 || read(instance) != DRIVEN[core::ptr::read_volatile((instance + INSTANCE_KIND) as *const u32) as usize % VANILLA_KINDS as usize].load(Ordering::Acquire) {
        return false;
    }
    route(instance, DRIVE_NOW_STATUS, SLOT_CHANGE_STATUS, SLOT_INSTALL_STATUS);
    true
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_enemy_drive_now(object: u32) -> u32 {
    u32::from(drive_now(object))
}

#[no_mangle]
pub extern "C" fn clone_engine_enemy_vtable_entry_v1(kind: u32, slot: u32, function: usize) -> i32 {
    if hook(kind, slot, function) {
        0
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_enemy_original_entry_v1(kind: u32, slot: u32) -> usize {
    original(kind, slot)
}

#[no_mangle]
pub extern "C" fn clone_engine_enemy_object_entry_v1(kind: u32, slot: u32, function: usize) -> i32 {
    if object_hook(kind, slot, function) {
        0
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_enemy_object_original_v1(slot: u32) -> usize {
    object_original(slot)
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_enemy_drive(kind: u32, driver: usize, native_statuses: u32) -> i32 {
    let driver = (driver != 0).then(|| core::mem::transmute::<usize, DriverFn>(driver));
    if drive(kind, driver, native_statuses) {
        0
    } else {
        -1
    }
}
