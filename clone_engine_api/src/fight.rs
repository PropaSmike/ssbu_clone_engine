use smash::app::lua_bind::{ControlModule, MotionModule, PostureModule, StatusModule, TeamModule, WorkModule};
use smash::app::{sv_battle_object, BattleObjectModuleAccessor, Fighter, FighterUtil};
use smash::lib::lua_const::*;
use smash::lua2cpp::L2CFighterCommon;
use std::sync::atomic::AtomicUsize;
use std::sync::Mutex;

pub use clone_engine_core::command_input::{Button, Command, StickRange};
pub use clone_engine_core::motion_input::Motion;
use clone_engine_core::motion_input::{numpad, InputHistory};

use crate::resolve;

const ENTRIES: i32 = 8;
const END_WITHIN: usize = 4;

type ReadyFn = unsafe extern "C" fn(*mut BattleObjectModuleAccessor, u64, *mut f32) -> bool;
type ClearFn = unsafe extern "C" fn(*mut BattleObjectModuleAccessor, u64) -> bool;

static READY_FN: AtomicUsize = AtomicUsize::new(0);
static CLEAR_FN: AtomicUsize = AtomicUsize::new(0);

pub unsafe fn command_lr(fighter: &mut L2CFighterCommon, name: &str) -> Option<f32> {
    let address = resolve(&READY_FN, b"clone_engine_command_ready_v1\0")?;
    let function: ReadyFn = core::mem::transmute(address);
    let mut lr = 0.0f32;
    function(fighter.module_accessor, smash::hash40(name), &mut lr).then_some(lr)
}

pub unsafe fn command(fighter: &mut L2CFighterCommon, name: &str) -> bool {
    command_lr(fighter, name).is_some()
}

pub unsafe fn clear_command(fighter: &mut L2CFighterCommon, name: &str) {
    if let Some(address) = resolve(&CLEAR_FN, b"clone_engine_command_clear_v1\0") {
        let function: ClearFn = core::mem::transmute(address);
        function(fighter.module_accessor, smash::hash40(name));
    }
}

static HISTORIES: Mutex<[InputHistory; ENTRIES as usize]> = Mutex::new([InputHistory::new(); ENTRIES as usize]);

unsafe fn entry(fighter: &mut L2CFighterCommon) -> Option<usize> {
    let entry = WorkModule::get_int(fighter.module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    (0..ENTRIES).contains(&entry).then_some(entry as usize)
}

pub unsafe fn track(fighter: &mut L2CFighterCommon) {
    let boma = fighter.module_accessor;
    let dir = numpad(ControlModule::get_stick_x(boma), ControlModule::get_stick_y(boma), PostureModule::lr(boma));
    if let (Some(entry), Ok(mut histories)) = (entry(fighter), HISTORIES.lock()) {
        histories[entry].push(dir);
    }
}

pub unsafe fn motion(fighter: &mut L2CFighterCommon, motion: Motion) -> bool {
    match (entry(fighter), HISTORIES.lock()) {
        (Some(entry), Ok(histories)) => histories[entry].matches(motion, END_WITHIN),
        _ => false,
    }
}

pub unsafe fn recognise(fighter: &mut L2CFighterCommon) -> Option<Motion> {
    match (entry(fighter), HISTORIES.lock()) {
        (Some(entry), Ok(histories)) => histories[entry].recognise(END_WITHIN),
        _ => None,
    }
}

pub unsafe fn forget(fighter: &mut L2CFighterCommon) {
    if let (Some(entry), Ok(mut histories)) = (entry(fighter), HISTORIES.lock()) {
        histories[entry].clear();
    }
}

pub unsafe fn opponent(fighter: &mut L2CFighterCommon) -> Option<*mut BattleObjectModuleAccessor> {
    let boma = fighter.module_accessor;
    if FighterUtil::get_opponent_fighter_num(boma, true) != 1 {
        return None;
    }
    let own = WorkModule::get_int(boma, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    let team = TeamModule::team_no(boma);
    let mut found = None;
    for entry in 0..ENTRIES {
        if entry == own {
            continue;
        }
        let id = Fighter::get_id_from_entry_id(entry);
        if !sv_battle_object::is_active(id) {
            continue;
        }
        let other = sv_battle_object::module_accessor(id);
        if other.is_null() || TeamModule::team_no(other) == team {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some(other);
    }
    found
}

pub unsafe fn opponent_lr(fighter: &mut L2CFighterCommon) -> Option<f32> {
    let other = opponent(fighter)?;
    let dx = PostureModule::pos_x(other) - PostureModule::pos_x(fighter.module_accessor);
    if dx.abs() < 0.01 {
        return None;
    }
    Some(dx.signum())
}

unsafe fn faces_always(status: i32) -> bool {
    [
        *FIGHTER_STATUS_KIND_WAIT,
        *FIGHTER_STATUS_KIND_SQUAT,
        *FIGHTER_STATUS_KIND_SQUAT_WAIT,
        *FIGHTER_STATUS_KIND_SQUAT_RV,
        *FIGHTER_STATUS_KIND_LANDING,
        *FIGHTER_STATUS_KIND_LANDING_LIGHT,
        *FIGHTER_STATUS_KIND_GUARD_ON,
        *FIGHTER_STATUS_KIND_GUARD,
    ]
    .contains(&status)
}

unsafe fn faces_on_start(status: i32) -> bool {
    [
        *FIGHTER_STATUS_KIND_ATTACK,
        *FIGHTER_STATUS_KIND_ATTACK_S3,
        *FIGHTER_STATUS_KIND_ATTACK_HI3,
        *FIGHTER_STATUS_KIND_ATTACK_LW3,
        *FIGHTER_STATUS_KIND_SPECIAL_N,
        *FIGHTER_STATUS_KIND_SPECIAL_S,
        *FIGHTER_STATUS_KIND_SPECIAL_HI,
        *FIGHTER_STATUS_KIND_SPECIAL_LW,
        *FIGHTER_STATUS_KIND_CATCH,
    ]
    .contains(&status)
}

pub unsafe fn face_opponent(fighter: &mut L2CFighterCommon) -> bool {
    let boma = fighter.module_accessor;
    if StatusModule::situation_kind(boma) != *SITUATION_KIND_GROUND {
        return false;
    }
    let status = StatusModule::status_kind(boma);
    let starting = StatusModule::is_changing(boma) || MotionModule::frame(boma) < 1.0;
    if !faces_always(status) && !(starting && faces_on_start(status)) {
        return false;
    }
    let Some(lr) = opponent_lr(fighter) else {
        return false;
    };
    if lr == PostureModule::lr(boma) {
        return false;
    }
    PostureModule::reverse_lr(boma);
    PostureModule::update_rot_y_lr(boma);
    true
}
