-- A [[patch]] (menu.toml, commented out there): runs in place of the game's
-- main_menu_layer_other and points its VR button at the template_training
-- [[sequence]], which starts Training. Requiring the module here gives the
-- game's own, or the next pack's patch.
local other = UiScriptPlayer.require("menu/main/main_menu_layer_other")
local initialize = other.initialize

function other:initialize()
  initialize(self)
  self.button_table[button.OTHER_VR].sequence = prg_func.engine_call("engine.sequence", "template_training")
end

return other
