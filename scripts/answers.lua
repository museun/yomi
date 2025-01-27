local function c(...) return { command = ... } end

local playlist = "museun don't use a playlist, they just pick a song and let spotify play from there"
local learn_rust = "you can find resources to learn rust here: https://rust-lang.org/learn"
local subscribe = "there will never be a subscribe button for this stream"

local function pattern(data)
    local pat = re.compile(data)
    if not pat then
        log:error(string.format("pattern is nil: %s"))
        return nil
    end
    if help == nil then
        log:error("help is nil?")
        return nil
    end
    for _, value in ipairs(help:available_commands() or {}) do
        if pat and pat:is_match(value) then
            log:warn(string.format("%s matched a command: %s", data, value))
            return nil
        end
    end
    return pat
end


---@return {[string|{command: string}]: Pattern[]}
local function create_answers()
    return {
        [playlist]       = {
            pattern("(?i).*?playlist.*?(\\?)?$")
        },

        [c("!editor")]   = {
            pattern("(?i)(ide|editor)\\?"),
            pattern("(?i)(what editor\\s*?(is (this|that))?)\\??"),
        },

        [c("!song")]     = {
            pattern("(?i)song name\\??"),
            pattern("(?i)which song\\??"),
            pattern("(?i)what song is this|that\\??"),
        },

        [c("!theme")]    = {
            pattern("(i?)what theme.*?\\??"),
        },

        [c("!font")]     = {
            pattern("(i?)what font.*?\\??"),
        },

        [c("!os")]       = {
            pattern("(?i)what os\\s*?((are you using)|(is this))?\\??")
        },

        [learn_rust]     = {
            pattern("started with rust"),
            pattern("start with rust"),
            pattern("(?i)learn.*?rust")
        },

        [c("!project")]  = {
            pattern("(?i)what are (u|you) building\\s?\\??"),
            pattern("(?i)what are you working on\\s?\\??"),
            pattern("(?i)what('s)? project(\\sis this)\\s??"),
            pattern("(?i)going on.*?today\\s?\\??"),
            pattern("(?i)what are you (making|doing)\\s?\\?"),
            pattern("(?i).*?project of today.*?"),
            pattern("(?i).*?random project\\s?\\?"),
            pattern("(?i)what('s)?.*?today\\s?\\?"),
        },

        [subscribe]      = {
            pattern("(?i)where is the sub(scribe)? button\\??"),
            pattern("(?i)how can I sub(scribe)?\\??"),
        },

        [c("!settings")] = {
            pattern("(?i)(what|where are\\s?)?editor (settings|config.*?)\\??")
        }
    }
end

local answers = {}

---@param msg Message
local function answer(msg)
    if not answers then create_answers() end

    for response, patterns in pairs(answers) do
        for _, pattern in ipairs(patterns) do
            if pattern and pattern:is_match(msg.data) then
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
