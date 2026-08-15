#![allow(dead_code)]

pub const OFF_STAGE_BASE_PRE_SETUP: usize = 0x25D8E00;

pub const PRE_SETUP_OPCODE: u32 = 0x6DB733ED;

pub const STAGE_ID: usize = 0x08;

pub const OPEN_SITE: usize = 0x25DCC64;
pub const OPEN_OPCODE: u32 = 0x6D4223E9;
pub const OPEN_STAGE_REG: usize = 20;

pub const CLOSE_SITES: [(usize, u32, usize); 3] = [
    (0x2602918, 0xF9419A68, 19),
    (0x2602CAC, 0xF9419AA8, 21),
    (0x2606888, 0xF9419A68, 19),
];

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Borrow { minted: u32, donor: u32 },
    PassThrough,
}

pub fn decide(minted: u32, donor_of: impl Fn(u32) -> Option<u32>) -> Action {
    match donor_of(minted) {
        Some(donor) if donor != minted => Action::Borrow { minted, donor },
        _ => Action::PassThrough,
    }
}

pub mod pending {
    use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

    pub const SLOTS: usize = 4;

    static PTR: [AtomicU64; SLOTS] = [
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
    ];
    static MINTED: [AtomicU32; SLOTS] = [
        AtomicU32::new(0),
        AtomicU32::new(0),
        AtomicU32::new(0),
        AtomicU32::new(0),
    ];

    pub fn open(ptr: u64, minted: u32) -> bool {
        for slot in 0..SLOTS {
            if PTR[slot]
                .compare_exchange(0, ptr, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                MINTED[slot].store(minted, Ordering::Release);
                return true;
            }
        }
        false
    }

    pub fn close(ptr: u64) -> Option<u32> {
        for slot in 0..SLOTS {
            if PTR[slot].load(Ordering::Acquire) != ptr {
                continue;
            }
            let minted = MINTED[slot].load(Ordering::Acquire);
            if PTR[slot]
                .compare_exchange(ptr, 0, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return Some(minted);
            }
        }
        None
    }

    #[cfg(test)]
    pub fn clear() {
        for slot in 0..SLOTS {
            PTR[slot].store(0, Ordering::Release);
            MINTED[slot].store(0, Ordering::Release);
        }
    }
}

#[cfg(all(not(test), feature = "stage_config_bridge"))]
mod live {
    use super::*;
    use core::sync::atomic::{AtomicU32, Ordering};

    static LAST_REPORTED: AtomicU32 = AtomicU32::new(u32::MAX);

    #[skyline::hook(offset = OPEN_SITE, inline)]
    unsafe fn open_hook(ctx: &mut skyline::hooks::InlineCtx) {
        let stage = ctx.registers[OPEN_STAGE_REG].x();
        if stage == 0 {
            return;
        }
        let at = (stage as *mut u8).add(STAGE_ID) as *mut u32;
        if let Some(stale) = pending::close(stage) {
            core::ptr::write_volatile(at, stale);
        }
        let minted = core::ptr::read_volatile(at);
        let Action::Borrow { donor, .. } = decide(minted, crate::stage_dispatch::donor_for) else {
            return;
        };
        if !pending::open(stage, minted) {
            skyline::println!(
                "[stagecfg] REFUSED to lend StageID {minted} the id {donor}: all {} \
                 restore slots are in use",
                pending::SLOTS
            );
            return;
        }
        core::ptr::write_volatile(at, donor);
        if LAST_REPORTED.swap(minted, Ordering::Relaxed) != minted {
            skyline::println!(
                "[stagecfg] StageID {minted} borrows {donor}'s config_stage.toml entry"
            );
        }
    }

    macro_rules! close_site {
        ($name:ident, $site:expr, $reg:expr) => {
            #[skyline::hook(offset = $site, inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let stage = ctx.registers[$reg].x();
                if stage == 0 {
                    return;
                }
                if let Some(minted) = pending::close(stage) {
                    core::ptr::write_volatile((stage as *mut u8).add(STAGE_ID) as *mut u32, minted);
                }
            }
        };
    }

    close_site!(close_at_2602918, 0x2602918, 19);
    close_site!(close_at_2602cac, 0x2602CAC, 21);
    close_site!(close_at_2606888, 0x2606888, 19);

    pub fn install() {
        unsafe {
            let text = skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as *const u8;
            let sites = [
                (
                    OFF_STAGE_BASE_PRE_SETUP,
                    PRE_SETUP_OPCODE,
                    "pre-setup entry",
                ),
                (OPEN_SITE, OPEN_OPCODE, "open"),
                (CLOSE_SITES[0].0, CLOSE_SITES[0].1, "close"),
                (CLOSE_SITES[1].0, CLOSE_SITES[1].1, "close"),
                (CLOSE_SITES[2].0, CLOSE_SITES[2].1, "close"),
            ];
            for (site, expected, label) in sites {
                let observed = core::ptr::read_volatile(text.add(site) as *const u32);
                if observed != expected {
                    skyline::println!(
                        "[stagecfg] REFUSED {label} site {site:#x}: expected \
                         {expected:#010x}, found {observed:#010x}; config_stage.toml \
                         bindings will not reach minted stages"
                    );
                    return;
                }
            }
            skyline::install_hooks!(
                open_hook,
                close_at_2602918,
                close_at_2602cac,
                close_at_2606888,
            );
        }
        skyline::println!(
            "[stagecfg] armed inside pre-setup at {OPEN_SITE:#x}, shut from its {} \
             callers; {OFF_STAGE_BASE_PRE_SETUP:#x} left untouched for stage_config",
            CLOSE_SITES.len()
        );
    }
}

#[cfg(all(not(test), feature = "stage_config_bridge"))]
pub(crate) fn install() {
    live::install();
}

#[cfg(not(all(not(test), feature = "stage_config_bridge")))]
pub(crate) fn install() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_offsets_are_the_ones_stage_config_uses() {
        assert_eq!(STAGE_ID, 0x08);
        assert_eq!(OFF_STAGE_BASE_PRE_SETUP, 0x25D8E00);
    }

    #[test]
    fn no_hook_site_is_the_function_entry() {
        assert_ne!(OPEN_SITE, OFF_STAGE_BASE_PRE_SETUP);
        assert!(
            OPEN_SITE > OFF_STAGE_BASE_PRE_SETUP + 0x20,
            "the open must be clear of the 20 bytes an entry hook overwrites"
        );
        for (site, _, _) in CLOSE_SITES {
            assert!(
                !(OFF_STAGE_BASE_PRE_SETUP..=OFF_STAGE_BASE_PRE_SETUP + 0x20).contains(&site),
                "close site {site:#x} is inside the entry patch window"
            );
        }
    }

    #[test]
    fn the_open_site_is_the_epilogue_fp_restore() {
        let expected = 0x6D40_0000u32 | ((0x20 / 8) << 15) | (8 << 10) | (31 << 5) | 9;
        assert_eq!(OPEN_OPCODE, expected);
        assert_eq!(
            OPEN_STAGE_REG, 20,
            "x20 is what 0x25dcbcc read the StageID from"
        );
    }

    #[test]
    fn every_close_site_reads_0x330_off_the_stage_register() {
        for (site, opcode, reg) in CLOSE_SITES {
            let expected = 0xF940_0000u32 | ((0x330 / 8) << 10) | ((reg as u32) << 5) | 8;
            assert_eq!(
                opcode, expected,
                "close site {site:#x} names register x{reg}"
            );
        }
    }

    #[test]
    fn a_minted_stage_borrows_its_donors_id() {
        assert_eq!(
            decide(383, |id| (id == 383).then_some(20)),
            Action::Borrow {
                minted: 383,
                donor: 20
            }
        );
    }

    #[test]
    fn a_vanilla_stage_is_untouched() {
        assert_eq!(decide(20, |_| None), Action::PassThrough);
    }

    #[test]
    fn a_stage_that_is_its_own_donor_is_untouched() {
        assert_eq!(decide(20, |id| Some(id)), Action::PassThrough);
    }

    #[test]
    fn the_body_never_sees_the_donor() {
        let mut id = 383u32;
        assert_eq!(id, 383, "the body must see the minted id");
        if let Action::Borrow { donor, .. } = decide(id, |_| Some(20)) {
            id = donor;
        }
        assert_eq!(id, 20, "stage_config must see the donor id");
        id = 383;
        assert_eq!(id, 383, "the game must have its own id back");
    }

    #[test]
    fn the_borrow_names_both_ids_so_it_can_be_undone() {
        let Action::Borrow { minted, donor } = decide(383, |_| Some(20)) else {
            panic!("expected a borrow");
        };
        assert_eq!(minted, 383);
        assert_eq!(donor, 20);
        assert_ne!(minted, donor);
    }

    static TABLE: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn with_table(body: impl FnOnce()) {
        let _held = TABLE.lock().unwrap_or_else(|e| e.into_inner());
        pending::clear();
        body();
        pending::clear();
    }

    #[test]
    fn a_window_is_shut_by_the_object_that_opened_it() {
        with_table(|| {
            assert!(pending::open(0x1000, 383));
            assert!(pending::open(0x2000, 364));
            assert_eq!(pending::close(0x2000), Some(364));
            assert_eq!(pending::close(0x1000), Some(383));
        });
    }

    #[test]
    fn closing_a_stage_that_borrowed_nothing_is_free() {
        with_table(|| {
            assert_eq!(pending::close(0x1000), None);
        });
    }

    #[test]
    fn closing_twice_only_restores_once() {
        with_table(|| {
            assert!(pending::open(0x1000, 383));
            assert_eq!(pending::close(0x1000), Some(383));
            assert_eq!(pending::close(0x1000), None);
        });
    }

    #[test]
    fn a_full_table_refuses_to_open_another_window() {
        with_table(|| {
            for slot in 0..pending::SLOTS {
                assert!(pending::open(0x1000 + slot as u64, 383));
            }
            assert!(!pending::open(0x9000, 383));
        });
    }
}
