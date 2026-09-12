use core::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

#[path = "fighter_param_row_table.rs"]
mod fighter_param_row_table;
#[path = "fighter_param_motion_row_table.rs"]
mod fighter_param_motion_row_table;

pub use fighter_param_row_table::{FIELDS, FIELD_COUNT};

pub const ROW_SIZE: usize = 0x628;
pub const MOTION_ROW_SIZE: usize = 0x218;
pub const ORDINAL_TABLE: usize = 0x14f0;
pub const ORDINAL_STRIDE: usize = 0xc;
pub const PAYLOAD_BEGIN: usize = 0x8;
pub const PAYLOAD_END: usize = 0x10;
pub const NEST_LIMIT: usize = 4;
pub const PARAM_MOTION: u64 = 0x000c_ad2e_e25e;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Table {
    FighterParam,
    Motion,
}

impl Table {
    pub const ALL: [Table; 2] = [Table::FighterParam, Table::Motion];

    pub const fn row_size(self) -> usize {
        match self {
            Table::FighterParam => ROW_SIZE,
            Table::Motion => MOTION_ROW_SIZE,
        }
    }

    pub const fn payload_slot(self) -> usize {
        match self {
            Table::FighterParam => 0x0,
            Table::Motion => 0x10,
        }
    }

    pub const fn ordinal_offset(self) -> usize {
        match self {
            Table::FighterParam => ORDINAL_TABLE,
            Table::Motion => ORDINAL_TABLE + 4,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Table::FighterParam => "fighter_param",
            Table::Motion => "fighter_param_motion",
        }
    }

    pub const fn other(self) -> Table {
        match self {
            Table::FighterParam => Table::Motion,
            Table::Motion => Table::FighterParam,
        }
    }

    pub fn fields(self) -> &'static [Field] {
        match self {
            Table::FighterParam => &fighter_param_row_table::FIELDS,
            Table::Motion => &fighter_param_motion_row_table::FIELDS,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FieldType {
    F32,
    U32,
    Bool,
    Hash40,
}

#[derive(Clone, Copy, Debug)]
pub struct Field {
    pub hash: u64,
    pub offset: u16,
    pub kind: FieldType,
    pub name: &'static str,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Op {
    Set(f64),
    Mul(f64),
    SetInt(i32),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct RowOverride {
    pub offset: u16,
    pub kind: FieldType,
    pub op: Op,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reject {
    UnknownField,
    OtherTable(Table),
    Immutable,
    Unsupported,
    NotFinite,
}

pub fn table_for_key(param_type: u64, param_hash: u64) -> Option<(Table, u64)> {
    if param_hash == 0 {
        Some((Table::FighterParam, param_type))
    } else if param_type == PARAM_MOTION {
        Some((Table::Motion, param_hash))
    } else {
        None
    }
}

pub fn field_in(table: Table, hash: u64) -> Option<&'static Field> {
    let fields = table.fields();
    fields
        .binary_search_by_key(&hash, |field| field.hash)
        .ok()
        .map(|index| &fields[index])
}

pub fn field(hash: u64) -> Option<&'static Field> {
    field_in(Table::FighterParam, hash)
}

pub fn resolve_in(table: Table, hash: u64, op: Op) -> Result<(RowOverride, &'static Field), Reject> {
    let Some(field) = field_in(table, hash) else {
        return Err(match field_in(table.other(), hash) {
            Some(_) => Reject::OtherTable(table.other()),
            None => Reject::UnknownField,
        });
    };
    if field.offset == 0 {
        return Err(Reject::Immutable);
    }
    Ok((check(field, op)?, field))
}

pub fn check(field: &Field, op: Op) -> Result<RowOverride, Reject> {
    match (field.kind, op) {
        (_, Op::Set(value)) | (_, Op::Mul(value)) if !value.is_finite() => {
            return Err(Reject::NotFinite)
        }
        (FieldType::Hash40, _) => return Err(Reject::Unsupported),
        (FieldType::F32, _) => {}
        (FieldType::U32 | FieldType::Bool, Op::Mul(_)) => return Err(Reject::Unsupported),
        (FieldType::U32 | FieldType::Bool, _) => {}
    }
    Ok(RowOverride {
        offset: field.offset,
        kind: field.kind,
        op,
    })
}

pub fn width(kind: FieldType) -> usize {
    match kind {
        FieldType::F32 | FieldType::U32 => 4,
        FieldType::Bool => 1,
        FieldType::Hash40 => 8,
    }
}

pub fn read_f32(row: &[u8], offset: u16) -> Option<f32> {
    let at = usize::from(offset);
    let bytes = row.get(at..at + 4)?;
    Some(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

pub fn resolve(hash: u64, op: Op) -> Result<(RowOverride, &'static Field), Reject> {
    resolve_in(Table::FighterParam, hash, op)
}

pub fn apply(row: &mut [u8], item: &RowOverride) {
    let at = usize::from(item.offset);
    match item.kind {
        FieldType::F32 => {
            let Some(bytes) = row.get_mut(at..at + 4) else {
                return;
            };
            let current = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            let next = match item.op {
                Op::Set(value) => value as f32,
                Op::Mul(value) => current * value as f32,
                Op::SetInt(value) => value as f32,
            };
            bytes.copy_from_slice(&next.to_le_bytes());
        }
        FieldType::U32 => {
            let Some(bytes) = row.get_mut(at..at + 4) else {
                return;
            };
            let next = match item.op {
                Op::Set(value) => value.round() as i32,
                Op::SetInt(value) => value,
                Op::Mul(_) => return,
            };
            bytes.copy_from_slice(&next.to_le_bytes());
        }
        FieldType::Bool => {
            let Some(byte) = row.get_mut(at) else {
                return;
            };
            *byte = match item.op {
                Op::Set(value) => u8::from(value != 0.0),
                Op::SetInt(value) => u8::from(value != 0),
                Op::Mul(_) => return,
            };
        }
        FieldType::Hash40 => {}
    }
}

pub fn build(vanilla: &[u8], overrides: &[RowOverride], out: &mut [u8]) {
    let n = vanilla.len().min(out.len());
    out[..n].copy_from_slice(&vanilla[..n]);
    for item in overrides {
        apply(&mut out[..n], item);
    }
}

pub fn row_address_in(table: Table, begin: usize, end: usize, ordinal: i32) -> Option<usize> {
    if begin == 0 || end < begin || begin & 3 != 0 {
        return None;
    }
    let count = (end - begin) / table.row_size();
    let ordinal = usize::try_from(ordinal).ok()?;
    (ordinal < count).then(|| begin + ordinal * table.row_size())
}

pub fn row_address(begin: usize, end: usize, ordinal: i32) -> Option<usize> {
    row_address_in(Table::FighterParam, begin, end, ordinal)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Install {
    Vanilla,
    Clone(i32),
}

pub struct NestStack {
    depth: AtomicUsize,
    clones: [AtomicI32; NEST_LIMIT],
}

impl NestStack {
    pub const fn new() -> Self {
        Self {
            depth: AtomicUsize::new(0),
            clones: [const { AtomicI32::new(-1) }; NEST_LIMIT],
        }
    }

    pub fn depth(&self) -> usize {
        self.depth.load(Ordering::Relaxed)
    }

    pub fn push(&self, clone_kind: i32) -> Option<usize> {
        let depth = self.depth.load(Ordering::Relaxed);
        if depth >= NEST_LIMIT {
            return None;
        }
        self.clones[depth].store(clone_kind, Ordering::Relaxed);
        self.depth.store(depth + 1, Ordering::Relaxed);
        Some(depth + 1)
    }

    pub fn pop(&self) -> Install {
        let depth = self.depth.load(Ordering::Relaxed);
        if depth == 0 {
            return Install::Vanilla;
        }
        let depth = depth - 1;
        self.clones[depth].store(-1, Ordering::Relaxed);
        self.depth.store(depth, Ordering::Relaxed);
        if depth == 0 {
            Install::Vanilla
        } else {
            Install::Clone(self.clones[depth - 1].load(Ordering::Relaxed))
        }
    }

    pub fn reset(&self) {
        self.depth.store(0, Ordering::Relaxed);
        for slot in &self.clones {
            slot.store(-1, Ordering::Relaxed);
        }
    }
}

impl Default for NestStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::hash40;

    #[test]
    fn both_tables_are_sorted_unique_and_inside_their_row() {
        for table in Table::ALL {
            let fields = table.fields();
            for pair in fields.windows(2) {
                assert!(pair[0].hash < pair[1].hash, "{} before {}", pair[0].name, pair[1].name);
            }
            for field in fields.iter() {
                let width = match field.kind {
                    FieldType::F32 | FieldType::U32 => 4,
                    FieldType::Bool => 1,
                    FieldType::Hash40 => 8,
                };
                assert!(usize::from(field.offset) + width <= table.row_size(), "{}", field.name);
                if !field.name.starts_with("unk_") {
                    assert_eq!(field.hash, hash40(field.name), "{}", field.name);
                }
            }
            assert_eq!(field_in(table, hash40("fighter_kind")).unwrap().offset, 0);
        }
        assert_eq!(FIELD_COUNT, 369);
        assert_eq!(Table::Motion.fields().len(), 132);
        assert_eq!(PARAM_MOTION, hash40("param_motion"));
    }

    #[test]
    fn known_offsets_match_the_direct_readers() {
        assert_eq!(field(hash40("scale")).unwrap().offset, 0xa8);
        assert_eq!(field(hash40("jump_initial_y")).unwrap().offset, 0x48);
        assert_eq!(field(hash40("jump_y")).unwrap().offset, 0x4c);
        assert_eq!(field(hash40("mini_jump_y")).unwrap().offset, 0x50);
        assert_eq!(field(hash40("walk_speed_max")).unwrap().offset, 0x10);
        assert_eq!(field(hash40("dash_speed")).unwrap().offset, 0x24);
        assert_eq!(field(hash40("weight")).unwrap().kind, FieldType::F32);
        assert_eq!(field(hash40("jump_count_max")).unwrap().kind, FieldType::U32);
    }

    #[test]
    fn the_binder_layout_correction_holds() {
        assert_eq!(FIELD_COUNT, 369);
        assert_eq!(field(hash40("item_grip_offset_x")).unwrap().offset, 0x444);
        assert_eq!(field(hash40("item_grip_offset_y")).unwrap().offset, 0x448);
        assert_eq!(field(hash40("item_grip_offset_z")).unwrap().offset, 0x44c);
        assert_eq!(field(hash40("throw_item_speed_mul")).unwrap().offset, 0x40c);
        let psycho = field(hash40("psychobreak_offset_y")).unwrap();
        assert_eq!((psycho.offset, psycho.kind), (0x25c, FieldType::F32));
        let hold = field(hash40("attack_s4_hold_frame")).unwrap();
        assert_eq!((hold.offset, hold.kind), (0x108, FieldType::U32));
        assert_eq!(field(hash40("attack_hi4_hold_frame")).unwrap().offset, 0x114);
        assert_eq!(field(hash40("bury_r_rot_angle")).unwrap().offset, 0x19c);
        assert_eq!(field(hash40("passive_wall_x_speed")).unwrap().offset, 0x1b0);
        assert_eq!(field(hash40("shoot_dash_speed_b")).unwrap().offset, 0x424);
        assert_eq!(field(hash40("size")).unwrap().offset, 0x60c);
        assert_eq!(field(hash40("aerial_type")).unwrap().offset, 0x624);
        assert!(field(hash40("fighter_param_table")).is_none());
        assert_eq!(field(hash40("pose_offset_y")).unwrap().offset, 0x548);
        assert_eq!(field(hash40("jostle_weight")).unwrap().offset, 0xc0);
        assert_eq!(field(hash40("camera_range_2nd_center_y")).unwrap().offset, 0x504);
    }

    #[test]
    fn motion_offsets_match_the_direct_readers() {
        let m = Table::Motion;
        assert_eq!(field_in(m, hash40("escape_n_cancel_frame")).unwrap().offset, 0x1c);
        assert_eq!(field_in(m, hash40("escape_f_cancel_frame")).unwrap().offset, 0x30);
        assert_eq!(field_in(m, hash40("escape_b_cancel_frame")).unwrap().offset, 0x44);
        assert_eq!(field_in(m, hash40("escape_air_slide_speed")).unwrap().offset, 0x7c);
        assert_eq!(field_in(m, hash40("escape_air_slide_distance")).unwrap().offset, 0x80);
        assert_eq!(field_in(m, hash40("escape_air_slide_end_speed")).unwrap().offset, 0x8c);
        assert_eq!(field_in(m, hash40("escape_air_stiff_frame")).unwrap().offset, 0x58);
        let flip = field_in(m, hash40("flip")).unwrap();
        assert_eq!((flip.offset, flip.kind), (0xa4, FieldType::Bool));
        let share = field_in(m, hash40("motion_share")).unwrap();
        assert_eq!((share.offset, share.kind), (0x208, FieldType::U32));
        let node = field_in(m, hash40("cliff_hang_node")).unwrap();
        assert_eq!((node.offset, node.kind), (0x1f0, FieldType::Hash40));
        assert_eq!(field_in(m, hash40("cloud_final_target_damage_offset_x")).unwrap().offset, 0x1d8);
        assert!(field_in(m, hash40("scale")).is_none());
        assert!(field(hash40("escape_n_cancel_frame")).is_none());
    }

    #[test]
    fn keys_route_to_their_table() {
        assert_eq!(
            table_for_key(hash40("scale"), 0),
            Some((Table::FighterParam, hash40("scale")))
        );
        assert_eq!(
            table_for_key(hash40("param_motion"), hash40("escape_n_cancel_frame")),
            Some((Table::Motion, hash40("escape_n_cancel_frame")))
        );
        assert_eq!(table_for_key(hash40("param_special_s"), hash40("start_x_spd_mul")), None);
        assert_eq!(
            resolve_in(Table::FighterParam, hash40("escape_air_slide_distance"), Op::Mul(2.0)).unwrap_err(),
            Reject::OtherTable(Table::Motion)
        );
        assert_eq!(
            resolve_in(Table::Motion, hash40("walk_speed_max"), Op::Set(1.0)).unwrap_err(),
            Reject::OtherTable(Table::FighterParam)
        );
        assert_eq!(
            resolve_in(Table::Motion, hash40("cliff_hang_node"), Op::Set(1.0)).unwrap_err(),
            Reject::Unsupported
        );
        assert!(resolve_in(Table::Motion, hash40("flip"), Op::SetInt(0)).is_ok());
        assert!(resolve_in(Table::Motion, hash40("escape_air_slide_speed"), Op::Mul(3.0)).is_ok());
    }

    #[test]
    fn resolve_refuses_what_the_row_cannot_express() {
        assert_eq!(resolve(0x1234, Op::Set(1.0)).unwrap_err(), Reject::UnknownField);
        assert_eq!(
            resolve(hash40("fighter_kind"), Op::Set(1.0)).unwrap_err(),
            Reject::Immutable
        );
        assert_eq!(
            resolve(hash40("damage_fly_smoke_node"), Op::Set(1.0)).unwrap_err(),
            Reject::Unsupported
        );
        assert_eq!(
            resolve(hash40("jump_count_max"), Op::Mul(2.0)).unwrap_err(),
            Reject::Unsupported
        );
        assert_eq!(
            resolve(hash40("scale"), Op::Set(f64::NAN)).unwrap_err(),
            Reject::NotFinite
        );
        assert!(resolve(hash40("scale"), Op::Set(3.0)).is_ok());
        assert!(resolve(hash40("jump_count_max"), Op::SetInt(6)).is_ok());
        assert!(resolve(hash40("jump_count_max"), Op::Set(6.0)).is_ok());
    }

    #[test]
    fn build_copies_vanilla_then_applies_in_order() {
        let mut vanilla = [0u8; ROW_SIZE];
        vanilla[0x0a8..0x0ac].copy_from_slice(&1.0f32.to_le_bytes());
        vanilla[0x010..0x014].copy_from_slice(&1.2f32.to_le_bytes());
        vanilla[0..8].copy_from_slice(&hash40("fighter_kind_donkey").to_le_bytes());
        let (scale, _) = resolve(hash40("scale"), Op::Set(3.0)).unwrap();
        let (walk_mul, _) = resolve(hash40("walk_speed_max"), Op::Mul(2.0)).unwrap();
        let (walk_set, _) = resolve(hash40("walk_speed_max"), Op::Set(4.0)).unwrap();
        let (jumps, _) = resolve(hash40("jump_count_max"), Op::SetInt(6)).unwrap();
        let mut out = [0xffu8; ROW_SIZE];
        build(&vanilla, &[scale, walk_mul, walk_set, walk_mul, jumps], &mut out);
        assert_eq!(&out[0..8], &vanilla[0..8]);
        assert_eq!(f32::from_le_bytes(out[0xa8..0xac].try_into().unwrap()), 3.0);
        assert_eq!(f32::from_le_bytes(out[0x10..0x14].try_into().unwrap()), 8.0);
        let jump_field = field(hash40("jump_count_max")).unwrap();
        let at = usize::from(jump_field.offset);
        assert_eq!(i32::from_le_bytes(out[at..at + 4].try_into().unwrap()), 6);
        assert_eq!(&out[0x14..0xa8], &vanilla[0x14..0xa8]);
    }

    #[test]
    fn a_motion_row_builds_in_its_own_width() {
        let mut vanilla = [0u8; MOTION_ROW_SIZE];
        vanilla[0x7c..0x80].copy_from_slice(&1.0f32.to_le_bytes());
        vanilla[0xa4] = 1;
        let (speed, _) = resolve_in(Table::Motion, hash40("escape_air_slide_speed"), Op::Mul(5.0)).unwrap();
        let (flip, _) = resolve_in(Table::Motion, hash40("flip"), Op::SetInt(0)).unwrap();
        let (cancel, _) = resolve_in(Table::Motion, hash40("escape_n_cancel_frame"), Op::Set(20.0)).unwrap();
        let mut out = [0xffu8; MOTION_ROW_SIZE];
        build(&vanilla, &[speed, flip, cancel], &mut out);
        assert_eq!(f32::from_le_bytes(out[0x7c..0x80].try_into().unwrap()), 5.0);
        assert_eq!(out[0xa4], 0);
        assert_eq!(f32::from_le_bytes(out[0x1c..0x20].try_into().unwrap()), 20.0);
        assert_eq!(&out[0x80..0xa4], &vanilla[0x80..0xa4]);
    }

    #[test]
    fn row_address_bounds_checks_the_ordinal() {
        let begin = 0x1000;
        let end = begin + 3 * ROW_SIZE;
        assert_eq!(row_address(begin, end, 0), Some(begin));
        assert_eq!(row_address(begin, end, 2), Some(begin + 2 * ROW_SIZE));
        assert_eq!(row_address(begin, end, 3), None);
        assert_eq!(row_address(begin, end, -1), None);
        assert_eq!(row_address(0, end, 0), None);
        assert_eq!(row_address(begin, begin, 0), None);
        let end = begin + 94 * MOTION_ROW_SIZE;
        assert_eq!(row_address_in(Table::Motion, begin, end, 93), Some(begin + 93 * MOTION_ROW_SIZE));
        assert_eq!(row_address_in(Table::Motion, begin, end, 94), None);
        assert_eq!(Table::Motion.ordinal_offset(), 0x14f4);
        assert_eq!(Table::Motion.payload_slot(), 0x10);
    }

    #[test]
    fn nesting_restores_the_row_of_the_bracket_below() {
        let stack = NestStack::new();
        assert_eq!(stack.push(120), Some(1));
        assert_eq!(stack.push(121), Some(2));
        assert_eq!(stack.pop(), Install::Clone(120));
        assert_eq!(stack.pop(), Install::Vanilla);
        assert_eq!(stack.pop(), Install::Vanilla);
        for _ in 0..NEST_LIMIT {
            assert!(stack.push(118).is_some());
        }
        assert_eq!(stack.push(118), None);
        stack.reset();
        assert_eq!(stack.depth(), 0);
    }
}
