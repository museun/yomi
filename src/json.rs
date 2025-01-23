use mlua::{LuaSerdeExt, UserData};

pub struct Json;

impl UserData for Json {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method("to_str", |lua, _this, value: mlua::Table| {
            let t: serde_json::Value = lua.from_value(mlua::Value::Table(value))?;
            let data = serde_json::to_string_pretty(&t).map_err(mlua::Error::external)?;
            Ok(data)
        });

        methods.add_method("from_str", |lua, _this, data: String| {
            let value: serde_json::Value =
                serde_json::from_str(&data).map_err(mlua::Error::external)?;
            lua.to_value(&value)
        });
    }
}
