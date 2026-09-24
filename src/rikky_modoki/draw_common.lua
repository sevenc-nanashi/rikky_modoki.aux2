return function(module)
  local common = require("rikky_modoki.common")
  local finite_number = common.finite_number

  local glass_fields = {
    "ox", "oy", "oz", "cx", "cy", "cz", "rx", "ry", "rz", "sx", "sy", "sz", "alpha"
  }

  local function glass_number(value, default)
    local number = tonumber(value)
    if number == nil then return default end -- 元DLLのlua_tonumberと既定値
    return finite_number(number)
  end

  local function glass_integer(value, default)
    local number = glass_number(value, default)
    if number < 0 then return math.ceil(number) end
    return math.floor(number)
  end

  local shader_script = "@初期化@rikky_modoki.aux2"
  local function gpu_buffer()
    return "cache:rikky_draw_" .. module.counter()
  end

  local function gpu_copy(destination, source)
    assert(obj.copybuffer(destination, source),
      "Drawing buffer unavailable; initialize the material again in this frame: " .. source)
  end

  local function glass_capture(target)
    local buffer = gpu_buffer()
    gpu_copy(buffer, target)
    return buffer
  end

  local function gpu_constants(geometry, extra)
    local values = {}
    for _, value in ipairs(geometry) do values[#values + 1] = value end
    for _, value in ipairs(extra) do
      assert(math.abs(value) <= 3.402823466e38, "Drawing constant exceeds GPU float range")
      values[#values + 1] = value
    end
    for i = #values + 1, 108 do values[i] = 0 end
    return values
  end

  local function gpu_geometry(state, args)
    return module.draw_constants(state.width, state.height, state.pose, state.camera, args, state.groups)
  end

  local function glass_groups()
    local groups = {}
    if obj.getoption("drawtarget") == "tempbuffer" or not obj.getoption("enable_group") then
      return groups
    end
    local index, previous = 0, nil
    while true do
      local layer = obj.getoption("group_info", index)
      if layer == 0 then break end
      -- index未対応のホストで無限に同じレイヤーを取得しない。
      assert(layer ~= previous, "glassdraw requires AviUtl2 2.1.10 or later")
      for _, field in ipairs({ "x", "y", "z", "cx", "cy", "cz", "rx", "ry", "rz", "sx", "sy", "sz" }) do
        groups[#groups + 1] = obj.getvalue("layer" .. layer .. "." .. field)
      end
      index, previous = index + 1, layer
    end
    return groups
  end

  local function glass_release_state(state)
    -- cache: buffers are owned by AviUtl2 and expire after the frame is rendered.
    if state ~= nil then state.original, state.background = nil, nil end
  end

  local function capture_draw_state(state)
    state.pose = { x = obj.x, y = obj.y, z = obj.z, billboard = obj.getoption("billboard") }
    for _, field in ipairs(glass_fields) do state.pose[field] = obj[field] end
    state.pose.base_sx, state.pose.base_sy, state.pose.base_sz = obj.getvalue("scale")
    state.camera = obj.getoption("camera_param")
    state.camera.mode = obj.getoption("camera_mode")
    state.groups = glass_groups()
    state.width, state.height = obj.w, obj.h
    state.original = glass_capture("object")
  end

  local function draw_arguments(...)
    local count, args = select("#", ...), { ... }
    assert(count <= 8 or count == 12 or count == 20 or count == 21, "Invalid draw argument count")
    for i = 1, count do args[i] = finite_number(tonumber(args[i])) end
    return count, args
  end

  local function draw_processed_image(state, count, args, render)
    local saved = {}
    for _, field in ipairs(glass_fields) do saved[field] = obj[field] end
    local function restore_fields()
      for _, field in ipairs(glass_fields) do obj[field] = saved[field] end
    end
    local ok, err = pcall(function()
      gpu_copy("object", state.original)
      restore_fields()
      if not render() then
        obj.setoption("draw_state", true)
        return
      end
      if state.blur > 0 then obj.effect("ぼかし", "範囲", state.blur, "サイズ固定", 1) end
      if count > 8 then
        obj.drawpoly(unpack(args, 1, count))
      else
        obj.draw(unpack(args, 1, count))
      end
    end)
    local restore_ok, restore_err = pcall(gpu_copy, "object", state.original)
    local fields_ok, fields_err = pcall(restore_fields)
    if restore_ok and not fields_ok then restore_ok, restore_err = fields_ok, fields_err end
    if not restore_ok then
      if not ok then error(tostring(err) .. "\nImage restoration failed: " .. tostring(restore_err), 0) end
      error(restore_err, 0)
    end
    if not ok then error(err, 0) end
  end

  return {
    glass_fields = glass_fields,
    glass_number = glass_number,
    glass_integer = glass_integer,
    shader_script = shader_script,
    gpu_buffer = gpu_buffer,
    gpu_copy = gpu_copy,
    glass_capture = glass_capture,
    gpu_constants = gpu_constants,
    gpu_geometry = gpu_geometry,
    glass_groups = glass_groups,
    glass_release_state = glass_release_state,
    capture_draw_state = capture_draw_state,
    draw_arguments = draw_arguments,
    draw_processed_image = draw_processed_image,
  }
end
