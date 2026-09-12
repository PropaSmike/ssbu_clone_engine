use super::*;

use clone_engine_core::fighter_param_row::{
    self as row, Install, NestStack, Op, Reject, RowOverride, Table, MOTION_ROW_SIZE,
    ORDINAL_STRIDE, PAYLOAD_BEGIN, PAYLOAD_END, ROW_SIZE,
};
use clone_engine_core::slots::{base_row, clone_row, CLONE_SLOTS, PARAM_NATIVE_KINDS};
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::RwLock;

const OFF_MODULE_ACCESSOR_INIT: usize = 0x6867e0;
const OFF_FIGHTER_INIT_SECOND: usize = 0x60b6b0;
const OFF_FIGHTER_PHASE_107: usize = 0x614630;
const OFF_FIGHTER_PHASE_109: usize = 0x6164a0;
const OFF_FIGHTER_PHASE_110: usize = 0x616580;
const OFF_FIGHTER_PHASE_111: usize = 0x619810;
const OFF_FIGHTER_PHASE_112: usize = 0x619850;
const OFF_FIGHTER_PHASE_113: usize = 0x619890;
const OFF_FIGHTER_PHASE_114: usize = 0x61a0a0;
const OFF_FIGHTER_PHASE_115: usize = 0x3a8bc0;
const ENTRY_STRUCT_ENTRY_ID: usize = 0xa8;
const OBJECT_ID: usize = 0x8;
const LOWEST_PLAUSIBLE_POINTER: usize = 0x1_0000;
const LOG_LIMIT: u32 = 48;
const TABLES: usize = 2;

struct RowCell<const N: usize> {
    bytes: UnsafeCell<[u8; N]>,
    built_from: AtomicUsize,
    generation: AtomicU32,
}

unsafe impl<const N: usize> Sync for RowCell<N> {}

impl<const N: usize> RowCell<N> {
    const fn new() -> Self {
        Self {
            bytes: UnsafeCell::new([0; N]),
            built_from: AtomicUsize::new(0),
            generation: AtomicU32::new(0),
        }
    }

    fn ptr(&self) -> *mut u8 {
        self.bytes.get() as *mut u8
    }
}

struct CloneSlot<const N: usize> {
    row: RowCell<N>,
    count: AtomicU32,
    generation: AtomicU32,
}

impl<const N: usize> CloneSlot<N> {
    const fn new() -> Self {
        Self {
            row: RowCell::new(),
            count: AtomicU32::new(0),
            generation: AtomicU32::new(0),
        }
    }
}

struct TableState<const N: usize> {
    vanilla: [RowCell<N>; PARAM_NATIVE_KINDS as usize],
    clones: [CloneSlot<N>; CLONE_SLOTS],
    installs: AtomicU32,
}

impl<const N: usize> TableState<N> {
    const fn new() -> Self {
        Self {
            vanilla: [const { RowCell::new() }; PARAM_NATIVE_KINDS as usize],
            clones: [const { CloneSlot::new() }; CLONE_SLOTS],
            installs: AtomicU32::new(0),
        }
    }

    const fn table(&self) -> Table {
        if N == ROW_SIZE {
            Table::FighterParam
        } else {
            Table::Motion
        }
    }

    fn overrides(&self) -> &'static RwLock<Vec<Vec<RowOverride>>> {
        match self.table() {
            Table::FighterParam => &FIGHTER_PARAM_OVERRIDES,
            Table::Motion => &MOTION_OVERRIDES,
        }
    }

    fn nest(&self, base: usize) -> &'static NestStack {
        match self.table() {
            Table::FighterParam => &FIGHTER_PARAM_NEST[base],
            Table::Motion => &MOTION_NEST[base],
        }
    }

    fn record(&self, index: usize, item: RowOverride) {
        let mut held = self.overrides().write().unwrap();
        if held.len() < CLONE_SLOTS {
            held.resize_with(CLONE_SLOTS, Vec::new);
        }
        held[index].push(item);
        self.clones[index]
            .count
            .store(held[index].len() as u32, Ordering::Release);
        self.clones[index].generation.fetch_add(1, Ordering::AcqRel);
    }

    fn has_overrides(&self, index: usize) -> bool {
        self.clones[index].count.load(Ordering::Acquire) != 0
    }

    unsafe fn live_row(&self, base_kind: i32) -> Option<usize> {
        let singleton = fighter_params::param_singleton()?;
        let payload =
            core::ptr::read_volatile((singleton + self.table().payload_slot()) as *const usize);
        if !plausible(payload) {
            return None;
        }
        let begin = core::ptr::read_volatile((payload + PAYLOAD_BEGIN) as *const usize);
        let end = core::ptr::read_volatile((payload + PAYLOAD_END) as *const usize);
        if !plausible(begin) || end < begin {
            return None;
        }
        let base = usize::try_from(base_kind).ok()?;
        if base >= PARAM_NATIVE_KINDS as usize {
            return None;
        }
        let ordinal = core::ptr::read_volatile(
            (singleton + self.table().ordinal_offset() + base * ORDINAL_STRIDE) as *const i32,
        );
        row::row_address_in(self.table(), begin, end, ordinal)
    }

    unsafe fn install(&self, target: usize, source: *const u8) {
        core::ptr::copy_nonoverlapping(source, target as *mut u8, N);
    }

    unsafe fn capture(&self, base: usize, source: usize) {
        let cell = &self.vanilla[base];
        core::ptr::copy_nonoverlapping(source as *const u8, cell.ptr(), N);
        cell.built_from.store(source, Ordering::Release);
    }

    unsafe fn source_for(&self, clone_kind: i32, base: usize) -> *const u8 {
        match clone_row(clone_kind) {
            Some(index) if self.has_overrides(index) => self.clones[index].row.ptr() as *const u8,
            _ => self.vanilla[base].ptr() as *const u8,
        }
    }

    unsafe fn ensure_clone_row(&self, clone_kind: i32, index: usize, base: usize, row_at: usize) {
        let slot = &self.clones[index];
        let generation = slot.generation.load(Ordering::Acquire);
        if slot.row.built_from.load(Ordering::Acquire) == row_at
            && slot.row.generation.load(Ordering::Acquire) == generation
        {
            return;
        }
        let held = self.overrides().read().unwrap();
        let overrides: &[RowOverride] = held.get(index).map(Vec::as_slice).unwrap_or(&[]);
        let vanilla = &*(self.vanilla[base].bytes.get() as *const [u8; N]);
        let out = &mut *slot.row.bytes.get();
        row::build(vanilla, overrides, out);
        slot.row.built_from.store(row_at, Ordering::Release);
        slot.row.generation.store(generation, Ordering::Release);
        push_deferred_accessor_sets(self.table(), clone_kind, out);
    }

    unsafe fn enter(&self, clone_kind: i32, base_kind: i32, base: usize, index: usize) -> usize {
        let has_overrides = self.has_overrides(index);
        let nest = self.nest(base);
        if !has_overrides && nest.depth() == 0 {
            return 0;
        }
        let name = self.table().name();
        let Some(row_at) = self.live_row(base_kind) else {
            log(&format!(
                "enter clone={clone_kind} base={base_kind}: {name} row not resident, direct readers keep vanilla"
            ));
            return 0;
        };
        let vanilla = &self.vanilla[base];
        if nest.depth() == 0 {
            if vanilla.built_from.load(Ordering::Acquire) != row_at {
                self.capture(base, row_at);
            }
        } else if vanilla.built_from.load(Ordering::Acquire) != row_at {
            log(&format!(
                "enter clone={clone_kind} base={base_kind}: {name} table moved to {row_at:#x} inside an open bracket, skipped"
            ));
            return 0;
        }
        if has_overrides {
            self.ensure_clone_row(clone_kind, index, base, row_at);
        }
        let Some(depth) = nest.push(clone_kind) else {
            log(&format!(
                "enter clone={clone_kind} base={base_kind}: {name} nest limit reached, skipped"
            ));
            return 0;
        };
        self.install(row_at, self.source_for(clone_kind, base));
        let n = self.installs.fetch_add(1, Ordering::Relaxed);
        if n < 8 {
            dbg_log_public(&format!(
                "[paramrow] #{n} clone={clone_kind} base={base_kind} {name} row {row_at:#x} installed (overrides={}, depth={depth})",
                self.clones[index].count.load(Ordering::Relaxed)
            ));
        }
        row_at
    }

    unsafe fn leave(&self, base_kind: i32, base: usize, row_at: usize) {
        if row_at == 0 {
            return;
        }
        let nest = self.nest(base);
        let next = nest.pop();
        if self.live_row(base_kind) != Some(row_at) {
            nest.reset();
            log(&format!(
                "leave base={base_kind}: {} row moved away from {row_at:#x} inside the bracket, nothing restored",
                self.table().name()
            ));
            return;
        }
        let source = match next {
            Install::Vanilla => self.vanilla[base].ptr() as *const u8,
            Install::Clone(kind) => self.source_for(kind, base),
        };
        self.install(row_at, source);
    }

}

static FIGHTER_PARAM: TableState<ROW_SIZE> = TableState::new();
static MOTION: TableState<MOTION_ROW_SIZE> = TableState::new();
static FIGHTER_PARAM_NEST: [NestStack; PARAM_NATIVE_KINDS as usize] =
    [const { NestStack::new() }; PARAM_NATIVE_KINDS as usize];
static MOTION_NEST: [NestStack; PARAM_NATIVE_KINDS as usize] =
    [const { NestStack::new() }; PARAM_NATIVE_KINDS as usize];
static FIGHTER_PARAM_OVERRIDES: RwLock<Vec<Vec<RowOverride>>> = RwLock::new(Vec::new());
static MOTION_OVERRIDES: RwLock<Vec<Vec<RowOverride>>> = RwLock::new(Vec::new());
static LOG: AtomicU32 = AtomicU32::new(0);
static ORDER_LOG: AtomicU32 = AtomicU32::new(0);

fn log(message: &str) {
    if LOG.fetch_add(1, Ordering::Relaxed) < LOG_LIMIT {
        dbg_log_public(&format!("[paramrow] {message}"));
    }
}

unsafe fn plausible(value: usize) -> bool {
    value >= LOWEST_PLAUSIBLE_POINTER && value & 7 == 0
}

pub(crate) struct Recorded {
    pub(crate) message: String,
    pub(crate) accessor_set_deferred: bool,
}

fn refused(message: String) -> Recorded {
    Recorded {
        message,
        accessor_set_deferred: false,
    }
}

pub(crate) fn record(kind: i32, slot: i32, param_type: u64, param_hash: u64, op: Op) -> Recorded {
    let Some((table, field_hash)) = row::table_for_key(param_type, param_hash) else {
        return refused(String::from("not a row field"));
    };
    let Some(index) = clone_row(kind) else {
        return refused(String::from("kind outside the clone range"));
    };
    let (item, field) = match row::resolve_in(table, field_hash, op) {
        Ok(resolved) => resolved,
        Err(Reject::UnknownField) => return refused(format!("not a {} field", table.name())),
        Err(Reject::OtherTable(Table::Motion)) => {
            return refused(String::from(
                "row: this is a fighter_param_motion field, register it as (param_motion, field) so the game finds it",
            ))
        }
        Err(Reject::OtherTable(Table::FighterParam)) => {
            return refused(String::from(
                "row: this is a fighter_param field, register it with no sub-key so the game finds it",
            ))
        }
        Err(Reject::Immutable) => return refused(String::from("fighter_kind is not overridable")),
        Err(Reject::Unsupported) => {
            return refused(String::from(
                "row: field type cannot take this op, direct readers keep vanilla",
            ))
        }
        Err(Reject::NotFinite) => return refused(String::from("row: value is not finite")),
    };
    if slot != param_overrides::ANY_SLOT {
        return refused(format!(
            "row: {} is slot-specific, skipped (direct readers see one row per kind)",
            field.name
        ));
    }
    match table {
        Table::FighterParam => FIGHTER_PARAM.record(index, item),
        Table::Motion => MOTION.record(index, item),
    }
    let deferred = matches!(op, Op::Mul(_));
    if deferred {
        DEFERRED_ACCESSOR_SETS.write().unwrap().push(DeferredSet {
            kind,
            table,
            offset: field.offset,
            key: (param_type, param_hash),
        });
    }
    Recorded {
        message: format!(
            "row: {} {} at +{:#x} queued for direct readers{}",
            table.name(),
            field.name,
            field.offset,
            if deferred {
                ", accessor set follows the first row build"
            } else {
                ""
            }
        ),
        accessor_set_deferred: deferred,
    }
}

struct DeferredSet {
    kind: i32,
    table: Table,
    offset: u16,
    key: (u64, u64),
}

static DEFERRED_ACCESSOR_SETS: RwLock<Vec<DeferredSet>> = RwLock::new(Vec::new());

unsafe fn push_deferred_accessor_sets(table: Table, kind: i32, built: &[u8]) {
    let mut held = DEFERRED_ACCESSOR_SETS.write().unwrap();
    if held.is_empty() {
        return;
    }
    let slots = [param_overrides::ANY_SLOT];
    held.retain(|entry| {
        if entry.kind != kind || entry.table != table {
            return true;
        }
        let Some(value) = row::read_f32(built, entry.offset) else {
            return false;
        };
        let pushed = param_overrides::push_to_param_config(
            kind,
            &slots,
            entry.key,
            param_overrides::OP_SET,
            f64::from(value),
        );
        log(&format!(
            "clone {kind}: {} +{:#x} multiplied to {value}, accessor set {}",
            table.name(),
            entry.offset,
            if pushed { "pushed to ParamConfig" } else { "refused by ParamConfig" }
        ));
        false
    });
}

pub(crate) struct RowGuard {
    base_kind: i32,
    rows: [usize; TABLES],
}

impl RowGuard {
    pub(crate) const IDLE: Self = Self {
        base_kind: -1,
        rows: [0; TABLES],
    };

    pub(crate) fn active(&self) -> bool {
        self.rows.iter().any(|row| *row != 0)
    }
}

pub(crate) unsafe fn enter(clone_kind: i32, base_kind: i32) -> RowGuard {
    let (Some(base), Some(index)) = (base_row(base_kind), clone_row(clone_kind)) else {
        return RowGuard::IDLE;
    };
    let idle = !FIGHTER_PARAM.has_overrides(index)
        && FIGHTER_PARAM.nest(base).depth() == 0
        && !MOTION.has_overrides(index)
        && MOTION.nest(base).depth() == 0;
    if idle {
        return RowGuard::IDLE;
    }
    let owned = fighter_params::swap_lock();
    let rows = [
        FIGHTER_PARAM.enter(clone_kind, base_kind, base, index),
        MOTION.enter(clone_kind, base_kind, base, index),
    ];
    fighter_params::swap_unlock(owned);
    RowGuard { base_kind, rows }
}

pub(crate) unsafe fn leave(guard: RowGuard) {
    if !guard.active() {
        return;
    }
    let Some(base) = base_row(guard.base_kind) else {
        return;
    };
    let owned = fighter_params::swap_lock();
    MOTION.leave(guard.base_kind, base, guard.rows[1]);
    FIGHTER_PARAM.leave(guard.base_kind, base, guard.rows[0]);
    fighter_params::swap_unlock(owned);
}

pub(crate) unsafe fn enter_for_clone(clone_kind: i32) -> RowGuard {
    match clone_base(clone_kind) {
        Some(base) => enter(clone_kind, base),
        None => RowGuard::IDLE,
    }
}

pub(crate) unsafe fn enter_for_object(object: usize) -> RowGuard {
    if object == 0 {
        return RowGuard::IDLE;
    }
    let id = core::ptr::read_volatile((object + OBJECT_ID) as *const u32);
    if id >> 28 != 0 {
        return RowGuard::IDLE;
    }
    match clone_kind_of_object(object as u64) {
        Some(clone_kind) => enter_for_clone(clone_kind),
        None => RowGuard::IDLE,
    }
}

unsafe fn entry_struct_clone(entry_struct: u64) -> Option<(i32, i32)> {
    let entry_id = entry_struct_entry(entry_struct)?;
    let clone_kind = entry_custom_kind(entry_id as u8)?;
    Some((entry_id, clone_kind))
}

#[skyline::hook(offset = OFF_MODULE_ACCESSOR_INIT)]
unsafe fn module_accessor_init_row_bridge(accessor: u64, entry_struct: u64) -> u64 {
    let found = entry_struct_clone(entry_struct);
    let guard = match found {
        Some((_, clone_kind)) => enter_for_clone(clone_kind),
        None => RowGuard::IDLE,
    };
    if let Some((entry_id, clone_kind)) = found {
        let n = ORDER_LOG.fetch_add(1, Ordering::Relaxed);
        if n < 8 {
            dbg_log_public(&format!(
                "[paramrow] #{n} module accessor init entry={entry_id} clone={clone_kind} inside init bracket={} row bracket={}",
                active_construction_kind_public().is_some(),
                guard.active()
            ));
        }
    }
    let result = call_original!(accessor, entry_struct);
    leave(guard);
    if let Some(entry_id) = entry_struct_entry(entry_struct) {
        fighter_common_copies::on_accessor_init(accessor, entry_id, found.map(|(_, kind)| kind));
    }
    result
}

unsafe fn entry_struct_entry(entry_struct: u64) -> Option<i32> {
    if entry_struct == 0 {
        return None;
    }
    let entry_id =
        core::ptr::read_volatile((entry_struct as usize + ENTRY_STRUCT_ENTRY_ID) as *const i32);
    (0..8).contains(&entry_id).then_some(entry_id)
}

#[skyline::hook(offset = OFF_FIGHTER_INIT_SECOND)]
unsafe fn fighter_init_second_row_bridge(object: u64) -> u64 {
    let guard = enter_for_object(object as usize);
    let result = call_original!(object);
    leave(guard);
    result
}

macro_rules! phase_bracket {
    ($name:ident, $offset:ident) => {
        #[skyline::hook(offset = $offset)]
        unsafe fn $name(object: u64) -> u64 {
            let guard = enter_for_object(object as usize);
            let result = call_original!(object);
            leave(guard);
            result
        }
    };
}

phase_bracket!(fighter_phase_107_row_bridge, OFF_FIGHTER_PHASE_107);
phase_bracket!(fighter_phase_109_row_bridge, OFF_FIGHTER_PHASE_109);
phase_bracket!(fighter_phase_110_row_bridge, OFF_FIGHTER_PHASE_110);
phase_bracket!(fighter_phase_111_row_bridge, OFF_FIGHTER_PHASE_111);
phase_bracket!(fighter_phase_112_row_bridge, OFF_FIGHTER_PHASE_112);
phase_bracket!(fighter_phase_113_row_bridge, OFF_FIGHTER_PHASE_113);
phase_bracket!(fighter_phase_114_row_bridge, OFF_FIGHTER_PHASE_114);
phase_bracket!(fighter_phase_115_row_bridge, OFF_FIGHTER_PHASE_115);

pub(crate) fn note_aux_data_init(kind: i32) {
    log(&format!("aux record for clone {kind} built under its row"));
}

pub(crate) fn install_hooks() {
    skyline::install_hooks!(
        module_accessor_init_row_bridge,
        fighter_init_second_row_bridge,
        fighter_phase_107_row_bridge,
        fighter_phase_109_row_bridge,
        fighter_phase_110_row_bridge,
        fighter_phase_111_row_bridge,
        fighter_phase_112_row_bridge,
        fighter_phase_113_row_bridge,
        fighter_phase_114_row_bridge,
        fighter_phase_115_row_bridge
    );
}
