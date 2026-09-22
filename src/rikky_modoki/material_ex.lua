return function(rikky_module, module, draw, material)
  local common = require("rikky_modoki.common")
  local finite_number = common.finite_number
  local clamp = common.clamp
  local glass_fields = draw.glass_fields
  local glass_integer = draw.glass_integer
  local shader_script = draw.shader_script
  local gpu_buffer = draw.gpu_buffer
  local gpu_copy = draw.gpu_copy
  local glass_release_state = draw.glass_release_state
  local capture_draw_state = draw.capture_draw_state
  local draw_arguments = draw.draw_arguments
  local draw_processed_image = draw.draw_processed_image
  local gpu_material = material.gpu_material

  local function material_ex_number(values, key, default)
    if type(values) == "table" and type(values[key]) == "number" then
      return finite_number(values[key])
    end
    return default
  end

  local function material_ex_layer(value)
    local layer
    if type(value) == "number" then
      layer = glass_integer(value, 0)
    elseif type(value) == "string" then
      local offset = tonumber(value:match("^@([+-]?%d+)$"))
      assert(offset ~= nil, "Invalid materialdrawEx layer: " .. value)
      layer = obj.layer + offset
    end
    assert(obj.getvalue("layer" .. layer), "materialdrawEx: layer is empty")
    return layer
  end

  local function material_ex_texture(state, option, light)
    local source, layer = option.texture, nil
    if type(source) ~= "string" or source:sub(1, 1) ~= "*" then
      layer = material_ex_layer(source)
      light.alpha = clamp(obj.getvalue("layer" .. layer .. ".alpha"), 0, 1)
    end
    local saved = {}
    for _, field in ipairs(glass_fields) do saved[field] = obj[field] end
    local texture = gpu_buffer()
    local ok, err = pcall(function()
      if layer ~= nil then
        assert(obj.load("layer", layer, true), "materialdrawEx: failed to load layer texture")
      else
        assert(obj.load("image", source:sub(2)), "materialdrawEx: failed to load image texture")
      end
      if light.width > 0 or light.height > 0 then
        local width, height = obj.w, obj.h
        if light.width > 0 then width = light.width end
        if light.height > 0 then height = light.height end
        obj.effect("リサイズ", "X", width / obj.w * 100, "Y", height / obj.h * 100)
      end
      light.width, light.height = obj.w, obj.h
      local width, height = math.ceil(light.width / light.partition), math.ceil(light.height / light.partition)
      obj.clearbuffer(texture, width, height)
      obj.computeshader("rikky_material_reduce" .. shader_script, { texture }, { "object" },
        { light.width, light.height, light.partition, light.alpha }, math.ceil(width / 8), math.ceil(height / 8), 1)
    end)
    local restore_ok, restore_err = pcall(gpu_copy, "object", state.original)
    local fields_ok, fields_err = pcall(function()
      for _, field in ipairs(glass_fields) do obj[field] = saved[field] end
    end)
    if restore_ok and not fields_ok then restore_ok, restore_err = fields_ok, fields_err end
    if not restore_ok then
      if not ok then error(tostring(err) .. "\nImage restoration failed: " .. tostring(restore_err), 0) end
      error(restore_err, 0)
    end
    if not ok then error(err, 0) end
    return texture
  end

  local function material_ex_light(state, input)
    assert(type(input) == "table", "materialdrawEx: each light must be a table")
    local position, option = input.position, input.option
    if type(option) ~= "table" then option = {} end
    local light = {
      kind = 0,
      double = false,
      layer_rotation = false,
      rx = 0,
      ry = 0,
      rz = 0,
      alpha = 1,
      shininess = math.max(0, material_ex_number(input.specular, "shininess", 1000) / 100),
      width = math.max(0, glass_integer(option.width, 0)),
      height = math.max(0, glass_integer(option.height, 0)),
      partition = math.max(1, glass_integer(option.partition, 2)),
    }
    local layer
    if type(position) == "table" then
      for _, axis in ipairs({ "x", "y", "z" }) do
        light[axis] = material_ex_number(position, axis, 0)
      end
    elseif position == "camera" then
      light.x, light.y, light.z = state.camera.x, state.camera.y, state.camera.z
    else
      assert(position ~= "shadow", "materialdrawEx: shadow light positions are unavailable in AviUtl2")
      layer = material_ex_layer(position)
      for _, axis in ipairs({ "x", "y", "z" }) do
        light[axis] = obj.getvalue("layer" .. layer .. "." .. axis)
        light["r" .. axis] = obj.getvalue("layer" .. layer .. ".r" .. axis)
      end
      light.layer_rotation = true
    end
    for _, channel in ipairs({ "R", "G", "B" }) do
      light[channel:lower()] = clamp(material_ex_number(input.color, channel, 255) / 255, 0, 1)
      light["specular_" .. channel:lower()] = math.max(0, material_ex_number(input.specular, channel, 100) / 100)
    end
    if option.type == "spotlight" then light.kind = 1 end
    if option.type == "directlight" then light.kind = 2 end
    local default_degree, default_z = 45, 1
    if light.kind == 2 then default_degree, default_z = 10, -1 end
    light.degree = clamp(material_ex_number(option, "degree", default_degree), 0, 90)
    light.degree2 = clamp(material_ex_number(option, "degree2", light.degree), 0, 90)
    light.nx, light.ny = material_ex_number(option, "nx", 0), material_ex_number(option, "ny", 0)
    light.nz = material_ex_number(option, "nz", default_z)
    light.wx, light.wy, light.wz = material_ex_number(option, "wx", 1),
        material_ex_number(option, "wy", 0), material_ex_number(option, "wz", 0)
    if light.kind == 1 then
      light.double = option.double == true
      for _, axis in ipairs({ "x", "y", "z" }) do
        local key = "n" .. axis .. "2"
        light[key] = material_ex_number(option, key, nil)
      end
    elseif light.kind == 2 then
      light.double = option.double == true
    end
    if light.kind == 2 and option.texture ~= "color" then
      local texture = material_ex_texture(state, option, light)
      state.lights[#state.lights + 1] = { constants = module.material_light_constants(light, true), texture = texture }
    else
      state.lights[#state.lights + 1] = { constants = module.material_light_constants(light, false) }
    end
  end

  local material_ex_id = 0
  function rikky_module.materialdrawEx(input)
    assert(type(input) == "table", "materialdrawEx expects a settings table")
    local state = { blur = 0, lights = {} }
    local ok, err = pcall(function()
      capture_draw_state(state)
      state.partition = math.max(1, glass_integer(input.drawhq_partition, 1))
      state.hq = input.drawhq == true and type(input.drawhq_partition) == "number"
      state.damping = math.max(0, material_ex_number(input, "damping", 0))
      state.base = {}
      for _, channel in ipairs({ "R", "G", "B" }) do
        state.base[#state.base + 1] = clamp(material_ex_number(input.ambient, channel, 0) / 255, 0, 1)
            + clamp(material_ex_number(input.emissive, channel, 10) / 100, 0, 1)
      end
      for i = 4, 8 do state.base[i] = 0 end
      if input.light ~= nil then
        assert(type(input.light) == "table", "materialdrawEx: light must be an array")
        for _, light in ipairs(input.light) do material_ex_light(state, light) end
      end
    end)
    if not ok then
      glass_release_state(state); error(err, 0)
    end
    material_ex_id = material_ex_id + 1
    local result = { id = material_ex_id }
    local function draw(self, polygon, ...)
      assert(self == result, "Call materialdrawEx methods with ':'")
      local count, args = draw_arguments(...)
      assert((polygon and count > 8) or (not polygon and count <= 8), "Invalid materialdrawEx method arguments")
      draw_processed_image(state, count, args, function() return gpu_material(state, count, args) end)
    end
    function result:draw(...) return draw(self, false, ...) end

    function result:drawpoly(...) return draw(self, true, ...) end

    return result
  end
end
