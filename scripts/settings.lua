---@class Settings
---@field editor_font string
---@field terminal_font string
---@field theme_variant string
---@field theme_url string

---@return Settings|boolean
local function get_current_settings(msg)
    local gists = github:get_gist_files(
        SETTINGS_GIST_ID
    ) or nil
    local settings = gists["vscode settings.json"] or nil
    if not settings then
        msg:reply("cannot get the current settings :(")
        return false
    end

    return json:from_str(settings)
end

---@type Command
local font = {
    command = "!font",
    help = "the current VSCode fonts",
    handler = function(msg, args)
        local settings = get_current_settings(msg)
        if not settings then
            return
        end

        msg:say(string.format("the editor is using '%s'",
            settings.editor_font
        ))

        msg:say(string.format("the editor terminal is using '%s'",
            settings.terminal_font
        ))
    end
}

---@type Command
local theme = {
    command = "!theme",
    help = "the current VSCode theme",
    handler = function(msg, args)
        local settings = get_current_settings(msg)
        if not settings then
            return
        end

        msg:say(string.format("%s from %s",
            settings.theme_variant,
            settings.theme_url
        ))
    end
}

---@type Command[]
return { theme, font }
