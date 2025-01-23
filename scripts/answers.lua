---@class response
---@field body string?
---@field command string?

---@type {[Pattern]: response}|{}
local answers = {
    [re.compile("(?i).*?playlist.*?(\\?)?$")] = {
        body = "museun don't use a playlist, they just pick a song and let spotify play from there",
    },
    [re.compile("(?i)((((editor|ide) is (this|.?*using).*?)|\z
                (ide|editor))\\?)|\z
                (what editor\\s*?(is (this|that))?)\\??")] = {
        command = "!editor",
    },
    [re.compile("(?i)song(\\sname)?\\?")] = {
        command = "!song",
    },
    [re.compile("(?i)what theme.*?\\?")] = {
        command = "!theme",
    },
    [re.compile("(?i)what font.*?\\?")] = {
        command = "!font",
    },
    [re.compile("(?i)how can I (make|get) my (editor|vsc)\\?")] = {
        command = "!settings",
    },
    [re.compile("(?i)what os\\s*?((are you using)|(is this))?\\??")] = {
        command = "!os",
    },
    [re.compile("(?i)what('?s)? is the extension for (vscode|vsc|visual studio)\\??")] = {
        command = "!extension",
    },
    [re.compile("(?i)(learn.*?rust)|(get started in rust)|(start with rust)")] = {
        body = "you can find resources to learn rust here: https://rust-lang.org/learn",
    },
    [re.compile("(?i)(what are (u|you) building\\s?\\??)|\z
                 (what are you working on\\s?\\??)|\z
                 (what('s)? project( is this)\\s??)|\z
                 (going on.*?today)\\s?\\??|\z
                 what are you (making|doing)\\s?\\?|\z
                 .*?project of today.*?|\z
                 .*?random project\\s?\\?|what('s)?(.*?)today\\s?\\?")] = {
        command = "!project",
    },
    [re.compile("(?i)(where is the sub(scribe)? button\\??)|\z
                (how can I sub\\??)|(subscribe\\??)")] = {
        body = "there will never be a subscribe button for this stream",
    },
}

---@param msg Message
local function answer(msg)
    for pattern, response in pairs(answers) do
        if pattern:is_match(msg.data) then
            if not response.body then
                bot:reroute_command(msg, response.command)
            else
                msg:reply(response.body)
            end
            break
        end
    end

    return Handled.bubble
end

---@type Command[]
return {
    listeners = { answer }
}
