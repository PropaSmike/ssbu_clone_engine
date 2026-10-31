use clone_engine_api::{ERROR_BACKEND_UNAVAILABLE, ERROR_NULL, ERROR_UNSUPPORTED};
use core::arch::aarch64::{float32x4_t, vgetq_lane_f32};
use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

const MATCH_SETUP: usize = 0x52B_84F8;
const ENTRY_TABLE: usize = 0x52C_4180;
const ENTRY_TABLE_READY: usize = 0x531_34D8;
const UI_ROOT: usize = 0x532_E730;
const EXPAND: usize = 0x65_DBC0;
const EXPAND_WORD: u32 = 0x9280_0008;
const INSTALL: usize = 0x65_3240;
const INSTALL_WORD: u32 = 0xD101_C3FF;
const INSTALL_FLAGS: usize = 0x453_B1BC;
const CHARACTER_INFO: usize = 0x326_3870;
const CHARACTER_INFO_WORD: u32 = 0xF81B_0FF9;
const UI_KIND: usize = 0x326_2130;
const UI_KIND_WORD: u32 = 0xD101_43FF;
const ACTIVATE: usize = 0x65_6EE0;
const SETTLE: usize = 0x65_9060;
const SETTLE_WORD: u32 = 0xA9BB_67FA;
const CARD_CHECK: usize = 0x138_B1C0;
const CARD_CHECK_WORD: u32 = 0xF940_0408;
const CARD_REFERENCE_VTABLE: usize = 0x506_2B98;
const KIND_UI: usize = 0x138_C9B0;
const STAGE_POINTS: usize = 0x532_99D8;
const HUD_OWNER: usize = 0x52C_2420;
const TICKETS: usize = 0x52C_40F8;
const TICKET_TAKE: usize = 0x175_6710;
const TICKET_TAKE_WORD: u32 = 0xF81C_0FF7;
const TICKET_GIVE: usize = 0x175_6860;
const TICKET_GIVE_WORD: u32 = 0xD102_03FF;
const FACE_DESCRIBE: usize = 0x138_A3A0;
const FACE_DESCRIBE_WORD: u32 = 0xA9BB_67FA;
const CARD_READY: usize = 0x138_B200;
const CARD_READY_WORD: u32 = 0xF81E_0FF3;
const MODE_ROOT: usize = 0x532_E8B0;
const MODE_DATA: usize = 8;
const MODE_ACTIVE: usize = 0x138;
const MODE_KIND: usize = 0x2B8;
const MODE_WITHOUT_CARDS: u8 = 0xB;

const ENTRY_SETUP: usize = 0x70;
const ENTRY_AI: usize = 0x50;
const ENTRY_GROUP_MODE: usize = 0x591B;
const GROUP_MODE_CHANGED: u8 = 2;
const ENTRY_STEP: usize = 0x591C;
const STEP_BUILT: u8 = 0x0E;
const ENTRY_INFO: usize = 0xF8;
const ENTRY_MEMBER_COUNT: usize = 0x4150;
const ENTRY_MEMBERS: usize = 0x4160;
const MEMBER_READY_SLOT: usize = 0x518;
const MEMBER_VISUAL: usize = 0x438;
const VISUAL_SHOW_SLOT: usize = 0x280;
const MEMBER_SPECIALIZER: usize = 0xF700;
const SPECIALIZER_ENTER_SLOT: usize = 0xC0;
const INFO_ENTER_POINT: usize = 0x9F0;
const INFO_COUNTER_A: usize = 0x7C;
const INFO_COUNTER_B: usize = 0x80;
const POINTS_SET: usize = 0x128;
const POINTS_STAGE_ID: usize = 0x17C;
const POINTS_START_COUNT: usize = 0x48;
const POINTS_START: usize = 0x50;
const POINTS_REBIRTH_COUNT: usize = 0x38;
const POINTS_REBIRTH: usize = 0x40;
const SCENE_CARD_DEFAULT: usize = 0xC50;
const SCENE_CARDS: usize = 0xC68;
const CARD_HOLDER_OBJECT: usize = 8;
const CARD_HOLDER_WORD: usize = 0x10;
const CARD_HOLDER_INDEX: usize = 0x14;
const CARD_LIST_LIVE: usize = 0x70;
const CARD_LIST: usize = 0xB0;
const CARD_FACES: usize = 0x1790;
const FACE_STRIDE: usize = 0x200;
const FACE_DESCRIPTION: usize = 0x120;
const FACE_DESCRIPTION_SIZE: usize = 0xE;
const FACE_SHOWN: usize = 0x140;
const FACE_CHANGED: usize = 0x198;
const FACE_ENABLED: usize = 0x1A8;
const ENTRY_BUSY: usize = 0x5935;
const WAIT_FRAMES: u32 = 600;
const CARD_WAIT_FRAMES: u32 = 300;

const ENTRIES: u32 = 8;
const ENTRY_SLOTS: usize = 0x20;
const ENTRY_STRIDE: usize = 0x260;
const ROW_CHARACTER: usize = 0x5B0;
const ROW_KIND: usize = 0x5B8;
const ROW_ENTRY: usize = 0x5BC;
const ROW_COLOR: usize = 0x5C8;
const ROW_FIRST: usize = 0x5BA;
const ROW_SKIN: usize = 0x5C9;
const ROW_SECOND: usize = 0x61B;
const ROW_THIRD: usize = 0x629;
const ROW_CARD_PLAIN: usize = 0x664;
const ENTRY_STATE: usize = 0x41A0;
const ENTRY_BYTES: usize = 0x5918;
const ENTRY_BYTE_COUNT: usize = 0x20;
const INFO_WORDS: [usize; 3] = [0x28, 0x84, 0x8C];
const TRACE_FRAMES: u32 = 240;
const TRACE_EVERY: u32 = 30;
const UI_DATA: usize = 8;
const UI_CHARACTERS: usize = 0x160;
const UI_KINDS: usize = 0x168;
const NANA: i32 = 0x6E;
const UI_HASH_TAG: u64 = 0xC1FF_FF00_0000_0000;
const HASH_MASK: u64 = 0xFF_FFFF_FFFF;
const NO_KIND: i32 = -1;

const BLOCK_ENTRY: usize = 0x04;
const BLOCK_SEARCH: usize = 0x10;
const BLOCK_COUNT: usize = 0x14;
const BLOCK_KINDS: usize = 0x18;
const BLOCK_MEMBERS: usize = 0x28;
const BLOCK_FLAGS: usize = 0x30;
const BLOCK_MAGIC: usize = 0x38;
const BLOCK_MAGIC_VALUE: u64 = 0x4_7F90_00CF;
const BLOCK_NONE: usize = 0x40;
const BLOCK_SKIN: usize = 0x41;
const BLOCK_SKIN_TAIL: usize = 0x44;
const BLOCK_SKIN_TAIL_VALUE: u8 = 0x0E;
const BLOCK_OPTIONS: usize = 0x50;
const BLOCK_OPTIONS_VALUE: u32 = 0x100_0001;
const BLOCK_SIZE: usize = 0x70;

type ExpandFn = unsafe extern "C" fn(i32, *mut i32) -> u32;
type InstallFn = unsafe extern "C" fn(usize, *const u8, usize);
type CharacterFn = unsafe extern "C" fn(usize, i32) -> u64;
type UiKindFn = unsafe extern "C" fn(usize, u64) -> i32;
type ActivateFn = unsafe extern "C" fn(usize, u32);
type SettleFn = unsafe extern "C" fn(usize, u32);
type CountFn = unsafe extern "C" fn(usize) -> i32;
#[allow(improper_ctypes_definitions)]
type PointFn = unsafe extern "C" fn(usize, i32) -> float32x4_t;
type ReadyFn = unsafe extern "C" fn(usize) -> u64;
type ShowFn = unsafe extern "C" fn(usize, u32);
type EnterFn = unsafe extern "C" fn(usize, usize, u32);
type CardCheckFn = unsafe extern "C" fn(*const CardReference) -> u64;
type KindUiFn = unsafe extern "C" fn(i32) -> u64;
type TicketTakeFn = unsafe extern "C" fn(usize) -> u32;
type TicketGiveFn = unsafe extern "C" fn(usize, u32);
type FaceDescribeFn = unsafe extern "C" fn(*mut FaceDescription, u64, u32, u32);

#[repr(C)]
struct CardReference {
    vtable: usize,
    holder: usize,
    word: u32,
    index: u32,
}

#[repr(C)]
struct FaceDescription {
    ui: u64,
    color: u32,
    flags: u32,
}

struct Pending {
    entry: u32,
    waited: u32,
}

enum CardStep {
    Ticket,
    Write,
    Settle,
}

struct Card {
    entry: u32,
    step: CardStep,
    ticket: Option<u16>,
    waited: u32,
}

static PENDING: Mutex<Vec<Pending>> = Mutex::new(Vec::new());

static CARDS: Mutex<Vec<Card>> = Mutex::new(Vec::new());

static TRACED: Mutex<Vec<(u32, u32)>> = Mutex::new(Vec::new());

static ARMED: AtomicBool = AtomicBool::new(false);

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

pub(crate) unsafe fn kind_of_ui(ui_chara: u64) -> Option<i32> {
    let root: usize = read(crate::game::addr(UI_ROOT));
    let data: usize = if root != 0 { read(root + UI_DATA) } else { 0 };
    let kinds: usize = if data != 0 { read(data + UI_KINDS) } else { 0 };
    if kinds == 0 {
        return None;
    }
    let kind = game::<UiKindFn>(UI_KIND)(kinds, UI_HASH_TAG | (ui_chara & HASH_MASK)) as i8 as i32;
    (kind >= 0).then_some(kind)
}

unsafe fn entry_object(entry: u32) -> usize {
    let setup: usize = read(crate::game::addr(MATCH_SETUP));
    let slots: usize = if setup != 0 { read(setup) } else { 0 };
    if slots != 0 {
        read(slots + ENTRY_SLOTS + entry as usize * 8)
    } else {
        0
    }
}

unsafe fn method<F: Copy>(object: usize, slot: usize) -> F {
    let vtable: usize = read(object);
    let function: usize = read(vtable + slot);
    core::mem::transmute_copy(&function)
}

unsafe fn enter_point(entry: u32) -> Option<[f32; 3]> {
    let stage: usize = read(crate::game::addr(STAGE_POINTS));
    let owner: usize = if stage != 0 { read(stage) } else { 0 };
    if owner == 0 || read::<i32>(owner + POINTS_STAGE_ID) == -1 {
        return None;
    }
    let points = owner + POINTS_SET;
    let mut found = None;
    for (count_slot, point_slot) in [(POINTS_START_COUNT, POINTS_START), (POINTS_REBIRTH_COUNT, POINTS_REBIRTH)] {
        let count = method::<CountFn>(points, count_slot)(points);
        if count > 0 {
            let point = method::<PointFn>(points, point_slot)(points, entry as i32 % count);
            found = Some([vgetq_lane_f32(point, 0), vgetq_lane_f32(point, 1), vgetq_lane_f32(point, 2)]);
        }
    }
    found
}

unsafe fn card_reference(entry: u32) -> Option<CardReference> {
    let owner: usize = read(crate::game::addr(HUD_OWNER));
    let holder_of_scene: usize = if owner != 0 { read(owner + CARD_HOLDER_OBJECT) } else { 0 };
    let scene: usize = if holder_of_scene != 0 { read(holder_of_scene) } else { 0 };
    if scene == 0 {
        return None;
    }
    let row = crate::game::addr(ENTRY_TABLE) + entry as usize * ENTRY_STRIDE;
    let card_index: i32 = read(row + ROW_ENTRY);
    let holder: usize = if card_index == -1 {
        scene + SCENE_CARD_DEFAULT
    } else {
        read(scene + SCENE_CARDS + card_index as usize * 8)
    };
    if holder == 0 {
        return None;
    }
    let reference = CardReference {
        vtable: crate::game::addr(CARD_REFERENCE_VTABLE),
        holder: read(holder + CARD_HOLDER_OBJECT),
        word: read(holder + CARD_HOLDER_WORD),
        index: read(holder + CARD_HOLDER_INDEX),
    };
    (reference.holder != 0).then_some(reference)
}

unsafe fn card_face(reference: &CardReference) -> Option<usize> {
    if game::<CardCheckFn>(CARD_CHECK)(reference) & 1 == 0 {
        return None;
    }
    let mode: usize = read(crate::game::addr(MODE_ROOT));
    let data: usize = if mode != 0 { read(mode + MODE_DATA) } else { 0 };
    if data != 0 && read::<u32>(data + MODE_ACTIVE) != 0 && read::<u8>(data + MODE_KIND) == MODE_WITHOUT_CARDS {
        return None;
    }
    if reference.index >= ENTRIES {
        return None;
    }
    let list: usize = read(reference.holder);
    if read::<u8>(list + CARD_LIST_LIVE) == 0 {
        return None;
    }
    let cards: usize = read(list + CARD_LIST);
    let faces: usize = if cards != 0 { read(cards + CARD_FACES) } else { 0 };
    if faces == 0 {
        return None;
    }
    let face = faces + reference.index as usize * FACE_STRIDE;
    (read::<u8>(face + FACE_ENABLED) != 0).then_some(face)
}

unsafe fn describe_face(entry: u32) -> FaceDescription {
    let row = crate::game::addr(ENTRY_TABLE) + entry as usize * ENTRY_STRIDE;
    let ui = game::<KindUiFn>(KIND_UI)(read::<i8>(row + ROW_KIND) as i32);
    let mut description = FaceDescription { ui: UI_HASH_TAG, color: 0, flags: 0 };
    game::<FaceDescribeFn>(FACE_DESCRIBE)(
        &mut description,
        ui,
        read::<u8>(row + ROW_SKIN) as u32,
        (read::<u8>(row + ROW_CARD_PLAIN) == 0) as u32,
    );
    description
}

unsafe fn show_face(entry: u32, settled: bool) -> Option<u64> {
    let face = card_face(&card_reference(entry)?)?;
    let description = describe_face(entry);
    core::ptr::copy_nonoverlapping(
        &description as *const FaceDescription as *const u8,
        (face + FACE_DESCRIPTION) as *mut u8,
        FACE_DESCRIPTION_SIZE,
    );
    write(face + FACE_SHOWN, settled as u8);
    if !settled {
        write(face + FACE_CHANGED, 1u8);
    }
    Some(description.ui)
}

unsafe fn give_ticket(ticket: Option<u16>) {
    let Some(ticket) = ticket else {
        return;
    };
    let holder: usize = read(crate::game::addr(TICKETS));
    let tickets: usize = if holder != 0 { read(holder) } else { 0 };
    if tickets != 0 {
        game::<TicketGiveFn>(TICKET_GIVE)(tickets, ticket as u32);
    }
}

unsafe fn advance_card(card: &mut Card) -> bool {
    let object = entry_object(card.entry);
    if object == 0 {
        give_ticket(card.ticket.take());
        return false;
    }
    match card.step {
        CardStep::Ticket => {
            if read::<u8>(object + ENTRY_BUSY) != 0 {
                return true;
            }
            let holder: usize = read(crate::game::addr(TICKETS));
            if holder != 0 {
                card.ticket = Some(game::<TicketTakeFn>(TICKET_TAKE)(holder) as u16);
            }
            card.step = CardStep::Write;
            true
        }
        CardStep::Write => {
            show_face(card.entry, false);
            card.step = CardStep::Settle;
            true
        }
        CardStep::Settle => {
            let ready = card_reference(card.entry)
                .is_some_and(|reference| game::<CardCheckFn>(CARD_READY)(&reference) & 1 != 0);
            if !ready {
                card.waited += 1;
                if card.waited < CARD_WAIT_FRAMES {
                    return true;
                }
                crate::dbg_log_public(&format!(
                    "[fighterchange] entry {}: the damage card did not take the new face in {CARD_WAIT_FRAMES} frames",
                    card.entry
                ));
            } else {
                match show_face(card.entry, true) {
                    Some(ui) => crate::dbg_log_public(&format!(
                        "[fighterchange] entry {}: the damage card shows {:#x}",
                        card.entry,
                        ui & HASH_MASK
                    )),
                    None => crate::dbg_log_public(&format!("[fighterchange] entry {}: no damage card", card.entry)),
                }
            }
            give_ticket(card.ticket.take());
            false
        }
    }
}

pub(crate) fn hud_gone() {
    if let Ok(mut cards) = CARDS.lock() {
        for card in cards.drain(..) {
            unsafe { give_ticket(card.ticket) };
        }
    }
}

unsafe fn entry_state(entry: u32, object: usize) -> String {
    let info: usize = read(object + ENTRY_INFO);
    let bytes: Vec<String> = (0..ENTRY_BYTE_COUNT).map(|k| format!("{:02x}", read::<u8>(object + ENTRY_BYTES + k))).collect();
    let words: Vec<String> = if info != 0 {
        INFO_WORDS.iter().map(|offset| format!("{:#x}", read::<u32>(info + offset))).collect()
    } else {
        Vec::new()
    };
    let manager: usize = read(crate::game::addr(MATCH_SETUP));
    let cpu = if manager != 0 {
        let information = smash::app::lua_bind::FighterManager::get_fighter_information(
            manager as *mut smash::app::FighterManager,
            smash::app::FighterEntryID(entry as i32),
        );
        if information.is_null() {
            "none".to_string()
        } else {
            smash::app::lua_bind::FighterInformation::is_operation_cpu(information).to_string()
        }
    } else {
        "none".to_string()
    };
    format!(
        "state {:#x} bytes {} info {:#x} [{}] cpu {cpu} ai {} members {}",
        read::<u32>(object + ENTRY_STATE),
        bytes.join(""),
        info,
        words.join(" "),
        read::<u8>(object + ENTRY_AI),
        read::<usize>(object + ENTRY_MEMBER_COUNT)
    )
}

unsafe fn activate(entry: u32, object: usize) {
    let info: usize = read(object + ENTRY_INFO);
    if let Some([x, y, z]) = enter_point(entry) {
        write(info + INFO_ENTER_POINT, x);
        write(info + INFO_ENTER_POINT + 4, y);
        write(info + INFO_ENTER_POINT + 8, z);
        write(info + INFO_ENTER_POINT + 12, 0.0f32);
    }
    write(object + ENTRY_GROUP_MODE, GROUP_MODE_CHANGED);
    game::<ActivateFn>(ACTIVATE)(object, 0);
    let members: usize = read(object + ENTRY_MEMBER_COUNT);
    for index in 0..members {
        let member: usize = read(object + ENTRY_MEMBERS + index * 8);
        if member == 0 || method::<ReadyFn>(member, MEMBER_READY_SLOT)(member) & 1 == 0 {
            continue;
        }
        let visual: usize = read(member + MEMBER_VISUAL);
        if visual != 0 {
            method::<ShowFn>(visual, VISUAL_SHOW_SLOT)(visual, 0);
        }
        let specializer: usize = read(member + MEMBER_SPECIALIZER);
        if specializer != 0 {
            method::<EnterFn>(specializer, SPECIALIZER_ENTER_SLOT)(specializer, member, 1);
        }
    }
    game::<SettleFn>(SETTLE)(object, 1);
    if let Ok(mut cards) = CARDS.lock() {
        for card in cards.iter_mut().filter(|card| card.entry == entry) {
            give_ticket(card.ticket.take());
        }
        cards.retain(|card| card.entry != entry);
        cards.push(Card { entry, step: CardStep::Ticket, ticket: None, waited: 0 });
    }
    write::<u32>(info + INFO_COUNTER_B, 0);
    write::<u32>(info + INFO_COUNTER_A, 0);
    crate::dbg_log_public(&format!(
        "[fighterchange] entry {entry}: the new fighter comes in; {}",
        entry_state(entry, object)
    ));
    if let Ok(mut traced) = TRACED.lock() {
        traced.retain(|(known, _)| *known != entry);
        traced.push((entry, 0));
    }
}

unsafe fn trace(entry: u32, frames: u32) {
    let object = entry_object(entry);
    if object != 0 {
        crate::dbg_log_public(&format!("[fighterchange] entry {entry} +{frames}: {}", entry_state(entry, object)));
    }
}

pub(crate) fn frame() {
    if !ARMED.load(Ordering::Acquire) {
        return;
    }
    if let Ok(mut traced) = TRACED.lock() {
        traced.retain_mut(|(entry, frames)| {
            *frames += 1;
            if *frames % TRACE_EVERY == 0 {
                unsafe { trace(*entry, *frames) };
            }
            *frames < TRACE_FRAMES
        });
    }
    if let Ok(mut cards) = CARDS.lock() {
        cards.retain_mut(|card| unsafe { advance_card(card) });
    }
    let Ok(mut pending) = PENDING.lock() else {
        return;
    };
    if pending.is_empty() {
        return;
    }
    pending.retain_mut(|waiting| unsafe {
        let object = entry_object(waiting.entry);
        if object == 0 {
            return false;
        }
        if read::<u8>(object + ENTRY_STEP) == STEP_BUILT {
            activate(waiting.entry, object);
            return false;
        }
        waiting.waited += 1;
        if waiting.waited >= WAIT_FRAMES {
            crate::dbg_log_public(&format!(
                "[fighterchange] entry {}: the new fighter was not built in {WAIT_FRAMES} frames (step {:#x})",
                waiting.entry,
                read::<u8>(object + ENTRY_STEP)
            ));
            return false;
        }
        true
    });
}

pub(crate) unsafe fn change(entry: u32, kind: i32, color: u8) -> Result<(), String> {
    if !ARMED.load(Ordering::Acquire) {
        return Err("fighter change is off in this engine build".to_string());
    }
    if entry >= ENTRIES {
        return Err(format!("no entry {entry}"));
    }
    if read::<u8>(crate::game::addr(ENTRY_TABLE_READY)) & 1 == 0 {
        return Err("the battle's entry table is not set up".to_string());
    }
    let object = entry_object(entry);
    if object == 0 {
        return Err(format!("entry {entry} is not in this battle"));
    }
    let root: usize = read(crate::game::addr(UI_ROOT));
    let data: usize = if root != 0 { read(root + UI_DATA) } else { 0 };
    if data == 0 {
        return Err("no UI database".to_string());
    }
    let mut block = [0u8; BLOCK_SIZE];
    core::ptr::copy_nonoverlapping((object + ENTRY_SETUP) as *const u8, block.as_mut_ptr(), BLOCK_SIZE);
    let base = block.as_mut_ptr() as usize;
    write(base + BLOCK_ENTRY, entry);
    write(base + BLOCK_SEARCH, u32::MAX);
    for slot in 0..3 {
        write(base + BLOCK_KINDS + slot * 4, NO_KIND);
    }
    let count = game::<ExpandFn>(EXPAND)(kind, (base + BLOCK_KINDS) as *mut i32);
    write(base + BLOCK_COUNT, count);
    crate::dbg_log_public(&format!("[fighterchange] entry {entry} before: {}", entry_state(entry, object)));
    let own_block = read::<u64>(base + BLOCK_MAGIC) == BLOCK_MAGIC_VALUE;
    write(base + BLOCK_MEMBERS, if kind == NANA { 2u32 } else { 1u32 });
    write(base + BLOCK_MEMBERS + 4, 1u32);
    if !own_block {
        write(base + BLOCK_FLAGS + 4, 1u8);
        write(base + BLOCK_OPTIONS, BLOCK_OPTIONS_VALUE);
    }
    write(base + BLOCK_MAGIC, BLOCK_MAGIC_VALUE);
    write(base + BLOCK_NONE, u8::MAX);
    write(base + BLOCK_SKIN, color);
    write(base + BLOCK_SKIN_TAIL, BLOCK_SKIN_TAIL_VALUE);
    let row = crate::game::addr(ENTRY_TABLE) + entry as usize * ENTRY_STRIDE;
    write(row + ROW_KIND, kind as i8);
    write(row + ROW_CHARACTER, game::<CharacterFn>(CHARACTER_INFO)(data + UI_CHARACTERS, kind));
    write(row + ROW_COLOR, color);
    write(row + ROW_SKIN, color);
    write(row + ROW_FIRST, read::<u8>(base + BLOCK_FLAGS + 4));
    write(row + ROW_SECOND, read::<u8>(base + BLOCK_FLAGS + 5));
    write(row + ROW_THIRD, read::<u8>(base + BLOCK_FLAGS + 7));
    write(row + ROW_ENTRY, entry);
    game::<InstallFn>(INSTALL)(object, block.as_ptr(), crate::game::addr(INSTALL_FLAGS));
    if let Ok(mut pending) = PENDING.lock() {
        pending.retain(|waiting| waiting.entry != entry);
        pending.push(Pending { entry, waited: 0 });
    }
    crate::dbg_log_public(&format!("[fighterchange] entry {entry} -> kind {kind} colour {color}"));
    Ok(())
}

pub(crate) fn install() {
    let sites = [
        (EXPAND, EXPAND_WORD),
        (INSTALL, INSTALL_WORD),
        (CHARACTER_INFO, CHARACTER_INFO_WORD),
        (UI_KIND, UI_KIND_WORD),
        (SETTLE, SETTLE_WORD),
        (CARD_CHECK, CARD_CHECK_WORD),
        (TICKET_TAKE, TICKET_TAKE_WORD),
        (TICKET_GIVE, TICKET_GIVE_WORD),
        (FACE_DESCRIBE, FACE_DESCRIBE_WORD),
        (CARD_READY, CARD_READY_WORD),
    ];
    for (offset, expected) in sites {
        let word = unsafe { crate::game::text_word(offset) };
        if word != expected {
            crate::dbg_log_public(&format!(
                "[fighterchange] code at {offset:#x} not recognised ({word:#010x}); fighter change stays off"
            ));
            return;
        }
    }
    ARMED.store(true, Ordering::Release);
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_fighter_change_v1(entry: u32, ui_chara: u64, color: u32) -> i32 {
    let Some(kind) = kind_of_ui(ui_chara) else {
        crate::dbg_log_public(&format!("[fighterchange] entry {entry}: no fighter for ui_chara {ui_chara:#x}"));
        return ERROR_NULL;
    };
    let color = match u8::try_from(color) {
        Ok(color) => color,
        Err(_) => return ERROR_UNSUPPORTED,
    };
    match change(entry, kind, color) {
        Ok(()) => 0,
        Err(message) => {
            crate::dbg_log_public(&format!("[fighterchange] entry {entry}: {message}"));
            ERROR_BACKEND_UNAVAILABLE
        }
    }
}
