use smash::app::lua_bind::{
    ControlModule, KineticEnergy, KineticEnergyNormal, KineticModule, MotionModule, PostureModule,
    StatusModule, WorkModule,
};
use smash::app::{BattleObjectModuleAccessor, KineticEnergyNormal as EnergyNormal};
use smash::lib::lua_const::*;
use smash::lib::L2CValue;
use smash::lua2cpp::L2CFighterCommon;
use smash::phx::{Hash40, Vector2f, Vector3f};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glide {
    pub max_angle: f32,
    pub min_angle: f32,
    pub start_speed_y: f32,
    pub start_accel_y: f32,
    pub start_speed_x_mul: f32,
    pub power_start: f32,
    pub power_rate: f32,
    pub max_speed: f32,
    pub end_speed: f32,
    pub gravity_accel: f32,
    pub gravity_max: f32,
    pub start_angle: f32,
    pub descent_angle: f32,
    pub descent_accel: f32,
    pub descent_brake: f32,
    pub stick_threshold: f32,
    pub angle_accel_up: f32,
    pub angle_accel_down: f32,
    pub angle_speed_max: f32,
    pub angle_speed_brake: f32,
    pub button_frame: i32,
}

impl Glide {
    pub const PIT: Glide = Glide::from_brawl(
        [45.0, -75.0, 0.75, 1.0, 0.5312, 1.65, 0.05, 1.8, 0.5, 0.05, 0.4, 0.0, -25.0, 0.01, 0.1, 0.25, 2.0, 0.5, 6.0, 0.15],
    );
    pub const META_KNIGHT: Glide = Glide::from_brawl(
        [45.0, -70.0, 0.75, 1.0, 1.0, 1.7, 0.04, 2.2, 0.7, 0.03, 0.6, 15.0, -25.0, 0.03, 0.15, 0.25, 0.55, 0.75, 7.0, 1.0],
    );
    pub const CHARIZARD: Glide = Glide::from_brawl(
        [50.0, -60.0, 1.2, 0.15, 1.0, 1.275, 0.05, 1.9, 0.5, 0.06, 0.55, 0.0, -25.0, 0.01, 0.1, 0.25, 0.8, 0.4, 8.0, 0.15],
    );

    pub const fn from_brawl(words: [f32; 20]) -> Glide {
        Glide {
            max_angle: words[0],
            min_angle: words[1],
            start_speed_y: words[2],
            start_accel_y: words[3],
            start_speed_x_mul: words[4],
            power_start: words[5],
            power_rate: words[6],
            max_speed: words[7],
            end_speed: words[8],
            gravity_accel: words[9],
            gravity_max: words[10],
            start_angle: words[11],
            descent_angle: words[12],
            descent_accel: words[13],
            descent_brake: words[14],
            stick_threshold: words[15],
            angle_accel_up: words[16],
            angle_accel_down: words[17],
            angle_speed_max: words[18],
            angle_speed_brake: words[19],
            button_frame: 8,
        }
    }

    pub const fn button_frame(mut self, frames: i32) -> Glide {
        self.button_frame = frames;
        self
    }
}

unsafe fn boma(fighter: &mut L2CFighterCommon) -> *mut BattleObjectModuleAccessor {
    fighter.module_accessor
}

unsafe fn status(boma: *mut BattleObjectModuleAccessor) -> i32 {
    StatusModule::status_kind(boma)
}

unsafe fn grounded(boma: *mut BattleObjectModuleAccessor) -> bool {
    StatusModule::situation_kind(boma) == *SITUATION_KIND_GROUND
}

pub unsafe fn frame(fighter: &mut L2CFighterCommon, glide: &Glide) -> bool {
    let boma = boma(fighter);
    if grounded(boma) {
        WorkModule::off_flag(boma, *FIGHTER_INSTANCE_WORK_ID_FLAG_NO_GLIDE);
        return false;
    }
    if status(boma) != *FIGHTER_STATUS_KIND_JUMP_AERIAL {
        return false;
    }
    if WorkModule::is_flag(boma, *FIGHTER_INSTANCE_WORK_ID_FLAG_NO_GLIDE) {
        return false;
    }
    if !ControlModule::check_button_on(boma, *CONTROL_PAD_BUTTON_JUMP) {
        return false;
    }
    if WorkModule::get_int(boma, *FIGHTER_STATUS_JUMP_WORK_INT_BUTTON_ON_FRAME) < glide.button_frame {
        return false;
    }
    if KineticModule::get_sum_speed_y(boma, *KINETIC_ENERGY_RESERVE_ATTRIBUTE_MAIN) > 0.0 {
        return false;
    }
    StatusModule::change_status_request_from_script(boma, *FIGHTER_STATUS_KIND_GLIDE_START, true);
    true
}

unsafe fn stop_energy(boma: *mut BattleObjectModuleAccessor) -> *mut EnergyNormal {
    KineticModule::get_energy(boma, *FIGHTER_KINETIC_ENERGY_ID_STOP) as *mut EnergyNormal
}

unsafe fn drive(boma: *mut BattleObjectModuleAccessor, x: f32, y: f32) {
    let energy = stop_energy(boma);
    KineticEnergyNormal::set_speed(energy, &Vector2f { x, y });
}

unsafe fn take_over(boma: *mut BattleObjectModuleAccessor, x: f32, y: f32) {
    for id in [
        *FIGHTER_KINETIC_ENERGY_ID_MOTION,
        *FIGHTER_KINETIC_ENERGY_ID_GRAVITY,
        *FIGHTER_KINETIC_ENERGY_ID_CONTROL,
    ] {
        KineticModule::unable_energy(boma, id);
    }
    KineticModule::enable_energy(boma, *FIGHTER_KINETIC_ENERGY_ID_STOP);
    let energy = stop_energy(boma);
    KineticEnergy::reset_energy(
        energy as *mut _,
        *ENERGY_STOP_RESET_TYPE_AIR,
        &Vector2f { x, y },
        &Vector3f { x: 0.0, y: 0.0, z: 0.0 },
        boma,
    );
    KineticEnergyNormal::set_brake(energy, &Vector2f { x: 0.0, y: 0.0 });
    KineticEnergyNormal::set_accel(energy, &Vector2f { x: 0.0, y: 0.0 });
    KineticEnergyNormal::set_stable_speed(energy, &Vector2f { x: 99.0, y: 99.0 });
    KineticEnergyNormal::set_limit_speed(energy, &Vector2f { x: 99.0, y: 99.0 });
    drive(boma, x, y);
}

pub unsafe fn glide_start_init(fighter: &mut L2CFighterCommon, glide: &Glide) -> L2CValue {
    let boma = boma(fighter);
    let x = KineticModule::get_sum_speed_x(boma, *KINETIC_ENERGY_RESERVE_ATTRIBUTE_MAIN) * glide.start_speed_x_mul;
    take_over(boma, x, glide.start_speed_y);
    WorkModule::set_float(boma, glide.start_speed_y, *FIGHTER_STATUS_GLIDE_WORK_FLOAT_GRAVITY);
    0.into()
}

pub unsafe fn glide_start_exec(fighter: &mut L2CFighterCommon, glide: &Glide) -> L2CValue {
    let boma = boma(fighter);
    let speed_y = WorkModule::get_float(boma, *FIGHTER_STATUS_GLIDE_WORK_FLOAT_GRAVITY);
    let next = (speed_y - glide.start_accel_y * glide.gravity_accel).max(-glide.gravity_max);
    WorkModule::set_float(boma, next, *FIGHTER_STATUS_GLIDE_WORK_FLOAT_GRAVITY);
    let x = KineticModule::get_sum_speed_x(boma, *KINETIC_ENERGY_RESERVE_ATTRIBUTE_MAIN);
    drive(boma, x, next);
    0.into()
}

pub unsafe fn glide_init(fighter: &mut L2CFighterCommon, glide: &Glide) -> L2CValue {
    let boma = boma(fighter);
    let speed_y = KineticModule::get_sum_speed_y(boma, *KINETIC_ENERGY_RESERVE_ATTRIBUTE_MAIN);
    WorkModule::set_float(boma, glide.power_start, *FIGHTER_STATUS_GLIDE_WORK_FLOAT_POWER);
    WorkModule::set_float(boma, -speed_y, *FIGHTER_STATUS_GLIDE_WORK_FLOAT_GRAVITY);
    WorkModule::set_float(boma, glide.start_angle, *FIGHTER_STATUS_GLIDE_WORK_FLOAT_ANGLE);
    WorkModule::set_float(boma, 0.0, *FIGHTER_STATUS_GLIDE_WORK_FLOAT_ANGLE_SPEED);
    let lr = PostureModule::lr(boma);
    take_over(boma, glide.power_start * lr, speed_y);
    if MotionModule::is_anim_resource(boma, Hash40::new("glide")) {
        MotionModule::change_motion(boma, Hash40::new("glide"), 0.0, 1.0, false, 0.0, false, false);
    }
    0.into()
}

pub unsafe fn glide_exec(fighter: &mut L2CFighterCommon, glide: &Glide) -> L2CValue {
    let boma = boma(fighter);
    let lr = PostureModule::lr(boma);
    let stopped = WorkModule::is_flag(boma, *FIGHTER_STATUS_GLIDE_FLAG_STOP);
    let stick_x = ControlModule::get_stick_x(boma) * lr;
    let stick_y = ControlModule::get_stick_y(boma);
    let magnitude = (stick_x * stick_x + stick_y * stick_y).sqrt().min(1.0);
    let mut angle = WorkModule::get_float(boma, *FIGHTER_STATUS_GLIDE_WORK_FLOAT_ANGLE);
    let mut angle_speed = WorkModule::get_float(boma, *FIGHTER_STATUS_GLIDE_WORK_FLOAT_ANGLE_SPEED);
    if !stopped && magnitude > glide.stick_threshold {
        let strength = (magnitude - glide.stick_threshold) / (1.0 - glide.stick_threshold);
        let accel = if stick_y - stick_x > 0.0 {
            glide.angle_accel_up * strength
        } else {
            -glide.angle_accel_down * strength
        };
        if accel * angle_speed < 0.0 {
            angle_speed = 0.0;
        }
        angle_speed = (angle_speed + accel).clamp(-glide.angle_speed_max, glide.angle_speed_max);
    } else if angle_speed.abs() <= glide.angle_speed_brake {
        angle_speed = 0.0;
    } else {
        angle_speed -= glide.angle_speed_brake * angle_speed.signum();
    }
    angle = (angle + angle_speed).clamp(glide.min_angle, glide.max_angle);
    let mut power = WorkModule::get_float(boma, *FIGHTER_STATUS_GLIDE_WORK_FLOAT_POWER);
    if !stopped {
        power -= angle * glide.power_rate / 90.0;
        if WorkModule::is_flag(boma, *FIGHTER_STATUS_GLIDE_FLAG_TOUCH_GROUND) {
            power -= 0.01;
        }
        if angle < glide.descent_angle && !WorkModule::is_flag(boma, *FIGHTER_STATUS_GLIDE_FLAG_RAPID_FALL) {
            power += (glide.descent_angle - angle) / 90.0 * glide.descent_accel;
        }
        power = power.max(0.0);
    }
    let mut gravity = WorkModule::get_float(boma, *FIGHTER_STATUS_GLIDE_WORK_FLOAT_GRAVITY);
    gravity = (gravity + glide.gravity_accel).min(glide.gravity_max);
    let radians = angle.to_radians();
    let mut x = power * radians.cos();
    let mut y = power * radians.sin() - gravity;
    let speed = (x * x + y * y).sqrt();
    if speed > glide.max_speed {
        x *= glide.max_speed / speed;
        y *= glide.max_speed / speed;
    }
    if !stopped && (speed < glide.end_speed || power <= 0.0) {
        WorkModule::on_flag(boma, *FIGHTER_STATUS_GLIDE_FLAG_STOP);
        angle_speed = 0.0;
    }
    WorkModule::set_float(boma, angle, *FIGHTER_STATUS_GLIDE_WORK_FLOAT_ANGLE);
    WorkModule::set_float(boma, angle_speed, *FIGHTER_STATUS_GLIDE_WORK_FLOAT_ANGLE_SPEED);
    WorkModule::set_float(boma, power, *FIGHTER_STATUS_GLIDE_WORK_FLOAT_POWER);
    WorkModule::set_float(boma, gravity, *FIGHTER_STATUS_GLIDE_WORK_FLOAT_GRAVITY);
    if MotionModule::end_frame(boma) >= 180.0 {
        MotionModule::set_frame(boma, 90.0 - angle, false);
    }
    drive(boma, x * lr, y);
    0.into()
}
