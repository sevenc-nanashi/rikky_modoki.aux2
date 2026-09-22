return function(rikky_module, module)
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
end
