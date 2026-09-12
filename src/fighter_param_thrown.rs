#![allow(improper_ctypes_definitions)]

use super::*;

use clone_engine_core::fighter_param_row::{Op, Reject};
use clone_engine_core::fighter_param_thrown::{self as thrown, Hold, Role, ThrownOverride};
use clone_engine_core::slots::{clone_row, CLONE_SLOTS};
use core::arch::aarch64::float32x4_t;
use core::sync::atomic::{AtomicU32, Ordering};
use std::sync::RwLock;

const OFF_THROWN_OFFSET: usize = 0x720f90;
const OFF_THROWN_OFFSET_THUNK: usize = 0x20a8000;
const OFF_THROWN_OFFSET_LUA_CALL: usize = 0x20a81b4;
const OFF_DONKEY_THROWN_OFFSET: usize = 0x7210e0;
const OFF_RIDLEY_DRAGGED_OFFSET: usize = 0x7211a0;
const OFF_DIDDY_SPECIAL_S_OFFSET: usize = 0x721240;
const OFF_MIIFIGHTER_SUPLEX_OFFSET: usize = 0x7212e0;
const OFF_GAOGAEN_FINAL_OFFSET: usize = 0x721380;
const OFF_DEMON_COMMAND_OFFSET: usize = 0x721430;
const OFF_DEMON_SPECIAL_LW_OFFSET: usize = 0x7214d0;
const OFF_BATTLE_OBJECT_FROM_ID: usize = 0x3ac560;

const OBJECT_KIND: usize = 0xc;
const OBJECT_MODULE_ACCESSOR: usize = 0x20;
const ACCESSOR_OBJECT_ID: usize = 0x8;
const FIGHTER_ENTRIES: usize = 8;
const LOG_LIMIT: u32 = 48;
const ADJUST_LOG_LIMIT: u32 = 16;

const LDR_X17_LITERAL_8: u32 = 0x5800_0051;
const BR_X17: u32 = 0xd61f_0220;

static OVERRIDES: RwLock<Vec<Vec<ThrownOverride>>> = RwLock::new(Vec::new());
static COUNTS: [AtomicU32; CLONE_SLOTS] = [const { AtomicU32::new(0) }; CLONE_SLOTS];
static TOTAL: AtomicU32 = AtomicU32::new(0);
static LOG: AtomicU32 = AtomicU32::new(0);
static ADJUSTED: AtomicU32 = AtomicU32::new(0);
static AMBIGUOUS: AtomicU32 = AtomicU32::new(0);

fn log(message: &str) {
    if LOG.fetch_add(1, Ordering::Relaxed) < LOG_LIMIT {
        dbg_log_public(&format!("[paramthrown] {message}"));
    }
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
        return refused(String::from("thrown: kind outside the clone range"));
    };
    let (item, key) = match thrown::resolve(param_hash, op) {
        Ok(resolved) => resolved,
        Err(Reject::UnknownField) => {
            return refused(String::from(
                "thrown: not a hold offset key (offset[_f|_b|_hi|_lw][_x|_y|_z], held_offset[_x|_y|_z])",
            ))
        }
        Err(Reject::Unsupported) => {
            return refused(String::from(
                "thrown: a whole offset takes Mul only, a component takes Set or Mul, never an int",
            ))
        }
        Err(Reject::NotFinite) => return refused(String::from("thrown: value is not finite")),
        Err(Reject::Immutable | Reject::OtherTable(_)) => {
            return refused(String::from("thrown: not overridable"))
        }
    };
    if slot != param_overrides::ANY_SLOT {
        return refused(format!(
            "thrown: {} is slot-specific, refused (one rule per clone)",
            key.name
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
    TOTAL.fetch_add(1, Ordering::Release);
    Recorded {
        message: format!(
            "thrown: {} ({}) queued for the accessor",
            key.name,
            match key.role {
                Role::Holder => "the clone holds someone",
                Role::Held => "someone holds the clone",
            }
        ),
        recorded: true,
    }
}

fn has_overrides(clone_kind: Option<i32>) -> bool {
    clone_kind
        .and_then(clone_row)
        .is_some_and(|index| COUNTS[index].load(Ordering::Acquire) != 0)
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Identity {
    holder: Option<i32>,
    held: Option<i32>,
}

unsafe fn object_from_id(id: u32) -> Option<usize> {
    if id == u32::MAX || id == *smash::lib::lua_const::BATTLE_OBJECT_ID_INVALID as u32 {
        return None;
    }
    let lookup: extern "C" fn(u32) -> u64 =
        core::mem::transmute(text_base() + OFF_BATTLE_OBJECT_FROM_ID);
    let object = lookup(id) as usize;
    (object != 0).then_some(object)
}

unsafe fn live_object_of_boma(boma: usize) -> Option<usize> {
    if boma == 0 {
        return None;
    }
    let id = core::ptr::read_volatile((boma + ACCESSOR_OBJECT_ID) as *const u32);
    let object = object_from_id(id)?;
    let accessor = core::ptr::read_volatile((object + OBJECT_MODULE_ACCESSOR) as *const usize);
    (accessor == boma).then_some(object)
}

unsafe fn kind_of_object(object: usize) -> i32 {
    core::ptr::read_volatile((object + OBJECT_KIND) as *const i32)
}

unsafe fn holder_of(boma: usize) -> Option<usize> {
    let parent = smash::app::lua_bind::LinkModule::get_parent_id(
        boma as *mut smash::app::BattleObjectModuleAccessor,
        *smash::lib::lua_const::LINK_NO_CAPTURE,
        true,
    ) as u32;
    object_from_id(parent)
}

unsafe fn identify(holder_kind: Option<i32>, held_kind: i32) -> Option<Identity> {
    let mut found: Option<Identity> = None;
    for entry in 0..FIGHTER_ENTRIES {
        let boma = ENTRY_FIGHTER_BOMA[entry].load(Ordering::SeqCst);
        let Some(object) = live_object_of_boma(boma) else {
            continue;
        };
        if kind_of_object(object) != held_kind {
            continue;
        }
        let Some(holder) = holder_of(boma) else {
            continue;
        };
        if holder_kind.is_some_and(|kind| kind_of_object(holder) != kind) {
            continue;
        }
        let identity = Identity {
            holder: clone_kind_of_object(holder as u64),
            held: clone_kind_of_object(object as u64),
        };
        match found {
            None => found = Some(identity),
            Some(previous) if previous == identity => {}
            Some(_) => {
                if AMBIGUOUS.fetch_add(1, Ordering::Relaxed) < 4 {
                    log(&format!(
                        "two holds with the same kinds (holder {holder_kind:?}, held {held_kind}) are live at once, this read keeps vanilla"
                    ));
                }
                return None;
            }
        }
    }
    found
}

unsafe fn adjust(result: float32x4_t, holder_kind: Option<i32>, held_kind: i32, hold: Hold, what: &str) -> float32x4_t {
    if TOTAL.load(Ordering::Acquire) == 0 {
        return result;
    }
    let Some(identity) = identify(holder_kind, held_kind) else {
        return result;
    };
    if !has_overrides(identity.holder) && !has_overrides(identity.held) {
        return result;
    }
    let mut lanes: [f32; 4] = core::mem::transmute(result);
    let mut vector = [lanes[0], lanes[1], lanes[2]];
    let before = vector;
    let mut applied = 0;
    {
        let held = OVERRIDES.read().unwrap();
        if let Some(index) = identity.holder.and_then(clone_row) {
            if let Some(rules) = held.get(index) {
                applied += thrown::apply(&mut vector, rules, Role::Holder, hold);
            }
        }
        if let Some(index) = identity.held.and_then(clone_row) {
            if let Some(rules) = held.get(index) {
                applied += thrown::apply(&mut vector, rules, Role::Held, hold);
            }
        }
    }
    if applied == 0 {
        return result;
    }
    if ADJUSTED.fetch_add(1, Ordering::Relaxed) < ADJUST_LOG_LIMIT {
        log(&format!(
            "{what} {hold:?} holder {holder_kind:?} (clone {:?}) held {held_kind} (clone {:?}): {before:?} -> {vector:?} ({applied} rule(s))",
            identity.holder, identity.held
        ));
    }
    lanes[0] = vector[0];
    lanes[1] = vector[1];
    lanes[2] = vector[2];
    core::mem::transmute(lanes)
}

unsafe extern "C" fn thrown_offset_handler(this: u64, holder: i32, held: i32, selector: i32) -> float32x4_t {
    let original: extern "C" fn(u64, i32, i32, i32) -> float32x4_t =
        core::mem::transmute(text_base() + OFF_THROWN_OFFSET);
    let result = original(this, holder, held, selector);
    adjust(result, Some(holder), held, Hold::from_selector(selector), "thrown_offset")
}

#[skyline::hook(offset = OFF_DONKEY_THROWN_OFFSET)]
unsafe fn donkey_thrown_offset_bridge(this: u64, held: i32, selector: i32) -> float32x4_t {
    let result = call_original!(this, held, selector);
    adjust(result, None, held, Hold::from_selector(selector), "donkey_thrown_offset")
}

#[skyline::hook(offset = OFF_RIDLEY_DRAGGED_OFFSET)]
unsafe fn ridley_dragged_offset_bridge(this: u64, held: i32, selector: i32) -> float32x4_t {
    let result = call_original!(this, held, selector);
    adjust(result, None, held, Hold::Other, "ridley_dragged_offset")
}

#[skyline::hook(offset = OFF_GAOGAEN_FINAL_OFFSET)]
unsafe fn gaogaen_final_offset_bridge(this: u64, held: i32, selector: i32) -> float32x4_t {
    let result = call_original!(this, held, selector);
    adjust(result, None, held, Hold::Other, "gaogaen_final_offset")
}

macro_rules! held_only_bridge {
    ($name:ident, $offset:ident, $what:literal) => {
        #[skyline::hook(offset = $offset)]
        unsafe fn $name(this: u64, held: i32) -> float32x4_t {
            let result = call_original!(this, held);
            adjust(result, None, held, Hold::Other, $what)
        }
    };
}

held_only_bridge!(diddy_special_s_offset_bridge, OFF_DIDDY_SPECIAL_S_OFFSET, "diddy_special_s_offset");
held_only_bridge!(miifighter_suplex_offset_bridge, OFF_MIIFIGHTER_SUPLEX_OFFSET, "miifighter_suplex_offset");
held_only_bridge!(demon_command_offset_bridge, OFF_DEMON_COMMAND_OFFSET, "demon_command_offset");
held_only_bridge!(demon_special_lw_offset_bridge, OFF_DEMON_SPECIAL_LW_OFFSET, "demon_special_lw_offset");

fn branch_link(site: usize, target: usize) -> u32 {
    let words = ((target as i64 - site as i64) >> 2) as u32;
    0x9400_0000 | (words & 0x03ff_ffff)
}

unsafe fn install_thrown_offset_trampoline() {
    let base = text_base();
    let thunk = base + OFF_THROWN_OFFSET_THUNK;
    let handler = thrown_offset_handler as *const () as usize as u64;
    let words = [
        LDR_X17_LITERAL_8,
        BR_X17,
        handler as u32,
        (handler >> 32) as u32,
    ];
    let expected_thunk = 0x1400_0000 | ((((OFF_THROWN_OFFSET as i64 - OFF_THROWN_OFFSET_THUNK as i64) >> 2) as u32) & 0x03ff_ffff);
    let found = core::ptr::read_volatile(thunk as *const u32);
    if found != expected_thunk {
        log(&format!(
            "thunk {OFF_THROWN_OFFSET_THUNK:#x} reads {found:#010x}, expected {expected_thunk:#010x}; thrown_offset is not bridged"
        ));
        return;
    }
    if !text_patch::write_words(thunk, &words) {
        log(&format!("thunk {OFF_THROWN_OFFSET_THUNK:#x} refused the trampoline; thrown_offset is not bridged"));
        return;
    }
    let call = base + OFF_THROWN_OFFSET_LUA_CALL;
    let expected_call = branch_link(OFF_THROWN_OFFSET_LUA_CALL, OFF_THROWN_OFFSET);
    let found = core::ptr::read_volatile(call as *const u32);
    if found != expected_call {
        log(&format!(
            "lua wrapper call {OFF_THROWN_OFFSET_LUA_CALL:#x} reads {found:#010x}, expected {expected_call:#010x}; left alone"
        ));
        return;
    }
    if !text_patch::write_word(call, branch_link(OFF_THROWN_OFFSET_LUA_CALL, OFF_THROWN_OFFSET_THUNK)) {
        log(&format!("lua wrapper call {OFF_THROWN_OFFSET_LUA_CALL:#x} refused the repoint; left alone"));
    }
}

pub(crate) fn install_hooks() {
    unsafe { install_thrown_offset_trampoline() };
    skyline::install_hooks!(
        donkey_thrown_offset_bridge,
        ridley_dragged_offset_bridge,
        gaogaen_final_offset_bridge,
        diddy_special_s_offset_bridge,
        miifighter_suplex_offset_bridge,
        demon_command_offset_bridge,
        demon_special_lw_offset_bridge
    );
}
