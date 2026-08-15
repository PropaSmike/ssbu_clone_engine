#![allow(dead_code)]

#[cfg(test)]
#[path = "stage_bounds.rs"]
mod stage_bounds;
#[cfg(not(test))]
use crate::stage_bounds::{STAGE_REFERENCES, STAGE_TABLES};
#[cfg(test)]
use stage_bounds::{STAGE_REFERENCES, STAGE_TABLES};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ownership {
    Vanilla,
    Foreign,
    ForeignPartial,
    Contested,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableVerdict {
    pub table: &'static str,
    pub ownership: Ownership,
    pub sites: usize,
    pub foreign_sites: usize,
    pub vanilla_at: Vec<usize>,
}

impl TableVerdict {
    pub fn vanilla_sites(&self) -> usize {
        self.vanilla_at.len()
    }

    pub fn unrecognised_sites(&self) -> usize {
        self.sites - self.vanilla_sites() - self.foreign_sites
    }
}

pub fn is_foreign_patch(first: u32, second: u32) -> bool {
    first & 0xFFE0_001F == 0xD280_0012 && second & 0xFC00_0000 == 0x1400_0000
}

fn classify_site(recorded: (u32, u32), observed: (u32, u32)) -> Option<bool> {
    if observed == recorded {
        Some(true)
    } else if is_foreign_patch(observed.0, observed.1) {
        Some(false)
    } else {
        None
    }
}

pub fn survey_table<R>(table: &'static str, read: &R) -> TableVerdict
where
    R: Fn(usize) -> u32,
{
    let mut sites = 0;
    let mut vanilla_at = Vec::new();
    let mut foreign_sites = 0;

    for reference in STAGE_REFERENCES {
        if reference.table != table {
            continue;
        }
        sites += 1;
        let observed = (read(reference.adrp_at), read(reference.adrp_at + 4));
        match classify_site((reference.adrp_opcode, reference.add_opcode), observed) {
            Some(true) => vanilla_at.push(reference.adrp_at),
            Some(false) => foreign_sites += 1,
            None => {}
        }
    }

    let recognised = vanilla_at.len() + foreign_sites;
    let ownership = if sites == 0 || recognised != sites {
        Ownership::Contested
    } else if foreign_sites == 0 {
        Ownership::Vanilla
    } else if vanilla_at.is_empty() {
        Ownership::Foreign
    } else {
        Ownership::ForeignPartial
    };

    TableVerdict {
        table,
        ownership,
        sites,
        foreign_sites,
        vanilla_at,
    }
}

pub fn survey<R>(read: &R) -> Vec<TableVerdict>
where
    R: Fn(usize) -> u32,
{
    STAGE_TABLES
        .iter()
        .map(|table| survey_table(table.name, read))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageIdBackend {
    Relocate,
    ExtendForeign,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceBackend {
    Relocate,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub verdicts: Vec<TableVerdict>,
    pub places: PlaceBackend,
    pub stage_ids: StageIdBackend,
}

impl Plan {
    pub fn can_mint(&self) -> bool {
        self.places == PlaceBackend::Relocate && self.stage_ids != StageIdBackend::None
    }

    fn ownership_of(&self, table: &str) -> Option<Ownership> {
        self.verdicts
            .iter()
            .find(|verdict| verdict.table == table)
            .map(|verdict| verdict.ownership)
    }
}

pub fn decide(verdicts: Vec<TableVerdict>) -> Plan {
    let of = |name: &str| {
        verdicts
            .iter()
            .find(|verdict| verdict.table == name)
            .map(|verdict| verdict.ownership)
    };

    let places = match (of("stage_place"), of("stage_place_aux")) {
        (Some(Ownership::Vanilla), Some(Ownership::Vanilla)) => PlaceBackend::Relocate,
        _ => PlaceBackend::None,
    };

    let stage_ids = match of("stage_id") {
        Some(Ownership::Vanilla) => StageIdBackend::Relocate,
        Some(Ownership::Foreign) | Some(Ownership::ForeignPartial) => StageIdBackend::ExtendForeign,
        _ => StageIdBackend::None,
    };

    Plan {
        verdicts,
        places,
        stage_ids,
    }
}

#[cfg(not(test))]
static OBSERVED: std::sync::OnceLock<Plan> = std::sync::OnceLock::new();

#[cfg(not(test))]
pub fn arm() -> &'static Plan {
    OBSERVED.get_or_init(|| {
        let text = crate::text_base();
        let read =
            move |offset: usize| unsafe { core::ptr::read_volatile((text + offset) as *const u32) };
        decide(survey(&read))
    })
}

#[cfg(not(test))]
pub fn observed() -> Option<&'static Plan> {
    OBSERVED.get()
}

#[cfg(not(test))]
pub fn report(plan: &Plan) {
    for verdict in &plan.verdicts {
        skyline::println!(
            "[stageown] {}: {:?} ({} sites: {} vanilla, {} foreign, {} unrecognised)",
            verdict.table,
            verdict.ownership,
            verdict.sites,
            verdict.vanilla_sites(),
            verdict.foreign_sites,
            verdict.unrecognised_sites(),
        );
        if verdict.ownership == Ownership::ForeignPartial {
            skyline::println!(
                "[stageown] {} left {} site(s) unpatched by its owner: {:x?}. We finish them.",
                verdict.table,
                verdict.vanilla_sites(),
                verdict.vanilla_at,
            );
        }
    }
    skyline::println!(
        "[stageown] places={:?} stage_ids={:?} -> minting {}",
        plan.places,
        plan.stage_ids,
        if plan.can_mint() {
            "available"
        } else {
            "UNAVAILABLE"
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pristine() -> impl Fn(usize) -> u32 {
        |offset| {
            for reference in STAGE_REFERENCES {
                if reference.adrp_at == offset {
                    return reference.adrp_opcode;
                }
                if reference.adrp_at + 4 == offset {
                    return reference.add_opcode;
                }
            }
            0
        }
    }

    fn foreign_words(id: u32) -> (u32, u32) {
        (0xD280_0012 | (id << 5), 0x1400_0000 | 0x12345)
    }

    fn patched(table: &'static str) -> impl Fn(usize) -> u32 {
        move |offset| {
            for reference in STAGE_REFERENCES {
                if reference.table == table {
                    let (first, second) = foreign_words(0);
                    if reference.adrp_at == offset {
                        return first;
                    }
                    if reference.adrp_at + 4 == offset {
                        return second;
                    }
                } else {
                    if reference.adrp_at == offset {
                        return reference.adrp_opcode;
                    }
                    if reference.adrp_at + 4 == offset {
                        return reference.add_opcode;
                    }
                }
            }
            0
        }
    }

    #[test]
    fn the_census_actually_covers_all_three_tables() {
        let plan = decide(survey(&pristine()));
        assert_eq!(plan.verdicts.len(), 3);
        for verdict in &plan.verdicts {
            assert!(
                verdict.sites > 0,
                "{} has no reference sites",
                verdict.table
            );
        }
    }

    #[test]
    fn a_pristine_image_is_ours_end_to_end() {
        let plan = decide(survey(&pristine()));
        for verdict in &plan.verdicts {
            assert_eq!(verdict.ownership, Ownership::Vanilla, "{}", verdict.table);
        }
        assert_eq!(plan.places, PlaceBackend::Relocate);
        assert_eq!(plan.stage_ids, StageIdBackend::Relocate);
        assert!(plan.can_mint());
    }

    #[test]
    fn csk_takes_stage_id_and_leaves_the_places_alone() {
        let plan = decide(survey(&patched("stage_id")));
        assert_eq!(plan.ownership_of("stage_id"), Some(Ownership::Foreign));
        assert_eq!(plan.ownership_of("stage_place"), Some(Ownership::Vanilla));
        assert_eq!(
            plan.ownership_of("stage_place_aux"),
            Some(Ownership::Vanilla)
        );
        assert_eq!(plan.places, PlaceBackend::Relocate);
        assert_eq!(plan.stage_ids, StageIdBackend::ExtendForeign);
        assert!(plan.can_mint());
    }

    #[test]
    fn losing_the_place_table_stops_minting_even_though_stage_id_is_fine() {
        let plan = decide(survey(&patched("stage_place")));
        assert_eq!(plan.places, PlaceBackend::None);
        assert_eq!(plan.stage_ids, StageIdBackend::Relocate);
        assert!(!plan.can_mint(), "a place we cannot write is not a stage");
    }

    fn mostly_patched(table: &'static str, leave_vanilla: usize) -> impl Fn(usize) -> u32 {
        let untouched: Vec<usize> = STAGE_REFERENCES
            .iter()
            .filter(|reference| reference.table == table)
            .rev()
            .take(leave_vanilla)
            .map(|reference| reference.adrp_at)
            .collect();
        move |offset| {
            for reference in STAGE_REFERENCES {
                let vanilla = reference.table != table || untouched.contains(&reference.adrp_at);
                if reference.adrp_at == offset {
                    return if vanilla {
                        reference.adrp_opcode
                    } else {
                        foreign_words(0).0
                    };
                }
                if reference.adrp_at + 4 == offset {
                    return if vanilla {
                        reference.add_opcode
                    } else {
                        foreign_words(0).1
                    };
                }
            }
            0
        }
    }

    #[test]
    fn the_243_of_245_the_console_actually_showed() {
        let plan = decide(survey(&mostly_patched("stage_id", 2)));
        let verdict = plan
            .verdicts
            .iter()
            .find(|verdict| verdict.table == "stage_id")
            .unwrap();
        assert_eq!(verdict.ownership, Ownership::ForeignPartial);
        assert_eq!(verdict.sites, 261);
        assert_eq!(verdict.foreign_sites, 259);
        assert_eq!(verdict.vanilla_sites(), 2);
        assert_eq!(verdict.unrecognised_sites(), 0);
        assert_eq!(plan.stage_ids, StageIdBackend::ExtendForeign);
        assert!(plan.can_mint());
    }

    #[test]
    fn the_stragglers_are_named_not_just_counted() {
        let plan = decide(survey(&mostly_patched("stage_id", 2)));
        let verdict = plan
            .verdicts
            .iter()
            .find(|verdict| verdict.table == "stage_id")
            .unwrap();
        assert_eq!(verdict.vanilla_at.len(), 2);
        for site in &verdict.vanilla_at {
            assert!(
                STAGE_REFERENCES
                    .iter()
                    .any(|reference| reference.adrp_at == *site && reference.table == "stage_id"),
                "{site:#x} is not a stage_id reference"
            );
        }
    }

    #[test]
    fn one_unrecognised_site_still_poisons_the_whole_table() {
        let first = STAGE_REFERENCES
            .iter()
            .find(|reference| reference.table == "stage_id")
            .unwrap();
        let inner = mostly_patched("stage_id", 2);
        let read = |offset: usize| {
            if offset == first.adrp_at {
                return 0xDEAD_BEEF;
            }
            inner(offset)
        };
        let plan = decide(survey(&read));
        assert_eq!(plan.ownership_of("stage_id"), Some(Ownership::Contested));
        assert_eq!(plan.stage_ids, StageIdBackend::None);
        assert!(!plan.can_mint());
    }

    #[test]
    fn a_wrong_image_is_contested_not_foreign() {
        let read = |_offset: usize| 0xDEAD_BEEFu32;
        let plan = decide(survey(&read));
        for verdict in &plan.verdicts {
            assert_eq!(verdict.ownership, Ownership::Contested, "{}", verdict.table);
            assert_eq!(verdict.unrecognised_sites(), verdict.sites);
        }
        assert!(!plan.can_mint());
    }

    #[test]
    fn the_foreign_signature_does_not_match_the_vanilla_pair() {
        for reference in STAGE_REFERENCES {
            assert!(
                !is_foreign_patch(reference.adrp_opcode, reference.add_opcode),
                "{:#x} reads as a foreign patch",
                reference.adrp_at
            );
        }
    }

    #[test]
    fn the_signature_accepts_any_table_id() {
        for id in [0u32, 1, 2, 7, 0xFFFF] {
            let (first, second) = foreign_words(id);
            assert!(is_foreign_patch(first, second), "id {id}");
        }
    }
}

#[cfg(test)]
pub fn arm() -> &'static Plan {
    unreachable!("stage_backend::arm surveys the running image; not available on the host")
}

#[cfg(test)]
pub fn observed() -> Option<&'static Plan> {
    None
}

#[cfg(test)]
pub fn report(_plan: &Plan) {}
