-- A mode that runs as a screen inside the modes page: enter shows its own
-- layout (a [[layout]] of the pack), update reads the host's keys each frame,
-- leave hides it again. The number survives a trip into a battle through
-- host:value / host:set_value.
local page = {}

local MAX = 99

local function draw(host)
  page.view:get_pane("value"):set_text_string(tostring(host:value()))
end

function page.enter(host, resumed)
  local root = LayoutRootList.example_page
  if root == nil then
    host:close()
    return
  end
  page.view = root:get_root_view()
  page.pane = page.view:get_pane("page")
  page.pane:set_visible(true)
  page.view:get_pane("title"):set_text_string(host.name)
  page.view:get_pane("help"):set_text_string("Up / Down  Change      B  Back")
  draw(host)
end

function page.update(host, keys)
  if keys.cancel then
    UiSoundManager:play_se_label("se_system_cancel")
    host:close()
    return
  end
  local value = host:value()
  if keys.up and value < MAX then
    UiSoundManager:play_se_label("se_system_cursor_l")
    host:set_value(value + 1)
    draw(host)
  elseif keys.down and value > 0 then
    UiSoundManager:play_se_label("se_system_cursor_l")
    host:set_value(value - 1)
    draw(host)
  end
end

function page.leave(host)
  if page.pane ~= nil then
    page.pane:set_visible(false)
  end
end

return page
