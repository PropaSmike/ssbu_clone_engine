pub const ROW: usize = 0x48;

pub const VANILLA_ROWS: usize = 364;

pub const SCAN_WORDS: usize = 0x200;

const BOUND_REACH: usize = 8;
const CONDITION_HS: u32 = 0b0010;
const CONDITION_LS: u32 = 0b1001;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterError {
    PatternNotFound,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Value {
    Unknown,
    Address(usize),
    Constant(usize),
    LoadedFrom(usize),
}

#[derive(Debug, Clone, Copy)]
struct Compare {
    at: usize,
    left: usize,
    right: usize,
    left_value: Value,
    right_value: Value,
}

#[derive(Debug, Clone, Copy)]
struct Bound {
    at: usize,
    condition: u32,
    compare: Compare,
}

#[derive(Debug)]
struct Machine {
    registers: [Value; 32],
    last_compare: Option<Compare>,
    bound: Option<Bound>,
}

fn register(word: u32, shift: u32) -> usize {
    ((word >> shift) & 0x1F) as usize
}

fn adrp_page(pc: usize, word: u32) -> usize {
    let low = ((word >> 29) & 3) as i64;
    let high = ((word >> 5) & 0x7_FFFF) as i64;
    let mut pages = (high << 2) | low;
    if pages & (1 << 20) != 0 {
        pages -= 1 << 21;
    }
    ((pc & !0xFFF) as i64 + (pages << 12)) as usize
}

fn twelve_bits(word: u32) -> usize {
    ((word >> 10) & 0xFFF) as usize
}

impl Machine {
    fn new() -> Self {
        Self {
            registers: [Value::Unknown; 32],
            last_compare: None,
            bound: None,
        }
    }

    fn address_in(&self, index: usize) -> Option<usize> {
        match self.registers[index] {
            Value::Address(address) => Some(address),
            _ => None,
        }
    }

    fn step(&mut self, index: usize, pc: usize, word: u32) {
        let target = register(word, 0);
        let source = register(word, 5);

        if word & 0x9F00_0000 == 0x9000_0000 {
            self.registers[target] = Value::Address(adrp_page(pc, word));
            return;
        }
        if word & 0xFFC0_0000 == 0x9100_0000 {
            self.registers[target] = self
                .address_in(source)
                .map_or(Value::Unknown, |address| Value::Address(address + twelve_bits(word)));
            return;
        }
        if word & 0xFFC0_0000 == 0xF940_0000 {
            self.registers[target] = self
                .address_in(source)
                .map_or(Value::Unknown, |address| {
                    Value::LoadedFrom(address + twelve_bits(word) * 8)
                });
            return;
        }
        if word & 0x7FE0_0000 == 0x5280_0000 {
            self.registers[target] = Value::Constant(((word >> 5) & 0xFFFF) as usize);
            return;
        }
        if word & 0xFFE0_FC1F == 0xEB00_001F {
            let right = register(word, 16);
            self.last_compare = Some(Compare {
                at: index,
                left: source,
                right,
                left_value: self.registers[source],
                right_value: self.registers[right],
            });
            return;
        }
        if word & 0xFF00_0010 == 0x5400_0000 {
            if let Some(compare) = self.last_compare.filter(|compare| compare.at + 1 == index) {
                self.bound = Some(Bound {
                    at: index,
                    condition: word & 0xF,
                    compare,
                });
            }
            return;
        }
        if word & 0xFC00_0000 == 0x9400_0000 || word & 0xFFFF_FC1F == 0xD63F_0000 {
            for value in self.registers[..19].iter_mut() {
                *value = Value::Unknown;
            }
            self.registers[30] = Value::Unknown;
            return;
        }
        if word & 0x7C00_0000 == 0x1400_0000
            || word & 0x7C00_0000 == 0x3400_0000
            || word & 0xFE00_0000 == 0xD600_0000
        {
            return;
        }
        if word & 0xFFC0_0000 == 0xD500_0000 {
            if word & 0x0020_0000 != 0 {
                self.registers[target] = Value::Unknown;
            }
            return;
        }
        if word & 0x0A00_0000 == 0x0800_0000 {
            self.load_store(word);
            return;
        }
        self.registers[target] = Value::Unknown;
    }

    fn load_store(&mut self, word: u32) {
        let vector = word & 0x0400_0000 != 0;
        let (load, pair, writeback) = match (word >> 27) & 7 {
            0b111 => (
                (word >> 22) & 3 != 0,
                false,
                word & 0x0100_0000 == 0 && word & 0x0020_0000 == 0 && word & 0x0400 != 0,
            ),
            0b101 => (word & 0x0040_0000 != 0, true, word & 0x0080_0000 != 0),
            0b011 => (true, false, false),
            0b001 if !vector => (word & 0x0040_0000 != 0, word & 0x0020_0000 != 0, false),
            _ => (true, true, true),
        };
        if load && !vector {
            self.registers[register(word, 0)] = Value::Unknown;
            if pair {
                self.registers[register(word, 10)] = Value::Unknown;
            }
        }
        if writeback {
            self.registers[register(word, 5)] = Value::Unknown;
        }
    }

    fn row_access(&self, index: usize, word: u32) -> Option<Result<VecFields, AdapterError>> {
        if word & 0xFFE0_8000 != 0x9BA0_0000 {
            return None;
        }
        let row_index = register(word, 5);
        let stride = self.registers[register(word, 16)];
        let pointer = self.registers[register(word, 10)];

        let bound = self
            .bound
            .filter(|bound| index.saturating_sub(bound.at) <= BOUND_REACH)?;
        let compare = bound.compare;
        let length = match bound.condition {
            CONDITION_LS if compare.right == row_index => compare.left_value,
            CONDITION_HS if compare.left == row_index => compare.right_value,
            _ => return None,
        };
        let (Value::LoadedFrom(length_at), Value::LoadedFrom(pointer_at)) = (length, pointer) else {
            return None;
        };

        match stride {
            Value::Constant(ROW) => {}
            Value::Constant(found) => return Some(Err(AdapterError::RowStride { found })),
            _ => return Some(Err(AdapterError::RowStride { found: 0 })),
        }

        let capacity_at = pointer_at.wrapping_sub(8);
        if pointer_at.wrapping_add(8) != length_at {
            return Some(Err(AdapterError::NotAVec {
                capacity: capacity_at,
                pointer: pointer_at,
                length: length_at,
            }));
        }
        Some(Ok(VecFields {
            capacity_at,
            pointer_at,
            length_at,
        }))
    }
}

pub fn decode<R>(base: usize, read: &R) -> Result<VecFields, AdapterError>
where
    R: Fn(usize) -> u32,
{
    let mut machine = Machine::new();
    let mut first_error = None;
    for index in 0..SCAN_WORDS {
        let pc = base + index * 4;
        let word = read(pc);
        match machine.row_access(index, word) {
            Some(Ok(fields)) => return Ok(fields),
            Some(Err(error)) => {
                first_error.get_or_insert(error);
            }
            None => {}
        }
        machine.step(index, pc, word);
    }
    Err(first_error.unwrap_or(AdapterError::PatternNotFound))
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

#[cfg(test)]
mod tests {
    use super::*;

    const BUILD_13_0_4_BASE: usize = 0x000F_0CCC;
    const BUILD_13_0_4: [u32; 52] = [
        0xD103_83FF, 0x9100_23E8, 0xD503_201F, 0x109E_BFC9, 0xA903_27E8, 0xB940_0808,
        0xF940_000A, 0xA90D_4FF4, 0xAA00_03F4, 0x9100_53E9, 0xB900_17E8, 0xD503_201F,
        0x109E_A648, 0x9000_01C0, 0x9125_D400, 0x9100_C3E1, 0xA908_7BFD, 0xA909_6FFC,
        0xA90A_67FA, 0xA90B_5FF8, 0xA90C_57F6, 0xF900_07EA, 0xA904_23E9, 0x97FD_3F16,
        0xF940_07F3, 0xF100_0A7F, 0x5400_4788, 0xD503_201F, 0x1035_9FF9, 0x8B13_0328,
        0x3941_6508, 0x7100_051F, 0x5400_21E1, 0x3941_1328, 0x3600_21A8, 0xF000_0348,
        0x9111_4108, 0xC8DF_FD08, 0xF100_091F, 0x5400_3EC1, 0xF000_0348, 0xB940_17E0,
        0xF942_0901, 0xEB00_003F, 0x5400_3EE9, 0xF000_0348, 0x9103_C108, 0x5280_0909,
        0xF941_8D0A, 0x9BA9_2809, 0xB940_0536, 0xF940_0935,
    ];

    const BUILD_13_0_5_BASE: usize = 0x0010_2970;
    const BUILD_13_0_5: [u32; 52] = [
        0xD103_43FF, 0xA909_67FA, 0x9100_43E8, 0xD503_201F, 0x1095_E389, 0xF940_000A,
        0xB940_081A, 0xA90C_4FF4, 0xAA00_03F3, 0xD000_0120, 0x9125_2800, 0xA902_27E8,
        0x9100_73E8, 0xD503_201F, 0x1095_C9E9, 0x9100_83E1, 0xA907_7BFD, 0xA908_6FFC,
        0xA90A_5FF8, 0xA90B_57F6, 0xF900_0BEA, 0xB900_1FFA, 0xA903_27E8, 0x97FC_F677,
        0xF940_0BF4, 0xF100_0A9F, 0x5400_5E88, 0xD503_201F, 0x102C_BADC, 0xB000_02DB,
        0x910D_837B, 0x8B14_0388, 0xB000_02D9, 0x3941_6508, 0x7100_051F, 0x5400_2161,
        0x3941_6388, 0x3600_2128, 0xC8DF_FF68, 0xF100_091F, 0x5400_4481, 0xB940_1FE0,
        0xF941_9321, 0xEB00_003F, 0x5400_44A9, 0xB000_02C8, 0x910A_0108, 0x5280_0909,
        0xF940_4D0A, 0x9BA9_2809, 0xB940_0536, 0xF940_0935,
    ];

    const FIELDS_13_0_4: VecFields = VecFields {
        capacity_at: 0x15_B400,
        pointer_at: 0x15_B408,
        length_at: 0x15_B410,
    };

    const FIELDS_13_0_5: VecFields = VecFields {
        capacity_at: 0x15_B310,
        pointer_at: 0x15_B318,
        length_at: 0x15_B320,
    };

    fn reader(base: usize, words: &[u32]) -> impl Fn(usize) -> u32 + '_ {
        move |address| {
            if address < base || (address - base) % 4 != 0 {
                return 0;
            }
            words.get((address - base) / 4).copied().unwrap_or(0)
        }
    }

    fn edited(words: &[u32; 52], at: usize, word: u32) -> [u32; 52] {
        let mut copy = *words;
        copy[at] = word;
        copy
    }

    fn shifted(fields: VecFields, shift: usize) -> VecFields {
        VecFields {
            capacity_at: fields.capacity_at + shift,
            pointer_at: fields.pointer_at + shift,
            length_at: fields.length_at + shift,
        }
    }

    #[test]
    fn decodes_the_13_0_4_build() {
        let read = reader(BUILD_13_0_4_BASE, &BUILD_13_0_4);
        assert_eq!(decode(BUILD_13_0_4_BASE, &read), Ok(FIELDS_13_0_4));
    }

    #[test]
    fn decodes_the_13_0_5_build() {
        let read = reader(BUILD_13_0_5_BASE, &BUILD_13_0_5);
        assert_eq!(decode(BUILD_13_0_5_BASE, &read), Ok(FIELDS_13_0_5));
    }

    #[test]
    fn the_fields_move_with_the_load_base() {
        let shift = 0x1234_5000usize;
        for (base, words, fields) in [
            (BUILD_13_0_4_BASE, &BUILD_13_0_4, FIELDS_13_0_4),
            (BUILD_13_0_5_BASE, &BUILD_13_0_5, FIELDS_13_0_5),
        ] {
            let read = reader(base + shift, words);
            assert_eq!(decode(base + shift, &read), Ok(shifted(fields, shift)));
        }
    }

    #[test]
    fn absent_csk_is_not_a_success() {
        let read = |_address: usize| 0u32;
        assert_eq!(
            decode(BUILD_13_0_5_BASE, &read),
            Err(AdapterError::PatternNotFound)
        );
    }

    #[test]
    fn a_different_row_stride_is_refused() {
        let words = edited(&BUILD_13_0_5, 47, 0x5280_0A09);
        let read = reader(BUILD_13_0_5_BASE, &words);
        assert_eq!(
            decode(BUILD_13_0_5_BASE, &read),
            Err(AdapterError::RowStride { found: 0x50 })
        );
    }

    #[test]
    fn fields_that_are_not_one_vec_are_refused() {
        let words = edited(&BUILD_13_0_4, 48, 0xF941_850A);
        let read = reader(BUILD_13_0_4_BASE, &words);
        assert!(matches!(
            decode(BUILD_13_0_4_BASE, &read),
            Err(AdapterError::NotAVec { .. })
        ));
    }

    #[test]
    fn a_clobbered_length_base_is_not_trusted() {
        let words = edited(&BUILD_13_0_5, 33, 0xAA1F_03F9);
        let read = reader(BUILD_13_0_5_BASE, &words);
        assert_eq!(
            decode(BUILD_13_0_5_BASE, &read),
            Err(AdapterError::PatternNotFound)
        );
    }

    #[test]
    fn a_call_between_the_page_and_the_load_is_not_trusted() {
        let words = edited(&BUILD_13_0_4, 41, 0x97FF_FFFF);
        let read = reader(BUILD_13_0_4_BASE, &words);
        assert_eq!(
            decode(BUILD_13_0_4_BASE, &read),
            Err(AdapterError::PatternNotFound)
        );
    }

    #[test]
    fn an_unchecked_index_is_not_a_bounded_access() {
        let words = edited(&BUILD_13_0_5, 43, 0xEB01_003F);
        let read = reader(BUILD_13_0_5_BASE, &words);
        assert_eq!(
            decode(BUILD_13_0_5_BASE, &read),
            Err(AdapterError::PatternNotFound)
        );
    }

    #[test]
    fn plans_a_growth_that_preserves_every_existing_row() {
        let growth = plan(FIELDS_13_0_5, 0x1000_0000, VANILLA_ROWS, 512).unwrap();
        assert_eq!(growth.copy_rows, VANILLA_ROWS);
        assert_eq!(growth.capacity_rows, 512);
        assert_eq!(growth.bytes, 512 * ROW);
        assert_eq!(growth.source, 0x1000_0000);
    }

    #[test]
    fn refuses_a_table_somebody_else_already_grew() {
        assert_eq!(
            plan(FIELDS_13_0_5, 0x1000_0000, 512, 640),
            Err(AdapterError::UnexpectedLength { found: 512 })
        );
    }

    #[test]
    fn accepts_the_363_rows_csk_actually_copies() {
        let growth = plan(FIELDS_13_0_4, 0x1000_0000, 363, 512).unwrap();
        assert_eq!(growth.copy_rows, 363);
        assert_eq!(growth.capacity_rows, 512);
    }

    #[test]
    fn refuses_a_null_or_misaligned_pointer() {
        assert_eq!(
            plan(FIELDS_13_0_4, 0, VANILLA_ROWS, 512),
            Err(AdapterError::BadPointer(0))
        );
        assert_eq!(
            plan(FIELDS_13_0_4, 0x1000_0004, VANILLA_ROWS, 512),
            Err(AdapterError::BadPointer(0x1000_0004))
        );
    }

    #[test]
    fn refuses_a_shrink_or_a_no_op() {
        assert!(matches!(
            plan(FIELDS_13_0_4, 0x1000_0000, VANILLA_ROWS, VANILLA_ROWS),
            Err(AdapterError::NotGrowth { .. })
        ));
        assert!(matches!(
            plan(FIELDS_13_0_4, 0x1000_0000, VANILLA_ROWS, 100),
            Err(AdapterError::NotGrowth { .. })
        ));
    }
}
