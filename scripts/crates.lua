---@type handler
local function lookup_crate(msg, args)
    local crate = crates(args.crate_name) or nil

    if crate then
        msg:say(string.format("%s = %s | last updated %s",
            crate.name, crate.max_version, crate.updated_at:elapsed():humanize(true)
        ))

        if crate.description ~= nil or crate.documentation ~= nil or crate.repository ~= nil then
            msg:say(string.format("%s | %s", crate.description or "<no description>",
                crate.documentation or crate.repository or "<no links>"))
        end

        if crate.exact_match ~= true then
            msg:say("(this isn't an exact match)")
        end
    else
        msg:reply(string.format("cannot find anything for %s", args.crate_name))
    end
end

---@type Command
local crate = {
    command = "!crate",
    args = "<crate_name>",
    help = "looks up a crate on crates.io",
    handler = lookup_crate
}

---@type Command[]
return { crate }
