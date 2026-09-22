return function(rikky_module, module)
  local common = require("rikky_modoki.common")
  local finite_number = common.finite_number

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
end
