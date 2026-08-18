use std::collections::HashMap;
use std::sync::RwLock;

use smash::lib::L2CValue;
use smash::lua2cpp::L2CFighterBase;

macro_rules! agent_log {
    ($($arg:tt)*) => {{
        let text = format!($($arg)*);
        #[allow(unused_unsafe)]
        unsafe { crate::dbg_out(&text) };
        skyline::println!("{}", text);
    }};
}

const SET_STATUS_SCRIPTS_SLOT: usize = 9;

const GLOBAL_TABLE_STATUS_TOTAL: usize = 0xC;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(i32)]
pub enum StatusLine {
    Pre = 0,
    Main = 1,
    End = 2,
    Init = 3,
    Exec = 4,
    ExecStop = 5,
    Post = 6,
    Exit = 7,
    MapCorrection = 8,
    FixCamera = 9,
    FixPosSlow = 10,
    CheckDamage = 11,
    CheckAttack = 12,
    OnChangeLr = 13,
    LeaveStop = 14,
    NotifyEventGimmick = 15,
    CalcParam = 16,
}

impl StatusLine {
    pub fn from_raw(value: i32) -> Option<Self> {
        use StatusLine::*;
        Some(match value {
            0 => Pre,
            1 => Main,
            2 => End,
            3 => Init,
            4 => Exec,
            5 => ExecStop,
            6 => Post,
            7 => Exit,
            8 => MapCorrection,
            9 => FixCamera,
            10 => FixPosSlow,
            11 => CheckDamage,
            12 => CheckAttack,
            13 => OnChangeLr,
            14 => LeaveStop,
            15 => NotifyEventGimmick,
            16 => CalcParam,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy)]
struct StatusScript {
    line: StatusLine,
    status_kind: i32,
    function: *const (),
}

unsafe impl Send for StatusScript {}
unsafe impl Sync for StatusScript {}

fn scripts() -> &'static RwLock<HashMap<i32, Vec<StatusScript>>> {
    static SCRIPTS: std::sync::OnceLock<RwLock<HashMap<i32, Vec<StatusScript>>>> =
        std::sync::OnceLock::new();
    SCRIPTS.get_or_init(|| RwLock::new(HashMap::new()))
}

struct AgentRecord {
    weapon_kind: i32,
}

fn agents() -> &'static RwLock<HashMap<usize, AgentRecord>> {
    static AGENTS: std::sync::OnceLock<RwLock<HashMap<usize, AgentRecord>>> =
        std::sync::OnceLock::new();
    AGENTS.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn register_status(weapon_kind: i32, line: i32, status_kind: i32, function: *const ()) -> bool {
    let Some(line) = StatusLine::from_raw(line) else {
        return false;
    };
    if function.is_null() || !crate::custom_articles::is_custom_weapon_kind(weapon_kind) {
        return false;
    }

    let Ok(mut registry) = scripts().write() else {
        return false;
    };
    let list = registry.entry(weapon_kind).or_default();

    if let Some(existing) = list
        .iter_mut()
        .find(|script| script.line == line && script.status_kind == status_kind)
    {
        existing.function = function;
    } else {
        list.push(StatusScript {
            line,
            status_kind,
            function,
        });
    }
    true
}

fn registered_for(weapon_kind: i32) -> Vec<StatusScript> {
    scripts()
        .read()
        .ok()
        .and_then(|registry| registry.get(&weapon_kind).cloned())
        .unwrap_or_default()
}

pub fn self_addresses() -> (usize, usize) {
    (
        attach as *const () as usize,
        set_status_scripts as *const () as usize,
    )
}

pub fn wants_agent(weapon_kind: i32) -> bool {
    crate::custom_articles::custom_weapon_source_kind(weapon_kind).is_some()
        && !registered_for(weapon_kind).is_empty()
}

pub unsafe fn attach(weapon_kind: i32, agent: *mut u8) -> bool {
    if agent.is_null() {
        return false;
    }

    let vtable = core::ptr::read_volatile(agent as *const *mut u64);
    if vtable.is_null() {
        return false;
    }

    let slot = vtable.add(SET_STATUS_SCRIPTS_SLOT);
    let current = core::ptr::read_volatile(slot);
    let ours = set_status_scripts as *const () as usize as u64;

    if current != ours {
        let Ok(mut originals) = vtable_originals().write() else {
            return false;
        };
        originals.insert(vtable as usize, core::mem::transmute(current as usize));
        core::ptr::write_volatile(slot, ours);
    }

    if let Ok(mut live) = agents().write() {
        live.insert(agent as usize, AgentRecord { weapon_kind });
    } else {
        return false;
    }

    agent_log!(
        "[articleagent] attached to agent {:#x} vtable {:#x} slot9 {:#x} -> ours, weapon kind {weapon_kind}",
        agent as usize,
        vtable as usize,
        current
    );
    true
}

fn vtable_originals() -> &'static RwLock<HashMap<usize, extern "C" fn(*mut u8)>> {
    static ORIGINALS: std::sync::OnceLock<RwLock<HashMap<usize, extern "C" fn(*mut u8)>>> =
        std::sync::OnceLock::new();
    ORIGINALS.get_or_init(|| RwLock::new(HashMap::new()))
}

extern "C" fn set_status_scripts(agent: *mut u8) {
    if agent.is_null() {
        return;
    }

    let vtable = unsafe { core::ptr::read_volatile(agent as *const usize) };
    let original = vtable_originals()
        .read()
        .ok()
        .and_then(|originals| originals.get(&vtable).copied());
    let Some(original) = original else {
        agent_log!(
            "[articleagent] set_status_scripts agent {:#x} vtable {:#x} has no recorded original; nothing to call",
            agent as usize,
            vtable
        );
        return;
    };

    let weapon_kind = agents()
        .read()
        .ok()
        .and_then(|live| live.get(&(agent as usize)).map(|record| record.weapon_kind));
    let Some(weapon_kind) = weapon_kind else {
        original(agent);
        return;
    };

    agent_log!(
        "[articleagent] set_status_scripts agent {:#x} kind {weapon_kind} original {:#x}",
        agent as usize,
        original as usize
    );

    original(agent);

    let scripts = registered_for(weapon_kind);
    agent_log!(
        "[articleagent] baseline installed; {} scripts to add",
        scripts.len()
    );
    if scripts.is_empty() {
        return;
    }

    let base = unsafe { &mut *(agent as *mut L2CFighterBase) };
    let mut highest = 0i32;
    for script in scripts.iter() {
        highest = highest.max(script.status_kind + 1);
        let id = L2CValue::new_int(script.status_kind as u64);
        let condition = L2CValue::new_int(script.line as i32 as u64);
        agent_log!(
            "[articleagent] sv_set_status_func status {} line {:?} fn {:#x}",
            script.status_kind,
            script.line,
            script.function as usize
        );
        unsafe {
            base.sv_set_status_func(id, condition, &mut *(script.function as *mut libc::c_void));
        }
    }
    agent_log!("[articleagent] scripts installed, highest {highest}");

    unsafe {
        let global_table = &base.global_table as *const _ as *const u8;
        let global_type = core::ptr::read_volatile(global_table as *const u32);
        let table = if global_type == L2C_TYPE_TABLE {
            core::ptr::read_volatile(global_table.add(8) as *const usize) as *mut u8
        } else {
            core::ptr::null_mut()
        };
        agent_log!(
            "[articleagent] global_table {:#x} type {global_type}",
            table as usize
        );
        if !table.is_null() {
            let slot = table_entry(table, GLOBAL_TABLE_STATUS_TOTAL);
            if !slot.is_null() && core::ptr::read_volatile(slot as *const u32) == L2C_TYPE_INT {
                let current = core::ptr::read_volatile(slot.add(8) as *const u64) as i32;
                if highest > current {
                    core::ptr::write_volatile(slot.add(8) as *mut u64, highest as u64);
                }
                agent_log!(
                    "[articleagent] status total {current} -> {}",
                    current.max(highest)
                );
            }
        }
    }
    agent_log!("[articleagent] set_status_scripts done");
}

const L2C_TYPE_INT: u32 = 2;

const L2C_TYPE_TABLE: u32 = 5;

const L2C_VALUE_SIZE: usize = 0x10;

unsafe fn table_entry(table: *mut u8, index: usize) -> *mut u8 {
    let begin = core::ptr::read_volatile(table.add(0x8) as *const usize);
    let end = core::ptr::read_volatile(table.add(0x10) as *const usize);
    if begin == 0 || end <= begin {
        return core::ptr::null_mut();
    }
    let count = (end - begin) / L2C_VALUE_SIZE;
    if index >= count {
        return core::ptr::null_mut();
    }
    (begin + index * L2C_VALUE_SIZE) as *mut u8
}
