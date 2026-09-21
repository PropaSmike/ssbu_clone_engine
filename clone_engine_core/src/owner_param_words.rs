#[path = "owner_param_words_table.rs"]
mod owner_param_words_table;

pub use owner_param_words_table::{FIELDS, KIND_COUNT, PARENTS, STARTS, WORDS, WORD_COUNT};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WordKind {
    F32,
    I32,
    U32,
    Bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Word {
    pub offset: u16,
    pub parent: u16,
    pub field: u16,
    pub kind: WordKind,
}

impl Word {
    pub fn parent_name(&self) -> &'static str {
        PARENTS[self.parent as usize]
    }

    pub fn field_name(&self) -> &'static str {
        FIELDS[self.field as usize]
    }

    pub fn path(&self) -> String {
        let parent = self.parent_name();
        if parent.is_empty() {
            self.field_name().to_string()
        } else {
            format!("{parent}/{}", self.field_name())
        }
    }

    pub fn shown(&self, bits: u32) -> String {
        match self.kind {
            WordKind::F32 => format!("{}", f32::from_bits(bits)),
            WordKind::I32 => format!("{}", bits as i32),
            WordKind::U32 | WordKind::Bool => format!("{bits}"),
        }
    }
}

pub fn words_of(kind: i32) -> &'static [Word] {
    if kind < 0 || kind as usize >= KIND_COUNT {
        return &[];
    }
    let from = STARTS[kind as usize] as usize;
    let to = STARTS[kind as usize + 1] as usize;
    &WORDS[from..to]
}

pub fn word_at(kind: i32, offset: u32) -> Option<&'static Word> {
    if offset > u16::MAX as u32 {
        return None;
    }
    let words = words_of(kind);
    words
        .binary_search_by(|word| (word.offset as u32).cmp(&offset))
        .ok()
        .map(|index| &words[index])
}

pub fn name_of(kind: i32, offset: u32) -> Option<String> {
    word_at(kind, offset).map(Word::path)
}

pub fn describe(kind: i32, offset: u32, bits: u32) -> String {
    match word_at(kind, offset) {
        Some(word) => format!("{} = {}", word.path(), word.shown(bits)),
        None => format!("no word this engine knows at +{offset:#x}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_console_words_of_pickel_resolve_by_name_and_type() {
        let life = word_at(88, 0x518).expect("life");
        assert_eq!(life.path(), "param_pickelobject/life");
        assert_eq!(life.kind, WordKind::I32);
        assert_eq!(life.shown(0xe10), "3600");
        let auto = word_at(88, 0x520).expect("auto_damage");
        assert_eq!(auto.path(), "param_pickelobject/auto_damage");
        assert_eq!(auto.kind, WordKind::F32);
        assert_eq!(auto.shown(0x3ca3d70a), "0.02");
        assert_eq!(name_of(88, 0x4f8).as_deref(), Some("param_pickelobject/hp_grade_1"));
        assert_eq!(name_of(88, 0x51c).as_deref(),
                   Some("param_pickelobject/flash_start_frame"));
    }

    #[test]
    fn unknown_offsets_and_kinds_resolve_to_nothing() {
        assert!(word_at(88, 0x51a).is_none());
        assert!(word_at(88, 0x3ffc).is_none());
        assert!(word_at(78, 0x100).is_none());
        assert!(words_of(78).is_empty());
        assert!(words_of(94).is_empty());
        assert!(words_of(-1).is_empty());
        assert!(word_at(88, 0x1_0000).is_none());
        assert_eq!(describe(88, 0x51a, 0), "no word this engine knows at +0x51a");
        assert_eq!(describe(88, 0x518, 0x258), "param_pickelobject/life = 600");
    }

    #[test]
    fn every_run_is_sorted_and_unique_so_the_search_is_sound() {
        let mut total = 0;
        for kind in 0..KIND_COUNT as i32 {
            let words = words_of(kind);
            for pair in words.windows(2) {
                assert!(pair[0].offset < pair[1].offset, "kind {kind}");
            }
            for word in words {
                assert_eq!(word.offset % 4, 0);
                assert!(!word.field_name().is_empty());
            }
            total += words.len();
        }
        assert_eq!(total, WORD_COUNT);
        assert_eq!(STARTS[KIND_COUNT] as usize, WORD_COUNT);
    }
}
