return function(rikky_module, module)
  local common = require("rikky_modoki.common")
  local finite_number = common.finite_number

  function rikky_module.soundregister(...)
    local count = select("#", ...)
    if count == 0 then
      return module.sound_receiving(obj.originframe)
    end
    local file, frame, volume, speed, pan, reverse = ...
    if type(file) ~= "string" or file == "" then
      return nil
    end
    if count == 1 then
      local length = module.sound_length(file)
      if length == nil then
        return false
      end
      return length
    end
    if not module.sound_receiving(obj.originframe) then
      return false
    end
    if pan == nil then
      pan = 0
    end
    if reverse == nil then
      reverse = false
    end
    assert(type(reverse) == "boolean", "Expected a reverse flag")
    return module.sound_register(
      obj.originframe,
      file,
      finite_number(frame),
      finite_number(volume),
      finite_number(speed),
      finite_number(pan),
      reverse
    )
  end
end
