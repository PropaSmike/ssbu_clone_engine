use super::fighter_param_row::{Op, Reject};

pub const PARAM_THROWN: u64 = 0x000c_982a_d62a;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Holder,
    Held,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hold {
    Any,
    F,
    B,
    Hi,
    Lw,
    Other,
}

impl Hold {
    pub const fn from_selector(selector: i32) -> Hold {
        match selector {
            0 => Hold::F,
            1 => Hold::B,
            2 => Hold::Hi,
            3 => Hold::Lw,
            _ => Hold::Other,
        }
    }

    pub const fn accepts(self, hold: Hold) -> bool {
        matches!(self, Hold::Any) || (self as u8) == (hold as u8)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Component {
    All,
    X,
    Y,
    Z,
}

#[derive(Clone, Copy, Debug)]
pub struct Key {
    pub hash: u64,
    pub name: &'static str,
    pub role: Role,
    pub hold: Hold,
    pub component: Component,
}

const fn key(hash: u64, name: &'static str, role: Role, hold: Hold, component: Component) -> Key {
    Key { hash, name, role, hold, component }
}

pub const KEYS: [Key; 24] = [
    key(0x0006_590a_cad0, "offset", Role::Holder, Hold::Any, Component::All),
    key(0x0008_d8ad_1dd1, "offset_x", Role::Holder, Hold::Any, Component::X),
    key(0x0008_afaa_2d47, "offset_y", Role::Holder, Hold::Any, Component::Y),
    key(0x0008_36a3_7cfd, "offset_z", Role::Holder, Hold::Any, Component::Z),
    key(0x0008_22a2_20b2, "offset_f", Role::Holder, Hold::F, Component::All),
    key(0x000a_cfb9_643a, "offset_f_x", Role::Holder, Hold::F, Component::X),
    key(0x000a_b8be_54ac, "offset_f_y", Role::Holder, Hold::F, Component::Y),
    key(0x000a_21b7_0516, "offset_f_z", Role::Holder, Hold::F, Component::Z),
    key(0x0008_25cf_e4ab, "offset_b", Role::Holder, Hold::B, Component::All),
    key(0x000a_c8b0_cce6, "offset_b_x", Role::Holder, Hold::B, Component::X),
    key(0x000a_bfb7_fc70, "offset_b_y", Role::Holder, Hold::B, Component::Y),
    key(0x000a_26be_adca, "offset_b_z", Role::Holder, Hold::B, Component::Z),
    key(0x0009_5da2_6b7f, "offset_hi", Role::Holder, Hold::Hi, Component::All),
    key(0x000b_50e3_2a21, "offset_hi_x", Role::Holder, Hold::Hi, Component::X),
    key(0x000b_27e4_1ab7, "offset_hi_y", Role::Holder, Hold::Hi, Component::Y),
    key(0x000b_beed_4b0d, "offset_hi_z", Role::Holder, Hold::Hi, Component::Z),
    key(0x0009_c3c1_9318, "offset_lw", Role::Holder, Hold::Lw, Component::All),
    key(0x000b_c939_330c, "offset_lw_x", Role::Holder, Hold::Lw, Component::X),
    key(0x000b_be3e_039a, "offset_lw_y", Role::Holder, Hold::Lw, Component::Y),
    key(0x000b_2737_5220, "offset_lw_z", Role::Holder, Hold::Lw, Component::Z),
    key(0x000b_65fd_2779, "held_offset", Role::Held, Hold::Any, Component::All),
    key(0x000d_79d3_b843, "held_offset_x", Role::Held, Hold::Any, Component::X),
    key(0x000d_0ed4_88d5, "held_offset_y", Role::Held, Hold::Any, Component::Y),
    key(0x000d_97dd_d96f, "held_offset_z", Role::Held, Hold::Any, Component::Z),
];

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ThrownOverride {
    pub key: u8,
    pub op: Op,
}

impl ThrownOverride {
    pub fn key(&self) -> &'static Key {
        &KEYS[usize::from(self.key)]
    }
}

pub fn key_for(hash: u64) -> Option<(u8, &'static Key)> {
    KEYS.iter()
        .position(|key| key.hash == hash)
        .map(|index| (index as u8, &KEYS[index]))
}

pub fn resolve(hash: u64, op: Op) -> Result<(ThrownOverride, &'static Key), Reject> {
    let (index, key) = key_for(hash).ok_or(Reject::UnknownField)?;
    let finite = match op {
        Op::Set(value) | Op::Mul(value) => value.is_finite(),
        Op::SetInt(_) => return Err(Reject::Unsupported),
    };
    if !finite {
        return Err(Reject::NotFinite);
    }
    if matches!(op, Op::Set(_)) && key.component == Component::All {
        return Err(Reject::Unsupported);
    }
    Ok((ThrownOverride { key: index, op }, key))
}

fn apply_one(vector: &mut [f32; 3], component: Component, op: Op) {
    let lanes: &mut [f32] = match component {
        Component::All => &mut vector[..],
        Component::X => &mut vector[0..1],
        Component::Y => &mut vector[1..2],
        Component::Z => &mut vector[2..3],
    };
    for lane in lanes {
        match op {
            Op::Set(value) => *lane = value as f32,
            Op::Mul(value) => *lane *= value as f32,
            Op::SetInt(_) => {}
        }
    }
}

pub fn apply(vector: &mut [f32; 3], overrides: &[ThrownOverride], role: Role, hold: Hold) -> usize {
    let mut applied = 0;
    for item in overrides {
        let key = item.key();
        if key.role != role || !key.hold.accepts(hold) {
            continue;
        }
        apply_one(vector, key.component, item.op);
        applied += 1;
    }
    applied
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::hash40;

    #[test]
    fn every_key_hashes_to_its_name_and_is_unique() {
        assert_eq!(PARAM_THROWN, hash40("param_thrown"));
        for (index, key) in KEYS.iter().enumerate() {
            assert_eq!(key.hash, hash40(key.name), "{}", key.name);
            assert!(KEYS[..index].iter().all(|other| other.hash != key.hash), "{}", key.name);
            assert_eq!(key_for(key.hash).map(|(i, _)| i), Some(index as u8));
        }
        assert!(KEYS.iter().all(|key| key.hold != Hold::Other));
        assert_eq!(KEYS.iter().filter(|key| key.role == Role::Held).count(), 4);
    }

    #[test]
    fn selectors_map_to_the_four_generic_holds() {
        assert_eq!(Hold::from_selector(0), Hold::F);
        assert_eq!(Hold::from_selector(3), Hold::Lw);
        assert_eq!(Hold::from_selector(4), Hold::Other);
        assert!(Hold::Any.accepts(Hold::Other));
        assert!(Hold::F.accepts(Hold::F));
        assert!(!Hold::F.accepts(Hold::B));
        assert!(!Hold::F.accepts(Hold::Other));
    }

    #[test]
    fn resolve_refuses_a_scalar_set_on_a_whole_vector_and_integers() {
        assert_eq!(resolve(hash40("no_such"), Op::Set(1.0)).err(), Some(Reject::UnknownField));
        assert_eq!(resolve(hash40("offset"), Op::Set(1.0)).err(), Some(Reject::Unsupported));
        assert_eq!(resolve(hash40("offset_f"), Op::SetInt(1)).err(), Some(Reject::Unsupported));
        assert_eq!(resolve(hash40("offset_y"), Op::Mul(f64::NAN)).err(), Some(Reject::NotFinite));
        assert!(resolve(hash40("offset"), Op::Mul(1.5)).is_ok());
        assert!(resolve(hash40("held_offset_y"), Op::Set(4.0)).is_ok());
    }

    #[test]
    fn apply_follows_role_hold_and_registration_order() {
        let holder_scale = resolve(hash40("offset"), Op::Mul(2.0)).unwrap().0;
        let holder_f_y = resolve(hash40("offset_f_y"), Op::Set(9.0)).unwrap().0;
        let held_y = resolve(hash40("held_offset_y"), Op::Mul(0.5)).unwrap().0;
        let overrides = [holder_scale, holder_f_y, held_y];

        let mut forward = [1.0, 2.0, 3.0];
        assert_eq!(apply(&mut forward, &overrides, Role::Holder, Hold::F), 2);
        assert_eq!(forward, [2.0, 9.0, 6.0]);
        assert_eq!(apply(&mut forward, &overrides, Role::Held, Hold::F), 1);
        assert_eq!(forward, [2.0, 4.5, 6.0]);

        let mut back = [1.0, 2.0, 3.0];
        assert_eq!(apply(&mut back, &overrides, Role::Holder, Hold::B), 1);
        assert_eq!(back, [2.0, 4.0, 6.0]);

        let mut cargo = [1.0, 2.0, 3.0];
        assert_eq!(apply(&mut cargo, &overrides, Role::Holder, Hold::Other), 1);
        assert_eq!(cargo, [2.0, 4.0, 6.0]);
        assert_eq!(apply(&mut cargo, &[], Role::Held, Hold::Other), 0);
    }
}
