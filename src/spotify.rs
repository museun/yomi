#![cfg_attr(debug_assertions, allow(dead_code, unused_variables,))]
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use mlua::{FromLua, IntoLua, LuaSerdeExt, UserData};
use url::Url;

use crate::{sql::DbError, time::TimeSpan, GlobalItem, ResultExt};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("that was an invalid url")]
    InvalidUrl,

    #[error("that is missing spotify urn")]
    MissingUrn,

    #[error("only tracks are allowed")]
    TrackOnly,

    #[error("cannot get new spotify token")]
    CannotGetNewToken,

    #[error("there is nothing is in the queue")]
    NothingInQueue,

    #[error("http error: {0}")]
    Http(#[from] attohttpc::Error),
}

#[derive(Debug)]
struct State {
    client_id: String,
    client_secret: String,
    refresh_token: String,
    access_token: String,
}

impl State {
    fn new(
        client_id: impl ToString,
        client_secret: impl ToString,
        refresh_token: impl ToString,
        session: &attohttpc::Session,
    ) -> Result<Self, Error> {
        let mut this = Self {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            refresh_token: refresh_token.to_string(),
            access_token: String::new(),
        };

        this.refresh(session)?;
        Ok(this)
    }

    fn refresh(&mut self, session: &attohttpc::Session) -> Result<(), Error> {
        #[derive(Debug, serde::Deserialize)]
        struct Response {
            access_token: String,
            #[allow(dead_code)]
            scope: String,
        }

        let resp = session
            .post("https://accounts.spotify.com/api/token")
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", &self.refresh_token),
                ("client_id", &self.client_id),
            ])?
            .send()?;

        let resp = resp.error_for_status()?;
        let resp = resp.json::<Response>()?;
        self.access_token = resp.access_token;

        Ok(())
    }
}

#[derive(Clone)]
pub struct Client {
    state: Arc<Mutex<State>>,
    session: Arc<Mutex<attohttpc::Session>>,
}

impl GlobalItem for Client {
    const MODULE: &'static str = "spotify";
}

impl UserData for Client {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method("current", |lua, this, ()| {
            match this.get_currently_playing().ok() {
                Some(CurrenlyPlaying::Playing(item)) => item.into_lua(lua),
                Some(CurrenlyPlaying::NotPlaying) | None => Ok(mlua::Value::Nil),
            }
        });

        methods.add_method("next", |lua, this, ()| match this.get_queue().ok() {
            Some((_, list)) => match list.first() {
                Some(item) => item.clone().into_lua(lua),
                None => Ok(mlua::Value::Nil),
            },
            None => Ok(mlua::Value::Nil),
        });

        methods.add_method("skip", |_lua, this, ()| {
            Ok(this.skip_song().ok().unwrap_or(false))
        });

        methods.add_method("search", |_lua, this, query: String| {
            this.search(&query).into_lua_tuple()
        });

        methods.add_function("parse", |_lua, input: String| {
            SpotifyUrn::try_from(input.as_str()).into_lua_tuple()
        });

        methods.add_method("add_to_queue", |_lua, this, input: SpotifyUrn| {
            match this.add_to_queue(&input) {
                Ok(false) => return Ok((None, Some(String::from("could not add that song")))),
                Err(err) => return Ok((None, Some(err.to_string()))),
                _ => {}
            };

            this.lookup_by_urn(&input).into_lua_tuple()
        });
    }
}

impl Client {
    pub fn new(
        client_id: impl ToString,
        client_secret: impl ToString,
        refresh_token: impl ToString,
    ) -> Result<Self, Error> {
        let session = {
            let mut session = attohttpc::Session::new();
            session.header("user-agent", crate::USER_AGENT);
            session
        };

        let state = State::new(client_id, client_secret, refresh_token, &session)?;
        Ok(Self {
            state: Arc::new(Mutex::new(state)),
            session: Arc::new(Mutex::new(session)),
        })
    }

    pub fn listen_for_changes(this: &Self, path: PathBuf) {
        std::thread::spawn({
            let this = this.clone();
            move || {
                let mut backoff = 10;
                loop {
                    match this.get_currently_playing() {
                        Ok(CurrenlyPlaying::Playing(item)) => {
                            {
                                let history = History::open(&path).unwrap();
                                let _ = history.push(&item.id, &item).unwrap();
                            }
                            backoff = 10;
                            std::thread::sleep(Duration::from_secs(30));
                        }
                        Ok(CurrenlyPlaying::NotPlaying) | Err(..) => {
                            std::thread::sleep(Duration::from_secs(backoff));
                            backoff += 10;
                        }
                    }
                }
            }
        });
    }

    pub fn get_currently_playing(&self) -> Result<CurrenlyPlaying, Error> {
        #[derive(serde::Deserialize)]
        struct Response {
            is_playing: bool,
            #[serde(rename = "progress_ms", with = "spotify_duration")]
            progress: Duration,
            item: Item,
        }
        let resp =
            self.send(|s| s.get("https://api.spotify.com/v1/me/player/currently-playing"))?;

        if resp.status() == attohttpc::StatusCode::NO_CONTENT {
            return Ok(CurrenlyPlaying::NotPlaying);
        }

        match resp.json::<Option<Response>>()? {
            Some(resp) if resp.is_playing => {
                let mut item = resp.item;
                item.progress = Some(resp.progress);
                Ok(CurrenlyPlaying::Playing(item))
            }
            Some(..) | None => Ok(CurrenlyPlaying::NotPlaying),
        }
    }

    pub fn get_queue(&self) -> Result<(Option<Item>, Vec<Item>), Error> {
        #[derive(serde::Deserialize)]
        struct Response {
            currently_playing: Option<Item>,
            queue: Vec<Item>,
        }

        let resp = self.send(|s| s.get("https://api.spotify.com/v1/me/player/queue"))?;
        let resp = resp.json::<Response>()?;
        Ok((resp.currently_playing, resp.queue))
    }

    pub fn skip_song(&self) -> Result<bool, Error> {
        let resp = self.send(|s| {
            s.post("https://api.spotify.com/v1/me/player/next")
                .header(attohttpc::header::CONTENT_LENGTH, 0)
        })?;

        Ok(matches!(
            resp.status(),
            attohttpc::StatusCode::OK | attohttpc::StatusCode::NO_CONTENT
        ))
    }

    pub fn search(&self, query: &str) -> Result<Vec<Item>, Error> {
        #[derive(serde::Deserialize)]
        struct Response {
            tracks: Tracks,
        }
        #[derive(serde::Deserialize)]
        struct Tracks {
            items: Vec<Item>,
        }

        let resp = self.send(|s| {
            s.get("https://api.spotify.com/v1/search").params(&[
                ("q", query),
                ("type", "track"),
                ("limit", "3"),
            ])
        })?;

        let resp = resp.json::<Response>()?;
        Ok(resp.tracks.items)
    }

    pub fn add_to_queue(&self, urn: &SpotifyUrn) -> Result<bool, Error> {
        let resp = self.send(|s| {
            s.post("https://api.spotify.com/v1/me/player/queue")
                .header(attohttpc::header::CONTENT_LENGTH, 0)
                .param("uri", format!("spotify:track:{}", urn.0))
        })?;

        Ok(matches!(
            resp.status(),
            attohttpc::StatusCode::OK | attohttpc::StatusCode::NO_CONTENT
        ))
    }

    fn lookup_by_urn(&self, urn: &SpotifyUrn) -> Result<Item, Error> {
        self.send(|s| s.get(format!("https://api.spotify.com/v1/tracks/{}", urn.0)))?
            .json::<Item>()
            .map_err(Into::into)
    }

    fn send(
        &self,
        req: impl Fn(&mut attohttpc::Session) -> attohttpc::RequestBuilder,
    ) -> Result<attohttpc::Response, Error> {
        let mut session = self.session.lock().unwrap();

        let mut n = 0;
        loop {
            if n > 1 {
                return Err(Error::CannotGetNewToken);
            }
            let resp = req(&mut session)
                .bearer_auth(&self.state.lock().unwrap().access_token)
                .send()?;
            if resp.status() == attohttpc::StatusCode::UNAUTHORIZED {
                self.state.lock().unwrap().refresh(&session)?;
                n += 1;
                continue;
            }

            break Ok(resp);
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SpotifyUrn(String);

impl FromLua for SpotifyUrn {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        lua.from_value(value)
    }
}

impl IntoLua for SpotifyUrn {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        lua.to_value(&self)
    }
}

impl<'a> TryFrom<&'a str> for SpotifyUrn {
    type Error = Error;
    fn try_from(input: &'a str) -> Result<Self, Self::Error> {
        let Ok(url) = Url::parse(input) else {
            return Err(Error::InvalidUrl);
        };

        if url.scheme() == "spotify" {
            let Some(urn) = url.path().strip_prefix("track:") else {
                return Err(Error::TrackOnly);
            };
            return Ok(Self(urn.to_string()));
        }

        if url.host_str() != Some("open.spotify.com") {
            return Err(Error::InvalidUrl);
        }

        let Some(urn) = url.path().strip_prefix("/track/") else {
            return Err(Error::MissingUrn);
        };

        Ok(Self(urn.to_string()))
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Item {
    #[serde(rename = "duration_ms", with = "spotify_duration")]
    pub duration: Duration,
    pub name: String,
    pub id: String,
    pub artists: Vec<Artist>,
    #[serde(default, skip_serializing)]
    pub progress: Option<Duration>,
}

impl IntoLua for Item {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let table = lua.create_table()?;
        let duration = TimeSpan(::time::Duration::new(self.duration.as_secs() as _, 0));
        table.set("duration", duration)?;
        table.set("name", self.name)?;
        table.set("id", self.id)?;
        table.set("artists", self.artists)?;
        let progress = self
            .progress
            .map(|d| TimeSpan(::time::Duration::new(d.as_secs() as _, 0)));
        table.set("progress", progress)?;
        Ok(mlua::Value::Table(table))
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Artist {
    pub name: String,
    pub id: String,
}

impl IntoLua for Artist {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        lua.create_table_from([("name", self.name), ("id", self.id)])
            .map(mlua::Value::Table)
    }
}

#[derive(Debug)]
pub enum CurrenlyPlaying {
    Playing(Item),
    NotPlaying,
}

mod spotify_duration {
    use serde::{Deserialize, Serialize};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(Duration::from_millis)
    }
}

pub struct SpotifyHistory(PathBuf);

impl GlobalItem for SpotifyHistory {
    const MODULE: &'static str = "spotify_history";
}

impl SpotifyHistory {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }
}

impl UserData for SpotifyHistory {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method("last", |_lua, this, ()| {
            let history = History::open(&this.0).map_err(mlua::Error::external)?;
            match history.last_n(2) {
                Ok(item) => match item.as_slice() {
                    [_, item] => Ok((Some(item.clone()), None)),
                    _ => Ok((None, None)),
                },
                Err(err) => Ok((None, Some(err.to_string()))),
            }
        });

        methods.add_method("history", |_lua, this, n: usize| {
            let history = History::open(&this.0).map_err(mlua::Error::external)?;
            match history.last_n(n) {
                Ok(items) => Ok((Some(items), None)),
                Err(err) => Ok((None, Some(err.to_string()))),
            }
        });

        methods.add_method("all", |_lua, this, ()| {
            let history = History::open(&this.0).map_err(mlua::Error::external)?;
            match history.all() {
                Ok(items) => Ok((Some(items), None)),
                Err(err) => Ok((None, Some(err.to_string()))),
            }
        });

        methods.add_method("count", |_lua, this, urn: SpotifyUrn| {
            let history = History::open(&this.0).map_err(mlua::Error::external)?;
            match history.count(&urn.0) {
                Ok(count) => Ok((Some(count), None)),
                Err(err) => Ok((None, Some(err.to_string()))),
            }
        });
    }
}

struct History {
    conn: rusqlite::Connection,
}

macro_rules! include_sql {
    ($name:expr) => {{
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/sql/spotify/",
            concat!($name, ".sql")
        ))
    }};
}

impl History {
    fn open(path: impl AsRef<Path>) -> Result<Self, DbError> {
        static SCHEMA: &str = include_sql!("schema");
        let conn = rusqlite::Connection::open(path)
            .map_err(|err| DbError::CannotOpenDb(err.to_string()))?;
        conn.execute(SCHEMA, [])?;
        Ok(Self { conn })
    }

    fn push(&self, key: &str, data: &Item) -> Result<usize, DbError> {
        static PUSH: &str = include_sql!("push");
        let value = serde_json::to_value(data).expect("valid shape");
        let params = rusqlite::params![key, value, key];
        Ok(self.conn.execute(PUSH, params)?)
    }

    fn remove(&self, key: &str) -> Result<usize, DbError> {
        static REMOVE: &str = include_sql!("remove");
        Ok(self.conn.execute(REMOVE, [key])?)
    }

    fn remove_all(&self, key: &str) -> Result<usize, DbError> {
        static REMOVE_ALL: &str = include_sql!("remove_all");
        Ok(self.conn.execute(REMOVE_ALL, [key])?)
    }

    fn clear(&self) -> Result<usize, DbError> {
        static CLEAR: &str = include_sql!("clear");

        Ok(self.conn.execute(CLEAR, [])?)
    }

    fn count(&self, key: &str) -> Result<usize, DbError> {
        static COUNT: &str = include_sql!("count");
        Ok(self.conn.query_row(COUNT, [key], |row| row.get(0))?)
    }

    fn all(&self) -> Result<Vec<Item>, DbError> {
        static ALL: &str = include_sql!("all");
        let mut stmt = self.conn.prepare(ALL)?;
        let query = stmt
            .query_map([], |row| {
                let value = row.get("value")?;
                Ok(serde_json::from_value(value).expect("valid shape"))
            })?
            .map(|c| Ok(c?));
        query.collect()
    }

    fn last_n(&self, n: usize) -> Result<Vec<Item>, DbError> {
        static LAST: &str = include_sql!("last");
        let mut stmt = self.conn.prepare(LAST)?;
        let query = stmt
            .query_map([n], |row| {
                let value = row.get("value")?;
                Ok(serde_json::from_value(value).expect("valid shape"))
            })?
            .map(|c| Ok(c?));
        query.collect()
    }

    fn last(&self) -> Result<Option<Item>, DbError> {
        static LAST: &str = include_sql!("last");
        let mut stmt = self.conn.prepare(LAST)?;
        let result = stmt.query_row([1], |row| {
            let value = row.get("value")?;
            Ok(serde_json::from_value(value).expect("valid shape"))
        });
        match result {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}
