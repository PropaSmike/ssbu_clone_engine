use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use crate::item_slots::{text_word, InlineHook};

const AGENT_MANAGER_GLOBAL: usize = 0x593A340;

const OFF_AGENT_GET_OR_CREATE: usize = 0x372BA50;
const AGENT_GET_OR_CREATE_EXPECTED: u32 = 0xD10143FF;

const OFF_AGENT_RELEASE: usize = 0x372BCB0;
const AGENT_RELEASE_EXPECTED: u32 = 0xD10203FF;

const OFF_AGENT_LOAD_CHUNK: usize = 0x372C470;
const AGENT_LOAD_CHUNK_EXPECTED: u32 = 0xA9BB67FA;

const OFF_LUA_LOAD_CHUNK: usize = 0x372D180;

const OFF_AGENT_LOCK: usize = 0x39C1490;
const AGENT_LOCK_EXPECTED: u32 = 0xB000C6F0;
const OFF_AGENT_UNLOCK: usize = 0x39C14A0;
const AGENT_UNLOCK_EXPECTED: u32 = 0xB000C6F0;
const AGENT_MANAGER_LOCK_FIELD: usize = 0x70;
const LUA_LOAD_CHUNK_EXPECTED: u32 = 0xA9BD57FC;
const LUA_CHUNK_NAME: &[u8] = b"buf\0";

const LUA_STATE_TOP: usize = 0x10;
const LUA_STATE_CI: usize = 0x20;
const LUA_TVALUE_STRIDE: usize = 0x10;
const LUA_TVALUE_TAG: usize = 8;

const ITEM_PARENT_AGENT: u64 = 0x9_846E_2F98;

const KIND_AGENT_TABLE: usize = 0x453E7B8;
const KIND_AGENT_ROWS: usize = 0x1B0;
const KIND_AGENT_STRIDE: usize = 0x10;

const AGENT_FALLBACK: u64 = 0xE_573A_5B8A;
const AGENT_KIND_1B1: u64 = 0x15_3EF2_779E;
const AGENT_KIND_1B2: u64 = 0x19_BE28_306A;

const ITEM_AGENT_HASH_FIELD: usize = 0x4D8;

const SCRIPT_OBJECT_SITE: usize = 0x161B2E0;
const SCRIPT_OBJECT_EXPECTED: u32 = 0xAA0103F4;
const SCRIPT_OBJECT_HASH_REGISTER: usize = 22;

const SCRIPT_MODULE_SITE: usize = 0x1620668;
const SCRIPT_MODULE_EXPECTED: u32 = 0x52805401;
const SCRIPT_MODULE_HASH_REGISTER: usize = 21;

const ITEM_REGISTER: usize = 1;

const SCRIPT_FILES: [&str; 3] = ["game", "effect", "sound"];

const MAX_CLONE_KINDS: usize = 16;

struct CloneAgent {
    public_kind: AtomicI32,
    hash: AtomicU64,
    paths: [AtomicU32; SCRIPT_FILES.len()],
    hashes: [AtomicU64; SCRIPT_FILES.len()],
    resolved: AtomicBool,
    requested: AtomicBool,
    stage: AtomicU32,
    built_agent: AtomicUsize,
    built_state: AtomicUsize,
    built_parent: AtomicUsize,
    built_parent_state: AtomicUsize,
    acquired: AtomicU32,
    reported: AtomicBool,
    completed: AtomicBool,
}

impl CloneAgent {
    const fn new() -> Self {
        Self {
            public_kind: AtomicI32::new(-1),
            hash: AtomicU64::new(0),
            paths: [const { AtomicU32::new(u32::MAX) }; SCRIPT_FILES.len()],
            hashes: [const { AtomicU64::new(0) }; SCRIPT_FILES.len()],
            resolved: AtomicBool::new(false),
            requested: AtomicBool::new(false),
            stage: AtomicU32::new(0),
            built_agent: AtomicUsize::new(0),
            built_state: AtomicUsize::new(0),
            built_parent: AtomicUsize::new(0),
            built_parent_state: AtomicUsize::new(0),
            acquired: AtomicU32::new(0),
            reported: AtomicBool::new(false),
            completed: AtomicBool::new(false),
        }
    }
}

static AGENTS: [CloneAgent; MAX_CLONE_KINDS] = [const { CloneAgent::new() }; MAX_CLONE_KINDS];

static PREFLIGHT_OK: AtomicBool = AtomicBool::new(false);
static HOOKS_INSTALLED: AtomicBool = AtomicBool::new(false);
static SWAP_REPORTED: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

const SWAP_REPORT_LIMIT: u32 = 64;
const DECLINE_REPORT_LIMIT: u32 = 64;

fn slot_for(public_kind: i32) -> Option<&'static CloneAgent> {
    if public_kind < 0 {
        return None;
    }
    AGENTS
        .iter()
        .find(|slot| slot.public_kind.load(Ordering::Acquire) == public_kind)
}

fn claim_slot(public_kind: i32) -> Option<&'static CloneAgent> {
    if let Some(slot) = slot_for(public_kind) {
        return Some(slot);
    }
    AGENTS.iter().find(|slot| {
        slot.public_kind
            .compare_exchange(-1, public_kind, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
    })
}

unsafe fn native_agent_hash(kind: i32) -> u64 {
    if (0..KIND_AGENT_ROWS as i32).contains(&kind) {
        let row = crate::text_base() + KIND_AGENT_TABLE + kind as usize * KIND_AGENT_STRIDE;
        if core::ptr::read_volatile((row + 8) as *const u32) == kind as u32 {
            return core::ptr::read_volatile(row as *const u64);
        }
    }
    match kind {
        0x1B1 => AGENT_KIND_1B1,
        0x1B2 => AGENT_KIND_1B2,
        _ => AGENT_FALLBACK,
    }
}

unsafe fn mint_agent_hash(public_kind: i32, content_root: &str, namespace: &str) -> Option<u64> {
    let hash =
        crate::hash40::hash40(&format!("clone_engine/{content_root}/{namespace}")) & 0xFF_FFFF_FFFF;
    if hash == 0 || hash == AGENT_FALLBACK || hash == AGENT_KIND_1B1 || hash == AGENT_KIND_1B2 {
        return None;
    }
    for kind in 0..KIND_AGENT_ROWS {
        let row = crate::text_base() + KIND_AGENT_TABLE + kind * KIND_AGENT_STRIDE;
        if core::ptr::read_volatile(row as *const u64) & 0xFF_FFFF_FFFF == hash {
            crate::dbg_log_public(&format!(
                "[itemlua] minted hash {hash:#x} for clone {public_kind:#x} collides with \
                 kind {kind:#x}; refusing"
            ));
            return None;
        }
    }
    Some(hash)
}

unsafe fn namespace_of(public_kind: i32) -> Option<String> {
    let name = crate::item_clones::clone_engine_item_resource_name(public_kind);
    if name.is_null() {
        return None;
    }
    core::ffi::CStr::from_ptr(name)
        .to_str()
        .ok()
        .map(str::to_owned)
}

pub(crate) fn prepare(public_kind: i32) {
    if !PREFLIGHT_OK.load(Ordering::Acquire) {
        return;
    }
    let category = crate::item_params::item_content_category(public_kind);
    if category == crate::item_params::ItemContentCategory::KoopagExternal {
        crate::dbg_log_public(&format!(
            "[itemlua] clone {public_kind:#x} uses Koopag's external lua2cpp_koopag agent; \
             lua2cpp_item script routing is unsupported"
        ));
        return;
    }
    let content_root = category.root();
    let Some(slot) = claim_slot(public_kind) else {
        crate::dbg_log_public(&format!(
            "[itemlua] no free agent slot for clone {public_kind:#x}"
        ));
        return;
    };
    if slot.resolved.swap(true, Ordering::AcqRel) {
        return;
    }
    unsafe {
        let Some(namespace) = namespace_of(public_kind) else {
            return;
        };
        let Some(hash) = mint_agent_hash(public_kind, content_root, &namespace) else {
            return;
        };
        slot.hash.store(hash, Ordering::Release);
        let mut found = 0usize;
        for (index, stem) in SCRIPT_FILES.iter().enumerate() {
            let path = format!("{content_root}/{namespace}/script/animcmd/body/{stem}.lc");
            let path_hash = crate::hash40::hash40(&path);
            slot.hashes[index].store(path_hash, Ordering::Release);
            let Some(file) = crate::item_params::scan_file_path_index(path_hash) else {
                continue;
            };
            slot.paths[index].store(file, Ordering::Release);
            found += 1;
        }
        request_files(slot);
        crate::dbg_log_public(&format!(
            "[itemlua] clone {public_kind:#x} ({content_root}/{namespace}) agent hash {hash:#x}; \
             {found} of {} script files resolved and requested",
            SCRIPT_FILES.len()
        ));
    }
}

unsafe fn reset_lua_stack(state: usize) {
    let ci = core::ptr::read_volatile((state + LUA_STATE_CI) as *const usize);
    if ci == 0 {
        return;
    }
    let want = core::ptr::read_volatile(ci as *const usize) + LUA_TVALUE_STRIDE;
    if want < LUA_TVALUE_STRIDE {
        return;
    }
    let mut top = core::ptr::read_volatile((state + LUA_STATE_TOP) as *const usize);
    while top < want {
        core::ptr::write_volatile((top + LUA_TVALUE_TAG) as *mut u32, 0);
        top += LUA_TVALUE_STRIDE;
    }
    core::ptr::write_volatile((state + LUA_STATE_TOP) as *mut usize, want);
}

unsafe fn with_agent_lock<T>(body: impl FnOnce() -> T) -> Option<T> {
    let manager = agent_manager()?;
    let target = manager + AGENT_MANAGER_LOCK_FIELD;
    let lock: unsafe extern "C" fn(usize) = core::mem::transmute(crate::text_base() + OFF_AGENT_LOCK);
    let unlock: unsafe extern "C" fn(usize) =
        core::mem::transmute(crate::text_base() + OFF_AGENT_UNLOCK);
    lock(target);
    let out = body();
    unlock(target);
    Some(out)
}

unsafe fn compile_own_chunk(state: usize, hash: u64) -> Result<usize, String> {
    let bytes = crate::item_params::read_own_file(hash)?;
    if bytes.len() < 4 || &bytes[..4] != b"\x1bLua" {
        return Err(format!(
            "{} bytes and it is not Lua bytecode ({:02x?})",
            bytes.len(),
            &bytes[..bytes.len().min(4)]
        ));
    }
    let load: unsafe extern "C" fn(usize, *const u8, usize, *const u8, u32) -> u32 =
        core::mem::transmute(crate::text_base() + OFF_LUA_LOAD_CHUNK);
    let Some(result) = with_agent_lock(|| {
        load(
            state,
            bytes.as_ptr(),
            bytes.len(),
            LUA_CHUNK_NAME.as_ptr(),
            1,
        )
    }) else {
        return Err("agent manager not constructed; chunk load skipped".to_string());
    };
    if result != 0 {
        reset_lua_stack(state);
        return Err(format!(
            "0x372D180 returned {result:#x} for {} bytes",
            bytes.len()
        ));
    }
    Ok(bytes.len())
}

fn request_files(slot: &CloneAgent) {
    for cell in &slot.paths {
        let file = cell.load(Ordering::Acquire);
        if file == u32::MAX {
            continue;
        }
        unsafe {
            if crate::item_params::resource_is_resident(file) {
                continue;
            }
            crate::item_params::request_file_path(file);
        }
    }
    slot.requested.store(true, Ordering::Release);
}

unsafe fn agent_manager() -> Option<usize> {
    let holder =
        core::ptr::read_volatile((crate::text_base() + AGENT_MANAGER_GLOBAL) as *const usize);
    if holder == 0 {
        return None;
    }
    let manager = core::ptr::read_volatile(holder as *const usize);
    (manager != 0).then_some(manager)
}

unsafe fn find_agent(hash: u64) -> Option<usize> {
    const MASK: u64 = 0xFF_FFFF_FFFF;
    let manager = agent_manager()?;
    let wanted = hash & MASK;
    let mut node = core::ptr::read_volatile((manager + 8) as *const usize);
    let mut count = 0usize;
    while node != manager && node != 0 && count < 512 {
        let agent = core::ptr::read_volatile((node + 0x10) as *const usize);
        if agent != 0 && core::ptr::read_volatile((agent + 0x18) as *const u64) & MASK == wanted {
            return Some(agent);
        }
        count += 1;
        node = core::ptr::read_volatile((node + 8) as *const usize);
    }
    None
}

unsafe fn survey_agents(manager: usize, wanted: [u64; 2]) -> (usize, [usize; 2]) {
    const MASK: u64 = 0xFF_FFFF_FFFF;
    let mut found = [0usize; 2];
    let mut count = 0usize;
    let mut node = core::ptr::read_volatile((manager + 8) as *const usize);
    while node != manager && node != 0 && count < 512 {
        let agent = core::ptr::read_volatile((node + 0x10) as *const usize);
        if agent != 0 {
            let hash = core::ptr::read_volatile((agent + 0x18) as *const u64) & MASK;
            for (slot, want) in found.iter_mut().zip(wanted) {
                if *slot == 0 && hash == want & MASK {
                    *slot = agent;
                }
            }
        }
        count += 1;
        node = core::ptr::read_volatile((node + 8) as *const usize);
    }
    (count, found)
}

unsafe fn load_chunks(
    slot: &CloneAgent,
    agent: usize,
    state: usize,
) -> (usize, Vec<&'static str>, Vec<String>) {
    let load_chunk: unsafe extern "C" fn(usize, *const u32, u32) =
        core::mem::transmute(crate::text_base() + OFF_AGENT_LOAD_CHUNK);
    let mut compiled = 0usize;
    let mut pending: Vec<&'static str> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    for (index, stem) in SCRIPT_FILES.iter().enumerate() {
        let file = slot.paths[index].load(Ordering::Acquire);
        if file != u32::MAX && crate::item_params::resource_is_resident(file) {
            load_chunk(agent, &file as *const u32, 1);
            compiled += 1;
            continue;
        }
        let hash = slot.hashes[index].load(Ordering::Acquire);
        match compile_own_chunk(state, hash) {
            Ok(_) => compiled += 1,
            Err(reason) => {
                pending.push(stem);
                if !slot.completed.load(Ordering::Acquire) {
                    failures.push(format!("{stem}: {reason}"));
                }
            }
        }
    }
    (compiled, pending, failures)
}

unsafe fn parent_identity() -> (usize, usize) {
    let Some(parent) = find_agent(ITEM_PARENT_AGENT) else {
        return (0, 0);
    };
    (
        parent,
        core::ptr::read_volatile((parent + 8) as *const usize),
    )
}

unsafe fn retire_agent(
    manager: usize,
    slot: &CloneAgent,
    public_kind: i32,
    hash: u64,
    parent: usize,
    parent_state: usize,
) {
    let release: unsafe extern "C" fn(usize, u64) =
        core::mem::transmute(crate::text_base() + OFF_AGENT_RELEASE);
    let taken = slot.acquired.swap(0, Ordering::AcqRel);
    for _ in 0..taken {
        release(manager, hash);
    }
    let residue = find_agent(hash);
    slot.built_agent.store(0, Ordering::Release);
    slot.built_state.store(0, Ordering::Release);
    slot.built_parent.store(0, Ordering::Release);
    slot.built_parent_state.store(0, Ordering::Release);
    slot.completed.store(false, Ordering::Release);
    static RETIRED: AtomicU32 = AtomicU32::new(0);
    let count = RETIRED.fetch_add(1, Ordering::Relaxed);
    if count < 32 {
        crate::dbg_log_public(&format!(
            "[itemlua] clone {public_kind:#x} hash {hash:#x} RETIRED #{count}: item parent agent is now {parent:#x} state {parent_state:#x}; released {taken} reference(s), residue {residue:#x?}"
        ));
    }
}

pub(crate) fn ensure_agent(public_kind: i32) -> Option<u64> {
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return None;
    }
    let slot = slot_for(public_kind)?;
    let hash = slot.hash.load(Ordering::Acquire);
    if hash == 0 {
        return None;
    }

    let report = |detail: String| {
        static REPORTED: AtomicU32 = AtomicU32::new(0);
        if REPORTED.fetch_add(1, Ordering::Relaxed) < 32 {
            crate::dbg_log_public(&format!(
                "[itemlua] clone {public_kind:#x} hash {hash:#x}: {detail}"
            ));
        }
    };
    unsafe {
        let Some(manager) = agent_manager() else {
            report("lua agent manager is not constructed yet".into());
            return None;
        };

        if slot.stage.load(Ordering::Acquire) == 2 {
            let live = find_agent(hash);
            let state = live
                .map(|agent| core::ptr::read_volatile((agent + 8) as *const usize))
                .unwrap_or(0);
            let (parent, parent_state) = parent_identity();
            let built = slot.built_agent.load(Ordering::Acquire);
            let parent_moved = parent != slot.built_parent.load(Ordering::Acquire)
                || parent_state != slot.built_parent_state.load(Ordering::Acquire);
            if live == Some(built)
                && state == slot.built_state.load(Ordering::Acquire)
                && parent != 0
                && !parent_moved
            {
                return Some(hash);
            }
            if live.is_none() {
                slot.acquired.store(0, Ordering::Release);
            } else if parent_moved {
                retire_agent(manager, slot, public_kind, hash, parent, parent_state);
            }
            slot.stage.store(0, Ordering::Release);
            slot.reported.store(false, Ordering::Release);
        }

        request_files(slot);
        let (agents, [parent_live, _]) = survey_agents(manager, [ITEM_PARENT_AGENT, hash]);
        let get_or_create: unsafe extern "C" fn(usize, u64, u64) -> u32 =
            core::mem::transmute(crate::text_base() + OFF_AGENT_GET_OR_CREATE);
        let result = get_or_create(manager, hash, ITEM_PARENT_AGENT);
        let (_, [_, agent]) = survey_agents(manager, [ITEM_PARENT_AGENT, hash]);
        if result != 0 || agent == 0 {
            report(format!(
                "0x372BA50 returned {result:#x} and left agent={agent:#x}: \
                 manager={manager:#x} agents={agents} parent present={:#x}",
                parent_live
            ));
            return None;
        }
        slot.acquired.fetch_add(1, Ordering::AcqRel);
        slot.stage.store(1, Ordering::Release);
        let parent_state = if parent_live == 0 {
            0
        } else {
            core::ptr::read_volatile((parent_live + 8) as *const usize)
        };
        let state = core::ptr::read_volatile((agent + 8) as *const usize);
        if state == 0 {
            report(format!(
                "agent {agent:#x} has no lua_State (agents={agents})"
            ));
            return None;
        }
        let (compiled, pending, failures) = load_chunks(slot, agent, state);
        let detail = format!(
            "agent {agent:#x} state {state:#x} (agents={agents}, parent {parent_live:#x} state {parent_state:#x}): {compiled} chunks compiled, still loading {pending:?} {failures:?}"
        );
        if pending.is_empty() && parent_live != 0 {
            slot.built_agent.store(agent, Ordering::Release);
            slot.built_state.store(state, Ordering::Release);
            slot.built_parent.store(parent_live, Ordering::Release);
            slot.built_parent_state.store(parent_state, Ordering::Release);
            slot.stage.store(2, Ordering::Release);
            if !slot.completed.swap(true, Ordering::AcqRel) {
                crate::dbg_log_public(&format!(
                    "[itemlua] clone {public_kind:#x} hash {hash:#x} COMPLETE: {detail}"
                ));
            }
        }
        report(detail);
        (compiled > 0).then_some(hash)
    }
}

unsafe fn clone_identity_of_item(item: usize) -> Option<(i32, i32)> {
    if item == 0 {
        return None;
    }
    let battle_object = core::ptr::read_volatile(item as *const usize);
    if let Some(identity) = crate::item_clones::live_identity_of_object(battle_object) {
        return Some(identity);
    }
    let public = crate::item_clones::active_public_kind()?;
    let base = crate::item_clones::clone_engine_item_base_kind(public);
    (base >= 0).then_some((public, base))
}

unsafe fn swap_agent_hash(
    item: usize,
    site: &str,
    seen: &core::sync::atomic::AtomicU32,
) -> Option<u64> {
    let announce = seen.fetch_add(1, Ordering::Relaxed) < SWAP_REPORT_LIMIT;
    let identity = clone_identity_of_item(item);
    let field = (item + ITEM_AGENT_HASH_FIELD) as *mut u64;
    let current = core::ptr::read_volatile(field);
    if announce {
        crate::dbg_log_public(&format!(
            "[itemlua] {site} first hit: item={item:#x} [item]={:#x} identity={identity:#x?} \
             field={current:#x}",
            core::ptr::read_volatile(item as *const usize)
        ));
    }
    let (public, base) = identity?;
    let hash = ensure_agent(public)?;
    if current == hash {
        return Some(hash);
    }
    if current != native_agent_hash(base) {
        static DECLINED: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        let count = DECLINED.fetch_add(1, Ordering::Relaxed);
        if count < DECLINE_REPORT_LIMIT {
            crate::dbg_log_public(&format!(
                "[itemlua] {site} DECLINED #{count}: item {item:#x} clone {public:#x} field                  {current:#x} is not base {base:#x} native {:#x}; item will run no scripts",
                native_agent_hash(base)
            ));
        }
        return None;
    }
    core::ptr::write_volatile(field, hash);
    crate::item_clones::bind_live_module(item, base);
    let reported = SWAP_REPORTED.fetch_add(1, Ordering::Relaxed);
    if reported < SWAP_REPORT_LIMIT {
        crate::dbg_log_public(&format!(
            "[itemlua] #{reported} item {item:#x} is clone {public:#x} (base {base:#x}): \
             agent hash {current:#x} -> {hash:#x}"
        ));
    }
    Some(hash)
}

pub(crate) unsafe fn restore_agent_hash(item: usize, base_kind: i32) {
    if item == 0 || base_kind < 0 {
        return;
    }
    let field = (item + ITEM_AGENT_HASH_FIELD) as *mut u64;
    let native = native_agent_hash(base_kind);
    if core::ptr::read_volatile(field) != native {
        core::ptr::write_volatile(field, native);
    }
}

macro_rules! swap_hook {
    ($name:ident, $register:expr) => {
        unsafe extern "C" fn $name(ctx: &mut skyline::hooks::InlineCtx) {
            static SEEN: core::sync::atomic::AtomicU32 =
                core::sync::atomic::AtomicU32::new(0);
            let item = ctx.registers[ITEM_REGISTER].x() as usize;
            if let Some(hash) = swap_agent_hash(item, stringify!($name), &SEEN) {
                ctx.registers[$register].set_x(hash);
            }
        }
    };
}

swap_hook!(script_object_agent, SCRIPT_OBJECT_HASH_REGISTER);
swap_hook!(script_module_agent, SCRIPT_MODULE_HASH_REGISTER);

const SITES: &[(usize, u32, InlineHook)] = &[
    (
        SCRIPT_OBJECT_SITE,
        SCRIPT_OBJECT_EXPECTED,
        script_object_agent,
    ),
    (
        SCRIPT_MODULE_SITE,
        SCRIPT_MODULE_EXPECTED,
        script_module_agent,
    ),
];

unsafe fn preflight() -> Result<(), String> {
    for (offset, expected) in [
        (OFF_AGENT_GET_OR_CREATE, AGENT_GET_OR_CREATE_EXPECTED),
        (OFF_AGENT_RELEASE, AGENT_RELEASE_EXPECTED),
        (OFF_AGENT_LOAD_CHUNK, AGENT_LOAD_CHUNK_EXPECTED),
        (OFF_LUA_LOAD_CHUNK, LUA_LOAD_CHUNK_EXPECTED),
        (OFF_AGENT_LOCK, AGENT_LOCK_EXPECTED),
        (OFF_AGENT_UNLOCK, AGENT_UNLOCK_EXPECTED),
        (SCRIPT_OBJECT_SITE, SCRIPT_OBJECT_EXPECTED),
        (SCRIPT_MODULE_SITE, SCRIPT_MODULE_EXPECTED),
    ] {
        let actual = text_word(offset);
        if actual != expected {
            return Err(format!(
                "{offset:#x}: expected {expected:#010x}, found {actual:#010x}"
            ));
        }
    }
    let table = crate::text_base() + KIND_AGENT_TABLE;
    for kind in 0..KIND_AGENT_ROWS {
        let row = table + kind * KIND_AGENT_STRIDE;
        if core::ptr::read_volatile((row + 8) as *const u32) != kind as u32 {
            return Err(format!(
                "kind -> agent table row {kind:#x} keys {:#x}, not itself",
                core::ptr::read_volatile((row + 8) as *const u32)
            ));
        }
    }
    Ok(())
}

pub(crate) fn install() {
    match unsafe { preflight() } {
        Ok(()) => unsafe {
            PREFLIGHT_OK.store(true, Ordering::Release);
            let text = crate::text_base();
            let mut failed: Vec<usize> = Vec::new();
            for (offset, original, hook) in SITES {
                skyline::hooks::A64InlineHook(
                    (text + *offset) as *const libc::c_void,
                    *hook as *const () as *const libc::c_void,
                );
                if text_word(*offset) == *original {
                    failed.push(*offset);
                }
            }
            if failed.is_empty() {
                HOOKS_INSTALLED.store(true, Ordering::Release);
                crate::dbg_log_public(&format!(
                    "[itemlua] installed {} hooks; clone item scripts armed",
                    SITES.len()
                ));
            } else {
                crate::dbg_log_public(&format!(
                    "[itemlua] DISARMED: {} of {} hooks failed to relocate ({:#x?})",
                    failed.len(),
                    SITES.len(),
                    failed
                ));
            }
        },
        Err(reason) => {
            crate::dbg_log_public(&format!(
                "[itemlua] preflight failed - {reason}; item scripts stay vanilla"
            ));
        }
    }
}

pub(crate) fn ready() -> bool {
    PREFLIGHT_OK.load(Ordering::Acquire) && HOOKS_INSTALLED.load(Ordering::Acquire)
}
