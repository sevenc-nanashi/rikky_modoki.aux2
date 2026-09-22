return function(draw)
  local shader_script = draw.shader_script
  local gpu_constants = draw.gpu_constants
  local gpu_geometry = draw.gpu_geometry

  local function gpu_material(state, count, args)
    local geometry = gpu_geometry(state, args)
    obj.pixelshader("rikky_material_base" .. shader_script, "object", { state.original },
      gpu_constants(geometry, state.base), "copy")
    if #state.lights == 0 then return true end
    local partition = 0
    if state.hq and count <= 8 then partition = state.partition end
    local width, height = 1, 1
    if partition > 0 then
      width, height = math.ceil(state.width / partition), math.ceil(state.height / partition)
    end
    local diffuse, specular = state.original .. "_diffuse", state.original .. "_specular"
    obj.clearbuffer(diffuse, width, height)
    obj.clearbuffer(specular, width, height)
    for _, light in ipairs(state.lights) do
      local constants = {}
      for _, value in ipairs(light.constants) do constants[#constants + 1] = value end
      for _, value in ipairs({ state.damping, partition, state.width, state.height }) do constants[#constants + 1] = value end
      -- The untextured shader does not sample t0; bind the owned original explicitly.
      local texture = state.original
      if light.texture ~= nil then texture = light.texture end
      obj.computeshader("rikky_material_light" .. shader_script, { diffuse, specular }, { texture },
        gpu_constants(geometry, constants), math.ceil(width / 8), math.ceil(height / 8), 1)
      obj.pixelshader("rikky_material_apply" .. shader_script, "object",
        { state.original, "object", diffuse, specular }, gpu_constants(geometry, { partition, 0, 0, 0 }), "copy")
    end
    return true
  end

  local function gpu_point_light(position, color, specular, shininess)
    return {
      constants = {
        position[1], position[2], position[3], 0,
        color[1], color[2], color[3], shininess,
        specular[1], specular[2], specular[3], 0,
        0, 0, -1, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0,
      }
    }
  end

  return {
    gpu_material = gpu_material,
    gpu_point_light = gpu_point_light,
  }
end
