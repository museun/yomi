local max_width <const> = 100

---@param seq Help[]
---@param width integer
local function wrap(seq, width)
    local names = {}

    for _, help in ipairs(seq) do
        table.insert(names, help)
    end

    local text = table.concat(names, " ")
    local lines = {}
    local current = ""

    for word in text:gmatch("%S+") do
        if #current + #word + 1 <= width then
            if #current == 0 then
                current = word
            else
                current = current .. " " .. word
            end
        else
            table.insert(lines, current)
            current = word
        end
    end

    if #current > 0 then
        table.insert(lines, current)
    end

    return lines
end

local function map(list)
    local out = {}
    for i, c in ipairs(list) do
        out[i] = c.command
    end
    return out
end

local function map_flat(input)
    local output = {}
    local index = 1

    for value, _ in pairs(input) do
        output[index] = value
        index = index + 1
    end
    return output
end

local function find_nearest(key)
    local function merge()
        local merged = {}

        local a = map(help:list())
        local b = map_flat(store:load("commands"))
        local c = map_flat(store:load("aliases"))

        table.move(a, 1, #a, #merged + 1, merged)
        table.move(b, 1, #b, #merged + 1, merged)
        table.move(c, 1, #c, #merged + 1, merged)

        return merged
    end

    return fuzzy(key, merge(), 0.7)[1]
end

local function lookup(msg, key)
    local value = help:lookup(key)
    if value ~= nil then
        msg:reply(string.format("%s | %s", value.usage, value.description))
        return true
    end

    local value = find_nearest(key)
    if not value then
        return false
    end

    local next = store:load("aliases")[value] or help:lookup(value)
    if next and next.usage and next.description then
        if next == key then
            msg:reply(string.format("%s | %s", next.usage, next.description))
        else
            msg:reply(string.format("(closest match for '%s') %s | %s", key, next.usage, next.description))
        end
        return true
    end

    local command = store:load("commands")[value]
    if command ~= nil then
        if value == key then
            msg:reply(string.format("%s is a user defined command: %s", value, command))
        else
            msg:reply(string.format("(closest match for '%s') %s is a user defined command: %s", key, value, command))
        end
        return true
    end

    return false
end

local function lookup_command(msg, key)
    local command = store:load("commands")[key]
    if command ~= nil then
        msg:reply(string.format("its a user defined command: %s", command))
        return true
    end
    return false
end

local function lookup_alias(msg, key)
    local alias = store:load("aliases")[key]
    if alias ~= nil then
        if not lookup(msg, alias) then
            return lookup_command(msg, alias)
        end
        return true
    end

    return false
end

---@type handler
local function show_help(msg, args)
    if not args.command then
        for _, line in pairs(wrap(map(help:list()), max_width)) do
            msg:say(line)
        end

        for _, line in pairs(wrap(map_flat(store:load("commands")), max_width)) do
            msg:say(line)
        end

        for _, line in pairs(wrap(map_flat(store:load("aliases")), max_width)) do
            msg:say(line)
        end
        return
    end

    if not lookup(msg, args.command) then
        if not lookup_command(msg, args.command) then
            if not lookup_alias(msg, args.command) then
                msg:reply(string.format("cannot find: %s", args.command))
            end
        end
    end
end

---@type Command
local help = {
    command = ",help",
    args = "<command?>",
    help = "list commands, or looks up a command",
    handler = show_help
}

return { help }
