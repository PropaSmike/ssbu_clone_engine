-- Every main menu page runs common.update_state_main each frame in its main
-- state: the modes page looks at the modes button there first.
local common = UiScriptPlayer.require("menu/main/main_menu_common")
local modes = UiScriptPlayer.require("menu/clone_engine/ce_modes")
local update_state_main = common.update_state_main

function common.update_state_main(layer)
  if modes ~= nil and modes:take_over(layer) then
    return
  end
  update_state_main(layer)
end

return common
