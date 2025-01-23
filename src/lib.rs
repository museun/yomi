const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

mod bot;
mod config;
mod crates;
mod format;
mod fuzzy;
mod github;
mod helix;
mod help;
mod json;
mod logger;
mod manifest;
mod pattern;
mod rand;
mod re;
mod responder;
mod spotify;
mod store;
mod time;
mod watcher;

pub mod irc;

pub use config::Config;
pub use github::Client as GithubClient;
pub use helix::Client as HelixClient;
pub use manifest::Manifest;
pub use responder::{Responder, ResponderChannel};
pub use spotify::Client as SpotifyClient;
pub use watcher::Watcher;
