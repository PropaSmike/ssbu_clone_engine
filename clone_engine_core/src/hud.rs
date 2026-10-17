pub const JACK: i32 = 82;
pub const CLOUD: i32 = 62;
pub const INKLING: i32 = 65;
pub const BRAVE: i32 = 83;
pub const LITTLEMAC: i32 = 52;
pub const SHULK: i32 = 57;
pub const REFLET: i32 = 56;
pub const PICKEL: i32 = 88;
pub const TRAIL: i32 = 93;
pub const DOLLY: i32 = 85;
pub const DEMON: i32 = 92;

pub const DONORS: &[(&str, i32)] = &[
    ("jack", JACK),
    ("cloud", CLOUD),
    ("inkling", INKLING),
    ("brave", BRAVE),
    ("littlemac", LITTLEMAC),
    ("shulk", SHULK),
    ("reflet", REFLET),
    ("pickel", PICKEL),
    ("trail", TRAIL),
    ("dolly", DOLLY),
    ("demon", DEMON),
];

pub const MENU_ROWS: usize = 4;
pub const MAX_MENU_COMMAND: u32 = 21;
pub const BRAVE_MP_MAX: f32 = 100.0;
pub const LITTLEMAC_METER_MAX: f32 = 100.0;
pub const SHULK_ARTS: i32 = 5;
pub const REFLET_TOMES: i32 = 5;
pub const PICKEL_MATERIALS: i32 = 7;
pub const PICKEL_COUNT_MAX: f32 = 999.0;
pub const TRAIL_SPELLS: i32 = 3;
pub const NO_SELECTION: i32 = -1;
pub const MAX_PART_NAME: usize = 63;
pub const MAX_TEXT: usize = 0x7fe;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuCommand {
    pub command: u32,
    pub cost: f32,
    pub usable: bool,
}

impl MenuCommand {
    pub const fn new(command: u32, cost: f32) -> Self {
        Self {
            command,
            cost,
            usable: true,
        }
    }

    pub const fn unusable(mut self) -> Self {
        self.usable = false;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HudAction<'a> {
    Gauge(f32),
    State(bool),
    Select(i32),
    Slot(i32, f32),
    MenuOpen([MenuCommand; MENU_ROWS]),
    MenuCursor(i32),
    MenuDecide(i32),
    MenuClose,
    Play(&'a str),
    Frame(&'a str, f32),
    Show(&'a str, bool),
    Text(&'a str, &'a str),
}

impl HudAction<'_> {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Gauge(_) => "Gauge",
            Self::State(_) => "State",
            Self::Select(_) => "Select",
            Self::Slot(..) => "Slot",
            Self::MenuOpen(_) => "MenuOpen",
            Self::MenuCursor(_) => "MenuCursor",
            Self::MenuDecide(_) => "MenuDecide",
            Self::MenuClose => "MenuClose",
            Self::Play(_) => "Play",
            Self::Frame(..) => "Frame",
            Self::Show(..) => "Show",
            Self::Text(..) => "Text",
        }
    }

    pub fn part(&self) -> Option<&str> {
        match self {
            Self::Play(part) | Self::Frame(part, _) | Self::Show(part, _) | Self::Text(part, _) => {
                Some(part)
            }
            _ => None,
        }
    }
}

pub fn check_layout(path: &str) -> Result<(), String> {
    let allowed = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_-./".contains(&byte);
    if path.ends_with(".arc")
        && !path.starts_with('/')
        && !path.contains("//")
        && path.bytes().all(allowed)
    {
        Ok(())
    } else {
        Err(format!(
            "hud layout {path:?} must be a lowercase arc path ending in .arc, such as \"ui/layout/hud/my_fighter/layout.arc\""
        ))
    }
}

pub fn check_part(action: &HudAction) -> Result<(), String> {
    let Some(part) = action.part() else {
        return Err(format!("{} is not a call for the fighter's own layout", action.name()));
    };
    if part.is_empty() || part.len() > MAX_PART_NAME || part.contains('%') || part.contains('\0') {
        return Err(format!(
            "{part:?} is not a part name: 1 to {MAX_PART_NAME} characters, without % or NUL"
        ));
    }
    match action {
        HudAction::Frame(_, frame) if !frame.is_finite() => Err("the frame must be a number".to_string()),
        HudAction::Text(_, text) if text.contains('\0') => Err("the text cannot contain NUL".to_string()),
        HudAction::Text(_, text) if text.chars().count() > MAX_TEXT => {
            Err(format!("the text is longer than {MAX_TEXT} characters"))
        }
        _ => Ok(()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Message {
    pub id: u32,
    pub payload: [u8; 16],
}

impl Message {
    fn new(id: u32) -> Self {
        Self {
            id,
            payload: [0; 16],
        }
    }

    fn word(mut self, offset: usize, value: u32) -> Self {
        self.payload[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        self
    }

    fn float(self, offset: usize, value: f32) -> Self {
        self.word(offset, value.to_bits())
    }

    fn byte(mut self, offset: usize, value: bool) -> Self {
        self.payload[offset] = u8::from(value);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub messages: Vec<Message>,
    pub selected: i32,
}

pub fn donor_kind(donor: &str) -> Option<i32> {
    DONORS
        .iter()
        .find(|(name, _)| *name == donor)
        .map(|(_, kind)| *kind)
}

pub fn donor_name(kind: i32) -> Option<&'static str> {
    DONORS
        .iter()
        .find(|(_, donor)| *donor == kind)
        .map(|(name, _)| *name)
}

pub fn check(donor: &str) -> Result<i32, String> {
    donor_kind(donor).ok_or_else(|| {
        format!(
            "hud from {donor:?} is not a HUD the engine drives; use {}",
            DONORS
                .iter()
                .map(|(name, _)| format!("{name:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

fn ratio(value: f32) -> Result<f32, String> {
    if value.is_finite() {
        Ok(value.clamp(0.0, 1.0))
    } else {
        Err("the value must be a number".to_string())
    }
}

fn index(value: i32, count: i32, what: &str) -> Result<u32, String> {
    if (0..count).contains(&value) {
        Ok(value as u32)
    } else {
        Err(format!("{what} {value} is outside 0..{}", count - 1))
    }
}

fn row(value: i32) -> Result<u32, String> {
    Ok(index(value, MENU_ROWS as i32, "menu row")? + 1)
}

pub fn plan(donor_kind: i32, action: &HudAction, selected: i32) -> Result<Plan, String> {
    let name = donor_name(donor_kind).ok_or_else(|| format!("kind {donor_kind} has no HUD here"))?;
    let mut next = selected;
    let messages = match (donor_kind, action) {
        (JACK, HudAction::Gauge(value)) => vec![Message::new(0x58).float(0, ratio(*value)?)],
        (CLOUD, HudAction::Gauge(value)) => {
            let value = ratio(*value)?;
            vec![Message::new(0x4f).float(0, value).byte(4, value >= 1.0)]
        }
        (INKLING, HudAction::Gauge(value)) => vec![Message::new(0x42).float(0, ratio(*value)?)],
        (BRAVE, HudAction::Gauge(value)) => {
            vec![Message::new(0x5d).float(0, ratio(*value)? * BRAVE_MP_MAX)]
        }
        (BRAVE, HudAction::MenuOpen(commands)) => {
            let mut out = Vec::with_capacity(MENU_ROWS * 2 + 2);
            for (slot, command) in commands.iter().enumerate() {
                if command.command > MAX_MENU_COMMAND {
                    return Err(format!(
                        "menu command {} is outside 0..{MAX_MENU_COMMAND}",
                        command.command
                    ));
                }
                if !command.cost.is_finite() || command.cost < 0.0 {
                    return Err("a menu cost must be a number of 0 or more".to_string());
                }
                let line = slot as u32 + 1;
                out.push(
                    Message::new(0x5f)
                        .word(0, line)
                        .word(4, command.command)
                        .float(8, command.cost),
                );
                out.push(Message::new(0x60).word(0, line).byte(4, command.usable));
            }
            out.push(Message::new(0x61));
            out.push(Message::new(0x62));
            out
        }
        (BRAVE, HudAction::MenuCursor(line)) => vec![Message::new(0x63).word(0, row(*line)?)],
        (BRAVE, HudAction::MenuDecide(line)) => {
            let line = row(*line)?;
            vec![Message::new(0x64), Message::new(0x65).word(0, line)]
        }
        (BRAVE, HudAction::MenuClose) => vec![Message::new(0x66), Message::new(0x67)],
        (LITTLEMAC, HudAction::Gauge(value)) => {
            let percent = (ratio(*value)? * LITTLEMAC_METER_MAX).round() as u32;
            vec![Message::new(0x12).word(0, percent)]
        }
        (SHULK, HudAction::Select(art)) => {
            let art = index(*art, SHULK_ARTS, "art")?;
            next = art as i32;
            vec![Message::new(0x49).word(0, art)]
        }
        (SHULK, HudAction::Gauge(value)) => {
            if selected == NO_SELECTION {
                return Err("select an art before setting its timer".to_string());
            }
            let value = ratio(*value)?;
            vec![Message::new(0x4b)
                .word(0, selected as u32)
                .float(4, value)
                .byte(8, true)
                .byte(9, value <= 0.0 || value >= 1.0)]
        }
        (REFLET, HudAction::Slot(tome, value)) => {
            let tome = index(*tome, REFLET_TOMES, "tome")?;
            vec![Message::new(0x46).word(0, tome).float(4, ratio(*value)?)]
        }
        (PICKEL, HudAction::Slot(material, count)) => {
            let material = index(*material, PICKEL_MATERIALS, "material")?;
            if !count.is_finite() {
                return Err("the count must be a number".to_string());
            }
            let count = count.round().clamp(0.0, PICKEL_COUNT_MAX) as u32;
            vec![Message::new(0x6c).word(0, material).word(4, count)]
        }
        (TRAIL, HudAction::Select(spell)) => {
            let spell = index(*spell, TRAIL_SPELLS, "spell")?;
            next = spell as i32;
            vec![Message::new(0x7b).word(0, spell).byte(4, true)]
        }
        (DOLLY, HudAction::State(on)) => vec![Message::new(0x6a).byte(0, *on)],
        (DEMON, HudAction::State(on)) => vec![Message::new(0x76).byte(0, *on)],
        _ => {
            return Err(format!("the {name} HUD has no {}", action.name()));
        }
    };
    Ok(Plan {
        messages,
        selected: next,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn only(donor: i32, action: HudAction) -> Message {
        let plan = plan(donor, &action, NO_SELECTION).unwrap();
        assert_eq!(plan.messages.len(), 1, "{plan:?}");
        plan.messages[0]
    }

    fn word(message: &Message, offset: usize) -> u32 {
        u32::from_le_bytes(message.payload[offset..offset + 4].try_into().unwrap())
    }

    fn float(message: &Message, offset: usize) -> f32 {
        f32::from_bits(word(message, offset))
    }

    #[test]
    fn donors_map_to_their_kinds_and_unknown_ones_are_refused() {
        assert_eq!(donor_kind("pickel"), Some(88));
        assert_eq!(donor_name(57), Some("shulk"));
        assert_eq!(check("brave"), Ok(83));
        let message = check("murabito").unwrap_err();
        assert!(message.contains("\"murabito\""), "{message}");
        assert!(message.contains("\"jack\", \"cloud\""), "{message}");
    }

    #[test]
    fn gauges_use_each_donor_scale() {
        let jack = only(JACK, HudAction::Gauge(1.5));
        assert_eq!((jack.id, float(&jack, 0)), (0x58, 1.0));
        let cloud = only(CLOUD, HudAction::Gauge(1.0));
        assert_eq!((cloud.id, float(&cloud, 0), cloud.payload[4]), (0x4f, 1.0, 1));
        assert_eq!(only(CLOUD, HudAction::Gauge(0.5)).payload[4], 0);
        assert_eq!(float(&only(INKLING, HudAction::Gauge(-2.0)), 0), 0.0);
        let brave = only(BRAVE, HudAction::Gauge(0.37));
        assert_eq!((brave.id, float(&brave, 0)), (0x5d, 37.0));
        let mac = only(LITTLEMAC, HudAction::Gauge(0.426));
        assert_eq!((mac.id, word(&mac, 0)), (0x12, 43));
        assert!(plan(JACK, &HudAction::Gauge(f32::NAN), NO_SELECTION).is_err());
    }

    #[test]
    fn states_are_one_byte() {
        assert_eq!(only(DOLLY, HudAction::State(true)), Message::new(0x6a).byte(0, true));
        assert!(plan(JACK, &HudAction::State(true), NO_SELECTION).is_err());
        assert_eq!(only(DEMON, HudAction::State(false)), Message::new(0x76));
    }

    #[test]
    fn shulk_remembers_the_art_for_its_timer() {
        assert!(plan(SHULK, &HudAction::Gauge(0.5), NO_SELECTION)
            .unwrap_err()
            .contains("select an art"));
        let chosen = plan(SHULK, &HudAction::Select(3), NO_SELECTION).unwrap();
        assert_eq!(chosen.selected, 3);
        assert_eq!(word(&chosen.messages[0], 0), 3);
        let timer = plan(SHULK, &HudAction::Gauge(0.25), chosen.selected).unwrap();
        let message = timer.messages[0];
        assert_eq!((message.id, word(&message, 0), float(&message, 4)), (0x4b, 3, 0.25));
        assert_eq!((message.payload[8], message.payload[9]), (1, 0));
        let empty = plan(SHULK, &HudAction::Gauge(0.0), 3).unwrap().messages[0];
        assert_eq!(empty.payload[9], 1);
        assert!(plan(SHULK, &HudAction::Select(5), NO_SELECTION).is_err());
    }

    #[test]
    fn slots_carry_their_index() {
        let tome = only(REFLET, HudAction::Slot(2, 0.5));
        assert_eq!((tome.id, word(&tome, 0), float(&tome, 4)), (0x46, 2, 0.5));
        let dirt = only(PICKEL, HudAction::Slot(0, 12.4));
        assert_eq!((dirt.id, word(&dirt, 0), word(&dirt, 4)), (0x6c, 0, 12));
        assert_eq!(word(&only(PICKEL, HudAction::Slot(6, 5000.0)), 4), 999);
        assert!(plan(PICKEL, &HudAction::Slot(7, 1.0), NO_SELECTION).is_err());
        assert!(plan(REFLET, &HudAction::Slot(-1, 1.0), NO_SELECTION).is_err());
    }

    #[test]
    fn sora_selects_a_spell() {
        let spell = only(TRAIL, HudAction::Select(2));
        assert_eq!((spell.id, word(&spell, 0), spell.payload[4]), (0x7b, 2, 1));
        assert!(plan(TRAIL, &HudAction::Select(3), NO_SELECTION).is_err());
    }

    #[test]
    fn the_brave_menu_opens_with_four_rows_then_moves_and_closes() {
        let commands = [
            MenuCommand::new(14, 8.0),
            MenuCommand::new(2, 8.0),
            MenuCommand::new(5, 37.0).unusable(),
            MenuCommand::new(8, 88.0),
        ];
        let open = plan(BRAVE, &HudAction::MenuOpen(commands), NO_SELECTION).unwrap().messages;
        let ids: Vec<u32> = open.iter().map(|message| message.id).collect();
        assert_eq!(ids, [0x5f, 0x60, 0x5f, 0x60, 0x5f, 0x60, 0x5f, 0x60, 0x61, 0x62]);
        assert_eq!((word(&open[4], 0), word(&open[4], 4), float(&open[4], 8)), (3, 5, 37.0));
        assert_eq!((word(&open[5], 0), open[5].payload[4]), (3, 0));
        let cursor = only(BRAVE, HudAction::MenuCursor(1));
        assert_eq!((cursor.id, word(&cursor, 0)), (0x63, 2));
        let decide = plan(BRAVE, &HudAction::MenuDecide(3), NO_SELECTION).unwrap().messages;
        assert_eq!((decide[0].id, decide[1].id, word(&decide[1], 0)), (0x64, 0x65, 4));
        let close = plan(BRAVE, &HudAction::MenuClose, NO_SELECTION).unwrap().messages;
        assert_eq!((close[0].id, close[1].id), (0x66, 0x67));
        assert!(plan(BRAVE, &HudAction::MenuCursor(4), NO_SELECTION).is_err());
        let bad = [MenuCommand::new(22, 1.0); MENU_ROWS];
        assert!(plan(BRAVE, &HudAction::MenuOpen(bad), NO_SELECTION).is_err());
    }

    #[test]
    fn an_action_the_donor_lacks_is_refused_by_name() {
        let message = plan(PICKEL, &HudAction::Gauge(0.5), NO_SELECTION).unwrap_err();
        assert_eq!(message, "the pickel HUD has no Gauge");
        assert!(plan(JACK, &HudAction::MenuClose, NO_SELECTION).is_err());
        assert_eq!(
            plan(JACK, &HudAction::Play("in"), NO_SELECTION).unwrap_err(),
            "the jack HUD has no Play"
        );
    }

    #[test]
    fn layout_paths_are_lowercase_arc_paths() {
        assert!(check_layout("ui/layout/hud/my_fighter/layout.arc").is_ok());
        assert!(check_layout("ui/layout/hud/My_Fighter/layout.arc").is_err());
        assert!(check_layout("ui/layout/hud/my_fighter/layout.bflyt").is_err());
        assert!(check_layout("/ui/layout.arc").is_err());
        assert!(check_layout("ui//layout.arc").is_err());
        assert!(check_layout("ui\\layout.arc").is_err());
    }

    #[test]
    fn layout_calls_name_a_part() {
        assert!(check_part(&HudAction::Play("charge_in")).is_ok());
        assert!(check_part(&HudAction::Text("count", "12")).is_ok());
        assert!(check_part(&HudAction::Gauge(0.5)).unwrap_err().contains("not a call"));
        assert!(check_part(&HudAction::Show("", true)).is_err());
        assert!(check_part(&HudAction::Show("p%d", true)).is_err());
        assert!(check_part(&HudAction::Show(&"x".repeat(64), true)).is_err());
        assert!(check_part(&HudAction::Frame("gauge", f32::NAN)).is_err());
        assert!(check_part(&HudAction::Text("count", "a\0b")).is_err());
        assert_eq!(HudAction::Frame("gauge", 3.0).part(), Some("gauge"));
        assert_eq!(HudAction::MenuClose.part(), None);
    }
}
