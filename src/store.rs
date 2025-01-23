use std::path::PathBuf;

use mlua::{LuaSerdeExt, UserData};

pub struct Store;

impl UserData for Store {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method("load", |lua, _this, key: String| {
            let dir = lua.globals().get::<PathBuf>("DATA_DIR")?;
            let path = dir.join(key).with_extension("json");
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

        methods.add_method("save", |lua, _this, (key, value): (String, mlua::Table)| {
            let dir = lua.globals().get::<PathBuf>("DATA_DIR")?;
            let path = dir.join(key).with_extension("json");
            let t: serde_json::Value = lua.from_value(mlua::Value::Table(value))?;
            let data = serde_json::to_string_pretty(&t).map_err(mlua::Error::external)?;
            std::fs::write(path, &data).map_err(mlua::Error::external)?;
            Ok(())
        });
    }
}
