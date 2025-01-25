local ns <const> = "commands"

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
        local body = store:get(ns, args.name);
        if body ~= nil then
            msg:reply(string.format("command %s already exists (%s)", args.name, body))
            return
        end

        local body = table.concat(args.body, " ");
        if is_empty(body) then
            msg:reply(string.format("an empty body for provided for %s", args.name))
        else
            store:set(ns, args.name, body)
            msg:reply(string.format("added %s to be '%s'", args.name, body))
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
        local cmd = store:get(ns, args.name)
        if not cmd then
            msg:reply(string.format("command %s does not exist", args.name))
            return
        end

        local body = table.concat(args.body, " ");
        if is_empty(body) then
            msg:reply(string.format("an empty body for provided for %s", args.name))
        else
            store:set(ns, args.name, body)
            msg:reply(string.format("updated %s from '%s' to '%s'", args.name, cmd, body))
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
        local removed = store:remove(ns, args.name)
        if not removed then
            msg:reply(string.format("command %s does not exist", args.name))
            return
        end

        msg:reply(string.format("removed %s", args.name))
    end
}

local function dispatch(msg)
    local c, t = split(msg.data)
    local body = store:get(ns, c) or nil
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
