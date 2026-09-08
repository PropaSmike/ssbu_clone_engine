use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

const GRID_SINGLETON: usize = 0x532_E490;
const CONTAINER_OFFSET: usize = 0x68160;
const CONTAINER_FRAME_COUNTER: usize = 0xb0;

const SCAN_MISS_SITES: [usize; 2] = [0x260_E644, 0x260_E654];
const MOV_W8_WZR: u32 = 0x2A1F_03E8;
const MOV_W8_ONE: u32 = 0x5280_0028;

const ROSTER_GRACE_FRAMES: u32 = 120;
const GRID_BLACKOUT_FRAMES: u32 = 30;

const PICKELOBJECT_BASE_KIND: i32 = 0x1AE;
const LOWEST_PLAUSIBLE_POINTER: usize = 0x1_0000;

const BLOCK_PLACED_STATUS_CONST: usize = 0x11cb8;
const STATUS_MODULE_OFFSET: usize = 0x40;
const STATUS_KIND_SLOT: usize = 0x110;
const STATUS_CHANGE_REQUEST_SLOT: usize = 0x48;
const POSTURE_MODULE_OFFSET: usize = 0x38;
const POSTURE_POS_2D_SLOT: usize = 0x68;
const SETTLED_FRAMES: u32 = 8;
const SETTLE_SLOTS: usize = 16;
const NO_POSITION: u64 = u64::MAX;

static NEEDED: AtomicBool = AtomicBool::new(false);
static EPOCH: AtomicUsize = AtomicUsize::new(0);
static STEVE_SEEN: AtomicBool = AtomicBool::new(false);
static PATCHED: AtomicBool = AtomicBool::new(false);
static PATCHED_FRAME: AtomicU32 = AtomicU32::new(0);
static REFUSED: AtomicBool = AtomicBool::new(false);
static BLACKOUT_LOGGED: AtomicBool = AtomicBool::new(false);

fn log(message: String) {
    crate::dbg_log_public(&message);
}

pub(crate) fn note_registered_base_kind(base_kind: i32) {
    if base_kind != PICKELOBJECT_BASE_KIND {
        return;
    }
    if !NEEDED.swap(true, Ordering::AcqRel) {
        log(format!(
            "[blockgrid] a clone of pickelobject ({PICKELOBJECT_BASE_KIND:#x}) is registered; \
             the block grid will be opened for matches with no pickel on the roster"
        ));
    }
}

unsafe fn manager_inner() -> Option<usize> {
    let holder =
        core::ptr::read_volatile((crate::text_base_public() + GRID_SINGLETON) as *const usize);
    if holder < LOWEST_PLAUSIBLE_POINTER {
        return None;
    }
    let inner = core::ptr::read_volatile(holder as *const usize);
    (inner >= LOWEST_PLAUSIBLE_POINTER).then_some(inner)
}

pub(crate) unsafe fn sample() {
    if !NEEDED.load(Ordering::Relaxed) {
        return;
    }
    let Some(inner) = manager_inner() else {
        return;
    };
    if EPOCH.swap(inner, Ordering::AcqRel) != inner {
        STEVE_SEEN.store(false, Ordering::Release);
        restore("a new match started");
    }
    if PATCHED.load(Ordering::Acquire) || REFUSED.load(Ordering::Acquire) {
        return;
    }
    if core::ptr::read_volatile(inner as *const u8) != 0 {
        STEVE_SEEN.store(true, Ordering::Release);
        return;
    }
    if STEVE_SEEN.load(Ordering::Acquire) {
        return;
    }
    let frames = core::ptr::read_volatile(
        (inner + CONTAINER_OFFSET + CONTAINER_FRAME_COUNTER) as *const u32,
    );
    if frames < ROSTER_GRACE_FRAMES {
        return;
    }
    apply(frames);
}

unsafe fn apply(frames: u32) {
    let base = crate::text_base_public();
    for site in SCAN_MISS_SITES {
        let found = core::ptr::read_volatile((base + site) as *const u32);
        if found != MOV_W8_WZR {
            REFUSED.store(true, Ordering::Release);
            log(format!(
                "[blockgrid] REFUSED: {site:#x} reads {found:#010x}, expected {MOV_W8_WZR:#010x} \
                 (mov w8, wzr); the pickel scan is not where this build thinks it is, so the \
                 grid stays closed"
            ));
            return;
        }
    }
    let mut written = 0usize;
    for site in SCAN_MISS_SITES {
        if !crate::text_patch::write_word(base + site, MOV_W8_ONE) {
            REFUSED.store(true, Ordering::Release);
            log(format!(
                "[blockgrid] REFUSED: the write to {site:#x} failed; rolling back {written} site(s)"
            ));
            for done in SCAN_MISS_SITES.iter().take(written) {
                crate::text_patch::write_word(base + done, MOV_W8_WZR);
            }
            return;
        }
        written += 1;
    }
    let mut live = 0usize;
    for site in SCAN_MISS_SITES {
        if core::ptr::read_volatile((base + site) as *const u32) == MOV_W8_ONE {
            live += 1;
        }
    }
    if live != SCAN_MISS_SITES.len() {
        for site in SCAN_MISS_SITES {
            let found = core::ptr::read_volatile((base + site) as *const u32);
            if found != MOV_W8_ONE {
                log(format!(
                    "[blockgrid] {site:#x} still holds {found:#010x}, wanted {MOV_W8_ONE:#010x}"
                ));
            }
            crate::text_patch::write_word(base + site, MOV_W8_WZR);
        }
        REFUSED.store(true, Ordering::Release);
        log(format!(
            "[blockgrid] REFUSED: {live} of {} site(s) verified live by read-back; the writes were rolled back and the grid stays closed",
            SCAN_MISS_SITES.len()
        ));
        return;
    }
    PATCHED_FRAME.store(frames, Ordering::Release);
    PATCHED.store(true, Ordering::Release);
    log(format!(
        "[blockgrid] no pickel on the roster after {frames} manager frames; the scan at \
         {:#x}/{:#x} now always reports present, so the grid stops being wiped and its 30 frame \
         blackout drains",
        SCAN_MISS_SITES[0], SCAN_MISS_SITES[1]
    ));
    log(format!(
        "[blockgrid] {live} of {} patched site(s) verified live by read-back",
        SCAN_MISS_SITES.len()
    ));
}

unsafe fn restore(why: &str) {
    if !PATCHED.swap(false, Ordering::AcqRel) {
        return;
    }
    let base = crate::text_base_public();
    for site in SCAN_MISS_SITES {
        crate::text_patch::write_word(base + site, MOV_W8_WZR);
    }
    log(format!(
        "[blockgrid] the pickel scan is the game's own again ({why})"
    ));
}

pub(crate) fn install() {
    let base = crate::text_base_public();
    for site in SCAN_MISS_SITES {
        let found = unsafe { core::ptr::read_volatile((base + site) as *const u32) };
        if found != MOV_W8_WZR {
            REFUSED.store(true, Ordering::Release);
            log(format!(
                "[blockgrid] preflight FAILED: {site:#x} reads {found:#010x}, expected \
                 {MOV_W8_WZR:#010x}; clone blocks will need a pickel in the match"
            ));
            return;
        }
    }
}


struct Settle {
    object: AtomicUsize,
    position: AtomicU64,
    still: AtomicU32,
    placed: AtomicBool,
}

impl Settle {
    const fn new() -> Self {
        Self {
            object: AtomicUsize::new(0),
            position: AtomicU64::new(NO_POSITION),
            still: AtomicU32::new(0),
            placed: AtomicBool::new(false),
        }
    }
}

static SETTLES: [Settle; SETTLE_SLOTS] = [const { Settle::new() }; SETTLE_SLOTS];
static PLACED_LOGGED: AtomicBool = AtomicBool::new(false);
static PLACED_REFUSED: AtomicBool = AtomicBool::new(false);

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

unsafe fn status_kind_of(boma: usize) -> Option<i32> {
    let module = module_of(boma, STATUS_MODULE_OFFSET)?;
    let entry = native_slot(module, STATUS_KIND_SLOT)?;
    let native: unsafe extern "C" fn(usize) -> i32 = core::mem::transmute(entry);
    Some(native(module))
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

fn pack(x: f32, y: f32) -> u64 {
    u64::from(x.to_bits()) | (u64::from(y.to_bits()) << 32)
}

fn settle_for(object: usize) -> Option<&'static Settle> {
    for slot in SETTLES.iter() {
        if slot.object.load(Ordering::Acquire) == object {
            return Some(slot);
        }
    }
    for slot in SETTLES.iter() {
        if slot
            .object
            .compare_exchange(0, object, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            slot.position.store(NO_POSITION, Ordering::Release);
            slot.still.store(0, Ordering::Relaxed);
            slot.placed.store(false, Ordering::Release);
            return Some(slot);
        }
    }
    None
}

unsafe fn grid_accepts_placement() -> bool {
    if STEVE_SEEN.load(Ordering::Acquire) {
        return true;
    }
    if !PATCHED.load(Ordering::Acquire) {
        return false;
    }
    let Some(inner) = manager_inner() else {
        return false;
    };
    let frames = core::ptr::read_volatile(
        (inner + CONTAINER_OFFSET + CONTAINER_FRAME_COUNTER) as *const u32,
    );
    frames >= PATCHED_FRAME
        .load(Ordering::Acquire)
        .saturating_add(GRID_BLACKOUT_FRAMES)
}

pub(crate) unsafe fn note_live_block(object: usize, base_kind: i32) {
    if base_kind != PICKELOBJECT_BASE_KIND || PLACED_REFUSED.load(Ordering::Relaxed) {
        return;
    }
    let Some(boma) = read_pointer(object + crate::item_clones::BATTLE_OBJECT_MODULE_TABLE) else {
        return;
    };
    let Some(slot) = settle_for(object) else {
        return;
    };
    let Some((x, y)) = position_of(boma) else {
        return;
    };
    let here = pack(x, y);
    if here == pack(0.0, 0.0) {
        slot.position.store(here, Ordering::Release);
        slot.still.store(0, Ordering::Relaxed);
        slot.placed.store(false, Ordering::Release);
        return;
    }
    let still = if slot.position.swap(here, Ordering::AcqRel) == here {
        slot.still.fetch_add(1, Ordering::Relaxed) + 1
    } else {
        slot.still.store(0, Ordering::Relaxed);
        0
    };
    if still < SETTLED_FRAMES {
        return;
    }
    if !grid_accepts_placement() {
        if !BLACKOUT_LOGGED.swap(true, Ordering::AcqRel) {
            log(format!(
                "[blockgrid] a clone block settled while the grid was still closed; holding its                  placed-status request until the scan patch is live and its {GRID_BLACKOUT_FRAMES}                  frame blackout has drained"
            ));
        }
        slot.still.store(SETTLED_FRAMES, Ordering::Relaxed);
        return;
    }
    if slot.placed.swap(true, Ordering::AcqRel) {
        return;
    }
    match request_placed_status(boma) {
        Some(status) => {
            if !PLACED_LOGGED.swap(true, Ordering::AcqRel) {
                log(format!(
                    "[blockgrid] settled clone block requested placed status {status}; the game's script now owns its collision, grid cell and life"
                ));
            }
        }
        None => {
            PLACED_REFUSED.store(true, Ordering::Release);
            log("[blockgrid] REFUSED: the placed status could not be resolved; clone blocks will settle without collision".to_string());
        }
    }
}
