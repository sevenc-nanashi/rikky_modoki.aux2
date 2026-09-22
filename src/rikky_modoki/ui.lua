return function(rikky_module, module)
  function rikky_module.progressbar(mode, value, color)
    if mode == "start" then
      if type(value) ~= "string" or value:find("\0", 1, true) then
        return false
      end
      if color == nil then
        color = 0
      end
      if type(color) ~= "number" or not (color >= 0 and color <= 0xffffff and color == math.floor(color)) then
        return false
      end
      return module.progress_start(value, color)
    elseif mode == "processing" then
      if type(value) ~= "number" or not (value > -math.huge and value < math.huge) then
        return false
      end
      return module.progress_processing(value)
    elseif mode == "end" then
      return module.progress_end()
    end
    return false
  end
end
