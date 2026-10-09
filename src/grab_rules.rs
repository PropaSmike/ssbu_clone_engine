use clone_engine_api::{
    CloneGrabRuleV1, API_VERSION_V1, ERROR_CUSTOM_KIND, ERROR_NAME, ERROR_NULL, ERROR_STRUCT_SIZE,
    ERROR_UNSUPPORTED, ERROR_VERSION, GRAB_CATCH, GRAB_SET_THEN, GRAB_SET_VICTIM,
};
use clone_engine_core::grab_rule::{
    answer, is_capture, status_label, GrabRule, Reply, DEFAULT_NODE, EVENT_CONSTRAINT,
    EVENT_CONSTRAINT_OFFSET, EVENT_IS_MOTION_HI_LW, EVENT_KIND, EVENT_MOTION_OFFSET,
    EVENT_MOTION_OFFSET_LW, EVENT_NODE, EVENT_RESULT, EVENT_SENDER_ID, EVENT_STATUS, MAX_RULES,
    ON_LINK_EVENT_SLOT, WORK_FLAG_CATCH_MOTION_HI_LW, WORK_INT_CATCH_MOTION_OFFSET,
    WORK_INT_CATCH_MOTION_OFFSET_LW,
};
use clone_engine_core::{clone_row, CLONE_SLOTS};
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use smash::app::lua_bind::{CatchModule, StatusModule, WorkModule};
use smash::app::BattleObjectModuleAccessor;
use std::ffi::CStr;
use std::sync::Mutex;

type LinkEventFn = unsafe extern "C" fn(usize, usize, usize) -> u64;

#[allow(clippy::declare_interior_mutable_const)]
const NO_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
#[allow(clippy::declare_interior_mutable_const)]
const NOT_INSTALLED: AtomicBool = AtomicBool::new(false);

static RULES: Mutex<Vec<(i32, GrabRule)>> = Mutex::new(Vec::new());
static ORIGINALS: [AtomicUsize; CLONE_SLOTS] = [NO_ORIGINAL; CLONE_SLOTS];
static INSTALLED: [AtomicBool; CLONE_SLOTS] = [NOT_INSTALLED; CLONE_SLOTS];
static ANSWER_LOGS: AtomicU32 = AtomicU32::new(0);
static MISS_LOGS: AtomicU32 = AtomicU32::new(0);
static ORPHAN_LOGS: AtomicU32 = AtomicU32::new(0);

unsafe fn read<T: Copy>(address: usize) -> T {
    core::ptr::read_unaligned(address as *const T)
}

unsafe fn write<T>(address: usize, value: T) {
    core::ptr::write_unaligned(address as *mut T, value);
}

unsafe fn grabber_kind(class: usize, object: usize) -> Option<i32> {
    crate::clone_vtables::fighter_kind_of_class(class)
        .or_else(|| crate::clone_kind_of_object(object as u64))
}

unsafe fn rule_reply(kind: i32, status: i32, victim: i32) -> Result<Reply, bool> {
    let Ok(held) = RULES.lock() else {
        return Err(false);
    };
    let owned = held.iter().filter(|(owner, _)| *owner == kind).map(|(_, rule)| rule);
    match answer(owned.clone(), status, victim) {
        Some(reply) => Ok(reply),
        None => Err(owned.clone().any(|rule| rule.status == status)),
    }
}

unsafe fn answer_capture(kind: i32, object: usize, event: usize) -> bool {
    let boma = read::<*mut BattleObjectModuleAccessor>(object + 0x20);
    if boma.is_null() {
        return false;
    }
    let status = StatusModule::status_kind(boma);
    let victim = read::<i32>(event + EVENT_STATUS);
    let reply = match rule_reply(kind, status, victim) {
        Ok(reply) => reply,
        Err(near_miss) => {
            if near_miss && MISS_LOGS.fetch_add(1, Ordering::Relaxed) < 8 {
                skyline::println!(
                    "[grab] kind {kind}: a {} capture in {} matched no rule's victim; the base answers",
                    status_label(victim),
                    status_label(status)
                );
            }
            return false;
        }
    };
    let sender = read::<u32>(event + EVENT_SENDER_ID);
    write::<u8>(event + EVENT_RESULT, 1);
    match reply.node {
        Some(node) => {
            write::<u64>(event + EVENT_NODE, node);
            write::<u8>(event + EVENT_CONSTRAINT, 1);
        }
        None => write::<u8>(event + EVENT_CONSTRAINT, 0),
    }
    for (index, value) in reply.offset.iter().enumerate() {
        write::<f32>(event + EVENT_CONSTRAINT_OFFSET + index * 4, *value);
    }
    if reply.catch_motion {
        write::<i32>(event + EVENT_MOTION_OFFSET, WorkModule::get_int(boma, WORK_INT_CATCH_MOTION_OFFSET));
        write::<i32>(
            event + EVENT_MOTION_OFFSET_LW,
            WorkModule::get_int(boma, WORK_INT_CATCH_MOTION_OFFSET_LW),
        );
        write::<u8>(
            event + EVENT_IS_MOTION_HI_LW,
            u8::from(WorkModule::is_flag(boma, WORK_FLAG_CATCH_MOTION_HI_LW)),
        );
    }
    if reply.catch {
        CatchModule::set_catch(boma, sender);
    }
    if let Some(then) = reply.then {
        StatusModule::change_status_request(boma, then, false);
    }
    if ANSWER_LOGS.fetch_add(1, Ordering::Relaxed) < 32 {
        skyline::println!(
            "[grab] kind {kind}: caught object {sender:#x} into {} from {}{}",
            status_label(victim),
            status_label(status),
            reply
                .then
                .map_or_else(String::new, |then| format!(", requested {}", status_label(then)))
        );
    }
    true
}

unsafe extern "C" fn grab_link_event(class: usize, object: usize, event: usize) -> u64 {
    let kind = grabber_kind(class, object);
    if let Some(kind) = kind {
        if event != 0 && is_capture(read::<u64>(event + EVENT_KIND)) && answer_capture(kind, object, event) {
            return 0;
        }
    }
    let original = kind
        .and_then(clone_row)
        .map_or(0, |row| ORIGINALS[row].load(Ordering::Acquire));
    if original == 0 {
        if ORPHAN_LOGS.fetch_add(1, Ordering::Relaxed) < 4 {
            skyline::println!(
                "[grab] a link event reached the grab entry for class {class:#x} with no original entry; returning the base default"
            );
        }
        return 1;
    }
    let original: LinkEventFn = core::mem::transmute(original);
    original(class, object, event)
}

unsafe fn install(kind: i32, row: usize) -> bool {
    if INSTALLED[row].swap(true, Ordering::AcqRel) {
        return true;
    }
    let installed = crate::clone_vtables::set_entry(
        kind,
        ON_LINK_EVENT_SLOT,
        crate::clone_vtables::Space::Fighter,
        grab_link_event as *const () as usize,
        &ORIGINALS[row] as *const AtomicUsize as usize,
    )
    .is_some();
    if !installed {
        INSTALLED[row].store(false, Ordering::Release);
    }
    installed
}

unsafe fn rule_from(registration: &CloneGrabRuleV1) -> Result<GrabRule, i32> {
    if registration.status < 0 {
        return Err(ERROR_UNSUPPORTED);
    }
    let mut rule = GrabRule::new(registration.status);
    if registration.present & GRAB_SET_VICTIM != 0 {
        rule.victim = Some(registration.victim);
    }
    if registration.present & GRAB_SET_THEN != 0 {
        rule.then = Some(registration.then);
    }
    rule.catch = registration.present & GRAB_CATCH != 0;
    rule.offset = registration.offset;
    rule.node = Some(if registration.node.is_null() {
        DEFAULT_NODE.to_string()
    } else {
        CStr::from_ptr(registration.node)
            .to_str()
            .map_err(|_| ERROR_NAME)?
            .to_string()
    });
    Ok(rule)
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_grab_rule_v1(registration: *const CloneGrabRuleV1) -> i32 {
    let Some(registration) = registration.as_ref() else {
        return ERROR_NULL;
    };
    if registration.api_version != API_VERSION_V1 {
        return ERROR_VERSION;
    }
    if (registration.struct_size as usize) < core::mem::size_of::<CloneGrabRuleV1>() {
        return ERROR_STRUCT_SIZE;
    }
    let kind = registration.fighter_kind;
    let (Some(row), Some(_)) = (clone_row(kind), crate::clone_base(kind)) else {
        skyline::println!("[grab] REFUSED a rule for kind {kind}: it is not a registered clone");
        return ERROR_CUSTOM_KIND;
    };
    let rule = match rule_from(registration) {
        Ok(rule) => rule,
        Err(code) => {
            skyline::println!("[grab] REFUSED a rule for kind {kind}: status or node is invalid");
            return code;
        }
    };
    let description = rule.describe();
    {
        let Ok(mut held) = RULES.lock() else {
            return ERROR_UNSUPPORTED;
        };
        if held.len() >= MAX_RULES {
            skyline::println!("[grab] REFUSED kind {kind} {description}: {MAX_RULES} rules are registered");
            return ERROR_UNSUPPORTED;
        }
        held.push((kind, rule));
    }
    if !install(kind, row) {
        if let Ok(mut held) = RULES.lock() {
            held.retain(|(owner, _)| *owner != kind);
        }
        skyline::println!("[grab] REFUSED kind {kind} {description}: its private vtable entry could not be written");
        return ERROR_UNSUPPORTED;
    }
    skyline::println!("[grab] kind {kind}: {description}");
    0
}
