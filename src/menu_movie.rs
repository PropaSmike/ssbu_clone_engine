use clone_engine_core::menu;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

const OFF_MOVIE_SCREEN: usize = 0x185d2e0;
const MOVIE_SCREEN_OPCODE: u32 = 0xd10483ff;
const MOVIE_SHOW_RETURN: usize = 0x185d8f8;
const OFF_SCREEN_STOP: usize = 0x1bf3870;
const SCREEN_STOP_OPCODE: u32 = 0xd10103ff;
const OFF_TASK_RELEASE: usize = 0x32ca770;
const TASK_RELEASE_OPCODE: u32 = 0xf81c0ff7;
const OFF_TASKS_UPDATE: usize = 0x32ca620;
const TASKS_UPDATE_OPCODE: u32 = 0xf81d0ff5;
const OFF_FILE_REQUEST: usize = 0x323b900;
const OFF_FILES_READY: usize = 0x323c5d0;
const FILES_READY_OPCODE: u32 = 0xa9ba6ffc;
const OFF_FILES_CLEAR: usize = 0x323b430;
const FILES_CLEAR_OPCODE: u32 = 0xf81d0ff5;
const OFF_LOADER_VTABLE: usize = 0x5180b40;
const OFF_GAME_ALLOCATE: usize = 0x392dce0;
const MOVIE_SCREEN_LAYOUT: u64 = 0x3f_a193_1e1d;
const ALLOCATION_ALIGN: u32 = 0x10;
const LOADER_SIZE: usize = 0x80;
const LOADER_TREES: [usize; 4] = [0x8, 0x20, 0x38, 0x50];
const LOADER_POOL: usize = 0x68;
const LOADER_POOL_DEFAULT: u32 = 2;
const LOADER_INDEX: usize = 0x70;
const VTABLE_DELETE: usize = 8;
const WIDGET_SUSPENDED: usize = 0x70;
const WIDGET_SCREEN: usize = 0x80;
const SCREEN_REQUESTS: usize = 0x88;
const SCREEN_REQUEST_INDEX: usize = 0xd4;
const SCREEN_STATE: usize = 0xd8;
const SCREEN_STOPPED: usize = 0x104;
const SCREEN_PLAYER: usize = 0xf8;
const PLAYER_VIEW: usize = 8;
const VIEW_PICTURE_SHOWN: usize = 0x111;
const VIEW_DECODER: usize = 0xe8;
const DECODER_PAUSE: usize = 0x10;
const DECODER_RESUME: usize = 0x18;
const STATE_REPORTS: u32 = 16;
const SHOWN_FRAMES: u32 = 4;
const SCREEN_STATE_COUNT: i32 = 0x13;
const SCREEN_START: u32 = 1;
const SCREEN_PLAYING: u32 = 2;
const SCREEN_ENDING: u32 = 4;
const SCREEN_DONE: u32 = 5;
const NAME_SIZE: usize = 0x48;
const NAME_TEXT: usize = 8;
const NAME_LIMIT: usize = 0x3e;
const POPUP_LAYER: u64 = 1;
const LINK_REGISTER: usize = 30;
const KEEP_FRAMES: u32 = 2;

pub(crate) const STATE_IDLE: i32 = 0;
pub(crate) const STATE_BUSY: i32 = 1;
pub(crate) const STATUS_IDLE: i32 = 0;
pub(crate) const STATUS_LOADING: i32 = 1;
pub(crate) const STATUS_SHOWN: i32 = 2;
pub(crate) const STATUS_HELD: i32 = 3;
pub(crate) const STATUS_PAUSED: i32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    Unavailable,
    Path,
    Busy,
    Memory,
}

impl Refusal {
    pub(crate) fn code(self) -> i32 {
        match self {
            Refusal::Unavailable => -1,
            Refusal::Path => -2,
            Refusal::Busy => -3,
            Refusal::Memory => -4,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Loading,
    Playing,
}

struct Movie {
    phase: Phase,
    holder: usize,
    widget: usize,
    name: [u8; NAME_SIZE],
    track: i32,
    stop: bool,
    keep: Option<u32>,
    kept_frames: u32,
    hold: bool,
    held: bool,
    paused: bool,
    shown_frames: u32,
    seen: Option<(u32, bool)>,
    reports: u32,
    pending: Option<(String, u32)>,
}

static ARMED: AtomicBool = AtomicBool::new(false);
static CREATING: AtomicBool = AtomicBool::new(false);
static BATTLE_FRAMES: AtomicU32 = AtomicU32::new(0);
static MOVIE: Mutex<Movie> = Mutex::new(Movie {
    phase: Phase::Idle,
    holder: 0,
    widget: 0,
    name: [0; NAME_SIZE],
    track: 0,
    stop: false,
    keep: None,
    kept_frames: 0,
    hold: false,
    held: false,
    paused: false,
    shown_frames: 0,
    seen: None,
    reports: 0,
    pending: None,
});

type MovieScreenFn = unsafe extern "C" fn(*mut usize, *const usize, *const u8, *const i32);
type FileRequestFn = unsafe extern "C" fn(usize, *const u64, i64, u32, u32) -> u64;
type TaskFn = unsafe extern "C" fn(usize);
type ReadyFn = unsafe extern "C" fn(usize) -> u32;
type AllocateFn = unsafe extern "C" fn(u32, usize) -> *mut u8;

unsafe fn game<T>(offset: usize) -> T {
    core::mem::transmute_copy(&crate::game::addr(offset))
}

unsafe fn read<T: Copy>(address: usize) -> T {
    core::ptr::read_volatile(address as *const T)
}

unsafe fn write<T>(address: usize, value: T) {
    core::ptr::write_volatile(address as *mut T, value);
}

unsafe fn new_loader() -> Option<usize> {
    let loader = game::<AllocateFn>(OFF_GAME_ALLOCATE)(ALLOCATION_ALIGN, LOADER_SIZE) as usize;
    if loader == 0 {
        return None;
    }
    core::ptr::write_bytes(loader as *mut u8, 0, LOADER_SIZE);
    write(loader, crate::game::addr(OFF_LOADER_VTABLE));
    for tree in LOADER_TREES {
        write(loader + tree, loader + tree + 8);
    }
    write(loader + LOADER_POOL, LOADER_POOL_DEFAULT);
    Some(loader)
}

unsafe fn free_loader(loader: usize) {
    game::<TaskFn>(OFF_FILES_CLEAR)(loader);
    let vtable: usize = read(loader);
    let delete: TaskFn = core::mem::transmute(read::<usize>(vtable + VTABLE_DELETE));
    delete(loader);
}

unsafe fn screen_of(widget: usize) -> usize {
    read(widget + WIDGET_SCREEN)
}

unsafe fn request_screen(screen: usize, state: u32) {
    let index: i32 = read(screen + SCREEN_REQUEST_INDEX);
    if (0..SCREEN_STATE_COUNT).contains(&index) {
        write(screen + SCREEN_REQUESTS + index as usize * 4, state);
    }
}

unsafe fn release(movie: &mut Movie) {
    if movie.widget != 0 {
        game::<TaskFn>(OFF_TASK_RELEASE)(movie.widget);
        movie.widget = 0;
    }
    if movie.holder != 0 {
        let holder = movie.holder as *mut usize;
        let loader = *holder;
        if loader != 0 {
            free_loader(loader);
        }
        drop(Box::from_raw(holder));
        movie.holder = 0;
    }
    movie.phase = Phase::Idle;
}

unsafe fn stop_screen(screen: usize) {
    let state: u32 = read(screen + SCREEN_STATE);
    if state < SCREEN_ENDING {
        request_screen(screen, SCREEN_ENDING);
    }
    if state <= SCREEN_ENDING && read::<u8>(screen + SCREEN_STOPPED) == 0 {
        game::<TaskFn>(OFF_SCREEN_STOP)(screen);
    }
}

unsafe fn decoder_of(screen: usize) -> usize {
    let player: usize = read(screen + SCREEN_PLAYER);
    if player == 0 {
        return 0;
    }
    let view: usize = read(player + PLAYER_VIEW);
    if view == 0 {
        return 0;
    }
    read(view + VIEW_DECODER)
}

unsafe fn call_decoder(decoder: usize, slot: usize) {
    let vtable: usize = read(decoder);
    let method: TaskFn = core::mem::transmute(read::<usize>(vtable + slot));
    method(decoder);
}

unsafe fn picture_shown(screen: usize) -> bool {
    let player: usize = read(screen + SCREEN_PLAYER);
    if player == 0 {
        return false;
    }
    let view: usize = read(player + PLAYER_VIEW);
    view != 0 && read::<u8>(view + VIEW_PICTURE_SHOWN) != 0
}

pub(crate) fn play(path: &str, track: u32) -> Result<(), Refusal> {
    if !ARMED.load(Ordering::Acquire) {
        return Err(Refusal::Unavailable);
    }
    if path.is_empty() || path.len() > NAME_LIMIT || path.contains('\0') {
        return Err(Refusal::Path);
    }
    let Ok(mut movie) = MOVIE.lock() else {
        return Err(Refusal::Busy);
    };
    if movie.phase == Phase::Idle {
        return begin(&mut movie, path, track);
    }
    if !(movie.held || movie.keep.is_some() || movie.stop) || movie.pending.is_some() {
        return Err(Refusal::Busy);
    }
    movie.pending = Some((path.to_string(), track));
    movie.keep = None;
    movie.stop = true;
    skyline::println!("[menumovie] {path} plays once the current movie has ended");
    Ok(())
}

fn begin(movie: &mut Movie, path: &str, track: u32) -> Result<(), Refusal> {
    let loader = unsafe { new_loader() }.ok_or(Refusal::Memory)?;
    unsafe {
        let index: i32 = read(loader + LOADER_INDEX);
        let pool: u32 = read(loader + LOADER_POOL);
        game::<FileRequestFn>(OFF_FILE_REQUEST)(loader, &MOVIE_SCREEN_LAYOUT, i64::from(index), pool, 0);
    }
    movie.holder = Box::into_raw(Box::new(loader)) as usize;
    movie.name = [0; NAME_SIZE];
    movie.name[NAME_TEXT..NAME_TEXT + path.len()].copy_from_slice(path.as_bytes());
    movie.track = track.min(i32::MAX as u32) as i32;
    movie.stop = false;
    movie.keep = None;
    movie.hold = false;
    movie.held = false;
    movie.paused = false;
    movie.shown_frames = 0;
    movie.seen = None;
    movie.reports = 0;
    movie.phase = Phase::Loading;
    skyline::println!("[menumovie] {path} (audio track {track}) loading");
    Ok(())
}

unsafe fn start(movie: &mut Movie) {
    let loader = *(movie.holder as *const usize);
    if game::<ReadyFn>(OFF_FILES_READY)(loader) & 1 == 0 {
        return;
    }
    let mut widget = 0usize;
    CREATING.store(true, Ordering::Release);
    game::<MovieScreenFn>(OFF_MOVIE_SCREEN)(
        &mut widget,
        movie.holder as *const usize,
        movie.name.as_ptr(),
        &movie.track,
    );
    CREATING.store(false, Ordering::Release);
    if widget == 0 || screen_of(widget) == 0 {
        skyline::println!("[menumovie] the movie screen was not built");
        release(movie);
        return;
    }
    movie.widget = widget;
    write::<u8>(widget + WIDGET_SUSPENDED, 0);
    request_screen(screen_of(widget), SCREEN_START);
    movie.phase = Phase::Playing;
    skyline::println!("[menumovie] playing");
}

unsafe fn follow(movie: &mut Movie) {
    let screen = screen_of(movie.widget);
    let state: u32 = read(screen + SCREEN_STATE);
    let shown = picture_shown(screen);
    if movie.seen != Some((state, shown)) && movie.reports < STATE_REPORTS {
        movie.reports += 1;
        skyline::println!("[menumovie] screen state {state}, picture {}", if shown { "shown" } else { "hidden" });
    }
    movie.seen = Some((state, shown));
    if shown && state >= SCREEN_PLAYING && movie.shown_frames < SHOWN_FRAMES {
        movie.shown_frames += 1;
    }
    if state == SCREEN_ENDING && read::<u8>(screen + SCREEN_STOPPED) == 0 {
        if movie.hold {
            if !movie.held {
                movie.held = true;
                skyline::println!("[menumovie] the movie ended; holding its last frame");
            }
            return;
        }
        game::<TaskFn>(OFF_SCREEN_STOP)(screen);
    }
    if state == SCREEN_DONE {
        release(movie);
        skyline::println!("[menumovie] finished");
    }
}

fn busy(movie: &Movie) -> i32 {
    if movie.phase == Phase::Idle {
        STATE_IDLE
    } else {
        STATE_BUSY
    }
}

pub(crate) fn state() -> i32 {
    MOVIE.lock().map_or(STATE_BUSY, |movie| busy(&movie))
}

pub(crate) fn status() -> i32 {
    let Ok(movie) = MOVIE.lock() else {
        return STATUS_LOADING;
    };
    if movie.pending.is_some() {
        return STATUS_LOADING;
    }
    match movie.phase {
        Phase::Idle => STATUS_IDLE,
        Phase::Loading => STATUS_LOADING,
        Phase::Playing if movie.held => STATUS_HELD,
        Phase::Playing if movie.paused => STATUS_PAUSED,
        Phase::Playing if movie.shown_frames >= SHOWN_FRAMES => STATUS_SHOWN,
        Phase::Playing => STATUS_LOADING,
    }
}

pub(crate) fn hold() -> i32 {
    let Ok(mut movie) = MOVIE.lock() else {
        return STATE_BUSY;
    };
    if movie.phase != Phase::Idle {
        movie.hold = true;
    }
    busy(&movie)
}

fn set_paused(paused: bool) -> i32 {
    let Ok(mut movie) = MOVIE.lock() else {
        return STATE_BUSY;
    };
    if movie.phase == Phase::Playing && !movie.held && movie.widget != 0 && movie.paused != paused {
        unsafe {
            let decoder = decoder_of(screen_of(movie.widget));
            if decoder != 0 {
                call_decoder(decoder, if paused { DECODER_PAUSE } else { DECODER_RESUME });
                movie.paused = paused;
                skyline::println!("[menumovie] {}", if paused { "paused" } else { "resumed" });
            }
        }
    }
    busy(&movie)
}

pub(crate) fn pause() -> i32 {
    set_paused(true)
}

pub(crate) fn resume() -> i32 {
    set_paused(false)
}

pub(crate) fn stop() -> i32 {
    let Ok(mut movie) = MOVIE.lock() else {
        return STATE_BUSY;
    };
    if movie.phase != Phase::Idle {
        movie.stop = true;
    }
    busy(&movie)
}

pub(crate) fn keep() -> i32 {
    let Ok(mut movie) = MOVIE.lock() else {
        return STATE_BUSY;
    };
    if movie.phase != Phase::Idle {
        movie.keep = Some(BATTLE_FRAMES.load(Ordering::Acquire));
        movie.kept_frames = 0;
        skyline::println!("[menumovie] kept until the next battle starts");
    }
    busy(&movie)
}

pub(crate) fn battle_frame() {
    BATTLE_FRAMES.fetch_add(1, Ordering::AcqRel);
}

fn follow_keep(movie: &mut Movie) {
    let Some(kept_at) = movie.keep else {
        return;
    };
    movie.kept_frames = BATTLE_FRAMES.load(Ordering::Acquire).wrapping_sub(kept_at);
    if movie.kept_frames >= KEEP_FRAMES {
        movie.keep = None;
        movie.stop = true;
        skyline::println!("[menumovie] the battle started; releasing the kept movie");
    }
}

unsafe fn end_early(movie: &mut Movie) {
    movie.stop = false;
    movie.hold = false;
    movie.held = false;
    if movie.paused && movie.widget != 0 {
        let decoder = decoder_of(screen_of(movie.widget));
        if decoder != 0 {
            call_decoder(decoder, DECODER_RESUME);
        }
    }
    movie.paused = false;
    match movie.phase {
        Phase::Idle => {}
        Phase::Loading => release(movie),
        Phase::Playing => {
            stop_screen(screen_of(movie.widget));
            skyline::println!("[menumovie] stopped");
        }
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_TASKS_UPDATE))]
unsafe fn tasks_update(manager: usize) {
    call_original!(manager);
    crate::menu_loading::follow();
    crate::battle_music::follow();
    let Ok(mut movie) = MOVIE.try_lock() else {
        return;
    };
    follow_keep(&mut movie);
    if movie.stop {
        end_early(&mut movie);
    }
    match movie.phase {
        Phase::Idle => {}
        Phase::Loading => start(&mut movie),
        Phase::Playing => follow(&mut movie),
    }
    if movie.phase == Phase::Idle {
        if let Some((path, track)) = movie.pending.take() {
            if let Err(refusal) = begin(&mut movie, &path, track) {
                skyline::println!("[menumovie] {path} REFUSED ({refusal:?})");
            }
        }
    }
}

pub(crate) unsafe fn place_layout(ctx: &mut skyline::hooks::InlineCtx) -> bool {
    if !CREATING.load(Ordering::Acquire) {
        return false;
    }
    let caller = crate::game::offset_of(ctx.registers[LINK_REGISTER].x() as usize);
    if caller != MOVIE_SHOW_RETURN {
        return false;
    }
    ctx.registers[1].set_x(POPUP_LAYER);
    ctx.registers[2].set_x(menu::MOVIE_PRIORITY);
    true
}

pub(crate) fn install() {
    let words = [
        (OFF_MOVIE_SCREEN, MOVIE_SCREEN_OPCODE),
        (OFF_SCREEN_STOP, SCREEN_STOP_OPCODE),
        (OFF_TASK_RELEASE, TASK_RELEASE_OPCODE),
        (OFF_FILES_READY, FILES_READY_OPCODE),
        (OFF_FILES_CLEAR, FILES_CLEAR_OPCODE),
        (OFF_TASKS_UPDATE, TASKS_UPDATE_OPCODE),
    ];
    for (offset, expected) in words {
        let observed = unsafe { crate::game::text_word(offset) };
        if observed != expected {
            skyline::println!(
                "[menumovie] REFUSED {offset:#x}: expected {expected:#010x}, found {observed:#010x}; movies stay off"
            );
            return;
        }
    }
    skyline::install_hook!(tasks_update);
    ARMED.store(true, Ordering::Release);
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_movie_play(path: *const core::ffi::c_char, track: u32) -> i32 {
    if path.is_null() {
        return Refusal::Path.code();
    }
    let Ok(path) = core::ffi::CStr::from_ptr(path).to_str() else {
        return Refusal::Path.code();
    };
    match play(path, track) {
        Ok(()) => 0,
        Err(refusal) => refusal.code(),
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_movie_state() -> i32 {
    state()
}

#[no_mangle]
pub extern "C" fn clone_engine_movie_stop() -> i32 {
    stop()
}

#[no_mangle]
pub extern "C" fn clone_engine_movie_keep() -> i32 {
    keep()
}

#[no_mangle]
pub extern "C" fn clone_engine_movie_status() -> i32 {
    status()
}

#[no_mangle]
pub extern "C" fn clone_engine_movie_hold() -> i32 {
    hold()
}

#[no_mangle]
pub extern "C" fn clone_engine_movie_pause() -> i32 {
    pause()
}

#[no_mangle]
pub extern "C" fn clone_engine_movie_resume() -> i32 {
    resume()
}
