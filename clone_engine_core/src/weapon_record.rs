use crate::hook_site::branch;

pub const SWITCH_FIRST_KIND: i32 = 2;
pub const SWITCH_LAST_KIND: i32 = 0x23d;
pub const KIND_REGISTER: u32 = 8;

const RD_MASK: u32 = 0x1f;
const MOVZ_W_MASK: u32 = 0xffe0_0000;
const MOVZ_W: u32 = 0x5280_0000;
const ORR_W_IMMEDIATE_MASK: u32 = 0xffc0_03e0;
const ORR_W_IMMEDIATE_WZR: u32 = 0x3200_03e0;
const MOV_W_WZR: u32 = 0x2a1f_03e0;

pub fn switch_covers(kind: i32) -> bool {
    (SWITCH_FIRST_KIND..=SWITCH_LAST_KIND).contains(&kind)
}

pub fn switch_index(kind: i32) -> Option<usize> {
    switch_covers(kind).then(|| (kind - SWITCH_FIRST_KIND) as usize)
}

pub fn arm_record_kind(kind: i32, arm: usize, join: usize, words: [u32; 2]) -> Option<i32> {
    if arm == join {
        return Some(kind);
    }
    if arm + 4 != join && words[1] != branch(arm + 4, join) {
        return None;
    }
    mov_immediate_w(words[0], KIND_REGISTER).and_then(|value| i32::try_from(value).ok())
}

pub fn mov_immediate_w(word: u32, register: u32) -> Option<u32> {
    if word & RD_MASK != register {
        return None;
    }
    if word & MOVZ_W_MASK == MOVZ_W {
        return Some((word >> 5) & 0xffff);
    }
    if word & ORR_W_IMMEDIATE_MASK == ORR_W_IMMEDIATE_WZR {
        return bitmask_w((word >> 16) & 0x3f, (word >> 10) & 0x3f);
    }
    if word & !RD_MASK == MOV_W_WZR {
        return Some(0);
    }
    None
}

fn bitmask_w(rotate: u32, ones: u32) -> Option<u32> {
    let pattern = !ones & 0x3f;
    if pattern == 0 {
        return None;
    }
    let length = 31 - pattern.leading_zeros();
    if length == 0 {
        return None;
    }
    let size = 1u32 << length;
    let levels = size - 1;
    let set = ones & levels;
    if set == levels {
        return None;
    }
    let rotate = rotate & levels;
    let mask = (1u64 << size) - 1;
    let element = (1u64 << (set + 1)) - 1;
    let element = ((element >> rotate) | (element << (size - rotate))) & mask;
    let mut value = 0u64;
    let mut shift = 0;
    while shift < 32 {
        value |= element << shift;
        shift += size;
    }
    Some(value as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_every_arm_form_the_switch_uses() {
        assert_eq!(mov_immediate_w(0x5280_43a8, 8), Some(0x21d));
        assert_eq!(mov_immediate_w(0x2a1f_03e8, 8), Some(0));
        assert_eq!(mov_immediate_w(0x3200_13e8, 8), Some(0x1f));
        assert_eq!(mov_immediate_w(0x321f_0fe8, 8), Some(0x1e));
        assert_eq!(mov_immediate_w(0x321b_03e8, 8), Some(0x20));
        assert_eq!(mov_immediate_w(0x321d_03e8, 8), Some(8));
        assert_eq!(mov_immediate_w(0x321d_13e8, 8), Some(0xf8));
        assert_eq!(mov_immediate_w(0x321f_1be8, 8), Some(0xfe));
        assert_eq!(mov_immediate_w(0x321e_17e8, 8), Some(0xfc));
        assert_eq!(mov_immediate_w(0x3218_03e8, 8), Some(0x100));
        assert_eq!(mov_immediate_w(0x3200_1fe8, 8), Some(0xff));
    }

    #[test]
    fn another_register_or_instruction_is_refused() {
        assert_eq!(mov_immediate_w(0x5280_43a0, 8), None);
        assert_eq!(mov_immediate_w(0x52a0_43a8, 8), None);
        assert_eq!(mov_immediate_w(0x1400_0087, 8), None);
        assert_eq!(mov_immediate_w(0x3200_13e9, 8), None);
    }

    #[test]
    fn toon_link_arrow_uses_link_arrow_record() {
        let arm = 0x33b_5d6c;
        let join = 0x33b_5f30;
        assert_eq!(arm_record_kind(195, arm, join, [0x3200_13e8, branch(arm + 4, join)]), Some(31));
    }

    #[test]
    fn the_last_arm_falls_through_to_the_join() {
        assert_eq!(arm_record_kind(9, 0x33b_5f2c, 0x33b_5f30, [0x5280_45a8, 0x5280_1d09]), Some(0x22d));
    }

    #[test]
    fn the_join_arm_keeps_the_kind() {
        assert_eq!(arm_record_kind(303, 0x33b_5f30, 0x33b_5f30, [0, 0]), Some(303));
    }

    #[test]
    fn an_arm_that_does_not_rejoin_is_refused() {
        assert_eq!(arm_record_kind(195, 0x33b_5d6c, 0x33b_5f30, [0x3200_13e8, 0x1400_0001]), None);
    }

    #[test]
    fn kinds_past_the_switch_keep_their_record() {
        assert_eq!(switch_index(1), None);
        assert_eq!(switch_index(2), Some(0));
        assert_eq!(switch_index(0x23d), Some(0x23b));
        assert_eq!(switch_index(619), None);
    }
}
