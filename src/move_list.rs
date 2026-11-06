use core::sync::atomic::{AtomicBool, Ordering};
use std::ffi::CString;
use std::sync::Mutex;

use dionysus_core::move_list::{MoveList, Pager, Turn, SLOTS};

const OFF_PAGE_COUNT: usize = 0x326_53f0;
const PAGE_COUNT_OPCODE: u32 = 0xd101_43ff;
const OFF_IS_IMAGE: usize = 0x326_56f0;
const IS_IMAGE_OPCODE: u32 = 0xd102_43ff;
const OFF_SETUP: usize = 0x1ba_4be0;
const SETUP_OPCODE: u32 = 0xd105_43ff;
const OFF_PICTURES: usize = 0x1ba_56a0;
const PICTURES_OPCODE: u32 = 0xd102_83ff;
const OFF_PREVIOUS: usize = 0x1ba_8720;
const OFF_NEXT: usize = 0x1ba_8880;
const OFF_FIRST: usize = 0x1ba_89e0;
const PAGE_TURN_OPCODE: u32 = 0xb945_d008;
const OFF_UI_CHARA_NAME: usize = 0x326_1d60;
const OFF_SET_LABEL: usize = 0x377_9b70;
const OFF_SET_TEXT: usize = 0x37a_1cc0;
const OFF_PAGE_TEXTURE: usize = 0x324_0360;
const OFF_SET_PICTURE: usize = 0x377_85d0;
const OFF_CLEAR_PICTURE: usize = 0x377_8af0;
const OFF_PLAY: usize = 0x377_a930;
const UI_DATABASE_HOLDER: usize = 0x532_e730;
const UI_DATABASE: usize = 8;
const UI_DATABASE_TABLE: usize = 0x168;
const UI_HASH_MASK: u64 = 0xff_ffff_ffff;
const SCREEN_RESOURCES: usize = 0x78;
const SCREEN_LOOKS: usize = 0x118;
const LOOK_STRIDE: usize = 0x20;
const IMAGE_LOOK: usize = 0x10;
const SCREEN_SLOTS: usize = 0x4a0;
const SLOT_SIZE: usize = 0x40;
const SLOT_TITLE: usize = 0x00;
const SLOT_TEXT_HELP: usize = 0x10;
const SLOT_TEXT_PANE: usize = 0x18;
const SLOT_IMAGE_HELP: usize = 0x20;
const SLOT_IMAGE_PANE: usize = 0x28;
const SLOT_PICTURE: usize = 0x38;
const HOLDER_PANE: usize = 0x10;
const SCREEN_PAGE: usize = 0x5d0;
const SCREEN_TOTAL: usize = 0x5d4;
const SCREEN_IMAGE_FLAGS: usize = 0x5e0;
const NAME_BUFFER: usize = 0x100;
const NAME_TEXT: usize = 8;
const SCREENS_KEPT: usize = 8;
const PICTURE_DEFAULT: u32 = 1;

type PageCountFn = unsafe extern "C" fn(u64, u64) -> i32;
type IsImageFn = unsafe extern "C" fn(u64, u64, i32) -> u32;
type UiCharaNameFn = unsafe extern "C" fn(*mut u8, u64, u64);
type SetLabelFn = unsafe extern "C" fn(u64, *const u8) -> u64;
type SetTextFn = unsafe extern "C" fn(u64, *const u16, u32, u32) -> u64;
type PageTextureFn = unsafe extern "C" fn(u64, u64, i32) -> u64;
type SetPictureFn = unsafe extern "C" fn(u64, u64) -> u64;
type ClearPictureFn = unsafe extern "C" fn(u64, u32) -> u64;
type PlayFn = unsafe extern "C" fn(u64, f32);

#[derive(Clone, Copy)]
struct Config {
    pages: i32,
    images: Option<u64>,
}

#[derive(Clone, Copy)]
struct Screen {
    address: usize,
    ui_chara: u64,
    pager: Pager,
}

static ARMED: AtomicBool = AtomicBool::new(false);
static CLAMP: AtomicBool = AtomicBool::new(false);
static CONFIGS: Mutex<Vec<(u64, Config)>> = Mutex::new(Vec::new());
static SCREENS: Mutex<Vec<Screen>> = Mutex::new(Vec::new());
static ANNOUNCED: Mutex<Vec<u64>> = Mutex::new(Vec::new());

unsafe fn game<T>(offset: usize) -> T {
    core::mem::transmute_copy(&crate::game::addr(offset))
}

unsafe fn read<T: Copy>(address: usize) -> T {
    core::ptr::read_volatile(address as *const T)
}

unsafe fn write<T>(address: usize, value: T) {
    core::ptr::write_volatile(address as *mut T, value);
}

pub(crate) fn armed() -> bool {
    ARMED.load(Ordering::Acquire)
}

pub(crate) fn register(ui_chara: &str, list: &MoveList) -> Result<i32, String> {
    let pages = i32::from(list.check()?);
    let hash = dionysus_core::hash40(ui_chara) & UI_HASH_MASK;
    let mut configs = CONFIGS.lock().map_err(|_| "the move list registry is unavailable".to_string())?;
    configs.retain(|(known, _)| *known != hash);
    configs.push((
        hash,
        Config {
            pages,
            images: list.image_mask(),
        },
    ));
    Ok(pages)
}

fn config(ui_chara: u64) -> Option<Config> {
    let hash = ui_chara & UI_HASH_MASK;
    CONFIGS
        .lock()
        .ok()?
        .iter()
        .find(|(known, _)| *known == hash)
        .map(|(_, config)| *config)
}

fn screen_of(address: usize) -> Option<Screen> {
    SCREENS.lock().ok()?.iter().find(|screen| screen.address == address).copied()
}

fn keep(screen: Screen) {
    if let Ok(mut screens) = SCREENS.lock() {
        screens.retain(|known| known.address != screen.address);
        if screens.len() >= SCREENS_KEPT {
            screens.remove(0);
        }
        screens.push(screen);
    }
}

fn forget(address: usize) {
    if let Ok(mut screens) = SCREENS.lock() {
        screens.retain(|known| known.address != address);
    }
}

unsafe fn ui_table() -> u64 {
    let holder: usize = read(crate::game::addr(UI_DATABASE_HOLDER));
    if holder == 0 {
        return 0;
    }
    let database: usize = read(holder + UI_DATABASE);
    if database == 0 {
        return 0;
    }
    read(database + UI_DATABASE_TABLE)
}

unsafe fn ui_name(table: u64, ui_chara: u64) -> Option<String> {
    let mut buffer = [0u8; NAME_BUFFER];
    game::<UiCharaNameFn>(OFF_UI_CHARA_NAME)(buffer.as_mut_ptr(), table, ui_chara);
    let text = &buffer[NAME_TEXT..];
    let end = text.iter().position(|byte| *byte == 0)?;
    (end > 0).then(|| String::from_utf8_lossy(&text[..end]).into_owned())
}

unsafe fn set_label(field: usize, label: &str) {
    if let Ok(label) = CString::new(label) {
        game::<SetLabelFn>(OFF_SET_LABEL)(field as u64, label.as_ptr() as *const u8);
    }
}

unsafe fn clear(field: usize) {
    let holder: usize = read(field);
    if holder == 0 {
        return;
    }
    let pane: u64 = read(holder + HOLDER_PANE);
    if pane != 0 {
        let empty = [0u16; 4];
        game::<SetTextFn>(OFF_SET_TEXT)(pane, empty.as_ptr(), 0, 0);
    }
}

unsafe fn refill(screen: &Screen) {
    let table = ui_table();
    if table == 0 {
        return;
    }
    let Some(name) = ui_name(table, screen.ui_chara) else {
        skyline::println!("[movelist] no name for {:#x}; the pages keep their text", screen.ui_chara);
        return;
    };
    let owner: usize = read(screen.address);
    let resources: u64 = if owner == 0 { 0 } else { read(owner + SCREEN_RESOURCES) };
    let mut flags: u64 = read(screen.address + SCREEN_IMAGE_FLAGS);
    for slot in 0..SLOTS as usize {
        let page = screen.pager.slot_page(slot as i32);
        let image = game::<IsImageFn>(OFF_IS_IMAGE)(table, screen.ui_chara, page) & 1 == 1;
        flags = if image { flags | 1 << slot } else { flags & !(1u64 << slot) };
        let look = screen.address + SCREEN_LOOKS + slot * LOOK_STRIDE + if image { IMAGE_LOOK } else { 0 };
        game::<PlayFn>(OFF_PLAY)(look as u64, 1.0);
        let at = screen.address + SCREEN_SLOTS + slot * SLOT_SIZE;
        set_label(at + SLOT_TITLE, &format!("nam_peculiar_{page:02}_{name}"));
        let help = format!("nam_peculiar_help_{page:02}_{name}");
        if image {
            set_label(at + SLOT_IMAGE_HELP, &help);
            clear(at + SLOT_TEXT_PANE);
        } else {
            set_label(at + SLOT_TEXT_HELP, &help);
            clear(at + SLOT_IMAGE_PANE);
        }
        let picture: u64 = read(at + SLOT_PICTURE);
        if picture == 0 {
            continue;
        }
        if image && resources != 0 {
            let texture = game::<PageTextureFn>(OFF_PAGE_TEXTURE)(resources, screen.ui_chara, page);
            game::<SetPictureFn>(OFF_SET_PICTURE)(picture, texture);
        } else {
            game::<ClearPictureFn>(OFF_CLEAR_PICTURE)(picture, PICTURE_DEFAULT);
        }
    }
    write(screen.address + SCREEN_IMAGE_FLAGS, flags);
    skyline::println!(
        "[movelist] page {} of {}: the panels hold pages {} to {}",
        screen.pager.page,
        screen.pager.count,
        screen.pager.base,
        screen.pager.base + SLOTS - 1
    );
}

unsafe fn turn(address: usize, step: impl FnOnce(&mut Pager, i32) -> Turn) -> Option<Turn> {
    let mut screen = screen_of(address)?;
    let physical: i32 = read(address + SCREEN_PAGE);
    let turn = step(&mut screen.pager, physical);
    keep(screen);
    if turn.refill.is_some() {
        refill(&screen);
    }
    if let Some(physical) = turn.physical {
        write::<i32>(address + SCREEN_PAGE, physical);
    }
    Some(turn)
}

#[skyline::hook(offset = crate::game::rel(OFF_PAGE_COUNT))]
unsafe fn page_count(table: u64, ui_chara: u64) -> i32 {
    let count = match config(ui_chara) {
        Some(own) => own.pages,
        None => call_original!(table, ui_chara),
    };
    if CLAMP.load(Ordering::Acquire) {
        count.min(SLOTS)
    } else {
        count
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_IS_IMAGE))]
unsafe fn is_image(table: u64, ui_chara: u64, page: i32) -> u32 {
    match config(ui_chara).and_then(|own| own.images) {
        Some(mask) => u32::from((0..64).contains(&page) && mask >> page & 1 == 1),
        None => call_original!(table, ui_chara, page),
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_SETUP))]
unsafe fn setup(screen: u64, ui_chara: u64, a2: u64, a3: u64, a4: u64, a5: u64) -> u64 {
    forget(screen as usize);
    let table = ui_table();
    let count = if table == 0 {
        0
    } else {
        game::<PageCountFn>(OFF_PAGE_COUNT)(table, ui_chara)
    };
    let paged = count > SLOTS;
    CLAMP.store(paged, Ordering::Release);
    let result = call_original!(screen, ui_chara, a2, a3, a4, a5);
    CLAMP.store(false, Ordering::Release);
    if paged {
        write::<i32>(screen as usize + SCREEN_TOTAL, count + 1);
        keep(Screen {
            address: screen as usize,
            ui_chara,
            pager: Pager::new(count),
        });
        let hash = ui_chara & UI_HASH_MASK;
        if let Ok(mut announced) = ANNOUNCED.lock() {
            if !announced.contains(&hash) {
                announced.push(hash);
                skyline::println!(
                    "[movelist] {:#x}: {count} extra pages, shown {SLOTS} panels at a time",
                    hash
                );
            }
        }
    }
    result
}

#[skyline::hook(offset = crate::game::rel(OFF_PICTURES))]
unsafe fn pictures(screen: u64, ui_chara: u64, a2: u64, a3: u64, a4: u64, a5: u64) -> u64 {
    let result = call_original!(screen, ui_chara, a2, a3, a4, a5);
    if let Some(own) = screen_of(screen as usize).filter(|own| own.pager.base != 0) {
        refill(&own);
    }
    result
}

#[skyline::hook(offset = crate::game::rel(OFF_NEXT))]
unsafe fn next_page(screen: u64, sound: u32) -> u64 {
    match turn(screen as usize, |pager, physical| pager.next(physical)) {
        Some(turn) if !turn.run => 0,
        _ => call_original!(screen, sound),
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_PREVIOUS))]
unsafe fn previous_page(screen: u64, sound: u32) -> u64 {
    match turn(screen as usize, |pager, physical| pager.previous(physical)) {
        Some(turn) if !turn.run => 0,
        _ => call_original!(screen, sound),
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_FIRST))]
unsafe fn first_page(screen: u64, sound: u32) -> u64 {
    let result = call_original!(screen, sound);
    turn(screen as usize, |pager, _| pager.first());
    result
}

pub(crate) fn install() {
    let sites = [
        (OFF_PAGE_COUNT, PAGE_COUNT_OPCODE),
        (OFF_IS_IMAGE, IS_IMAGE_OPCODE),
        (OFF_SETUP, SETUP_OPCODE),
        (OFF_PICTURES, PICTURES_OPCODE),
        (OFF_PREVIOUS, PAGE_TURN_OPCODE),
        (OFF_NEXT, PAGE_TURN_OPCODE),
        (OFF_FIRST, PAGE_TURN_OPCODE),
    ];
    for (offset, expected) in sites {
        let found = unsafe { crate::game::text_word(offset) };
        if found != expected {
            skyline::println!(
                "[movelist] REFUSED: {offset:#x} holds {found:#010x}, not {expected:#010x}; move lists keep at most {SLOTS} extra pages"
            );
            return;
        }
    }
    skyline::install_hooks!(page_count, is_image, setup, pictures, next_page, previous_page, first_page);
    ARMED.store(true, Ordering::Release);
}
