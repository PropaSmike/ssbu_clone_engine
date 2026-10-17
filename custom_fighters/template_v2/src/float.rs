//! Peach's float, with her rules and values: hold jump past the top of a
//! jump, or hold down and jump on the ground. This fighter ships no
//! `fuwafuwa_start` or `fuwafuwa` animation, so it floats in its fall.
//!
//! Brawl's glide (`clone_engine_api::glide`) starts from the same input, so a
//! fighter takes one or the other.

use crate::smash;
use clone_engine_api::float::{self, Float};
use smash::lib::lua_const::*;
use smashline::{Agent, End, Init, Main, Pre};

pub fn install() {
    // The float's two statuses follow the base's own. Mario's status table is
    // sized with FIGHTER_MARIO_STATUS_KIND_NUM; Donkey Kong, Bowser, Peach and
    // Pit use _MAX, and an echo uses its original's. Statuses of your own
    // start at start + 2.
    let start = *FIGHTER_MARIO_STATUS_KIND_NUM;
    float::register(
        "template_fighter", // the fighter's name
        Float::PEACH,       // her values; .frames(n), .start_y(y), .motions(start, hover) change them
        start,              // the first of the two status numbers
    );
    Agent::new("template_fighter")
        .on_start(float::on_start)
        .on_line(Main, float::frame) // the way in, checked every frame
        .status(Pre, start, float::start_pre) // the float's start
        .status(Init, start, float::start_init)
        .status(Main, start, float::start_main)
        .status(End, start, float::end)
        .status(Pre, start + 1, float::float_pre) // the hover
        .status(Main, start + 1, float::float_main)
        .status(End, start + 1, float::end)
        // an aerial in the middle of a float keeps floating; a fighter with its
        // own aerial status calls these two from it instead
        .status(Pre, *FIGHTER_STATUS_KIND_ATTACK_AIR, float::attack_air_pre)
        .status(Main, *FIGHTER_STATUS_KIND_ATTACK_AIR, float::attack_air_main)
        .install();
}
