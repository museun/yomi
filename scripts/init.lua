---@type Manifest
return {
    commands = {
        ["help"] = require("help"),
        ["twitch"] = require("twitch"),
        ["crates"] = require("crates"),
        ["commands"] = require("commands"),
        ["greeting"] = require("greeting"),
        ["settings"] = require("settings"),
        ["answers"] = require("answers"),
        ["spotify"] = require("spotify"),
        ["aliases"] = require("aliases"),
    },
    listeners = {
        require("another_viewer"),
    }
}
