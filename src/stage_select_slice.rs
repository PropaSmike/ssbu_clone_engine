#![allow(dead_code)]

pub const STAGE_LIST_BEGIN: usize = 0x00;
pub const STAGE_LIST_END: usize = 0x08;
pub const STAGE_LIST_STRIDE: usize = 8;

#[cfg(not(feature = "stage_slice_diag"))]
pub const PAGE_SIZE: usize = 121;
#[cfg(feature = "stage_slice_diag")]
pub const PAGE_SIZE: usize = 111;

pub const COUNT_MODE_THRESHOLD: usize = 110;

const _: () = assert!(
    PAGE_SIZE > COUNT_MODE_THRESHOLD,
    "a page must draw more stages than the count threshold at 0x1b2a278, or the      stage-select screen loads a different resource set and faults"
);

pub const MAX_STAGES: usize = 4096;

pub const ADVANCE_ON_BUILD: bool = true;

pub fn next_page(page: usize, pages: usize) -> usize {
    if !ADVANCE_ON_BUILD || pages <= 1 {
        return 0;
    }
    (page + 1) % pages
}

#[derive(Debug, PartialEq, Eq)]
pub enum SliceError {
    NoList,
    Malformed { begin: usize, end: usize },
    TooLong(usize),
    PageOutOfRange { page: usize, pages: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageList {
    pub begin: usize,
    pub count: usize,
}

impl StageList {
    pub fn read(at: usize, read_usize: impl Fn(usize) -> usize) -> Result<Self, SliceError> {
        if at == 0 {
            return Err(SliceError::NoList);
        }
        let begin = read_usize(at + STAGE_LIST_BEGIN);
        let end = read_usize(at + STAGE_LIST_END);
        if begin == 0 || end < begin || (end - begin) % STAGE_LIST_STRIDE != 0 {
            return Err(SliceError::Malformed { begin, end });
        }
        let count = (end - begin) / STAGE_LIST_STRIDE;
        if count > MAX_STAGES {
            return Err(SliceError::TooLong(count));
        }
        Ok(StageList { begin, count })
    }

    pub fn pages(&self) -> usize {
        self.count.div_ceil(PAGE_SIZE).max(1)
    }

    pub fn drawn_on(&self, _page: usize) -> usize {
        self.count.min(PAGE_SIZE)
    }

    pub fn start_index(&self, page: usize) -> Result<usize, SliceError> {
        let pages = self.pages();
        if page >= pages {
            return Err(SliceError::PageOutOfRange { page, pages });
        }
        if self.count <= PAGE_SIZE {
            return Ok(0);
        }
        Ok((page * PAGE_SIZE).min(self.count - PAGE_SIZE))
    }

    pub fn begin_for(&self, page: usize) -> Result<usize, SliceError> {
        Ok(self.begin + self.start_index(page)? * STAGE_LIST_STRIDE)
    }

    pub fn end_for(&self, page: usize) -> Result<usize, SliceError> {
        Ok(self.begin_for(page)? + self.drawn_on(page) * STAGE_LIST_STRIDE)
    }

    pub fn pages_are_safe_to_draw(&self) -> bool {
        let real = self.count > COUNT_MODE_THRESHOLD;
        let paged = self.drawn_on(0) > COUNT_MODE_THRESHOLD;
        real == paged
    }
}

#[cfg(all(not(test), feature = "stage_slice"))]
mod live {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};

    pub(super) const OFF_BUILD_PANELS: usize = 0x1B2A1F0;

    pub(super) static WANTED_PAGE: AtomicUsize = AtomicUsize::new(0);
    pub(super) static LAST_PAGES: AtomicUsize = AtomicUsize::new(1);

    unsafe fn read_usize(at: usize) -> usize {
        core::ptr::read_volatile(at as *const usize)
    }

    #[cfg(feature = "stage_pane_table")]
    #[skyline::hook(offset = OFF_BUILD_PANELS)]
    pub(super) unsafe fn build_panels_hook(holder: usize, list: usize) {
        use crate::stage_pane_table::live;
        live::begin_capture();
        call_original!(holder, list);
        let drawn = match StageList::read(list, |at| read_usize(at)) {
            Ok(stage_list) => stage_list.count,
            Err(error) => {
                skyline::println!("[stageslice] stage list unreadable: {error:?}");
                0
            }
        };
        let panes = live::end_capture();
        live::set_drawn(drawn);
        live::census(drawn);
        let touched = live::show_page(0, drawn);
        skyline::println!(
            "[stageslice] {panes} pane(s), {drawn} cell(s) drawn; page 1 applied to {touched}"
        );
    }

    #[cfg(not(feature = "stage_pane_table"))]
    #[skyline::hook(offset = OFF_BUILD_PANELS)]
    pub(super) unsafe fn build_panels_hook(holder: usize, list: usize) {
        let page = WANTED_PAGE.load(Ordering::Acquire);

        let sliced = match StageList::read(list, |at| read_usize(at)) {
            Ok(stage_list) => {
                LAST_PAGES.store(stage_list.pages(), Ordering::Release);
                if !stage_list.pages_are_safe_to_draw() {
                    skyline::println!(
                        "[stageslice] REFUSING to page: {} stage(s) but a page is {},                          which crosses the count threshold of {} that selects the                          screen's resources at 0x1b2a278",
                        stage_list.count,
                        stage_list.drawn_on(0),
                        COUNT_MODE_THRESHOLD
                    );
                    call_original!(holder, list);
                    return;
                }
                if stage_list.pages() == 1 {
                    None
                } else {
                    match (stage_list.begin_for(page), stage_list.end_for(page)) {
                        (Ok(begin), Ok(end)) => Some((stage_list, begin, end)),
                        (Err(error), _) | (_, Err(error)) => {
                            skyline::println!(
                                "[stageslice] page {} not drawn: {:?}",
                                page + 1,
                                error
                            );
                            None
                        }
                    }
                }
            }
            Err(error) => {
                LAST_PAGES.store(1, Ordering::Release);
                if page != 0 {
                    skyline::println!("[stageslice] stage list unreadable: {error:?}");
                }
                None
            }
        };

        let Some((stage_list, begin, end)) = sliced else {
            call_original!(holder, list);
            return;
        };

        let begin_field = (list + STAGE_LIST_BEGIN) as *mut usize;
        let end_field = (list + STAGE_LIST_END) as *mut usize;
        let original_begin = stage_list.begin;
        let original_end = stage_list.begin + stage_list.count * STAGE_LIST_STRIDE;

        core::ptr::write_volatile(begin_field, begin);
        core::ptr::write_volatile(end_field, end);
        call_original!(holder, list);
        core::ptr::write_volatile(begin_field, original_begin);
        core::ptr::write_volatile(end_field, original_end);

        let next = next_page(page, stage_list.pages());
        WANTED_PAGE.store(next, Ordering::Release);

        skyline::println!(
            "[stageslice] page {}/{} drawn: {} of {} stage(s), cells 0..{}; \
             leave the screen and re-enter for page {}",
            page + 1,
            stage_list.pages(),
            stage_list.drawn_on(page),
            stage_list.count,
            stage_list.drawn_on(page).saturating_sub(1),
            next + 1
        );
    }

    pub(super) fn install() {
        skyline::install_hook!(build_panels_hook);
        skyline::println!(
            "[stageslice] armed at {:#x}: paging by stage-list slice, {} cells per page",
            OFF_BUILD_PANELS,
            PAGE_SIZE
        );
    }
}

pub const SLICE_SAFE: bool = true;

#[cfg(all(not(test), feature = "stage_slice"))]
pub(crate) fn install() {
    if !SLICE_SAFE {
        skyline::println!(
            "[stageslice] DISARMED: moving `begin` alone draws a TAIL, not a page              (page 0 drew the whole list on console), and the first sliced build              crashed for reasons not yet established. Hook not installed."
        );
        return;
    }
    live::install();
}

#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn clone_engine_stage_select_slice(page: u32) -> i32 {
    #[cfg(not(feature = "stage_slice"))]
    {
        let _ = page;
        skyline::println!(
            "[stageslice] ignored: this engine build has no stage-select paging \
             (rebuild with --features stage_slice)"
        );
        -1
    }
    #[cfg(feature = "stage_slice")]
    {
        use core::sync::atomic::Ordering;
        let pages = live::LAST_PAGES.load(Ordering::Acquire);
        if page as usize >= pages {
            skyline::println!(
                "[stageslice] page {} refused: the last build had {} page(s)",
                page + 1,
                pages
            );
            return -1;
        }
        live::WANTED_PAGE.store(page as usize, Ordering::Release);
        skyline::println!("[stageslice] page {} set for the next build", page + 1);
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        memory: Vec<u8>,
        object: usize,
    }

    impl Fake {
        fn new(count: usize) -> Self {
            let mut memory = vec![0u8; 0x20 + count * 8];
            let base = memory.as_ptr() as usize;
            let object = base;
            let entries = base + 0x20;
            memory[0..8].copy_from_slice(&entries.to_le_bytes());
            memory[8..16].copy_from_slice(&(entries + count * 8).to_le_bytes());
            for i in 0..count {
                let value = 0x6900_0000_0000_0000u64 | (0x0d00_0000u64 + i as u64);
                memory[0x20 + i * 8..0x28 + i * 8].copy_from_slice(&value.to_le_bytes());
            }
            Fake { memory, object }
        }

        fn read(&self) -> impl Fn(usize) -> usize + '_ {
            let base = self.memory.as_ptr() as usize;
            move |at: usize| {
                let offset = at - base;
                usize::from_le_bytes(self.memory[offset..offset + 8].try_into().unwrap())
            }
        }

        fn entries(&self) -> usize {
            self.memory.as_ptr() as usize + 0x20
        }
    }

    #[test]
    fn the_constants_are_the_ones_the_builder_uses() {
        assert_eq!(STAGE_LIST_BEGIN, 0x00);
        assert_eq!(STAGE_LIST_END, 0x08);
        assert_eq!(STAGE_LIST_STRIDE, 8);
    }

    #[test]
    fn reads_a_well_formed_list() {
        let fake = Fake::new(119);
        let list = StageList::read(fake.object, fake.read()).unwrap();
        assert_eq!(list.count, 119);
        assert_eq!(list.begin, fake.entries());
        assert_eq!(list.pages(), 1);
    }

    #[test]
    fn page_one_is_the_list_exactly_as_it_is() {
        let fake = Fake::new(200);
        let list = StageList::read(fake.object, fake.read()).unwrap();
        assert_eq!(list.begin_for(0), Ok(list.begin));
    }

    #[test]
    fn page_two_starts_one_page_in_when_there_is_room() {
        let fake = Fake::new(PAGE_SIZE * 3);
        let list = StageList::read(fake.object, fake.read()).unwrap();
        assert_eq!(list.start_index(1), Ok(PAGE_SIZE));
        assert_eq!(list.begin_for(1), Ok(list.begin + PAGE_SIZE * 8));
    }

    #[test]
    fn a_page_needs_both_ends_moved() {
        let fake = Fake::new(300);
        let list = StageList::read(fake.object, fake.read()).unwrap();
        let end_of_list = list.begin + list.count * STAGE_LIST_STRIDE;
        let begin = list.begin_for(1).unwrap();
        let tail = (end_of_list - begin) / STAGE_LIST_STRIDE;
        let page_len = (list.end_for(1).unwrap() - begin) / STAGE_LIST_STRIDE;
        assert!(
            tail > page_len,
            "begin-only would draw {tail}, not {page_len}"
        );
        assert_eq!(page_len, PAGE_SIZE);
    }

    #[test]
    fn every_page_draws_the_same_number_of_stages() {
        for count in [111usize, 150, 242, 300, 400] {
            let fake = Fake::new(count);
            let list = StageList::read(fake.object, fake.read()).unwrap();
            let expected = count.min(PAGE_SIZE);
            for page in 0..list.pages() {
                assert_eq!(list.drawn_on(page), expected, "count {count} page {page}");
                let span = list.end_for(page).unwrap() - list.begin_for(page).unwrap();
                assert_eq!(span / STAGE_LIST_STRIDE, expected);
            }
        }
    }

    #[test]
    fn the_last_page_overlaps_rather_than_shrinking() {
        let fake = Fake::new(PAGE_SIZE + 8);
        let list = StageList::read(fake.object, fake.read()).unwrap();
        assert_eq!(list.pages(), 2);
        assert_eq!(list.start_index(0), Ok(0));
        assert_eq!(list.start_index(1), Ok(8));
        assert_eq!(list.drawn_on(1), PAGE_SIZE);
    }

    #[test]
    fn paging_is_refused_when_it_would_cross_the_threshold() {
        let fake = Fake::new(COUNT_MODE_THRESHOLD + 40);
        let list = StageList::read(fake.object, fake.read()).unwrap();
        if PAGE_SIZE > COUNT_MODE_THRESHOLD {
            assert!(list.pages_are_safe_to_draw());
        }
        let small = StageList {
            begin: list.begin,
            count: COUNT_MODE_THRESHOLD + 1,
        };
        assert!(small.count > COUNT_MODE_THRESHOLD);
        assert_eq!(
            small.drawn_on(0) > COUNT_MODE_THRESHOLD,
            small.pages_are_safe_to_draw()
        );
    }

    #[test]
    fn page_one_is_sliced_too_when_there_is_more_than_one_page() {
        let fake = Fake::new(116.max(PAGE_SIZE + 5));
        let list = StageList::read(fake.object, fake.read()).unwrap();
        assert!(list.pages() > 1);
        for page in 0..list.pages() {
            assert_eq!(list.drawn_on(page), PAGE_SIZE);
            let span = list.end_for(page).unwrap() - list.begin_for(page).unwrap();
            assert_eq!(span / STAGE_LIST_STRIDE, PAGE_SIZE);
        }
        let small = Fake::new(PAGE_SIZE - 1);
        let small_list = StageList::read(small.object, small.read()).unwrap();
        assert_eq!(small_list.pages(), 1);
    }

    #[test]
    fn a_list_below_the_threshold_is_consistent_with_itself() {
        let fake = Fake::new(50);
        let list = StageList::read(fake.object, fake.read()).unwrap();
        assert!(list.pages_are_safe_to_draw());
        assert_eq!(list.pages(), 1);
        assert_eq!(list.drawn_on(0), 50);
    }

    #[test]
    fn a_single_page_list_has_no_second_page() {
        let fake = Fake::new(119);
        let list = StageList::read(fake.object, fake.read()).unwrap();
        assert_eq!(
            list.begin_for(1),
            Err(SliceError::PageOutOfRange { page: 1, pages: 1 })
        );
    }

    #[test]
    fn an_exactly_full_page_has_no_second_page() {
        let fake = Fake::new(121);
        let list = StageList::read(fake.object, fake.read()).unwrap();
        assert_eq!(list.pages(), 1);
        assert!(list.begin_for(1).is_err());
        assert_eq!(list.drawn_on(0), 121);
    }

    #[test]
    fn refuses_a_null_list() {
        assert_eq!(StageList::read(0, |_| 0), Err(SliceError::NoList));
    }

    #[test]
    fn refuses_a_span_that_is_not_whole_entries() {
        let read = |at: usize| match at {
            0x1000 => 0x9000,
            0x1008 => 0x9000 + 12,
            _ => 0,
        };
        assert_eq!(
            StageList::read(0x1000, read),
            Err(SliceError::Malformed {
                begin: 0x9000,
                end: 0x9000 + 12
            })
        );
    }

    #[test]
    fn refuses_a_reversed_or_null_span() {
        let reversed = |at: usize| match at {
            0x1000 => 0x9000,
            0x1008 => 0x8000,
            _ => 0,
        };
        assert!(matches!(
            StageList::read(0x1000, reversed),
            Err(SliceError::Malformed { .. })
        ));
        let null_begin = |at: usize| if at == 0x1008 { 0x9000 } else { 0 };
        assert!(matches!(
            StageList::read(0x1000, null_begin),
            Err(SliceError::Malformed { .. })
        ));
    }

    #[test]
    fn refuses_something_that_is_not_the_stage_list() {
        let count = MAX_STAGES + 1;
        let read = move |at: usize| match at {
            0x1000 => 0x9000,
            0x1008 => 0x9000 + count * 8,
            _ => 0,
        };
        assert_eq!(
            StageList::read(0x1000, read),
            Err(SliceError::TooLong(count))
        );
    }

    #[test]
    fn every_page_stays_inside_the_list() {
        for count in [1usize, 120, 121, 122, 242, 243, 255, 300] {
            let fake = Fake::new(count);
            let list = StageList::read(fake.object, fake.read()).unwrap();
            let end = list.begin + count * 8;
            for page in 0..list.pages() {
                let begin = list.begin_for(page).unwrap();
                assert!(begin < end, "count {count} page {page} starts past the end");
                assert!(
                    list.drawn_on(page) > 0,
                    "count {count} page {page} draws nothing"
                );
                assert!(
                    begin + list.drawn_on(page) * 8 <= end,
                    "count {count} page {page} runs past the end"
                );
                assert!(list.end_for(page).unwrap() <= end);
            }
        }
    }

    #[test]
    fn each_build_hands_the_next_one_a_different_page() {
        let fake = Fake::new(129);
        let list = StageList::read(fake.object, fake.read()).unwrap();
        assert_eq!(
            list.pages(),
            2,
            "129 stages over {PAGE_SIZE} cells is two pages"
        );
        assert_eq!(next_page(0, list.pages()), 1);
        assert_ne!(
            next_page(0, list.pages()),
            0,
            "page 1 must not follow page 1"
        );
    }

    #[test]
    fn advancing_wraps_and_never_leaves_the_list() {
        for pages in 1..8usize {
            for page in 0..pages {
                let next = next_page(page, pages);
                assert!(next < pages, "{pages} page(s): {next} is out of range");
            }
        }
    }

    #[test]
    fn a_single_page_list_never_advances() {
        assert_eq!(next_page(0, 1), 0);
    }

    #[test]
    fn cycling_reaches_every_page() {
        for pages in [2usize, 3, 5] {
            let mut seen = vec![false; pages];
            let mut page = 0;
            for _ in 0..pages {
                seen[page] = true;
                page = next_page(page, pages);
            }
            assert!(
                seen.iter().all(|s| *s),
                "{pages} page(s): a page is never shown"
            );
            assert_eq!(page, 0, "the cycle must return to the first page");
        }
    }

    #[test]
    fn the_pages_cover_every_stage() {
        for count in [PAGE_SIZE + 1, 150usize, 242, 300] {
            let fake = Fake::new(count);
            let list = StageList::read(fake.object, fake.read()).unwrap();
            let mut seen = vec![false; count];
            for page in 0..list.pages() {
                let start = list.start_index(page).unwrap();
                for i in start..start + list.drawn_on(page) {
                    seen[i] = true;
                }
            }
            assert!(
                seen.iter().all(|s| *s),
                "count {count}: a stage is unreachable"
            );
        }
    }
}
