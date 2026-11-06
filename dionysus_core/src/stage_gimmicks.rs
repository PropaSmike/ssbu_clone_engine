pub const LADDER_WIDTH: f32 = 10.0;
pub const MAX_GIMMICKS: usize = 64;
pub const MAX_COORDINATE: f32 = 100_000.0;
pub const AREA_LAYER: u32 = 0x5000_0000;
pub const AREA_OWNER_NONE: u32 = u32::MAX;
pub const FULL_TURN: f32 = 360.0;
pub const CLASS_TURN: f32 = 90.0;
pub const TOLERANCE_RATE: f32 = 0.5;
pub const TOLERANCE_MIN: f32 = 0.01;
pub const BARREL_KIND_NATURAL: u16 = 11;
pub const BARREL_KIND_NATURAL_AUTO: u16 = 13;
pub const BARREL_KIND_PATH: u16 = 12;
pub const BARREL_KIND_PATH_AUTO: u16 = 14;
pub const BARREL_EVENT_SET_PATH: u32 = 6;
pub const BARREL_EVENT_END_PATH: u32 = 7;
pub const BARREL_EVENT_WORDS: usize = 0x30;
pub const MAX_PATH_POINTS: usize = 1024;
pub const BARREL_HOLD_FOREVER: f32 = 1.0e9;
pub const BARREL_ATTACK_KIND: u32 = 18;
pub const BARREL_FLAGS: u32 = 1;
pub const MAX_FIRE_DELAY: u32 = 3600;
pub const MAX_MODEL_NAME: usize = 64;
pub const SPRING_AREA_WIDTH: f32 = 10.0;
pub const SPRING_AREA_HEIGHT: f32 = 6.0;
pub const SPRING_SPEED: f32 = 4.0;
pub const MAX_SPRING_SPEED: f32 = 100.0;
pub const MAX_LAUNCH_FRAME: f32 = 600.0;
pub const DOOR_KIND_GROUND: u16 = 0x14;
pub const DOOR_KIND_AIR: u16 = 0x15;
pub const DOOR_INPUT_STICK_UP: u16 = 1;
pub const DOOR_EVENT_IN: u32 = 0x16;
pub const DOOR_AREA_WIDTH: f32 = 15.0;
pub const DOOR_AREA_HEIGHT: f32 = 20.0;
pub const MAX_PLAYER: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GimmickKind {
    Ladder,
    Barrel,
    Spring,
    Door,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GimmickError {
    Missing(&'static str),
    OutOfRange(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ladder {
    pub x: f32,
    pub y: f32,
    pub height: f32,
    pub width: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LadderDraft {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub height: Option<f32>,
    pub width: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Barrel {
    pub model: Option<String>,
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub rotation_max: f32,
    pub rotation_speed: f32,
    pub fire_rotation: f32,
    pub full_rotate: bool,
    pub always_rotate: bool,
    pub auto_fire: bool,
    pub fire_delay: u32,
    pub speed: f32,
    pub decrease: f32,
    pub gravity: f32,
    pub fall_speed: f32,
    pub width: f32,
    pub height: f32,
    pub path: Vec<[f32; 2]>,
    pub path_speed: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BarrelDraft {
    pub model: Option<String>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub rotation: Option<f32>,
    pub rotation_max: Option<f32>,
    pub rotation_speed: Option<f32>,
    pub fire_rotation: Option<f32>,
    pub full_rotate: Option<bool>,
    pub always_rotate: Option<bool>,
    pub auto_fire: Option<bool>,
    pub fire_delay: Option<f32>,
    pub speed: Option<f32>,
    pub decrease: Option<f32>,
    pub gravity: Option<f32>,
    pub fall_speed: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub path: Option<Vec<f32>>,
    pub path_speed: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spring {
    pub model: Option<String>,
    pub joint: Option<String>,
    pub launch_frame: Option<f32>,
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub speed: f32,
    pub width: f32,
    pub height: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpringDraft {
    pub model: Option<String>,
    pub joint: Option<String>,
    pub launch_frame: Option<f32>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub rotation: Option<f32>,
    pub speed: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub offset_x: Option<f32>,
    pub offset_y: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Door {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub air: bool,
    pub player: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DoorDraft {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub offset_x: Option<f32>,
    pub offset_y: Option<f32>,
    pub air: Option<bool>,
    pub player: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C, align(16))]
pub struct BarrelEvent(pub [u32; BARREL_EVENT_WORDS]);

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C, align(16))]
pub struct AreaParam {
    pub offset: [f32; 4],
    pub range: [f32; 4],
    pub reserved: u64,
    pub layer: u32,
    pub owner: u32,
    pub flags: u32,
    pub kind: u16,
    pub pad: u16,
    pub layer_again: u32,
    pub tail: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C, align(16))]
pub struct Vector4(pub [f32; 4]);

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C, align(16))]
pub struct BarrelData {
    pub speed: f32,
    pub decrease: f32,
    pub gravity: f32,
    pub fall_speed: f32,
    pub fire_delay: u32,
    pub kind: u16,
    pub pad: u16,
    pub flags: u32,
    pub pad_again: u32,
    pub area: [f32; 4],
    pub position: [f32; 4],
    pub hold_frames: f32,
    pub attack_kind: u32,
    pub reserved: [u32; 6],
    pub power: f32,
    pub vector: f32,
    pub reaction_effect: f32,
    pub reaction_fix: f32,
    pub reaction_add: f32,
    pub slip: f32,
    pub hit_stop_frame: f32,
    pub hit_stop_delay: f32,
    pub rate: f32,
    pub tail: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarrelMotion {
    pub angle: f32,
    pub speed: f32,
}

pub fn gimmick_header(line: &str) -> Option<GimmickKind> {
    let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    match compact.as_str() {
        "[[ladder]]" | "[[stage.ladder]]" => Some(GimmickKind::Ladder),
        "[[barrel]]" | "[[stage.barrel]]" => Some(GimmickKind::Barrel),
        "[[spring]]" | "[[stage.spring]]" => Some(GimmickKind::Spring),
        "[[door]]" | "[[stage.door]]" => Some(GimmickKind::Door),
        _ => None,
    }
}

fn coordinate(name: &'static str, value: f32) -> Result<f32, GimmickError> {
    if !value.is_finite() || value.abs() > MAX_COORDINATE {
        return Err(GimmickError::OutOfRange(name));
    }
    Ok(value)
}

fn size(name: &'static str, value: f32) -> Result<f32, GimmickError> {
    if !value.is_finite() || value <= 0.0 || value > MAX_COORDINATE {
        return Err(GimmickError::OutOfRange(name));
    }
    Ok(value)
}

fn angle(name: &'static str, value: f32) -> Result<f32, GimmickError> {
    if !value.is_finite() || !(0.0..=FULL_TURN).contains(&value) {
        return Err(GimmickError::OutOfRange(name));
    }
    Ok(value)
}

fn tilt(name: &'static str, value: f32) -> Result<f32, GimmickError> {
    if !value.is_finite() || value.abs() > FULL_TURN {
        return Err(GimmickError::OutOfRange(name));
    }
    Ok(value)
}

fn model_name(model: &Option<String>) -> Result<(), GimmickError> {
    if let Some(model) = model {
        let valid = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_';
        if model.is_empty() || model.len() > MAX_MODEL_NAME || !model.chars().all(valid) {
            return Err(GimmickError::OutOfRange("model"));
        }
    }
    Ok(())
}

impl LadderDraft {
    pub fn set(&mut self, key: &str, value: f32) -> bool {
        let slot = match key {
            "x" => &mut self.x,
            "y" => &mut self.y,
            "height" => &mut self.height,
            "width" => &mut self.width,
            _ => return false,
        };
        *slot = Some(value);
        true
    }

    pub fn finish(self) -> Result<Ladder, GimmickError> {
        Ok(Ladder {
            x: coordinate("x", self.x.ok_or(GimmickError::Missing("x"))?)?,
            y: coordinate("y", self.y.ok_or(GimmickError::Missing("y"))?)?,
            height: size("height", self.height.ok_or(GimmickError::Missing("height"))?)?,
            width: size("width", self.width.unwrap_or(LADDER_WIDTH))?,
        })
    }
}

impl Ladder {
    pub fn area(&self) -> AreaParam {
        AreaParam {
            offset: [0.0; 4],
            range: [self.width, self.height, 0.0, 0.0],
            reserved: 0,
            layer: AREA_LAYER,
            owner: AREA_OWNER_NONE,
            flags: 0,
            kind: 0,
            pad: 0,
            layer_again: AREA_LAYER,
            tail: 0,
        }
    }

    pub fn bottom(&self) -> Vector4 {
        Vector4([self.x, self.y, 0.0, 0.0])
    }

    pub fn top(&self) -> Vector4 {
        Vector4([self.x, self.y + self.height, 0.0, 0.0])
    }
}

impl BarrelDraft {
    pub fn set_number(&mut self, key: &str, value: f32) -> bool {
        let slot = match key {
            "x" => &mut self.x,
            "y" => &mut self.y,
            "rotation" => &mut self.rotation,
            "rotation_max" => &mut self.rotation_max,
            "rotation_speed" => &mut self.rotation_speed,
            "fire_rotation" => &mut self.fire_rotation,
            "fire_delay" => &mut self.fire_delay,
            "speed" => &mut self.speed,
            "decrease" => &mut self.decrease,
            "gravity" => &mut self.gravity,
            "fall_speed" => &mut self.fall_speed,
            "width" => &mut self.width,
            "height" => &mut self.height,
            "path_speed" => &mut self.path_speed,
            _ => return false,
        };
        *slot = Some(value);
        true
    }

    pub fn set_numbers(&mut self, key: &str, values: &[f32]) -> bool {
        if key != "path" {
            return false;
        }
        self.path = Some(values.to_vec());
        true
    }

    pub fn set_text(&mut self, key: &str, value: &str) -> bool {
        if key != "model" {
            return false;
        }
        self.model = Some(value.to_ascii_lowercase());
        true
    }

    pub fn set_flag(&mut self, key: &str, value: bool) -> bool {
        let slot = match key {
            "full_rotate" => &mut self.full_rotate,
            "always_rotate" => &mut self.always_rotate,
            "auto_fire" => &mut self.auto_fire,
            _ => return false,
        };
        *slot = Some(value);
        true
    }

    pub fn finish(self) -> Result<Barrel, GimmickError> {
        let rotation = angle("rotation", self.rotation.unwrap_or(0.0))?;
        let fire_delay = self.fire_delay.unwrap_or(0.0);
        if !fire_delay.is_finite() || fire_delay < 0.0 || fire_delay > MAX_FIRE_DELAY as f32 {
            return Err(GimmickError::OutOfRange("fire_delay"));
        }
        let rotation_speed = self.rotation_speed.unwrap_or(0.0);
        if !rotation_speed.is_finite() || rotation_speed.abs() > FULL_TURN {
            return Err(GimmickError::OutOfRange("rotation_speed"));
        }
        let shot = |name: &'static str, value: Option<f32>, default: f32| {
            let value = value.unwrap_or(default);
            if !value.is_finite() || value.abs() > MAX_COORDINATE {
                return Err(GimmickError::OutOfRange(name));
            }
            Ok(value)
        };
        model_name(&self.model)?;
        let path = self.path.unwrap_or_default();
        if path.len() % 2 != 0 || path.len() / 2 > MAX_PATH_POINTS || path.len() == 2 {
            return Err(GimmickError::OutOfRange("path"));
        }
        let path: Vec<[f32; 2]> = path
            .chunks(2)
            .map(|pair| Ok([coordinate("path", pair[0])?, coordinate("path", pair[1])?]))
            .collect::<Result<_, GimmickError>>()?;
        let path_speed = self.path_speed.unwrap_or(1.0);
        if !path_speed.is_finite() || path_speed <= 0.0 || path_speed > MAX_PATH_POINTS as f32 {
            return Err(GimmickError::OutOfRange("path_speed"));
        }
        Ok(Barrel {
            path,
            path_speed,
            model: self.model,
            x: coordinate("x", self.x.ok_or(GimmickError::Missing("x"))?)?,
            y: coordinate("y", self.y.ok_or(GimmickError::Missing("y"))?)?,
            rotation,
            rotation_max: angle("rotation_max", self.rotation_max.unwrap_or(rotation))?,
            rotation_speed,
            fire_rotation: angle("fire_rotation", self.fire_rotation.unwrap_or(rotation))?,
            full_rotate: self.full_rotate.unwrap_or(false),
            always_rotate: self.always_rotate.unwrap_or(false),
            auto_fire: self.auto_fire.unwrap_or(false),
            fire_delay: fire_delay as u32,
            speed: shot("speed", self.speed, 5.0)?,
            decrease: shot("decrease", self.decrease, 0.075)?,
            gravity: shot("gravity", self.gravity, 0.025)?,
            fall_speed: shot("fall_speed", self.fall_speed, 0.25)?,
            width: size("width", self.width.unwrap_or(20.0))?,
            height: size("height", self.height.unwrap_or(15.0))?,
        })
    }
}

impl SpringDraft {
    pub fn set_number(&mut self, key: &str, value: f32) -> bool {
        let slot = match key {
            "x" => &mut self.x,
            "y" => &mut self.y,
            "rotation" => &mut self.rotation,
            "speed" => &mut self.speed,
            "width" => &mut self.width,
            "height" => &mut self.height,
            "offset_x" => &mut self.offset_x,
            "offset_y" => &mut self.offset_y,
            "launch_frame" => &mut self.launch_frame,
            _ => return false,
        };
        *slot = Some(value);
        true
    }

    pub fn set_text(&mut self, key: &str, value: &str) -> bool {
        match key {
            "model" => self.model = Some(value.to_ascii_lowercase()),
            "joint" => self.joint = Some(value.to_string()),
            _ => return false,
        }
        true
    }

    pub fn finish(self) -> Result<Spring, GimmickError> {
        model_name(&self.model)?;
        if let Some(joint) = &self.joint {
            let valid = |c: char| c.is_ascii_alphanumeric() || c == '_';
            if joint.is_empty() || joint.len() > MAX_MODEL_NAME || !joint.chars().all(valid) {
                return Err(GimmickError::OutOfRange("joint"));
            }
        }
        if let Some(frame) = self.launch_frame {
            if !frame.is_finite() || frame < 0.0 || frame > MAX_LAUNCH_FRAME {
                return Err(GimmickError::OutOfRange("launch_frame"));
            }
        }
        let speed = self.speed.unwrap_or(SPRING_SPEED);
        if !speed.is_finite() || speed <= 0.0 || speed > MAX_SPRING_SPEED {
            return Err(GimmickError::OutOfRange("speed"));
        }
        Ok(Spring {
            model: self.model,
            joint: self.joint,
            launch_frame: self.launch_frame,
            x: coordinate("x", self.x.ok_or(GimmickError::Missing("x"))?)?,
            y: coordinate("y", self.y.ok_or(GimmickError::Missing("y"))?)?,
            rotation: tilt("rotation", self.rotation.unwrap_or(0.0))?,
            speed,
            width: size("width", self.width.unwrap_or(SPRING_AREA_WIDTH))?,
            height: size("height", self.height.unwrap_or(SPRING_AREA_HEIGHT))?,
            offset_x: coordinate("offset_x", self.offset_x.unwrap_or(0.0))?,
            offset_y: coordinate("offset_y", self.offset_y.unwrap_or(0.0))?,
        })
    }
}

impl Spring {
    pub fn area(&self) -> AreaParam {
        AreaParam {
            offset: [self.x + self.offset_x, self.y + self.offset_y, 0.0, 0.0],
            range: [self.width, self.height, 0.0, 0.0],
            reserved: 0,
            layer: AREA_LAYER,
            owner: AREA_OWNER_NONE,
            flags: 0,
            kind: 0,
            pad: 0,
            layer_again: AREA_LAYER,
            tail: 0,
        }
    }

    pub fn position(&self) -> Vector4 {
        Vector4([self.x, self.y, 0.0, 0.0])
    }

    pub fn launches_at_once(&self) -> bool {
        self.model.is_none() || self.launch_frame.is_none()
    }
}

impl DoorDraft {
    pub fn set_number(&mut self, key: &str, value: f32) -> bool {
        let slot = match key {
            "x" => &mut self.x,
            "y" => &mut self.y,
            "width" => &mut self.width,
            "height" => &mut self.height,
            "offset_x" => &mut self.offset_x,
            "offset_y" => &mut self.offset_y,
            "player" => &mut self.player,
            _ => return false,
        };
        *slot = Some(value);
        true
    }

    pub fn set_flag(&mut self, key: &str, value: bool) -> bool {
        match key {
            "air" => self.air = Some(value),
            _ => return false,
        }
        true
    }

    pub fn finish(self) -> Result<Door, GimmickError> {
        let player = match self.player {
            Some(player) if player.fract() != 0.0 || !(1.0..=MAX_PLAYER).contains(&player) => {
                return Err(GimmickError::OutOfRange("player"))
            }
            Some(player) => Some(player as u32),
            None => None,
        };
        Ok(Door {
            x: coordinate("x", self.x.ok_or(GimmickError::Missing("x"))?)?,
            y: coordinate("y", self.y.ok_or(GimmickError::Missing("y"))?)?,
            width: size("width", self.width.unwrap_or(DOOR_AREA_WIDTH))?,
            height: size("height", self.height.unwrap_or(DOOR_AREA_HEIGHT))?,
            offset_x: coordinate("offset_x", self.offset_x.unwrap_or(0.0))?,
            offset_y: coordinate("offset_y", self.offset_y.unwrap_or(0.0))?,
            air: self.air.unwrap_or(false),
            player,
        })
    }
}

impl Door {
    pub fn kind(&self) -> u16 {
        if self.air {
            DOOR_KIND_AIR
        } else {
            DOOR_KIND_GROUND
        }
    }

    pub fn host(&self) -> Ladder {
        Ladder {
            x: self.x + self.offset_x,
            y: self.y + self.offset_y - self.height / 2.0,
            height: self.height,
            width: self.width,
        }
    }

    pub fn entry(&self) -> u32 {
        self.player.map_or(AREA_OWNER_NONE, |player| player - 1)
    }

    pub fn center(&self) -> [f32; 2] {
        [self.x, self.y]
    }
}

impl Barrel {
    pub fn kind(&self) -> u16 {
        match (self.follows_path(), self.auto_fire) {
            (false, false) => BARREL_KIND_NATURAL,
            (false, true) => BARREL_KIND_NATURAL_AUTO,
            (true, false) => BARREL_KIND_PATH,
            (true, true) => BARREL_KIND_PATH_AUTO,
        }
    }

    pub fn follows_path(&self) -> bool {
        !self.path.is_empty()
    }

    pub fn path_point(&self, frame: f32) -> Option<[f32; 2]> {
        let last = self.path.len().checked_sub(1)?;
        if !(frame >= 0.0) || frame >= last as f32 {
            return None;
        }
        let index = frame.floor() as usize;
        let t = frame - index as f32;
        let [x0, y0] = self.path[index];
        let [x1, y1] = self.path[index + 1];
        Some([x0 + (x1 - x0) * t, y0 + (y1 - y0) * t])
    }

    pub fn event(kind: u32, target: u32, sender: u32, at: [f32; 2]) -> BarrelEvent {
        let mut words = [0u32; BARREL_EVENT_WORDS];
        words[3] = target;
        words[4] = kind;
        words[5] = sender;
        words[8] = at[0].to_bits();
        words[9] = at[1].to_bits();
        words[0x90 / 4] = u32::MAX;
        words[0xa4 / 4] = 25.0f32.to_bits();
        words[0xa8 / 4] = 1.0f32.to_bits();
        words[0xb4 / 4] = 1;
        BarrelEvent(words)
    }

    pub fn data(&self) -> BarrelData {
        BarrelData {
            speed: if self.follows_path() { 0.0 } else { self.speed },
            decrease: self.decrease,
            gravity: self.gravity,
            fall_speed: self.fall_speed,
            fire_delay: self.fire_delay,
            kind: self.kind(),
            pad: 0,
            flags: BARREL_FLAGS,
            pad_again: 0,
            area: [self.width, self.height, 0.0, 0.0],
            position: [self.x, self.y, 0.0, 0.0],
            hold_frames: BARREL_HOLD_FOREVER,
            attack_kind: BARREL_ATTACK_KIND,
            reserved: [0; 6],
            power: 4.0,
            vector: 80.0,
            reaction_effect: 100.0,
            reaction_fix: 0.0,
            reaction_add: 100.0,
            slip: 0.0,
            hit_stop_frame: 1.5,
            hit_stop_delay: 1.0,
            rate: 1.0,
            tail: [0.0; 3],
        }
    }

    pub fn position(&self) -> Vector4 {
        Vector4([self.x, self.y, 0.0, 0.0])
    }

    pub fn motion(&self) -> BarrelMotion {
        BarrelMotion {
            angle: self.rotation,
            speed: self.rotation_speed,
        }
    }

    pub fn class_rotation(angle: f32) -> f32 {
        (angle - CLASS_TURN).rem_euclid(FULL_TURN)
    }

    pub fn tolerance(&self) -> f32 {
        (self.rotation_speed * TOLERANCE_RATE).abs().max(TOLERANCE_MIN)
    }

    pub fn turns(&self, holding: bool) -> bool {
        self.rotation_speed != 0.0 && (holding || self.always_rotate)
    }

    pub fn step(&self, motion: &mut BarrelMotion) {
        let mut next = motion.angle + motion.speed;
        let tolerance = self.tolerance();
        if !self.full_rotate
            && ((next - self.rotation_max).abs() < tolerance || (next - self.rotation).abs() < tolerance)
        {
            motion.speed = -motion.speed;
        }
        if next > FULL_TURN {
            next -= FULL_TURN;
        }
        if next < 0.0 {
            next += FULL_TURN;
        }
        motion.angle = next;
    }

    pub fn fires_at(&self, angle: f32) -> bool {
        self.auto_fire && (angle - self.fire_rotation).abs() < self.tolerance()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ladder(pairs: &[(&str, f32)]) -> LadderDraft {
        let mut draft = LadderDraft::default();
        for (key, value) in pairs {
            assert!(draft.set(key, *value));
        }
        draft
    }

    fn barrel(pairs: &[(&str, f32)], flags: &[(&str, bool)]) -> Barrel {
        let mut draft = BarrelDraft::default();
        for (key, value) in pairs {
            assert!(draft.set_number(key, *value));
        }
        for (key, value) in flags {
            assert!(draft.set_flag(key, *value));
        }
        draft.finish().unwrap()
    }

    #[test]
    fn a_ladder_needs_its_bottom_and_height_and_defaults_its_width() {
        let ladder = ladder(&[("x", 120.0), ("y", -20.0), ("height", 80.0)]).finish().unwrap();
        assert_eq!(
            ladder,
            Ladder {
                x: 120.0,
                y: -20.0,
                height: 80.0,
                width: LADDER_WIDTH
            }
        );
        assert_eq!(ladder.bottom(), Vector4([120.0, -20.0, 0.0, 0.0]));
        assert_eq!(ladder.top(), Vector4([120.0, 60.0, 0.0, 0.0]));
    }

    #[test]
    fn a_missing_key_names_itself() {
        assert_eq!(
            ladder(&[("x", 1.0), ("height", 5.0)]).finish(),
            Err(GimmickError::Missing("y"))
        );
        assert_eq!(
            ladder(&[("x", 1.0), ("y", 5.0)]).finish(),
            Err(GimmickError::Missing("height"))
        );
        assert_eq!(BarrelDraft::default().finish(), Err(GimmickError::Missing("x")));
    }

    #[test]
    fn a_gimmick_must_have_a_size_and_stay_on_the_map() {
        assert_eq!(
            ladder(&[("x", 1.0), ("y", 0.0), ("height", 0.0)]).finish(),
            Err(GimmickError::OutOfRange("height"))
        );
        assert_eq!(
            ladder(&[("x", 1.0), ("y", 0.0), ("height", 9.0), ("width", -1.0)]).finish(),
            Err(GimmickError::OutOfRange("width"))
        );
        assert_eq!(
            ladder(&[("x", f32::NAN), ("y", 0.0), ("height", 9.0)]).finish(),
            Err(GimmickError::OutOfRange("x"))
        );
        let mut draft = BarrelDraft::default();
        draft.set_number("x", 0.0);
        draft.set_number("y", 0.0);
        draft.set_number("rotation", 400.0);
        assert_eq!(draft.finish(), Err(GimmickError::OutOfRange("rotation")));
    }

    #[test]
    fn unknown_keys_are_not_taken() {
        let mut draft = LadderDraft::default();
        assert!(!draft.set("top", 3.0));
        assert_eq!(draft, LadderDraft::default());
        let mut barrel = BarrelDraft::default();
        assert!(!barrel.set_number("auto_fire", 1.0));
        assert!(!barrel.set_flag("speed", true));
    }

    #[test]
    fn the_ladder_area_is_the_size_the_ladder_class_centres_itself() {
        let ladder = ladder(&[("x", 0.0), ("y", 0.0), ("height", 48.0)]).finish().unwrap();
        let area = ladder.area();
        assert_eq!(area.offset, [0.0; 4]);
        assert_eq!(area.range, [10.0, 48.0, 0.0, 0.0]);
        assert_eq!(area.layer, 0x5000_0000);
        assert_eq!(area.owner, u32::MAX);
        assert_eq!(area.layer_again, 0x5000_0000);
    }

    #[test]
    fn a_barrel_defaults_to_brawls_shot_and_area() {
        let barrel = barrel(&[("x", 1165.0), ("y", 190.0)], &[]);
        assert_eq!(barrel.rotation_max, 0.0);
        assert_eq!(barrel.fire_rotation, 0.0);
        let data = barrel.data();
        assert_eq!([data.speed, data.decrease, data.gravity, data.fall_speed], [5.0, 0.075, 0.025, 0.25]);
        assert_eq!(data.area, [20.0, 15.0, 0.0, 0.0]);
        assert_eq!(data.position, [1165.0, 190.0, 0.0, 0.0]);
        assert_eq!(data.kind, BARREL_KIND_NATURAL);
        assert_eq!(data.hold_frames, BARREL_HOLD_FOREVER);
    }

    #[test]
    fn a_barrel_may_name_its_model_set() {
        let mut draft = BarrelDraft::default();
        draft.set_number("x", 0.0);
        draft.set_number("y", 0.0);
        assert!(draft.set_text("model", "GBC_01_set"));
        assert!(!draft.set_text("name", "x"));
        assert_eq!(draft.clone().finish().unwrap().model.as_deref(), Some("gbc_01_set"));
        draft.set_text("model", "bad name");
        assert_eq!(draft.finish(), Err(GimmickError::OutOfRange("model")));
    }

    #[test]
    fn an_auto_barrel_uses_the_auto_area_kind() {
        let barrel = barrel(&[("x", 0.0), ("y", 0.0), ("fire_delay", 5.0)], &[("auto_fire", true)]);
        assert_eq!(barrel.data().kind, BARREL_KIND_NATURAL_AUTO);
        assert_eq!(barrel.data().fire_delay, 5);
    }

    #[test]
    fn a_sweeping_barrel_turns_back_at_both_ends() {
        let barrel = barrel(
            &[("x", 0.0), ("y", 0.0), ("rotation", 0.0), ("rotation_max", 90.0), ("rotation_speed", 1.0)],
            &[],
        );
        let mut motion = barrel.motion();
        let mut highest: f32 = 0.0;
        for _ in 0..120 {
            barrel.step(&mut motion);
            highest = highest.max(if motion.angle > 180.0 { 0.0 } else { motion.angle });
        }
        assert!(highest >= 89.0 && highest <= 90.5, "{highest}");
        assert!(motion.speed < 0.0);
        assert!(motion.angle < 90.0);
    }

    #[test]
    fn a_full_turn_barrel_wraps_instead_of_turning_back() {
        let barrel = barrel(&[("x", 0.0), ("y", 0.0), ("rotation_speed", 4.0)], &[("full_rotate", true)]);
        let mut motion = BarrelMotion { angle: 358.0, speed: 4.0 };
        barrel.step(&mut motion);
        assert!((motion.angle - 2.0).abs() < 1e-4);
        assert_eq!(motion.speed, 4.0);
    }

    #[test]
    fn the_tolerance_and_the_fire_angle_follow_brawl() {
        let barrel = barrel(
            &[("x", 0.0), ("y", 0.0), ("rotation_speed", 2.4), ("fire_rotation", 62.0)],
            &[("auto_fire", true)],
        );
        assert!((barrel.tolerance() - 1.2).abs() < 1e-6);
        assert!(barrel.fires_at(61.0));
        assert!(!barrel.fires_at(60.0));
        let still = self::barrel(&[("x", 0.0), ("y", 0.0)], &[("auto_fire", true)]);
        assert_eq!(still.tolerance(), TOLERANCE_MIN);
        assert!(still.fires_at(0.0));
        assert!(!still.turns(true));
    }

    #[test]
    fn a_path_barrel_walks_its_points_and_shoots_nothing_itself() {
        let mut draft = BarrelDraft::default();
        draft.set_number("x", 990.0);
        draft.set_number("y", 55.0);
        draft.set_number("path_speed", 2.5);
        assert!(draft.set_numbers("path", &[990.0, 55.0, 1000.0, 75.0, 1020.0, 95.0]));
        draft.set_flag("auto_fire", true);
        let barrel = draft.finish().unwrap();
        assert!(barrel.follows_path());
        assert_eq!(barrel.kind(), BARREL_KIND_PATH_AUTO);
        assert_eq!(barrel.data().speed, 0.0);
        assert_eq!(barrel.path_point(0.0), Some([990.0, 55.0]));
        assert_eq!(barrel.path_point(1.5), Some([1010.0, 85.0]));
        assert_eq!(barrel.path_point(2.0), None);
        let mut odd = BarrelDraft::default();
        odd.set_number("x", 0.0);
        odd.set_number("y", 0.0);
        odd.set_numbers("path", &[1.0, 2.0, 3.0]);
        assert_eq!(odd.finish(), Err(GimmickError::OutOfRange("path")));
    }

    #[test]
    fn a_barrel_event_puts_its_fields_where_the_game_reads_them() {
        let event = Barrel::event(BARREL_EVENT_SET_PATH, 0, 0x3000_0000, [10.0, -2.5]);
        assert_eq!(core::mem::size_of::<BarrelEvent>(), 0xc0);
        assert_eq!(event.0[0xc / 4], 0);
        assert_eq!(event.0[0x10 / 4], 6);
        assert_eq!(event.0[0x14 / 4], 0x3000_0000);
        assert_eq!(f32::from_bits(event.0[0x20 / 4]), 10.0);
        assert_eq!(f32::from_bits(event.0[0x24 / 4]), -2.5);
        assert_eq!(event.0[0x90 / 4], u32::MAX);
        assert_eq!(event.0[0xb4 / 4], 1);
    }

    #[test]
    fn the_class_aims_a_quarter_turn_later_than_the_barrel() {
        assert_eq!(Barrel::class_rotation(90.0), 0.0);
        assert_eq!(Barrel::class_rotation(0.0), 270.0);
        assert_eq!(Barrel::class_rotation(330.0), 240.0);
    }

    #[test]
    fn a_barrel_turns_while_held_or_always() {
        let held = barrel(&[("x", 0.0), ("y", 0.0), ("rotation_speed", 1.0)], &[]);
        assert!(held.turns(true));
        assert!(!held.turns(false));
        let always = barrel(&[("x", 0.0), ("y", 0.0), ("rotation_speed", 1.0)], &[("always_rotate", true)]);
        assert!(always.turns(false));
    }

    #[test]
    fn the_game_layouts_hold() {
        assert_eq!(core::mem::size_of::<AreaParam>(), 0x40);
        assert_eq!(core::mem::align_of::<AreaParam>(), 0x10);
        assert_eq!(core::mem::offset_of!(AreaParam, range), 0x10);
        assert_eq!(core::mem::offset_of!(AreaParam, layer), 0x28);
        assert_eq!(core::mem::offset_of!(AreaParam, owner), 0x2c);
        assert_eq!(core::mem::offset_of!(AreaParam, flags), 0x30);
        assert_eq!(core::mem::offset_of!(AreaParam, kind), 0x34);
        assert_eq!(core::mem::offset_of!(AreaParam, layer_again), 0x38);
        assert_eq!(core::mem::size_of::<Vector4>(), 0x10);
        assert_eq!(core::mem::size_of::<BarrelData>(), 0x90);
        assert_eq!(core::mem::offset_of!(BarrelData, fire_delay), 0x10);
        assert_eq!(core::mem::offset_of!(BarrelData, kind), 0x14);
        assert_eq!(core::mem::offset_of!(BarrelData, flags), 0x18);
        assert_eq!(core::mem::offset_of!(BarrelData, area), 0x20);
        assert_eq!(core::mem::offset_of!(BarrelData, position), 0x30);
        assert_eq!(core::mem::offset_of!(BarrelData, hold_frames), 0x40);
        assert_eq!(core::mem::offset_of!(BarrelData, attack_kind), 0x44);
        assert_eq!(core::mem::offset_of!(BarrelData, power), 0x60);
        assert_eq!(core::mem::offset_of!(BarrelData, hit_stop_delay), 0x7c);
        assert_eq!(core::mem::offset_of!(BarrelData, rate), 0x80);
    }

    #[test]
    fn a_spring_defaults_to_brawls_power_and_area() {
        let mut draft = SpringDraft::default();
        assert!(draft.set_number("x", 155.0));
        assert!(draft.set_number("y", -30.0));
        assert!(!draft.set_number("power", 4.0));
        let spring = draft.finish().unwrap();
        assert_eq!(spring.speed, SPRING_SPEED);
        assert_eq!(spring.rotation, 0.0);
        assert_eq!(spring.position(), Vector4([155.0, -30.0, 0.0, 0.0]));
        let area = spring.area();
        assert_eq!(area.offset, [155.0, -30.0, 0.0, 0.0]);
        assert_eq!(area.range, [SPRING_AREA_WIDTH, SPRING_AREA_HEIGHT, 0.0, 0.0]);
        assert_eq!(area.layer, AREA_LAYER);
    }

    #[test]
    fn a_spring_leans_both_ways_and_checks_its_speed() {
        let mut draft = SpringDraft::default();
        draft.set_number("x", 0.0);
        draft.set_number("y", 0.0);
        draft.set_number("rotation", -33.6);
        draft.set_number("speed", 5.5);
        draft.set_number("offset_y", 2.0);
        assert!(draft.set_text("model", "Spring_01_set"));
        let spring = draft.clone().finish().unwrap();
        assert_eq!(spring.rotation, -33.6);
        assert_eq!(spring.speed, 5.5);
        assert_eq!(spring.model.as_deref(), Some("spring_01_set"));
        assert_eq!(spring.area().offset, [0.0, 2.0, 0.0, 0.0]);
        draft.set_number("speed", 0.0);
        assert_eq!(draft.clone().finish(), Err(GimmickError::OutOfRange("speed")));
        draft.set_number("speed", 4.0);
        draft.set_number("rotation", 400.0);
        assert_eq!(draft.finish(), Err(GimmickError::OutOfRange("rotation")));
        assert_eq!(SpringDraft::default().finish(), Err(GimmickError::Missing("x")));
    }

    #[test]
    fn a_spring_with_a_motion_waits_for_its_launch_frame() {
        let mut draft = SpringDraft::default();
        draft.set_number("x", 0.0);
        draft.set_number("y", 0.0);
        assert!(draft.set_text("joint", "SpringTop"));
        assert!(draft.set_number("launch_frame", 3.0));
        let spring = draft.clone().finish().unwrap();
        assert_eq!(spring.joint.as_deref(), Some("SpringTop"));
        assert!(spring.launches_at_once());
        draft.set_text("model", "gspr_00_set");
        assert!(!draft.clone().finish().unwrap().launches_at_once());
        draft.set_text("joint", "Spring Top");
        assert_eq!(draft.clone().finish(), Err(GimmickError::OutOfRange("joint")));
        draft.set_text("joint", "SpringTop");
        draft.set_number("launch_frame", -1.0);
        assert_eq!(draft.finish(), Err(GimmickError::OutOfRange("launch_frame")));
    }

    #[test]
    fn every_header_spelling_names_its_kind() {
        assert_eq!(gimmick_header("[[ladder]]"), Some(GimmickKind::Ladder));
        assert_eq!(gimmick_header("  [[ stage.ladder ]] "), Some(GimmickKind::Ladder));
        assert_eq!(gimmick_header("[[barrel]]"), Some(GimmickKind::Barrel));
        assert_eq!(gimmick_header("[[stage.barrel]]"), Some(GimmickKind::Barrel));
        assert_eq!(gimmick_header("[[spring]]"), Some(GimmickKind::Spring));
        assert_eq!(gimmick_header("[[stage.spring]]"), Some(GimmickKind::Spring));
        assert_eq!(gimmick_header("[[door]]"), Some(GimmickKind::Door));
        assert_eq!(gimmick_header("[[stage.door]]"), Some(GimmickKind::Door));
        assert_eq!(gimmick_header("[[stage]]"), None);
        assert_eq!(gimmick_header("[ladder]"), None);
    }

    #[test]
    fn a_door_is_a_ground_door_anyone_may_enter_by_default() {
        let mut draft = DoorDraft::default();
        assert!(draft.set_number("x", 625.0));
        assert!(draft.set_number("y", 193.0));
        assert!(!draft.set_number("model", 1.0));
        let door = draft.finish().unwrap();
        assert_eq!(door.kind(), DOOR_KIND_GROUND);
        assert_eq!(door.entry(), AREA_OWNER_NONE);
        assert_eq!(door.center(), [625.0, 193.0]);
        assert_eq!((door.width, door.height), (DOOR_AREA_WIDTH, DOOR_AREA_HEIGHT));
        assert_eq!(DoorDraft::default().finish(), Err(GimmickError::Missing("x")));
    }

    #[test]
    fn a_door_area_is_centred_on_its_offset_and_hosted_from_its_foot() {
        let mut draft = DoorDraft::default();
        draft.set_number("x", 625.0);
        draft.set_number("y", 193.0);
        draft.set_number("offset_y", 10.0);
        draft.set_number("width", 15.0);
        draft.set_number("height", 20.0);
        let host = draft.finish().unwrap().host();
        assert_eq!(host, Ladder { x: 625.0, y: 193.0, height: 20.0, width: 15.0 });
        assert_eq!(host.bottom(), Vector4([625.0, 193.0, 0.0, 0.0]));
        assert_eq!(host.top(), Vector4([625.0, 213.0, 0.0, 0.0]));
    }

    #[test]
    fn a_door_can_be_an_air_door_for_one_player() {
        let mut draft = DoorDraft::default();
        draft.set_number("x", 0.0);
        draft.set_number("y", 0.0);
        assert!(draft.set_flag("air", true));
        assert!(!draft.set_flag("goal", true));
        assert!(draft.set_number("player", 1.0));
        let door = draft.finish().unwrap();
        assert_eq!(door.kind(), DOOR_KIND_AIR);
        assert_eq!(door.entry(), 0);
        for bad in [0.0, 9.0, 1.5] {
            draft.set_number("player", bad);
            assert_eq!(draft.finish(), Err(GimmickError::OutOfRange("player")));
        }
    }
}
