local module = obj.module("rikky_modoki")
print(module.is_development())
if module.is_development() then
  print("reload")
  package.loaded["rikky_module"] = nil
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
