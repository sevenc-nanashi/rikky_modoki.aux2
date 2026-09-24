return function(rikky_module, module, draw)
  local common = require("rikky_modoki.common")
  local clamp = common.clamp
  local glass_number = draw.glass_number
  local glass_integer = draw.glass_integer
  local shader_script = draw.shader_script
  local glass_capture = draw.glass_capture
  local gpu_constants = draw.gpu_constants
  local gpu_geometry = draw.gpu_geometry
  local glass_release_state = draw.glass_release_state
  local capture_draw_state = draw.capture_draw_state
  local draw_arguments = draw.draw_arguments
  local draw_processed_image = draw.draw_processed_image

  -- glassdrawの画像はeffectごとに所有し、別のスクリプトのinitで上書きしない。
  local glass_states = {}

  function rikky_module.glassdraw_init(settings)
    local id = obj.effect_id
    glass_release_state(glass_states[id])
    glass_states[id] = nil
    local state = {}
    local ok, err = pcall(function()
      local table_settings = type(settings) == "table"
      if not table_settings then
        settings = {}
      end
      local reverse = 0
      if glass_integer(settings.reverse, 0) == 1 then
        reverse = 3
      else
        if glass_integer(settings.reverseUp, 0) == 1 then
          reverse = reverse + 1
        end
        if glass_integer(settings.reverseSide, 0) == 1 then
          reverse = reverse + 2
        end
      end
      local color = glass_integer(settings.color, -1) % 4294967296
      if color >= 2147483648 then
        color = color - 4294967296
      end
      local blur = glass_integer(settings.blur, 0) % 65536
      if blur >= 32768 then
        blur = blur - 65536
      end
      state.blur = table_settings and clamp(blur, 0, 30) or -1
      local zoom = glass_number(settings.zoom, 1)
      if zoom <= 0 then
        zoom = 1
      end
      local boundary, lens = 0, 0
      if settings.boundary == "loop" then
        boundary = 1
      end
      if settings.boundary == "inverted" then
        boundary = 2
      end
      if settings.lens == "convex" then
        lens = 1
      end
      if settings.lens == "concave" then
        lens = 2
      end
      state.settings = {
        color = color,
        reverse = reverse,
        boundary = boundary,
        lens = lens,
        culling = glass_integer(settings.culling, 0) % 256 == 1,
        refractive = clamp(glass_number(settings.refractive, 0), 0, 1),
        offset_z = math.max(0, glass_number(settings.offsetZ, 300)),
        inverse_zoom = 1 / zoom,
      }
      capture_draw_state(state)
      if glass_integer(settings.async, 0) == 1 then
        state.background = glass_capture("framebuffer")
      end
    end)
    if not ok then
      glass_release_state(state)
      error(err, 0)
    end
    glass_states[id] = state
  end

  function rikky_module.glassdraw(...)
    local state = glass_states[obj.effect_id]
    assert(state ~= nil, "Call glassdraw_init before glassdraw")
    local count, args = draw_arguments(...)
    local geometry = gpu_geometry(state, args)
    local settings = state.settings
    draw_processed_image(state, count, args, function()
      if settings.culling and geometry[20] >= 0 then
        return false
      end
      local background = state.background
      if background == nil then
        background = "framebuffer"
      end
      local r, g, b = 1, 1, 1
      if settings.color ~= -1 then
        r, g, b =
          math.floor(settings.color / 65536) % 256 / 255,
          math.floor(settings.color / 256) % 256 / 255,
          settings.color % 256 / 255
      end
      local xsign = settings.reverse >= 2 and -1 or 1
      local ysign = settings.reverse % 2 == 1 and -1 or 1
      local constants = gpu_constants(geometry, {
        r,
        g,
        b,
        0,
        settings.refractive,
        settings.offset_z,
        settings.inverse_zoom,
        settings.lens,
        xsign,
        ysign,
        0,
        0,
      })
      local sampler = ({ "clamp", "loop", "mirror" })[settings.boundary + 1]
      obj.pixelshader(
        "rikky_glass" .. shader_script,
        "object",
        { state.original, background },
        constants,
        "copy",
        sampler
      )
      return true
    end)
  end
end
