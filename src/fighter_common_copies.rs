use super::*;

use clone_engine_core::fighter_common_copies::{
    self as copies, Copy, ACCESSOR_PARAM_OBJECT, COPIES, PARAM_OBJECT_POWER_UP_CLONE,
    PARAM_OBJECT_SIZE, POWER_UP,
};
use clone_engine_core::fighter_param_row::{Op, Reject, RowOverride};
use clone_engine_core::slots::{clone_row, CLONE_SLOTS};
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicI32, AtomicU32, AtomicUsize, Ordering};
use std::sync::RwLock;

const OFF_BINDER_COMMON: usize = 0x736a90;
const OFF_BINDER_ITEM: usize = 0x782c00;
const OFF_BINDER_ETC: usize = 0x7741f0;
const OFF_BINDER_POWER_UP: usize = 0x798540;
const OFF_BINDER_EFFECT: usize = 0x761f70;
const OFF_PARAM_BIND_KIND: usize = 0x77d100;
const WRAPPER_PARAM_OBJECT: usize = 0x8;
const POWER_UP_SIZE: usize = 0x2f0;
const ENTRIES: usize = 8;
const LOWEST_PLAUSIBLE_POINTER: usize = 0x1_0000;
const LOG_LIMIT: u32 = 48;

struct Snapshot {
    bytes: UnsafeCell<[u8; PARAM_OBJECT_SIZE]>,
}

unsafe impl Sync for Snapshot {}

impl Snapshot {
    const fn new() -> Self {
        Self {
            bytes: UnsafeCell::new([0; PARAM_OBJECT_SIZE]),
        }
    }
}

struct PowerUpSnapshot {
    bytes: UnsafeCell<[u8; POWER_UP_SIZE]>,
}

unsafe impl Sync for PowerUpSnapshot {}

impl PowerUpSnapshot {
    const fn new() -> Self {
        Self {
            bytes: UnsafeCell::new([0; POWER_UP_SIZE]),
        }
    }
}

struct Registration {
    param_object: AtomicUsize,
    accessor: AtomicUsize,
    clone_kind: AtomicI32,
    applied: AtomicU32,
    power_up_clone: AtomicUsize,
    snapshot: Snapshot,
    power_up_snapshot: PowerUpSnapshot,
}

impl Registration {
    const fn new() -> Self {
        Self {
            param_object: AtomicUsize::new(0),
            accessor: AtomicUsize::new(0),
            clone_kind: AtomicI32::new(0),
            applied: AtomicU32::new(0),
            power_up_clone: AtomicUsize::new(0),
            snapshot: Snapshot::new(),
            power_up_snapshot: PowerUpSnapshot::new(),
        }
    }

    fn clear(&self) {
        self.param_object.store(0, Ordering::Release);
        self.accessor.store(0, Ordering::Release);
        self.clone_kind.store(0, Ordering::Release);
        self.applied.store(0, Ordering::Release);
        self.power_up_clone.store(0, Ordering::Release);
    }
}

static REGISTRY: [Registration; ENTRIES] = [const { Registration::new() }; ENTRIES];
static OVERRIDES: RwLock<Vec<Vec<RowOverride>>> = RwLock::new(Vec::new());
static COUNTS: [AtomicU32; CLONE_SLOTS] = [const { AtomicU32::new(0) }; CLONE_SLOTS];
static LOG: AtomicU32 = AtomicU32::new(0);

fn log(message: &str) {
    if LOG.fetch_add(1, Ordering::Relaxed) < LOG_LIMIT {
        dbg_log_public(&format!("[paramcopy] {message}"));
    }
}

fn plausible(value: usize) -> bool {
    value >= LOWEST_PLAUSIBLE_POINTER && value & 7 == 0
}

pub(crate) struct Recorded {
    pub(crate) message: String,
    pub(crate) recorded: bool,
}

pub(crate) fn record(kind: i32, slot: i32, param_hash: u64, op: Op) -> Recorded {
    let refused = |message: String| Recorded {
        message,
        recorded: false,
    };
    let Some(index) = clone_row(kind) else {
        return refused(String::from("copy: kind outside the clone range"));
    };
    let (item, field, copy) = match copies::resolve(param_hash, op) {
        Ok(resolved) => resolved,
        Err(Reject::UnknownField) => {
            return refused(String::from(
                "copy: not a field of the six per-fighter tables, ParamConfig only",
            ))
        }
        Err(Reject::Unsupported) => {
            return refused(String::from(
                "copy: field type cannot take this op, ParamConfig only",
            ))
        }
        Err(Reject::NotFinite) => return refused(String::from("copy: value is not finite")),
        Err(Reject::Immutable | Reject::OtherTable(_)) => {
            return refused(String::from("copy: not overridable"))
        }
    };
    if slot != param_overrides::ANY_SLOT {
        return refused(format!(
            "copy: {} is slot-specific, ParamConfig only (one copy per fighter)",
            field.name
        ));
    }
    {
        let mut held = OVERRIDES.write().unwrap();
        if held.len() < CLONE_SLOTS {
            held.resize_with(CLONE_SLOTS, Vec::new);
        }
        held[index].push(item);
        COUNTS[index].store(held[index].len() as u32, Ordering::Release);
    }
    Recorded {
        message: format!(
            "copy: {} {} at +{:#x} of the fighter's param object queued",
            copy.file,
            field.name,
            field.offset
        ),
        recorded: true,
    }
}

fn has_overrides(kind: i32) -> bool {
    clone_row(kind).is_some_and(|index| COUNTS[index].load(Ordering::Acquire) != 0)
}

unsafe fn apply(registration: &Registration, base: usize, end: usize, why: &str) {
    let param_object = registration.param_object.load(Ordering::Acquire);
    let clone_kind = registration.clone_kind.load(Ordering::Acquire);
    let Some(index) = clone_row(clone_kind) else {
        return;
    };
    if !plausible(param_object) {
        return;
    }
    let object = core::slice::from_raw_parts_mut(param_object as *mut u8, PARAM_OBJECT_SIZE);
    let vanilla = &mut *registration.snapshot.bytes.get();
    vanilla[base..end].copy_from_slice(&object[base..end]);
    let held = OVERRIDES.read().unwrap();
    let overrides: &[RowOverride] = held.get(index).map(Vec::as_slice).unwrap_or(&[]);
    let applied = copies::apply_range(object, vanilla, overrides, base, end);
    let n = registration.applied.fetch_add(1, Ordering::Relaxed);
    if n < 12 {
        log(&format!(
            "clone {clone_kind} param object {param_object:#x} {why}: {applied} override(s) applied in +{base:#x}..+{end:#x}"
        ));
    }
}

unsafe fn apply_power_up_clone(registration: &Registration, why: &str) {
    let param_object = registration.param_object.load(Ordering::Acquire);
    let clone_kind = registration.clone_kind.load(Ordering::Acquire);
    let Some(index) = clone_row(clone_kind) else {
        return;
    };
    if !plausible(param_object) {
        return;
    }
    let detached =
        core::ptr::read_volatile((param_object + PARAM_OBJECT_POWER_UP_CLONE) as *const usize);
    if !plausible(detached) {
        log(&format!(
            "clone {clone_kind} param object {param_object:#x} {why}: no power_up object at +{PARAM_OBJECT_POWER_UP_CLONE:#x}, power_up.prc keeps vanilla"
        ));
        return;
    }
    registration.power_up_clone.store(detached, Ordering::Release);
    let object = core::slice::from_raw_parts_mut(detached as *mut u8, POWER_UP_SIZE);
    let vanilla = &mut *registration.power_up_snapshot.bytes.get();
    vanilla.copy_from_slice(object);
    let held = OVERRIDES.read().unwrap();
    let overrides: &[RowOverride] = held.get(index).map(Vec::as_slice).unwrap_or(&[]);
    let applied = copies::apply_detached(object, vanilla, overrides, &COPIES[POWER_UP]);
    if applied != 0 {
        log(&format!(
            "clone {clone_kind} power_up object {detached:#x} {why}: {applied} override(s) applied"
        ));
    }
}

pub(crate) unsafe fn on_accessor_init(accessor: u64, entry_id: i32, clone_kind: Option<i32>) {
    let Ok(entry) = usize::try_from(entry_id) else {
        return;
    };
    let Some(registration) = REGISTRY.get(entry) else {
        return;
    };
    registration.clear();
    let Some(clone_kind) = clone_kind else {
        return;
    };
    if !has_overrides(clone_kind) || accessor == 0 {
        return;
    }
    let param_object =
        core::ptr::read_volatile((accessor as usize + ACCESSOR_PARAM_OBJECT) as *const usize);
    if !plausible(param_object) {
        log(&format!(
            "entry {entry} clone {clone_kind}: no param object at accessor+{ACCESSOR_PARAM_OBJECT:#x}, copies keep vanilla"
        ));
        return;
    }
    registration.param_object.store(param_object, Ordering::Release);
    registration.accessor.store(accessor as usize, Ordering::Release);
    registration.clone_kind.store(clone_kind, Ordering::Release);
    apply(registration, 0, PARAM_OBJECT_SIZE, "after accessor init");
    apply_power_up_clone(registration, "after accessor init");
}

#[skyline::hook(offset = OFF_PARAM_BIND_KIND)]
unsafe fn param_bind_kind_bridge(wrapper: u64, kind: i32) -> u64 {
    let result = call_original!(wrapper, kind);
    if wrapper != 0 {
        let param_object =
            core::ptr::read_volatile((wrapper as usize + WRAPPER_PARAM_OBJECT) as *const usize);
        for registration in REGISTRY.iter() {
            if registration.param_object.load(Ordering::Acquire) == param_object && param_object != 0 {
                apply_power_up_clone(registration, "after kind bind");
                break;
            }
        }
    }
    result
}

unsafe fn after_bind(object: usize, copy: &Copy) {
    if !plausible(object) || object < copy.base {
        return;
    }
    let param_object = object - copy.base;
    for registration in REGISTRY.iter() {
        if registration.param_object.load(Ordering::Acquire) != param_object {
            continue;
        }
        let accessor = registration.accessor.load(Ordering::Acquire);
        if !plausible(accessor)
            || core::ptr::read_volatile((accessor + ACCESSOR_PARAM_OBJECT) as *const usize)
                != param_object
        {
            registration.clear();
            return;
        }
        apply(registration, copy.base, copy.end(), copy.name);
        return;
    }
}

macro_rules! binder_bridge {
    ($name:ident, $offset:ident, $index:expr) => {
        #[skyline::hook(offset = $offset)]
        unsafe fn $name(object: u64, tree: u64) -> u64 {
            let result = call_original!(object, tree);
            after_bind(object as usize, &COPIES[$index]);
            result
        }
    };
}

binder_bridge!(common_binder_bridge, OFF_BINDER_COMMON, 0);
binder_bridge!(item_binder_bridge, OFF_BINDER_ITEM, 1);
binder_bridge!(etc_binder_bridge, OFF_BINDER_ETC, 2);
binder_bridge!(power_up_binder_bridge, OFF_BINDER_POWER_UP, 3);
binder_bridge!(effect_binder_bridge, OFF_BINDER_EFFECT, 4);

pub(crate) fn install_hooks() {
    skyline::install_hooks!(
        common_binder_bridge,
        item_binder_bridge,
        etc_binder_bridge,
        power_up_binder_bridge,
        effect_binder_bridge,
        param_bind_kind_bridge
    );
}
