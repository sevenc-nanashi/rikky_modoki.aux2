return function(rikky_module, module, draw, material)
  local common = require("rikky_modoki.common")
  local finite_number = common.finite_number
  local clamp = common.clamp
  local glass_integer = draw.glass_integer
  local glass_release_state = draw.glass_release_state
  local capture_draw_state = draw.capture_draw_state
  local draw_arguments = draw.draw_arguments
  local draw_processed_image = draw.draw_processed_image
  local gpu_material = material.gpu_material
  local gpu_point_light = material.gpu_point_light

  local material_states = {}

  local function material_defaults()
    local settings = {
      light_count = 0,
      lights = {},
      shininess = 0,
      ambient_r = 0,
      ambient_g = 0,
      ambient_b = 0,
      specular_r = 0,
      specular_g = 0,
      specular_b = 0,
      emissive_r = 0,
      emissive_g = 0,
      emissive_b = 0,
    }
    for i = 1, 4 do
      settings.lights[i] = { position = { 0, 0, 0 }, color = { 0, 0, 0 }, source = 0 }
    end
    return settings
  end

  local function material_component(values, key, index)
    if type(values) ~= "table" then return nil end
    local value = values[key]
    -- 数値キーの成分指定は元DLLの公開仕様。数値文字列は設定値として扱わない。
    if type(value) ~= "number" then value = values[index] end
    if type(value) ~= "number" then return nil end
    return finite_number(value)
  end

  local function material_update(previous, input)
    local settings = material_defaults()
    for key, value in pairs(previous) do
      if key ~= "lights" then settings[key] = value end
    end
    for i, light in ipairs(previous.lights) do
      settings.lights[i] = { position = { unpack(light.position) }, color = { unpack(light.color) }, source = light.source }
    end
    if type(input) ~= "table" then return settings end
    if type(input.light_num) == "number" then
      settings.light_count = clamp(glass_integer(input.light_num, 0), 0, 4)
      for i = 1, settings.light_count do
        local light, position, color = settings.lights[i], input["position" .. i], input["light" .. i]
        for axis, key in ipairs({ "x", "y", "z" }) do
          local value = material_component(position, key, axis)
          if value ~= nil then light.position[axis] = value end
        end
        if type(position) == "table" then
          local source = position.object
          if type(source) ~= "string" then source = position[4] end
          if type(source) ~= "string" or source == "" then
            light.source = 0
          elseif source == "camera" then
            light.source = "camera"
          else
            assert(source ~= "shadow", "materialdraw: shadow light positions are unavailable in AviUtl2")
            local layer = tonumber(source:match("^L([1-9]%d*)$"))
            assert(layer ~= nil and layer <= 100, "Invalid material light source: " .. source)
            light.source = layer
          end
        end
        for channel, key in ipairs({ "R", "G", "B" }) do
          local value = material_component(color, key, channel)
          if value ~= nil then light.color[channel] = clamp(value / 255, 0, 1) end
        end
      end
    end
    for _, name in ipairs({ "ambient", "specular", "emissive" }) do
      for channel, key in ipairs({ "R", "G", "B" }) do
        local value = material_component(input[name], key, channel)
        if value ~= nil then
          if name == "ambient" then
            value = clamp(value / 100, 0, 1)
          elseif name == "emissive" then
            value = clamp(glass_integer(value, 0), 0, 255)
          else
            -- 元DLLのspecular.Rだけ上限判定を誤る不具合は再現しない。
            value = clamp(value, 0, 255)
          end
          settings[name .. "_" .. key:lower()] = value
        end
      end
    end
    if type(input.specular) == "table" and type(input.specular.shininess) == "number" then
      settings.shininess = math.max(0, finite_number(input.specular.shininess) / 100)
    end
    return settings
  end

  function rikky_module.materialdraw_init(input)
    local id, previous = obj.effect_id, material_states[obj.effect_id]
    local settings
    if previous == nil then settings = material_defaults() else settings = previous.settings end
    glass_release_state(previous)
    -- init失敗時に前フレームの画像で描画しない。設定のみ保持する。
    material_states[id] = { settings = settings }
    if type(input) == "number" and glass_integer(input, 0) == 0 then
      material_states[id].settings = material_defaults()
      return true
    end
    local state = { settings = material_update(settings, input), blur = 0, lights = {}, damping = 0, hq = false }
    local ok, err = pcall(function()
      capture_draw_state(state)
      local s = state.settings
      state.base = { s.ambient_r, s.ambient_g, s.ambient_b, 0,
        s.emissive_r / 255, s.emissive_g / 255, s.emissive_b / 255, 0 }
      for i = 1, state.settings.light_count do
        local light = state.settings.lights[i]
        if light.source == "camera" then
          light.position = { state.camera.x, state.camera.y, state.camera.z }
        elseif light.source ~= 0 then
          local layer = "layer" .. light.source
          -- 指定レイヤーが空なら座標指定を使うのが元APIの仕様。
          if obj.getvalue(layer) then
            light.position = module.material_layer_position({
              obj.getvalue(layer .. ".x"), obj.getvalue(layer .. ".y"), obj.getvalue(layer .. ".z"),
            }, state.groups)
          end
        end
        state.lights[#state.lights + 1] = gpu_point_light(light.position, light.color,
          { s.specular_r / 255, s.specular_g / 255, s.specular_b / 255 }, s.shininess)
      end
    end)
    if not ok then
      glass_release_state(state)
      error(err, 0)
    end
    material_states[id] = state
    return true
  end

  function rikky_module.materialdraw(...)
    local state = material_states[obj.effect_id]
    assert(state ~= nil and state.original ~= nil, "Call materialdraw_init before materialdraw")
    local count, args = draw_arguments(...)
    draw_processed_image(state, count, args, function() return gpu_material(state, count, args) end)
  end

  -- Exの既定値は旧materialdrawと異なり、RGBは名前付き成分のみを読む。
end
