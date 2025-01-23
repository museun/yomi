pub struct Logger;

impl mlua::UserData for Logger {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method("trace", |_lua, _this, string: String| {
            log::trace!(target: "lua", "{string}");
            Ok(())
        });
        methods.add_method("trace", |_lua, _this, value: mlua::Value| {
            log::trace!(target: "lua", "{value:#?}");
            Ok(())
        });

        methods.add_method("debug", |_lua, _this, string: String| {
            log::debug!(target: "lua", "{string}");
            Ok(())
        });
        methods.add_method("debug", |_lua, _this, value: mlua::Value| {
            log::debug!(target: "lua", "{value:#?}");
            Ok(())
        });

        methods.add_method("info", |_lua, _this, string: String| {
            log::info!(target: "lua", "{string}");
            Ok(())
        });
        methods.add_method("info", |_lua, _this, value: mlua::Value| {
            log::info!(target: "lua", "{value:#?}");
            Ok(())
        });

        methods.add_method("warn", |_lua, _this, string: String| {
            log::warn!(target: "lua", "{string}");
            Ok(())
        });
        methods.add_method("warn", |_lua, _this, value: mlua::Value| {
            log::warn!(target: "lua", "{value:#?}");
            Ok(())
        });

        methods.add_method("error", |_lua, _this, string: String| {
            log::error!(target: "lua", "{string}");
            Ok(())
        });
        methods.add_method("error", |_lua, _this, value: mlua::Value| {
            log::error!(target: "lua", "{value:#?}");
            Ok(())
        });
    }
}
