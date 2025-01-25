use std::path::{Path, PathBuf};

use mlua::UserData;

use crate::{sql::DbError, GlobalItem, ResultExt};

pub struct Aliases(PathBuf);

impl GlobalItem for Aliases {
    const MODULE: &'static str = "aliases";
}

impl Aliases {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }
}

impl UserData for Aliases {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method("lookup", |_lua, this, command: String| {
            let aliases = AliasesDb::open(&this.0).map_err(mlua::Error::external)?;
            Ok(aliases.get_aliases(&command))
        });

        methods.add_method("contains", |_lua, this, query: String| {
            let aliases = AliasesDb::open(&this.0).map_err(mlua::Error::external)?;
            Ok(aliases.contains(&query).unwrap_or(false))
        });

        methods.add_method("resolve", |_lua, this, query: String| {
            let aliases = AliasesDb::open(&this.0).map_err(mlua::Error::external)?;
            aliases.resolve(&query).into_lua_tuple()
        });

        methods.add_method_mut("add", |_lua, this, (command, alias): (String, String)| {
            let mut aliases = AliasesDb::open(&this.0).map_err(mlua::Error::external)?;
            aliases.add_alias(&command, &alias).into_lua_tuple()
        });

        methods.add_method("remove", |_lua, this, alias: String| {
            let aliases = AliasesDb::open(&this.0).map_err(mlua::Error::external)?;
            aliases.remove_alias(&alias).into_lua_tuple()
        });

        methods.add_method("clear", |_lua, this, command: String| {
            let aliases = AliasesDb::open(&this.0).map_err(mlua::Error::external)?;
            aliases.clear_aliases(&command).into_lua_tuple()
        });
    }

    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        Self::add_fields(registry);
        Self::add_methods(registry);
    }
}

macro_rules! include_sql {
    ($name:expr) => {{
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/sql/aliases/",
            concat!($name, ".sql")
        ))
    }};
}

// this is basically a key=val[] store which can be used for a lot of things
// like the commands stuff just needs to be key=val
struct AliasesDb {
    conn: rusqlite::Connection,
}

impl AliasesDb {
    fn open(path: impl AsRef<Path>) -> Result<Self, DbError> {
        static SCHEMA: &str = include_sql!("schema");
        let conn = rusqlite::Connection::open(path)
            .map_err(|err| DbError::CannotOpenDb(err.to_string()))?;

        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    fn contains(&self, query: &str) -> Result<bool, DbError> {
        static CONTAINS: &str = include_sql!("contains");
        match self
            .conn
            .query_row(CONTAINS, [query, query], |row| Ok(row.get(0)?))
        {
            Ok(value) => Ok(value),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
            Err(err) => Err(err.into()),
        }
    }

    fn resolve(&self, query: &str) -> Result<String, DbError> {
        static RESOLVE: &str = include_sql!("resolve");
        Ok(self
            .conn
            .query_row(RESOLVE, [query], |row| Ok(row.get("command")?))?)
    }

    fn get_aliases(&self, command: &str) -> Vec<String> {
        static GET_ALIASES: &str = include_sql!("get_aliases");
        let mut stmt = self.conn.prepare(GET_ALIASES).expect("valid sql");
        let query = stmt.query_map([command], |row| Ok(row.get("alias")?));
        let Ok(iter) = query else { return vec![] };
        iter.flatten().collect()
    }

    fn add_alias(&mut self, command: &str, alias: &str) -> Result<bool, DbError> {
        static ADD_COMMAND: &str = include_sql!("add_command");
        static ADD_ALIAS: &str = include_sql!("add_alias");
        let tx = self.conn.transaction()?;
        let c = tx.execute(ADD_COMMAND, [command])?;
        let a = tx.execute(ADD_ALIAS, [command, alias])?;
        tx.commit()?;
        Ok(c + a > 0)
    }

    fn remove_alias(&self, alias: &str) -> Result<bool, DbError> {
        static REMOVE_ALIAS: &str = include_sql!("remove_alias");
        let n = self.conn.execute(REMOVE_ALIAS, [alias])?;
        Ok(n > 0)
    }

    fn clear_aliases(&self, command: &str) -> Result<bool, DbError> {
        static CLEAR_ALIASES: &str = include_sql!("clear_aliases");
        let n = self.conn.execute(CLEAR_ALIASES, [command])?;
        Ok(n > 0)
    }
}
