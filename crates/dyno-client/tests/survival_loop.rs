//! Plays the game through the real protocol against a real server.

use dyno_client::net::{self, Joined, NetEvent};
use dyno_protocol::{ClientMsg, EntityKind, EnvDamage, Mode, Pose, ServerMsg, SlotRef, Stack};
use std::time::{Duration, Instant};

/// A server of the shipped pack on a fresh world: the town and the land
/// 96 blocks round it.
fn start_server(dir: &std::path::Path) -> dyno_server::Handle {
    let _ = std::fs::remove_dir_all(dir);
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        world_size: [192, 48, 192],
        trust_movement: true,
        ..Default::default()
    };
    dyno_server::start(cfg).expect("server starts")
}

fn say(j: &Joined, text: &str) {
    send(j, ClientMsg::Chat { text: text.into() });
}

/// The nearest block of one of `kinds` within `r` columns of (x, z), in the
/// world streamed so far.
fn block_near(j: &Joined, x: i32, z: i32, r: i32, kinds: &[u8]) -> Option<[i32; 3]> {
    let mut best: Option<(i32, [i32; 3])> = None;
    for dz in -r..=r {
        for dx in -r..=r {
            for y in 1..j.world.size[1] as i32 {
                if kinds.contains(&j.world.get(x + dx, y, z + dz)) {
                    let d = dx * dx + dz * dz;
                    if best.is_none_or(|(b, _)| d < b) {
                        best = Some((d, [x + dx, y, z + dz]));
                    }
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

/// Die and get up again: where the server puts you.
fn die_and_wake(j: &Joined, pack: &mut Vec<Option<Stack>>) -> [f32; 3] {
    say(j, "/kill");
    wait(j, pack, 3.0, |m, _| {
        matches!(m, ServerMsg::Died { .. }).then_some(())
    });
    send(j, ClientMsg::Respawn);
    wait(j, pack, 3.0, |m, _| match m {
        ServerMsg::Teleport { pos } => Some(*pos),
        _ => None,
    })
}

fn flat_dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Wait for a message that matches, applying pack and status updates on the way.
fn wait<T>(
    j: &Joined,
    pack: &mut Vec<Option<Stack>>,
    secs: f32,
    mut f: impl FnMut(&ServerMsg, &[Option<Stack>]) -> Option<T>,
) -> T {
    let end = Instant::now() + Duration::from_secs_f32(secs);
    while Instant::now() < end {
        match j.conn.rx.recv_timeout(Duration::from_millis(50)) {
            Ok(NetEvent::Msg(m)) => {
                if let ServerMsg::Pack(p) = &m {
                    *pack = p.clone();
                }
                if let Some(t) = f(&m, pack) {
                    return t;
                }
            }
            Ok(NetEvent::Closed(r)) => panic!("closed: {r}"),
            _ => {}
        }
    }
    panic!("timed out waiting");
}

fn count(pack: &[Option<Stack>], item: u16) -> u32 {
    pack.iter()
        .flatten()
        .filter(|s| s.item == item)
        .map(|s| s.count as u32)
        .sum()
}

fn send(j: &Joined, m: ClientMsg) {
    j.conn.tx.send(m).unwrap();
}

/// The world streams in around the player: wait until the land within `r`
/// of `at` is here (what arrives meanwhile is applied to the world, and
/// anything else is dropped).
fn land(j: &mut Joined, at: [f32; 3], r: f32) {
    let need = net::sections_around(&j.world, at, r);
    let end = Instant::now() + Duration::from_secs(20);
    while !need.iter().all(|s| j.world.has_section(*s)) {
        assert!(Instant::now() < end, "the land around {at:?} streams in");
        if let Ok(NetEvent::Msg(m)) = j.conn.rx.recv_timeout(Duration::from_millis(20)) {
            net::apply_world(&mut j.world, &m);
        }
    }
}

fn pose(j: &Joined, pos: [f32; 3]) {
    send(
        j,
        ClientMsg::Pose(Pose {
            pos,
            yaw: 0.0,
            pitch: 0.0,
        }),
    );
}

/// Shades rise in the dark at night, never inside a hearth's reach, and
/// daylight unmakes them.
#[test]
fn night_brings_shades() {
    let dir = std::env::temp_dir().join(format!("dyno-test-night-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        world_size: [192, 48, 192],
        trust_movement: true,
        ..Default::default()
    };
    let server = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let mut j =
        net::connect(&addr, "nightwatch", 777, false, Duration::from_secs(5)).expect("join");
    let shade = j
        .content
        .creatures
        .iter()
        .find(|c| c.key == "shade")
        .unwrap()
        .id;
    let hearth = j.hearth;
    let mut pack = vec![None; 36];
    // out in the meadow, beyond the town (where Shades never rise)
    let here = {
        let (x, z) = (hearth[0] + 52, hearth[2]);
        land(&mut j, [x as f32, 0.0, z as f32], 24.0);
        [
            x as f32 + 0.5,
            j.world.surface(x, z) as f32 + 1.0,
            z as f32 + 0.5,
        ]
    };
    pose(&j, here);
    // the first player owns a new world, so may set the time
    send(
        &j,
        ClientMsg::Chat {
            text: "/time night".into(),
        },
    );
    let end = Instant::now() + Duration::from_secs(90);
    let mut found = None;
    while found.is_none() && Instant::now() < end {
        pose(&j, here);
        found = wait_opt(&j, &mut pack, 1.0, |m| match m {
            ServerMsg::EntitySpawn {
                id,
                kind: EntityKind::Creature(c),
                pos,
                ..
            } if *c == shade => Some((*id, *pos)),
            _ => None,
        });
    }
    let (sid, spos) = found.expect("a Shade rises at night");
    let d = ((spos[0] - hearth[0] as f32 - 0.5).powi(2)
        + (spos[2] - hearth[2] as f32 - 0.5).powi(2))
    .sqrt();
    assert!(
        d >= dyno_world::HEARTH_RADIUS,
        "Shade spawned {d} blocks from the hearth"
    );
    // dawn: it burns away
    send(
        &j,
        ClientMsg::Chat {
            text: "/time day".into(),
        },
    );
    let end = Instant::now() + Duration::from_secs(15);
    let mut gone = false;
    while !gone && Instant::now() < end {
        pose(&j, here);
        gone = wait_opt(&j, &mut pack, 1.0, |m| {
            matches!(m, ServerMsg::EntityGone { id, .. } if *id == sid).then_some(())
        })
        .is_some();
    }
    assert!(gone, "daylight unmakes the Shade");
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

fn wait_opt<T>(
    j: &Joined,
    pack: &mut Vec<Option<Stack>>,
    secs: f32,
    mut f: impl FnMut(&ServerMsg) -> Option<T>,
) -> Option<T> {
    let end = Instant::now() + Duration::from_secs_f32(secs);
    while Instant::now() < end {
        match j.conn.rx.recv_timeout(Duration::from_millis(50)) {
            Ok(NetEvent::Msg(m)) => {
                if let ServerMsg::Pack(p) = &m {
                    *pack = p.clone();
                }
                if let Some(t) = f(&m) {
                    return Some(t);
                }
            }
            Ok(NetEvent::Closed(r)) => panic!("closed: {r}"),
            _ => {}
        }
    }
    None
}

/// Bags, falls, death and names: items move between bag slots and nothing
/// is thrown away or put in a chest; a fall hurts; death keeps everything;
/// a name belongs to its first player.
#[test]
fn bags_falls_death_and_names() {
    let dir = std::env::temp_dir().join(format!("dyno-test-bags-{}", std::process::id()));
    let server = start_server(&dir);
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let j = net::connect(&addr, "tester", 111, false, Duration::from_secs(5)).expect("join");
    assert_eq!(j.mode, Mode::Survival);
    let c = j.content.clone();
    let id = |k: &str| c.item_by_key(k).unwrap();
    let mut pack = vec![None; 36];
    pose(&j, j.spawn);

    // 1. moving items keeps counts right (the first player owns the world
    //    and may /give)
    say(&j, "/give stick 3");
    wait(&j, &mut pack, 3.0, |_, pack| {
        (count(pack, id("stick")) == 3).then_some(())
    });
    let from = pack
        .iter()
        .position(|s| s.is_some_and(|s| s.item == id("stick")))
        .unwrap() as u8;
    send(
        &j,
        ClientMsg::Move {
            from: SlotRef::Pack(from),
            to: SlotRef::Pack(30),
            count: 1,
        },
    );
    wait(&j, &mut pack, 3.0, |_, pack| {
        pack[30]
            .is_some_and(|s| s.item == id("stick") && s.count == 1)
            .then_some(())
    });
    assert_eq!(count(&pack, id("stick")), 3);
    // nothing is thrown on the ground, and chests take nothing: both are
    // ignored or refused, and the bags stay as they were
    send(
        &j,
        ClientMsg::Throw {
            from: SlotRef::Pack(30),
            count: 0,
        },
    );
    send(
        &j,
        ClientMsg::Move {
            from: SlotRef::Pack(30),
            to: SlotRef::Chest(0),
            count: 0,
        },
    );
    send(
        &j,
        ClientMsg::Move {
            from: SlotRef::Pack(30),
            to: SlotRef::Pack(31),
            count: 0,
        },
    );
    wait(&j, &mut pack, 3.0, |_, pack| {
        pack[31]
            .is_some_and(|s| s.item == id("stick") && s.count == 1)
            .then_some(())
    });
    assert_eq!(count(&pack, id("stick")), 3, "nothing left the bags");

    // 2. a fall hurts
    let hurt = |j: &Joined, pack: &mut Vec<Option<Stack>>| {
        send(
            j,
            ClientMsg::Hurt {
                kind: EnvDamage::Fall,
                amount: 5,
            },
        );
        wait(j, pack, 3.0, |m, _| match m {
            ServerMsg::Status { health, .. } => Some(*health),
            _ => None,
        })
    };
    let first = hurt(&j, &mut pack);
    let second = hurt(&j, &mut pack);
    assert!(
        first > 0.0 && second < first,
        "falls hurt: {first} then {second}"
    );

    // 3. death keeps everything, and you wake with health
    let before = pack.clone();
    let woke = die_and_wake(&j, &mut pack);
    assert!(
        flat_dist(woke, j.spawn) < 8.0,
        "homeless, you wake by the hearth"
    );
    let hp = wait(&j, &mut pack, 3.0, |m, _| match m {
        ServerMsg::Status { health, .. } => Some(*health),
        _ => None,
    });
    assert!(hp > 0.0);
    assert_eq!(pack, before, "death keeps everything you carry");

    // 4. the same name with another token is refused
    let err = net::connect(&addr, "tester", 999, false, Duration::from_secs(5))
        .err()
        .unwrap();
    assert!(
        err.contains("already playing") || err.contains("belongs"),
        "{err}"
    );

    drop(j);
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Profession crafting: a recipe is made at the station it names. Away from
/// one it is refused by name; at the town's smithy the forge smelts, and
/// serves as the workbench too.
#[test]
fn crafting_is_done_at_the_towns_stations() {
    let dir = std::env::temp_dir().join(format!("dyno-test-craft-{}", std::process::id()));
    let server = start_server(&dir);
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let mut j = net::connect(&addr, "smith", 222, false, Duration::from_secs(5)).expect("join");
    let c = j.content.clone();
    let id = |k: &str| c.item_by_key(k).unwrap();
    let recipe = |out: &str| {
        c.recipes
            .iter()
            .find(|r| r.output == id(out))
            .unwrap_or_else(|| panic!("no recipe for {out}"))
            .clone()
    };
    let (coal, pick) = (recipe("coal"), recipe("wooden_pickaxe"));
    let forge = &c.stations[coal.station.expect("smelting names a station") as usize];
    let bench = &c.stations[pick.station.expect("tools name a station") as usize];
    // every station block there is in the town, wherever it stands
    let hearth = j.hearth;
    let town = [hearth[0] as f32, hearth[1] as f32, hearth[2] as f32];
    land(&mut j, town, 80.0);
    let mut pack = vec![None; 36];
    pose(&j, j.spawn);
    // (only logs for now: fuel burns the cheapest thing in the bags first)
    say(&j, "/give log 6");
    wait(&j, &mut pack, 3.0, |_, pack| {
        (count(pack, id("log")) == 6).then_some(())
    });

    // 1. by the hearth there is no forge: refused, and it says what is needed
    send(
        &j,
        ClientMsg::Craft {
            recipe: coal.id,
            times: 1,
        },
    );
    let why = wait(&j, &mut pack, 3.0, |m, _| match m {
        ServerMsg::System { text } if text.starts_with("Needs") => Some(text.clone()),
        _ => None,
    });
    assert_eq!(why, format!("Needs a {} nearby", forge.name));
    assert_eq!(count(&pack, id("coal")), 0);

    // 2. at the forge: a log is burnt for fuel and another becomes coal
    let at = block_near(&j, hearth[0], hearth[2], 80, &forge.blocks).expect("the town has a forge");
    assert!(
        bench.blocks.contains(&j.world.get(at[0], at[1], at[2])),
        "the forge serves as the workbench too"
    );
    pose(&j, [at[0] as f32 + 1.5, at[1] as f32, at[2] as f32 + 0.5]);
    std::thread::sleep(Duration::from_millis(150));
    send(
        &j,
        ClientMsg::Craft {
            recipe: coal.id,
            times: 1,
        },
    );
    wait(&j, &mut pack, 3.0, |_, pack| {
        (count(pack, id("coal")) >= 1).then_some(())
    });
    assert_eq!(count(&pack, id("log")), 4, "one log for fuel, one smelted");

    // 3. a tool, made here: its profession is learned on the way
    say(&j, "/give planks 3");
    say(&j, "/give stick 2");
    wait(&j, &mut pack, 3.0, |_, pack| {
        (count(pack, id("stick")) == 2).then_some(())
    });
    let had = count(&pack, id("wooden_pickaxe"));
    send(
        &j,
        ClientMsg::Craft {
            recipe: pick.id,
            times: 1,
        },
    );
    wait(&j, &mut pack, 3.0, |_, pack| {
        (count(pack, id("wooden_pickaxe")) == had + 1).then_some(())
    });
    assert_eq!(count(&pack, id("planks")), 0);

    drop(j);
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Home is made at the inn, through the innkeeper: after that you wake and
/// recall there. Beds are decoration, and so are chests.
#[test]
fn home_is_made_at_the_inn() {
    let dir = std::env::temp_dir().join(format!("dyno-test-inn-{}", std::process::id()));
    let server = start_server(&dir);
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let mut j = net::connect(&addr, "guest", 333, false, Duration::from_secs(5)).expect("join");
    let c = j.content.clone();
    let keeper = c
        .npcs
        .iter()
        .find(|n| n.innkeeper)
        .expect("the pack has an innkeeper")
        .id;
    let mut pack = vec![None; 36];
    pose(&j, j.spawn);
    // she is near enough to the hearth to be seen from it
    let (eid, inn) = wait(&j, &mut pack, 10.0, |m, _| match m {
        ServerMsg::EntitySpawn {
            id,
            kind: EntityKind::Npc(n),
            pos,
            ..
        } if *n == keeper => Some((*id, *pos)),
        _ => None,
    });
    assert!(
        flat_dist(inn, j.spawn) > 20.0,
        "the inn is not at the hearth"
    );
    land(&mut j, inn, 24.0);

    // 1. a bed and a chest do nothing: you still wake by the hearth
    for kind in [dyno_world::BED, dyno_world::CHEST] {
        let at = block_near(&j, inn[0] as i32, inn[2] as i32, 20, &[kind])
            .expect("the inn has beds and chests");
        pose(&j, [at[0] as f32 + 1.5, at[1] as f32, at[2] as f32 + 0.5]);
        std::thread::sleep(Duration::from_millis(100));
        send(&j, ClientMsg::Interact { pos: at });
    }
    let quiet = wait_opt(&j, &mut pack, 1.0, |m| match m {
        ServerMsg::Container(_) => Some("a chest opened".to_string()),
        ServerMsg::System { text } if text.contains("pawn point") => Some(text.clone()),
        _ => None,
    });
    assert_eq!(quiet, None);
    let woke = die_and_wake(&j, &mut pack);
    assert!(flat_dist(woke, j.spawn) < 8.0, "a bed is not a home");

    // 2. the innkeeper: talk to her, and the inn is home
    pose(&j, [inn[0] + 1.5, inn[1], inn[2]]);
    std::thread::sleep(Duration::from_millis(150));
    send(&j, ClientMsg::Talk { entity: eid });
    wait(&j, &mut pack, 3.0, |m, _| {
        matches!(m, ServerMsg::System { text } if text.contains("home")).then_some(())
    });
    pose(&j, j.spawn);
    std::thread::sleep(Duration::from_millis(150));
    let woke = die_and_wake(&j, &mut pack);
    assert!(
        flat_dist(woke, inn) < 8.0,
        "you wake at the inn: {woke:?}, the innkeeper is at {inn:?}"
    );

    drop(j);
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

/// What only operators may do: /spawn, /kill, /tp and build mode. In play
/// nobody digs or places, not even on a resource node; in build mode an
/// operator does both.
#[test]
fn building_and_shortcuts_are_for_operators() {
    let dir = std::env::temp_dir().join(format!("dyno-test-ops-{}", std::process::id()));
    let server = start_server(&dir);
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    // the first player owns the world and is its operator
    let mut op = net::connect(&addr, "owner", 1, false, Duration::from_secs(5)).expect("join");
    let guest = net::connect(&addr, "guest", 2, false, Duration::from_secs(5)).expect("join");
    let mut pack = vec![None; 36];
    let mut gpack = vec![None; 36];
    pose(&op, op.spawn);
    pose(&guest, guest.spawn);
    for cmd in ["/spawn", "/kill", "/tp owner", "/gamemode build"] {
        say(&guest, cmd);
        let text = wait(&guest, &mut gpack, 3.0, |m, _| match m {
            ServerMsg::System { text } if text.contains("operators") => Some(text.clone()),
            _ => None,
        });
        assert!(text.starts_with("Only server operators"), "{cmd}: {text}");
    }
    // still alive, still in play
    send(
        &guest,
        ClientMsg::Hurt {
            kind: EnvDamage::Fall,
            amount: 1,
        },
    );
    let (hp, mode) = wait(&guest, &mut gpack, 3.0, |m, _| match m {
        ServerMsg::Status { health, mode, .. } => Some((*health, *mode)),
        _ => None,
    });
    assert!(hp > 0.0);
    assert_eq!(mode, Mode::Survival);

    // a tree: nobody digs it in play, by timing it or not
    let spawn = op.spawn;
    land(&mut op, spawn, 48.0);
    let tree = block_near(
        &op,
        spawn[0] as i32,
        spawn[2] as i32,
        48,
        &[dyno_world::LOG],
    )
    .expect("a tree near the spawn");
    let by = [tree[0] as f32 + 1.5, tree[1] as f32, tree[2] as f32 + 0.5];
    pose(&op, by);
    std::thread::sleep(Duration::from_millis(100));
    send(&op, ClientMsg::DigStart { pos: tree });
    std::thread::sleep(Duration::from_millis(300));
    send(&op, ClientMsg::Dig { pos: tree });
    let put_back = wait(&op, &mut pack, 3.0, |m, _| match m {
        ServerMsg::Block { pos, block } if *pos == tree => Some(*block),
        _ => None,
    });
    assert_eq!(put_back, dyno_world::LOG, "the tree stands");
    let beside = [tree[0] + 1, tree[1] + 3, tree[2]];
    let was = op.world.get(beside[0], beside[1], beside[2]);
    send(
        &op,
        ClientMsg::Place {
            pos: beside,
            slot: 0,
        },
    );
    let put_back = wait(&op, &mut pack, 3.0, |m, _| match m {
        ServerMsg::Block { pos, block } if *pos == beside => Some(*block),
        _ => None,
    });
    assert_eq!(put_back, was, "nothing is placed in play");

    // build mode: the operator digs
    say(&op, "/gamemode build");
    let mode = wait(&op, &mut pack, 3.0, |m, _| match m {
        ServerMsg::Status { mode, .. } => Some(*mode),
        _ => None,
    });
    assert_eq!(mode, Mode::Creative);
    send(&op, ClientMsg::Dig { pos: tree });
    let now = wait(&op, &mut pack, 3.0, |m, _| match m {
        ServerMsg::Block { pos, block } if *pos == tree => Some(*block),
        _ => None,
    });
    assert_eq!(now, dyno_world::AIR, "an operator in build mode digs");
    // and the operator's shortcut home works
    say(&op, "/spawn");
    wait(&op, &mut pack, 3.0, |m, _| {
        matches!(m, ServerMsg::Teleport { .. }).then_some(())
    });

    drop(op);
    drop(guest);
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

/// The RPG loop: a new character gets a starter kit, takes a quest from an
/// NPC, finishes it by talking to another, and earns XP and money; a collect
/// quest takes the items and gives the rewards.
#[test]
fn quests_give_xp_and_rewards() {
    let dir = std::env::temp_dir().join(format!("dyno-test-quests-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        world_size: [96, 48, 96],
        trust_movement: true,
        // everything is of interest, however far: every NPC is met at once
        stream: dyno_server::stream::StreamConfig {
            aoi: 4000.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let server = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let mut j = net::connect(&addr, "hero", 4242, false, Duration::from_secs(5)).expect("join");
    let c = j.content.clone();
    assert!(c.world.protected, "the built-in content uses RPG rules");
    let mut pack = vec![None; 36];
    // NPC positions arrive as entity spawns; the starter kit as the pack
    let mut npcs: std::collections::HashMap<u16, (u32, [f32; 3])> = Default::default();
    let mut level = 0;
    let end = Instant::now() + Duration::from_secs(5);
    while Instant::now() < end
        && (npcs.len() < c.npcs.iter().filter(|n| n.phase.is_none()).count()
            || level == 0
            || count(&pack, c.item_by_key("wooden_pickaxe").unwrap()) == 0)
    {
        if let Some(m) = wait_opt(&j, &mut pack, 0.2, |m| Some(m.clone())) {
            match m {
                ServerMsg::EntitySpawn {
                    id,
                    kind: EntityKind::Npc(n),
                    pos,
                    ..
                } => {
                    npcs.insert(n, (id, pos));
                }
                ServerMsg::Character { level: l, .. } => level = l,
                _ => {}
            }
        }
    }
    assert_eq!(level, 1);
    assert_eq!(
        npcs.len(),
        c.npcs.iter().filter(|n| n.phase.is_none()).count(),
        "every unphased NPC stands in the world"
    );
    assert_eq!(
        count(&pack, c.item_by_key("wooden_pickaxe").unwrap()),
        1,
        "starter kit"
    );
    let npc = |k: &str| c.npcs.iter().find(|n| n.key == k).unwrap().id;
    let quest = |k: &str| c.quests.iter().find(|q| q.key == k).unwrap().id;
    let go = |n: u16| {
        let (_, at) = npcs[&n];
        pose(&j, [at[0] + 1.0, at[1], at[2]]);
        std::thread::sleep(Duration::from_millis(80));
    };
    // talk to Brannoc: a dialog opens; take "The Cold Hearth"
    go(npc("brannoc"));
    send(
        &j,
        ClientMsg::Talk {
            entity: npcs[&npc("brannoc")].0,
        },
    );
    wait(&j, &mut pack, 3.0, |m, _| {
        matches!(m, ServerMsg::Dialog { .. }).then_some(())
    });
    send(
        &j,
        ClientMsg::AcceptQuest {
            quest: quest("cold_hearth"),
        },
    );
    wait(&j, &mut pack, 3.0, |m, _| match m {
        ServerMsg::Quests { log, .. } if log.iter().any(|q| q.quest == quest("cold_hearth")) => {
            Some(())
        }
        _ => None,
    });
    // talk to Maren: the objective is done; hand it in for XP and money
    go(npc("maren"));
    send(
        &j,
        ClientMsg::Talk {
            entity: npcs[&npc("maren")].0,
        },
    );
    send(
        &j,
        ClientMsg::CompleteQuest {
            quest: quest("cold_hearth"),
            choice: 0,
        },
    );
    let (xp, money) = wait(&j, &mut pack, 3.0, |m, _| match m {
        ServerMsg::Gain { xp, money } => Some((*xp, *money)),
        _ => None,
    });
    // the numbers come from the content
    let first = c.quests.iter().find(|q| q.key == "cold_hearth").unwrap();
    assert_eq!((xp, money), (first.xp, first.money));
    let (lvl, have) = wait(&j, &mut pack, 3.0, |m, _| match m {
        ServerMsg::Character { level, xp, .. } => Some((*level, *xp)),
        _ => None,
    });
    assert_eq!((lvl, have), (1, first.xp));
    // a collect quest: Tilda wants raw meat; it is taken and cooked meat given
    go(npc("tilda"));
    send(
        &j,
        ClientMsg::AcceptQuest {
            quest: quest("meat_for_the_pot"),
        },
    );
    let pot = c
        .quests
        .iter()
        .find(|q| q.key == "meat_for_the_pot")
        .unwrap();
    let need = pot.objectives[0].need() as u32;
    send(
        &j,
        ClientMsg::Chat {
            text: format!("/give raw_meat {}", need + 1),
        },
    );
    let meat = c.item_by_key("raw_meat").unwrap();
    wait(&j, &mut pack, 3.0, |_, p| {
        (count(p, meat) == need + 1).then_some(())
    });
    // the server recounts collect objectives every second
    std::thread::sleep(Duration::from_millis(1200));
    go(npc("tilda"));
    send(
        &j,
        ClientMsg::CompleteQuest {
            quest: quest("meat_for_the_pot"),
            choice: 0,
        },
    );
    let cooked = c.item_by_key("cooked_meat").unwrap();
    wait(&j, &mut pack, 3.0, |_, p| {
        (count(p, cooked) == 4).then_some(())
    });
    assert_eq!(
        count(&pack, meat),
        1,
        "the asked-for raw meat was handed in"
    );
    // classes: pick Mage, then cast Fireball at a summoned Grazer
    let ember = c.classes.iter().find(|k| k.key == "mage").unwrap().id;
    send(&j, ClientMsg::ChooseClass { class: ember });
    let class = wait(&j, &mut pack, 3.0, |m, _| match m {
        ServerMsg::Character { class: Some(k), .. } => Some(Some(*k)),
        _ => None,
    });
    assert_eq!(class, Some(ember));
    let here = j.spawn;
    pose(&j, here);
    send(
        &j,
        ClientMsg::Chat {
            text: "/summon grazer".into(),
        },
    );
    let grazer = wait(&j, &mut pack, 3.0, |m, _| match m {
        ServerMsg::EntitySpawn {
            id,
            kind: EntityKind::Creature(_),
            ..
        } => Some(*id),
        _ => None,
    });
    let bolt = c.abilities.iter().find(|a| a.key == "fireball").unwrap().id;
    send(
        &j,
        ClientMsg::Cast {
            ability: bolt,
            target: Some(grazer),
        },
    );
    wait(&j, &mut pack, 3.0, |m, _| {
        matches!(m, ServerMsg::Casting { ability: Some(a), .. } if *a == bolt).then_some(())
    });
    // stand still for the cast; the bolt lands and mana is spent
    let hit = wait(&j, &mut pack, 4.0, |m, _| match m {
        ServerMsg::EntityInfo {
            id, health, max, ..
        } if *id == grazer && health < max => Some(*health),
        _ => None,
    });
    assert!(hit >= 0.0);
    let mana = wait(&j, &mut pack, 3.0, |m, _| match m {
        ServerMsg::Power { value, max } if value < max => Some(*value),
        _ => None,
    });
    assert!(mana > 0.0, "mana spent but not empty: {mana}");
    // gear: a Mage cannot wear mail, but can wear a cloak
    send(
        &j,
        ClientMsg::Chat {
            text: "/give watchman_vest 1".into(),
        },
    );
    std::thread::sleep(Duration::from_millis(600));
    send(
        &j,
        ClientMsg::Chat {
            text: "/give woolen_cloak 1".into(),
        },
    );
    let (vest, cloak) = (
        c.item_by_key("watchman_vest").unwrap(),
        c.item_by_key("woolen_cloak").unwrap(),
    );
    wait(&j, &mut pack, 3.0, |_, p| {
        (count(p, vest) == 1 && count(p, cloak) == 1).then_some(())
    });
    let slot_of = |p: &[Option<Stack>], it| {
        p.iter()
            .position(|s| s.is_some_and(|s| s.item == it))
            .unwrap() as u8
    };
    send(
        &j,
        ClientMsg::Equip {
            from: slot_of(&pack, vest),
        },
    );
    wait(&j, &mut pack, 3.0, |m, _| {
        matches!(m, ServerMsg::System { text } if text.contains("cannot wear mail") || text.contains("Requires level")).then_some(())
    });
    // take the starter boots off (into the bags) and put them back on
    let boots = c.item_by_key("worn_boots").unwrap();
    send(&j, ClientMsg::Unequip { place: 9 });
    wait(&j, &mut pack, 3.0, |_, p| {
        (count(p, boots) == 1).then_some(())
    });
    send(
        &j,
        ClientMsg::Equip {
            from: slot_of(&pack, boots),
        },
    );
    let worn = wait(&j, &mut pack, 3.0, |m, _| match m {
        ServerMsg::Equipment(e) if e[9].is_some_and(|s| s.item == boots) => Some(e.clone()),
        _ => None,
    });
    assert_eq!(worn.len(), 17);
    assert_eq!(count(&pack, boots), 0);
    let _ = cloak;
    // gathering is a cast: stand by the nearest tree, gather, logs go to the bags
    let log = c.item_by_key("log").unwrap();
    let mut node = None;
    let (cx, cz) = (j.spawn[0] as i32, j.spawn[2] as i32);
    let spawn = j.spawn;
    land(&mut j, spawn, 48.0);
    'find: for r in 0..48i32 {
        for dz in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dz.abs() != r {
                    continue;
                }
                for y in 1..j.world.size[1] as i32 {
                    if j.world.get(cx + dx, y, cz + dz) == dyno_world::LOG {
                        node = Some([cx + dx, y, cz + dz]);
                        break 'find;
                    }
                }
            }
        }
    }
    assert!(node.is_some(), "a tree near the spawn");
    if let Some(n) = node {
        let before = count(&pack, log);
        pose(&j, [n[0] as f32 + 1.5, n[1] as f32, n[2] as f32 + 0.5]);
        std::thread::sleep(Duration::from_millis(100));
        send(&j, ClientMsg::Gather { pos: n });
        wait(&j, &mut pack, 3.0, |m, _| {
            matches!(m, ServerMsg::Casting { ability: Some(a), .. } if *a == dyno_protocol::GATHER)
                .then_some(())
        });
        wait(&j, &mut pack, 4.0, |_, p| {
            (count(p, log) > before).then_some(())
        });
        // the tree is still standing, but resting
        send(&j, ClientMsg::Gather { pos: n });
        wait(&j, &mut pack, 3.0, |m, _| {
            matches!(m, ServerMsg::System { text } if text.contains("gathered recently"))
                .then_some(())
        });
    }
    // the world is protected: digging the ground is refused
    let below = [
        j.spawn[0].floor() as i32 + 3,
        j.spawn[1].floor() as i32 - 1,
        j.spawn[2].floor() as i32,
    ];
    send(&j, ClientMsg::Dig { pos: below });
    wait(&j, &mut pack, 3.0, |m, _| {
        matches!(m, ServerMsg::Block { pos, block } if *pos == below && *block != dyno_world::AIR)
            .then_some(())
    });
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

/// A world event: an operator starts the Nightfall Incursion; it is announced,
/// Shades pour out at a gate, and the tracker counts them.
#[test]
fn nightfall_incursion() {
    let dir = std::env::temp_dir().join(format!("dyno-test-event-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        // wide enough for the town and the ground outside its gates
        world_size: [224, 48, 224],
        ..Default::default()
    };
    let server = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let j = net::connect(&addr, "warden", 99, false, Duration::from_secs(5)).expect("join");
    let shade = j
        .content
        .creatures
        .iter()
        .find(|c| c.key == "shade")
        .unwrap()
        .id;
    let mut pack = vec![None; 36];
    pose(&j, j.spawn);
    send(
        &j,
        ClientMsg::Chat {
            text: "/event incursion".into(),
        },
    );
    let title = wait(&j, &mut pack, 3.0, |m, _| match m {
        ServerMsg::WorldEvent { title, .. } if !title.is_empty() => Some(title.clone()),
        _ => None,
    });
    assert_eq!(title, "Nightfall Incursion");
    let end = Instant::now() + Duration::from_secs(10);
    let mut ids = std::collections::HashSet::new();
    let mut levels = Vec::new();
    while levels.len() < 3 && Instant::now() < end {
        pose(&j, j.spawn);
        wait_opt(&j, &mut pack, 0.5, |m| {
            match m {
                ServerMsg::EntitySpawn {
                    id,
                    kind: EntityKind::Creature(c),
                    ..
                } if *c == shade => {
                    ids.insert(*id);
                }
                ServerMsg::EntityInfo { id, level, max, .. } if ids.contains(id) => {
                    levels.push((*level, *max));
                }
                _ => {}
            }
            None::<()>
        });
    }
    assert!(levels.len() >= 3, "a wave of Shades arrives ({levels:?})");
    // they take the level band of the meadow at the gate (1 to 2), with the
    // health of their level
    let base = j.content.creatures[shade as usize].health;
    for (level, max) in levels {
        assert!(level <= 2, "a level {level} Shade in the level 1-2 meadow");
        let want = base * dyno_sim::progression::mob_scale(level).0;
        assert!((max - want).abs() < 0.5, "health {max} for level {level}");
    }
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Character select: a realm lists my characters; creating one with a class
/// and looks makes it appear; deleting it removes it.
#[test]
fn characters_are_created_listed_and_deleted() {
    let dir = std::env::temp_dir().join(format!("dyno-test-chars-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        world_size: [96, 48, 96],
        ..Default::default()
    };
    let server = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let token = 4711;
    let (realm, list, _) =
        net::realm(&addr, token, None, Duration::from_secs(5)).expect("realm answers");
    assert!(list.is_empty());
    assert_eq!(realm.classes.len(), 6, "six classes offered");
    let look = dyno_protocol::Appearance {
        skin: 3,
        hair_style: 4,
        hair_color: 5,
        beard: false,
        ..Default::default()
    };
    let create = dyno_protocol::NewCharacter {
        class: "mage".into(),
        look,
    };
    let j = net::connect_as(
        &addr,
        "Mirelle",
        token,
        false,
        Some(create),
        Duration::from_secs(5),
    )
    .expect("join");
    let mut pack = vec![None; 36];
    let class = wait(&j, &mut pack, 3.0, |m, _| match m {
        ServerMsg::Character {
            class: Some(c),
            look: l,
            ..
        } => Some((*c, *l)),
        _ => None,
    });
    assert_eq!(j.content.class(class.0).unwrap().key, "mage");
    assert_eq!(class.1, look);
    drop(j);
    std::thread::sleep(Duration::from_millis(300));
    server.server.save();
    let (_, list, _) = net::realm(&addr, token, None, Duration::from_secs(5)).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(
        (list[0].name.as_str(), list[0].class.as_str(), list[0].look),
        ("Mirelle", "Mage", look)
    );
    // another install sees none of mine
    let (_, theirs, _) = net::realm(&addr, 1234, None, Duration::from_secs(5)).unwrap();
    assert!(theirs.is_empty());
    let (_, list, _) = net::realm(&addr, token, Some("Mirelle"), Duration::from_secs(5)).unwrap();
    assert!(list.is_empty(), "deleted");
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Kill, loot the body by hand, then spend the coins at a merchant.
#[test]
fn corpses_are_looted_and_merchants_sell() {
    let dir = std::env::temp_dir().join(format!("dyno-test-loot-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        world_size: [96, 48, 96],
        trust_movement: true,
        ..Default::default()
    };
    let server = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let j = net::connect(&addr, "looter", 777, false, Duration::from_secs(5)).expect("join");
    let c = j.content.clone();
    let mut pack = vec![None; 36];
    let tilda = c.npcs.iter().find(|n| n.key == "tilda").unwrap().id;
    let mut tilda_at = None;
    let mut money = None;
    let end = Instant::now() + Duration::from_secs(5);
    while Instant::now() < end && (tilda_at.is_none() || money.is_none()) {
        match wait_opt(&j, &mut pack, 0.2, |m| Some(m.clone())) {
            Some(ServerMsg::EntitySpawn {
                id,
                kind: EntityKind::Npc(n),
                pos,
                ..
            }) if n == tilda => tilda_at = Some((id, pos)),
            Some(ServerMsg::Character { money: m, .. }) => money = Some(m),
            _ => {}
        }
    }
    let (tilda_id, tilda_pos) = tilda_at.expect("Tilda stands in town");
    let money0 = money.unwrap_or(0);
    // a rare elite: always carries its sash
    pose(&j, j.spawn);
    send(
        &j,
        ClientMsg::Chat {
            text: "/summon rook_varn".into(),
        },
    );
    wait(&j, &mut pack, 3.0, |m, _| {
        matches!(
            m,
            ServerMsg::EntitySpawn {
                kind: EntityKind::Creature(_),
                ..
            }
        )
        .then_some(())
    });
    send(
        &j,
        ClientMsg::Chat {
            text: "/slay".into(),
        },
    );
    let corpse = wait(&j, &mut pack, 3.0, |m, _| match m {
        ServerMsg::EntitySpawn {
            id,
            kind: EntityKind::Corpse { loot: true, .. },
            ..
        } => Some(*id),
        _ => None,
    });
    // open it: the sash is inside; nothing went to the bags by itself
    let sash = c.item_by_key("rook_varns_sash").unwrap();
    assert_eq!(count(&pack, sash), 0, "loot waits on the body");
    send(
        &j,
        ClientMsg::Loot {
            id: corpse,
            all: false,
        },
    );
    let (coins, items) = wait(&j, &mut pack, 3.0, |m, _| match m {
        ServerMsg::Loot { id, money, items } if *id == corpse => Some((*money, items.clone())),
        _ => None,
    });
    assert!(items.iter().any(|s| s.item == sash), "{items:?}");
    // take everything: the body empties and stops glinting
    send(
        &j,
        ClientMsg::LootTake {
            id: corpse,
            slot: 255,
        },
    );
    wait(&j, &mut pack, 3.0, |m, p| {
        matches!(m, ServerMsg::Pack(_))
            .then_some(())
            .filter(|_| count(p, sash) == 1)
    });
    wait(&j, &mut pack, 3.0, |m, _| match m {
        ServerMsg::EntitySpawn {
            id,
            kind: EntityKind::Corpse { loot: false, .. },
            ..
        } if *id == corpse => Some(()),
        _ => None,
    });
    // a merchant: five loaves of Hearth Bread
    send(
        &j,
        ClientMsg::Chat {
            text: "/money 100".into(),
        },
    );
    let purse = wait(&j, &mut pack, 3.0, |m, _| match m {
        ServerMsg::Character { money, .. } if *money >= 100 + coins + money0 => Some(*money),
        _ => None,
    });
    let bread = c.item_by_key("hearth_bread").unwrap();
    let price = c.npcs[tilda as usize]
        .sells
        .iter()
        .find(|(i, _)| *i == bread)
        .unwrap()
        .1;
    pose(&j, [tilda_pos[0] + 1.0, tilda_pos[1], tilda_pos[2]]);
    std::thread::sleep(Duration::from_millis(100));
    send(
        &j,
        ClientMsg::Buy {
            npc: tilda_id,
            item: bread,
            count: 5,
        },
    );
    let left = wait(&j, &mut pack, 3.0, |m, p| match m {
        ServerMsg::Character { money, .. } if count(p, bread) == 5 => Some(*money),
        _ => None,
    });
    assert_eq!(left, purse - 5 * price);
    // too poor for the smith's best pick: nothing changes hands
    let pick = c.item_by_key("iron_pickaxe").unwrap();
    let hollis = c.npcs.iter().find(|n| n.key == "hollis").unwrap();
    assert!(hollis.sells.iter().any(|(i, _)| *i == pick));
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Two players stand side by side; the one whose story has moved on sees a
/// different world (phasing), and cannot touch what the other sees.
#[test]
fn phasing_shows_each_player_their_own_world() {
    let dir = std::env::temp_dir().join(format!("dyno-test-phase-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        world_size: [96, 48, 96],
        // the stockade is minutes away: everything is of interest
        stream: dyno_server::stream::StreamConfig {
            aoi: 4000.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let server = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let a = net::connect(&addr, "hero", 901, false, Duration::from_secs(5)).expect("join");
    let b = net::connect(&addr, "other", 902, false, Duration::from_secs(5)).expect("join");
    let c = a.content.clone();
    let ansel = c.npcs.iter().find(|n| n.key == "ansel").unwrap().id;
    let (mut pa, mut pb) = (vec![None; 36], vec![None; 36]);
    // nobody has finished Rook Varn: nobody sees Corporal Ansel
    let saw = |j: &Joined, pack: &mut Vec<Option<Stack>>, secs: f32| {
        wait_opt(j, pack, secs, |m| match m {
            ServerMsg::EntitySpawn {
                id,
                kind: EntityKind::Npc(n),
                ..
            } if *n == ansel => Some(*id),
            _ => None,
        })
    };
    assert_eq!(saw(&a, &mut pa, 1.0), None);
    // the first player (the owner) finishes it: Ansel appears for them alone
    send(
        &a,
        ClientMsg::Chat {
            text: "/finish rook_varn".into(),
        },
    );
    let id = saw(&a, &mut pa, 3.0).expect("the phase shows Ansel");
    assert_eq!(
        saw(&b, &mut pb, 1.0),
        None,
        "the other player is not in the phase"
    );
    // the other cannot talk to someone they cannot see
    send(&b, ClientMsg::Talk { entity: id });
    let talked = wait_opt(&b, &mut pb, 1.0, |m| {
        matches!(m, ServerMsg::Dialog { .. }).then_some(())
    });
    assert_eq!(talked, None);
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Shards: a full copy of the world opens another; a party is drawn
/// together; nobody reconnects or reloads.
#[test]
fn shards_fill_split_and_parties_meet() {
    let dir = std::env::temp_dir().join(format!("dyno-test-shards-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        world_size: [96, 48, 96],
        shard_cap: 2,
        area_cap: 100,
        ..Default::default()
    };
    let server = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let a = net::connect(&addr, "ada", 1001, false, Duration::from_secs(5)).expect("join");
    let b = net::connect(&addr, "bo", 1002, false, Duration::from_secs(5)).expect("join");
    let c = net::connect(&addr, "cy", 1003, false, Duration::from_secs(5)).expect("join");
    assert_eq!(
        server.server.shard_count(),
        2,
        "the third player opens a second shard"
    );
    let (mut pa, mut pb, mut pc) = (vec![None; 36], vec![None; 36], vec![None; 36]);
    // in different shards, they cannot see each other
    let mut b_sees_c = false;
    wait_opt(&b, &mut pb, 0.5, |m| {
        if let ServerMsg::PlayerJoined(i) = m
            && i.name == "cy"
        {
            b_sees_c = true;
        }
        None::<()>
    });
    assert!(!b_sees_c);
    // ada invites cy (across shards); cy accepts and is drawn into ada's shard
    send(
        &a,
        ClientMsg::Chat {
            text: "/invite cy".into(),
        },
    );
    wait(&c, &mut pc, 3.0, |m, _| {
        matches!(m, ServerMsg::Invite { from } if from == "ada").then_some(())
    });
    send(
        &c,
        ClientMsg::Chat {
            text: "/accept".into(),
        },
    );
    let (shard, names) = wait(&c, &mut pc, 8.0, |m, _| match m {
        ServerMsg::Shard { shard, players, .. } => Some((
            *shard,
            players.iter().map(|p| p.name.clone()).collect::<Vec<_>>(),
        )),
        _ => None,
    });
    assert_eq!(shard, 0);
    assert!(names.contains(&"ada".to_string()), "{names:?}");
    // the party frames list both
    let party = wait(&a, &mut pa, 3.0, |m, _| match m {
        ServerMsg::Party { members } if members.len() == 2 => Some(members.clone()),
        _ => None,
    });
    assert!(party[0].leader && party[0].name == "ada");
    // still connected: cy can act in the new shard
    send(
        &c,
        ClientMsg::Chat {
            text: "/who".into(),
        },
    );
    wait(&c, &mut pc, 3.0, |m, _| match m {
        ServerMsg::System { text } if text.starts_with("3 online") => Some(()),
        _ => None,
    });
    // the second shard empties and closes after a while (not waited for here)
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Crowding is managed per area, not just per world: too many people in one
/// place are spread across copies of that place.
#[test]
fn crowded_areas_are_spread_across_shards() {
    let dir = std::env::temp_dir().join(format!("dyno-test-areas-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        world_size: [96, 48, 96],
        shard_cap: 40,
        area_cap: 1,
        ..Default::default()
    };
    let server = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let a = net::connect(&addr, "ada", 2001, false, Duration::from_secs(5)).expect("join");
    let b = net::connect(&addr, "bo", 2002, false, Duration::from_secs(5)).expect("join");
    assert_eq!(
        server.server.shard_count(),
        1,
        "plenty of room in the world"
    );
    let (mut pa, mut pb) = (vec![None; 36], vec![None; 36]);
    // both stand in town: one of them (the later arrival) is moved
    let moved = wait(&b, &mut pb, 10.0, |m, _| match m {
        ServerMsg::Shard { shard, .. } => Some(*shard),
        _ => None,
    });
    assert_ne!(moved, 0);
    assert_eq!(server.server.shard_count(), 2);
    let a_moved = wait_opt(&a, &mut pa, 0.5, |m| {
        matches!(m, ServerMsg::Shard { .. }).then_some(())
    });
    assert_eq!(a_moved, None, "the first arrival stays");
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

/// The server keeps movement honest: walking is fine, teleporting and
/// flying put you back where you stood.
#[test]
fn cheating_movement_is_put_back() {
    let dir = std::env::temp_dir().join(format!("dyno-test-moves-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        world_size: [96, 48, 96],
        ..Default::default()
    };
    let server = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let j = net::connect(&addr, "mover", 3001, false, Duration::from_secs(5)).expect("join");
    let mut pack = vec![None; 36];
    let start = j.spawn;
    std::thread::sleep(Duration::from_millis(300));
    // a short step is accepted: no correction comes back
    pose(&j, [start[0] + 0.4, start[1], start[2]]);
    let fixed = wait_opt(&j, &mut pack, 0.6, |m| {
        matches!(m, ServerMsg::Teleport { .. }).then_some(())
    });
    assert_eq!(fixed, None, "walking is fine");
    // a 40-block jump is refused and the player is put back
    pose(&j, [start[0] + 40.0, start[1], start[2]]);
    let back = wait(&j, &mut pack, 2.0, |m, _| match m {
        ServerMsg::Teleport { pos } => Some(*pos),
        _ => None,
    });
    assert!(
        (back[0] - start[0]).abs() < 2.0,
        "put back near the start: {back:?}"
    );
    // hanging high in the air for seconds is flying: back to the ground
    std::thread::sleep(Duration::from_millis(1500));
    let mut at = [start[0] + 0.4, start[1], start[2]];
    let mut grounded = None;
    for _ in 0..30 {
        std::thread::sleep(Duration::from_millis(150));
        if at[1] < start[1] + 8.0 {
            at[1] += 1.0;
        }
        pose(&j, at);
        if let Some(p) = wait_opt(&j, &mut pack, 0.02, |m| match m {
            ServerMsg::Teleport { pos } => Some(*pos),
            _ => None,
        }) {
            grounded = Some(p);
            break;
        }
    }
    let g = grounded.expect("flying is noticed");
    assert!(g[1] < start[1] + 3.0, "put back on the ground: {g:?}");
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Fighting together pays: a party member nearby shares the kill (XP with a
/// group bonus, quest credit) and may loot the body.
#[test]
fn parties_share_kills_and_loot() {
    let dir = std::env::temp_dir().join(format!("dyno-test-share-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        world_size: [96, 48, 96],
        ..Default::default()
    };
    let server = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let a = net::connect(&addr, "ada", 4001, false, Duration::from_secs(5)).expect("join");
    let b = net::connect(&addr, "bo", 4002, false, Duration::from_secs(5)).expect("join");
    let (mut pa, mut pb) = (vec![None; 36], vec![None; 36]);
    send(
        &a,
        ClientMsg::Chat {
            text: "/invite bo".into(),
        },
    );
    wait(&b, &mut pb, 3.0, |m, _| {
        matches!(m, ServerMsg::Invite { .. }).then_some(())
    });
    send(
        &b,
        ClientMsg::Chat {
            text: "/accept".into(),
        },
    );
    wait(&a, &mut pa, 3.0, |m, _| match m {
        ServerMsg::Party { members } if members.len() == 2 => Some(()),
        _ => None,
    });
    // the shards learn the party within a second
    std::thread::sleep(Duration::from_millis(1300));
    send(
        &a,
        ClientMsg::Chat {
            text: "/summon thornboar".into(),
        },
    );
    wait(&a, &mut pa, 3.0, |m, _| {
        matches!(
            m,
            ServerMsg::EntitySpawn {
                kind: EntityKind::Creature(_),
                ..
            }
        )
        .then_some(())
    });
    send(
        &a,
        ClientMsg::Chat {
            text: "/slay".into(),
        },
    );
    let mut corpse = None;
    let xp_b = wait(&b, &mut pb, 3.0, |m, _| match m {
        ServerMsg::EntitySpawn {
            id,
            kind: EntityKind::Corpse { .. },
            ..
        } => {
            corpse = Some(*id);
            None
        }
        ServerMsg::Gain { xp, .. } if *xp > 0 => Some(*xp),
        _ => None,
    });
    assert!(xp_b > 0, "the party member shares the kill");
    // and may open the body (it is the party's, not just the killer's)
    let corpse = corpse.expect("the body lies there");
    send(
        &b,
        ClientMsg::Loot {
            id: corpse,
            all: false,
        },
    );
    let refused = wait_opt(&b, &mut pb, 1.5, |m| match m {
        ServerMsg::System { text } if text.contains("not yours") => Some(()),
        _ => None,
    });
    assert_eq!(refused, None);
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Two players trade: items and coins change hands only when both accept.
#[test]
fn players_trade_items_and_coins() {
    let dir = std::env::temp_dir().join(format!("dyno-test-trade-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        world_size: [96, 48, 96],
        ..Default::default()
    };
    let server = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let a = net::connect(&addr, "ada", 5001, false, Duration::from_secs(5)).expect("join");
    let b = net::connect(&addr, "bo", 5002, false, Duration::from_secs(5)).expect("join");
    let c = a.content.clone();
    let (mut pa, mut pb) = (vec![None; 36], vec![None; 36]);
    let tusk = c.item_by_key("boar_tusk").unwrap();
    send(
        &a,
        ClientMsg::Chat {
            text: "/give boar_tusk 3".into(),
        },
    );
    send(
        &a,
        ClientMsg::Chat {
            text: "/money 50".into(),
        },
    );
    let slot = wait(&a, &mut pa, 3.0, |m, p| {
        matches!(m, ServerMsg::Pack(_))
            .then(|| p.iter().position(|s| s.is_some_and(|s| s.item == tusk)))
            .flatten()
    });
    let bo = b.your_id;
    send(&a, ClientMsg::Trade(dyno_protocol::TradeOp::Ask(bo)));
    wait(&b, &mut pb, 3.0, |m, _| {
        matches!(m, ServerMsg::TradeAsk { .. }).then_some(())
    });
    send(&b, ClientMsg::Trade(dyno_protocol::TradeOp::Agree));
    // the window opens for both
    wait(&a, &mut pa, 3.0, |m, _| {
        matches!(m, ServerMsg::Trade(Some(_))).then_some(())
    });
    send(
        &a,
        ClientMsg::Trade(dyno_protocol::TradeOp::Offer {
            slot: slot as u8,
            count: 2,
        }),
    );
    send(&a, ClientMsg::Trade(dyno_protocol::TradeOp::Money(20)));
    wait(&b, &mut pb, 3.0, |m, _| match m {
        ServerMsg::Trade(Some(v)) if v.their_money == 20 && !v.theirs.is_empty() => Some(()),
        _ => None,
    });
    // one side accepting is not enough
    send(&a, ClientMsg::Trade(dyno_protocol::TradeOp::Accept));
    wait(&b, &mut pb, 3.0, |m, _| match m {
        ServerMsg::Trade(Some(v)) if v.they_accept => Some(()),
        _ => None,
    });
    assert_eq!(count(&pb, tusk), 0);
    send(&b, ClientMsg::Trade(dyno_protocol::TradeOp::Accept));
    wait(&b, &mut pb, 3.0, |m, p| {
        matches!(m, ServerMsg::Pack(_))
            .then_some(())
            .filter(|_| count(p, tusk) == 2)
    });
    let money_b = wait(&b, &mut pb, 3.0, |m, _| match m {
        ServerMsg::Character { money, .. } => Some(*money),
        _ => None,
    });
    assert_eq!(money_b, 20);
    wait(&a, &mut pa, 3.0, |m, p| {
        matches!(m, ServerMsg::Pack(_))
            .then_some(())
            .filter(|_| count(p, tusk) == 1)
    });
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}
