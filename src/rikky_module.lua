local module = obj.module("rikky_modoki")
rikky_module = {}

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

function rikky_module.getinfo(target, option)
  if target == "version" then
    if option == nil then
      return "1.10"
    elseif option == 1 then
      return "0.92"
    elseif option == 2 then
      return "1.4b"
    elseif option == 3 then
      return "0.6b"
    elseif option == 4 then
      return 53
    elseif option == 5 then
      return 53
    end
    error(string.format("Invalid option for target '%s': %s", target, tostring(option)))
  elseif target == "frame" then
    return obj.frame, obj.totalframe
  elseif target == "aup" then
    if option == 1 then
      -- basename_only
      return module.project_path(true)
    else
      return module.project_path(false)
    end
  elseif target == "output" then
    -- 出力中のファイル名を返すが、AviUtl2にはそういうAPIがないので断念
    return ""
  elseif target == "state" then
    return module.edit_state()
  elseif target == "path" then
    if option == 2 then
      return module.desktop_dir()
    else
      return module.project_dir()
    end
  elseif target == "focus" then
    -- NOTE: 本来はcall_read_sectionで自身が選択されているかを見たほうがいいはず
    -- 問題はobj.effect_layerはあるのにeffect_frameがないこと...
    return obj.getoption("gui")
  elseif target == "blend" then
    if option == 1 then
      local value = obj.getvalue("標準描画", "合成モード")
      if value == "通常" then
        return 0
      elseif value == "加算" then
        return 1
      elseif value == "減算" then
        return 2
      elseif value == "乗算" then
        return 3
      elseif value == "スクリーン" then
        return 4
      elseif value == "オーバーレイ" then
        return 5
      elseif value == "比較(明)" then
        return 6
      elseif value == "比較(暗)" then
        return 7
      elseif value == "輝度" then
        return 8
      elseif value == "色差" then
        return 9
      elseif value == "陰影" then
        return 10
      elseif value == "明暗" then
        return 11
      elseif value == "差分" then
        return 12
      else
        print("@warn", "Unknown blend mode: " .. tostring(value))
        return -1
      end
    else
      local value = obj.getoption("blend")
      if value == "none" then
        return 0
      elseif value == "add" then
        return 1
      elseif value == "sub" then
        return 2
      elseif value == "mul" then
        return 3
      elseif value == "screen" then
        return 4
      elseif value == "overlay" then
        return 5
      elseif value == "light" then
        return 6
      elseif value == "dark" then
        return 7
      elseif value == "brightness" then
        return 8
      elseif value == "chroma" then
        return 9
      elseif value == "shadow" then
        return 10
      elseif value == "light_dark" then
        return 11
      elseif value == "diff" then
        return 12
      elseif value == "alpha_add" then
        if option == 2 then
          return "alpha_add"
        else
          return 20
        end
      elseif value == "alpha_max" then
        if option == 2 then
          return "alpha_max"
        else
          return 21
        end
      elseif value == "alpha_sub" then
        if option == 2 then
          return "alpha_sub"
        else
          return 22
        end
      elseif value == "alpha_add2" then
        if option == 2 then
          return "alpha_add2"
        else
          return 25
        end
      else
        print("@warn", "Unknown blend mode: " .. tostring(value))
        return -1
      end
    end
  elseif target == "group" then
    local group = {
      zoom = 1,
      x = 0,
      y = 0,
      z = 0,
      Xx = 1,
      Xy = 0,
      Xz = 0,
      Yx = 0,
      Yy = 1,
      Yz = 0,
      Zx = 0,
      Zy = 0,
      Zz = 1,
    }
    if obj.getoption("drawtarget") == "tempbuffer" then
      return group, false
    end

    local index = 0
    local group_layer = obj.getoption("group_info", index)
    while group_layer ~= 0 do
      local group_x = obj.getvalue(group_layer, "グループ制御", "X")
      local group_y = obj.getvalue(group_layer, "グループ制御", "Y")
      local group_z = obj.getvalue(group_layer, "グループ制御", "Z")
      local group_rotation_x = math.rad(obj.getvalue(group_layer, "グループ制御", "X軸回転"))
      local group_rotation_y = math.rad(obj.getvalue(group_layer, "グループ制御", "Y軸回転"))
      local group_rotation_z = math.rad(obj.getvalue(group_layer, "グループ制御", "Z軸回転"))
      local group_zoom = obj.getvalue(group_layer, "グループ制御", "拡大率") / 100
      local sx, cx = math.sin(group_rotation_x), math.cos(group_rotation_x)
      local sy, cy = math.sin(group_rotation_y), math.cos(group_rotation_y)
      local sz, cz = math.sin(group_rotation_z), math.cos(group_rotation_z)

      local Xx, Xy, Xz = cy * cz, cx * sz + sx * sy * cz, sx * sz - cx * sy * cz
      local Yx, Yy, Yz = -cy * sz, cx * cz - sx * sy * sz, sx * cz + cx * sy * sz
      local Zx, Zy, Zz = sy, -sx * cy, cx * cy

      -- 直前のグループから上位へ、合成済みの移動と各軸に変換を適用する。
      local x, y, z = group.x, group.y, group.z
      group.x = group_zoom * (Xx * x + Yx * y + Zx * z) + group_x
      group.y = group_zoom * (Xy * x + Yy * y + Zy * z) + group_y
      group.z = group_zoom * (Xz * x + Yz * y + Zz * z) + group_z
      for _, axis in ipairs({ "X", "Y", "Z" }) do
        x, y, z = group[axis .. "x"], group[axis .. "y"], group[axis .. "z"]
        group[axis .. "x"] = Xx * x + Yx * y + Zx * z
        group[axis .. "y"] = Xy * x + Yy * y + Zy * z
        group[axis .. "z"] = Xz * x + Yz * y + Zz * z
      end
      group.zoom = group.zoom * group_zoom
      index = index + 1
      group_layer = obj.getoption("group_info", index)
    end
    return group, index > 0
  elseif target == "root" then
    return module.scene_id()
  elseif target == "text" then
    if option == nil then
      -- obj.load("text")のテキストも返すらしいが、一旦パス...
      -- フックしてあげればできそうではあるが面倒
      return obj.getvalue("テキスト", "テキスト")
    else
      return obj.getvalue(option, "テキスト", "テキスト")
    end
  elseif target == "buffer" then
    local buffer = obj.getoption("drawtarget")
    if buffer == "tempbuffer" then
      return "tmp"
    elseif buffer == "framebuffer" then
      return "frm"
    else
      error("Unknown buffer type: " .. tostring(buffer))
    end
  elseif target == "dialog" then
    return obj.getoption("gui")
  elseif target == "object" then
    local script_name = module.script_name_of(obj.layer, obj.frame_s)
    if script_name == "動画ファイル" then
      return script_name, {
        file = obj.getvalue(obj.layer, script_name, "ファイル"),
        loop = tonumber(obj.getvalue(obj.layer, script_name, "ループ再生")),
        alphachannel = 1
      }
    elseif script_name == "画像ファイル" then
      return script_name, {
        file = obj.getvalue(obj.layer, script_name, "ファイル"),
      }
    elseif script_name == "テキスト" then
      return script_name, {
        color = tonumber(obj.getvalue(obj.layer, script_name, "文字色"), 16),
        color2 = tonumber(obj.getvalue(obj.layer, script_name, "影・縁色"), 16),
        type = ({
          ["標準文字"] = 0,
          ["影付き文字"] = 1,
          ["影付き文字(薄)"] = 2,
          ["縁取り文字"] = 3,
          ["縁取り文字(細)"] = 4,
          ["縁取り文字(太)"] = 5,
          ["縁取り文字(角)"] = 6,
        })[obj.getvalue(obj.layer, script_name, "文字種別")],
        autoadjust = tonumber(obj.getvalue(obj.layer, script_name, "オブジェクトの長さを自動調節")),
        soft = 1,
        monospace = 0,
        align = ({
          ["左寄せ[上]"] = 0,
          ["中央寄せ[上]"] = 1,
          ["右寄せ[上]"] = 2,
          ["左寄せ[中]"] = 3,
          ["中央寄せ[中]"] = 4,
          ["右寄せ[中]"] = 5,
          ["左寄せ[下]"] = 6,
          ["中央寄せ[下]"] = 7,
          ["右寄せ[下]"] = 8,
          ["縦書 上寄[右]"] = 9,
          ["縦書 中央[右]"] = 10,
          ["縦書 下寄[右]"] = 11,
          ["縦書 上寄[中]"] = 12,
          ["縦書 中央[中]"] = 13,
          ["縦書 下寄[中]"] = 14,
          ["縦書 上寄[左]"] = 15,
          ["縦書 中央[左]"] = 16,
          ["縦書 下寄[左]"] = 17,
        })[obj.getvalue(obj.layer, script_name, "文字揃え")],
        spacing_x = tonumber(obj.getvalue(obj.layer, script_name, "字間")),
        spacing_y = tonumber(obj.getvalue(obj.layer, script_name, "行間")),
        presision = 1,
        font = obj.getvalue(obj.layer, script_name, "フォント"),
        individual = tonumber(obj.getvalue(obj.layer, script_name, "文字毎に個別オブジェクト")),
        display = tonumber(obj.getvalue(obj.layer, script_name, "移動座標上に表示")),
        autoscroll = tonumber(obj.getvalue(obj.layer, script_name, "自動スクロール")),
        bold = tonumber(obj.getvalue(obj.layer, script_name, "B")),
        italic = tonumber(obj.getvalue(obj.layer, script_name, "I")),
      }
    elseif script_name == "図形" then
      return script_name, {
        color = tonumber(obj.getvalue(obj.layer, script_name, "色"), 16),
        figure = obj.getvalue(obj.layer, script_name, "図形の種類"),
      }
    elseif script_name == "フレームバッファ" then
      return script_name, {
        bufferclear = tonumber(obj.getvalue(obj.layer, script_name, "フレームバッファをクリア")),
      }
    elseif script_name == "音声波形表示" then
      local file = obj.getvalue(obj.layer, script_name, "ファイル")
      local mode = tonumber(obj.getvalue(obj.layer, script_name, "スペクトラム表示"))
      local mirror = tonumber(obj.getvalue(obj.layer, script_name, "ミラー表示"))
      local pad_w = tonumber(obj.getvalue(obj.layer, script_name, "横スペース"))
      local pad_h = tonumber(obj.getvalue(obj.layer, script_name, "縦スペース"))
      local wave_type
      if mode == 1 and mirror == 1 then
        wave_type = 4
      elseif mode == 1 and pad_w == 0 and pad_h == 0 then
        wave_type = 3
      elseif mode == 1 then
        wave_type = 2
      elseif mode == 0 and pad_w == 0 and pad_h == 0 then
        wave_type = 0
      else
        wave_type = 1
      end
      return "音声波形", {
        color = tonumber(obj.getvalue(obj.layer, script_name, "波形の色"), 16),
        projectsound = file and 0 or 1,
        type = wave_type,
        file = file,
        mode = mode,
        res_w = tonumber(obj.getvalue(obj.layer, script_name, "横解像度")),
        res_h = tonumber(obj.getvalue(obj.layer, script_name, "縦解像度")),
        pad_w = pad_w,
        pad_h = pad_h,
        mirror = mirror,
      }
    elseif script_name == "シーン" then
      return script_name, {
        scenenumber = tonumber(obj.getvalue(obj.layer, script_name, "シーン")),
        loop = tonumber(obj.getvalue(obj.layer, script_name, "ループ再生")),
      }
    elseif script_name == "カメラ制御" then
      return script_name, {
        zbuffer = 1
      }
    elseif script_name == "直前オブジェクト" or script_name == "フィルタオブジェクト" or script_name == "グループ制御" then
      return script_name, {}
    else
      return "カスタムオブジェクト", {}
    end
  elseif target == "start_end" then
    return obj.frame_s, obj.frame_e
  elseif target == "filter" then
    -- 同じく現在のオブジェクトを取得する方法がないのでそれっぽい値だけ返す
    -- script_nameだとグローとかの標準エフェクトを取得できない...
    local skip = option == 1
    local prevs = {}
    while true do
      local prev_name = obj.getoption("script_name", -(#prevs + 1), skip)
      if prev_name == "" then
        break
      end
      table.insert(prevs, prev_name)
    end
    local afters = {}
    while true do
      local after_name = obj.getoption("script_name", #afters + 1, skip)
      if after_name == "" then
        break
      end
      table.insert(afters, after_name)
    end

    local result = {}
    for i = #prevs, 1, -1 do
      table.insert(result, prevs[i])
    end
    table.insert(result, obj.getoption("script_name"))
    for i = 1, #afters do
      table.insert(result, afters[i])
    end

    return result, #prevs + 1, #result
  elseif target == "shadow" then
    return 0
  elseif target == "antialias" then
    return 1
  elseif target == "culling" then
    return obj.getoption("culling")
  elseif target == "billboard" then
    return obj.getoption("billboard")
  elseif target == "force" then
    -- これもフックをしないと取得できないので、適当に0を返す
    return 0
  elseif target == "input" then
    -- 読み込み可能な拡張子の一覧は取得が面倒すぎる
    return {}
  elseif target == "hwnd" then
    return module.hwnd(), ""
  elseif target == "count" then
    -- 適当な単調増加の値を返す
    return module.counter()
  elseif target == "cache" then
    -- フック
    return {}, 0
  elseif target == "groups" then
    -- グループ化のAPIはないので常にグループ化していないとする
    return 0, 1, 0
  elseif target == "font" then
    local name, size, style_type, col1, col2, bold, italic = obj.getfont()
    return {
      name = name,
      size = size,
      bold = bold,
      italic = italic,
      color = col1,
      color2 = col2,
      type = style_type,
    }
  elseif target == "draw_state" then
    return obj.getoption("draw_state")
  else
    error(string.format("Unknown target: %s", tostring(target)))
  end
end

-- TODO: anm以外のものにも対応したい
function rikky_module.file(...)
  local indices = { ... }
  for _, index in ipairs(indices) do
    module.rewrite_parameter(
      get_script_name(),
      "anm",
      "file",
      index
    )
  end
end

function rikky_module.fold(...)
  local indices = { ... }
  for _, index in ipairs(indices) do
    module.rewrite_parameter(
      get_script_name(),
      "anm",
      "folder",
      index
    )
  end
end

function rikky_module.font(...)
  local indices = { ... }
  for _, index in ipairs(indices) do
    module.rewrite_parameter(
      get_script_name(),
      "anm",
      "font",
      index
    )
  end
end

function rikky_module.checkbox(...)
  local indices = { ... }
  for _, index in ipairs(indices) do
    module.rewrite_parameter(
      get_script_name(),
      "anm",
      "check",
      index
    )
  end
end

function rikky_module.colordialog(...)
  local indices = { ... }
  local current = 1
  while current <= #indices do
    local index = indices[current]
    module.rewrite_parameter(
      get_script_name(),
      "anm",
      "color",
      index
    )
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
    module.rewrite_select_parameter(
      get_script_name(),
      "anm",
      index,
      choices
    )
  end
end

function rikky_module.fileCS(...)
  local indices = { ... }
  for _, index in ipairs(indices) do
    module.rewrite_parameter(
      get_script_name(),
      "obj",
      "file",
      index
    )
  end
end

function rikky_module.foldCS(...)
  local indices = { ... }
  for _, index in ipairs(indices) do
    module.rewrite_parameter(
      get_script_name(),
      "obj",
      "folder",
      index
    )
  end
end

function rikky_module.fontCS(...)
  local indices = { ... }
  for _, index in ipairs(indices) do
    module.rewrite_parameter(
      get_script_name(),
      "obj",
      "font",
      index
    )
  end
end

function rikky_module.listCS(...)
  local indices = { ... }
  for i = 1, #indices, 2 do
    local index = indices[i]
    local choices = indices[i + 1]
    module.rewrite_select_parameter(
      get_script_name(),
      "obj",
      index,
      choices
    )
  end
end

function rikky_module.checkboxCS(...)
  local indices = { ... }
  for _, index in ipairs(indices) do
    module.rewrite_parameter(
      get_script_name(),
      "obj",
      "check",
      index
    )
  end
end

function rikky_module.colordialogCS(...)
  local indices = { ... }
  local current = 1
  while current <= #indices do
    local index = indices[current]
    module.rewrite_parameter(
      get_script_name(),
      "anm",
      "color",
      index
    )
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
  if type(value) == "table" then
    return unpack(value, 1, count)
  end
  assert(type(value) == "string", "Parameter value must be a string or a generated value table")

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
  return unpack(defaults, 1, count)
end

function rikky_module.parameter(value, index, definition)
  return parameter(value, index, definition, "anm")
end

function rikky_module.parameterCS(value, index, definition)
  return parameter(value, index, definition, "obj")
end

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

local function image_key(id)
  if type(id) == "string" then
    assert(not id:find("\0", 1, true), "Image ID contains NUL")
    return "s:" .. id
  end
  assert(type(id) == "number" and id > -math.huge and id < math.huge, "Invalid image ID")
  if id == 0 then
    id = 0
  end -- -0 と 0 は同じID。
  return "n:" .. string.format("%.17g", id)
end

local function image_integer(value, minimum, maximum)
  assert(
    type(value) == "number" and value == math.floor(value) and value >= minimum and value <= maximum,
    "Invalid image integer"
  )
  return value
end

local function save_image_file(file, format, quality)
  assert(type(file) == "string" and not file:find("\0", 1, true), "Invalid image filename")
  if file == "" then
    return
  end
  if quality == nil then
    quality = 100
  else
    quality = tonumber(quality)
    assert(quality ~= nil and quality > -math.huge and quality < math.huge, "Invalid JPEG quality")
    quality = math.max(1, math.min(100, math.floor(quality)))
  end
  local data, width, height = obj.getpixeldata("object", "rgba")
  if data == nil or width == 0 or height == 0 then
    return
  end
  module.image_save_file(file, format, data, width, height, quality)
end

function rikky_module.png(file)
  save_image_file(file, "png")
end

function rikky_module.jpg(file, quality)
  save_image_file(file, "jpg", quality)
end

function rikky_module.bmp(file)
  save_image_file(file, "bmp")
end

local function load_image(data, width, height, lease, reset)
  if reset and not obj.load("figure", "四角形", 0, 1) then
    module.image_release(lease)
    return false
  end
  obj.clearbuffer("object", width, height)
  obj.putpixeldata("object", data, width, height, "rgba")
  module.image_release(lease)
  return true
end

function rikky_module.image(mode, id, a, b, c, d)
  if mode == "w" or mode == "w+" then
    local key = image_key(id)
    local target = "object"
    if mode == "w" and a ~= nil then
      assert(a == "object" or a == "tempbuffer", "Invalid image buffer")
      target = a
    end
    local data, width, height = obj.getpixeldata(target, "rgba")
    if data == nil or width == 0 or height == 0 then
      return false
    end
    local alpha = 1
    if mode == "w+" then
      alpha = obj.alpha * obj.getvalue("alpha")
    end
    return module.image_write(key, data, width, height, alpha, false)
  elseif mode == "r" or mode == "r+" or mode == "i" or mode == "i+" then
    local data, width, height, lease = module.image_read(image_key(id), mode == "i")
    if data == nil then
      return false
    end
    if mode == "i" then
      module.image_release(lease)
      return data, width, height
    elseif mode == "i+" then
      local r, g, blue, alpha = module.image_channels(lease)
      module.image_release(lease)
      return { A = alpha, R = r, G = g, B = blue }, width, height
    end
    return load_image(data, width, height, lease, mode == "r")
  elseif mode == "c" then
    return module.image_delete(image_key(id))
  elseif mode == "c+" then
    return module.image_delete(nil)
  elseif mode == "g" or mode == "g+" then
    local count = 0
    local single = false
    if mode == "g" then
      if id == nil or id == 0 then
        count, single = 1, true
      else
        count = image_integer(id, 1, 2147483647)
      end
    end
    local ids = module.image_ids(count)
    for i, key in ipairs(ids) do
      if key:sub(1, 2) == "n:" then
        ids[i] = tonumber(key:sub(3))
      else
        ids[i] = key:sub(3)
      end
    end
    if single then
      return ids[1]
    end
    return ids
  elseif mode == "m" or mode == "m+" then
    if b == nil then
      b = 0
    end
    if c == nil then
      c = 0
    end
    local x = image_integer(b, -2147483648, 2147483647)
    local y = image_integer(c, -2147483648, 2147483647)
    local data, width, height, lease = module.image_merge(image_key(id), image_key(a), x, y, mode == "m")
    if data == nil then
      return false
    end
    if mode == "m" then
      module.image_release(lease)
      return data, width, height
    end
    return load_image(data, width, height, lease, false)
  elseif mode == "p" or mode == "p+" then
    local key = image_key(id)
    if type(a) == "userdata" then
      local width = image_integer(b, 1, 2147483647)
      local height = image_integer(c, 1, 2147483647)
      return module.image_write(key, a, width, height, 1, mode == "p+")
    end
    return module.image_copy(key, image_key(a), mode == "p+")
  elseif mode == "u" or mode == "u+" then
    local data, width, height, lease, y
    local x = image_integer(a, -9007199254740991, 9007199254740991)
    if type(id) == "userdata" then
      data = id
      if d == nil then
        width, height = b, c
      else
        y, width, height = b, c, d
      end
      width = image_integer(width, 1, 2147483647)
      height = image_integer(height, 1, 2147483647)
    else
      y = b
    end
    if y ~= nil then
      y = image_integer(y, -9007199254740991, 9007199254740991)
    end
    if data == nil then
      data, width, height, lease = module.image_read(image_key(id), false)
      if data == nil then
        return false
      end
    end
    local index = x
    if y ~= nil then
      if x < 1 or x > width or y < 1 or y > height then
        if lease ~= nil then
          module.image_release(lease)
        end
        return false
      end
      index = (y - 1) * width + x - 1
    end
    local r, g, blue, alpha = module.image_pixel(data, width, height, index)
    if lease ~= nil then
      module.image_release(lease)
    end
    if r == nil then
      return false
    end
    if mode == "u+" then
      return r, g, blue, alpha
    end
    return r * 65536 + g * 256 + blue, alpha / 255
  end
  error("Unknown image mode: " .. tostring(mode))
end

local function finite_number(value)
  assert(type(value) == "number" and value > -math.huge and value < math.huge, "Expected a finite number")
  return value
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

local text_color = string.rep("[0-9a-f]", 6)
local text_coordinate = "[+-]?%d*%.?%d*"
local text_tag_patterns = {
  s = { "^%d*$", "^%d*,[^,]*$", "^%d*,[^,]*,[BI]*$" },
  ["#"] = { "^$", "^" .. text_color .. "$", "^" .. text_color .. "," .. text_color .. "$" },
  p = {
    "^" .. text_coordinate .. "," .. text_coordinate .. "$",
    "^" .. text_coordinate .. "," .. text_coordinate .. "," .. text_coordinate .. "$",
  },
  c = { "^%*?%d*%.?%d*$" },
  w = { "^%*?%d*%.?%d*$" },
  r = { "^%d*%.?%d*$" },
}

function rikky_module.textsplit(text, extra_tags)
  if type(text) ~= "string" then
    return ""
  end
  local result, position = {}, 1
  while position <= #text do
    local _, ending
    local first = text:sub(position, position)
    if first == "&" then
      _, ending = text:find("^&#%d+;", position)
    elseif first == "<" then
      local kind = text:sub(position + 1, position + 1)
      if kind == "?" then
        _, ending = text:find("?>", position + 2, true)
      elseif text_tag_patterns[kind] then
        local closing = text:find(">", position + 2, true)
        if closing then
          local body = text:sub(position + 2, closing - 1)
          for _, pattern in ipairs(text_tag_patterns[kind]) do
            if body:match(pattern) then
              ending = closing
              break
            end
          end
        end
      elseif type(extra_tags) == "table" then
        for i = 1, #extra_tags do
          local tag = rawget(extra_tags, i)
          if type(tag) == "string" and tag ~= "" and text:sub(position + 1, position + #tag) == tag then
            ending = text:find(">", position + 1 + #tag, true)
            if ending then
              break
            end
          end
        end
      end
    end
    if not ending then
      -- AviUtl2から渡される文字列はUTF-8。次の先頭バイトまでを1文字として扱う。
      local next_position = text:find("[^\128-\191]", position + 1)
      if next_position then
        ending = next_position - 1
      else
        ending = #text
      end
    end
    result[#result + 1] = text:sub(position, ending)
    position = ending + 1
  end
  return result
end

function rikky_module.string2table(source)
  if type(source) ~= "string" then
    return
  end
  -- 旧版と同じくLuaの式として評価するため、変数参照や入れ子のテーブルも使える。
  local evaluate = assert(loadstring("return " .. source, "string2table"))
  local result = evaluate()
  if type(result) == "table" then
    return result
  end
  return nil
end

function rikky_module.textload(text, alignment, center, orientation, font, size)
  assert(type(text) == "string", "Expected text")
  if alignment == nil then
    alignment = 0
  end
  if center == nil then
    center = 4
  end
  if orientation == nil then
    orientation = 0
  end
  assert(alignment == 0 or alignment == 1 or alignment == 2, "Invalid text alignment")
  assert(finite_number(center) == math.floor(center) and center >= 0 and center <= 8, "Invalid text center")
  assert(orientation == 0 or orientation == 1, "Invalid text orientation")

  local font_control, size_control = "", ""
  if font ~= nil then
    assert(type(font) == "string", "Expected font name")
    font_control = "," .. font
  end
  if size ~= nil then
    size_control = string.format("%d", finite_number(size))
  end
  if font ~= nil or size ~= nil then
    text = "<s" .. size_control .. font_control .. ">" .. text
  end

  if obj.load("text", text, 0, 0, alignment + orientation * 9) then
    -- alignによる中心座標を、第3引数の指定で上書きする。
    obj.cx = (center % 3 - 1) * obj.w / 2
    obj.cy = (math.floor(center / 3) - 1) * obj.h / 2
    obj.cz = 0
  end
end

local function clamp(value, minimum, maximum)
  return math.max(minimum, math.min(maximum, value))
end

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

function rikky_module.camerainfo(args)
  if args == nil then
    args = {}
  end
  assert(type(args) == "table", "Expected draw arguments")
  local count = #args
  assert(count <= 8 or count == 12 or count == 20 or count == 21, "Invalid draw argument count")
  local values = {}
  for i = 1, count do
    values[i] = finite_number(args[i])
  end
  local function argument(index, default)
    if index > count then
      return default
    end
    return values[index]
  end
  local function transform(m, x, y, z)
    return m[1] * x + m[2] * y + m[3] * z,
        m[4] * x + m[5] * y + m[6] * z,
        m[7] * x + m[8] * y + m[9] * z
  end

  local camera = obj.getoption("camera_param")
  local ex, ey, ez = unit_vector(camera.tx - camera.x, camera.ty - camera.y, camera.tz - camera.z)
  local nx, ny, nz = unit_vector(
    ey * camera.uz - ez * camera.uy,
    ez * camera.ux - ex * camera.uz,
    ex * camera.uy - ey * camera.ux
  )
  local ux, uy, uz = ny * ez - nz * ey, nz * ex - nx * ez, nx * ey - ny * ex
  local roll = math.rad(camera.rz)
  local cos_roll, sin_roll = math.cos(roll), math.sin(roll)
  local group = rikky_module.getinfo("group")
  local group_matrix = {
    group.Xx, group.Yx, group.Zx,
    group.Xy, group.Yy, group.Zy,
    group.Xz, group.Yz, group.Zz,
  }
  local x, y, z, rx, ry, rz, zoom
  local ax, ay, az, mx, my, mz = 1, 0, 0, 0, 0, -1
  local base_zoom = obj.getvalue("zoom") / 100
  local base_aspect = obj.getvalue("aspect") / 100
  local scale = obj.zoom * base_zoom
  local scale_x = scale * (1 - math.max(obj.aspect, 0)) * (1 - math.max(base_aspect, 0))
  local scale_y = scale * (1 + math.min(obj.aspect, 0)) * (1 + math.min(base_aspect, 0))
  if count <= 8 then
    zoom = argument(4, 1)
    x, y, z = -obj.cx * scale_x * zoom, -obj.cy * scale_y * zoom, -obj.cz
    rx, ry, rz = argument(6, 0), argument(7, 0), argument(8, 0)
  else
    -- drawpolyの中心は4頂点の平均。UVと不透明度は位置に影響しない。
    x, y, z = 0, 0, 0
    for i = 1, 12, 3 do
      x, y, z = x + values[i] / 4, y + values[i + 1] / 4, z + values[i + 2] / 4
    end
    -- 頂点を重ねて指定する三角形にも対応する。
    local edge = 4
    while edge <= 10 and values[edge] == values[1]
        and values[edge + 1] == values[2] and values[edge + 2] == values[3] do
      edge = edge + 3
    end
    assert(edge <= 7, "Polygon must have at least three distinct vertices")
    ax, ay, az = unit_vector(values[edge] - values[1], values[edge + 1] - values[2], values[edge + 2] - values[3])
    local bx, by, bz
    for i = edge + 3, 10, 3 do
      bx, by, bz = values[i] - values[1], values[i + 1] - values[2], values[i + 2] - values[3]
      if by * az - bz * ay ~= 0 or bz * ax - bx * az ~= 0 or bx * ay - by * ax ~= 0 then
        break
      end
    end
    mx, my, mz = unit_vector(by * az - bz * ay, bz * ax - bx * az, bx * ay - by * ax)
    x = x * obj.zoom * (1 - math.max(obj.aspect, 0)) - obj.cx * scale_x
    y = y * obj.zoom * (1 + math.min(obj.aspect, 0)) - obj.cy * scale_y
    z = z * obj.zoom - obj.cz * scale
    rx, ry, rz, zoom = 0, 0, 0, 1
  end
  local rotation = multiply_matrix(
    rotation_matrix(1, 0, 0, math.rad(obj.rx + rx)),
    multiply_matrix(rotation_matrix(0, 1, 0, math.rad(obj.ry + ry)),
      rotation_matrix(0, 0, 1, math.rad(obj.rz + rz)))
  )
  local billboard = obj.getoption("billboard")
  local use_billboard = billboard ~= 0 and obj.getoption("camera_mode") ~= 0
  if use_billboard then
    local basis
    if billboard == 1 then
      -- 横方向のみ追従する。
      basis = rotation_matrix(0, 1, 0, math.atan2(ex, ez))
    elseif billboard == 2 then
      -- 縦横方向に追従するが、傾きは反映しない。
      basis = { nx, -ux, ex, ny, -uy, ey, nz, -uz, ez }
    else
      -- カメラの右・下・前をオブジェクトの基底にする。
      basis = {
        nx * cos_roll + ux * sin_roll, nx * sin_roll - ux * cos_roll, ex,
        ny * cos_roll + uy * sin_roll, ny * sin_roll - uy * cos_roll, ey,
        nz * cos_roll + uz * sin_roll, nz * sin_roll - uz * cos_roll, ez,
      }
    end
    rotation = multiply_matrix(basis, rotation)
  end
  x, y, z = transform(rotation, x, y, z)
  ax, ay, az = transform(rotation, ax, ay, az)
  mx, my, mz = transform(rotation, mx, my, mz)
  if use_billboard then
    -- 基準位置にはグループ回転を適用するが、カメラ基準の面には重ねて適用しない。
    x, y, z = transform({
      group.Xx, group.Xy, group.Xz,
      group.Yx, group.Yy, group.Yz,
      group.Zx, group.Zy, group.Zz,
    }, x, y, z)
  end
  x, y, z = x + obj.x + obj.ox, y + obj.y + obj.oy, z + obj.z + obj.oz
  if count <= 8 then
    x, y, z = x + argument(1, 0), y + argument(2, 0), z + argument(3, 0)
  end
  x, y, z = transform(group_matrix, x, y, z)
  x, y, z = x * group.zoom + group.x, y * group.zoom + group.y, z * group.zoom + group.z
  if not use_billboard then
    ax, ay, az = transform(group_matrix, ax, ay, az)
    mx, my, mz = transform(group_matrix, mx, my, mz)
  end
  local vx, vy, vz = x - camera.x, y - camera.y, z - camera.z
  local normal_d = vx * ex + vy * ey + vz * ez
  local real_d = math.sqrt(vx * vx + vy * vy + vz * vz)
  assert(normal_d ~= 0, "Cannot project an object on the camera plane")
  local perspective = camera.d / normal_d
  local ix = (vx * nx + vy * ny + vz * nz) * perspective
  local iy = -(vx * ux + vy * uy + vz * uz) * perspective
  return {
    x = camera.x, y = camera.y, z = camera.z,
    tx = camera.tx, ty = camera.ty, tz = camera.tz,
    ux = ux, uy = uy, uz = uz,
    ex = ex, ey = ey, ez = ez,
    nx = nx, ny = ny, nz = nz,
    rz = roll, d = camera.d, blur = obj.getoption("camera_focus").bokeh,
    normal_d = normal_d, real_d = real_d,
    vx = vx / real_d, vy = vy / real_d, vz = vz / real_d,
    mx = mx, my = my, mz = mz,
    ax = ax, ay = ay, az = az,
    ix = ix * cos_roll - iy * sin_roll,
    iy = ix * sin_roll + iy * cos_roll,
    zoom = perspective * scale * zoom * group.zoom,
    -- AviUtl1のカメラ制御のシャドーに対応する取得APIはない。
    shadow_x = 0, shadow_y = 0, shadow_z = 0,
    shadow_accu = 0, shadow_conce = 0, shadow_flag = 0,
    billboard = billboard,
  }
end

function rikky_module.fillarea(x, y, mode, threshold)
  x, y = finite_number(x), finite_number(y)
  assert(x == math.floor(x) and y == math.floor(y), "Invalid fill coordinates")
  mode = image_integer(mode, 0, 24)
  if threshold == nil then
    threshold = 0
  end
  threshold = finite_number(threshold)
  local maximum = ({ 255, 360, 100, 100, 255 })[math.floor(mode / 5) + 1]
  assert(threshold >= 0 and threshold <= maximum, "Invalid fill threshold")
  local data, width, height = obj.getpixeldata("object", "rgba")
  if data == nil or x < 0 or y < 0 or x >= width or y >= height then
    return false
  end
  local mask, mask_width, mask_height, lease, bounds = module.fillarea(data, width, height, x, y, mode, threshold)
  if mask == nil then
    return false
  end
  -- 同じ寸法の画像を書き戻す。書き戻し失敗時にも所有メモリを解放する。
  local ok, err = pcall(obj.putpixeldata, "object", mask, mask_width, mask_height, "rgba")
  module.image_release(lease)
  if not ok then
    error(err, 0)
  end
  return bounds[1], bounds[2], bounds[3], bounds[4]
end

function rikky_module.bordering(resolution, threshold, is_zoom, is_rotate, hq)
  local skip = 0
  if type(resolution) == "number" then
    skip = math.floor(clamp(finite_number(resolution), 0, 5000))
  end
  if type(threshold) ~= "number" then
    threshold = 0
  end
  local pixel = resolution == "pixel"
  local zoom = 1
  if not pixel and (is_zoom == true or is_zoom == 1) then
    zoom = obj.getvalue("zoom") * obj.zoom / 100
  end
  local matrix
  if not pixel and (is_rotate == true or is_rotate == 1) then
    matrix = multiply_matrix(
      rotation_matrix(1, 0, 0, math.rad(obj.rx)),
      multiply_matrix(rotation_matrix(0, 1, 0, math.rad(obj.ry)), rotation_matrix(0, 0, 1, math.rad(obj.rz)))
    )
  end
  local data, width, height = obj.getpixeldata("object", "rgba")
  if data == nil or width == 0 or height == 0 then
    return {}, {}, 0
  end
  local indices, counts = module.bordering(data, width, height, skip, threshold, hq == true or hq == 1)
  local points, offset = {}, 1
  for i, count in ipairs(counts) do
    local contour = {}
    for j = 1, count do
      local index = indices[offset]
      offset = offset + 1
      local x, y = index % width, math.floor(index / width)
      if not pixel then
        x, y = (x + (1 - width) / 2) * zoom, (y + (1 - height) / 2) * zoom
      end
      if matrix then
        contour[j * 3 - 2] = matrix[1] * x + matrix[2] * y
        contour[j * 3 - 1] = matrix[4] * x + matrix[5] * y
        contour[j * 3] = matrix[7] * x + matrix[8] * y
      else
        contour[j * 2 - 1], contour[j * 2] = x, y
      end
    end
    points[i] = contour
  end
  return points, counts, #counts
end

function rikky_module.linedetection(precision, background, centered)
  if precision == nil then
    precision = 80
  end
  assert(finite_number(precision) > 0, "Detection precision must be positive")
  if background == nil then
    background = 0
  end
  background = image_integer(background, 0, 0xFFFFFF)
  local data, width, height = obj.getpixeldata("object", "rgba")
  if data == nil or width == 0 or height == 0 then
    return false
  end
  local coordinates = module.linedetection(data, width, height, precision / 100, background)
  if #coordinates == 0 then
    return false
  end
  local ox, oy = 0, 0
  if centered == true then
    ox, oy = width / 2, height / 2
  end
  local lines = {}
  for i = 1, #coordinates, 4 do
    lines[#lines + 1] = {
      x0 = coordinates[i] - ox,
      y0 = coordinates[i + 1] - oy,
      x1 = coordinates[i + 2] - ox,
      y1 = coordinates[i + 3] - oy,
    }
  end
  return lines, #lines
end

local function aviutl_angles(matrix, radians)
  -- AviUtl の回転は Rx * Ry * Rz（座標にはZ、Y、Xの順に適用）。
  -- 旧版と同じく cos(Y) <= 0 の解を選ぶ。
  local cosine = math.sqrt(matrix[1] * matrix[1] + matrix[2] * matrix[2])
  local y = math.atan2(matrix[3], -cosine)
  local x, z
  if cosine > 1e-8 then
    x = math.atan2(matrix[6], -matrix[9])
    z = math.atan2(matrix[2], -matrix[1])
  else
    -- ジンバルロック時は分離できないX/Zを、同じ姿勢になるZ=-piに固定する。
    x, z = math.atan2(matrix[8], matrix[5]) + math.pi, -math.pi
  end
  x = (x + math.pi) % (2 * math.pi) - math.pi
  y = (y + math.pi) % (2 * math.pi) - math.pi
  z = (z + math.pi) % (2 * math.pi) - math.pi
  if radians == 1 then
    return x, y, z
  end
  return math.deg(x), math.deg(y), math.deg(z)
end

local previous_rotation, previous_center

function rikky_module.rotation(x, y, z, angle, axis, center)
  x, y, z = finite_number(x), finite_number(y), finite_number(z)
  if angle ~= nil then
    if axis == "X" then
      axis = { 1, 0, 0 }
    elseif axis == "Y" then
      axis = { 0, 1, 0 }
    elseif axis == "Z" then
      axis = { 0, 0, 1 }
    end
    assert(type(axis) == "table", "Invalid rotation axis")
    local matrix = rotation_matrix(axis[1], axis[2], axis[3], angle)
    if center == nil then
      center = { 0, 0, 0 }
    end
    assert(type(center) == "table", "Invalid rotation center")
    local origin = { finite_number(center[1]), finite_number(center[2]), finite_number(center[3]) }
    previous_rotation, previous_center = matrix, origin
  else
    assert(axis == nil and center == nil and previous_rotation ~= nil, "No previous rotation")
  end
  local m, origin = previous_rotation, previous_center
  x, y, z = x - origin[1], y - origin[2], z - origin[3]
  return m[1] * x + m[2] * y + m[3] * z + origin[1],
      m[4] * x + m[5] * y + m[6] * z + origin[2],
      m[7] * x + m[8] * y + m[9] * z + origin[3]
end

function rikky_module.axisconvertEx(axes, radians, moving)
  assert(type(axes) == "table" and #axes % 4 == 0, "Axes must contain groups of four numbers")
  if radians == nil then
    radians = 0
  end
  if moving == nil then
    moving = 0
  end
  assert(radians == 0 or radians == 1, "Invalid angle unit")
  assert(moving == 0 or moving == 1, "Invalid moving-axis option")
  local basis = { 1, 0, 0, 0, 1, 0, 0, 0, 1 }
  if axes.Xx ~= nil or axes.Xy ~= nil or axes.Xz ~= nil or axes.Yx ~= nil or axes.Yy ~= nil or axes.Yz ~= nil then
    local xx, xy, xz = unit_vector(axes.Xx, axes.Xy, axes.Xz)
    local yx, yy, yz = unit_vector(axes.Yx, axes.Yy, axes.Yz)
    assert(math.abs(xx * yx + xy * yy + xz * yz) < 1e-6, "Initial axes must be orthogonal")
    basis = { xx, yx, xy * yz - xz * yy, xy, yy, xz * yx - xx * yz, xz, yz, xx * yy - xy * yx }
  end
  local matrix = { 1, 0, 0, 0, 1, 0, 0, 0, 1 }
  for i = 1, #axes, 4 do
    local angle = finite_number(axes[i + 3])
    if radians == 0 then
      angle = math.rad(angle)
    end
    local x, y, z = finite_number(axes[i]), finite_number(axes[i + 1]), finite_number(axes[i + 2])
    local rotation = rotation_matrix(x, y, z, angle)
    if moving == 1 then
      axes[i] = matrix[1] * x + matrix[2] * y + matrix[3] * z
      axes[i + 1] = matrix[4] * x + matrix[5] * y + matrix[6] * z
      axes[i + 2] = matrix[7] * x + matrix[8] * y + matrix[9] * z
      matrix = multiply_matrix(matrix, rotation)
    else
      matrix = multiply_matrix(rotation, matrix)
    end
  end
  return aviutl_angles(multiply_matrix(matrix, basis), radians)
end

function rikky_module.axisconvert(axes, radians)
  assert(type(axes) == "table", "Expected an axis table")
  return rikky_module.axisconvertEx({
    finite_number(axes.Zx),
    finite_number(axes.Zy),
    finite_number(axes.Zz),
    finite_number(axes.rz),
    finite_number(axes.Yx),
    finite_number(axes.Yy),
    finite_number(axes.Yz),
    finite_number(axes.ry),
    finite_number(axes.Xx),
    finite_number(axes.Xy),
    finite_number(axes.Xz),
    finite_number(axes.rx),
    Xx = axes.Xx,
    Xy = axes.Xy,
    Xz = axes.Xz,
    Yx = axes.Yx,
    Yy = axes.Yy,
    Yz = axes.Yz,
  }, radians, 0)
end

local function rgb_color(r, g, b)
  return RGB(math.floor(clamp(r, 0, 255) + 0.5), math.floor(clamp(g, 0, 255) + 0.5), math.floor(clamp(b, 0, 255) + 0.5))
end

local function rgb_to_xyz(r, g, b)
  -- 旧版はRGB成分を線形値として扱い、ガンマ補正を行わない。
  r, g, b = r / 255 * 100, g / 255 * 100, b / 255 * 100
  return 0.412391 * r + 0.357584 * g + 0.180481 * b,
      0.212639 * r + 0.715169 * g + 0.072192 * b,
      0.019331 * r + 0.119195 * g + 0.950532 * b
end

local function xyz_color(x, y, z)
  x, y, z = x / 100 * 255, y / 100 * 255, z / 100 * 255
  return rgb_color(
    3.24096637658435 * x - 1.53737885234726 * y - 0.498611723228325 * z,
    -0.969242037979635 * x + 1.87596526849091 * y + 0.041555768342051 * z,
    0.0556295671173938 * x - 0.203976940895256 * y + 1.0569716994422 * z
  )
end

local function lab_curve(value)
  if value > (6 / 29) ^ 3 then
    return value ^ (1 / 3)
  end
  return value / (3 * (6 / 29) ^ 2) + 4 / 29
end

local function inverse_lab_curve(value)
  if value > 6 / 29 then
    return value ^ 3
  end
  return 3 * (6 / 29) ^ 2 * (value - 4 / 29)
end

function rikky_module.colorconvert(mode, a, b, c, d)
  a = finite_number(a)
  if b == nil and c == nil and d == nil then
    assert(a >= 0 and a <= 0xffffff and a == math.floor(a), "Invalid RGB color")
    local r, g, blue = RGB(a)
    if mode == "rgb" then
      return r, g, blue
    elseif mode == "hsv" then
      return HSV(a)
    elseif mode == "opposite" then
      return RGB(255 - r, 255 - g, 255 - blue)
    elseif mode == "complemntary" or mode == "complementary" then
      local hue, saturation, value = HSV(a)
      return HSV((hue + 180) % 360, saturation, value)
    elseif mode == "hsl" then
      local high, low = math.max(r, g, blue) / 255, math.min(r, g, blue) / 255
      local lightness = (high + low) / 2
      local saturation = 0
      if high ~= low then
        saturation = (high - low) / (1 - math.abs(2 * lightness - 1))
      end
      return HSV(a), saturation * 100, lightness * 100
    elseif mode == "yc" then
      r, g, blue = r / 255, g / 255, blue / 255
      local y = 0.299 * r + 0.587 * g + 0.114 * blue
      return math.floor(y * 4096 + 0.5),
          math.floor((blue - y) / 1.772 * 4096 + 0.5),
          math.floor((r - y) / 1.402 * 4096 + 0.5)
    elseif mode == "xyz" then
      return rgb_to_xyz(r, g, blue)
    elseif mode == "lab" then
      local x, y, z = rgb_to_xyz(r, g, blue)
      x, y, z = lab_curve(x / 95.0456), lab_curve(y / 100), lab_curve(z / 108.9058)
      return 116 * y - 16, 500 * (x - y), 200 * (y - z)
    elseif mode == "cmy" then
      return (1 - r / 255) * 100, (1 - g / 255) * 100, (1 - blue / 255) * 100
    elseif mode == "cmyk" then
      local high = math.max(r, g, blue)
      if high == 0 then
        return 0, 0, 0, 100
      end
      return (1 - r / high) * 100, (1 - g / high) * 100, (1 - blue / high) * 100, (1 - high / 255) * 100
    end
  else
    b, c = finite_number(b), finite_number(c)
    if mode == "cmyk" then
      d = finite_number(d)
    else
      assert(d == nil, "Unexpected fourth color component")
    end
    if mode == "rgb" then
      return rgb_color(a, b, c)
    elseif mode == "hsv" then
      return HSV(a % 360, clamp(b, 0, 100), clamp(c, 0, 100))
    elseif mode == "hsl" then
      local saturation, lightness = clamp(b, 0, 100) / 100, clamp(c, 0, 100) / 100
      local value = lightness + saturation * math.min(lightness, 1 - lightness)
      if value == 0 then
        return 0
      end
      return HSV(a % 360, 2 * (1 - lightness / value) * 100, value * 100)
    elseif mode == "yc" then
      local y, cb, cr = a / 4096, b / 4096, c / 4096
      local r, blue = y + 1.402 * cr, y + 1.772 * cb
      return rgb_color(r * 255, (y - 0.299 * r - 0.114 * blue) / 0.587 * 255, blue * 255)
    elseif mode == "xyz" then
      return xyz_color(a, b, c)
    elseif mode == "lab" then
      local y = (a + 16) / 116
      return xyz_color(
        95.0456 * inverse_lab_curve(y + b / 500),
        100 * inverse_lab_curve(y),
        108.9058 * inverse_lab_curve(y - c / 200)
      )
    elseif mode == "cmy" then
      return rgb_color((1 - a / 100) * 255, (1 - b / 100) * 255, (1 - c / 100) * 255)
    elseif mode == "cmyk" then
      local black = 1 - clamp(d, 0, 100) / 100
      return rgb_color(
        (1 - clamp(a, 0, 100) / 100) * black * 255,
        (1 - clamp(b, 0, 100) / 100) * black * 255,
        (1 - clamp(c, 0, 100) / 100) * black * 255
      )
    end
  end
  error("Unknown color conversion: " .. tostring(mode))
end

local effect_stack = {}
local obj_fields = {
  "ox", "oy", "oz", "cx", "cy", "cz", "rx", "ry", "rz", "sx", "sy", "sz", "alpha", "zoom", "aspect"
}
local function create_obj_table()
  local obj_table = {}
  for _, field in ipairs(obj_fields) do
    obj_table[field] = obj[field]
  end
  return obj_table
end

-- NOTE: 以下のことを信じている：
-- - index + 1 == numの呼び出しは、effectの最後の呼び出しである。
-- - 0 <= index < num - 1の呼び出しはindex == num - 1の呼び出しの前にすべて行われる。
function rikky_module.effect(index, num, ...)
  local effect_args = { ... }
  if not effect_stack[obj.effect_id] then
    effect_stack[obj.effect_id] = {}
  end
  local current_obj = create_obj_table()
  effect_stack[obj.effect_id][index + 1] = { obj = current_obj, args = effect_args }
  rikky_module.image("w", ("effect_" .. obj.effect_id .. "_" .. index))
  if index == num - 1 then
    local effect_data = effect_stack[obj.effect_id]
    local current_index = -1
    obj.multiobject(num, function()
      current_index = current_index + 1
      rikky_module.image("r", ("effect_" .. obj.effect_id .. "_" .. current_index))
      rikky_module.image("c", ("effect_" .. obj.effect_id .. "_" .. current_index))
      local obj_state = effect_data[current_index + 1]
      for _, field in ipairs(obj_fields) do
        obj[field] = obj_state.obj[field]
      end
      obj.effect(unpack(obj_state.args))
    end)
  end
end

-- glassdrawの画像はeffectごとに所有し、別のスクリプトのinitで上書きしない。
local glass_states = {}
local glass_fields = {
  "ox", "oy", "oz", "cx", "cy", "cz", "rx", "ry", "rz", "sx", "sy", "sz", "alpha"
}

local function glass_number(value, default)
  local number = tonumber(value)
  if number == nil then return default end -- 元DLLのlua_tonumberと既定値
  return finite_number(number)
end

local function glass_integer(value, default)
  local number = glass_number(value, default)
  if number < 0 then return math.ceil(number) end
  return math.floor(number)
end

local function glass_capture(target)
  local data, width, height = obj.getpixeldata(target, "rgba")
  return module.glass_capture(data, width, height)
end

local function glass_groups()
  local groups = {}
  if obj.getoption("drawtarget") == "tempbuffer" or not obj.getoption("enable_group") then
    return groups
  end
  local index, previous = 0, nil
  while true do
    local layer = obj.getoption("group_info", index)
    if layer == 0 then break end
    -- index未対応のホストで無限に同じレイヤーを取得しない。
    assert(layer ~= previous, "glassdraw requires AviUtl2 2.1.10 or later")
    for _, field in ipairs({ "x", "y", "z", "cx", "cy", "cz", "rx", "ry", "rz", "sx", "sy", "sz" }) do
      groups[#groups + 1] = obj.getvalue("layer" .. layer .. "." .. field)
    end
    index, previous = index + 1, layer
  end
  return groups
end

local function glass_release_state(state)
  if state ~= nil then
    if state.original ~= nil then module.glass_release(state.original) end
    if state.background ~= nil then module.glass_release(state.background) end
  end
end

function rikky_module.glassdraw_init(settings)
  local id = obj.effect_id
  glass_release_state(glass_states[id])
  glass_states[id] = nil
  local state = {}
  local ok, err = pcall(function()
    local table_settings = type(settings) == "table"
    if not table_settings then settings = {} end
    local reverse = 0
    if glass_integer(settings.reverse, 0) == 1 then
      reverse = 3
    else
      if glass_integer(settings.reverseUp, 0) == 1 then reverse = reverse + 1 end
      if glass_integer(settings.reverseSide, 0) == 1 then reverse = reverse + 2 end
    end
    local color = glass_integer(settings.color, -1) % 4294967296
    if color >= 2147483648 then color = color - 4294967296 end
    local blur = glass_integer(settings.blur, 0) % 65536
    if blur >= 32768 then blur = blur - 65536 end
    state.blur = table_settings and clamp(blur, 0, 30) or -1
    local zoom = glass_number(settings.zoom, 1)
    if zoom <= 0 then zoom = 1 end
    local boundary, lens = 0, 0
    if settings.boundary == "loop" then boundary = 1 end
    if settings.boundary == "inverted" then boundary = 2 end
    if settings.lens == "convex" then lens = 1 end
    if settings.lens == "concave" then lens = 2 end
    state.settings = {
      color = color, reverse = reverse, boundary = boundary, lens = lens,
      culling = glass_integer(settings.culling, 0) % 256 == 1,
      refractive = clamp(glass_number(settings.refractive, 0), 0, 1),
      offset_z = math.max(0, glass_number(settings.offsetZ, 300)), inverse_zoom = 1 / zoom,
    }
    state.pose = { x = obj.x, y = obj.y, z = obj.z, billboard = obj.getoption("billboard") }
    for _, field in ipairs(glass_fields) do state.pose[field] = obj[field] end
    state.pose.base_sx, state.pose.base_sy, state.pose.base_sz = obj.getvalue("scale")
    state.camera = obj.getoption("camera_param")
    state.camera.mode = obj.getoption("camera_mode")
    state.groups = glass_groups()
    state.original = glass_capture("object")
    if glass_integer(settings.async, 0) == 1 then
      state.background = glass_capture("framebuffer")
    end
  end)
  if not ok then
    glass_release_state(state)
    error(err, 0)
  end
  glass_states[id] = state
end

function rikky_module.glassdraw(...)
  local state = glass_states[obj.effect_id]
  assert(state ~= nil, "Call glassdraw_init before glassdraw")
  local count, args = select("#", ...), { ... }
  assert(count <= 8 or count == 12 or count == 20 or count == 21, "Invalid glassdraw argument count")
  for i = 1, count do args[i] = finite_number(tonumber(args[i])) end
  local saved = {}
  for _, field in ipairs(glass_fields) do saved[field] = obj[field] end
  local background, output, modified
  local ok, err = pcall(function()
    background = state.background
    if background == nil then background = glass_capture("framebuffer") end
    output = module.glass_render(state.original, background, state.settings,
      state.pose, state.camera, args, state.groups)
    if output == nil then
      obj.setoption("draw_state", true)
      return
    end
    local data, width, height = module.glass_data(output)
    modified = true
    obj.clearbuffer("object", width, height)
    obj.putpixeldata("object", data, width, height, "rgba")
    if state.blur > 0 then obj.effect("ぼかし", "範囲", state.blur, "サイズ固定", 1) end
    if count > 8 then
      if count > 12 then
        -- 元DLLは明示UVも元画像全体に置き換える。
        args[13], args[14], args[15], args[16] = 0, 0, width, 0
        args[17], args[18], args[19], args[20] = width, height, 0, height
      end
      obj.drawpoly(unpack(args, 1, count))
    else
      obj.draw(unpack(args, 1, count))
    end
  end)
  local restore_ok, restore_err = true, nil
  if modified then
    restore_ok, restore_err = pcall(function()
      local data, width, height = module.glass_data(state.original)
      obj.clearbuffer("object", width, height)
      obj.putpixeldata("object", data, width, height, "rgba")
    end)
    -- zoom/aspectはsx/syの別名なので、軸別拡大率の復元後に書き戻さない。
    local fields_ok, fields_err = pcall(function()
      for _, field in ipairs(glass_fields) do obj[field] = saved[field] end
    end)
    if restore_ok and not fields_ok then restore_ok, restore_err = fields_ok, fields_err end
  end
  if output ~= nil then module.glass_release(output) end
  if background ~= nil and state.background == nil then module.glass_release(background) end
  if not restore_ok then
    if not ok then error(tostring(err) .. "\nImage restoration failed: " .. tostring(restore_err), 0) end
    error(restore_err, 0)
  end
  if not ok then error(err, 0) end
end

return rikky_module
