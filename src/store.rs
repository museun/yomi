use std::path::{Path, PathBuf};

use mlua::{LuaSerdeExt, UserData};
use rusqlite::OptionalExtension;

use crate::{sql::DbError, GlobalItem};

pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }
}

impl GlobalItem for Store {
    const MODULE: &'static str = "store";
}

impl UserData for Store {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method("load", |lua, this, key: String| {
            let path = this.dir.join(key).with_extension("json");
            let data = match std::fs::read_to_string(&path) {
                Ok(data) => data,
                Err(_) => {
                    log::warn!("cannot read: {path}", path = path.display());
                    return Ok(mlua::Value::Nil);
                }
            };

            let value =
                serde_json::from_str::<serde_json::Value>(&data).map_err(mlua::Error::external)?;
            lua.to_value(&value)
        });

        methods.add_method("save", |lua, this, (key, value): (String, mlua::Table)| {
            let path = this.dir.join(key).with_extension("json");
            let t: serde_json::Value = lua.from_value(mlua::Value::Table(value))?;
            let data = serde_json::to_string_pretty(&t).map_err(mlua::Error::external)?;
            std::fs::write(path, &data).map_err(mlua::Error::external)?;
            Ok(())
        });

        methods.add_method(
            "set",
            |_lua, this, (ns, key, value): (String, String, mlua::Value)| {
                let db = KvSqlStore::open(this.dir.join(ns).with_extension("db"))
                    .map_err(mlua::Error::external)?;
                db.set(&key, value).map_err(mlua::Error::external)
            },
        );

        methods.add_method("get", |lua, this, (ns, key): (String, String)| {
            let db = KvSqlStore::open(this.dir.join(ns).with_extension("db"))
                .map_err(mlua::Error::external)?;
            match db.get(&key) {
                Ok(val) => match lua.to_value(&val) {
                    Ok(mlua::Value::LightUserData(..)) => Ok(mlua::Value::Nil),
                    Ok(val) => Ok(val),
                    Err(err) => Err(err),
                },
                Err(..) => Ok(mlua::Value::Nil),
            }
        });

        methods.add_method("remove", |_lua, this, (ns, key): (String, String)| {
            let db = KvSqlStore::open(this.dir.join(ns).with_extension("db"))
                .map_err(mlua::Error::external)?;
            match db.remove(&key) {
                Ok(val) => Ok(val),
                Err(..) => Ok(false),
            }
        });
    }
}

macro_rules! include_sql {
    ($name:expr) => {{
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/sql/store/",
            concat!($name, ".sql")
        ))
    }};
}

struct KvSqlStore {
    conn: rusqlite::Connection,
}

impl KvSqlStore {
    fn open(path: impl AsRef<Path>) -> Result<Self, DbError> {
        static SCHEMA: &str = include_sql!("schema");
        let conn = rusqlite::Connection::open(path)
            .map_err(|err| DbError::CannotOpenDb(err.to_string()))?;

        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    fn set(&self, key: &str, value: impl serde::Serialize) -> Result<(), DbError> {
        static SET: &str = include_sql!("set");
        let value = serde_json::to_value(value).expect("valid json");
        self.conn.execute(SET, rusqlite::params![key, value])?;
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<serde_json::Value>, DbError> {
        static GET: &str = include_sql!("get");
        let mut stmt = self.conn.prepare(GET)?;
        Ok(stmt.query_row([key], |row| row.get(0)).optional()?)
    }

    fn remove(&self, key: &str) -> Result<bool, DbError> {
        static REMOVE: &str = include_sql!("remove");
        Ok(self.conn.execute(REMOVE, [key])? > 0)
    }
}
