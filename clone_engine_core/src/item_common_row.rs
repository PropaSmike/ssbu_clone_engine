pub use crate::item_common_row_table::{
    HashField, Word, WordKind, HASHES, HASHES_BASE, HASHES_STRIDE, LABELS, WORDS, WORDS_BASE, WORDS_STRIDE,
};

pub const FLOATS_BASE: usize = 0x3908;
pub const FLOATS_STRIDE: usize = 0x284;
pub const NATIVE_KINDS: usize = 432;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Table {
    Float,
    Word,
    Hash,
}

impl Table {
    pub const fn base(self) -> usize {
        match self {
            Table::Float => FLOATS_BASE,
            Table::Word => WORDS_BASE,
            Table::Hash => HASHES_BASE,
        }
    }

    pub const fn stride(self) -> usize {
        match self {
            Table::Float => FLOATS_STRIDE,
            Table::Word => WORDS_STRIDE,
            Table::Hash => HASHES_STRIDE,
        }
    }

    pub const fn width(self) -> usize {
        match self {
            Table::Hash => 8,
            Table::Float | Table::Word => 4,
        }
    }

    pub const fn cell(self, kind: usize, offset: u32) -> usize {
        self.base() + kind * self.stride() + offset as usize
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reject {
    UnknownField,
    NotABool,
    NotAWord,
    NotAHash,
    UnknownLabel,
}

pub fn word(hash: u64) -> Option<&'static Word> {
    WORDS.iter().find(|word| word.hash == hash)
}

pub fn hash_field(hash: u64) -> Option<&'static HashField> {
    HASHES.iter().find(|field| field.hash == hash)
}

pub fn label_value(label: u64) -> Option<u32> {
    LABELS
        .binary_search_by(|(known, _, _)| known.cmp(&label))
        .ok()
        .map(|index| LABELS[index].1)
}

pub fn resolve_i32(field: u64, value: i32) -> Result<(u32, u32), Reject> {
    let word = word(field).ok_or(Reject::UnknownField)?;
    if word.kind == WordKind::Bool && !(0..=1).contains(&value) {
        return Err(Reject::NotABool);
    }
    Ok((word.offset, value as u32))
}

pub fn resolve_label(field: u64, label: u64) -> Result<(u32, u32), Reject> {
    let word = word(field).ok_or(Reject::UnknownField)?;
    if word.kind != WordKind::Enum {
        return Err(Reject::NotAWord);
    }
    let value = label_value(label).ok_or(Reject::UnknownLabel)?;
    Ok((word.offset, value))
}

pub fn resolve_hash(field: u64, value: u64) -> Result<(u32, u64), Reject> {
    let field = hash_field(field).ok_or(Reject::UnknownField)?;
    if value >> 40 != 0 {
        return Err(Reject::NotAHash);
    }
    Ok((field.offset, value))
}

pub fn describe(field: u64) -> Option<(Table, &'static str)> {
    if let Some(word) = word(field) {
        return Some((Table::Word, word.name));
    }
    hash_field(field).map(|field| (Table::Hash, field.name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::hash40;

    #[test]
    fn every_named_field_hashes_to_its_name_and_fits_its_table() {
        for word in WORDS.iter() {
            if !word.name.starts_with("unk_") {
                assert_eq!(word.hash, hash40(word.name), "{}", word.name);
            }
            assert!(word.offset as usize + 4 <= WORDS_STRIDE, "{}", word.name);
            assert_eq!(word.offset % 4, 0, "{}", word.name);
        }
        for field in HASHES.iter() {
            if !field.name.starts_with("unk_") {
                assert_eq!(field.hash, hash40(field.name), "{}", field.name);
            }
            assert!(field.offset as usize + 8 <= HASHES_STRIDE, "{}", field.name);
            assert_eq!(field.offset % 8, 0, "{}", field.name);
        }
        assert_eq!(WORDS.len(), WORDS_STRIDE / 4);
        assert_eq!(HASHES.len(), HASHES_STRIDE / 8);
    }

    #[test]
    fn offsets_are_unique_within_each_table() {
        let mut words: Vec<u32> = WORDS.iter().map(|word| word.offset).collect();
        words.sort_unstable();
        words.dedup();
        assert_eq!(words.len(), WORDS.len());
        let mut hashes: Vec<u32> = HASHES.iter().map(|field| field.offset).collect();
        hashes.sort_unstable();
        hashes.dedup();
        assert_eq!(hashes.len(), HASHES.len());
    }

    #[test]
    fn the_word_table_holds_the_prc_row_minus_its_floats_and_hashes() {
        let bools = WORDS.iter().filter(|word| word.kind == WordKind::Bool).count();
        let ints = WORDS.iter().filter(|word| word.kind == WordKind::Int).count();
        let enums = WORDS.iter().filter(|word| word.kind == WordKind::Enum).count();
        assert_eq!((bools, ints, enums), (16, 4, 23));
        assert_eq!(HASHES.len(), 30);
        assert_eq!(hash_field(hash40("name_hash")).map(|f| f.offset), Some(0));
    }

    #[test]
    fn labels_are_sorted_and_resolve() {
        assert!(LABELS.windows(2).all(|pair| pair[0].0 < pair[1].0));
        for (hash, _, name) in LABELS.iter() {
            if !name.is_empty() {
                assert_eq!(*hash, hash40(name), "{name}");
            }
        }
        assert_eq!(label_value(hash40("item_have_kind_have")), Some(3));
        assert_eq!(label_value(hash40("item_size_kind_large")), Some(1));
        assert_eq!(label_value(hash40("item_trait_flag_throw")), Some(8));
        assert_eq!(label_value(hash40("no_such_label")), None);
    }

    #[test]
    fn resolution_checks_the_field_kind() {
        let have_kind = hash40("have_kind");
        assert_eq!(resolve_label(have_kind, hash40("item_have_kind_grip")), Ok((0x54, 5)));
        assert_eq!(resolve_i32(have_kind, 5), Ok((0x54, 5)));
        assert_eq!(resolve_i32(hash40("eatable"), 2).err(), Some(Reject::NotABool));
        assert_eq!(resolve_i32(hash40("eatable"), 0), Ok((0x10, 0)));
        assert_eq!(resolve_label(hash40("ai_pri"), hash40("item_have_kind_grip")).err(), Some(Reject::NotAWord));
        assert_eq!(resolve_label(have_kind, hash40("nothing")).err(), Some(Reject::UnknownLabel));
        assert_eq!(resolve_hash(hash40("thrown_node"), hash40("top")), Ok((0x30, hash40("top"))));
        assert_eq!(resolve_hash(hash40("thrown_node"), 1 << 41).err(), Some(Reject::NotAHash));
        assert_eq!(resolve_hash(hash40("scale"), hash40("top")).err(), Some(Reject::UnknownField));
        assert_eq!(resolve_i32(hash40("scale"), 1).err(), Some(Reject::UnknownField));
        assert_eq!(describe(hash40("thrown_node")), Some((Table::Hash, "thrown_node")));
    }

    #[test]
    fn the_three_tables_tile_the_accessor_without_touching_each_other() {
        assert_eq!(Table::Float.cell(NATIVE_KINDS, 0), WORDS_BASE);
        assert_eq!(Table::Word.cell(NATIVE_KINDS, 0), HASHES_BASE);
        assert_eq!(Table::Hash.cell(NATIVE_KINDS, 0), 0x72f08);
        assert_eq!(Table::Word.cell(0x3f, 0x8), WORDS_BASE + 0x3f * WORDS_STRIDE + 8);
    }
}
