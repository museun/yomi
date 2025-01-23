local function strip_prefix(str, prefix)
    if str:sub(1, #prefix) == prefix then
        return str:sub(#prefix + 1)
    else
        return str
    end
end

---@param msg Message
---@param args {channel: string?}
---@param on_success fun(stream: Stream)
---@return Stream?
local function fetch(msg, args, on_success)
    local channel = strip_prefix(args.channel or msg.channel, '#')
    local stream = helix:get_stream(channel)

    if stream ~= nil then
        on_success(stream)
    else
        msg:reply(string.format("I don't think %s is streaming", channel))
    end
end

---@type Command
local uptime = {
    command = "!uptime",
    args = "<channel?>",
    help = "get the a twitch stream's uptime",
    ---@param args {channel: string?}
    handler = function(msg, args)
        fetch(msg, args, function(stream)
            msg:say(string.format("%s has been streaming for: %s",
                stream.user_name,
                stream.started_at:elapsed():humanize()
            ))
        end)
    end
}

---@type Command
local viewers = {
    command = "!viewers",
    args = "<channel?>",
    help = "get the number of viewers for twitch stream",
    ---@param args {channel: string?}
    handler = function(msg, args)
        fetch(msg, args, function(stream)
            msg:say(string.format("there are %s viewers watching %s",
                stream.viewer_count,
                stream.user_name
            ))
        end)
    end
}

---@type Command[]
return { uptime, viewers }
