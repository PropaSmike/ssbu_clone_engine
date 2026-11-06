use crate::hash40;

pub const ENTRY_SIZE: usize = 0x38;
pub const ENTRY_WORDS: usize = ENTRY_SIZE / 8;
pub const MAX_PER_ARTICLE: usize = 16;
pub const SETUP_SLOT: u32 = 3;
pub const OBJECT_MODULE_ACCESSOR: usize = 0x20;
pub const ACCESSOR_HIT_MODULE: usize = 0xb0;
pub const HIT_MODULE_SET_HIT_DATA: usize = 0x78;

pub const PARTS: &[(&str, i32)] = &[
    ("body", 0),
    ("etc", 1),
    ("legs", 2),
    ("body_legs", 3),
    ("head", 4),
];
pub const HEIGHTS: &[(&str, i32)] = &[("low", 0), ("center", 1), ("high", 2)];
pub const STATUSES: &[(&str, i32)] = &[
    ("normal", 0),
    ("invincible", 1),
    ("xlu", 2),
    ("off", 3),
];
pub const SHAPES: &[(&str, i32)] = &[("sphere", 0), ("aabb", 1), ("capsule", 2)];

const PART_PREFIX: &str = "COLLISION_PART_";
const HEIGHT_PREFIX: &str = "HIT_HEIGHT_";
const STATUS_PREFIX: &str = "HIT_STATUS_";
const SHAPE_PREFIX: &str = "COLLISION_SHAPE_TYPE_";

pub const DEFAULT_PART: i32 = 1;
pub const DEFAULT_HEIGHT: i32 = 1;
pub const DEFAULT_STATUS: i32 = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Part,
    Height,
    Status,
    Shape,
}

impl Field {
    fn table(self) -> &'static [(&'static str, i32)] {
        match self {
            Self::Part => PARTS,
            Self::Height => HEIGHTS,
            Self::Status => STATUSES,
            Self::Shape => SHAPES,
        }
    }

    fn prefix(self) -> &'static str {
        match self {
            Self::Part => PART_PREFIX,
            Self::Height => HEIGHT_PREFIX,
            Self::Status => STATUS_PREFIX,
            Self::Shape => SHAPE_PREFIX,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Part => "part",
            Self::Height => "height",
            Self::Status => "status",
            Self::Shape => "shape",
        }
    }

    pub fn value(self, name: &str) -> Option<i32> {
        let name = name.trim();
        let upper = name.to_ascii_uppercase();
        let bare = upper.strip_prefix(self.prefix()).unwrap_or(&upper);
        self.table()
            .iter()
            .find(|(known, _)| known.eq_ignore_ascii_case(bare))
            .map(|(_, value)| *value)
    }

    pub fn name(self, value: i32) -> Option<&'static str> {
        self.table()
            .iter()
            .find(|(_, known)| *known == value)
            .map(|(name, _)| *name)
    }

    pub fn names(self) -> String {
        self.table()
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hurtbox {
    pub node: String,
    pub offset: [f32; 3],
    pub offset2: Option<[f32; 3]>,
    pub size: f32,
    pub part: i32,
    pub height: i32,
    pub status: i32,
    pub shape: Option<i32>,
}

impl Default for Hurtbox {
    fn default() -> Self {
        Self {
            node: String::new(),
            offset: [0.0; 3],
            offset2: None,
            size: 0.0,
            part: DEFAULT_PART,
            height: DEFAULT_HEIGHT,
            status: DEFAULT_STATUS,
            shape: None,
        }
    }
}

impl Hurtbox {
    pub fn new(node: &str, size: f32) -> Self {
        Self {
            node: node.to_string(),
            size,
            ..Self::default()
        }
    }

    pub fn offset(mut self, x: f32, y: f32, z: f32) -> Self {
        self.offset = [x, y, z];
        self
    }

    pub fn offset2(mut self, x: f32, y: f32, z: f32) -> Self {
        self.offset2 = Some([x, y, z]);
        self
    }

    pub fn part(mut self, part: i32) -> Self {
        self.part = part;
        self
    }

    pub fn height(mut self, height: i32) -> Self {
        self.height = height;
        self
    }

    pub fn status(mut self, status: i32) -> Self {
        self.status = status;
        self
    }

    pub fn shape(mut self, shape: i32) -> Self {
        self.shape = Some(shape);
        self
    }

    pub fn resolved_shape(&self) -> i32 {
        self.shape
            .unwrap_or(if self.offset2.is_some() { 2 } else { 0 })
    }

    pub fn end(&self) -> [f32; 3] {
        self.offset2.unwrap_or(self.offset)
    }

    pub fn check(&self) -> Result<(), String> {
        if self.node.trim().is_empty() {
            return Err("a hurtbox needs a node".to_string());
        }
        if !self.size.is_finite() || self.size <= 0.0 {
            return Err(format!("hurtbox on {}: size must be above 0", self.node));
        }
        if self.offset.iter().chain(self.end().iter()).any(|value| !value.is_finite()) {
            return Err(format!("hurtbox on {}: offsets must be numbers", self.node));
        }
        for (field, value) in [
            (Field::Part, self.part),
            (Field::Height, self.height),
            (Field::Status, self.status),
            (Field::Shape, self.resolved_shape()),
        ] {
            if field.name(value).is_none() {
                return Err(format!(
                    "hurtbox on {}: {} {value} is not one of {}",
                    self.node,
                    field.key(),
                    field.names()
                ));
            }
        }
        Ok(())
    }

    pub fn node_hash(&self) -> u64 {
        hash40(&self.node.trim().to_ascii_lowercase())
    }

    pub fn entry(&self) -> [u8; ENTRY_SIZE] {
        let mut out = [0u8; ENTRY_SIZE];
        let end = self.end();
        for (index, value) in self.offset.iter().chain(end.iter()).enumerate() {
            out[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        out[0x18..0x1c].copy_from_slice(&self.size.to_le_bytes());
        out[0x20..0x28].copy_from_slice(&self.node_hash().to_le_bytes());
        out[0x28..0x2c].copy_from_slice(&(self.part as u32).to_le_bytes());
        out[0x2c..0x30].copy_from_slice(&(self.height as u32).to_le_bytes());
        out[0x30..0x34].copy_from_slice(&(self.status as u32).to_le_bytes());
        out[0x34..0x38].copy_from_slice(&(self.resolved_shape() as u32).to_le_bytes());
        out
    }

    pub fn entry_words(&self) -> [u64; ENTRY_WORDS] {
        let bytes = self.entry();
        let mut words = [0u64; ENTRY_WORDS];
        for (index, word) in words.iter_mut().enumerate() {
            let mut chunk = [0u8; 8];
            chunk.copy_from_slice(&bytes[index * 8..index * 8 + 8]);
            *word = u64::from_le_bytes(chunk);
        }
        words
    }

    pub fn lines(&self) -> Vec<String> {
        let vector = |values: [f32; 3]| {
            format!(
                "[{}, {}, {}]",
                float(values[0]),
                float(values[1]),
                float(values[2])
            )
        };
        let mut lines = vec![
            format!("node = \"{}\"", self.node.replace('\\', "\\\\").replace('"', "\\\"")),
            format!("size = {}", float(self.size)),
        ];
        if self.offset != [0.0; 3] {
            lines.push(format!("offset = {}", vector(self.offset)));
        }
        if let Some(offset2) = self.offset2 {
            lines.push(format!("offset2 = {}", vector(offset2)));
        }
        for (field, value, default) in [
            (Field::Part, self.part, Some(DEFAULT_PART)),
            (Field::Height, self.height, Some(DEFAULT_HEIGHT)),
            (Field::Status, self.status, Some(DEFAULT_STATUS)),
            (Field::Shape, self.shape.unwrap_or(-1), None),
        ] {
            if value < 0 || Some(value) == default {
                continue;
            }
            match field.name(value) {
                Some(name) => lines.push(format!("{} = \"{name}\"", field.key())),
                None => lines.push(format!("{} = {value}", field.key())),
            }
        }
        lines
    }

    pub fn describe(&self) -> String {
        let shape = Field::Shape.name(self.resolved_shape()).unwrap_or("?");
        let mut text = format!("{shape} {} on {}", self.size, self.node);
        if self.offset != [0.0; 3] || self.offset2.is_some() {
            text.push_str(&format!(" {:?}", self.offset));
        }
        if let Some(offset2) = self.offset2 {
            text.push_str(&format!("..{offset2:?}"));
        }
        text
    }
}

fn float(value: f32) -> String {
    let text = format!("{value:?}");
    if text.contains('.') || text.contains('e') || text.contains("inf") || text.contains("NaN") {
        text
    } else {
        format!("{text}.0")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(bytes: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
    }

    fn real(bytes: &[u8], at: usize) -> f32 {
        f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
    }

    #[test]
    fn the_hydrant_entry_measured_on_eden_encodes_byte_for_byte() {
        let hydrant = Hurtbox::new("hip", 4.0).offset(0.0, 4.0, 0.0).offset2(0.0, 9.0, 0.0);
        let bytes = hydrant.entry();
        assert_eq!(real(&bytes, 0x04), 4.0);
        assert_eq!(real(&bytes, 0x10), 9.0);
        assert_eq!(real(&bytes, 0x18), 4.0);
        assert_eq!(word(&bytes, 0x1c), 0);
        assert_eq!(u64::from_le_bytes(bytes[0x20..0x28].try_into().unwrap()), hash40("hip"));
        assert_eq!(word(&bytes, 0x28), 1);
        assert_eq!(word(&bytes, 0x2c), 1);
        assert_eq!(word(&bytes, 0x30), 0);
        assert_eq!(word(&bytes, 0x34), 2);
    }

    #[test]
    fn a_single_point_is_a_sphere_and_repeats_its_offset() {
        let ball = Hurtbox::new("Top", 2.5).offset(1.0, 2.0, 3.0);
        assert_eq!(ball.resolved_shape(), 0);
        let bytes = ball.entry();
        assert_eq!(real(&bytes, 0x0c), 1.0);
        assert_eq!(real(&bytes, 0x14), 3.0);
        assert_eq!(ball.node_hash(), hash40("top"));
    }

    #[test]
    fn words_match_bytes() {
        let hurtbox = Hurtbox::new("hip", 4.0).offset2(0.0, 9.0, 0.0);
        let words = hurtbox.entry_words();
        let bytes = hurtbox.entry();
        assert_eq!(words[4], u64::from_le_bytes(bytes[0x20..0x28].try_into().unwrap()));
    }

    #[test]
    fn names_accept_bare_and_prefixed_forms() {
        assert_eq!(Field::Part.value("etc"), Some(1));
        assert_eq!(Field::Part.value("COLLISION_PART_HEAD"), Some(4));
        assert_eq!(Field::Height.value("High"), Some(2));
        assert_eq!(Field::Status.value("HIT_STATUS_XLU"), Some(2));
        assert_eq!(Field::Shape.value("capsule"), Some(2));
        assert_eq!(Field::Shape.value("cube"), None);
    }

    #[test]
    fn check_refuses_what_the_game_cannot_read() {
        assert!(Hurtbox::new("", 1.0).check().is_err());
        assert!(Hurtbox::new("hip", 0.0).check().is_err());
        assert!(Hurtbox::new("hip", 1.0).part(9).check().is_err());
        assert!(Hurtbox::new("hip", 1.0).offset(f32::NAN, 0.0, 0.0).check().is_err());
        assert!(Hurtbox::new("hip", 1.0).check().is_ok());
    }

    #[test]
    fn lines_leave_out_defaults() {
        let lines = Hurtbox::new("hip", 4.0).offset2(0.0, 9.0, 0.0).status(2).lines();
        assert_eq!(
            lines,
            vec![
                "node = \"hip\"".to_string(),
                "size = 4.0".to_string(),
                "offset2 = [0.0, 9.0, 0.0]".to_string(),
                "status = \"xlu\"".to_string(),
            ]
        );
    }
}
