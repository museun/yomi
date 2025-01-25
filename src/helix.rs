use std::collections::{HashMap, HashSet};

use mlua::{IntoLua, UserData};

use crate::GlobalItem;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Twitch client id was empty")]
    EmptyClientId,
    #[error("Twitch client client was empty")]
    EmptyClientSecret,
    #[error("Http error: {0}")]
    Http(#[from] attohttpc::Error),
}

pub struct Client {
    agent: attohttpc::Session,
    oauth: OAuth,
    base: Option<String>,
}

impl GlobalItem for Client {
    const MODULE: &'static str = "helix";
}

impl Client {
    pub fn new(client_id: &str, client_secret: &str) -> Result<Self, Error> {
        let oauth = OAuth::create(client_id, client_secret)?;
        Ok(Self::new_with_ep(Option::<String>::None, oauth))
    }

    fn new_with_ep(ep: impl Into<Option<String>>, oauth: OAuth) -> Self {
        let mut agent = attohttpc::Session::new();
        agent.header_append("user-agent", crate::USER_AGENT);

        Self {
            agent,
            oauth,
            base: ep.into().map(Into::into),
        }
    }

    pub fn get_streams<const N: usize>(
        &self,
        names: [&str; N],
    ) -> Result<Vec<data::Stream>, Error> {
        self.get_response(
            "streams",
            &std::iter::repeat("user_login")
                .zip(names)
                .collect::<Vec<_>>(),
        )
        .map(|data| data.data)
    }

    pub fn get_global_emotes(&self) -> Result<(String, Vec<data::Emote>), Error> {
        self.get_response("chat/emotes/global", &[])
            .map(|data| (data.template, data.data))
    }

    pub fn get_emotes_for(
        &self,
        broadcaster_id: &str,
    ) -> Result<(String, Vec<data::Emote>), Error> {
        self.get_response("chat/emotes/global", &[("broadcaster_id", broadcaster_id)])
            .map(|data| (data.template, data.data))
    }

    fn get_response<T>(&self, ep: &str, query: &[(&str, &str)]) -> Result<data::Data<T>, Error>
    where
        for<'de> T: ::serde::Deserialize<'de> + Send + 'static,
    {
        const BASE_URL: &str = "https://api.twitch.tv/helix";
        let url = format!("{}/{}", self.base.as_deref().unwrap_or(BASE_URL), ep);

        let response = [
            ("client-id", self.oauth.get_client_id()),
            ("authorization", self.oauth.get_bearer_token()),
        ]
        .into_iter()
        .fold(self.agent.get(&url), |req, (k, v)| req.header(k, v))
        .params(query)
        .header("User-Agent", crate::USER_AGENT)
        .send()?;

        Ok(response.json()?)
    }
}

impl UserData for Client {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method("get_stream", |lua, this, name: String| {
            let name = name.strip_prefix('#').unwrap_or(&name);
            // TODO Ok((Option, Option)) so local ok, err = func() will work
            let mut list = this.get_streams([name]).map_err(mlua::Error::external)?;
            let item = match list.len() {
                0 => return Ok(mlua::Value::Nil),
                1 => list.pop().unwrap(),
                _ => list.remove(0),
            };
            item.into_lua(lua)
        });

        methods.add_method("get_emotes_for", |_lua, this, broadcaster_id: String| {
            // TODO Ok((Option, Option)) so local ok, err = func() will work
            let (_, emotes) = this
                .get_emotes_for(&broadcaster_id)
                .map_err(mlua::Error::external)?;
            Ok(emotes)
        });
    }
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
#[allow(dead_code)]
struct OAuth {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
    token_type: String,

    #[serde(default)]
    client_id: String,

    #[serde(skip)]
    bearer_token: String,
}

impl OAuth {
    fn create(client_id: &str, client_secret: &str) -> Result<Self, Error> {
        if client_id.is_empty() {
            return Err(Error::EmptyClientId);
        }
        if client_secret.is_empty() {
            return Err(Error::EmptyClientId);
        }

        let req = attohttpc::post("https://id.twitch.tv/oauth2/token").params(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("grant_type", "client_credentials"),
        ]);

        let resp = req.send()?.json();

        Ok(resp.map(|this: Self| Self {
            client_id: client_id.to_string(),
            bearer_token: format!("Bearer {}", this.access_token),
            ..this
        })?)
    }

    fn get_client_id(&self) -> &str {
        &self.client_id
    }

    fn get_bearer_token(&self) -> &str {
        &self.bearer_token
    }
}

// this comes from TMI not Helix
#[derive(Clone, Debug, Default)]
pub struct EmoteMap {
    name_to_id: HashMap<String, String>,
    id_to_name: HashMap<String, String>,
    names: HashSet<String>,
}

impl GlobalItem for EmoteMap {
    const MODULE: &'static str = "emotes";
}

impl UserData for EmoteMap {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method("get_name", |_lua, this, id: String| {
            Ok(this.get_name(&id).map(ToString::to_string))
        });

        methods.add_method("get_id", |_lua, this, name: String| {
            Ok(this.get_id(&name).map(ToString::to_string))
        });

        methods.add_method("has", |_lua, this, name: String| {
            Ok(this.has(&name)) //
        });

        methods.add_method("names", |lua, this, ()| {
            Ok(lua.create_sequence_from(this.names()))
        });
    }
}

impl EmoteMap {
    pub fn fetch_emotes(client: &Client) -> Result<Self, Error> {
        client.get_global_emotes().map(|(_, map)| {
            map.iter()
                .map(|emote| (&*emote.name, &*emote.id))
                .fold(EmoteMap::default(), |map, (name, id)| {
                    map.with_emote(name, id)
                })
        })
    }

    pub fn with_emotes<'k, 'v, I>(self, iter: I) -> Self
    where
        I: Iterator<Item = (&'k str, &'v str)>,
    {
        iter.fold(self, |this, (name, id)| this.with_emote(name, id))
    }

    pub fn with_emote(mut self, name: &str, id: &str) -> Self {
        self.id_to_name.insert(id.into(), name.into());
        self.name_to_id.insert(name.into(), id.into());
        self.names.insert(name.into());
        self
    }

    pub fn get_name(&self, id: &str) -> Option<&str> {
        self.id_to_name.get(id).map(|s| &**s)
    }

    pub fn get_id(&self, name: &str) -> Option<&str> {
        self.name_to_id.get(name).map(|s| &**s)
    }

    pub fn has(&self, name: &str) -> bool {
        self.name_to_id.contains_key(name)
    }

    // will we use this?
    pub fn names(&self) -> impl ExactSizeIterator<Item = &str> + use<'_> {
        self.names.iter().map(|s| &**s)
    }
}

pub mod data {
    use std::{borrow::Cow, str::FromStr};

    use mlua::IntoLua;
    use serde::Deserializer;
    use time::{format_description::FormatItem, macros::format_description, OffsetDateTime};

    use crate::time::UtcTime;

    #[derive(serde::Deserialize)]
    pub struct Data<T> {
        pub data: Vec<T>,
        #[serde(default)]
        pub template: String,
    }

    #[derive(Clone, Debug, serde::Deserialize)]
    pub struct Stream {
        #[serde(deserialize_with = "self::from_str")]
        pub id: u64,

        #[serde(deserialize_with = "self::from_str")]
        pub user_id: u64,
        pub user_name: String,

        #[serde(deserialize_with = "self::from_str")]
        pub game_id: u64,
        pub title: String,
        pub viewer_count: u64,

        #[serde(deserialize_with = "self::assume_utc_date_time")]
        pub started_at: UtcTime,
    }

    impl IntoLua for Stream {
        fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
            let table = lua.create_table()?;
            table.set("id", self.id)?;
            table.set("user_id", self.user_id)?;
            table.set("user_name", self.user_name)?;
            table.set("game_id", self.game_id)?;
            table.set("title", self.title)?;
            table.set("viewer_count", self.viewer_count)?;
            table.set("started_at", self.started_at)?;
            Ok(mlua::Value::Table(table))
        }
    }

    #[derive(Debug, Clone, serde::Deserialize)]
    pub struct Emote {
        pub id: String,
        pub name: String,
    }

    impl IntoLua for Emote {
        fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
            let table = lua.create_table()?;
            table.set("id", self.id)?;
            table.set("name", self.name)?;
            Ok(mlua::Value::Table(table))
        }
    }

    fn assume_utc_date_time<'de, D>(deser: D) -> Result<UtcTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Deserialize as _;
        const FORMAT: &[FormatItem<'static>] = format_description!(
            "[year]-[month]-[day]T\
        [hour]:[minute]:[second]Z\
        [offset_hour sign:mandatory][offset_minute]"
        );

        let s = <Cow<'_, str>>::deserialize(deser)? + "+0000";
        OffsetDateTime::parse(&s, &FORMAT)
            .map_err(serde::de::Error::custom)
            .map(UtcTime)
    }

    fn from_str<'de, D, T>(deser: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: std::fmt::Display,
        D: Deserializer<'de>,
    {
        use serde::de::Deserialize as _;
        <Cow<'_, str>>::deserialize(deser)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}
