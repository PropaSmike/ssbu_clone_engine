pub const SLOTS: i32 = 4;
pub const MAX_PAGES: u8 = 32;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MoveList {
    pub pages: Option<u8>,
    pub image_pages: Option<Vec<u8>>,
}

impl MoveList {
    pub fn check(&self) -> Result<u8, String> {
        let pages = self.pages.ok_or_else(|| "[move_list] needs pages".to_string())?;
        if pages == 0 || pages > MAX_PAGES {
            return Err(format!("move_list pages must be 1 to {MAX_PAGES}"));
        }
        for page in self.image_pages.iter().flatten() {
            if *page >= pages {
                return Err(format!(
                    "move_list image page {page} is past the last page, {}",
                    pages - 1
                ));
            }
        }
        Ok(pages)
    }

    pub fn image_mask(&self) -> Option<u64> {
        self.image_pages
            .as_ref()
            .map(|pages| pages.iter().fold(0u64, |mask, page| mask | 1u64 << page))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Turn {
    pub refill: Option<i32>,
    pub physical: Option<i32>,
    pub run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pager {
    pub count: i32,
    pub base: i32,
    pub page: i32,
}

impl Pager {
    pub fn new(count: i32) -> Self {
        Self {
            count,
            base: 0,
            page: 0,
        }
    }

    pub fn slot_page(&self, slot: i32) -> i32 {
        self.base + slot
    }

    pub fn next(&mut self, physical: i32) -> Turn {
        if self.page >= self.count {
            return Turn::default();
        }
        let mut turn = Turn {
            run: true,
            ..Turn::default()
        };
        if self.page == 0 && self.base != 0 {
            self.base = 0;
            turn.refill = Some(0);
        } else if physical >= SLOTS {
            self.base += 1;
            turn.refill = Some(self.base);
            turn.physical = Some(SLOTS - 1);
        }
        self.page += 1;
        turn
    }

    pub fn previous(&mut self, physical: i32) -> Turn {
        let mut turn = Turn {
            run: true,
            ..Turn::default()
        };
        if self.page == 0 {
            return turn;
        }
        if physical == 1 && self.base > 0 {
            self.base -= 1;
            turn.refill = Some(self.base);
            turn.physical = Some(2);
        }
        self.page -= 1;
        turn
    }

    pub fn first(&mut self) -> Turn {
        self.page = 0;
        Turn {
            run: true,
            ..Turn::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Screen {
        physical: i32,
        slots: [i32; SLOTS as usize],
    }

    impl Screen {
        fn new() -> Self {
            Self {
                physical: 0,
                slots: [0, 1, 2, 3],
            }
        }

        fn shown(&self) -> Option<i32> {
            (self.physical > 0).then(|| self.slots[self.physical as usize - 1])
        }

        fn apply(&mut self, pager: &Pager, turn: Turn, step: i32) {
            if let Some(base) = turn.refill {
                for slot in 0..SLOTS {
                    self.slots[slot as usize] = base + slot;
                }
            }
            if let Some(physical) = turn.physical {
                self.physical = physical;
            }
            if turn.run {
                self.physical = (self.physical + step).clamp(0, pager.count.min(SLOTS));
            }
        }
    }

    fn walk(count: i32) {
        let mut pager = Pager::new(count);
        let mut screen = Screen::new();
        for expected in 1..=count {
            let turn = pager.next(screen.physical);
            screen.apply(&pager, turn, 1);
            assert_eq!(screen.shown(), Some(expected - 1), "forward to page {expected} of {count}");
            assert_eq!(pager.page, expected);
        }
        assert_eq!(pager.next(screen.physical), Turn::default());
        for expected in (0..count).rev() {
            let turn = pager.previous(screen.physical);
            screen.apply(&pager, turn, -1);
            assert_eq!(pager.page, expected);
            assert_eq!(screen.shown(), (expected > 0).then(|| expected - 1), "back to page {expected} of {count}");
        }
    }

    #[test]
    fn every_page_shows_its_own_content_both_ways() {
        for count in 1..=12 {
            walk(count);
        }
    }

    #[test]
    fn the_window_only_moves_past_the_fourth_slot() {
        let mut pager = Pager::new(6);
        for physical in 0..4 {
            assert_eq!(pager.next(physical).refill, None);
        }
        let turn = pager.next(4);
        assert_eq!((turn.refill, turn.physical, turn.run), (Some(1), Some(3), true));
        assert_eq!(pager.slot_page(3), 4);
    }

    #[test]
    fn going_back_to_the_first_page_resets_the_window_on_the_next_turn() {
        let mut pager = Pager::new(7);
        let mut screen = Screen::new();
        for _ in 0..6 {
            let turn = pager.next(screen.physical);
            screen.apply(&pager, turn, 1);
        }
        assert_eq!(pager.base, 2);
        pager.first();
        screen.physical = 0;
        let turn = pager.next(screen.physical);
        assert_eq!(turn.refill, Some(0));
        screen.apply(&pager, turn, 1);
        assert_eq!(screen.shown(), Some(0));
    }

    #[test]
    fn a_move_list_checks_its_pages() {
        let list = MoveList { pages: Some(6), image_pages: Some(vec![0, 5]) };
        assert_eq!(list.check(), Ok(6));
        assert_eq!(list.image_mask(), Some(0b100001));
        assert!(MoveList { pages: Some(6), image_pages: Some(vec![6]) }.check().is_err());
        assert!(MoveList { pages: Some(0), image_pages: None }.check().is_err());
        assert!(MoveList { pages: Some(MAX_PAGES + 1), image_pages: None }.check().is_err());
        assert!(MoveList::default().check().is_err());
        assert_eq!(MoveList { pages: Some(3), image_pages: None }.image_mask(), None);
    }
}
