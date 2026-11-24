//! Two bot faults of round 2 and the way to find a group (round 3 of the
//! first 30 minutes):
//! - a warrior with a two-handed axe and a buckler wore one, then the other,
//!   for ever: the bot scored the buckler beside the axe, the server took the
//!   axe off for it;
//! - a group quest (`Quest::group`): the server tells those who carry it
//!   about each other (`ServerMsg::Seekers`), and two bots who never met
//!   find each other, group, and kill Old Tusk together. Alone, a bot leaves
//!   him be.

use dyno_bot::agent::{Agent, Deal, Task};
use dyno_bot::body::{Body, TICK};
use dyno_bot::brain::{Personality, Scripted};
use std::time::{Duration, Instant};

fn server(name: &str, ops: &[&str]) -> (dyno_server::Handle, String) {
    let dir = std::env::temp_dir().join(format!("dyno-bot-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        ops: ops.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    };
    let server = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", server.local_addr.port());
    (server, addr)
}

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
fn a_two_handed_weapon_and_a_shield_do_not_swap_for_ever() {
    let (_server, addr) = server("gear", &["Swapper"]);
    let mut a = Agent::new(
        Body::join(&addr, "Swapper", Some("warrior"), Default::default()).expect("join"),
        "test",
    );
    assert!(run_until(&mut [&mut a], 3.0, |a| a[0].body.class.is_some()));
    for line in ["/xp 2700", "/give watch_greataxe", "/give battered_buckler"] {
        a.body.say(line);
        run_until(&mut [&mut a], 0.5, |_| false);
    }
    let c = a.body.content.clone();
    let axe = c.item_by_key("watch_greataxe").unwrap();
    assert!(
        run_until(&mut [&mut a], 5.0, |a| a[0].body.count(axe) > 0),
        "the axe arrives: {:?}",
        a.log
    );
    // wear upgrades as the brain does, and count what goes on
    let mut equips = 0;
    let end = Instant::now() + Duration::from_secs_f32(8.0 * time_scale());
    let mut next = Instant::now();
    while Instant::now() < end {
        if Instant::now() >= next {
            equips += a.equip_upgrades().len();
            next = Instant::now() + Duration::from_millis(400);
        }
        tick(&mut [&mut a]);
    }
    let worn = |item: u16| a.body.equip.iter().flatten().any(|s| s.item == item);
    assert!(worn(axe), "the axe is worn: {:?}", a.body.equip);
    // the axe, perhaps the buckler first: never more than a couple of changes
    assert!(equips <= 3, "{equips} things put on in 8 s: a swap loop");
    // and from here on, nothing
    let quiet = a.equip_upgrades();
    assert!(quiet.is_empty(), "still swapping: {quiet:?}");
}

#[test]
fn two_strangers_find_each_other_for_old_tusk() {
    let (_server, addr) = server("lfg", &["Ana", "Bo"]);
    let mut ana = Agent::new(
        Body::join(&addr, "Ana", Some("warrior"), Default::default()).expect("join"),
        "test",
    );
    let mut bo = Agent::new(
        Body::join(&addr, "Bo", Some("priest"), Default::default()).expect("join"),
        "test",
    );
    assert!(run_until(&mut [&mut ana, &mut bo], 3.0, |a| a
        .iter()
        .all(|a| a.body.class.is_some())));
    let c = ana.body.content.clone();
    let tusk = c.quests.iter().find(|q| q.key == "old_tusk").unwrap();
    assert!(tusk.group >= 2, "Old Tusk is a group quest in the rules");
    // level 5, everything else done: Old Tusk is all there is to do
    let mut lines = vec!["/xp 4800".to_string()];
    lines.extend(
        c.quests
            .iter()
            .filter(|q| q.key != "old_tusk")
            .map(|q| format!("/finish {}", q.key)),
    );
    for line in &lines {
        ana.body.say(line);
        bo.body.say(line);
        run_until(&mut [&mut ana, &mut bo], 0.05, |_| false);
    }
    // every other quest done before going on (on a busy machine a line can
    // land late): say again what is missing
    let others = c.quests.len() - 1;
    for _ in 0..10 {
        if run_until(&mut [&mut ana, &mut bo], 3.0, |a| {
            a.iter().all(|a| a.body.done.len() >= others)
        }) {
            break;
        }
        for a in [&mut ana, &mut bo] {
            for q in c.quests.iter().filter(|q| q.key != "old_tusk") {
                if !a.body.done.contains(&q.id) {
                    a.body.say(&format!("/finish {}", q.key));
                }
            }
        }
    }
    assert!(
        [&ana, &bo].iter().all(|a| a.body.done.len() >= others),
        "every other quest finished"
    );
    // both walk to Holt and take it
    ana.set(Task::talk(tusk.giver, vec![Deal::Accept(tusk.id)]));
    bo.set(Task::talk(tusk.giver, vec![Deal::Accept(tusk.id)]));
    assert!(
        run_until(&mut [&mut ana, &mut bo], 150.0, |a| a.iter().all(|a| a
            .body
            .quests
            .iter()
            .any(|q| q.quest == tusk.id))),
        "both take Old Tusk"
    );
    // the server tells each about the other
    assert!(
        run_until(&mut [&mut ana, &mut bo], 5.0, |a| a.iter().all(|a| a
            .body
            .seekers
            .get(&tusk.id)
            .is_some_and(|(_, l)| l.len() == 1))),
        "each hears of the other"
    );
    // the brains take it from here: they group, and kill him together
    let mut ba = Scripted::new(Personality::Completionist, 1);
    let mut bb = Scripted::new(Personality::Completionist, 2);
    let mut grouped = false;
    let end = Instant::now() + Duration::from_secs_f32(240.0 * time_scale());
    let mut killed = false;
    while Instant::now() < end {
        ba.think(&mut ana);
        bb.think(&mut bo);
        tick(&mut [&mut ana, &mut bo]);
        grouped |= ana.body.party.len() == 2;
        let has_kill = |a: &Agent| a.quest_ready(tusk.id) || a.body.done.contains(&tusk.id);
        if has_kill(&ana) && has_kill(&bo) {
            killed = true;
            break;
        }
    }
    assert!(grouped, "they never grouped");
    assert!(
        killed,
        "Old Tusk still stands: {} / {}",
        ana.doing, bo.doing
    );
}

/// Real-time waits stretch on slow machines: `DYNO_TEST_TIME_SCALE` (CI sets 3).
fn time_scale() -> f32 {
    std::env::var("DYNO_TEST_TIME_SCALE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1.0)
}
