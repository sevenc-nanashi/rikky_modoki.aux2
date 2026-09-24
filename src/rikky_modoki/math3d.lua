local common = require("rikky_modoki.common")
local finite_number = common.finite_number

local function unit_vector(x, y, z)
  x, y, z = finite_number(x), finite_number(y), finite_number(z)
  local scale = math.max(math.abs(x), math.abs(y), math.abs(z))
  assert(scale > 0, "Rotation axis must not be zero")
  x, y, z = x / scale, y / scale, z / scale
  local length = math.sqrt(x * x + y * y + z * z)
  return x / length, y / length, z / length
end

local function rotation_matrix(x, y, z, angle)
  x, y, z = unit_vector(x, y, z)
  angle = finite_number(angle)
  local s, c = math.sin(angle), math.cos(angle)
  local t = 1 - c
  return {
    c + x * x * t,
    x * y * t - z * s,
    x * z * t + y * s,
    y * x * t + z * s,
    c + y * y * t,
    y * z * t - x * s,
    z * x * t - y * s,
    z * y * t + x * s,
    c + z * z * t,
  }
end

local function multiply_matrix(a, b)
  local result = {}
  for row = 0, 2 do
    for column = 1, 3 do
      result[row * 3 + column] = a[row * 3 + 1] * b[column]
        + a[row * 3 + 2] * b[column + 3]
        + a[row * 3 + 3] * b[column + 6]
    end
  end
  return result
end

return {
  unit_vector = unit_vector,
  rotation_matrix = rotation_matrix,
  multiply_matrix = multiply_matrix,
}
