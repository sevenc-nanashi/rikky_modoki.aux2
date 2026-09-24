return function(rikky_module, module)
  local common = require("rikky_modoki.common")
  local finite_number = common.finite_number
  local clamp = common.clamp

  local function rgb_color(r, g, b)
    return RGB(
      math.floor(clamp(r, 0, 255) + 0.5),
      math.floor(clamp(g, 0, 255) + 0.5),
      math.floor(clamp(b, 0, 255) + 0.5)
    )
  end

  local function rgb_to_xyz(r, g, b)
    -- 旧版はRGB成分を線形値として扱い、ガンマ補正を行わない。
    r, g, b = r / 255 * 100, g / 255 * 100, b / 255 * 100
    return 0.412391 * r + 0.357584 * g + 0.180481 * b,
      0.212639 * r + 0.715169 * g + 0.072192 * b,
      0.019331 * r + 0.119195 * g + 0.950532 * b
  end

  local function xyz_color(x, y, z)
    x, y, z = x / 100 * 255, y / 100 * 255, z / 100 * 255
    return rgb_color(
      3.24096637658435 * x - 1.53737885234726 * y - 0.498611723228325 * z,
      -0.969242037979635 * x + 1.87596526849091 * y + 0.041555768342051 * z,
      0.0556295671173938 * x - 0.203976940895256 * y + 1.0569716994422 * z
    )
  end

  local function lab_curve(value)
    if value > (6 / 29) ^ 3 then
      return value ^ (1 / 3)
    end
    return value / (3 * (6 / 29) ^ 2) + 4 / 29
  end

  local function inverse_lab_curve(value)
    if value > 6 / 29 then
      return value ^ 3
    end
    return 3 * (6 / 29) ^ 2 * (value - 4 / 29)
  end

  function rikky_module.colorconvert(mode, a, b, c, d)
    a = finite_number(a)
    if b == nil and c == nil and d == nil then
      assert(a >= 0 and a <= 0xffffff and a == math.floor(a), "Invalid RGB color")
      local r, g, blue = RGB(a)
      if mode == "rgb" then
        return r, g, blue
      elseif mode == "hsv" then
        return HSV(a)
      elseif mode == "opposite" then
        return RGB(255 - r, 255 - g, 255 - blue)
      elseif mode == "complemntary" or mode == "complementary" then
        local hue, saturation, value = HSV(a)
        return HSV((hue + 180) % 360, saturation, value)
      elseif mode == "hsl" then
        local high, low = math.max(r, g, blue) / 255, math.min(r, g, blue) / 255
        local lightness = (high + low) / 2
        local saturation = 0
        if high ~= low then
          saturation = (high - low) / (1 - math.abs(2 * lightness - 1))
        end
        return HSV(a), saturation * 100, lightness * 100
      elseif mode == "yc" then
        r, g, blue = r / 255, g / 255, blue / 255
        local y = 0.299 * r + 0.587 * g + 0.114 * blue
        return math.floor(y * 4096 + 0.5),
          math.floor((blue - y) / 1.772 * 4096 + 0.5),
          math.floor((r - y) / 1.402 * 4096 + 0.5)
      elseif mode == "xyz" then
        return rgb_to_xyz(r, g, blue)
      elseif mode == "lab" then
        local x, y, z = rgb_to_xyz(r, g, blue)
        x, y, z = lab_curve(x / 95.0456), lab_curve(y / 100), lab_curve(z / 108.9058)
        return 116 * y - 16, 500 * (x - y), 200 * (y - z)
      elseif mode == "cmy" then
        return (1 - r / 255) * 100, (1 - g / 255) * 100, (1 - blue / 255) * 100
      elseif mode == "cmyk" then
        local high = math.max(r, g, blue)
        if high == 0 then
          return 0, 0, 0, 100
        end
        return (1 - r / high) * 100, (1 - g / high) * 100, (1 - blue / high) * 100, (1 - high / 255) * 100
      end
    else
      b, c = finite_number(b), finite_number(c)
      if mode == "cmyk" then
        d = finite_number(d)
      else
        assert(d == nil, "Unexpected fourth color component")
      end
      if mode == "rgb" then
        return rgb_color(a, b, c)
      elseif mode == "hsv" then
        return HSV(a % 360, clamp(b, 0, 100), clamp(c, 0, 100))
      elseif mode == "hsl" then
        local saturation, lightness = clamp(b, 0, 100) / 100, clamp(c, 0, 100) / 100
        local value = lightness + saturation * math.min(lightness, 1 - lightness)
        if value == 0 then
          return 0
        end
        return HSV(a % 360, 2 * (1 - lightness / value) * 100, value * 100)
      elseif mode == "yc" then
        local y, cb, cr = a / 4096, b / 4096, c / 4096
        local r, blue = y + 1.402 * cr, y + 1.772 * cb
        return rgb_color(r * 255, (y - 0.299 * r - 0.114 * blue) / 0.587 * 255, blue * 255)
      elseif mode == "xyz" then
        return xyz_color(a, b, c)
      elseif mode == "lab" then
        local y = (a + 16) / 116
        return xyz_color(
          95.0456 * inverse_lab_curve(y + b / 500),
          100 * inverse_lab_curve(y),
          108.9058 * inverse_lab_curve(y - c / 200)
        )
      elseif mode == "cmy" then
        return rgb_color((1 - a / 100) * 255, (1 - b / 100) * 255, (1 - c / 100) * 255)
      elseif mode == "cmyk" then
        local black = 1 - clamp(d, 0, 100) / 100
        return rgb_color(
          (1 - clamp(a, 0, 100) / 100) * black * 255,
          (1 - clamp(b, 0, 100) / 100) * black * 255,
          (1 - clamp(c, 0, 100) / 100) * black * 255
        )
      end
    end
    error("Unknown color conversion: " .. tostring(mode))
  end
end
