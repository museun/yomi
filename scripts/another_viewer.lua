---@param msg Message
---@return Handled
return function(msg)
    local found = {}
    for part in msg.data:gmatch("%S+") do
        if emotes:has(part) then
            table.insert(found, part)
        end
    end

    local shuffled = rand:shuffle(found);
    local emote = shuffled[1] or nil
    if emote then
        msg:say(string.format("~ %s", emote))
    end

    return Handled.bubble
end
