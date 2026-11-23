//! The `dyno` executable: client, server and updater in one binary.

use std::time::Duration;

const HELP: &str = "\
dyno - live multiplayer block world (bootstrap build)

USAGE:
  dyno                         Open the game (connect screen)
  dyno connect <address>       Open the game with the server address filled in
  dyno connect <address>       Open the game with the server address filled in
  dyno server [--port 7400] [--bind 0.0.0.0] [--world PATH] [--auto-update]
                               [--creative] [--content FILE.json] [--op NAME]...
                               [--webhook DISCORD_URL] (or DYNO_WEBHOOK)
                               [--shard-cap 40] [--area-cap 16] [--max-shards 16]
                               Run a dedicated game server
  dyno say <address> <text>    Join, send chat or a /command, print replies, leave
  dyno shot <address> [--pos x,y,z] [--look yaw,pitch in degrees] [--time 0-1] [--size WxH] [--out f.ppm]
                               [--ui hud|pack|dig|dead|talk|log|char|bags|spells|skills] [--as NAME]
                               Join, render one frame off-screen to an image, leave
  dyno shot --anim-sheet <out.ppm|out.png>
                               Render animation contact sheets (no server needed)
  dyno bot <address> [name] [seconds]
                               Join as a bot that walks in a circle (demos, tests)
  dyno fill <address> x,y,z x,y,z <block>
                               Fill a box with a block (Air clears it)
  dyno update                  Check for a new release and install it
  dyno version                 Print version, target and protocol
  dyno verify-manifest <manifest.json> <manifest.json.sig>
                               Verify a release manifest with the built-in key

Set DYNO_NO_UPDATE=1 to turn off automatic updates.
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("");
    match cmd {
        "version" | "--version" | "-V" => {
            println!("dyno {} ({})", dyno_update::VERSION, dyno_update::TARGET);
        }
        "help" | "--help" | "-h" => print!("{HELP}"),
        "update" => match dyno_update::check_and_update(true, Duration::from_secs(30)) {
            Ok(dyno_update::Outcome::Updated { from, to }) => println!("Updated {from} -> {to}"),
            Ok(other) => println!("{other:?}"),
            Err(e) => {
                eprintln!("update failed: {e}");
                std::process::exit(1);
            }
        },
        "server" => run_server(&args[1..]),
        "ping" => {
            let Some(addr) = args.get(1) else {
                eprintln!("usage: dyno ping <address> [name]");
                std::process::exit(2);
            };
            let name = args.get(2).map(String::as_str).unwrap_or("ping");
            let t = std::time::Instant::now();
            match dyno_client::net::connect(addr, name, 0, true, Duration::from_secs(10)) {
                Ok(j) => {
                    println!(
                        "OK {} in {} ms: server dyno {}, you are #{}, world {:?}, {} other players. MOTD: {}",
                        j.url,
                        t.elapsed().as_millis(),
                        j.server_version,
                        j.your_id,
                        j.world.size,
                        j.players.len(),
                        j.motd
                    );
                }
                Err(e) => {
                    eprintln!("FAILED: {e}");
                    std::process::exit(1);
                }
            }
        }
        "say" => {
            let (Some(addr), Some(text)) = (args.get(1), args.get(2)) else {
                eprintln!("usage: dyno say <address> <text or /command> [name]");
                std::process::exit(2);
            };
            let name = args.get(3).map(String::as_str).unwrap_or("console");
            match dyno_client::net::connect(
                addr,
                name,
                dyno_client::settings::tool_token(name),
                false,
                Duration::from_secs(10),
            ) {
                Ok(j) => {
                    let _ = j
                        .conn
                        .tx
                        .send(dyno_protocol::ClientMsg::Chat { text: text.clone() });
                    let until = std::time::Instant::now() + Duration::from_millis(800);
                    while std::time::Instant::now() < until {
                        if let Ok(dyno_client::net::NetEvent::Msg(
                            dyno_protocol::ServerMsg::System { text },
                        )) = j.conn.rx.recv_timeout(Duration::from_millis(100))
                        {
                            println!("{text}");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("FAILED: {e}");
                    std::process::exit(1);
                }
            }
        }
        "shot" if args.get(1).is_some_and(|a| a == "--anim-sheet") => {
            // no server needed: every figure in every key pose
            let out = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| "anim-sheet.ppm".into());
            match dyno_client::sheet::anim_sheet(std::path::Path::new(&out)) {
                Ok(msg) => println!("{msg}"),
                Err(e) => {
                    eprintln!("FAILED: {e}");
                    std::process::exit(1);
                }
            }
        }
        "shot" => {
            let Some(addr) = args.get(1) else {
                eprintln!(
                    "usage: dyno shot <address> [--pos x,y,z] [--look yaw,pitch] [--time 0-1] [--size WxH] [--out file.ppm]"
                );
                std::process::exit(2);
            };
            let mut o = dyno_client::shot::ShotOptions {
                addr: addr.clone(),
                name: "camera".into(),
                pos: None,
                yaw: 0.0,
                pitch: -0.15,
                time: None,
                width: 1280,
                height: 720,
                out: "shot.ppm".into(),
                ui: "none".into(),
                player: None,
                zoom: None,
                front: None,
            };
            let nums = |v: &str| -> Vec<f32> {
                v.split(',').filter_map(|x| x.trim().parse().ok()).collect()
            };
            let mut i = 2;
            while i < args.len() {
                let v = args.get(i + 1).cloned().unwrap_or_default();
                match args[i].as_str() {
                    "--pos" => o.pos = nums(&v).try_into().ok(),
                    "--front" => o.front = Some(v.clone()),
                    "--look" => {
                        let n = nums(&v);
                        // degrees: yaw 0 faces north (-z), 90 west; pitch up is positive
                        o.yaw = n.first().copied().unwrap_or(0.0).to_radians();
                        o.pitch = n
                            .get(1)
                            .copied()
                            .unwrap_or(0.0)
                            .to_radians()
                            .clamp(-1.55, 1.55);
                    }
                    "--time" => o.time = v.parse().ok(),
                    "--size" => {
                        let mut it = v.split('x').filter_map(|x| x.parse().ok());
                        o.width = it.next().unwrap_or(1280);
                        o.height = it.next().unwrap_or(720);
                    }
                    "--out" => o.out = v.into(),
                    "--name" => o.name = v,
                    "--ui" => o.ui = v,
                    "--as" => o.player = Some(v),
                    // camera distance behind the player; 0 is first person
                    "--zoom" => o.zoom = v.parse().ok(),
                    other => fail(&format!("unknown shot option {other}")),
                }
                i += 2;
            }
            match dyno_client::shot::shot(o) {
                Ok(msg) => println!("{msg}"),
                Err(e) => {
                    eprintln!("FAILED: {e}");
                    std::process::exit(1);
                }
            }
        }
        "bot" => {
            // A simple bot: walks in a circle near spawn and says hello. For demos and tests.
            let Some(addr) = args.get(1) else {
                eprintln!("usage: dyno bot <address> [name] [seconds]");
                std::process::exit(2);
            };
            let name = args.get(2).cloned().unwrap_or_else(|| "bot".into());
            let secs: u64 = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(60);
            let j = dyno_client::net::connect(
                addr,
                &name,
                dyno_client::settings::tool_token(&name),
                false,
                Duration::from_secs(10),
            )
            .unwrap_or_else(|e| fail(&e));
            let _ = j.conn.tx.send(dyno_protocol::ClientMsg::Chat {
                text: "Hello! I am a test bot.".into(),
            });
            let start = std::time::Instant::now();
            let c = j.spawn;
            while start.elapsed() < Duration::from_secs(secs) {
                let t = start.elapsed().as_secs_f32() * 0.5;
                let (x, z) = (c[0] + 4.0 * t.cos(), c[2] + 4.0 * t.sin());
                let y = j.world.surface(x.floor() as i32, z.floor() as i32) as f32 + 1.0;
                let pose = dyno_protocol::Pose {
                    pos: [x, y, z],
                    yaw: -t,
                    pitch: 0.0,
                };
                if j.conn
                    .tx
                    .send(dyno_protocol::ClientMsg::Pose(pose))
                    .is_err()
                {
                    break;
                }
                while j.conn.rx.try_recv().is_ok() {}
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        "fill" => {
            // dyno fill <address> x,y,z x,y,z <block name or id>
            let (Some(addr), Some(a), Some(b), Some(blk)) =
                (args.get(1), args.get(2), args.get(3), args.get(4))
            else {
                eprintln!("usage: dyno fill <address> x,y,z x,y,z <block name or id>");
                std::process::exit(2);
            };
            let p3 = |v: &str| -> [i32; 3] {
                let n: Vec<i32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                n.try_into().unwrap_or_else(|_| fail("corners are x,y,z"))
            };
            let block = blk.parse::<u8>().ok().or_else(|| {
                dyno_world::BLOCKS
                    .iter()
                    .position(|d| d.name.eq_ignore_ascii_case(blk))
                    .map(|i| i as u8)
            });
            let Some(block) = block else {
                fail("unknown block")
            };
            let (a, b) = (p3(a), p3(b));
            let j = dyno_client::net::connect(
                addr,
                "builder",
                dyno_client::settings::tool_token("builder"),
                false,
                Duration::from_secs(10),
            )
            .unwrap_or_else(|e| fail(&e));
            // stand next to the box so the server accepts the reach check
            let c = [
                (a[0] + b[0]) as f32 * 0.5,
                (a[1].max(b[1]) + 2) as f32,
                (a[2] + b[2]) as f32 * 0.5,
            ];
            let _ = j
                .conn
                .tx
                .send(dyno_protocol::ClientMsg::Pose(dyno_protocol::Pose {
                    pos: c,
                    yaw: 0.0,
                    pitch: 0.0,
                }));
            std::thread::sleep(Duration::from_millis(150));
            let _ = j
                .conn
                .tx
                .send(dyno_protocol::ClientMsg::Fill { a, b, block });
            let until = std::time::Instant::now() + Duration::from_millis(1500);
            while std::time::Instant::now() < until {
                match j.conn.rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(dyno_client::net::NetEvent::Msg(dyno_protocol::ServerMsg::Fill {
                        a,
                        b,
                        block,
                    })) => {
                        println!(
                            "filled {a:?} to {b:?} with {}",
                            dyno_world::block(block).name
                        );
                        break;
                    }
                    Ok(dyno_client::net::NetEvent::Msg(dyno_protocol::ServerMsg::System {
                        text,
                    })) => println!("{text}"),
                    _ => {}
                }
            }
        }
        "verify-manifest" => {
            let (Some(m), Some(s)) = (args.get(1), args.get(2)) else {
                eprintln!("usage: dyno verify-manifest <manifest.json> <manifest.json.sig>");
                std::process::exit(2);
            };
            let mb = std::fs::read(m).expect("read manifest");
            let sb = std::fs::read(s).expect("read signature");
            match dyno_update::verify_manifest(&mb, &sb) {
                Ok(man) => println!(
                    "OK: version {} with {} assets",
                    man.version,
                    man.assets.len()
                ),
                Err(e) => {
                    eprintln!("FAILED: {e}");
                    std::process::exit(1);
                }
            }
        }
        "" | "connect" | "play" => {
            let note = auto_update();
            let prefill = if cmd == "connect" {
                args.get(1).cloned()
            } else {
                None
            };
            if let Err(e) = dyno_client::run(prefill, note) {
                eprintln!("dyno could not start the game: {e}");
                std::process::exit(1);
            }
        }
        other => {
            eprintln!("unknown command '{other}'\n");
            print!("{HELP}");
            std::process::exit(2);
        }
    }
}

/// Check for updates at start. Every failure is soft. Returns a short status for the UI.
fn auto_update() -> String {
    if std::env::var_os(dyno_update::JUST_UPDATED_ENV).is_some() {
        return "just updated".into();
    }
    match dyno_update::check_and_update(false, Duration::from_secs(8)) {
        Ok(dyno_update::Outcome::Updated { from, to }) => {
            println!("Updated {from} -> {to}. Restarting...");
            dyno_update::restart();
        }
        Ok(dyno_update::Outcome::UpToDate(_)) => "up to date".into(),
        Ok(dyno_update::Outcome::Skipped(why)) => format!("updates off: {why}"),
        Err(e) => {
            eprintln!(
                "update check failed, continuing with {}: {e}",
                dyno_update::VERSION
            );
            "update check failed".into()
        }
    }
}

fn run_server(args: &[String]) {
    let mut cfg = dyno_server::Config::default();
    let mut port = dyno_protocol::DEFAULT_PORT;
    let mut bind = "0.0.0.0".to_string();
    let mut auto_update = false;
    let mut i = 0;
    while i < args.len() {
        let val = || args.get(i + 1).cloned().unwrap_or_default();
        match args[i].as_str() {
            "--port" => {
                port = val()
                    .parse()
                    .unwrap_or_else(|_| fail("--port needs a number"));
                i += 1;
            }
            "--bind" => {
                bind = val();
                i += 1;
            }
            "--world" => {
                cfg.world_path = val().into();
                i += 1;
            }
            "--motd" => {
                cfg.motd = val();
                i += 1;
            }
            "--auto-update" => auto_update = true,
            "--creative" => cfg.default_mode = dyno_protocol::Mode::Creative,
            "--content" => {
                cfg.content_path = Some(val().into());
                i += 1;
            }
            "--op" => {
                cfg.ops.push(val());
                i += 1;
            }
            "--webhook" => {
                cfg.webhook = Some(val());
                i += 1;
            }
            "--shard-cap" | "--area-cap" | "--max-shards" => {
                let n: usize = val()
                    .parse()
                    .unwrap_or_else(|_| fail(&format!("{} needs a number", args[i])));
                match args[i].as_str() {
                    "--shard-cap" => cfg.shard_cap = n,
                    "--area-cap" => cfg.area_cap = n,
                    _ => cfg.max_shards = n,
                }
                i += 1;
            }
            other => fail(&format!("unknown server option {other}")),
        }
        i += 1;
    }
    if cfg.webhook.is_none() {
        cfg.webhook = std::env::var("DYNO_WEBHOOK").ok().filter(|u| !u.is_empty());
    }
    cfg.bind = format!("{bind}:{port}")
        .parse()
        .or_else(|_| format!("[{bind}]:{port}").parse())
        .unwrap_or_else(|_| fail("bad --bind address"));
    if auto_update {
        auto_update_now();
    }
    let _ = dyno_update::JUST_UPDATED_ENV;
    let handle =
        dyno_server::start(cfg).unwrap_or_else(|e| fail(&format!("cannot start server: {e}")));
    println!("Players can join with the address <this machine's IP>:{port}. Press Ctrl+C to stop.");
    let mut last_check = std::time::Instant::now();
    loop {
        std::thread::sleep(Duration::from_secs(5));
        if auto_update && last_check.elapsed() > Duration::from_secs(5 * 60) {
            last_check = std::time::Instant::now();
            if let Ok(dyno_update::Outcome::Updated { from, to }) =
                dyno_update::check_and_update(false, Duration::from_secs(30))
            {
                handle.server.announce(&format!(
                    "Server updated {from} -> {to}. Restarting in 10 seconds."
                ));
                std::thread::sleep(Duration::from_secs(10));
                handle.stop();
                dyno_update::restart();
            }
        }
    }
}

fn auto_update_now() {
    if let Ok(dyno_update::Outcome::Updated { from, to }) =
        dyno_update::check_and_update(false, Duration::from_secs(15))
    {
        println!("Updated {from} -> {to}. Restarting...");
        dyno_update::restart();
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(2)
}
