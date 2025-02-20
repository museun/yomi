use mlua::{IntoLua, UserData};
use time::OffsetDateTime;

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

    let resp = attohttpc::get("https://crates.io/api/v1/crates")
        .header("User-Agent", crate::USER_AGENT)
        .params([("page", "1"), ("per_page", "1"), ("q", name)])
        .send()
        .ok()?;

    let value: serde_json::Value = resp.json().ok()?;
    eprintln!("{value:#?}");

    let mut resp: Resp = serde_json::from_value(value).ok()?;

    match resp.crates.len() {
        0 => None,
        _ => Some(resp.crates.remove(0)),
    }
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct Crate {
    pub name: String,
    #[serde(default)]
    pub yanked: bool,
    pub default_version: String,
    pub description: Option<String>,
    pub documentation: Option<String>,
    pub repository: Option<String>,
    pub exact_match: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl IntoLua for Crate {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let table = lua.create_table()?;
        table.set("name", self.name)?;
        table.set("yanked", self.yanked)?;
        table.set("default_version", self.default_version)?;
        table.set("description", self.description)?;
        table.set("documentation", self.documentation)?;
        table.set("repository", self.repository)?;
        table.set("exact_match", self.exact_match)?;
        table.set("updated_at", UtcTime(self.updated_at))?;
        Ok(mlua::Value::Table(table))
    }
}
