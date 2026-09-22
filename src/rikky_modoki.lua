--script:lua
--label:@rikky_modoki.aux2

--[[pixelshader@rikky_glass:
---$include "shader/glass.hlsl"
]]
--[[pixelshader@rikky_material_base:
---$include "shader/material_base.hlsl"
]]
--[[computeshader@rikky_material_light:
---$include "shader/material_light.hlsl"
]]
--[[pixelshader@rikky_material_apply:
---$include "shader/material_apply.hlsl"
]]
--[[computeshader@rikky_material_reduce:
---$include "shader/material_reduce.hlsl"
]]

local module = obj.module("rikky_modoki")
if module.is_development() then
  package.loaded["rikky_module"] = nil
end

require("rikky_module")
