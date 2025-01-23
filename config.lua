---@type Config
return {
    paths = {
        data = "./data",
        scripts = "./scripts",
    },
    twitch = {
        name = "shaken_bot",
        channels = { "#museun", "#shaken_bot" },
        helix_oauth = get_env("SHAKEN_TWITCH_OAUTH_TOKEN"),
        client_id = get_env("SHAKEN_TWITCH_CLIENT_ID"),
        client_secret = get_env("SHAKEN_TWITCH_CLIENT_SECRET"),
    },
    spotify = {
        client_id = get_env("SHAKEN_SPOTIFY_CLIENT_ID"),
        client_secret = get_env("SHAKEN_SPOTIFY_CLIENT_SECRET"),
        refresh_token = get_env("SHAKEN_SPOTIFY_REFRESH_TOKEN"),
    },
    github = {
        settings_gist_id = "6f7b1d5e0c293e927959f74c884b039c",
        oauth_token = get_env("SHAKEN_GITHUB_OAUTH_TOKEN")
    }
}
