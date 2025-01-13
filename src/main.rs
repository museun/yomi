#![cfg_attr(debug_assertions, allow(dead_code, unused_variables,))]
use std::{
    cell::RefCell,
    collections::HashMap,
    fs::FileType,
    path::{Path, PathBuf},
    rc::Rc,
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime},
};

use mlua::{FromLua, IntoLua, LuaSerdeExt, UserData, UserDataFields};

// TODO better logging

struct Watcher {
    events: flume::Receiver<Notification>,
    _handle: JoinHandle<()>,
}

#[derive(Debug)]
enum Notification {
    Modified(PathBuf),
    Removed(PathBuf),
}

impl Watcher {
    // TODO make this not like this. at the very least it should use XDG_DIRECTORIES
    const SCRIPTS_DIR: &str = "./scripts/";

    fn new(path: impl Into<PathBuf>) -> Self {
        let (tx, events) = flume::unbounded();
        let _handle = std::thread::spawn({
            let path = path.into();
            move || Self::watch_task(path, tx)
        });
        Self { events, _handle }
    }

    fn load_script(path: impl AsRef<Path>, lua: &mlua::Lua) -> mlua::Result<Plugin> {
        let path = path.as_ref();

        if path.extension().and_then(|c| c.to_str()) != Some("lua") {
            return Err(std::io::Error::other(format!(
                "{} does not have a lua extension",
                path.to_string_lossy()
            ))
            .into());
        }

        eprintln!("trying to load: {path}", path = path.to_string_lossy());
        let data = std::fs::read_to_string(path)?;
        lua.load(data).eval()
    }

    fn wait_for_changes(&self) -> impl Iterator<Item = Notification> + use<'_> {
        std::iter::from_fn(|| self.events.recv().ok())
    }

    fn watch_task(path: PathBuf, tx: flume::Sender<Notification>) {
        let mut tracked = HashMap::new();
        let mut last = Instant::now();

        loop {
            std::thread::sleep(Duration::from_millis(100));
            let dir = match std::fs::read_dir(&path) {
                Ok(dir) => dir,
                Err(err) => {
                    eprintln!("cannot read dir: {err}");
                    continue;
                }
            };

            // we don't need to hammer the file system that much for this
            // so every 5 seconds we'll check to see what files are gone
            // TODO actually write a notify crate that isn't trash
            if last.elapsed() >= Duration::from_secs(5) {
                tracked.retain(|k: &PathBuf, _| {
                    let result = std::fs::metadata(k).is_ok();
                    if !result {
                        _ = tx.send(Notification::Removed(k.clone()));
                    }
                    result
                });
                last = Instant::now();
            }

            for entry in dir.flatten() {
                macro_rules! remove_file {
                    ($path:expr) => {{
                        eprintln!("removing: {}", $path.to_string_lossy());
                        tracked.remove(&$path);
                        if tx.send(Notification::Removed($path)).is_err() {
                            return;
                        }
                        continue;
                    }};
                }

                if entry.file_type().ok().filter(FileType::is_file).is_none() {
                    remove_file!(entry.path());
                }

                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("lua") {
                    eprintln!("not a lua script: {}", path.to_string_lossy());
                    continue;
                }

                let Ok(st) = entry.metadata().and_then(|md| md.modified()) else {
                    remove_file!(entry.path())
                };

                let old = tracked
                    .entry(path.clone())
                    .or_insert(SystemTime::UNIX_EPOCH);

                let Ok(elapsed) = st.duration_since(*old) else {
                    remove_file!(entry.path())
                };

                if elapsed >= Duration::from_millis(100) {
                    if tx.send(Notification::Modified(path)).is_err() {
                        return;
                    }
                    *old = st;
                }
            }
        }
    }
}

#[derive(Default, Clone)]
struct Registry {
    scripts: Rc<RefCell<Vec<Script>>>,
}

impl Registry {
    fn dispatch(&self, event: irc::Event) {
        let scripts = self.scripts.borrow();

        for script in &*scripts {
            match &event {
                irc::Event::Connected { user } => {
                    let Some(f) = &script.plugin.on_connected else {
                        continue;
                    };
                    _ = f.call::<()>(user);
                }

                irc::Event::Disconnected {} => {
                    let Some(f) = &script.plugin.on_disconnected else {
                        continue;
                    };
                    _ = f.call::<()>(());
                }

                irc::Event::Message { msg } => {
                    let Some(f) = &script.plugin.on_message else {
                        continue;
                    };

                    let msg = Message {
                        channel: msg.channel.to_string(),
                        channel_id: msg
                            .room_id()
                            .map(ToString::to_string)
                            .expect("Privmsg can only happen on channels"),
                        msg_id: msg
                            .msg_id()
                            .map(ToString::to_string)
                            .expect("msg-id should be available"),
                        user: msg
                            .display_name()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| msg.sender.to_string()),
                        user_id: msg
                            .user_id()
                            .map(ToString::to_string)
                            .expect("user-id should be available"),
                        data: msg.data.to_string(),
                    };

                    impl UserData for Message {
                        fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
                            fields.add_meta_field("zxc", true);
                        }
                    }
                    // let data = msg.into_lua(&script.state).unwrap();
                    // data.as_table().unwrap().set("test", "false").unwrap();
                    // data.as_table().unwrap().

                    _ = f.call::<()>(&msg);
                }
            }
        }
    }
}

impl mlua::UserData for Registry {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method("loaded_scripts", |lua, this, ()| {
            Ok(lua.create_table_from(
                this.scripts
                    .borrow()
                    .iter()
                    .map(|k| (&*k.plugin.name, k.plugin.version)),
            ))
        });

        methods.add_method("memory_stats", |lua, this, ()| {
            Ok(lua.create_table_from(this.memory_stats()))
        });
    }
}

impl Registry {
    fn memory_stats(&self) -> Vec<(String, usize)> {
        self.scripts
            .borrow()
            .iter()
            .map(|s| (s.plugin.to_string(), s.state.used_memory()))
            .collect()
    }

    fn insert(&self, path: impl Into<PathBuf>, state: mlua::Lua, plugin: Plugin) {
        let mut scripts = self.scripts.borrow_mut();
        let path = path.into();

        match scripts.iter().rev().position(|s| s.path == path) {
            Some(existing) => {
                let script = Script {
                    path,
                    state,
                    plugin,
                };

                let index = scripts.len() - existing - 1;
                let old = std::mem::replace(&mut scripts[index], script);
                let new = &scripts[index];
                eprintln!(
                    "updating: {old} -> {new} ",
                    old = old.plugin,
                    new = new.plugin
                )
            }
            None => {
                let script = Script {
                    path,
                    state,
                    plugin,
                };
                eprintln!("created: {script}");
                scripts.push(script);
            }
        }
    }

    fn remove(&self, path: impl AsRef<Path>) -> Option<Script> {
        let mut scripts = self.scripts.borrow_mut();
        let path = path.as_ref();
        let index = scripts.iter().position(|s| s.path == path)?;

        // TODO maybe we should be doing a swap-remove. is script order important?
        let script = scripts.remove(index);
        eprintln!("unloaded: {script}");
        Some(script)
    }
}

struct Script {
    path: PathBuf,
    state: mlua::Lua,
    plugin: Plugin,
}

impl std::fmt::Display for Script {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (from {})", self.plugin, self.path.to_string_lossy())
    }
}

#[derive(serde::Deserialize, Clone, Debug)]
struct Config {
    name: String,
    channel: String,

    #[serde(skip)]
    oauth: String,
}

#[derive(Debug)]
struct Plugin {
    name: String,
    version: usize,
    on_disconnected: Option<mlua::Function>,
    on_connected: Option<mlua::Function>,
    on_message: Option<mlua::Function>,
}

impl std::fmt::Display for Plugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.name, self.version)
    }
}

impl FromLua for Plugin {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::Table(table) => Ok(Self {
                name: table.get("name")?,
                version: table.get("version")?,
                on_connected: table.get("on_connected")?,
                on_disconnected: table.get("on_disconnected")?,
                on_message: table.get("on_message")?,
            }),
            _ => Err(mlua::Error::runtime("expected a table")),
        }
    }
}

#[derive(Debug)]
enum Response {
    Join {
        channel: String,
    },
    Error {
        channel: String,
        data: String,
    },
    Reply {
        channel: String,
        msg_id: String,
        data: String,
    },
    Say {
        channel: String,
        data: String,
    },
}

#[derive(Debug)]
struct Message {
    channel: String,
    channel_id: String,
    msg_id: String,
    user: String,
    user_id: String,
    data: String,
}

// impl UserData for Message {
//     fn add_methods<M>(methods: &mut M)
//     where
//         M: mlua::UserDataMethods<Self>,
//     {
//         methods.add_method("something", |_, this, ()| {
//             eprintln!("called something: {this:#?}");
//             Ok(())
//         });

//         methods.add_method("parse", |lua, this, pattern: mlua::Function| {
//             eprintln!("whats going on?");

//             let (head, tail) = this
//                 .data
//                 .split_once(' ')
//                 .unwrap_or_else(|| (&*this.data, ""));

//             let head = head.strip_prefix('@').unwrap_or(head);

//             let head = head.trim();
//             if head.is_empty() {
//                 eprintln!("head is empty");
//                 return Ok(mlua::Value::Nil);
//             }

//             let tail = tail.trim();

//             let out = lua.create_table()?;
//             out.set("command", head)?;

//             if tail.is_empty() {
//                 eprintln!("?? {out:#?}");
//                 return Ok(mlua::Value::Table(out));
//             }

//             let table: mlua::Table = pattern.call(tail)?;
//             for (k, v) in table.pairs::<String, mlua::Value>().flatten() {
//                 match v {
//                     mlua::Value::Boolean(b) => out.set(k, b)?,
//                     mlua::Value::Integer(i) => out.set(k, i)?,
//                     mlua::Value::Number(f) => out.set(k, f)?,
//                     mlua::Value::String(s) => out.set(k, s)?,
//                     _ => {}
//                 }
//             }
//             out.set("data", &*this.data)?;
//             eprintln!("?? {out:#?}");
//             Ok(mlua::Value::Table(out))
//         });
//     }
// }

impl IntoLua for &Message {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        lua.create_table_from([
            ("channel", &*self.channel),
            ("channel_id", &*self.channel_id),
            ("msg_id", &*self.msg_id),
            ("sender", &*self.user),
            ("user_id", &*self.user_id),
            ("data", &*self.data),
        ])
        .map(mlua::Value::Table)
    }
}

impl FromLua for Message {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        let table = value
            .as_table()
            .ok_or_else(|| mlua::Error::runtime("Message must be a table"))?;

        Ok(Self {
            channel: table.get("channel")?,
            channel_id: table.get("channel_id")?,
            msg_id: table.get("msg_id")?,
            user: table.get("sender")?,
            user_id: table.get("user_id")?,
            data: table.get("data")?,
        })
    }
}

#[derive(Clone)]
struct Bot {
    tx: flume::Sender<Response>,
}

impl mlua::UserData for Bot {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method("error", |_, this, data| {
            let (msg, data): (Message, String) = data;
            Ok(this.send(Response::Error {
                channel: msg.channel,
                data,
            }))
        });

        methods.add_method("reply", |_, this, data| {
            let (msg, data): (Message, String) = data;
            Ok(this.send(Response::Reply {
                channel: msg.channel,
                msg_id: msg.msg_id,
                data,
            }))
        });

        methods.add_method("say", |_, this, data| {
            let (msg, data): (Message, String) = data;
            Ok(this.send(Response::Say {
                channel: msg.channel,
                data,
            }))
        });

        methods.add_method("join", |_, this, channel| {
            Ok(this.send(Response::Join { channel }))
        });
    }
}

impl Bot {
    const fn new(tx: flume::Sender<Response>) -> Self {
        Self { tx }
    }

    fn send(&self, response: Response) {
        _ = self.tx.send(response)
    }
}

#[derive(Clone)]
struct State {
    bot: Bot,
    registry: Registry,
    current: mlua::Lua,
}

impl State {
    fn new(bot: Bot) -> Self {
        Self {
            bot,
            registry: Registry::default(),
            current: mlua::Lua::new(),
        }
    }

    fn dispatch(&self, event: irc::Event) {
        self.registry.dispatch(event)
    }

    fn init(&mut self) -> mlua::Lua {
        let lua = std::mem::replace(&mut self.current, mlua::Lua::new());
        lua.globals()
            .set("bot", self.bot.clone())
            .expect("set bot global");

        lua.globals()
            .set("registry", self.registry.clone())
            .expect("set registry global");
        lua
    }
}

mod irc {
    use std::{
        io::{BufReader, Write},
        net::TcpStream,
        time::Duration,
    };

    use mlua::IntoLua;
    use twitch_message::{
        encode::{Encodable, ALL_CAPABILITIES},
        messages::{Message, MsgIdRef, Privmsg, TwitchMessage},
        IntoStatic, PingTracker,
    };

    use crate::{Config, Response};

    #[derive(Clone, Debug)]
    pub struct User {
        pub name: String,
        pub display: String,
        pub user_id: String,
    }

    impl IntoLua for &User {
        fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
            lua.create_table_from([
                ("name", &*self.name),
                ("display", &*self.display),
                ("user_id", &*self.user_id),
            ])
            .map(mlua::Value::Table)
        }
    }

    #[derive(Debug)]
    pub enum Event {
        Connected { user: User },
        Disconnected {},
        Message { msg: Privmsg<'static> },
    }

    pub fn connect(config: Config, response: flume::Receiver<Response>) -> flume::Receiver<Event> {
        let (events, out) = flume::unbounded();
        let _ = std::thread::spawn(move || {
            connect_loop(config, response, events);
        });
        out
    }

    fn delay(msg: std::fmt::Arguments<'_>) {
        eprintln!("{msg}");
        std::thread::sleep(Duration::from_secs(5));
    }

    fn connect_loop(
        config: Config,
        response: flume::Receiver<Response>,
        events: flume::Sender<Event>,
    ) {
        fn send(events: &flume::Sender<Event>, event: Event) -> bool {
            events.send(event).is_ok()
        }

        let mut buf = String::with_capacity(1024);
        'outer: loop {
            eprintln!("trying to connect to Twitch");
            let stream = match TcpStream::connect(twitch_message::TWITCH_IRC_ADDRESS) {
                Ok(stream) => stream,
                Err(err) => {
                    if !send(&events, Event::Disconnected {}) {
                        return;
                    }
                    delay(format_args!("cannot connect, trying again because: {err}"));
                    continue 'outer;
                }
            };
            stream.set_nonblocking(true).expect("non-blocking sockets");

            let (mut read, mut write) = (BufReader::new(&stream), &stream);

            macro_rules! send {
                ($msg:expr) => {{
                    if let Err(err) = $msg.encode(&mut write) {
                        if !send(&events, Event::Disconnected {}) {
                            return;
                        }
                        delay(format_args!("cannot write, reconnecting because: {err}"));
                        continue 'outer;
                    }
                    if let Err(err) = write.flush() {
                        delay(format_args!("cannot flush, reconnecting because: {err}"));
                        continue 'outer;
                    }
                }};
            }

            send!(twitch_message::encode::register(
                &config.name,
                &config.oauth,
                ALL_CAPABILITIES
            ));

            let mut user = User {
                name: config.name.clone(),
                display: config.name.clone(),
                user_id: String::new(),
            };

            let pt = PingTracker::new(Duration::from_secs(3 * 60));

            'start: loop {
                buf.clear();
                let messages = match read_many(&mut read, &mut buf) {
                    Ok(msg) => msg,
                    Err(err) => {
                        if !send(&events, Event::Disconnected {}) {
                            return;
                        }
                        delay(format_args!("cannot read, reconnecting because: {err}"));
                        continue 'outer;
                    }
                };

                for message in messages {
                    pt.update(&message);

                    if let Some(msg) = pt.should_pong() {
                        send!(msg)
                    }

                    match message.as_enum() {
                        TwitchMessage::Ready(ready) => user.name = ready.name.to_string(),
                        TwitchMessage::GlobalUserState(gus) => {
                            user.display = gus
                                .display_name()
                                .map(ToString::to_string)
                                .unwrap_or_else(|| user.name.clone());

                            user.user_id = gus
                                .user_id()
                                .map(ToString::to_string)
                                .expect("we must have a user id");
                            break 'start;
                        }

                        _ => {}
                    }
                }

                // we might be spinning for a while, so lets not hammer the cpu
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            if !send(&events, Event::Connected { user: user.clone() }) {
                return;
            }

            // drain any pending responses
            for _ in response.try_iter() {}

            loop {
                buf.clear();

                let messages = match read_many(&mut read, &mut buf) {
                    Ok(msg) => msg,
                    Err(err) => {
                        if !send(&events, Event::Disconnected {}) {
                            return;
                        }
                        delay(format_args!("cannot read, reconnecting because: {err}"));
                        continue 'outer;
                    }
                };

                for message in messages {
                    eprintln!("<- {}", message.raw.escape_debug());

                    pt.update(&message);
                    if let Some(msg) = pt.should_pong() {
                        send!(msg);
                    }

                    match message.as_enum() {
                        TwitchMessage::Reconnect(..) => {
                            if !send(&events, Event::Disconnected {}) {
                                return;
                            }
                            delay(format_args!("cannot read, server asked us to reconnect"));
                            continue 'outer;
                        }

                        TwitchMessage::RoomState(..) => {}
                        TwitchMessage::Privmsg(privmsg) => {
                            let event = Event::Message {
                                msg: privmsg.into_static(),
                            };
                            if !send(&events, event) {
                                return;
                            }
                        }
                        _ => {}
                    }
                }

                for resp in response.try_iter() {
                    match resp {
                        Response::Join { channel } => {
                            send!(twitch_message::encode::join(&channel))
                        }

                        Response::Error { channel, data } => {
                            send!(twitch_message::encode::privmsg(&channel, &data))
                        }

                        Response::Reply {
                            channel,
                            msg_id,
                            data,
                        } => {
                            send!(twitch_message::encode::reply(
                                &MsgIdRef::from_str(&msg_id),
                                &channel,
                                &data
                            ))
                        }

                        Response::Say { channel, data } => {
                            send!(twitch_message::encode::privmsg(&channel, &data))
                        }
                    }
                }
            }
        }
    }

    fn read_many(
        mut read: impl std::io::BufRead,
        buf: &mut String,
    ) -> std::io::Result<impl Iterator<Item = Message<'_>>> {
        let mut a = None;

        match read.read_line(buf) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "read zero bytes",
                ));
            }
            Ok(n) => {
                let data = &buf[..n];
                let mut iter = twitch_message::parse_many(data).flatten();
                a = Some(std::iter::from_fn(move || iter.next()))
            }
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::Interrupted
                        | std::io::ErrorKind::TimedOut
                ) => {}

            Err(err) => return Err(err),
        };

        Ok(std::iter::from_fn(move || a.as_mut()?.next()))
    }
}

mod twitch {}
mod http {}
mod spotify {}

fn env_get(key: &str) -> anyhow::Result<String> {
    std::env::var(key).map_err(|_| anyhow::anyhow!("expected `{key}` to be in env"))
}

fn main() -> anyhow::Result<()> {
    simple_env_load::load_env_from([".dev.env", ".secrets.env"]);

    let lua = mlua::Lua::new();
    let value = lua
        .load(std::fs::read("config.lua")?)
        .eval()
        .map_err(|err| anyhow::anyhow!("cannot load config.lua: {err}"))?;

    let mut config: Config = lua.from_value(value).expect("valid config mapping");
    config.oauth = env_get("SHAKEN_OAUTH_CHAT_TOKEN")?;

    let (tx, response) = flume::unbounded();
    let state = State::new(Bot::new(tx));
    let watcher = Watcher::new(Watcher::SCRIPTS_DIR);

    let events = irc::connect(config, response);

    while flume::Selector::new()
        .recv(&events, {
            let state = state.clone();
            move |ev| {
                let Ok(ev) = ev else {
                    return false;
                };
                state.dispatch(ev);
                true
            }
        })
        .recv(&watcher.events, {
            let mut state = state.clone();
            move |ev| {
                let Ok(ev) = ev else {
                    return false;
                };

                match ev {
                    Notification::Modified(path) => {
                        let lua = state.init();
                        match Watcher::load_script(&path, &lua) {
                            Ok(plugin) => state.registry.insert(path, lua, plugin),
                            Err(err) => eprintln!(
                                "cannot load: {path} because: {err}",
                                path = path.to_string_lossy()
                            ),
                        }
                    }
                    Notification::Removed(path) => {
                        _ = state.registry.remove(&path);
                    }
                }

                true
            }
        })
        .wait()
    {}

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use twitch_message::{
        messages::{types::NicknameRef, Privmsg},
        Tags,
    };

    use super::*;
    #[test]
    fn asdf() {
        let (tx, response) = flume::unbounded();
        let mut state = State::new(Bot::new(tx));

        let lua = state.init();
        let plugin: Plugin = lua
            .load(
                r#"
        return {
            name = "test",
            version = 0,
            on_message = function(msg)
                -- msg:something()
                print(msg.data, msg.zxc)
            end
        }
        "#,
            )
            .eval()
            .unwrap();
        state.registry.insert("testing.lua", lua, plugin);

        state.dispatch(irc::Event::Message {
            msg: Privmsg {
                channel: Cow::from("#testing"),
                sender: Cow::from(NicknameRef::from_static("museun")),
                tags: Tags::builder()
                    .add("id", "123")
                    .add("user-id", "321")
                    .add("room-id", "0")
                    .finish(),
                data: Cow::from("@hello 1234"),
                action: false,
                raw: Cow::default(),
            },
        });
    }
}
