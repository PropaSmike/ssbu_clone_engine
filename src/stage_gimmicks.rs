#![allow(dead_code)]

use dionysus_core::stage_gimmicks::{
    Barrel, BarrelEvent, BarrelMotion, Door, Ladder, Spring, BARREL_EVENT_END_PATH, BARREL_EVENT_SET_PATH,
    DOOR_EVENT_IN, DOOR_INPUT_STICK_UP,
};
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

pub const OFF_SETUP_DONE: usize = 0x25E4E1C;
pub const SETUP_DONE_OPCODE: u32 = 0xF940_0268;
pub const SETUP_STAGE_REG: usize = 19;
pub const OFF_ALLOCATE: usize = 0x392DCE0;
pub const ALLOCATE_OPCODE: u32 = 0xD102_03FF;
pub const OFF_GIMMICK_BASE: usize = 0x313FCB0;
pub const GIMMICK_BASE_OPCODE: u32 = 0xD101_C3FF;
pub const OFF_SET_POINTS: usize = 0x3142FE0;
pub const SET_POINTS_OPCODE: u32 = 0xA9BC_5FF8;
pub const OFF_START: usize = 0x313E970;
pub const START_OPCODE: u32 = 0xD101_83FF;
pub const OFF_LADDER_AREA: usize = 0x3142E60;
pub const LADDER_AREA_OPCODE: u32 = 0xA9BE_4FF4;
pub const OFF_LADDER_VTABLE: usize = 0x5177FC0;
pub const OFF_BARREL_CTOR: usize = 0x27E99C0;
pub const BARREL_CTOR_OPCODE: u32 = 0xD101_03FF;
pub const OFF_BARREL_SET_DATA: usize = 0x27EAF50;
pub const BARREL_SET_DATA_OPCODE: u32 = 0xD101_43FF;
pub const OFF_BARREL_UPDATE: usize = 0x27E9D00;
pub const BARREL_UPDATE_OPCODE: u32 = 0x3941_2408;
pub const OFF_BARREL_DELETE: usize = 0x27E9CB0;
pub const BARREL_DELETE_OPCODE: u32 = 0xF81E_0FF3;
pub const OFF_BARREL_EVENT: usize = 0x27EAB30;
pub const BARREL_EVENT_OPCODE: u32 = 0xF81A_0FFB;
pub const OFF_BARREL_VTABLE: usize = 0x50E1E50;
pub const STAGE_ID: usize = 0x08;
pub const STAGE_SOUND: usize = 0x338;
pub const STAGE_GIMMICKS: usize = 0x350;
pub const GIMMICK_ALIGN: u64 = 0x10;
pub const LADDER_SIZE: u64 = 0x160;
pub const LADDER_TOP: usize = 0x110;
pub const LADDER_BOTTOM: usize = 0x120;
pub const LADDER_TOP_OFFSET: usize = 0x130;
pub const LADDER_BOTTOM_OFFSET: usize = 0x140;
pub const GIMMICK_POSITION: usize = 0x70;
pub const GIMMICK_ROTATION_Z: usize = 0x98;
pub const GIMMICK_MODEL: usize = 0xB8;
pub const LADDER_MODEL: usize = 0x150;
pub const LADDER_ATTACHED: usize = 0x158;
pub const BARREL_SIZE: u64 = 0x220;
pub const BARREL_STATE: usize = 0x198;
pub const BARREL_HOLDING: u32 = 1;
pub const BARREL_FIRING: u32 = 2;
pub const BARREL_EMPTY: u32 = 0;
pub const EVENT_KIND: usize = 0x10;
pub const EVENT_MAY_ENTER: u32 = 0xE;
pub const EVENT_ANSWER: usize = 0xB4;
pub const EVENT_SLOT: usize = 0xA0;
pub const GIMMICK_HOLDER: usize = 0x38;
pub const HOLDER_AREAS: usize = 0x130;
pub const AREAS_INSTANCE_SLOT: usize = 0x1C8;
pub const INSTANCE_POSITION: usize = 0x450;
pub const BARREL_EFFECT_POSITION: usize = 0x1E0;
pub const OFF_GIMMICK_MATRIX: usize = 0x3140740;
pub const GIMMICK_MATRIX_OPCODE: u32 = 0xD102_C3FF;
pub const OFF_EVENT_QUEUE: usize = 0x315F230;
pub const EVENT_QUEUE_OPCODE: u32 = 0xF81B_0FF9;
pub const OFF_EVENT_CENTER: usize = 0x53299D8;
pub const EVENT_CENTER_QUEUE: usize = 0x1E0;
pub const OFF_BARREL_EVENT_VTABLE: usize = 0x50B38C8;
pub const GIMMICK_OBJECT: usize = 0x30;
pub const OBJECT_ID: usize = 0x08;
pub const BARREL_RIDERS: usize = 0x208;
pub const BARREL_RIDER_COUNT: usize = 0x210;
pub const MAX_RIDERS: u64 = 2;
pub const EVENT_SENDER: usize = 0x14;
pub const BARREL_EVENT_SET_SPEED: u32 = 5;
pub const AREA_SLOT: usize = 0x98;
pub const DELETE_SLOT: usize = 0x08;
pub const UPDATE_SLOT: usize = 0x10;
pub const MAX_GIMMICKS: u64 = 4096;
pub const OFF_SPRING_CTOR: usize = 0x31432D0;
pub const SPRING_CTOR_OPCODE: u32 = 0xA9BD_57F6;
pub const OFF_SPRING_SYNC: usize = 0x3143A20;
pub const SPRING_SYNC_OPCODE: u32 = 0xA9BE_4FF4;
pub const OFF_SPRING_AREA: usize = 0x3143B40;
pub const SPRING_AREA_OPCODE: u32 = 0xF81E_0FF3;
pub const OFF_SPRING_VTABLE: usize = 0x5178078;
pub const SPRING_SIZE: u64 = 0x1A0;
pub const SPRING_SPEED_Y: usize = 0x11C;
pub const SPRING_ROT_Z: usize = 0x120;
pub const SPRING_AT_ONCE: usize = 0x190;
pub const SPRING_LAUNCH_FRAME: usize = 0x124;
pub const OFF_DOOR_EVENT_VTABLE: usize = 0x4F7F878;
pub const OFF_DOOR_EVENT_DTOR: usize = 0x3CB810;
pub const EVENT_TARGET: usize = 0x0C;
pub const DOOR_EVENT_CENTER: usize = 0x20;
pub const DOOR_EVENT_NO_INTO: usize = 0x30;
pub const AREA_INFO_KIND: usize = 0x2A0;
pub const AREA_INFO_GIMMICK: usize = 0x2A4;
pub const AREA_INFO_ENTRY: usize = 0x2AC;
pub const AREA_INFO_INPUT: usize = 0x2B0;

#[derive(Debug, Clone, Default)]
struct Declared {
    ladders: Vec<Ladder>,
    barrels: Vec<Barrel>,
    springs: Vec<Spring>,
    doors: Vec<Door>,
}

static DECLARED: OnceLock<Mutex<HashMap<String, Declared>>> = OnceLock::new();
static LIVE_BARRELS: OnceLock<Mutex<HashMap<u64, (Barrel, BarrelMotion)>>> = OnceLock::new();
static FLIGHTS: OnceLock<Mutex<Flights>> = OnceLock::new();
static PATH_SENDERS: OnceLock<Mutex<HashSet<u32>>> = OnceLock::new();
static JOINT_NAMES: OnceLock<Mutex<HashMap<String, std::ffi::CString>>> = OnceLock::new();
static LIVE_DOORS: OnceLock<Mutex<LiveDoors>> = OnceLock::new();

#[derive(Debug, Default)]
struct LiveDoors {
    stage: u64,
    doors: HashMap<u32, (usize, Door)>,
}

fn live_doors() -> &'static Mutex<LiveDoors> {
    LIVE_DOORS.get_or_init(|| Mutex::new(LiveDoors::default()))
}

fn joint_name(name: &str) -> u64 {
    let Ok(mut names) = JOINT_NAMES.get_or_init(|| Mutex::new(HashMap::new())).lock() else {
        return 0;
    };
    if !names.contains_key(name) {
        let Ok(text) = std::ffi::CString::new(name) else {
            return 0;
        };
        names.insert(name.to_string(), text);
    }
    names.get(name).map_or(0, |text| text.as_ptr() as u64)
}

fn path_senders() -> &'static Mutex<HashSet<u32>> {
    PATH_SENDERS.get_or_init(|| Mutex::new(HashSet::new()))
}

#[derive(Debug, Default)]
struct Flights {
    loaded: HashMap<u64, Vec<u32>>,
    flying: Vec<Flight>,
}

#[derive(Debug, Clone, Copy)]
struct Flight {
    object: u64,
    rider: u32,
    frame: f32,
}

fn flights() -> &'static Mutex<Flights> {
    FLIGHTS.get_or_init(|| Mutex::new(Flights::default()))
}

fn declared() -> &'static Mutex<HashMap<String, Declared>> {
    DECLARED.get_or_init(|| Mutex::new(HashMap::new()))
}

fn live_barrels() -> &'static Mutex<HashMap<u64, (Barrel, BarrelMotion)>> {
    LIVE_BARRELS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn set_gimmicks(
    place_name: &str,
    ladders: Vec<Ladder>,
    barrels: Vec<Barrel>,
    springs: Vec<Spring>,
    doors: Vec<Door>,
) {
    let wants_barrels = !barrels.is_empty();
    let wants_doors = !doors.is_empty();
    if let Ok(mut map) = declared().lock() {
        map.insert(
            place_name.to_ascii_lowercase(),
            Declared {
                ladders,
                barrels,
                springs,
                doors,
            },
        );
    }
    arm(wants_barrels, wants_doors);
}

fn declared_for(stage_id: u32) -> Declared {
    let place = match crate::stage_registry::registry().lock() {
        Ok(registry) => registry
            .stages()
            .iter()
            .find(|stage| stage.normal_stage_id() == Some(stage_id))
            .map(|stage| stage.place_name.clone()),
        Err(_) => None,
    };
    let Some(place) = place else {
        return Declared::default();
    };
    declared()
        .lock()
        .ok()
        .and_then(|map| map.get(&place).cloned())
        .unwrap_or_default()
}

#[cfg(all(not(test), feature = "stage_config_bridge"))]
mod live {
    use super::*;
    use dionysus_core::stage_gimmicks::{AreaParam, BarrelData, Vector4};
    use core::sync::atomic::{AtomicBool, Ordering};

    static INSTALLED: AtomicBool = AtomicBool::new(false);
    static BARRELS_INSTALLED: AtomicBool = AtomicBool::new(false);
    static BARRELS_READY: AtomicBool = AtomicBool::new(false);
    static SPRINGS_READY: AtomicBool = AtomicBool::new(false);
    static QUEUE_INSTALLED: AtomicBool = AtomicBool::new(false);
    static DOORS_READY: AtomicBool = AtomicBool::new(false);

    type Allocate = extern "C" fn(u64, u64) -> u64;
    type GimmickBase = extern "C" fn(u64, u64, u64, u32) -> u64;
    type SetArea = extern "C" fn(u64, *mut AreaParam, u32);
    type SetPoints = extern "C" fn(u64, *const Vector4, u32, u32);
    type Start = extern "C" fn(u64);
    type BarrelCtor = extern "C" fn(u64, u64, *const u64, u64) -> u64;
    type BarrelSetData = extern "C" fn(u64, *const BarrelData);
    type SpringCtor = extern "C" fn(u64, u64, u64, u32, u64, u32) -> u64;
    type SpringSync = extern "C" fn(u64);

    unsafe fn word(at: u64) -> u64 {
        core::ptr::read_volatile(at as *const u64)
    }

    unsafe fn put<T>(at: u64, value: T) {
        core::ptr::write_volatile(at as *mut T, value)
    }

    unsafe fn holds(stage: u64, vtable: u64) -> bool {
        let list = word(stage + STAGE_GIMMICKS as u64);
        if list == 0 {
            return false;
        }
        let begin = word(list);
        let end = word(list + 8);
        if begin == 0 || end < begin || (end - begin) % 8 != 0 || (end - begin) / 8 > MAX_GIMMICKS {
            return false;
        }
        (0..(end - begin) / 8).any(|index| {
            let gimmick = word(begin + index * 8);
            gimmick != 0 && word(gimmick) == vtable
        })
    }

    unsafe fn allocate(size: u64) -> u64 {
        let allocate: Allocate = core::mem::transmute(crate::game::addr(OFF_ALLOCATE));
        allocate(GIMMICK_ALIGN, size)
    }

    unsafe fn start(object: u64) {
        let start: Start = core::mem::transmute(crate::game::addr(OFF_START));
        start(object);
    }

    unsafe fn build_ladder(stage: u64, ladder: &Ladder, vtable: u64) -> u64 {
        let gimmick_base: GimmickBase = core::mem::transmute(crate::game::addr(OFF_GIMMICK_BASE));
        let set_area: SetArea = core::mem::transmute(word(vtable + AREA_SLOT as u64));
        let set_points: SetPoints = core::mem::transmute(crate::game::addr(OFF_SET_POINTS));
        let object = allocate(LADDER_SIZE);
        if object == 0 {
            return 0;
        }
        gimmick_base(object, stage, 0, 0);
        for offset in [LADDER_TOP, LADDER_BOTTOM] {
            put(object + offset as u64, 0u64);
            put(object + offset as u64 + 8, 0u64);
        }
        put(object, vtable);
        for offset in [LADDER_TOP_OFFSET, LADDER_BOTTOM_OFFSET] {
            put(object + offset as u64, 0u64);
            put(object + offset as u64 + 8, 0u64);
        }
        put(object + LADDER_MODEL as u64, word(object + GIMMICK_MODEL as u64));
        put(object + LADDER_ATTACHED as u64, 0u8);
        let mut area = ladder.area();
        set_area(object, &mut area, 0);
        let bottom = ladder.bottom();
        set_points(object, &bottom, 0, 0);
        start(object);
        object
    }

    unsafe fn build_barrel(stage: u64, barrel: &Barrel, sound: u64) -> bool {
        let ctor: BarrelCtor = core::mem::transmute(crate::game::addr(OFF_BARREL_CTOR));
        let set_data: BarrelSetData = core::mem::transmute(crate::game::addr(OFF_BARREL_SET_DATA));
        let object = allocate(BARREL_SIZE);
        if object == 0 {
            return false;
        }
        let model = barrel
            .model
            .as_deref()
            .and_then(|name| std::ffi::CString::new(name).ok());
        let names = [0u64, model.as_ref().map_or(0, |name| name.as_ptr() as u64), 0u64];
        ctor(object, stage, names.as_ptr(), sound);
        if model.is_some() && word(object + GIMMICK_MODEL as u64) == 0 {
            skyline::println!(
                "[gimmick] barrel model {:?} is not among the stage's model sets; the barrel draws nothing",
                barrel.model
            );
        }
        put(object + GIMMICK_POSITION as u64, barrel.position());
        put(object + GIMMICK_ROTATION_Z as u64, Barrel::class_rotation(barrel.rotation));
        let data = barrel.data();
        set_data(object, &data);
        if let Ok(mut live) = live_barrels().lock() {
            live.insert(object, (barrel.clone(), barrel.motion()));
        }
        start(object);
        if barrel.follows_path() {
            if let (Some(sender), Ok(mut senders)) = (sender_of(object), path_senders().lock()) {
                senders.insert(sender);
            }
        }
        true
    }

    unsafe fn build_spring(stage: u64, spring: &Spring, vtable: u64) -> bool {
        let ctor: SpringCtor = core::mem::transmute(crate::game::addr(OFF_SPRING_CTOR));
        let set_area: SetArea = core::mem::transmute(word(vtable + AREA_SLOT as u64));
        let sync: SpringSync = core::mem::transmute(crate::game::addr(OFF_SPRING_SYNC));
        let object = allocate(SPRING_SIZE);
        if object == 0 {
            return false;
        }
        let model = spring
            .model
            .as_deref()
            .and_then(|name| std::ffi::CString::new(name).ok());
        let joint = match (&model, spring.joint.as_deref()) {
            (Some(_), Some(name)) => joint_name(name),
            _ => 0,
        };
        ctor(object, stage, model.as_ref().map_or(0, |name| name.as_ptr() as u64), 0, joint, 0);
        if model.is_some() && word(object + GIMMICK_MODEL as u64) == 0 {
            skyline::println!(
                "[gimmick] spring model {:?} is not among the stage's model sets; the spring draws nothing",
                spring.model
            );
        }
        let mut area = spring.area();
        set_area(object, &mut area, 0);
        put(object + GIMMICK_POSITION as u64, spring.position());
        put(object + GIMMICK_ROTATION_Z as u64, spring.rotation);
        sync(object);
        put(object + SPRING_SPEED_Y as u64, spring.speed);
        put(object + SPRING_ROT_Z as u64, spring.rotation);
        put(object + SPRING_AT_ONCE as u64, u8::from(spring.launches_at_once()));
        put(object + SPRING_LAUNCH_FRAME as u64, spring.launch_frame.unwrap_or(0.0));
        start(object);
        true
    }

    unsafe fn area_instance(object: u64) -> u64 {
        let holder = word(object + GIMMICK_HOLDER as u64);
        if holder == 0 {
            return 0;
        }
        let areas = word(holder + HOLDER_AREAS as u64);
        if areas == 0 {
            return 0;
        }
        let instance: extern "C" fn(u64) -> u64 =
            core::mem::transmute(word(word(areas) + AREAS_INSTANCE_SLOT as u64));
        instance(areas)
    }

    unsafe fn build_door(stage: u64, door: &Door, vtable: u64) -> Option<u32> {
        let object = build_ladder(stage, &door.host(), vtable);
        if object == 0 {
            return None;
        }
        let instance = area_instance(object);
        if instance == 0 {
            return None;
        }
        put(instance + AREA_INFO_KIND as u64, door.kind());
        put(instance + AREA_INFO_ENTRY as u64, door.entry());
        put(instance + AREA_INFO_INPUT as u64, DOOR_INPUT_STICK_UP);
        let id = core::ptr::read_volatile((instance + AREA_INFO_GIMMICK as u64) as *const u32);
        (id != 0).then_some(id)
    }

    fn report(stage_id: u32, kind: &str, built: usize, wanted: usize) {
        skyline::println!("[gimmick] StageID {stage_id}: built {built} of {wanted} {kind}(s)");
    }

    #[skyline::hook(offset = crate::game::rel(OFF_SETUP_DONE), inline)]
    unsafe fn setup_done(ctx: &mut skyline::hooks::InlineCtx) {
        let stage = ctx.registers[SETUP_STAGE_REG].x();
        if stage == 0 {
            return;
        }
        let stage_id = core::ptr::read_volatile((stage + STAGE_ID as u64) as *const u32);
        crate::stage_motions::apply_paused(stage);
        let declared = declared_for(stage_id);
        if !declared.ladders.is_empty() {
            let vtable = crate::game::addr(OFF_LADDER_VTABLE) as u64;
            if holds(stage, vtable) {
                skyline::println!("[gimmick] StageID {stage_id}: ladders already built; left alone");
            } else {
                let built = declared
                    .ladders
                    .iter()
                    .take_while(|ladder| build_ladder(stage, ladder, vtable) != 0)
                    .count();
                report(stage_id, "ladder", built, declared.ladders.len());
            }
        }
        if !declared.barrels.is_empty() {
            let vtable = crate::game::addr(OFF_BARREL_VTABLE) as u64;
            let sound = word(stage + STAGE_SOUND as u64);
            if !BARRELS_READY.load(Ordering::Acquire) {
                skyline::println!("[gimmick] StageID {stage_id}: barrel class refused at arm time; no barrels");
            } else if sound == 0 {
                skyline::println!("[gimmick] StageID {stage_id}: no stage sound handle at +{STAGE_SOUND:#x}; no barrels");
            } else if holds(stage, vtable) {
                skyline::println!("[gimmick] StageID {stage_id}: barrels already built; left alone");
            } else {
                let built = declared
                    .barrels
                    .iter()
                    .take_while(|barrel| build_barrel(stage, barrel, sound))
                    .count();
                report(stage_id, "barrel", built, declared.barrels.len());
            }
        }
        if !declared.springs.is_empty() {
            let vtable = crate::game::addr(OFF_SPRING_VTABLE) as u64;
            if !SPRINGS_READY.load(Ordering::Acquire) {
                skyline::println!("[gimmick] StageID {stage_id}: spring class refused at arm time; no springs");
            } else if holds(stage, vtable) {
                skyline::println!("[gimmick] StageID {stage_id}: springs already built; left alone");
            } else {
                let built = declared
                    .springs
                    .iter()
                    .take_while(|spring| build_spring(stage, spring, vtable))
                    .count();
                report(stage_id, "spring", built, declared.springs.len());
            }
        }
        let Ok(mut live) = live_doors().lock() else {
            return;
        };
        if declared.doors.is_empty() {
            live.stage = 0;
            live.doors.clear();
        } else if !DOORS_READY.load(Ordering::Acquire) {
            skyline::println!("[gimmick] StageID {stage_id}: door event refused at arm time; no doors");
        } else if live.stage == stage {
            skyline::println!("[gimmick] StageID {stage_id}: doors already built; left alone");
        } else {
            live.stage = stage;
            live.doors.clear();
            let vtable = crate::game::addr(OFF_LADDER_VTABLE) as u64;
            for (index, door) in declared.doors.iter().enumerate() {
                let Some(id) = build_door(stage, door, vtable) else {
                    break;
                };
                live.doors.insert(id, (index, *door));
            }
            report(stage_id, "door", live.doors.len(), declared.doors.len());
        }
    }

    unsafe fn put_point(at: u64, barrel: &Barrel) {
        let xy = (barrel.x.to_bits() as u64) | ((barrel.y.to_bits() as u64) << 32);
        put(at, xy);
        put(at + 8, 0u64);
    }

    unsafe fn place(object: u64, barrel: &Barrel) {
        put_point(object + GIMMICK_POSITION as u64, barrel);
        put_point(object + BARREL_EFFECT_POSITION as u64, barrel);
        let holder = word(object + GIMMICK_HOLDER as u64);
        if holder == 0 {
            return;
        }
        let areas = word(holder + HOLDER_AREAS as u64);
        if areas == 0 {
            return;
        }
        let instance: extern "C" fn(u64) -> u64 =
            core::mem::transmute(word(word(areas) + AREAS_INSTANCE_SLOT as u64));
        let instance = instance(areas);
        if instance != 0 {
            put_point(instance + INSTANCE_POSITION as u64, barrel);
        }
    }

    unsafe fn riders(object: u64) -> Vec<u32> {
        let count = word(object + BARREL_RIDER_COUNT as u64).min(MAX_RIDERS);
        (0..count)
            .map(|index| core::ptr::read_volatile((object + BARREL_RIDERS as u64 + index * 4) as *const u32))
            .collect()
    }

    unsafe fn sender_of(object: u64) -> Option<u32> {
        let gimmick = word(object + GIMMICK_OBJECT as u64);
        if gimmick == 0 {
            return None;
        }
        let holder = word(gimmick);
        if holder == 0 {
            return None;
        }
        Some(core::ptr::read_volatile((holder + OBJECT_ID as u64) as *const u32))
    }

    unsafe fn send(kind: u32, rider: u32, object: u64, at: [f32; 2]) {
        let Some(sender) = sender_of(object) else {
            return;
        };
        let slot = word(crate::game::addr(OFF_EVENT_CENTER) as u64);
        if slot == 0 {
            return;
        }
        let center = word(slot);
        if center == 0 {
            return;
        }
        let queue = word(center + EVENT_CENTER_QUEUE as u64);
        if queue == 0 {
            return;
        }
        let mut event = Barrel::event(kind, rider, sender, at);
        let vtable = crate::game::addr(OFF_BARREL_EVENT_VTABLE) as u64;
        event.0[0] = vtable as u32;
        event.0[1] = (vtable >> 32) as u32;
        let dispatch: extern "C" fn(u64, *mut BarrelEvent) =
            core::mem::transmute(crate::game::addr(OFF_EVENT_QUEUE));
        dispatch(queue, &mut event);
    }

    unsafe fn fly(object: u64, barrel: &Barrel) {
        let Ok(mut flights) = flights().lock() else {
            return;
        };
        let mut done = Vec::new();
        for (index, flight) in flights.flying.iter_mut().enumerate() {
            if flight.object != object {
                continue;
            }
            match barrel.path_point(flight.frame) {
                Some(at) => send(BARREL_EVENT_SET_PATH, flight.rider, object, at),
                None => {
                    let at = *barrel.path.last().unwrap_or(&[barrel.x, barrel.y]);
                    send(BARREL_EVENT_END_PATH, flight.rider, object, at);
                    done.push(index);
                }
            }
            flight.frame += barrel.path_speed;
        }
        for index in done.into_iter().rev() {
            flights.flying.remove(index);
        }
    }

    #[skyline::hook(offset = crate::game::rel(OFF_BARREL_UPDATE))]
    unsafe fn barrel_update(object: u64, delta: f32) {
        let mut placed = None;
        if let Ok(mut live) = live_barrels().lock() {
            if let Some((barrel, motion)) = live.get_mut(&object) {
                let state = core::ptr::read_volatile((object + BARREL_STATE as u64) as *const u32);
                if barrel.turns(state == BARREL_HOLDING) {
                    barrel.step(motion);
                    put(object + GIMMICK_ROTATION_Z as u64, Barrel::class_rotation(motion.angle));
                }
                if state == BARREL_HOLDING && barrel.fires_at(motion.angle) {
                    put(object + BARREL_STATE as u64, BARREL_FIRING);
                }
                placed = Some(barrel.clone());
            }
        }
        let firing = placed.as_ref().is_some_and(|barrel| barrel.follows_path())
            && core::ptr::read_volatile((object + BARREL_STATE as u64) as *const u32) == BARREL_FIRING;
        if firing {
            let loaded = riders(object);
            if let Ok(mut flights) = flights().lock() {
                flights.loaded.insert(object, loaded);
            }
        }
        call_original!(object, delta);
        if let Some(barrel) = placed {
            if barrel.follows_path() {
                let still = core::ptr::read_volatile((object + BARREL_STATE as u64) as *const u32) == BARREL_FIRING;
                if !still {
                    if let Ok(mut flights) = flights().lock() {
                        if let Some(loaded) = flights.loaded.remove(&object) {
                            for rider in loaded {
                                flights.flying.push(Flight { object, rider, frame: 0.0 });
                            }
                        }
                    }
                }
                fly(object, &barrel);
            }
            place(object, &barrel);
            if word(object + GIMMICK_MODEL as u64) != 0 {
                let matrix: extern "C" fn(u64) = core::mem::transmute(crate::game::addr(OFF_GIMMICK_MATRIX));
                matrix(object);
            }
        }
    }

    #[skyline::hook(offset = crate::game::rel(OFF_BARREL_EVENT))]
    unsafe fn barrel_event(object: u64, event: u64) {
        call_original!(object, event);
        let ours = live_barrels()
            .lock()
            .map(|live| live.contains_key(&object))
            .unwrap_or(false);
        if ours
            && core::ptr::read_volatile((event + EVENT_KIND as u64) as *const u32) == EVENT_MAY_ENTER
            && core::ptr::read_volatile((object + BARREL_STATE as u64) as *const u32) == BARREL_EMPTY
        {
            put(event + EVENT_ANSWER as u64, 1u8);
        }
    }

    #[skyline::hook(offset = crate::game::rel(OFF_BARREL_DELETE))]
    unsafe fn barrel_delete(object: u64) -> u64 {
        if let Ok(mut live) = live_barrels().lock() {
            live.remove(&object);
        }
        if let Ok(mut flights) = flights().lock() {
            flights.loaded.remove(&object);
            flights.flying.retain(|flight| flight.object != object);
        }
        if let (Some(sender), Ok(mut senders)) = (sender_of(object), path_senders().lock()) {
            senders.remove(&sender);
        }
        call_original!(object)
    }

    unsafe fn answer_door(event: u64) -> bool {
        if word(event) != crate::game::addr(OFF_DOOR_EVENT_VTABLE) as u64
            || core::ptr::read_volatile((event + EVENT_KIND as u64) as *const u32) != DOOR_EVENT_IN
        {
            return false;
        }
        let target = core::ptr::read_volatile((event + EVENT_TARGET as u64) as *const u32);
        let Some((index, door)) = live_doors()
            .lock()
            .ok()
            .and_then(|live| live.doors.get(&target).copied())
        else {
            return false;
        };
        let [x, y] = door.center();
        put(event + DOOR_EVENT_CENTER as u64, x);
        put(event + DOOR_EVENT_CENTER as u64 + 4, y);
        put(event + DOOR_EVENT_NO_INTO as u64, 0u8);
        let sender = core::ptr::read_volatile((event + EVENT_SENDER as u64) as *const u32);
        skyline::println!("[gimmick] door {index} ({target:#x}) takes fighter {sender:#x} to ({x}, {y})");
        true
    }

    #[skyline::hook(offset = crate::game::rel(OFF_EVENT_QUEUE))]
    unsafe fn queue_event(queue: u64, event: u64) {
        if answer_door(event) {
            return;
        }
        let barrel_event = word(event) == crate::game::addr(OFF_BARREL_EVENT_VTABLE) as u64;
        if barrel_event
            && core::ptr::read_volatile((event + EVENT_KIND as u64) as *const u32) == BARREL_EVENT_SET_SPEED
        {
            let sender = core::ptr::read_volatile((event + EVENT_SENDER as u64) as *const u32);
            if path_senders().lock().map(|senders| senders.contains(&sender)).unwrap_or(false) {
                return;
            }
        }
        call_original!(queue, event)
    }

    unsafe fn sites_hold(sites: &[(usize, u32, &str)], what: &str) -> bool {
        for &(site, expected, label) in sites {
            let observed = crate::game::text_word(site);
            if observed != expected {
                skyline::println!(
                    "[gimmick] REFUSED {label} at {site:#x}: expected {expected:#010x}, \
                     found {observed:#010x}; no {what} will be built"
                );
                return false;
            }
        }
        true
    }

    unsafe fn slot_holds(vtable_offset: usize, slot: usize, target: usize, what: &str) -> bool {
        let vtable = crate::game::addr(vtable_offset) as u64;
        let found = word(vtable + slot as u64);
        let expected = crate::game::addr(target) as u64;
        if found != expected {
            skyline::println!(
                "[gimmick] REFUSED vtable {vtable_offset:#x}: slot {slot:#x} holds {found:#x}, \
                 not {expected:#x}; no {what} will be built"
            );
            return false;
        }
        true
    }

    unsafe fn arm_queue() -> bool {
        if QUEUE_INSTALLED.load(Ordering::Acquire) {
            return true;
        }
        if !sites_hold(&[(OFF_EVENT_QUEUE, EVENT_QUEUE_OPCODE, "gimmick event queue")], "barrels or doors") {
            return false;
        }
        if !QUEUE_INSTALLED.swap(true, Ordering::AcqRel) {
            skyline::install_hooks!(queue_event);
        }
        true
    }

    pub fn arm(barrels: bool, doors: bool) {
        unsafe {
            if !INSTALLED.swap(true, Ordering::AcqRel) {
                let sites = [
                    (OFF_SETUP_DONE, SETUP_DONE_OPCODE, "stage setup end"),
                    (OFF_ALLOCATE, ALLOCATE_OPCODE, "allocator"),
                    (OFF_GIMMICK_BASE, GIMMICK_BASE_OPCODE, "gimmick constructor"),
                    (OFF_SET_POINTS, SET_POINTS_OPCODE, "ladder points"),
                    (OFF_START, START_OPCODE, "gimmick start"),
                    (OFF_LADDER_AREA, LADDER_AREA_OPCODE, "ladder area"),
                ];
                if !sites_hold(&sites, "gimmicks")
                    || !slot_holds(OFF_LADDER_VTABLE, AREA_SLOT, OFF_LADDER_AREA, "ladders")
                {
                    return;
                }
                skyline::install_hooks!(setup_done);
                skyline::println!("[gimmick] armed at the stage setup end {OFF_SETUP_DONE:#x}");
                let sites = [
                    (OFF_SPRING_CTOR, SPRING_CTOR_OPCODE, "spring constructor"),
                    (OFF_SPRING_SYNC, SPRING_SYNC_OPCODE, "spring sync"),
                    (OFF_SPRING_AREA, SPRING_AREA_OPCODE, "spring area"),
                ];
                if sites_hold(&sites, "springs") && slot_holds(OFF_SPRING_VTABLE, AREA_SLOT, OFF_SPRING_AREA, "springs") {
                    SPRINGS_READY.store(true, Ordering::Release);
                }
            }
            if barrels && !BARRELS_INSTALLED.swap(true, Ordering::AcqRel) {
                let sites = [
                    (OFF_BARREL_CTOR, BARREL_CTOR_OPCODE, "barrel constructor"),
                    (OFF_BARREL_SET_DATA, BARREL_SET_DATA_OPCODE, "barrel data"),
                    (OFF_BARREL_UPDATE, BARREL_UPDATE_OPCODE, "barrel update"),
                    (OFF_BARREL_DELETE, BARREL_DELETE_OPCODE, "barrel delete"),
                    (OFF_BARREL_EVENT, BARREL_EVENT_OPCODE, "barrel event"),
                    (OFF_GIMMICK_MATRIX, GIMMICK_MATRIX_OPCODE, "gimmick matrix"),
                ];
                if sites_hold(&sites, "barrels")
                    && slot_holds(OFF_BARREL_VTABLE, UPDATE_SLOT, OFF_BARREL_UPDATE, "barrels")
                    && slot_holds(OFF_BARREL_VTABLE, DELETE_SLOT, OFF_BARREL_DELETE, "barrels")
                    && slot_holds(OFF_BARREL_VTABLE, EVENT_SLOT, OFF_BARREL_EVENT, "barrels")
                    && arm_queue()
                {
                    skyline::install_hooks!(barrel_update, barrel_delete, barrel_event);
                    BARRELS_READY.store(true, Ordering::Release);
                    skyline::println!("[gimmick] barrels armed: class {OFF_BARREL_VTABLE:#x}");
                }
            }
            if doors
                && !DOORS_READY.load(Ordering::Acquire)
                && slot_holds(OFF_DOOR_EVENT_VTABLE, 0, OFF_DOOR_EVENT_DTOR, "doors")
                && arm_queue()
            {
                DOORS_READY.store(true, Ordering::Release);
                skyline::println!("[gimmick] doors armed: event {OFF_DOOR_EVENT_VTABLE:#x}");
            }
        }
    }
}

#[cfg(all(not(test), feature = "stage_config_bridge"))]
pub fn arm(barrels: bool, doors: bool) {
    live::arm(barrels, doors);
}

#[cfg(not(all(not(test), feature = "stage_config_bridge")))]
pub fn arm(_barrels: bool, _doors: bool) {}
