#![allow(dead_code)]

pub(crate) struct StageTable {
    pub name: &'static str,
    pub address: usize,
    pub element_size: usize,
    pub native_length: usize,
    pub expanded_length: usize,
}

pub(crate) struct ExpectedOpcode {
    pub offset: usize,
    pub opcode: u32,
    pub label: &'static str,
}

pub(crate) struct StageReference {
    pub table: &'static str,
    pub adrp_at: usize,
    pub adrp_opcode: u32,
    pub add_opcode: u32,
    pub register: u8,
    pub delta: usize,
}

#[derive(Clone, Copy)]
pub(crate) struct StageBound {
    pub table: &'static str,
    pub address: usize,
    pub old_opcode: u32,
    pub new_opcode: u32,
    pub old_value: u32,
    pub new_value: u32,
}

#[derive(Clone, Copy)]
pub(crate) struct StageSwitch {
    pub cmp_at: usize,
    pub cmp_opcode: u32,
    pub new_cmp_opcode: u32,
    pub adrp_at: usize,
    pub adrp_opcode: u32,
    pub add_opcode: u32,
    pub base_register: u8,
    pub jump_table: usize,
    pub entries: usize,
    pub default_target: usize,
}

include!("stage_tables_13_0_4.rs");

#[derive(Debug, PartialEq, Eq)]
pub enum BoundError {
    OpcodeMismatch {
        address: usize,
        expected: u32,
        actual: u32,
    },
    TableNotRelocated {
        address: usize,
        table: &'static str,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct BoundPatch {
    pub address: usize,
    pub word: u32,
}

pub fn plan_widening(
    bounds: &[StageBound],
    relocated: &[&str],
    read_word: impl Fn(usize) -> u32,
) -> Result<Vec<BoundPatch>, BoundError> {
    let mut patches = Vec::with_capacity(bounds.len());
    for bound in bounds {
        if !relocated.contains(&bound.table) {
            return Err(BoundError::TableNotRelocated {
                address: bound.address,
                table: bound.table,
            });
        }
        let actual = read_word(bound.address);
        if actual != bound.old_opcode {
            return Err(BoundError::OpcodeMismatch {
                address: bound.address,
                expected: bound.old_opcode,
                actual,
            });
        }
        patches.push(BoundPatch {
            address: bound.address,
            word: bound.new_opcode,
        });
    }
    Ok(patches)
}

pub fn bounds_for(tables: &[&str]) -> Vec<StageBound> {
    STAGE_BOUNDS
        .iter()
        .filter(|bound| tables.contains(&bound.table))
        .copied()
        .collect()
}

pub fn unwidened_count() -> usize {
    UNWIDENED_BOUNDS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reader(overrides: &'static [(usize, u32)]) -> impl Fn(usize) -> u32 + '_ {
        move |address| {
            overrides
                .iter()
                .find(|(at, _)| *at == address)
                .map(|(_, word)| *word)
                .unwrap_or_else(|| {
                    STAGE_BOUNDS
                        .iter()
                        .find(|b| b.address == address)
                        .map(|b| b.old_opcode)
                        .expect("unknown address")
                })
        }
    }

    const ALL: &[&str] = &["stage_place", "stage_place_aux", "stage_id"];

    #[test]
    fn the_generated_census_is_the_shape_the_documents_claim() {
        assert_eq!(STAGE_TABLES.len(), 3);
        assert_eq!(STAGE_REFERENCES.len(), REFERENCE_COUNT);
        assert_eq!(STAGE_BOUNDS.len(), QUALIFIED_BOUND_COUNT);
        assert_eq!(REFERENCE_COUNT, 284);
        assert_eq!(QUALIFIED_BOUND_COUNT, 283);
        assert_eq!(UNWIDENED_BOUNDS, 4);
    }

    #[test]
    fn the_stage_id_clamp_is_widened() {
        let bound = STAGE_BOUNDS
            .iter()
            .find(|bound| bound.address == 0x178ab8c)
            .expect("the StageID clamp must be widened");
        assert_eq!(bound.table, "stage_id");
        assert_eq!(bound.old_value, 363);
        assert_eq!(bound.new_value, 511);
        assert_eq!(
            bound.old_opcode & 0xFF80_03FF,
            bound.new_opcode & 0xFF80_03FF
        );
        assert_eq!((bound.new_opcode >> 10) & 0xFFF, 511);
    }

    #[test]
    fn the_unrolled_place_scans_keep_their_residue() {
        let unrolled: Vec<_> = STAGE_BOUNDS
            .iter()
            .filter(|bound| bound.table == "stage_place" && bound.old_value == 138)
            .filter(|bound| bound.new_value == 190)
            .collect();
        assert_eq!(unrolled.len(), 4);
        let place = STAGE_TABLES
            .iter()
            .find(|table| table.name == "stage_place")
            .unwrap();
        for bound in &unrolled {
            assert_eq!(
                bound.new_value % 4,
                place.native_length as u32 % 4,
                "{:#x} must keep the residue the unroll depends on",
                bound.address
            );
            assert!((bound.new_value as usize) <= place.expanded_length);
        }
        for address in [0x32b21fc, 0x32b244c, 0x240ca5c, 0x240cfec] {
            assert!(unrolled.iter().any(|bound| bound.address == address));
        }
    }

    #[test]
    fn the_byte_length_loop_bounds_are_qualified() {
        let byte_length: Vec<_> = STAGE_BOUNDS
            .iter()
            .filter(|bound| bound.old_value == 364 * 0x48)
            .collect();
        assert_eq!(byte_length.len(), 13);
        for bound in &byte_length {
            assert_eq!(bound.table, "stage_id");
            assert_eq!(bound.new_value, 512 * 0x48);
            assert_eq!(bound.old_opcode & 0x7F80_0000, 0x5280_0000);
            assert_eq!(bound.new_opcode & 0x7F80_0000, 0x5280_0000);
            assert_eq!(bound.old_opcode & 0x1F, bound.new_opcode & 0x1F);
            assert_eq!((bound.new_opcode >> 5) & 0xFFFF, 512 * 0x48);
        }
        assert!(byte_length.iter().any(|bound| bound.address == 0x1739ec8));
    }

    #[test]
    fn the_stage_name_resolver_bound_is_widened() {
        let bound = STAGE_BOUNDS
            .iter()
            .find(|bound| bound.address == 0x13fa980)
            .expect("the stage-name scan bound must be qualified");
        assert_eq!(bound.table, "stage_id");
        assert_eq!(bound.old_value, 364);
        assert_eq!(bound.new_value, 512);
    }

    #[test]
    fn the_interior_references_are_carried_with_their_bias() {
        let interior: Vec<_> = STAGE_REFERENCES.iter().filter(|r| r.delta != 0).collect();
        assert_eq!(interior.len(), 21, "16 stage_id + 5 stage_place");
        for reference in &interior {
            let table = STAGE_TABLES
                .iter()
                .find(|table| table.name == reference.table)
                .unwrap();
            assert!(
                reference.delta < table.element_size * table.native_length,
                "{:#x} biased {:#x} past the end of {}",
                reference.adrp_at,
                reference.delta,
                reference.table
            );
        }
        let resolver = STAGE_REFERENCES
            .iter()
            .find(|r| r.adrp_at == 0x32b3d6c)
            .expect("the place resolver must be in the census");
        assert_eq!(resolver.table, "stage_place");
        assert_eq!(resolver.delta, 0x30);
    }

    #[test]
    fn the_place_resolver_scan_bound_is_widened() {
        let bound = STAGE_BOUNDS
            .iter()
            .find(|bound| bound.address == 0x32b3d9c)
            .expect("the resolver's scan bound must be qualified");
        assert_eq!(bound.table, "stage_place");
        assert_eq!(bound.old_value, 138);
        assert_eq!(bound.new_value, 192);
    }

    #[test]
    fn every_table_keeps_its_measured_geometry() {
        let by_name = |name| STAGE_TABLES.iter().find(|t| t.name == name).unwrap();
        let id = by_name("stage_id");
        assert_eq!(
            (id.address, id.element_size, id.native_length),
            (0x45497E8, 0x48, 364)
        );
        let place = by_name("stage_place");
        assert_eq!(
            (place.address, place.element_size, place.native_length),
            (0x4548258, 0x28, 138)
        );
        let aux = by_name("stage_place_aux");
        assert_eq!(
            (aux.address, aux.element_size, aux.native_length),
            (0x4545B70, 0x20, 138)
        );
        assert_eq!(
            place.address + place.element_size * place.native_length,
            id.address
        );
    }

    fn is_byte_length(bound: &StageBound, table: &StageTable) -> bool {
        bound.old_value as usize == table.element_size * table.native_length
    }

    const UNROLLED_TAIL: &[usize] = &[0x240ca5c, 0x240cfec, 0x32b21fc, 0x32b244c];

    #[test]
    fn every_bound_widens_to_its_own_tables_expanded_length() {
        for bound in STAGE_BOUNDS {
            let table = STAGE_TABLES.iter().find(|t| t.name == bound.table).unwrap();
            let expected = if is_byte_length(bound, table) {
                table.element_size * table.expanded_length
            } else if UNROLLED_TAIL.contains(&bound.address) {
                table.expanded_length - ((table.expanded_length - table.native_length) % 4)
            } else if bound.old_value as usize == table.native_length {
                table.expanded_length
            } else {
                table.expanded_length - 1
            };
            assert_eq!(
                bound.new_value as usize, expected,
                "bound at {:#x} widens to the wrong ceiling",
                bound.address
            );
            assert!(bound.new_value > bound.old_value);
            let ceiling = if is_byte_length(bound, table) {
                0xFFFF
            } else {
                0xFFF
            };
            assert!(bound.new_value <= ceiling);
        }
    }

    #[test]
    fn widened_opcodes_differ_only_in_the_immediate_field() {
        for bound in STAGE_BOUNDS {
            let table = STAGE_TABLES.iter().find(|t| t.name == bound.table).unwrap();
            let (shift, width) = if is_byte_length(bound, table) {
                (5, 0xFFFF)
            } else {
                (10, 0xFFF)
            };
            let mask = !(width << shift);
            assert_eq!(
                bound.old_opcode & mask,
                bound.new_opcode & mask,
                "bound at {:#x} changed more than its immediate",
                bound.address
            );
            assert_eq!((bound.new_opcode >> shift) & width, bound.new_value);
            assert_eq!((bound.old_opcode >> shift) & width, bound.old_value);
        }
    }

    #[test]
    fn plans_every_bound_when_all_tables_are_relocated() {
        let patches = plan_widening(STAGE_BOUNDS, ALL, reader(&[])).unwrap();
        assert_eq!(patches.len(), STAGE_BOUNDS.len());
        assert_eq!(patches[0].word, STAGE_BOUNDS[0].new_opcode);
    }

    #[test]
    fn refuses_when_a_site_does_not_hold_the_recorded_opcode() {
        let poisoned = STAGE_BOUNDS[5].address;
        let error = plan_widening(STAGE_BOUNDS, ALL, |address| {
            if address == poisoned {
                0xDEAD_BEEF
            } else {
                reader(&[])(address)
            }
        })
        .unwrap_err();
        assert_eq!(
            error,
            BoundError::OpcodeMismatch {
                address: poisoned,
                expected: STAGE_BOUNDS[5].old_opcode,
                actual: 0xDEAD_BEEF,
            }
        );
    }

    #[test]
    fn refuses_bounds_for_a_table_that_was_not_relocated() {
        let error = plan_widening(STAGE_BOUNDS, &["stage_place"], reader(&[])).unwrap_err();
        assert!(matches!(error, BoundError::TableNotRelocated { .. }));
    }
}
