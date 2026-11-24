//! Measurements for world streaming (docs/design/data-and-streaming.md §4),
//! run by hand against a real `dyno server` process:
//!
//! ```sh
//! dyno server --port 7651 --world /tmp/m/world.dynw --trust-movement &
//! DYNO_MEASURE_ADDR=127.0.0.1:7651 DYNO_MEASURE_WORLD=/tmp/m/world.dynw \
//! DYNO_MEASURE_PID=<server pid> cargo test --release -p dyno-client \
//!     --test world_measure -- --ignored --nocapture
//! ```
//!
//! It reports the bytes and time of a fresh join until the world around the
//! spawn is there, the bytes per second one player receives while running
//! across the map with ten others wandering, by message kind, and the
//! memory of the server and of a joined client world.
//!
//! The players here place themselves by pose, which only a server started
//! with `--trust-movement` takes (others run movement from steps:
//! docs/design/netcode.md).

use dyno_protocol::{self as proto, Appearance, ClientMsg, NewCharacter, Pose, ServerMsg};
use dyno_world::World;
use std::collections::{BTreeMap, HashSet};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

type Ws = WebSocket<MaybeTlsStream<TcpStream>>;

/// Bytes and messages received, by message kind.
#[derive(Default, Clone)]
struct Tally {
    kinds: BTreeMap<String, (u64, u64)>,
}

impl Tally {
    fn add(&mut self, kind: &str, bytes: usize) {
        let e = self.kinds.entry(kind.to_string()).or_default();
        e.0 += bytes as u64;
        e.1 += 1;
    }
    fn total(&self) -> u64 {
        self.kinds.values().map(|v| v.0).sum()
    }
    fn minus(&self, before: &Tally) -> Tally {
        let mut out = Tally::default();
        for (k, v) in &self.kinds {
            let b = before.kinds.get(k).copied().unwrap_or_default();
            if v.0 > b.0 {
                out.kinds.insert(k.clone(), (v.0 - b.0, v.1 - b.1));
            }
        }
        out
    }
}

/// The variant name of a message ("Poses", "World", "Sections"…).
fn kind(m: &ServerMsg) -> String {
    let s = format!("{m:?}");
    s.split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or("")
        .to_string()
}

/// Sections of the world held, by section position (streamed worlds).
fn sections_in(m: &ServerMsg, held: &mut HashSet<[i32; 3]>) {
    match m {
        ServerMsg::Sections(list) => {
            for (p, _) in list {
                held.insert(*p);
            }
        }
        ServerMsg::SectionsGone(list) => {
            for p in list {
                held.remove(p);
            }
        }
        _ => {}
    }
}

struct Seen {
    tally: Tally,
    spawn: Option<[f32; 3]>,
    welcome_at: Option<Instant>,
    world_at: Option<Instant>,
    held: HashSet<[i32; 3]>,
}

struct Bot {
    seen: Arc<Mutex<Seen>>,
    pose: Arc<Mutex<Option<Pose>>>,
    stop: Arc<AtomicBool>,
    started: Instant,
}

fn connect(addr: &str) -> Ws {
    let (ws, _) = tungstenite::client::connect_with_config(
        format!("ws://{addr}"),
        Some(
            tungstenite::protocol::WebSocketConfig::default()
                .max_message_size(Some(proto::MAX_SERVER_MESSAGE))
                .max_frame_size(Some(proto::MAX_SERVER_MESSAGE)),
        ),
        3,
    )
    .expect("connects");
    if let MaybeTlsStream::Plain(s) = ws.get_ref() {
        s.set_read_timeout(Some(Duration::from_millis(5))).unwrap();
    }
    ws
}

/// A raw client: joins as a new character, counts what arrives, and sends
/// its pose 20 times a second (set `pose` to move it).
fn spawn_bot(addr: &str, name: &str, token: u64) -> Bot {
    let mut ws = connect(addr);
    let send = |ws: &mut Ws, m: &ClientMsg| {
        ws.send(Message::Binary(proto::encode(m).into())).unwrap();
    };
    send(
        &mut ws,
        &ClientMsg::Hello {
            protocol: proto::PROTOCOL_VERSION,
            version: env!("CARGO_PKG_VERSION").into(),
            name: name.into(),
        },
    );
    send(
        &mut ws,
        &ClientMsg::Session {
            token,
            observer: false,
            create: Some(NewCharacter {
                class: "warrior".into(),
                look: Appearance::default(),
            }),
        },
    );
    let started = Instant::now();
    let seen = Arc::new(Mutex::new(Seen {
        tally: Tally::default(),
        spawn: None,
        welcome_at: None,
        world_at: None,
        held: HashSet::new(),
    }));
    let pose: Arc<Mutex<Option<Pose>>> = Arc::new(Mutex::new(None));
    let stop = Arc::new(AtomicBool::new(false));
    let (s2, p2, st2) = (seen.clone(), pose.clone(), stop.clone());
    std::thread::spawn(move || {
        let mut last_pose = Instant::now();
        while !st2.load(Ordering::Relaxed) {
            match ws.read() {
                Ok(Message::Binary(b)) => {
                    let m: ServerMsg = proto::decode(&b).expect("a server message");
                    let k = kind(&m);
                    let mut s = s2.lock().unwrap();
                    s.tally.add(&k, b.len());
                    sections_in(&m, &mut s.held);
                    match &m {
                        ServerMsg::Welcome { spawn, .. } => {
                            s.spawn = Some(*spawn);
                            s.welcome_at = Some(Instant::now());
                        }
                        ServerMsg::Teleport { pos } => {
                            if let Some(p) = p2.lock().unwrap().as_mut() {
                                p.pos = *pos;
                            }
                        }
                        _ => {}
                    }
                    if k == "World" {
                        s.world_at = Some(Instant::now());
                    }
                }
                Ok(Message::Close(_)) => break,
                Ok(_) => {}
                Err(tungstenite::Error::Io(e))
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) => {}
                Err(_) => break,
            }
            if last_pose.elapsed() >= Duration::from_millis(50) {
                last_pose = Instant::now();
                if let Some(p) = *p2.lock().unwrap() {
                    let _ = ws.write(Message::Binary(proto::encode(&ClientMsg::Pose(p)).into()));
                    let _ = ws.flush();
                }
            }
        }
        let _ = ws.close(None);
        let _ = ws.flush();
    });
    Bot {
        seen,
        pose,
        stop,
        started,
    }
}

fn rss_kb(pid: &str) -> u64 {
    std::fs::read_to_string(format!("/proc/{pid}/status"))
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
        })
        .unwrap_or(0)
}

/// Feet height to stand at (x, z): on the ground, or swimming at the top of water.
fn feet(world: &World, x: f32, z: f32) -> f32 {
    let (xi, zi) = (x.floor() as i32, z.floor() as i32);
    let g = world.ground(xi, zi);
    let mut y = g + 1;
    while world.get(xi, y, zi) == dyno_world::WATER {
        y += 1;
    }
    y as f32
}

/// The sections a player at `spawn` needs before the first frame: every
/// section of the columns whose centre is within `r` metres.
fn near_sections(size: [u32; 3], spawn: [f32; 3], r: f32) -> Vec<[i32; 3]> {
    let d = [
        size[0] as i32 / 16,
        size[1].div_ceil(16) as i32,
        size[2] as i32 / 16,
    ];
    let mut out = Vec::new();
    for sx in 0..d[0] {
        for sz in 0..d[2] {
            let (cx, cz) = (sx as f32 * 16.0 + 8.0, sz as f32 * 16.0 + 8.0);
            if (cx - spawn[0]).hypot(cz - spawn[2]) <= r {
                for sy in 0..d[1] {
                    out.push([sx, sy, sz]);
                }
            }
        }
    }
    out
}

#[test]
#[ignore]
fn measure_world_streaming() {
    let Ok(addr) = std::env::var("DYNO_MEASURE_ADDR") else {
        eprintln!("set DYNO_MEASURE_ADDR (and DYNO_MEASURE_WORLD, DYNO_MEASURE_PID)");
        return;
    };
    let world = World::load(std::path::Path::new(
        &std::env::var("DYNO_MEASURE_WORLD").expect("DYNO_MEASURE_WORLD"),
    ))
    .expect("the server's world");
    let pid = std::env::var("DYNO_MEASURE_PID").unwrap_or_default();
    let tag = std::process::id() % 100_000;
    let rss0 = rss_kb(&pid);

    // 1. a fresh join: bytes and time until the world around the spawn is here
    let runner = spawn_bot(&addr, &format!("Run{tag}"), 9_000_000 + tag as u64);
    let until = Instant::now() + Duration::from_secs(60);
    let (spawn, ready_at, ready_bytes) = loop {
        assert!(Instant::now() < until, "the world arrives");
        std::thread::sleep(Duration::from_millis(2));
        let s = runner.seen.lock().unwrap();
        let Some(spawn) = s.spawn else { continue };
        let ready = s.world_at.is_some() || {
            let need = near_sections(world.size, spawn, 40.0);
            need.iter().all(|p| s.held.contains(p))
        };
        if ready {
            break (spawn, Instant::now(), s.tally.total());
        }
    };
    let welcome_at = runner.seen.lock().unwrap().welcome_at.unwrap();
    // then until nothing more streams for a second (the whole view)
    let mut last = (0u64, Instant::now());
    let full = loop {
        std::thread::sleep(Duration::from_millis(20));
        let t = runner.seen.lock().unwrap().tally.total();
        if t != last.0 {
            last = (t, Instant::now());
        } else if last.1.elapsed() > Duration::from_secs(1) {
            break (t, last.1);
        }
    };
    let join_tally = runner.seen.lock().unwrap().tally.clone();
    eprintln!(
        "JOIN: welcome after {} ms; world around the spawn after {} ms, {} bytes; \
         everything after {} ms, {} bytes; by kind {:?}",
        (welcome_at - runner.started).as_millis(),
        (ready_at - runner.started).as_millis(),
        ready_bytes,
        (full.1 - runner.started).as_millis(),
        full.0,
        join_tally.kinds
    );
    let held_join = runner.seen.lock().unwrap().held.len();

    // 2. ten others wander out in every direction; the runner crosses the map
    let mut others = Vec::new();
    for i in 0..10u32 {
        let b = spawn_bot(
            &addr,
            &format!("Wan{tag}x{i}"),
            9_100_000 + (tag * 16 + i) as u64,
        );
        others.push(b);
    }
    std::thread::sleep(Duration::from_secs(3));
    for b in &others {
        let s = b.seen.lock().unwrap().spawn.unwrap();
        *b.pose.lock().unwrap() = Some(Pose {
            pos: s,
            yaw: 0.0,
            pitch: 0.0,
        });
    }
    *runner.pose.lock().unwrap() = Some(Pose {
        pos: spawn,
        yaw: 0.0,
        pitch: 0.0,
    });
    std::thread::sleep(Duration::from_secs(2));
    let rss_joined = rss_kb(&pid);
    let before_run = runner.seen.lock().unwrap().tally.clone();
    let others_before: Vec<u64> = others
        .iter()
        .map(|b| b.seen.lock().unwrap().tally.total())
        .collect();
    let size = [world.size[0] as f32, world.size[2] as f32];
    // east to the edge, then west to the other edge
    let legs = [(spawn[0], size[0] - 24.0), (size[0] - 24.0, 24.0)];
    let speed = 7.0;
    let t0 = Instant::now();
    let mut per_second = Vec::new();
    let mut tick = Instant::now();
    let mut last_total = before_run.total();
    let mut ran = 0.0f32;
    for (from, to) in legs {
        let len = (to - from).abs();
        let dir = (to - from).signum();
        let leg0 = Instant::now();
        loop {
            let d = (leg0.elapsed().as_secs_f32() * speed).min(len);
            let x = from + dir * d;
            let z = spawn[2];
            *runner.pose.lock().unwrap() = Some(Pose {
                pos: [x, feet(&world, x, z), z],
                yaw: 0.0,
                pitch: 0.0,
            });
            // the others wander outwards, ten directions, at walking pace
            let t = t0.elapsed().as_secs_f32();
            for (i, b) in others.iter().enumerate() {
                let a = i as f32 / others.len() as f32 * std::f32::consts::TAU;
                let s = b.seen.lock().unwrap().spawn.unwrap();
                let r = (t * 4.0).min(170.0);
                let x = (s[0] + a.cos() * r).clamp(8.0, size[0] - 8.0);
                let z = (s[2] + a.sin() * r).clamp(8.0, size[1] - 8.0);
                *b.pose.lock().unwrap() = Some(Pose {
                    pos: [x, feet(&world, x, z), z],
                    yaw: a,
                    pitch: 0.0,
                });
            }
            if tick.elapsed() >= Duration::from_secs(1) {
                tick = Instant::now();
                let tot = runner.seen.lock().unwrap().tally.total();
                per_second.push(tot - last_total);
                last_total = tot;
            }
            if d >= len {
                ran += len;
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    let secs = t0.elapsed().as_secs_f64();
    let run = runner.seen.lock().unwrap().tally.minus(&before_run);
    let peak = per_second.iter().copied().max().unwrap_or(0);
    let others_rate: f64 = others
        .iter()
        .zip(&others_before)
        .map(|(b, t0)| (b.seen.lock().unwrap().tally.total() - t0) as f64 / secs)
        .sum::<f64>()
        / others.len() as f64;
    let mut kinds: Vec<(String, f64)> = run
        .kinds
        .iter()
        .map(|(k, v)| (k.clone(), v.0 as f64 / secs / 1024.0))
        .collect();
    kinds.sort_by(|a, b| b.1.total_cmp(&a.1));
    eprintln!(
        "RUN: {ran:.0} m in {secs:.1} s: runner {:.1} KB/s average, {:.1} KB/s peak second; \
         wanderers {:.1} KB/s average; runner by kind (KB/s): {:?}",
        run.total() as f64 / secs / 1024.0,
        peak as f64 / 1024.0,
        others_rate / 1024.0,
        kinds
            .iter()
            .map(|(k, v)| format!("{k} {v:.2}"))
            .collect::<Vec<_>>()
    );
    let held_end = runner.seen.lock().unwrap().held.len();
    let rss_end = rss_kb(&pid);
    eprintln!(
        "SERVER RSS: {} MB before, {} MB with 11 players, {} MB after the run; \
         runner held {} sections after the join, {} after the run",
        rss0 / 1024,
        rss_joined / 1024,
        rss_end / 1024,
        held_join,
        held_end
    );
    runner.stop.store(true, Ordering::Relaxed);
    for b in &others {
        b.stop.store(true, Ordering::Relaxed);
    }

    // 3. a joined client's world in memory (the real join path), once the
    //    whole view has streamed in. Before streaming, a client held the
    //    whole world (one byte a cell) and light (two bytes a cell).
    let mut j = dyno_client::net::connect(
        &addr,
        &format!("Mem{tag}"),
        9_200_000 + tag as u64,
        false,
        Duration::from_secs(30),
    )
    .expect("joins");
    let mut quiet = Instant::now();
    while quiet.elapsed() < Duration::from_secs(1) {
        if let Ok(dyno_client::net::NetEvent::Msg(m)) =
            j.conn.rx.recv_timeout(Duration::from_millis(20))
            && dyno_client::net::apply_world(&mut j.world, &m).is_some_and(|a| !a.is_empty())
        {
            quiet = Instant::now();
        }
    }
    let light = dyno_world::light::Light::new(&j.world);
    let (n, cells) = j.world.held_sections();
    let s = j.world.size;
    let whole = s[0] as usize * s[1] as usize * s[2] as usize;
    eprintln!(
        "CLIENT: holds {} of {} sections: world {:.1} MB, light {:.1} MB \
         (the whole world and its light would be {:.1} + {:.1} MB)",
        n,
        {
            let d = j.world.section_dims();
            d[0] * d[1] * d[2]
        },
        cells as f64 / 1048576.0,
        light.bytes() as f64 / 1048576.0,
        whole as f64 / 1048576.0,
        2.0 * whole as f64 / 1048576.0
    );
    drop((j, light));
    std::thread::sleep(Duration::from_millis(300));
}

/// A real client's join at a point (the spawn, or a far hub): how long until
/// it can play, how many bytes that took, and what the whole view and the
/// far view cost after.
///
/// ```sh
/// DYNO_MEASURE_ADDR=127.0.0.1:7661 DYNO_MEASURE_AT=x,y,z DYNO_CACHE_DIR=… \
///   cargo test --release -p dyno-client --test world_measure measure_join -- --ignored --nocapture
/// ```
#[test]
#[ignore]
fn measure_join() {
    let Ok(addr) = std::env::var("DYNO_MEASURE_ADDR") else {
        eprintln!("set DYNO_MEASURE_ADDR (and DYNO_MEASURE_AT=x,y,z for a point)");
        return;
    };
    let at: Option<[f32; 3]> = std::env::var("DYNO_MEASURE_AT").ok().and_then(|v| {
        let n: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        (n.len() == 3).then(|| [n[0], n[1], n[2]])
    });
    let tag = std::process::id() % 100_000;
    let t0 = Instant::now();
    let j = dyno_client::net::connect_with(
        &addr,
        &format!("Join{tag}"),
        dyno_client::net::JoinOptions {
            token: 9_300_000 + tag as u64,
            observer: true,
            view_at: at,
            ..Default::default()
        },
        Duration::from_secs(120),
    )
    .expect("joins");
    let play = t0.elapsed();
    let (before, pack) = (j.world_bytes, j.downloaded);
    let held0 = j.world.held_sections().0;
    // the rest of the view and the far view, until nothing arrives for a second
    let (mut sec_bytes, mut uniform, mut mixed, mut far_bytes, mut far_tiles) =
        (0u64, 0u32, 0u32, 0u64, 0u32);
    let mut quiet = Instant::now();
    let mut last = Instant::now();
    while quiet.elapsed() < Duration::from_secs(2) {
        if let Ok(dyno_client::net::NetEvent::Msg(m)) =
            j.conn.rx.recv_timeout(Duration::from_millis(20))
        {
            match m {
                ServerMsg::Sections(l) => {
                    for (_, d) in &l {
                        sec_bytes += d.len() as u64 + 12;
                        if d.len() <= 3 {
                            uniform += 1;
                        } else {
                            mixed += 1;
                        }
                    }
                    quiet = Instant::now();
                    last = Instant::now();
                }
                ServerMsg::Far(l) => {
                    for (_, d) in &l {
                        far_bytes += d.len() as u64 + 12;
                        far_tiles += 1;
                    }
                    quiet = Instant::now();
                    last = Instant::now();
                }
                _ => {}
            }
        }
    }
    eprintln!(
        "JOIN at {:?} (spawn {:?}, hearth {:?}): playable after {} ms ({} KB of sections, {} sections; pack {} KB); \
         the rest of the view {} KB ({} sections: {} all one block, {} mixed); \
         far view {} KB ({} tiles); everything after {} ms",
        at,
        j.spawn,
        j.hearth,
        play.as_millis(),
        before / 1024,
        held0,
        pack / 1024,
        sec_bytes / 1024,
        uniform + mixed,
        uniform,
        mixed,
        far_bytes / 1024,
        far_tiles,
        (last - t0).as_millis()
    );
}
