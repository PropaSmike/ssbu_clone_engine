#[path = "final_smash_table.rs"]
mod final_smash_table;

use crate::motion_list::MotionList;

pub use final_smash_table::{ANIMATION_COUNT, DONOR_COUNT, MOTION_COUNT, STATUS_COUNT};

pub const FINAL: i32 = 0x1e0;
pub const NEEDS_ARTICLES: u8 = 0x01;
pub const NEEDS_MODULE: u8 = 0x02;
pub const NEEDS_BACKGROUND: u8 = 0x04;
pub const NEEDS_MOVIE: u8 = 0x08;
pub const NEEDS_SPECIAL: u8 = 0x10;
pub const SUPPORTED: u8 = 0;

pub struct DonorRow {
    pub name: &'static str,
    pub kind: i32,
    pub needs: u8,
    pub statuses: (usize, usize),
    pub motions: (usize, usize),
    pub animations: (usize, usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Donor {
    pub name: &'static str,
    pub kind: i32,
    pub needs: u8,
    pub statuses: &'static [u16],
    pub motions: &'static [u64],
    pub animations: &'static [&'static str],
}

impl Donor {
    fn of(row: &'static DonorRow) -> Self {
        Self {
            name: row.name,
            kind: row.kind,
            needs: row.needs,
            statuses: &final_smash_table::STATUSES[row.statuses.0..row.statuses.1],
            motions: &final_smash_table::MOTIONS[row.motions.0..row.motions.1],
            animations: &final_smash_table::ANIMATIONS[row.animations.0..row.animations.1],
        }
    }

    pub fn is_final_status(&self, status: i32) -> bool {
        (0..=u16::MAX as i32).contains(&status) && self.statuses.contains(&(status as u16))
    }

    pub fn own_statuses(&self) -> impl Iterator<Item = i32> + '_ {
        self.statuses
            .iter()
            .map(|status| *status as i32)
            .filter(|status| *status != FINAL)
    }

    pub fn highest_status(&self) -> i32 {
        self.statuses.iter().map(|status| *status as i32).max().unwrap_or(FINAL)
    }

    pub fn missing(&self) -> u8 {
        self.needs & !SUPPORTED
    }

    pub fn is_final_motion(&self, kind: u64) -> bool {
        self.motions.contains(&(kind & 0xff_ffff_ffff))
    }
}

pub fn donor(name: &str) -> Option<Donor> {
    final_smash_table::DONORS
        .iter()
        .find(|row| row.name == name)
        .map(Donor::of)
}

pub fn donor_of_kind(kind: i32) -> Option<Donor> {
    final_smash_table::DONORS
        .iter()
        .find(|row| row.kind == kind)
        .map(Donor::of)
}

pub fn donors() -> impl Iterator<Item = Donor> {
    final_smash_table::DONORS.iter().map(Donor::of)
}

pub fn needs_text(bits: u8) -> String {
    let names = [
        (NEEDS_ARTICLES, "its articles"),
        (NEEDS_MODULE, "its final module"),
        (NEEDS_BACKGROUND, "its background scene"),
        (NEEDS_MOVIE, "its movie"),
        (NEEDS_SPECIAL, "a fighter team"),
    ];
    let parts: Vec<&str> = names
        .iter()
        .filter(|(bit, _)| bits & bit != 0)
        .map(|(_, name)| *name)
        .collect();
    match parts.len() {
        0 => String::new(),
        1 => parts[0].to_string(),
        n => format!("{} and {}", parts[..n - 1].join(", "), parts[n - 1]),
    }
}

pub fn supported() -> Vec<&'static str> {
    donors()
        .filter(|donor| donor.missing() == 0)
        .map(|donor| donor.name)
        .collect()
}

pub fn check(name: &str) -> Result<Donor, String> {
    let Some(found) = donor(name) else {
        return Err(format!(
            "final smash donor {name:?} is not a fighter with a Final Smash; donors: {}",
            supported().join(", ")
        ));
    };
    let missing = found.missing();
    if missing != 0 {
        return Err(format!(
            "{name}'s Final Smash needs {}, which a clone cannot borrow yet; donors: {}",
            needs_text(missing),
            supported().join(", ")
        ));
    }
    Ok(found)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grafted {
    Added,
    Replaced,
    AbsentFromDonor,
}

pub fn graft_motions(clone: &mut MotionList, donor_list: &MotionList, donor: &Donor) -> Vec<(u64, Grafted)> {
    let mut outcome = Vec::new();
    for kind in donor.motions {
        let Some(motion) = donor_list.get(*kind) else {
            outcome.push((*kind, Grafted::AbsentFromDonor));
            continue;
        };
        let replaced = clone.get(*kind).is_some();
        clone.insert(*kind, motion.clone());
        outcome.push((*kind, if replaced { Grafted::Replaced } else { Grafted::Added }));
    }
    outcome
}

pub fn final_scripts(donor_list: &MotionList, donor: &Donor) -> Vec<u64> {
    let mut scripts = Vec::new();
    for kind in donor.motions {
        let Some(motion) = donor_list.get(*kind) else {
            continue;
        };
        for script in core::iter::once(&motion.game_script).chain(motion.scripts.iter()) {
            let script = *script & 0xff_ffff_ffff;
            if script != 0 && !scripts.contains(&script) {
                scripts.push(script);
            }
        }
    }
    scripts
}

pub fn final_animations(donor_list: &MotionList, donor: &Donor) -> Vec<u64> {
    let mut files = Vec::new();
    for kind in donor.motions {
        if let Some(motion) = donor_list.get(*kind) {
            for animation in &motion.animations {
                if !files.contains(&animation.name) {
                    files.push(animation.name);
                }
            }
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motion_list::{Animation, Motion};

    fn motion(game: u64, file: u64) -> Motion {
        Motion {
            game_script: game,
            animations: vec![Animation { name: file, unk: 0 }],
            scripts: vec![game + 1, game + 2, game + 3],
            ..Motion::default()
        }
    }

    #[test]
    fn every_vanilla_donor_has_final_and_its_motions() {
        assert!(DONOR_COUNT > 80);
        for donor in donors() {
            assert!(donor.is_final_status(FINAL), "{}", donor.name);
            assert!(!donor.motions.is_empty(), "{}", donor.name);
            assert!(donor.kind >= 0 && donor.kind < 94, "{}", donor.name);
        }
    }

    #[test]
    fn marth_borrows_four_statuses_and_seven_motions() {
        let marth = check("marth").unwrap();
        assert_eq!(marth.kind, 21);
        assert_eq!(marth.statuses, &[0x1e0, 0x1e8, 0x1e9, 0x1ea]);
        assert_eq!(marth.own_statuses().collect::<Vec<_>>(), vec![0x1e8, 0x1e9, 0x1ea]);
        assert_eq!(marth.highest_status(), 0x1ea);
        assert_eq!(marth.motions.len(), 7);
        assert!(marth.is_final_motion(crate::hash40("final_dash")));
        assert!(!marth.is_final_motion(crate::hash40("wait_1")));
        assert!(marth.animations.contains(&"d04finaldash.nuanmb"));
        assert_eq!(marth.animations.len(), 7);
    }

    #[test]
    fn donors_with_unbuilt_pieces_are_refused_by_name() {
        let joker = check("jack").unwrap_err();
        assert!(joker.contains("its final module"), "{joker}");
        assert!(joker.contains("its movie"), "{joker}");
        let ice = check("popo").unwrap_err();
        assert!(ice.contains("a fighter team"), "{ice}");
        let unknown = check("bluster").unwrap_err();
        assert!(unknown.contains("not a fighter with a Final Smash"), "{unknown}");
        assert!(unknown.contains("marth"), "{unknown}");
        assert_eq!(needs_text(NEEDS_ARTICLES | NEEDS_MOVIE | NEEDS_MODULE), "its articles, its final module and its movie");
    }

    #[test]
    fn final_motions_replace_and_extend_the_clone_list() {
        let marth = donor("marth").unwrap();
        let start = crate::hash40("final_start");
        let dash = crate::hash40("final_dash");
        let mut donor_list = MotionList::default();
        donor_list.insert(start, motion(0x10, 0x100));
        donor_list.insert(dash, motion(0x20, 0x200));
        let mut clone = MotionList::default();
        clone.insert(start, motion(0x90, 0x900));
        clone.insert(crate::hash40("wait_1"), motion(0x70, 0x700));
        let outcome = graft_motions(&mut clone, &donor_list, &marth);
        assert!(outcome.contains(&(start, Grafted::Replaced)));
        assert!(outcome.contains(&(dash, Grafted::Added)));
        assert_eq!(outcome.iter().filter(|(_, grafted)| *grafted == Grafted::AbsentFromDonor).count(), 5);
        assert_eq!(clone.get(start).unwrap().game_script, 0x10);
        assert_eq!(clone.get(crate::hash40("wait_1")).unwrap().game_script, 0x70);
        assert_eq!(final_scripts(&donor_list, &marth), vec![0x10, 0x11, 0x12, 0x13, 0x20, 0x21, 0x22, 0x23]);
        assert_eq!(final_animations(&donor_list, &marth), vec![0x100, 0x200]);
    }
}
