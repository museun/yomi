use mlua::UserData;

use crate::GlobalItem;

pub struct RePattern(regex::Regex);
impl UserData for RePattern {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method("is_match", |_lua, this, data: String| {
            Ok(this.0.is_match(&data))
        });
    }
}

pub struct Regexp;

impl GlobalItem for Regexp {
    const MODULE: &'static str = "re";
}

impl UserData for Regexp {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_function("compile", |_lua, pattern: String| {
            regex::Regex::new(&pattern)
                .map_err(mlua::Error::external)
                .map(RePattern)
        });
    }
}
