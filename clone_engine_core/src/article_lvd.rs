pub const COSTUMES: usize = 8;

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
    fn a_full_pack_borrows_nothing() {
        let shipped: Vec<Option<u32>> = (0..COSTUMES as u32).map(|c| Some(100 + c)).collect();
        let plan = plan(&shipped);
        assert!(plan.borrowed.is_empty());
        for (costume, chosen) in plan.chosen.iter().enumerate() {
            assert_eq!(*chosen, Some((costume, 100 + costume as u32)));
        }
    }
}
