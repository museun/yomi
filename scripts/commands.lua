local store_file <const> = "commands"
local commands = store:load(store_file) or {};

local function split(s)
    local head, tail = s:match("^(%S+)%s*(.*)$")
    if not head then
        return s, nil
    end
    return head, tail
end

local function is_empty(s)
    return s:match("^%s*$") ~= nil
end

---@type Command
local add = {
    command = "!add",
    args = "<name> <body...>",
    help = "add a command",
    elevated = true,
    handler = function(msg, args)
        if commands[args.name] ~= nil then
            msg:reply(string.format("command %s already exists (%s)", args.name, commands[args.name]))
        else
            local body = table.concat(args.body, " ");
            if is_empty(body) then
                msg:reply(string.format("an empty body for provided for %s", args.name))
            else
                commands[args.name] = body
                store:save(store_file, commands)
                msg:reply(string.format("added %s to be '%s'", args.name, body))
            end
        end
    end
}

---@type Command
local update = {
    command = "!update",
    args = "<name> <body...>",
    help = "update a command",
    elevated = true,
    handler = function(msg, args)
        if not commands[args.name] then
            msg:reply(string.format("command %s does not exist", args.name))
            return
        end

        local old = commands[args.name];
        local body = table.concat(args.body, " ");
        if is_empty(body) then
            msg:reply(string.format("an empty body for provided for %s", args.name))
        else
            commands[args.name] = table.concat(args.body, " ")
            store:save(store_file, commands)
            msg:reply(string.format("updated %s from '%s' to '%s'", args.name, old, commands[args.name]))
        end
    end
}

---@type Command
local remove = {
    command = "!remove",
    args = "<name>",
    help = "remove a command",
    elevated = true,
    handler = function(msg, args)
        if not commands[args.name] then
            msg:reply(string.format("command %s does not exist", args.name))
            return
        end

        commands[args.name] = nil
        store:save(store_file, commands)
        msg:reply(string.format("removed %s", args.name))
    end
}

local function dispatch(msg)
    local c, t = split(msg.data)
    local body = commands[c] or nil
    if body ~= nil then
        if t ~= nil and t ~= "" then
            msg:say(string.format("%s: %s", t, body))
        else
            msg:reply(body)
        end
    end
    return Handled.bubble
end

---@type Command[]
return {
    add,
    update,
    remove,
    listeners = { dispatch }
}
