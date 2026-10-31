-- A mode that runs as a screen inside the modes page (it has enter): it plays
-- the pack's movie through the plugin and closes when the movie ends or B is
-- pressed.
local intro = {}

function intro.enter(host, resumed)
  local result = prg_func.engine_call("template_menu.movie")
  if type(result) == "number" and result < 0 then
    host:close()
  end
end

function intro.update(host, keys)
  if keys.cancel then
    prg_func.engine_call("template_menu.movie_stop")
  end
  if prg_func.engine_call("template_menu.movie_playing") ~= 1 then
    host:close()
  end
end

function intro.leave(host)
  prg_func.engine_call("template_menu.movie_stop")
end

return intro
