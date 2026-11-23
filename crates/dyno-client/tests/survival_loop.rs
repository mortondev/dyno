//! Plays the survival loop through the real protocol against a real server.

use dyno_client::net::{self, Joined, NetEvent};
use dyno_protocol::{ClientMsg, EntityKind, EnvDamage, Mode, Pose, ServerMsg, SlotRef, Stack};
use dyno_world::content::Station;
use std::time::{Duration, Instant};

fn start_server(dir: &std::path::Path) -> dyno_server::Handle {
    // the survival rules: the built-in content with the world unprotected
    std::fs::create_dir_all(dir).unwrap();
    let survival = dir.join("survival.json");
    std::fs::write(
        &survival,
        dyno_world::content::DEFAULT_JSON.replace("\"protected\": true", "\"protected\": false"),
    )
    .unwrap();
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        world_size: [48, 48, 48],
        content_path: Some(survival),
        ..Default::default()
    };
    dyno_server::start(cfg).expect("server starts")
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
            Err(_) => {}
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

/// A standing place on grass, a few blocks from the spawn.
fn grass_near(j: &Joined) -> [f32; 3] {
    let (sx, sz) = (j.spawn[0].floor() as i32, j.spawn[2].floor() as i32);
    for r in 6..20 {
        for (dx, dz) in [(r, 0), (-r, 0), (0, r), (0, -r)] {
            let (x, z) = (sx + dx, sz + dz);
            let y = j.world.surface(x, z);
            if y > 0 && j.world.get(x, y, z) == dyno_world::GRASS {
                return [x as f32 + 0.5, y as f32 + 1.0, z as f32 + 0.5];
            }
        }
    }
    panic!("no grass near spawn");
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
        ..Default::default()
    };
    let server = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let j = net::connect(&addr, "nightwatch", 777, false, Duration::from_secs(5)).expect("join");
    let shade = j
        .content
        .creatures
        .iter()
        .find(|c| c.key == "shade")
        .unwrap()
        .id;
    let hearth = j
        .world
        .find_near(dyno_world::HEARTH, 96, 96, 24)
        .expect("a spawn hearth");
    let mut pack = vec![None; 36];
    // out in the meadow, beyond the town (where Shades never rise)
    let here = {
        let (x, z) = (hearth[0] + 52, hearth[2]);
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
            Err(_) => {}
        }
    }
    None
}

#[test]
fn survival_loop() {
    let dir = std::env::temp_dir().join(format!("dyno-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let server = start_server(&dir);
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let j = net::connect(&addr, "tester", 12345, false, Duration::from_secs(5)).expect("join");
    assert_eq!(j.mode, Mode::Survival);
    let c = j.content.clone();
    let mut pack = vec![None; 36];
    let id = |k: &str| c.item_by_key(k).unwrap();

    // stand on grass near the spawn hearth (the hearth itself sits on a stone floor)
    let spawn = grass_near(&j);
    send(
        &j,
        ClientMsg::Pose(Pose {
            pos: spawn,
            yaw: 0.0,
            pitch: 0.0,
        }),
    );

    // 1. mine the grass under our feet by hand: it drops dirt, which we pick up
    let below = [
        spawn[0].floor() as i32,
        spawn[1].floor() as i32 - 1,
        spawn[2].floor() as i32,
    ];
    let b = j.world.get(below[0], below[1], below[2]);
    let need = dyno_sim::mining::break_time(&c, b, None).expect("breakable");
    send(&j, ClientMsg::DigStart { pos: below });
    // digging too early is refused
    send(&j, ClientMsg::Dig { pos: below });
    let refused = wait(&j, &mut pack, 3.0, |m, _pack| match m {
        ServerMsg::Block { pos, block } if *pos == below => Some(*block),
        _ => None,
    });
    assert_eq!(
        refused, b,
        "an early dig is refused and the block is sent back"
    );
    send(&j, ClientMsg::DigStart { pos: below });
    std::thread::sleep(Duration::from_secs_f32(need + 0.1));
    send(&j, ClientMsg::Dig { pos: below });
    wait(&j, &mut pack, 3.0, |m, _pack| {
        matches!(m, ServerMsg::Block { pos, block: 0 } if *pos == below).then_some(())
    });
    let dropped = wait(&j, &mut pack, 3.0, |m, _pack| match m {
        ServerMsg::EntitySpawn {
            kind: EntityKind::Item(s),
            ..
        } => Some(*s),
        _ => None,
    });
    let expected = c.rule(b).drops[0].item;
    assert_eq!(dropped.item, expected);
    // keep reporting our pose; the item falls and we pick it up
    let end = Instant::now() + Duration::from_secs(4);
    while count(&pack, expected) == 0 && Instant::now() < end {
        send(
            &j,
            ClientMsg::Pose(Pose {
                pos: [spawn[0], spawn[1] - 1.0, spawn[2]],
                yaw: 0.0,
                pitch: 0.0,
            }),
        );
        let _ = j.conn.rx.recv_timeout(Duration::from_millis(50)).map(|e| {
            if let NetEvent::Msg(ServerMsg::Pack(p)) = e {
                pack = p;
            }
        });
    }
    assert_eq!(count(&pack, expected), 1, "picked up the drop");

    // 2. crafting: logs -> planks -> table and sticks -> pickaxe at the table
    send(
        &j,
        ClientMsg::Chat {
            text: "/give log 3".into(),
        },
    );
    wait(&j, &mut pack, 3.0, |_, pack| {
        (count(pack, id("log")) == 3).then_some(())
    });
    let recipe = |out: &str, st: Station| {
        c.recipes
            .iter()
            .find(|r| r.output == id(out) && r.station == st)
            .unwrap()
            .id
    };
    send(
        &j,
        ClientMsg::Craft {
            recipe: recipe("planks", Station::None),
            times: 3,
        },
    );
    wait(&j, &mut pack, 3.0, |_, pack| {
        (count(pack, id("planks")) == 12).then_some(())
    });
    assert_eq!(count(&pack, id("log")), 0);
    send(
        &j,
        ClientMsg::Craft {
            recipe: recipe("crafting_table", Station::None),
            times: 1,
        },
    );
    send(
        &j,
        ClientMsg::Craft {
            recipe: recipe("stick", Station::None),
            times: 1,
        },
    );
    wait(&j, &mut pack, 3.0, |_, pack| {
        (count(pack, id("stick")) == 4 && count(pack, id("crafting_table")) == 1).then_some(())
    });
    assert_eq!(count(&pack, id("planks")), 12 - 4 - 2);
    // the pickaxe needs a table nearby: refused first
    send(
        &j,
        ClientMsg::Craft {
            recipe: recipe("wooden_pickaxe", Station::Table),
            times: 1,
        },
    );
    wait(&j, &mut pack, 3.0, |m, _pack| {
        matches!(m, ServerMsg::System { text } if text.contains("crafting table")).then_some(())
    });
    // place the table next to us (it uses the item)
    let slot = pack
        .iter()
        .position(|s| s.is_some_and(|s| s.item == id("crafting_table")))
        .unwrap() as u8;
    let table_pos = [below[0] + 2, below[1] + 1, below[2]];
    let ground = j.world.get(table_pos[0], table_pos[1], table_pos[2]);
    let table_pos = if ground == 0 {
        table_pos
    } else {
        [table_pos[0], table_pos[1] + 1, table_pos[2]]
    };
    send(
        &j,
        ClientMsg::Place {
            pos: table_pos,
            slot,
        },
    );
    wait(&j, &mut pack, 3.0, |m, _pack| {
        matches!(m, ServerMsg::Block { pos, block } if *pos == table_pos && *block == dyno_world::CRAFTING_TABLE).then_some(())
    });
    send(
        &j,
        ClientMsg::Craft {
            recipe: recipe("wooden_pickaxe", Station::Table),
            times: 1,
        },
    );
    wait(&j, &mut pack, 3.0, |_, pack| {
        (count(pack, id("wooden_pickaxe")) == 1).then_some(())
    });
    assert_eq!(count(&pack, id("crafting_table")), 0);
    assert_eq!(count(&pack, id("planks")), 6 - 3);

    // 3. moving and throwing items keeps counts right
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
    assert_eq!(count(&pack, id("stick")), 2);
    send(
        &j,
        ClientMsg::Throw {
            from: SlotRef::Pack(30),
            count: 0,
        },
    );
    wait(&j, &mut pack, 3.0, |_, pack| {
        (count(pack, id("stick")) == 1).then_some(())
    });

    // 4. health: a fall hurts, /kill drops all but the hotbar, respawn restores
    send(
        &j,
        ClientMsg::Hurt {
            kind: EnvDamage::Fall,
            amount: 5,
        },
    );
    let hp = wait(&j, &mut pack, 3.0, |m, _pack| match m {
        ServerMsg::Status { health, .. } => Some(*health),
        _ => None,
    });
    assert_eq!(hp, 15.0);
    send(
        &j,
        ClientMsg::Chat {
            text: "/kill".into(),
        },
    );
    wait(&j, &mut pack, 3.0, |m, _pack| {
        matches!(m, ServerMsg::Died { .. }).then_some(())
    });
    assert!(
        pack[9..].iter().all(|s| s.is_none()),
        "death drops everything but the hotbar"
    );
    assert!(pack[..9].iter().any(|s| s.is_some()), "the hotbar is kept");
    send(&j, ClientMsg::Respawn);
    let hp = wait(&j, &mut pack, 3.0, |m, _pack| match m {
        ServerMsg::Status { health, .. } => Some(*health),
        _ => None,
    });
    assert_eq!(hp, 20.0);

    // 5. the same name with another token is refused
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
        ..Default::default()
    };
    let server = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let j = net::connect(&addr, "hero", 4242, false, Duration::from_secs(5)).expect("join");
    let c = j.content.clone();
    assert!(c.world.protected, "the built-in content uses RPG rules");
    let mut pack = vec![None; 36];
    // NPC positions arrive as entity spawns; the starter kit as the pack
    let mut npcs: std::collections::HashMap<u16, (u32, [f32; 3])> = Default::default();
    let mut level = 0;
    let end = Instant::now() + Duration::from_secs(5);
    while Instant::now() < end
        && (npcs.len() < c.npcs.len()
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
    assert_eq!(npcs.len(), c.npcs.len(), "every NPC stands in the world");
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
    assert_eq!((xp, money), (120, 10));
    let (lvl, have) = wait(&j, &mut pack, 3.0, |m, _| match m {
        ServerMsg::Character { level, xp, .. } => Some((*level, *xp)),
        _ => None,
    });
    assert_eq!((lvl, have), (1, 120));
    // a collect quest: Tilda wants raw meat; it is taken and cooked meat given
    go(npc("tilda"));
    send(
        &j,
        ClientMsg::AcceptQuest {
            quest: quest("meat_for_the_pot"),
        },
    );
    send(
        &j,
        ClientMsg::Chat {
            text: "/give raw_meat 5".into(),
        },
    );
    let meat = c.item_by_key("raw_meat").unwrap();
    wait(&j, &mut pack, 3.0, |_, p| {
        (count(p, meat) == 5).then_some(())
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
    assert_eq!(count(&pack, meat), 1, "four raw meat were handed in");
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
    'find: for r in 0..48i32 {
        for dz in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dz.abs() != r {
                    continue;
                }
                for y in 1..47 {
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
        matches!(m, ServerMsg::Block { pos, .. } if *pos == below).then_some(())
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
        world_size: [128, 48, 128],
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
    let mut seen = 0;
    while seen < 3 && Instant::now() < end {
        pose(&j, j.spawn);
        if wait_opt(&j, &mut pack, 0.5, |m| {
            matches!(m, ServerMsg::EntitySpawn { kind: EntityKind::Creature(c), .. } if *c == shade)
                .then_some(())
        })
        .is_some()
        {
            seen += 1;
        }
    }
    assert!(seen >= 3, "a wave of Shades arrives ({seen})");
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
