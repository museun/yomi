local song_request = store:load("spotify") or {}

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
    command = "!song",
    help = "tries to get the currently playing song from spotify",
    handler = function(msg, args)
        local current = spotify:current()
        if current ~= nil then
            msg:say(string.format("%s - %s @ %s",
                join_artists(current),
                current.name,
                get_link(current)
            ))
        end
    end
}
---@type Command
local next = {
    command = "!next",
    help = "tries to get the next song from spotify",
    handler = function(msg, args)
        local item = spotify:next()
        if item ~= nil then
            msg:say(string.format("next in the queue: %s - %s @ %s",
                join_artists(item),
                item.name,
                get_link(item)
            ))
        else
            msg:say("I don't think there is anything in the queue")
        end
    end
}

---@type Command
local search = {
    command = "!search",
    args = "<query...>",
    help = "looks up a song by its title on spotify",
    handler = function(msg, args)
        if msg.channel_id ~= BOT_USER.user_id then
            return
        end
        local items, _ = spotify:search(table.concat(args.query, " "));
        if items then
            for _, item in ipairs(items) do
                msg:say(string.format("%s - %s @ %s",
                    join_artists(item),
                    item.name,
                    get_link(item)
                ))
            end
        end
    end
}

---@type Command
local previous = {
    command = "!previous",
    help = "tries to get the previous song from spotify",
    handler = function(msg, args)
        local item, err = spotify_history:last()
        if err ~= nil then
            log:warn(string.format("cannot get spotify_history:last(): %s", err));
            return;
        end

        if item ~= nil then
            msg:say(string.format("%s - %s @ %s",
                join_artists(item),
                item.name,
                get_link(item)
            ))
        end
    end
}

---@type Command
local skip = {
    command = "!skip",
    help = "tries to skip the current song",
    elevated = true,
    handler = function(msg, args)
        spotify:skip()
    end
}

---@type Command
local request = {
    command = "!request",
    args = "<song>",
    help = "requests a song to be played on spotify",
    handler = function(msg, args)
        if not song_request.enabled then
            msg:reply("song request is not enabled")
            return
        end

        local urn, err = spotify.parse(args.song)
        if err ~= nil then
            msg:reply(string.format("%s", err))
            return
        end

        local item = spotify:add_to_queue(urn)
        if not item then
            msg:reply("I cannot find anything for that")
            return
        end

        msg:reply(string.format("queued: %s - %s @ %s",
            join_artists(item),
            item.name,
            get_link(item)
        ))
    end
}

---@type Command
local toggle = {
    command = "!spotify-toggle",
    args = "<mode?>",
    help = "enables or disables song request",
    elevated = true,
    handler = function(msg, args)
        if args.mode then
            if args.mode == "on" then
                song_request.enabled = true
            elseif args.mode == "off" then
                song_request.enabled = false
            end
        else
            song_request.enabled = not song_request.enabled;
        end
        store:save("spotify", song_request);
        local out
        if song_request.enabled then
            out = "on"
        else
            out = "off"
        end
        msg:reply(string.format("song request is now %s", out))
        return Handled.sink
    end
}

---@type Command
local status = {
    command = "!spotify-state",
    help = "gets the song request mode state",
    handler = function(msg, args)
        local out
        if song_request.enabled then
            out = "on"
        else
            out = "off"
        end
        msg:reply(string.format("song request is %s", out))
        return Handled.sink
    end
}

---@type Command[]
return { song, next, previous, request, skip, status, toggle, search }
