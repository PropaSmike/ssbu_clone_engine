use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use smash::phx::Hash40;

const BEAM_SWORD_KIND: i32 = 0x32;
const BEAM_SWORD_KIND_HASH: u64 = 0x13CC565112;

const OFF_ITEM_LOWER_CREATOR: usize = 0x15DB0B0;
const OFF_ITEM_PARAM_FLOAT_ORDINARY: usize = 0x1602ACC;
const OFF_ITEM_PARAM_INT_ORDINARY: usize = 0x1602C28;
const OFF_ITEM_REMOVE_REQUEST_FOUND: usize = 0x15CA9D0;
const OFF_ITEM_DEACTIVATE: usize = 0x15D4570;
const OFF_ITEM_RESOURCE_INIT: usize = 0x1607FC0;
const OFF_ITEM_RESOURCE_RELEASE: usize = 0x160A650;
const OFF_ITEM_RESOURCE_IDENTITY: usize = 0x16082C0;
const OFF_ITEM_RESOURCE_GAME_GROUP: usize = 0x16086F8;
const OFF_ITEM_RESOURCE_EFFECT_GROUP: usize = 0x160870C;
const OFF_ITEM_RESOURCE_SOUND_GROUP: usize = 0x1608720;

const ITEM_NRO_DISPATCHER: usize = 0x480;
const ITEM_NRO_DISPATCHER_WORDS: [u32; 5] =
    [0xFC190FE8, 0xA9016FFC, 0xA90267FA, 0xA9035FF8, 0xA90457F6];

const MAIN_PREFLIGHT: &[(usize, &[u32])] = &[
    (
        OFF_ITEM_LOWER_CREATOR,
        &[0xD103C3FF, 0xA9096FFC, 0xA90A67FA, 0xA90B5FF8, 0xA90C57F6],
    ),
    (OFF_ITEM_PARAM_FLOAT_ORDINARY, &[0xF9400008]),
    (OFF_ITEM_PARAM_INT_ORDINARY, &[0xF9400008]),
    (OFF_ITEM_REMOVE_REQUEST_FOUND, &[0xF9400109]),
    (
        OFF_ITEM_DEACTIVATE,
        &[0xD101C3FF, 0xF9000BFB, 0xA90267FA, 0xA9035FF8, 0xA90457F6],
    ),
    (
        OFF_ITEM_RESOURCE_INIT,
        &[0xD10783FF, 0xA9186FFC, 0xA91967FA, 0xA91A5FF8, 0xA91B57F6],
    ),
    (
        OFF_ITEM_RESOURCE_RELEASE,
        &[0xA9BC5FF8, 0xA90157F6, 0xA9024FF4, 0xA9037BFD, 0x9100C3FD],
    ),
    (OFF_ITEM_RESOURCE_IDENTITY, &[0xF9400134]),
    (OFF_ITEM_RESOURCE_GAME_GROUP, &[0x9484794E]),
    (OFF_ITEM_RESOURCE_EFFECT_GROUP, &[0x94847949]),
    (OFF_ITEM_RESOURCE_SOUND_GROUP, &[0x94847944]),
];

static CREATE_LOGS: AtomicU32 = AtomicU32::new(0);
static PARAM_FLOAT_LOGS: AtomicU32 = AtomicU32::new(0);
static PARAM_INT_LOGS: AtomicU32 = AtomicU32::new(0);
static REMOVE_LOGS: AtomicU32 = AtomicU32::new(0);
static DEACTIVATE_LOGS: AtomicU32 = AtomicU32::new(0);
static RESOURCE_INIT_LOGS: AtomicU32 = AtomicU32::new(0);
static RESOURCE_RELEASE_LOGS: AtomicU32 = AtomicU32::new(0);
static RESOURCE_IDENTITY_LOGS: AtomicU32 = AtomicU32::new(0);
static RESOURCE_GROUP_LOGS: AtomicU32 = AtomicU32::new(0);
static AGENT_LOGS: AtomicU32 = AtomicU32::new(0);
static ITEM_NRO_BASE: AtomicUsize = AtomicUsize::new(0);
static ITEM_AGENT_ORIGINAL: AtomicUsize = AtomicUsize::new(0);

fn log(message: String) {
    crate::dbg_log_public(&message);
}

unsafe fn read_i32(base: *const u8, offset: usize) -> i32 {
    (base.add(offset) as *const i32).read_unaligned()
}

unsafe fn read_u32(base: *const u8, offset: usize) -> u32 {
    (base.add(offset) as *const u32).read_unaligned()
}

unsafe fn read_u8(base: *const u8, offset: usize) -> u8 {
    base.add(offset).read_unaligned()
}

unsafe fn read_usize(base: *const u8, offset: usize) -> usize {
    (base.add(offset) as *const usize).read_unaligned()
}

unsafe fn words_match(address: usize, expected: &[u32]) -> bool {
    expected
        .iter()
        .enumerate()
        .all(|(index, word)| (address as *const u32).add(index).read() == *word)
}

unsafe fn main_preflight() -> Result<(), (usize, usize, u32, u32)> {
    let text = skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as usize;
    for (offset, expected) in MAIN_PREFLIGHT {
        for (index, wanted) in expected.iter().enumerate() {
            let address = text + *offset + index * 4;
            let found = (address as *const u32).read();
            if found != *wanted {
                return Err((address, index, found, *wanted));
            }
        }
    }
    Ok(())
}

#[skyline::hook(offset = OFF_ITEM_LOWER_CREATOR)]
unsafe fn item_lower_creator_trace(
    manager: *mut u8,
    descriptor: *const u8,
    creator_flag: i32,
    arg3: i32,
    arg4: i32,
) -> *mut u8 {
    let kind = if descriptor.is_null() {
        -1
    } else {
        read_i32(descriptor, 0x20)
    };
    let tracked = kind == BEAM_SWORD_KIND;
    let sequence = if tracked {
        CREATE_LOGS.fetch_add(1, Ordering::Relaxed)
    } else {
        u32::MAX
    };

    if sequence < 48 {
        let variant = if descriptor.is_null() {
            u32::MAX
        } else {
            read_u32(descriptor, 0x28)
        };
        log(format!(
            "[itemre] create-enter #{sequence} manager={manager:p} descriptor={descriptor:p} kind={kind:#x} creator_flag={creator_flag} arg3={arg3} arg4={arg4} descriptor_variant={variant:#x}"
        ));
    }

    let item = call_original!(manager, descriptor, creator_flag, arg3, arg4);

    if sequence < 48 {
        if item.is_null() {
            log(format!("[itemre] create-exit #{sequence} item=NULL"));
        } else {
            let id = read_u32(item, 0x08);
            let stored_kind = read_i32(item, 0x0C);
            let variant = read_u32(item, 0x258);
            let vtable = read_usize(item, 0);
            let remove = if vtable == 0 {
                0
            } else {
                read_usize(vtable as *const u8, 0x520)
            };
            log(format!(
                "[itemre] create-exit #{sequence} item={item:p} id={id:#x} stored_kind={stored_kind:#x} variant={variant:#x} vtable={vtable:#x} remove_vfunc={remove:#x}"
            ));
        }
    }
    item
}

unsafe fn trace_param(ctx: &skyline::hooks::InlineCtx, counter: &AtomicU32, value_type: &str) {
    let kind = ctx.registers[1].x() as u32 as i32;
    if kind != BEAM_SWORD_KIND {
        return;
    }
    let sequence = counter.fetch_add(1, Ordering::Relaxed);
    if sequence < 64 {
        log(format!(
            "[itemre] param-{value_type} #{sequence} accessor={:#x} kind={kind:#x} key={:#x}",
            ctx.registers[0].x(),
            ctx.registers[2].x()
        ));
    }
}

#[skyline::hook(offset = OFF_ITEM_PARAM_FLOAT_ORDINARY, inline)]
unsafe fn item_param_float_trace(ctx: &mut skyline::hooks::InlineCtx) {
    trace_param(ctx, &PARAM_FLOAT_LOGS, "float");
}

#[skyline::hook(offset = OFF_ITEM_PARAM_INT_ORDINARY, inline)]
unsafe fn item_param_int_trace(ctx: &mut skyline::hooks::InlineCtx) {
    trace_param(ctx, &PARAM_INT_LOGS, "int");
}

#[skyline::hook(offset = OFF_ITEM_REMOVE_REQUEST_FOUND, inline)]
unsafe fn item_remove_request_trace(ctx: &mut skyline::hooks::InlineCtx) {
    let item = ctx.registers[8].x() as *const u8;
    if item.is_null() || read_i32(item, 0x0C) != BEAM_SWORD_KIND {
        return;
    }
    let sequence = REMOVE_LOGS.fetch_add(1, Ordering::Relaxed);
    if sequence >= 48 {
        return;
    }
    let vtable = read_usize(item, 0);
    let remove = if vtable == 0 {
        0
    } else {
        read_usize(vtable as *const u8, 0x520)
    };
    log(format!(
        "[itemre] remove-request #{sequence} item={item:p} id={:#x} stored_id={:#x} vtable={vtable:#x} remove_vfunc={remove:#x}",
        ctx.registers[0].x() as u32,
        read_u32(item, 0x08)
    ));
}

#[skyline::hook(offset = OFF_ITEM_DEACTIVATE)]
unsafe fn item_deactivate_trace(manager: *mut u8, item: *mut u8, recycle: u32) {
    let tracked = !item.is_null() && read_i32(item, 0x0C) == BEAM_SWORD_KIND;
    let sequence = if tracked {
        DEACTIVATE_LOGS.fetch_add(1, Ordering::Relaxed)
    } else {
        u32::MAX
    };
    let id = if tracked { read_u32(item, 0x08) } else { 0 };
    let state = if tracked {
        read_usize(item, 0x90) as *const u8
    } else {
        core::ptr::null()
    };

    if sequence < 48 {
        let flags = if state.is_null() {
            [0; 4]
        } else {
            [
                read_u8(state, 0x18),
                read_u8(state, 0x19),
                read_u8(state, 0x1A),
                read_u8(state, 0x1B),
            ]
        };
        log(format!(
            "[itemre] deactivate-enter #{sequence} manager={manager:p} item={item:p} id={id:#x} kind={BEAM_SWORD_KIND:#x} recycle={recycle} state={state:p} flags={:02x}/{:02x}/{:02x}/{:02x}",
            flags[0], flags[1], flags[2], flags[3]
        ));
    }

    call_original!(manager, item, recycle);

    if sequence < 48 {
        log(format!(
            "[itemre] deactivate-exit #{sequence} item={item:p} id={id:#x}"
        ));
    }
}

#[skyline::hook(offset = OFF_ITEM_RESOURCE_INIT)]
unsafe fn item_resource_init_trace(slot: *mut u8, arg: u32) {
    let kind = if slot.is_null() {
        -1
    } else {
        read_i32(slot, 0x04)
    };
    let sequence = if kind == BEAM_SWORD_KIND {
        RESOURCE_INIT_LOGS.fetch_add(1, Ordering::Relaxed)
    } else {
        u32::MAX
    };
    if sequence < 32 {
        log(format!(
            "[itemre] resource-init-enter #{sequence} slot={slot:p} kind={kind:#x} arg={arg} refs={} flags={:02x}/{:02x}/{:02x}/{:02x}/{:02x}/{:02x}",
            read_u32(slot, 0),
            read_u8(slot, 0x48),
            read_u8(slot, 0x49),
            read_u8(slot, 0x4A),
            read_u8(slot, 0x4B),
            read_u8(slot, 0x4C),
            read_u8(slot, 0x4D)
        ));
    }

    call_original!(slot, arg);

    if sequence < 32 {
        log(format!(
            "[itemre] resource-init-exit #{sequence} slot={slot:p} refs={} flags={:02x}/{:02x}/{:02x}/{:02x}/{:02x}/{:02x}",
            read_u32(slot, 0),
            read_u8(slot, 0x48),
            read_u8(slot, 0x49),
            read_u8(slot, 0x4A),
            read_u8(slot, 0x4B),
            read_u8(slot, 0x4C),
            read_u8(slot, 0x4D)
        ));
    }
}

#[skyline::hook(offset = OFF_ITEM_RESOURCE_RELEASE)]
unsafe fn item_resource_release_trace(manager: *mut u8, kind: i32) {
    let sequence = if kind == BEAM_SWORD_KIND {
        RESOURCE_RELEASE_LOGS.fetch_add(1, Ordering::Relaxed)
    } else {
        u32::MAX
    };
    if sequence < 32 {
        log(format!(
            "[itemre] resource-release-enter #{sequence} manager={manager:p} kind={kind:#x}"
        ));
    }
    call_original!(manager, kind);
    if sequence < 32 {
        log(format!(
            "[itemre] resource-release-exit #{sequence} kind={kind:#x}"
        ));
    }
}

#[skyline::hook(offset = OFF_ITEM_RESOURCE_IDENTITY, inline)]
unsafe fn item_resource_identity_trace(ctx: &mut skyline::hooks::InlineCtx) {
    let slot = ctx.registers[19].x() as *const u8;
    let row = ctx.registers[9].x() as *const u8;
    if slot.is_null() || row.is_null() || read_i32(slot, 0x04) != BEAM_SWORD_KIND {
        return;
    }
    let sequence = RESOURCE_IDENTITY_LOGS.fetch_add(1, Ordering::Relaxed);
    if sequence < 32 {
        log(format!(
            "[itemre] resource-identity #{sequence} slot={slot:p} kind={:#x} name_hash={:#x}",
            read_i32(slot, 0x04),
            read_usize(row, 0)
        ));
    }
}

unsafe fn trace_resource_group(ctx: &skyline::hooks::InlineCtx, group: &str) {
    let slot = ctx.registers[19].x() as *const u8;
    if slot.is_null() || read_i32(slot, 0x04) != BEAM_SWORD_KIND {
        return;
    }
    let sequence = RESOURCE_GROUP_LOGS.fetch_add(1, Ordering::Relaxed);
    if sequence < 48 {
        log(format!(
            "[itemre] resource-group #{sequence} slot={slot:p} kind={:#x} group={group} prefix={:#x} storage={:#x}",
            read_i32(slot, 0x04),
            ctx.registers[1].x(),
            ctx.registers[2].x()
        ));
    }
}

#[skyline::hook(offset = OFF_ITEM_RESOURCE_GAME_GROUP, inline)]
unsafe fn item_resource_game_group_trace(ctx: &mut skyline::hooks::InlineCtx) {
    trace_resource_group(ctx, "game_");
}

#[skyline::hook(offset = OFF_ITEM_RESOURCE_EFFECT_GROUP, inline)]
unsafe fn item_resource_effect_group_trace(ctx: &mut skyline::hooks::InlineCtx) {
    trace_resource_group(ctx, "effect_");
}

#[skyline::hook(offset = OFF_ITEM_RESOURCE_SOUND_GROUP, inline)]
unsafe fn item_resource_sound_group_trace(ctx: &mut skyline::hooks::InlineCtx) {
    trace_resource_group(ctx, "sound_");
}

type ItemAgentDispatcher =
    unsafe extern "C" fn(Hash40, *mut u8, *mut u8, *mut libc::c_void) -> *mut u8;

unsafe extern "C" fn item_agent_dispatch_trace(
    agent: Hash40,
    object: *mut u8,
    boma: *mut u8,
    lua_state: *mut libc::c_void,
) -> *mut u8 {
    let kind = if object.is_null() {
        -1
    } else {
        read_i32(object, 0x0C)
    };
    if kind == BEAM_SWORD_KIND || agent.hash == BEAM_SWORD_KIND_HASH {
        let sequence = AGENT_LOGS.fetch_add(1, Ordering::Relaxed);
        if sequence < 48 {
            log(format!(
                "[itemre] agent #{sequence} hash={:#x} object={object:p} boma={boma:p} kind={kind:#x} lua={lua_state:p}",
                agent.hash
            ));
        }
    }

    let original = ITEM_AGENT_ORIGINAL.load(Ordering::Acquire);
    if original == 0 {
        log("[itemre] FATAL: item-agent trampoline is null".to_string());
        return core::ptr::null_mut();
    }
    let original: ItemAgentDispatcher = core::mem::transmute(original);
    original(agent, object, boma, lua_state)
}

fn item_nro_hook(info: &skyline::nro::NroInfo) {
    if info.name != "item" {
        return;
    }
    unsafe {
        let module_object = info.module.ModuleObject;
        if module_object.is_null() {
            log("[itemre] item NRO event has no ModuleObject; agent trace inert".to_string());
            return;
        }
        let base = (*module_object).module_base as usize;
        let address = base + ITEM_NRO_DISPATCHER;
        if !words_match(address, &ITEM_NRO_DISPATCHER_WORDS) {
            log(format!(
                "[itemre] item NRO dispatcher preflight failed at {address:#x}; agent trace inert"
            ));
            return;
        }
        if ITEM_NRO_BASE.load(Ordering::Acquire) == base
            && ITEM_AGENT_ORIGINAL.load(Ordering::Acquire) != 0
        {
            return;
        }

        let mut original: *mut libc::c_void = core::ptr::null_mut();
        skyline::hooks::A64HookFunction(
            address as *const libc::c_void,
            item_agent_dispatch_trace as *const () as *const libc::c_void,
            &mut original,
        );
        if original.is_null() {
            log(format!(
                "[itemre] item NRO dispatcher hook failed at {address:#x}"
            ));
            return;
        }
        ITEM_AGENT_ORIGINAL.store(original as usize, Ordering::Release);
        ITEM_NRO_BASE.store(base, Ordering::Release);
        log(format!(
            "[itemre] item status-agent dispatcher armed at {address:#x} (base={base:#x})"
        ));
    }
}

pub fn install() {
    unsafe {
        if let Err((address, index, found, expected)) = main_preflight() {
            log(format!(
                "[itemre] main preflight failed at {address:#x} word={index}: found={found:#010x} expected={expected:#010x}; all item traces inert"
            ));
            return;
        }
        skyline::install_hooks!(
            item_lower_creator_trace,
            item_param_float_trace,
            item_param_int_trace,
            item_remove_request_trace,
            item_deactivate_trace,
            item_resource_init_trace,
            item_resource_release_trace,
            item_resource_identity_trace,
            item_resource_game_group_trace,
            item_resource_effect_group_trace,
            item_resource_sound_group_trace
        );
    }

    match skyline::nro::add_hook(item_nro_hook) {
        Ok(()) => log(
            "[itemre] observation-only Beam Sword lifecycle tracer armed; no values are modified"
                .to_string(),
        ),
        Err(_) => log(
            "[itemre] main traces armed, but libnro_hook is unavailable; item-agent trace inert"
                .to_string(),
        ),
    }
}
