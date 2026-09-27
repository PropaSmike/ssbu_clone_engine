#![feature(proc_macro_hygiene)]
#![allow(non_snake_case, non_upper_case_globals, unused_imports, unused_variables)]

pub use smashline::skyline_smash as smash;

use clone_engine_api::v2::{fighter, Manifest, Motion};
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

/// The engine features this pack uses. `runtime_capabilities()` is what the
/// installed engine has AND passed its checks for on this boot.
const NEEDED: u64 = clone_engine_api::CAP_FIGHTER_IDENTITY
    | clone_engine_api::CAP_SMASHLINE_BRIDGE
    | clone_engine_api::CAP_FIGHTER_ARTICLES
    | clone_engine_api::CAP_PARAMCONFIG_BRIDGE
    | clone_engine_api::CAP_KIRBY_COPY
    | clone_engine_api::CAP_SHARED_HOOKS;

extern "C" fn mods_mounted() {
    // work that needs the mods' files mounted by ARCropolis goes here
    clone_engine_api::elog!("[template_v2] mods mounted");
}

#[skyline::main(name = "clone_engine_moveset_template_v2")]
pub fn main() {
    let missing = NEEDED & !clone_engine_api::runtime_capabilities();
    if missing != 0 {
        clone_engine_api::elog!("[template_v2] disabled: the engine lacks capabilities {missing:#x}");
        return;
    }

    // The same declaration can be a fighter.toml beside config.json; the engine
    // then registers at boot and register() below just returns that kind.
    let manifest = Manifest::new(
        "template_fighter", // name: folder under fighter/, Smashline agent name, identity
        BASE,               // base: the vanilla fighter it is built on
    )
    .costumes(COSTUMES as u32)                      // the costume count
    .color_start(0)                                 // the first costume folder, c00
    .own_css()                                      // csk::publish writes the select entry, not the engine
    .article("template_fireball", "mario/fireball") // fighter/template_fighter/model/template_fireball, copied from Mario's fireball
    .kirby_article("template_fighter_fireball", "mario/fireball") // the copy's own article, files under fighter/kirby/model/template_fighter_fireball
    .kirby(1)                                       // one status number for Kirby's copy
    .kirby_motion(
        // name (must start with the fighter's name), animation file
        Motion::new("template_fighter_special_n", "template_fighterd00specialn.nuanmb")
            .template("mario_special_n")        // the Kirby motion whose settings it copies
            .scripts("templatefighterspecialn"), // game_/sound_/effect_/expression_ + this, on Agent::new("kirby")
    )
    .kirby_motion(
        Motion::new("template_fighter_special_air_n", "template_fighterd00specialairn.nuanmb")
            .template("mario_special_air_n")
            .scripts("templatefighterspecialairn"),
    );
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
        .expression_acmd("expression_attack11", acmd::expression_attack11, Priority::Default)
        .game_acmd("game_appeallw", acmd::game_appeallw, Priority::Default)
        .status(smashline::Exec, *smash::lib::lua_const::FIGHTER_STATUS_KIND_WAIT, status::wait_exec) // status line, status kind, function
        .on_line(Main, frame::once_per_frame) // once per frame
        .on_start(frame::on_start)            // when the fighter is created
        .install();

    articles::install(kind);
    kirby::install();
    hooks::install();

    if let Err(error) = clone_engine_api::v2::on_mods_mounted(mods_mounted) {
        clone_engine_api::elog!("[template_v2] mount event unavailable: {error:?}");
    }

    clone_engine_api::elog!("[template_v2] installed; kind {kind}");
}
