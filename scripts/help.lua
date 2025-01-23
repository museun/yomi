---@param seq Help[]
---@param width integer
local function wrap(seq, width, f)
    local names = {}
    local extract = f or function(help)
        return help.command
    end

    for _, help in ipairs(seq) do
        table.insert(names, extract(help))
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

local function map(input)
    local output = {}
    local index = 1

    for value, _ in pairs(input) do
        output[index] = value
        index = index + 1
    end
    return output
end

local function lookup(msg, key)
    local value = help:lookup(key);
    if value ~= nil then
        msg:reply(string.format("%s | %s", value.usage, value.description))
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

local function identity(t) return t end

---@type handler
local function show_help(msg, args)
    if not args.command then
        for _, line in pairs(wrap(help:list(), 50)) do
            msg:say(line)
        end

        local commands = wrap(map(store:load("commands")), 50, identity)
        for _, line in pairs(commands) do
            msg:say(line)
        end

        local aliases = wrap(map(store:load("aliases")), 50, identity)
        for _, line in pairs(aliases) do
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
