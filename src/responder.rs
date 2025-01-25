use mlua::UserData;

use crate::{
    irc::{Message, Response},
    GlobalItem,
};

#[derive(Clone, Debug)]
pub struct Responder {
    tx: flume::Sender<Response>,
}

impl GlobalItem for Responder {
    const MODULE: &'static str = "_RESPONDER";
}

impl Responder {
    pub const fn new(tx: flume::Sender<Response>) -> Self {
        Self { tx }
    }
}

impl UserData for Responder {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method("say", |_lua, this, (msg, data): (Message, String)| {
            Ok(this.say(&msg, data))
        });

        methods.add_method("reply", |_lua, this, (msg, data): (Message, String)| {
            Ok(this.reply(&msg, data))
        });

        methods.add_method("error", |_lua, this, (msg, data): (Message, String)| {
            Ok(this.error(&msg, data))
        });
    }
}

impl Responder {
    pub fn send(&self, response: Response) {
        _ = self.tx.send(response)
    }

    pub fn say(&self, msg: &Message, data: String) {
        self.send(Response::Say {
            channel: msg.channel.clone(),
            data,
        });
    }

    pub fn reply(&self, msg: &Message, data: String) {
        self.send(Response::Reply {
            channel: msg.channel.clone(),
            msg_id: msg.msg_id.clone(),
            data,
        });
    }
    pub fn error(&self, msg: &Message, data: String) {
        self.send(Response::Error {
            channel: msg.channel.clone(),
            data,
        });
    }
}
