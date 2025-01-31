---@type Command
local alias = {
    command = "!alias",
    args = "<src> to <dst>",
    help = "aliases a command to another name",
    elevated = true,
    handler = function(msg, args)
        if args.src == args.dst then
            msg:reply(string.format("cannot create a recursive alias for %s", args.src))
            return
        end

        if aliases:contains(args.src) then
            msg:reply(string.format("alias %s already exists", args.src))
            return
        end

        if aliases:contains(args.dst) then
            msg:reply(string.format("alias %s already exists", args.dst))
            return
        end

        local found = false
        for _, cmd in ipairs(help:available_commands()) do
            if args.src == cmd then
                found = true
            end
        end

        if found then
            aliases:add(args.dst, args.src)
            msg:reply(string.format("aliased %s to %s", args.src, args.dst))
        else
            msg:reply(string.format("%s is not a command", args.src))
        end
    end
}

local function redirect(msg)
    local pattern <const> = "^(!%S+)(.*)$"

    local head, tail = msg.data:match(pattern)
    head = head or msg.data
    tail = tail or ""

    local item, err = aliases:resolve(head);
    if err ~= nil then
        return Handled.bubble
    end

    if head == item then
        return Handled.sink
    end

    if item ~= nil then
        log:debug(string.format("redirecting %s to %s", head, item))
        bot:reroute_command(msg, item .. tail)
        return Handled.sink
    end

    return Handled.bubble
end

return { alias, listeners = { redirect } }
