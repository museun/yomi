use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use crate::{help::HelpProvider, irc::Message, pattern::Pattern, responder::Responder};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    #[error("lua error: {0}")]
    Lua(#[from] mlua::Error),
}

mod handled;
pub use handled::Handled;

mod mapping;
pub use mapping::Mapping;

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
        source: &str,
    ) -> mlua::Result<Self> {
        let scripts = scripts_dir.as_ref();

        // BUG figure out the syntax for excluding a specific file
        // we don't want a cycle between init -> foo -> init
        lua.globals()
            .get::<mlua::Table>("package")?
            .set("path", scripts.join("?.lua"))?;

        let mut this = Self {
            init: scripts.join("init").with_extension("lua"),
            commands: vec![],
            listeners: vec![],
        };
        if let Err(err) = this.load(lua, source) {
            log::warn!("{err}")
        }
        Ok(this)
    }

    pub fn load(&mut self, lua: &mlua::Lua, data: &str) -> Result<(), Error> {
        let loaded = lua
            .globals()
            .get::<mlua::Table>("package")?
            .get::<mlua::Table>("loaded")?;

        let modules = lua.globals().get::<mlua::Table>("_LOADED_MODULES")?;
        for (k, _) in modules.pairs::<String, mlua::Value>().flatten() {
            modules.set(&*k, false)?;
            loaded.set(k, false)?;
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
                a.push_str(&pattern);
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

    pub fn dispatch(&self, msg: Message, lua: &mlua::Lua, responder: &Responder) {
        log::trace!("[{}] {}: {}", msg.channel, msg.sender, msg.data);

        for listener in &self.listeners {
            match listener.call::<Handled>(&msg) {
                Ok(Handled::Sink) => break,
                Ok(Handled::Bubble) => {}
                Err(err) => log::warn!("cannot call listener because: {err}"),
            }
        }

        let mut sink = false;
        for mapping in &self.commands {
            mapping.dispatch(&msg, lua, responder, &mut sink);
            if sink {
                break;
            }
        }
    }
}
