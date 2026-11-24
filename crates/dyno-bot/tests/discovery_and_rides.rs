//! Walking into a named area for the first time pays XP, once; and a
//! carter only drives you to places you have already found on foot.

use dyno_bot::agent::{Agent, Deal, Target, Task};
use dyno_bot::body::{Body, TICK};
use std::time::{Duration, Instant};

fn run_until(a: &mut Agent, secs: f32, mut done: impl FnMut(&Agent) -> bool) -> bool {
    let end = Instant::now() + Duration::from_secs_f32(secs);
    while Instant::now() < end {
        assert!(a.update(), "disconnected: {:?}", a.body.closed);
        std::thread::sleep(Duration::from_secs_f32(TICK));
        if done(a) {
            return true;
        }
    }
    false
}

#[test]
fn discovery_pays_once_and_rides_need_it() {
    let dir = std::env::temp_dir().join(format!("dyno-bot-rides-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        ops: vec!["Walker".into()],
        ..Default::default()
    };
    let server = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let mut a = Agent::new(
        Body::join(&addr, "Walker", Some("warrior"), Default::default()).expect("join"),
        "test",
    );
    run_until(&mut a, 3.0, |a| a.body.class.is_some());
    let c = a.body.content.clone();
    let [base, per] = c.world.discovery_xp.expect("the pack pays for discovery");
    // waking up in the town is not a discovery
    assert_eq!(a.body.xp, 0);

    // a carter will not drive to a place you have not found
    a.body.say("/money 500");
    // the carter nearest the hearth, and the first ride on the cart (to
    // somewhere a new character has not been)
    let is_ride = |i: u16| {
        c.items
            .iter()
            .find(|x| x.id == i)
            .is_some_and(|x| x.ride.as_ref().is_some_and(|r| r.area.is_some()))
    };
    let carter = c
        .npcs
        .iter()
        .filter(|n| n.sells.iter().any(|(i, _)| is_ride(*i)))
        .min_by_key(|n| n.at[0].abs() + n.at[2].abs())
        .expect("a carter in the pack");
    let ride = carter
        .sells
        .iter()
        .map(|(i, _)| *i)
        .find(|i| is_ride(*i))
        .unwrap();
    assert!(run_until(&mut a, 5.0, |a| a.body.money == 500));
    let before = a.body.pos();
    a.set(Task::talk(carter.id, vec![Deal::Buy(ride, 1)]));
    run_until(&mut a, 30.0, |a| a.task.is_none());
    run_until(&mut a, 1.0, |_| false);
    assert_eq!(a.body.money, 500, "an unknown place costs nothing");
    assert!(a.body.pos().distance(before) < 80.0, "and you stay in town");

    // out of the town into the meadow: found, and paid for
    let home = a.body.pos();
    let meadow = c.areas.iter().find(|x| x.key == "hearthmeadow").unwrap();
    let town = &c.areas[0];
    let out = home + glam::Vec3::new(town.radius + 14.0, 0.0, 0.0);
    let want = base + per * meadow.levels[0] as u32;
    for round in 0..2 {
        a.set(Task::Goto {
            to: Target::Pos(out),
            range: 3.0,
            why: "out".into(),
        });
        assert!(
            run_until(&mut a, 60.0, |a| a.task.is_none()),
            "walked out of town"
        );
        run_until(&mut a, 1.0, |_| false);
        assert_eq!(a.body.xp, want, "round {round}: the meadow pays once");
        a.set(Task::Goto {
            to: Target::Pos(home),
            range: 3.0,
            why: "home".into(),
        });
        assert!(run_until(&mut a, 60.0, |a| a.task.is_none()));
    }
    drop(server);
    let _ = std::fs::remove_dir_all(&dir);
}
