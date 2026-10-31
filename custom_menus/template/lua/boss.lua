-- The game's own boss battle: the plugin picks the player, the stage and the
-- boss (src/setup.rs, boss); the game does the rest.
local boss = {}

function boss.decide(host)
  local result = prg_func.engine_call("template_menu.boss", host:pad())
  return not (type(result) == "number" and result < 0)
end

return boss
