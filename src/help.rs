use std::path::PathBuf;

use mlua::IntoLua;

use crate::{AliasesDb, KvSqlStore, Mapping};

struct Help {
    command: String,
    usage: String,
    description: String,
}

impl IntoLua for &Help {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        lua.create_table_from([
            ("command", &*self.command),
            ("usage", &*self.usage),
            ("description", &*self.description),
        ])
        .map(mlua::Value::Table)
    }
}

pub struct HelpProvider {
    list: Vec<Help>,
    aliases_db: PathBuf,
    commands_db: PathBuf,
}

impl mlua::UserData for HelpProvider {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method("available_commands", |_lua, this, sort: Option<bool>| {
            let mut list = this
                .list
                .iter()
                .map(|Help { command, .. }| command)
                .map(Clone::clone)
                .chain(
                    AliasesDb::open(&this.aliases_db)
                        .ok()
                        .and_then(|db| db.list_all(false).ok())
                        .unwrap_or_default(),
                )
                .chain(
                    KvSqlStore::open(&this.commands_db)
                        .ok()
                        .and_then(|db| db.keys().ok())
                        .unwrap_or_default(),
                )
                .collect::<Vec<_>>();

            if sort.unwrap_or(false) {
                list.sort_unstable();
            }

            Ok(list)
        });

        methods.add_method("list", |lua, this, ()| {
            lua.create_sequence_from(this.list.iter())
        });

        methods.add_method("lookup", |lua, this, pat: String| {
            let Help {
                usage, description, ..
            } = match this
                .list
                .iter()
                .find(|Help { command, .. }| command == &pat)
            {
                Some(help) => help,
                None => return Ok(mlua::Value::Nil),
            };

            let table = lua.create_table()?;
            table.set("usage", usage.as_str())?;
            table.set("description", description.as_str())?;
            Ok(mlua::Value::Table(table))
        });
    }
}

impl HelpProvider {
    pub fn build(
        commands: &[Mapping],
        lua: &mlua::Lua,
        aliases_db: impl Into<PathBuf>,
        commands_db: impl Into<PathBuf>,
    ) -> mlua::Result<()> {
        let list = commands
            .iter()
            .map(|mapping| Help {
                command: mapping.command.clone(),
                usage: match &mapping.raw_pattern {
                    Some(p) => format!("{} {p}", mapping.command),
                    None => mapping.command.clone(),
                },
                description: mapping.help.clone(),
            })
            .collect();

        lua.globals().set(
            "help",
            Self {
                list,
                aliases_db: aliases_db.into(),
                commands_db: commands_db.into(),
            },
        )
    }
}
