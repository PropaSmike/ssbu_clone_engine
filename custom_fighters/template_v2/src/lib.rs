#![feature(proc_macro_hygiene)]
#![allow(non_snake_case, non_upper_case_globals, unused_imports, unused_variables)]

pub use smashline::skyline_smash as smash;

use clone_engine_api::v2::{fighter, Manifest};
use smashline::{Agent, Main, Priority};

mod acmd;
mod articles;
mod csk;
mod frame;
mod hooks;
mod kirby;
mod params;
mod status;

// TEMPLATE: the handle the rest of the crate uses.
// "template_fighter": the identity, the same string as Manifest::new's name.
fighter!(TEMPLATE, "template_fighter");

/// How many costumes the pack ships (c00..). Drives the manifest, the CSK
/// entry and the ParamConfig slots, so a pack with 6 or 12 changes one line.
pub const COSTUMES: u8 = 8;

/// The vanilla fighter the clone is built on, by resource name.
pub const BASE: &str = "mario";

#[skyline::main(name = "clone_engine_moveset_template_v2")]
pub fn main() {
    // The same declaration can be a fighter.toml beside config.json; the engine
    // then registers at boot and register() below just returns that kind.
    let manifest = Manifest::new(
        "template_fighter", // name: folder under fighter/, Smashline agent name, identity
        BASE,               // base: the vanilla fighter it is built on
    )
    .costumes(COSTUMES as u32)                      // the costume count
    .own_css()                                      // csk::publish writes the select entry, not the engine
    .article("template_fireball", "mario/fireball") // fighter/template_fighter/model/template_fireball, copied from Mario's fireball
    .kirby(1);                                      // one status number for Kirby's copy
    let kind = match TEMPLATE.register(manifest) {
        Ok(kind) => kind,
        Err(error) => {
            clone_engine_api::elog!("[template_v2] registration refused: {error:?}");
            return;
        }
    };

    csk::publish();
    params::install(kind);

    Agent::new("template_fighter") // scripts under the clone's own name, never "mario"
        .game_acmd("game_attack11", acmd::game_attack11, Priority::Default) // script name, function, priority
        .effect_acmd("effect_attack11", acmd::effect_attack11, Priority::Default)
        .sound_acmd("sound_attack11", acmd::sound_attack11, Priority::Default)
        .game_acmd("game_appeallw", acmd::game_appeallw, Priority::Default)
        .status(smashline::Exec, *smash::lib::lua_const::FIGHTER_STATUS_KIND_WAIT, status::wait_exec) // status line, status kind, function
        .on_line(Main, frame::once_per_frame) // once per frame
        .on_start(frame::on_start)            // when the fighter is created
        .install();

    articles::install();
    kirby::install();
    hooks::install();

    clone_engine_api::elog!("[template_v2] installed; kind {kind}");
}
