const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

mod aliases;
mod bot;
mod config;
mod format;
mod github;
mod globals;
mod helix;
mod help;
mod json;
mod loaded;
mod logger;
mod manifest;
mod pattern;
mod rand;
mod re;
mod responder;
mod spotify;
mod sql;
mod store;
mod time;
mod watcher;

pub mod crates;
pub mod fuzzy;
pub mod irc;

pub use aliases::Aliases;
pub use bot::Bot;
pub use config::Config;
pub use github::Client as GithubClient;
pub use globals::{GlobalItem, Globals};
pub use helix::{Client as HelixClient, EmoteMap};
pub use json::Json;
pub use loaded::LoadedModules;
pub use logger::Logger;
pub use manifest::{Handled, Manifest, Mapping};
pub use rand::Rando;
pub use re::Regexp;
pub use responder::Responder;
pub use spotify::{Client as SpotifyClient, SpotifyHistory};
pub use store::Store;
pub use watcher::Watcher;

use mlua::{IntoLua, IntoLuaMulti};
trait ResultExt: Sized {
    type Out: IntoLuaMulti;
    fn into_lua_tuple(self) -> mlua::Result<Self::Out>;
}

// TODO is this actually the what I want to do?
// maybe we should use mlua::Function::wrap_raw that'll do pcall / pexec for us
impl<T: IntoLuaMulti + IntoLua, E: std::error::Error> ResultExt for Result<T, E> {
    type Out = (Option<T>, Option<String>);
    fn into_lua_tuple(self) -> mlua::Result<Self::Out> {
        let val = match self {
            Ok(ok) => (Some(ok), None),
            Err(err) => (None, Some(err.to_string())),
        };
        Ok(val)
    }
}
