use std::borrow::Cow;

use mlua::{IntoLua, UserData};
use time::{format_description::FormatItem, macros::format_description, OffsetDateTime};

use crate::{time::UtcTime, GlobalItem};

pub struct Crates;
impl UserData for Crates {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_function("lookup", |_lua, name: String| Ok(lookup_crate(&name)));
    }
}

impl GlobalItem for Crates {
    const MODULE: &'static str = "crates";
}

pub fn lookup_crate(name: &str) -> Option<Crate> {
    #[derive(serde::Deserialize)]
    struct Resp {
        crates: Vec<Crate>,
    }

    let mut resp: Resp = attohttpc::get("https://crates.io/api/v1/crates")
        .header("User-Agent", crate::USER_AGENT)
        .params([("page", "1"), ("per_page", "1"), ("q", name)])
        .send()
        .ok()?
        .json()
        .ok()?;

    match resp.crates.len() {
        0 => None,
        _ => Some(resp.crates.remove(0)),
    }
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct Crate {
    pub name: String,
    pub max_version: String,
    pub description: Option<String>,
    pub documentation: Option<String>,
    pub repository: Option<String>,
    pub exact_match: bool,
    #[serde(deserialize_with = "crates_utc_date_time")]
    pub updated_at: UtcTime,
}

impl IntoLua for Crate {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let table = lua.create_table()?;
        table.set("name", self.name)?;
        table.set("max_version", self.max_version)?;
        table.set("description", self.description)?;
        table.set("documentation", self.documentation)?;
        table.set("repository", self.repository)?;
        table.set("exact_match", self.exact_match)?;
        table.set("updated_at", self.updated_at)?;
        Ok(mlua::Value::Table(table))
    }
}

fn crates_utc_date_time<'de, D>(deser: D) -> Result<UtcTime, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{Deserialize as _, Error as _};
    const FORMAT: &[FormatItem<'static>] = format_description!(
        "[year]-[month]-[day]T\
            [hour]:[minute]:[second]\
            .[subsecond digits:6]\
            [offset_hour sign:mandatory]:[offset_minute]"
    );
    let s = <Cow<'_, str>>::deserialize(deser)?;
    OffsetDateTime::parse(&s, &FORMAT)
        .map_err(D::Error::custom)
        .map(UtcTime)
}
