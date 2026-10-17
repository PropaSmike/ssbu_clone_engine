use clone_engine_api::{
    CloneHudV1, API_VERSION_V1, ERROR_BACKEND_UNAVAILABLE, ERROR_CUSTOM_KIND, ERROR_NULL,
    ERROR_STRUCT_SIZE, ERROR_UNSUPPORTED, ERROR_VERSION, HUD_FRAME, HUD_GAUGE, HUD_MENU_CLOSE,
    HUD_MENU_CURSOR, HUD_MENU_DECIDE, HUD_MENU_OPEN, HUD_PLAY, HUD_SELECT, HUD_SHOW, HUD_SLOT,
    HUD_STATE, HUD_TEXT, RESULT_OK,
};
use clone_engine_core::hud::{donor_name, plan, HudAction, MenuCommand, Message, MENU_ROWS, NO_SELECTION};
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use smash::app::lua_bind::WorkModule;
use smash::app::BattleObjectModuleAccessor;
use std::sync::RwLock;

const PANEL_KIND_PICK: usize = 0x1426FE0;
const PANEL_KIND_PICK_WORD: u32 = 0xF940_0674;
const PANEL_KIND_CALL: usize = 0x1426FDC;
const PANEL_KIND_CALL_WORD: u32 = 0x9478_EC55;
const PANEL_UI_REGISTER: usize = 20;
const PANEL_KIND_REGISTER: usize = 0;
const UI_HASH_MASK: u64 = 0xff_ffff_ffff;

const FIGHTER_MANAGER: usize = 0x52B84F8;
const MANAGER_ENTRIES: usize = 0x20;
const ENTRY_HUD_INDEX: usize = 0x41E4;
const ENTRY_HUD_SHOWN: usize = 0x41E8;
const BUS_ID_COUNT: usize = 0;
const BUS_LISTS: usize = 0x8;
const BUS_DEPTH: usize = 0x18;
const BUS_PENDING: usize = 0x1C;
const BUS_LIST_STRIDE: usize = 0x18;
const BUS_LIST_HEAD: usize = 0x8;
const BUS_FLUSH: usize = 0x37AE1C0;
const LISTENER_FUNCTOR: usize = 0x10;
const LISTENER_CALL: usize = 0x18;
const MAX_LISTENERS: usize = 64;
const ENTRIES: usize = 8;

const MESSAGE_VTABLES: &[(u32, usize)] = &[
    (0x12, 0x4F81CC8),
    (0x42, 0x4F829E8),
    (0x46, 0x4F82AD8),
    (0x49, 0x4F82BC8),
    (0x4B, 0x4F82C68),
    (0x4F, 0x4F82E98),
    (0x58, 0x4F83028),
    (0x5D, 0x4F831B8),
    (0x5F, 0x4F83258),
    (0x60, 0x4F832A8),
    (0x61, 0x4F832F8),
    (0x62, 0x4F83348),
    (0x63, 0x4F83398),
    (0x64, 0x4F833E8),
    (0x65, 0x4F83438),
    (0x66, 0x4F83488),
    (0x67, 0x4F834D8),
    (0x6A, 0x4F83578),
    (0x6C, 0x4F83618),
    (0x76, 0x4F83938),
    (0x7B, 0x4F83A28),
];

const PANEL_REPORTS: usize = 8;
const POST_REPORTS: usize = 12;
const REFUSAL_REPORTS: usize = 6;

#[repr(C)]
struct HudMessage {
    vtable: usize,
    id: u32,
    hud: u32,
    payload: [u8; 16],
}

#[derive(Clone, Copy)]
struct Graft {
    kind: i32,
    ui_hash: u64,
    donor: i32,
}

static GRAFTS: RwLock<Vec<Graft>> = RwLock::new(Vec::new());
static HOOK_ARMED: AtomicBool = AtomicBool::new(false);
static SELECTED: [AtomicI32; ENTRIES] = [const { AtomicI32::new(NO_SELECTION) }; ENTRIES];
static PANEL_SWAPS: AtomicUsize = AtomicUsize::new(0);
static POSTS: AtomicUsize = AtomicUsize::new(0);
static REFUSALS: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn register(kind: i32, ui_chara: &str, donor: &str) -> Result<(), String> {
    let donor_kind = clone_engine_core::hud::check(donor)?;
    let ui_hash = clone_engine_core::hash40(ui_chara) & UI_HASH_MASK;
    let mut grafts = GRAFTS.write().map_err(|_| "the HUD table is poisoned".to_string())?;
    if grafts.iter().any(|graft| graft.kind == kind) {
        return Err(format!("kind {kind} already borrows a HUD"));
    }
    grafts.push(Graft {
        kind,
        ui_hash,
        donor: donor_kind,
    });
    Ok(())
}

fn graft_where(test: impl Fn(&Graft) -> bool) -> Option<Graft> {
    GRAFTS.read().ok()?.iter().copied().find(|graft| test(graft))
}

#[skyline::hook(offset = crate::game::rel(PANEL_KIND_PICK), inline)]
unsafe fn panel_kind_pick(ctx: &mut skyline::hooks::InlineCtx) {
    let ui_hash = ctx.registers[PANEL_UI_REGISTER].x() & UI_HASH_MASK;
    let Some(graft) = graft_where(|graft| graft.ui_hash == ui_hash) else {
        return;
    };
    let answered = ctx.registers[PANEL_KIND_REGISTER].x() as i32;
    ctx.registers[PANEL_KIND_REGISTER].set_x(graft.donor as u64);
    for selected in &SELECTED {
        selected.store(NO_SELECTION, Ordering::Relaxed);
    }
    if PANEL_SWAPS.fetch_add(1, Ordering::Relaxed) < PANEL_REPORTS {
        crate::dbg_log_public(&format!(
            "[hud] kind {} gets the {} HUD panel (the HUD read kind {answered})",
            graft.kind,
            donor_name(graft.donor).unwrap_or("?")
        ));
    }
}

pub(crate) fn install() {
    let call = unsafe { crate::game::text_word(PANEL_KIND_CALL) };
    let pick = unsafe { crate::game::text_word(PANEL_KIND_PICK) };
    if call != PANEL_KIND_CALL_WORD || pick != PANEL_KIND_PICK_WORD {
        crate::dbg_log_public(&format!(
            "[hud] HUD panel switch not recognised ({call:#010x} {pick:#010x}); borrowed HUDs stay off"
        ));
        return;
    }
    skyline::install_hooks!(panel_kind_pick);
    HOOK_ARMED.store(true, Ordering::Release);
}

unsafe fn read<T: Copy>(address: usize) -> T {
    core::ptr::read_volatile(address as *const T)
}

unsafe fn post(manager: usize, hud: u32, message: &Message) -> bool {
    let Some(&(_, vtable)) = MESSAGE_VTABLES.iter().find(|(id, _)| *id == message.id) else {
        return false;
    };
    if read::<u32>(manager + BUS_ID_COUNT) <= message.id {
        return false;
    }
    let posted = HudMessage {
        vtable: crate::game::addr(vtable),
        id: message.id,
        hud,
        payload: message.payload,
    };
    let depth = (manager + BUS_DEPTH) as *mut u32;
    core::ptr::write_volatile(depth, core::ptr::read_volatile(depth) + 1);
    let head = read::<usize>(manager + BUS_LISTS) + message.id as usize * BUS_LIST_STRIDE + BUS_LIST_HEAD;
    let mut node: usize = read(head);
    let mut walked = 0;
    while node != head && node != 0 && walked < MAX_LISTENERS {
        let functor: usize = read(node + LISTENER_FUNCTOR);
        if functor != 0 {
            let call: extern "C" fn(usize, *const HudMessage) =
                core::mem::transmute(read::<usize>(read::<usize>(functor) + LISTENER_CALL));
            call(functor, &posted);
        }
        node = read(node);
        walked += 1;
    }
    let left = core::ptr::read_volatile(depth) - 1;
    core::ptr::write_volatile(depth, left);
    if left == 0 && read::<u32>(manager + BUS_PENDING) != 0 {
        let flush: extern "C" fn(usize) = core::mem::transmute(crate::game::addr(BUS_FLUSH));
        flush(manager);
    }
    true
}

unsafe fn entry_hud(entry: usize) -> Result<Option<(usize, u32)>, i32> {
    let holder: usize = read(crate::game::addr(FIGHTER_MANAGER));
    if holder == 0 {
        return Err(ERROR_BACKEND_UNAVAILABLE);
    }
    let manager: usize = read(holder);
    if manager == 0 {
        return Err(ERROR_BACKEND_UNAVAILABLE);
    }
    let information: usize = read(manager + MANAGER_ENTRIES + entry * 8);
    if information == 0 {
        return Err(ERROR_BACKEND_UNAVAILABLE);
    }
    if read::<u8>(information + ENTRY_HUD_SHOWN) == 0 {
        return Ok(None);
    }
    Ok(Some((manager, read(information + ENTRY_HUD_INDEX))))
}

pub(crate) unsafe fn entry_hud_index(entry: usize) -> Option<u32> {
    entry_hud(entry).ok().flatten().map(|(_, hud)| hud)
}

unsafe fn action_from(request: &CloneHudV1) -> Option<HudAction<'_>> {
    Some(match request.action {
        HUD_PLAY => HudAction::Play(request.part()?),
        HUD_FRAME => HudAction::Frame(request.part()?, request.value),
        HUD_SHOW => HudAction::Show(request.part()?, request.index != 0),
        HUD_TEXT => HudAction::Text(request.part()?, request.text()?),
        HUD_GAUGE => HudAction::Gauge(request.value),
        HUD_STATE => HudAction::State(request.index != 0),
        HUD_SELECT => HudAction::Select(request.index),
        HUD_SLOT => HudAction::Slot(request.index, request.value),
        HUD_MENU_OPEN => {
            let mut commands = [MenuCommand::new(0, 0.0); MENU_ROWS];
            for (command, given) in commands.iter_mut().zip(request.commands.iter()) {
                *command = MenuCommand {
                    command: given.command,
                    cost: given.cost,
                    usable: given.usable != 0,
                };
            }
            HudAction::MenuOpen(commands)
        }
        HUD_MENU_CURSOR => HudAction::MenuCursor(request.index),
        HUD_MENU_DECIDE => HudAction::MenuDecide(request.index),
        HUD_MENU_CLOSE => HudAction::MenuClose,
        _ => return None,
    })
}

fn refuse(kind: i32, message: &str) -> i32 {
    if REFUSALS.fetch_add(1, Ordering::Relaxed) < REFUSAL_REPORTS {
        crate::dbg_log_public(&format!("[hud] kind {kind}: REFUSED: {message}"));
    }
    ERROR_UNSUPPORTED
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_hud_v1(module_accessor: u64, request: *const CloneHudV1) -> i32 {
    let Some(request) = request.as_ref() else {
        return ERROR_NULL;
    };
    if module_accessor == 0 {
        return ERROR_NULL;
    }
    if request.api_version != API_VERSION_V1 {
        return ERROR_VERSION;
    }
    if (request.struct_size as usize) < core::mem::size_of::<CloneHudV1>() {
        return ERROR_STRUCT_SIZE;
    }
    let accessor = module_accessor as *mut BattleObjectModuleAccessor;
    if smash::app::utility::get_category(&mut *accessor)
        != *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_FIGHTER
    {
        return ERROR_UNSUPPORTED;
    }
    let entry = WorkModule::get_int(
        accessor,
        *smash::lib::lua_const::FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
    );
    if !(0..ENTRIES as i32).contains(&entry) {
        return ERROR_UNSUPPORTED;
    }
    let entry = entry as usize;
    let Some(kind) = crate::css_registration::entry_custom_kind(entry as u8) else {
        return ERROR_CUSTOM_KIND;
    };
    let Some(action) = action_from(request) else {
        return refuse(kind, &format!("unknown HUD action {}", request.action));
    };
    if action.part().is_some() {
        return match crate::hud_layouts::act(kind, entry, &action) {
            Ok(_) => RESULT_OK,
            Err(message) => refuse(kind, &message),
        };
    }
    if !HOOK_ARMED.load(Ordering::Acquire) {
        return ERROR_BACKEND_UNAVAILABLE;
    }
    let Some(graft) = graft_where(|graft| graft.kind == kind) else {
        return refuse(kind, "the fighter borrows no HUD");
    };
    let planned = match plan(graft.donor, &action, SELECTED[entry].load(Ordering::Relaxed)) {
        Ok(planned) => planned,
        Err(message) => return refuse(kind, &message),
    };
    SELECTED[entry].store(planned.selected, Ordering::Relaxed);
    let hud = match entry_hud(entry) {
        Ok(Some((manager, hud))) => {
            for message in &planned.messages {
                post(manager, hud, message);
            }
            Some(hud)
        }
        Ok(None) => None,
        Err(code) => return code,
    };
    if POSTS.fetch_add(1, Ordering::Relaxed) < POST_REPORTS {
        crate::dbg_log_public(&format!(
            "[hud] kind {kind} entry {entry}: {} {:?} as message(s) {} ({})",
            donor_name(graft.donor).unwrap_or("?"),
            action,
            planned
                .messages
                .iter()
                .map(|message| format!("{:#x}", message.id))
                .collect::<Vec<_>>()
                .join(" "),
            hud.map_or("this entry has no HUD".to_string(), |hud| format!("HUD {hud}"))
        ));
    }
    RESULT_OK
}
