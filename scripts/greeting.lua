local greetings = store:load("greetings") or {}

local function contains(table, value)
    for _, key in pairs(table) do
        if key == value then
            return true
        end
    end

    return false
end

local function greet_user(msg)
    if contains(greetings, msg.data) then
        local greeting = rand:choose(greetings) or "hello"
        msg:say(greeting);
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

---@type Command[]
return { greet, listeners = { greet_user } }
