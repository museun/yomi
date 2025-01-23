use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use mlua::{IntoLua, UserData};
use url::Url;

use crate::time::TimeSpan;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid url")]
    InvalidUrl,

    #[error("Missing spotify urn")]
    MissingUrn,

    #[error("Only tracks are allowed")]
    TrackOnly,

    #[error("Cannot get new spotify token")]
    CannotGetNewToken,

    #[error("Http error: {0}")]
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

struct Queue<T, const N: usize = 10> {
    inner: VecDeque<T>,
}

impl<T> Default for Queue<T, 10> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> Queue<T, N> {
    pub const fn new() -> Self {
        const { assert!(N != 0, "Queue cannot be empty") }
        Self {
            inner: VecDeque::new(),
        }
    }

    pub fn push(&mut self, item: T) {
        while self.inner.len() >= N {
            self.inner.pop_front();
        }
        self.inner.push_back(item);
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a Queue<T, N> {
    type Item = &'a T;
    type IntoIter = std::collections::vec_deque::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        (&self.inner).into_iter()
    }
}

#[derive(Default, Clone)]
pub struct History {
    queue: Arc<Mutex<Queue<Item>>>,
}

impl UserData for History {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("previous", |_lua, this, ()| Ok(this.last()));
    }
}

impl History {
    pub fn push(&self, item: Item) {
        self.queue.lock().unwrap().push(item);
    }

    pub fn last(&self) -> Option<Item> {
        self.queue.lock().unwrap().inner.back().cloned()
    }

    pub fn listen_for_changes(client: &Client) -> Self {
        let this = Self::default();
        std::thread::spawn({
            let client = client.clone();
            let this = this.clone();
            move || {
                let mut backoff = 10;
                loop {
                    match client.get_currently_playing() {
                        Ok(CurrenlyPlaying::Playing(item)) => {
                            backoff = 10;
                            let next = item.duration - item.progress.unwrap();
                            this.push(item);
                            std::thread::sleep(next + Duration::from_secs(2));
                        }
                        _ => {
                            std::thread::sleep(std::time::Duration::from_secs(backoff));
                            backoff += 10
                        }
                    }
                }
            }
        });
        this
    }
}

#[derive(Clone)]
pub struct Client {
    state: Arc<Mutex<State>>,
    session: Arc<Mutex<attohttpc::Session>>,
    history: History,
}

impl UserData for Client {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method("current", |lua, this, ()| {
            let current = this
                .get_currently_playing()
                .map_err(mlua::Error::external)?;
            match current {
                CurrenlyPlaying::Playing(item) => item.into_lua(lua),
                CurrenlyPlaying::NotPlaying => Ok(mlua::Value::Nil),
            }
        });

        methods.add_method("previous", |_lua, this, ()| Ok(this.history.last()));
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
        let mut this = Self {
            state: Arc::new(Mutex::new(state)),
            session: Arc::new(Mutex::new(session)),
            history: History::default(),
        };

        this.history = History::listen_for_changes(&this);
        Ok(this)
    }

    pub fn get_currently_playing(&self) -> Result<CurrenlyPlaying, Error> {
        #[derive(serde::Deserialize)]
        struct Response {
            is_playing: bool,
            #[serde(rename = "progress_ms", deserialize_with = "spotify_duration")]
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

    pub fn add_to_queue(&self, urn: &SpotifyUrn) -> Result<bool, Error> {
        let resp = self.send(|s| {
            s.post("https://api.spotify.com/v1/me/player/queue")
                .header(attohttpc::header::CONTENT_LENGTH, 0)
                .param("uri", &format!("spotify:track:{}", urn.0))
        })?;

        Ok(matches!(
            resp.status(),
            attohttpc::StatusCode::OK | attohttpc::StatusCode::NO_CONTENT
        ))
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

fn spotify_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde::Deserialize::deserialize(deserializer).map(Duration::from_millis)
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpotifyUrn(String);

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

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Item {
    #[serde(rename = "duration_ms", deserialize_with = "spotify_duration")]
    pub duration: Duration,
    pub name: String,
    pub id: String,
    pub artists: Vec<Artist>,
    #[serde(default)]
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

#[derive(Clone, Debug, serde::Deserialize)]
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
