//! The content pack streams from the server (docs/design/data-and-streaming.md
//! §1, §2): the first join downloads it, a second join with nothing changed
//! downloads nothing, a changed file is the only one fetched, streamed files
//! come during play without holding play up, and /reload sends new rules.

use dyno_client::cache::Cache;
use dyno_client::net::{self, JoinOptions, Joined, NetEvent, Progress};
use dyno_protocol::{ClientMsg, ServerMsg};
use dyno_server::pack::Pack;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("dyno-packtest-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn start(dir: &Path, content: Option<PathBuf>, rate: usize) -> (dyno_server::Handle, String) {
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        world_size: [48, 48, 48],
        content_path: content,
        asset_rate: rate,
        trust_movement: true,
        ..Default::default()
    };
    let h = dyno_server::start(cfg).expect("server starts");
    let addr = format!("127.0.0.1:{}", h.local_addr.port());
    (h, addr)
}

/// The built-in pack written out as a directory.
fn write_pack(dir: &Path) {
    for (path, bytes) in Pack::builtin().files() {
        let p = dir.join(&path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, &bytes[..]).unwrap();
    }
}

fn join(addr: &str, name: &str, token: u64, cache: &Path) -> (Joined, Arc<Progress>) {
    let progress = Arc::new(Progress::default());
    let j = net::connect_with(
        addr,
        name,
        JoinOptions {
            token,
            cache: Some(Cache::at(cache)),
            progress: Some(progress.clone()),
            ..Default::default()
        },
        Duration::from_secs(10),
    )
    .unwrap_or_else(|e| panic!("{name} joins: {e}"));
    (j, progress)
}

fn wait_msg<T>(j: &Joined, secs: f32, mut f: impl FnMut(&ServerMsg) -> Option<T>) -> Option<T> {
    let until = Instant::now() + Duration::from_secs_f32(secs);
    while Instant::now() < until {
        if let Ok(NetEvent::Msg(m)) = j.conn.rx.recv_timeout(Duration::from_millis(20))
            && let Some(t) = f(&m)
        {
            return Some(t);
        }
    }
    None
}

#[test]
fn a_second_join_with_nothing_changed_downloads_nothing() {
    let dir = temp("second");
    let cache = dir.join("cache");
    let (server, addr) = start(&dir, None, 4 * 1024 * 1024);
    let pack = server.server.pack();

    let (first, progress) = join(&addr, "ada", 1, &cache);
    assert_eq!(first.manifest, pack.manifest, "the welcome names the pack");
    assert_eq!(
        first.downloaded,
        pack.manifest.download_bytes(),
        "the first join fetches every file"
    );
    assert_eq!(progress.fraction(), Some(1.0));
    assert_eq!(first.content.name, pack.content.name, "rules from the pack");
    // every file is in the asset store for the game, from the stream alone
    // (this test never loads a local pack): blocks.json, textures, sounds,
    // fonts, the stage and the strings
    // (the store is shared with the other tests here, so check the bytes in
    // this test's own cache)
    // (streamed files, such as creatures, wait until something asks)
    for f in &pack.manifest.files {
        if !pack.manifest.preload(&f.path) {
            assert!(Cache::at(&cache).read(f).is_none(), "{} came early", f.path);
            continue;
        }
        assert!(
            dyno_client::assets::get(&f.path).is_some(),
            "{} did not arrive",
            f.path
        );
        assert!(Cache::at(&cache).read(f).is_some(), "{} not cached", f.path);
    }
    // they travel compressed, and a first join stays small
    assert!(first.downloaded < pack.bytes() / 2);
    assert!(
        first.downloaded < 4_000_000,
        "a first join downloads {} bytes",
        first.downloaded
    );
    assert!(pack.manifest.file("blocks.json").is_some());
    // what the server sent while the files downloaded still reaches the game
    assert!(
        wait_msg(&first, 3.0, |m| matches!(m, ServerMsg::Pack(_))
            .then_some(()))
        .is_some(),
        "the pack (bags) message is not lost behind the download"
    );

    let (second, _) = join(&addr, "bo", 2, &cache);
    assert_eq!(second.downloaded, 0, "nothing changed: nothing downloads");
    assert_eq!(second.content.name, pack.content.name);
    eprintln!(
        "join bytes: first {} ({} files), second {}",
        first.downloaded,
        pack.manifest.files.len(),
        second.downloaded
    );
    // the title screen finds the last pack in the cache
    assert_eq!(
        Cache::at(&cache).last_manifest(),
        Some(pack.manifest.clone())
    );
    drop((first, second));
    server.stop();

    // one file changes: only it downloads
    let packdir = dir.join("pack");
    write_pack(&packdir);
    let strings = packdir.join("ui/strings.json");
    let text = std::fs::read_to_string(&strings).unwrap();
    std::fs::write(&strings, text.replace("Ashfall Vale", "Ashfall Vale!")).unwrap();
    let (server2, addr2) = start(&dir.join("2"), Some(packdir), 4 * 1024 * 1024);
    let (third, _) = join(&addr2, "cy", 3, &cache);
    let changed = server2
        .server
        .pack()
        .manifest
        .file("ui/strings.json")
        .unwrap()
        .wire as u64;
    assert_eq!(third.downloaded, changed, "only the changed file downloads");
    drop(third);
    server2.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn streamed_files_arrive_during_play_without_holding_it_up() {
    let dir = temp("stream");
    let packdir = dir.join("pack");
    write_pack(&packdir);
    // a large file the pack streams when first needed
    let meta = std::fs::read_to_string(packdir.join("pack.json")).unwrap();
    let mut meta: serde_json::Value = serde_json::from_str(&meta).unwrap();
    meta["stream"].as_array_mut().unwrap().push("big/".into());
    std::fs::write(packdir.join("pack.json"), meta.to_string()).unwrap();
    // noise, so it travels as it is and the budget is easy to check
    let big: Vec<u8> = (0..640 * 1024u32)
        .scan(0x9E37_79B9u32, |x, _| {
            *x ^= *x << 13;
            *x ^= *x >> 17;
            *x ^= *x << 5;
            Some((*x >> 24) as u8)
        })
        .collect();
    std::fs::create_dir_all(packdir.join("big")).unwrap();
    std::fs::write(packdir.join("big/blob.bin"), &big).unwrap();
    let rate = 256 * 1024;
    let (server, addr) = start(&dir, Some(packdir), rate);
    let pack = server.server.pack();
    assert!(!pack.manifest.preload("big/blob.bin"));

    let (j, _) = join(&addr, "ada", 1, &dir.join("cache"));
    assert!(
        j.downloaded < pack.manifest.download_bytes() + 1,
        "the streamed file is not fetched before play"
    );
    assert!(dyno_client::assets::get("big/blob.bin").is_none());

    // the game asks for it; meanwhile the player keeps playing
    dyno_client::assets::request("big/blob.bin");
    let t0 = Instant::now();
    let mut worst = Duration::ZERO;
    while dyno_client::assets::get("big/blob.bin").is_none() {
        assert!(t0.elapsed() < Duration::from_secs(20), "the file arrives");
        let asked = Instant::now();
        j.conn
            .tx
            .send(ClientMsg::Chat {
                text: "/who".into(),
            })
            .unwrap();
        let answered = wait_msg(&j, 5.0, |m| match m {
            ServerMsg::System { text } if text.contains("online") => Some(()),
            _ => None,
        });
        assert!(
            answered.is_some(),
            "commands are answered during the download"
        );
        worst = worst.max(asked.elapsed());
        std::thread::sleep(Duration::from_millis(100));
    }
    let took = t0.elapsed();
    assert_eq!(
        &dyno_client::assets::get("big/blob.bin").unwrap()[..],
        &big[..]
    );
    // the budget holds: after a 64 KB burst, about 256 KB/s
    let budget = (big.len() - 64 * 1024) as f64 / rate as f64;
    eprintln!(
        "streamed {} KB in {:.2} s (budget {:.2} s); slowest reply during it {} ms",
        big.len() / 1024,
        took.as_secs_f64(),
        budget,
        worst.as_millis()
    );
    assert!(took.as_secs_f64() > budget * 0.8, "kept to the budget");
    assert!(
        worst < Duration::from_millis(500),
        "play never waits behind the download ({worst:?})"
    );
    drop(j);
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn reload_sends_new_rules_through_the_pack() {
    let dir = temp("reload");
    let rules = dir.join("rules.json");
    std::fs::write(&rules, dyno_world::content::DEFAULT_JSON).unwrap();
    let (server, addr) = start(&dir, Some(rules.clone()), 4 * 1024 * 1024);
    // the first player owns the world and may /reload
    let (j, _) = join(&addr, "ada", 1, &dir.join("cache"));
    let before = j.content.version;
    std::fs::write(
        &rules,
        dyno_world::content::DEFAULT_JSON.replacen("\"version\": ", "\"version\": 1", 1),
    )
    .unwrap();
    j.conn
        .tx
        .send(ClientMsg::Chat {
            text: "/reload".into(),
        })
        .unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    let mut got = None;
    while Instant::now() < until && got.is_none() {
        if let Ok(NetEvent::Content(c)) = j.conn.rx.recv_timeout(Duration::from_millis(50)) {
            got = Some(c.version);
        }
    }
    let after = got.expect("the new rules arrive");
    assert_ne!(after, before);
    assert_eq!(after, server.server.pack().content.version);
    drop(j);
    server.stop();
    let _ = std::fs::remove_dir_all(&dir);
}
