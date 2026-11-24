//! Movement over a poor line (docs/design/netcode.md): a real server, real
//! client networking through the network conditioner (`NetSim`), the
//! client's prediction (`predict.rs`) and remote smoothing.
//!
//! `prediction_holds_under_a_poor_line` runs a scripted course (walk, run,
//! sprint, jump, stairs, a ledge, a doorway, a step up) with 150 ms of
//! latency, 30 ms of jitter and 2% loss, while a second player watches. It
//! fails on a snap, or when the watched runner's drawn speed wobbles. It
//! runs for 50 s; `DYNO_NETCODE_SECS=600` makes it the 10-minute session.
//! The watcher also measures the creatures and townsfolk it sees walking.
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
//! Either way the watcher draws the runner both as before and from the
//! buffer (interp.rs), from the same poses.

use dyno_client::game::{Input, Player, Remote, apply_rules};
use dyno_client::interp;
use dyno_client::net::{self, Joined, NetEvent, NetSim};
use dyno_client::predict::Predictor;
use dyno_protocol::{self as proto, ClientMsg, EntityKind, Pose, ServerMsg};
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
    seed: 0,
};

/// The line of the session: `POOR`, or `DYNO_NET_SIM` when set.
fn line() -> NetSim {
    NetSim::from_env().unwrap_or(POOR)
}

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

/// The course: a stone loop high above the town, along x. The east-bound
/// lane has the features; the west-bound lane, past a dividing wall, is
/// flat.
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
        (66.0..=94.0).contains(&x)
    }

    /// The middle of the lane the runner keeps to, going `dir`.
    fn lane(&self, dir: f32) -> f32 {
        self.z0 as f32 + if dir > 0.0 { 0.5 } else { 6.5 }
    }

    /// On the middle of either lane.
    fn on_lane(&self, z: f32) -> bool {
        (z - self.lane(1.0)).abs() < 0.4 || (z - self.lane(-1.0)).abs() < 0.4
    }

    /// The fills that make it.
    fn fills(&self) -> Vec<([i32; 3], [i32; 3], u8)> {
        use dyno_world::{AIR, STONE};
        let (x0, x1, z0, f) = (self.x0, self.x0 + self.len, self.z0, self.feet);
        let e = |a: i32, b: i32, h: i32| ([x0 + a, f, z0 - 2], [x0 + b, f + h - 1, z0 + 2], STONE);
        vec![
            // clear sky, and the floor of both lanes
            ([x0 - 2, f - 1, z0 - 4], [x1 + 2, f + 7, z0 + 10], AIR),
            ([x0, f - 1, z0 - 2], [x1, f - 1, z0 + 8], STONE),
            // walls along the sides, and between the lanes (open at both ends)
            ([x0, f - 1, z0 - 3], [x1, f + 1, z0 - 3], STONE),
            ([x0, f - 1, z0 + 9], [x1, f + 1, z0 + 9], STONE),
            ([x0 + 14, f, z0 + 3], [x1 - 14, f + 1, z0 + 3], STONE),
            ([x0 - 1, f - 1, z0 - 3], [x0 - 1, f + 1, z0 + 9], STONE),
            ([x1 + 1, f - 1, z0 - 3], [x1 + 1, f + 1, z0 + 9], STONE),
            // stairs of 1 m up to a 3 m deck, and a 3 m ledge down from it
            e(20, 20, 1),
            e(21, 21, 2),
            e(22, 30, 3),
            // a wall with a doorway 1 m wide and 2 m high
            e(40, 40, 4),
            ([x0 + 40, f, z0], [x0 + 40, f + 1, z0], AIR),
            // a step up of 1 m and down again
            e(48, 53, 1),
        ]
    }
}

/// What the runner does at `x` going `dir` (+1 east, -1 west), at time `t`.
fn runner_input(c: &Course, x: f32, dir: f32, t: f32, lap: u32) -> Input {
    let rel = x - c.x0 as f32;
    let mut i = Input {
        forward: 1.0,
        ..Default::default()
    };
    if dir > 0.0 && rel < 60.0 {
        // the features: walk some laps (the first stretch), sprint others,
        // jump now and then, and stop and start again
        match lap % 4 {
            1 if rel < 35.0 => i.forward = 0.45,
            2 => i.sprint = true,
            _ => {}
        }
        i.jump = (t * 1.7).fract() < 0.12 && lap % 3 != 0;
        if lap % 4 == 3 && (t * 0.4).fract() < 0.15 {
            i.forward = 0.0;
        }
        // and a little strafe in the open
        if rel < 18.0 && lap % 5 == 4 {
            i.right = ((t * 0.9).sin() * 0.4).clamp(-1.0, 1.0);
        }
    }
    if dir < 0.0 && rel < 60.0 && lap % 3 == 2 {
        // the flat lane back: a jump or two there too
        i.jump = (t * 1.3).fract() < 0.1;
    }
    i
}

/// Facing along the lane, steering back to its middle.
fn runner_yaw(c: &Course, pos: Vec3, dir: f32) -> f32 {
    let dz = ((c.lane(dir) - pos.z) * 0.8).clamp(-0.6, 0.6);
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

/// How the runner was drawn before (the old `Remote`): between the last
/// two poses, by when they arrived.
struct OldLerp {
    from: Vec3,
    to: Vec3,
    at: Instant,
}

impl OldLerp {
    fn set(&mut self, pos: Vec3) {
        self.from = self.current();
        self.to = pos;
        self.at = Instant::now();
    }
    fn current(&self) -> Vec3 {
        let t = (self.at.elapsed().as_secs_f32() / 0.05).clamp(0.0, 1.0);
        self.from.lerp(self.to, t)
    }
}

/// A creature or townsperson the watcher draws: as before (between where
/// it was drawn and the newest position, over 100 ms from its arrival),
/// from the buffer, and the positions as stamped (the truth).
struct Walker {
    track: interp::Track,
    old_from: Vec3,
    old_to: Vec3,
    old_at: Instant,
    stamped: std::collections::VecDeque<(f64, Vec3)>,
    prev: Option<(Vec3, Vec3)>,
    prev_v: Option<(f32, f32)>,
}

impl Walker {
    fn old(&self) -> Vec3 {
        let t = (self.old_at.elapsed().as_secs_f32() / 0.1).clamp(0.0, 1.0);
        self.old_from.lerp(self.old_to, t)
    }
    /// Its speed as stamped at `t`, if `t` is between two stamped positions.
    fn true_speed(&self, t: f64) -> Option<f32> {
        let i = self.stamped.iter().position(|s| s.0 > t)?;
        let (a, b) = (self.stamped.get(i.checked_sub(1)?)?, self.stamped[i]);
        let d = Vec3::new(b.1.x - a.1.x, 0.0, b.1.z - a.1.z).length();
        Some(d / (b.0 - a.0).max(1e-3) as f32)
    }
}

/// What the watcher saw of the runner on the straight.
#[derive(Default)]
struct Watched {
    /// drawn as before (between the last two poses, by arrival)
    old: Wobble,
    /// drawn from the buffer (interp.rs): every frame
    new: Wobble,
    /// the frames drawn from the buffer, not past its newest position
    buffered: Wobble,
    /// walking creatures and townsfolk, drawn from the buffer, against
    /// their stamped speed
    walkers: Wobble,
    /// their drawn speed's change from frame to frame (m/s), from the
    /// buffer and as before
    walk_jerk: Wobble,
    walk_jerk_old: Wobble,
}

/// Drawn speed this frame, if both ends are on the straight.
fn on_straight(course: &Course, prev: Vec3, at: Vec3, dt: f32) -> Option<f32> {
    let on = |p: Vec3| {
        course.steady(p.x) && (p.y - course.feet as f32).abs() < 0.05 && course.on_lane(p.z)
    };
    (on(prev) && on(at)).then(|| Vec3::new(at.x - prev.x, 0.0, at.z - prev.z).length() / dt)
}

/// The watcher: another player who draws the runner as the game would, 60
/// frames a second, and measures its drawn speed on the steady stretch.
fn watch(
    j: Joined,
    runner: proto::PlayerId,
    course: Arc<Course>,
    want: f32,
    stop: Arc<AtomicBool>,
) -> Watched {
    let mut old: Option<OldLerp> = None;
    let mut remote: Option<Remote> = None;
    let mut walkers: std::collections::HashMap<u32, Walker> = Default::default();
    let mut clock = interp::Clock::default();
    let mut w = Watched::default();
    let mut prev: Option<(Instant, Vec3, Vec3)> = None;
    let frame = Duration::from_micros(16_667);
    let mut next = Instant::now();
    // what came while the course was built (a backlog no client would
    // have): only the entities in it, then draw for 2 s before measuring
    while let Ok(ev) = j.conn.rx.try_recv() {
        if let NetEvent::Msg(ServerMsg::EntitySpawn { id, kind, pos, yaw }) = ev
            && matches!(kind, EntityKind::Creature(_) | EntityKind::Npc(_))
        {
            let p = Vec3::from(pos);
            walkers.insert(
                id,
                Walker {
                    track: interp::Track::new(p, yaw),
                    old_from: p,
                    old_to: p,
                    old_at: Instant::now(),
                    stamped: Default::default(),
                    prev: None,
                    prev_v: None,
                },
            );
        }
    }
    let started = Instant::now();
    let mut last_frame: Option<Instant> = None;
    while !stop.load(Ordering::Relaxed) {
        while let Ok(ev) = j.conn.rx.try_recv() {
            match &ev {
                NetEvent::Msg(ServerMsg::EntitySpawn { id, kind, pos, yaw })
                    if matches!(kind, EntityKind::Creature(_) | EntityKind::Npc(_)) =>
                {
                    let p = Vec3::from(*pos);
                    walkers.insert(
                        *id,
                        Walker {
                            track: interp::Track::new(p, *yaw),
                            old_from: p,
                            old_to: p,
                            old_at: Instant::now(),
                            stamped: Default::default(),
                            prev: None,
                            prev_v: None,
                        },
                    );
                }
                NetEvent::Msg(ServerMsg::EntityGone { id, .. }) => {
                    walkers.remove(id);
                }
                NetEvent::Msg(ServerMsg::EntityMoves { at, list }) => {
                    let t = *at as f64 / 1000.0;
                    clock.arrived(interp::now(), t);
                    for (id, pos, yaw) in list {
                        if let Some(w) = walkers.get_mut(id) {
                            let p = Vec3::from(*pos);
                            w.track.push(t, p, *yaw);
                            w.old_from = w.old();
                            w.old_to = p;
                            w.old_at = Instant::now();
                            if w.stamped.back().is_some_and(|s| s.0 < t - 0.06) {
                                // it stood still in between
                                let last = w.stamped.back().unwrap().1;
                                w.stamped.push_back((t - 0.05, last));
                            }
                            w.stamped.push_back((t, p));
                            while w.stamped.len() > 200 {
                                w.stamped.pop_front();
                            }
                        }
                    }
                    for w in walkers.values_mut() {
                        w.track.still_at(t);
                    }
                }
                _ => {}
            }
            if let NetEvent::Msg(ServerMsg::Poses(list)) = ev {
                for (id, stamp, pose) in list {
                    if id != runner {
                        continue;
                    }
                    let pos = Vec3::from(pose.pos);
                    old.get_or_insert(OldLerp {
                        from: pos,
                        to: pos,
                        at: Instant::now(),
                    })
                    .set(pos);
                    remote
                        .get_or_insert_with(|| Remote::new("runner".into(), pose))
                        .set(stamp, pose, interp::now());
                }
            }
        }
        let now = Instant::now();
        let t = clock.render(interp::now());
        let warm = started.elapsed() > Duration::from_secs(2);
        let frame_dt = last_frame.map_or(1.0 / 60.0, |l: Instant| {
            now.duration_since(l).as_secs_f32().max(1e-4)
        });
        last_frame = Some(now);
        for wk in walkers.values_mut() {
            wk.track.update(t);
            let (a, b) = (wk.old(), wk.track.pos());
            if let Some((pa, pb)) = wk.prev {
                let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z).length() / frame_dt;
                let (va, vb) = (flat(a - pa), flat(b - pb));
                if let Some(truth) = t.and_then(|t| wk.true_speed(t))
                    && truth > 0.3
                    && !wk.track.beyond()
                    && warm
                {
                    w.walkers.add(vb, truth);
                    if let Some((_, pvb)) = wk.prev_v {
                        w.walk_jerk.add(vb - pvb, 0.0);
                    }
                }
                if let Some((pva, _)) = wk.prev_v
                    && (va > 0.3 || pva > 0.3)
                    && warm
                {
                    w.walk_jerk_old.add(va - pva, 0.0);
                }
                wk.prev_v = Some((va, vb));
            }
            wk.prev = Some((a, b));
        }
        if let (Some(o), Some(r)) = (&old, &mut remote) {
            r.update(interp::now());
            let (a, b) = (o.current(), Vec3::from(r.current().pos));
            if let Some((t, pa, pb)) = prev {
                let dt = now.duration_since(t).as_secs_f32().max(1e-4);
                if let Some(v) = on_straight(&course, pa, a, dt).filter(|_| warm) {
                    w.old.add(v, want);
                }
                if let Some(v) = on_straight(&course, pb, b, dt).filter(|_| warm) {
                    w.new.add(v, want);
                    if !r.late() {
                        w.buffered.add(v, want);
                    }
                }
            }
            prev = Some((now, a, b));
        }
        next += frame;
        match next.checked_duration_since(Instant::now()) {
            Some(d) => std::thread::sleep(d),
            None => next = Instant::now(),
        }
    }
    w
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
    // (each line its own fixed seed, unless DYNO_NET_SIM gives one: the
    // same losses each run)
    let seeded = |seed| {
        let l = line();
        NetSim {
            seed: if l.seed == 0 { seed } else { l.seed },
            ..l
        }
    };
    let mut m = Mover::new(join(&addr, "Runner", 8101, Some(seeded(11))), pose);
    let watcher = join(&addr, "Watcher", 8102, Some(seeded(12)));
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
    let start = [course.x0 as f32 + 3.5, course.feet as f32, course.lane(1.0)];
    std::thread::sleep(Duration::from_millis(600));
    m.send(ClientMsg::Pose(Pose {
        pos: start,
        yaw: 0.0,
        pitch: 0.0,
    }));
    m.land([course.x0 as f32 + 55.0, start[1], start[2] + 3.0], 72.0);
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
    m.last = Instant::now();

    // the watcher, from the town
    let stop = Arc::new(AtomicBool::new(false));
    let (c2, s2) = (course.clone(), stop.clone());
    let want = 6.2;
    let watching = std::thread::spawn(move || watch(watcher, runner_id, c2, want, s2));

    let run_for = secs(50.0);
    let t0 = Instant::now();
    let (mut dir_x, mut lap) = (1.0f32, 0u32);
    let frame = Duration::from_micros(16_667);
    let mut next = Instant::now();
    let mut lowest = f32::MAX;
    while t0.elapsed().as_secs_f32() < run_for {
        let seen = m.pred.stats.eased + m.pred.stats.snaps;
        m.pump();
        let rel = m.body.pos.x - course.x0 as f32;
        if (dir_x > 0.0 && rel > course.len as f32 - 2.0) || (dir_x < 0.0 && rel < 2.0) {
            dir_x = -dir_x;
            lap += 1;
        }
        let t = t0.elapsed().as_secs_f32();
        let input = runner_input(&course, m.body.pos.x, dir_x, t, lap);
        let yaw = runner_yaw(&course, m.body.pos, dir_x);
        m.frame(input, yaw);
        lowest = lowest.min(m.body.pos.y);
        if m.pred.stats.eased + m.pred.stats.snaps != seen {
            eprintln!(
                "CORRECTION at {:.1} s: {:.2} m at {:?} (lap {lap}, {} unanswered)",
                t,
                m.pred.stats.last,
                m.body.pos,
                m.pred.unanswered()
            );
        }
        next += frame;
        match next.checked_duration_since(Instant::now()) {
            Some(d) => std::thread::sleep(d),
            None => next = Instant::now(),
        }
    }
    stop.store(true, Ordering::Relaxed);
    let w = watching.join().unwrap();
    let st = m.pred.stats;
    let (snaps, eased) = (st.snaps - base.snaps, st.eased - base.eased);
    eprintln!(
        "NETCODE {}: {:.0} s, {lap} laps; snaps {snaps}, eased {eased}, worst correction {:.2} m, \
         {} answers, teleports {}",
        if pose { "pose" } else { "steps" },
        t0.elapsed().as_secs_f32(),
        st.worst,
        st.acks,
        st.teleports,
    );
    eprintln!(
        "  walkers from the buffer: {} frames, drawn speed against stamped: rms error {:.3} m/s, \
         worst {:.2}; change frame to frame: rms {:.3} m/s from the buffer, {:.3} m/s as before \
         ({} frames)",
        w.walkers.n,
        w.walkers.rms(),
        w.walkers.worst,
        w.walk_jerk.rms(),
        w.walk_jerk_old.rms(),
        w.walk_jerk_old.n
    );
    for (name, wb) in [
        ("as before (last two poses)", &w.old),
        ("from the buffer, every frame", &w.new),
        ("from the buffer, not late", &w.buffered),
    ] {
        eprintln!(
            "  runner drawn {name}: {} frames on the straight, mean {:.2} m/s (true {want}), \
             rms error {:.3} m/s, worst {:.2} m/s",
            wb.n,
            wb.mean(),
            wb.rms(),
            wb.worst
        );
    }

    assert!(
        lap >= 2,
        "the runner went up and down the course: {lap} laps"
    );
    assert!(
        lowest > course.feet as f32 - 0.5,
        "the runner stayed on the course"
    );
    assert!(w.new.n > 100, "the runner was watched on the straight");
    if !pose {
        assert_eq!(snaps, 0, "no snap in normal play");
        // drawn from the buffer: steady where its positions were on time,
        // and far steadier than before through the line's stalls
        assert!(
            w.buffered.rms() < 0.5 && w.buffered.worst < 2.0,
            "the runner's drawn speed wobbles: {:.2} m/s rms, worst {:.2}",
            w.buffered.rms(),
            w.buffered.worst
        );
        assert!(
            w.walkers.n < 100 || w.walkers.rms() < 0.5,
            "walking creatures and townsfolk wobble: {:.2} m/s rms",
            w.walkers.rms()
        );
        assert!(
            w.new.rms() < 1.5,
            "the runner's drawn speed wobbles through stalls: {:.2} m/s rms",
            w.new.rms()
        );
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
