#![allow(dead_code)]

pub(crate) use clone_engine_core::motion_append::MotionRecord as CopyMotionRecord;
use clone_engine_core::motion_append::{apply_copy, Rejected, BYTES_PER_MOTION};
use clone_engine_core::motion_list::{hash40, parse, serialize};
use std::collections::HashSet;
use std::sync::{OnceLock, RwLock};

pub(crate) const KIRBY_MOTION_LIST_PATHS: [&str; 8] = [
    "fighter/kirby/motion/body/c00/motion_list.bin",
    "fighter/kirby/motion/body/c01/motion_list.bin",
    "fighter/kirby/motion/body/c02/motion_list.bin",
    "fighter/kirby/motion/body/c03/motion_list.bin",
    "fighter/kirby/motion/body/c04/motion_list.bin",
    "fighter/kirby/motion/body/c05/motion_list.bin",
    "fighter/kirby/motion/body/c06/motion_list.bin",
    "fighter/kirby/motion/body/c07/motion_list.bin",
];

pub(crate) const VANILLA_LIST_BYTES: usize = 99_580;
pub(crate) const MAX_MOTIONS: usize = 1024;

pub(crate) const fn callback_capacity() -> usize {
    VANILLA_LIST_BYTES + BYTES_PER_MOTION * MAX_MOTIONS
}

pub(crate) fn rebuild(original: &[u8], records: &[CopyMotionRecord]) -> Option<Vec<u8>> {
    let mut list = parse(original).ok()?;
    let outcome = apply_copy(&mut list, records);
    let bytes = serialize(&list).ok()?;
    note_outcome(&outcome);
    log_outcome(&outcome, list.entries.len());
    Some(bytes)
}

fn refused() -> &'static RwLock<HashSet<String>> {
    static REFUSED: OnceLock<RwLock<HashSet<String>>> = OnceLock::new();
    REFUSED.get_or_init(|| RwLock::new(HashSet::new()))
}

fn note_outcome(outcome: &[(String, Result<(), Rejected>)]) {
    let Ok(mut held) = refused().write() else {
        return;
    };
    for (name, result) in outcome {
        if result.is_err() {
            held.insert(name.clone());
        } else {
            held.remove(name);
        }
    }
}

pub(crate) fn refused_names() -> HashSet<String> {
    refused().read().map(|held| held.clone()).unwrap_or_default()
}

fn records() -> &'static RwLock<Vec<CopyMotionRecord>> {
    static RECORDS: OnceLock<RwLock<Vec<CopyMotionRecord>>> = OnceLock::new();
    RECORDS.get_or_init(|| RwLock::new(Vec::new()))
}

pub(crate) fn registered() -> Vec<CopyMotionRecord> {
    records().read().map(|held| held.clone()).unwrap_or_default()
}

pub(crate) fn motion_hashes(fighter_kind: i32) -> Vec<u64> {
    let refused = refused_names();
    records()
        .read()
        .map(|held| {
            held.iter()
                .filter(|known| known.fighter_kind == fighter_kind)
                .filter(|known| !refused.contains(&known.name))
                .filter_map(|known| hash40(&known.name))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn owns_motions(fighter_kind: i32) -> bool {
    records()
        .read()
        .map(|held| held.iter().any(|known| known.fighter_kind == fighter_kind))
        .unwrap_or(false)
}

pub(crate) fn record(entry: CopyMotionRecord) -> Result<(), &'static str> {
    let Ok(mut held) = records().write() else {
        return Err("registry poisoned");
    };
    if held.len() >= MAX_MOTIONS {
        return Err("copy motion capacity reached");
    }
    if held.iter().any(|known| known.name == entry.name) {
        return Err("a copy motion with that name is already registered");
    }
    held.push(entry);
    Ok(())
}

#[cfg(test)]
fn log_outcome(_outcome: &[(String, Result<(), Rejected>)], _total: usize) {}

#[cfg(not(test))]
fn log_outcome(outcome: &[(String, Result<(), Rejected>)], total: usize) {
    for (name, result) in outcome {
        match result {
            Ok(()) => skyline::println!("[kirbymotion] added '{name}' to Kirby's motion list"),
            Err(reason) => skyline::println!(
                "[kirbymotion] REFUSED '{name}': {reason:?}; the copy falls back to its base fighter's motion"
            ),
        }
    }
    skyline::println!("[kirbymotion] Kirby's motion list now holds {total} entries");
}

#[cfg(not(test))]
pub(crate) use live::{clone_engine_clone_copy_motion_v1, motion_record};

#[cfg(not(test))]
mod live {
    use super::*;
    use clone_engine_api::{
        CloneCopyMotionV1, API_VERSION_V1, COPY_MOTION_NO_EXTRA, COPY_MOTION_SET_ANIMATION_UNK,
        COPY_MOTION_SET_BLEND_FRAMES, COPY_MOTION_SET_CANCEL_FRAME, COPY_MOTION_SET_FLAGS,
        COPY_MOTION_SET_GAME_SCRIPT, COPY_MOTION_SET_NO_STOP_INTP, COPY_MOTION_SET_SCRIPTS,
        COPY_MOTION_SET_XLU, COPY_MOTION_VICTIM, ERROR_NAME, ERROR_NULL, ERROR_STRUCT_SIZE,
        ERROR_UNSUPPORTED, ERROR_VERSION,
    };
    use std::ffi::CStr;
    use std::os::raw::c_char;
    use std::sync::atomic::{AtomicBool, Ordering};

    static INSTALLED: AtomicBool = AtomicBool::new(false);

    unsafe fn text(pointer: *const c_char) -> Option<String> {
        if pointer.is_null() {
            return None;
        }
        CStr::from_ptr(pointer).to_str().ok().map(str::to_string)
    }

    pub(crate) unsafe fn motion_record(
        registration: *const CloneCopyMotionV1,
        animation_required: bool,
    ) -> Result<CopyMotionRecord, i32> {
        if registration.is_null() {
            return Err(ERROR_NULL);
        }
        let registration = &*registration;
        if registration.api_version != API_VERSION_V1 {
            return Err(ERROR_VERSION);
        }
        if registration.struct_size < core::mem::size_of::<CloneCopyMotionV1>() as u32 {
            return Err(ERROR_STRUCT_SIZE);
        }
        let Some(name) = text(registration.name).filter(|name| !name.is_empty()) else {
            return Err(ERROR_NAME);
        };
        let animation = text(registration.animation).unwrap_or_default();
        if animation_required && animation.is_empty() {
            return Err(ERROR_NAME);
        }

        let present = registration.present;
        let mut entry = CopyMotionRecord::new(registration.fighter_kind, &name, &animation);
        entry.template = text(registration.template);
        if present & COPY_MOTION_SET_GAME_SCRIPT != 0 {
            entry.game_script = text(registration.game_script);
        }
        if present & COPY_MOTION_SET_SCRIPTS != 0 {
            let (Some(a), Some(b), Some(c)) = (
                text(registration.script_expression),
                text(registration.script_sound),
                text(registration.script_effect),
            ) else {
                return Err(ERROR_NAME);
            };
            entry.scripts = Some([a, b, c]);
        }
        if present & COPY_MOTION_SET_FLAGS != 0 {
            entry.flags = Some(registration.motion_flags);
        }
        if present & COPY_MOTION_SET_BLEND_FRAMES != 0 {
            entry.blend_frames = Some(registration.blend_frames);
        }
        if present & COPY_MOTION_SET_XLU != 0 {
            entry.xlu = Some((registration.xlu_start, registration.xlu_end));
        }
        if present & COPY_MOTION_SET_CANCEL_FRAME != 0 {
            entry.cancel_frame = Some(registration.cancel_frame);
        }
        if present & COPY_MOTION_SET_NO_STOP_INTP != 0 {
            entry.no_stop_intp = Some(registration.no_stop_intp != 0);
        }
        if present & COPY_MOTION_SET_ANIMATION_UNK != 0 {
            entry.animation_unk = Some(registration.animation_unk);
        }
        entry.no_extra = present & COPY_MOTION_NO_EXTRA != 0;
        entry.victim = present & COPY_MOTION_VICTIM != 0;
        Ok(entry)
    }

    #[no_mangle]
    pub unsafe extern "C" fn clone_engine_clone_copy_motion_v1(
        registration: *const CloneCopyMotionV1,
    ) -> i32 {
        let entry = match motion_record(registration, true) {
            Ok(entry) => entry,
            Err(code) => return code,
        };
        if entry.victim {
            skyline::println!("[kirbymotion] REFUSED '{}': victim motions are global motions", entry.name);
            return ERROR_UNSUPPORTED;
        }
        let (kind, name, animation) = (entry.fighter_kind, entry.name.clone(), entry.animation.clone());
        match record(entry) {
            Ok(()) => {
                skyline::println!("[kirbymotion] kind {kind} registered '{name}' -> {animation}");
                0
            }
            Err(reason) => {
                skyline::println!("[kirbymotion] REFUSED '{name}': {reason}");
                ERROR_UNSUPPORTED
            }
        }
    }

    extern "C" fn serve(hash: u64, buffer: *mut u8, length: usize, out_size: &mut usize) -> bool {
        if buffer.is_null() || length == 0 {
            return false;
        }
        let entries = registered();
        if entries.is_empty() {
            return false;
        }
        let mut original = vec![0u8; length];
        let Some(read) = arcropolis_api::load_original_file(hash, original.as_mut_slice()) else {
            skyline::println!(
                "[kirbymotion] could not read Kirby's original motion list; leaving it alone"
            );
            return false;
        };
        original.truncate(read);
        let Some(rebuilt) = rebuild(&original, &entries) else {
            skyline::println!("[kirbymotion] Kirby's motion list did not parse; leaving it alone");
            return false;
        };
        if rebuilt.len() > length {
            skyline::println!(
                "[kirbymotion] rebuilt list is {} bytes but only {length} were reserved; leaving it alone",
                rebuilt.len()
            );
            return false;
        }
        unsafe { core::ptr::copy_nonoverlapping(rebuilt.as_ptr(), buffer, rebuilt.len()) };
        *out_size = rebuilt.len();
        true
    }

    pub(crate) fn install() {
        if INSTALLED.swap(true, Ordering::AcqRel) {
            return;
        }
        for path in KIRBY_MOTION_LIST_PATHS {
            arcropolis_api::register_callback(path, callback_capacity(), serve);
        }
        skyline::println!(
            "[kirbymotion] watching Kirby's {} motion lists; clones may register their own copy motions",
            KIRBY_MOTION_LIST_PATHS.len()
        );
    }
}

#[cfg(not(test))]
pub(crate) fn install() {
    live::install();
}

#[cfg(test)]
pub(crate) fn install() {}

#[cfg(test)]
mod tests {
    use super::*;
    use clone_engine_core::motion_list::{Animation, Extra, Motion, MotionList};

    fn empty_list() -> MotionList {
        MotionList {
            motion_path: hash40("fighter/kirby/motion/body/c00").unwrap(),
            entries: Vec::new(),
        }
    }

    fn list_with_template() -> MotionList {
        let mut list = empty_list();
        list.insert(
            hash40("donkey_special_n").unwrap(),
            Motion {
                game_script: hash40("game_donkeyspecialn").unwrap(),
                flags: 0,
                blend_frames: 3,
                animations: vec![Animation {
                    name: hash40("donkeyd00specialn.nuanmb").unwrap(),
                    unk: 1,
                }],
                scripts: vec![
                    hash40("expression_donkeyspecialn").unwrap(),
                    hash40("sound_donkeyspecialn").unwrap(),
                    hash40("effect_donkeyspecialn").unwrap(),
                ],
                extra: Some(Extra {
                    xlu_start: 0,
                    xlu_end: 0,
                    cancel_frame: 63,
                    no_stop_intp: false,
                }),
            },
        );
        list
    }

    #[test]
    fn rebuild_grows_the_file_and_keeps_it_parseable() {
        let list = list_with_template();
        let original = serialize(&list).unwrap();
        let mut entry =
            CopyMotionRecord::new(121, "bluster_special_n", "blusterd00specialn.nuanmb");
        entry.template = Some("donkey_special_n".to_string());

        let grown = rebuild(&original, std::slice::from_ref(&entry)).unwrap();
        assert!(grown.len() > original.len());
        let reparsed = parse(&grown).unwrap();
        assert_eq!(reparsed.entries.len(), list.entries.len() + 1);
        assert!(reparsed
            .get(hash40("bluster_special_n").unwrap())
            .is_some());
    }

    #[test]
    fn rebuild_refuses_a_file_it_cannot_parse() {
        let entry = CopyMotionRecord::new(121, "any_special_n", "anyd00specialn.nuanmb");
        assert!(rebuild(&[0u8; 64], std::slice::from_ref(&entry)).is_none());
    }

    #[test]
    fn the_reserved_buffer_covers_the_vanilla_list_plus_every_allowed_motion() {
        assert!(callback_capacity() > VANILLA_LIST_BYTES);
        assert_eq!(
            callback_capacity(),
            VANILLA_LIST_BYTES + BYTES_PER_MOTION * MAX_MOTIONS
        );
    }

    #[test]
    fn a_kind_owns_motions_only_once_it_has_registered_one() {
        assert!(!owns_motions(4242));
        assert_eq!(
            record(CopyMotionRecord::new(4242, "owned_special_n", "ownedd00specialn.nuanmb")),
            Ok(())
        );
        assert!(owns_motions(4242));
        assert!(!owns_motions(4243));
    }

    #[test]
    fn a_refused_motion_is_never_bound_to_the_copy() {
        let list = list_with_template();
        let original = serialize(&list).unwrap();

        let mut landed = CopyMotionRecord::new(4444, "bound_special_n", "boundd00specialn.nuanmb");
        landed.template = Some("donkey_special_n".to_string());
        let mut dropped =
            CopyMotionRecord::new(4444, "unbound_special_n", "unboundd00specialn.nuanmb");
        dropped.template = Some("nobody_special_n".to_string());
        assert_eq!(record(landed.clone()), Ok(()));
        assert_eq!(record(dropped.clone()), Ok(()));

        assert_eq!(motion_hashes(4444).len(), 2);

        assert!(rebuild(&original, &[landed, dropped]).is_some());

        assert_eq!(
            motion_hashes(4444),
            vec![hash40("bound_special_n").unwrap()]
        );
        assert!(refused_names().contains("unbound_special_n"));
    }

    #[test]
    fn the_registry_refuses_a_duplicate_name() {
        let entry = CopyMotionRecord::new(121, "dupe_special_n", "duped00specialn.nuanmb");
        assert_eq!(record(entry.clone()), Ok(()));
        assert!(record(entry).is_err());
    }
}
