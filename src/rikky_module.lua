local module = obj.module("rikky_modoki")
rikky_module = {}

local function to_sjis(str)
  local sjis_bytes = module.to_sjis(str)
  local sjis_str = ""
  local stack_size = 100
  for i = 1, #sjis_bytes, stack_size do
    local chunk = {}
    for j = i, math.min(i + stack_size - 1, #sjis_bytes) do
      table.insert(chunk, string.char(sjis_bytes[j]))
    end
    sjis_str = sjis_str .. table.concat(chunk)
  end
  return sjis_str
end

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
      return to_sjis(module.project_path(true))
    else
      return to_sjis(module.project_path(false))
    end
  elseif target == "output" then
    -- 出力中のファイル名を返すが、AviUtl2にはそういうAPIがないので断念
    return ""
  elseif target == "state" then
    return module.edit_state()
  elseif target == "path" then
    if option == 2 then
      return to_sjis(module.desktop_dir())
    else
      return to_sjis(module.project_dir())
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
      return to_sjis(obj.getvalue("テキスト", "テキスト"))
    else
      return to_sjis(obj.getvalue(option, "テキスト", "テキスト"))
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
      return to_sjis(script_name), {
        file = to_sjis(obj.getvalue(obj.layer, script_name, "ファイル")),
        loop = tonumber(obj.getvalue(obj.layer, script_name, "ループ再生")),
        alphachannel = 1
      }
    elseif script_name == "画像ファイル" then
      return to_sjis(script_name), {
        file = to_sjis(obj.getvalue(obj.layer, script_name, "ファイル")),
      }
    elseif script_name == "テキスト" then
      return to_sjis(script_name), {
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
        font = to_sjis(obj.getvalue(obj.layer, script_name, "フォント")),
        individual = tonumber(obj.getvalue(obj.layer, script_name, "文字毎に個別オブジェクト")),
        display = tonumber(obj.getvalue(obj.layer, script_name, "移動座標上に表示")),
        autoscroll = tonumber(obj.getvalue(obj.layer, script_name, "自動スクロール")),
        bold = tonumber(obj.getvalue(obj.layer, script_name, "B")),
        italic = tonumber(obj.getvalue(obj.layer, script_name, "I")),
      }
    elseif script_name == "図形" then
      return to_sjis(script_name), {
        color = tonumber(obj.getvalue(obj.layer, script_name, "色"), 16),
        figure = to_sjis(obj.getvalue(obj.layer, script_name, "図形の種類")),
      }
    elseif script_name == "フレームバッファ" then
      return to_sjis(script_name), {
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
      return to_sjis("音声波形"), {
        color = tonumber(obj.getvalue(obj.layer, script_name, "波形の色"), 16),
        projectsound = file and 0 or 1,
        type = wave_type,
        file = file and to_sjis(file),
        mode = mode,
        res_w = tonumber(obj.getvalue(obj.layer, script_name, "横解像度")),
        res_h = tonumber(obj.getvalue(obj.layer, script_name, "縦解像度")),
        pad_w = pad_w,
        pad_h = pad_h,
        mirror = mirror,
      }
    elseif script_name == "シーン" then
      return to_sjis(script_name), {
        scenenumber = tonumber(obj.getvalue(obj.layer, script_name, "シーン")),
        loop = tonumber(obj.getvalue(obj.layer, script_name, "ループ再生")),
      }
    elseif script_name == "カメラ制御" then
      return to_sjis(script_name), {
        zbuffer = 1
      }
    elseif script_name == "直前オブジェクト" or script_name == "フィルタオブジェクト" or script_name == "グループ制御" then
      return to_sjis(script_name), {}
    else
      return to_sjis("カスタムオブジェクト")
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
      table.insert(result, to_sjis(prevs[i]))
    end
    table.insert(result, to_sjis(obj.getoption("script_name")))
    for i = 1, #afters do
      table.insert(result, to_sjis(afters[i]))
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
      name = to_sjis(name),
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
      "font",
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
      "font",
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

return rikky_module
