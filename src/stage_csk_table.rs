#![allow(dead_code)]

pub const ROW: usize = 0x48;

pub const VANILLA_ROWS: usize = 364;

pub const FINGERPRINT: &[(usize, u32)] = &[
    (0xA0, 0xF000_0348),
    (0xA4, 0xB940_17E0),
    (0xA8, 0xF942_0901),
    (0xAC, 0xEB00_003F),
    (0xB0, 0x5400_3EE9),
    (0xB4, 0xF000_0348),
    (0xB8, 0x9103_C108),
    (0xBC, 0x5280_0909),
    (0xC0, 0xF941_8D0A),
    (0xC4, 0x9BA9_2809),
];

const AT_LENGTH_ADRP: usize = 0xA0;
const AT_LENGTH_LDR: usize = 0xA8;
const AT_POINTER_ADRP: usize = 0xB4;
const AT_POINTER_ADD: usize = 0xB8;
const AT_ROW_MOV: usize = 0xBC;
const AT_POINTER_LDR: usize = 0xC0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterError {
    Fingerprint {
        offset: usize,
        expected: u32,
        actual: u32,
    },
    NotAVec {
        capacity: usize,
        pointer: usize,
        length: usize,
    },
    RowStride {
        found: usize,
    },
    UnexpectedLength {
        found: usize,
    },
    BadPointer(usize),
    NotGrowth {
        current: usize,
        wanted: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VecFields {
    pub capacity_at: usize,
    pub pointer_at: usize,
    pub length_at: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Growth {
    pub fields: VecFields,
    pub source: usize,
    pub copy_rows: usize,
    pub capacity_rows: usize,
    pub bytes: usize,
}

fn adrp_page(pc: usize, word: u32) -> Option<usize> {
    if word & 0x9F00_0000 != 0x9000_0000 {
        return None;
    }
    let low = ((word >> 29) & 3) as i64;
    let high = ((word >> 5) & 0x7_FFFF) as i64;
    let mut pages = (high << 2) | low;
    if pages & (1 << 20) != 0 {
        pages -= 1 << 21;
    }
    Some(((pc & !0xFFF) as i64 + (pages << 12)) as usize)
}

fn add_immediate(word: u32) -> Option<usize> {
    (word & 0xFF80_0000 == 0x9100_0000).then(|| ((word >> 10) & 0xFFF) as usize)
}

fn ldr_offset(word: u32) -> Option<usize> {
    (word & 0xFFC0_0000 == 0xF940_0000).then(|| (((word >> 10) & 0xFFF) as usize) * 8)
}

fn movz_immediate(word: u32) -> Option<usize> {
    (word & 0xFF80_0000 == 0x5280_0000).then(|| ((word >> 5) & 0xFFFF) as usize)
}

pub fn decode<R>(base: usize, read: &R) -> Result<VecFields, AdapterError>
where
    R: Fn(usize) -> u32,
{
    for &(offset, expected) in FINGERPRINT {
        let actual = read(base + offset);
        if actual != expected {
            return Err(AdapterError::Fingerprint {
                offset,
                expected,
                actual,
            });
        }
    }

    let length_page = adrp_page(base + AT_LENGTH_ADRP, read(base + AT_LENGTH_ADRP)).ok_or(
        AdapterError::NotAVec {
            capacity: 0,
            pointer: 0,
            length: 0,
        },
    )?;
    let length_at = length_page
        + ldr_offset(read(base + AT_LENGTH_LDR)).ok_or(AdapterError::NotAVec {
            capacity: 0,
            pointer: 0,
            length: 0,
        })?;

    let pointer_page = adrp_page(base + AT_POINTER_ADRP, read(base + AT_POINTER_ADRP)).ok_or(
        AdapterError::NotAVec {
            capacity: 0,
            pointer: 0,
            length: 0,
        },
    )?;
    let pointer_at = pointer_page
        + add_immediate(read(base + AT_POINTER_ADD)).ok_or(AdapterError::NotAVec {
            capacity: 0,
            pointer: 0,
            length: 0,
        })?
        + ldr_offset(read(base + AT_POINTER_LDR)).ok_or(AdapterError::NotAVec {
            capacity: 0,
            pointer: 0,
            length: 0,
        })?;

    let row =
        movz_immediate(read(base + AT_ROW_MOV)).ok_or(AdapterError::RowStride { found: 0 })?;
    if row != ROW {
        return Err(AdapterError::RowStride { found: row });
    }

    let capacity_at = pointer_at.wrapping_sub(8);
    if pointer_at + 8 != length_at || capacity_at + 8 != pointer_at {
        return Err(AdapterError::NotAVec {
            capacity: capacity_at,
            pointer: pointer_at,
            length: length_at,
        });
    }

    Ok(VecFields {
        capacity_at,
        pointer_at,
        length_at,
    })
}

pub fn plan(
    fields: VecFields,
    pointer: usize,
    length: usize,
    wanted_rows: usize,
) -> Result<Growth, AdapterError> {
    if pointer == 0 || pointer % 8 != 0 {
        return Err(AdapterError::BadPointer(pointer));
    }
    if length > VANILLA_ROWS {
        return Err(AdapterError::UnexpectedLength { found: length });
    }
    if wanted_rows <= length {
        return Err(AdapterError::NotGrowth {
            current: length,
            wanted: wanted_rows,
        });
    }

    Ok(Growth {
        fields,
        source: pointer,
        copy_rows: length,
        capacity_rows: wanted_rows,
        bytes: wanted_rows * ROW,
    })
}

#[cfg(not(test))]
pub unsafe fn apply(growth: &Growth, buffer: *mut u8) -> *mut u8 {
    if buffer.is_null() {
        skyline::println!("[stagecsk] no storage reserved for the grown stage_id table");
        return core::ptr::null_mut();
    }

    core::ptr::copy_nonoverlapping(growth.source as *const u8, buffer, growth.copy_rows * ROW);

    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    core::ptr::write_volatile(growth.fields.pointer_at as *mut usize, buffer as usize);
    core::ptr::write_volatile(
        growth.fields.capacity_at as *mut usize,
        growth.capacity_rows,
    );
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    core::ptr::write_volatile(growth.fields.length_at as *mut usize, growth.copy_rows);

    skyline::println!(
        "[stagecsk] grew CSK's stage_id table: {} rows copied into {} rows at {:p} \
         (was {:#x}); length stays {} until rows are published",
        growth.copy_rows,
        growth.capacity_rows,
        buffer,
        growth.source,
        growth.copy_rows,
    );
    buffer
}

#[cfg(not(test))]
pub unsafe fn publish_length(fields: VecFields, rows: usize) {
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    core::ptr::write_volatile(fields.length_at as *mut usize, rows);
}

#[cfg(not(test))]
pub unsafe fn resolve_csk() -> Option<usize> {
    let mut address = 0usize;
    let result = skyline::nn::ro::LookupSymbol(
        &mut address as *mut usize,
        b"setup_stage_offseted\0".as_ptr(),
    );
    (result == 0 && address != 0).then_some(address)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeCsk {
        base: usize,
    }

    impl FakeCsk {
        fn read(&self) -> impl Fn(usize) -> u32 + '_ {
            move |address| {
                for &(offset, word) in FINGERPRINT {
                    if address == self.base + offset {
                        return word;
                    }
                }
                0
            }
        }

        fn expected_fields(&self) -> VecFields {
            let length_page = adrp_page(self.base + AT_LENGTH_ADRP, 0xF000_0348).unwrap();
            let length_at = length_page + ldr_offset(0xF942_0901).unwrap();
            let pointer_page = adrp_page(self.base + AT_POINTER_ADRP, 0xF000_0348).unwrap();
            let pointer_at = pointer_page
                + add_immediate(0x9103_C108).unwrap()
                + ldr_offset(0xF941_8D0A).unwrap();
            VecFields {
                capacity_at: pointer_at - 8,
                pointer_at,
                length_at,
            }
        }
    }

    const RECORDED_BASE: usize = 0x000F_0CCC;

    #[test]
    fn decodes_the_addresses_the_tool_reported() {
        let csk = FakeCsk {
            base: RECORDED_BASE,
        };
        let fields = decode(csk.base, &csk.read()).unwrap();
        assert_eq!(fields.length_at, 0x15B410);
        assert_eq!(fields.pointer_at, 0x15B408);
        assert_eq!(fields.capacity_at, 0x15B400);
    }

    #[test]
    fn the_fields_move_with_the_load_base() {
        let shift = 0x1234_5000usize;
        let csk = FakeCsk {
            base: RECORDED_BASE + shift,
        };
        let fields = decode(csk.base, &csk.read()).unwrap();
        assert_eq!(fields, csk.expected_fields());
        assert_eq!(fields.length_at, 0x15B410 + shift);
    }

    #[test]
    fn one_changed_word_stands_the_adapter_down() {
        for &(offset, word) in FINGERPRINT {
            let csk = FakeCsk {
                base: RECORDED_BASE,
            };
            let base = csk.base;
            let inner = csk.read();
            let read = |address: usize| {
                if address == base + offset {
                    word ^ 1
                } else {
                    inner(address)
                }
            };
            match decode(base, &read) {
                Err(AdapterError::Fingerprint { offset: at, .. }) => assert_eq!(at, offset),
                other => panic!("changing +{offset:#x} was accepted: {other:?}"),
            }
        }
    }

    #[test]
    fn absent_csk_reads_as_a_mismatch_not_a_success() {
        let read = |_address: usize| 0u32;
        assert!(matches!(
            decode(RECORDED_BASE, &read),
            Err(AdapterError::Fingerprint { .. })
        ));
    }

    #[test]
    fn plans_a_growth_that_preserves_every_existing_row() {
        let csk = FakeCsk {
            base: RECORDED_BASE,
        };
        let fields = decode(csk.base, &csk.read()).unwrap();
        let growth = plan(fields, 0x1000_0000, VANILLA_ROWS, 512).unwrap();
        assert_eq!(growth.copy_rows, VANILLA_ROWS);
        assert_eq!(growth.capacity_rows, 512);
        assert_eq!(growth.bytes, 512 * ROW);
        assert_eq!(growth.source, 0x1000_0000);
    }

    #[test]
    fn refuses_a_table_somebody_else_already_grew() {
        let csk = FakeCsk {
            base: RECORDED_BASE,
        };
        let fields = decode(csk.base, &csk.read()).unwrap();
        assert_eq!(
            plan(fields, 0x1000_0000, 512, 640),
            Err(AdapterError::UnexpectedLength { found: 512 })
        );
    }

    #[test]
    fn accepts_the_363_rows_csk_actually_copies() {
        let csk = FakeCsk {
            base: RECORDED_BASE,
        };
        let fields = decode(csk.base, &csk.read()).unwrap();
        let growth = plan(fields, 0x1000_0000, 363, 512).unwrap();
        assert_eq!(growth.copy_rows, 363);
        assert_eq!(growth.capacity_rows, 512);
    }

    #[test]
    fn a_shorter_table_still_copies_only_what_is_there() {
        let csk = FakeCsk {
            base: RECORDED_BASE,
        };
        let fields = decode(csk.base, &csk.read()).unwrap();
        for length in [1usize, 100, 363, VANILLA_ROWS] {
            let growth = plan(fields, 0x1000_0000, length, 512).unwrap();
            assert_eq!(growth.copy_rows, length);
            assert!(growth.copy_rows * ROW <= growth.bytes);
        }
    }

    #[test]
    fn refuses_a_null_or_misaligned_pointer() {
        let csk = FakeCsk {
            base: RECORDED_BASE,
        };
        let fields = decode(csk.base, &csk.read()).unwrap();
        assert_eq!(
            plan(fields, 0, VANILLA_ROWS, 512),
            Err(AdapterError::BadPointer(0))
        );
        assert_eq!(
            plan(fields, 0x1000_0004, VANILLA_ROWS, 512),
            Err(AdapterError::BadPointer(0x1000_0004))
        );
    }

    #[test]
    fn refuses_a_shrink_or_a_no_op() {
        let csk = FakeCsk {
            base: RECORDED_BASE,
        };
        let fields = decode(csk.base, &csk.read()).unwrap();
        assert!(matches!(
            plan(fields, 0x1000_0000, VANILLA_ROWS, VANILLA_ROWS),
            Err(AdapterError::NotGrowth { .. })
        ));
        assert!(matches!(
            plan(fields, 0x1000_0000, VANILLA_ROWS, 100),
            Err(AdapterError::NotGrowth { .. })
        ));
    }

    #[test]
    fn the_stride_in_the_multiply_is_the_row_we_build() {
        assert_eq!(movz_immediate(0x5280_0909), Some(ROW));
    }

    #[test]
    fn a_non_vec_layout_is_refused() {
        let base = RECORDED_BASE;
        let read = |address: usize| {
            if address == base + AT_POINTER_LDR {
                0xF941_8509
            } else {
                for &(offset, word) in FINGERPRINT {
                    if address == base + offset {
                        return word;
                    }
                }
                0
            }
        };
        assert!(matches!(
            decode(base, &read),
            Err(AdapterError::Fingerprint { .. }) | Err(AdapterError::NotAVec { .. })
        ));
    }
}
