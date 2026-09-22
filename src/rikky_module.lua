local module = obj.module("rikky_modoki")
if module.is_development() then
  local loaded = {}
  for name in pairs(package.loaded) do
    if name:match("^rikky_modoki%.") then
      loaded[#loaded + 1] = name
    end
  end
  for _, name in ipairs(loaded) do
    package.loaded[name] = nil
  end
end

return require("rikky_modoki.init")
