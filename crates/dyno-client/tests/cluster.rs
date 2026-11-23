//! Many instances, one world: plays through the real protocol against three
//! in-process server instances that form a cluster.

use dyno_client::net::{self, Joined, NetEvent};
use dyno_protocol::{ClientMsg, ServerMsg, Stack};
use dyno_server::cluster::ClusterConfig;
use std::path::Path;
use std::time::{Duration, Instant};

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

struct Node {
    handle: dyno_server::Handle,
    addr: String,
    cluster_port: u16,
}

fn node(root: &Path, name: &str, seed: Option<u16>) -> Node {
    let dir = root.join(name);
    let cluster_port = free_port();
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        world_size: [96, 48, 96],
        ops: vec!["ada".into()],
        cluster: Some(ClusterConfig {
            listen: format!("127.0.0.1:{cluster_port}").parse().unwrap(),
            advertise: None,
            join: seed.map(|p| format!("127.0.0.1:{p}")),
            secret: b"a test secret of some length".to_vec(),
            dir: dir.join("cluster"),
        }),
        ..Default::default()
    };
    let handle = dyno_server::start(cfg).expect("instance starts");
    let addr = format!("127.0.0.1:{}", handle.local_addr.port());
    Node {
        handle,
        addr,
        cluster_port,
    }
}

fn three(root: &Path) -> Vec<Node> {
    let _ = std::fs::remove_dir_all(root);
    let a = node(root, "a", None);
    let b = node(root, "b", Some(a.cluster_port));
    let c = node(root, "c", Some(b.cluster_port));
    let nodes = vec![a, b, c];
    assert!(wait_for(10.0, || nodes.iter().all(|n| n
        .handle
        .cluster()
        .unwrap()
        .shared
        .machine
        .read()
        .unwrap()
        .members
        .len()
        == 3)));
    // let everyone hear everyone's load
    std::thread::sleep(Duration::from_millis(1500));
    nodes
}

fn id_of(n: &Node) -> u64 {
    n.handle.cluster().unwrap().id()
}

fn wait_for(secs: f32, mut f: impl FnMut() -> bool) -> bool {
    let until = Instant::now() + Duration::from_secs_f32(secs);
    while Instant::now() < until {
        if f() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

/// Read messages until one matches (keeping the pack up to date).
fn wait<T>(
    j: &Joined,
    pack: &mut Vec<Option<Stack>>,
    secs: f32,
    mut f: impl FnMut(&ServerMsg) -> Option<T>,
) -> Option<T> {
    let until = Instant::now() + Duration::from_secs_f32(secs);
    while Instant::now() < until {
        match j.conn.rx.recv_timeout(Duration::from_millis(50)) {
            Ok(NetEvent::Msg(m)) => {
                if let ServerMsg::Pack(p) = &m {
                    *pack = p.clone();
                }
                if let Some(t) = f(&m) {
                    return Some(t);
                }
            }
            Ok(NetEvent::Closed(_)) => return None,
            Err(_) => {}
        }
    }
    None
}

fn say(j: &Joined, text: &str) {
    j.conn
        .tx
        .send(ClientMsg::Chat { text: text.into() })
        .unwrap();
}

/// "/who" through whichever instance hosts the player; returns the count.
fn who(j: &Joined, pack: &mut Vec<Option<Stack>>) -> Option<usize> {
    say(j, "/who");
    wait(j, pack, 5.0, |m| match m {
        ServerMsg::System { text } if text.contains(" online") => {
            text.split_whitespace().next()?.parse().ok()
        }
        _ => None,
    })
}

fn count(pack: &[Option<Stack>]) -> u32 {
    pack.iter().flatten().map(|s| s.count as u32).sum()
}

/// Give items with an operator command and wait until the pack shows them.
/// Returns the new total.
fn give(j: &Joined, pack: &mut Vec<Option<Stack>>, n: u32) -> u32 {
    // settle: take whatever pack updates are already on the way
    wait(j, pack, 0.5, |_| None::<()>);
    let before = count(pack);
    say(j, &format!("/give stick {n}"));
    wait(j, pack, 5.0, |m| match m {
        ServerMsg::Pack(p) => (count(p) == before + n).then_some(()),
        _ => None,
    })
    .expect("the items arrive");
    before + n
}

fn hosted_here(n: &Node) -> usize {
    n.handle.server.player_count()
}

#[test]
fn a_player_relayed_through_one_instance_plays_on_another_and_moves() {
    let root = std::env::temp_dir().join(format!("dyno-cl-relay-{}", std::process::id()));
    let nodes = three(&root);
    let (a, b, c) = (&nodes[0], &nodes[1], &nodes[2]);
    // bo joins b directly: b hosts him
    let bo = net::connect(&b.addr, "bo", 31, false, Duration::from_secs(10)).expect("bo joins");
    assert!(wait_for(5.0, || hosted_here(b) == 1));
    std::thread::sleep(Duration::from_millis(1500));
    // ada connects to a; the busiest shard with room is on b, so a relays her there
    let ada = net::connect(&a.addr, "ada", 32, false, Duration::from_secs(10)).expect("ada joins");
    let mut pack = vec![None; 36];
    assert!(wait_for(5.0, || hosted_here(b) == 2), "ada lives on b");
    assert_eq!(hosted_here(a), 0, "a only relays");
    // she plays through the relay: commands reach b, answers come back
    assert_eq!(who(&ada, &mut pack), Some(2));
    let items = give(&ada, &mut pack, 7);
    // she is moved to c mid-session: same connection, no reload
    say(&ada, &format!("/instance {}", id_of(c)));
    let shard = wait(&ada, &mut pack, 10.0, |m| match m {
        ServerMsg::System { text } => {
            eprintln!("SYS {text}");
            None
        }
        ServerMsg::Shard { shard, .. } => Some(*shard),
        _ => None,
    });
    assert!(shard.is_some(), "a seamless move between instances");
    assert!(wait_for(5.0, || hosted_here(c) == 1 && hosted_here(b) == 1));
    // still playing, now on c, with everything she carried
    assert_eq!(who(&ada, &mut pack), Some(1), "c hosts only her");
    assert_eq!(
        give(&ada, &mut pack, 1),
        items + 1,
        "nothing lost or duplicated"
    );
    // the cluster agrees on where she is
    let cl = a.handle.cluster().unwrap();
    let lock = cl.shared.machine.read().unwrap().locks.get("ada").copied();
    assert_eq!(lock.map(|l| l.node), Some(id_of(c)));
    drop(ada);
    drop(bo);
    assert!(
        wait_for(5.0, || cl.shared.machine.read().unwrap().locks.is_empty()),
        "leaving releases the characters"
    );
    for n in &nodes {
        n.handle.kill();
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn killing_the_leader_mid_move_loses_and_duplicates_nothing() {
    let root = std::env::temp_dir().join(format!("dyno-cl-kill-{}", std::process::id()));
    let nodes = three(&root);
    // ada plays on a non-leader instance; she moves to the other non-leader
    // while the leader is killed
    let leader = nodes
        .iter()
        .position(|n| n.handle.cluster().unwrap().is_leader())
        .unwrap();
    let others: Vec<usize> = (0..3).filter(|i| *i != leader).collect();
    let (from, to) = (&nodes[others[0]], &nodes[others[1]]);
    let ada = net::connect(&from.addr, "ada", 41, false, Duration::from_secs(10)).expect("join");
    let mut pack = vec![None; 36];
    assert!(wait_for(5.0, || hosted_here(from) == 1));
    let items = give(&ada, &mut pack, 9);
    // a save under the current epoch, so the record matches what she holds
    std::thread::sleep(Duration::from_millis(10_500));
    // the leader dies just as the move starts: the move's proposal is in
    // flight during the election
    nodes[leader].handle.kill();
    say(&ada, &format!("/instance {}", id_of(to)));
    // the move either happens or is refused, but it finishes
    let outcome = wait(&ada, &mut pack, 30.0, |m| match m {
        ServerMsg::Shard { .. } => Some("moved".to_string()),
        ServerMsg::System { text } if text.starts_with("Cannot move") => Some(text.clone()),
        _ => None,
    });
    eprintln!("move outcome: {outcome:?}");
    assert!(outcome.is_some(), "the move finishes one way or the other");
    // wherever she ends up, it is exactly one place, and she still plays
    assert!(
        wait_for(5.0, || hosted_here(from) + hosted_here(to) == 1),
        "online in exactly one place"
    );
    assert!(
        who(&ada, &mut pack).is_some(),
        "still connected and playing"
    );
    assert_eq!(
        give(&ada, &mut pack, 1),
        items + 1,
        "nothing lost or duplicated"
    );
    // with a new leader, a move works again
    let here = if hosted_here(from) == 1 { from } else { to };
    let there = if hosted_here(from) == 1 { to } else { from };
    say(&ada, &format!("/instance {}", id_of(there)));
    assert!(
        wait(&ada, &mut pack, 15.0, |m| match m {
            ServerMsg::Shard { .. } => Some(()),
            ServerMsg::System { text } => {
                eprintln!("SYS2 {text}");
                None
            }
            _ => None,
        })
        .is_some(),
        "moved under the new leader"
    );
    assert!(wait_for(5.0, || hosted_here(there) == 1 && hosted_here(here) == 0));
    assert_eq!(
        give(&ada, &mut pack, 1),
        items + 2,
        "nothing lost or duplicated"
    );
    let (from, to) = (here, there);
    let _ = to;
    let cl = from.handle.cluster().unwrap();
    let holder = cl.shared.machine.read().unwrap().locks.get("ada").copied();
    assert_eq!(holder.map(|l| l.node), Some(id_of(there)));
    let _ = from;
    drop(ada);
    for n in &nodes {
        n.handle.kill();
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_cut_off_host_drops_its_players_who_can_return_elsewhere_once() {
    let root = std::env::temp_dir().join(format!("dyno-cl-cut-{}", std::process::id()));
    let nodes = three(&root);
    // ada plays on a non-leader instance
    let victim = nodes
        .iter()
        .position(|n| !n.handle.cluster().unwrap().is_leader())
        .unwrap();
    let v = &nodes[victim];
    let ada = net::connect(&v.addr, "ada", 51, false, Duration::from_secs(10)).expect("join");
    let mut pack = vec![None; 36];
    let items = give(&ada, &mut pack, 5);
    std::thread::sleep(Duration::from_millis(10_500)); // saved under her epoch
    // cut the host off from the cluster
    v.handle
        .cluster()
        .unwrap()
        .link
        .cut
        .store(true, std::sync::atomic::Ordering::SeqCst);
    // it fences itself: she is dropped with a reason
    let dropped = wait(&ada, &mut pack, 10.0, |m| match m {
        ServerMsg::Reject { reason } => Some(reason.clone()),
        _ => None,
    });
    assert!(dropped.is_some(), "the fenced host drops her");
    assert!(wait_for(3.0, || hosted_here(v) == 0));
    drop(ada);
    // until the cluster declares the host down, she cannot play anywhere
    let other = &nodes[(victim + 1) % 3];
    let early = net::connect(&other.addr, "ada", 51, false, Duration::from_secs(10));
    assert!(early.is_err(), "not online in two places");
    // after the down timer she can, with everything she had at the last save
    std::thread::sleep(Duration::from_secs(10));
    let back = net::connect(&other.addr, "ada", 51, false, Duration::from_secs(10))
        .expect("back in the world");
    let mut pack2 = vec![None; 36];
    wait(&back, &mut pack2, 5.0, |m| {
        matches!(m, ServerMsg::Pack(_)).then_some(())
    });
    assert_eq!(count(&pack2), items, "nothing lost or duplicated");
    let hosts: usize = nodes.iter().map(hosted_here).sum();
    assert_eq!(hosts, 1, "online in exactly one place");
    drop(back);
    for n in &nodes {
        n.handle.kill();
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_new_instance_takes_the_founders_world() {
    let root = std::env::temp_dir().join(format!("dyno-cl-world-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let a = node(&root, "a", None);
    let b = node(&root, "b", Some(a.cluster_port));
    let wa = std::fs::read(root.join("a/cluster/base.dynw")).unwrap();
    let wb = std::fs::read(root.join("b/cluster/base.dynw")).unwrap();
    assert_eq!(wa, wb, "the same base world, checked by hash");
    let hash = a
        .handle
        .cluster()
        .unwrap()
        .shared
        .machine
        .read()
        .unwrap()
        .world_hash
        .clone();
    assert_eq!(
        hash,
        Some(dyno_server::cluster::sha256_hex(&wa)),
        "the cluster records its hash"
    );
    // a wrong secret cannot join
    let bad = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: root.join("x/world.dynw"),
        world_size: [96, 48, 96],
        cluster: Some(ClusterConfig {
            listen: format!("127.0.0.1:{}", free_port()).parse().unwrap(),
            advertise: None,
            join: Some(format!("127.0.0.1:{}", a.cluster_port)),
            secret: b"not the right secret at all".to_vec(),
            dir: root.join("x/cluster"),
        }),
        ..Default::default()
    };
    let t = Instant::now();
    let joined = std::thread::spawn(move || dyno_server::start(bad).is_ok());
    // it waits for an answer that never comes, then gives up
    assert!(!joined.join().unwrap());
    assert!(t.elapsed() >= Duration::from_secs(20));
    a.handle.kill();
    b.handle.kill();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_instance_on_a_different_version_drains() {
    let root = std::env::temp_dir().join(format!("dyno-cl-drain-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let a = node(&root, "a", None);
    // b runs other content (as if it updated before the rest of the cluster)
    std::fs::create_dir_all(root.join("b")).unwrap();
    let newer = root.join("b/newer.json");
    let text = dyno_world::content::DEFAULT_JSON.replacen("\"version\": ", "\"version\": 1", 1);
    std::fs::write(&newer, text).unwrap();
    let cluster_port = free_port();
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: root.join("b/world.dynw"),
        world_size: [96, 48, 96],
        content_path: Some(newer),
        cluster: Some(ClusterConfig {
            listen: format!("127.0.0.1:{cluster_port}").parse().unwrap(),
            advertise: None,
            join: Some(format!("127.0.0.1:{}", a.cluster_port)),
            secret: b"a test secret of some length".to_vec(),
            dir: root.join("b/cluster"),
        }),
        ..Default::default()
    };
    let b = dyno_server::start(cfg).expect("b starts");
    let b_addr = format!("127.0.0.1:{}", b.local_addr.port());
    std::thread::sleep(Duration::from_millis(1500));
    // b takes no one new while it differs from the leader
    let refused = net::connect(&b_addr, "cy", 61, false, Duration::from_secs(10));
    let why = refused.err().unwrap_or_default();
    assert!(why.contains("updating"), "{why}");
    // a still serves, and never places anyone on b
    let ada = net::connect(&a.addr, "ada", 62, false, Duration::from_secs(10)).expect("joins a");
    assert_eq!(a.handle.server.player_count(), 1);
    drop(ada);
    a.handle.kill();
    b.kill();
    let _ = std::fs::remove_dir_all(&root);
}
