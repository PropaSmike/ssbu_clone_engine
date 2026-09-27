pub const SLOTS: usize = 15;
pub const ITEM_SLOTS: usize = 10;
pub const HEADER_WORDS: usize = 2;
pub const WORDS: usize = HEADER_WORDS + SLOTS;

pub const SPACE_FIGHTER: u32 = 0;
pub const SPACE_WEAPON: u32 = 1;
pub const SPACE_ITEM: u32 = 2;

pub fn slots_in(space: u32) -> Option<usize> {
    match space {
        SPACE_FIGHTER | SPACE_WEAPON => Some(SLOTS),
        SPACE_ITEM => Some(ITEM_SLOTS),
        _ => None,
    }
}

pub fn words_in(space: u32) -> Option<usize> {
    slots_in(space).map(|slots| HEADER_WORDS + slots)
}

pub fn is_slot_in(space: u32, slot: u32) -> bool {
    slots_in(space).is_some_and(|slots| (slot as usize) < slots)
}

pub const DESTRUCTOR: u32 = 0;
pub const DELETER: u32 = 1;
pub const COROUTINE_YIELD: u32 = 2;
pub const START_COROUTINE: u32 = 3;
pub const RESUME_COROUTINE: u32 = 4;
pub const GET_UNUSED_COROUTINE_INDEX: u32 = 5;
pub const CLEAN_COROUTINE: u32 = 6;
pub const SET_COROUTINE_RELEASE_CONTROL: u32 = 7;
pub const IS_COROUTINE_RELEASE_CONTROL: u32 = 8;
pub const SET_STATUS_SCRIPTS: u32 = 9;
pub const SYS_LINE_SYSTEM_INIT: u32 = 10;
pub const SUB_BEGIN_ADDED_LINES: u32 = 11;
pub const SYS_LINE_STATUS_END_CONTROL: u32 = 12;
pub const SUB_END_ADDED_LINES: u32 = 13;
pub const RESET: u32 = 14;

pub const SMASHLINE_MAGIC: u64 = u64::from_le_bytes(*b"VRTMANIP");
pub const OUR_MAGIC: u64 = u64::from_le_bytes(*b"CLONEVTB");
pub const SMASHLINE_CONTEXT_WORD: usize = 1;
pub const OUR_CONTEXT_WORD: usize = HEADER_WORDS;
pub const CONTEXT_WORDS: usize = 2;
pub const ARENA_WORDS: usize = 8192;
pub const SMASHLINE_MODULE_BLOCKS: usize = 3;

pub fn slot_name(slot: u32) -> Option<&'static str> {
    Some(match slot {
        DESTRUCTOR => "destructor",
        DELETER => "deleter",
        COROUTINE_YIELD => "coroutine_yield",
        START_COROUTINE => "start_coroutine",
        RESUME_COROUTINE => "resume_coroutine",
        GET_UNUSED_COROUTINE_INDEX => "get_unused_coroutine_index",
        CLEAN_COROUTINE => "clean_coroutine",
        SET_COROUTINE_RELEASE_CONTROL => "set_coroutine_release_control",
        IS_COROUTINE_RELEASE_CONTROL => "is_coroutine_release_control",
        SET_STATUS_SCRIPTS => "set_status_scripts",
        SYS_LINE_SYSTEM_INIT => "sys_line_system_init",
        SUB_BEGIN_ADDED_LINES => "sub_begin_added_lines",
        SYS_LINE_STATUS_END_CONTROL => "sys_line_status_end_control",
        SUB_END_ADDED_LINES => "sub_end_added_lines",
        RESET => "RESET",
        _ => return None,
    })
}

pub fn is_slot(slot: u32) -> bool {
    (slot as usize) < SLOTS
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shared {
    Yes,
    SmashlinePrivate,
    OursPrivate,
}

pub fn classify(smashline: Option<u64>, ours: Option<u64>) -> Shared {
    if smashline == Some(SMASHLINE_MAGIC) {
        Shared::SmashlinePrivate
    } else if ours == Some(OUR_MAGIC) {
        Shared::OursPrivate
    } else {
        Shared::Yes
    }
}

pub fn inside_blocks(address: u64, blocks: &[(u64, u64)]) -> bool {
    blocks
        .iter()
        .any(|&(start, size)| address >= start && address - start < size)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Placed {
    pub space: u32,
    pub kind: i32,
    pub original: usize,
    pub at: usize,
    pub slots: usize,
}

impl Placed {
    pub fn context(&self) -> usize {
        self.at
    }

    pub fn vtable(&self) -> usize {
        self.at + CONTEXT_WORDS + HEADER_WORDS
    }

    pub fn end(&self) -> usize {
        self.vtable() + self.slots
    }
}

pub struct Arena {
    used: usize,
    placed: Vec<Placed>,
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

impl Arena {
    pub const fn new() -> Self {
        Self { used: 0, placed: Vec::new() }
    }

    pub fn used(&self) -> usize {
        self.used
    }

    pub fn find(&self, space: u32, kind: i32, original: usize) -> Option<Placed> {
        self.placed
            .iter()
            .copied()
            .find(|p| p.space == space && p.kind == kind && p.original == original)
    }

    pub fn place(
        &mut self,
        space: u32,
        kind: i32,
        original: usize,
        slots: usize,
    ) -> Option<(Placed, bool)> {
        if let Some(placed) = self.find(space, kind, original) {
            return Some((placed, false));
        }
        if slots == 0 {
            return None;
        }
        let placed = Placed { space, kind, original, at: self.used, slots };
        if placed.end() > ARENA_WORDS {
            return None;
        }
        self.used = placed.end();
        self.placed.push(placed);
        Some((placed, true))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Entry {
    pub function: usize,
    pub original_out: usize,
}

#[derive(Clone, Copy)]
pub struct Entries {
    slots: [Option<Entry>; SLOTS],
}

impl Default for Entries {
    fn default() -> Self {
        Self { slots: [None; SLOTS] }
    }
}

impl Entries {
    pub fn set(&mut self, slot: u32, function: usize) -> Result<Option<usize>, ()> {
        self.set_in(SPACE_FIGHTER, slot, function)
    }

    pub fn set_in(
        &mut self,
        space: u32,
        slot: u32,
        function: usize,
    ) -> Result<Option<usize>, ()> {
        self.set_followed(space, slot, function, 0)
    }

    pub fn set_followed(
        &mut self,
        space: u32,
        slot: u32,
        function: usize,
        original_out: usize,
    ) -> Result<Option<usize>, ()> {
        if !is_slot_in(space, slot) || function == 0 {
            return Err(());
        }
        let cell = &mut self.slots[slot as usize];
        let previous = cell.map(|entry| entry.function);
        *cell = Some(Entry { function, original_out });
        Ok(previous)
    }

    pub fn get(&self, slot: u32) -> Option<usize> {
        if is_slot(slot) {
            self.slots[slot as usize].map(|entry| entry.function)
        } else {
            None
        }
    }

    pub fn any(&self) -> bool {
        self.slots.iter().any(Option::is_some)
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, Entry)> + '_ {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, entry)| entry.map(|entry| (i as u32, entry)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vtable_is_fifteen_slots_behind_two_header_words() {
        assert_eq!(WORDS * 8, 0x88);
    }

    #[test]
    fn every_slot_is_named_and_nothing_past_the_end_is() {
        for slot in 0..SLOTS as u32 {
            assert!(slot_name(slot).is_some(), "slot {slot} has no name");
        }
        assert!(slot_name(SLOTS as u32).is_none());
        assert!(!is_slot(SLOTS as u32));
    }

    #[test]
    fn the_system_lines_are_where_the_relocations_put_them() {
        assert_eq!(slot_name(SYS_LINE_SYSTEM_INIT), Some("sys_line_system_init"));
        assert_eq!(slot_name(RESET), Some("RESET"));
        assert_eq!(RESET as usize, SLOTS - 1);
    }

    #[test]
    fn registering_returns_what_was_there_so_a_second_mod_can_chain() {
        let mut entries = Entries::default();
        assert_eq!(entries.set(SYS_LINE_SYSTEM_INIT, 0x1000), Ok(None));
        assert_eq!(entries.set(SYS_LINE_SYSTEM_INIT, 0x2000), Ok(Some(0x1000)));
        assert_eq!(entries.get(SYS_LINE_SYSTEM_INIT), Some(0x2000));
    }

    #[test]
    fn an_item_agent_shares_ten_slots_because_set_status_scripts_is_slot_nine() {
        assert!(is_slot_in(SPACE_ITEM, SET_STATUS_SCRIPTS));
        assert!(!is_slot_in(SPACE_ITEM, SYS_LINE_SYSTEM_INIT));
        assert_eq!(slots_in(SPACE_ITEM), Some(10));
    }

    #[test]
    fn an_item_copy_is_never_sized_from_slots_in_because_agents_run_to_118_slots() {
        assert!(slots_in(SPACE_ITEM).unwrap() < 118);
    }

    #[test]
    fn the_original_is_carried_because_only_install_time_knows_the_vanilla_entry() {
        let mut entries = Entries::default();
        assert_eq!(
            entries.set_followed(SPACE_FIGHTER, SYS_LINE_SYSTEM_INIT, 0x1000, 0x9000),
            Ok(None)
        );
        let (slot, entry) = entries.iter().next().unwrap();
        assert_eq!(slot, SYS_LINE_SYSTEM_INIT);
        assert_eq!(entry.function, 0x1000);
        assert_eq!(entry.original_out, 0x9000);
    }

    #[test]
    fn a_slot_past_the_end_or_a_null_function_is_refused() {
        let mut entries = Entries::default();
        assert!(entries.set(SLOTS as u32, 0x1000).is_err());
        assert!(entries.set(RESET, 0).is_err());
        assert!(!entries.any());
    }

    #[test]
    fn the_vanilla_offset_to_top_of_zero_must_not_read_as_private() {
        assert_eq!(classify(Some(SMASHLINE_MAGIC), None), Shared::SmashlinePrivate);
        assert_eq!(classify(None, Some(OUR_MAGIC)), Shared::OursPrivate);
        assert_eq!(classify(Some(0), Some(0)), Shared::Yes);
        assert_eq!(classify(None, None), Shared::Yes);
        assert_eq!(classify(Some(0x7100_0000_0000), None), Shared::Yes);
    }

    #[test]
    fn smashline_keeps_its_context_one_word_before_the_vtable_not_two() {
        assert_eq!(SMASHLINE_CONTEXT_WORD, 1);
        assert_ne!(SMASHLINE_CONTEXT_WORD, OUR_CONTEXT_WORD);
        assert_eq!(classify(None, Some(SMASHLINE_MAGIC)), Shared::Yes);
        assert_eq!(classify(Some(OUR_MAGIC), None), Shared::Yes);
        assert_eq!(
            classify(Some(SMASHLINE_MAGIC), Some(OUR_MAGIC)),
            Shared::SmashlinePrivate
        );
    }

    #[test]
    fn a_copy_is_shared_by_every_agent_of_its_kind_and_original() {
        let mut arena = Arena::new();
        let (first, fresh) = arena.place(SPACE_ITEM, 0x36c, 0x7000, 10).unwrap();
        assert!(fresh);
        let (again, fresh) = arena.place(SPACE_ITEM, 0x36c, 0x7000, 10).unwrap();
        assert!(!fresh);
        assert_eq!(first, again);
        let (other, fresh) = arena.place(SPACE_ITEM, 0x36d, 0x7000, 10).unwrap();
        assert!(fresh);
        assert_eq!(other.at, first.end());
        let (fighter, _) = arena.place(SPACE_FIGHTER, 0x36c, 0x7000, SLOTS).unwrap();
        assert_ne!(fighter.at, first.at);
    }

    #[test]
    fn the_vtable_sits_behind_our_context_and_the_two_header_words() {
        let mut arena = Arena::new();
        let (placed, _) = arena.place(SPACE_FIGHTER, 123, 0x7000, SLOTS).unwrap();
        assert_eq!(placed.context(), 0);
        assert_eq!(placed.vtable() - OUR_CONTEXT_WORD, CONTEXT_WORDS);
        assert_eq!(placed.vtable() - SMASHLINE_CONTEXT_WORD, CONTEXT_WORDS + 1);
        assert_eq!(placed.end(), CONTEXT_WORDS + HEADER_WORDS + SLOTS);
        assert_eq!(arena.used(), placed.end());
    }

    #[test]
    fn a_full_arena_refuses_instead_of_overrunning() {
        let mut arena = Arena::new();
        let big = ARENA_WORDS - CONTEXT_WORDS - HEADER_WORDS;
        assert!(arena.place(SPACE_ITEM, 1, 0x7000, big + 1).is_none());
        assert!(arena.place(SPACE_ITEM, 1, 0x7000, 0).is_none());
        assert!(arena.place(SPACE_ITEM, 1, 0x7000, big).is_some());
        assert!(arena.place(SPACE_ITEM, 2, 0x7000, 1).is_none());
        assert!(arena.place(SPACE_ITEM, 1, 0x7000, big).is_some());
    }

    #[test]
    fn smashline_counts_an_address_as_a_module_s_only_inside_its_first_three_blocks() {
        let blocks = [(0x1000, 0x1000), (0x2000, 0x800), (0x2800, 0x4000)];
        assert_eq!(blocks.len(), SMASHLINE_MODULE_BLOCKS);
        assert!(inside_blocks(0x1000, &blocks));
        assert!(inside_blocks(0x67ff, &blocks));
        assert!(!inside_blocks(0x6800, &blocks));
        assert!(!inside_blocks(0xfff, &blocks));
    }

    #[test]
    fn a_fighter_and_a_weapon_agent_vtable_are_both_0x88() {
        assert_eq!(words_in(SPACE_FIGHTER).unwrap() * 8, 0x88);
        assert_eq!(words_in(SPACE_WEAPON).unwrap() * 8, 0x88);
        assert_eq!(slots_in(3), None);
    }

    #[test]
    fn the_item_slots_mean_the_same_as_the_fighter_ones_they_share() {
        for slot in 0..ITEM_SLOTS as u32 {
            assert!(is_slot_in(SPACE_ITEM, slot));
            assert!(slot_name(slot).is_some());
        }
    }

    #[test]
    fn an_item_has_set_status_scripts_but_the_slots_above_it_are_not_system_lines() {
        let mut entries = Entries::default();
        assert!(is_slot_in(SPACE_ITEM, SET_STATUS_SCRIPTS));
        assert!(entries.set_in(SPACE_ITEM, SET_STATUS_SCRIPTS, 0x1000).is_ok());
        for slot in [SYS_LINE_SYSTEM_INIT, RESET] {
            assert!(!is_slot_in(SPACE_ITEM, slot));
            assert!(entries.set_in(SPACE_ITEM, slot, 0x1000).is_err());
        }
        assert!(entries.set_in(SPACE_ITEM, IS_COROUTINE_RELEASE_CONTROL, 0x1000).is_ok());
    }

    #[test]
    fn the_two_magics_differ() {
        assert_ne!(SMASHLINE_MAGIC, OUR_MAGIC);
    }
}
