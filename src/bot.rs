use mlua::UserData;

use crate::{irc, GlobalItem};

pub struct Bot {
    tx: flume::Sender<irc::Message>,
}

impl GlobalItem for Bot {
    const MODULE: &'static str = "bot";
}

impl Bot {
    pub const fn new(tx: flume::Sender<irc::Message>) -> Self {
        Self { tx }
    }
}

impl UserData for Bot {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method(
            "reroute_command",
            |_lua, this, (mut msg, command): (irc::Message, Option<String>)| {
                msg.data = command.unwrap_or(msg.data);
                let _ = this.tx.send(msg);
                Ok(())
            },
        );
    }
}
