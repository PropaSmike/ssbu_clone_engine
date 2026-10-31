//! A new enemy kind: a clone of Nabbit with files and logic of its own,
//! brought into every battle on any stage.
//!
//! The clone loads its own files; copy Nabbit's and change them:
//! - enemy/template_enemy/model, motion, param and script
//! - enemy/template_enemy/model/body/c00/param.prc: its parameters
//! - effect/enemy/template_enemy and sound/bank/enemy/se_enemy_template_enemy.*

pub use smashline::skyline_smash as smash;

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use clone_engine_api::{enemy, menu};
use smash::app::lua_bind::*;
use smash::app::{sv_battle_object, BattleObjectModuleAccessor};
use smash::lib::lua_const::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::phx::{Hash40, Vector3f};
use smashline::{Agent, Main};

/// The enemy's name: its folders under enemy/, effect/enemy/ and sound/.
const NAME: &str = "template_enemy";
/// Nabbit's class statuses the game keeps running: 0 its reactions to hits
/// and grabs, 21 its fall once knocked down, 23 its wait while knocked out,
/// 24 a state it shares with the other enemies. Every other status runs
/// `drive` below.
const NATIVE: &[u32] = &[0, 21, 23, 24];
/// The battle frame it arrives at, and where.
const ARRIVE_FRAME: u64 = 120;
const ARRIVE_AT: (f32, f32) = (0.0, 30.0);
/// Frames it stays before it leaves the stage.
const STAY_FRAMES: u64 = 60 * 30;
/// Frames a hit's reaction may last before the driver takes it back.
const REACTION_FRAMES: u32 = 40;
/// Its walk: units per frame, and frames before it turns round.
const WALK_SPEED: f32 = 0.8;
const TURN_FRAMES: u32 = 120;
/// A motion of its motion list.
const WALK_MOTION: &str = "run";

const NONE: u32 = u32::MAX;
static KIND: AtomicU32 = AtomicU32::new(NONE);
static SERIAL: AtomicU32 = AtomicU32::new(NONE);
static REQUEST: AtomicU32 = AtomicU32::new(NONE);
static OBJECT: AtomicU32 = AtomicU32::new(enemy::NO_OBJECT);
static ARRIVED: AtomicU64 = AtomicU64::new(0);
static DONE: AtomicBool = AtomicBool::new(false);
static WALKED: AtomicU32 = AtomicU32::new(0);
static REACTING: AtomicU32 = AtomicU32::new(0);

/// The clone's own logic, every frame it is in a driven status: object id,
/// module accessor, and the class status the base would have run.
unsafe extern "C" fn drive(_object: u32, module_accessor: usize, _status: i32) {
    let boma = module_accessor as *mut BattleObjectModuleAccessor;
    if MotionModule::motion_kind(boma) != smash::hash40(WALK_MOTION) {
        MotionModule::change_motion(
            boma,
            Hash40::new(WALK_MOTION), // motion
            0.0,                      // start frame
            1.0,                      // rate
            false,                    // the game's own arguments, as for any change_motion
            0.0,
            false,
            false,
        );
    }
    let walked = WALKED.fetch_add(1, Ordering::Relaxed);
    if walked % TURN_FRAMES == TURN_FRAMES - 1 {
        PostureModule::reverse_lr(boma);
        PostureModule::update_rot_y_lr(boma);
    }
    let step = Vector3f {
        x: WALK_SPEED * PostureModule::lr(boma),
        y: 0.0,
        z: 0.0,
    };
    PostureModule::add_pos(boma, &step);
}

/// Once per frame for every fighter; player 1's frame runs the enemy.
unsafe extern "C" fn battle_frame(fighter: &mut L2CFighterCommon) {
    let boma = fighter.module_accessor;
    if WorkModule::get_int(boma, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) != 0 {
        return;
    }
    let kind = KIND.load(Ordering::Acquire);
    let (Ok(serial), Ok(frame)) = (menu::battle_serial(), menu::battle_frame()) else {
        return;
    };
    if SERIAL.swap(serial, Ordering::AcqRel) != serial {
        // a new battle: nothing of the last one is left
        REQUEST.store(NONE, Ordering::Release);
        OBJECT.store(enemy::NO_OBJECT, Ordering::Release);
        DONE.store(false, Ordering::Release);
    }
    if DONE.load(Ordering::Acquire) {
        return;
    }

    if frame >= ARRIVE_FRAME && REQUEST.load(Ordering::Acquire) == NONE {
        // kind, x, y, z, facing (1.0 right, -1.0 left); it is made on a later frame
        match enemy::spawn(kind, ARRIVE_AT.0, ARRIVE_AT.1, 0.0, 1.0) {
            Ok(request) => REQUEST.store(request, Ordering::Release),
            Err(error) => {
                println!("[{NAME}] no spawn: {error:?}");
                DONE.store(true, Ordering::Release);
                return;
            }
        }
    }

    let request = REQUEST.load(Ordering::Acquire);
    let mut object = OBJECT.load(Ordering::Acquire);
    if request != NONE && object == enemy::NO_OBJECT {
        // brings it out once it is made; None until then
        let Some(started) = enemy::start(request) else {
            return;
        };
        object = started;
        OBJECT.store(object, Ordering::Release);
        ARRIVED.store(frame, Ordering::Release);
        println!("[{NAME}] arrived as object {object:#x}");
    }
    if object == enemy::NO_OBJECT {
        return;
    }

    // its class status: 0 while the game plays a hit's reaction
    if enemy::class_status(object) == Some(0) {
        if REACTING.fetch_add(1, Ordering::Relaxed) == REACTION_FRAMES {
            enemy::drive_now(object); // the driver takes it back at once
        }
    } else {
        REACTING.store(0, Ordering::Relaxed);
    }

    if frame >= ARRIVED.load(Ordering::Acquire) + STAY_FRAMES {
        enemy::remove(object); // takes it off the stage
        DONE.store(true, Ordering::Release);
        for index in 0..enemy::OBJECTS {
            // every enemy object of the battle, live or not
            let id = enemy::object(index);
            if id != enemy::NO_OBJECT && sv_battle_object::is_active(id) {
                println!("[{NAME}] enemy object {index} is still out: {id:#x}");
            }
        }
    }
}

#[skyline::main(name = "clone_engine_enemy_template")]
pub fn main() {
    let kind = match enemy::clone(
        enemy::NABBIT, // the base: YELLOW_DEVIL, DARK_EMPEROR, FLYING_MAN, METAL_FACE or NABBIT
        NAME,          // its files, under enemy/template_enemy and the rest
    ) {
        Ok(kind) => kind,
        Err(error) => {
            println!("[{NAME}] disabled: {error:?}");
            return;
        }
    };
    KIND.store(kind, Ordering::Release);

    // kind, the driver, the class statuses the base keeps
    if let Err(error) = enemy::drive(kind, Some(drive), enemy::statuses(NATIVE)) {
        println!("[{NAME}] runs as plain Nabbit: {error:?}");
    }
    // one class slot of this clone, here every status change; enemy::original_entry gives the base's:
    // enemy::set_vtable_entry(kind, enemy::slot::CHANGE_STATUS, my_change as *const ());
    // loaded in every battle from the next one on
    if let Err(error) = enemy::arm(enemy::kinds(&[kind])) {
        println!("[{NAME}] not loaded: {error:?}");
        return;
    }
    // "fighter": every fighter, once per frame
    Agent::new("fighter").on_line(Main, battle_frame).install();
    println!("[{NAME}] enemy kind {kind}");
}
