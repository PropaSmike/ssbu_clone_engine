//! A menu pack: three modes on the engine's modes page (pack/menu.toml), the
//! Lua that starts them (lua/, compiled into pack/ui/script/menu/template_menu/)
//! and this plugin, which answers that Lua and runs the battles.
//!
//! - Gauntlet: rounds against CPUs, one battle each, ending on a boss the
//!   plugin brings in itself (setup.rs, battle.rs).
//! - Boss: the game's own boss battle against Galleom.
//! - Intro: a screen inside the modes page that plays a movie.

pub use smashline::skyline_smash as smash;

use clone_engine_api::menu;
use smashline::{Agent, Main};

mod battle;
mod setup;

#[skyline::main(name = "clone_engine_menu_template")]
pub fn main() {
    // the names the Lua passes to prg_func.engine_call, and the functions that answer
    let calls: [(&str, menu::MenuCall); 6] = [
        ("template_menu.gauntlet", setup::gauntlet),
        ("template_menu.boss", setup::boss),
        ("template_menu.how_it_ended", setup::how_it_ended),
        ("template_menu.movie", setup::movie),
        ("template_menu.movie_playing", setup::movie_playing),
        ("template_menu.movie_stop", setup::movie_stop),
    ];
    for (name, handler) in calls {
        if let Err(error) = menu::register_call(name, handler) {
            println!("[template_menu] {name} refused: {error:?}");
        }
    }
    // "fighter": every fighter, once per frame; battle.rs keeps to player 1's
    Agent::new("fighter").on_line(Main, battle::frame).install();
}
