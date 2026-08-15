use crate::smash;
use smash::{
    app::{lua_bind::*, sv_animcmd::*},
    lib::lua_const::*,
    lua2cpp::L2CAgentBase,
    phx::Hash40,
};
use smash_script::macros;

unsafe extern "C" fn game_attack11(agent: &mut L2CAgentBase) {
    frame(agent.lua_state_agent, 2.0);
    if macros::is_excute(agent) {
        macros::ATTACK(
            agent,
            0,
            0,
            Hash40::new("top"),
            4.0,
            361,
            35,
            0,
            25,
            4.0,
            0.0,
            7.0,
            6.0,
            Some(0.0),
            Some(7.0),
            Some(10.0),
            1.0,
            1.0,
            *ATTACK_SETOFF_KIND_ON,
            *ATTACK_LR_CHECK_F,
            false,
            0,
            0.0,
            0,
            false,
            false,
            false,
            false,
            true,
            *COLLISION_SITUATION_MASK_GA,
            *COLLISION_CATEGORY_MASK_ALL,
            *COLLISION_PART_MASK_ALL,
            false,
            Hash40::new("collision_attr_normal"),
            *ATTACK_SOUND_LEVEL_S,
            *COLLISION_SOUND_ATTR_PUNCH,
            *ATTACK_REGION_PUNCH,
        );
    }
    wait(agent.lua_state_agent, 2.0);
    if macros::is_excute(agent) {
        AttackModule::clear_all(agent.module_accessor);
    }
}

unsafe extern "C" fn effect_attack11(agent: &mut L2CAgentBase) {
    frame(agent.lua_state_agent, 2.0);
    if macros::is_excute(agent) {
        macros::EFFECT_FOLLOW(
            agent,
            Hash40::new("sys_attack_line"),
            Hash40::new("top"),
            0.0,
            7.0,
            2.0,
            0.0,
            0.0,
            0.0,
            0.8,
            true,
        );
        macros::LAST_PARTICLE_SET_COLOR(agent, 0.2, 0.8, 1.0);
    }
}

unsafe extern "C" fn sound_attack11(agent: &mut L2CAgentBase) {
    frame(agent.lua_state_agent, 2.0);
    if macros::is_excute(agent) {
        macros::PLAY_SE(agent, Hash40::new("se_common_swing_02"));
    }
}

unsafe extern "C" fn expression_attack11(agent: &mut L2CAgentBase) {
    frame(agent.lua_state_agent, 2.0);
    if macros::is_excute(agent) {
        ControlModule::set_rumble(
            agent.module_accessor,
            Hash40::new("rbkind_attacks"),
            0,
            false,
            *BATTLE_OBJECT_ID_INVALID as u32,
        );
    }
}

unsafe extern "C" fn game_appeallw(agent: &mut L2CAgentBase) {
    frame(agent.lua_state_agent, 12.0);
    if macros::is_excute(agent) {
        crate::articles::spawn(agent.module_accessor);
    }
}

pub fn install() {
    smashline::Agent::new(crate::RESOURCE_NAME)
        .game_acmd("game_attack11", game_attack11, smashline::Priority::Default)
        .effect_acmd(
            "effect_attack11",
            effect_attack11,
            smashline::Priority::Default,
        )
        .sound_acmd(
            "sound_attack11",
            sound_attack11,
            smashline::Priority::Default,
        )
        .expression_acmd(
            "expression_attack11",
            expression_attack11,
            smashline::Priority::Default,
        )
        .game_acmd("game_appeallw", game_appeallw, smashline::Priority::Default)
        .install();
}
