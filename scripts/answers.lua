local function c(...) return { command = ... } end

local playlist = "museun don't use a playlist, they just pick a song and let spotify play from there"
local learn_rust = "you can find resources to learn rust here: https://rust-lang.org/learn"
local subscribe = "there will never be a subscribe button for this stream"


---@type {[string|{command: string}]: Pattern[]}
local answers = {
    [playlist]       = {
        re.compile("(?i).*?playlist.*?(\\?)?$")
    },

    [c("!editor")]   = {
        re.compile("(?i)(ide|editor)\\?"),
        re.compile("(?i)(what editor\\s*?(is (this|that))?)\\??"),
    },

    [c("!song")]     = {
        re.compile("(?i)song name\\??"),
        re.compile("(?i)which song\\??"),
        re.compile("(?i)what song is this|that\\??"),
    },

    [c("!theme")]    = {
        re.compile("(i?)what theme.*?\\??"),
    },

    [c("!font")]     = {
        re.compile("(i?)what font.*?\\??"),
    },

    [c("!os")]       = {
        re.compile("(?i)what os\\s*?((are you using)|(is this))?\\??")
    },

    [learn_rust]     = {
        re.compile("started with rust"),
        re.compile("start with rust"),
        re.compile("(?i)learn.*?rust")
    },

    [c("!project")]  = {
        re.compile("(?i)what are (u|you) building\\s?\\??"),
        re.compile("(?i)what are you working on\\s?\\??"),
        re.compile("(?i)what('s)? project(\\sis this)\\s??"),
        re.compile("(?i)going on.*?today\\s?\\??"),
        re.compile("(?i)what are you (making|doing)\\s?\\?"),
        re.compile("(?i).*?project of today.*?"),
        re.compile("(?i).*?random project\\s?\\?"),
        re.compile("(?i)what('s)?.*?today\\s?\\?"),
    },

    [subscribe]      = {
        re.compile("(?i)where is the sub(scribe)? button\\??"),
        re.compile("(?i)how can I sub(scribe)?\\??"),
    },

    [c("!settings")] = {
        re.compile("(?i)(what|where are\\s?)?editor (settings|config.*?)\\??")
    }
}

---@param msg Message
local function answer(msg)
    for response, patterns in pairs(answers) do
        for _, pattern in ipairs(patterns) do
            if pattern:is_match(msg.data) then
                if response.command ~= nil then
                    log:info(string.format("rerouting to %s", response.command))
                    bot:reroute_command(msg, response.command)
                elseif type(response) == "string" then
                    msg:reply(response)
                end
                return Handled.sink
            end
        end
    end

    return Handled.bubble
end

---@type Command[]
return {
    listeners = { answer }
}
