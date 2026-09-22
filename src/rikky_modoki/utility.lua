return function(rikky_module, module)
  function rikky_module.dir(directory, ...)
    local extensions = { ... }
    for i = 1, select("#", ...) do
      assert(type(extensions[i]) == "string", "Expected an extension or directory mode")
    end
    local paths = module.dir(directory, extensions)
    return paths
  end

  function rikky_module.find(value, needle)
    return (string.find(value, needle, 1, true))
  end

  function rikky_module.type(...)
    local values = { ... }
    for i = 1, select("#", ...) do
      values[i] = type(values[i])
    end
    return unpack(values)
  end

  function rikky_module.assign(mode, name, value)
    assert(mode == "make" or mode == "copy", "Unknown assign mode")
    local valid_name = type(name) == "string"
        and name:match("^[A-Za-z_][A-Za-z0-9_]*$")
        and loadstring("local " .. name) ~= nil
    if mode == "copy" then
      if valid_name then
        return _G[name]
      end
      return nil
    end
    local value_type = type(value)
    if
        not valid_name
        or (value_type ~= "string" and value_type ~= "number" and value_type ~= "boolean" and value_type ~= "table")
    then
      return false
    end
    _G[name] = value
    return true
  end

  function rikky_module.convert(value, mode, format, byte_order)
    assert(type(mode) == "string", "Expected conversion mode")
    mode = mode:lower()
    assert(mode == "shift-jis" or mode == "unicode" or mode == "utf8", "Unknown conversion mode")
    local from_string = type(value) == "string"
    assert(from_string or type(value) == "table", "Expected a string or code table")
    local unicode = mode == "unicode"
    local hex = unicode and (format == "hex" or format == "HEX")
    if unicode then
      assert(byte_order == nil or byte_order == "big" or byte_order == "little", "Invalid byte order")
    end

    local codes = {}
    if from_string then
      -- mod2の文字列変換を避け、Shift-JISのバイト列をそのまま渡す。
      for i = 1, #value do
        codes[i] = value:byte(i)
      end
      if mode ~= "shift-jis" then
        codes = module.convert_to_codes(codes, unicode)
      end
    else
      local maximum = unicode and 65535 or 255
      for i = 1, #value do
        local code = value[i]
        if type(code) == "string" then
          code = tonumber(code, hex and 16 or 10)
        end
        assert(
          type(code) == "number" and code == math.floor(code) and code >= 0 and code <= maximum,
          "Invalid character code"
        )
        codes[i] = code
      end
    end
    if unicode and byte_order == "little" then
      for i = 1, #codes do
        codes[i] = codes[i] % 256 * 256 + math.floor(codes[i] / 256)
      end
    end
    if from_string then
      if hex then
        for i = 1, #codes do
          codes[i] = string.format(format == "HEX" and "%04X" or "%04x", codes[i])
        end
      end
      return codes, #codes
    end
    if mode ~= "shift-jis" then
      codes = module.convert_from_codes(codes, unicode)
    end
    for i = 1, #codes do
      codes[i] = string.char(codes[i])
    end
    return table.concat(codes)
  end
end
