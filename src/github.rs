use std::collections::HashMap;

use mlua::{IntoLua, UserData};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Http error: {0}")]
    Http(#[from] attohttpc::Error),
}

pub struct Client {
    bearer_token: String,
}

impl UserData for Client {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method("get_gist_files", |_lua, this, id: String| {
            this.get_gist_files(&id).map_err(mlua::Error::external)
        });
    }
}

impl Client {
    pub fn new(bearer_token: &str) -> Self {
        Self {
            bearer_token: format!("token {bearer_token}"),
        }
    }

    pub fn get_gist_files(&self, id: &str) -> Result<HashMap<String, GistFile>, Error> {
        #[derive(Debug, serde::Deserialize)]
        struct Response {
            files: HashMap<String, GistFile>,
        }

        let resp: Response = attohttpc::get(format!("https://api.github.com/gists/{id}"))
            .header("user-agent", crate::USER_AGENT)
            .header("accept", "application/vnd.github+json")
            .header("authorization", &self.bearer_token)
            .send()?
            .json()?;

        Ok(resp.files)
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct GistFile {
    pub content: String,
}

impl IntoLua for GistFile {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        self.content.into_lua(lua)
    }
}
