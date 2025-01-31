local greetings = store:load("greetings") or {}

local function contains(table, value)
    for _, key in pairs(table) do
        if key == value then
            return true
        end
    end

    return false
end

---@type listener
local function greet_user(msg)
    if contains(greetings, msg.data) then
        local greeting = rand:choose(greetings) or "hello"
        msg:reply(greeting);
    end
    return Handled.bubble
end

---@type Command
local greet = {
    command = "!hello",
    help = "greets the user",
    handler = function(msg, args)
        local greeting = rand:choose(greetings) or "hello"
        msg:reply(greeting)
    end
}

---@type Command
local greet_add = {
    command = "!greeting",
    args = "add <greeting>",
    help = "adds a greeting the bot can use",
    elevated = true,
    handler = function(msg, args)
        if contains(greetings, args.greeting) then
            msg:reply("that greeting already exists")
            return
        end
        greetings[#greetings + 1] = args.greeting
        store:save("greetings", greetings)
        msg:reply(string.format("added %s as a greeting", args.greeting))
    end
}

---@type Command[]
return { greet, greet_add, listeners = { greet_user } }
