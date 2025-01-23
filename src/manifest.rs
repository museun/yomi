use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    rc::Rc,
};

use mlua::AnyUserData;

use crate::{
    github, helix,
    help::HelpProvider,
    irc::{self, Message},
    pattern::{Extract, Pattern},
    rand,
    responder::Responder,
    spotify,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    #[error("lua error: {0}")]
    Lua(#[from] mlua::Error),
}

#[derive(Debug)]
pub struct Mapping {
    pub module: String,
    pub command: String,
    pub pattern: Option<Pattern>,
    pub raw_pattern: Option<String>,
    pub help: String,
    pub elevated: bool,
    pub handler: mlua::Function,
}

impl Mapping {
    fn make_error(&self) -> String {
        match &self.raw_pattern {
            Some(p) => format!("invalid usage. syntax: {} {p}", self.command),
            None => format!("invalid usage. syntax: {}", self.command),
        }
    }

    pub fn dispatch(&self, msg: &Message, lua: &mlua::Lua, responder: &impl Responder) {
        let Some(data) = msg.data.strip_prefix(&self.command) else {
            return;
        };

        let data = data.trim();
        let value = match &self.pattern {
            Some(pat) if pat.is_optional() && data.is_empty() => {
                responder.reply(msg, self.make_error());
                return;
            }

            None if !data.is_empty() => {
                responder.reply(msg, self.make_error());
                return;
            }

            Some(pat) => match pat.extract(&data) {
                Extract::NoMatch => {
                    responder.reply(msg, self.make_error());
                    return;
                }
                Extract::Match => mlua::Value::Nil,
                Extract::Bindings { map } => Extract::map_to_lua(map, lua),
            },
            None => mlua::Value::Nil,
        };

        if self.elevated && !msg.is_elevated() {
            responder.reply(msg, "you cannot do that command".to_string());
            return;
        }

        let Err(err) = self.handler.call::<()>((msg, value)) else {
            return;
        };

        if let Some(err) = err.to_string().lines().nth(0).and_then(|c| {
            c.split_terminator(": ")
                .skip_while(|c| {
                    c.contains("runtime error") || c.contains("./scripts") || c.contains("src")
                })
                .next()
        }) {
            responder.error(msg, err.to_string());
        }

        log::warn!(
            "cannot call: {command} because {err}",
            command = self.command
        )
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Handled {
    Bubble,
    Sink,
}

impl mlua::FromLua for Handled {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Handled".to_string(),
                message: None,
            }),
        }
    }
}

impl mlua::UserData for Handled {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_function_get("bubble", |_lua, _| Ok(Self::Bubble));
        fields.add_field_function_get("sink", |_lua, _| Ok(Self::Sink));
    }
}

#[derive(Debug)]
pub struct Manifest {
    pub init: PathBuf,
    commands: Vec<Mapping>,
    listeners: Vec<mlua::Function>,
}

impl Manifest {
    pub fn initialize(
        lua: &mlua::Lua,
        scripts_dir: impl AsRef<Path>,
        data_dir: impl AsRef<Path>,
        source: &str,
        gist_id: &str,
        reroute: flume::Sender<irc::Message>,
        github: github::Client,
        helix: helix::Client,
        spotify: spotify::Client,
        emote_map: helix::EmoteMap,
    ) -> mlua::Result<Self> {
        lua.globals().set("log", crate::Logger)?;

        // this is an enum, but it somehow still works
        lua.globals().set("Handled", Handled::Sink)?;
        lua.globals().set("_LOADED_MODULES", lua.create_table()?)?;
        lua.globals().set("DATA_DIR", data_dir.as_ref())?;
        lua.globals().set("SETTINGS_GIST_ID", gist_id)?;

        lua.globals().set("github", github)?;
        lua.globals().set("spotify", spotify)?;
        lua.globals().set("helix", helix)?;
        lua.globals().set("emotes", emote_map)?;

        lua.globals().set("bot", crate::bot::Bot::new(reroute))?;
        lua.globals().set("re", crate::re::Regex)?;
        lua.globals().set("json", crate::json::Json)?;
        lua.globals().set("store", crate::store::Store)?;

        lua.globals().set(
            "rand", // TODO make this mockable
            rand::Rando::new(fastrand::Rng::new()),
        )?;

        lua.globals().set("crates", {
            lua.create_function(|_, key: String| Ok(crate::crates::lookup_crate(&key)))?
        })?;

        let require = lua.globals().get::<mlua::Function>("require")?;
        lua.globals().set("require", {
            lua.create_function(move |lua, name: String| {
                lua.globals()
                    .get::<mlua::Table>("_LOADED_MODULES")?
                    .set(&*name, true)?;
                require.call::<mlua::Value>(name)
            })?
        })?;

        let package = lua.globals().get::<mlua::Table>("package")?;
        // BUG figure out the syntax for excluding a specific file
        // we don't want a cycle between init -> foo -> init
        package.set("path", scripts_dir.as_ref().join("?.lua"))?;

        let mut this = Self {
            init: scripts_dir.as_ref().join("init.lua"),
            commands: vec![],
            listeners: vec![],
        };
        if let Err(err) = this.load(lua, source) {
            log::warn!("{err}")
        }
        Ok(this)
    }

    pub fn read_init_lua(path: impl AsRef<Path>) -> Result<String, Error> {
        let path = path.as_ref();
        loop {
            let data = std::fs::read_to_string(path)?;
            if !data.trim().is_empty() {
                break Ok(data);
            }
            std::hint::spin_loop();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    pub fn set_responder(lua: &mlua::Lua, responder: impl Responder + 'static) -> mlua::Result<()> {
        lua.globals().set(
            "_RESPONDER",
            AnyUserData::wrap(Rc::new(responder) as Rc<dyn Responder>),
        )
    }

    pub fn load(&mut self, lua: &mlua::Lua, data: &str) -> Result<(), Error> {
        let loaded = lua
            .globals()
            .get::<mlua::Table>("package")?
            .get::<mlua::Table>("loaded")?;

        let modules = lua.globals().get::<mlua::Table>("_LOADED_MODULES")?;
        for (k, _) in modules.pairs::<String, mlua::Value>().flatten() {
            loaded.set(k, mlua::Nil)?;
        }

        _ = std::mem::take(&mut self.commands);
        _ = std::mem::take(&mut self.listeners);

        let value = match lua.load(data).eval::<mlua::Table>() {
            Ok(value) => value,
            Err(err) => {
                log::warn!("invalid manifest: {err}");
                return Ok(());
            }
        };

        let mut report = String::from("loaded script");

        // TODO redo this stuff

        // always load listeners
        self.listeners = value
            .get::<mlua::Table>("listeners")
            .map(|listeners| {
                listeners
                    .pairs::<mlua::Value, mlua::Function>()
                    .flatten()
                    .map(|(_, function)| function)
                    .collect()
            })
            .unwrap_or_default();

        let mut errors = vec![];

        let commands = match value.get::<mlua::Table>("commands") {
            Ok(commands) if !commands.is_empty() => Some(commands),
            Ok(..) => {
                log::warn!("empty commands table");
                None
            }
            Err(..) => {
                log::warn!("missing commands table");
                None
            }
        };

        for (module, table) in commands
            .iter()
            .flat_map(|t| t.pairs::<String, mlua::Table>().flatten())
        {
            if let Ok(listeners) = table.get::<Vec<mlua::Function>>("listeners") {
                self.listeners.extend(listeners);
            }

            for (index, table) in table.pairs::<usize, mlua::Table>().flatten() {
                match (
                    table.get("command"),
                    table.get::<Option<String>>("args"),
                    table.get("help"),
                    table.get("elevated"),
                    table.get("handler"),
                ) {
                    (Ok(command), Ok(raw_pattern), Ok(help), Ok(elevated), Ok(handler)) => {
                        let pattern = match raw_pattern.as_deref().map(Pattern::parse) {
                            Some(Ok(pat)) => Some(pat),
                            Some(Err(err)) => {
                                errors.push(err.to_string());
                                continue;
                            }
                            None => None,
                        };

                        let mapping = Mapping {
                            module: module.clone(),
                            command,
                            pattern,
                            raw_pattern,
                            help,
                            elevated,
                            handler,
                        };
                        self.commands.push(mapping);
                    }
                    (command, pattern, help, _, handler, ..) => {
                        if command.is_err() {
                            errors.push(format!("missing `command` for `{module}[{index}]`"));
                        }
                        if pattern.is_err() {
                            errors.push(format!("missing `args` for `{module}[{index}]`"));
                        }
                        if help.is_err() {
                            errors.push(format!("missing `help` for `{module}[{index}]`"));
                        }
                        if handler.is_err() {
                            errors.push(format!("missing `handler` for `{module}[{index}]`"));
                        }
                    }
                }
            }
        }

        report.push_str(&format!("\nlisteners: {}", self.listeners.len()));
        log::info!("{report}");

        // TODO redo this
        if !errors.is_empty() {
            let join = |mut s: String, c: String| {
                if !s.is_empty() {
                    s.push('\n');
                }
                s.push_str(&c);
                s
            };
            let out = errors
                .into_iter()
                .fold(String::from("problems found in init.lua"), join);
            log::warn!("{out}");
        }

        if !self.commands.is_empty() {
            let join = |mut a: String, pattern: Cow<'_, str>| {
                if !a.is_empty() {
                    a.push('\n');
                }
                a.push_str(&*pattern);
                a
            };

            let out = self
                .commands
                .iter()
                .map(|m| {
                    m.raw_pattern
                        .as_deref()
                        .map(|p| Cow::from(format!("{} {p}", m.command)))
                        .unwrap_or(Cow::from(&m.command))
                })
                .fold(String::from("loaded commands"), join);
            log::info!("{out}");
        }

        HelpProvider::build(&self.commands, lua)?;
        Ok(())
    }

    pub fn dispatch(&self, msg: Message, lua: &mlua::Lua, responder: &impl Responder) {
        log::trace!("[{}] {}: {}", msg.channel, msg.sender, msg.data);

        for listener in &self.listeners {
            match listener.call::<Handled>(&msg) {
                Ok(Handled::Sink) => break,
                Ok(Handled::Bubble) => {}
                Err(err) => log::warn!("cannot call listener because: {err}"),
            }
        }

        for mapping in &self.commands {
            mapping.dispatch(&msg, lua, responder);
        }
    }
}
