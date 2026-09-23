return function(rikky_module, module)
  local common = require("rikky_modoki.common")
  local finite_number = common.finite_number

  function rikky_module.audiobuffer(...)
    local count = select("#", ...)
    if count == 0 then
      return module.audio_buffer_info()
    end
    assert(count <= 5, "PCM audiobuffer accepts at most five arguments")
    local frame, mode, position, channels, size = ...
    frame = finite_number(frame)
    assert(frame == math.floor(frame), "Expected an integer audio frame")
    if mode == nil then
      mode = "PCM"
    end
    if position == nil then
      position = "relative"
    end
    if channels == nil then
      channels = "stereo"
    end
    assert(mode == "PCM", "Only PCM audiobuffer is supported")
    assert(position == "relative" or position == "absolute", "Expected relative or absolute")
    assert(channels == "stereo" or channels == "monaural", "Expected stereo or monaural")
    if size ~= nil then
      size = finite_number(size)
      assert(size >= 1 and size == math.floor(size), "Expected a positive integer PCM size")
    end
    if position == "relative" then
      frame = frame + obj.originframe
    end
    assert(frame >= -2147483648 and frame < 2147483647, "Audio frame is out of range")
    local left, right = module.audio_buffer_pcm(frame, size)
    if channels == "monaural" then
      for i = 1, #left do
        left[i] = (left[i] + right[i]) / 2
      end
      return left
    end
    return left, right
  end

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
