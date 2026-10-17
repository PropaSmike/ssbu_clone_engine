use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock, RwLock};

use clone_engine_core::final_smash::{self, Donor, FINAL};
use clone_engine_core::motion_list::{self, MotionList};

const STATUS_CHANGE: usize = 0x4D_EBE0;
const STATUS_CHANGE_WORD: u32 = 0xD102_03FF;
const MODULE_REQUEST: usize = 0x17E_4940;
const MODULE_REQUEST_WORD: u32 = 0xD101_C3FF;
const SET_FUNCTION_HASH: usize = 0x372_DD10;
const SET_FUNCTION_HASH_WORD: u32 = 0xD101_83FF;

const LINES: usize = 22;
const RECORD_BYTES: usize = LINES * 8;
const AGENT_BOMA: usize = 0x40;
const AGENT_STATUS_BEGIN: usize = 0x48;
const AGENT_STATUS_END: usize = 0x50;
const AGENT_GLOBAL_TABLE: usize = 0xC8;
const AGENT_SCRIPT_FIRST: usize = 0x20;
const NODE_KEY: usize = 0x10;
const NODE_VALUE: usize = 0x18;
const STATUS_MODULE_BOMA: usize = 8;
const OBJECT_KIND: usize = 0xC;
const BOMA_WORK_MODULE: usize = 0x50;
const WORK_PARAM_ACCESSOR: usize = 0x38;
const ACCESSOR_BLOCK: usize = 8;
const BLOCK_PAYLOAD: usize = 0x1B08;
const NRO_MAGIC: u32 = 0x304F_524E;
const NRO_MAGIC_AT: usize = 0x10;
const NRO_SIZE_AT: usize = 0x18;
const PAGE: usize = 0x1000;
const MAX_MODULE_PAGES: usize = 0x2000;
const SET_STATUS_SCRIPTS_SLOT: usize = 9;
const DELETING_DESTRUCTOR_SLOT: usize = 1;
const GLOBAL_STATUS_TOTAL: usize = 0xC;
const L2C_TYPE_INT: u32 = 2;
const L2C_TYPE_TABLE: u32 = 5;
const L2C_VALUE_BYTES: usize = 0x10;
const HASH_MASK: u64 = 0xFF_FFFF_FFFF;
const LOWEST_POINTER: usize = 0x1_0000;
const MAX_NODES: usize = 0x4000;
const LOG_BUDGET: u32 = 64;
const NRO_MOD0_AT: usize = 4;
const MOD0_MAGIC: u32 = 0x3044_4F4D;
const MOD0_DYNAMIC_AT: usize = 4;
const DYNAMIC_ENTRY_BYTES: usize = 16;
const MAX_DYNAMIC_ENTRIES: usize = 256;
const DT_NULL: i64 = 0;
const DT_PLTRELSZ: i64 = 2;
const DT_STRTAB: i64 = 5;
const DT_SYMTAB: i64 = 6;
const DT_JMPREL: i64 = 23;
const RELA_BYTES: usize = 24;
const SYMBOL_BYTES: usize = 24;
const R_AARCH64_JUMP_SLOT: u32 = 1026;
const PARAM_INT_IMPORT: &[u8] =
    b"_ZN3app8lua_bind30WorkModule__get_param_int_implEPNS_26BattleObjectModuleAccessorEmm\0";
const PARAM_FLOAT_IMPORT: &[u8] =
    b"_ZN3app8lua_bind32WorkModule__get_param_float_implEPNS_26BattleObjectModuleAccessorEmm\0";
const PARAM_INT64_IMPORT: &[u8] =
    b"_ZN3app8lua_bind32WorkModule__get_param_int64_implEPNS_26BattleObjectModuleAccessorEmm\0";

const ACMD_CATEGORIES: [&str; 4] = ["game", "effect", "expression", "sound"];

type Creator = unsafe extern "C" fn(u64, u64, u64, u64) -> usize;
type AgentCall = unsafe extern "C" fn(usize);
type StatusFn = unsafe extern "C" fn(*mut u8) -> smash::lib::L2CValue;
type ModuleRequest = unsafe extern "C" fn(usize, i32);
type SetFunctionHash = unsafe extern "C" fn(usize, usize, u64);

static ARMED: AtomicBool = AtomicBool::new(false);
static TRACKED: AtomicUsize = AtomicUsize::new(0);
static OPEN: AtomicUsize = AtomicUsize::new(0);
static ANSWERED: AtomicUsize = AtomicUsize::new(0);
static LOG_LINES: AtomicU32 = AtomicU32::new(0);

struct Graft {
    clone_kind: i32,
    donor: Donor,
}

struct Captured {
    donor_kind: i32,
    creator: usize,
    code: (usize, usize),
    records: Vec<(i32, [usize; LINES])>,
}

struct Fighter {
    boma: usize,
    object: usize,
    agent: usize,
    clone_kind: i32,
    donor: Donor,
    installed: bool,
    open: bool,
    held_kind: i32,
    saved: Vec<(i32, [usize; LINES])>,
    donor_records: Vec<(i32, [usize; LINES])>,
}

fn grafts() -> &'static RwLock<Vec<Graft>> {
    static GRAFTS: OnceLock<RwLock<Vec<Graft>>> = OnceLock::new();
    GRAFTS.get_or_init(|| RwLock::new(Vec::new()))
}

fn captured() -> &'static Mutex<Vec<Captured>> {
    static CAPTURED: OnceLock<Mutex<Vec<Captured>>> = OnceLock::new();
    CAPTURED.get_or_init(|| Mutex::new(Vec::new()))
}

fn fighters() -> &'static Mutex<Vec<Fighter>> {
    static FIGHTERS: OnceLock<Mutex<Vec<Fighter>>> = OnceLock::new();
    FIGHTERS.get_or_init(|| Mutex::new(Vec::new()))
}

fn requested() -> &'static Mutex<Vec<(usize, i32)>> {
    static REQUESTED: OnceLock<Mutex<Vec<(usize, i32)>>> = OnceLock::new();
    REQUESTED.get_or_init(|| Mutex::new(Vec::new()))
}

fn donor_lists() -> &'static Mutex<Vec<(i32, Option<MotionList>)>> {
    static LISTS: OnceLock<Mutex<Vec<(i32, Option<MotionList>)>>> = OnceLock::new();
    LISTS.get_or_init(|| Mutex::new(Vec::new()))
}

fn log(message: String) {
    if LOG_LINES.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        crate::dbg_log_public(&message);
    }
}

fn readable(address: usize) -> bool {
    address >= LOWEST_POINTER && address % 8 == 0
}

pub(crate) fn register(kind: i32, donor_name: &str) -> Result<Donor, String> {
    let donor = final_smash::check(donor_name)?;
    let Some(definition) = crate::clone_definition(kind) else {
        return Err(format!("kind {kind} is not a registered clone"));
    };
    if definition.base_kind == donor.kind {
        return Err(format!(
            "{donor_name} is the base fighter already; its Final Smash is the clone's own"
        ));
    }
    let mut held = grafts().write().map_err(|_| "registry poisoned".to_string())?;
    held.retain(|graft| graft.clone_kind != kind);
    held.push(Graft { clone_kind: kind, donor });
    Ok(donor)
}

pub(crate) fn donor_of(kind: i32) -> Option<Donor> {
    grafts()
        .read()
        .ok()?
        .iter()
        .find(|graft| graft.clone_kind == kind)
        .map(|graft| graft.donor)
}

pub(crate) fn donor_for_motion_directory(directory: &str) -> Option<Donor> {
    let owner = directory.strip_prefix("fighter/")?.split('/').next()?;
    let definition = crate::clone_definition_from_name(owner)?;
    donor_of(definition.kind)
}

pub(crate) fn motion_owners() -> Vec<String> {
    let Ok(held) = grafts().read() else {
        return Vec::new();
    };
    held.iter()
        .filter_map(|graft| crate::clone_definition(graft.clone_kind))
        .map(|definition| definition.resource_name.to_string())
        .collect()
}

fn read_arc_file(path: &str) -> Option<Vec<u8>> {
    let hash = crate::hash40::hash40(path);
    let size = crate::global_motions::arc_file_size(path)?;
    let mut bytes = vec![0u8; size];
    let read = arcropolis_api::load_original_file(hash, bytes.as_mut_slice())?;
    bytes.truncate(read);
    Some(bytes)
}

pub(crate) fn with_donor_list<T>(donor: &Donor, use_list: impl FnOnce(&MotionList) -> T) -> Option<T> {
    let mut lists = donor_lists().lock().ok()?;
    if !lists.iter().any(|(kind, _)| *kind == donor.kind) {
        let path = format!("fighter/{}/motion/body/c00/motion_list.bin", donor.name);
        let parsed = read_arc_file(&path).and_then(|bytes| motion_list::parse(&bytes).ok());
        if parsed.is_none() {
            log(format!("[finalsmash] {path} did not read; {}'s Final Smash motions are not added", donor.name));
        }
        lists.push((donor.kind, parsed));
    }
    let (_, list) = lists.iter().find(|(kind, _)| *kind == donor.kind)?;
    list.as_ref().map(use_list)
}

pub(crate) fn graft_motion_list(
    directory: &str,
    list: &mut MotionList,
    has_animation: impl Fn(&str) -> bool,
) -> Option<String> {
    let donor = donor_for_motion_directory(directory)?;
    let outcome = with_donor_list(&donor, |donor_list| final_smash::graft_motions(list, donor_list, &donor))?;
    let added = outcome
        .iter()
        .filter(|(_, grafted)| *grafted != final_smash::Grafted::AbsentFromDonor)
        .count();
    let missing: Vec<&str> = donor
        .animations
        .iter()
        .copied()
        .filter(|file| !has_animation(file))
        .collect();
    Some(if missing.is_empty() {
        format!("{} Final Smash motion(s) of {}", added, donor.name)
    } else {
        format!(
            "{} Final Smash motion(s) of {}, MISSING {} in the folder: {}",
            added,
            donor.name,
            missing.len(),
            missing.join(", ")
        )
    })
}

pub(crate) unsafe fn request_donor(context: usize, kind: i32) {
    if !ARMED.load(Ordering::Acquire) || context < LOWEST_POINTER {
        return;
    }
    let Some(donor) = donor_of(kind) else {
        return;
    };
    if let Ok(mut held) = requested().lock() {
        if held.contains(&(context, donor.kind)) {
            return;
        }
        held.retain(|(known, _)| *known == context);
        held.push((context, donor.kind));
    }
    let request: ModuleRequest = core::mem::transmute(crate::game::addr(MODULE_REQUEST));
    request(context, donor.kind);
    log(format!(
        "[finalsmash] clone kind {kind} loads lua2cpp_{} for its Final Smash",
        donor.name
    ));
}

fn mangled(category: &str, donor: &str) -> Vec<u8> {
    let body = format!("create_agent_fighter_{category}_{donor}");
    format!(
        "_ZN7lua2cpp{}{body}EN3phx6Hash40EPN3app12BattleObjectEPNS2_26BattleObjectModuleAccessorEP9lua_State\0",
        body.len()
    )
    .into_bytes()
}

unsafe fn creator(category: &str, donor: &Donor) -> Option<usize> {
    crate::css_registration::lookup_symbol(&mangled(category, donor.name))
}

unsafe fn scratch_agent(creator: usize, donor: &Donor, object: u64, boma: u64, lua_state: u64) -> Option<usize> {
    let create: Creator = core::mem::transmute(creator);
    let agent = create(crate::hash40::hash40(donor.name), object, boma, lua_state);
    readable(agent).then_some(agent)
}

unsafe fn destroy(agent: usize) {
    let vtable = core::ptr::read_volatile(agent as *const usize);
    if !readable(vtable) {
        return;
    }
    let deleter: AgentCall =
        core::mem::transmute(core::ptr::read_volatile((vtable + DELETING_DESTRUCTOR_SLOT * 8) as *const usize));
    deleter(agent);
}

unsafe fn status_vector(agent: usize) -> (usize, usize) {
    let begin = core::ptr::read_volatile((agent + AGENT_STATUS_BEGIN) as *const usize);
    let end = core::ptr::read_volatile((agent + AGENT_STATUS_END) as *const usize);
    if begin == 0 || end < begin {
        return (0, 0);
    }
    (begin, (end - begin) / RECORD_BYTES)
}

unsafe fn record_at(agent: usize, status: i32) -> Option<[usize; LINES]> {
    let (begin, count) = status_vector(agent);
    if status < 0 || status as usize >= count {
        return None;
    }
    let mut record = [0usize; LINES];
    for (line, slot) in record.iter_mut().enumerate() {
        *slot = core::ptr::read_volatile((begin + status as usize * RECORD_BYTES + line * 8) as *const usize);
    }
    Some(record)
}

pub(crate) unsafe fn module_range(address: usize) -> (usize, usize) {
    let mut page = address & !(PAGE - 1);
    for _ in 0..MAX_MODULE_PAGES {
        if core::ptr::read_volatile((page + NRO_MAGIC_AT) as *const u32) == NRO_MAGIC {
            let size = core::ptr::read_volatile((page + NRO_SIZE_AT) as *const u32) as usize;
            return (page, page + size);
        }
        page -= PAGE;
    }
    (0, 0)
}

unsafe fn capture(
    donor: &Donor,
    object: u64,
    boma: u64,
    lua_state: u64,
) -> Option<(Vec<(i32, [usize; LINES])>, (usize, usize))> {
    let creator = creator("status_script", donor)?;
    if let Ok(held) = captured().lock() {
        if let Some(found) = held
            .iter()
            .find(|known| known.donor_kind == donor.kind && known.creator == creator)
        {
            return Some((found.records.clone(), found.code));
        }
    }
    let code = module_range(creator);
    let agent = scratch_agent(creator, donor, object, boma, lua_state)?;
    let vtable = core::ptr::read_volatile(agent as *const usize);
    let setter: AgentCall =
        core::mem::transmute(core::ptr::read_volatile((vtable + SET_STATUS_SCRIPTS_SLOT * 8) as *const usize));
    setter(agent);
    let records: Vec<(i32, [usize; LINES])> = donor
        .statuses
        .iter()
        .filter_map(|status| record_at(agent, *status as i32).map(|record| (*status as i32, record)))
        .collect();
    destroy(agent);
    if let Ok(mut held) = captured().lock() {
        held.retain(|known| known.donor_kind != donor.kind);
        held.push(Captured {
            donor_kind: donor.kind,
            creator,
            code,
            records: records.clone(),
        });
    }
    Some((records, code))
}

pub(crate) unsafe fn on_status_agent(object: u64, boma: u64, lua_state: u64, agent: u64, clone_kind: Option<i32>) {
    if !ARMED.load(Ordering::Acquire) {
        return;
    }
    if let Ok(mut held) = fighters().lock() {
        if held.iter().any(|fighter| fighter.boma == boma as usize || fighter.object == object as usize) {
            let stale_open = held
                .iter()
                .filter(|fighter| {
                    fighter.open && (fighter.boma == boma as usize || fighter.object == object as usize)
                })
                .count();
            OPEN.fetch_sub(stale_open.min(OPEN.load(Ordering::Acquire)), Ordering::AcqRel);
            held.retain(|fighter| fighter.boma != boma as usize && fighter.object != object as usize);
            TRACKED.store(held.len(), Ordering::Release);
        }
    }
    let Some(clone_kind) = clone_kind else {
        return;
    };
    if !readable(agent as usize) || !readable(object as usize) {
        return;
    }
    let Some(donor) = donor_of(clone_kind) else {
        return;
    };
    let Some((records, code)) = capture(&donor, object, boma, lua_state) else {
        log(format!(
            "[finalsmash] lua2cpp_{} is not loaded; clone kind {clone_kind} keeps its base's Final Smash",
            donor.name
        ));
        return;
    };
    route_params(code, &donor);
    let Ok(mut held) = fighters().lock() else {
        return;
    };
    held.push(Fighter {
        boma: boma as usize,
        object: object as usize,
        agent: agent as usize,
        clone_kind,
        donor,
        installed: false,
        open: false,
        held_kind: -1,
        saved: Vec::new(),
        donor_records: records,
    });
    TRACKED.store(held.len(), Ordering::Release);
    log(format!(
        "[finalsmash] clone kind {clone_kind} carries {}'s Final Smash: {} status(es) captured, lua2cpp_{} at {:#x}..{:#x}",
        donor.name,
        held.last().map(|fighter| fighter.donor_records.len()).unwrap_or(0),
        donor.name,
        code.0,
        code.1
    ));
}

pub(crate) unsafe fn on_acmd_agent(index: usize, object: u64, boma: u64, lua_state: u64, agent: u64, clone_kind: i32) {
    if !ARMED.load(Ordering::Acquire) || !readable(agent as usize) || index >= ACMD_CATEGORIES.len() {
        return;
    }
    let Some(donor) = donor_of(clone_kind) else {
        return;
    };
    let category = ACMD_CATEGORIES[index];
    let Some(scripts) = with_donor_list(&donor, |list| final_smash::final_scripts(list, &donor)) else {
        return;
    };
    let Some(creator) = creator(&format!("animcmd_{category}"), &donor) else {
        log(format!("[finalsmash] lua2cpp_{} has no {category} scripts loaded", donor.name));
        return;
    };
    let Some(scratch) = scratch_agent(creator, &donor, object, boma, lua_state) else {
        return;
    };
    let set: SetFunctionHash = core::mem::transmute(crate::game::addr(SET_FUNCTION_HASH));
    let mut node = core::ptr::read_volatile((scratch + AGENT_SCRIPT_FIRST) as *const usize);
    let mut copied = 0usize;
    let mut walked = 0usize;
    while readable(node) && walked < MAX_NODES {
        let key = core::ptr::read_volatile((node + NODE_KEY) as *const u64) & HASH_MASK;
        let function = core::ptr::read_volatile((node + NODE_VALUE) as *const usize);
        if function != 0 && scripts.contains(&key) {
            set(agent as usize, function, key);
            copied += 1;
        }
        node = core::ptr::read_volatile(node as *const usize);
        walked += 1;
    }
    destroy(scratch);
    log(format!(
        "[finalsmash] clone kind {clone_kind} {category}: {copied} of {}'s Final Smash scripts copied",
        donor.name
    ));
}

unsafe fn table_entry(table: usize, index: usize) -> Option<usize> {
    let begin = core::ptr::read_volatile((table + 8) as *const usize);
    let end = core::ptr::read_volatile((table + 0x10) as *const usize);
    if begin == 0 || end <= begin || index >= (end - begin) / L2C_VALUE_BYTES {
        return None;
    }
    Some(begin + index * L2C_VALUE_BYTES)
}

unsafe fn raise_status_total(agent: usize, highest: i32) -> Option<(i64, i64)> {
    let global = agent + AGENT_GLOBAL_TABLE;
    if core::ptr::read_volatile(global as *const u32) != L2C_TYPE_TABLE {
        return None;
    }
    let table = core::ptr::read_volatile((global + 8) as *const usize);
    let slot = table_entry(table, GLOBAL_STATUS_TOTAL)?;
    if core::ptr::read_volatile(slot as *const u32) != L2C_TYPE_INT {
        return None;
    }
    let current = core::ptr::read_volatile((slot + 8) as *const i64);
    let wanted = highest as i64 + 1;
    if wanted > current {
        core::ptr::write_volatile((slot + 8) as *mut i64, wanted);
    }
    Some((current, current.max(wanted)))
}

unsafe fn write_line(agent: usize, status: i32, line: usize, function: usize) {
    let (begin, count) = status_vector(agent);
    if status >= 0 && (status as usize) < count {
        core::ptr::write_volatile((begin + status as usize * RECORD_BYTES + line * 8) as *mut usize, function);
        return;
    }
    if function == 0 {
        return;
    }
    let base = &mut *(agent as *mut smash::lua2cpp::L2CAgentBase);
    let id = smash::lib::L2CValue::new_int(status as u64);
    let condition = smash::lib::L2CValue::new_int(line as u64);
    base.sv_set_status_func(id, condition, &mut *(function as *mut libc::c_void));
}

unsafe fn install(fighter: &mut Fighter) {
    let agent = fighter.agent;
    let mut shared = 0usize;
    let mut direct = 0usize;
    for (status, donor_record) in fighter.donor_records.clone() {
        let base_record = record_at(agent, status).unwrap_or([0; LINES]);
        let collides = status != FINAL && base_record.iter().any(|line| *line != 0);
        if collides {
            fighter.saved.push((status, base_record));
            for line in 0..LINES {
                let function = if donor_record[line] != 0 || base_record[line] != 0 {
                    SHIMS[line] as usize
                } else {
                    0
                };
                write_line(agent, status, line, function);
            }
            shared += 1;
        } else {
            for (line, function) in donor_record.iter().enumerate() {
                write_line(agent, status, line, *function);
            }
            direct += 1;
        }
    }
    let total = raise_status_total(agent, fighter.donor.highest_status());
    fighter.installed = true;
    log(format!(
        "[finalsmash] clone kind {} installs {}'s Final Smash: {direct} status(es) written, {shared} shared with the base by status history, status total {:?}",
        fighter.clone_kind, fighter.donor.name, total
    ));
}

unsafe fn open(fighter: &mut Fighter) {
    if !fighter.installed {
        install(fighter);
    }
    let kind = core::ptr::read_volatile((fighter.object + OBJECT_KIND) as *const i32);
    fighter.held_kind = kind;
    core::ptr::write_volatile((fighter.object + OBJECT_KIND) as *mut i32, fighter.donor.kind);
    fighter.open = true;
    OPEN.fetch_add(1, Ordering::AcqRel);
    let params = crate::fighter_params::ensure_kind_params_loaded(fighter.donor.kind)
        && crate::fighter_params::payload_of(fighter.donor.kind).is_some()
        && payload_cell(fighter.boma).is_some();
    log(format!(
        "[finalsmash] clone kind {} starts {}'s Final Smash (kind {kind} reads as {}, {}'s params {})",
        fighter.clone_kind,
        fighter.donor.name,
        fighter.donor.kind,
        fighter.donor.name,
        if params { "answer its own code" } else { "NOT loaded" }
    ));
}

unsafe fn payload_cell(boma: usize) -> Option<usize> {
    let work = core::ptr::read_volatile((boma + BOMA_WORK_MODULE) as *const usize);
    if !readable(work) {
        return None;
    }
    let accessor = core::ptr::read_volatile((work + WORK_PARAM_ACCESSOR) as *const usize);
    if !readable(accessor) {
        return None;
    }
    let block = core::ptr::read_volatile((accessor + ACCESSOR_BLOCK) as *const usize);
    if !readable(block) {
        return None;
    }
    let cell = block + BLOCK_PAYLOAD;
    readable(core::ptr::read_volatile(cell as *const usize)).then_some(cell)
}

unsafe fn close(fighter: &mut Fighter, status: i32) {
    if fighter.held_kind >= 0 {
        core::ptr::write_volatile((fighter.object + OBJECT_KIND) as *mut i32, fighter.held_kind);
    }
    if fighter.open {
        OPEN.fetch_sub(1, Ordering::AcqRel);
    }
    fighter.open = false;
    log(format!(
        "[finalsmash] clone kind {} ends {}'s Final Smash into status {status:#x}; {} of {}'s param read(s) answered from its own params",
        fighter.clone_kind,
        fighter.donor.name,
        ANSWERED.swap(0, Ordering::AcqRel),
        fighter.donor.name
    ));
}

unsafe fn donor_payload(boma: usize) -> Option<(usize, usize)> {
    if OPEN.load(Ordering::Acquire) == 0 {
        return None;
    }
    let donor_kind = {
        let held = fighters().lock().ok()?;
        held.iter().find(|fighter| fighter.boma == boma && fighter.open)?.donor.kind
    };
    let payload = crate::fighter_params::payload_of(donor_kind)?;
    let cell = payload_cell(boma)?;
    ANSWERED.fetch_add(1, Ordering::Relaxed);
    Some((cell, payload))
}

unsafe fn with_donor_payload<T>(boma: usize, read: impl FnOnce() -> T) -> T {
    let Some((cell, payload)) = donor_payload(boma) else {
        return read();
    };
    let own = core::ptr::read_volatile(cell as *const usize);
    core::ptr::write_volatile(cell as *mut usize, payload);
    let value = read();
    core::ptr::write_volatile(cell as *mut usize, own);
    value
}

unsafe extern "C" fn donor_param_int(boma: *mut smash::app::BattleObjectModuleAccessor, group: u64, key: u64) -> i32 {
    with_donor_payload(boma as usize, || smash::app::lua_bind::WorkModule::get_param_int(boma, group, key))
}

unsafe extern "C" fn donor_param_float(boma: *mut smash::app::BattleObjectModuleAccessor, group: u64, key: u64) -> f32 {
    with_donor_payload(boma as usize, || smash::app::lua_bind::WorkModule::get_param_float(boma, group, key))
}

unsafe extern "C" fn donor_param_int64(boma: *mut smash::app::BattleObjectModuleAccessor, group: u64, key: u64) -> u64 {
    with_donor_payload(boma as usize, || smash::app::lua_bind::WorkModule::get_param_int64(boma, group, key))
}

fn routed_imports() -> [(&'static [u8], usize); 3] {
    [
        (PARAM_INT_IMPORT, donor_param_int as *const () as usize),
        (PARAM_FLOAT_IMPORT, donor_param_float as *const () as usize),
        (PARAM_INT64_IMPORT, donor_param_int64 as *const () as usize),
    ]
}

fn module_pointer(code: (usize, usize), value: usize) -> Option<usize> {
    if value < code.1 - code.0 {
        Some(code.0 + value)
    } else if value >= code.0 && value < code.1 {
        Some(value)
    } else {
        None
    }
}

unsafe fn name_is(code: (usize, usize), at: usize, name: &[u8]) -> bool {
    if at + name.len() > code.1 {
        return false;
    }
    core::slice::from_raw_parts(at as *const u8, name.len()) == name
}

unsafe fn route_params(code: (usize, usize), donor: &Donor) {
    if code.0 == 0 || code.1 <= code.0 {
        return;
    }
    let mod0 = code.0 + core::ptr::read_volatile((code.0 + NRO_MOD0_AT) as *const u32) as usize;
    if mod0 + 8 > code.1 || core::ptr::read_volatile(mod0 as *const u32) != MOD0_MAGIC {
        log(format!("[finalsmash] lua2cpp_{} has no MOD0 header; its params stay the base's", donor.name));
        return;
    }
    let dynamic = (mod0 as isize + core::ptr::read_volatile((mod0 + MOD0_DYNAMIC_AT) as *const i32) as isize) as usize;
    let (mut strtab, mut symtab, mut jmprel, mut jmprel_bytes) = (None, None, None, 0usize);
    for index in 0..MAX_DYNAMIC_ENTRIES {
        let entry = dynamic + index * DYNAMIC_ENTRY_BYTES;
        if entry + DYNAMIC_ENTRY_BYTES > code.1 {
            break;
        }
        let tag = core::ptr::read_volatile(entry as *const i64);
        let value = core::ptr::read_volatile((entry + 8) as *const u64) as usize;
        match tag {
            DT_NULL => break,
            DT_PLTRELSZ => jmprel_bytes = value,
            DT_STRTAB => strtab = module_pointer(code, value),
            DT_SYMTAB => symtab = module_pointer(code, value),
            DT_JMPREL => jmprel = module_pointer(code, value),
            _ => {}
        }
    }
    let (Some(strtab), Some(symtab), Some(jmprel)) = (strtab, symtab, jmprel) else {
        log(format!("[finalsmash] lua2cpp_{} has no import table; its params stay the base's", donor.name));
        return;
    };
    let imports = routed_imports();
    let mut routed = 0usize;
    for index in 0..jmprel_bytes / RELA_BYTES {
        let rela = jmprel + index * RELA_BYTES;
        if rela + RELA_BYTES > code.1 {
            break;
        }
        let offset = core::ptr::read_volatile(rela as *const u64) as usize;
        let info = core::ptr::read_volatile((rela + 8) as *const u64);
        if info as u32 != R_AARCH64_JUMP_SLOT {
            continue;
        }
        let symbol = symtab + (info >> 32) as usize * SYMBOL_BYTES;
        if symbol + SYMBOL_BYTES > code.1 {
            continue;
        }
        let name = strtab + core::ptr::read_volatile(symbol as *const u32) as usize;
        let Some((_, target)) = imports.iter().find(|(import, _)| name_is(code, name, import)) else {
            continue;
        };
        let Some(slot) = module_pointer(code, offset) else {
            continue;
        };
        if core::ptr::read_volatile(slot as *const usize) != *target
            && crate::text_patch::write_bytes(slot, &(*target as u64).to_le_bytes())
        {
            routed += 1;
        }
    }
    if routed > 0 {
        log(format!(
            "[finalsmash] lua2cpp_{} reads its params through the engine: {routed} import(s) routed, so its Final Smash code reads {}'s params on a clone",
            donor.name, donor.name
        ));
    }
}

fn shim_target(agent: usize, line: usize) -> usize {
    unsafe {
        let boma = core::ptr::read_volatile((agent + AGENT_BOMA) as *const usize);
        if !readable(boma) {
            return 0;
        }
        let status = smash::app::lua_bind::StatusModule::status_kind(
            boma as *mut smash::app::BattleObjectModuleAccessor,
        );
        let Ok(held) = fighters().lock() else {
            return 0;
        };
        let Some(fighter) = held.iter().find(|fighter| fighter.agent == agent) else {
            return 0;
        };
        let source = if fighter.open {
            &fighter.donor_records
        } else {
            &fighter.saved
        };
        source
            .iter()
            .find(|(known, _)| *known == status)
            .map(|(_, record)| record[line])
            .unwrap_or(0)
    }
}

struct Shim<const LINE: usize>;

impl<const LINE: usize> Shim<LINE> {
    unsafe extern "C" fn call(agent: *mut u8) -> smash::lib::L2CValue {
        let target = shim_target(agent as usize, LINE);
        if target == 0 {
            return smash::lib::L2CValue::new_int(0);
        }
        let function: StatusFn = core::mem::transmute(target);
        function(agent)
    }
}

static SHIMS: [StatusFn; LINES] = [
    Shim::<0>::call,
    Shim::<1>::call,
    Shim::<2>::call,
    Shim::<3>::call,
    Shim::<4>::call,
    Shim::<5>::call,
    Shim::<6>::call,
    Shim::<7>::call,
    Shim::<8>::call,
    Shim::<9>::call,
    Shim::<10>::call,
    Shim::<11>::call,
    Shim::<12>::call,
    Shim::<13>::call,
    Shim::<14>::call,
    Shim::<15>::call,
    Shim::<16>::call,
    Shim::<17>::call,
    Shim::<18>::call,
    Shim::<19>::call,
    Shim::<20>::call,
    Shim::<21>::call,
];

#[skyline::hook(offset = crate::game::rel(STATUS_CHANGE))]
unsafe fn status_change(module: usize, status: i32) -> u64 {
    crate::tether::refresh(module);
    if TRACKED.load(Ordering::Acquire) == 0 || !readable(module) {
        return call_original!(module, status);
    }
    let boma = core::ptr::read_volatile((module + STATUS_MODULE_BOMA) as *const usize);
    let tracked = {
        let Ok(mut held) = fighters().lock() else {
            return call_original!(module, status);
        };
        match held.iter_mut().find(|fighter| fighter.boma == boma) {
            Some(fighter) => {
                if status == FINAL && !fighter.open {
                    open(fighter);
                }
                true
            }
            None => false,
        }
    };
    let result = call_original!(module, status);
    if tracked {
        if let Ok(mut held) = fighters().lock() {
            if let Some(fighter) = held.iter_mut().find(|fighter| fighter.boma == boma) {
                if fighter.open && !fighter.donor.is_final_status(status) {
                    close(fighter, status);
                }
            }
        }
    }
    result
}

pub(crate) fn install_hooks() {
    let sites = [
        (STATUS_CHANGE, STATUS_CHANGE_WORD),
        (MODULE_REQUEST, MODULE_REQUEST_WORD),
        (SET_FUNCTION_HASH, SET_FUNCTION_HASH_WORD),
    ];
    for (offset, expected) in sites {
        let word = unsafe { crate::game::text_word(offset) };
        if word != expected {
            crate::dbg_log_public(&format!(
                "[finalsmash] status code at {offset:#x} not recognised ({word:#010x}); borrowed Final Smashes stay off"
            ));
            return;
        }
    }
    skyline::install_hooks!(status_change);
    ARMED.store(true, Ordering::Release);
    crate::dbg_log_public("[finalsmash] armed: a clone's [final] donor runs its own statuses and scripts during the Final Smash");
}
