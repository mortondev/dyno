//! Plays the survival loop through the real protocol against a real server.

use dyno_client::net::{self, Joined, NetEvent};
use dyno_protocol::{ClientMsg, EntityKind, EnvDamage, Mode, Pose, ServerMsg, SlotRef, Stack};
use dyno_world::content::Station;
use std::time::{Duration, Instant};

fn start_server(dir: &std::path::Path) -> dyno_server::Handle {
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        world_size: [48, 48, 48],
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

    // stand at spawn
    let spawn = j.spawn;
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
        (count(&pack, id("log")) == 3).then_some(())
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
        (count(&pack, id("planks")) == 12).then_some(())
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
        (count(&pack, id("stick")) == 4 && count(&pack, id("crafting_table")) == 1).then_some(())
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
        (count(&pack, id("wooden_pickaxe")) == 1).then_some(())
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
        (count(&pack, id("stick")) == 1).then_some(())
    });

    // 4. health: a fall hurts, /kill drops everything, respawn restores
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
    assert!(pack.iter().all(|s| s.is_none()), "death empties the pack");
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
