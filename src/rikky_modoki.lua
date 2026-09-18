--script:lua
--label:@rikky_modoki.aux2

local module = obj.module("rikky_modoki")
if module.is_development() then
  package.loaded["rikky_module"] = nil
end

require("rikky_module")
