use std::sync::atomic::AtomicUsize;

use crate::{resolve, Error};

type SpawnFn = unsafe extern "C" fn(i32, f32, f32, f32, f32, i32) -> u32;

static SPAWN_FN: AtomicUsize = AtomicUsize::new(0);

const BOMA_IN_ITEM: usize = 0x98;
const ITEM_ALIAS_OWNER: usize = 0x90;
const ALIAS_FIRST_NODE: usize = 0x200;
const NODE_KEY: usize = 0x10;
const NODE_VALUE: usize = 0x18;
const HASH_MASK: u64 = 0xff_ffff_ffff;
const NODE_LIMIT: usize = 512;

fn hash40(name: &str) -> u64 {
    let mut crc = 0xffff_ffffu32;
    for byte in name.as_bytes() {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xedb8_8320 } else { crc >> 1 };
        }
    }
    ((name.len() as u64) << 32) | u64::from(!crc)
}

pub unsafe fn alias_status(item: usize, name: &str) -> Option<i32> {
    let owner = core::ptr::read_volatile((item - BOMA_IN_ITEM + ITEM_ALIAS_OWNER) as *const usize);
    if owner == 0 {
        return None;
    }
    let wanted = hash40(name);
    let mut node = core::ptr::read_volatile((owner + ALIAS_FIRST_NODE) as *const usize);
    let mut seen = 0;
    while node != 0 && seen < NODE_LIMIT {
        let key = core::ptr::read_volatile((node + NODE_KEY) as *const u64) & HASH_MASK;
        if key == wanted {
            return Some(core::ptr::read_volatile((node + NODE_VALUE) as *const i32));
        }
        node = core::ptr::read_volatile(node as *const usize);
        seen += 1;
    }
    None
}

pub const NO_OBJECT: u32 = 0x5000_0000;

pub fn spawn(kind: i32, x: f32, y: f32, z: f32, lr: f32, variation: i32) -> Result<u32, Error> {
    let address = resolve(&SPAWN_FN, b"clone_engine_item_spawn\0").ok_or(Error::EngineUnavailable)?;
    let function: SpawnFn = unsafe { std::mem::transmute(address) };
    let id = unsafe { function(kind, x, y, z, lr, variation) };
    if id == NO_OBJECT {
        return Err(Error::Engine(-1));
    }
    Ok(id)
}
