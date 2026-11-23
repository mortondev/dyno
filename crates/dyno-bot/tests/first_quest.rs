//! A bot plays the first quest against a real in-process server: it walks to
//! Old Brannoc, accepts "The Cold Hearth", walks to Captain Maren and hands
//! it in. Then the scripted brain plays for a little while, and the report
//! is built from what it measured.

use dyno_bot::agent::{Agent, Deal, Outcome, Task};
use dyno_bot::body::{Body, TICK};
use dyno_bot::brain::{Personality, Scripted};
use std::time::{Duration, Instant};

fn run_until(a: &mut Agent, secs: f32, mut done: impl FnMut(&Agent) -> bool) -> bool {
    let end = Instant::now() + Duration::from_secs_f32(secs);
    while Instant::now() < end {
        assert!(a.update(), "disconnected: {:?}", a.body.closed);
        if done(a) {
            return true;
        }
        std::thread::sleep(Duration::from_secs_f32(TICK));
    }
    false
}

#[test]
fn bot_hands_in_the_first_quest_and_plays() {
    let dir = std::env::temp_dir().join(format!("dyno-bot-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        ops: vec!["nobody".into()],
        ..Default::default()
    };
    let server = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let body = Body::join(&addr, "Testbot", Some("warrior"), Default::default()).expect("join");
    let mut a = Agent::new(body, "test");
    run_until(&mut a, 2.0, |a| a.body.class.is_some());
    a.baseline();
    let c = &a.body.content;
    let quest = c
        .quests
        .iter()
        .find(|q| q.key == "cold_hearth")
        .unwrap()
        .clone();
    assert!(a.can_take(quest.id));

    // accept from the giver (walking there), then talk to the ender and hand in
    a.set(Task::talk(quest.giver, vec![Deal::Accept(quest.id)]));
    assert!(run_until(&mut a, 30.0, |a| a.task.is_none()));
    assert!(
        matches!(a.last, Some((_, Outcome::Done(_)))),
        "{:?}",
        a.last
    );
    assert!(run_until(&mut a, 3.0, |a| a
        .body
        .quests
        .iter()
        .any(|q| q.quest == quest.id)));
    let ender = quest.ender;
    a.set(Task::talk(ender, Vec::new()));
    assert!(run_until(&mut a, 30.0, |a| a.task.is_none()));
    assert!(
        run_until(&mut a, 3.0, |a| a.quest_ready(quest.id)),
        "talking finished the objective"
    );
    a.set(Task::talk(ender, vec![Deal::Complete(quest.id, 0)]));
    assert!(run_until(&mut a, 30.0, |a| a.body.done.contains(&quest.id)));
    assert_eq!(a.tel.quests.iter().filter(|q| q.done.is_some()).count(), 1);
    assert!(a.tel.xp_total >= 100, "quest XP: {}", a.tel.xp_total);

    // the brain takes over: it finds more quests and kills something
    let mut brain = Scripted::new(Personality::Completionist, 1);
    let end = Instant::now() + Duration::from_secs(60);
    while Instant::now() < end && a.tel.kills == 0 {
        brain.think(&mut a);
        assert!(a.update());
        std::thread::sleep(Duration::from_secs_f32(TICK));
    }
    assert!(a.tel.kills > 0, "a kill within a minute; doing {}", a.doing);
    assert!(a.tel.quests.len() >= 2, "took more quests");
    let r = dyno_bot::report::build("test", "local", vec![a.tel.clone()]);
    assert_eq!(r.aggregate.bots, 1);
    assert!(r.aggregate.median_first_kill_s.is_some());
    server.stop();
}
