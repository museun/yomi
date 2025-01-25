use mlua::IntoLua;

use crate::GlobalItem;

pub struct LoadedModules;

impl IntoLua for LoadedModules {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        lua.create_table().map(mlua::Value::Table)
    }
}

impl GlobalItem for LoadedModules {
    const MODULE: &'static str = "_LOADED_MODULES";

    fn register(self, g: crate::Globals<'_>) -> mlua::Result<()> {
        g.set(Self::MODULE, self)?;

        let require = g.get::<mlua::Function>("require")?;
        let require = g.0.create_function(move |lua, name: String| {
            lua.globals()
                .get::<mlua::Table>("_LOADED_MODULES")?
                .set(&*name, true)?;
            require.call::<mlua::Value>(name)
        })?;

        g.set("require", require)
    }
}
