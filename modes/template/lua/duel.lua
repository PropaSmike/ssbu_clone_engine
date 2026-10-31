-- A mode that starts a battle straight from the modes page: menu.toml gives
-- it runs_as and scene, and decide sets the battle up just before it starts.
local duel = {}

local MARIO = "ui_chara_mario"
local LUIGI = "ui_chara_luigi"
local BATTLEFIELD = "ui_stage_battle_field"
local HUMAN = "human"
local CPU = "cpu"
local NONE = "none"
local STOCK = "stock"
local PLAYERS = 8

-- Every slot keeps what the last battle left in it: the ones the duel does
-- not use are turned off.
function duel.decide(host)
  prg_func.engine_call("engine.battle_player", 0, HUMAN, MARIO, host:pad())
  prg_func.engine_call("engine.battle_player", 1, CPU, LUIGI, -1)
  for slot = 2, PLAYERS - 1 do
    prg_func.engine_call("engine.battle_player", slot, NONE, 0, -1)
  end
  prg_func.engine_call("engine.battle_stage", BATTLEFIELD)
  prg_func.engine_call("engine.battle_rules", STOCK, 2, 0, 0)
  return true
end

return duel
