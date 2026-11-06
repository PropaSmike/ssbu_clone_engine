use crate::hash::hash40;

pub const GENERATOR_RANDOM: u32 = 0x36a;
pub const GENERATOR_ASSIST: u32 = 0;
pub const GENERATOR_POKEBALL: u32 = 6;
pub const GENERATOR_MASTERBALL: u32 = 7;
pub const GENERATOR_RANDOM_HASH: u64 = 0x0011_d0ed_cb06;
pub const VARIATION_AUTO: i32 = -1;
pub const LAST_ORDINARY_KIND: i32 = 0x1af;
pub const KIND_MASK_WORDS: usize = 14;

pub const RECORD_SIZE: usize = 0x28;
pub const RECORD_GENERATOR: usize = 0x0;
pub const RECORD_ENTRIES_BEGIN: usize = 0x10;
pub const RECORD_ENTRIES_END: usize = 0x18;
pub const RECORD_ENTRIES_CAPACITY: usize = 0x20;

pub const ENTRY_SIZE: usize = 0x14;
pub const ENTRY_KIND: usize = 0x0;
pub const ENTRY_VARIATION: usize = 0x4;
pub const ENTRY_PER: usize = 0x8;
pub const ENTRY_MIN: usize = 0xc;
pub const ENTRY_MAX: usize = 0x10;

pub const TABLE_VECTOR_BEGIN: usize = 0x8;
pub const TABLE_VECTOR_END: usize = 0x10;
pub const TABLES_IN_LOOKUP_ORDER: [usize; 4] = [0x1cd8, 0x1cb8, 0x1c98, 0x1c78];
pub const TABLE_NAMES: [&str; 4] = ["stage", "pokemon", "assist", "item"];

pub const MAX_COUNT: i32 = 8;
pub const MAX_PER: i32 = 1000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Entry {
    pub kind: i32,
    pub variation: i32,
    pub per: i32,
    pub min: i32,
    pub max: i32,
}

impl Entry {
    pub fn from_bytes(bytes: &[u8]) -> Entry {
        let word = |at: usize| i32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
        Entry {
            kind: word(ENTRY_KIND),
            variation: word(ENTRY_VARIATION),
            per: word(ENTRY_PER),
            min: word(ENTRY_MIN),
            max: word(ENTRY_MAX),
        }
    }

    pub fn write(&self, bytes: &mut [u8]) {
        for (at, value) in [
            (ENTRY_KIND, self.kind),
            (ENTRY_VARIATION, self.variation),
            (ENTRY_PER, self.per),
            (ENTRY_MIN, self.min),
            (ENTRY_MAX, self.max),
        ] {
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reject {
    Per,
    Count,
    Variation,
}

pub fn validate(kind: i32, per: i32, min: i32, max: i32, variation: i32) -> Result<Entry, Reject> {
    if !(0..=MAX_PER).contains(&per) {
        return Err(Reject::Per);
    }
    if min < 0 || max < min || max > MAX_COUNT {
        return Err(Reject::Count);
    }
    if variation < VARIATION_AUTO {
        return Err(Reject::Variation);
    }
    Ok(Entry { kind, variation, per, min, max })
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rule {
    pub public_kind: i32,
    pub base_kind: i32,
    pub generator: u32,
    pub entry: Entry,
}

pub fn kind_enabled(mask: &[u32; KIND_MASK_WORDS], kind: i32) -> bool {
    if !(0..=LAST_ORDINARY_KIND).contains(&kind) {
        return true;
    }
    mask[(kind as usize) >> 5] & (1u32 << (kind & 31)) != 0
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Plan {
    pub entries: Vec<Entry>,
    pub vanilla: usize,
    pub appended: usize,
    pub held_back: usize,
}

pub fn plan(
    generator: u32,
    live: &[Entry],
    rules: &[Rule],
    mask: &[u32; KIND_MASK_WORDS],
    is_clone: impl Fn(i32) -> bool,
) -> Plan {
    let mut entries: Vec<Entry> = live.iter().copied().filter(|entry| !is_clone(entry.kind)).collect();
    let vanilla = entries.len();
    let mut appended = 0;
    let mut held_back = 0;
    for rule in rules.iter().filter(|rule| rule.generator == generator) {
        if kind_enabled(mask, rule.base_kind) {
            entries.push(rule.entry);
            appended += 1;
        } else {
            held_back += 1;
        }
    }
    Plan { entries, vanilla, appended, held_back }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Family {
    Assist,
    Pokemon,
}

pub fn family_for_generator(generator: u32) -> Option<Family> {
    match generator {
        GENERATOR_ASSIST => Some(Family::Assist),
        GENERATOR_POKEBALL | GENERATOR_MASTERBALL => Some(Family::Pokemon),
        _ => None,
    }
}

pub fn pack_lot(kind: i32, variation: i32) -> u64 {
    (kind as u32 as u64) | ((variation as u32 as u64) << 32)
}

pub fn unpack_lot(packed: u64) -> (i32, i32) {
    (packed as u32 as i32, (packed >> 32) as u32 as i32)
}

pub fn generator_label(generator: u32, kind_name: impl Fn(i32) -> Option<String>) -> String {
    if generator == GENERATOR_RANDOM {
        return String::from("item_genid_random");
    }
    match kind_name(generator as i32) {
        Some(name) => format!("item_kind_{name}"),
        None => format!("generator {generator:#x}"),
    }
}

pub fn generator_for_hash(hash: u64, kind_name: impl Fn(i32) -> Option<String>) -> Option<u32> {
    if hash == GENERATOR_RANDOM_HASH {
        return Some(GENERATOR_RANDOM);
    }
    (0..=LAST_ORDINARY_KIND)
        .find(|kind| kind_name(*kind).is_some_and(|name| hash40(&format!("item_kind_{name}")) == hash))
        .map(|kind| kind as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(kind: i32) -> Option<String> {
        ["assist", "barrel", "box", "capsule"].get(kind as usize).map(|name| String::from(*name))
    }

    #[test]
    fn the_random_generator_hash_and_kind_names_resolve() {
        assert_eq!(GENERATOR_RANDOM_HASH, hash40("item_genid_random"));
        assert_eq!(generator_for_hash(GENERATOR_RANDOM_HASH, names), Some(GENERATOR_RANDOM));
        assert_eq!(generator_for_hash(hash40("item_kind_box"), names), Some(2));
        assert_eq!(generator_for_hash(hash40("item_kind_nothing"), names), None);
        assert_eq!(generator_label(GENERATOR_RANDOM, names), "item_genid_random");
        assert_eq!(generator_label(3, names), "item_kind_capsule");
        assert_eq!(generator_label(0x300, names), "generator 0x300");
    }

    #[test]
    fn an_entry_round_trips_through_its_twenty_bytes() {
        let entry = validate(0x36a, 40, 1, 2, VARIATION_AUTO).unwrap();
        let mut bytes = [0u8; ENTRY_SIZE];
        entry.write(&mut bytes);
        assert_eq!(&bytes[..4], &0x36au32.to_le_bytes());
        assert_eq!(&bytes[4..8], &u32::MAX.to_le_bytes());
        assert_eq!(&bytes[8..12], &40u32.to_le_bytes());
        assert_eq!(Entry::from_bytes(&bytes), entry);
    }

    #[test]
    fn validation_bounds_the_weight_count_and_variation() {
        assert_eq!(validate(1, -1, 1, 1, -1).err(), Some(Reject::Per));
        assert_eq!(validate(1, MAX_PER + 1, 1, 1, -1).err(), Some(Reject::Per));
        assert_eq!(validate(1, 5, 2, 1, -1).err(), Some(Reject::Count));
        assert_eq!(validate(1, 5, 0, MAX_COUNT + 1, -1).err(), Some(Reject::Count));
        assert_eq!(validate(1, 5, 1, 1, -2).err(), Some(Reject::Variation));
        assert!(validate(1, 0, 0, 0, 3).is_ok());
    }

    #[test]
    fn a_kind_outside_the_switch_list_counts_as_enabled() {
        let mut mask = [0u32; KIND_MASK_WORDS];
        assert!(!kind_enabled(&mask, 0x3f));
        mask[1] |= 1 << 31;
        assert!(kind_enabled(&mask, 0x3f));
        assert!(kind_enabled(&mask, 0x1b0));
        assert!(kind_enabled(&mask, -1));
    }

    #[test]
    fn the_plan_strips_old_clone_entries_and_gates_on_the_base_switch() {
        let live = [
            Entry { kind: 0x3f, variation: -1, per: 30, min: 1, max: 1 },
            Entry { kind: 0x36a, variation: -1, per: 99, min: 1, max: 1 },
            Entry { kind: 0x40, variation: -1, per: 30, min: 1, max: 1 },
        ];
        let rules = [
            Rule {
                public_kind: 0x36a,
                base_kind: 0x3f,
                generator: GENERATOR_RANDOM,
                entry: Entry { kind: 0x36a, variation: -1, per: 20, min: 1, max: 1 },
            },
            Rule {
                public_kind: 0x36b,
                base_kind: 0x40,
                generator: GENERATOR_RANDOM,
                entry: Entry { kind: 0x36b, variation: -1, per: 20, min: 1, max: 1 },
            },
            Rule {
                public_kind: 0x36b,
                base_kind: 0x40,
                generator: 2,
                entry: Entry { kind: 0x36b, variation: -1, per: 5, min: 1, max: 1 },
            },
        ];
        let mut mask = [u32::MAX; KIND_MASK_WORDS];
        mask[2] &= !(1 << 0);
        let planned = plan(GENERATOR_RANDOM, &live, &rules, &mask, |kind| kind >= 0x36a);
        assert_eq!(planned.vanilla, 2);
        assert_eq!(planned.appended, 1);
        assert_eq!(planned.held_back, 1);
        assert_eq!(planned.entries.len(), 3);
        assert_eq!(planned.entries[2].kind, 0x36a);
        assert_eq!(planned.entries[2].per, 20);

        let box_plan = plan(2, &[], &rules, &[u32::MAX; KIND_MASK_WORDS], |kind| kind >= 0x36a);
        assert_eq!(box_plan.entries, vec![rules[2].entry]);
    }

    #[test]
    fn the_ball_generators_are_reserved_for_their_families() {
        assert_eq!(family_for_generator(GENERATOR_ASSIST), Some(Family::Assist));
        assert_eq!(family_for_generator(GENERATOR_POKEBALL), Some(Family::Pokemon));
        assert_eq!(family_for_generator(GENERATOR_MASTERBALL), Some(Family::Pokemon));
        assert_eq!(family_for_generator(GENERATOR_RANDOM), None);
        assert_eq!(family_for_generator(2), None);
    }

    #[test]
    fn a_lot_result_packs_the_kind_low_and_the_variation_high() {
        assert_eq!(pack_lot(0x3f, -1), 0xffff_ffff_0000_003f);
        assert_eq!(unpack_lot(0xffff_ffff_0000_003f), (0x3f, -1));
        assert_eq!(unpack_lot(pack_lot(-1, -1)), (-1, -1));
        assert_eq!(unpack_lot(pack_lot(0x36a, 2)), (0x36a, 2));
    }
}
