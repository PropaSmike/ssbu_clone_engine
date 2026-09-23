pub const PROLOGUE_WORDS: usize = 4;
pub const RELOCATED_WORDS: usize = 5;

const LDR_X17_LITERAL: u32 = 0x5800_0051;
const LDR_LITERAL_MASK: u32 = 0xff00_001f;
const LDR_LITERAL_X17: u32 = 0x5800_0011;
const BR_X17: u32 = 0xd61f_0220;
const B_MASK: u32 = 0xfc00_0000;
const B_OPCODE: u32 = 0x1400_0000;
const BL_OPCODE: u32 = 0x9400_0000;

const STUB_LDR_MASK: u32 = 0xff00_001e;
const STUB_LDR: u32 = 0x5800_0010;
const BR_X16: u32 = 0xd61f_0200;

pub fn is_inline_trampoline(words: &[u32; PROLOGUE_WORDS]) -> bool {
    (words[0] == LDR_X17_LITERAL || words[0] & LDR_LITERAL_MASK == LDR_LITERAL_X17)
        && words[1] == BR_X17
}

pub fn entry_looks_untouched(words: &[u32; PROLOGUE_WORDS]) -> bool {
    if words.iter().any(|word| *word == 0) {
        return false;
    }
    if is_inline_trampoline(words) {
        return false;
    }
    let first = words[0] & B_MASK;
    first != B_OPCODE && first != BL_OPCODE
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hazard {
    BackwardLiteral { index: usize, word: u32 },
    BackwardBranch { index: usize, word: u32 },
    PlantedStub { index: usize },
}

impl Hazard {
    pub fn index(&self) -> usize {
        match self {
            Hazard::BackwardLiteral { index, .. }
            | Hazard::BackwardBranch { index, .. }
            | Hazard::PlantedStub { index } => *index,
        }
    }

    pub fn reason(&self) -> &'static str {
        match self {
            Hazard::BackwardLiteral { .. } => "backward literal load",
            Hazard::BackwardBranch { .. } => "backward conditional branch",
            Hazard::PlantedStub { .. } => "another plugin's stub",
        }
    }
}

fn imm19(word: u32) -> i32 {
    let raw = ((word >> 5) & 0x7ffff) as i32;
    if raw & 0x40000 != 0 {
        raw - 0x80000
    } else {
        raw
    }
}

fn imm14(word: u32) -> i32 {
    let raw = ((word >> 5) & 0x3fff) as i32;
    if raw & 0x2000 != 0 {
        raw - 0x4000
    } else {
        raw
    }
}

pub fn is_literal_load(word: u32) -> bool {
    word & 0xbf00_0000 == 0x1800_0000
        || word & 0x3f00_0000 == 0x1c00_0000
        || word & 0xff00_0000 == 0x9800_0000
        || word & 0xff00_0000 == 0xd800_0000
}

pub fn is_compare_branch(word: u32) -> bool {
    word & 0xff00_0010 == 0x5400_0000
        || word & 0x7f00_0000 == 0x3400_0000
        || word & 0x7f00_0000 == 0x3500_0000
}

pub fn is_test_branch(word: u32) -> bool {
    word & 0x7f00_0000 == 0x3600_0000 || word & 0x7f00_0000 == 0x3700_0000
}

fn planted_stub_at(words: &[u32], index: usize) -> bool {
    if words[index] & STUB_LDR_MASK != STUB_LDR {
        return false;
    }
    match words.get(index + 1) {
        Some(next) => *next == BR_X17 || *next == BR_X16,
        None => false,
    }
}

pub fn relocation_hazard(words: &[u32]) -> Option<Hazard> {
    for (index, word) in words.iter().enumerate() {
        let word = *word;
        if planted_stub_at(words, index) {
            return Some(Hazard::PlantedStub { index });
        }
        if is_literal_load(word) && imm19(word) < 0 {
            return Some(Hazard::BackwardLiteral { index, word });
        }
        if is_compare_branch(word) && imm19(word) < 0 {
            return Some(Hazard::BackwardBranch { index, word });
        }
        if is_test_branch(word) && imm14(word) < 0 {
            return Some(Hazard::BackwardBranch { index, word });
        }
    }
    None
}

pub fn windows_overlap(first: usize, second: usize) -> bool {
    let span = RELOCATED_WORDS * 4;
    first < second + span && second < first + span
}

#[cfg(test)]
mod tests {
    use super::*;

    const GANONDORF_ON_LINK_EVENT: [u32; 4] = [0xd101_83ff, 0xa903_57f6, 0xa904_4ff4, 0xa905_7bfd];

    #[test]
    fn a_plain_prologue_is_untouched() {
        assert!(entry_looks_untouched(&GANONDORF_ON_LINK_EVENT));
    }

    #[test]
    fn a_planted_trampoline_is_seen() {
        assert!(!entry_looks_untouched(&[0x5800_0051, 0xd61f_0220, 0x1234_5678, 0x9abc_def0]));
        assert!(is_inline_trampoline(&[0x5800_0091, 0xd61f_0220, 1, 1]));
    }

    #[test]
    fn a_branch_at_the_entry_is_not_a_prologue() {
        assert!(!entry_looks_untouched(&[0x1400_0010, 1, 1, 1]));
        assert!(!entry_looks_untouched(&[0x9400_0010, 1, 1, 1]));
    }

    #[test]
    fn zero_words_never_pass() {
        assert!(!entry_looks_untouched(&[0, 1, 1, 1]));
    }

    #[test]
    fn a_vanilla_thunk_relocates_cleanly() {
        let thunk = [0xf941_c000, 0xf940_0008, 0xf941_1103, 0xd61f_0060, 0xf940_0008];
        assert_eq!(relocation_hazard(&thunk), None);
    }

    #[test]
    fn a_forward_literal_load_is_fine() {
        assert_eq!(relocation_hazard(&[0x5800_0051, 0xd61f_0220, 1, 1, 1][..1]), None);
    }

    #[test]
    fn the_word_that_crashed_training_is_caught() {
        let words = [0x0000_0038, 0x58d8_c960, 0xd503_201f, 0xf940_0008, 0xd61f_0060];
        assert_eq!(
            relocation_hazard(&words),
            Some(Hazard::BackwardLiteral { index: 1, word: 0x58d8_c960 })
        );
    }

    #[test]
    fn a_planted_stub_is_refused_before_its_pointer_word() {
        let words = [0xd503_201f, 0x5800_0051, 0xd61f_0220, 0x20df_d6b8, 0x0000_0038];
        assert_eq!(relocation_hazard(&words), Some(Hazard::PlantedStub { index: 1 }));
    }

    #[test]
    fn a_backward_conditional_branch_is_caught() {
        let words = [0x5400_0fe1, 0x54ff_ffe1, 1, 1, 1];
        assert_eq!(
            relocation_hazard(&words),
            Some(Hazard::BackwardBranch { index: 1, word: 0x54ff_ffe1 })
        );
    }

    #[test]
    fn a_backward_test_branch_is_caught() {
        assert_eq!(
            relocation_hazard(&[0x3704_ffe8]),
            Some(Hazard::BackwardBranch { index: 0, word: 0x3704_ffe8 })
        );
        assert_eq!(relocation_hazard(&[0x3700_0068]), None);
    }

    #[test]
    fn twenty_byte_windows_that_touch_overlap() {
        assert!(windows_overlap(0x4e53a0, 0x4e53b0));
        assert!(windows_overlap(0x48abb8, 0x48abc4));
        assert!(!windows_overlap(0x48abb0, 0x48abc4));
        assert!(!windows_overlap(0x4e53a0, 0x4e53e0));
        assert!(!windows_overlap(0x4e53a0, 0x4e53b4));
    }
}
