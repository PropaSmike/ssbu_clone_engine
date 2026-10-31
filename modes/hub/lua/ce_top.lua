-- The top page initializes while the main menu loads: the modes page sets
-- itself up there (its states, and every mode's script loaded while
-- scripts still load at once).
local top = UiScriptPlayer.require("menu/main/main_menu_layer_top")
local modes = UiScriptPlayer.require("menu/clone_engine/ce_modes")
local initialize = top.initialize

function top:initialize()
  initialize(self)
  if modes ~= nil then
    modes:setup()
  end
end

return top
