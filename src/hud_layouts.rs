use clone_engine_core::hud::{check_layout, check_part, HudAction};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::ffi::CString;
use std::sync::{Mutex, RwLock};

const BATTLE_HUD_FILE: u64 = 0x2F_26FF_8CC2;
const HASH_MASK: u64 = 0xFF_FFFF_FFFF;
const ENTRIES: usize = 8;

const FILE_REQUEST: usize = 0x323_B900;
const FILE_REQUEST_WORD: u32 = 0xD101_C3FF;
const FILE_RELEASE: usize = 0x323_E5F0;
const FILE_RELEASE_WORD: u32 = 0xA9BE_4FF4;
const LAYOUT_BUILD: usize = 0x32C_B980;
const HANDLE_INIT: usize = 0x377_2C50;
const HANDLE_DESTROY: usize = 0x377_31C0;
const HANDLE_DESTROY_WORD: u32 = 0xD102_03FF;
const LAYOUT_SHOW: usize = 0x377_5810;
const LAYOUT_SHOW_WORD: u32 = 0xD101_43FF;
const LAYOUT_HIDE: usize = 0x377_59F0;
const RENDERER_REMOVE: usize = 0x378_4190;
const RENDERER_REMOVE_WORD: u32 = 0xD101_C3FF;
const FIND_PANE: usize = 0x377_6360;
const FIND_ANIMATION: usize = 0x377_7B30;
const ANIMATION_PLAY: usize = 0x377_A930;
const ANIMATION_FRAME: usize = 0x377_AA60;
const TEXT_SET: usize = 0x37A_22F0;
const GAME_FREE: usize = 0x392_E590;
const HUD_UPDATE: usize = 0x13B_8600;
const HUD_UPDATE_WORD: u32 = 0xD102_83FF;
const HUD_SHOW_CALL: usize = 0x139_E278;
const HUD_SHOW_CALL_WORD: u32 = 0x948F_5D66;
const HUD_SHOW_RETURN: usize = 0x139_E27C;
const LINK_REGISTER: usize = 30;

const LOADER_FILES: usize = 0x10;
const LOADER_RELEASE_TABLE: usize = 8;
const NODE_LEFT: usize = 0;
const NODE_RIGHT: usize = 8;
const NODE_KEY: usize = 0x20;
const LAYOUT_SHOWN: usize = 0x18;
const LAYOUT_FINDER: usize = 8;
const LAYOUT_HOLDER: usize = 0x10;
const HOLDER_UI2D: usize = 8;
const UI2D_ROOT: usize = 0x18;
const CORE_LAYER: usize = 0x648;
const CORE_PRIORITY: usize = 0x64C;
const FOUND_DATA: usize = 1;
const DATA_PANE: usize = 0;
const DATA_TEXT_BOX: usize = 0x10;
const DATA_ANIMATION: usize = 8;
const PANE_TRANSLATE: usize = 0x30;
const PANE_SCALE: usize = 0x48;
const PANE_FLAGS: usize = 0x58;
const PANE_ALPHA: usize = 0x59;
const PANE_GLOBAL_ALPHA: usize = 0x5A;
const PANE_GLOBAL_MATRIX: usize = 0x70;
const MATRIX_FLOATS: usize = 12;
const MATRIX_X: usize = 3;
const MATRIX_Y: usize = 7;
const MATRIX_SCALE_X: usize = 0;
const MATRIX_SCALE_Y: usize = 5;
const PANE_VISIBLE: u8 = 1;
const PANE_MATRIX_DIRTY: u8 = 1 << 4;
const PLAY_SPEED: f32 = 1.0;

const REQUEST_REPORTS: usize = 4;
const BUILD_REPORTS: usize = 16;
const ACT_REPORTS: usize = 12;
const CARD_REPORTS: usize = 2;

#[repr(C)]
struct Found {
    words: [usize; 4],
}

type ReleaseFn = unsafe extern "C" fn(usize, *const u64) -> u64;
type BuildFn = unsafe extern "C" fn(*mut usize, usize, *const u64, u32) -> u64;
type HandleFn = unsafe extern "C" fn(*mut usize) -> u64;
type ShowFn = unsafe extern "C" fn(usize, i32, i32) -> u64;
type HideFn = unsafe extern "C" fn(usize) -> u64;
type FindFn = unsafe extern "C" fn(usize, *const u8, ...) -> Found;
type AnimationFn = unsafe extern "C" fn(*const usize, f32) -> u64;
type TextFn = unsafe extern "C" fn(usize, *const u8) -> u64;
type FreeFn = unsafe extern "C" fn(usize);

struct Layout {
    kind: i32,
    path: String,
    hash: &'static u64,
}

#[derive(Clone)]
enum Kept {
    Shown(bool),
    Frame(f32),
    Text(String),
}

struct Remembered {
    entry: usize,
    part: String,
    kept: Kept,
}

struct Instance {
    kind: i32,
    entry: usize,
    handle: *mut usize,
    layout: usize,
    root: usize,
    card: usize,
    shown: bool,
    animations: Vec<(String, [usize; 2])>,
    panes: Vec<(String, usize, usize)>,
}

struct Stage {
    hud: usize,
    built: bool,
    instances: Vec<Instance>,
    remembered: Vec<Remembered>,
}

unsafe impl Send for Stage {}

static LAYOUTS: RwLock<Vec<Layout>> = RwLock::new(Vec::new());
static STAGE: Mutex<Stage> = Mutex::new(Stage {
    hud: 0,
    built: false,
    instances: Vec::new(),
    remembered: Vec::new(),
});
static ARMED: AtomicBool = AtomicBool::new(false);
static ANY: AtomicBool = AtomicBool::new(false);
static LOADER: AtomicUsize = AtomicUsize::new(0);
static HUD_LAYOUT: AtomicUsize = AtomicUsize::new(0);
static HUD_CORE: AtomicUsize = AtomicUsize::new(0);
static REQUESTS: AtomicUsize = AtomicUsize::new(0);
static BUILDS: AtomicUsize = AtomicUsize::new(0);
static ACTS: AtomicUsize = AtomicUsize::new(0);
static CARDS: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn register(kind: i32, path: &str) -> Result<(), String> {
    check_layout(path)?;
    let mut layouts = LAYOUTS.write().map_err(|_| "the HUD layout table is poisoned".to_string())?;
    if layouts.iter().any(|layout| layout.kind == kind) {
        return Err(format!("kind {kind} already has a HUD layout"));
    }
    let hash = clone_engine_core::hash40(path) & HASH_MASK;
    layouts.push(Layout {
        kind,
        path: path.to_string(),
        hash: Box::leak(Box::new(hash)),
    });
    ANY.store(true, Ordering::Release);
    Ok(())
}

fn layout_of(kind: i32) -> Option<(String, &'static u64)> {
    LAYOUTS
        .read()
        .ok()?
        .iter()
        .find(|layout| layout.kind == kind)
        .map(|layout| (layout.path.clone(), layout.hash))
}

unsafe fn read<T: Copy>(address: usize) -> T {
    core::ptr::read_volatile(address as *const T)
}

unsafe fn write<T: Copy>(address: usize, value: T) {
    core::ptr::write_volatile(address as *mut T, value)
}

unsafe fn game<F: Copy>(offset: usize) -> F {
    let address = crate::game::addr(offset);
    core::mem::transmute_copy(&address)
}

unsafe fn requested(loader: usize, hash: u64) -> bool {
    let end = loader + LOADER_FILES;
    let mut node: usize = read(end);
    let mut candidate = end;
    while node != 0 {
        if read::<u64>(node + NODE_KEY) & HASH_MASK < hash {
            node = read(node + NODE_RIGHT);
        } else {
            candidate = node;
            node = read(node + NODE_LEFT);
        }
    }
    candidate != end && read::<u64>(candidate + NODE_KEY) & HASH_MASK == hash
}

unsafe fn find(finder: usize, offset: usize, name: &str) -> Option<usize> {
    let name = CString::new(name).ok()?;
    let found = game::<FindFn>(offset)(finder, name.as_ptr() as *const u8);
    let data = found.words[FOUND_DATA];
    (data != 0).then_some(data)
}

unsafe fn find_pane(layout: usize, name: &str) -> Option<(usize, usize)> {
    let data = find(layout + LAYOUT_FINDER, FIND_PANE, name)?;
    let pane: usize = read(data + DATA_PANE);
    let text_box: usize = read(data + DATA_TEXT_BOX);
    game::<FreeFn>(GAME_FREE)(data);
    (pane != 0).then_some((pane, text_box))
}

unsafe fn find_animation(layout: usize, name: &str) -> Option<[usize; 2]> {
    let name_c = CString::new(name).ok()?;
    let found = game::<FindFn>(FIND_ANIMATION)(layout + LAYOUT_FINDER, name_c.as_ptr() as *const u8);
    let data = found.words[FOUND_DATA];
    if data == 0 {
        return None;
    }
    if read::<usize>(data + DATA_ANIMATION) == 0 {
        game::<FreeFn>(GAME_FREE)(data);
        return None;
    }
    Some([found.words[0], data])
}

impl Instance {
    unsafe fn pane(&mut self, part: &str) -> Result<(usize, usize), String> {
        if let Some((_, pane, text_box)) = self.panes.iter().find(|(name, ..)| name == part) {
            return Ok((*pane, *text_box));
        }
        let (pane, text_box) =
            find_pane(self.layout, part).ok_or_else(|| format!("the layout has no part {part:?}"))?;
        self.panes.push((part.to_string(), pane, text_box));
        Ok((pane, text_box))
    }

    unsafe fn animation(&mut self, part: &str) -> Result<*const usize, String> {
        if let Some(index) = self.animations.iter().position(|(name, _)| name == part) {
            return Ok(self.animations[index].1.as_ptr());
        }
        let handle = find_animation(self.layout, part)
            .ok_or_else(|| format!("the layout has no animation {part:?}"))?;
        self.animations.push((part.to_string(), handle));
        Ok(self.animations.last().unwrap().1.as_ptr())
    }

    unsafe fn perform(&mut self, action: &HudAction) -> Result<(), String> {
        match *action {
            HudAction::Play(part) => {
                let handle = self.animation(part)?;
                game::<AnimationFn>(ANIMATION_PLAY)(handle, PLAY_SPEED);
            }
            HudAction::Frame(part, frame) => {
                let handle = self.animation(part)?;
                game::<AnimationFn>(ANIMATION_FRAME)(handle, frame);
            }
            HudAction::Show(part, on) => {
                let (pane, _) = self.pane(part)?;
                let flags: u8 = read(pane + PANE_FLAGS);
                write(pane + PANE_FLAGS, if on { flags | PANE_VISIBLE } else { flags & !PANE_VISIBLE });
            }
            HudAction::Text(part, text) => {
                let (_, text_box) = self.pane(part)?;
                if text_box == 0 {
                    return Err(format!("part {part:?} is not a text box"));
                }
                let text = CString::new(text).map_err(|_| "the text cannot contain NUL".to_string())?;
                game::<TextFn>(TEXT_SET)(text_box, text.as_ptr() as *const u8);
            }
            _ => return Err(format!("{} is not a call for the fighter's own layout", action.name())),
        }
        Ok(())
    }

    unsafe fn follow(&self) {
        let matrix = self.card + PANE_GLOBAL_MATRIX;
        let x: f32 = read(matrix + MATRIX_X * 4);
        let y: f32 = read(matrix + MATRIX_Y * 4);
        write(self.root + PANE_TRANSLATE, x);
        write(self.root + PANE_TRANSLATE + 4, y);
        let scale_x: f32 = read(matrix + MATRIX_SCALE_X * 4);
        let scale_y: f32 = read(matrix + MATRIX_SCALE_Y * 4);
        if scale_x.is_finite() && scale_y.is_finite() {
            write(self.root + PANE_SCALE, scale_x);
            write(self.root + PANE_SCALE + 4, scale_y);
        }
        write(self.root + PANE_ALPHA, read::<u8>(self.card + PANE_GLOBAL_ALPHA));
        let card_flags: u8 = read(self.card + PANE_FLAGS);
        let flags: u8 = read(self.root + PANE_FLAGS);
        let visible = if card_flags & PANE_VISIBLE != 0 { PANE_VISIBLE } else { 0 };
        write(self.root + PANE_FLAGS, (flags & !PANE_VISIBLE) | visible | PANE_MATRIX_DIRTY);
        if CARDS.fetch_add(1, Ordering::Relaxed) < CARD_REPORTS {
            let floats: Vec<String> = (0..MATRIX_FLOATS)
                .map(|index| format!("{:.1}", read::<f32>(matrix + index * 4)))
                .collect();
            crate::dbg_log_public(&format!(
                "[hudlayout] entry {} follows its card at ({x:.1}, {y:.1}), flags {card_flags:#x}, alpha {}, matrix [{}]",
                self.entry,
                read::<u8>(self.card + PANE_GLOBAL_ALPHA),
                floats.join(" ")
            ));
        }
    }

    unsafe fn destroy(mut self) {
        for (_, handle) in self.animations.drain(..) {
            game::<FreeFn>(GAME_FREE)(handle[1]);
        }
        game::<HandleFn>(HANDLE_DESTROY)(self.handle);
        drop(Box::from_raw(self.handle));
    }
}

impl Stage {
    unsafe fn tear_down(&mut self) -> usize {
        let count = self.instances.len();
        for instance in self.instances.drain(..) {
            instance.destroy();
        }
        self.remembered.clear();
        self.built = false;
        self.hud = 0;
        count
    }

    fn remember(&mut self, entry: usize, action: &HudAction) {
        let (part, kept) = match *action {
            HudAction::Show(part, on) => (part, Kept::Shown(on)),
            HudAction::Frame(part, frame) => (part, Kept::Frame(frame)),
            HudAction::Text(part, text) => (part, Kept::Text(text.to_string())),
            _ => return,
        };
        let same = |kept_now: &Kept| core::mem::discriminant(kept_now) == core::mem::discriminant(&kept);
        self.remembered
            .retain(|old| !(old.entry == entry && old.part == part && same(&old.kept)));
        self.remembered.push(Remembered {
            entry,
            part: part.to_string(),
            kept,
        });
    }

    unsafe fn build(&mut self, hud: usize) {
        self.built = true;
        self.hud = hud;
        let loader = LOADER.load(Ordering::Acquire);
        for entry in 0..ENTRIES {
            let Some(kind) = crate::css_registration::entry_custom_kind(entry as u8) else {
                continue;
            };
            let Some((path, hash)) = layout_of(kind) else {
                continue;
            };
            let report = |message: String| {
                if BUILDS.fetch_add(1, Ordering::Relaxed) < BUILD_REPORTS {
                    crate::dbg_log_public(&format!("[hudlayout] entry {entry} kind {kind}: {message}"));
                }
            };
            if loader == 0 || !requested(loader, *hash) {
                report(format!("{path} is not loaded (no pack ships it, or the battle HUD never asked); no layout this match"));
                continue;
            }
            let Some(hud_index) = crate::hud_grafts::entry_hud_index(entry) else {
                report("the entry has no HUD card; no layout".to_string());
                continue;
            };
            let card_name = format!("p{}", hud_index + 1);
            let Some((card, _)) = find_pane(hud, &card_name) else {
                report(format!("the battle HUD has no card {card_name}; no layout"));
                continue;
            };
            let handle = Box::into_raw(Box::new(0usize));
            game::<HandleFn>(HANDLE_INIT)(handle);
            let mut slot = handle as usize;
            game::<BuildFn>(LAYOUT_BUILD)(&mut slot, loader, hash, 0);
            let layout = *handle;
            let holder: usize = if layout != 0 { read(layout + LAYOUT_HOLDER) } else { 0 };
            let ui2d: usize = if holder != 0 { read(holder + HOLDER_UI2D) } else { 0 };
            let root: usize = if ui2d != 0 { read(ui2d + UI2D_ROOT) } else { 0 };
            if root == 0 {
                game::<HandleFn>(HANDLE_DESTROY)(handle);
                drop(Box::from_raw(handle));
                report(format!("{path} built no layout (is info/layout.info in the archive?)"));
                continue;
            }
            self.instances.push(Instance {
                kind,
                entry,
                handle,
                layout,
                root,
                card,
                shown: false,
                animations: Vec::new(),
                panes: Vec::new(),
            });
            report(format!("built {path} on card {card_name}"));
        }
        if loader != 0 {
            let hashes: Vec<&'static u64> = match LAYOUTS.read() {
                Ok(layouts) => layouts.iter().map(|layout| layout.hash).collect(),
                Err(_) => Vec::new(),
            };
            for hash in hashes {
                if requested(loader, *hash) {
                    game::<ReleaseFn>(FILE_RELEASE)(loader + LOADER_RELEASE_TABLE, hash);
                }
            }
        }
        let remembered: Vec<(usize, String, Kept)> = self
            .remembered
            .iter()
            .map(|kept| (kept.entry, kept.part.clone(), kept.kept.clone()))
            .collect();
        for (entry, part, kept) in remembered {
            let Some(instance) = self.instances.iter_mut().find(|instance| instance.entry == entry) else {
                continue;
            };
            let action = match &kept {
                Kept::Shown(on) => HudAction::Show(&part, *on),
                Kept::Frame(frame) => HudAction::Frame(&part, *frame),
                Kept::Text(text) => HudAction::Text(&part, text),
            };
            if let Err(message) = instance.perform(&action) {
                crate::dbg_log_public(&format!("[hudlayout] entry {entry}: {} {part:?} kept from before the HUD: {message}", action.name()));
            }
        }
    }

    unsafe fn show(&mut self, hud: usize) {
        let core: usize = read(hud);
        let layer: i32 = read(core + CORE_LAYER);
        let priority: i32 = read(core + CORE_PRIORITY);
        for instance in self.instances.iter_mut().filter(|instance| !instance.shown) {
            game::<ShowFn>(LAYOUT_SHOW)(instance.layout, layer, priority);
            instance.shown = true;
            if BUILDS.fetch_add(1, Ordering::Relaxed) < BUILD_REPORTS {
                crate::dbg_log_public(&format!(
                    "[hudlayout] entry {} kind {}: shown on layer {layer} priority {priority:#x}",
                    instance.entry, instance.kind
                ));
            }
        }
    }

    unsafe fn hide(&mut self) {
        for instance in self.instances.iter_mut().filter(|instance| instance.shown) {
            game::<HideFn>(LAYOUT_HIDE)(instance.layout);
            instance.shown = false;
        }
    }
}

#[skyline::hook(offset = crate::game::rel(FILE_REQUEST))]
unsafe fn file_request(loader: usize, hash: *const u64, index: i64, flags: u32, extra: u32) -> u64 {
    let result = original!()(loader, hash, index, flags, extra);
    if !ANY.load(Ordering::Acquire) || hash.is_null() || *hash & HASH_MASK != BATTLE_HUD_FILE {
        return result;
    }
    LOADER.store(loader, Ordering::Release);
    let wanted: Vec<(String, &'static u64)> = match LAYOUTS.read() {
        Ok(layouts) => layouts.iter().map(|layout| (layout.path.clone(), layout.hash)).collect(),
        Err(_) => return result,
    };
    let mut asked: Vec<u64> = Vec::new();
    for (path, hash) in wanted {
        if asked.contains(hash) {
            continue;
        }
        asked.push(*hash);
        original!()(loader, hash, index, flags, extra);
        if REQUESTS.fetch_add(1, Ordering::Relaxed) < REQUEST_REPORTS {
            crate::dbg_log_public(&format!("[hudlayout] {path} requested beside the battle HUD"));
        }
    }
    result
}

#[skyline::hook(offset = crate::game::rel(LAYOUT_SHOW), inline)]
unsafe fn layout_show(ctx: &mut skyline::hooks::InlineCtx) {
    if !ANY.load(Ordering::Acquire) {
        return;
    }
    let caller = crate::game::offset_of(ctx.registers[LINK_REGISTER].x() as usize);
    if caller != HUD_SHOW_RETURN {
        return;
    }
    let layout = ctx.registers[0].x() as usize;
    if layout == 0 {
        return;
    }
    HUD_LAYOUT.store(layout, Ordering::Release);
    HUD_CORE.store(read(layout), Ordering::Release);
}

#[skyline::hook(offset = crate::game::rel(RENDERER_REMOVE))]
unsafe fn renderer_remove(renderer: usize, core: usize) -> u64 {
    let result = original!()(renderer, core);
    if core != 0 && core == HUD_CORE.load(Ordering::Acquire) {
        if let Ok(mut stage) = STAGE.lock() {
            stage.hide();
        }
    }
    result
}

#[skyline::hook(offset = crate::game::rel(HANDLE_DESTROY))]
unsafe fn handle_destroy(handle: *mut usize) -> u64 {
    let hud = HUD_LAYOUT.load(Ordering::Acquire);
    if hud != 0 && !handle.is_null() && *handle == hud {
        HUD_LAYOUT.store(0, Ordering::Release);
        HUD_CORE.store(0, Ordering::Release);
        if let Ok(mut stage) = STAGE.lock() {
            let count = stage.tear_down();
            if count != 0 {
                crate::dbg_log_public(&format!("[hudlayout] the battle HUD went away; {count} layout(s) destroyed"));
            }
        }
    }
    original!()(handle)
}

#[skyline::hook(offset = crate::game::rel(HUD_UPDATE))]
unsafe fn hud_update(manager: usize) -> u64 {
    let result = original!()(manager);
    if !ANY.load(Ordering::Acquire) {
        return result;
    }
    let hud = HUD_LAYOUT.load(Ordering::Acquire);
    if hud == 0 || read::<u8>(hud + LAYOUT_SHOWN) == 0 {
        return result;
    }
    let Ok(mut stage) = STAGE.lock() else {
        return result;
    };
    if stage.built && stage.hud != hud {
        let count = stage.tear_down();
        crate::dbg_log_public(&format!("[hudlayout] a new battle HUD replaced the old one; {count} stale layout(s) destroyed"));
    }
    if !stage.built {
        stage.build(hud);
    }
    stage.show(hud);
    for instance in &stage.instances {
        instance.follow();
    }
    result
}

pub(crate) fn act(kind: i32, entry: usize, action: &HudAction) -> Result<bool, String> {
    if !ARMED.load(Ordering::Acquire) {
        return Err("own HUD layouts are off in this engine build".to_string());
    }
    let Some((path, _)) = layout_of(kind) else {
        return Err("the fighter has no hud layout".to_string());
    };
    check_part(action)?;
    let mut stage = STAGE.lock().map_err(|_| "the HUD layout state is poisoned".to_string())?;
    stage.remember(entry, action);
    let Some(instance) = stage.instances.iter_mut().find(|instance| instance.entry == entry) else {
        return Ok(false);
    };
    unsafe { instance.perform(action)? };
    if ACTS.fetch_add(1, Ordering::Relaxed) < ACT_REPORTS {
        crate::dbg_log_public(&format!("[hudlayout] entry {entry} kind {kind}: {action:?} on {path}"));
    }
    Ok(true)
}

pub(crate) fn install() {
    let sites = [
        (FILE_REQUEST, FILE_REQUEST_WORD),
        (FILE_RELEASE, FILE_RELEASE_WORD),
        (HANDLE_DESTROY, HANDLE_DESTROY_WORD),
        (LAYOUT_SHOW, LAYOUT_SHOW_WORD),
        (RENDERER_REMOVE, RENDERER_REMOVE_WORD),
        (HUD_UPDATE, HUD_UPDATE_WORD),
        (HUD_SHOW_CALL, HUD_SHOW_CALL_WORD),
    ];
    for (offset, expected) in sites {
        let word = unsafe { crate::game::text_word(offset) };
        if word != expected {
            crate::dbg_log_public(&format!(
                "[hudlayout] layout code at {offset:#x} not recognised ({word:#010x}); own HUD layouts stay off"
            ));
            return;
        }
    }
    skyline::install_hooks!(
        file_request,
        layout_show,
        renderer_remove,
        handle_destroy,
        hud_update
    );
    ARMED.store(true, Ordering::Release);
}
