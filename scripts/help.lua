local max_width <const> = 100

---@param seq Help[]
---@param w integer?
local function wrap(seq, w)
    local names = {}
    local width = w or max_width

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

local function find_nearest(key)
    local data = help:available_commands()
    return fuzzy.closest(key, data, true)
end

local function lookup(msg, key, opts)
    local opts = opts or { closest = false }

    local value = help:lookup(key)
    if not value then
        return false
    end

    if not opts.closest then
        msg:reply(string.format("%s | %s", value.usage, value.description))
    else
        msg:reply(string.format("(closest match for '%s') %s | %s",
            opts.value,
            value.usage,
            value.description
        ))
    end

    return true
end

local function lookup_command(msg, key, opts)
    local function resolve_command(msg, key, opts)
        local command = store:get("commands", key)
        if not command then
            return false
        end

        if not opts.closest then
            msg:reply(string.format("its a user defined command: %s", command))
        else
            msg:reply(string.format(
                "(closest match for '%s') %s is a user defined command: %s",
                opts.value,
                key,
                command
            ))
        end
        return true
    end

    local opts = opts or { closest = false }
    if resolve_command(msg, key, opts) then
        return true
    end

    local command = aliases:resolve(key)
    if not command then
        return false
    end
    return resolve_command(msg, command, opts)
end

local function lookup_fuzzy(msg, key)
    local value = find_nearest(key)
    if not value then
        return false
    end

    if lookup(msg, value, { closest = true, value = key }) then
        return true
    end

    if lookup_command(msg, value, { closest = true, value = key }) then
        return true
    end

    return false
end

---@type handler
local function show_help(msg, args)
    if not args.command then
        for _, line in pairs(wrap(help:available_commands())) do
            msg:say(line)
        end
        return
    end

    if lookup(msg, args.command) then
        return
    end

    if lookup_command(msg, args.command) then
        return
    end

    if lookup_fuzzy(msg, args.command) then
        return
    end

    msg:reply(string.format("cannot find: %s", args.command))
end

---@type Command
local help = {
    command = "!help",
    args = "<command?>",
    help = "list commands, or looks up a command",
    handler = show_help
}

return { help }
