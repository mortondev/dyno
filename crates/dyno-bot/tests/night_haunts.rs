//! The night in a world of areas (`world.dark_in_areas`, `Area::dark` and a
//! creature's `spares_below` in the rules): Shades rise in the Dimwood by
//! the North Watch, not round a traveller on the road there, and they leave
//! a new character alone until struck.

use dyno_bot::agent::{Agent, Outcome, Target, Task};
use dyno_bot::body::{Body, TICK};
use std::time::{Duration, Instant};

fn run_until(a: &mut Agent, secs: f32, mut done: impl FnMut(&mut Agent) -> bool) -> bool {
    let end = Instant::now() + Duration::from_secs_f32(secs * time_scale());
    while Instant::now() < end {
        assert!(a.update(), "disconnected: {:?}", a.body.closed);
        std::thread::sleep(Duration::from_secs_f32(TICK));
        if done(a) {
            return true;
        }
    }
    false
}

fn walk(a: &mut Agent, to: Target, range: f32, secs: f32) {
    a.last = None;
    a.set(Task::Goto {
        to,
        range,
        why: "test".into(),
    });
    assert!(
        run_until(a, secs, |a| a.task.is_none()),
        "still walking after {secs} s"
    );
    assert!(
        matches!(a.last, Some((_, Outcome::Done(_)))),
        "the walk ended: {:?}",
        a.last
    );
}

#[test]
fn shades_keep_to_their_haunts_and_spare_new_characters() {
    let dir = std::env::temp_dir().join(format!("dyno-bot-night-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        ops: vec!["Nightwalker".into()],
        ..Default::default()
    };
    let server = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    let mut a = Agent::new(
        Body::join(&addr, "Nightwalker", Some("warrior"), Default::default()).expect("join"),
        "test",
    );
    run_until(&mut a, 3.0, |a| a.body.class.is_some());
    let c = a.body.content.clone();
    assert!(
        c.world.dark_in_areas,
        "the pack keeps the dark to its haunts"
    );
    let shade = c.creatures.iter().find(|x| x.key == "shade").unwrap();
    assert!(
        shade.spares_below > a.body.level,
        "Shades spare a new character"
    );
    let wood = c
        .areas
        .iter()
        .find(|ar| ar.dark && ar.levels[0] <= 3)
        .expect("a haunt for the first levels");
    let holt = c.npcs.iter().find(|n| n.key == "holt").unwrap().id;
    // up the north road in the dark: nothing rises round us on the way
    a.body.say("/time night");
    walk(&mut a, Target::Npc(holt), 5.0, 150.0);
    let on_the_road = a
        .body
        .creatures()
        .filter(|e| e.creature() == Some(shade.id))
        .count();
    assert!(a.body.is_night(), "it is still night at the North Watch");
    // (an incursion's Shades gather at a gate of the town, far behind us)
    assert_eq!(on_the_road, 0, "no Shade at the North Watch");
    // into the haunt
    let centre = a.body.rel2(wood.at);
    walk(&mut a, Target::Pos(centre), 6.0, 90.0);
    a.body.say("/time night");
    let mut seen = None;
    assert!(
        run_until(&mut a, 60.0, |a| {
            seen = a
                .body
                .creatures()
                .find(|e| e.creature() == Some(shade.id))
                .map(|e| e.id);
            seen.is_some()
        }),
        "a Shade rises in {}",
        wood.name
    );
    // stand beside it: it lets a level 1 be
    let id = seen.unwrap();
    walk(&mut a, Target::Ent(id), 2.5, 30.0);
    let full = a.body.health;
    run_until(&mut a, 12.0, |_| false);
    assert!(
        a.body.health >= full,
        "the Shade left us alone (health {} of {full})",
        a.body.health
    );
    assert!(a.tel.deaths.is_empty());
    server.stop();
}

/// Real-time waits stretch on slow machines: `DYNO_TEST_TIME_SCALE` (CI sets 3).
fn time_scale() -> f32 {
    std::env::var("DYNO_TEST_TIME_SCALE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1.0)
}
