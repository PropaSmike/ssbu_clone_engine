use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicUsize, Ordering};

const NO_POSITION: u64 = u64::MAX;

use crate::item_clones::live_identity_of_object;

const OFF_CREATE_GROUND_COLLISION: usize = 0x166A210;
const OFF_CHECK_RANGE_FROM_OWNER: usize = 0x1669370;
const OFF_REGIST_BLOCK_TO_GRID: usize = 0x166A060;

const PREFLIGHT: &[(usize, &[u32])] = &[
    (
        OFF_CREATE_GROUND_COLLISION,
        &[0xD103C3FF, 0xF9005BF7, 0xA90C57F6, 0xA90D4FF4],
    ),
    (
        OFF_CHECK_RANGE_FROM_OWNER,
        &[0xD10183FF, 0x6D0123E9, 0xF90013F7, 0xA90357F6],
    ),
    (
        OFF_REGIST_BLOCK_TO_GRID,
        &[0xD10103FF, 0xA9024FF4, 0xA9037BFD, 0x9100C3FD],
    ),
    (
        OFF_ITEM_SET_LIFE,
        &[0xF81D0FF5, 0xA9014FF4, 0xA9027BFD, 0x910083FD],
    ),
    (
        OFF_ITEM_SET_LIFE_TYPE,
        &[0xF85F8008, 0xF940D108, 0xF940C908, 0xF9411108],
    ),
];

const WORK_MODULE_OFFSET: usize = 0x50;
const STATUS_MODULE_OFFSET: usize = 0x40;
const WORK_GET_FLOAT_SLOT: usize = 0x58;
const WORK_GET_INT_SLOT: usize = 0x98;
const WORK_IS_FLAG_SLOT: usize = 0x108;
const STATUS_CHANGE_REQUEST_SLOT: usize = 0x48;
const BLOCK_PLACED_STATUS_CONST: usize = 0x11cb8;
const COLLISION_READY_FLAG: i32 = 0x2000_0001;
const STATUS_KIND_SLOT: usize = 0x110;
const POSTURE_MODULE_OFFSET: usize = 0x38;
const POSTURE_POS_2D_SLOT: usize = 0x68;
const LOWEST_PLAUSIBLE_POINTER: usize = 0x1_0000;

const COLLISION_WORK_FLOATS: [i32; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
const WORK_INT_MATERIAL: i32 = 0x1000_0001;
const WORK_INT_COLLISION: i32 = 0x1100_000A;

const OFF_UNREGIST_BLOCK_FROM_GRID: usize = 0x166A0F0;
const OFF_UPDATE_COLLISION_WORK: usize = 0x166BBC0;
const OFF_GRID_STATUS: usize = 0x1669E70;
const OFF_ITEM_SET_LIFE: usize = 0x15BF7C0;
const OFF_ITEM_SET_LIFE_TYPE: usize = 0x15BFA40;
const OFF_ID_FROM_GRID_POS: usize = 0x166A170;
const PICKELOBJECT_BASE_KIND: i32 = 0x1AE;
const VANILLA_KIND: i32 = -1;
const SETTLED_FRAMES: u32 = 8;

const WATCH_SLOTS: usize = 16;
const SAMPLE_INTERVAL: u32 = 60;
const MAX_SAMPLE_LINES: u32 = 600;
const MAX_ENTRY_LINES: u32 = 64;

static SAMPLE_LINES: AtomicU32 = AtomicU32::new(0);
static GROUND_LINES: AtomicU32 = AtomicU32::new(0);
static OWNER_LINES: AtomicU32 = AtomicU32::new(0);
static GRID_LINES: AtomicU32 = AtomicU32::new(0);
static KNOWN_BLOCKS: [AtomicUsize; WATCH_SLOTS] = [const { AtomicUsize::new(0) }; WATCH_SLOTS];
static LIFE_LINES: AtomicU32 = AtomicU32::new(0);
static ARMED: AtomicU32 = AtomicU32::new(0);

const LUA_STATE_OWNER_BACK: usize = 8;
const LUA_STATE_OWNER_SLOT: usize = 0x1a0;

struct Watch {
    object: AtomicUsize,
    public_kind: AtomicI32,
    frames: AtomicU32,
    last_status: AtomicI32,
    last_position: AtomicU64,
    still_for: AtomicU32,
    registered_at: AtomicU64,
    registered: AtomicBool,
    replaced: AtomicBool,
}

impl Watch {
    const fn new() -> Self {
        Self {
            object: AtomicUsize::new(0),
            public_kind: AtomicI32::new(0),
            frames: AtomicU32::new(0),
            last_status: AtomicI32::new(i32::MIN),
            last_position: AtomicU64::new(NO_POSITION),
            still_for: AtomicU32::new(0),
            registered_at: AtomicU64::new(NO_POSITION),
            registered: AtomicBool::new(false),
            replaced: AtomicBool::new(false),
        }
    }
}

static WATCHES: [Watch; WATCH_SLOTS] = [const { Watch::new() }; WATCH_SLOTS];

fn plausible(value: usize) -> bool {
    value >= LOWEST_PLAUSIBLE_POINTER && value & 7 == 0
}

unsafe fn read_pointer(at: usize) -> Option<usize> {
    if !plausible(at) {
        return None;
    }
    let value = core::ptr::read_volatile(at as *const usize);
    plausible(value).then_some(value)
}

unsafe fn module_of(boma: usize, offset: usize) -> Option<usize> {
    read_pointer(boma + offset)
}

unsafe fn native_slot(module: usize, slot: usize) -> Option<usize> {
    let vtable = read_pointer(module)?;
    let entry = read_pointer(vtable + slot)?;
    (entry >= crate::text_base_public() && entry < crate::text_end()).then_some(entry)
}

unsafe fn work_float(module: usize, id: i32) -> Option<f32> {
    let entry = native_slot(module, WORK_GET_FLOAT_SLOT)?;
    let native: unsafe extern "C" fn(usize, i32) -> f32 = core::mem::transmute(entry);
    Some(native(module, id))
}

unsafe fn work_int(module: usize, id: i32) -> Option<i32> {
    let entry = native_slot(module, WORK_GET_INT_SLOT)?;
    let native: unsafe extern "C" fn(usize, i32) -> i32 = core::mem::transmute(entry);
    Some(native(module, id))
}

const BLOCK_STATUS_CONSTS: [(usize, &str); 5] = [
    (0x101e4, "sentinel/first"),
    (0x11cb8, "second"),
    (0x11cbc, "third"),
    (0x1e60, "fourth"),
    (0xd9a4, "fifth"),
];

static STATUS_CONSTS_LOGGED: AtomicBool = AtomicBool::new(false);

unsafe fn grid_owner_at(at: &[f32; 2]) -> u32 {
    let native: unsafe extern "C" fn(*const [f32; 2]) -> u32 =
        core::mem::transmute(crate::text_base_public() + OFF_ID_FROM_GRID_POS);
    native(at as *const [f32; 2])
}

unsafe fn request_placed_status(boma: usize) -> Option<i32> {
    let table = crate::item_clones::const_table()?;
    let status = core::ptr::read_volatile((table + BLOCK_PLACED_STATUS_CONST) as *const i32);
    if status <= 0 {
        return None;
    }
    if status_kind_of(boma) == Some(status) {
        return Some(status);
    }
    let module = module_of(boma, STATUS_MODULE_OFFSET)?;
    let entry = native_slot(module, STATUS_CHANGE_REQUEST_SLOT)?;
    let native: unsafe extern "C" fn(usize, i32, bool) -> u64 = core::mem::transmute(entry);
    native(module, status, false);
    Some(status)
}

const GRID_SINGLETON: usize = 0x532E490;

unsafe fn report_grid_state(when: &str) {
    let slot = crate::text_base_public() + GRID_SINGLETON;
    let singleton = core::ptr::read_volatile(slot as *const usize);
    if singleton == 0 {
        crate::dbg_log_public(&format!(
            "[itemwork] grid {when}: singleton [{GRID_SINGLETON:#x}] is NULL - the grid was never constructed"
        ));
        return;
    }
    if !plausible(singleton) {
        crate::dbg_log_public(&format!(
            "[itemwork] grid {when}: singleton {singleton:#x} is not a plausible pointer"
        ));
        return;
    }
    let inner = core::ptr::read_volatile(singleton as *const usize);
    if !plausible(inner) {
        crate::dbg_log_public(&format!(
            "[itemwork] grid {when}: singleton {singleton:#x} holds inner {inner:#x}, unusable - the lookup would fault or bail"
        ));
        return;
    }
    let gate = core::ptr::read_volatile(inner as *const u8);
    crate::dbg_log_public(&format!(
        "[itemwork] grid {when}: singleton {singleton:#x} inner {inner:#x} gate byte = {gate} ({})",
        if gate == 0 {
            "ZERO - every grid lookup bails here"
        } else {
            "non-zero - the grid is live"
        }
    ));
}

unsafe fn report_block_status_constants() {
    if STATUS_CONSTS_LOGGED.swap(true, Ordering::AcqRel) {
        return;
    }
    let Some(table) = crate::item_clones::const_table() else {
        crate::dbg_log_public(
            "[itemwork] the lua2cpp_item const table is not available; block status constants unknown",
        );
        return;
    };
    let mut line = String::new();
    for (offset, label) in BLOCK_STATUS_CONSTS {
        let value = core::ptr::read_volatile((table + offset) as *const i32);
        line.push_str(&format!(" [{offset:#x} {label}]={value}"));
    }
    crate::dbg_log_public(&format!(
        "[itemwork] pickelobject registers these status kinds, from const table {table:#x}:{line}"
    ));
}

unsafe fn work_flag(module: usize, id: i32) -> Option<bool> {
    let entry = native_slot(module, WORK_IS_FLAG_SLOT)?;
    let native: unsafe extern "C" fn(usize, i32) -> u64 = core::mem::transmute(entry);
    Some(native(module, id) & 1 != 0)
}

unsafe fn position_of(boma: usize) -> Option<(f32, f32)> {
    let module = module_of(boma, POSTURE_MODULE_OFFSET)?;
    let entry = native_slot(module, POSTURE_POS_2D_SLOT)?;
    let native: unsafe extern "C" fn(usize) -> f64 = core::mem::transmute(entry);
    let packed = native(module).to_bits();
    Some((
        f32::from_bits(packed as u32),
        f32::from_bits((packed >> 32) as u32),
    ))
}

unsafe fn status_kind_of(boma: usize) -> Option<i32> {
    let module = module_of(boma, STATUS_MODULE_OFFSET)?;
    let entry = native_slot(module, STATUS_KIND_SLOT)?;
    let native: unsafe extern "C" fn(usize) -> i32 = core::mem::transmute(entry);
    Some(native(module))
}

const BOMA_OFFSET_IN_OBJECT: usize = 0x98;
const LUA_MODULE_OFFSET: usize = 0x190;
const LUA_AGENT_OFFSET: usize = 0x220;
const AGENT_ID_SLOT: usize = 0x8;

unsafe fn agent_state(boma: usize) -> String {
    let Some(lua) = module_of(boma, LUA_MODULE_OFFSET) else {
        return "LuaModule unreadable".to_string();
    };
    let Some(agent) = read_pointer(lua + LUA_AGENT_OFFSET) else {
        return format!("LuaModule {lua:#x} holds no agent at +{LUA_AGENT_OFFSET:#x}");
    };
    let nro = crate::item_clones::item_nro_base();
    let id = core::ptr::read_volatile((agent + AGENT_ID_SLOT) as *const u32);
    let vtable = core::ptr::read_volatile(agent as *const usize);
    let origin = if nro != 0 && vtable > nro {
        format!("item NRO +{:#x}", vtable - nro)
    } else if vtable >= crate::text_base_public() && vtable < crate::text_end() {
        format!("main +{:#x}", vtable - crate::text_base_public())
    } else {
        format!("{vtable:#x}")
    };
    format!("lua={lua:#x} agent={agent:#x} vtable={origin} id={id:#x}")
}

unsafe fn describe(boma: usize) -> String {
    let identity = live_identity_of_object(boma).or_else(|| {
        boma.checked_sub(BOMA_OFFSET_IN_OBJECT)
            .and_then(|object| live_identity_of_object(object))
    });
    match identity {
        Some((public, base)) => format!("clone {public:#x} (base {base:#x})"),
        None => format!(
            "an object the live table does not claim (yet), boma={boma:#x} object={:#x}",
            boma.wrapping_sub(BOMA_OFFSET_IN_OBJECT)
        ),
    }
}

unsafe fn dump(boma: usize) -> String {
    let Some(work) = module_of(boma, WORK_MODULE_OFFSET) else {
        return format!("WorkModule unreadable at boma+{WORK_MODULE_OFFSET:#x}");
    };
    if native_slot(work, WORK_GET_FLOAT_SLOT).is_none() {
        return format!("WorkModule {work:#x} has no usable get_float slot");
    }
    let mut floats = String::new();
    for id in COLLISION_WORK_FLOATS {
        match work_float(work, id) {
            Some(value) => floats.push_str(&format!(" f{id}={value:.3}")),
            None => floats.push_str(&format!(" f{id}=?")),
        }
    }
    let material = match work_int(work, WORK_INT_MATERIAL) {
        Some(value) => format!("{value}"),
        None => "?".to_string(),
    };
    let ready = match work_flag(work, COLLISION_READY_FLAG) {
        Some(true) => "on",
        Some(false) => "OFF",
        None => "?",
    };
    let position = match position_of(boma) {
        Some((x, y)) => format!("({x:.3}, {y:.3})"),
        None => "unreadable".to_string(),
    };
    let collision = match work_int(work, WORK_INT_COLLISION) {
        Some(value) => format!("{value}"),
        None => "?".to_string(),
    };
    let status = match status_kind_of(boma) {
        Some(value) => format!("{value}"),
        None => "?".to_string(),
    };
    format!(
        "status={status} pos={position} flag0x20000001={ready} material(int 0x10000001)={material} int0x1100000a={collision} rhombus{floats}"
    )
}

struct Registration {
    object: AtomicUsize,
    position: AtomicU64,
}

impl Registration {
    const fn new() -> Self {
        Self {
            object: AtomicUsize::new(0),
            position: AtomicU64::new(NO_POSITION),
        }
    }
}

static REGISTRATIONS: [Registration; WATCH_SLOTS] = [const { Registration::new() }; WATCH_SLOTS];

fn pack(x: f32, y: f32) -> u64 {
    ((y.to_bits() as u64) << 32) | x.to_bits() as u64
}

fn unpack(value: u64) -> [f32; 2] {
    [f32::from_bits(value as u32), f32::from_bits((value >> 32) as u32)]
}

fn note_registration(object: usize, x: f32, y: f32) {
    for slot in REGISTRATIONS.iter() {
        if slot.object.load(Ordering::Acquire) == object {
            slot.position.store(pack(x, y), Ordering::Release);
            return;
        }
    }
    for slot in REGISTRATIONS.iter() {
        if slot
            .object
            .compare_exchange(0, object, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            slot.position.store(pack(x, y), Ordering::Release);
            return;
        }
    }
}

fn take_registration(object: usize) -> Option<[f32; 2]> {
    for slot in REGISTRATIONS.iter() {
        if slot.object.load(Ordering::Acquire) == object {
            let value = slot.position.swap(NO_POSITION, Ordering::AcqRel);
            slot.object.store(0, Ordering::Release);
            if value != NO_POSITION {
                return Some(unpack(value));
            }
        }
    }
    None
}

unsafe fn replace_stale_registration(
    watch: &Watch,
    boma: usize,
    object: usize,
    public_kind: i32,
    settled: [f32; 2],
) {
    if watch.replaced.swap(true, Ordering::AcqRel) {
        return;
    }
    let base = crate::text_base_public();
    let create: unsafe extern "C" fn(usize) -> u64 =
        core::mem::transmute(base + OFF_CREATE_GROUND_COLLISION);
    let regist: unsafe extern "C" fn(usize, *const [f32; 2]) =
        core::mem::transmute(base + OFF_REGIST_BLOCK_TO_GRID);
    let unregist: unsafe extern "C" fn(*const [f32; 2]) =
        core::mem::transmute(base + OFF_UNREGIST_BLOCK_FROM_GRID);

    let update_collision: unsafe extern "C" fn(usize) -> u64 =
        core::mem::transmute(base + OFF_UPDATE_COLLISION_WORK);

    report_grid_state("at settle");
    let stale = take_registration(object);
    if let Some(stale) = stale {
        unregist(&stale as *const [f32; 2]);
    }
    update_collision(boma);
    let built = create(boma) & 1;
    if built != 0 {
        regist(boma, &settled as *const [f32; 2]);
    }
    let seeded = grid_owner_at(&settled);
    let requested = request_placed_status(boma);
    let grid_status: unsafe extern "C" fn(*const [f32; 2]) -> i32 =
        core::mem::transmute(base + OFF_GRID_STATUS);
    let id_from_grid: unsafe extern "C" fn(*const [f32; 2]) -> u32 =
        core::mem::transmute(base + OFF_ID_FROM_GRID_POS);
    let cell_status = grid_status(&settled as *const [f32; 2]);
    let cell_owner = id_from_grid(&settled as *const [f32; 2]);
    let own_id = core::ptr::read_volatile((object + AGENT_ID_SLOT) as *const u32);
    crate::dbg_log_public(&format!(
        "[itemwork]   after the full sequence: {}",
        dump(boma)
    ));
    crate::dbg_log_public(&format!(
        "[itemwork]   read-back at the settled cell: grid_status={cell_status} owner_id={cell_owner:#x} our_id={own_id:#x} {}",
        if cell_owner == own_id {
            "MATCH - the grid now points at this block"
        } else {
            "MISMATCH - the grid does not point at this block"
        }
    ));
    crate::dbg_log_public(&format!(
        "[itemwork] clone {public_kind:#x} object={object:#x} settled at ({:.3}, {:.3}); {}",
        settled[0],
        settled[1],
        match requested {
            Some(status) => format!(
                "seeded the grid cell (owner now {seeded:#x}, create returned {built}), then requested placed status {status}"
            ),
            None => format!(
                "could NOT resolve the placed status, fell back to the hand rebuild (create returned {built}, stale cell {})",
                match stale {
                    Some(at) => format!("({:.3}, {:.3})", at[0], at[1]),
                    None => "none recorded".to_string(),
                }
            ),
        }
    ));
}

fn reset_watch(watch: &Watch, public_kind: i32) {
    watch.public_kind.store(public_kind, Ordering::Relaxed);
    watch.frames.store(0, Ordering::Relaxed);
    watch.last_status.store(i32::MIN, Ordering::Relaxed);
    watch.last_position.store(NO_POSITION, Ordering::Relaxed);
    watch.still_for.store(0, Ordering::Relaxed);
    watch.registered_at.store(NO_POSITION, Ordering::Relaxed);
    watch.registered.store(false, Ordering::Relaxed);
    watch.replaced.store(false, Ordering::Relaxed);
}

fn watch_for(object: usize, public_kind: i32) -> Option<&'static Watch> {
    for watch in WATCHES.iter() {
        if watch.object.load(Ordering::Acquire) == object
            && watch.public_kind.load(Ordering::Relaxed) == public_kind
        {
            return Some(watch);
        }
    }
    for watch in WATCHES.iter() {
        if watch
            .object
            .compare_exchange(0, object, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            reset_watch(watch, public_kind);
            return Some(watch);
        }
    }
    for watch in WATCHES.iter() {
        if watch.object.load(Ordering::Acquire) == object {
            reset_watch(watch, public_kind);
            return Some(watch);
        }
    }
    None
}

fn note_block(object: usize) {
    for slot in KNOWN_BLOCKS.iter() {
        if slot.load(Ordering::Acquire) == object {
            return;
        }
    }
    for slot in KNOWN_BLOCKS.iter() {
        if slot
            .compare_exchange(0, object, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            return;
        }
    }
}

fn is_known_block(object: usize) -> bool {
    KNOWN_BLOCKS
        .iter()
        .any(|slot| slot.load(Ordering::Acquire) == object)
}

pub(crate) unsafe fn observe_vanilla(object: usize) {
    if ARMED.load(Ordering::Acquire) == 0 || !is_known_block(object) {
        return;
    }
    observe(object, VANILLA_KIND, VANILLA_KIND);
}

pub(crate) unsafe fn observe(object: usize, public_kind: i32, base_kind: i32) {
    if ARMED.load(Ordering::Acquire) == 0 {
        return;
    }
    let Some(boma) = read_pointer(object + crate::item_clones::BATTLE_OBJECT_MODULE_TABLE) else {
        static MISSED: AtomicU32 = AtomicU32::new(0);
        if MISSED.fetch_add(1, Ordering::Relaxed) == 0 {
            crate::dbg_log_public(&format!(
                "[itemwork] object {object:#x} (clone {public_kind:#x}) has no readable module table; sampler skipped"
            ));
        }
        return;
    };
    let Some(watch) = watch_for(object, public_kind) else {
        static FULL: AtomicU32 = AtomicU32::new(0);
        if FULL.fetch_add(1, Ordering::Relaxed) == 0 {
            crate::dbg_log_public(&format!(
                "[itemwork] no free watch slot for object {object:#x}; sampler skipped"
            ));
        }
        return;
    };
    let frame = watch.frames.fetch_add(1, Ordering::Relaxed);
    if base_kind == PICKELOBJECT_BASE_KIND {
        if let Some((x, y)) = position_of(boma) {
            let here = pack(x, y);
            let previous = watch.last_position.swap(here, Ordering::Relaxed);
            let still = if previous == here {
                watch.still_for.fetch_add(1, Ordering::Relaxed) + 1
            } else {
                watch.still_for.store(0, Ordering::Relaxed);
                0
            };
            if here == pack(0.0, 0.0) {
                if watch.replaced.swap(false, Ordering::AcqRel) {
                    watch.still_for.store(0, Ordering::Relaxed);
                    crate::dbg_log_public(&format!(
                        "[itemwork] clone {public_kind:#x} object={object:#x} is back at the origin; a new block is using this slot, re-arming the settle fix"
                    ));
                }
            } else if still >= SETTLED_FRAMES {
                replace_stale_registration(watch, boma, object, public_kind, [x, y]);
            }
        }
    }
    let status = status_kind_of(boma).unwrap_or(i32::MIN);
    let previous = watch.last_status.swap(status, Ordering::Relaxed);
    let changed = status != previous;
    if !changed && frame % SAMPLE_INTERVAL != 0 {
        return;
    }
    if SAMPLE_LINES.fetch_add(1, Ordering::Relaxed) >= MAX_SAMPLE_LINES {
        return;
    }
    let note = if changed && previous != i32::MIN {
        format!(" STATUS CHANGED from {previous}")
    } else {
        String::new()
    };
    let who = if public_kind == VANILLA_KIND {
        "VANILLA block".to_string()
    } else {
        format!("clone {public_kind:#x} (base {base_kind:#x})")
    };
    crate::dbg_log_public(&format!(
        "[itemwork] frame {frame} {who} object={object:#x} {}{note}",
        dump(boma)
    ));
}

#[skyline::hook(offset = OFF_CREATE_GROUND_COLLISION)]
unsafe fn create_ground_collision_probe(boma: u64) -> u64 {
    note_block((boma as usize).wrapping_sub(BOMA_OFFSET_IN_OBJECT));
    report_block_status_constants();
    report_grid_state("at the first block's collision build");
    let report = GROUND_LINES.fetch_add(1, Ordering::Relaxed) < MAX_ENTRY_LINES;
    if report {
        let boma = boma as usize;
        crate::dbg_log_public(&format!(
            "[itemwork] create_ground_collision ENTER for {} {}",
            describe(boma),
            dump(boma)
        ));
        crate::dbg_log_public(&format!("[itemwork]   before: {}", agent_state(boma)));
    }
    let result = call_original!(boma);
    if report {
        crate::dbg_log_public(&format!(
            "[itemwork]   after:  {}",
            agent_state(boma as usize)
        ));
    }
    result
}

#[skyline::hook(offset = OFF_CHECK_RANGE_FROM_OWNER)]
unsafe fn check_range_from_owner_probe(boma: u64) -> u64 {
    let result = call_original!(boma);
    if OWNER_LINES.fetch_add(1, Ordering::Relaxed) < MAX_ENTRY_LINES {
        crate::dbg_log_public(&format!(
            "[itemwork] check_range_from_onwer for {} returned {result:#x}",
            describe(boma as usize)
        ));
    }
    result
}

#[skyline::hook(offset = OFF_REGIST_BLOCK_TO_GRID)]
unsafe fn regist_block_to_grid_probe(boma: u64, position: u64) -> u64 {
    if GRID_LINES.fetch_add(1, Ordering::Relaxed) < MAX_ENTRY_LINES {
        let coordinates = if plausible(position as usize) {
            let x = core::ptr::read_volatile(position as *const f32);
            let y = core::ptr::read_volatile((position as usize + 4) as *const f32);
            format!("({x:.3}, {y:.3})")
        } else {
            format!("unreadable position {position:#x}")
        };
        crate::dbg_log_public(&format!(
            "[itemwork] regist_block_to_grid for {} at {coordinates}",
            describe(boma as usize)
        ));
    }
    if plausible(position as usize) {
        let x = core::ptr::read_volatile(position as *const f32);
        let y = core::ptr::read_volatile((position as usize + 4) as *const f32);
        note_registration((boma as usize).wrapping_sub(BOMA_OFFSET_IN_OBJECT), x, y);
    }
    call_original!(boma, position)
}

unsafe fn object_of_lua_state(lua_state: usize) -> Option<(usize, usize)> {
    if lua_state < LUA_STATE_OWNER_BACK {
        return None;
    }
    let owner = read_pointer(lua_state - LUA_STATE_OWNER_BACK)?;
    let boma = read_pointer(owner + LUA_STATE_OWNER_SLOT)?;
    let object = read_pointer(module_of(boma, LUA_MODULE_OFFSET)? + LUA_AGENT_OFFSET)?;
    Some((boma, object))
}

#[skyline::hook(offset = OFF_ITEM_SET_LIFE)]
unsafe fn item_set_life_probe(lua_state: u64, life: i32) -> u64 {
    if LIFE_LINES.fetch_add(1, Ordering::Relaxed) < MAX_ENTRY_LINES {
        match object_of_lua_state(lua_state as usize) {
            Some((boma, _object)) => crate::dbg_log_public(&format!(
                "[itemwork] item::set_life({life}) for {}{}",
                describe(boma),
                if life < 0 {
                    " - NEGATIVE, the game substitutes its 900 frame default"
                } else {
                    ""
                }
            )),
            None => crate::dbg_log_public(&format!(
                "[itemwork] item::set_life({life}) for an unresolvable lua_State {lua_state:#x}"
            )),
        }
    }
    call_original!(lua_state, life)
}

#[skyline::hook(offset = OFF_ITEM_SET_LIFE_TYPE)]
unsafe fn item_set_life_type_probe(lua_state: u64, kind: i32) -> u64 {
    if LIFE_LINES.fetch_add(1, Ordering::Relaxed) < MAX_ENTRY_LINES {
        match object_of_lua_state(lua_state as usize) {
            Some((boma, _object)) => crate::dbg_log_public(&format!(
                "[itemwork] item::set_life_type({kind}) for {}",
                describe(boma)
            )),
            None => crate::dbg_log_public(&format!(
                "[itemwork] item::set_life_type({kind}) for an unresolvable lua_State {lua_state:#x}"
            )),
        }
    }
    call_original!(lua_state, kind)
}

pub(crate) fn install() {
    unsafe {
        for (offset, expected) in PREFLIGHT {
            let address = crate::text_base_public() + offset;
            for (index, word) in expected.iter().enumerate() {
                let found = core::ptr::read_volatile((address + index * 4) as *const u32);
                if found != *word {
                    crate::dbg_log_public(&format!(
                        "[itemwork] preflight FAILED at {offset:#x} word {index}: found={found:#010x} expected={word:#010x}; probe inert"
                    ));
                    return;
                }
            }
        }
        skyline::install_hooks!(
            create_ground_collision_probe,
            check_range_from_owner_probe,
            regist_block_to_grid_probe,
            item_set_life_probe,
            item_set_life_type_probe
        );
    }
    ARMED.store(1, Ordering::Release);
    crate::dbg_log_public(
        "[itemwork] armed: per-frame work sampler plus create_ground_collision, check_range_from_onwer, regist_block_to_grid, item::set_life and item::set_life_type entries",
    );
}
