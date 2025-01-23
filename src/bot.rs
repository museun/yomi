use mlua::UserData;

use crate::irc;

pub struct Bot {
    tx: flume::Sender<irc::Message>,
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
        methods.add_method_mut(
            "reroute_command",
            |_lua, this, (mut msg, command): (irc::Message, String)| {
                msg.data = command;
                let _ = this.tx.send(msg);
                Ok(())
            },
        );
    }
}
