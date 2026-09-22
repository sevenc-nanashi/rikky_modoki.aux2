return function(rikky_module, module)
  local common = require("rikky_modoki.common")
  local finite_number = common.finite_number
  local math3d = require("rikky_modoki.math3d")
  local unit_vector = math3d.unit_vector
  local rotation_matrix = math3d.rotation_matrix
  local multiply_matrix = math3d.multiply_matrix

  local function aviutl_angles(matrix, radians)
    -- AviUtl の回転は Rx * Ry * Rz（座標にはZ、Y、Xの順に適用）。
    -- 旧版と同じく cos(Y) <= 0 の解を選ぶ。
    local cosine = math.sqrt(matrix[1] * matrix[1] + matrix[2] * matrix[2])
    local y = math.atan2(matrix[3], -cosine)
    local x, z
    if cosine > 1e-8 then
      x = math.atan2(matrix[6], -matrix[9])
      z = math.atan2(matrix[2], -matrix[1])
    else
      -- ジンバルロック時は分離できないX/Zを、同じ姿勢になるZ=-piに固定する。
      x, z = math.atan2(matrix[8], matrix[5]) + math.pi, -math.pi
    end
    x = (x + math.pi) % (2 * math.pi) - math.pi
    y = (y + math.pi) % (2 * math.pi) - math.pi
    z = (z + math.pi) % (2 * math.pi) - math.pi
    if radians == 1 then
      return x, y, z
    end
    return math.deg(x), math.deg(y), math.deg(z)
  end

  local previous_rotation, previous_center

  function rikky_module.rotation(x, y, z, angle, axis, center)
    x, y, z = finite_number(x), finite_number(y), finite_number(z)
    if angle ~= nil then
      if axis == "X" then
        axis = { 1, 0, 0 }
      elseif axis == "Y" then
        axis = { 0, 1, 0 }
      elseif axis == "Z" then
        axis = { 0, 0, 1 }
      end
      assert(type(axis) == "table", "Invalid rotation axis")
      local matrix = rotation_matrix(axis[1], axis[2], axis[3], angle)
      if center == nil then
        center = { 0, 0, 0 }
      end
      assert(type(center) == "table", "Invalid rotation center")
      local origin = { finite_number(center[1]), finite_number(center[2]), finite_number(center[3]) }
      previous_rotation, previous_center = matrix, origin
    else
      assert(axis == nil and center == nil and previous_rotation ~= nil, "No previous rotation")
    end
    local m, origin = previous_rotation, previous_center
    x, y, z = x - origin[1], y - origin[2], z - origin[3]
    return m[1] * x + m[2] * y + m[3] * z + origin[1],
        m[4] * x + m[5] * y + m[6] * z + origin[2],
        m[7] * x + m[8] * y + m[9] * z + origin[3]
  end

  function rikky_module.axisconvertEx(axes, radians, moving)
    assert(type(axes) == "table" and #axes % 4 == 0, "Axes must contain groups of four numbers")
    if radians == nil then
      radians = 0
    end
    if moving == nil then
      moving = 0
    end
    assert(radians == 0 or radians == 1, "Invalid angle unit")
    assert(moving == 0 or moving == 1, "Invalid moving-axis option")
    local basis = { 1, 0, 0, 0, 1, 0, 0, 0, 1 }
    if axes.Xx ~= nil or axes.Xy ~= nil or axes.Xz ~= nil or axes.Yx ~= nil or axes.Yy ~= nil or axes.Yz ~= nil then
      local xx, xy, xz = unit_vector(axes.Xx, axes.Xy, axes.Xz)
      local yx, yy, yz = unit_vector(axes.Yx, axes.Yy, axes.Yz)
      assert(math.abs(xx * yx + xy * yy + xz * yz) < 1e-6, "Initial axes must be orthogonal")
      basis = { xx, yx, xy * yz - xz * yy, xy, yy, xz * yx - xx * yz, xz, yz, xx * yy - xy * yx }
    end
    local matrix = { 1, 0, 0, 0, 1, 0, 0, 0, 1 }
    for i = 1, #axes, 4 do
      local angle = finite_number(axes[i + 3])
      if radians == 0 then
        angle = math.rad(angle)
      end
      local x, y, z = finite_number(axes[i]), finite_number(axes[i + 1]), finite_number(axes[i + 2])
      local rotation = rotation_matrix(x, y, z, angle)
      if moving == 1 then
        axes[i] = matrix[1] * x + matrix[2] * y + matrix[3] * z
        axes[i + 1] = matrix[4] * x + matrix[5] * y + matrix[6] * z
        axes[i + 2] = matrix[7] * x + matrix[8] * y + matrix[9] * z
        matrix = multiply_matrix(matrix, rotation)
      else
        matrix = multiply_matrix(rotation, matrix)
      end
    end
    return aviutl_angles(multiply_matrix(matrix, basis), radians)
  end

  function rikky_module.axisconvert(axes, radians)
    assert(type(axes) == "table", "Expected an axis table")
    return rikky_module.axisconvertEx({
      finite_number(axes.Zx),
      finite_number(axes.Zy),
      finite_number(axes.Zz),
      finite_number(axes.rz),
      finite_number(axes.Yx),
      finite_number(axes.Yy),
      finite_number(axes.Yz),
      finite_number(axes.ry),
      finite_number(axes.Xx),
      finite_number(axes.Xy),
      finite_number(axes.Xz),
      finite_number(axes.rx),
      Xx = axes.Xx,
      Xy = axes.Xy,
      Xz = axes.Xz,
      Yx = axes.Yx,
      Yy = axes.Yy,
      Yz = axes.Yz,
    }, radians, 0)
  end
end
