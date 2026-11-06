use smash::app::lua_bind::{
    AttackModule, CancelModule, ControlModule, FighterControlModuleImpl, FighterStatusModuleImpl, GroundModule,
    ItemModule, KineticEnergy, KineticModule, LuaModule, MotionModule, PostureModule, StatusModule, WorkModule,
};
use smash::app::{sv_battle_object, sv_module_access, BattleObjectModuleAccessor, GroundCliffCheckKind, SituationKind};
use smash::lib::lua_const::*;
use smash::lib::{L2CValue, L2CValueInner, L2CValueType};
use smash::lua2cpp::L2CFighterCommon;
use smash::phx::{Hash40, Vector2f, Vector3f};
use std::sync::atomic::AtomicUsize;
use std::sync::Mutex;

use crate::{resolve, CloneKind};

const PREV_STATUS_KIND: i32 = 0xA;
const CURRENT_FRAME: i32 = 0xE;
const SITUATION_KIND: i32 = 0x16;
const STICK_Y: i32 = 0x1B;
const PAD_FLAG: i32 = 0x1F;
const CMD_CAT1: i32 = 0x20;
const AIR_JUMP_UNIQ: i32 = 0x32;
const AIR_JUMP_AERIAL_UNIQ: i32 = 0x33;

const FALL_CONTROL: u8 = 1;
const ATTACK_AIR_CONTROL: u8 = 2;
const FALL_TIME: u8 = 4;
const ATTACK_AIR_TIME: u8 = 8;
const ITEM_THROW: u8 = 16;

const IDLE: i32 = 2;
const AERIAL: i32 = 1;

const AERIALS: [u64; 5] = [
    smash::hash40("attack_air_n"),
    smash::hash40("attack_air_f"),
    smash::hash40("attack_air_b"),
    smash::hash40("attack_air_hi"),
    smash::hash40("attack_air_lw"),
];
const FALL: u64 = smash::hash40("fall");
const LOG_ACTION: u64 = 0x2b94de0d96;

const LINE_PRE: i32 = 0;
const LINE_MAIN: i32 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Float {
    pub frames: i32,
    pub start_y: f32,
    pub water_height: f32,
    pub start_motion: u64,
    pub motion: u64,
    pub item: i32,
}

impl Float {
    pub const PEACH: Float = Float {
        frames: 150,
        start_y: 1.5,
        water_height: 14.0,
        start_motion: smash::hash40("fuwafuwa_start"),
        motion: smash::hash40("fuwafuwa"),
        item: -1,
    };
    pub const DAISY: Float = Float::PEACH;

    pub const fn frames(mut self, frames: i32) -> Float {
        self.frames = frames;
        self
    }

    pub const fn start_y(mut self, start_y: f32) -> Float {
        self.start_y = start_y;
        self
    }

    pub const fn water_height(mut self, water_height: f32) -> Float {
        self.water_height = water_height;
        self
    }

    pub const fn motions(mut self, start_motion: u64, motion: u64) -> Float {
        self.start_motion = start_motion;
        self.motion = motion;
        self
    }

    pub const fn item(mut self, item_kind: i32) -> Float {
        self.item = item_kind;
        self
    }
}

struct Entry {
    name: &'static str,
    kind: CloneKind,
    float: Float,
    start: i32,
    uniq: [usize; 2],
}

#[derive(Clone, Copy)]
struct Floater {
    float: Float,
    start: i32,
    uniq: [usize; 2],
}

#[derive(Clone, Copy, Default)]
struct State {
    id: u32,
    used: bool,
    water: Option<f32>,
    ray: bool,
    frame: i32,
    enabled: bool,
    mtrans: i32,
    terms: u8,
    aerial_loop: usize,
    last_status: i32,
}

type StatusFn = unsafe extern "C" fn(&mut L2CFighterCommon) -> L2CValue;
type OriginalFn = unsafe extern "C" fn(*mut L2CFighterCommon, i32, i32) -> usize;

static ENTRIES: Mutex<Vec<Entry>> = Mutex::new(Vec::new());
static STATES: Mutex<Vec<State>> = Mutex::new(Vec::new());
static ORIGINAL_FN: AtomicUsize = AtomicUsize::new(0);

pub fn register(resource_name: &'static str, float: Float, start_status: i32) {
    if let Ok(mut entries) = ENTRIES.lock() {
        entries.retain(|entry| entry.name != resource_name);
        entries.push(Entry {
            name: resource_name,
            kind: CloneKind::new(resource_name),
            float,
            start: start_status,
            uniq: [0; 2],
        });
    }
}

unsafe fn floater(boma: *mut BattleObjectModuleAccessor) -> Option<Floater> {
    let kind = crate::fighter_kind(boma as u64);
    if kind < 0 {
        return None;
    }
    let entries = ENTRIES.lock().ok()?;
    entries.iter().find(|entry| entry.kind.get() == Some(kind)).map(|entry| Floater {
        float: entry.float,
        start: entry.start,
        uniq: entry.uniq,
    })
}

unsafe fn object_id(boma: *mut BattleObjectModuleAccessor) -> u32 {
    (*boma).battle_object_id
}

fn load(id: u32) -> State {
    STATES
        .lock()
        .ok()
        .and_then(|states| states.iter().find(|state| state.id == id).copied())
        .unwrap_or(State { id, ..State::default() })
}

fn store(state: State) {
    if let Ok(mut states) = STATES.lock() {
        match states.iter_mut().find(|slot| slot.id == state.id) {
            Some(slot) => *slot = state,
            None => states.push(state),
        }
    }
}

fn pointer(function: usize) -> L2CValue {
    L2CValue { val_type: L2CValueType::Pointer, unk1: 0, inner: L2CValueInner { raw: function as u64 }, unk2: 0 }
}

fn hash_value(hash: u64) -> L2CValue {
    L2CValue { val_type: L2CValueType::Hash, unk1: 0, inner: L2CValueInner { raw: hash }, unk2: 0 }
}

fn pointer_of(value: &L2CValue) -> usize {
    if value.val_type == L2CValueType::Pointer {
        unsafe { value.inner.raw as usize }
    } else {
        0
    }
}

fn truthy(value: &L2CValue) -> bool {
    match value.val_type {
        L2CValueType::Void => false,
        L2CValueType::Bool => unsafe { value.inner.raw & 1 != 0 },
        L2CValueType::Int => unsafe { value.inner.raw != 0 },
        L2CValueType::Num => unsafe { value.inner.raw_float != 0.0 },
        _ => true,
    }
}

unsafe fn global_int(fighter: &mut L2CFighterCommon, index: i32) -> i64 {
    let value = &fighter.global_table[index];
    match value.val_type {
        L2CValueType::Int => value.inner.raw as i64,
        L2CValueType::Bool => (value.inner.raw & 1) as i64,
        L2CValueType::Num => value.inner.raw_float as i64,
        _ => 0,
    }
}

unsafe fn global_num(fighter: &mut L2CFighterCommon, index: i32) -> f32 {
    let value = &fighter.global_table[index];
    match value.val_type {
        L2CValueType::Num => value.inner.raw_float,
        L2CValueType::Int => value.inner.raw as i64 as f32,
        _ => 0.0,
    }
}

unsafe fn common(boma: *mut BattleObjectModuleAccessor, name: &str) -> f32 {
    WorkModule::get_param_float(boma, smash::hash40("common"), smash::hash40(name))
}

unsafe fn motion_or(boma: *mut BattleObjectModuleAccessor, wanted: u64, fallback: u64) -> u64 {
    if MotionModule::is_anim_resource(boma, Hash40::new_raw(wanted)) {
        wanted
    } else {
        fallback
    }
}

unsafe fn loop_motion(boma: *mut BattleObjectModuleAccessor, float: &Float) -> u64 {
    motion_or(boma, float.motion, FALL)
}

unsafe fn start_motion(boma: *mut BattleObjectModuleAccessor, float: &Float) -> u64 {
    motion_or(boma, float.start_motion, loop_motion(boma, float))
}

unsafe fn change_motion(boma: *mut BattleObjectModuleAccessor, motion: u64) {
    MotionModule::change_motion(boma, Hash40::new_raw(motion), 0.0, 1.0, false, 0.0, false, false);
}

unsafe fn change_status(fighter: &mut L2CFighterCommon, status: i32, keep: bool) {
    fighter.change_status(status.into(), keep.into());
}

unsafe fn original(fighter: &mut L2CFighterCommon, line: i32, kind: i32) -> Option<StatusFn> {
    let address = resolve(&ORIGINAL_FN, b"smashline_get_original_status\0")?;
    let function: OriginalFn = core::mem::transmute(address);
    let found = function(fighter as *mut L2CFighterCommon, line, kind);
    (found != 0).then(|| core::mem::transmute::<usize, StatusFn>(found))
}

unsafe fn log(fighter: &mut L2CFighterCommon, category: i32, kind: i32) {
    fighter.clear_lua_stack();
    let mut action = hash_value(LOG_ACTION);
    let mut category: L2CValue = category.into();
    let mut kind: L2CValue = kind.into();
    fighter.push_lua_stack(&mut action);
    fighter.push_lua_stack(&mut category);
    fighter.push_lua_stack(&mut kind);
    sv_battle_object::notify_event_msc_cmd(fighter.lua_state_agent);
    fighter.pop_lua_stack(1);
}

unsafe fn unable_cancel(fighter: &mut L2CFighterCommon) {
    fighter.clear_lua_stack();
    let mut command: L2CValue = (*MA_MSC_CMD_CANCEL_UNABLE_CANCEL).into();
    fighter.push_lua_stack(&mut command);
    sv_module_access::cancel(fighter.lua_state_agent);
    fighter.pop_lua_stack(1);
}

unsafe fn may_float(boma: *mut BattleObjectModuleAccessor, state: &State) -> bool {
    !state.used && state.water.is_none() && WorkModule::is_enable_transition_term(boma, *FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_JUMP_AERIAL_BUTTON)
}

unsafe fn begin(fighter: &mut L2CFighterCommon, floater: &Floater, mut state: State) {
    let boma = fighter.module_accessor;
    state.ray = StatusModule::status_kind(boma) == *FIGHTER_STATUS_KIND_JUMP && global_int(fighter, CURRENT_FRAME) == 0;
    store(state);
    change_status(fighter, floater.start, true);
}

unsafe fn air_jump_check(fighter: &mut L2CFighterCommon, floater: &Floater) -> bool {
    let boma = fighter.module_accessor;
    let attack = global_int(fighter, PAD_FLAG) & *FIGHTER_PAD_FLAG_ATTACK_TRIGGER as i64 != 0;
    let state = load(object_id(boma));
    if !may_float(boma, &state) || attack {
        return false;
    }
    if !ControlModule::check_button_on(boma, *CONTROL_PAD_BUTTON_JUMP) {
        return false;
    }
    if global_num(fighter, STICK_Y) > common(boma, "squat_stick_y") {
        return false;
    }
    if global_int(fighter, SITUATION_KIND) != *SITUATION_KIND_AIR as i64 {
        return false;
    }
    begin(fighter, floater, state);
    true
}

unsafe fn air_jump_aerial_check(fighter: &mut L2CFighterCommon, floater: &Floater) -> bool {
    let boma = fighter.module_accessor;
    let attack = global_int(fighter, PAD_FLAG) & *FIGHTER_PAD_FLAG_ATTACK_TRIGGER as i64 != 0;
    let state = load(object_id(boma));
    if state.used || state.water.is_some() || attack {
        return false;
    }
    let falling = KineticModule::get_sum_speed_y(boma, *KINETIC_ENERGY_RESERVE_ATTRIBUTE_MAIN) < 0.0;
    let air = global_int(fighter, SITUATION_KIND) == *SITUATION_KIND_AIR as i64;
    if WorkModule::is_enable_transition_term(boma, *FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_JUMP_AERIAL_BUTTON)
        && ControlModule::check_button_on(boma, *CONTROL_PAD_BUTTON_JUMP)
        && falling
        && air
    {
        begin(fighter, floater, state);
        return true;
    }
    if WorkModule::is_enable_transition_term(boma, *FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_JUMP_AERIAL)
        && common(boma, "jump_stick_y") <= global_num(fighter, STICK_Y)
        && falling
        && air
    {
        begin(fighter, floater, state);
        return true;
    }
    false
}

unsafe fn uniq(fighter: &mut L2CFighterCommon, floater: Option<Floater>, slot: usize) -> L2CValue {
    match floater.map(|floater| floater.uniq[slot]).filter(|function| *function != 0) {
        Some(function) => core::mem::transmute::<usize, StatusFn>(function)(fighter),
        None => false.into(),
    }
}

pub unsafe extern "C" fn air_jump(fighter: &mut L2CFighterCommon) -> L2CValue {
    let floater = floater(fighter.module_accessor);
    if let Some(floater) = floater {
        if air_jump_check(fighter, &floater) {
            return true.into();
        }
    }
    uniq(fighter, floater, 0)
}

pub unsafe extern "C" fn air_jump_aerial(fighter: &mut L2CFighterCommon) -> L2CValue {
    let floater = floater(fighter.module_accessor);
    if let Some(floater) = floater {
        if air_jump_aerial_check(fighter, &floater) {
            return true.into();
        }
    }
    uniq(fighter, floater, 1)
}

pub unsafe extern "C" fn on_start(fighter: &mut L2CFighterCommon) {
    let boma = fighter.module_accessor;
    let kind = crate::fighter_kind(boma as u64);
    store(State { id: object_id(boma), ..State::default() });
    let Ok(mut entries) = ENTRIES.lock() else {
        return;
    };
    let Some(entry) = entries.iter_mut().find(|entry| entry.kind.get() == Some(kind)) else {
        return;
    };
    for (index, (slot, ours)) in [(AIR_JUMP_UNIQ, air_jump as *const () as usize), (AIR_JUMP_AERIAL_UNIQ, air_jump_aerial as *const () as usize)]
        .into_iter()
        .enumerate()
    {
        let current = pointer_of(&fighter.global_table[slot]);
        if current != ours && current != 0 {
            entry.uniq[index] = current;
        }
        fighter.global_table[slot].assign(&pointer(ours));
    }
}

pub unsafe extern "C" fn frame(fighter: &mut L2CFighterCommon) {
    let boma = fighter.module_accessor;
    let Some(floater) = floater(boma) else {
        return;
    };
    let mut state = load(object_id(boma));
    let situation = StatusModule::situation_kind(boma);
    if situation != *SITUATION_KIND_AIR {
        if situation != *SITUATION_KIND_CLIFF {
            state.used = false;
        }
        state.water = None;
    }
    let status = StatusModule::status_kind(boma);
    let y = (*PostureModule::pos(boma)).y;
    if status == *FIGHTER_STATUS_KIND_JUMP && state.last_status != status {
        state.water = (StatusModule::prev_situation_kind(boma) == *SITUATION_KIND_WATER).then_some(y);
    }
    if let Some(start) = state.water {
        if y - start > floater.float.water_height * PostureModule::scale(boma) {
            state.water = None;
        }
    }
    state.last_status = status;
    store(state);
}

unsafe fn pre(fighter: &mut L2CFighterCommon, kinetic: i32) -> L2CValue {
    let boma = fighter.module_accessor;
    StatusModule::init_settings(
        boma,
        SituationKind(*SITUATION_KIND_AIR),
        kinetic,
        *GROUND_CORRECT_KIND_AIR as u32,
        GroundCliffCheckKind(*GROUND_CLIFF_CHECK_KIND_NONE),
        true,
        *FIGHTER_STATUS_WORK_KEEP_FLAG_NONE_FLAG,
        *FIGHTER_STATUS_WORK_KEEP_FLAG_ALL_INT,
        *FIGHTER_STATUS_WORK_KEEP_FLAG_NONE_FLOAT,
        0,
    );
    FighterStatusModuleImpl::set_fighter_status_data(
        boma,
        false,
        *FIGHTER_TREADED_KIND_NO_REAC,
        false,
        false,
        false,
        0,
        *FIGHTER_STATUS_ATTR_INTO_DOOR as u32,
        *FIGHTER_POWER_UP_ATTACK_BIT_ATTACK_AIR as u32,
        0,
    );
    0.into()
}

pub unsafe extern "C" fn start_pre(fighter: &mut L2CFighterCommon) -> L2CValue {
    pre(fighter, *FIGHTER_KINETIC_TYPE_UNIQ)
}

pub unsafe extern "C" fn float_pre(fighter: &mut L2CFighterCommon) -> L2CValue {
    pre(fighter, *FIGHTER_KINETIC_TYPE_UNIQ)
}

pub unsafe extern "C" fn start_init(fighter: &mut L2CFighterCommon) -> L2CValue {
    let boma = fighter.module_accessor;
    let x = KineticModule::get_sum_speed_x(boma, *KINETIC_ENERGY_RESERVE_ATTRIBUTE_MAIN);
    let control_no_stop = KineticModule::is_enable_energy(boma, *FIGHTER_KINETIC_ENERGY_ID_CONTROL_NO_STOP);
    KineticModule::unable_energy_all(boma);
    for id in [
        *FIGHTER_KINETIC_ENERGY_ID_DAMAGE,
        *FIGHTER_KINETIC_ENERGY_ID_DAMAGE_NO_STOP,
        *FIGHTER_KINETIC_ENERGY_ID_GROUND_MOVEMENT,
    ] {
        KineticModule::enable_energy(boma, id);
    }
    if control_no_stop {
        KineticModule::enable_energy(boma, *FIGHTER_KINETIC_ENERGY_ID_CONTROL_NO_STOP);
    }
    let control = KineticModule::get_energy(boma, *FIGHTER_KINETIC_ENERGY_ID_CONTROL);
    KineticEnergy::reset_energy(
        control as *mut _,
        *ENERGY_CONTROLLER_RESET_TYPE_FALL_ADJUST,
        &Vector2f { x, y: 0.0 },
        &Vector3f { x: 0.0, y: 0.0, z: 0.0 },
        boma,
    );
    KineticModule::enable_energy(boma, *FIGHTER_KINETIC_ENERGY_ID_CONTROL);
    let outside = !WorkModule::is_flag(boma, *FIGHTER_STATUS_WORK_ID_FLAG_RESERVE_DISABLE_OUTSIDE_ENERGY);
    for id in [*FIGHTER_KINETIC_ENERGY_ID_ENV_WIND, *FIGHTER_KINETIC_ENERGY_ID_JOSTLE] {
        if outside {
            KineticModule::enable_energy(boma, id);
        } else {
            KineticModule::unable_energy(boma, id);
        }
    }
    WorkModule::off_flag(boma, *FIGHTER_STATUS_WORK_ID_FLAG_RESERVE_DISABLE_OUTSIDE_ENERGY);
    0.into()
}

unsafe fn open(fighter: &mut L2CFighterCommon, state: &mut State) {
    let boma = fighter.module_accessor;
    WorkModule::enable_transition_term(boma, *FIGHTER_STATUS_TRANSITION_TERM_ID_LANDING_ATTACK_AIR);
    state.terms = FALL_CONTROL | ATTACK_AIR_CONTROL | FALL_TIME | ATTACK_AIR_TIME | ITEM_THROW;
    WorkModule::enable_transition_term_group(boma, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_LANDING);
    WorkModule::enable_transition_term_group(boma, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_SPECIAL);
}

unsafe fn hover(fighter: &mut L2CFighterCommon, state: &mut State) {
    let boma = fighter.module_accessor;
    WorkModule::enable_transition_term_group(boma, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_SPECIAL);
    state.terms |= ITEM_THROW;
    log(fighter, *FIGHTER_LOG_ACTION_CATEGORY_NONE, *FIGHTER_LOG_ACTION_KIND_NONE);
    state.mtrans = IDLE;
    state.terms = (state.terms | FALL_CONTROL | FALL_TIME) & !(ATTACK_AIR_CONTROL | ATTACK_AIR_TIME);
    WorkModule::unable_transition_term(boma, *FIGHTER_STATUS_TRANSITION_TERM_ID_LANDING_ATTACK_AIR);
}

unsafe fn aerial(fighter: &mut L2CFighterCommon, state: &mut State) {
    let boma = fighter.module_accessor;
    let motion = fighter.sub_attack_air_kind_set_log_info();
    if matches!(motion.val_type, L2CValueType::Hash | L2CValueType::Int) {
        change_motion(boma, motion.inner.raw & 0xff_ffff_ffff);
    }
    unable_cancel(fighter);
    ControlModule::clear_command(boma, false);
    WorkModule::unable_transition_term_group(boma, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_SPECIAL);
    state.terms = (state.terms | ATTACK_AIR_CONTROL | ATTACK_AIR_TIME) & !(ITEM_THROW | FALL_CONTROL | FALL_TIME);
    WorkModule::enable_transition_term(boma, *FIGHTER_STATUS_TRANSITION_TERM_ID_LANDING_ATTACK_AIR);
    state.mtrans = AERIAL;
}

unsafe fn hover_or_aerial(fighter: &mut L2CFighterCommon, state: &mut State) {
    let attack = global_int(fighter, CMD_CAT1) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_N as i64 != 0;
    if attack && state.mtrans != AERIAL {
        aerial(fighter, state);
    } else {
        hover(fighter, state);
    }
}

unsafe fn stiff(boma: *mut BattleObjectModuleAccessor) {
    WorkModule::off_flag(boma, *FIGHTER_INSTANCE_WORK_ID_FLAG_DAMAGE_FLY_AIR);
    AttackModule::set_shield_stiff_mul(boma, common(boma, "shield_stiff_mul_attack_air"));
}

pub unsafe extern "C" fn start_main(fighter: &mut L2CFighterCommon) -> L2CValue {
    let boma = fighter.module_accessor;
    let Some(floater) = floater(boma) else {
        return 0.into();
    };
    let mut state = load(object_id(boma));
    open(fighter, &mut state);
    state.frame = floater.float.frames;
    state.enabled = true;
    state.used = true;
    change_motion(boma, start_motion(boma, &floater.float));
    stiff(boma);
    hover_or_aerial(fighter, &mut state);
    if state.ray {
        let pos = *PostureModule::pos(boma);
        let mut hit = Vector2f { x: 0.0, y: 0.0 };
        if GroundModule::ray_check_hit_pos(
            boma,
            &Vector2f { x: pos.x, y: pos.y },
            &Vector2f { x: 0.0, y: -6.0 },
            &mut hit,
            true,
        ) {
            PostureModule::set_pos(boma, &Vector3f { x: pos.x, y: hit.y + floater.float.start_y, z: pos.z });
        }
        state.ray = false;
    }
    store(state);
    fighter.sub_shift_status_main(pointer(float_loop as *const () as usize))
}

pub unsafe extern "C" fn float_main(fighter: &mut L2CFighterCommon) -> L2CValue {
    let boma = fighter.module_accessor;
    let Some(floater) = floater(boma) else {
        return 0.into();
    };
    let mut state = load(object_id(boma));
    open(fighter, &mut state);
    state.enabled = true;
    change_motion(boma, loop_motion(boma, &floater.float));
    stiff(boma);
    hover_or_aerial(fighter, &mut state);
    store(state);
    fighter.sub_shift_status_main(pointer(float_loop as *const () as usize))
}

pub unsafe extern "C" fn end(fighter: &mut L2CFighterCommon) -> L2CValue {
    let boma = fighter.module_accessor;
    if StatusModule::status_kind_next(boma) == *FIGHTER_STATUS_KIND_LANDING_ATTACK_AIR {
        WorkModule::set_int64(boma, MotionModule::motion_kind(boma) as _, *FIGHTER_STATUS_ATTACK_AIR_WORK_INT_MOTION_KIND);
    }
    AttackModule::set_shield_stiff_mul(boma, 1.0);
    0.into()
}

unsafe fn fall(fighter: &mut L2CFighterCommon) {
    let boma = fighter.module_accessor;
    ControlModule::clear_command_one(boma, *FIGHTER_PAD_COMMAND_CATEGORY1, *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_N);
    let cat1 = ControlModule::get_command_flag_cat(boma, 0);
    fighter.global_table[CMD_CAT1].assign(&L2CValue::new_int(cat1 as i64 as u64));
    change_status(fighter, *FIGHTER_STATUS_KIND_FALL, true);
}

unsafe extern "C" fn float_loop(fighter: &mut L2CFighterCommon) -> L2CValue {
    let boma = fighter.module_accessor;
    let Some(floater) = floater(boma) else {
        return 0.into();
    };
    if truthy(&fighter.sub_transition_group_check_air_cliff()) {
        return 1.into();
    }
    let id = object_id(boma);
    let mut state = load(id);
    if StatusModule::status_kind(boma) == floater.start {
        let motion = MotionModule::motion_kind(boma);
        if (motion == start_motion(boma, &floater.float) || AERIALS.contains(&motion)) && MotionModule::is_end(boma) {
            change_status(fighter, floater.start + 1, false);
            return 0.into();
        }
    }
    if StatusModule::is_changing(boma)
        && global_int(fighter, PREV_STATUS_KIND) == *FIGHTER_STATUS_KIND_ATTACK_AIR as i64
        && ControlModule::check_button_on(boma, *CONTROL_PAD_BUTTON_ATTACK)
    {
        FighterControlModuleImpl::update_attack_air_kind(boma, true);
        aerial(fighter, &mut state);
        store(state);
        return 0.into();
    }
    let situation = global_int(fighter, SITUATION_KIND);
    if WorkModule::is_enable_transition_term(boma, *FIGHTER_STATUS_TRANSITION_TERM_ID_LANDING_ATTACK_AIR)
        && situation == *SITUATION_KIND_GROUND as i64
        && WorkModule::is_flag(boma, *FIGHTER_STATUS_ATTACK_AIR_FLAG_ENABLE_LANDING)
    {
        change_status(fighter, *FIGHTER_STATUS_KIND_LANDING_ATTACK_AIR, false);
        return 0.into();
    }
    let stick_y = global_num(fighter, STICK_Y);
    let released = ControlModule::check_button_off(boma, *CONTROL_PAD_BUTTON_JUMP) && stick_y <= common(boma, "jump_stick_y");
    if state.terms & FALL_CONTROL != 0 && released {
        fall(fighter);
        return 0.into();
    }
    if state.terms & ATTACK_AIR_CONTROL != 0 && released {
        change_status(fighter, *FIGHTER_STATUS_KIND_ATTACK_AIR, true);
        return 0.into();
    }
    if state.terms & FALL_TIME != 0 && state.frame <= 0 {
        fall(fighter);
        return 0.into();
    }
    if state.terms & ATTACK_AIR_TIME != 0 && state.frame <= 0 {
        change_status(fighter, *FIGHTER_STATUS_KIND_ATTACK_AIR, false);
        return 0.into();
    }
    if state.terms & ITEM_THROW != 0
        && floater.float.item >= 0
        && global_int(fighter, PAD_FLAG) & *FIGHTER_PAD_FLAG_SPECIAL_TRIGGER as i64 != 0
        && common(boma, "special_stick_y") <= -stick_y
        && ItemModule::get_have_item_kind(boma, 0) == floater.float.item
    {
        change_status(fighter, *FIGHTER_STATUS_KIND_ITEM_THROW, true);
        return 0.into();
    }
    if state.enabled && state.frame > 0 {
        state.frame -= 1;
        if state.frame <= 0 {
            state.enabled = false;
        }
    }
    store(state);
    if WorkModule::is_enable_transition_term_group(boma, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_LANDING) {
        fighter.sub_transition_group_check_air_landing();
    }
    if WorkModule::is_enable_transition_term_group(boma, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_SPECIAL) {
        fighter.sub_transition_group_check_air_special();
    }
    if StatusModule::is_changing(boma) {
        return 0.into();
    }
    let mut state = load(id);
    let attack = global_int(fighter, CMD_CAT1) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_N as i64 != 0;
    let mut again = false;
    if state.mtrans == AERIAL {
        if MotionModule::is_end(boma) {
            again = true;
        }
        if CancelModule::is_enable_cancel(boma) && attack {
            again = true;
        }
    } else if state.mtrans == IDLE {
        if attack {
            again = true;
        }
        if MotionModule::is_end(boma) {
            change_motion(boma, loop_motion(boma, &floater.float));
        }
    }
    if again {
        hover_or_aerial(fighter, &mut state);
    }
    store(state);
    0.into()
}

pub unsafe extern "C" fn attack_air_pre(fighter: &mut L2CFighterCommon) -> L2CValue {
    let boma = fighter.module_accessor;
    if let Some(floater) = floater(boma) {
        let prev = global_int(fighter, PREV_STATUS_KIND);
        if prev == floater.start as i64 || prev == floater.start as i64 + 1 {
            StatusModule::init_settings(
                boma,
                SituationKind(*SITUATION_KIND_AIR),
                *FIGHTER_KINETIC_TYPE_MOTION_FALL,
                *GROUND_CORRECT_KIND_AIR as u32,
                GroundCliffCheckKind(*GROUND_CLIFF_CHECK_KIND_NONE),
                true,
                *FIGHTER_PEACH_STATUS_WORK_KEEP_FLAG_ATTACK_AIR_FLAG,
                *FIGHTER_PEACH_STATUS_WORK_KEEP_FLAG_ATTACK_AIR_INT,
                *FIGHTER_PEACH_STATUS_WORK_KEEP_FLAG_ATTACK_AIR_FLOAT,
                *FS_SUCCEEDS_KEEP_VISIBILITY | *FS_SUCCEEDS_KEEP_ATTACK,
            );
            FighterStatusModuleImpl::set_fighter_status_data(
                boma,
                false,
                *FIGHTER_TREADED_KIND_NO_REAC,
                false,
                false,
                false,
                *FIGHTER_LOG_MASK_FLAG_ACTION_CATEGORY_KEEP as u64,
                *FIGHTER_STATUS_ATTR_CLEAR_MOTION_ENERGY as u32,
                *FIGHTER_POWER_UP_ATTACK_BIT_ATTACK_AIR as u32,
                0,
            );
            return 0.into();
        }
    }
    match original(fighter, LINE_PRE, *FIGHTER_STATUS_KIND_ATTACK_AIR) {
        Some(function) => function(fighter),
        None => fighter.status_pre_AttackAir(),
    }
}

unsafe extern "C" fn attack_air_hold(fighter: &mut L2CFighterCommon) -> L2CValue {
    let boma = fighter.module_accessor;
    fighter.status_AttackAir_Main_common();
    WorkModule::set_int64(boma, MotionModule::motion_kind(boma) as _, *FIGHTER_STATUS_ATTACK_AIR_WORK_INT_MOTION_KIND);
    0.into()
}

unsafe extern "C" fn attack_air_loop(fighter: &mut L2CFighterCommon) -> L2CValue {
    let boma = fighter.module_accessor;
    let state = load(object_id(boma));
    if let Some(floater) = floater(boma) {
        if CancelModule::is_enable_cancel(boma) && global_int(fighter, CMD_CAT1) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_N as i64 != 0 {
            let shoot = WorkModule::is_enable_transition_term(boma, *FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_ITEM_SHOOT_AIR)
                && truthy(&fighter.sub_is_item_shoot_air());
            let again = shoot || WorkModule::is_enable_transition_term(boma, *FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_ATTACK_AIR);
            if again && may_float(boma, &state) {
                let low = KineticModule::get_sum_speed_y(boma, *KINETIC_ENERGY_RESERVE_ATTRIBUTE_MAIN) < 0.0
                    || global_num(fighter, STICK_Y) <= common(boma, "squat_stick_y");
                if low
                    && ControlModule::check_button_on(boma, *CONTROL_PAD_BUTTON_JUMP)
                    && global_int(fighter, SITUATION_KIND) == *SITUATION_KIND_AIR as i64
                {
                    store(State { ray: false, ..state });
                    change_status(fighter, floater.start, true);
                    return 1.into();
                }
            }
        }
    }
    match state.aerial_loop {
        0 => fighter.status_AttackAir_Main_common(),
        function => core::mem::transmute::<usize, StatusFn>(function)(fighter),
    }
}

pub unsafe extern "C" fn attack_air_main(fighter: &mut L2CFighterCommon) -> L2CValue {
    let boma = fighter.module_accessor;
    let floater = floater(boma);
    if let Some(floater) = floater {
        let prev = global_int(fighter, PREV_STATUS_KIND);
        if prev == floater.start as i64 || prev == floater.start as i64 + 1 {
            let motion = MotionModule::motion_kind(boma);
            if let Some(index) = AERIALS.iter().position(|aerial| *aerial == motion) {
                let logs = [
                    *FIGHTER_LOG_ATTACK_KIND_ATTACK_AIR_N,
                    *FIGHTER_LOG_ATTACK_KIND_ATTACK_AIR_F,
                    *FIGHTER_LOG_ATTACK_KIND_ATTACK_AIR_B,
                    *FIGHTER_LOG_ATTACK_KIND_ATTACK_AIR_HI,
                    *FIGHTER_LOG_ATTACK_KIND_ATTACK_AIR_LW,
                ];
                log(fighter, *FIGHTER_LOG_ACTION_CATEGORY_KEEP, logs[index]);
                attack_air_hold(fighter);
                return fighter.sub_shift_status_main(pointer(attack_air_hold as *const () as usize));
            }
        }
    }
    let result = match original(fighter, LINE_MAIN, *FIGHTER_STATUS_KIND_ATTACK_AIR) {
        Some(function) => function(fighter),
        None => fighter.status_AttackAir(),
    };
    if floater.is_some() && !StatusModule::is_changing(boma) {
        let line = LuaModule::get_execute_line(boma) as i32;
        let current = pointer_of(&fighter.line_state__get_value_from_index(line.into()));
        if current != 0 && current != attack_air_loop as *const () as usize {
            let mut state = load(object_id(boma));
            state.aerial_loop = current;
            store(state);
            fighter.line_state__set_value_from_index(line.into(), pointer(attack_air_loop as *const () as usize));
        }
    }
    result
}
