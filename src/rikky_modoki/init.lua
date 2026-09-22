local module = obj.module("rikky_modoki")
local api = {}
local draw = require("rikky_modoki.draw_common")(module)
local material = require("rikky_modoki.material_common")(draw)

require("rikky_modoki.info")(api, module)
require("rikky_modoki.dialog")(api, module)
require("rikky_modoki.utility")(api, module)
require("rikky_modoki.image")(api, module)
require("rikky_modoki.audio")(api, module)
require("rikky_modoki.ui")(api, module)
require("rikky_modoki.text")(api, module)
require("rikky_modoki.camera")(api, module)
require("rikky_modoki.transform")(api, module)
require("rikky_modoki.color")(api, module)
require("rikky_modoki.effect")(api, module)
require("rikky_modoki.glass")(api, module, draw)
require("rikky_modoki.material")(api, module, draw, material)
require("rikky_modoki.material_ex")(api, module, draw, material)

rikky_module = api
return api
