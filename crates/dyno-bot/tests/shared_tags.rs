//! Shared tags in the open world (`world.shared_tags` in the rules): two
//! players who are not in a party fight the same Thornboars. Whoever hits
//! second used to get nothing; now both get the kill for "Boar Trouble" and
//! their own XP. (Their own quest drops come by the same path as a party's:
//! see party_drops.)

use dyno_bot::agent::{Agent, Deal, Task};
use dyno_bot::body::{Body, TICK};
use std::time::{Duration, Instant};

fn tick(agents: &mut [&mut Agent]) {
    for a in agents.iter_mut() {
        assert!(a.update(), "disconnected: {:?}", a.body.closed);
    }
    std::thread::sleep(Duration::from_secs_f32(TICK));
}

fn run_until(
    agents: &mut [&mut Agent],
    secs: f32,
    mut done: impl FnMut(&[&mut Agent]) -> bool,
) -> bool {
    let end = Instant::now() + Duration::from_secs_f32(secs);
    while Instant::now() < end {
        tick(agents);
        if done(agents) {
            return true;
        }
    }
    false
}

#[test]
fn two_strangers_on_one_creature_both_get_the_kill() {
    let dir = std::env::temp_dir().join(format!("dyno-bot-tags-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        ops: vec!["First".into(), "Second".into()],
        ..Default::default()
    };
    let server = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let mut a = Agent::new(
        Body::join(&addr, "First", Some("warrior"), Default::default()).expect("join"),
        "test",
    );
    let mut b = Agent::new(
        Body::join(&addr, "Second", Some("paladin"), Default::default()).expect("join"),
        "test",
    );
    run_until(&mut [&mut a, &mut b], 3.0, |a| {
        a.iter().all(|a| a.body.class.is_some())
    });
    let c = a.body.content.clone();
    let rule = c
        .world
        .shared_tags
        .expect("the pack's rules share tags in the open world");
    assert!(rule.damage > 0.0 && rule.xp > 0.0);
    // both hold Boar Trouble (its earlier quests by decree), at level 3
    let boars = c.quests.iter().find(|q| q.key == "boar_trouble").unwrap();
    for x in [&mut a, &mut b] {
        x.body.say("/xp 1300");
        for q in &boars.requires {
            let key = &c.quest(*q).unwrap().key;
            x.body.say(&format!("/finish {key}"));
        }
    }
    run_until(&mut [&mut a, &mut b], 1.0, |_| false);
    let holt = boars.giver;
    a.set(Task::talk(holt, vec![Deal::Accept(boars.id)]));
    b.set(Task::talk(holt, vec![Deal::Accept(boars.id)]));
    assert!(
        run_until(&mut [&mut a, &mut b], 150.0, |a| {
            a.iter()
                .all(|a| a.body.quests.iter().any(|x| x.quest == boars.id))
        }),
        "both took Boar Trouble from Holt"
    );
    assert!(a.body.party.is_empty() && b.body.party.is_empty());
    let boar = c
        .creatures
        .iter()
        .find(|x| x.key == "thornboar")
        .unwrap()
        .id;
    let count = |x: &Agent| {
        x.body
            .quests
            .iter()
            .find(|q| q.quest == boars.id)
            .map_or(0, |q| q.counts[0])
    };
    // four boars, one at a time, both players on each; a round counts when
    // both landed a blow before it died
    let mut shared = 0;
    for _ in 0..4 {
        let before = [count(&a), count(&b)];
        a.body.say("/summon thornboar");
        assert!(run_until(&mut [&mut a, &mut b], 3.0, |x| x.iter().all(
            |x| x.body.creatures().any(|e| e.creature() == Some(boar))
        )));
        let me = a.body.pos();
        let (id, level) = a
            .body
            .creatures()
            .filter(|e| e.creature() == Some(boar))
            .min_by(|x, y| x.pos.distance(me).total_cmp(&y.pos.distance(me)))
            .map(|e| (e.id, e.level))
            .unwrap();
        a.set(Task::fight(id, boar, level));
        b.set(Task::fight(id, boar, level));
        let mut both = false;
        assert!(
            run_until(&mut [&mut a, &mut b], 60.0, |x| {
                both |= x
                    .iter()
                    .all(|x| x.body.ents.get(&id).is_some_and(|e| e.my_hit.is_some()));
                !x[0].body.ents.contains_key(&id)
            }),
            "the Thornboar died"
        );
        run_until(&mut [&mut a, &mut b], 1.0, |_| false);
        if both {
            shared += 1;
            assert_eq!(
                [count(&a), count(&b)],
                [before[0] + 1, before[1] + 1],
                "both fought it, both have the kill: {:?} / {:?}",
                a.log,
                b.log
            );
        }
    }
    assert!(shared >= 2, "both landed a blow on {shared} of 4 boars");
    assert!(a.tel.xp_kills > 0 && b.tel.xp_kills > 0, "both got kill XP");
    server.stop();
}
