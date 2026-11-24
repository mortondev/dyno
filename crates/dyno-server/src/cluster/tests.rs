use super::*;
use crate::store::PlayerData;

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Start an instance, trying a fresh port if the one picked was taken in the
/// meantime (another test can grab it between `free_port` and the bind), or the
/// join was refused while the seed settled. Returns the instance and its port.
fn start_retrying(dir: &std::path::Path, join: Option<u16>) -> (Arc<Cluster>, u16) {
    let mut last = None;
    for _ in 0..4 {
        let port = free_port();
        match Cluster::start(cfg(dir, port, join), load()) {
            Ok(c) => return (c, port),
            Err(e) => {
                last = Some(e);
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
        }
    }
    panic!(
        "the cluster instance would not start: {:?}",
        last.map(|e| e.to_string())
    );
}

fn cfg(dir: &std::path::Path, port: u16, join: Option<u16>) -> ClusterConfig {
    ClusterConfig {
        listen: format!("127.0.0.1:{port}").parse().unwrap(),
        advertise: None,
        join: join.map(|p| format!("127.0.0.1:{p}")),
        secret: b"test secret".to_vec(),
        dir: dir.to_path_buf(),
    }
}

fn load() -> Load {
    Load {
        protocol: 14,
        content: "test".into(),
        version: "0".into(),
        capacity: 10,
        ..Default::default()
    }
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

fn claim(c: &Cluster, name: &str) -> Outcome {
    let mut d = PlayerData::new([0.0; 3]);
    d.token = 5;
    c.propose(
        Cmd::Claim {
            name: name.into(),
            token: 5,
            node: c.id(),
            fresh: Box::new(d),
        },
        Duration::from_secs(5),
    )
    .unwrap_or(Outcome::Refused("timeout".into()))
}

fn three(root: &std::path::Path) -> [Arc<Cluster>; 3] {
    let _ = std::fs::remove_dir_all(root);
    let (a, port_a) = start_retrying(&root.join("a"), None);
    assert!(
        wait_for(5.0, || a.is_leader()),
        "the founder leads a cluster of one"
    );
    let (b, port_b) = start_retrying(&root.join("b"), Some(port_a));
    // the third joins through the second (any member is a seed)
    let (c, _) = start_retrying(&root.join("c"), Some(port_b));
    assert_eq!((a.id(), b.id(), c.id()), (1, 2, 3));
    let all = [a, b, c];
    assert!(
        wait_for(10.0, || all.iter().all(|x| x
            .shared
            .machine
            .read()
            .unwrap()
            .members
            .len()
            == 3)),
        "everyone sees three members"
    );
    all
}

#[test]
fn three_instances_agree_survive_a_lost_leader_and_fence_without_quorum() {
    let root = std::env::temp_dir().join(format!("dyno-cluster-{}", std::process::id()));
    let all = three(&root);
    let c = &all[2];
    // a claim proposed on a follower commits everywhere
    assert!(matches!(claim(c, "ada"), Outcome::Claimed { epoch: 1, .. }));
    assert!(wait_for(5.0, || all.iter().all(|x| x
        .shared
        .machine
        .read()
        .unwrap()
        .locks
        .contains_key("ada"))));
    // and nobody else can take her
    assert!(matches!(claim(&all[1], "Ada"), Outcome::Refused(_)));
    // the leader dies: the other two elect a new one and keep working
    let leader = all.iter().find(|x| x.is_leader()).unwrap().clone();
    leader.stop();
    let rest: Vec<_> = all
        .iter()
        .filter(|x| x.id() != leader.id())
        .cloned()
        .collect();
    assert!(
        wait_for(10.0, || rest.iter().any(|x| x.is_leader())),
        "a new leader within a few seconds"
    );
    assert!(matches!(claim(&rest[0], "bo"), Outcome::Claimed { .. }));
    // with two left, cut one off: both lose the quorum and fence
    let cut = rest[1].clone();
    cut.link.cut.store(true, Ordering::SeqCst);
    assert!(
        wait_for(8.0, || !cut.in_contact()),
        "a cut instance fences itself"
    );
    assert!(
        wait_for(10.0, || !rest[0].in_contact()),
        "one of three is not a quorum"
    );
    // heal: they agree again
    cut.link.cut.store(false, Ordering::SeqCst);
    assert!(wait_for(10.0, || rest.iter().all(|x| x.in_contact())));
    assert!(matches!(claim(&cut, "cy"), Outcome::Claimed { .. }));
    for x in &all {
        x.stop();
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_cut_off_instance_is_marked_down_and_its_characters_released() {
    let root = std::env::temp_dir().join(format!("dyno-cluster-down-{}", std::process::id()));
    let all = three(&root);
    // ada plays on a follower; that follower is cut off
    let victim = all.iter().find(|x| !x.is_leader()).unwrap().clone();
    assert!(matches!(claim(&victim, "ada"), Outcome::Claimed { .. }));
    victim.link.cut.store(true, Ordering::SeqCst);
    let cut_at = Instant::now();
    let others: Vec<_> = all
        .iter()
        .filter(|x| x.id() != victim.id())
        .cloned()
        .collect();
    let held = |x: &Arc<Cluster>| x.shared.machine.read().unwrap().locks.contains_key("ada");
    // it fences long before the others release her: by the rule, not by
    // how fast this machine happens to be (a slow CI runner can take a
    // while to see the fence)
    assert!(wait_for(7.0, || !victim.in_contact()));
    let fenced = cut_at.elapsed();
    assert!(
        fenced < DOWN_AFTER,
        "fenced after {fenced:?}, before the down timer"
    );
    assert!(
        wait_for(30.0, || !held(&others[0])),
        "released after the down timer"
    );
    // never released before the down timer ran out (it counts from her
    // instance's last word, a heartbeat or so before the cut)
    assert!(
        cut_at.elapsed() + Duration::from_secs(1) >= DOWN_AFTER,
        "released {:?} after the cut, before the down timer ({DOWN_AFTER:?})",
        cut_at.elapsed()
    );
    // she can now play elsewhere, exactly once
    assert!(matches!(claim(&others[0], "ada"), Outcome::Claimed { .. }));
    assert!(matches!(claim(&others[1], "ada"), Outcome::Refused(_)));
    // the cut instance comes back and is marked up again
    victim.link.cut.store(false, Ordering::SeqCst);
    let vid = victim.id();
    assert!(wait_for(20.0, || others[0]
        .shared
        .machine
        .read()
        .unwrap()
        .members
        .get(&vid)
        .is_some_and(|m| !m.down)));
    for x in &all {
        x.stop();
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_restarted_founder_keeps_its_state() {
    let root = std::env::temp_dir().join(format!("dyno-cluster-restart-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let port = free_port();
    let a = Cluster::start(cfg(&root, port, None), load()).unwrap();
    assert!(wait_for(5.0, || a.is_leader()));
    for i in 0..30 {
        assert!(matches!(
            claim(&a, &format!("p{i}")),
            Outcome::Claimed { .. }
        ));
    }
    a.stop();
    drop(a);
    std::thread::sleep(Duration::from_millis(300));
    let port = free_port();
    let a = Cluster::start(cfg(&root, port, None), load()).unwrap();
    assert!(
        wait_for(5.0, || a.is_leader()),
        "a cluster of one elects itself again"
    );
    assert!(
        wait_for(5.0, || a.shared.machine.read().unwrap().locks.len() == 30),
        "the log replays"
    );
    assert!(matches!(claim(&a, "p31"), Outcome::Claimed { .. }));
    a.stop();
    let _ = std::fs::remove_dir_all(&root);
}
