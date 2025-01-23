use std::{future::Future, sync::Arc, time::Duration};

use mlua::{AnyUserData, FromLua, IntoLua};
use serde::de::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
};
use twitch_message::{
    encode::{self, Encodable, ALL_CAPABILITIES},
    messages::{MsgIdRef, Privmsg, TwitchMessage},
    IntoStatic, PingTracker,
};

use crate::{config::Twitch, responder::Responder};

#[derive(Debug, PartialEq, Eq)]
pub enum Response {
    Join {
        channel: String,
    },
    Error {
        channel: String,
        data: String,
    },
    Reply {
        channel: String,
        msg_id: String,
        data: String,
    },
    Say {
        channel: String,
        data: String,
    },
    Disconnect,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub our_user: String,
    pub our_id: String,
    pub channel: String,
    pub channel_id: String,
    pub msg_id: String,
    pub sender: String,
    pub sender_id: String,
    pub data: String,
    pub elevated: bool,
}
impl Message {
    pub const fn is_elevated(&self) -> bool {
        self.elevated
    }
}

impl IntoLua for &Message {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let table = lua.create_table_from([
            ("our_user", &*self.our_user),
            ("our_id", &*self.our_id),
            ("channel", &*self.channel),
            ("channel_id", &*self.channel_id),
            ("msg_id", &*self.msg_id),
            ("sender", &*self.sender),
            ("sender_id", &*self.sender_id),
            ("data", &*self.data),
        ])?;

        table.set("elevated", self.elevated)?;

        let responder = lua
            .globals()
            .get::<AnyUserData>("_RESPONDER")?
            .borrow::<Arc<dyn Responder>>()?;

        // this could just be a global lua function `irc.say(msg, data)` and `irc.reply(msg, data)`
        table.set("_responder", AnyUserData::wrap(responder.clone()))?;

        // BUG where is `msg:error`?
        table.set(
            "reply",
            lua.create_function({
                let responder = responder.clone();
                move |_lua, (this, data): (Message, String)| {
                    responder.send(Response::Reply {
                        channel: this.channel.to_string(),
                        msg_id: this.msg_id.to_string(),
                        data,
                    });
                    Ok(())
                }
            })?,
        )?;

        table.set(
            "say",
            lua.create_function({
                let responder = responder.clone();
                move |_lua, (this, data): (Message, String)| {
                    responder.send(Response::Say {
                        channel: this.channel.to_string(),
                        data,
                    });
                    Ok(())
                }
            })?,
        )?;

        Ok(mlua::Value::Table(table))
    }
}

// why do we need fromlua?
impl FromLua for Message {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        let table = value
            .as_table()
            .ok_or_else(|| mlua::Error::custom("Message type was invalid"))?;

        Ok(Self {
            our_user: table.get("our_user")?,
            our_id: table.get("our_id")?,
            channel: table.get("channel")?,
            channel_id: table.get("channel_id")?,
            msg_id: table.get("msg_id")?,
            sender: table.get("sender")?,
            sender_id: table.get("sender_id")?,
            data: table.get("data")?,
            elevated: table.get("elevated")?,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct User {
    pub name: String,
    pub display: String,
    pub user_id: String,
}

impl IntoLua for &User {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        lua.create_table_from([
            ("name", &*self.name),
            ("display", &*self.display),
            ("user_id", &*self.user_id),
        ])
        .map(mlua::Value::Table)
    }
}

#[derive(Debug)]
pub enum Event {
    Connected { user: User },
    Disconnected {},
    Message { msg: Privmsg<'static> },
}

pub fn connect(config: Twitch, response: flume::Receiver<Response>) -> flume::Receiver<Event> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("valid tokio runtime");

    let (events, out) = flume::unbounded();
    let _ = std::thread::spawn(move || {
        rt.block_on(async move {
            loop {
                // std::future::pending::<()>().await;
                match connect_to_twitch(config.clone(), &events, &response).await {
                    Next::Restart => continue,
                    Next::Stop => return,
                    Next::Nothing => {}
                }
            }
        });
    });
    out
}

enum Next {
    Restart,
    Stop,
    Nothing,
}

fn send(events: &flume::Sender<Event>, event: Event) -> bool {
    events.send(event).is_ok()
}

async fn maybe_reconnect(events: &flume::Sender<Event>, msg: std::fmt::Arguments<'_>) -> Next {
    log::warn!("{msg}");
    if !send(events, Event::Disconnected {}) {
        return Next::Stop;
    }
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    Next::Restart
}

async fn connect_to_twitch(
    config: Twitch,
    events: &flume::Sender<Event>,
    response: &flume::Receiver<Response>,
) -> Next {
    let stream = match TcpStream::connect(twitch_message::TWITCH_IRC_ADDRESS).await {
        Ok(stream) => stream,
        Err(err) => {
            return maybe_reconnect(
                events,
                format_args!("cannot connect, trying again because: {err}"),
            )
            .await;
        }
    };

    let mut user = User {
        name: config.name.clone(),
        display: config.name.clone(),
        user_id: String::new(),
    };

    let (read, mut write) = stream.into_split();
    let read = tokio::io::BufReader::new(read);

    let msg = encode::register(
        &config.name, //
        &config.helix_oauth,
        ALL_CAPABILITIES,
    );

    match encode_to(msg, &mut write, events).await {
        Next::Nothing => {}
        next => return next,
    }

    let pt = PingTracker::new(Duration::from_secs(3 * 60));

    let mut lines = read.lines();
    // is it this one?

    'outer: loop {
        let data = match lines.next_line().await {
            Ok(Some(data)) => data,
            Ok(None) => {
                return maybe_reconnect(events, format_args!("read an unexpected eof")).await
            }
            Err(err) => {
                return maybe_reconnect(events, format_args!("cannot read line: {err}")).await
            }
        };

        for msg in twitch_message::parse_many(&data).flatten() {
            pt.update(&msg);
            if let Some(msg) = pt.should_pong() {
                match encode_to(msg, &mut write, events).await {
                    Next::Nothing => {}
                    next => return next,
                }
            }

            match msg.as_enum() {
                TwitchMessage::Ready(ready) => user.name = ready.name.to_string(),
                TwitchMessage::GlobalUserState(gus) => {
                    user.display = gus
                        .display_name()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| user.name.clone());

                    user.user_id = gus
                        .user_id()
                        .map(ToString::to_string)
                        .expect("we must have a user id");
                    break 'outer;
                }
                _ => {}
            }
        }
    }

    let next = main_loop(user, events, response, lines, &mut write).await;

    log::info!("sending quit message");
    let quit = encode::quit("bye");
    let _ = encode_to(quit, &mut write, events).await;
    next
}

async fn main_loop(
    user: User,
    events: &flume::Sender<Event>,
    response: &flume::Receiver<Response>,
    mut stream: tokio::io::Lines<tokio::io::BufReader<tokio::net::tcp::OwnedReadHalf>>,
    mut write: &mut (impl AsyncWrite + Unpin),
) -> Next {
    log::info!("connected to Twitch: {user:#?}");

    if !send(events, Event::Connected { user: user.clone() }) {
        return Next::Stop;
    }

    let pt = PingTracker::new(Duration::from_secs(3 * 60));

    // drain any pending responses
    for _ in response.try_iter() {}

    loop {
        tokio::pin! {
            let read_line = stream.next_line();
            let next_response = response.recv_async();
        }

        let msg = match select2(&mut read_line, &mut next_response).await {
            Either::Left(Err(err)) => {
                return maybe_reconnect(events, format_args!("cannot read: {err}")).await;
            }

            Either::Left(Ok(None)) => {
                return maybe_reconnect(events, format_args!("unexpected EOF")).await;
            }

            Either::Left(Ok(Some(data))) => {
                for msg in twitch_message::parse_many(&data).flatten() {
                    pt.update(&msg);
                    if let Some(msg) = pt.should_pong() {
                        match encode_to(msg, &mut write, events).await {
                            Next::Nothing => {}
                            next => return next,
                        }
                    }

                    match msg.as_enum() {
                        TwitchMessage::Reconnect(..) => {
                            return maybe_reconnect(
                                events,
                                format_args!("cannot read, server asked us to reconnect"),
                            )
                            .await
                        }

                        TwitchMessage::RoomState(..) => {}
                        TwitchMessage::Privmsg(privmsg) => {
                            let event = Event::Message {
                                msg: privmsg.into_static(),
                            };
                            if !send(events, event) {
                                return Next::Stop;
                            }
                        }
                        _ => {}
                    }
                }
                continue;
            }
            Either::Right(msg) => msg,
        };

        let Ok(msg) = msg else { return Next::Stop };

        match msg {
            Response::Join { channel } => {
                let msg = encode::join(&channel);
                match encode_to(msg, &mut write, events).await {
                    Next::Nothing => {}
                    next => return next,
                }
            }

            Response::Error { channel, data } => {
                let data = format!("error: {data}");
                let msg = encode::privmsg(&channel, &data);
                match encode_to(msg, &mut write, events).await {
                    Next::Nothing => {}
                    next => return next,
                }
            }

            Response::Reply {
                channel,
                msg_id,
                data,
            } => {
                let msg = encode::reply(MsgIdRef::from_str(&msg_id), &channel, &data);
                match encode_to(msg, &mut write, events).await {
                    Next::Nothing => {}
                    next => return next,
                }
            }

            Response::Say { channel, data } => {
                let msg = encode::privmsg(&channel, &data);
                match encode_to(msg, &mut write, events).await {
                    Next::Nothing => {}
                    next => return next,
                }
            }

            Response::Disconnect => {
                return maybe_reconnect(events, format_args!("user requested to reconnect")).await
            }
        }
    }
}

enum Either<L, R> {
    Left(L),
    Right(R),
}

async fn select2<L, R>(left: &mut L, right: &mut R) -> Either<L::Output, R::Output>
where
    L: Future + Unpin,
    R: Future + Unpin,
{
    tokio::select! {
        left = left => Either::Left(left),
        right = right => Either::Right(right),
    }
}

async fn encode_to(
    msg: impl Encodable,
    write: &mut (impl AsyncWrite + Unpin),
    events: &flume::Sender<Event>,
) -> Next {
    let mut buf = vec![];
    msg.encode(&mut buf).expect("valid encoding");

    if let Err(err) = write.write_all(&buf).await {
        return maybe_reconnect(
            events,
            format_args!("cannot write, reconnecting because: {err}"),
        )
        .await;
    }

    if let Err(err) = write.flush().await {
        return maybe_reconnect(
            events,
            format_args!("cannot flush, reconnecting because: {err}"),
        )
        .await;
    }

    Next::Nothing
}
