use std::path::{Path, PathBuf};

use mlua::LuaSerdeExt as _;

#[derive(Clone, Default)]
pub struct Secret<T>(T);

impl<T> std::ops::Deref for Secret<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> serde::Deserialize<'de> for Secret<String> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String>::deserialize(deserializer)?;
        Ok(Self(s.trim().to_string()))
    }
}

impl std::fmt::Debug for Secret<String> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("<hidden (len = {})>", self.0.len()))
    }
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct Paths {
    #[serde(default)]
    pub data: PathBuf,

    #[serde(default)]
    pub scripts: PathBuf,
}

#[derive(Default, Clone, Debug, serde::Deserialize)]
pub struct Spotify {
    #[serde(default)]
    pub client_id: String,

    #[serde(default)]
    pub client_secret: Secret<String>,

    #[serde(default)]
    pub refresh_token: Secret<String>,
}

#[derive(Default, Clone, Debug, serde::Deserialize)]
pub struct Twitch {
    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub channels: Vec<String>,

    // TODO this should be called tmi_oauth
    #[serde(default)]
    pub helix_oauth: Secret<String>,

    #[serde(default)]
    pub client_id: String,

    #[serde(default)]
    pub client_secret: Secret<String>,
}

#[derive(Default, Clone, Debug, serde::Deserialize)]
pub struct Github {
    #[serde(default)]
    pub settings_gist_id: String,

    #[serde(default)]
    pub oauth_token: Secret<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Config {
    #[serde(default)]
    pub paths: Paths,

    #[serde(default)]
    pub twitch: Twitch,

    #[serde(default)]
    pub spotify: Spotify,

    #[serde(default)]
    pub github: Github,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> mlua::Result<Self> {
        fn validate(table: &str, key: &str, input: &str, hint: &str, errors: &mut Vec<String>) {
            if input.trim().is_empty() {
                errors.push(format!("error: {table}.{key} is required\nnote: {hint}"));
            }
        }

        let path = path.as_ref();
        let config = match std::fs::read_to_string(path) {
            Ok(data) => data,
            Err(..) => {
                log::error!(
                    "cannot read configuration file at {path}",
                    path = path.to_string_lossy()
                );
                std::process::exit(1)
            }
        };

        let lua = mlua::Lua::new();

        lua.globals().set(
            "get_env",
            lua.create_function(|_, key: String| Ok(std::env::var(&key).ok()))?,
        )?;

        let value = lua.load(config).eval()?;
        let mut config: Config = lua.from_value(value)?;

        let mut errors = vec![];

        for (table, key, val, hint) in [
            (
                "twitch",
                "name",
                &*config.twitch.name,
                "this is the bots name",
            ),
            (
                "twitch",
                "helix_oauth",
                &**config.twitch.helix_oauth,
                "this is an OAuth token",
            ),
            (
                "twitch",
                "client_id",
                &*config.twitch.client_id,
                "this is an public token",
            ),
            (
                "twitch",
                "client_secret",
                &**config.twitch.client_secret,
                "this is an private token",
            ),
            (
                "spotify",
                "client_id",
                &*config.spotify.client_id,
                "this is an public token",
            ),
            (
                "spotify",
                "client_secret",
                &*config.spotify.client_secret,
                "this is an private token",
            ),
            (
                "spotify",
                "refresh_token",
                &*config.spotify.refresh_token,
                "this is an private token",
            ),
            (
                "github",
                "settings_gist_id",
                &*config.github.settings_gist_id,
                "this is the gist for the current user configuration",
            ),
            (
                "github",
                "oauth_token",
                &*config.github.oauth_token,
                "this is an OAuth token",
            ),
            (
                "paths",
                "data",
                &config.paths.data.to_string_lossy(),
                "this is where the bots data is stored",
            ),
            (
                "paths",
                "scripts",
                &config.paths.scripts.to_string_lossy(),
                "this is where the bot's scripts are located",
            ),
        ] {
            validate(table, key, val, hint, &mut errors);
        }

        config.twitch.channels.retain_mut(|c| {
            let s = c.trim();
            if s.is_empty() {
                return false;
            }
            *c = s.to_string();
            true
        });

        if config.twitch.channels.is_empty() {
            errors.push(format!(
                "a channel must be provided for twitch.channels = {{}}"
            ));
        }

        if !errors.is_empty() {
            log::warn!("invalid configuration file:");
            for error in errors {
                for line in error.lines() {
                    log::warn!("  {line}");
                }
            }

            log::info!(
                "help:\n  \
                 you can load secrets from the environment with:\n  \
                 get_env(key) -> String"
            );

            std::process::exit(1);
        }

        Ok(config)
    }
}
