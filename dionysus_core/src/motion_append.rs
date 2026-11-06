use crate::motion_list::{hash40, Animation, Extra, Motion, MotionList};

pub const BYTES_PER_MOTION: usize = 128;
pub const VICTIM_FLAGS: u16 = 0xa8;

pub const VANILLA_BODY_MOTION_OWNERS: [&str; 97] = [
    "bayonetta", "brave", "buddy", "captain", "chrom", "cloud", "daisy", "dedede", "demon",
    "diddy", "dolly", "donkey", "duckhunt", "edge", "eflame", "element", "elight", "falco", "fox",
    "gamewatch", "ganon", "gaogaen", "gekkouga", "ike", "inkling", "jack", "kamui", "ken", "kirby",
    "koopa", "koopag", "koopajr", "krool", "link", "littlemac", "lucario", "lucas", "lucina",
    "luigi", "mario", "mariod", "marth", "master", "metaknight", "mewtwo", "miienemyf",
    "miienemyg", "miienemys", "miifighter", "miigunner", "miiswordsman", "murabito", "nana",
    "ness", "packun", "pacman", "palutena", "peach", "pfushigisou", "pichu", "pickel", "pikachu",
    "pikmin", "pit", "pitb", "plizardon", "popo", "ptrainer", "ptrainer_low", "purin",
    "pzenigame", "reflet", "richter", "ridley", "robot", "rockman", "rosetta", "roy", "ryu",
    "samus", "samusd", "sheik", "shizue", "shulk", "simon", "snake", "sonic", "szerosuit",
    "tantan", "toonlink", "trail", "wario", "wiifit", "wolf", "yoshi", "younglink", "zelda",
];

pub fn body_motion_directory(owner: &str, costume: u32) -> String {
    format!("fighter/{owner}/motion/body/c{costume:02}")
}

pub fn body_motion_list_path(owner: &str, costume: u32) -> String {
    format!("{}/motion_list.bin", body_motion_directory(owner, costume))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MotionRecord {
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
    pub victim: bool,
}

impl MotionRecord {
    pub fn new(fighter_kind: i32, name: &str, animation: &str) -> Self {
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
            victim: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejected {
    NameTooLong,
    AnimationTooLong,
    TemplateMissing,
    ScriptNameTooLong,
    AnimationMissing,
    AlreadyPresent,
}

pub type Outcome = Vec<(String, Result<(), Rejected>)>;

pub fn apply_copy(list: &mut MotionList, records: &[MotionRecord]) -> Outcome {
    records
        .iter()
        .map(|record| (record.name.clone(), copy_one(list, record)))
        .collect()
}

pub fn apply_global(
    list: &mut MotionList,
    records: &[MotionRecord],
    has_animation: impl Fn(&str) -> bool,
) -> Outcome {
    records
        .iter()
        .map(|record| (record.name.clone(), global_one(list, record, &has_animation)))
        .collect()
}

pub fn copy_one(list: &mut MotionList, record: &MotionRecord) -> Result<(), Rejected> {
    let name = hash40(&record.name).ok_or(Rejected::NameTooLong)?;
    let animation = hash40(&record.animation).ok_or(Rejected::AnimationTooLong)?;
    let mut motion = start(list, record)?;
    motion.animations = vec![Animation {
        name: animation,
        unk: motion.animations.first().map_or(0, |existing| existing.unk),
    }];
    customize(&mut motion, record)?;
    list.insert(name, motion);
    Ok(())
}

pub fn global_one(
    list: &mut MotionList,
    record: &MotionRecord,
    has_animation: impl Fn(&str) -> bool,
) -> Result<(), Rejected> {
    let name = hash40(&record.name).ok_or(Rejected::NameTooLong)?;
    if list.get(name).is_some() {
        return Err(Rejected::AlreadyPresent);
    }
    let own = if record.animation.is_empty() {
        None
    } else {
        let animation = hash40(&record.animation).ok_or(Rejected::AnimationTooLong)?;
        has_animation(&record.animation).then_some(animation)
    };
    let mut motion = match (record.victim, record.template.is_none()) {
        (true, true) => victim_motion(&record.name)?,
        _ => start(list, record)?,
    };
    match own {
        Some(animation) => {
            motion.animations = vec![Animation {
                name: animation,
                unk: motion.animations.first().map_or(0, |existing| existing.unk),
            }];
        }
        None if record.victim => motion.animations.clear(),
        None if record.template.is_none() => return Err(Rejected::AnimationMissing),
        None => {}
    }
    customize(&mut motion, record)?;
    list.insert(name, motion);
    Ok(())
}

pub fn victim_motion(name: &str) -> Result<Motion, Rejected> {
    let stem = name.replace('_', "");
    let script = |prefix: &str| hash40(&format!("{prefix}_{stem}")).ok_or(Rejected::ScriptNameTooLong);
    Ok(Motion {
        game_script: script("game")?,
        flags: VICTIM_FLAGS,
        blend_frames: 0,
        animations: Vec::new(),
        scripts: vec![script("expression")?, script("sound")?, script("effect")?],
        extra: Some(Extra::default()),
    })
}

fn start(list: &MotionList, record: &MotionRecord) -> Result<Motion, Rejected> {
    match record.template.as_deref() {
        Some(template) => {
            let key = hash40(template).ok_or(Rejected::TemplateMissing)?;
            list.get(key).cloned().ok_or(Rejected::TemplateMissing)
        }
        None => Ok(Motion::default()),
    }
}

fn customize(motion: &mut Motion, record: &MotionRecord) -> Result<(), Rejected> {
    if let Some(unk) = record.animation_unk {
        for animation in &mut motion.animations {
            animation.unk = unk;
        }
    }
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
        let mut extra = motion.extra.take().unwrap_or_default();
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motion_list::{parse, serialize, Extra, HEADER_BYTES, MAX_ANIMATIONS};

    const LOOP: u16 = 1 << 1;

    fn empty_list() -> MotionList {
        MotionList {
            motion_path: hash40("fighter/kirby/motion/body/c00").unwrap(),
            entries: Vec::new(),
        }
    }

    fn template_motion() -> Motion {
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
        }
    }

    fn list_with_template() -> MotionList {
        let mut list = empty_list();
        list.insert(hash40("donkey_special_n").unwrap(), template_motion());
        list
    }

    fn victim_list(owner: &str) -> MotionList {
        let mut list = MotionList {
            motion_path: hash40(&body_motion_directory(owner, 0)).unwrap(),
            entries: Vec::new(),
        };
        list.insert(
            hash40("catched_ganon").unwrap(),
            Motion {
                game_script: hash40("game_catchedganon").unwrap(),
                flags: 0,
                blend_frames: 0,
                animations: vec![Animation {
                    name: hash40("d05catchedganon.nuanmb").unwrap(),
                    unk: 0,
                }],
                scripts: vec![],
                extra: None,
            },
        );
        list
    }

    fn global(name: &str, template: Option<&str>, animation: &str) -> MotionRecord {
        let mut record = MotionRecord::new(121, name, animation);
        record.template = template.map(str::to_string);
        record
    }

    #[test]
    fn a_template_supplies_everything_the_pack_did_not_override() {
        let mut list = list_with_template();
        let mut entry = MotionRecord::new(121, "bluster_special_n", "blusterd00specialn.nuanmb");
        entry.template = Some("donkey_special_n".to_string());

        assert_eq!(copy_one(&mut list, &entry), Ok(()));

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
        let mut entry = MotionRecord::new(121, "mecha_special_n", "mechad00specialn.nuanmb");
        entry.template = Some("donkey_special_n".to_string());
        entry.game_script = Some("game_mechaspecialn".to_string());
        entry.scripts = Some([
            "expression_mechaspecialn".to_string(),
            "sound_mechaspecialn".to_string(),
            "effect_mechaspecialn".to_string(),
        ]);
        entry.flags = Some(LOOP);
        entry.blend_frames = Some(7);
        entry.xlu = Some((2, 9));
        entry.cancel_frame = Some(11);
        entry.no_stop_intp = Some(true);
        entry.animation_unk = Some(4);

        assert_eq!(copy_one(&mut list, &entry), Ok(()));

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
        assert_eq!(added.flags & LOOP, LOOP);
        let extra = added.extra.as_ref().unwrap();
        assert_eq!((extra.xlu_start, extra.xlu_end), (2, 9));
        assert_eq!(extra.cancel_frame, 11);
        assert!(extra.no_stop_intp);
    }

    #[test]
    fn a_missing_template_is_refused_rather_than_silently_defaulted() {
        let mut list = list_with_template();
        let mut entry = MotionRecord::new(121, "ghost_special_n", "ghostd00specialn.nuanmb");
        entry.template = Some("nobody_special_n".to_string());

        assert_eq!(copy_one(&mut list, &entry), Err(Rejected::TemplateMissing));
        assert!(list.get(hash40("ghost_special_n").unwrap()).is_none());
    }

    #[test]
    fn a_pack_can_build_an_entry_with_no_template_at_all() {
        let mut list = empty_list();
        let mut entry = MotionRecord::new(121, "solo_special_n", "solod00specialn.nuanmb");
        entry.blend_frames = Some(2);
        entry.no_extra = true;

        assert_eq!(copy_one(&mut list, &entry), Ok(()));
        let added = list.get(hash40("solo_special_n").unwrap()).unwrap();
        assert_eq!(added.blend_frames, 2);
        assert!(added.extra.is_none());
    }

    #[test]
    fn existing_entries_are_left_exactly_as_they_were() {
        let mut list = list_with_template();
        let key = hash40("donkey_special_n").unwrap();
        let before = list.get(key).unwrap().clone();
        let mut entry = MotionRecord::new(121, "bluster_special_n", "blusterd00specialn.nuanmb");
        entry.template = Some("donkey_special_n".to_string());

        assert_eq!(copy_one(&mut list, &entry), Ok(()));

        assert_eq!(&before, list.get(key).unwrap());
    }

    #[test]
    fn the_worst_case_entry_fits_the_per_motion_allowance() {
        let mut list = empty_list();
        list.insert(
            hash40("worst_case").unwrap(),
            Motion {
                game_script: hash40("game_worstcase").unwrap(),
                flags: 0,
                blend_frames: 0,
                animations: (0..MAX_ANIMATIONS)
                    .map(|index| Animation {
                        name: hash40(&format!("worstcased0{index}.nuanmb")).unwrap(),
                        unk: 0,
                    })
                    .collect(),
                scripts: vec![
                    hash40("expression_worstcase").unwrap(),
                    hash40("sound_worstcase").unwrap(),
                    hash40("effect_worstcase").unwrap(),
                ],
                extra: Some(Extra::default()),
            },
        );
        let bytes = serialize(&list).unwrap();
        assert!(bytes.len() - HEADER_BYTES <= BYTES_PER_MOTION);
    }

    #[test]
    fn a_global_motion_borrows_each_fighters_own_template_animation() {
        let mut mario = victim_list("mario");
        let mut link = victim_list("link");
        link.insert(
            hash40("catched_ganon").unwrap(),
            Motion {
                animations: vec![Animation {
                    name: hash40("d05catchedganonlink.nuanmb").unwrap(),
                    unk: 0,
                }],
                ..Motion::default()
            },
        );
        let record = global("catched_bluster", Some("catched_ganon"), "");

        assert_eq!(global_one(&mut mario, &record, |_| false), Ok(()));
        assert_eq!(global_one(&mut link, &record, |_| false), Ok(()));

        let key = hash40("catched_bluster").unwrap();
        assert_eq!(
            mario.get(key).unwrap().animations[0].name,
            hash40("d05catchedganon.nuanmb").unwrap()
        );
        assert_eq!(
            link.get(key).unwrap().animations[0].name,
            hash40("d05catchedganonlink.nuanmb").unwrap()
        );
    }

    #[test]
    fn a_fighter_that_ships_the_animation_uses_it() {
        let mut list = victim_list("mario");
        let record = global("catched_bluster", Some("catched_ganon"), "d05catchedbluster.nuanmb");

        assert_eq!(
            global_one(&mut list, &record, |file| file == "d05catchedbluster.nuanmb"),
            Ok(())
        );

        let added = list.get(hash40("catched_bluster").unwrap()).unwrap();
        assert_eq!(
            added.animations,
            vec![Animation {
                name: hash40("d05catchedbluster.nuanmb").unwrap(),
                unk: 0,
            }]
        );
        assert_eq!(added.game_script, hash40("game_catchedganon").unwrap());
    }

    #[test]
    fn a_fighter_without_the_animation_falls_back_to_the_template() {
        let mut list = victim_list("mario");
        let record = global("catched_bluster", Some("catched_ganon"), "d05catchedbluster.nuanmb");

        assert_eq!(global_one(&mut list, &record, |_| false), Ok(()));

        assert_eq!(
            list.get(hash40("catched_bluster").unwrap()).unwrap().animations[0].name,
            hash40("d05catchedganon.nuanmb").unwrap()
        );
    }

    #[test]
    fn with_no_template_a_fighter_without_the_animation_is_skipped() {
        let mut list = victim_list("mario");
        let record = global("bluster_victim", None, "d05blustervictim.nuanmb");

        assert_eq!(
            global_one(&mut list, &record, |_| false),
            Err(Rejected::AnimationMissing)
        );
        assert!(list.get(hash40("bluster_victim").unwrap()).is_none());

        assert_eq!(global_one(&mut list, &record, |_| true), Ok(()));
        assert!(list.get(hash40("bluster_victim").unwrap()).is_some());
    }

    #[test]
    fn a_global_motion_never_replaces_one_the_fighter_already_has() {
        let mut list = victim_list("mario");
        let key = hash40("catched_ganon").unwrap();
        let before = list.get(key).unwrap().clone();
        let mut record = global("catched_ganon", None, "d05other.nuanmb");
        record.blend_frames = Some(9);

        assert_eq!(
            global_one(&mut list, &record, |_| true),
            Err(Rejected::AlreadyPresent)
        );
        assert_eq!(list.get(key), Some(&before));
        assert_eq!(list.entries.len(), 1);
    }

    #[test]
    fn a_fighter_missing_the_template_is_skipped() {
        let mut list = victim_list("mario");
        let record = global("catched_bluster", Some("catched_nobody"), "");

        assert_eq!(
            global_one(&mut list, &record, |_| true),
            Err(Rejected::TemplateMissing)
        );
        assert_eq!(list.entries.len(), 1);
    }

    #[test]
    fn a_global_motion_takes_the_same_overrides_as_a_copy_motion() {
        let mut list = victim_list("mario");
        let mut record = global("catched_bluster", Some("catched_ganon"), "");
        record.scripts = Some([
            "expression_catchedbluster".to_string(),
            "sound_catchedbluster".to_string(),
            "effect_catchedbluster".to_string(),
        ]);
        record.game_script = Some("game_catchedbluster".to_string());
        record.flags = Some(LOOP);
        record.cancel_frame = Some(20);

        assert_eq!(global_one(&mut list, &record, |_| false), Ok(()));

        let added = list.get(hash40("catched_bluster").unwrap()).unwrap();
        assert_eq!(added.game_script, hash40("game_catchedbluster").unwrap());
        assert_eq!(added.scripts.len(), 3);
        assert_eq!(added.flags, LOOP);
        assert_eq!(added.extra.as_ref().unwrap().cancel_frame, 20);
    }

    #[test]
    fn one_refused_motion_does_not_stop_the_others() {
        let mut list = victim_list("mario");
        let records = [
            global("catched_bluster", Some("catched_nobody"), ""),
            global("catched_mecha", Some("catched_ganon"), ""),
        ];

        let outcome = apply_global(&mut list, &records, |_| false);

        assert_eq!(
            outcome,
            vec![
                ("catched_bluster".to_string(), Err(Rejected::TemplateMissing)),
                ("catched_mecha".to_string(), Ok(())),
            ]
        );
        assert_eq!(list.entries.len(), 2);
    }

    fn victim(name: &str, template: Option<&str>, animation: &str) -> MotionRecord {
        let mut record = global(name, template, animation);
        record.victim = true;
        record
    }

    #[test]
    fn a_victim_motion_has_no_animation_so_the_grabber_supplies_it() {
        let mut list = victim_list("mario");

        assert_eq!(global_one(&mut list, &victim("thrown_bluster_f", None, ""), |_| true), Ok(()));

        let added = list.get(hash40("thrown_bluster_f").unwrap()).unwrap();
        assert!(added.animations.is_empty());
        assert_eq!(added.flags, VICTIM_FLAGS);
        assert_eq!(added.blend_frames, 0);
        assert_eq!(added.game_script, hash40("game_thrownblusterf").unwrap());
        assert_eq!(
            added.scripts,
            vec![
                hash40("expression_thrownblusterf").unwrap(),
                hash40("sound_thrownblusterf").unwrap(),
                hash40("effect_thrownblusterf").unwrap(),
            ]
        );
        assert_eq!(added.extra, Some(Extra::default()));
    }

    #[test]
    fn only_the_fighter_that_ships_the_victim_animation_gets_it() {
        let record = victim("thrown_bluster_f", None, "e01thrownblusterf.nuanmb");
        let mut grabber = victim_list("bluster");
        let mut opponent = victim_list("mario");

        assert_eq!(global_one(&mut grabber, &record, |_| true), Ok(()));
        assert_eq!(global_one(&mut opponent, &record, |_| false), Ok(()));

        let key = hash40("thrown_bluster_f").unwrap();
        let held = grabber.get(key).unwrap();
        assert_eq!(
            held.animations,
            vec![Animation {
                name: hash40("e01thrownblusterf.nuanmb").unwrap(),
                unk: 0,
            }]
        );
        assert_eq!(held.flags, VICTIM_FLAGS);
        assert!(opponent.get(key).unwrap().animations.is_empty());
    }

    #[test]
    fn a_grabber_list_that_already_has_the_victim_motion_is_kept() {
        let mut list = victim_list("bluster");
        let key = hash40("catched_ganon").unwrap();
        let before = list.get(key).unwrap().clone();

        assert_eq!(
            global_one(&mut list, &victim("catched_ganon", None, ""), |_| false),
            Err(Rejected::AlreadyPresent)
        );
        assert_eq!(list.get(key), Some(&before));
    }

    #[test]
    fn a_victim_template_keeps_its_fields_but_never_its_animation() {
        let mut list = victim_list("mario");

        assert_eq!(
            global_one(&mut list, &victim("catched_bluster", Some("catched_ganon"), ""), |_| false),
            Ok(())
        );

        let added = list.get(hash40("catched_bluster").unwrap()).unwrap();
        assert!(added.animations.is_empty());
        assert_eq!(added.game_script, hash40("game_catchedganon").unwrap());
    }

    #[test]
    fn a_victim_motion_takes_the_overrides() {
        let mut list = victim_list("mario");
        let mut record = victim("thrown_bluster_f_dx", None, "");
        record.scripts = Some([
            "expression_thrownblusterf".to_string(),
            "sound_thrownblusterf".to_string(),
            "effect_thrownblusterf".to_string(),
        ]);
        record.game_script = Some("game_thrownblusterf".to_string());
        record.flags = Some(VICTIM_FLAGS | LOOP);

        assert_eq!(global_one(&mut list, &record, |_| false), Ok(()));

        let added = list.get(hash40("thrown_bluster_f_dx").unwrap()).unwrap();
        assert_eq!(added.game_script, hash40("game_thrownblusterf").unwrap());
        assert_eq!(added.scripts[0], hash40("expression_thrownblusterf").unwrap());
        assert_eq!(added.flags, VICTIM_FLAGS | LOOP);
    }

    #[test]
    fn a_victim_entry_fits_the_per_motion_allowance() {
        let mut list = empty_list();
        let mut record = victim("thrown_bluster_f", None, "e01thrownblusterf.nuanmb");
        record.cancel_frame = Some(1);
        global_one(&mut list, &record, |_| true).unwrap();
        let bytes = serialize(&list).unwrap();
        assert!(bytes.len() - HEADER_BYTES <= BYTES_PER_MOTION);
    }

    #[test]
    fn a_victim_name_too_long_for_its_script_names_is_refused() {
        let mut list = empty_list();
        assert_eq!(
            global_one(&mut list, &victim(&"v".repeat(250), None, ""), |_| false),
            Err(Rejected::ScriptNameTooLong)
        );
        assert!(list.entries.is_empty());
    }

    #[test]
    fn the_paths_match_the_arc() {
        assert_eq!(
            body_motion_list_path("dolly", 70),
            "fighter/dolly/motion/body/c70/motion_list.bin"
        );
        assert_eq!(body_motion_directory("mario", 3), "fighter/mario/motion/body/c03");
        assert_eq!(body_motion_directory("mario", 128), "fighter/mario/motion/body/c128");
    }

    fn vanilla_lists() -> Vec<(String, Vec<u8>)> {
        let root = std::env::var("SSBU_MOTION_LISTS")
            .expect("SSBU_MOTION_LISTS must name a folder holding fighter/<name>/motion/body/cNN/motion_list.bin");
        let mut found = Vec::new();
        for owner in VANILLA_BODY_MOTION_OWNERS {
            for costume in 0..8 {
                let path = format!("{root}/{}", body_motion_list_path(owner, costume));
                if let Ok(bytes) = std::fs::read(&path) {
                    found.push((path, bytes));
                }
            }
        }
        found
    }

    #[test]
    #[ignore]
    fn every_vanilla_body_list_survives_a_read_write_cycle_byte_for_byte() {
        let lists = vanilla_lists();
        assert_eq!(lists.len(), 94 * 8 + 3);
        for (path, bytes) in &lists {
            let parsed = parse(bytes).unwrap_or_else(|error| panic!("{path}: {error:?}"));
            assert_eq!(&serialize(&parsed).unwrap(), bytes, "{path}");
        }
    }

    #[test]
    #[ignore]
    fn a_template_global_motion_lands_in_every_vanilla_list_that_has_the_template() {
        let record = global("catched_clone_engine_test", Some("catched_ganon"), "");
        let mut landed = 0;
        for (path, bytes) in vanilla_lists() {
            let mut list = parse(&bytes).unwrap();
            let before = list.entries.len();
            let outcome = apply_global(&mut list, std::slice::from_ref(&record), |_| false);
            let grown = serialize(&list).unwrap();
            match outcome[0].1 {
                Ok(()) => {
                    landed += 1;
                    assert_eq!(list.entries.len(), before + 1, "{path}");
                    assert!(grown.len() - bytes.len() <= BYTES_PER_MOTION, "{path}");
                    assert_eq!(parse(&grown).unwrap(), list, "{path}");
                }
                Err(reason) => {
                    assert_eq!(reason, Rejected::TemplateMissing, "{path}");
                    assert_eq!(&grown, &bytes, "{path}");
                }
            }
        }
        assert!(landed > 0);
    }

    #[test]
    #[ignore]
    fn a_victim_motion_matches_the_games_own_empty_victim_entries() {
        let names = ["thrown_f_donkey_f", "gaogaen_thrown_01", "mewtwo_thrown", "jack_special_hi_thrown"];
        let mut compared = 0;
        for (path, bytes) in vanilla_lists() {
            let list = parse(&bytes).unwrap();
            for name in names {
                let Some(vanilla) = list.get(hash40(name).unwrap()) else {
                    continue;
                };
                if !vanilla.animations.is_empty() {
                    continue;
                }
                assert_eq!(&victim_motion(name).unwrap(), vanilla, "{path} {name}");
                compared += 1;
            }
        }
        assert!(compared > 2000, "{compared}");
    }
}
