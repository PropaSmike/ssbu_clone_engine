pub const STATUS_STARTS: &[(i32, &str, i32)] = &[
    (0, "mario", 0x1e3),
    (1, "donkey", 0x202),
    (2, "link", 0x1e8),
    (3, "samus", 0x1f6),
    (4, "samusd", 0x1f6),
    (5, "yoshi", 0x1ef),
    (6, "kirby", 0x37d),
    (7, "fox", 0x1ea),
    (8, "pikachu", 0x1f0),
    (9, "luigi", 0x1ee),
    (10, "ness", 0x1ed),
    (11, "captain", 0x1ec),
    (12, "purin", 0x1ea),
    (13, "peach", 0x1ea),
    (14, "daisy", 0x1ea),
    (15, "koopa", 0x1eb),
    (16, "sheik", 0x1f3),
    (17, "zelda", 0x1eb),
    (18, "mariod", 0x1e1),
    (19, "pichu", 0x1f0),
    (20, "falco", 0x1e8),
    (21, "marth", 0x1eb),
    (22, "lucina", 0x1eb),
    (23, "younglink", 0x1e8),
    (24, "ganon", 0x1ec),
    (25, "mewtwo", 0x1e9),
    (26, "roy", 0x1f2),
    (27, "chrom", 0x1f2),
    (28, "gamewatch", 0x205),
    (29, "metaknight", 0x1f0),
    (30, "pit", 0x1ee),
    (31, "pitb", 0x1ee),
    (32, "szerosuit", 0x1ec),
    (33, "wario", 0x200),
    (34, "snake", 0x201),
    (35, "ike", 0x1f2),
    (36, "pzenigame", 0x1ea),
    (37, "pfushigisou", 0x1e5),
    (38, "plizardon", 0x1e6),
    (39, "diddy", 0x1ff),
    (40, "lucas", 0x1ed),
    (41, "sonic", 0x1f3),
    (42, "dedede", 0x209),
    (43, "pikmin", 0x1e8),
    (44, "lucario", 0x1f0),
    (45, "robot", 0x1eb),
    (46, "toonlink", 0x1e8),
    (47, "wolf", 0x1ea),
    (48, "murabito", 0x204),
    (49, "rockman", 0x1f0),
    (50, "wiifit", 0x1ef),
    (51, "rosetta", 0x1e6),
    (52, "littlemac", 0x1f4),
    (53, "gekkouga", 0x1f1),
    (54, "palutena", 0x1e9),
    (55, "pacman", 0x1f0),
    (56, "reflet", 0x1fd),
    (57, "shulk", 0x1ed),
    (58, "koopajr", 0x1ef),
    (59, "duckhunt", 0x1eb),
    (60, "ryu", 0x202),
    (61, "ken", 0x202),
    (62, "cloud", 0x1f7),
    (63, "kamui", 0x1f4),
    (64, "bayonetta", 0x1f2),
    (65, "inkling", 0x1f4),
    (66, "ridley", 0x202),
    (67, "simon", 0x1ec),
    (68, "richter", 0x1ec),
    (69, "krool", 0x1f9),
    (70, "shizue", 0x210),
    (71, "gaogaen", 0x1f4),
    (72, "miifighter", 0x207),
    (73, "miiswordsman", 0x1f9),
    (74, "miigunner", 0x1fa),
    (75, "popo", 0x1ef),
    (76, "nana", 0x1ef),
    (77, "koopag", 0x1eb),
    (78, "miienemyf", 0x1fe),
    (79, "miienemys", 0x1fa),
    (80, "miienemyg", 0x1f7),
    (81, "packun", 0x1f4),
    (82, "jack", 0x1fc),
    (83, "brave", 0x1fe),
    (84, "buddy", 0x1fb),
    (85, "dolly", 0x203),
    (86, "master", 0x1f8),
    (87, "tantan", 0x1ff),
    (88, "pickel", 0x21b),
    (89, "edge", 0x1f3),
    (90, "eflame", 0x1ee),
    (91, "elight", 0x1f1),
    (92, "demon", 0x213),
    (93, "trail", 0x1f5),
    (117, "element", 0x1e7),
];

pub const OWN_STATUS: i32 = 0x4000_0000;

pub fn status_start(base: &str) -> Option<i32> {
    let base = base.trim().to_ascii_lowercase();
    let base = base.strip_prefix("fighter_kind_").unwrap_or(&base);
    STATUS_STARTS
        .iter()
        .find(|(_, name, _)| *name == base)
        .map(|(_, _, start)| *start)
}

pub fn status_start_of_kind(kind: i32) -> Option<i32> {
    STATUS_STARTS
        .iter()
        .find(|(known, _, _)| *known == kind)
        .map(|(_, _, start)| *start)
}

pub fn own_status(text: &str) -> Option<i32> {
    let rest = text.trim().strip_prefix("own")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    rest.trim().parse::<i32>().ok().filter(|index| (0..0x1000).contains(index))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_statuses_start_where_the_bases_table_ends() {
        assert_eq!(status_start("mario"), Some(0x1e3));
        assert_eq!(status_start("donkey"), Some(0x202));
        assert_eq!(status_start("lucina"), status_start("marth"));
        assert_eq!(status_start("FIGHTER_KIND_SAMUS"), status_start("samus"));
        assert_eq!(status_start("wawa"), None);
        assert_eq!(status_start_of_kind(0), Some(0x1e3));
        assert_eq!(own_status("own 3"), Some(3));
        assert_eq!(own_status(" own 0 "), Some(0));
        assert_eq!(own_status("owner 1"), None);
        assert_eq!(own_status("own -1"), None);
    }
}
