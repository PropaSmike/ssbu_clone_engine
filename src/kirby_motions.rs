#![allow(dead_code)]

use crate::motion_list::{hash40, parse, serialize, Animation, Motion, MotionList};
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
pub(crate) const BYTES_PER_MOTION: usize = 128;
pub(crate) const MAX_MOTIONS: usize = 256;

pub(crate) const fn callback_capacity() -> usize {
    VANILLA_LIST_BYTES + BYTES_PER_MOTION * MAX_MOTIONS
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CopyMotionRecord {
    pub fighter_kind: i32,
    pub name: String,
    pub animation: String,
    pub template: Option<String>,
    pub game_script: Option<String>,
    pub scripts: Option<[String; 3]>,
    pub flags: Option<u16>,
    pub blend_frames: Option<u8>,
    pub xlu: Option<(u8, u8)>,
    pub cancel_frame: Option<u8>,
    pub no_stop_intp: Option<bool>,
    pub animation_unk: Option<u8>,
    pub no_extra: bool,
}

impl CopyMotionRecord {
    pub(crate) fn new(fighter_kind: i32, name: &str, animation: &str) -> Self {
        Self {
            fighter_kind,
            name: name.to_string(),
            animation: animation.to_string(),
            template: None,
            game_script: None,
            scripts: None,
            flags: None,
            blend_frames: None,
            xlu: None,
            cancel_frame: None,
            no_stop_intp: None,
            animation_unk: None,
            no_extra: false,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Rejected {
    NameTooLong,
    AnimationTooLong,
    TemplateMissing,
    ScriptNameTooLong,
}

pub(crate) fn apply(
    list: &mut MotionList,
    records: &[CopyMotionRecord],
) -> Vec<(String, Result<(), Rejected>)> {
    records
        .iter()
        .map(|record| (record.name.clone(), apply_one(list, record)))
        .collect()
}

fn apply_one(list: &mut MotionList, record: &CopyMotionRecord) -> Result<(), Rejected> {
    let name = hash40(&record.name).ok_or(Rejected::NameTooLong)?;
    let animation = hash40(&record.animation).ok_or(Rejected::AnimationTooLong)?;

    let mut motion = match record.template.as_deref() {
        Some(template) => {
            let key = hash40(template).ok_or(Rejected::TemplateMissing)?;
            list.get(key).cloned().ok_or(Rejected::TemplateMissing)?
        }
        None => Motion::default(),
    };

    let inherited_unk = motion.animations.first().map_or(0, |existing| existing.unk);
    motion.animations = vec![Animation {
        name: animation,
        unk: record.animation_unk.unwrap_or(inherited_unk),
    }];

    if let Some(script) = record.game_script.as_deref() {
        motion.game_script = hash40(script).ok_or(Rejected::ScriptNameTooLong)?;
    }
    if let Some(scripts) = record.scripts.as_ref() {
        let mut resolved = Vec::with_capacity(scripts.len());
        for script in scripts {
            resolved.push(hash40(script).ok_or(Rejected::ScriptNameTooLong)?);
        }
        motion.scripts = resolved;
    }
    if let Some(flags) = record.flags {
        motion.flags = flags;
    }
    if let Some(frames) = record.blend_frames {
        motion.blend_frames = frames;
    }

    if record.no_extra {
        motion.extra = None;
    } else if record.xlu.is_some() || record.cancel_frame.is_some() || record.no_stop_intp.is_some()
    {
        let mut extra = motion.extra.unwrap_or_default();
        if let Some((start, end)) = record.xlu {
            extra.xlu_start = start;
            extra.xlu_end = end;
        }
        if let Some(frame) = record.cancel_frame {
            extra.cancel_frame = frame;
        }
        if let Some(value) = record.no_stop_intp {
            extra.no_stop_intp = value;
        }
        motion.extra = Some(extra);
    }

    list.insert(name, motion);
    Ok(())
}

pub(crate) fn rebuild(original: &[u8], records: &[CopyMotionRecord]) -> Option<Vec<u8>> {
    let mut list = parse(original).ok()?;
    let outcome = apply(&mut list, records);
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
mod live {
    use super::*;
    use clone_engine_api::{
        CloneCopyMotionV1, API_VERSION_V1, COPY_MOTION_NO_EXTRA, COPY_MOTION_SET_ANIMATION_UNK,
        COPY_MOTION_SET_BLEND_FRAMES, COPY_MOTION_SET_CANCEL_FRAME, COPY_MOTION_SET_FLAGS,
        COPY_MOTION_SET_GAME_SCRIPT, COPY_MOTION_SET_NO_STOP_INTP, COPY_MOTION_SET_SCRIPTS,
        COPY_MOTION_SET_XLU, ERROR_NAME, ERROR_NULL, ERROR_STRUCT_SIZE, ERROR_UNSUPPORTED,
        ERROR_VERSION,
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

    #[no_mangle]
    pub unsafe extern "C" fn clone_engine_clone_copy_motion_v1(
        registration: *const CloneCopyMotionV1,
    ) -> i32 {
        if registration.is_null() {
            return ERROR_NULL;
        }
        let registration = &*registration;
        if registration.api_version != API_VERSION_V1 {
            return ERROR_VERSION;
        }
        if registration.struct_size < core::mem::size_of::<CloneCopyMotionV1>() as u32 {
            return ERROR_STRUCT_SIZE;
        }
        let (Some(name), Some(animation)) = (text(registration.name), text(registration.animation))
        else {
            return ERROR_NAME;
        };

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
                return ERROR_NAME;
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

        match record(entry) {
            Ok(()) => {
                skyline::println!(
                    "[kirbymotion] kind {} registered '{name}' -> {animation}",
                    registration.fighter_kind
                );
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
    use crate::motion_list::Extra;

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
    fn a_template_supplies_everything_the_pack_did_not_override() {
        let mut list = list_with_template();
        let mut entry =
            CopyMotionRecord::new(121, "bluster_special_n", "blusterd00specialn.nuanmb");
        entry.template = Some("donkey_special_n".to_string());

        assert_eq!(apply_one(&mut list, &entry), Ok(()));

        let added = list.get(hash40("bluster_special_n").unwrap()).unwrap();
        assert_eq!(
            added.animations,
            vec![Animation {
                name: hash40("blusterd00specialn.nuanmb").unwrap(),
                unk: 1,
            }]
        );
        assert_eq!(added.blend_frames, 3);
        assert_eq!(added.scripts.len(), 3);
        assert_eq!(added.extra.as_ref().unwrap().cancel_frame, 63);
    }

    #[test]
    fn every_field_can_be_overridden_on_top_of_a_template() {
        let mut list = list_with_template();
        let mut entry = CopyMotionRecord::new(121, "mecha_special_n", "mechad00specialn.nuanmb");
        entry.template = Some("donkey_special_n".to_string());
        entry.game_script = Some("game_mechaspecialn".to_string());
        entry.scripts = Some([
            "expression_mechaspecialn".to_string(),
            "sound_mechaspecialn".to_string(),
            "effect_mechaspecialn".to_string(),
        ]);
        entry.flags = Some(clone_engine_api::MotionFlags::LOOP);
        entry.blend_frames = Some(7);
        entry.xlu = Some((2, 9));
        entry.cancel_frame = Some(11);
        entry.no_stop_intp = Some(true);
        entry.animation_unk = Some(4);

        assert_eq!(apply_one(&mut list, &entry), Ok(()));

        let added = list.get(hash40("mecha_special_n").unwrap()).unwrap();
        assert_eq!(added.game_script, hash40("game_mechaspecialn").unwrap());
        assert_eq!(
            added.scripts,
            vec![
                hash40("expression_mechaspecialn").unwrap(),
                hash40("sound_mechaspecialn").unwrap(),
                hash40("effect_mechaspecialn").unwrap(),
            ]
        );
        assert_eq!(added.blend_frames, 7);
        assert_eq!(added.animations[0].unk, 4);
        assert_eq!(added.flags & clone_engine_api::MotionFlags::LOOP,
            clone_engine_api::MotionFlags::LOOP);
        let extra = added.extra.as_ref().unwrap();
        assert_eq!((extra.xlu_start, extra.xlu_end), (2, 9));
        assert_eq!(extra.cancel_frame, 11);
        assert!(extra.no_stop_intp);
    }

    #[test]
    fn a_missing_template_is_refused_rather_than_silently_defaulted() {
        let mut list = list_with_template();
        let mut entry = CopyMotionRecord::new(121, "ghost_special_n", "ghostd00specialn.nuanmb");
        entry.template = Some("nobody_special_n".to_string());

        assert_eq!(apply_one(&mut list, &entry), Err(Rejected::TemplateMissing));
        assert!(list.get(hash40("ghost_special_n").unwrap()).is_none());
    }

    #[test]
    fn a_pack_can_build_an_entry_with_no_template_at_all() {
        let mut list = empty_list();
        let mut entry = CopyMotionRecord::new(121, "solo_special_n", "solod00specialn.nuanmb");
        entry.blend_frames = Some(2);
        entry.no_extra = true;

        assert_eq!(apply_one(&mut list, &entry), Ok(()));
        let added = list.get(hash40("solo_special_n").unwrap()).unwrap();
        assert_eq!(added.blend_frames, 2);
        assert!(added.extra.is_none());
    }

    #[test]
    fn existing_entries_are_left_exactly_as_they_were() {
        let mut list = list_with_template();
        let key = hash40("donkey_special_n").unwrap();
        let before = list.get(key).unwrap().clone();
        let mut entry =
            CopyMotionRecord::new(121, "bluster_special_n", "blusterd00specialn.nuanmb");
        entry.template = Some("donkey_special_n".to_string());

        assert_eq!(apply_one(&mut list, &entry), Ok(()));

        assert_eq!(&before, list.get(key).unwrap());
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
