//! What the Lua asks for through prg_func.engine_call. A handler gets the
//! call's integer arguments and returns an integer to the Lua: 0, or the
//! engine's negative code when it refuses.

use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};

use crate::smash::lib::lua_const::*;
use dionysus_api::{menu, Error};

/// One gauntlet round: the CPU's fighter and the stage.
pub struct Round {
    pub cpu: &'static str,
    pub stage: &'static str,
}

/// The rounds against CPUs; the boss round comes after them.
pub const ROUNDS: [Round; 2] = [
    Round { cpu: "ui_chara_luigi", stage: "ui_stage_battle_field" },
    Round { cpu: "ui_chara_koopa", stage: "ui_stage_end" },
];
/// The boss round's stage.
pub const BOSS_STAGE: &str = "ui_stage_battle_field_l";
/// Player 1's team: each lost stock brings the next one in.
pub const TEAM: [&str; 3] = ["ui_chara_mario", "ui_chara_link", "ui_chara_samus"];
/// A song of ui_bgm_db, for the boss round.
pub const BOSS_SONG: &str = "ui_bgm_a01_smb_chijyou";
/// Layouts the pack ships, drawn on the battle HUD; None draws nothing.
pub const ROUND_LAYOUT: Option<&str> = None; // e.g. Some("ui/layout/hud/template_menu/round/layout.arc")
pub const CARD_LAYOUT: Option<&str> = None; // e.g. Some("ui/layout/hud/template_menu/card/layout.arc")
/// A movie the pack ships as stream;/movie/template_menu/intro.webm.
pub const INTRO_MOVIE: &str = "stream:/movie/template_menu/intro.webm";

const PLAYERS: u32 = 8;

/// The round being played, and player 1's controller.
pub static ROUND: AtomicU32 = AtomicU32::new(0);
pub static PAD: AtomicI32 = AtomicI32::new(-1);

fn code(result: Result<(), Error>) -> i64 {
    match result {
        Ok(()) => 0,
        Err(Error::Engine(code)) => code as i64,
        Err(_) => -1,
    }
}

unsafe fn first_argument(args: *const i64, count: usize) -> i64 {
    if args.is_null() || count == 0 {
        -1
    } else {
        *args
    }
}

/// Sets up gauntlet round `index` with player 1 as `fighter` carrying
/// `damage`; called before the first battle and between battles.
pub fn round(index: u32, fighter: &str, color: u32, damage: u32) -> Result<(), Error> {
    let pad = PAD.load(Ordering::Acquire);
    // slot, human/CPU/amiibo/none, fighter, controller (-1 for none); resets damage and costume
    menu::battle_player(0, menu::PlayerType::Human, fighter, pad)?;
    menu::battle_color(0, color)?; // slot, costume (0 = c00)
    menu::battle_damage(0, damage)?; // slot, starting damage in percent
    let first_empty = match ROUNDS.get(index as usize) {
        Some(round) => {
            menu::battle_player(1, menu::PlayerType::Cpu, round.cpu, -1)?;
            menu::battle_color(1, index)?;
            menu::battle_stage(round.stage)?;
            // stocks, time limit in seconds (0 = none), item frequency (0 = none)
            menu::battle_rules(menu::Rules { kind: menu::RuleKind::Stock, stock: TEAM.len() as u32, seconds: 0, items: 0 })?;
            menu::battle_pause(true)?; // the rules turn it on already; false keeps the players from pausing
            menu::battle_entrance(true)?; // the entrances and the 3-2-1 countdown
            menu::battle_ready_go(true)?; // "Ready" and "GO!"
            2
        }
        None => {
            // the boss round: player 1 alone, so a time battle (a stock one would end at once)
            menu::battle_stage(BOSS_STAGE)?;
            menu::battle_rules(menu::Rules { kind: menu::RuleKind::Time, stock: 0, seconds: 0, items: 0 })?;
            dionysus_api::boss_gauge_battle()?; // the game's boss HP bar, for the boss battle.rs brings in
            menu::battle_entrance(false)?; // the fighters start in place
            menu::battle_ready_go(false)?; // and fight at once
            1
        }
    };
    for slot in first_empty..PLAYERS {
        menu::battle_player(slot, menu::PlayerType::None, "", -1)?;
    }
    Ok(())
}

/// "template_menu.gauntlet" (controller): the first round.
pub unsafe extern "C" fn gauntlet(args: *const i64, count: usize) -> i64 {
    PAD.store(first_argument(args, count) as i32, Ordering::Release);
    ROUND.store(0, Ordering::Release);
    crate::battle::reset_team();
    let setup = (|| {
        round(0, TEAM[0], 0, 0)?;
        // a row of the pack's own in the pause menu, in place of Connect Controller:
        // a msg_melee label, and a msg_popup label asked instead of "End the game?"
        menu::pause_exit(Some(menu::PauseExit {
            row: "mel_pause_template_exit",
            confirm: Some("pop_pause_template_exit"),
        }))?;
        menu::keep_music_into_battle(true)?; // the modes page's song carries into round 1
        if let Some(path) = CARD_LAYOUT {
            dionysus_api::hud_layout_every(path)?; // on every player's damage card
        }
        if let Some(path) = ROUND_LAYOUT {
            // a name of your own, the layout, x and y (screen centre 0, 1920 x 1080), scale
            dionysus_api::hud_screen_layout("template_round", path, 0.0, 420.0, 1.0)?;
        }
        Ok(())
    })();
    code(setup)
}

/// "template_menu.boss" (controller): the game's own boss battle.
pub unsafe extern "C" fn boss(args: *const i64, count: usize) -> i64 {
    let pad = first_argument(args, count) as i32;
    let setup = (|| {
        menu::battle_player(0, menu::PlayerType::Human, TEAM[0], pad)?;
        for slot in 1..PLAYERS {
            menu::battle_player(slot, menu::PlayerType::None, "", -1)?;
        }
        menu::battle_stage("ui_stage_battle_field")?;
        menu::battle_rules(menu::Rules { kind: menu::RuleKind::Stock, stock: 2, seconds: 0, items: 0 })?;
        // the boss item kind; the game creates it, plays its intro and ends the battle on its defeat
        menu::battle_boss(Some(*ITEM_KIND_GALLEOM))?;
        dionysus_api::boss_gauge_hide(false)?; // true leaves its bar out, for a mod that draws its own
        menu::pause_exit(None)?; // the vanilla pause menu
        Ok(())
    })();
    code(setup)
}

/// "template_menu.how_it_ended": 1 Quit, 2 the pack's own pause row, 0 neither.
pub unsafe extern "C" fn how_it_ended(_args: *const i64, _count: usize) -> i64 {
    match menu::pause_choice() {
        Ok(menu::PauseChoice::Quit) => 1,
        Ok(menu::PauseChoice::Exit) => 2,
        _ => 0,
    }
}

/// "template_menu.movie": plays the intro full screen, with audio track 0.
pub unsafe extern "C" fn movie(_args: *const i64, _count: usize) -> i64 {
    code(menu::play_movie(INTRO_MOVIE, 0))
}

/// "template_menu.movie_playing": 1 while the movie loads or plays.
pub unsafe extern "C" fn movie_playing(_args: *const i64, _count: usize) -> i64 {
    menu::movie_playing().map_or(0, i64::from)
}

/// "template_menu.movie_stop": ends the movie early.
pub unsafe extern "C" fn movie_stop(_args: *const i64, _count: usize) -> i64 {
    menu::stop_movie().map_or(0, i64::from)
}
