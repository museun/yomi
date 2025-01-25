use yomi::{
    irc, Aliases, Config, EmoteMap, GithubClient, GlobalItem, Globals, HelixClient, Manifest,
    SpotifyClient, SpotifyHistory, Watcher,
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

    let data = loop {
        let data = match std::fs::read_to_string(&manifest.init) {
            Ok(data) => data,
            Err(err) => {
                log::error!("{err}");
                return Next::Continue;
            }
        };
        if !data.trim().is_empty() {
            break data;
        }
        std::hint::spin_loop();
        std::thread::sleep(std::time::Duration::from_millis(10));
    };

    if let Err(err) = manifest.load(lua, &data) {
        log::error!("{err}");
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
    let responder = yomi::Responder::new(sender);

    let watcher = Watcher::new(&config.paths.scripts);

    let helix = HelixClient::new(
        &config.twitch.client_id, //
        &config.twitch.client_secret,
    )?;
    let emote_map = EmoteMap::fetch_emotes(&helix)?;

    let github = GithubClient::new(&config.github.oauth_token);

    let spotify = SpotifyClient::new(
        &config.spotify.client_id,
        &*config.spotify.client_secret,
        &*config.spotify.refresh_token,
    )?;

    let spotify_history_db = config.paths.data("spotify_history").with_extension("db");
    SpotifyClient::listen_for_changes(&spotify, &spotify_history_db);

    let aliases_db = config.paths.data("aliases").with_extension("db");

    let (reroute_tx, reroute) = flume::unbounded();

    Globals::new(&lua)
        .register(&config)?
        .register(yomi::LoadedModules)?
        .register(yomi::Logger)?
        .register(yomi::Regexp)?
        .register(yomi::Json)?
        .register(yomi::Store::new(&config.paths.data))?
        .register(yomi::Bot::new(reroute_tx))?
        .register(yomi::Rando::new())?
        .register(yomi::Handled::Sink)?
        .register(yomi::fuzzy::Search)?
        .register(yomi::crates::Crates)?
        .register(responder.clone())?
        .register(helix)?
        .register(emote_map)?
        .register(github)?
        .register(spotify)?
        .register(SpotifyHistory::new(spotify_history_db))?
        .register(Aliases::new(aliases_db))?;

    let data = std::fs::read_to_string(config.paths.script("init"))?;
    let mut manifest = Manifest::initialize(&lua, &config.paths.scripts, &data)?;

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
                our_user.register(Globals::new(&lua))?;

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
