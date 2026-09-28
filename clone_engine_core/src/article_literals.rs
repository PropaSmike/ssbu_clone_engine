pub const MODEL: i32 = 0;
pub const MOTION_DIRECTORY: i32 = 1;
pub const MOTION_LIST: i32 = 5;

const FIGHTER_ROOT: &str = "fighter/";
const COSTUME_FORMAT: &str = "c%02d";
const MOTION_LIST_SUFFIX: &str = "/motion_list.bin";

pub const LITERAL_PATHS: &[(i32, i32, &str)] = &[
    (0x37, 0, "fighter/samusd/model/body/c%02d/model.numdlb"),
    (0x37, 1, "fighter/samusd/motion/bunshin/c00"),
    (0x37, 5, "fighter/samusd/motion/bunshin/c00"),
    (0x46, 0, "fighter/rosetta/model/tico/c00/model.numdlb"),
    (0x46, 1, "fighter/kirby/motion/rosettaticomissile/c00"),
    (0x46, 5, "fighter/kirby/motion/rosettaticomissile/c00"),
    (0x4a, 0, "fighter/kirby/model/body/c%02d/model.numdlb"),
    (0x4a, 1, "fighter/kirby/motion/windummy/c%02d"),
    (0x4a, 5, "fighter/kirby/motion/windummy/c%02d"),
    (0x52, 0, "fighter/pikachu/model/body/c%02d/model.numdlb"),
    (0x6e, 0, "fighter/koopag/model/body/c00/model.numdlb"),
    (0x8e, 0, "fighter/metaknight/model/body/c%02d/model.numdlb"),
    (0x98, 0, "fighter/samus/model/body/c00/model.numdlb"),
    (0xe2, 0, "fighter/pichu/model/body/c%02d/model.numdlb"),
    (0xf4, 0, "fighter/littlemac/model/body/c%02d/model.numdlb"),
    (0xf4, 1, "fighter/littlemac/motion/body/c%02d"),
    (0xf4, 5, "fighter/littlemac/motion/body/c%02d"),
    (0x15a, 0, "fighter/mewtwo/model/body/c%02d/model.numdlb"),
    (0x1b3, 0, "fighter/simon/model/whip/c%02d/model.numdlb"),
    (0x1b4, 0, "fighter/simon/model/whip/c%02d/model.numdlb"),
    (0x1b4, 1, "fighter/simon/motion/whip/c%02d"),
    (0x1b4, 5, "fighter/simon/motion/whip/c%02d"),
    (0x1c4, 0, "fighter/richter/model/whip/c%02d/model.numdlb"),
    (0x1c5, 0, "fighter/richter/model/whip/c%02d/model.numdlb"),
    (0x1c5, 1, "fighter/richter/motion/whip/c%02d"),
    (0x1c5, 5, "fighter/richter/motion/whip/c%02d"),
    (0x1cc, 0, "fighter/pzenigame/model/body/c%02d/model.numdlb"),
    (0x1cd, 0, "fighter/pfushigisou/model/body/c%02d/model.numdlb"),
    (0x1ce, 0, "fighter/plizardon/model/body/c%02d/model.numdlb"),
    (0x1f2, 0, "fighter/jack/model/body/c%02d/model.numdlb"),
    (0x1f2, 1, "fighter/jack/motion/windummy/c%02d"),
    (0x1f2, 5, "fighter/jack/motion/windummy/c%02d"),
    (0x24f, 0, "fighter/elight/model/body/c%02d/model.numdlb"),
    (0x250, 0, "fighter/elight/model/esword/c%02d/model.numdlb"),
    (0x252, 0, "fighter/elight/model/body/c%02d/model.numdlb"),
    (0x257, 0, "fighter/eflame/model/body/c%02d/model.numdlb"),
    (0x258, 0, "fighter/eflame/model/esword/c%02d/model.numdlb"),
];

pub fn literal(weapon_kind: i32, resource_type: i32) -> Option<&'static str> {
    LITERAL_PATHS
        .iter()
        .find(|(kind, kind_type, _)| *kind == weapon_kind && *kind_type == resource_type)
        .map(|(_, _, path)| *path)
}

pub fn owner(literal: &str) -> Option<&str> {
    literal.strip_prefix(FIGHTER_ROOT)?.split('/').next()
}

pub fn expand(literal: &str, resource_type: i32, color: i32) -> String {
    let mut path = literal.replace(COSTUME_FORMAT, &format!("c{color:02}"));
    if resource_type == MOTION_LIST {
        path.push_str(MOTION_LIST_SUFFIX);
    }
    path
}

pub fn base_path(weapon_kind: i32, resource_type: i32, color: i32) -> Option<String> {
    Some(expand(literal(weapon_kind, resource_type)?, resource_type, color))
}

pub fn clone_path(
    weapon_kind: i32,
    resource_type: i32,
    color: i32,
    base: &str,
    clone: &str,
) -> Option<String> {
    let literal = literal(weapon_kind, resource_type)?;
    if owner(literal)? != base || clone.is_empty() || clone == base {
        return None;
    }
    let rest = &literal[FIGHTER_ROOT.len() + base.len()..];
    Some(expand(&format!("{FIGHTER_ROOT}{clone}{rest}"), resource_type, color))
}

pub fn residency_probe(path: &str, resource_type: i32) -> String {
    if resource_type == MOTION_DIRECTORY {
        format!("{path}{MOTION_LIST_SUFFIX}")
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_the_builders_37_entries() {
        assert_eq!(LITERAL_PATHS.len(), 37);
        assert_eq!(literal(0x15a, MODEL), Some("fighter/mewtwo/model/body/c%02d/model.numdlb"));
        assert_eq!(literal(0x15a, MOTION_LIST), None);
        assert_eq!(literal(0x15, MODEL), None);
    }

    #[test]
    fn a_clone_gets_its_own_folder_for_its_bases_literal() {
        assert_eq!(
            clone_path(0x15a, MODEL, 0, "mewtwo", "herobrine").as_deref(),
            Some("fighter/herobrine/model/body/c00/model.numdlb")
        );
        assert_eq!(
            clone_path(0x1f2, MOTION_LIST, 3, "jack", "wawa").as_deref(),
            Some("fighter/wawa/motion/windummy/c03/motion_list.bin")
        );
        assert_eq!(
            clone_path(0x1f2, MOTION_DIRECTORY, 12, "jack", "wawa").as_deref(),
            Some("fighter/wawa/motion/windummy/c12")
        );
    }

    #[test]
    fn a_fixed_costume_stays_fixed() {
        assert_eq!(
            clone_path(0x37, MOTION_LIST, 5, "samusd", "wawa").as_deref(),
            Some("fighter/wawa/motion/bunshin/c00/motion_list.bin")
        );
    }

    #[test]
    fn a_literal_naming_another_fighter_is_left_alone() {
        assert_eq!(literal(0x24f, MODEL), Some("fighter/elight/model/body/c%02d/model.numdlb"));
        assert_eq!(clone_path(0x24f, MODEL, 0, "eflame", "wawa"), None);
        assert_eq!(clone_path(0x6e, MODEL, 0, "koopa", "wawa"), None);
        assert_eq!(clone_path(0x15a, MODEL, 0, "mewtwo", "mewtwo"), None);
        assert_eq!(clone_path(0x15a, MODEL, 0, "mew", "wawa"), None);
    }

    #[test]
    fn the_base_path_is_the_one_the_game_builds() {
        assert_eq!(
            base_path(0x15a, MODEL, 7).as_deref(),
            Some("fighter/mewtwo/model/body/c07/model.numdlb")
        );
        assert_eq!(
            residency_probe("fighter/jack/motion/windummy/c00", MOTION_DIRECTORY),
            "fighter/jack/motion/windummy/c00/motion_list.bin"
        );
    }
}
