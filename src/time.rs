use mlua::{FromLua, UserData};
use time::format_description::well_known::Rfc2822;

use crate::format::FormatTime;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeSpan(pub ::time::Duration);

impl FromLua for TimeSpan {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
            _ => Err(mlua::Error::runtime("invalid type")),
        }
    }
}

impl UserData for TimeSpan {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method("seconds", |_lua, this, ()| {
            Ok(this.0.whole_seconds()) //
        });

        methods.add_method("milliseconds", |_lua, this, ()| {
            Ok(this.0.whole_milliseconds())
        });

        methods.add_method("humanize", |_lua, this, ()| Ok(this.0.as_readable_time()));
        methods.add_method("humanize", |_lua, this, short: bool| {
            let out = if short {
                this.0.as_fuzzy_time()
            } else {
                this.0.as_readable_time()
            };
            Ok(out)
        });

        methods.add_meta_method("__tostring", |_lua, this, ()| {
            Ok(format!("{:.2?}", this.0.unsigned_abs()))
        });
        methods.add_meta_method("__le", |_lua, this, other: Self| Ok(*this <= other));
        methods.add_meta_method("__lt", |_lua, this, other: Self| Ok(*this < other));
        methods.add_meta_method("__eq", |_lua, this, other: Self| Ok(*this == other));
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UtcTime(pub time::OffsetDateTime);

impl FromLua for UtcTime {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
            _ => Err(mlua::Error::runtime("invalid type")),
        }
    }
}

impl UserData for UtcTime {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method("elapsed", |_lua, this, ()| {
            Ok(TimeSpan(time::OffsetDateTime::now_utc() - this.0))
        });

        methods.add_meta_method("__tostring", |_lua, this, ()| {
            Ok(this.0.format(&Rfc2822).unwrap())
        });

        methods.add_meta_method("__le", |_lua, this, other: Self| Ok(*this <= other));
        methods.add_meta_method("__lt", |_lua, this, other: Self| Ok(*this < other));
        methods.add_meta_method("__eq", |_lua, this, other: Self| Ok(*this == other));
    }
}
