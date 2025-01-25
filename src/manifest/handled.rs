use crate::GlobalItem;

#[derive(Debug, Copy, Clone)]
pub enum Handled {
    Bubble,
    Sink,
}

impl GlobalItem for Handled {
    const MODULE: &'static str = "Handled";
}

impl mlua::FromLua for Handled {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
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
