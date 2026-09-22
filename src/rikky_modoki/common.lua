local function finite_number(value)
  assert(type(value) == "number" and value > -math.huge and value < math.huge, "Expected a finite number")
  return value
end

local function clamp(value, minimum, maximum)
  return math.max(minimum, math.min(maximum, value))
end

return {
  finite_number = finite_number,
  clamp = clamp,
}
