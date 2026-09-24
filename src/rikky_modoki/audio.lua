return function(rikky_module, module)
  local common = require("rikky_modoki.common")
  local finite_number = common.finite_number

  local function spectrum(samples, first, last, size)
    local result = {}
    if first > last then
      for i = 1, size do
        result[i] = 0
      end
      return result
    end
    local function interpolate(index)
      index = math.max(first, math.min(last, index))
      local lower = math.floor(index)
      local upper = math.min(last, lower + 1)
      return samples[lower] + (samples[upper] - samples[lower]) * (index - lower)
    end
    -- 周波数ビン+1を対数軸で分割する。DCも扱えるようにする。
    -- 各帯域の最大振幅を返す。ビン間は線形補間し、狭い帯域も連続にする。
    local lower_log = math.log(first)
    local span = math.log(last) - lower_log
    for i = 1, size do
      local lower = math.exp(lower_log + span * (i - 1) / size)
      local upper = math.exp(lower_log + span * i / size)
      local peak = math.max(interpolate(lower), interpolate(upper))
      for bin = math.max(first, math.ceil(lower)), math.min(last, math.floor(upper)) do
        peak = math.max(peak, samples[bin])
      end
      result[i] = peak
    end
    return result
  end

  local function frequency_values(samples, mode, first, last, size)
    if mode == "SPECTRUM" then
      return spectrum(samples, first, last, size)
    end
    local result = {}
    for i = first, last do
      local value = samples[i]
      if mode == "DECIBEL" then
        if value > 0 then
          result[i - first + 1] = 20 * math.log(value) / math.log(10)
        end
      else
        -- 旧版FOURIERは16bit PCMをHann窓で変換し、4/N倍して整数化する。
        -- Rust側は単位振幅に正規化しているため、ここで旧版の単位へ戻す。
        local scale = 32768
        if i == 1 then
          scale = scale * 2 -- Rust側だけDCを2/N倍にしている。
        end
        result[i - first + 1] = math.floor(value * scale)
      end
    end
    return result
  end

  function rikky_module.audiobuffer(...)
    local count = select("#", ...)
    if count == 0 then
      return module.audio_buffer_info()
    end
    assert(count <= 7, "audiobuffer accepts at most seven arguments")
    local frame, mode, position, channels, size, resolution, range = ...
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
    if mode == "DESIBEL" then
      mode = "DECIBEL"
    end
    assert(mode == "PCM" or mode == "FOURIER" or mode == "SPECTRUM" or mode == "DECIBEL", "Unknown audio mode")
    assert(position == "relative" or position == "absolute", "Expected relative or absolute")
    assert(channels == "stereo" or channels == "monaural", "Expected stereo or monaural")
    if size ~= nil then
      size = finite_number(size)
      assert(size >= 0 and size == math.floor(size), "Expected a nonnegative integer audio size")
      if mode == "PCM" or mode == "SPECTRUM" then
        assert(size >= 1, "Expected a positive audio size")
      end
    end
    if position == "relative" then
      frame = frame + obj.originframe
    end
    assert(frame >= -2147483648 and frame < 2147483647, "Audio frame is out of range")
    if mode == "PCM" then
      assert(resolution == nil and range == nil, "PCM does not accept resolution or frequency range")
      local left, right = module.audio_buffer_pcm(frame, size)
      if channels == "monaural" then
        for i = 1, #left do
          left[i] = (left[i] + right[i]) / 2
        end
        return left
      end
      return left, right
    end
    if resolution == nil then
      resolution = 1
    end
    resolution = finite_number(resolution)
    assert(
      resolution >= 0 and resolution <= 3 and resolution == math.floor(resolution),
      "Expected resolution 0, 1, 2 or 3"
    )
    local bins = 512 * 2 ^ resolution
    if mode == "SPECTRUM" then
      if size == nil then
        size = bins
      end
      assert(size <= bins, "Spectrum size exceeds the FFT bin count")
    end
    local _, sample_rate = module.audio_buffer_info()
    local lower, upper = 0, sample_rate / 2
    if range ~= nil then
      assert(type(range) == "table" and #range == 2, "Expected {lower, upper} frequency range")
      lower, upper = finite_number(range[1]), finite_number(range[2])
      assert(lower >= 0 and lower <= upper and upper <= sample_rate / 2, "Frequency range must be within 0 and Nyquist")
    end
    local step = sample_rate / (bins * 2)
    local first = math.ceil(lower / step) + 1
    local last = math.min(bins, math.floor(upper / step) + 1)
    local left, right = module.audio_buffer_fourier(frame, resolution, channels == "monaural")
    left = frequency_values(left, mode, first, last, size)
    if channels == "monaural" then
      return left
    end
    return left, frequency_values(right, mode, first, last, size)
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
