-- The gauntlet: the plugin sets the whole battle up (src/setup.rs) from the
-- controller that chose the mode, and runs the rounds (src/battle.rs).
local gauntlet = {}

local function refused(result)
  return type(result) == "number" and result < 0
end

function gauntlet.decide(host)
  -- how the last battle ended: 1 Quit, 2 the pack's own pause row, 0 neither
  prg_func.engine_call("engine.log", prg_func.engine_call("template_menu.how_it_ended"))
  -- a negative code keeps the page open instead of starting a broken battle
  return not refused(prg_func.engine_call("template_menu.gauntlet", host:pad()))
end

return gauntlet
