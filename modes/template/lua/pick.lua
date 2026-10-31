-- A mode that runs Smash without its rules page and stage select (menu.toml
-- skip): the players pick fighters on the fighter select, then fight on the
-- stage and rules set here; after the battle the menu comes back to this
-- mode's row (after_battle = "menu").
local pick = {}

local BATTLEFIELD = "ui_stage_battle_field"
local STOCK = "stock"
local STOCKS = 2
local SECONDS = 120
local NO_ITEMS = 0

function pick.decide(host)
  prg_func.engine_call("engine.battle_stage", BATTLEFIELD)
  prg_func.engine_call("engine.battle_rules", STOCK, STOCKS, SECONDS, NO_ITEMS)
  return true
end

return pick
