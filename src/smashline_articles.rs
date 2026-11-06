use core::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const UNRESOLVED: usize = 1;
const SMASHLINE_2_MARKER: &[u8] = b"smashline_replace_fighter_vtable\0";
const CLONE_WEAPON: &[u8] = b"smashline_clone_weapon\0";
const INSTALL_STATUS: &[u8] = b"smashline_install_status_script\0";
const STATUS_LINES: i32 = 17;

#[repr(C)]
struct StringFfi {
    ptr: *mut u8,
    len: usize,
}

impl StringFfi {
    fn owned(bytes: &[u8]) -> Self {
        let leaked = Box::leak(bytes.to_vec().into_boxed_slice());
        Self {
            ptr: leaked.as_mut_ptr(),
            len: leaked.len(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CloneWeapon {
    generate_id: i32,
    weapon_id: i32,
}

type CloneWeaponFn = unsafe extern "C" fn(StringFfi, StringFfi, i32, bool) -> CloneWeapon;
type InstallStatusFn = unsafe extern "C" fn(u64, i32, i32, *const ());

static CLONE_WEAPON_FN: AtomicUsize = AtomicUsize::new(UNRESOLVED);
static INSTALL_STATUS_FN: AtomicUsize = AtomicUsize::new(UNRESOLVED);
static ANNOUNCED: Mutex<Vec<i32>> = Mutex::new(Vec::new());

pub(crate) struct SharedArticle {
    pub(crate) weapon_kind: i32,
    pub(crate) generate_id: i32,
}

unsafe fn symbol(name: &[u8]) -> usize {
    let mut address = 0usize;
    let result = skyline::nn::ro::LookupSymbol(&mut address, name.as_ptr());
    if result == 0 {
        address
    } else {
        0
    }
}

unsafe fn resolve() -> usize {
    if crate::game::version() != crate::game::GameVersion::V13_0_5 || symbol(SMASHLINE_2_MARKER) == 0 {
        return 0;
    }
    symbol(CLONE_WEAPON)
}

fn clone_weapon_fn() -> Option<CloneWeaponFn> {
    let mut address = CLONE_WEAPON_FN.load(Ordering::Acquire);
    if address == UNRESOLVED {
        address = unsafe { resolve() };
        CLONE_WEAPON_FN.store(address, Ordering::Release);
        skyline::println!(
            "[clone_engine] smashline 2 article numbering: {}",
            if address == 0 {
                "off; the engine numbers its own articles"
            } else {
                "on; articles take Smashline's weapon kinds and the engine keeps its own article work on top"
            }
        );
    }
    (address != 0).then(|| unsafe { core::mem::transmute::<usize, CloneWeaponFn>(address) })
}

fn install_status_fn() -> Option<InstallStatusFn> {
    let mut address = INSTALL_STATUS_FN.load(Ordering::Acquire);
    if address == UNRESOLVED {
        address = unsafe { symbol(INSTALL_STATUS) };
        INSTALL_STATUS_FN.store(address, Ordering::Release);
    }
    (address != 0).then(|| unsafe { core::mem::transmute::<usize, InstallStatusFn>(address) })
}

fn trimmed(bytes: &[u8]) -> Option<&str> {
    core::str::from_utf8(bytes.strip_suffix(&[0]).unwrap_or(bytes)).ok()
}

pub(crate) fn agent_name(weapon_kind: i32) -> Option<String> {
    let (owner, name) = crate::custom_articles::agent_names(weapon_kind)?;
    Some(format!("{}_{}", trimmed(owner)?, trimmed(name)?))
}

pub(crate) fn install_status(weapon_kind: i32, line: i32, status_kind: i32, function: *const ()) -> i32 {
    if !(0..STATUS_LINES).contains(&line) || function.is_null() {
        return dionysus_api::ERROR_ARTICLE_SOURCE;
    }
    let Some(agent) = agent_name(weapon_kind) else {
        return dionysus_api::ERROR_ARTICLE_SOURCE;
    };
    let Some(install) = install_status_fn() else {
        skyline::println!("[articlestatus] {agent}: Smashline is not loaded; status {status_kind} is not installed");
        return dionysus_api::ERROR_SMASHLINE_REQUIRED;
    };
    unsafe { install(dionysus_core::hash40(&agent), status_kind, line, function) };
    if let Ok(mut announced) = ANNOUNCED.lock() {
        if !announced.contains(&weapon_kind) {
            announced.push(weapon_kind);
            skyline::println!("[articlestatus] weapon kind {weapon_kind}: statuses go to Smashline on {agent}");
        }
    }
    dionysus_api::RESULT_OK
}

pub(crate) fn numbering_shared() -> bool {
    clone_weapon_fn().is_some()
}

pub(crate) unsafe fn clone_weapon(owner: &[u8], name: &[u8], source_weapon_kind: i32) -> Option<SharedArticle> {
    let function = clone_weapon_fn()?;
    let answer = function(
        StringFfi::owned(owner),
        StringFfi::owned(name),
        source_weapon_kind,
        false,
    );
    Some(SharedArticle {
        weapon_kind: answer.weapon_id,
        generate_id: answer.generate_id,
    })
}
