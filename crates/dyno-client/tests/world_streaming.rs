//! World streaming and area of interest through the real protocol
//! (docs/design/data-and-streaming.md §4): each client holds only the
//! sections near it, drops those it leaves behind, hears edits only for what
//! it holds, and hears about entities and players only near it.

use dyno_client::net::{self, Joined, NetEvent};
use dyno_protocol::{ClientMsg, PlayerId, Pose, ServerMsg};
use dyno_world::SECTION;
use std::collections::HashSet;
use std::time::{Duration, Instant};

fn start(dir: &std::path::Path) -> (dyno_server::Handle, String) {
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        world_size: [192, 48, 192],
        trust_movement: true,
        stream: dyno_server::stream::StreamConfig {
            view: 48.0,
            keep: 16.0,
            aoi: 32.0,
            aoi_keep: 8.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let h = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", h.local_addr.port());
    (h, addr)
}

/// A joined client and what it has heard.
struct Client {
    j: Joined,
    /// entities it has been introduced to and not told are gone
    known: HashSet<u32>,
    /// players whose poses it receives (not told they went far)
    near: HashSet<PlayerId>,
    log: Vec<ServerMsg>,
}

impl Client {
    fn join(addr: &str, name: &str, token: u64) -> Client {
        let j = net::connect(addr, name, token, false, Duration::from_secs(20)).expect("joins");
        Client {
            j,
            known: HashSet::new(),
            near: HashSet::new(),
            log: Vec::new(),
        }
    }

    /// Take in what arrives for `secs`: the world, entities, poses.
    fn pump(&mut self, secs: f32) {
        let end = Instant::now() + Duration::from_secs_f32(secs);
        while Instant::now() < end {
            match self.j.conn.rx.recv_timeout(Duration::from_millis(20)) {
                Ok(NetEvent::Msg(m)) => {
                    net::apply_world(&mut self.j.world, &m);
                    match &m {
                        ServerMsg::EntitySpawn { id, .. } => {
                            self.known.insert(*id);
                        }
                        ServerMsg::EntityGone { id, .. } => {
                            self.known.remove(id);
                        }
                        ServerMsg::Poses(list) => {
                            self.near.extend(list.iter().map(|(i, _)| *i));
                        }
                        ServerMsg::PlayersFar(ids) => {
                            for i in ids {
                                self.near.remove(i);
                            }
                        }
                        _ => {}
                    }
                    self.log.push(m);
                }
                Ok(NetEvent::Closed(r)) => panic!("closed: {r}"),
                _ => {}
            }
        }
    }

    fn go(&self, pos: [f32; 3]) {
        self.j
            .conn
            .tx
            .send(ClientMsg::Pose(Pose {
                pos,
                yaw: 0.0,
                pitch: 0.0,
            }))
            .unwrap();
    }

    /// The columns (section x, z) it holds.
    fn columns(&self) -> HashSet<[i32; 2]> {
        self.j
            .world
            .held_list()
            .into_iter()
            .map(|s| [s[0], s[2]])
            .collect()
    }
}

fn col_dist(c: [i32; 2], at: [f32; 3]) -> f32 {
    let (cx, cz) = (
        (c[0] * SECTION + SECTION / 2) as f32,
        (c[1] * SECTION + SECTION / 2) as f32,
    );
    (cx - at[0]).hypot(cz - at[2])
}

#[test]
fn the_world_streams_around_each_player_and_so_do_entities_and_players() {
    let dir = std::env::temp_dir().join(format!("dyno-test-wstream-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (server, addr) = start(&dir);

    // 1. a join holds the land around the spawn, not the whole world
    let mut a = Client::join(&addr, "ada", 41);
    let spawn = a.j.spawn;
    assert!(a.j.world.is_sparse());
    assert!(a.j.world_bytes > 0, "the world came as sections");
    for s in net::sections_around(&a.j.world, spawn, net::LOAD_RADIUS.min(a.j.view)) {
        assert!(a.j.world.has_section(s), "{s:?} is there before play");
    }
    a.pump(1.5);
    let cols = a.columns();
    let d = a.j.world.section_dims();
    assert!(cols.len() < (d[0] * d[2]) as usize, "not the whole world");
    assert!(
        cols.iter().all(|c| col_dist(*c, spawn) <= a.j.view),
        "only within the view"
    );
    assert!(
        cols.iter()
            .all(|c| (0..d[1]).all(|y| a.j.world.has_section([c[0], y, c[1]]))),
        "whole columns"
    );
    // the spawn hearth comes from the server, wherever it streamed
    assert_eq!(a.j.hearth, server.server.main_hearth());

    // 2. a second player nearby: each hears the other's poses
    let mut b = Client::join(&addr, "bo", 42);
    let here = [spawn[0] + 3.0, spawn[1], spawn[2]];
    for _ in 0..10 {
        a.go(spawn);
        b.go(here);
        a.pump(0.05);
        b.pump(0.05);
    }
    a.pump(0.3);
    b.pump(0.3);
    assert!(a.near.contains(&b.j.your_id), "a hears b nearby");
    let a_knew = a.known.clone();
    assert!(
        !a_knew.is_empty(),
        "the creatures and folk around the spawn"
    );
    assert_eq!(b.known, a_knew, "the same place, the same company");

    // 3. b walks far east: the land behind is let go, the land ahead streams in,
    //    the creatures behind are forgotten, and a stops hearing b
    let far = [spawn[0] + 80.0, spawn[1] + 8.0, spawn[2]];
    for _ in 0..30 {
        b.go(far);
        a.go(spawn);
        b.pump(0.05);
        a.pump(0.02);
    }
    b.pump(1.0);
    a.pump(0.3);
    assert!(
        b.log
            .iter()
            .any(|m| matches!(m, ServerMsg::SectionsGone(l) if !l.is_empty())),
        "far sections are dropped"
    );
    let bcols = b.columns();
    assert!(
        bcols
            .iter()
            .all(|c| col_dist(*c, far) <= b.j.view + 16.0 + 12.0)
    );
    assert!(
        bcols.iter().any(|c| col_dist(*c, far) < 16.0),
        "the land where b stands is there"
    );
    assert!(
        !bcols.iter().any(|c| col_dist(*c, spawn) < 12.0),
        "the spawn is let go"
    );
    assert!(!a.near.contains(&b.j.your_id), "a was told b went far");
    assert!(
        b.known.intersection(&a_knew).count() * 4 <= a_knew.len(),
        "b forgot the spawn's creatures ({} of {} still known)",
        b.known.intersection(&a_knew).count(),
        a_knew.len()
    );
    // moves of the spawn's creatures no longer reach b
    let since = b.log.len();
    b.go(far);
    b.pump(1.0);
    for m in &b.log[since..] {
        if let ServerMsg::EntityMoves(list) = m {
            assert!(
                list.iter().all(|(id, ..)| b.known.contains(id)),
                "only moves of what b knows"
            );
        }
    }

    // 4. an edit at the spawn reaches a, who holds it, and not b
    a.j.conn
        .tx
        .send(ClientMsg::Chat {
            text: "/gamemode build".into(),
        })
        .unwrap();
    a.pump(0.5);
    let s = [spawn[0] as i32, spawn[1] as i32 + 6, spawn[2] as i32];
    a.j.conn
        .tx
        .send(ClientMsg::Fill {
            a: s,
            b: [s[0] + 1, s[1], s[2] + 1],
            block: dyno_world::BRICK,
        })
        .unwrap();
    let (sa, sb) = (a.log.len(), b.log.len());
    a.pump(1.0);
    b.pump(0.5);
    assert!(
        a.log[sa..]
            .iter()
            .any(|m| matches!(m, ServerMsg::Fill { .. })),
        "the holder hears the edit"
    );
    assert_eq!(a.j.world.get(s[0], s[1], s[2]), dyno_world::BRICK);
    assert!(
        !b.log[sb..]
            .iter()
            .any(|m| matches!(m, ServerMsg::Fill { .. })),
        "a client that does not hold the section does not"
    );

    // 5. b comes back: the spawn streams in again, with the edit in it, and
    //    a hears b again
    for _ in 0..30 {
        b.go(here);
        a.go(spawn);
        b.pump(0.05);
        a.pump(0.02);
    }
    b.pump(1.0);
    a.pump(0.3);
    assert_eq!(b.j.world.get(s[0], s[1], s[2]), dyno_world::BRICK);
    assert!(a.near.contains(&b.j.your_id));
    assert!(!b.known.is_empty(), "the spawn's creatures are back");

    // 6. b dies beside a far from the spawn and comes back to life at the
    //    spawn: a hears it get up, and is told b went far (no body is left
    //    lying where a can see it)
    for _ in 0..40 {
        a.go(far);
        b.go([far[0] + 2.0, far[1], far[2]]);
        a.pump(0.05);
        b.pump(0.05);
    }
    a.pump(0.5);
    assert!(a.near.contains(&b.j.your_id));
    // a fall no one survives (/kill is an operator's)
    b.j.conn
        .tx
        .send(ClientMsg::Hurt {
            kind: dyno_protocol::EnvDamage::Fall,
            amount: 40,
        })
        .unwrap();
    let bid = b.j.your_id;
    let saw = |c: &Client, from: usize, act: dyno_protocol::Act| {
        c.log[from..].iter().any(|m| {
            matches!(m, ServerMsg::Act { actor: dyno_protocol::Actor::Player(p), act: x }
                if *p == bid && *x == act)
        })
    };
    let since = a.log.len();
    a.go(far);
    a.pump(1.0);
    assert!(saw(&a, since, dyno_protocol::Act::Died), "a sees b fall");
    b.j.conn.tx.send(ClientMsg::Respawn).unwrap();
    for _ in 0..20 {
        a.go(far);
        a.pump(0.05);
        b.pump(0.05);
    }
    a.pump(0.5);
    assert!(
        saw(&a, since, dyno_protocol::Act::Revived),
        "a hears b get up"
    );
    assert!(!a.near.contains(&bid), "and that b is far away now");

    drop((a, b));
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_observer_streams_the_world_around_its_camera() {
    let dir = std::env::temp_dir().join(format!("dyno-test-wview-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (server, addr) = start(&dir);
    let at = [170.0, 30.0, 20.0];
    let j = net::connect_with(
        &addr,
        "camera",
        net::JoinOptions {
            observer: true,
            view_at: Some(at),
            ..Default::default()
        },
        Duration::from_secs(20),
    )
    .expect("joins");
    for s in net::sections_around(&j.world, at, net::LOAD_RADIUS) {
        assert!(j.world.has_section(s), "{s:?} around the camera");
    }
    assert!(
        !j.world.has_section([96 / SECTION, 1, 96 / SECTION]),
        "not the spawn"
    );
    drop(j);
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

/// `dyno shot --as NAME --pos`: a player looking from somewhere else gets
/// the world round that point.
#[test]
fn a_player_can_look_from_elsewhere() {
    let dir = std::env::temp_dir().join(format!("dyno-test-wlook-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (server, addr) = start(&dir);
    let at = [20.0, 30.0, 170.0];
    let j = net::connect_with(
        &addr,
        "looker",
        net::JoinOptions {
            token: 77,
            view_at: Some(at),
            ..Default::default()
        },
        Duration::from_secs(20),
    )
    .expect("joins");
    for s in net::sections_around(&j.world, at, net::LOAD_RADIUS) {
        assert!(j.world.has_section(s), "{s:?} round where the player looks");
    }
    drop(j);
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}
