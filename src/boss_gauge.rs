use clone_engine_api::{ERROR_BACKEND_UNAVAILABLE, ERROR_NULL, ERROR_UNSUPPORTED};
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::ffi::CString;
use std::sync::Mutex;

const BOSS_MANAGER: usize = 0x52B_7EF8;
const HUD_OWNER: usize = 0x52C_2420;
const ENTRY_VTABLE: usize = 0x4F7_3AD0;
const BLOCK_VTABLE: usize = 0x4F7_39D0;
const GAME_ALLOC: usize = 0x392_DCE0;
const GROW_ENTRIES: usize = 0x4E9_B60;
const GROW_ENTRIES_WORD: u32 = 0xD101_83FF;
const ANIMATION_FRAME: usize = 0x377_AA60;
const TEXT_SET: usize = 0x37A_22F0;
const HUD_STATE_WAIT: usize = 0x138_FB80;
const HUD_STATE_WAIT_WORD: u32 = 0xFC16_0FEE;
const HUD_SETUP: usize = 0x13B_3B80;
const HUD_SETUP_WORD: u32 = 0xD105_83FF;
const CONFIG_BOSS_BATTLE: usize = 0x2C;
const MANAGER_GATE: usize = 0x134_B15C;
const MANAGER_GATE_WORD: u32 = 0x3400_6368;
const CBZ_W8_MASK: u32 = 0xFF00_001F;
const CBZ_W8: u32 = 0x3400_0008;
const NOP: u32 = 0xD503_201F;

const MANAGER_DATA: usize = 8;
const ENTRIES_BEGIN: usize = 0x110;
const ENTRIES_END: usize = 0x118;
const ENTRIES_CAPACITY: usize = 0x120;
const ENTRY_SIZE: usize = 0x10;
const BLOCK_SIZE: usize = 0x38;
const BLOCK_ALIGN: usize = 0x10;
const BLOCK_SHARED: usize = 8;
const BLOCK_WEAK: usize = 0x10;
const BLOCK_ENTRY: usize = 0x18;
const ENTRY_ITEM: usize = 0x20;
const ENTRY_OBJECT: usize = 0x28;
const ENTRY_KIND: usize = 0x2C;
const ENTRY_FLAG: usize = 0x30;
const BOMA_IN_ITEM: usize = 0x98;
const ITEM_OBJECT_ID: usize = 8;

const OWNER_SCENE: usize = 8;
const SCENE_HUD: usize = 0xC28;
const HUD_BOSS_ENABLED: usize = 0x540;
const HUD_BOSS_DATA: usize = 0x580;
const DATA_CONTROLLER: usize = 0x10;
const DATA_STATES: usize = 0xA0;
const DATA_STATE_INDEX: usize = 0xEC;
const DATA_STATE_SLOTS: u32 = 0x13;
const DATA_START: usize = 0x14C;
const CONTROLLER_PARTS: usize = 0x80;
const PART_SECOND_GAUGE: usize = 0x48;
const PART_NAMES: [usize; 2] = [0xD0, 0xE0];
const PART_LAYOUT: usize = 0x10;
const FACE_PANE: &str = "face_bg_00";
const FOUND_TEXT_BOX: usize = 0x10;
const PANE_FLAGS: usize = 0x58;
const PANE_VISIBLE: u8 = 1;
const STATE_READY: u32 = 2;
const SECOND_ON: f32 = 1.0;

pub(crate) const GAUGES: usize = 2;

type AllocFn = unsafe extern "C" fn(usize, usize) -> usize;
type GrowFn = unsafe extern "C" fn(usize, *const [usize; 2]);
type FrameFn = unsafe extern "C" fn(usize, f32) -> u64;
type TextFn = unsafe extern "C" fn(usize, *const u8) -> u64;

struct Gauge {
    object: u32,
    name: String,
}

struct Battle {
    manager: usize,
    gauges: Vec<Gauge>,
    shown: bool,
}

static ARMED: AtomicBool = AtomicBool::new(false);
static NEXT_BATTLE: AtomicBool = AtomicBool::new(false);
static HIDE_NEXT: AtomicBool = AtomicBool::new(false);
static GATE_WORD: AtomicU32 = AtomicU32::new(0);
static BATTLE: Mutex<Battle> = Mutex::new(Battle {
    manager: 0,
    gauges: Vec::new(),
    shown: false,
});
static REPORTS: AtomicUsize = AtomicUsize::new(0);
const REPORT_LIMIT: usize = 16;

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

fn report(message: String) {
    if REPORTS.fetch_add(1, Ordering::Relaxed) < REPORT_LIMIT {
        crate::dbg_log_public(&format!("[bossgauge] {message}"));
    }
}

unsafe fn boss_manager() -> usize {
    read(crate::game::addr(BOSS_MANAGER))
}

unsafe fn hud_data() -> Option<usize> {
    let owner: usize = read(crate::game::addr(HUD_OWNER));
    if owner == 0 {
        return None;
    }
    let scene_holder: usize = read(owner + OWNER_SCENE);
    let scene: usize = if scene_holder != 0 { read(scene_holder) } else { 0 };
    let hud_holder: usize = if scene != 0 { read(scene + SCENE_HUD) } else { 0 };
    let hud: usize = if hud_holder != 0 { read(hud_holder) } else { 0 };
    if hud == 0 || read::<u8>(hud + HUD_BOSS_ENABLED) == 0 {
        return None;
    }
    let data: usize = read(hud + HUD_BOSS_DATA);
    (data != 0 && read::<usize>(data + DATA_CONTROLLER) != 0).then_some(data)
}

unsafe fn hud_state(data: usize) -> Option<u32> {
    let index: u32 = read(data + DATA_STATE_INDEX);
    (index < DATA_STATE_SLOTS).then(|| read(data + DATA_STATES + index as usize * 4))
}

unsafe fn push_entry(manager: usize, item: usize) -> Result<(), String> {
    let data: usize = read(manager + MANAGER_DATA);
    if data == 0 {
        return Err("the boss manager has no data yet".to_string());
    }
    let block = game::<AllocFn>(GAME_ALLOC)(BLOCK_ALIGN, BLOCK_SIZE);
    if block == 0 {
        return Err("no memory for the boss entry".to_string());
    }
    let boma = item + BOMA_IN_ITEM;
    let kind = smash::app::utility::get_kind(&mut *(boma as *mut smash::app::BattleObjectModuleAccessor));
    write(block, crate::game::addr(BLOCK_VTABLE));
    write::<usize>(block + BLOCK_SHARED, 0);
    write::<usize>(block + BLOCK_WEAK, 0);
    write(block + BLOCK_ENTRY, crate::game::addr(ENTRY_VTABLE));
    write(block + ENTRY_ITEM, item);
    write(block + ENTRY_OBJECT, read::<u32>(item + ITEM_OBJECT_ID));
    write(block + ENTRY_KIND, kind);
    write::<u8>(block + ENTRY_FLAG, 0);
    let pair = [block + BLOCK_ENTRY, block];
    let end: usize = read(data + ENTRIES_END);
    if end != read::<usize>(data + ENTRIES_CAPACITY) {
        write(end, pair[0]);
        write(end + 8, pair[1]);
        write(data + ENTRIES_END, end + ENTRY_SIZE);
    } else {
        game::<GrowFn>(GROW_ENTRIES)(data + ENTRIES_BEGIN, &pair);
        write::<usize>(block + BLOCK_SHARED, 0);
    }
    Ok(())
}

unsafe fn adopt_entry(manager: usize, item: usize) -> bool {
    let data: usize = read(manager + MANAGER_DATA);
    if data == 0 {
        return false;
    }
    let first: usize = read(data + ENTRIES_BEGIN);
    if first == 0 || first >= read::<usize>(data + ENTRIES_END) {
        return false;
    }
    let entry: usize = read(first);
    if entry == 0 {
        return false;
    }
    let boma = item + BOMA_IN_ITEM;
    let kind = smash::app::utility::get_kind(&mut *(boma as *mut smash::app::BattleObjectModuleAccessor));
    write(entry + ENTRY_ITEM - BLOCK_ENTRY, item);
    write(entry + ENTRY_OBJECT - BLOCK_ENTRY, read::<u32>(item + ITEM_OBJECT_ID));
    write(entry + ENTRY_KIND - BLOCK_ENTRY, kind);
    true
}

unsafe fn item_of(module_accessor: usize) -> Option<usize> {
    (module_accessor > BOMA_IN_ITEM).then(|| module_accessor - BOMA_IN_ITEM)
}

pub(crate) fn add(module_accessor: usize, name: &str) -> Result<usize, String> {
    if !ARMED.load(Ordering::Acquire) {
        return Err("the boss gauge is off in this engine build".to_string());
    }
    let mut battle = BATTLE.lock().map_err(|_| "the boss gauge state is poisoned".to_string())?;
    unsafe {
        let manager = boss_manager();
        if manager == 0 {
            return Err("no boss manager: call it during a battle".to_string());
        }
        if battle.manager != manager {
            battle.manager = manager;
            battle.gauges.clear();
            battle.shown = false;
        }
        let item = item_of(module_accessor).ok_or_else(|| "not an item".to_string())?;
        let object: u32 = read(item + ITEM_OBJECT_ID);
        if let Some(slot) = battle.gauges.iter().position(|gauge| gauge.object == object) {
            battle.gauges[slot].name = name.to_string();
            return Ok(slot);
        }
        if battle.gauges.len() >= GAUGES {
            return Err(format!("the boss gauge shows {GAUGES} items at most"));
        }
        if !(battle.gauges.is_empty() && adopt_entry(manager, item)) {
            push_entry(manager, item)?;
        }
        battle.gauges.push(Gauge {
            object,
            name: name.to_string(),
        });
        report(format!("item {object:#x} is boss gauge {} ({name:?})", battle.gauges.len() - 1));
        Ok(battle.gauges.len() - 1)
    }
}

pub(crate) fn rename(slot: usize, name: &str) -> Result<(), String> {
    let mut battle = BATTLE.lock().map_err(|_| "the boss gauge state is poisoned".to_string())?;
    let gauge = battle.gauges.get_mut(slot).ok_or_else(|| format!("no boss gauge {slot}"))?;
    gauge.name = name.to_string();
    let shown = battle.shown;
    if shown {
        unsafe {
            if let Some(data) = hud_data() {
                set_name(data, slot, name);
            }
        }
    }
    Ok(())
}

unsafe fn set_name(data: usize, slot: usize, name: &str) {
    let parts: usize = read(read::<usize>(data + DATA_CONTROLLER) + CONTROLLER_PARTS);
    let found: usize = read(parts + PART_NAMES[slot]);
    if found == 0 {
        return;
    }
    let text_box: usize = read(found + FOUND_TEXT_BOX);
    let Ok(text) = CString::new(name) else {
        return;
    };
    if text_box != 0 {
        game::<TextFn>(TEXT_SET)(text_box, text.as_ptr() as *const u8);
    }
}

unsafe fn hide_face(data: usize) {
    let parts: usize = read(read::<usize>(data + DATA_CONTROLLER) + CONTROLLER_PARTS);
    let handle: usize = read(parts + PART_LAYOUT);
    let layout: usize = if handle != 0 { read(handle) } else { 0 };
    if layout == 0 {
        return;
    }
    if let Some((pane, _)) = crate::hud_layouts::find_pane(layout, FACE_PANE) {
        write(pane + PANE_FLAGS, read::<u8>(pane + PANE_FLAGS) & !PANE_VISIBLE);
    }
}

pub(crate) fn frame() {
    if !ARMED.load(Ordering::Acquire) {
        return;
    }
    let Ok(mut battle) = BATTLE.lock() else {
        return;
    };
    unsafe {
        let manager = boss_manager();
        if manager != 0 {
            close_gate();
        }
        if battle.manager != manager {
            battle.manager = manager;
            battle.gauges.clear();
            battle.shown = false;
        }
        if battle.gauges.is_empty() || battle.shown {
            return;
        }
        let Some(data) = hud_data() else {
            return;
        };
        if hud_state(data) != Some(STATE_READY) {
            return;
        }
        let parts: usize = read(read::<usize>(data + DATA_CONTROLLER) + CONTROLLER_PARTS);
        if battle.gauges.len() > 1 {
            game::<FrameFn>(ANIMATION_FRAME)(parts + PART_SECOND_GAUGE, SECOND_ON);
        }
        for (slot, gauge) in battle.gauges.iter().enumerate() {
            set_name(data, slot, &gauge.name);
        }
        hide_face(data);
        write::<u8>(data + DATA_START, 1);
        battle.shown = true;
        report(format!("{} gauge(s) shown", battle.gauges.len()));
    }
}

pub(crate) fn want_next_battle() -> Result<(), String> {
    if !ARMED.load(Ordering::Acquire) {
        return Err("the boss gauge is off in this engine build".to_string());
    }
    NEXT_BATTLE.store(true, Ordering::Release);
    unsafe { open_gate() };
    Ok(())
}

unsafe fn open_gate() {
    let address = crate::game::addr(MANAGER_GATE);
    let word: u32 = read(address);
    if word & CBZ_W8_MASK == CBZ_W8 && GATE_WORD.compare_exchange(0, word, Ordering::AcqRel, Ordering::Acquire).is_ok() {
        if !crate::text_patch::write_words(address, &[NOP]) {
            GATE_WORD.store(0, Ordering::Release);
            report("the boss manager gate could not be opened".to_string());
        }
    }
}

unsafe fn close_gate() {
    let word = GATE_WORD.swap(0, Ordering::AcqRel);
    if word != 0 {
        crate::text_patch::write_words(crate::game::addr(MANAGER_GATE), &[word]);
        report("the battle made its boss manager".to_string());
    }
}

#[skyline::hook(offset = crate::game::rel(HUD_SETUP))]
unsafe fn hud_setup(hud: usize, config: usize) -> u64 {
    if config != 0 && NEXT_BATTLE.swap(false, Ordering::AcqRel) {
        if read::<u32>(config + CONFIG_BOSS_BATTLE) == 0 {
            write::<u32>(config + CONFIG_BOSS_BATTLE, 1);
        }
        report("the battle HUD carries the boss gauge".to_string());
    }
    if config != 0 && HIDE_NEXT.swap(false, Ordering::AcqRel) {
        write::<u32>(config + CONFIG_BOSS_BATTLE, 0);
        report("the battle HUD leaves the boss gauge out".to_string());
    }
    original!()(hud, config)
}

pub(crate) fn hide_next_battle(hide: bool) -> Result<(), String> {
    if !ARMED.load(Ordering::Acquire) {
        return Err("the boss gauge is off in this engine build".to_string());
    }
    HIDE_NEXT.store(hide, Ordering::Release);
    Ok(())
}

pub(crate) fn install() {
    let sites = [
        (GROW_ENTRIES, GROW_ENTRIES_WORD),
        (HUD_STATE_WAIT, HUD_STATE_WAIT_WORD),
        (HUD_SETUP, HUD_SETUP_WORD),
        (MANAGER_GATE, MANAGER_GATE_WORD),
    ];
    for (offset, expected) in sites {
        let word = unsafe { crate::game::text_word(offset) };
        if word != expected {
            crate::dbg_log_public(&format!(
                "[bossgauge] boss code at {offset:#x} not recognised ({word:#010x}); the boss gauge stays off"
            ));
            return;
        }
    }
    skyline::install_hooks!(hud_setup);
    ARMED.store(true, Ordering::Release);
}

#[no_mangle]
pub extern "C" fn clone_engine_boss_gauge_battle_v1() -> i32 {
    match want_next_battle() {
        Ok(()) => 0,
        Err(message) => {
            report(message);
            ERROR_BACKEND_UNAVAILABLE
        }
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_boss_gauge_hide_v1(hide: u32) -> i32 {
    match hide_next_battle(hide != 0) {
        Ok(()) => 0,
        Err(message) => {
            report(message);
            ERROR_BACKEND_UNAVAILABLE
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_boss_gauge_add_v1(module_accessor: u64, name: *const u8, length: usize) -> i32 {
    let Some(name) = text(name, length) else {
        return ERROR_NULL;
    };
    match add(module_accessor as usize, name) {
        Ok(slot) => slot as i32,
        Err(message) => {
            report(message);
            ERROR_BACKEND_UNAVAILABLE
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_boss_gauge_name_v1(slot: u32, name: *const u8, length: usize) -> i32 {
    let Some(name) = text(name, length) else {
        return ERROR_NULL;
    };
    match rename(slot as usize, name) {
        Ok(()) => 0,
        Err(message) => {
            report(message);
            ERROR_UNSUPPORTED
        }
    }
}

unsafe fn text<'a>(pointer: *const u8, length: usize) -> Option<&'a str> {
    if pointer.is_null() {
        return None;
    }
    std::str::from_utf8(std::slice::from_raw_parts(pointer, length)).ok()
}
