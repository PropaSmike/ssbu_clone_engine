use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

const OFF_UI_BGM_STOP: usize = 0x331_37f0;
const UI_BGM_STOP_OPCODE: u32 = 0xd101_03ff;
const BATTLE_START_STOP_RETURN: usize = 0x230_c308;
const OFF_UI_BGM_PLAY: usize = 0x331_4020;
const UI_BGM_PLAY_OPCODE: u32 = 0xf81d_0ff5;
const OFF_BGM_ENTRY: usize = 0x325_7010;
const BGM_ENTRY_OPCODE: u32 = 0xa9be_4ff4;
const OFF_BGM_DATABASE: usize = 0x532_e730;
const DATABASE_INNER: usize = 8;
const DATABASE_BGM: usize = 0x278;
const OFF_UI_SOUND: usize = 0x532_e8d0;
const UI_SOUND_INNER: usize = 8;
const PLAY_NOW: u32 = 1;
const NO_FADE: u32 = 0;

pub(crate) const UNAVAILABLE: i32 = -1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Request {
    Play(u64),
    Stop,
}

static ARMED: AtomicBool = AtomicBool::new(false);
static KEEP: AtomicBool = AtomicBool::new(false);
static REQUEST: Mutex<Option<Request>> = Mutex::new(None);

type StopFn = unsafe extern "C" fn(usize, u32) -> u64;
type EntryFn = unsafe extern "C" fn(usize, u64) -> usize;
type PlayFn = unsafe extern "C" fn(usize, usize, u32) -> u64;

unsafe fn game<T>(offset: usize) -> T {
    core::mem::transmute_copy(&crate::game::addr(offset))
}

unsafe fn read(address: usize) -> usize {
    core::ptr::read_volatile(address as *const usize)
}

fn queue(request: Request) -> i32 {
    if !ARMED.load(Ordering::Acquire) {
        return UNAVAILABLE;
    }
    match REQUEST.lock() {
        Ok(mut pending) => {
            *pending = Some(request);
            0
        }
        Err(_) => UNAVAILABLE,
    }
}

pub(crate) fn keep_into_battle(keep: bool) -> i32 {
    if !ARMED.load(Ordering::Acquire) {
        return UNAVAILABLE;
    }
    KEEP.store(keep, Ordering::Release);
    0
}

pub(crate) fn play(name: &str) -> i32 {
    queue(Request::Play(clone_engine_core::hash40(name)))
}

pub(crate) fn stop() -> i32 {
    queue(Request::Stop)
}

unsafe fn play_now(hash: u64) {
    let database = read(read(crate::game::addr(OFF_BGM_DATABASE)) + DATABASE_INNER);
    if database == 0 {
        skyline::println!("[battlemusic] no music database; {hash:#012x} not played");
        return;
    }
    let entry = game::<EntryFn>(OFF_BGM_ENTRY)(read(database + DATABASE_BGM), hash);
    if entry == 0 {
        skyline::println!("[battlemusic] no song {hash:#012x}");
        return;
    }
    game::<PlayFn>(OFF_UI_BGM_PLAY)(read(crate::game::addr(OFF_UI_SOUND)), entry, PLAY_NOW);
    skyline::println!("[battlemusic] playing {hash:#012x}");
}

unsafe fn stop_now() {
    let sound = read(read(crate::game::addr(OFF_UI_SOUND)) + UI_SOUND_INNER);
    if sound != 0 {
        game::<StopFn>(OFF_UI_BGM_STOP)(sound, NO_FADE);
    }
}

pub(crate) unsafe fn follow() {
    let Ok(mut pending) = REQUEST.try_lock() else {
        return;
    };
    match pending.take() {
        Some(Request::Play(hash)) => play_now(hash),
        Some(Request::Stop) => stop_now(),
        None => {}
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_UI_BGM_STOP))]
unsafe fn ui_bgm_stop(sound: usize, fade: u32) -> u64 {
    let lr: usize;
    core::arch::asm!("mov {}, x30", out(reg) lr, options(nomem, nostack, preserves_flags));
    if crate::game::offset_of(lr) == BATTLE_START_STOP_RETURN && KEEP.swap(false, Ordering::AcqRel) {
        skyline::println!("[battlemusic] the menu song carries into the battle");
        return 0;
    }
    call_original!(sound, fade)
}

pub(crate) fn install() {
    let words = [
        (OFF_UI_BGM_STOP, UI_BGM_STOP_OPCODE),
        (OFF_UI_BGM_PLAY, UI_BGM_PLAY_OPCODE),
        (OFF_BGM_ENTRY, BGM_ENTRY_OPCODE),
    ];
    for (offset, expected) in words {
        let observed = unsafe { crate::game::text_word(offset) };
        if observed != expected {
            skyline::println!(
                "[battlemusic] REFUSED {offset:#x}: expected {expected:#010x}, found {observed:#010x}; battle music stays vanilla"
            );
            return;
        }
    }
    skyline::install_hook!(ui_bgm_stop);
    ARMED.store(true, Ordering::Release);
}

#[no_mangle]
pub extern "C" fn clone_engine_keep_music(keep: u32) -> i32 {
    keep_into_battle(keep != 0)
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_play_bgm(name: *const core::ffi::c_char) -> i32 {
    if name.is_null() {
        return UNAVAILABLE;
    }
    match core::ffi::CStr::from_ptr(name).to_str() {
        Ok(text) => play(text),
        Err(_) => UNAVAILABLE,
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_stop_bgm() -> i32 {
    stop()
}
