return function(rikky_module, module)
  local common = require("rikky_modoki.common")
  local finite_number = common.finite_number
  local clamp = common.clamp
  local math3d = require("rikky_modoki.math3d")
  local rotation_matrix = math3d.rotation_matrix
  local multiply_matrix = math3d.multiply_matrix

  local function image_key(id)
    if type(id) == "string" then
      assert(not id:find("\0", 1, true), "Image ID contains NUL")
      return "s:" .. id
    end
    assert(type(id) == "number" and id > -math.huge and id < math.huge, "Invalid image ID")
    if id == 0 then
      id = 0
    end -- -0 と 0 は同じID。
    return "n:" .. string.format("%.17g", id)
  end

  local function image_integer(value, minimum, maximum)
    assert(
      type(value) == "number" and value == math.floor(value) and value >= minimum and value <= maximum,
      "Invalid image integer"
    )
    return value
  end

  local function save_image_file(file, format, quality)
    assert(type(file) == "string" and not file:find("\0", 1, true), "Invalid image filename")
    if file == "" then
      return
    end
    if quality == nil then
      quality = 100
    else
      quality = tonumber(quality)
      assert(quality ~= nil and quality > -math.huge and quality < math.huge, "Invalid JPEG quality")
      quality = math.max(1, math.min(100, math.floor(quality)))
    end
    local data, width, height = obj.getpixeldata("object", "rgba")
    if data == nil or width == 0 or height == 0 then
      return
    end
    module.image_save_file(file, format, data, width, height, quality)
  end

  function rikky_module.png(file)
    save_image_file(file, "png")
  end

  function rikky_module.jpg(file, quality)
    save_image_file(file, "jpg", quality)
  end

  function rikky_module.bmp(file)
    save_image_file(file, "bmp")
  end

  local function load_image(data, width, height, lease, reset)
    if reset and not obj.load("figure", "四角形", 0, 1) then
      module.image_release(lease)
      return false
    end
    obj.clearbuffer("object", width, height)
    obj.putpixeldata("object", data, width, height, "rgba")
    module.image_release(lease)
    return true
  end

  function rikky_module.image(mode, id, a, b, c, d)
    if mode == "w" or mode == "w+" then
      local key = image_key(id)
      local target = "object"
      if mode == "w" and a ~= nil then
        assert(a == "object" or a == "tempbuffer", "Invalid image buffer")
        target = a
      end
      local data, width, height = obj.getpixeldata(target, "rgba")
      if data == nil or width == 0 or height == 0 then
        return false
      end
      local alpha = 1
      if mode == "w+" then
        alpha = obj.alpha * obj.getvalue("alpha")
      end
      return module.image_write(key, data, width, height, alpha, false)
    elseif mode == "r" or mode == "r+" or mode == "i" or mode == "i+" then
      local data, width, height, lease = module.image_read(image_key(id), mode == "i")
      if data == nil then
        return false
      end
      if mode == "i" then
        module.image_release(lease)
        return data, width, height
      elseif mode == "i+" then
        local r, g, blue, alpha = module.image_channels(lease)
        module.image_release(lease)
        return { A = alpha, R = r, G = g, B = blue }, width, height
      end
      return load_image(data, width, height, lease, mode == "r")
    elseif mode == "c" then
      return module.image_delete(image_key(id))
    elseif mode == "c+" then
      return module.image_delete(nil)
    elseif mode == "g" or mode == "g+" then
      local count = 0
      local single = false
      if mode == "g" then
        if id == nil or id == 0 then
          count, single = 1, true
        else
          count = image_integer(id, 1, 2147483647)
        end
      end
      local ids = module.image_ids(count)
      for i, key in ipairs(ids) do
        if key:sub(1, 2) == "n:" then
          ids[i] = tonumber(key:sub(3))
        else
          ids[i] = key:sub(3)
        end
      end
      if single then
        return ids[1]
      end
      return ids
    elseif mode == "m" or mode == "m+" then
      if b == nil then
        b = 0
      end
      if c == nil then
        c = 0
      end
      local x = image_integer(b, -2147483648, 2147483647)
      local y = image_integer(c, -2147483648, 2147483647)
      local data, width, height, lease = module.image_merge(image_key(id), image_key(a), x, y, mode == "m")
      if data == nil then
        return false
      end
      if mode == "m" then
        module.image_release(lease)
        return data, width, height
      end
      return load_image(data, width, height, lease, false)
    elseif mode == "p" or mode == "p+" then
      local key = image_key(id)
      if type(a) == "userdata" then
        local width = image_integer(b, 1, 2147483647)
        local height = image_integer(c, 1, 2147483647)
        return module.image_write(key, a, width, height, 1, mode == "p+")
      end
      return module.image_copy(key, image_key(a), mode == "p+")
    elseif mode == "u" or mode == "u+" then
      local data, width, height, lease, y
      local x = image_integer(a, -9007199254740991, 9007199254740991)
      if type(id) == "userdata" then
        data = id
        if d == nil then
          width, height = b, c
        else
          y, width, height = b, c, d
        end
        width = image_integer(width, 1, 2147483647)
        height = image_integer(height, 1, 2147483647)
      else
        y = b
      end
      if y ~= nil then
        y = image_integer(y, -9007199254740991, 9007199254740991)
      end
      if data == nil then
        data, width, height, lease = module.image_read(image_key(id), false)
        if data == nil then
          return false
        end
      end
      local index = x
      if y ~= nil then
        if x < 1 or x > width or y < 1 or y > height then
          if lease ~= nil then
            module.image_release(lease)
          end
          return false
        end
        index = (y - 1) * width + x - 1
      end
      local r, g, blue, alpha = module.image_pixel(data, width, height, index)
      if lease ~= nil then
        module.image_release(lease)
      end
      if r == nil then
        return false
      end
      if mode == "u+" then
        return r, g, blue, alpha
      end
      return r * 65536 + g * 256 + blue, alpha / 255
    end
    error("Unknown image mode: " .. tostring(mode))
  end

  function rikky_module.fillarea(x, y, mode, threshold)
    x, y = finite_number(x), finite_number(y)
    assert(x == math.floor(x) and y == math.floor(y), "Invalid fill coordinates")
    mode = image_integer(mode, 0, 24)
    if threshold == nil then
      threshold = 0
    end
    threshold = finite_number(threshold)
    local maximum = ({ 255, 360, 100, 100, 255 })[math.floor(mode / 5) + 1]
    assert(threshold >= 0 and threshold <= maximum, "Invalid fill threshold")
    local data, width, height = obj.getpixeldata("object", "rgba")
    if data == nil or x < 0 or y < 0 or x >= width or y >= height then
      return false
    end
    local mask, mask_width, mask_height, lease, bounds = module.fillarea(data, width, height, x, y, mode, threshold)
    if mask == nil then
      return false
    end
    -- 同じ寸法の画像を書き戻す。書き戻し失敗時にも所有メモリを解放する。
    local ok, err = pcall(obj.putpixeldata, "object", mask, mask_width, mask_height, "rgba")
    module.image_release(lease)
    if not ok then
      error(err, 0)
    end
    return bounds[1], bounds[2], bounds[3], bounds[4]
  end

  function rikky_module.bordering(resolution, threshold, is_zoom, is_rotate, hq)
    local skip = 0
    if type(resolution) == "number" then
      skip = math.floor(clamp(finite_number(resolution), 0, 5000))
    end
    if type(threshold) ~= "number" then
      threshold = 0
    end
    local pixel = resolution == "pixel"
    local zoom = 1
    if not pixel and (is_zoom == true or is_zoom == 1) then
      zoom = obj.getvalue("zoom") * obj.zoom / 100
    end
    local matrix
    if not pixel and (is_rotate == true or is_rotate == 1) then
      matrix = multiply_matrix(
        rotation_matrix(1, 0, 0, math.rad(obj.rx)),
        multiply_matrix(rotation_matrix(0, 1, 0, math.rad(obj.ry)), rotation_matrix(0, 0, 1, math.rad(obj.rz)))
      )
    end
    local data, width, height = obj.getpixeldata("object", "rgba")
    if data == nil or width == 0 or height == 0 then
      return {}, {}, 0
    end
    local indices, counts = module.bordering(data, width, height, skip, threshold, hq == true or hq == 1)
    local points, offset = {}, 1
    for i, count in ipairs(counts) do
      local contour = {}
      for j = 1, count do
        local index = indices[offset]
        offset = offset + 1
        local x, y = index % width, math.floor(index / width)
        if not pixel then
          x, y = (x + (1 - width) / 2) * zoom, (y + (1 - height) / 2) * zoom
        end
        if matrix then
          contour[j * 3 - 2] = matrix[1] * x + matrix[2] * y
          contour[j * 3 - 1] = matrix[4] * x + matrix[5] * y
          contour[j * 3] = matrix[7] * x + matrix[8] * y
        else
          contour[j * 2 - 1], contour[j * 2] = x, y
        end
      end
      points[i] = contour
    end
    return points, counts, #counts
  end

  function rikky_module.linedetection(precision, background, centered)
    if precision == nil then
      precision = 80
    end
    assert(finite_number(precision) > 0, "Detection precision must be positive")
    if background == nil then
      background = 0
    end
    background = image_integer(background, 0, 0xFFFFFF)
    local data, width, height = obj.getpixeldata("object", "rgba")
    if data == nil or width == 0 or height == 0 then
      return false
    end
    local coordinates = module.linedetection(data, width, height, precision / 100, background)
    if #coordinates == 0 then
      return false
    end
    local ox, oy = 0, 0
    if centered == true then
      ox, oy = width / 2, height / 2
    end
    local lines = {}
    for i = 1, #coordinates, 4 do
      lines[#lines + 1] = {
        x0 = coordinates[i] - ox,
        y0 = coordinates[i + 1] - oy,
        x1 = coordinates[i + 2] - ox,
        y1 = coordinates[i + 3] - oy,
      }
    end
    return lines, #lines
  end
end
