---@type Command
local alias = {
    command = "!alias",
    args = "<src> <dst>",
    help = "aliases a command to another name",
    elevated = true,
    handler = function(msg, args)
        if aliases:contains(args.src) then
            msg:reply(string.format("alias %s already exists (%s)", args.src, aliases[args.src]))
            return;
        end

        if aliases:contains(args.dst) then
            msg:reply(string.format("alias %s already exists (%s)", args.dst, aliases[args.dst]))
            return;
        end

        aliases:add(args.dst, args.src)
        msg:reply(string.format("aliased %s to %s", args.src, args.dst))
    end
}

local function redirect(msg)
    local pattern <const> = "^(,%S+)(.*)$"

    local head, tail = msg.data:match(pattern)
    head = head or msg.data
    tail = tail or ""

    local item, err = aliases:resolve(head);
    if item ~= nil then
        log:debug(string.format("redirecting %s to %s", head, item))
        bot:reroute_command(msg, item .. tail)
        return Handled.sink
    end

    return Handled.bubble
end

return { alias, listeners = { redirect } }
