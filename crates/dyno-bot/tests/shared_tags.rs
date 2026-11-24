//! Shared tags in the open world (`world.shared_tags` in the rules): two
//! players who are not in a party fight the same Grazers. Whoever hits
//! second used to get nothing; now both get the kill for "Thinning the
//! Herd", their own XP, and their own meat for "Meat for the Pot".

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
    assert!(
        c.world.shared_tags.is_some(),
        "the pack's rules share tags in the open world"
    );
    let quest = |key: &str| c.quests.iter().find(|q| q.key == key).unwrap();
    let (herd, meat_quest) = (quest("thin_the_herd"), quest("meat_for_the_pot"));
    // both may take Thinning the Herd (it follows The Cold Hearth), and take both quests
    for x in [&mut a, &mut b] {
        x.body.say("/finish cold_hearth");
    }
    run_until(&mut [&mut a, &mut b], 1.0, |_| false);
    a.set(Task::talk(herd.giver, vec![Deal::Accept(herd.id)]));
    b.set(Task::talk(herd.giver, vec![Deal::Accept(herd.id)]));
    assert!(run_until(&mut [&mut a, &mut b], 40.0, |a| {
        a.iter()
            .all(|a| a.body.quests.iter().any(|x| x.quest == herd.id))
    }));
    a.set(Task::talk(
        meat_quest.giver,
        vec![Deal::Accept(meat_quest.id)],
    ));
    b.set(Task::talk(
        meat_quest.giver,
        vec![Deal::Accept(meat_quest.id)],
    ));
    assert!(run_until(&mut [&mut a, &mut b], 40.0, |a| {
        a.iter()
            .all(|a| a.body.quests.iter().any(|x| x.quest == meat_quest.id))
    }));
    assert!(a.body.party.is_empty() && b.body.party.is_empty());
    let grazer = c.creatures.iter().find(|x| x.key == "grazer").unwrap().id;
    let count = |x: &Agent| {
        x.body
            .quests
            .iter()
            .find(|q| q.quest == herd.id)
            .map_or(0, |q| q.counts[0])
    };
    // six Grazers, one at a time, both players on each
    for _ in 0..6 {
        a.body.say("/summon grazer");
        assert!(run_until(&mut [&mut a, &mut b], 3.0, |x| x[1]
            .body
            .creatures()
            .any(|e| e.creature() == Some(grazer))));
        let (id, level) = b
            .body
            .creatures()
            .find(|e| e.creature() == Some(grazer))
            .map(|e| (e.id, e.level))
            .unwrap();
        a.set(Task::fight(id, grazer, level));
        b.set(Task::fight(id, grazer, level));
        assert!(
            run_until(&mut [&mut a, &mut b], 40.0, |x| !x[0]
                .body
                .ents
                .contains_key(&id)),
            "the Grazer died"
        );
        run_until(&mut [&mut a, &mut b], 1.0, |_| false);
    }
    let meat = c.item_by_key("raw_meat").unwrap();
    for x in [&a, &b] {
        assert!(
            count(x) >= 4,
            "{} got {} of 6 kills for the quest (fought {}, no credit {}): {:?}",
            x.body.name,
            count(x),
            x.tel.kills,
            x.tel.kills_no_credit,
            x.log
        );
        assert!(x.tel.xp_kills > 0, "{} got kill XP", x.body.name);
        assert!(
            x.body.count(meat) > 0 || x.tel.loots > 0,
            "{} got meat of their own",
            x.body.name
        );
    }
    server.stop();
}
