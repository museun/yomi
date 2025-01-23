use yomi::{
    irc, Config, GithubClient, HelixClient, Manifest, Responder, ResponderChannel, SpotifyClient,
    SpotifyHistory, Watcher,
};

enum Next {
    Event(irc::Event),
    Route(irc::Message),
    Continue,
    Quit,
}

fn handle_fs_event(
    ev: Result<(), flume::RecvError>,
    manifest: &mut Manifest,
    lua: &mlua::Lua,
) -> Next {
    if ev.is_err() {
        return Next::Quit;
    }
    match Manifest::read_init_lua(&manifest.init) {
        Ok(data) => {
            if let Err(err) = manifest.load(lua, &data) {
                log::error!("{err}");
            }
        }
        Err(err) => log::error!("cannot read init.lua: {err}"),
    }

    Next::Continue
}

fn handle_irc_event(ev: Result<irc::Event, flume::RecvError>) -> Next {
    ev.map(Next::Event).unwrap_or(Next::Continue)
}

fn handle_reoute_event(ev: Result<irc::Message, flume::RecvError>) -> Next {
    ev.map(Next::Route).unwrap_or(Next::Continue)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_env_load::load_env_from([".dev.env", ".secrets.env"]);
    alto_logger::init_term_logger().expect("single initalization of logger");

    // TODO actually parse cli args instead of this hack
    let arg = std::env::args().nth(1);
    let config_path = arg.as_deref().unwrap_or("config.lua");
    let config = Config::load(config_path)?;

    let lua = mlua::Lua::new();

    let (sender, responses) = flume::unbounded();
    let events = irc::connect(
        config.twitch.clone(), //
        responses,
    );

    let (reroute_tx, reroute) = flume::unbounded();

    let responder = ResponderChannel::new(sender);
    let watcher = Watcher::new(&config.paths.scripts);

    let data = Manifest::read_init_lua(config.paths.scripts.join("init.lua"))?;
    Manifest::set_responder(&lua, responder.clone())?;

    let helix = HelixClient::new(&config.twitch.client_id, &config.twitch.client_secret)?;
    let github = GithubClient::new(&config.github.oauth_token);
    let spotify = SpotifyClient::new(
        &config.spotify.client_id,
        &*config.spotify.client_secret,
        &*config.spotify.refresh_token,
        config.paths.data.join("spotify.db"),
    )?;

    let spotify_history = SpotifyHistory::new(config.paths.data.join("spotify.db"));

    let mut manifest = Manifest::initialize(
        &lua,
        &config.paths.scripts,
        &config.paths.data,
        &data,
        &config.github.settings_gist_id,
        reroute_tx,
        github,
        helix,
        spotify,
        spotify_history,
    )?;

    let mut our_user = irc::User::default();

    loop {
        let next = flume::Selector::new()
            .recv(watcher.next_event(), |ev| {
                handle_fs_event(ev, &mut manifest, &lua)
            })
            .recv(&events, handle_irc_event)
            .recv(&reroute, handle_reoute_event)
            .wait();

        let event = match next {
            Next::Event(event) => event,
            Next::Route(msg) => {
                manifest.dispatch(msg, &lua, &responder);
                continue;
            }

            Next::Continue => continue,
            Next::Quit => break,
        };

        match event {
            irc::Event::Connected { user } => {
                our_user = user;
                for channel in &config.twitch.channels {
                    responder.send(irc::Response::Join {
                        channel: channel.clone(),
                    });
                }
            }
            irc::Event::Disconnected {} => {}
            irc::Event::Message { msg } => {
                let msg = irc::Message {
                    our_user: our_user.name.clone(),
                    our_id: our_user.user_id.clone(),
                    channel: msg.channel.to_string(),
                    channel_id: msg.room_id().expect("attached room-id").to_string(),
                    msg_id: msg.msg_id().expect("attached msg-id").to_string(),
                    sender: msg.sender.to_string(),
                    sender_id: msg.user_id().expect("attached user-id").to_string(),
                    data: msg.data.to_string(),
                    elevated: msg.is_from_vip()
                        || msg.is_from_moderator()
                        || msg.is_from_broadcaster(),
                };
                manifest.dispatch(msg, &lua, &responder)
            }
        }
    }

    Ok(())
}
