---@meta

---@alias json string
---@alias handler fun(msg: Message, args: {}?): nil A message handler

---@type string The directory for data files
DATA_DIR = ""

---@type string The gist id for the user settings
SETTINGS_GIST_ID = ""

--- Tries to get a `key` from the env.
---@param key string
---@return string | nil
function get_env(key) end

--- Does a fuzzy search for 'needle' in 'haystack' with a minimum tolerance
---@param needle string
---@param haystack string[]
---@param tolerance number
function fuzzy(needle, haystack, tolerance) end

---@class Crate A crates.io crate
---@field name string           The name of the crate
---@field max_version string    The current version of the crate
---@field description string?   The description of the crate
---@field documentation string? The documentation link for the crate
---@field repository string?    The repository link for the crate
---@field exact_match boolean   Was the search an exact match?
---@field updated_at UtcTime    When the crate was last updated
Crate = {}

--- Tries to look up a crate on crates.io
---@param crate string
---@return Crate?
function crates(crate) end

---@enum Handled Returned from listeners to determine if other listeners should run
Handled = {
    --- The message should continue to other listeners
    bubble = 0,
    --- The message should be consumed by this listener
    sink = 1
}

---@class Config Configuration for the bot
---@field paths Paths Path configuration
---@field twitch Twitch Twitch configuration
---@field spotify Spotify Spotify configuration
Config = {}

---@class Paths Configuration for directories used by the bot
---@field data string The directory to store the bot data
---@field scripts string The directory to store the bots scripts
Paths = {}

---@class Twitch Configuration for the Twitch parts of the bot
---@field name string The name of the bot (that is associated with `helix_oauth`)
---@field channels string[] A list of channels to join
---@field helix_oauth string? An OAuth token for 'TMI' (e.g. helix)
---@field client_id string? A Helix Client-Id
---@field client_secret string? A Helix Client-Secret
Twitch = {}

---@class Spotify Configuration for the Spotify parts of the bot
---@field client_id string? A spotify Client-Id
---@field client_secret string? A spotify Client-Secret
Spotify = {}

---@class Manifest
---@field commands {[string]: Command[]} Commands
---@field listeners (fun(msg: Message): Handled)[] Passive listeners
Manifest = {}

---@class Message
---@field our_user   string The bot's user name
---@field our_id     string The bot's user id
---@field channel    string The channel this message happened on
---@field channel_id string The Twitch ID for this channel
---@field msg_id     string A unique ID for this message
---@field sender     string The sender of this message
---@field sender_id  string The Twitch ID for the sender
---@field data       string The text sent by the user
---@field elevated   boolean The message was from an elevated user
---@field say fun(msg: Message, data: string): nil Send a message in response
---@field reply fun(msg: Message, data: string): nil Reply to user from a message
Message = {}

---@class Command         A command binding
---@field command string  A unique ID of the command
---@field args string?    A pattern for matching this command
---@field help string     Help description for the command
---@field handler handler Callback for the command
---@field elevated boolean? Whether this command requires moderator or higher status to use
Command = {}

bot = {
    --- Get the name of the bot
    ---@type string
    name = "",

    --- Get the channels the bot should join
    ---@type string[]
    channels = {},

    --- Get the user id of the bot, once connected to Twitch
    ---@type string
    user_id = "",

    --- Get the display name of the bot, once connected to Twitch
    ---@type string
    display_name = "",

    --- Joins a channel
    ---@param channel string
    ---@return nil
    join = function(channel) end,

    --- Replies to the message
    ---@param message Message
    ---@param data string
    ---@return nil
    reply = function(message, data) end,

    --- Sends a message
    ---@param message Message
    ---@param data string
    ---@return nil
    say = function(message, data) end,
}

log = {
    --- Logs this data at the `trace` level
    ---@param data string|any
    ---@return nil
    trace = function(self, data) end,

    --- Logs this data at the `debug` level
    ---@param data string|any
    ---@return nil
    debug = function(self, data) end,

    --- Logs this data at the `info` level
    ---@param data string|any
    ---@return nil
    info = function(self, data) end,

    --- Logs this data at the `warn` level
    ---@param data string|any
    ---@return nil
    warn = function(self, data) end,

    --- Logs this data at the `error` level
    ---@param data string|any
    ---@return nil
    error = function(self, data) end,
}

---@class Help A help listing
---@field command string     The command name
---@field usage string       The command usage
---@field description string The command description
Help = {}

help = {
    --- Gets a list of available commands
    ---@return Help[]
    list = function(self) end,

    --- Look up a command by its name
    ---@return Help
    lookup = function(self, name) end,
}

---@class TimeSpan A UTC timespan
TimeSpan = {
    --- Gets the seconds for this timespan
    ---@return integer
    seconds = function(self) end,
    --- Gets the milliseconds for this timespan
    ---@return integer
    milliseconds = function(self) end,
    --- Humanize this timespan
    ---@param short boolean? Whether it should be a fuzzy time
    ---@return string
    humanize = function(self, short) end,
}

---@class UtcTime A UTC datetime
UtcTime = {
    --- Gets the elapsed timespan since this one was created
    ---@return TimeSpan
    elapsed = function(self) end
}

---@class Stream A Twitch Stream
---@field id integer           An ID for the stream
---@field user_id integer      The ID for the broadcaster
---@field user_name string     The user name for the broadcaster
---@field game_id integer      The ID for the game being streamed
---@field title string         The title of the stream
---@field viewer_count integer How many viewers are watching
---@field started_at UtcTime   When the stream started
Stream = {}

---@class Emote A Twitch Emote
---@field id string A unique ID for the stream
---@field name string The emote name used in the chat
Emote = {}

helix = {
    ---@param name string The stream name to lookup
    ---@return Stream
    get_stream = function(self, name) end,
    ---@param id string The broadcaster id to fetch emotes for.
    ---@return Emote[]
    get_emotes_for = function(self, id) end,
}

emotes = {
    --- Looks up an emote name by id
    ---@param id string
    ---@return string? The id that was found
    get_name = function(self, id) end,

    --- Looks up an emote id by name
    ---@param name string
    ---@return string? The name that was found
    get_id = function(self, name) end,

    --- Checks to see if this emote is in the map
    ---@param name string
    ---@return boolean
    has = function(self, name) end,

    --- Get all of the names
    ---@return string[]
    names = function(self) end
}

rand = {
    --- Shuffle a table
    ---@param table {[integer]: any}  The table to shuffle
    ---@return {[integer]: any} The table, shuffled.
    shuffle = function(self, table) end
}

store = {
    --- Load a table from the data directory at `key`
    ---@param key string The store key to use
    ---@return {}
    load = function(self, key) end,
    --- Save a table to the data directory at `key`
    ---@param key string The store key to use
    ---@param value {} The table to store
    save = function(self, key, value) end,
}

json = {
    --- Deserialize a table from a json string
    ---@param string string the json string
    ---@return {}
    from_str = function(self, string) end,
    --- Serialize a table to a json string
    ---@param data {} the table to serialize
    ---@return string
    to_str = function(self, data) end
}

github = {
    --- Get the files for a gist
    ---@param id string the gist id
    ---@return {[string]: json}
    get_gist_files = function(self, id) end
}

bot = {
    --- Reroute this command through the but
    ---@param msg Message The message to respond to with the new command
    ---@param command string The command to reroute
    reroute_command = function(self, msg, command) end
}

---@class Pattern
---@field is_match fun(this: Pattern, data: string): boolean Does this pattern match the data?

re = {
    --- Compiles a regex pattern
    ---@param pattern string the pattern to compile
    ---@return Pattern
    compile = function(pattern) end
}

---@class SpotifyItem
---@field duration TimeSpan
---@field name string
---@field id string
---@field artists string[]
---@field progress TimeSpan?

---@alias SpotifyUrn string

spotify = {
    --- Tries to get the currently playing song from spotify
    ---@return SpotifyItem?, string
    current = function(self) end,
    --- Tries to get the next queued song from spotify
    ---@return SpotifyItem?, string
    next = function(self) end,
    --- Tries to get the previously playing song from spotify
    ---@return SpotifyItem?, string
    previous = function(self) end,
    --- Tries to skip the current song
    ---@return boolean
    skip = function(self) end,
    --- Tries to get the currently playing song from spotify
    ---@param urn string A Spotify URN to parse
    ---@return SpotifyUrn, string
    parse = function(urn) end,
    --- Tries to queue a song on spotify
    ---@param urn SpotifyUrn
    ---@return SpotifyItem?
    add_to_queue = function(self, urn) end,
}

spotify_history = {
    --- Gets the previously played song
    ---@return SpotifyItem?, string
    last = function(self) end,
    --- Gets the N most recent songs
    ---@param n integer
    ---@return SpotifyItem[]?,string
    history = function(self, n) end,
    --- Gets the entire playing history
    ---@return SpotifyItem[]?,string
    all = function(self) end,
    --- Counts how many time a song has been played
    ---@param urn SpotifyUrn
    ---@return integer?,string
    count = function(self, urn) end,
}
