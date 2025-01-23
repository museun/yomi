local function get_link(item)
    return string.format("https://open.spotify.com/track/%s", item.id)
end

local function join_artists(item)
    local t = {}
    for i, artist in ipairs(item.artists) do
        table.insert(t, i, artist.name)
    end
    return table.concat(t, ", ")
end

---@type Command
local song = {
    command = ",song",
    help = "tries to get the currently playing song from spotify",
    handler = function(msg, args)
        local current = spotify:current()
        if current ~= nil then
            msg:say(string.format("%s - %s @ %s", join_artists(current), current.name, get_link(current)))
        end
    end
}

local previous = {
    command = ",previous",
    help = "tries to get the previous song from spotify",
    handler = function(msg, args)
        local item = spotify:previous()
        if item ~= nil then
            msg:say(string.format("%s - %s @ %s", join_artists(item), item.name, get_link(item)))
        end
    end
}

return { song, previous }
