#![allow(dead_code)]

const ADRP_PAGE_REACH: i64 = 1 << 20;

const ADD_IMM_MAX: usize = 0xFFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableReference {
    pub adrp_at: usize,
    pub register: u8,
    pub delta: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PlanError {
    OutOfAdrpRange {
        adrp_at: usize,
        target: usize,
        pages: i64,
    },
    InvalidRegister {
        adrp_at: usize,
        register: u8,
    },
    Misaligned {
        address: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SitePatch {
    pub adrp_at: usize,
    pub register: u8,
    pub adrp_word: u32,
    pub add_word: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RelocationPlan {
    pub patches: Vec<SitePatch>,
    pub table_pointer: usize,
}

pub fn encode_adrp(pc: usize, target: usize, register: u8) -> Result<u32, PlanError> {
    if pc % 4 != 0 {
        return Err(PlanError::Misaligned { address: pc });
    }
    let pages = ((target & !0xFFF) as i64 - (pc & !0xFFF) as i64) >> 12;
    if pages >= ADRP_PAGE_REACH || pages < -ADRP_PAGE_REACH {
        return Err(PlanError::OutOfAdrpRange {
            adrp_at: pc,
            target,
            pages,
        });
    }
    let immediate = pages as u32 & 0x1F_FFFF;
    Ok(0x9000_0000
        | ((immediate & 0b11) << 29)
        | (((immediate >> 2) & 0x7_FFFF) << 5)
        | register as u32)
}

pub fn encode_add(target: usize, register: u8) -> u32 {
    let offset = (target & 0xFFF) as u32;
    debug_assert!(offset as usize <= ADD_IMM_MAX);
    0x9100_0000 | (offset << 10) | ((register as u32) << 5) | register as u32
}

pub fn decode_pair(pc: usize, adrp_word: u32, add_word: u32) -> Option<usize> {
    if adrp_word & 0x9F00_0000 != 0x9000_0000 {
        return None;
    }
    if add_word & 0xFF80_0000 != 0x9100_0000 {
        return None;
    }
    let immlo = ((adrp_word >> 29) & 0b11) as i64;
    let immhi = ((adrp_word >> 5) & 0x7_FFFF) as i64;
    let mut pages = (immhi << 2) | immlo;
    if pages & (1 << 20) != 0 {
        pages -= 1 << 21;
    }
    let page = (pc & !0xFFF) as i64 + (pages << 12);
    let offset = ((add_word >> 10) & 0xFFF) as i64;
    Some((page + offset) as usize)
}

unsafe fn write_text_pair(address: usize, first: u32, second: u32) -> bool {
    crate::text_patch::write_words(address, &[first, second])
}

impl RelocationPlan {
    pub fn build(references: &[TableReference], table_pointer: usize) -> Result<Self, PlanError> {
        let mut patches = Vec::with_capacity(references.len());
        for reference in references {
            if reference.register > 30 {
                return Err(PlanError::InvalidRegister {
                    adrp_at: reference.adrp_at,
                    register: reference.register,
                });
            }
            let target = table_pointer + reference.delta;
            let adrp_word = encode_adrp(reference.adrp_at, target, reference.register)?;
            let add_word = encode_add(target, reference.register);
            debug_assert_eq!(
                decode_pair(reference.adrp_at, adrp_word, add_word),
                Some(target)
            );
            patches.push(SitePatch {
                adrp_at: reference.adrp_at,
                register: reference.register,
                adrp_word,
                add_word,
            });
        }
        Ok(Self {
            patches,
            table_pointer,
        })
    }

    pub unsafe fn apply(&self) -> usize {
        let mut refused = 0usize;
        for patch in &self.patches {
            let site = patch.adrp_at as *const u32;
            if site.read_volatile() == patch.adrp_word
                && site.add(1).read_volatile() == patch.add_word
            {
                continue;
            }
            if !write_text_pair(patch.adrp_at, patch.adrp_word, patch.add_word) {
                refused += 1;
            }
        }
        refused
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REAL_SITE: usize = 0x0309_8020;
    const REAL_ADRP: u32 = 0xB000_A58A;
    const REAL_ADD: u32 = 0x911F_A14A;
    const STAGE_ID_TABLE: usize = 0x0454_97E8;

    const SURVEYED_FREE_REGION: usize = 0x1114_F000;

    fn reference(adrp_at: usize, register: u8) -> TableReference {
        TableReference {
            adrp_at,
            register,
            delta: 0,
        }
    }

    fn biased(adrp_at: usize, register: u8, delta: usize) -> TableReference {
        TableReference {
            adrp_at,
            register,
            delta,
        }
    }

    #[test]
    fn a_biased_reference_keeps_its_bias() {
        let plan =
            RelocationPlan::build(&[biased(REAL_SITE, 10, 0x30)], SURVEYED_FREE_REGION).unwrap();
        let patch = &plan.patches[0];
        assert_eq!(
            decode_pair(patch.adrp_at, patch.adrp_word, patch.add_word),
            Some(SURVEYED_FREE_REGION + 0x30)
        );
    }

    #[test]
    fn an_unbiased_reference_still_lands_on_the_base() {
        let plan =
            RelocationPlan::build(&[reference(REAL_SITE, 10)], SURVEYED_FREE_REGION).unwrap();
        let patch = &plan.patches[0];
        assert_eq!(
            decode_pair(patch.adrp_at, patch.adrp_word, patch.add_word),
            Some(SURVEYED_FREE_REGION)
        );
    }

    #[test]
    fn decodes_the_real_pair_from_the_image() {
        assert_eq!(
            decode_pair(REAL_SITE, REAL_ADRP, REAL_ADD),
            Some(STAGE_ID_TABLE)
        );
    }

    #[test]
    fn re_encoding_the_vanilla_target_reproduces_the_original_words() {
        assert_eq!(
            encode_adrp(REAL_SITE, STAGE_ID_TABLE, 10).unwrap(),
            REAL_ADRP
        );
        assert_eq!(encode_add(STAGE_ID_TABLE, 10), REAL_ADD);
    }

    #[test]
    fn round_trips_through_the_surveyed_free_region() {
        let target = SURVEYED_FREE_REGION + 0x123;
        let adrp = encode_adrp(REAL_SITE, target, 10).unwrap();
        let add = encode_add(target, 10);
        assert_eq!(decode_pair(REAL_SITE, adrp, add), Some(target));
    }

    #[test]
    fn round_trips_for_every_register_and_page_offset() {
        for register in 0..=30u8 {
            for offset in [0usize, 1, 0x7E8, 0xFFF] {
                let target = SURVEYED_FREE_REGION + offset;
                let adrp = encode_adrp(REAL_SITE, target, register).unwrap();
                let add = encode_add(target, register);
                assert_eq!(decode_pair(REAL_SITE, adrp, add), Some(target));
                assert_eq!(adrp & 0x1F, register as u32);
                assert_eq!(add & 0x1F, register as u32);
                assert_eq!((add >> 5) & 0x1F, register as u32);
            }
        }
    }

    #[test]
    fn handles_backward_relocation() {
        let target = 0x0083_2000;
        let adrp = encode_adrp(REAL_SITE, target, 8).unwrap();
        let add = encode_add(target, 8);
        assert_eq!(decode_pair(REAL_SITE, adrp, add), Some(target));
    }

    #[test]
    fn plans_every_reference_span_extreme() {
        let references = [reference(0x002E_B058, 10), reference(0x034C_0BE0, 0)];
        let plan = RelocationPlan::build(&references, SURVEYED_FREE_REGION).unwrap();
        assert_eq!(plan.patches.len(), 2);
        for patch in &plan.patches {
            assert_eq!(
                decode_pair(patch.adrp_at, patch.adrp_word, patch.add_word),
                Some(SURVEYED_FREE_REGION)
            );
        }
    }

    #[test]
    fn refuses_a_target_beyond_adrp_reach() {
        let references = [reference(REAL_SITE, 10)];
        let error = RelocationPlan::build(&references, 0x11_0256_0000).unwrap_err();
        assert!(matches!(error, PlanError::OutOfAdrpRange { .. }));
    }

    #[test]
    fn refuses_the_whole_plan_when_a_single_site_is_unreachable() {
        let references = [reference(REAL_SITE, 10), reference(0x0000_1000, 9)];
        assert!(RelocationPlan::build(&references, 0x1_0000_0000 + 0x1000).is_err());
    }

    #[test]
    fn refuses_sp_as_a_destination_register() {
        let references = [reference(REAL_SITE, 31)];
        assert_eq!(
            RelocationPlan::build(&references, SURVEYED_FREE_REGION).unwrap_err(),
            PlanError::InvalidRegister {
                adrp_at: REAL_SITE,
                register: 31
            }
        );
    }

    #[test]
    fn refuses_misaligned_sites() {
        let references = [reference(REAL_SITE + 2, 10)];
        assert!(RelocationPlan::build(&references, SURVEYED_FREE_REGION).is_err());
    }
}
