#![allow(dead_code)]

use crate::stage_ledger::hash40;

pub const COLUMNS: u8 = 16;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MusicRequest {
    pub set: Option<String>,
    pub setting_no: Option<i64>,
    pub selector: Option<bool>,
}

impl MusicRequest {
    pub fn is_empty(&self) -> bool {
        self.set.is_none() && self.setting_no.is_none() && self.selector.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Music {
    pub set_label: String,
    pub set_id: u64,
    pub setting_no: u8,
    pub selector: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    Borrowed { place: String },
    NoSet,
    UnknownSet { label: String },
    EmptyColumn { label: String, column: u8 },
}

impl core::fmt::Display for Note {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Note::Borrowed { place } => write!(out, "music borrowed from {place}"),
            Note::NoSet => write!(
                out,
                "no bgm set and no vanilla donor to borrow one from; My Music will be empty"
            ),
            Note::UnknownSet { label } => write!(
                out,
                "{label} is not a vanilla playlist; it plays only if a mod publishes that playlist"
            ),
            Note::EmptyColumn { label, column } => write!(
                out,
                "{label} column {column} carries no tracks; My Music will be empty"
            ),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum MusicError {
    ColumnOutOfRange(i64),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Resolution {
    pub music: Music,
    pub notes: Vec<Note>,
}

pub fn set_columns(label: &str) -> Option<u16> {
    VANILLA_SETS
        .iter()
        .find(|(name, _)| *name == label)
        .map(|(_, columns)| *columns)
}

pub fn set_label(requested: &str) -> String {
    let wanted = requested.trim().to_ascii_lowercase();
    if set_columns(&wanted).is_some() {
        return wanted;
    }
    let prefixed = format!("bgm{wanted}");
    if set_columns(&prefixed).is_some() || !wanted.starts_with("bgm") {
        return prefixed;
    }
    wanted
}

pub fn place_music(place: &str) -> Option<(&'static str, &'static str, u8, bool)> {
    let wanted = place.trim().to_ascii_lowercase();
    let underscored = format!("_{wanted}");
    VANILLA_PLACE_MUSIC
        .iter()
        .find(|(name, ..)| *name == wanted || *name == underscored)
        .copied()
}

pub fn resolve(request: &MusicRequest, donor_place: Option<&str>) -> Result<Resolution, MusicError> {
    let column = match request.setting_no {
        Some(value) if !(0..COLUMNS as i64).contains(&value) => {
            return Err(MusicError::ColumnOutOfRange(value))
        }
        other => other.map(|value| value as u8),
    };

    let mut notes = Vec::new();
    let mut music = Music::default();
    let requested = request
        .set
        .as_deref()
        .map(str::trim)
        .filter(|set| !set.is_empty());

    match requested {
        Some(set) => {
            let label = set_label(set);
            if set_columns(&label).is_none() {
                notes.push(Note::UnknownSet {
                    label: label.clone(),
                });
            }
            music.set_label = label;
            music.setting_no = column.unwrap_or(0);
            music.selector = request.selector.unwrap_or(false);
        }
        None => match donor_place.and_then(place_music) {
            Some((place, set, setting_no, selector)) => {
                notes.push(Note::Borrowed {
                    place: place.to_string(),
                });
                music.set_label = set.to_string();
                music.setting_no = column.unwrap_or(setting_no);
                music.selector = request.selector.unwrap_or(selector);
            }
            None => {
                notes.push(Note::NoSet);
                music.setting_no = column.unwrap_or(0);
                music.selector = request.selector.unwrap_or(false);
            }
        },
    }

    if !music.set_label.is_empty() {
        music.set_id = hash40(&music.set_label);
        if let Some(columns) = set_columns(&music.set_label) {
            if columns & (1 << music.setting_no) == 0 {
                notes.push(Note::EmptyColumn {
                    label: music.set_label.clone(),
                    column: music.setting_no,
                });
            }
        }
    }

    Ok(Resolution { music, notes })
}

pub const VANILLA_SETS: &[(&str, u16)] = &[
    ("bgmadventure", 0x0001),
    ("bgmanimal", 0x0007),
    ("bgmbeyo", 0x0001),
    ("bgmboss", 0x003f),
    ("bgmbrave", 0x0001),
    ("bgmbuddy", 0x0001),
    ("bgmdemon", 0x0001),
    ("bgmdk", 0x000f),
    ("bgmdolly", 0x0001),
    ("bgmdracula", 0x0001),
    ("bgmedge", 0x0001),
    ("bgmelement", 0x0001),
    ("bgmfe", 0x0007),
    ("bgmff", 0x0001),
    ("bgmfox", 0x0007),
    ("bgmfzero", 0x0007),
    ("bgmgamewatch", 0x0001),
    ("bgmicaros", 0x0007),
    ("bgmjack", 0x0001),
    ("bgmkirby", 0x003f),
    ("bgmmario", 0xffff),
    ("bgmmaster", 0x0001),
    ("bgmmetalgear", 0x0001),
    ("bgmmetroid", 0x000f),
    ("bgmmkart", 0x0003),
    ("bgmmother", 0x000f),
    ("bgmother", 0x07ff),
    ("bgmpacman", 0x0001),
    ("bgmpickel", 0x0001),
    ("bgmpikmin", 0x0003),
    ("bgmpokemon", 0x007f),
    ("bgmpunchout", 0x0003),
    ("bgmrockman", 0x0001),
    ("bgmsf", 0x0001),
    ("bgmsmashbtl", 0x000f),
    ("bgmsmashmenu", 0x0001),
    ("bgmsmashmode", 0x007f),
    ("bgmsonic", 0x0003),
    ("bgmspla", 0x0001),
    ("bgmstageedit", 0x0001),
    ("bgmtantan", 0x0001),
    ("bgmtrail", 0x0001),
    ("bgmwario", 0x0003),
    ("bgmwiifit", 0x0001),
    ("bgmxenoblade", 0x0001),
    ("bgmyoshi", 0x000f),
    ("bgmzelda", 0x01ff),
];

pub const VANILLA_PLACE_MUSIC: &[(&str, &str, u8, bool)] = &[
    ("_75m", "bgmdk", 3, false),
    ("animal_city", "bgmanimal", 2, false),
    ("animal_island", "bgmanimal", 1, false),
    ("animal_village", "bgmanimal", 0, false),
    ("balloonfight", "bgmother", 3, false),
    ("battlefield", "bgmsmashbtl", 0, true),
    ("battlefield_l", "bgmsmashbtl", 1, true),
    ("battlefield_s", "bgmsmashbtl", 0, true),
    ("bayo_clock", "bgmbeyo", 0, false),
    ("bonusgame", "bgmsmashmode", 0, false),
    ("bossstage_dracula", "bgmboss", 3, false),
    ("bossstage_final1", "bgmboss", 5, false),
    ("bossstage_final2", "bgmboss", 6, false),
    ("bossstage_final3", "bgmboss", 7, false),
    ("bossstage_galleom", "bgmboss", 4, false),
    ("bossstage_ganonboss", "bgmboss", 0, false),
    ("bossstage_marx", "bgmboss", 2, false),
    ("bossstage_rathalos", "bgmboss", 1, false),
    ("brave_altar", "bgmbrave", 0, false),
    ("buddy_spiral", "bgmbuddy", 0, false),
    ("campaignmap", "bgmadventure", 0, false),
    ("demon_dojo", "bgmdemon", 0, false),
    ("dk_jungle", "bgmdk", 0, false),
    ("dk_lodge", "bgmdk", 2, false),
    ("dk_waterfall", "bgmdk", 1, false),
    ("dolly_stadium", "bgmdolly", 0, false),
    ("dracula_castle", "bgmdracula", 0, false),
    ("duckhunt", "bgmother", 1, false),
    ("end", "bgmsmashbtl", 2, true),
    ("fe_arena", "bgmfe", 1, false),
    ("fe_colloseum", "bgmfe", 2, false),
    ("fe_shrine", "bgmmaster", 0, false),
    ("fe_siege", "bgmfe", 0, false),
    ("ff_cave", "bgmedge", 0, false),
    ("ff_midgar", "bgmff", 0, false),
    ("flatzonex", "bgmgamewatch", 0, false),
    ("fox_corneria", "bgmfox", 0, false),
    ("fox_lylatcruise", "bgmfox", 2, false),
    ("fox_venom", "bgmfox", 1, false),
    ("fzero_bigblue", "bgmfzero", 0, false),
    ("fzero_mutecity3ds", "bgmfzero", 2, false),
    ("fzero_porttown", "bgmfzero", 1, false),
    ("homeruncontest", "bgmsmashmode", 2, false),
    ("icarus_angeland", "bgmicaros", 2, false),
    ("icarus_skyworld", "bgmicaros", 0, false),
    ("icarus_uprising", "bgmicaros", 1, false),
    ("ice_top", "bgmother", 0, false),
    ("jack_mementoes", "bgmjack", 0, false),
    ("kart_circuitfor", "bgmmkart", 1, false),
    ("kart_circuitx", "bgmmkart", 0, false),
    ("kirby_cave", "bgmkirby", 5, false),
    ("kirby_fountain", "bgmkirby", 1, false),
    ("kirby_gameboy", "bgmkirby", 4, false),
    ("kirby_greens", "bgmkirby", 2, false),
    ("kirby_halberd", "bgmkirby", 3, false),
    ("kirby_pupupu64", "bgmkirby", 0, false),
    ("luigimansion", "bgmmario", 7, false),
    ("mario_3dland", "bgmmario", 9, false),
    ("mario_castle64", "bgmmario", 0, false),
    ("mario_castledx", "bgmmario", 2, false),
    ("mario_dolpic", "bgmmario", 5, false),
    ("mario_galaxy", "bgmmario", 13, false),
    ("mario_maker", "bgmmario", 14, false),
    ("mario_newbros2", "bgmmario", 10, false),
    ("mario_odyssey", "bgmmario", 15, false),
    ("mario_paper", "bgmmario", 11, false),
    ("mario_past64", "bgmmario", 1, false),
    ("mario_pastusa", "bgmmario", 4, false),
    ("mario_pastx", "bgmmario", 6, false),
    ("mario_rainbow", "bgmmario", 3, false),
    ("mario_uworld", "bgmmario", 12, false),
    ("mariobros", "bgmmario", 8, false),
    ("metroid_kraid", "bgmmetroid", 1, false),
    ("metroid_norfair", "bgmmetroid", 2, false),
    ("metroid_orpheon", "bgmmetroid", 3, false),
    ("metroid_zebesdx", "bgmmetroid", 0, false),
    ("mg_shadowmoses", "bgmmetalgear", 0, false),
    ("mother_fourside", "bgmmother", 1, false),
    ("mother_magicant", "bgmmother", 3, false),
    ("mother_newpork", "bgmmother", 2, false),
    ("mother_onett", "bgmmother", 0, false),
    ("nintendogs", "bgmother", 4, false),
    ("pac_land", "bgmpacman", 0, false),
    ("pickel_world", "bgmpickel", 0, false),
    ("pictochat2", "bgmother", 7, false),
    ("pikmin_garden", "bgmpikmin", 1, false),
    ("pikmin_planet", "bgmpikmin", 0, false),
    ("pilotwings", "bgmother", 9, false),
    ("plankton", "bgmother", 8, false),
    ("poke_kalos", "bgmpokemon", 6, false),
    ("poke_stadium", "bgmpokemon", 1, false),
    ("poke_stadium2", "bgmpokemon", 2, false),
    ("poke_tengam", "bgmpokemon", 3, false),
    ("poke_tower", "bgmpokemon", 5, false),
    ("poke_unova", "bgmpokemon", 4, false),
    ("poke_yamabuki", "bgmpokemon", 0, false),
    ("punchoutsb", "bgmpunchout", 0, false),
    ("punchoutw", "bgmpunchout", 1, false),
    ("rock_wily", "bgmrockman", 0, false),
    ("settingstage", "bgmsmashmode", 4, false),
    ("sf_suzaku", "bgmsf", 0, false),
    ("shamfight", "bgmsmashmode", 6, false),
    ("sonic_greenhill", "bgmsonic", 0, false),
    ("sonic_windyhill", "bgmsonic", 1, false),
    ("sp_edit", "bgmstageedit", 0, false),
    ("spla_parking", "bgmspla", 0, false),
    ("streetpass", "bgmother", 5, false),
    ("tantan_spring", "bgmtantan", 0, false),
    ("tomodachi", "bgmother", 6, false),
    ("trail_castle", "bgmtrail", 0, false),
    ("training", "bgmsmashbtl", 3, false),
    ("wario_gamer", "bgmwario", 1, false),
    ("wario_madein", "bgmwario", 0, false),
    ("wiifit", "bgmwiifit", 0, false),
    ("wreckingcrew", "bgmother", 2, false),
    ("wufuisland", "bgmother", 10, false),
    ("xeno_alst", "bgmelement", 0, false),
    ("xeno_gaur", "bgmxenoblade", 0, false),
    ("yoshi_cartboard", "bgmyoshi", 2, false),
    ("yoshi_island", "bgmyoshi", 3, false),
    ("yoshi_story", "bgmyoshi", 0, false),
    ("yoshi_yoster", "bgmyoshi", 1, false),
    ("zelda_gerudo", "bgmzelda", 5, false),
    ("zelda_greatbay", "bgmzelda", 1, false),
    ("zelda_hyrule", "bgmzelda", 0, false),
    ("zelda_oldin", "bgmzelda", 3, false),
    ("zelda_pirates", "bgmzelda", 4, false),
    ("zelda_skyward", "bgmzelda", 7, false),
    ("zelda_temple", "bgmzelda", 2, false),
    ("zelda_tower", "bgmzelda", 8, false),
    ("zelda_train", "bgmzelda", 6, false),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn request(set: Option<&str>, setting_no: Option<i64>, selector: Option<bool>) -> MusicRequest {
        MusicRequest {
            set: set.map(str::to_string),
            setting_no,
            selector,
        }
    }

    #[test]
    fn a_short_series_name_resolves_to_the_vanilla_playlist() {
        let resolved = resolve(&request(Some("demon"), Some(0), None), None).unwrap();
        assert_eq!(resolved.music.set_label, "bgmdemon");
        assert_eq!(resolved.music.set_id, hash40("bgmdemon"));
        assert_eq!(resolved.music.setting_no, 0);
        assert!(resolved.notes.is_empty());
    }

    #[test]
    fn the_full_label_is_accepted_as_written() {
        let resolved = resolve(&request(Some("bgmdemon"), None, None), None).unwrap();
        assert_eq!(resolved.music.set_label, "bgmdemon");
    }

    #[test]
    fn an_unknown_playlist_is_kept_and_flagged() {
        let resolved = resolve(&request(Some("bgmpumpkin"), None, None), None).unwrap();
        assert_eq!(resolved.music.set_id, hash40("bgmpumpkin"));
        assert_eq!(
            resolved.notes,
            vec![Note::UnknownSet {
                label: "bgmpumpkin".to_string()
            }]
        );
    }

    #[test]
    fn a_custom_playlist_named_without_the_prefix_still_gets_one() {
        let resolved = resolve(&request(Some("mariop"), None, None), None).unwrap();
        assert_eq!(resolved.music.set_label, "bgmmariop");
        assert_eq!(resolved.music.set_id, hash40("bgmmariop"));
        assert_eq!(
            resolved.notes,
            vec![Note::UnknownSet {
                label: "bgmmariop".to_string()
            }]
        );
    }

    #[test]
    fn a_vanilla_donor_supplies_the_music_when_the_pack_names_none() {
        let resolved = resolve(&MusicRequest::default(), Some("dolly_stadium")).unwrap();
        assert_eq!(resolved.music.set_label, "bgmdolly");
        assert_eq!(resolved.music.setting_no, 0);
        assert_eq!(
            resolved.notes,
            vec![Note::Borrowed {
                place: "dolly_stadium".to_string()
            }]
        );
    }

    #[test]
    fn the_donor_column_is_the_one_that_stage_uses() {
        let resolved = resolve(&MusicRequest::default(), Some("mario_odyssey")).unwrap();
        assert_eq!(resolved.music.set_label, "bgmmario");
        assert_eq!(resolved.music.setting_no, 15);
        assert!(!resolved
            .notes
            .iter()
            .any(|note| matches!(note, Note::EmptyColumn { .. })));
    }

    #[test]
    fn an_explicit_column_outranks_the_donor_default() {
        let resolved = resolve(&request(None, Some(2), None), Some("mario_odyssey")).unwrap();
        assert_eq!(resolved.music.set_label, "bgmmario");
        assert_eq!(resolved.music.setting_no, 2);
    }

    #[test]
    fn a_column_past_the_sixteen_is_refused() {
        assert_eq!(
            resolve(&request(Some("demon"), Some(16), None), None),
            Err(MusicError::ColumnOutOfRange(16))
        );
        assert_eq!(
            resolve(&request(Some("demon"), Some(-1), None), None),
            Err(MusicError::ColumnOutOfRange(-1))
        );
    }

    #[test]
    fn a_column_with_no_tracks_is_flagged() {
        let resolved = resolve(&request(Some("demon"), Some(3), None), None).unwrap();
        assert_eq!(
            resolved.notes,
            vec![Note::EmptyColumn {
                label: "bgmdemon".to_string(),
                column: 3
            }]
        );
    }

    #[test]
    fn no_set_and_no_vanilla_donor_leaves_the_row_silent() {
        let resolved = resolve(&MusicRequest::default(), Some("pumpkin_hill")).unwrap();
        assert_eq!(resolved.music.set_id, 0);
        assert_eq!(resolved.notes, vec![Note::NoSet]);
    }

    #[test]
    fn the_underscored_place_is_found_by_its_asset_name() {
        let resolved = resolve(&MusicRequest::default(), Some("75m")).unwrap();
        assert_eq!(resolved.music.set_label, "bgmdk");
        assert_eq!(resolved.music.setting_no, 3);
    }

    #[test]
    fn the_album_selector_follows_the_donor_until_the_pack_says_otherwise() {
        let inherited = resolve(&MusicRequest::default(), Some("battlefield")).unwrap();
        assert!(inherited.music.selector);
        let refused = resolve(&request(None, None, Some(false)), Some("battlefield")).unwrap();
        assert!(!refused.music.selector);
        let asked = resolve(&request(Some("other"), None, Some(true)), None).unwrap();
        assert!(asked.music.selector);
        assert_eq!(asked.music.set_label, "bgmother");
    }

    #[test]
    fn every_table_row_names_a_playlist_the_game_ships() {
        for (place, set, column, _) in VANILLA_PLACE_MUSIC {
            let columns = set_columns(set)
                .unwrap_or_else(|| panic!("{place} points at {set}, which has no playlist"));
            assert!(*column < COLUMNS, "{place} column {column} is out of range");
            let _ = columns;
        }
        assert_eq!(VANILLA_SETS.len(), 47);
        assert_eq!(VANILLA_PLACE_MUSIC.len(), 131);
    }
}
