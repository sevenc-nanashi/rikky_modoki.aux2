return function(rikky_module, module)
  local common = require("rikky_modoki.common")
  local finite_number = common.finite_number
  local math3d = require("rikky_modoki.math3d")
  local unit_vector = math3d.unit_vector
  local rotation_matrix = math3d.rotation_matrix
  local multiply_matrix = math3d.multiply_matrix

  function rikky_module.camerainfo(arg_or_table, ...)
    local args
    if arg_or_table == nil then
      args = {}
    elseif type(arg_or_table) ~= "table" then
      args = { arg_or_table, ... }
    else
      args = arg_or_table
    end
    local count = #args
    assert(count <= 8 or count == 12 or count == 20 or count == 21, "Invalid draw argument count")
    local values = {}
    for i = 1, count do
      values[i] = finite_number(args[i])
    end
    local function argument(index, default)
      if index > count then
        return default
      end
      return values[index]
    end
    local function transform(m, x, y, z)
      return m[1] * x + m[2] * y + m[3] * z,
          m[4] * x + m[5] * y + m[6] * z,
          m[7] * x + m[8] * y + m[9] * z
    end

    local camera = obj.getoption("camera_param")
    local ex, ey, ez = unit_vector(camera.tx - camera.x, camera.ty - camera.y, camera.tz - camera.z)
    local nx, ny, nz = unit_vector(
      ey * camera.uz - ez * camera.uy,
      ez * camera.ux - ex * camera.uz,
      ex * camera.uy - ey * camera.ux
    )
    local ux, uy, uz = ny * ez - nz * ey, nz * ex - nx * ez, nx * ey - ny * ex
    local roll = math.rad(camera.rz)
    local cos_roll, sin_roll = math.cos(roll), math.sin(roll)
    local group = rikky_module.getinfo("group")
    local group_matrix = {
      group.Xx, group.Yx, group.Zx,
      group.Xy, group.Yy, group.Zy,
      group.Xz, group.Yz, group.Zz,
    }
    local x, y, z, rx, ry, rz, zoom
    local ax, ay, az, mx, my, mz = 1, 0, 0, 0, 0, -1
    local base_zoom = obj.getvalue("zoom") / 100
    local base_aspect = obj.getvalue("aspect") / 100
    local scale = obj.zoom * base_zoom
    local scale_x = scale * (1 - math.max(obj.aspect, 0)) * (1 - math.max(base_aspect, 0))
    local scale_y = scale * (1 + math.min(obj.aspect, 0)) * (1 + math.min(base_aspect, 0))
    if count <= 8 then
      zoom = argument(4, 1)
      x, y, z = -obj.cx * scale_x * zoom, -obj.cy * scale_y * zoom, -obj.cz
      rx, ry, rz = argument(6, 0), argument(7, 0), argument(8, 0)
    else
      -- drawpolyの中心は4頂点の平均。UVと不透明度は位置に影響しない。
      x, y, z = 0, 0, 0
      for i = 1, 12, 3 do
        x, y, z = x + values[i] / 4, y + values[i + 1] / 4, z + values[i + 2] / 4
      end
      -- 頂点を重ねて指定する三角形にも対応する。
      local edge = 4
      while edge <= 10 and values[edge] == values[1]
        and values[edge + 1] == values[2] and values[edge + 2] == values[3] do
        edge = edge + 3
      end
      assert(edge <= 7, "Polygon must have at least three distinct vertices")
      ax, ay, az = unit_vector(values[edge] - values[1], values[edge + 1] - values[2], values[edge + 2] - values[3])
      local bx, by, bz
      for i = edge + 3, 10, 3 do
        bx, by, bz = values[i] - values[1], values[i + 1] - values[2], values[i + 2] - values[3]
        if by * az - bz * ay ~= 0 or bz * ax - bx * az ~= 0 or bx * ay - by * ax ~= 0 then
          break
        end
      end
      mx, my, mz = unit_vector(by * az - bz * ay, bz * ax - bx * az, bx * ay - by * ax)
      x = x * obj.zoom * (1 - math.max(obj.aspect, 0)) - obj.cx * scale_x
      y = y * obj.zoom * (1 + math.min(obj.aspect, 0)) - obj.cy * scale_y
      z = z * obj.zoom - obj.cz * scale
      rx, ry, rz, zoom = 0, 0, 0, 1
    end
    local rotation = multiply_matrix(
      rotation_matrix(1, 0, 0, math.rad(obj.rx + rx)),
      multiply_matrix(rotation_matrix(0, 1, 0, math.rad(obj.ry + ry)),
        rotation_matrix(0, 0, 1, math.rad(obj.rz + rz)))
    )
    local billboard = obj.getoption("billboard")
    local use_billboard = billboard ~= 0 and obj.getoption("camera_mode") ~= 0
    if use_billboard then
      local basis
      if billboard == 1 then
        -- 横方向のみ追従する。
        basis = rotation_matrix(0, 1, 0, math.atan2(ex, ez))
      elseif billboard == 2 then
        -- 縦横方向に追従するが、傾きは反映しない。
        basis = { nx, -ux, ex, ny, -uy, ey, nz, -uz, ez }
      else
        -- カメラの右・下・前をオブジェクトの基底にする。
        basis = {
          nx * cos_roll + ux * sin_roll, nx * sin_roll - ux * cos_roll, ex,
          ny * cos_roll + uy * sin_roll, ny * sin_roll - uy * cos_roll, ey,
          nz * cos_roll + uz * sin_roll, nz * sin_roll - uz * cos_roll, ez,
        }
      end
      rotation = multiply_matrix(basis, rotation)
    end
    x, y, z = transform(rotation, x, y, z)
    ax, ay, az = transform(rotation, ax, ay, az)
    mx, my, mz = transform(rotation, mx, my, mz)
    if use_billboard then
      -- 基準位置にはグループ回転を適用するが、カメラ基準の面には重ねて適用しない。
      x, y, z = transform({
        group.Xx, group.Xy, group.Xz,
        group.Yx, group.Yy, group.Yz,
        group.Zx, group.Zy, group.Zz,
      }, x, y, z)
    end
    x, y, z = x + obj.x + obj.ox, y + obj.y + obj.oy, z + obj.z + obj.oz
    if count <= 8 then
      x, y, z = x + argument(1, 0), y + argument(2, 0), z + argument(3, 0)
    end
    x, y, z = transform(group_matrix, x, y, z)
    x, y, z = x * group.zoom + group.x, y * group.zoom + group.y, z * group.zoom + group.z
    if not use_billboard then
      ax, ay, az = transform(group_matrix, ax, ay, az)
      mx, my, mz = transform(group_matrix, mx, my, mz)
    end
    local vx, vy, vz = x - camera.x, y - camera.y, z - camera.z
    local normal_d = vx * ex + vy * ey + vz * ez
    local real_d = math.sqrt(vx * vx + vy * vy + vz * vz)
    assert(normal_d ~= 0, "Cannot project an object on the camera plane")
    local perspective = camera.d / normal_d
    local ix = (vx * nx + vy * ny + vz * nz) * perspective
    local iy = -(vx * ux + vy * uy + vz * uz) * perspective
    return {
      x = camera.x,
      y = camera.y,
      z = camera.z,
      tx = camera.tx,
      ty = camera.ty,
      tz = camera.tz,
      ux = ux,
      uy = uy,
      uz = uz,
      ex = ex,
      ey = ey,
      ez = ez,
      nx = nx,
      ny = ny,
      nz = nz,
      rz = roll,
      d = camera.d,
      blur = obj.getoption("camera_focus").bokeh,
      normal_d = normal_d,
      real_d = real_d,
      vx = vx / real_d,
      vy = vy / real_d,
      vz = vz / real_d,
      mx = mx,
      my = my,
      mz = mz,
      ax = ax,
      ay = ay,
      az = az,
      ix = ix * cos_roll - iy * sin_roll,
      iy = ix * sin_roll + iy * cos_roll,
      zoom = perspective * scale * zoom * group.zoom,
      -- AviUtl1のカメラ制御のシャドーに対応する取得APIはない。
      shadow_x = 0,
      shadow_y = 0,
      shadow_z = 0,
      shadow_accu = 0,
      shadow_conce = 0,
      shadow_flag = 0,
      billboard = billboard,
    }
  end
end
