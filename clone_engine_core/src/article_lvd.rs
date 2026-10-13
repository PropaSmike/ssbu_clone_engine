use crate::hash40;

pub const COSTUMES: usize = 8;

pub const SETUP_SLOT: u32 = 8;
pub const MAX_VERTICES: usize = 32;
pub const MAX_VERTEX_INDEX: u32 = 0xffff;
pub const ENTRY_WORDS: usize = 2;
pub const ACCESSOR_GROUND_MODULE: usize = 0x58;
pub const GROUND_FILE_INDEX: usize = 0x128;
pub const GROUND_FIRST_GROUP: usize = 0x130;
pub const GROUND_FIRST_CONTROL: usize = 0x140;
pub const GROUND_SECOND_GROUP: usize = 0x148;
pub const GROUND_SECOND_CONTROL: usize = 0x158;
pub const GROUND_DESCRIPTOR: usize = 0x168;
pub const GROUP_BODY: usize = 0x8;
pub const GROUP_VERTEX_COUNT: usize = 0x60;
pub const GROUND_FOLLOW_SLOT: usize = 18;
pub const SPEC_SLOT: u32 = 55;
pub const SPEC_VIRTUAL_JOINTS: usize = 0xb8;
pub const VIRTUAL_JOINT_WORDS: usize = 5;
pub const VIRTUAL_JOINT_GROUP_TAG: u32 = 10000;

#[derive(Clone, Debug, PartialEq)]
pub struct LvdVertex {
    pub vertex: u32,
    pub joint: String,
    pub offset: Option<[f32; 3]>,
    pub joint_left: Option<String>,
}

impl LvdVertex {
    pub fn new(vertex: u32, joint: &str) -> Self {
        Self {
            vertex,
            joint: joint.to_string(),
            offset: None,
            joint_left: None,
        }
    }

    pub fn offset(mut self, x: f32, y: f32, z: f32) -> Self {
        self.offset = Some([x, y, z]);
        self
    }

    pub fn joint_left(mut self, joint: &str) -> Self {
        self.joint_left = Some(joint.to_string());
        self
    }

    pub fn anchor(&self) -> u64 {
        match self.offset {
            Some(_) => anchor_hash(self.vertex),
            None => joint_hash(&self.joint),
        }
    }

    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("vertex = {}", self.vertex),
            format!("joint = \"{}\"", quote(&self.joint)),
        ];
        if let Some([x, y, z]) = self.offset {
            lines.push(format!("offset = [{}, {}, {}]", float(x), float(y), float(z)));
        }
        if let Some(left) = &self.joint_left {
            lines.push(format!("joint_left = \"{}\"", quote(left)));
        }
        lines
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ArticleLvd {
    pub file: String,
    pub vertices: Vec<LvdVertex>,
}

impl ArticleLvd {
    pub fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            vertices: Vec::new(),
        }
    }

    pub fn vertex(mut self, vertex: u32, joint: &str) -> Self {
        self.vertices.push(LvdVertex::new(vertex, joint));
        self
    }

    pub fn vertex_at(mut self, vertex: u32, joint: &str, x: f32, y: f32, z: f32) -> Self {
        self.vertices.push(LvdVertex::new(vertex, joint).offset(x, y, z));
        self
    }

    pub fn vertex_facing(mut self, vertex: u32, joint: &str, joint_left: &str) -> Self {
        self.vertices
            .push(LvdVertex::new(vertex, joint).joint_left(joint_left));
        self
    }

    pub fn file_name(&self) -> &str {
        self.file.trim()
    }

    pub fn check(&self) -> Result<(), String> {
        let file = self.file_name();
        if file.is_empty()
            || file.contains('/')
            || file.contains('\\')
            || !file.to_ascii_lowercase().ends_with(".lvd")
            || file.len() <= ".lvd".len()
        {
            return Err(format!(
                "lvd = \"{file}\" must be a file name in the article's model folder, like \"platform.lvd\""
            ));
        }
        if self.vertices.is_empty() {
            return Err(format!(
                "lvd {file} needs at least one lvd_vertex tying a vertex to a joint"
            ));
        }
        if self.vertices.len() > MAX_VERTICES {
            return Err(format!("lvd {file}: at most {MAX_VERTICES} lvd_vertex entries"));
        }
        for (index, vertex) in self.vertices.iter().enumerate() {
            if vertex.vertex > MAX_VERTEX_INDEX {
                return Err(format!(
                    "lvd {file}: vertex {} is above {MAX_VERTEX_INDEX}",
                    vertex.vertex
                ));
            }
            if vertex.joint.trim().is_empty()
                || vertex.joint_left.as_deref().is_some_and(|left| left.trim().is_empty())
            {
                return Err(format!("lvd {file}: vertex {} needs a joint", vertex.vertex));
            }
            if vertex
                .offset
                .is_some_and(|offset| offset.iter().any(|value| !value.is_finite()))
            {
                return Err(format!("lvd {file}: vertex {} offset must be numbers", vertex.vertex));
            }
            if self.vertices[..index]
                .iter()
                .any(|earlier| earlier.vertex == vertex.vertex)
            {
                return Err(format!("lvd {file}: vertex {} is listed twice", vertex.vertex));
            }
        }
        Ok(())
    }

    pub fn highest_vertex(&self) -> Option<u32> {
        self.vertices.iter().map(|vertex| vertex.vertex).max()
    }

    pub fn right(&self) -> Vec<(u32, u64)> {
        self.vertices
            .iter()
            .map(|vertex| (vertex.vertex, vertex.anchor()))
            .collect()
    }

    pub fn left(&self) -> Vec<(u32, u64)> {
        let count = self.vertices.len();
        self.vertices
            .iter()
            .enumerate()
            .map(|(index, vertex)| {
                let joint = match &vertex.joint_left {
                    Some(left) => joint_hash(left),
                    None => self.vertices[count - 1 - index].anchor(),
                };
                (vertex.vertex, joint)
            })
            .collect()
    }

    pub fn virtual_joint_words(&self) -> Vec<u64> {
        self.vertices
            .iter()
            .filter_map(|vertex| {
                let [x, y, z] = vertex.offset?;
                Some([
                    anchor_hash(vertex.vertex),
                    joint_hash(&vertex.joint),
                    pair(x, y),
                    pair(z, 0.0),
                    0,
                ])
            })
            .flatten()
            .collect()
    }

    pub fn entry_words(entries: &[(u32, u64)]) -> Vec<u64> {
        entries
            .iter()
            .flat_map(|(vertex, joint)| [*vertex as u64, *joint])
            .collect()
    }

    pub fn describe(&self) -> String {
        let pairs: Vec<String> = self
            .vertices
            .iter()
            .map(|vertex| {
                let mut text = format!("{}<-{}", vertex.vertex, vertex.joint);
                if let Some(offset) = vertex.offset {
                    text.push_str(&format!("+{offset:?}"));
                }
                if let Some(left) = &vertex.joint_left {
                    text.push_str(&format!("/{left}"));
                }
                text
            })
            .collect();
        format!("{} ({})", self.file_name(), pairs.join(" "))
    }
}

pub fn joint_hash(joint: &str) -> u64 {
    hash40(&joint.trim().to_ascii_lowercase())
}

pub fn anchor_hash(vertex: u32) -> u64 {
    hash40(&format!("clone_engine_lvd_vertex_{vertex}"))
}

fn pair(low: f32, high: f32) -> u64 {
    low.to_bits() as u64 | ((high.to_bits() as u64) << 32)
}

fn quote(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn float(value: f32) -> String {
    let text = format!("{value:?}");
    if text.contains('.') || text.contains('e') {
        text
    } else {
        format!("{text}.0")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Remap {
    pub source: u32,
    pub kind: i32,
    pub clone: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemapTable {
    entries: Vec<Remap>,
}

impl RemapTable {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn note(&mut self, source: u32, kind: i32, clone: u32) -> bool {
        if self
            .entries
            .iter()
            .any(|entry| entry.source == source && entry.kind == kind)
        {
            return false;
        }
        self.entries.push(Remap {
            source,
            kind,
            clone,
        });
        true
    }

    pub fn lookup(&self, source: u32, kind: Option<i32>) -> Option<u32> {
        let mut matching = self.entries.iter().filter(|entry| entry.source == source);
        match kind {
            Some(kind) => matching.find(|entry| entry.kind == kind),
            None => matching.next(),
        }
        .map(|entry| entry.clone)
    }

    pub fn clones_of(&self, kind: i32) -> Vec<u32> {
        let mut out: Vec<u32> = self
            .entries
            .iter()
            .filter(|entry| entry.kind == kind)
            .map(|entry| entry.clone)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub fn nearest_costume(shipped: &[Option<u32>], costume: usize) -> Option<(usize, u32)> {
    shipped
        .iter()
        .enumerate()
        .filter_map(|(slot, index)| index.map(|index| (slot, index)))
        .min_by_key(|(slot, _)| (slot.abs_diff(costume), *slot))
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub chosen: Vec<Option<(usize, u32)>>,
    pub borrowed: Vec<(usize, usize)>,
}

pub fn plan(shipped: &[Option<u32>]) -> Plan {
    let mut out = Plan::default();
    for costume in 0..shipped.len() {
        let chosen = nearest_costume(shipped, costume);
        if let Some((from, _)) = chosen {
            if from != costume {
                out.borrowed.push((costume, from));
            }
        }
        out.chosen.push(chosen);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE_C00: u32 = 0x3604e;
    const SOURCE_C01: u32 = 0x3604f;
    const VANILLA_KIND: i32 = 0x140;

    #[test]
    fn three_clones_of_one_source_each_keep_their_own_file() {
        let mut table = RemapTable::new();
        assert!(table.note(SOURCE_C00, 621, 0x91434));
        assert!(table.note(SOURCE_C00, 622, 0x91500));
        assert!(table.note(SOURCE_C00, 623, 0x91600));
        assert_eq!(table.lookup(SOURCE_C00, Some(621)), Some(0x91434));
        assert_eq!(table.lookup(SOURCE_C00, Some(622)), Some(0x91500));
        assert_eq!(table.lookup(SOURCE_C00, Some(623)), Some(0x91600));
    }

    #[test]
    fn the_vanilla_article_never_gets_a_clone_file() {
        let mut table = RemapTable::new();
        table.note(SOURCE_C00, 621, 0x91434);
        assert_eq!(table.lookup(SOURCE_C00, Some(VANILLA_KIND)), None);
    }

    #[test]
    fn an_unreadable_kind_keeps_the_first_registration() {
        let mut table = RemapTable::new();
        table.note(SOURCE_C00, 621, 0x91434);
        table.note(SOURCE_C00, 622, 0x91500);
        assert_eq!(table.lookup(SOURCE_C00, None), Some(0x91434));
    }

    #[test]
    fn a_repeated_note_is_ignored() {
        let mut table = RemapTable::new();
        assert!(table.note(SOURCE_C00, 621, 0x91434));
        assert!(!table.note(SOURCE_C00, 621, 0x99999));
        assert_eq!(table.len(), 1);
        assert_eq!(table.lookup(SOURCE_C00, Some(621)), Some(0x91434));
    }

    #[test]
    fn costumes_are_kept_apart() {
        let mut table = RemapTable::new();
        table.note(SOURCE_C00, 621, 0x91434);
        table.note(SOURCE_C01, 621, 0x91435);
        assert_eq!(table.lookup(SOURCE_C01, Some(621)), Some(0x91435));
    }

    #[test]
    fn an_unknown_source_is_not_remapped() {
        let mut table = RemapTable::new();
        table.note(SOURCE_C00, 621, 0x91434);
        assert_eq!(table.lookup(0x12345, Some(621)), None);
    }

    #[test]
    fn clones_of_lists_each_file_once() {
        let mut table = RemapTable::new();
        table.note(SOURCE_C00, 621, 0x91434);
        table.note(SOURCE_C01, 621, 0x91434);
        table.note(SOURCE_C00, 622, 0x91500);
        assert_eq!(table.clones_of(621), vec![0x91434]);
        assert_eq!(table.clones_of(622), vec![0x91500]);
        assert!(table.clones_of(999).is_empty());
    }

    #[test]
    fn a_costume_that_ships_its_own_file_uses_it() {
        let shipped = [Some(10), Some(11), Some(12)];
        assert_eq!(nearest_costume(&shipped, 1), Some((1, 11)));
    }

    #[test]
    fn a_missing_costume_borrows_the_nearest() {
        let shipped = [Some(10), None, None, None, Some(14), None, None, None];
        assert_eq!(nearest_costume(&shipped, 1), Some((0, 10)));
        assert_eq!(nearest_costume(&shipped, 3), Some((4, 14)));
        assert_eq!(nearest_costume(&shipped, 7), Some((4, 14)));
    }

    #[test]
    fn a_tie_goes_to_the_lower_costume() {
        let shipped = [Some(10), None, Some(12)];
        assert_eq!(nearest_costume(&shipped, 1), Some((0, 10)));
    }

    #[test]
    fn nothing_shipped_is_nothing_chosen() {
        let shipped = [None; COSTUMES];
        assert_eq!(nearest_costume(&shipped, 0), None);
        let plan = plan(&shipped);
        assert!(plan.chosen.iter().all(Option::is_none));
        assert!(plan.borrowed.is_empty());
    }

    #[test]
    fn only_c00_shipped_lends_it_to_every_costume() {
        let mut shipped = [None; COSTUMES];
        shipped[0] = Some(0x91434);
        let plan = plan(&shipped);
        assert!(plan.chosen.iter().all(|c| *c == Some((0, 0x91434))));
        assert_eq!(
            plan.borrowed,
            (1..COSTUMES).map(|c| (c, 0)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn facing_left_reverses_the_joints_like_the_hydrant() {
        let lvd = ArticleLvd::new("firehydrant.lvd")
            .vertex(1, "front")
            .vertex(0, "back");
        assert_eq!(lvd.right(), vec![(1, joint_hash("front")), (0, joint_hash("back"))]);
        assert_eq!(lvd.left(), vec![(1, joint_hash("back")), (0, joint_hash("front"))]);
    }

    #[test]
    fn an_explicit_left_joint_wins() {
        let lvd = ArticleLvd::new("p.lvd")
            .vertex_facing(0, "a", "c")
            .vertex(1, "b");
        assert_eq!(lvd.left(), vec![(0, joint_hash("c")), (1, joint_hash("a"))]);
    }

    #[test]
    fn entries_are_vertex_then_joint_words() {
        let lvd = ArticleLvd::new("p.lvd").vertex(3, "Top");
        assert_eq!(
            ArticleLvd::entry_words(&lvd.right()),
            vec![3, hash40("top")]
        );
    }

    #[test]
    fn an_offset_vertex_rides_a_virtual_joint_laid_out_like_the_hydrants() {
        let lvd = ArticleLvd::new("p.lvd")
            .vertex_at(1, "top", 0.0, 13.0, 4.0)
            .vertex_at(0, "top", 0.0, 13.0, -4.0);
        assert_eq!(lvd.right(), vec![(1, anchor_hash(1)), (0, anchor_hash(0))]);
        assert_eq!(lvd.left(), vec![(1, anchor_hash(0)), (0, anchor_hash(1))]);
        let words = lvd.virtual_joint_words();
        assert_eq!(words.len(), 2 * VIRTUAL_JOINT_WORDS);
        assert_eq!(words[0], anchor_hash(1));
        assert_eq!(words[1], 0x31ed91fca);
        assert_eq!(f32::from_bits(words[2] as u32), 0.0);
        assert_eq!(f32::from_bits((words[2] >> 32) as u32), 13.0);
        assert_eq!(f32::from_bits(words[3] as u32), 4.0);
        assert_eq!(words[3] >> 32, 0);
        assert_eq!(words[4], 0);
        assert_eq!(f32::from_bits(words[8] as u32), -4.0);
    }

    #[test]
    fn plain_joints_make_no_virtual_joints() {
        assert!(ArticleLvd::new("p.lvd").vertex(0, "a").virtual_joint_words().is_empty());
        assert!(ArticleLvd::new("p.lvd")
            .vertex_at(0, "a", f32::NAN, 0.0, 0.0)
            .check()
            .is_err());
    }

    #[test]
    fn check_refuses_what_the_engine_cannot_place() {
        assert!(ArticleLvd::new("p.lvd").vertex(0, "a").check().is_ok());
        assert!(ArticleLvd::new("p.lvd").check().is_err());
        assert!(ArticleLvd::new("model/p.lvd").vertex(0, "a").check().is_err());
        assert!(ArticleLvd::new("p.bin").vertex(0, "a").check().is_err());
        assert!(ArticleLvd::new(".lvd").vertex(0, "a").check().is_err());
        assert!(ArticleLvd::new("p.lvd").vertex(0, "a").vertex(0, "b").check().is_err());
        assert!(ArticleLvd::new("p.lvd").vertex(0, " ").check().is_err());
        assert!(ArticleLvd::new("p.lvd").vertex(0x10000, "a").check().is_err());
        let many = (0..=MAX_VERTICES as u32).fold(ArticleLvd::new("p.lvd"), |lvd, vertex| {
            lvd.vertex(vertex, "a")
        });
        assert!(many.check().is_err());
    }

    #[test]
    fn vertex_lines_round_trip_the_keys() {
        assert_eq!(
            LvdVertex::new(1, "platr").joint_left("platl").lines(),
            vec![
                "vertex = 1".to_string(),
                "joint = \"platr\"".to_string(),
                "joint_left = \"platl\"".to_string(),
            ]
        );
    }

    #[test]
    fn a_full_pack_borrows_nothing() {
        let shipped: Vec<Option<u32>> = (0..COSTUMES as u32).map(|c| Some(100 + c)).collect();
        let plan = plan(&shipped);
        assert!(plan.borrowed.is_empty());
        for (costume, chosen) in plan.chosen.iter().enumerate() {
            assert_eq!(*chosen, Some((costume, 100 + costume as u32)));
        }
    }
}
