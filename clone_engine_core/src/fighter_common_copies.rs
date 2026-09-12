use super::fighter_param_row::{apply, check, width, Field, FieldType, Op, Reject, RowOverride};

#[path = "fighter_common_copies_table.rs"]
mod fighter_common_copies_table;

pub use fighter_common_copies_table::{FIELDS, FIELD_COUNT};

pub const COMMON: u64 = 0x0006_e5ec_7051;
pub const PARAM_OBJECT_SIZE: usize = 0x1b60;
pub const ACCESSOR_PARAM_OBJECT: usize = 0x2758;
pub const PARAM_OBJECT_POWER_UP_CLONE: usize = 0x1b10;
pub const POWER_UP: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Copy {
    pub name: &'static str,
    pub file: &'static str,
    pub base: usize,
    pub size: usize,
}

impl Copy {
    pub const fn end(&self) -> usize {
        self.base + self.size
    }

    pub const fn contains(&self, offset: usize) -> bool {
        offset >= self.base && offset < self.end()
    }
}

pub const COPIES: [Copy; 6] = [
    Copy { name: "common", file: "common.prc", base: 0x50, size: 0xc58 },
    Copy { name: "item", file: "item.prc", base: 0xca8, size: 0x3d8 },
    Copy { name: "etc", file: "etc.prc", base: 0x1080, size: 0x2c8 },
    Copy { name: "power_up", file: "power_up.prc", base: 0x1348, size: 0x2f0 },
    Copy { name: "effect", file: "effect.prc", base: 0x1638, size: 0x430 },
    Copy { name: "sound", file: "sound.prc", base: 0x1a68, size: 0x70 },
];

pub fn field(hash: u64) -> Option<&'static Field> {
    FIELDS
        .binary_search_by_key(&hash, |field| field.hash)
        .ok()
        .map(|index| &FIELDS[index])
}

pub fn copy_of(offset: usize) -> Option<&'static Copy> {
    COPIES.iter().find(|copy| copy.contains(offset))
}

pub fn resolve(hash: u64, op: Op) -> Result<(RowOverride, &'static Field, &'static Copy), Reject> {
    let field = field(hash).ok_or(Reject::UnknownField)?;
    let copy = copy_of(usize::from(field.offset)).ok_or(Reject::UnknownField)?;
    Ok((check(field, op)?, field, copy))
}

pub fn apply_from(object: &mut [u8], vanilla: &[u8], item: &RowOverride) {
    let at = usize::from(item.offset);
    let end = at + width(item.kind);
    if end > object.len() || end > vanilla.len() {
        return;
    }
    object[at..end].copy_from_slice(&vanilla[at..end]);
    apply(object, item);
}

pub fn apply_range(
    object: &mut [u8],
    vanilla: &[u8],
    overrides: &[RowOverride],
    base: usize,
    end: usize,
) -> usize {
    let mut applied = 0;
    for item in overrides {
        let at = usize::from(item.offset);
        if at >= base && at < end {
            apply_from(object, vanilla, item);
            applied += 1;
        }
    }
    applied
}

pub fn apply_detached(
    object: &mut [u8],
    vanilla: &[u8],
    overrides: &[RowOverride],
    copy: &Copy,
) -> usize {
    let mut applied = 0;
    for item in overrides {
        let at = usize::from(item.offset);
        if !copy.contains(at) {
            continue;
        }
        let shifted = RowOverride {
            offset: (at - copy.base) as u16,
            kind: item.kind,
            op: item.op,
        };
        apply_from(object, vanilla, &shifted);
        applied += 1;
    }
    applied
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::hash40;

    #[test]
    fn the_union_is_sorted_unique_and_inside_one_copy_each() {
        for pair in FIELDS.windows(2) {
            assert!(pair[0].hash < pair[1].hash, "{} before {}", pair[0].name, pair[1].name);
        }
        for field in FIELDS.iter() {
            let at = usize::from(field.offset);
            let copy = copy_of(at).unwrap_or_else(|| panic!("{} at {at:#x} is in no copy", field.name));
            assert!(at + width(field.kind) <= copy.end(), "{}", field.name);
            if !field.name.starts_with("unk_") {
                assert_eq!(field.hash, hash40(field.name), "{}", field.name);
            }
        }
        assert_eq!(FIELD_COUNT, 1238);
        assert_eq!(COMMON, hash40("common"));
        for pair in COPIES.windows(2) {
            assert!(pair[0].end() <= pair[1].base, "{} overlaps {}", pair[0].name, pair[1].name);
        }
        assert!(COPIES[5].end() <= PARAM_OBJECT_SIZE);
    }

    #[test]
    fn known_offsets_match_the_constructor_defaults_and_the_lookup_chain() {
        let at = |name: &str| usize::from(field(hash40(name)).unwrap().offset);
        assert_eq!(at("precede"), 0x50 + 0x8);
        assert_eq!(at("walk_stick_x"), 0x50 + 0x44);
        assert_eq!(at("shield_max"), 0x50 + 0x160);
        assert_eq!(at("dash_stick_x"), 0x50 + 0x74);
        assert_eq!(at("shield_size"), 0x50 + 0xbc8);
        assert_eq!(copy_of(at("metal_frame")).unwrap().name, "item");
        assert_eq!(copy_of(at("finish_camera_frame")).unwrap().name, "etc");
        assert_eq!(copy_of(at("power_attack_1")).unwrap().name, "power_up");
        assert_eq!(copy_of(at("bio_disp_prob")).unwrap().name, "effect");
        assert_eq!(copy_of(at("damage_fly_se_dist_m")).unwrap().name, "sound");
        assert_eq!(copy_of(at("furafura_frame")).unwrap().name, "common");
        assert_eq!(field(hash40("precede")).unwrap().kind, FieldType::U32);
    }

    #[test]
    fn resolve_refuses_what_the_row_refuses() {
        assert_eq!(resolve(hash40("no_such_field"), Op::Set(1.0)).err(), Some(Reject::UnknownField));
        assert_eq!(resolve(hash40("precede"), Op::Mul(2.0)).err(), Some(Reject::Unsupported));
        assert_eq!(resolve(hash40("shield_max"), Op::Set(f64::NAN)).err(), Some(Reject::NotFinite));
        let (item, field, copy) = resolve(hash40("shield_max"), Op::Set(10.0)).unwrap();
        assert_eq!(field.name, "shield_max");
        assert_eq!(copy.name, "common");
        assert_eq!(item.offset, 0x50 + 0x160);
    }

    #[test]
    fn apply_from_recomputes_from_the_snapshot_not_the_live_value() {
        let mut vanilla = vec![0u8; PARAM_OBJECT_SIZE];
        let (item, ..) = resolve(hash40("shield_size"), Op::Mul(3.0)).unwrap();
        let at = usize::from(item.offset);
        vanilla[at..at + 4].copy_from_slice(&1.0f32.to_le_bytes());
        let mut object = vanilla.clone();
        apply_from(&mut object, &vanilla, &item);
        apply_from(&mut object, &vanilla, &item);
        assert_eq!(f32::from_le_bytes(object[at..at + 4].try_into().unwrap()), 3.0);
        let common = &COPIES[0];
        assert_eq!(apply_range(&mut object, &vanilla, &[item], common.base, common.end()), 1);
        assert_eq!(apply_range(&mut object, &vanilla, &[item], COPIES[1].base, COPIES[1].end()), 0);
        assert_eq!(f32::from_le_bytes(object[at..at + 4].try_into().unwrap()), 3.0);
    }

    #[test]
    fn apply_detached_shifts_into_a_standalone_power_up_object() {
        let power_up = &COPIES[POWER_UP];
        assert_eq!(power_up.name, "power_up");
        let (item, field, copy) = resolve(hash40("escape_air_slide_distance_mul"), Op::Mul(3.0)).unwrap();
        assert_eq!(copy.name, "power_up");
        let at = usize::from(field.offset) - power_up.base;
        let mut vanilla = vec![0u8; power_up.size];
        vanilla[at..at + 4].copy_from_slice(&1.0f32.to_le_bytes());
        let mut object = vanilla.clone();
        assert_eq!(apply_detached(&mut object, &vanilla, &[item], power_up), 1);
        assert_eq!(apply_detached(&mut object, &vanilla, &[item], power_up), 1);
        assert_eq!(f32::from_le_bytes(object[at..at + 4].try_into().unwrap()), 3.0);
        let (common_item, ..) = resolve(hash40("shield_max"), Op::Set(10.0)).unwrap();
        assert_eq!(apply_detached(&mut object, &vanilla, &[common_item], power_up), 0);
        assert!(PARAM_OBJECT_POWER_UP_CLONE >= COPIES[5].end());
    }
}
