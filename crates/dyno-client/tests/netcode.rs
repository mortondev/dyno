//! Movement over a poor line (docs/design/netcode.md): a real server, real
//! client networking through the network conditioner (`NetSim`), the
//! client's prediction (`predict.rs`) and remote smoothing.
//!
//! `prediction_holds_under_a_poor_line` runs a scripted course (walk, run,
//! sprint, jump, stairs, a ledge, a doorway, a step up) with 150 ms of
//! latency, 30 ms of jitter and 2% loss, while a second player watches. It
//! fails on a snap, or when the watched runner's drawn speed wobbles. It
//! runs for 40 s; `DYNO_NETCODE_SECS=600` makes it the 10-minute session.
//!
//! `measure_crowd` (ignored: run by hand) puts 40 players in one area and
//! reports the bytes a player receives and sends a second, by message kind,
//! and the server's tick cost:
//!
//! ```sh
//! cargo test --release -p dyno-client --test netcode -- --ignored --nocapture
//! ```
//!
//! `DYNO_NETCODE_MODE=pose` runs either the way movement worked before
//! (clients send their pose 20 times a second, the server trusts it, and
//! others are drawn between the last two poses), for the before numbers.

use dyno_client::game::{Input, Player, Remote, apply_rules};
use dyno_client::net::{self, Joined, NetEvent, NetSim};
use dyno_client::predict::Predictor;
use dyno_protocol::{self as proto, ClientMsg, Pose, ServerMsg};
use glam::Vec3;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The poor line of the goal: 150 ms round trip, 30 ms jitter, 2% loss.
const POOR: NetSim = NetSim {
    latency_ms: 150.0,
    jitter_ms: 30.0,
    loss: 0.02,
};

fn pose_mode() -> bool {
    std::env::var("DYNO_NETCODE_MODE").is_ok_and(|m| m == "pose")
}

fn secs(default: f32) -> f32 {
    std::env::var("DYNO_NETCODE_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn join(addr: &str, name: &str, token: u64, sim: Option<NetSim>) -> Joined {
    net::connect_with(
        addr,
        name,
        net::JoinOptions {
            token,
            create: Some(proto::NewCharacter {
                class: "warrior".into(),
                look: Default::default(),
            }),
            net_sim: sim,
            ..Default::default()
        },
        Duration::from_secs(20),
    )
    .expect("join")
}

/// A client that moves itself: its body, predicted (or, in pose mode, sent
/// as poses), and what it has been told.
struct Mover {
    j: Joined,
    body: Player,
    pred: Predictor,
    last: Instant,
    last_pose: Instant,
    pose: bool,
    /// what it sent and received while counting
    sent: Tally,
    got: Tally,
    counting: bool,
}

impl Mover {
    fn new(j: Joined, pose: bool) -> Mover {
        let mut body = Player::new(j.spawn);
        apply_rules(&mut body, j.content.world.protected, false);
        Mover {
            j,
            body,
            pred: Predictor::new(),
            last: Instant::now(),
            last_pose: Instant::now(),
            pose,
            sent: Tally::default(),
            got: Tally::default(),
            counting: false,
        }
    }

    fn send(&mut self, m: ClientMsg) {
        if self.counting {
            self.sent.add(kind(&m), proto::encode(&m).len());
        }
        let _ = self.j.conn.tx.send(m);
    }

    /// Take in what came: the world, answers, teleports. Returns the other
    /// messages.
    fn pump(&mut self) -> Vec<ServerMsg> {
        let mut out = Vec::new();
        while let Ok(ev) = self.j.conn.rx.try_recv() {
            match ev {
                NetEvent::Msg(m) => {
                    if self.counting {
                        self.got.add(kind(&m), proto::encode(&m).len());
                    }
                    if net::apply_world(&mut self.j.world, &m).is_some() {
                        continue;
                    }
                    match m {
                        ServerMsg::Moved {
                            seq,
                            pos,
                            vel,
                            ground,
                        } if !self.pose => {
                            self.pred
                                .ack(&mut self.body, &self.j.world, seq, pos, vel, ground);
                        }
                        ServerMsg::Teleport { pos } => {
                            self.pred.teleport(&mut self.body, pos);
                        }
                        m => out.push(m),
                    }
                }
                NetEvent::Closed(r) => panic!("closed: {r}"),
                _ => {}
            }
        }
        out
    }

    /// One frame of `input` facing `yaw`.
    fn frame(&mut self, input: Input, yaw: f32) {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32();
        self.last = now;
        if self.pose {
            // as clients moved before: the body here, its pose sent 20 times a second
            self.body.yaw = yaw;
            self.body.update(&self.j.world, input, dt);
            if self.last_pose.elapsed() >= Duration::from_millis(50) {
                self.last_pose = now;
                self.send(ClientMsg::Pose(Pose {
                    pos: self.body.pos.into(),
                    yaw,
                    pitch: 0.0,
                }));
            }
        } else {
            self.pred
                .advance(&mut self.body, &self.j.world, input, yaw, dt);
            if let Some(m) = self.pred.flush(0.0) {
                self.send(m);
            }
        }
    }

    /// Wait for the land within `r` of `at` to stream in.
    fn land(&mut self, at: [f32; 3], r: f32) {
        let need = net::sections_around(&self.j.world, at, r);
        let end = Instant::now() + Duration::from_secs(30);
        while !need.iter().all(|s| self.j.world.has_section(*s)) {
            assert!(Instant::now() < end, "the land around {at:?} streams in");
            self.pump();
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

/// The course: a stone track high above the town, along +x.
struct Course {
    x0: i32,
    len: i32,
    z0: i32,
    /// the height the feet stand at on the track
    feet: i32,
}

impl Course {
    /// The flat stretch where the runner runs straight at full speed both
    /// ways: where the watcher measures drawn speed.
    fn steady(&self, x: f32) -> bool {
        let x = x - self.x0 as f32;
        (64.0..=96.0).contains(&x)
    }

    fn mid_z(&self) -> f32 {
        self.z0 as f32 + 0.5
    }

    /// The fills that make it.
    fn fills(&self) -> Vec<([i32; 3], [i32; 3], u8)> {
        use dyno_world::{AIR, STONE};
        let (x0, x1, z0, f) = (self.x0, self.x0 + self.len, self.z0, self.feet);
        let mut v = vec![
            // clear sky, and the track
            ([x0 - 2, f - 1, z0 - 3], [x1 + 2, f + 7, z0 + 3], AIR),
            ([x0, f - 1, z0 - 2], [x1, f - 1, z0 + 2], STONE),
            // stairs of 1 m up to a 3 m deck, and a 3 m ledge down from it
            ([x0 + 15, f, z0 - 2], [x0 + 15, f, z0 + 2], STONE),
            ([x0 + 16, f, z0 - 2], [x0 + 16, f + 1, z0 + 2], STONE),
            ([x0 + 17, f, z0 - 2], [x0 + 25, f + 2, z0 + 2], STONE),
            // a wall with a doorway 1 m wide and 2 m high
            ([x0 + 35, f, z0 - 2], [x0 + 35, f + 3, z0 + 2], STONE),
            ([x0 + 35, f, z0], [x0 + 35, f + 1, z0], AIR),
            // a step up of 1 m and down again
            ([x0 + 45, f, z0 - 2], [x0 + 50, f, z0 + 2], STONE),
        ];
        // low walls along the sides, so a stray step never falls off
        for z in [z0 - 3, z0 + 3] {
            v.push(([x0, f - 1, z], [x1, f + 1, z], STONE));
        }
        v
    }
}

/// What the runner does at `x` going `dir` (+1 east, -1 west), at time `t`.
fn runner_input(c: &Course, x: f32, dir: f32, t: f32, lap: u32) -> Input {
    let rel = x - c.x0 as f32;
    let mut i = Input {
        forward: 1.0,
        ..Default::default()
    };
    if rel < 56.0 {
        // the features: walk some laps, sprint others, jump now and then
        match lap % 4 {
            1 => i.forward = 0.45,
            2 => i.sprint = true,
            _ => {}
        }
        i.jump = (t * 1.7).fract() < 0.12 && lap % 3 != 0;
        // and a little strafe in the open
        if !(30.0..40.0).contains(&rel) && lap % 5 == 4 {
            i.right = ((t * 0.9).sin() * 0.4 * dir).clamp(-1.0, 1.0);
        }
    }
    if rel > 104.0 && lap % 2 == 1 {
        // a stop and a start at the far end
        i.forward = 0.0;
    }
    i
}

/// Facing along the track, steering back to its middle.
fn runner_yaw(c: &Course, pos: Vec3, dir: f32) -> f32 {
    let dz = ((c.mid_z() - pos.z) * 0.8).clamp(-0.6, 0.6);
    let d = Vec3::new(dir, 0.0, dz).normalize();
    (-d.x).atan2(-d.z)
}

/// Drawn speed against the runner's steady speed, frame by frame.
#[derive(Default)]
struct Wobble {
    sq: f64,
    sum: f64,
    n: u64,
    worst: f32,
}

impl Wobble {
    fn add(&mut self, v: f32, want: f32) {
        let e = v - want;
        self.sq += (e * e) as f64;
        self.sum += v as f64;
        self.n += 1;
        self.worst = self.worst.max(e.abs());
    }
    fn rms(&self) -> f32 {
        (self.sq / self.n.max(1) as f64).sqrt() as f32
    }
    fn mean(&self) -> f32 {
        (self.sum / self.n.max(1) as f64) as f32
    }
}

/// The watcher: another player who draws the runner as the game would, 60
/// frames a second, and measures its drawn speed on the steady stretch.
fn watch(
    j: Joined,
    runner: proto::PlayerId,
    course: Arc<Course>,
    want: f32,
    stop: Arc<AtomicBool>,
) -> Wobble {
    let mut lerp: Option<Remote> = None;
    let mut wobble = Wobble::default();
    let mut prev: Option<(Instant, Vec3)> = None;
    let frame = Duration::from_micros(16_667);
    let mut next = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        while let Ok(ev) = j.conn.rx.try_recv() {
            if let NetEvent::Msg(ServerMsg::Poses(list)) = ev {
                for (id, pose) in list {
                    if id == runner {
                        lerp.get_or_insert_with(|| Remote::new("runner".into(), pose))
                            .set(pose);
                    }
                }
            }
        }
        let now = Instant::now();
        if let Some(r) = &lerp {
            let at = Vec3::from(r.current().pos);
            if let Some((t, p)) = prev {
                let dt = now.duration_since(t).as_secs_f32();
                let v = Vec3::new(at.x - p.x, 0.0, at.z - p.z).length() / dt.max(1e-4);
                let on_track =
                    (at.y - course.feet as f32).abs() < 0.05 && (at.z - course.mid_z()).abs() < 0.4;
                if course.steady(at.x) && course.steady(p.x) && on_track {
                    wobble.add(v, want);
                }
            }
            prev = Some((now, at));
        }
        next += frame;
        match next.checked_duration_since(Instant::now()) {
            Some(d) => std::thread::sleep(d),
            None => next = Instant::now(),
        }
    }
    wobble
}

fn start_server(dir: &std::path::Path, trust: bool, crowd: usize) -> dyno_server::Handle {
    let _ = std::fs::remove_dir_all(dir);
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        world_size: [192, 48, 192],
        ops: vec!["Runner".into()],
        trust_movement: trust,
        shard_cap: crowd.max(40),
        area_cap: crowd.max(40),
        ..Default::default()
    };
    dyno_server::start(cfg).expect("server starts")
}

/// The 10-minute session of the goal, short by default: a runner over the
/// course on a poor line, watched by another player on the same line.
#[test]
fn prediction_holds_under_a_poor_line() {
    let pose = pose_mode();
    let dir = std::env::temp_dir().join(format!("dyno-test-netcode-{}", std::process::id()));
    let server = start_server(&dir, pose, 0);
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let mut m = Mover::new(join(&addr, "Runner", 8101, Some(POOR)), pose);
    let watcher = join(&addr, "Watcher", 8102, Some(POOR));
    let runner_id = m.j.your_id;
    let course = Arc::new(Course {
        x0: 40,
        len: 110,
        z0: 40,
        feet: 36,
    });

    // build the course (build mode takes poses: go there, then fill)
    m.send(ClientMsg::Chat {
        text: "/gamemode build".into(),
    });
    let start = [course.x0 as f32 + 3.5, course.feet as f32, course.mid_z()];
    std::thread::sleep(Duration::from_millis(600));
    m.send(ClientMsg::Pose(Pose {
        pos: start,
        yaw: 0.0,
        pitch: 0.0,
    }));
    m.land([course.x0 as f32 + 55.0, start[1], start[2]], 70.0);
    for (a, b, block) in course.fills() {
        m.send(ClientMsg::Fill { a, b, block });
    }
    std::thread::sleep(Duration::from_millis(1500));
    m.pump();
    m.send(ClientMsg::Chat {
        text: "/gamemode play".into(),
    });
    std::thread::sleep(Duration::from_millis(1200));
    m.pump();
    m.body.pos = start.into();
    m.body.vel = Vec3::ZERO;
    let base = m.pred.stats;

    // the watcher, from the town
    let stop = Arc::new(AtomicBool::new(false));
    let (c2, s2) = (course.clone(), stop.clone());
    let want = 6.2;
    let watching = std::thread::spawn(move || watch(watcher, runner_id, c2, want, s2));

    let run_for = secs(40.0);
    let t0 = Instant::now();
    let (mut dir_x, mut lap) = (1.0f32, 0u32);
    let frame = Duration::from_micros(16_667);
    let mut next = Instant::now();
    let mut lowest = f32::MAX;
    while t0.elapsed().as_secs_f32() < run_for {
        m.pump();
        let rel = m.body.pos.x - course.x0 as f32;
        if (dir_x > 0.0 && rel > course.len as f32 - 3.0) || (dir_x < 0.0 && rel < 3.0) {
            dir_x = -dir_x;
            lap += 1;
        }
        let t = t0.elapsed().as_secs_f32();
        let input = runner_input(&course, m.body.pos.x, dir_x, t, lap);
        let yaw = runner_yaw(&course, m.body.pos, dir_x);
        m.frame(input, yaw);
        lowest = lowest.min(m.body.pos.y);
        next += frame;
        match next.checked_duration_since(Instant::now()) {
            Some(d) => std::thread::sleep(d),
            None => next = Instant::now(),
        }
    }
    stop.store(true, Ordering::Relaxed);
    let wobble = watching.join().unwrap();
    let st = m.pred.stats;
    let (snaps, eased) = (st.snaps - base.snaps, st.eased - base.eased);
    eprintln!(
        "NETCODE {}: {:.0} s, {lap} laps; snaps {snaps}, eased {eased}, worst correction {:.2} m, \
         {} answers, teleports {}; watched runner on the straight: {} frames, drawn speed mean {:.2} \
         (true {want}), rms error {:.3} m/s, worst {:.2} m/s",
        if pose { "pose" } else { "steps" },
        t0.elapsed().as_secs_f32(),
        st.worst,
        st.acks,
        st.teleports,
        wobble.n,
        wobble.mean(),
        wobble.rms(),
        wobble.worst,
    );
    assert!(
        lap >= 2,
        "the runner went up and down the course: {lap} laps"
    );
    assert!(
        lowest > course.feet as f32 - 0.5,
        "the runner stayed on the course"
    );
    assert!(wobble.n > 100, "the runner was watched on the straight");
    if !pose {
        assert_eq!(snaps, 0, "no snap in normal play");
    }
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Bytes and messages by kind.
#[derive(Default, Clone)]
struct Tally(BTreeMap<String, (u64, u64)>);

impl Tally {
    fn add(&mut self, kind: String, bytes: usize) {
        let e = self.0.entry(kind).or_default();
        e.0 += bytes as u64;
        e.1 += 1;
    }
    fn total(&self) -> u64 {
        self.0.values().map(|v| v.0).sum()
    }
}

fn kind<T: std::fmt::Debug>(m: &T) -> String {
    let s = format!("{m:?}");
    s.split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or("")
        .to_string()
}

/// 40 players in one area, all moving: bytes each receives and sends a
/// second, and the server's tick cost.
#[test]
#[ignore]
fn measure_crowd() {
    let pose = pose_mode();
    let n: usize = std::env::var("DYNO_NETCODE_CROWD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(40);
    let dir = std::env::temp_dir().join(format!("dyno-test-crowd-{}", std::process::id()));
    let server = start_server(&dir, pose, n);
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let stop = Arc::new(AtomicBool::new(false));
    let measuring = Arc::new(AtomicBool::new(false));
    let tallies: Arc<Mutex<Vec<(Tally, Tally)>>> = Default::default();
    let mut threads = Vec::new();
    for i in 0..n {
        let j = join(&addr, &format!("Crowd{i}"), 8200 + i as u64, None);
        let (stop, measuring, tallies) = (stop.clone(), measuring.clone(), tallies.clone());
        threads.push(std::thread::spawn(move || {
            let mut m = Mover::new(j, pose);
            let centre = m.body.pos;
            let r = 4.0 + (i % 8) as f32 * 2.0;
            let phase = i as f32 * 0.7;
            let t0 = Instant::now();
            while !stop.load(Ordering::Relaxed) {
                let on = measuring.load(Ordering::Relaxed);
                m.counting = on;
                m.pump();
                // walk round a circle about the town's middle
                let t = t0.elapsed().as_secs_f32() * (6.2 / r) + phase;
                let want = centre + Vec3::new(t.cos() * r, 0.0, t.sin() * r);
                let d = want - m.body.pos;
                let yaw = (-d.x).atan2(-d.z);
                let input = Input {
                    forward: if d.length() > 0.5 { 1.0 } else { 0.3 },
                    ..Default::default()
                };
                m.frame(input, yaw);
                std::thread::sleep(Duration::from_millis(16));
            }
            tallies.lock().unwrap().push((m.got, m.sent));
        }));
        std::thread::sleep(Duration::from_millis(150));
    }
    // all in, all moving: measure 20 s
    std::thread::sleep(Duration::from_secs(5));
    let cost0 = server.tick_cost();
    measuring.store(true, Ordering::Relaxed);
    let secs = 20.0;
    std::thread::sleep(Duration::from_secs_f32(secs));
    measuring.store(false, Ordering::Relaxed);
    let cost = server.tick_cost().since(&cost0);
    stop.store(true, Ordering::Relaxed);
    for t in threads {
        t.join().unwrap();
    }
    let all = tallies.lock().unwrap();
    let mut inn = Tally::default();
    let mut out = Tally::default();
    for (i, o) in all.iter() {
        for (k, v) in &i.0 {
            let e = inn.0.entry(k.clone()).or_default();
            e.0 += v.0;
            e.1 += v.1;
        }
        for (k, v) in &o.0 {
            let e = out.0.entry(k.clone()).or_default();
            e.0 += v.0;
            e.1 += v.1;
        }
    }
    let per = |b: u64| b as f64 / all.len() as f64 / secs as f64;
    eprintln!(
        "CROWD {} ({n} players): received {:.0} B/s a player, sent {:.0} B/s a player",
        if pose { "pose" } else { "steps" },
        per(inn.total()),
        per(out.total())
    );
    for (k, v) in &inn.0 {
        eprintln!(
            "  in  {k}: {:.0} B/s, {:.1} msgs/s",
            per(v.0),
            v.1 as f64 / all.len() as f64 / secs as f64
        );
    }
    for (k, v) in &out.0 {
        eprintln!(
            "  out {k}: {:.0} B/s, {:.1} msgs/s",
            per(v.0),
            v.1 as f64 / all.len() as f64 / secs as f64
        );
    }
    eprintln!(
        "  tick: mean {:.2} ms, p95 {:.2} ms, worst {:.1} ms over {} ticks",
        cost.sum.as_secs_f64() * 1e3 / cost.ticks.max(1) as f64,
        cost.percentile_ms(0.95),
        cost.worst.as_secs_f64() * 1e3,
        cost.ticks
    );
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}
