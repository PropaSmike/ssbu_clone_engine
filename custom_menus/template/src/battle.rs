//! The battles, run from player 1's frame: the gauntlet's rounds, its team
//! swaps and its boss, and the boss mode's boss.

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use crate::setup::{self, BOSS_SONG, CARD_LAYOUT, ROUNDS, ROUND_LAYOUT, TEAM};
use crate::smash::app::lua_bind::*;
use crate::smash::app::{sv_battle_object, BattleObjectModuleAccessor};
use crate::smash::lib::lua_const::*;
use crate::smash::lua2cpp::L2CFighterCommon;
use clone_engine_api::{item, menu, HudAction};

/// The battle frame the gauntlet's boss arrives at.
const BOSS_FRAME: u64 = 60;
/// A team of its own fights everyone.
const BOSS_TEAM: i32 = 7;
/// Frames a swap takes to settle: the stock count reads low while the new
/// fighter comes in, which must not count as another KO.
const SWAP_SETTLE_FRAMES: u64 = 180;
const NONE: u32 = u32::MAX;

static SERIAL: AtomicU32 = AtomicU32::new(NONE);
static TEAM_INDEX: AtomicU32 = AtomicU32::new(0);
static STOCKS: AtomicU32 = AtomicU32::new(0);
static CPU_STOCKS: AtomicU32 = AtomicU32::new(0);
static SETTLED_AT: AtomicU64 = AtomicU64::new(0);
static ENDING: AtomicBool = AtomicBool::new(false);
static BOSS: AtomicU32 = AtomicU32::new(NONE);
static BOSS_ROW: AtomicU32 = AtomicU32::new(NONE);
static ANGRY: AtomicBool = AtomicBool::new(false);
static BOSS_SEEN: AtomicBool = AtomicBool::new(false);
static BOSS_DONE: AtomicBool = AtomicBool::new(false);

pub fn reset_team() {
    TEAM_INDEX.store(0, Ordering::Release);
}

/// Once per frame for every fighter (lib.rs installs it).
pub unsafe extern "C" fn frame(fighter: &mut L2CFighterCommon) {
    let boma = fighter.module_accessor;
    if WorkModule::get_int(boma, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) != 0 {
        return;
    }
    // the code that started this battle's mode, and the codes the engine gave ours
    let Some(active) = menu::active_sequence() else {
        return;
    };
    // changes each time a battle starts: per-battle setup keys on it
    let Ok(serial) = menu::battle_serial() else {
        return;
    };
    let new_battle = SERIAL.swap(serial, Ordering::AcqRel) != serial;
    if Some(active) == menu::mode_sequence("template_gauntlet") {
        gauntlet(boma, new_battle);
    } else if Some(active) == menu::mode_sequence("template_boss") {
        boss_mode(new_battle);
    }
}

unsafe fn gauntlet(boma: *mut BattleObjectModuleAccessor, new_battle: bool) {
    if new_battle {
        ENDING.store(false, Ordering::Release);
        SETTLED_AT.store(0, Ordering::Release);
        BOSS.store(NONE, Ordering::Release);
        BOSS_ROW.store(NONE, Ordering::Release);
        ANGRY.store(false, Ordering::Release);
        STOCKS.store(menu::battle_stock(0).unwrap_or(0), Ordering::Release); // slot: stocks left
        CPU_STOCKS.store(menu::battle_stock(1).unwrap_or(0), Ordering::Release);
    }
    if ENDING.load(Ordering::Acquire) {
        return;
    }
    let round = setup::ROUND.load(Ordering::Acquire);
    let Ok(frame) = menu::battle_frame() else {
        return;
    };
    if frame == 1 {
        if ROUND_LAYOUT.is_some() {
            let title = format!("Round {}", round + 1);
            // the screen layout's name (setup.rs), then a text pane of it and its text
            let _ = clone_engine_api::hud_screen("template_round", &HudAction::Text("round", &title));
        }
        if round as usize == ROUNDS.len() {
            let _ = menu::play_bgm(BOSS_SONG); // a ui_bgm_db label
        }
    }

    // player 1 lost a stock: the next team member comes in at the rebirth
    let stocks = menu::battle_stock(0).unwrap_or(0);
    let lost = stocks < STOCKS.swap(stocks, Ordering::AcqRel);
    if lost && frame >= SETTLED_AT.load(Ordering::Acquire) {
        SETTLED_AT.store(frame + SWAP_SETTLE_FRAMES, Ordering::Release);
        let member = (TEAM_INDEX.fetch_add(1, Ordering::AcqRel) as usize + 1) % TEAM.len();
        let _ = menu::change_fighter(0, TEAM[member], 0); // slot, fighter, costume
        if CARD_LAYOUT.is_some() {
            // player 1's card layout (setup.rs): show its pane "swap"
            let _ = clone_engine_api::hud_entry(0, &HudAction::Show("swap", true));
        }
    }

    if (round as usize) < ROUNDS.len() {
        // the CPU lost a stock: the next round, player 1's damage carried over
        let cpu = menu::battle_stock(1).unwrap_or(0);
        if cpu < CPU_STOCKS.swap(cpu, Ordering::AcqRel) {
            let damage = DamageModule::damage(boma, 0) as u32;
            let next = round + 1;
            let member = TEAM_INDEX.load(Ordering::Acquire) as usize % TEAM.len();
            setup::ROUND.store(next, Ordering::Release);
            if setup::round(next, TEAM[member], 0, damage).is_ok() {
                ENDING.store(true, Ordering::Release);
                let _ = menu::next_battle(); // ends this battle and loads the next one
            }
        }
        return;
    }
    boss_round(frame);
}

/// The gauntlet's last round: the plugin brings Galleom in itself and puts it
/// on the boss bar setup.rs asked for.
unsafe fn boss_round(frame: u64) {
    let boss = BOSS.load(Ordering::Acquire);
    if boss == NONE {
        if frame < BOSS_FRAME {
            return;
        }
        // kind, x, y, z, facing (-1.0 left), variation; its battle object id
        let Ok(id) = item::spawn(*ITEM_KIND_GALLEOM, 0.0, 40.0, 0.0, -1.0, 0) else {
            ENDING.store(true, Ordering::Release);
            return;
        };
        BOSS.store(id, Ordering::Release);
        let boss = sv_battle_object::module_accessor(id);
        TeamModule::set_team(boss, BOSS_TEAM, true); // a new item has no team
        // a boss made this way waits, unseen, until it is sent to its wait status
        if let Some(status) = item::alias_status(boss as usize, "wait") {
            StatusModule::change_status_request(boss, status, false);
        }
        // the item and the name its row shows; the row number comes back
        if let Ok(row) = clone_engine_api::boss_gauge(boss as *const core::ffi::c_void, "Galleom") {
            BOSS_ROW.store(row, Ordering::Release);
        }
        return;
    }
    let defeated = !sv_battle_object::is_active(boss) || {
        let item = sv_battle_object::module_accessor(boss);
        let hp = WorkModule::get_float(item, *ITEM_INSTANCE_WORK_FLOAT_HP);
        let max = WorkModule::get_float(item, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
        let row = BOSS_ROW.load(Ordering::Acquire);
        if row != NONE && hp < max / 2.0 && !ANGRY.swap(true, Ordering::AcqRel) {
            let _ = clone_engine_api::boss_gauge_name(row, "Galleom (angry)"); // the row, its new name
        }
        hp <= 0.0
    };
    if defeated {
        ENDING.store(true, Ordering::Release);
        let _ = menu::stop_bgm();
        let _ = menu::end_battle(); // ends the battle and returns to the main menu
    }
}

/// The game's own boss battle: it creates the boss and ends the battle on its
/// defeat; the plugin only watches.
fn boss_mode(new_battle: bool) {
    if new_battle {
        BOSS_SEEN.store(false, Ordering::Release);
        BOSS_DONE.store(false, Ordering::Release);
    }
    if !BOSS_SEEN.load(Ordering::Acquire) {
        if let Some(id) = menu::battle_boss_object() {
            BOSS_SEEN.store(true, Ordering::Release);
            println!("[template_menu] the boss is battle object {id:#x}");
        }
    }
    if !BOSS_DONE.load(Ordering::Acquire) && menu::battle_boss_finished() {
        BOSS_DONE.store(true, Ordering::Release);
        println!("[template_menu] the boss is defeated");
    }
}
