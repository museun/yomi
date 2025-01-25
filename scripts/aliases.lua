local alias_file <const> = "aliases"
local aliases = store:load(alias_file) or {}

---@type Command
local alias = {
    command = "!alias",
    args = "<src> <dst>",
    help = "aliases a command to another name",
    elevated = true,
    handler = function(msg, args)
        if aliases[args.src] ~= nil then
            msg:reply(string.format("alias %s already exists (%s)", args.src, aliases[args.src]))
            return;
        end

        if aliases[args.dst] ~= nil then
            msg:reply(string.format("alias %s already exists (%s)", args.dst, aliases[args.dst]))
            return;
        end

        aliases[args.dst] = args.src
        store:save(alias_file, aliases)
        msg:reply(string.format("aliased %s to %s", args.src, args.dst))
    end
}

local function redirect(msg)
    local pattern <const> = "^(,%S+)(.*)$"

    local head, tail = msg.data:match(pattern)
    head = head or msg.data
    tail = tail or ""

    local item = aliases[head]
    if item ~= nil then
        log:info(string.format("redirecting %s to %s", head, item))
        bot:reroute_command(msg, item .. tail)
        return Handled.sink
    end

    return Handled.bubble
end

return { alias, listeners = { redirect } }
