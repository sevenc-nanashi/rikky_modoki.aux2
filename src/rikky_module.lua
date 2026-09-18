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
    local group_layer = obj.getoption("group_info")
    local buffer = obj.getoption("drawtarget");
    if buffer == "tempbuffer" or group_layer == 0 then
      return {
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
      }, false
    end

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

    return {
          zoom = group_zoom,
          x = group_x,
          y = group_y,
          z = group_z,
          Xx = cy * cz,
          Xy = cx * sz + sx * sy * cz,
          Xz = sx * sz - cx * sy * cz,
          Yx = -cy * sz,
          Yy = cx * cz - sx * sy * sz,
          Yz = sx * cz + cx * sy * sz,
          Zx = sy,
          Zy = -sx * cy,
          Zz = cx * cy,
        },
        true
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

return rikky_module
