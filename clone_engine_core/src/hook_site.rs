pub const PROLOGUE_WORDS: usize = 4;

const LDR_X17_LITERAL: u32 = 0x5800_0051;
const LDR_LITERAL_MASK: u32 = 0xff00_001f;
const LDR_LITERAL_X17: u32 = 0x5800_0011;
const BR_X17: u32 = 0xd61f_0220;
const B_MASK: u32 = 0xfc00_0000;
const B_OPCODE: u32 = 0x1400_0000;
const BL_OPCODE: u32 = 0x9400_0000;

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
}
