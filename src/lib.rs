// #![cfg_attr(debug_assertions, allow(dead_code, unused_variables,))]
const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub mod config;
pub use config::Config;

pub mod bot;
pub mod crates;
pub mod format;
pub mod fuzzy;
pub mod github;
pub mod helix;
pub mod help;
pub mod irc;
pub mod json;
pub mod pattern;
pub mod rand;
pub mod re;
pub mod responder;
pub mod spotify;
pub mod store;
pub mod time;

pub mod manifest;
pub use manifest::Manifest;

pub mod watcher;
pub use watcher::Watcher;

pub mod logger;
use logger::Logger;
