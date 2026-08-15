#![allow(dead_code)]

#[cfg(test)]
#[path = "stage_select_cap.rs"]
mod stage_select_cap;
#[cfg(not(test))]
use crate::stage_select_cap::TARGET_CAP;
#[cfg(test)]
use stage_select_cap::TARGET_CAP;

pub const PANEL_LIST_BEGIN: usize = 0x168;
pub const PANEL_LIST_END: usize = 0x170;
pub const PANEL_LIST_CAPACITY: usize = 0x178;
pub const PANEL_ELEMENT: usize = 24;
pub const RECORD_STAGE_ID: usize = 0x00;
pub const RECORD_RECT: usize = 0x08;

pub const PANE_NODE: usize = 0x18;
pub const NODE_FLAGS: usize = 0x58;
pub const FLAG_VISIBLE: u8 = 0x01;

pub const PANE_SOURCE: Option<&str> = None;

pub const PAGE_SIZE: usize = 121;

pub const OFF_BUILD_PANELS: usize = 0x1B2A1F0;

pub const OFF_HOLDER_MEMBER_DTOR: usize = 0x1B29A50;
pub const HOLDER_MEMBER_OFFSET: usize = 0x208;

pub const OFF_FIND_BY_DISP_ORDER: usize = 0x32B2500;

pub const RESULT_TAG: u64 = 0x69;

pub fn is_found(result: u64) -> bool {
    let tag = result >> 56;
    let hash = result & 0xFF_FFFF_FFFF;
    !(tag == RESULT_TAG && hash == 0)
}

#[derive(Debug, PartialEq, Eq)]
pub enum PageError {
    NoScene,
    ListMalformed { begin: usize, end: usize },
    CapacityMismatch { count: usize, capacity: usize },
    ListTooLong(usize),
    PageOutOfRange { page: usize, pages: usize },
}

#[derive(Debug, PartialEq, Eq)]
pub enum FollowError {
    NoPanels,
    NotDense { disp_order: u32, panels: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelList {
    pub begin: usize,
    pub count: usize,
}

impl PanelList {
    pub fn read(holder: usize, read_usize: impl Fn(usize) -> usize) -> Result<Self, PageError> {
        if holder == 0 {
            return Err(PageError::NoScene);
        }
        let begin = read_usize(holder + PANEL_LIST_BEGIN);
        let end = read_usize(holder + PANEL_LIST_END);
        if begin == 0 || end < begin || (end - begin) % PANEL_ELEMENT != 0 {
            return Err(PageError::ListMalformed { begin, end });
        }
        let count = (end - begin) / PANEL_ELEMENT;
        if count > TARGET_CAP as usize {
            return Err(PageError::ListTooLong(count));
        }
        let capacity_end = read_usize(holder + PANEL_LIST_CAPACITY);
        if capacity_end < end || (capacity_end - begin) % PANEL_ELEMENT != 0 {
            return Err(PageError::CapacityMismatch {
                count,
                capacity: capacity_end.wrapping_sub(begin),
            });
        }
        Ok(PanelList { begin, count })
    }

    pub fn pages(&self) -> usize {
        self.count.div_ceil(PAGE_SIZE).max(1)
    }

    pub fn element(&self, index: usize) -> usize {
        self.begin + index * PANEL_ELEMENT
    }

    pub fn on_page(&self, index: usize, page: usize) -> bool {
        index / PAGE_SIZE == page
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Follow {
    pub panel_count: usize,
}

impl Follow {
    pub fn page_for(&self, disp_order: u32) -> Result<usize, FollowError> {
        if self.panel_count == 0 {
            return Err(FollowError::NoPanels);
        }
        if disp_order as usize >= self.panel_count {
            return Err(FollowError::NotDense {
                disp_order,
                panels: self.panel_count,
            });
        }
        Ok(disp_order as usize / PAGE_SIZE)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Baseline {
    bits: [u64; 4],
}

impl Baseline {
    pub const CAPACITY: usize = 256;

    pub fn new() -> Self {
        Baseline { bits: [0; 4] }
    }

    pub fn set(&mut self, index: usize, visible: bool) {
        if index >= Self::CAPACITY {
            return;
        }
        let (word, bit) = (index / 64, index % 64);
        if visible {
            self.bits[word] |= 1 << bit;
        } else {
            self.bits[word] &= !(1 << bit);
        }
    }

    pub fn get(&self, index: usize) -> bool {
        if index >= Self::CAPACITY {
            return false;
        }
        self.bits[index / 64] & (1 << (index % 64)) != 0
    }

    pub fn words(&self) -> &[u64; 4] {
        &self.bits
    }

    pub fn from_words(bits: [u64; 4]) -> Self {
        Baseline { bits }
    }
}

pub fn desired_visibility(baseline: bool, index: usize, page: usize) -> bool {
    baseline && index / PAGE_SIZE == page
}

pub fn apply_with(
    list: &PanelList,
    page: usize,
    read_usize: impl Fn(usize) -> usize,
    baseline: impl Fn(usize) -> bool,
    mut set_visible: impl FnMut(usize, bool),
) -> Result<usize, PageError> {
    let pages = list.pages();
    if page >= pages {
        return Err(PageError::PageOutOfRange { page, pages });
    }
    let mut touched = 0usize;
    for index in 0..list.count {
        let pane = read_usize(list.element(index));
        if pane == 0 {
            continue;
        }
        let node = read_usize(pane + PANE_NODE);
        if node == 0 {
            continue;
        }
        set_visible(node, desired_visibility(baseline(index), index, page));
        touched += 1;
    }
    Ok(touched)
}

pub fn capture_with(
    list: &PanelList,
    read_usize: impl Fn(usize) -> usize,
    is_visible: impl Fn(usize) -> bool,
) -> Baseline {
    let mut baseline = Baseline::new();
    for index in 0..list.count {
        let pane = read_usize(list.element(index));
        if pane == 0 {
            continue;
        }
        let node = read_usize(pane + PANE_NODE);
        if node == 0 {
            continue;
        }
        baseline.set(index, is_visible(node));
    }
    baseline
}

#[cfg(not(test))]
unsafe fn read_usize(at: usize) -> usize {
    core::ptr::read_volatile(at as *const usize)
}

#[cfg(not(test))]
unsafe fn set_visible(node: usize, visible: bool) {
    let at = (node + NODE_FLAGS) as *mut u8;
    let flags = core::ptr::read_volatile(at);
    let updated = if visible {
        flags | FLAG_VISIBLE
    } else {
        flags & !FLAG_VISIBLE
    };
    if updated != flags {
        core::ptr::write_volatile(at, updated);
    }
}

#[cfg(not(test))]
unsafe fn is_visible(node: usize) -> bool {
    core::ptr::read_volatile((node + NODE_FLAGS) as *const u8) & FLAG_VISIBLE != 0
}

#[cfg(not(test))]
pub unsafe fn apply(holder: usize, page: usize, baseline: &Baseline) -> Result<usize, PageError> {
    let list = PanelList::read(holder, |at| read_usize(at))?;
    let touched = apply_with(
        &list,
        page,
        |at| read_usize(at),
        |index| baseline.get(index),
        |node, visible| set_visible(node, visible),
    )?;
    Ok(touched)
}

#[cfg(not(test))]
pub unsafe fn capture(holder: usize) -> Result<Baseline, PageError> {
    let list = PanelList::read(holder, |at| read_usize(at))?;
    Ok(capture_with(
        &list,
        |at| read_usize(at),
        |node| is_visible(node),
    ))
}

#[cfg(all(not(test), feature = "stage_page"))]
mod live {
    use super::*;
    use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

    pub(super) static HOLDER: AtomicUsize = AtomicUsize::new(0);
    pub(super) static PANEL_COUNT: AtomicUsize = AtomicUsize::new(0);
    pub(super) static PAGES: AtomicUsize = AtomicUsize::new(1);
    static BASELINE: [AtomicU64; 4] = [
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
    ];
    pub(super) static APPLIED_PAGE: AtomicUsize = AtomicUsize::new(usize::MAX);

    fn store_baseline(baseline: &Baseline) {
        for (slot, word) in BASELINE.iter().zip(baseline.words()) {
            slot.store(*word, Ordering::Release);
        }
    }

    fn load_baseline() -> Baseline {
        let mut words = [0u64; 4];
        for (word, slot) in words.iter_mut().zip(BASELINE.iter()) {
            *word = slot.load(Ordering::Acquire);
        }
        Baseline::from_words(words)
    }
    pub(super) static WANTED_PAGE: AtomicUsize = AtomicUsize::new(0);
    pub(super) static FOLLOW: AtomicBool = AtomicBool::new(true);
    static FOLLOW_COMPLAINED: AtomicBool = AtomicBool::new(false);

    pub(super) unsafe fn show(page: usize) -> Result<usize, PageError> {
        let holder = HOLDER.load(Ordering::Acquire);
        if holder == 0 {
            return Err(PageError::NoScene);
        }
        let touched = apply(holder, page, &load_baseline())?;
        APPLIED_PAGE.store(page, Ordering::Release);
        Ok(touched)
    }

    unsafe fn show_if_changed(page: usize) {
        if PAGES.load(Ordering::Acquire) <= 1 {
            return;
        }
        if APPLIED_PAGE.load(Ordering::Acquire) == page {
            return;
        }
        match show(page) {
            Ok(touched) => {
                skyline::println!("[stagepage] page {} shown ({} panes)", page + 1, touched)
            }
            Err(error) => {
                skyline::println!("[stagepage] page {} not applied: {:?}", page + 1, error)
            }
        }
    }

    #[skyline::hook(offset = OFF_BUILD_PANELS)]
    pub(super) unsafe fn build_panels_hook(holder: usize, argument: u64) {
        call_original!(holder, argument);

        HOLDER.store(holder, Ordering::Release);
        APPLIED_PAGE.store(usize::MAX, Ordering::Release);

        let list = match PanelList::read(holder, |at| read_usize(at)) {
            Ok(list) => list,
            Err(error) => {
                PANEL_COUNT.store(0, Ordering::Release);
                PAGES.store(1, Ordering::Release);
                skyline::println!("[stagepage] panel vector unreadable after build: {error:?}");
                return;
            }
        };
        PANEL_COUNT.store(list.count, Ordering::Release);
        PAGES.store(list.pages(), Ordering::Release);
        FOLLOW_COMPLAINED.store(false, Ordering::Release);

        let baseline = capture_with(&list, |at| read_usize(at), |node| is_visible(node));
        store_baseline(&baseline);
        let visible = (0..list.count).filter(|i| baseline.get(*i)).count();

        if list.pages() == 1 {
            skyline::println!(
                "[stagepage] {} panel(s) ({} visible), one page -- nothing to do",
                list.count,
                visible
            );
            return;
        }
        let wanted = WANTED_PAGE.load(Ordering::Acquire).min(list.pages() - 1);
        skyline::println!(
            "[stagepage] {} panel(s) ({} visible) over {} pages; showing page {}",
            list.count,
            visible,
            list.pages(),
            wanted + 1
        );
        show_if_changed(wanted);
    }

    #[skyline::hook(offset = OFF_HOLDER_MEMBER_DTOR)]
    pub(super) unsafe fn holder_member_dtor_hook(member: usize) {
        let holder = HOLDER.load(Ordering::Acquire);
        if holder != 0 && member == holder + HOLDER_MEMBER_OFFSET {
            HOLDER.store(0, Ordering::Release);
            PANEL_COUNT.store(0, Ordering::Release);
            PAGES.store(1, Ordering::Release);
            APPLIED_PAGE.store(usize::MAX, Ordering::Release);
            skyline::println!("[stagepage] stage-select torn down; holder retracted");
        }
        call_original!(member)
    }

    #[skyline::hook(offset = OFF_FIND_BY_DISP_ORDER)]
    pub(super) unsafe fn find_by_disp_order_hook(db: u64, disp_order: u32) -> u64 {
        let found = call_original!(db, disp_order);
        if !FOLLOW.load(Ordering::Acquire) || !is_found(found) {
            return found;
        }
        if HOLDER.load(Ordering::Acquire) == 0 {
            return found;
        }
        let follow = Follow {
            panel_count: PANEL_COUNT.load(Ordering::Acquire),
        };
        match follow.page_for(disp_order) {
            Ok(page) => show_if_changed(page),
            Err(FollowError::NoPanels) => {}
            Err(error) => {
                if !FOLLOW_COMPLAINED.swap(true, Ordering::AcqRel) {
                    skyline::println!(
                        "[stagepage] cursor-follow off for this screen: {error:?}. \
                         disp_order must be dense over selectable stages \
                         (tools/stage_disp_order.py renumber --dense)"
                    );
                }
            }
        }
        found
    }
}

#[cfg(all(not(test), feature = "stage_page"))]
pub(crate) fn install() {
    let Some(source) = PANE_SOURCE else {
        skyline::println!(
            "[stagepage] DISARMED: no pane source. The panel vector at +0x168              holds tagged hash40 stage ids, not panes (0x1b2b6d0), so there is              nothing to toggle. Hooks not installed."
        );
        return;
    };
    skyline::install_hooks!(
        live::build_panels_hook,
        live::holder_member_dtor_hook,
        live::find_by_disp_order_hook,
    );
    skyline::println!(
        "[stagepage] armed via {}: build {:#x}, teardown {:#x}, cursor {:#x}",
        source,
        OFF_BUILD_PANELS,
        OFF_HOLDER_MEMBER_DTOR,
        OFF_FIND_BY_DISP_ORDER
    );
}

#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn clone_engine_stage_select_page(page: u32) -> i32 {
    #[cfg(not(feature = "stage_page"))]
    {
        let _ = page;
        skyline::println!(
            "[stagepage] page ignored: this engine build has no stage-select paging \
             (rebuild with --features stage_page)"
        );
        -1
    }
    #[cfg(feature = "stage_page")]
    {
        use core::sync::atomic::Ordering;
        live::WANTED_PAGE.store(page as usize, Ordering::Release);
        match live::show(page as usize) {
            Ok(touched) => {
                skyline::println!(
                    "[stagepage] page {} shown now ({} panes)",
                    page + 1,
                    touched
                );
                0
            }
            Err(PageError::NoScene) => {
                skyline::println!("[stagepage] page {} queued for the next build", page + 1);
                0
            }
            Err(error) => {
                skyline::println!("[stagepage] page {} refused: {:?}", page + 1, error);
                -1
            }
        }
    }
}

#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn clone_engine_stage_select_page_follow(enable: bool) -> i32 {
    #[cfg(not(feature = "stage_page"))]
    {
        let _ = enable;
        skyline::println!(
            "[stagepage] follow ignored: this engine build has no stage-select paging \
             (rebuild with --features stage_page)"
        );
        -1
    }
    #[cfg(feature = "stage_page")]
    {
        live::FOLLOW.store(enable, core::sync::atomic::Ordering::Release);
        skyline::println!(
            "[stagepage] cursor-follow {}",
            if enable { "on" } else { "off" }
        );
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        memory: Vec<u8>,
        holder: usize,
        list: usize,
    }

    impl Fake {
        fn new(count: usize) -> Self {
            let mut memory = vec![0u8; 0x200 + count * (PANEL_ELEMENT + 0x20 + 0x60)];
            let base = memory.as_ptr() as usize;
            let holder = base;
            let list = base + 0x200;
            let panes = list + count * PANEL_ELEMENT;
            let nodes = panes + count * 0x20;
            let write = |memory: &mut Vec<u8>, at: usize, value: usize| {
                let offset = at - base;
                memory[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
            };
            for i in 0..count {
                let pane = panes + i * 0x20;
                let node = nodes + i * 0x60;
                write(&mut memory, list + i * PANEL_ELEMENT, pane);
                write(&mut memory, pane + PANE_NODE, node);
            }
            write(&mut memory, holder + PANEL_LIST_BEGIN, list);
            write(
                &mut memory,
                holder + PANEL_LIST_END,
                list + count * PANEL_ELEMENT,
            );
            write(
                &mut memory,
                holder + PANEL_LIST_CAPACITY,
                list + count * PANEL_ELEMENT,
            );
            Fake {
                memory,
                holder,
                list,
            }
        }

        fn read(&self) -> impl Fn(usize) -> usize + '_ {
            let base = self.memory.as_ptr() as usize;
            move |at: usize| {
                let offset = at - base;
                usize::from_le_bytes(self.memory[offset..offset + 8].try_into().unwrap())
            }
        }
    }

    #[test]
    fn the_constants_are_the_ones_the_game_uses() {
        assert_eq!(PANE_NODE, 0x18);
        assert_eq!(NODE_FLAGS, 0x58);
        assert_eq!(FLAG_VISIBLE, 0x01);
        assert_eq!(PANEL_ELEMENT, 24);
        assert_eq!(PANEL_LIST_CAPACITY, 0x178);
    }

    #[test]
    fn the_hook_sites_are_the_verified_ones() {
        assert_eq!(OFF_BUILD_PANELS, 0x1B2A1F0);
        assert_ne!(OFF_BUILD_PANELS, 0x1B2A21C);
        assert_eq!(OFF_HOLDER_MEMBER_DTOR, 0x1B29A50);
        assert_ne!(OFF_HOLDER_MEMBER_DTOR, 0x1B29EB0);
        assert_eq!(HOLDER_MEMBER_OFFSET, 0x208);
        assert_eq!(OFF_FIND_BY_DISP_ORDER, 0x32B2500);
    }

    #[test]
    fn reads_a_well_formed_list() {
        let fake = Fake::new(242);
        let list = PanelList::read(fake.holder, fake.read()).unwrap();
        assert_eq!(list.count, 242);
        assert_eq!(list.pages(), 2);
        assert_eq!(list.begin, fake.list);
    }

    #[test]
    fn page_one_shows_the_first_121_and_hides_the_rest() {
        let fake = Fake::new(242);
        let list = PanelList::read(fake.holder, fake.read()).unwrap();
        let mut shown = Vec::new();
        let touched = apply_with(
            &list,
            0,
            fake.read(),
            |_| true,
            |node, visible| shown.push((node, visible)),
        )
        .unwrap();
        assert_eq!(touched, 242);
        assert!(shown[..121].iter().all(|(_, visible)| *visible));
        assert!(shown[121..].iter().all(|(_, visible)| !*visible));
    }

    #[test]
    fn page_two_is_the_exact_inverse() {
        let fake = Fake::new(242);
        let list = PanelList::read(fake.holder, fake.read()).unwrap();
        let mut shown = Vec::new();
        apply_with(
            &list,
            1,
            fake.read(),
            |_| true,
            |node, visible| shown.push((node, visible)),
        )
        .unwrap();
        assert!(shown[..121].iter().all(|(_, visible)| !*visible));
        assert!(shown[121..].iter().all(|(_, visible)| *visible));
    }

    #[test]
    fn a_partial_last_page_still_works() {
        let fake = Fake::new(150);
        let list = PanelList::read(fake.holder, fake.read()).unwrap();
        assert_eq!(list.pages(), 2);
        let mut shown = Vec::new();
        apply_with(
            &list,
            1,
            fake.read(),
            |_| true,
            |_, visible| shown.push(visible),
        )
        .unwrap();
        assert_eq!(shown.iter().filter(|v| **v).count(), 150 - 121);
    }

    #[test]
    fn refuses_a_page_that_does_not_exist() {
        let fake = Fake::new(60);
        let list = PanelList::read(fake.holder, fake.read()).unwrap();
        assert_eq!(list.pages(), 1);
        assert_eq!(
            apply_with(&list, 1, fake.read(), |_| true, |_, _| {}),
            Err(PageError::PageOutOfRange { page: 1, pages: 1 })
        );
    }

    #[test]
    fn a_single_page_grid_is_untouched_by_page_zero() {
        let fake = Fake::new(119);
        let list = PanelList::read(fake.holder, fake.read()).unwrap();
        let mut shown = Vec::new();
        apply_with(
            &list,
            0,
            fake.read(),
            |_| true,
            |_, visible| shown.push(visible),
        )
        .unwrap();
        assert!(shown.iter().all(|visible| *visible));
    }

    #[test]
    fn refuses_a_null_holder() {
        assert_eq!(PanelList::read(0, |_| 0), Err(PageError::NoScene));
    }

    #[test]
    fn refuses_a_list_that_is_not_a_whole_number_of_elements() {
        let read = |at: usize| match at {
            a if a == 0x1000 + PANEL_LIST_BEGIN => 0x9000,
            a if a == 0x1000 + PANEL_LIST_END => 0x9000 + 25,
            _ => 0,
        };
        assert_eq!(
            PanelList::read(0x1000, read),
            Err(PageError::ListMalformed {
                begin: 0x9000,
                end: 0x9000 + 25
            })
        );
    }

    #[test]
    fn refuses_a_list_longer_than_the_cap_allows() {
        let count = TARGET_CAP as usize + 1;
        let read = move |at: usize| match at {
            a if a == 0x1000 + PANEL_LIST_BEGIN => 0x9000,
            a if a == 0x1000 + PANEL_LIST_END => 0x9000 + count * PANEL_ELEMENT,
            _ => 0,
        };
        assert_eq!(
            PanelList::read(0x1000, read),
            Err(PageError::ListTooLong(count))
        );
    }

    #[test]
    fn refuses_a_vector_whose_capacity_disagrees() {
        let read = |at: usize| match at {
            a if a == 0x1000 + PANEL_LIST_BEGIN => 0x9000,
            a if a == 0x1000 + PANEL_LIST_END => 0x9000 + 10 * PANEL_ELEMENT,
            a if a == 0x1000 + PANEL_LIST_CAPACITY => 0x9000 + 3 * PANEL_ELEMENT,
            _ => 0,
        };
        assert!(matches!(
            PanelList::read(0x1000, read),
            Err(PageError::CapacityMismatch { .. })
        ));
    }

    #[test]
    fn a_null_pane_or_node_is_skipped_not_dereferenced() {
        let fake = Fake::new(4);
        let list = PanelList::read(fake.holder, fake.read()).unwrap();
        let base = fake.memory.as_ptr() as usize;
        let read = |at: usize| {
            let offset = at - base;
            if at == list.element(1) {
                return 0;
            }
            usize::from_le_bytes(fake.memory[offset..offset + 8].try_into().unwrap())
        };
        let read2 = |at: usize| {
            let value = read(at);
            if at == read(list.element(2)) + PANE_NODE {
                0
            } else {
                value
            }
        };
        let mut touched = 0;
        let count = apply_with(&list, 0, read2, |_| true, |_, _| touched += 1).unwrap();
        assert_eq!(count, 2, "two elements were unusable and must be skipped");
        assert_eq!(touched, 2);
    }

    #[test]
    fn paging_never_reveals_a_panel_the_game_hid() {
        let fake = Fake::new(242);
        let list = PanelList::read(fake.holder, fake.read()).unwrap();
        let hidden_by_game = |index: usize| !(100..115).contains(&index);
        let mut shown = Vec::new();
        apply_with(&list, 0, fake.read(), hidden_by_game, |_, visible| {
            shown.push(visible)
        })
        .unwrap();
        assert!(
            shown[100..115].iter().all(|visible| !*visible),
            "a panel the game hid was revealed by paging"
        );
        assert!(shown[..100].iter().all(|visible| *visible));
        assert!(shown[115..121].iter().all(|visible| *visible));
    }

    #[test]
    fn a_hidden_panel_stays_hidden_on_every_page() {
        let fake = Fake::new(242);
        let list = PanelList::read(fake.holder, fake.read()).unwrap();
        for page in 0..list.pages() {
            let mut shown = Vec::new();
            apply_with(
                &list,
                page,
                fake.read(),
                |_| false,
                |_, visible| shown.push(visible),
            )
            .unwrap();
            assert!(
                shown.iter().all(|visible| !*visible),
                "page {page} revealed something"
            );
        }
    }

    #[test]
    fn the_baseline_is_read_from_the_nodes_the_builder_left() {
        let fake = Fake::new(10);
        let list = PanelList::read(fake.holder, fake.read()).unwrap();
        let visible_nodes: Vec<usize> = (0..10)
            .map(|i| fake.read()(fake.read()(list.element(i)) + PANE_NODE))
            .collect();
        let baseline = capture_with(&list, fake.read(), |node| {
            visible_nodes.iter().position(|n| *n == node).unwrap() % 2 == 0
        });
        for index in 0..10 {
            assert_eq!(baseline.get(index), index % 2 == 0);
        }
    }

    #[test]
    fn desired_visibility_is_page_and_baseline() {
        assert!(desired_visibility(true, 0, 0));
        assert!(
            !desired_visibility(false, 0, 0),
            "baseline wins over the page"
        );
        assert!(!desired_visibility(true, 121, 0), "wrong page");
        assert!(desired_visibility(true, 121, 1));
    }

    #[test]
    fn the_baseline_holds_every_index_the_cap_allows() {
        let mut baseline = Baseline::new();
        for index in 0..TARGET_CAP as usize {
            baseline.set(index, index % 3 == 0);
        }
        for index in 0..TARGET_CAP as usize {
            assert_eq!(baseline.get(index), index % 3 == 0, "index {index}");
        }
        let mut edge = Baseline::new();
        edge.set(Baseline::CAPACITY, true);
        assert!(!edge.get(Baseline::CAPACITY));
        assert_eq!(edge, Baseline::new());
    }

    #[test]
    fn the_baseline_survives_a_word_round_trip() {
        let mut baseline = Baseline::new();
        for index in [0usize, 63, 64, 127, 128, 191, 192, 255] {
            baseline.set(index, true);
        }
        let restored = Baseline::from_words(*baseline.words());
        assert_eq!(restored, baseline);
        for index in [0usize, 63, 64, 127, 128, 191, 192, 255] {
            assert!(restored.get(index), "index {index} lost in the round trip");
        }
    }

    #[test]
    fn the_not_found_sentinel_is_the_cursors_own() {
        assert!(!is_found(0x6900_0000_0000_0000));
        assert!(is_found(0x6900_0000_0000_0001));
        assert!(is_found(0x69_00_0000_000d_1234));
        assert!(is_found(0x6800_0000_0000_0000));
        assert!(is_found(0x69_1234_5678));
    }

    #[test]
    fn the_cursor_maps_onto_its_page() {
        let follow = Follow { panel_count: 242 };
        assert_eq!(follow.page_for(0), Ok(0));
        assert_eq!(follow.page_for(120), Ok(0));
        assert_eq!(follow.page_for(121), Ok(1));
        assert_eq!(follow.page_for(241), Ok(1));
    }

    #[test]
    fn a_cursor_beyond_the_panels_proves_the_numbering_is_not_dense() {
        let follow = Follow { panel_count: 130 };
        assert_eq!(
            follow.page_for(200),
            Err(FollowError::NotDense {
                disp_order: 200,
                panels: 130
            })
        );
        assert_eq!(follow.page_for(129), Ok(1));
    }

    #[test]
    fn no_build_means_no_follow() {
        let follow = Follow { panel_count: 0 };
        assert_eq!(follow.page_for(0), Err(FollowError::NoPanels));
    }

    #[test]
    fn a_single_page_of_stages_never_leaves_page_one() {
        let follow = Follow { panel_count: 121 };
        for disp_order in 0..121u32 {
            assert_eq!(follow.page_for(disp_order), Ok(0));
        }
    }

    #[test]
    fn the_follow_mapping_agrees_with_the_panel_mapping() {
        let fake = Fake::new(242);
        let list = PanelList::read(fake.holder, fake.read()).unwrap();
        let follow = Follow {
            panel_count: list.count,
        };
        for index in 0..list.count {
            let page = follow.page_for(index as u32).unwrap();
            assert!(
                list.on_page(index, page),
                "panel {index} is not on the page its cursor value implies"
            );
        }
    }
}
