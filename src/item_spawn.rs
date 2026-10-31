use core::sync::atomic::{AtomicBool, Ordering};

const OFF_CREATE_ITEM: usize = 0x15d_b0b0;
const CREATE_ITEM_OPCODE: u32 = 0xd103_c3ff;
const OFF_ITEM_MANAGER: usize = 0x52c_3070;
const ITEM_OBJECT_ID: usize = 8;
const POOL_FREE_BEGIN: usize = 0x58;
const POOL_FREE_END: usize = 0x60;
const NO_OBJECT: u32 = 0x5000_0000;
const NO_LINK: i32 = -1;
const SLOW_DEFAULT: u32 = 0x14;
const ONE: f32 = 1.0;
const STRAIGHT: u32 = 0;
const PLACE: u32 = 1;
const QUIET: u32 = 0;

pub(crate) const UNAVAILABLE: u32 = NO_OBJECT;

#[repr(C, align(16))]
struct Descriptor {
    position: [f32; 4],
    origin: [f32; 4],
    kind: i32,
    creator: u32,
    variation: i32,
    lr: f32,
    slow_owner: u32,
    slow_rate: u32,
    link: i32,
    power: f32,
    reserved: [i32; 2],
    scale: f32,
    tail: [u32; 1],
}

type CreateFn = unsafe extern "C" fn(usize, *const Descriptor, u32, u32, u32) -> usize;

static ARMED: AtomicBool = AtomicBool::new(false);

pub(crate) fn spawn(kind: i32, x: f32, y: f32, z: f32, lr: f32, variation: i32) -> u32 {
    if !ARMED.load(Ordering::Acquire) {
        return UNAVAILABLE;
    }
    let descriptor = Descriptor {
        position: [x, y, z, 0.0],
        origin: [x, y, z, 0.0],
        kind,
        creator: NO_OBJECT,
        variation,
        lr: if lr < 0.0 { -ONE } else { ONE },
        slow_owner: NO_OBJECT,
        slow_rate: SLOW_DEFAULT,
        link: NO_LINK,
        power: 0.0,
        reserved: [0; 2],
        scale: ONE,
        tail: [0],
    };
    unsafe {
        let manager = core::ptr::read_volatile(crate::game::addr(OFF_ITEM_MANAGER) as *const usize);
        if manager == 0 {
            return UNAVAILABLE;
        }
        crate::item_slots::prepare(kind);
        let create: CreateFn = core::mem::transmute(crate::game::addr(OFF_CREATE_ITEM));
        let item = create(manager, &descriptor, STRAIGHT, PLACE, QUIET);
        if item == 0 {
            skyline::println!("[itemspawn] kind {kind:#x} at ({x:.0}, {y:.0}): refused");
            return UNAVAILABLE;
        }
        let id = core::ptr::read_volatile((item + ITEM_OBJECT_ID) as *const u32);
        let pool = core::ptr::read_volatile(manager as *const usize);
        let free = if pool == 0 {
            -1
        } else {
            let begin = core::ptr::read_volatile((pool + POOL_FREE_BEGIN) as *const usize);
            let end = core::ptr::read_volatile((pool + POOL_FREE_END) as *const usize);
            (end.wrapping_sub(begin) / 4) as isize
        };
        skyline::println!("[itemspawn] kind {kind:#x} at ({x:.0}, {y:.0}): object {id:#x}, {free} free item slots");
        id
    }
}

pub(crate) fn install() {
    let observed = unsafe { crate::game::text_word(OFF_CREATE_ITEM) };
    if observed != CREATE_ITEM_OPCODE {
        skyline::println!(
            "[itemspawn] REFUSED {OFF_CREATE_ITEM:#x}: expected {CREATE_ITEM_OPCODE:#010x}, found {observed:#010x}"
        );
        return;
    }
    ARMED.store(true, Ordering::Release);
}

#[no_mangle]
pub extern "C" fn clone_engine_item_spawn(kind: i32, x: f32, y: f32, z: f32, lr: f32, variation: i32) -> u32 {
    spawn(kind, x, y, z, lr, variation)
}
