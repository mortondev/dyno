//! In a party, quest items drop for each member who needs one: two bots
//! group up, both take "Meat for the Pot", the leader kills grazers, and the
//! follower gets raw meat into its bags without looting anything.

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
    let end = Instant::now() + Duration::from_secs_f32(secs * time_scale());
    while Instant::now() < end {
        tick(agents);
        if done(agents) {
            return true;
        }
    }
    false
}

#[test]
fn party_members_each_get_their_quest_drops() {
    let dir = std::env::temp_dir().join(format!("dyno-bot-party-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        ops: vec!["Leader".into()],
        ..Default::default()
    };
    let server = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let mut l = Agent::new(
        Body::join(&addr, "Leader", Some("warrior"), Default::default()).expect("join"),
        "test",
    );
    let mut f = Agent::new(
        Body::join(&addr, "Follower", Some("priest"), Default::default()).expect("join"),
        "test",
    );
    run_until(&mut [&mut l, &mut f], 3.0, |a| {
        a.iter().all(|a| a.body.class.is_some())
    });
    // group up
    l.body.say("/invite Follower");
    assert!(run_until(&mut [&mut l, &mut f], 5.0, |a| a[1]
        .body
        .invite
        .is_some()));
    f.body.say("/accept");
    assert!(run_until(&mut [&mut l, &mut f], 5.0, |a| a[1]
        .body
        .party
        .len()
        == 2));
    // both take Meat for the Pot from Tilda
    let c = l.body.content.clone();
    let q = c
        .quests
        .iter()
        .find(|q| q.key == "meat_for_the_pot")
        .unwrap();
    let (quest, tilda) = (q.id, q.giver);
    l.set(Task::talk(tilda, vec![Deal::Accept(quest)]));
    f.set(Task::talk(tilda, vec![Deal::Accept(quest)]));
    assert!(run_until(&mut [&mut l, &mut f], 40.0, |a| {
        a.iter()
            .all(|a| a.body.quests.iter().any(|x| x.quest == quest))
    }));
    // the leader summons and slays grazers; the follower never loots
    let meat = c.item_by_key("raw_meat").unwrap();
    let mut got = false;
    for _ in 0..12 {
        l.body.say("/summon grazer");
        run_until(&mut [&mut l, &mut f], 0.6, |_| false);
        l.body.say("/slay");
        if run_until(&mut [&mut l, &mut f], 1.5, |a| a[1].body.count(meat) > 0) {
            got = true;
            break;
        }
    }
    assert!(got, "the follower got raw meat of its own");
    assert!(f.tel.loots > 0, "and was told: {:?}", f.log);
    server.stop();
}

/// Real-time waits stretch on slow machines: `DYNO_TEST_TIME_SCALE` (CI sets 3).
fn time_scale() -> f32 {
    std::env::var("DYNO_TEST_TIME_SCALE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1.0)
}
