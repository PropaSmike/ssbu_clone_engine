use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Mutex, RwLock};

use clone_engine_core::partner_params::PartnerParam;
use clone_engine_core::switch_group::{Shape, TRAINER_KINDS};

pub(crate) const NANA: i32 = 0x4c;
const POPO: i32 = 0x4b;
const ELEMENT: i32 = 0x75;
const PTRAINER: i32 = 0x72;
const OFF_LOAD_KIND_GROUPS: usize = 0x17e6510;
const ICE_CLIMBER: u64 = 0x6e;
const CTX_KIND: usize = 0x58;
const CTX_SECOND_KIND: usize = 0x5c;
const LOG_BUDGET: u32 = 24;
const OFF_WIN_ARTICLE_PRELOAD: usize = 0x65a650;
const ENTRY_FIGHTER_KIND: usize = 0x18;
const WIN_PRELOAD_KINDS: [i32; 4] = [0x6, 0x52, 0x5a, 0x5b];

static LOG: AtomicU32 = AtomicU32::new(0);
static WIN_PRELOAD_LOG: AtomicU32 = AtomicU32::new(0);
static PARTNERED: RwLock<Vec<Partnered>> = RwLock::new(Vec::new());
static PARTNER_PARAMS_LOGGED: AtomicBool = AtomicBool::new(false);

struct Partnered {
    leader: i32,
    with: Option<String>,
    params: Vec<PartnerParam>,
}
static PARTNER_REFUSED: Mutex<Vec<i32>> = Mutex::new(Vec::new());
static PARTNER_DECLINED: RwLock<Vec<i32>> = RwLock::new(Vec::new());
static NAMED_LOGGED: Mutex<Vec<i32>> = Mutex::new(Vec::new());
static DEFERRED: Mutex<Vec<(usize, i32)>> = Mutex::new(Vec::new());
static MEMBER_OBJECTS: Mutex<Vec<(i32, u32, i32)>> = Mutex::new(Vec::new());
static SAME_BASE_LOGGED: AtomicBool = AtomicBool::new(false);
static OWN_CONTROLLER_LOGGED: AtomicBool = AtomicBool::new(false);

static SWITCH_DECLARED: RwLock<Vec<(i32, Vec<String>)>> = RwLock::new(Vec::new());
static SWITCH_GROUPS: RwLock<Option<Vec<Vec<i32>>>> = RwLock::new(None);
static SWITCH_REFUSED: Mutex<Vec<i32>> = Mutex::new(Vec::new());

pub(crate) fn register_switch(kind: i32, members: Vec<String>) {
    if members.is_empty() {
        return;
    }
    if let Ok(mut declared) = SWITCH_DECLARED.write() {
        declared.retain(|(leader, _)| *leader != kind);
        declared.push((kind, members));
    }
    if let Ok(mut groups) = SWITCH_GROUPS.write() {
        *groups = None;
    }
}

fn refuse_once(leader: i32, message: String) {
    let Ok(mut refused) = SWITCH_REFUSED.lock() else {
        return;
    };
    if !refused.contains(&leader) {
        refused.push(leader);
        crate::dbg_log_public(&format!("[switch] kind {leader}: {message}; it plays alone"));
    }
}

fn resolve_group(leader: i32, names: &[String]) -> Option<Vec<i32>> {
    let mut kinds = vec![leader];
    for name in names {
        match crate::clone_definition_from_name(name) {
            Some(definition) => kinds.push(definition.kind),
            None => {
                refuse_once(leader, format!("switch member {name} is not a registered fighter"));
                return None;
            }
        }
    }
    let bases: Option<Vec<i32>> = kinds
        .iter()
        .map(|kind| crate::clone_definition(*kind).map(|definition| definition.base_kind))
        .collect();
    match clone_engine_core::switch_group::shape(&bases?) {
        Ok(_) => Some(kinds),
        Err(message) => {
            refuse_once(leader, message);
            None
        }
    }
}

fn resolved_groups() -> Vec<Vec<i32>> {
    if let Some(groups) = SWITCH_GROUPS.read().ok().and_then(|groups| groups.clone()) {
        return groups;
    }
    let Ok(declared) = SWITCH_DECLARED.read() else {
        return Vec::new();
    };
    if declared.is_empty() {
        return Vec::new();
    }
    let mut complete = true;
    let mut groups = Vec::new();
    for (leader, names) in declared.iter() {
        match resolve_group(*leader, names) {
            Some(group) => groups.push(group),
            None => complete = false,
        }
    }
    if complete {
        if let Ok(mut cache) = SWITCH_GROUPS.write() {
            *cache = Some(groups.clone());
        }
    }
    groups
}

pub(crate) fn switch_group(kind: i32) -> Option<Vec<i32>> {
    resolved_groups().into_iter().find_map(|group| {
        let first = group.iter().position(|member| *member == kind)?;
        Some(clone_engine_core::switch_group::rotate(&group, first))
    })
}

pub(crate) fn switch_members(kind: i32) -> Vec<i32> {
    switch_group(kind).map(|group| group[1..].to_vec()).unwrap_or_default()
}

pub(crate) fn members(leader: i32) -> Vec<i32> {
    let mut members = switch_members(leader);
    members.extend(partner_clone(leader));
    members
}

pub(crate) fn member_definition(entry: Option<usize>, kind: i32) -> Option<&'static crate::CloneDefinition> {
    let leader = entry.filter(|entry| *entry < 8).and_then(|entry| crate::entry_custom_kind(entry as u8))?;
    member_of(leader, kind).and_then(crate::clone_definition)
}

#[skyline::from_offset(crate::game::rel(OFF_LOAD_KIND_GROUPS))]
unsafe fn load_kind_groups(record: u64, kind: i32);

fn shared_owner(bases: &[i32]) -> Option<i32> {
    if bases.iter().all(|base| matches!(base, 0x5a | 0x5b)) {
        Some(ELEMENT)
    } else if bases.iter().all(|base| matches!(base, 0x24..=0x26)) {
        Some(PTRAINER)
    } else {
        None
    }
}

pub(crate) fn loader_owner(leader: i32) -> Option<i32> {
    let members = switch_members(leader);
    if members.is_empty() {
        return solo_owner(leader);
    }
    let bases: Option<Vec<i32>> = core::iter::once(leader)
        .chain(members)
        .map(|kind| crate::clone_definition(kind).map(|definition| definition.base_kind))
        .collect();
    shared_owner(&bases?)
}

pub(crate) fn solo_owner(kind: i32) -> Option<i32> {
    if !switch_members(kind).is_empty() {
        return None;
    }
    shared_owner(&[crate::clone_definition(kind)?.base_kind])
}

pub(crate) unsafe fn load_members(record: usize, leader: i32) {
    let members = switch_members(leader);
    let Some(leader_definition) = crate::clone_definition(leader) else {
        return;
    };
    if let Some(partner) = partner_clone(leader) {
        crate::with_resource_context(partner, || load_kind_groups(record as u64, partner));
        if log_budget() {
            crate::dbg_log_public(&format!("[partner] loader for kind {leader} also loads its partner {partner}"));
        }
    }
    if members.is_empty() {
        if let Some(owner) = solo_owner(leader) {
            load_kind_groups(record as u64, owner);
            if log_budget() {
                crate::dbg_log_public(&format!(
                    "[switch] kind {leader} plays alone; its loader also loads shared owner {owner}"
                ));
            }
        }
        return;
    }
    let mut bases = vec![leader_definition.base_kind];
    for member in &members {
        let Some(definition) = crate::clone_definition(*member) else {
            continue;
        };
        bases.push(definition.base_kind);
        crate::with_resource_context(definition.kind, || load_kind_groups(record as u64, definition.kind));
    }
    let owner = shared_owner(&bases);
    if let Some(owner) = owner {
        load_kind_groups(record as u64, owner);
    }
    if log_budget() {
        crate::dbg_log_public(&format!(
            "[switch] loader for kind {leader} also loads {members:?} and shared owner {owner:?}"
        ));
    }
}

const OFF_GROUP_FLAGS_SETUP: usize = 0x654dc0;
const OFF_GROUP_ARTICLES_SETUP: usize = 0x655320;
const OFF_GROUP_HELPERS_SETUP: usize = 0x64f820;
const OFF_GROUP_MEMBER_LOOKUP: usize = 0x64fb70;
const OFF_GROUP_CHANGE: usize = 0x6581f0;
const ENTRY_COUNT: usize = 0x14;
const ENTRY_KINDS: usize = 0x18;

struct PresentedKinds {
    group: Vec<i32>,
    replaced: Vec<(usize, i32, i32)>,
}

unsafe fn present_base_kinds(object: u64) -> Option<PresentedKinds> {
    if object == 0 {
        return None;
    }
    let count = core::ptr::read_volatile((object as usize + ENTRY_COUNT) as *const i32);
    if !(2..=3).contains(&count) {
        return None;
    }
    let kinds = (object as usize + ENTRY_KINDS) as *mut i32;
    let first = core::ptr::read_volatile(kinds);
    let group = switch_group(first)?;
    let mut replaced = Vec::new();
    for index in 0..count as usize {
        let kind = core::ptr::read_volatile(kinds.add(index));
        if let Some(definition) = crate::clone_definition(kind).filter(|_| group.contains(&kind)) {
            core::ptr::write_volatile(kinds.add(index), definition.base_kind);
            replaced.push((index, kind, definition.base_kind));
        }
    }
    Some(PresentedKinds { group, replaced })
}

unsafe fn restore_kinds(object: u64, presented: Option<PresentedKinds>) {
    let Some(presented) = presented else {
        return;
    };
    let kinds = (object as usize + ENTRY_KINDS) as *mut i32;
    for (index, member, base) in presented.replaced {
        if core::ptr::read_volatile(kinds.add(index)) == base {
            core::ptr::write_volatile(kinds.add(index), member);
        }
    }
}

const FREE_GROUP_FLAGS: u8 = 0x44;
const FLAG_HELPER_TABLE: u8 = 4;
const ENTRY_CURRENT_HELPER: usize = 0x4178;
const ENTRY_MEMBER_HELPERS: usize = 0x4188;
const ENTRY_MAIN_MEMBER: usize = 0x5918;
const ENTRY_TARGET_MEMBER: usize = 0x591a;
const ENTRY_STATE: usize = 0x5920;
const ENTRY_STATE_PLAYING: u8 = 6;
const ENTRY_STATE_CHANGING: u8 = 3;
const ENTRY_CHANGE_STEP: usize = 0x591c;
const CHANGE_STEP_WAIT_OUT: u8 = 0xc;
const ENTRY_CHANGE_OUT_DONE: usize = 0x5933;
const ENTRY_CHANGE_CARRY: usize = 0x5932;
const ENTRY_MEMBER_COUNT: usize = 0x4150;
const ENTRY_PLAYER: usize = 0xe0;
const ENTRY_INFO: usize = 0xf8;
const INFO_HELPER_MODE: usize = 0x92;
const ENTRY_HELPER_STYLE: usize = 0x50;
const PLAYER_SETTINGS: usize = 0x52c48ea;
const PLAYER_SETTINGS_STRIDE: usize = 0x260;
const OFF_ALLOC_ALIGNED: usize = 0x392dce0;
const OFF_HELPER_INIT: usize = 0x6b6f10;
const HELPER_ALIGN: u64 = 0x10;
const HELPER_SIZE: u64 = 0x80;
const NO_TARGET: u8 = 0xff;

static SWITCH_REQUEST_LOGGED: AtomicBool = AtomicBool::new(false);

fn group_shape(group: &[i32]) -> Option<Shape> {
    let bases: Option<Vec<i32>> = group
        .iter()
        .map(|kind| crate::clone_definition(*kind).map(|definition| definition.base_kind))
        .collect();
    clone_engine_core::switch_group::shape(&bases?).ok()
}

unsafe fn free_group(object: u64) -> Option<usize> {
    if object == 0 {
        return None;
    }
    let count = core::ptr::read_volatile((object as usize + ENTRY_COUNT) as *const i32);
    if !(2..=3).contains(&count) {
        return None;
    }
    let first = core::ptr::read_volatile((object as usize + ENTRY_KINDS) as *const i32);
    let group = switch_group(first)?;
    (group_shape(&group) == Some(Shape::Free)).then_some(count as usize)
}

unsafe fn present_table_kinds(object: u64) -> Option<[i32; 3]> {
    let count = free_group(object)?;
    let kinds = (object as usize + ENTRY_KINDS) as *mut i32;
    let mut saved = [-1; 3];
    for (index, slot) in saved.iter_mut().enumerate().take(count) {
        *slot = core::ptr::read_volatile(kinds.add(index));
        core::ptr::write_volatile(kinds.add(index), TRAINER_KINDS[index]);
    }
    Some(saved)
}

unsafe fn restore_table_kinds(object: u64, saved: Option<[i32; 3]>) {
    let Some(saved) = saved else {
        return;
    };
    let kinds = (object as usize + ENTRY_KINDS) as *mut i32;
    for (index, kind) in saved.iter().enumerate().filter(|(_, kind)| **kind != -1) {
        core::ptr::write_volatile(kinds.add(index), *kind);
    }
}

unsafe fn new_helper(entry: usize, kind: i32) -> usize {
    type Alloc = extern "C" fn(u64, u64) -> usize;
    type Init = extern "C" fn(usize, i32, i32, u8, bool, u8, usize);
    let alloc: Alloc = core::mem::transmute(crate::game::addr(OFF_ALLOC_ALIGNED));
    let helper = alloc(HELPER_ALIGN, HELPER_SIZE);
    if helper == 0 {
        return 0;
    }
    let player = core::ptr::read_volatile((entry + ENTRY_PLAYER) as *const i32);
    let info = core::ptr::read_volatile((entry + ENTRY_INFO) as *const usize);
    let mode = if info == 0 { 0 } else { core::ptr::read_volatile((info + INFO_HELPER_MODE) as *const u8) };
    let setting = (0..8).contains(&player)
        && core::ptr::read_volatile(
            (crate::game::addr(PLAYER_SETTINGS) + player as usize * PLAYER_SETTINGS_STRIDE) as *const u8,
        ) != 0;
    let style = core::ptr::read_volatile((entry + ENTRY_HELPER_STYLE) as *const u8);
    let init: Init = core::mem::transmute(crate::game::addr(OFF_HELPER_INIT));
    init(helper, player, kind, mode, setting, style, 0);
    helper
}

unsafe fn build_member_helpers(entry: usize, count: usize) {
    let main = core::ptr::read_volatile((entry + ENTRY_MAIN_MEMBER) as *const u8) as usize;
    let made = core::ptr::read_volatile((entry + ENTRY_CURRENT_HELPER) as *const usize);
    let kinds = (entry + ENTRY_KINDS) as *const i32;
    let mut helpers = [0usize; 3];
    for (index, helper) in helpers.iter_mut().enumerate().take(count) {
        *helper = if index == main { made } else { new_helper(entry, core::ptr::read_volatile(kinds.add(index))) };
    }
    for (index, helper) in helpers.iter().enumerate() {
        core::ptr::write_volatile((entry + ENTRY_MEMBER_HELPERS + index * 8) as *mut usize, *helper);
    }
    core::ptr::write_volatile((entry + ENTRY_CURRENT_HELPER) as *mut usize, 0);
    core::ptr::write_volatile((entry + ENTRY_CURRENT_HELPER + 8) as *mut usize, 0);
    if log_budget() {
        crate::dbg_log_public(&format!("[switch] free group: one HUD helper per member {helpers:x?}"));
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_GROUP_FLAGS_SETUP))]
unsafe fn group_flags_setup(object: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    if free_group(object).is_some() {
        let result = call_original!(object, a1, a2, a3);
        core::ptr::write_volatile((object as usize + ENTRY_GROUP_FLAGS) as *mut u8, FREE_GROUP_FLAGS);
        if log_budget() {
            crate::dbg_log_public(&format!("[switch] free group: entry flags {FREE_GROUP_FLAGS:#x}"));
        }
        return result;
    }
    base_kind_window(object, "group_flags_setup", || call_original!(object, a1, a2, a3))
}

#[skyline::hook(offset = crate::game::rel(OFF_GROUP_ARTICLES_SETUP))]
unsafe fn group_articles_setup(object: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    if let Some(count) = free_group(object) {
        let flags = (object as usize + ENTRY_GROUP_FLAGS) as *mut u8;
        let before = core::ptr::read_volatile(flags);
        core::ptr::write_volatile(flags, before & !FLAG_HELPER_TABLE);
        let result = call_original!(object, a1, a2, a3);
        core::ptr::write_volatile(flags, before);
        if before & FLAG_HELPER_TABLE != 0 {
            build_member_helpers(object as usize, count);
        }
        return result;
    }
    base_kind_window(object, "group_articles_setup", || call_original!(object, a1, a2, a3))
}

unsafe fn base_kind_window(object: u64, name: &str, original: impl FnOnce() -> u64) -> u64 {
    let saved = present_base_kinds(object);
    if let Some(presented) = saved.as_ref().filter(|_| log_budget()) {
        crate::dbg_log_public(&format!("[switch] {name} sees the group {:?} as its base fighters", presented.group));
    }
    let result = original();
    restore_kinds(object, saved);
    result
}

macro_rules! group_window {
    ($name:ident, $offset:expr) => {
        #[skyline::hook(offset = crate::game::rel($offset))]
        unsafe fn $name(object: u64, a1: u64, a2: u64, a3: u64) -> u64 {
            let table = present_table_kinds(object);
            let result = base_kind_window(object, stringify!($name), || call_original!(object, a1, a2, a3));
            restore_table_kinds(object, table);
            result
        }
    };
}

group_window!(group_helpers_setup, OFF_GROUP_HELPERS_SETUP);
group_window!(group_member_lookup, OFF_GROUP_MEMBER_LOOKUP);

const OFF_MEMBER_ACTIVATE: usize = 0x656ee0;
const ENTRY_STATE_ACTIVATING: u8 = 5;
const PLACE_IDLE: u8 = 0;
const PLACE_REQUESTED: u8 = 1;
const PLACE_SAVED: u8 = 2;

static PLACE_ENTRY: [AtomicUsize; ENTRY_SLOTS] = [const { AtomicUsize::new(0) }; ENTRY_SLOTS];
static PLACE_STATE: [AtomicU8; ENTRY_SLOTS] = [const { AtomicU8::new(PLACE_IDLE) }; ENTRY_SLOTS];
static PLACE_POS: [AtomicU64; ENTRY_SLOTS] = [const { AtomicU64::new(0) }; ENTRY_SLOTS];
static PLACE_LR: [AtomicU32; ENTRY_SLOTS] = [const { AtomicU32::new(0) }; ENTRY_SLOTS];

fn place_slot(entry: usize, state: u8) -> Option<usize> {
    (0..ENTRY_SLOTS).find(|slot| {
        PLACE_ENTRY[*slot].load(Ordering::Acquire) == entry && PLACE_STATE[*slot].load(Ordering::Acquire) == state
    })
}

unsafe fn main_member_boma(entry: usize) -> Option<*mut smash::app::BattleObjectModuleAccessor> {
    let main = core::ptr::read_volatile((entry + ENTRY_MAIN_MEMBER) as *const u8) as usize;
    if main >= 3 {
        return None;
    }
    let object = core::ptr::read_volatile((entry + ENTRY_MEMBER_OBJECTS + main * 8) as *const usize);
    if object == 0 {
        return None;
    }
    let id = core::ptr::read_volatile((object + OBJECT_ID) as *const u32);
    if id == OBJECT_ID_INVALID || !smash::app::sv_battle_object::is_active(id) {
        return None;
    }
    let boma = smash::app::sv_battle_object::module_accessor(id);
    (!boma.is_null()).then_some(boma)
}

unsafe fn save_outgoing_place(entry: usize) {
    let Some(slot) = place_slot(entry, PLACE_REQUESTED) else {
        return;
    };
    let Some(boma) = main_member_boma(entry) else {
        PLACE_STATE[slot].store(PLACE_IDLE, Ordering::Release);
        return;
    };
    let x = smash::app::lua_bind::PostureModule::pos_x(boma);
    let y = smash::app::lua_bind::PostureModule::pos_y(boma);
    let lr = smash::app::lua_bind::PostureModule::lr(boma);
    PLACE_POS[slot].store(((x.to_bits() as u64) << 32) | y.to_bits() as u64, Ordering::Release);
    PLACE_LR[slot].store(lr.to_bits(), Ordering::Release);
    PLACE_STATE[slot].store(PLACE_SAVED, Ordering::Release);
}

unsafe fn place_incoming(entry: usize) {
    let Some(slot) = place_slot(entry, PLACE_SAVED) else {
        return;
    };
    PLACE_STATE[slot].store(PLACE_IDLE, Ordering::Release);
    let Some(boma) = main_member_boma(entry) else {
        return;
    };
    let packed = PLACE_POS[slot].load(Ordering::Acquire);
    let x = f32::from_bits((packed >> 32) as u32);
    let y = f32::from_bits(packed as u32);
    let lr = f32::from_bits(PLACE_LR[slot].load(Ordering::Acquire));
    let before = smash::app::lua_bind::PostureModule::pos_x(boma);
    let z = smash::app::lua_bind::PostureModule::pos_z(boma);
    let pos = smash::phx::Vector3f { x, y, z };
    smash::app::lua_bind::PostureModule::set_pos(boma, &pos);
    smash::app::lua_bind::PostureModule::set_lr(boma, lr);
    smash::app::lua_bind::PostureModule::update_rot_y_lr(boma);
    if log_budget() {
        crate::dbg_log_public(&format!(
            "[switch] entry {slot}: the incoming member takes the outgoing one's place (x {before} -> {x}, y {y}, lr {lr})"
        ));
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_MEMBER_ACTIVATE))]
unsafe fn member_activate(entry: u64, mode: u64) -> u64 {
    let changing = core::ptr::read_volatile((entry as usize + ENTRY_STATE) as *const u8) == ENTRY_STATE_ACTIVATING;
    let result = call_original!(entry, mode);
    if changing {
        place_incoming(entry as usize);
    } else if let Some(slot) = place_slot(entry as usize, PLACE_SAVED) {
        PLACE_STATE[slot].store(PLACE_IDLE, Ordering::Release);
    }
    result
}

#[skyline::hook(offset = crate::game::rel(OFF_GROUP_CHANGE))]
unsafe fn group_change(object: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    save_outgoing_place(object as usize);
    let free = free_group(object).is_some();
    let table = present_table_kinds(object);
    let result = base_kind_window(object, "group_change", || call_original!(object, a1, a2, a3));
    restore_table_kinds(object, table);
    let entry = object as usize;
    if free
        && core::ptr::read_volatile((entry + ENTRY_STATE) as *const u8) == ENTRY_STATE_CHANGING
        && core::ptr::read_volatile((entry + ENTRY_CHANGE_STEP) as *const u8) == CHANGE_STEP_WAIT_OUT
    {
        core::ptr::write_volatile((entry + ENTRY_CHANGE_OUT_DONE) as *mut u8, 1);
    }
    result
}

const OFF_TRAINER_FACE: usize = 0x15605f0;
const OFF_KIND_FACE: usize = 0x138c9b0;
const ENTRY_SLOTS: usize = 8;

static FACE_THREAD: AtomicUsize = AtomicUsize::new(0);
static FACE_KINDS: AtomicU64 = AtomicU64::new(0);

unsafe fn nth_entry(index: u32) -> Option<usize> {
    let holder = core::ptr::read_volatile(crate::game::addr(FIGHTER_MANAGER) as *const usize);
    if holder == 0 {
        return None;
    }
    let manager = core::ptr::read_volatile(holder as *const usize);
    if manager == 0 {
        return None;
    }
    (0..ENTRY_SLOTS)
        .map(|slot| core::ptr::read_volatile((manager + MANAGER_ENTRIES + slot * 8) as *const usize))
        .filter(|entry| *entry != 0)
        .nth(index as usize)
}

unsafe fn trainer_face_kinds(index: u32) -> Option<(i32, i32)> {
    let entry = nth_entry(index)?;
    let main = core::ptr::read_volatile((entry + ENTRY_MAIN_MEMBER) as *const u8) as usize;
    if main >= 3 {
        return None;
    }
    let kind = core::ptr::read_volatile((entry + ENTRY_KINDS + main * 4) as *const i32);
    let group = switch_group(kind)?;
    if group_shape(&group) != Some(Shape::Trainer) {
        return None;
    }
    Some((crate::clone_definition(kind)?.base_kind, kind))
}

unsafe fn current_thread() -> usize {
    skyline::nn::os::GetCurrentThread() as usize
}

#[skyline::hook(offset = crate::game::rel(OFF_TRAINER_FACE))]
unsafe fn trainer_face(index: u32, hud: u64, players: u64) {
    let Some((base, kind)) = trainer_face_kinds(index) else {
        return call_original!(index, hud, players);
    };
    FACE_KINDS.store(((base as u32 as u64) << 32) | kind as u32 as u64, Ordering::Release);
    FACE_THREAD.store(current_thread(), Ordering::Release);
    call_original!(index, hud, players);
    FACE_THREAD.store(0, Ordering::Release);
    if log_budget() {
        crate::dbg_log_public(&format!("[switch] HUD face for kind {kind} instead of its base {base}"));
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_KIND_FACE))]
unsafe fn kind_face(kind: i32) -> u64 {
    let thread = FACE_THREAD.load(Ordering::Acquire);
    if thread != 0 && thread == current_thread() {
        let kinds = FACE_KINDS.load(Ordering::Acquire);
        if kind == (kinds >> 32) as i32 {
            return call_original!(kinds as u32 as i32);
        }
    }
    call_original!(kind)
}

unsafe fn request_switch(entry_id: i32) -> bool {
    let Some(leader) = u8::try_from(entry_id).ok().filter(|entry| *entry < 8).and_then(crate::entry_custom_kind) else {
        return false;
    };
    let Some(group) = switch_group(leader) else {
        return false;
    };
    if !matches!(group_shape(&group), Some(Shape::Free | Shape::Aegis)) {
        return false;
    }
    let Some(entry) = entry_struct(entry_id) else {
        return false;
    };
    let count = core::ptr::read_volatile((entry + ENTRY_MEMBER_COUNT) as *const i32);
    let state = core::ptr::read_volatile((entry + ENTRY_STATE) as *const u8);
    let target = core::ptr::read_volatile((entry + ENTRY_TARGET_MEMBER) as *const u8);
    if count < 2 || state != ENTRY_STATE_PLAYING || target != NO_TARGET {
        return false;
    }
    let main = core::ptr::read_volatile((entry + ENTRY_MAIN_MEMBER) as *const u8) as i32;
    let next = ((main + 1) % count) as u8;
    core::ptr::write_volatile((entry + ENTRY_TARGET_MEMBER) as *mut u8, next);
    core::ptr::write_volatile((entry + ENTRY_CHANGE_CARRY) as *mut u8, 0);
    let slot = entry_id as usize;
    PLACE_ENTRY[slot].store(entry, Ordering::Release);
    PLACE_STATE[slot].store(PLACE_REQUESTED, Ordering::Release);
    if !SWITCH_REQUEST_LOGGED.swap(true, Ordering::Relaxed) {
        crate::dbg_log_public(&format!("[switch] kind {leader}: switch requested from member {main} to {next}"));
    }
    true
}

#[no_mangle]
pub extern "C" fn clone_engine_switch_next_v1(entry_id: i32) -> bool {
    unsafe { request_switch(entry_id) }
}

pub(crate) fn install() {
    skyline::install_hooks!(
        group_flags_setup,
        group_articles_setup,
        group_helpers_setup,
        group_member_lookup,
        group_change,
        member_activate,
        trainer_face,
        kind_face,
        win_article_preload,
        entries_teardown,
        entry_teardown,
        entry_restart
    );
}

#[skyline::hook(offset = crate::game::rel(OFF_WIN_ARTICLE_PRELOAD))]
unsafe fn win_article_preload(entry: u64, ending: u64) -> u64 {
    if entry == 0 {
        return call_original!(entry, ending);
    }
    let slot = (entry as usize + ENTRY_FIGHTER_KIND) as *mut i32;
    let kind = core::ptr::read_volatile(slot);
    let Some(base) = crate::clone_base(kind).filter(|base| WIN_PRELOAD_KINDS.contains(base)) else {
        return call_original!(entry, ending);
    };
    core::ptr::write_volatile(slot, base);
    let result = call_original!(entry, ending);
    core::ptr::write_volatile(slot, kind);
    if WIN_PRELOAD_LOG.fetch_add(1, Ordering::Relaxed) < 8 {
        crate::dbg_log_public(&format!(
            "[switch] kind {kind} preloads its victory-screen articles as its base {base}"
        ));
    }
    result
}

pub(crate) fn clone_for(entry: u8, kind: i32) -> Option<i32> {
    let leader = crate::entry_custom_kind(entry)?;
    let definition = crate::clone_definition(leader)?;
    if definition.base_kind == kind || leader == kind {
        return Some(leader);
    }
    member_of(leader, kind).or(Some(leader))
}

pub(crate) unsafe fn clone_of_boma(boma: *mut smash::app::BattleObjectModuleAccessor) -> Option<i32> {
    if boma.is_null() {
        return None;
    }
    let entry = smash::app::lua_bind::WorkModule::get_int(
        boma,
        *smash::lib::lua_const::FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
    );
    let entry = u8::try_from(entry).ok().filter(|entry| *entry < 8)?;
    let object_id = core::ptr::read_volatile((boma as usize + BOMA_OBJECT_ID) as *const u32);
    clone_for_object(entry, smash::app::utility::get_kind(&mut *boma), object_id)
}

pub(crate) fn member_of(leader: i32, kind: i32) -> Option<i32> {
    if crate::clone_definition(leader).is_some_and(|definition| definition.base_kind == kind) {
        return Some(leader);
    }
    members(leader).into_iter().find(|member| {
        crate::clone_definition(*member)
            .is_some_and(|definition| definition.base_kind == kind || definition.kind == kind)
    })
}

pub(crate) unsafe fn member_for_release(context: usize, kind: i32) -> Option<i32> {
    let own = core::ptr::read_volatile((context + CTX_KIND) as *const i32);
    members(own)
        .into_iter()
        .find(|member| crate::clone_definition(*member).is_some_and(|definition| definition.base_kind == kind))
}

pub(crate) enum Release {
    Normal,
    Deferred,
    EarlierFirst(i32),
}

fn log_budget() -> bool {
    LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET
}

pub(crate) fn decline_partner(kind: i32) {
    if let Ok(mut declined) = PARTNER_DECLINED.write() {
        if !declined.contains(&kind) {
            declined.push(kind);
        }
    }
}

fn named_by(partnered: &[Partnered], kind: i32) -> Option<&Partnered> {
    if PARTNER_DECLINED.read().ok()?.contains(&kind) || !switch_members(kind).is_empty() {
        return None;
    }
    partnered.iter().find(|partnered| {
        partnered.leader != kind
            && partnered
                .with
                .as_deref()
                .and_then(crate::clone_definition_from_name)
                .is_some_and(|definition| definition.kind == kind)
    })
}

pub(crate) fn register_partner(kind: i32, with: Option<String>, params: Vec<PartnerParam>) {
    if !params.is_empty() {
        crate::dbg_log_public(&format!("[partner] kind {kind} tunes its partner with {} param(s)", params.len()));
    }
    if let Ok(mut partnered) = PARTNERED.write() {
        partnered.retain(|partnered| partnered.leader != kind);
        partnered.push(Partnered { leader: kind, with, params });
    }
}

fn params_of(leader: i32) -> Vec<PartnerParam> {
    PARTNERED
        .read()
        .ok()
        .and_then(|partnered| {
            partnered
                .iter()
                .find(|partnered| partnered.leader == leader)
                .or_else(|| named_by(&partnered, leader))
                .map(|partnered| partnered.params.clone())
        })
        .unwrap_or_default()
}

fn refuse_partner_once(leader: i32, message: String) {
    let Ok(mut refused) = PARTNER_REFUSED.lock() else {
        return;
    };
    if !refused.contains(&leader) {
        refused.push(leader);
        crate::dbg_log_public(&format!("[partner] kind {leader}: {message}; it plays alone"));
    }
}

pub(crate) fn partner_of(leader: i32) -> Option<i32> {
    let partnered = PARTNERED.read().ok()?;
    let Some(own) = partnered.iter().find(|partnered| partnered.leader == leader) else {
        let named = named_by(&partnered, leader)?.leader;
        if NAMED_LOGGED.lock().is_ok_and(|mut logged| {
            let first = !logged.contains(&leader);
            if first {
                logged.push(leader);
            }
            first
        }) {
            crate::dbg_log_public(&format!(
                "[partner] kind {leader} is kind {named}'s partner; picked on its own it leads and {named} follows"
            ));
        }
        return Some(named);
    };
    let with = &own.with;
    let definition = crate::clone_definition(leader)?;
    let Some(name) = with else {
        return Some(if definition.base_kind == POPO { NANA } else { leader });
    };
    let Some(partner) = crate::clone_definition_from_name(name) else {
        refuse_partner_once(leader, format!("partner {name} is not a registered fighter"));
        return None;
    };
    if partner.kind == leader {
        return Some(leader);
    }
    Some(partner.kind)
}

pub(crate) fn same_base_partner(leader: i32) -> Option<i32> {
    let partner = partner_clone(leader)?;
    let base = crate::clone_definition(leader)?.base_kind;
    (crate::clone_definition(partner)?.base_kind == base).then_some(partner)
}

pub(crate) fn note_member_object(entry_id: i32, object_id: u32, kind: i32) {
    if let Some(index) = u8::try_from(entry_id).ok().filter(|index| *index < 8) {
        SWAPPED.fetch_and(!(1u8 << index), Ordering::AcqRel);
    }
    if let Ok(mut noted) = MEMBER_OBJECTS.lock() {
        noted.retain(|(entry, id, held)| *id != object_id && !(*entry == entry_id && *held == kind));
        noted.push((entry_id, object_id, kind));
    }
}

fn noted_member(entry_id: i32, object_id: u32) -> Option<i32> {
    let noted = MEMBER_OBJECTS.lock().ok()?;
    noted
        .iter()
        .find(|(entry, id, _)| *entry == entry_id && *id == object_id)
        .map(|(_, _, kind)| *kind)
}

pub(crate) unsafe fn same_base_member(entry_id: i32, object_id: u32) -> Option<i32> {
    if object_id == OBJECT_ID_INVALID {
        return None;
    }
    let leader = crate::entry_custom_kind(u8::try_from(entry_id).ok().filter(|entry| *entry < 8)?)?;
    let partner = same_base_partner(leader)?;
    if let Some(kind) = noted_member(entry_id, object_id).filter(|kind| *kind == leader || *kind == partner) {
        return Some(kind);
    }
    let [first, second] = partnered_member_ids(entry_id)?;
    if object_id == second {
        Some(partner)
    } else if object_id == first {
        Some(leader)
    } else {
        None
    }
}

pub(crate) unsafe fn scoped_same_base_partner(
    entry: Option<usize>,
    kind: i32,
) -> Option<&'static crate::CloneDefinition> {
    let leader = entry.filter(|entry| *entry < 8).and_then(|entry| crate::entry_custom_kind(entry as u8))?;
    let partner = same_base_partner(leader)?;
    let definition = crate::clone_definition(partner)?;
    if kind != definition.base_kind && kind != partner {
        return None;
    }
    let scoped = crate::active_construction_kind().or_else(|| crate::active_resource_kind());
    (scoped == Some(partner)).then_some(definition)
}

pub(crate) unsafe fn as_same_base_partner<R>(object: u64, callback: impl FnOnce() -> R) -> R {
    if object == 0 {
        return callback();
    }
    let entry_id = core::ptr::read_volatile((object as usize + OBJECT_ENTRY) as *const i32);
    let object_id = core::ptr::read_volatile((object as usize + OBJECT_ID) as *const u32);
    let leader = u8::try_from(entry_id).ok().filter(|entry| *entry < 8).and_then(crate::entry_custom_kind);
    let partner = same_base_member(entry_id, object_id).filter(|member| Some(*member) != leader);
    let Some(partner) = partner else {
        return callback();
    };
    if !SAME_BASE_LOGGED.swap(true, Ordering::Relaxed) {
        crate::dbg_log_public(&format!(
            "[partner] kind {partner} shares its leader's base; its object {object_id:#x} resolves its own files"
        ));
    }
    crate::with_resource_context(partner, callback)
}

pub(crate) unsafe fn clone_for_object(entry: u8, kind: i32, object_id: u32) -> Option<i32> {
    same_base_member(entry as i32, object_id).or_else(|| clone_for(entry, kind))
}

const FIGHTER_MANAGER: usize = 0x52b84f8;
const MANAGER_ENTRIES: usize = 0x20;
const ENTRY_MEMBER_OBJECTS: usize = 0x4160;
const ENTRY_GROUP_FLAGS: usize = 0x591f;
const GROUP_BOTH_ACTIVE: u8 = 2;
const OBJECT_ID: usize = 0x8;
const BOMA_OBJECT_ID: usize = 0x8;
const OBJECT_ID_INVALID: u32 = 0x5000_0000;

unsafe fn entry_struct(entry: i32) -> Option<usize> {
    if !(0..8).contains(&entry) {
        return None;
    }
    let holder = core::ptr::read_volatile(crate::game::addr(FIGHTER_MANAGER) as *const usize);
    if holder == 0 {
        return None;
    }
    let manager = core::ptr::read_volatile(holder as *const usize);
    if manager == 0 {
        return None;
    }
    let entry = core::ptr::read_volatile((manager + MANAGER_ENTRIES + entry as usize * 8) as *const usize);
    (entry != 0).then_some(entry)
}

pub(crate) unsafe fn entry_is_cpu(entry_id: i32) -> bool {
    let Some(entry) = entry_struct(entry_id) else {
        return false;
    };
    let info = core::ptr::read_volatile((entry + ENTRY_INFO) as *const usize);
    info != 0 && core::ptr::read_volatile((info + INFO_HELPER_MODE) as *const u8) != 0
}

unsafe fn partnered_member_ids(entry_id: i32) -> Option<[u32; 2]> {
    let leader = crate::entry_custom_kind(u8::try_from(entry_id).ok()?)?;
    partner_of(leader)?;
    let entry = entry_struct(entry_id)?;
    if core::ptr::read_volatile((entry + ENTRY_GROUP_FLAGS) as *const u8) & GROUP_BOTH_ACTIVE == 0 {
        return None;
    }
    let mut ids = [OBJECT_ID_INVALID; 2];
    for (index, id) in ids.iter_mut().enumerate() {
        let object = core::ptr::read_volatile((entry + ENTRY_MEMBER_OBJECTS + index * 8) as *const usize);
        if object != 0 {
            *id = core::ptr::read_volatile((object + OBJECT_ID) as *const u32);
        }
    }
    Some(ids)
}

const ENTRY_MAIN_VIEW: usize = 0xf0;
const ENTRY_SUB_VIEW: usize = 0x100;
const VIEW_SIZE: usize = 0x10;
const OBJECT_ENTRY: usize = 0x10;

unsafe fn swap_views(entry: usize) {
    let main = (entry + ENTRY_MAIN_VIEW) as *mut [u8; VIEW_SIZE];
    let sub = (entry + ENTRY_SUB_VIEW) as *mut [u8; VIEW_SIZE];
    core::ptr::swap(main, sub);
}

unsafe fn enter_partner_view(object: u64) -> usize {
    if object == 0 {
        return 0;
    }
    let entry_id = core::ptr::read_volatile((object as usize + OBJECT_ENTRY) as *const i32);
    let Some([_, second]) = partnered_member_ids(entry_id) else {
        return 0;
    };
    if second == OBJECT_ID_INVALID || core::ptr::read_volatile((object as usize + OBJECT_ID) as *const u32) != second {
        return 0;
    }
    if crate::entry_custom_kind(entry_id as u8).is_some_and(nana_partner) {
        return 0;
    }
    let Some(entry) = entry_struct(entry_id) else {
        return 0;
    };
    swap_views(entry);
    entry
}

unsafe fn own_controller(object: u64) {
    if object == 0 {
        return;
    }
    let entry_id = core::ptr::read_volatile((object as usize + OBJECT_ENTRY) as *const i32);
    let Some(ids) = partnered_member_ids(entry_id) else {
        return;
    };
    let id = core::ptr::read_volatile((object as usize + OBJECT_ID) as *const u32);
    let Some(index) = ids.iter().position(|member| *member == id && *member != OBJECT_ID_INVALID) else {
        return;
    };
    let Some(entry) = entry_struct(entry_id) else {
        return;
    };
    let controller = read_pointer(entry + ENTRY_CURRENT_HELPER + index * 8);
    let ai = if controller == 0 { 0 } else { read_pointer(controller + CONTROLLER_AI) };
    let record = if ai == 0 { 0 } else { read_pointer(ai + AI_RECORD) };
    if record == 0 || read_pointer(record + RECORD_OBJECT) != object as usize {
        return;
    }
    let boma = read_pointer(object as usize + OBJECT_BOMA);
    let control = if boma == 0 { 0 } else { read_pointer(boma + BOMA_CONTROL_MODULE) };
    if control == 0 || read_pointer(control + CONTROL_CONTROLLER) == controller {
        return;
    }
    core::ptr::write_volatile((control + CONTROL_CONTROLLER) as *mut usize, controller);
    if !OWN_CONTROLLER_LOGGED.swap(true, Ordering::Relaxed) {
        crate::dbg_log_public(&format!(
            "[partner] entry {entry_id}: member {index} reads its own controller {controller:#x} (the follower replays the player's input)"
        ));
    }
}

unsafe fn leave_partner_view(entry: usize) {
    if entry != 0 {
        swap_views(entry);
    }
}

pub(crate) struct PartnerFrame {
    view: usize,
    payload: usize,
    saved: Vec<(u32, u32)>,
}

unsafe fn apply_partner_params(object: u64) -> (usize, Vec<(u32, u32)>) {
    if object == 0 {
        return (0, Vec::new());
    }
    let entry_id = core::ptr::read_volatile((object as usize + OBJECT_ENTRY) as *const i32);
    let Some(leader) = u8::try_from(entry_id).ok().filter(|entry| *entry < 8).and_then(crate::entry_custom_kind) else {
        return (0, Vec::new());
    };
    let params = params_of(leader);
    if params.is_empty() {
        return (0, Vec::new());
    }
    let Some(payload) = crate::fighter_params::payload_of(POPO) else {
        return (0, Vec::new());
    };
    let saved = params
        .iter()
        .map(|param| {
            let at = (payload + param.offset as usize) as *mut u32;
            let before = core::ptr::read_volatile(at);
            core::ptr::write_volatile(at, param.bits);
            (param.offset, before)
        })
        .collect();
    if !PARTNER_PARAMS_LOGGED.swap(true, Ordering::Relaxed) {
        crate::dbg_log_public(&format!(
            "[partner] kind {leader}: {} partner param(s) apply to the pair's frames over the Ice Climbers' record {payload:#x}",
            params.len()
        ));
    }
    (payload, saved)
}

pub(crate) unsafe fn enter_partner_frame(object: u64) -> PartnerFrame {
    own_controller(object);
    let view = enter_partner_view(object);
    let (payload, saved) = apply_partner_params(object);
    PartnerFrame { view, payload, saved }
}

pub(crate) unsafe fn leave_partner_frame(frame: PartnerFrame) {
    for (offset, bits) in frame.saved.iter().rev() {
        core::ptr::write_volatile((frame.payload + *offset as usize) as *mut u32, *bits);
    }
    leave_partner_view(frame.view);
}

#[no_mangle]
pub extern "C" fn clone_engine_partner_of_v1(entry_id: i32, object_id: u32) -> u32 {
    let Some([first, second]) = (unsafe { partnered_member_ids(entry_id) }) else {
        return OBJECT_ID_INVALID;
    };
    if object_id == first {
        second
    } else if object_id == second {
        first
    } else {
        OBJECT_ID_INVALID
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_is_partner_v1(entry_id: i32, object_id: u32) -> bool {
    unsafe { partnered_member_ids(entry_id) }.is_some_and(|[_, second]| second == object_id && second != OBJECT_ID_INVALID)
}

const OBJECT_BOMA: usize = 0x20;
const BOMA_CONTROL_MODULE: usize = 0x48;
const CONTROL_CONTROLLER: usize = 0x118;
const CONTROLLER_AI: usize = 0x58;
const AI_RECORD: usize = 0x168;
const RECORD_OBJECT: usize = 0x10;
const RECORD_AI: usize = 0x20;
const RECORD_SWAPPED: [usize; 2] = [0x10, 0x28];
const OFF_ENTRY_TEARDOWN: usize = 0x6524e0;
const OFF_ENTRIES_TEARDOWN: usize = 0x66e850;
const OFF_ENTRY_RESTART: usize = 0x654570;

static SWAPPED: AtomicU8 = AtomicU8::new(0);
const SWAP_COOLDOWN: std::time::Duration = std::time::Duration::from_millis(300);

static LAST_SWAP: Mutex<[Option<std::time::Instant>; 8]> = Mutex::new([None; 8]);
static SWAP_LOG: AtomicU32 = AtomicU32::new(0);

unsafe fn read_pointer(address: usize) -> usize {
    core::ptr::read_volatile(address as *const usize)
}

unsafe fn swap_words(first: usize, second: usize) {
    let held = core::ptr::read_volatile(first as *const u64);
    core::ptr::write_volatile(first as *mut u64, core::ptr::read_volatile(second as *const u64));
    core::ptr::write_volatile(second as *mut u64, held);
}

unsafe fn in_play(object: usize) -> bool {
    let boma = read_pointer(object + OBJECT_BOMA);
    if boma == 0 {
        return false;
    }
    let status = smash::app::lua_bind::StatusModule::status_kind(boma as *mut smash::app::BattleObjectModuleAccessor);
    status != *smash::lib::lua_const::FIGHTER_STATUS_KIND_DEAD
        && status != *smash::lib::lua_const::FIGHTER_STATUS_KIND_STANDBY
}

unsafe fn control_record(object: usize) -> Option<(usize, usize)> {
    let control = read_pointer(read_pointer(object + OBJECT_BOMA) + BOMA_CONTROL_MODULE);
    if control == 0 {
        return None;
    }
    let controller = read_pointer(control + CONTROL_CONTROLLER);
    let ai = if controller == 0 { 0 } else { read_pointer(controller + CONTROLLER_AI) };
    let record = if ai == 0 { 0 } else { read_pointer(ai + AI_RECORD) };
    (record != 0 && read_pointer(record + RECORD_AI) == ai && read_pointer(record + RECORD_OBJECT) == object)
        .then_some((control, record))
}

unsafe fn exchange(entry: usize, controls: [usize; 2], records: [usize; 2]) {
    swap_words(controls[0] + CONTROL_CONTROLLER, controls[1] + CONTROL_CONTROLLER);
    for offset in RECORD_SWAPPED {
        swap_words(records[0] + offset, records[1] + offset);
    }
    swap_words(entry + ENTRY_MEMBER_OBJECTS, entry + ENTRY_MEMBER_OBJECTS + 8);
}

unsafe fn restore_order(entry: usize) -> String {
    swap_words(entry + ENTRY_MEMBER_OBJECTS, entry + ENTRY_MEMBER_OBJECTS + 8);
    let objects = [read_pointer(entry + ENTRY_MEMBER_OBJECTS), read_pointer(entry + ENTRY_MEMBER_OBJECTS + 8)];
    let controllers = [read_pointer(entry + ENTRY_CURRENT_HELPER), read_pointer(entry + ENTRY_CURRENT_HELPER + 8)];
    let mut controls = 0;
    for (object, controller) in objects.iter().zip(controllers.iter()) {
        let boma = if *object == 0 { 0 } else { read_pointer(object + OBJECT_BOMA) };
        let control = if boma == 0 { 0 } else { read_pointer(boma + BOMA_CONTROL_MODULE) };
        if control != 0 && *controller != 0 {
            core::ptr::write_volatile((control + CONTROL_CONTROLLER) as *mut usize, *controller);
            controls += 1;
        }
    }
    let records: Vec<usize> = controllers
        .iter()
        .map(|controller| {
            let ai = if *controller == 0 { 0 } else { read_pointer(controller + CONTROLLER_AI) };
            let record = if ai == 0 { 0 } else { read_pointer(ai + AI_RECORD) };
            if record != 0 && read_pointer(record + RECORD_AI) == ai { record } else { 0 }
        })
        .collect();
    let crossed = records.iter().all(|record| *record != 0)
        && read_pointer(records[0] + RECORD_OBJECT) == objects[1]
        && read_pointer(records[1] + RECORD_OBJECT) == objects[0];
    if crossed {
        for offset in RECORD_SWAPPED {
            swap_words(records[0] + offset, records[1] + offset);
        }
    }
    let seen: Vec<String> = records
        .iter()
        .map(|record| if *record == 0 { "none".to_string() } else { format!("{:#x}", read_pointer(record + RECORD_OBJECT)) })
        .collect();
    format!("objects {objects:x?}, controls reset {controls}, records crossed {crossed}, record objects {seen:?}")
}

pub(crate) unsafe fn unswap(index: u8, when: &str) {
    if index >= 8 {
        return;
    }
    let bit = 1u8 << index;
    if SWAPPED.fetch_and(!bit, Ordering::AcqRel) & bit == 0 {
        return;
    }
    let Some(entry) = entry_struct(index as i32) else {
        return;
    };
    let objects = [read_pointer(entry + ENTRY_MEMBER_OBJECTS), read_pointer(entry + ENTRY_MEMBER_OBJECTS + 8)];
    let found = if objects.contains(&0) { None } else { control_record(objects[0]).zip(control_record(objects[1])) };
    let report = match found {
        Some(((first_control, first_record), (second_control, second_record))) => {
            exchange(entry, [first_control, second_control], [first_record, second_record]);
            "controllers and records".to_string()
        }
        None => restore_order(entry),
    };
    crate::dbg_log_public(&format!("[partner] entry {index}: back to the leader in front {when} ({report})"));
}

#[skyline::hook(offset = crate::game::rel(OFF_ENTRIES_TEARDOWN))]
unsafe fn entries_teardown(manager: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    for index in 0..8u8 {
        unswap(index, "as the match ends");
    }
    crate::fight_commands::forget_all();
    call_original!(manager, a1, a2, a3)
}

#[skyline::hook(offset = crate::game::rel(OFF_ENTRY_TEARDOWN))]
unsafe fn entry_teardown(entry: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    if entry != 0 {
        let entry_id = (0..8).find(|index| entry_struct(*index) == Some(entry as usize)).unwrap_or(-1);
        let bit = u8::try_from(entry_id).ok().filter(|index| *index < 8).map(|index| 1u8 << index);
        if bit.is_some_and(|bit| SWAPPED.fetch_and(!bit, Ordering::AcqRel) & bit != 0) {
            let report = restore_order(entry as usize);
            crate::dbg_log_public(&format!(
                "[partner] entry {entry_id}: back to the leader in front before the entry is torn down ({report})"
            ));
        }
    }
    call_original!(entry, a1, a2, a3)
}

#[skyline::hook(offset = crate::game::rel(OFF_ENTRY_RESTART))]
unsafe fn entry_restart(entry: u64, mode: u64) -> u64 {
    if let Some(index) = (0..8u8).find(|index| entry_struct(*index as i32) == Some(entry as usize)) {
        unswap(index, "as the fighter restarts");
    }
    crate::fight_commands::forget_all();
    call_original!(entry, mode)
}

unsafe fn swap_partner(entry_id: i32) -> Result<(), &'static str> {
    let index = u8::try_from(entry_id).ok().filter(|entry| *entry < 8).ok_or("no such entry")?;
    let leader = crate::entry_custom_kind(index).ok_or("not a clone")?;
    partner_of(leader).ok_or("no partner")?;
    if nana_partner(leader) {
        return Err("Nana's own code decides her KO, so an Ice Climbers clone keeps Popo in front");
    }
    let entry = entry_struct(entry_id).ok_or("no entry")?;
    if core::ptr::read_volatile((entry + ENTRY_GROUP_FLAGS) as *const u8) & GROUP_BOTH_ACTIVE == 0 {
        return Err("the pair is not active");
    }
    let objects = [
        read_pointer(entry + ENTRY_MEMBER_OBJECTS),
        read_pointer(entry + ENTRY_MEMBER_OBJECTS + 8),
    ];
    if objects.contains(&0) || !objects.iter().all(|object| in_play(*object)) {
        return Err("both must be in play");
    }
    let (Some((first_control, first_record)), Some((second_control, second_record))) =
        (control_record(objects[0]), control_record(objects[1]))
    else {
        return Err("controllers not as expected");
    };
    let now = std::time::Instant::now();
    let mut last = LAST_SWAP.lock().map_err(|_| "busy")?;
    if last[index as usize].is_some_and(|at| now.duration_since(at) < SWAP_COOLDOWN) {
        return Err("too soon after the last swap");
    }
    last[index as usize] = Some(now);
    exchange(entry, [first_control, second_control], [first_record, second_record]);
    SWAPPED.fetch_xor(1 << index, Ordering::AcqRel);
    Ok(())
}

#[no_mangle]
pub extern "C" fn clone_engine_partner_swap_v1(entry_id: i32) -> bool {
    let result = unsafe { swap_partner(entry_id) };
    if SWAP_LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        match result {
            Ok(()) => crate::dbg_log_public(&format!("[partner] entry {entry_id}: the player now plays the other one")),
            Err(reason) => crate::dbg_log_public(&format!("[partner] entry {entry_id}: swap refused, {reason}")),
        }
    }
    result.is_ok()
}

pub(crate) unsafe fn prepare_partner(leader: i32) {
    if !matches!(partner_of(leader), Some(partner) if partner != NANA) {
        return;
    }
    let loaded = crate::fighter_params::ensure_kind_params_loaded(POPO);
    if log_budget() {
        crate::dbg_log_public(&format!(
            "[partner] kind {leader}: the partner follows by the Ice Climbers' nana_* params (loaded {loaded})"
        ));
    }
}

pub(crate) fn nana_partner(leader: i32) -> bool {
    partner_of(leader) == Some(NANA)
}

pub(crate) fn partner_clone(leader: i32) -> Option<i32> {
    partner_of(leader).filter(|partner| *partner != NANA && *partner != leader)
}

pub(crate) fn leader_of_entry(entry: u8) -> Option<i32> {
    crate::entry_custom_kind(entry).filter(|kind| nana_partner(*kind))
}

pub(crate) fn resource_kind(entry: Option<usize>, kind: i32) -> i32 {
    if kind != NANA {
        return kind;
    }
    entry
        .filter(|entry| *entry < 8)
        .and_then(|entry| leader_of_entry(entry as u8))
        .and_then(crate::clone_definition)
        .map(|definition| definition.base_kind)
        .unwrap_or(kind)
}

pub(crate) unsafe fn leader_of_object(object: u64) -> Option<i32> {
    if object == 0 || core::ptr::read_volatile((object + 0xc) as *const i32) != NANA {
        return None;
    }
    let entry = core::ptr::read_volatile((object + 0x10) as *const i32);
    (0..8).contains(&entry).then(|| leader_of_entry(entry as u8)).flatten()
}

pub(crate) unsafe fn expand(kind: i32, out: *mut i32, count: i32) -> i32 {
    if let Some(owner) = solo_owner(kind) {
        crate::fighter_modules::request(owner);
    }
    let members = switch_members(kind);
    if !members.is_empty() && !out.is_null() && count == 1 {
        let mut bases: Vec<i32> = crate::clone_definition(kind).map(|definition| definition.base_kind).into_iter().collect();
        for (index, member) in members.iter().enumerate() {
            core::ptr::write_volatile(out.add(1 + index), *member);
            if let Some(definition) = crate::clone_definition(*member) {
                crate::fighter_modules::request(definition.base_kind);
                bases.push(definition.base_kind);
            }
        }
        if let Some(owner) = shared_owner(&bases) {
            crate::fighter_modules::request(owner);
        }
        if log_budget() {
            crate::dbg_log_public(&format!("[switch] kind {kind} enters the match with {members:?}"));
        }
        return 1 + members.len() as i32;
    }
    let Some(partner) = partner_of(kind) else {
        return count;
    };
    if out.is_null() || count != 1 {
        return count;
    }
    core::ptr::write_volatile(out.add(1), partner);
    match crate::clone_definition(partner) {
        Some(definition) => crate::fighter_modules::request(definition.base_kind),
        None => crate::fighter_modules::request(partner),
    };
    if log_budget() {
        let what = if partner == NANA {
            "Nana's code, its own files".to_string()
        } else if partner == kind {
            "a copy of itself".to_string()
        } else {
            format!("fighter kind {partner}")
        };
        crate::dbg_log_public(&format!("[partner] kind {kind} enters the match with its partner ({what})"));
    }
    2
}

pub(crate) fn roster_kind(custom_kind: i32) -> Option<u64> {
    partner_of(custom_kind).map(|_| ICE_CLIMBER)
}

pub(crate) unsafe fn release(context: usize, kind: i32) -> Release {
    let own = core::ptr::read_volatile((context + CTX_KIND) as *const i32);
    let Some(partner) = partner_of(own) else {
        return Release::Normal;
    };
    let second = core::ptr::read_volatile((context + CTX_SECOND_KIND) as *const i32);
    if second != partner {
        return Release::Normal;
    }
    let Ok(mut deferred) = DEFERRED.lock() else {
        return Release::Normal;
    };
    let Some(at) = deferred.iter().position(|(held, _)| *held == context) else {
        deferred.push((context, kind));
        return Release::Deferred;
    };
    let (_, earlier) = deferred.remove(at);
    if earlier == kind {
        Release::Normal
    } else {
        Release::EarlierFirst(earlier)
    }
}

pub(crate) unsafe fn hold_partner_agent_name(object: u64) -> crate::smashline_names::NameGuard {
    match leader_of_object(object).and_then(crate::clone_definition) {
        Some(definition) => crate::smashline_names::hold_fighter_name(NANA, definition.resource_name_cstr),
        None => crate::smashline_names::NameGuard::inactive(),
    }
}
