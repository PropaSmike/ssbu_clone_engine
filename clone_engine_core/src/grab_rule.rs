use crate::hash40;

pub const CAPTURE: u64 = 0x7_8bfe_a6e5;
pub const EVENT_ID_MASK: u64 = 0xff_ffff_ffff;
pub const ON_LINK_EVENT_SLOT: u32 = 46;
pub const DEFAULT_NODE: &str = "throw";
pub const MAX_RULES: usize = 256;

pub const EVENT_KIND: usize = 0x10;
pub const EVENT_SENDER_ID: usize = 0x20;
pub const EVENT_RESULT: usize = 0x28;
pub const EVENT_NODE: usize = 0x30;
pub const EVENT_STATUS: usize = 0x38;
pub const EVENT_MOTION_OFFSET: usize = 0x44;
pub const EVENT_MOTION_OFFSET_LW: usize = 0x48;
pub const EVENT_IS_MOTION_HI_LW: usize = 0x4c;
pub const EVENT_CONSTRAINT: usize = 0x4f;
pub const EVENT_CONSTRAINT_OFFSET: usize = 0x50;

pub const STATUS_CAPTURE_PULLED: i32 = 0x41;
pub const WORK_INT_CATCH_MOTION_OFFSET: i32 = 0x1000_003e;
pub const WORK_INT_CATCH_MOTION_OFFSET_LW: i32 = 0x1000_003f;
pub const WORK_FLAG_CATCH_MOTION_HI_LW: i32 = 0x2000_0053;

pub const STATUS_NAMES: &[(&str, i32)] = &[
    ("ATTACK", 0x27),
    ("ATTACK_DASH", 0x29),
    ("ATTACK_S3", 0x2a),
    ("ATTACK_HI3", 0x2b),
    ("ATTACK_LW3", 0x2c),
    ("ATTACK_S4", 0x2f),
    ("ATTACK_LW4", 0x32),
    ("ATTACK_HI4", 0x35),
    ("ATTACK_AIR", 0x36),
    ("CATCH", 0x37),
    ("CATCH_PULL", 0x38),
    ("CATCH_DASH", 0x39),
    ("CATCH_DASH_PULL", 0x3a),
    ("CATCH_TURN", 0x3b),
    ("CATCH_WAIT", 0x3c),
    ("THROW", 0x40),
    ("CAPTURE_PULLED", 0x41),
    ("CAPTURE_WAIT", 0x42),
    ("CLUNG_GANON", 0xbd),
    ("CATCHED_GANON", 0xbe),
    ("CATCHED_AIR_GANON", 0xbf),
    ("CLUNG_CAPTAIN", 0xc3),
    ("KOOPA_DIVED", 0xc4),
    ("SHOULDERED_DONKEY_START", 0xc5),
    ("SWALLOWED", 0xe6),
    ("BITTEN_WARIO_START", 0xed),
    ("CLUNG_DIDDY", 0xf0),
    ("CAPTURE_YOSHI", 0xf3),
    ("CATCHED_REFLET", 0xfe),
    ("CATCHED_RIDLEY", 0x171),
    ("MIIFIGHTER_SUPLEX_AIR_CAPTURED", 0x186),
    ("CAPTURE_JACK_WIRE", 0x19c),
    ("DOLLY_SUPER_SPECIAL2_CAPTURE", 0x1ae),
    ("CAPTURE_MASTER_SWORD", 0x1b4),
    ("DEMON_DIVED", 0x1ca),
    ("SPECIAL_N", 0x1dc),
    ("SPECIAL_S", 0x1dd),
    ("SPECIAL_HI", 0x1de),
    ("SPECIAL_LW", 0x1df),
];

pub fn status_kind(name: &str) -> Option<i32> {
    let name = name.trim();
    let bare = name.strip_prefix("FIGHTER_STATUS_KIND_").unwrap_or(name);
    STATUS_NAMES
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(bare))
        .map(|(_, value)| *value)
}

pub fn status_label(value: i32) -> String {
    match STATUS_NAMES.iter().find(|(_, known)| *known == value) {
        Some((name, _)) => (*name).to_string(),
        None => format!("{value:#x}"),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GrabRule {
    pub status: i32,
    pub victim: Option<i32>,
    pub then: Option<i32>,
    pub node: Option<String>,
    pub offset: [f32; 3],
    pub catch: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reply {
    pub node: Option<u64>,
    pub offset: [f32; 3],
    pub then: Option<i32>,
    pub catch: bool,
    pub catch_motion: bool,
}

impl GrabRule {
    pub fn new(status: i32) -> Self {
        Self {
            status,
            victim: None,
            then: None,
            node: Some(DEFAULT_NODE.to_string()),
            offset: [0.0; 3],
            catch: false,
        }
    }

    pub fn matches(&self, status: i32, victim: i32) -> bool {
        self.status == status && self.victim.map_or(true, |wanted| wanted == victim)
    }

    pub fn node_hash(&self) -> Option<u64> {
        self.node
            .as_deref()
            .filter(|node| !node.is_empty())
            .map(|node| hash40(&node.to_ascii_lowercase()))
    }

    pub fn reply(&self, victim: i32) -> Reply {
        Reply {
            node: self.node_hash(),
            offset: self.offset,
            then: self.then,
            catch: self.catch,
            catch_motion: victim == STATUS_CAPTURE_PULLED,
        }
    }

    pub fn describe(&self) -> String {
        let victim = self.victim.map_or_else(|| "any victim status".to_string(), status_label);
        let then = self.then.map_or_else(|| "stays".to_string(), |then| format!("-> {}", status_label(then)));
        let node = match self.node.as_deref() {
            Some(node) if !node.is_empty() => format!("held at {node}"),
            _ => "not pinned".to_string(),
        };
        let mut text = format!("{} + {victim}: {then}, {node}", status_label(self.status));
        if self.offset != [0.0; 3] {
            text.push_str(&format!(" offset {:?}", self.offset));
        }
        if self.catch {
            text.push_str(", set_catch");
        }
        text
    }
}

pub fn is_capture(event_kind: u64) -> bool {
    event_kind & EVENT_ID_MASK == CAPTURE
}

pub fn answer<'a>(
    rules: impl IntoIterator<Item = &'a GrabRule>,
    status: i32,
    victim: i32,
) -> Option<Reply> {
    rules
        .into_iter()
        .find(|rule| rule.matches(status, victim))
        .map(|rule| rule.reply(victim))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_capture_event_id_is_hash40_capture_under_the_mask() {
        assert_eq!(hash40("capture"), CAPTURE);
        assert!(is_capture(CAPTURE | 0xab00_0000_0000));
        assert!(!is_capture(hash40("capture_cut")));
    }

    #[test]
    fn the_status_table_matches_the_game_constants() {
        assert_eq!(status_kind("SPECIAL_S"), Some(0x1dd));
        assert_eq!(status_kind("FIGHTER_STATUS_KIND_CATCHED_GANON"), Some(0xbe));
        assert_eq!(status_kind("catched_ganon"), Some(0xbe));
        assert_eq!(status_kind("CATCH_PULL"), Some(0x38));
        assert_eq!(status_kind("KOOPA_DIVED"), Some(0xc4));
        assert_eq!(status_kind("NOT_A_STATUS"), None);
        assert_eq!(status_label(0x41), "CAPTURE_PULLED");
        assert_eq!(status_label(0x1e2), "0x1e2");
    }

    #[test]
    fn a_rule_matches_its_status_and_victim_or_any_victim() {
        let mut rule = GrabRule::new(0x1dd);
        assert!(rule.matches(0x1dd, 0xbe));
        assert!(rule.matches(0x1dd, 0x41));
        assert!(!rule.matches(0x1de, 0xbe));
        rule.victim = Some(0xbe);
        assert!(rule.matches(0x1dd, 0xbe));
        assert!(!rule.matches(0x1dd, 0xbf));
    }

    #[test]
    fn the_first_matching_rule_answers() {
        let mut ground = GrabRule::new(0x1dd);
        ground.victim = Some(0xbe);
        ground.then = Some(0x1e2);
        let mut air = GrabRule::new(0x1dd);
        air.victim = Some(0xbf);
        air.then = Some(0x1e3);
        let rules = [ground, air];
        assert_eq!(answer(&rules, 0x1dd, 0xbf).and_then(|reply| reply.then), Some(0x1e3));
        assert_eq!(answer(&rules, 0x1dd, 0xbe).and_then(|reply| reply.then), Some(0x1e2));
        assert_eq!(answer(&rules, 0x1dd, 0x41), None);
        assert_eq!(answer(&rules, 0x37, 0xbe), None);
    }

    #[test]
    fn the_reply_holds_the_victim_at_throw_like_the_game() {
        let reply = GrabRule::new(0x27).reply(0xbe);
        assert_eq!(reply.node, Some(hash40("throw")));
        assert_eq!(reply.node, Some(0x5_4f93_4137));
        assert!(!reply.catch_motion);
        assert!(GrabRule::new(0x27).reply(STATUS_CAPTURE_PULLED).catch_motion);
    }

    #[test]
    fn an_empty_node_leaves_the_victim_unpinned() {
        let mut rule = GrabRule::new(0x1de);
        rule.node = Some(String::new());
        assert_eq!(rule.reply(0xbd).node, None);
        rule.node = None;
        assert_eq!(rule.reply(0xbd).node, None);
        rule.node = Some("Hip".to_string());
        assert_eq!(rule.reply(0xbd).node, Some(hash40("hip")));
    }

    #[test]
    fn a_description_names_what_it_can() {
        let mut rule = GrabRule::new(0x1dd);
        rule.victim = Some(0xbe);
        rule.then = Some(0x1e2);
        assert_eq!(rule.describe(), "SPECIAL_S + CATCHED_GANON: -> 0x1e2, held at throw");
        rule.node = Some(String::new());
        rule.catch = true;
        rule.offset = [0.0, 2.0, 0.0];
        assert_eq!(
            rule.describe(),
            "SPECIAL_S + CATCHED_GANON: -> 0x1e2, not pinned offset [0.0, 2.0, 0.0], set_catch"
        );
    }
}
