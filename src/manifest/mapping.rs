use crate::{
    irc::{Message, Owner},
    manifest::handled::Handled,
    pattern::{Extract, Pattern},
    HelixClient, Responder,
};

#[derive(Debug)]
pub struct Mapping {
    pub command: String,
    pub pattern: Option<Pattern>,
    pub raw_pattern: Option<String>,
    pub help: String,
    pub elevated: bool,
    pub requires_live: bool,
    pub handler: mlua::Function,
}

impl Mapping {
    fn make_error(&self) -> String {
        match &self.raw_pattern {
            Some(p) => format!("invalid usage. syntax: {} {p}", self.command),
            None => format!("invalid usage. syntax: {}", self.command),
        }
    }

    pub fn dispatch(&self, msg: &Message, lua: &mlua::Lua, responder: &Responder, sink: &mut bool) {
        let Some(data) = msg.data.strip_prefix(&self.command) else {
            return;
        };

        let data = data.trim();
        let value = match &self.pattern {
            Some(pat) if pat.is_optional() && data.is_empty() => {
                responder.reply(msg, self.make_error());
                return;
            }

            None if !data.is_empty() => {
                responder.reply(msg, self.make_error());
                return;
            }

            Some(pat) => match pat.extract(data) {
                Extract::NoMatch => {
                    responder.reply(msg, self.make_error());
                    return;
                }
                Extract::Match => mlua::Value::Nil,
                Extract::Bindings { map } => Extract::map_to_lua(map, lua),
            },
            None => mlua::Value::Nil,
        };

        if self.requires_live {
            match check_liveness(lua, responder, msg) {
                Ok(false) | Err(..) => return,
                Ok(true) => {}
            }
        }

        // TODO PartialOrd so we can see if this message matches what they
        // specified as the minimum access level
        if self.elevated && !msg.is_elevated() {
            responder.reply(msg, "you cannot do that command".to_string());
            return;
        }

        let err = match self.handler.call::<Option<Handled>>((msg, value)) {
            Ok(res) => {
                *sink = matches!(res, Some(Handled::Sink));
                return;
            }
            Err(err) => err,
        };

        if let Some(err) = err.to_string().lines().nth(0).and_then(|c| {
            c.split_terminator(": ").find(|c| {
                !(c.contains("runtime error") || c.contains("./scripts") || c.contains("src"))
            })
        }) {
            responder.error(msg, err.to_string());
        }

        log::warn!(
            "cannot call: {command} because {err}",
            command = self.command
        )
    }
}

fn check_liveness(lua: &mlua::Lua, responder: &Responder, msg: &Message) -> mlua::Result<bool> {
    let helix = lua
        .globals()
        .get::<mlua::AnyUserData>("helix")?
        .borrow_mut::<HelixClient>()?;

    let owner = lua.globals().get::<Owner>("BOT_OWNER")?;

    if helix.is_stream_live(owner.name()) {
        return Ok(true);
    }

    let data = format!(
        "{} is not streaming, this command is disabled",
        owner.name()
    );
    responder.reply(msg, data);
    log::warn!("stream not live, cannot run command");
    Ok(false)
}
