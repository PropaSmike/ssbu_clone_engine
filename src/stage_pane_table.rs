#![allow(dead_code)]

pub const HANDLE_INNER: usize = 0x08;
pub const PANE_NODE: usize = 0x18;
pub const NODE_FLAGS: usize = 0x58;
pub const FLAG_VISIBLE: u8 = 0x01;

pub const MAX_PANES: usize = 256;

pub const PAGE_SIZE: usize = 121;

pub fn page_of(index: usize) -> usize {
    index / PAGE_SIZE
}

pub fn pages_for(panes: usize) -> usize {
    panes.div_ceil(PAGE_SIZE).max(1)
}

pub fn visible_on(index: usize, page: usize, drawn: usize) -> bool {
    index < drawn && page_of(index) == page
}

#[cfg(all(not(test), feature = "stage_pane_table"))]
pub(crate) mod live {
    use super::*;
    use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    const OFF_PANE_CONSUMER: usize = 0x3777D50;

    static CAPTURING: AtomicBool = AtomicBool::new(false);
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    static HANDLES: [AtomicUsize; MAX_PANES] = [const { AtomicUsize::new(0) }; MAX_PANES];
    static APPLIED: AtomicUsize = AtomicUsize::new(usize::MAX);

    pub fn begin_capture() {
        PAGE.store(0, Ordering::Release);
        TRIGGER_WAS_DOWN.store(false, Ordering::Release);
        COUNT.store(0, Ordering::Release);
        APPLIED.store(usize::MAX, Ordering::Release);
        CAPTURING.store(true, Ordering::Release);
    }

    pub fn end_capture() -> usize {
        CAPTURING.store(false, Ordering::Release);
        COUNT.load(Ordering::Acquire)
    }

    pub fn retract() {
        CAPTURING.store(false, Ordering::Release);
        COUNT.store(0, Ordering::Release);
        APPLIED.store(usize::MAX, Ordering::Release);
    }

    pub fn captured() -> usize {
        COUNT.load(Ordering::Acquire)
    }

    #[skyline::hook(offset = OFF_PANE_CONSUMER)]
    unsafe fn pane_consumer_hook(handle: u64, second: u64, third: f64) -> u64 {
        let lr: usize;
        #[cfg(target_arch = "aarch64")]
        core::arch::asm!("mov {}, x30", out(reg) lr);
        #[cfg(not(target_arch = "aarch64"))]
        {
            lr = 0;
        }
        if CAPTURING.load(Ordering::Acquire) && handle != 0 && is_the_panel_loop(lr) {
            let index = COUNT.load(Ordering::Acquire);
            if index < MAX_PANES {
                HANDLES[index].store(handle as usize, Ordering::Release);
                COUNT.store(index + 1, Ordering::Release);
            }
        }
        call_original!(handle, second, third)
    }

    fn is_the_panel_loop(lr: usize) -> bool {
        let text =
            unsafe { skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as usize };
        lr.wrapping_sub(text) == PANEL_LOOP_RETURN
    }

    const PANEL_LOOP_RETURN: usize = 0x1B2B6BC;

    unsafe fn pane_of(index: usize) -> Option<*mut u8> {
        let handle = HANDLES.get(index)?.load(Ordering::Acquire);
        if handle == 0 {
            return None;
        }
        let inner = core::ptr::read_volatile((handle + HANDLE_INNER) as *const usize);
        if inner == 0 {
            return None;
        }
        let pane = core::ptr::read_volatile((inner + PANE_NODE) as *const usize);
        (pane != 0).then_some(pane as *mut u8)
    }

    unsafe fn set_visible(pane: *mut u8, visible: bool) {
        let at = pane.add(NODE_FLAGS);
        let flags = core::ptr::read_volatile(at);
        let next = if visible {
            flags | FLAG_VISIBLE
        } else {
            flags & !FLAG_VISIBLE
        };
        if next != flags {
            core::ptr::write_volatile(at, next);
        }
    }

    pub unsafe fn show_page(page: usize, drawn: usize) -> usize {
        if APPLIED.load(Ordering::Acquire) == page {
            return 0;
        }
        let count = COUNT.load(Ordering::Acquire).min(MAX_PANES);
        if count != drawn {
            skyline::println!(
                "[stagepane] REFUSING to page: captured {count} pane(s) but the                  build drew {drawn} cell(s); these are not the grid's panes"
            );
            return 0;
        }
        let mut touched = 0;
        for index in 0..count {
            let Some(pane) = pane_of(index) else { continue };
            set_visible(pane, visible_on(index, page, drawn));
            touched += 1;
        }
        APPLIED.store(page, Ordering::Release);
        touched
    }

    static DRAWN: AtomicUsize = AtomicUsize::new(0);

    pub fn set_drawn(drawn: usize) {
        DRAWN.store(drawn, Ordering::Release);
    }

    const OFF_FIND_BY_DISP_ORDER: usize = 0x32B2500;

    #[skyline::hook(offset = OFF_FIND_BY_DISP_ORDER)]
    unsafe fn find_by_disp_order_hook(db: u64, disp_order: u32) -> u64 {
        let found = call_original!(db, disp_order);
        let drawn = DRAWN.load(Ordering::Acquire);
        if drawn == 0 || COUNT.load(Ordering::Acquire) == 0 {
            return found;
        }
        let index = disp_order as usize;
        if index < drawn {
            let page = page_of(index);
            note_page(page);
            show_page(page, drawn);
        }
        found
    }

    const OFF_CURSOR_UPDATE: usize = 0x17935A0;

    static TRIGGER_WAS_DOWN: core::sync::atomic::AtomicBool =
        core::sync::atomic::AtomicBool::new(false);
    static PAGE: AtomicUsize = AtomicUsize::new(0);

    #[skyline::hook(offset = OFF_CURSOR_UPDATE)]
    unsafe fn cursor_update_hook(cursor: u64) -> u64 {
        let drawn = DRAWN.load(Ordering::Acquire);
        let panes = COUNT.load(Ordering::Acquire);
        if drawn == 0 || panes == 0 {
            return call_original!(cursor);
        }
        let pages = pages_for(drawn);
        if pages > 1 {
            let back = ninput::any::is_down(ninput::Buttons::ZL);
            let forward = ninput::any::is_down(ninput::Buttons::ZR);
            let down = back || forward;
            if down && !TRIGGER_WAS_DOWN.swap(true, Ordering::AcqRel) {
                let page = PAGE.load(Ordering::Acquire);
                let next = if forward {
                    (page + 1) % pages
                } else {
                    (page + pages - 1) % pages
                };
                PAGE.store(next, Ordering::Release);
                let touched = show_page(next, drawn);
                skyline::println!(
                    "[stagepane] {} -> page {} of {pages} ({touched} pane(s))",
                    if forward { "ZR" } else { "ZL" },
                    next + 1
                );
            } else if !down {
                TRIGGER_WAS_DOWN.store(false, Ordering::Release);
            }
        }
        call_original!(cursor)
    }

    pub fn note_page(page: usize) {
        PAGE.store(page, Ordering::Release);
    }

    pub fn census(drawn: usize) {
        let count = COUNT.load(Ordering::Acquire);
        skyline::println!(
            "[stagepane] captured {count} pane(s) for {drawn} drawn cell(s); \
             {} page(s) of {PAGE_SIZE}",
            pages_for(count.min(drawn.max(1)))
        );
    }

    pub fn install() {
        unsafe {
            skyline::install_hook!(pane_consumer_hook);
            skyline::install_hook!(find_by_disp_order_hook);
            skyline::install_hook!(cursor_update_hook);
        }
        skyline::println!(
            "[stagepane] pane capture armed at {OFF_PANE_CONSUMER:#x}, bracketed by \
             the panel build, up to {MAX_PANES} panes; ZL/ZR page the grid via \
             {OFF_CURSOR_UPDATE:#x} and the cursor follows via \
             {OFF_FIND_BY_DISP_ORDER:#x}"
        );
    }
}

#[cfg(not(all(not(test), feature = "stage_pane_table")))]
pub(crate) fn install() {}

#[cfg(all(not(test), feature = "stage_pane_table"))]
pub(crate) fn install() {
    live::install();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_offsets_match_the_builders_hide_path() {
        assert_eq!(HANDLE_INNER, 0x08);
        assert_eq!(PANE_NODE, 0x18);
        assert_eq!(NODE_FLAGS, 0x58);
        assert_eq!(FLAG_VISIBLE, 0x01);
    }

    #[test]
    fn a_page_is_a_contiguous_run_of_cells() {
        assert_eq!(page_of(0), 0);
        assert_eq!(page_of(PAGE_SIZE - 1), 0);
        assert_eq!(page_of(PAGE_SIZE), 1);
        assert_eq!(page_of(PAGE_SIZE * 2), 2);
    }

    #[test]
    fn pages_cover_every_pane() {
        assert_eq!(pages_for(0), 1);
        assert_eq!(pages_for(1), 1);
        assert_eq!(pages_for(PAGE_SIZE), 1);
        assert_eq!(pages_for(PAGE_SIZE + 1), 2);
        assert_eq!(pages_for(242), 2);
        assert_eq!(pages_for(243), 3);
    }

    #[test]
    fn an_unfilled_pane_is_never_shown() {
        let drawn = 130;
        for index in drawn..242 {
            for page in 0..2 {
                assert!(!visible_on(index, page, drawn), "pane {index} page {page}");
            }
        }
    }

    #[test]
    fn each_page_shows_its_own_cells_and_no_others() {
        let drawn = 130;
        for index in 0..PAGE_SIZE {
            assert!(visible_on(index, 0, drawn));
            assert!(!visible_on(index, 1, drawn));
        }
        for index in PAGE_SIZE..drawn {
            assert!(!visible_on(index, 0, drawn));
            assert!(visible_on(index, 1, drawn));
        }
    }

    #[test]
    fn every_drawn_cell_appears_on_exactly_one_page() {
        for drawn in [1usize, 120, 121, 122, 130, 242] {
            for index in 0..drawn {
                let shown: usize = (0..pages_for(drawn))
                    .filter(|page| visible_on(index, *page, drawn))
                    .count();
                assert_eq!(shown, 1, "drawn {drawn} index {index}");
            }
        }
    }
}
