-- A mode without Smash's rules page and stage select (menu.toml skip): the
-- players pick fighters, then fight on the stage and rules set here. No
-- plugin is needed for this one.
local pick = {}

local BATTLEFIELD = "ui_stage_battle_field"
local STOCK = "stock"
local STOCKS = 2
local SECONDS = 120
local NO_ITEMS = 0

function pick.decide(host)
  prg_func.engine_call("engine.battle_stage", BATTLEFIELD)
  -- rule ("time", "stock" or "stamina"), stocks, time limit, item frequency
  prg_func.engine_call("engine.battle_rules", STOCK, STOCKS, SECONDS, NO_ITEMS)
  return true
end

return pick
