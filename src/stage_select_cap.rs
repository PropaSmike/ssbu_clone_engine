#![allow(dead_code)]

pub const TARGET_CAP: u32 = 255;

pub const VANILLA_CAP: u32 = 0x79;

pub const SITES: &[(usize, u32, SiteKind, &str)] = &[
    (0x1B2B4A8, 0x5280_0F29, SiteKind::MovzW(9), "clamp value"),
    (
        0x1B2B4B0,
        0x7101_E51F,
        SiteKind::CmpW(8),
        "clamp comparison",
    ),
    (
        0x1B2BD08,
        0xF101_E79F,
        SiteKind::CmpX(28),
        "panel loop back-edge",
    ),
    (
        0x1C2AFF4,
        0x5280_0F2A,
        SiteKind::MovzW(10),
        "STAGE_PANEL_LIST_NUM clamp value",
    ),
    (
        0x1C2AFF8,
        0x7101_E51F,
        SiteKind::CmpW(8),
        "STAGE_PANEL_LIST_NUM comparison",
    ),
];

pub const OFF_PANEL_LOOP_TAIL: usize = 0x1B2BD04;
pub const PANEL_LOOP_TAIL_OPCODE: u32 = 0x9100_079C;
pub const OFF_PANEL_LOOP_CMP: usize = 0x1B2BD08;

#[cfg(not(test))]
static PANEL_LOOP_REFRESH_INSTALLED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(not(test))]
#[skyline::hook(offset = OFF_PANEL_LOOP_TAIL, inline)]
unsafe fn panel_loop_translation_refresh(ctx: &mut skyline::hooks::InlineCtx) {
    #[cfg(feature = "stage_resolve_probe")]
    {
        let index = ctx.registers[28].x() as u32;
        if (116..=135).contains(&index) {
            let live_cmp =
                core::ptr::read_volatile((crate::text_base() + OFF_PANEL_LOOP_CMP) as *const u32);
            skyline::println!("[paneltail] index={index} live_cmp={live_cmp:#010x}");
        }
    }

    #[cfg(not(feature = "stage_resolve_probe"))]
    let _ = ctx;
}

pub(crate) fn install_runtime_refresh() {
    #[cfg(not(test))]
    {
        use std::sync::atomic::Ordering;

        if PANEL_LOOP_REFRESH_INSTALLED.load(Ordering::Acquire) {
            return;
        }

        let observed = unsafe {
            core::ptr::read_volatile((crate::text_base() + OFF_PANEL_LOOP_TAIL) as *const u32)
        };
        if observed != PANEL_LOOP_TAIL_OPCODE {
            skyline::println!(
                "[stagecap] panel-loop refresh REFUSED at {:#x}: expected {:#010x}, found {:#010x}",
                OFF_PANEL_LOOP_TAIL,
                PANEL_LOOP_TAIL_OPCODE,
                observed,
            );
            return;
        }

        skyline::install_hook!(panel_loop_translation_refresh);
        PANEL_LOOP_REFRESH_INSTALLED.store(true, Ordering::Release);
        skyline::println!(
            "[stagecap] panel-loop translation refresh armed at {:#x}",
            OFF_PANEL_LOOP_TAIL,
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiteKind {
    MovzW(u8),
    CmpW(u8),
    CmpX(u8),
}

pub fn encode_movz_w(register: u8, immediate: u32) -> u32 {
    0x5280_0000 | ((immediate & 0xFFFF) << 5) | register as u32
}

pub fn encode_cmp_w(register: u8, immediate: u32) -> u32 {
    0x7100_0000 | ((immediate & 0xFFF) << 10) | ((register as u32) << 5) | 31
}

pub fn encode_cmp_x(register: u8, immediate: u32) -> u32 {
    0xF100_0000 | ((immediate & 0xFFF) << 10) | ((register as u32) << 5) | 31
}

pub fn encode(kind: SiteKind, immediate: u32) -> u32 {
    match kind {
        SiteKind::MovzW(r) => encode_movz_w(r, immediate),
        SiteKind::CmpW(r) => encode_cmp_w(r, immediate),
        SiteKind::CmpX(r) => encode_cmp_x(r, immediate),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CapError {
    ImmediateTooLarge(u32),
    OpcodeMismatch {
        offset: usize,
        expected: u32,
        actual: u32,
    },
}

pub fn plan(cap: u32, read_word: impl Fn(usize) -> u32) -> Result<Vec<(usize, u32)>, CapError> {
    if cap > 0xFFF {
        return Err(CapError::ImmediateTooLarge(cap));
    }
    let mut patches = Vec::with_capacity(SITES.len());
    for (offset, expected, kind, _) in SITES {
        let actual = read_word(*offset);
        let patched = encode(*kind, cap);
        if actual != *expected && actual != patched {
            return Err(CapError::OpcodeMismatch {
                offset: *offset,
                expected: *expected,
                actual,
            });
        }
        patches.push((*offset, patched));
    }
    Ok(patches)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vanilla_reader(offset: usize) -> u32 {
        SITES
            .iter()
            .find(|(at, ..)| *at == offset)
            .map(|(_, opcode, ..)| *opcode)
            .expect("unknown site")
    }

    #[test]
    fn encoders_reproduce_the_shipped_words() {
        for (offset, expected, kind, label) in SITES {
            assert_eq!(
                encode(*kind, VANILLA_CAP),
                *expected,
                "{} at {:#x} did not round-trip",
                label,
                offset
            );
        }
    }

    #[test]
    fn plans_every_site() {
        let patches = plan(TARGET_CAP, vanilla_reader).unwrap();
        assert_eq!(patches.len(), SITES.len());
        for ((offset, word), (site, _, kind, _)) in patches.iter().zip(SITES) {
            assert_eq!(offset, site);
            assert_eq!(*word, encode(*kind, TARGET_CAP));
        }
    }

    #[test]
    fn planning_is_idempotent() {
        let patches = plan(TARGET_CAP, |offset| {
            let (_, _, kind, _) = SITES
                .iter()
                .find(|(site, ..)| *site == offset)
                .expect("known site");
            encode(*kind, TARGET_CAP)
        })
        .unwrap();
        assert_eq!(patches.len(), SITES.len());
    }

    #[test]
    fn the_lua_global_clamp_is_covered() {
        assert!(
            SITES.iter().any(|(off, _, _, _)| *off == 0x1C2AFF4),
            "the STAGE_PANEL_LIST_NUM clamp value must be raised"
        );
        assert!(
            SITES.iter().any(|(off, _, _, _)| *off == 0x1C2AFF8),
            "the STAGE_PANEL_LIST_NUM comparison must be raised"
        );
        let in_that_function = SITES
            .iter()
            .filter(|(off, _, _, _)| (0x1C2A000..0x1C2C000).contains(off))
            .count();
        assert_eq!(in_that_function, 2);
    }

    #[test]
    fn patched_words_differ_only_in_the_immediate() {
        for (_, expected, kind, _) in SITES {
            let patched = encode(*kind, TARGET_CAP);
            let mask = match kind {
                SiteKind::MovzW(_) => !(0xFFFFu32 << 5),
                _ => !(0xFFFu32 << 10),
            };
            assert_eq!(expected & mask, patched & mask);
        }
    }

    #[test]
    fn refuses_an_immediate_that_does_not_fit() {
        assert_eq!(
            plan(0x1000, vanilla_reader),
            Err(CapError::ImmediateTooLarge(0x1000))
        );
    }

    #[test]
    fn refuses_when_something_else_patched_a_site() {
        let poisoned = SITES[1].0;
        let error = plan(TARGET_CAP, |offset| {
            if offset == poisoned {
                0xDEAD_BEEF
            } else {
                vanilla_reader(offset)
            }
        })
        .unwrap_err();
        assert_eq!(
            error,
            CapError::OpcodeMismatch {
                offset: poisoned,
                expected: SITES[1].1,
                actual: 0xDEAD_BEEF
            }
        );
    }

    #[test]
    fn the_cap_reaches_what_disp_order_can_express() {
        assert_eq!(TARGET_CAP, 255);
        assert!(TARGET_CAP > 127);
        assert!(TARGET_CAP > VANILLA_CAP);
    }

    #[test]
    fn panel_loop_refresh_precedes_the_cap_compare() {
        assert_eq!(OFF_PANEL_LOOP_TAIL, 0x1B2BD04);
        assert_eq!(PANEL_LOOP_TAIL_OPCODE, 0x9100_079C);
        assert_eq!(OFF_PANEL_LOOP_CMP, OFF_PANEL_LOOP_TAIL + 4);
    }
}
