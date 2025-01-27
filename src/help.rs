use mlua::IntoLua;

use crate::Mapping;

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
}

impl mlua::UserData for HelpProvider {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
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
    pub fn build(commands: &[Mapping], lua: &mlua::Lua) -> mlua::Result<()> {
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

        lua.globals().set("help", Self { list })
    }
}
