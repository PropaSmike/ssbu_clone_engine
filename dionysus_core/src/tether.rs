pub const SWITCH_PARAM: &str = "air_lasso_type";
pub const LASSO_GROUP: &str = "air_lasso_data";
pub const CAPSULE_LIST: u64 = 0x1b_60e1_4151;
pub const FIRST_BOX: usize = 1;
pub const MAX_BOXES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lasso {
    Int(i32),
    Float(f32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TetherBox {
    pub p1: [f32; 2],
    pub p2: [f32; 2],
}

impl TetherBox {
    pub const fn new(p1: [f32; 2], p2: [f32; 2]) -> Self {
        Self { p1, p2 }
    }

    pub const fn words(&self) -> [f32; 4] {
        [self.p1[0], self.p1[1], self.p2[0], self.p2[1]]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Capsule {
    pub node: &'static str,
    pub p1: [f32; 3],
    pub p2: [f32; 3],
    pub size: f32,
}

impl Capsule {
    pub const fn new(node: &'static str, p1: [f32; 3], p2: [f32; 3], size: f32) -> Self {
        Self { node, p1, p2, size }
    }
}

#[derive(Debug)]
pub struct TetherDonor {
    pub name: &'static str,
    pub kind: i32,
    pub lasso: &'static [(&'static str, Lasso)],
    pub boxes: &'static [TetherBox],
    pub capsules: &'static [Capsule],
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tether {
    pub from: Option<String>,
    pub boxes: Vec<TetherBox>,
}

impl Tether {
    pub fn donor(&self) -> Option<&'static TetherDonor> {
        self.from.as_deref().and_then(donor)
    }

    pub fn boxes(&self) -> Vec<TetherBox> {
        if !self.boxes.is_empty() {
            return self.boxes.clone();
        }
        self.donor().map(|donor| donor.boxes.to_vec()).unwrap_or_default()
    }

    pub fn capsules(&self) -> &'static [Capsule] {
        self.donor().map(|donor| donor.capsules).unwrap_or(&[])
    }

    pub fn params(&self) -> Vec<(String, Lasso)> {
        let mut out = vec![(SWITCH_PARAM.to_string(), Lasso::Int(1))];
        if let Some(donor) = self.donor() {
            out.extend(
                donor
                    .lasso
                    .iter()
                    .map(|(key, value)| (format!("{LASSO_GROUP}.{key}"), *value)),
            );
        }
        out
    }

    pub fn check(&self) -> Result<(), String> {
        if let Some(name) = &self.from {
            check(name)?;
        }
        if self.boxes.len() > MAX_BOXES {
            return Err(format!("at most {MAX_BOXES} tether boxes, found {}", self.boxes.len()));
        }
        Ok(())
    }
}

pub fn donor(name: &str) -> Option<&'static TetherDonor> {
    DONORS.iter().find(|donor| donor.name == name)
}

pub fn names() -> String {
    DONORS.iter().map(|donor| donor.name).collect::<Vec<_>>().join(", ")
}

pub fn check(name: &str) -> Result<&'static TetherDonor, String> {
    donor(name).ok_or_else(|| format!("{name:?} has no tether; use one of {}", names()))
}

pub static DONORS: [TetherDonor; 7] = [
    TetherDonor { name: "samus", kind: 3, lasso: &[("frame", Lasso::Int(120)), ("rewind_frame", Lasso::Float(9.0)), ("rewind_brake_x", Lasso::Int(0)), ("shoot_end_frame", Lasso::Int(32)), ("body_angle_intp", Lasso::Int(30)), ("hang_up_accel_rate", Lasso::Float(4.0)), ("failure_wait_frame", Lasso::Int(16)), ("hang_down_accel_rate", Lasso::Float(2.0)), ("rewind_body_angle_intp", Lasso::Int(5))], boxes: &[TetherBox::new([57.0, 44.0], [10.0, -34.0]), TetherBox::new([15.0, 15.0], [-3.0, -5.0])], capsules: &[Capsule::new("bust", [0.0, 0.0, -0.4], [-2.0, 0.0, -0.4], 2.0), Capsule::new("head", [0.0, 0.5, 0.0], [0.0, -0.5, 0.0], 1.8)] },
    TetherDonor { name: "samusd", kind: 4, lasso: &[("frame", Lasso::Int(120)), ("rewind_frame", Lasso::Float(9.0)), ("rewind_brake_x", Lasso::Int(0)), ("shoot_end_frame", Lasso::Int(32)), ("body_angle_intp", Lasso::Int(30)), ("hang_up_accel_rate", Lasso::Float(4.0)), ("failure_wait_frame", Lasso::Int(16)), ("hang_down_accel_rate", Lasso::Float(2.0)), ("rewind_body_angle_intp", Lasso::Int(5))], boxes: &[TetherBox::new([57.0, 44.0], [10.0, -34.0]), TetherBox::new([15.0, 15.0], [-3.0, -5.0])], capsules: &[Capsule::new("bust", [0.0, 0.0, -0.4], [-2.0, 0.0, -0.4], 2.0), Capsule::new("head", [0.0, 0.5, 0.0], [0.0, -0.5, 0.0], 1.8)] },
    TetherDonor { name: "luigi", kind: 9, lasso: &[("frame", Lasso::Int(3000))], boxes: &[], capsules: &[Capsule::new("waist", [0.3, 0.0, -0.4], [1.9, 0.0, -0.4], 2.0), Capsule::new("head", [1.5, -0.1, 0.0], [1.5, -0.9, 0.0], 2.0)] },
    TetherDonor { name: "younglink", kind: 23, lasso: &[("frame", Lasso::Int(120)), ("rewind_frame", Lasso::Float(9.0)), ("rewind_brake_x", Lasso::Int(0)), ("shoot_end_frame", Lasso::Int(53)), ("body_angle_intp", Lasso::Int(30)), ("hang_up_accel_rate", Lasso::Float(4.0)), ("failure_wait_frame", Lasso::Int(16)), ("hang_down_accel_rate", Lasso::Float(2.0)), ("rewind_body_angle_intp", Lasso::Int(5))], boxes: &[TetherBox::new([60.0, 48.0], [5.0, -34.0]), TetherBox::new([13.0, 12.0], [-3.0, -8.0])], capsules: &[Capsule::new("waist", [0.3, 0.0, -0.4], [1.9, 0.0, -0.4], 2.2), Capsule::new("head", [1.5, -0.1, 0.0], [1.5, -0.9, 0.0], 2.2)] },
    TetherDonor { name: "szerosuit", kind: 32, lasso: &[("frame", Lasso::Int(120)), ("rewind_frame", Lasso::Float(9.0)), ("rewind_brake_x", Lasso::Int(0)), ("shoot_end_frame", Lasso::Int(30)), ("body_angle_intp", Lasso::Int(20)), ("hang_up_accel_rate", Lasso::Float(4.0)), ("failure_wait_frame", Lasso::Int(16)), ("hang_down_accel_rate", Lasso::Float(2.0)), ("rewind_body_angle_intp", Lasso::Int(5))], boxes: &[TetherBox::new([60.0, 40.0], [10.0, -10.0]), TetherBox::new([15.0, 15.0], [-3.0, -5.0])], capsules: &[Capsule::new("waist", [0.3, 0.0, -0.4], [1.9, 0.0, -0.4], 2.0), Capsule::new("head", [1.5, -0.1, 0.0], [1.5, -0.9, 0.0], 2.0)] },
    TetherDonor { name: "lucas", kind: 40, lasso: &[("frame", Lasso::Int(120)), ("rewind_frame", Lasso::Float(10.0)), ("rewind_brake_x", Lasso::Int(0)), ("shoot_end_frame", Lasso::Int(40)), ("body_angle_intp", Lasso::Int(30)), ("hang_up_accel_rate", Lasso::Float(4.0)), ("failure_wait_frame", Lasso::Int(5)), ("hang_down_accel_rate", Lasso::Float(2.0)), ("rewind_body_angle_intp", Lasso::Int(5))], boxes: &[TetherBox::new([60.0, 48.0], [5.0, -34.0]), TetherBox::new([14.0, 10.0], [-3.0, -10.0]), TetherBox::new([15.0, 22.0], [-9.0, 3.0])], capsules: &[Capsule::new("waist", [-2.0, 0.2, 2.1], [2.0, -0.7, 2.1], 3.0), Capsule::new("head", [100.0, 100.0, 100.0], [100.0, 100.0, 100.0], 3.0)] },
    TetherDonor { name: "toonlink", kind: 46, lasso: &[("frame", Lasso::Int(120)), ("rewind_frame", Lasso::Float(9.0)), ("rewind_brake_x", Lasso::Int(0)), ("shoot_end_frame", Lasso::Int(53)), ("body_angle_intp", Lasso::Int(30)), ("hang_up_accel_rate", Lasso::Float(4.0)), ("failure_wait_frame", Lasso::Int(16)), ("hang_down_accel_rate", Lasso::Float(2.0)), ("rewind_body_angle_intp", Lasso::Int(5))], boxes: &[TetherBox::new([60.0, 48.0], [5.0, -34.0]), TetherBox::new([13.0, 12.0], [-3.0, -8.0])], capsules: &[Capsule::new("waist", [0.3, 0.0, -0.4], [1.9, 0.0, -0.4], 2.0), Capsule::new("head", [1.5, -0.1, 0.0], [1.5, -0.9, 0.0], 2.0)] },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_donor_but_luigi_brings_ledge_boxes_and_all_nine_lasso_values() {
        for donor in DONORS.iter() {
            if donor.name == "luigi" {
                assert!(donor.boxes.is_empty());
                assert_eq!(donor.lasso.len(), 1);
                continue;
            }
            assert!(donor.boxes.len() >= 2, "{}", donor.name);
            assert_eq!(donor.capsules.len(), 2, "{}", donor.name);
            assert_eq!(donor.lasso.len(), 9, "{}", donor.name);
        }
    }

    #[test]
    fn a_donor_switches_the_air_tether_on_and_names_its_values_in_the_group() {
        let tether = Tether { from: Some("lucas".into()), boxes: Vec::new() };
        let params = tether.params();
        assert_eq!(params[0], ("air_lasso_type".to_string(), Lasso::Int(1)));
        assert!(params.contains(&("air_lasso_data.rewind_frame".to_string(), Lasso::Float(10.0))));
        assert_eq!(tether.boxes()[0].words(), [60.0, 48.0, 5.0, -34.0]);
    }

    #[test]
    fn own_boxes_replace_the_donors_and_no_donor_still_switches_it_on() {
        let own = TetherBox::new([50.0, 40.0], [0.0, -30.0]);
        let tether = Tether { from: Some("samus".into()), boxes: vec![own] };
        assert_eq!(tether.boxes(), vec![own]);
        let bare = Tether::default();
        assert_eq!(bare.params().len(), 1);
        assert!(bare.boxes().is_empty());
        assert!(check("mario").unwrap_err().contains("lucas"));
    }
}
