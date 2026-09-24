return function(rikky_module, module)
  -- NOTE: AviUtl2のmod2で文字列を返すと勝手にSJISに変換されてくれるっぽい？
  --
  -- local function to_sjis(str)
  --   local sjis_bytes = module.to_sjis(str)
  --   local sjis_str = ""
  --   local stack_size = 100
  --   for i = 1, #sjis_bytes, stack_size do
  --     local chunk = {}
  --     for j = i, math.min(i + stack_size - 1, #sjis_bytes) do
  --       table.insert(chunk, string.char(sjis_bytes[j]))
  --     end
  --     sjis_str = sjis_str .. table.concat(chunk)
  --   end
  --   return sjis_str
  -- end

  -- スクリプト名を取得する。
  -- script_nameはアニメーション効果でしか使えないので、
  -- それ以外の場合 = カスタムオブジェクトではobjから位置を取得して
  -- そこからscript_nameを取得する。
  local function get_script_name()
    local anm_name = obj.getoption("script_name")
    if anm_name ~= "" then
      return anm_name
    end

    return module.script_name_of(obj.layer, obj.frame_s)
  end

  function rikky_module.file(...)
    local indices = { ... }
    for _, index in ipairs(indices) do
      module.rewrite_parameter(get_script_name(), "anm", "file", index)
    end
  end

  function rikky_module.fold(...)
    local indices = { ... }
    for _, index in ipairs(indices) do
      module.rewrite_parameter(get_script_name(), "anm", "folder", index)
    end
  end

  function rikky_module.font(...)
    local indices = { ... }
    for _, index in ipairs(indices) do
      module.rewrite_parameter(get_script_name(), "anm", "font", index)
    end
  end

  function rikky_module.checkbox(...)
    local indices = { ... }
    for _, index in ipairs(indices) do
      module.rewrite_parameter(get_script_name(), "anm", "check", index)
    end
  end

  function rikky_module.colordialog(...)
    local indices = { ... }
    local current = 1
    while current <= #indices do
      local index = indices[current]
      module.rewrite_parameter(get_script_name(), "anm", "color", index)
      if type(indices[current + 1]) ~= "number" then
        current = current + 2
      else
        current = current + 1
      end
    end
  end

  function rikky_module.list(...)
    local indices = { ... }
    for i = 1, #indices, 2 do
      local index = indices[i]
      local choices = indices[i + 1]
      module.rewrite_select_parameter(get_script_name(), "anm", index, choices)
    end
  end

  function rikky_module.fileCS(...)
    local indices = { ... }
    for _, index in ipairs(indices) do
      module.rewrite_parameter(get_script_name(), "obj", "file", index)
    end
  end

  function rikky_module.foldCS(...)
    local indices = { ... }
    for _, index in ipairs(indices) do
      module.rewrite_parameter(get_script_name(), "obj", "folder", index)
    end
  end

  function rikky_module.fontCS(...)
    local indices = { ... }
    for _, index in ipairs(indices) do
      module.rewrite_parameter(get_script_name(), "obj", "font", index)
    end
  end

  function rikky_module.listCS(...)
    local indices = { ... }
    for i = 1, #indices, 2 do
      local index = indices[i]
      local choices = indices[i + 1]
      module.rewrite_select_parameter(get_script_name(), "obj", index, choices)
    end
  end

  function rikky_module.checkboxCS(...)
    local indices = { ... }
    for _, index in ipairs(indices) do
      module.rewrite_parameter(get_script_name(), "obj", "check", index)
    end
  end

  function rikky_module.colordialogCS(...)
    local indices = { ... }
    local current = 1
    while current <= #indices do
      local index = indices[current]
      module.rewrite_parameter(get_script_name(), "anm", "color", index)
      if type(indices[current + 1]) ~= "number" then
        current = current + 2
      else
        current = current + 1
      end
    end
  end

  local function parameter(value, index, definition, extension)
    assert(type(index) == "number" and index >= 1 and index == math.floor(index), "Invalid parameter index")
    assert(
      type(definition) == "table" and #definition > 0 and #definition % 4 == 0,
      "Parameter definitions must contain groups of four values"
    )
    local count = #definition / 4
    local generated = type(value) == "table"
    if generated and not module.group_parameter_needs_rewrite(get_script_name(), extension, index) then
      return unpack(value, 1, count)
    end
    assert(generated or type(value) == "string", "Parameter value must be a string or a generated value table")

    local entries = {}
    local defaults = {}
    for i = 1, #definition, 4 do
      local label, source, minimum, maximum = definition[i], definition[i + 1], definition[i + 2], definition[i + 3]
      assert(type(label) == "string" and label ~= "" and not label:find("[,\r\n]"), "Invalid parameter label")
      assert(type(source) == "string" and not source:find("[\r\n]"), "Invalid parameter default")
      assert(
        type(minimum) == "number"
          and type(maximum) == "number"
          and minimum > -math.huge
          and maximum < math.huge
          and minimum <= maximum,
        "Invalid parameter range"
      )
      local evaluate = assert(loadstring("return (" .. source .. "\n)", "parameter default"))
      local initial = evaluate()
      local initial_type = type(initial)
      local normalized
      if initial_type == "string" then
        normalized = string.format("%q", initial):gsub("\\\n", "\\n")
      elseif initial_type == "number" then
        assert(initial > -math.huge and initial < math.huge, "Invalid numeric default")
        normalized = string.format("%.17g", initial)
      elseif initial_type == "boolean" or initial_type == "nil" then
        normalized = tostring(initial)
      else
        assert(initial_type == "table", "Unsupported parameter default type")
        normalized = source
      end
      defaults[(i + 3) / 4] = initial
      entries[i] = label
      entries[i + 1] = normalized
      entries[i + 2] = string.format("%.17g", minimum)
      entries[i + 3] = string.format("%.17g", maximum)
    end

    module.rewrite_group_parameter(get_script_name(), extension, index, entries)
    if generated then
      return unpack(value, 1, count)
    end
    return unpack(defaults, 1, count)
  end

  function rikky_module.parameter(value, index, definition)
    return parameter(value, index, definition, "anm")
  end

  function rikky_module.parameterCS(value, index, definition)
    return parameter(value, index, definition, "obj")
  end
end
