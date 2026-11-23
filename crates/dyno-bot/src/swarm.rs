//! Many bots at once: one thread each, staggered joins, unique names, a
//! progress line now and then, and the fun report at the end.

use crate::agent::Agent;
use crate::body::{Body, TICK};
use crate::brain::{Personality, Scripted};
use crate::report;
use crate::telemetry::Telemetry;
use dyno_protocol::Appearance;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct Options {
    /// server address; None with `local` starts one here
    pub addr: Option<String>,
    pub local: bool,
    pub count: usize,
    pub minutes: f32,
    /// "mix" or a personality
    pub brain: String,
    /// a class key for everyone, or None to rotate through the classes
    pub class: Option<String>,
    pub report: PathBuf,
    /// seconds between joins
    pub stagger: f32,
    /// a name prefix (a run id is added)
    pub prefix: String,
    pub quiet: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            addr: None,
            local: false,
            count: 10,
            minutes: 10.0,
            brain: "mix".into(),
            class: None,
            report: "bots-report.json".into(),
            stagger: 2.0,
            prefix: "Bot".into(),
            quiet: false,
        }
    }
}

/// What the progress line shows of one bot.
#[derive(Clone, Debug, Default)]
pub struct Live {
    pub level: u8,
    pub quests: usize,
    pub kills: u32,
    pub deaths: usize,
    pub doing: String,
    pub done: bool,
}

pub type Board = Arc<Mutex<BTreeMap<String, Live>>>;

const CLASSES: [&str; 6] = ["warrior", "paladin", "hunter", "rogue", "priest", "mage"];

/// A look for a bot, from its number.
pub fn look(i: usize) -> Appearance {
    let n = i as u8;
    Appearance {
        skin: n % 5,
        eyes: n % 4,
        hair_style: n % 5,
        hair_color: (n / 2) % 6,
        beard: i.is_multiple_of(3),
        build: n % 3,
        mood: (n / 3) % 3,
        ears: (n / 5) % 3,
    }
}

/// Play one bot until `end` (or `stop`). Always returns its telemetry.
#[allow(clippy::too_many_arguments)]
pub fn run_bot(
    addr: &str,
    name: &str,
    kind: Personality,
    class: &str,
    i: usize,
    end: Instant,
    stop: Arc<AtomicBool>,
    board: Option<Board>,
) -> Telemetry {
    let mut tries = 0;
    let body = loop {
        match Body::join(addr, name, Some(class), look(i)) {
            Ok(b) => break b,
            Err(e) => {
                tries += 1;
                if tries >= 3 {
                    return Telemetry {
                        name: name.into(),
                        brain: kind.key().into(),
                        class: class.into(),
                        disconnects: vec![format!("could not join: {e}")],
                        ..Default::default()
                    };
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        }
    };
    let mut a = Agent::new(body, kind.key());
    // take in the join burst before measuring anything
    let settle = Instant::now();
    while settle.elapsed() < Duration::from_millis(1500) {
        if !a.update() {
            break;
        }
        std::thread::sleep(Duration::from_secs_f32(TICK));
    }
    a.baseline();
    let mut brain = Scripted::new(kind, crate::body::token_for(name));
    let mut next_board = Instant::now();
    loop {
        let t0 = Instant::now();
        if stop.load(Ordering::Relaxed) || t0 >= end || brain.quit {
            break;
        }
        brain.think(&mut a);
        if !a.update() {
            break;
        }
        if let Some(b) = &board
            && t0 >= next_board
        {
            next_board = t0 + Duration::from_secs(2);
            let live = Live {
                level: a.body.level,
                quests: a.tel.quests.iter().filter(|q| q.done.is_some()).count(),
                kills: a.tel.kills,
                deaths: a.tel.deaths.len(),
                doing: a.doing.clone(),
                done: false,
            };
            b.lock().unwrap().insert(name.to_string(), live);
        }
        let spent = t0.elapsed().as_secs_f32();
        if spent < TICK {
            std::thread::sleep(Duration::from_secs_f32(TICK - spent));
        }
    }
    if let Some(b) = &board
        && let Some(l) = b.lock().unwrap().get_mut(name)
    {
        l.done = true;
    }
    a.tel.level = a.body.level;
    a.tel
}

/// A short run id for names, so each swarm makes fresh characters.
fn run_id() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let a = b"abcdefghjkmnpqrstuvwxyz";
    let mut n = t ^ (std::process::id() as u64) << 7;
    let mut s = String::new();
    for _ in 0..3 {
        s.push(a[(n % a.len() as u64) as usize] as char);
        n /= a.len() as u64;
    }
    s
}

/// Start a server in this process on a fresh world. Returns its address.
pub fn local_server(players: usize) -> Result<(String, dyno_server::Handle), String> {
    let dir = std::env::temp_dir().join(format!("dyno-bots-{}-{}", std::process::id(), run_id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let cfg = dyno_server::Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        world_path: dir.join("world.dynw"),
        ops: vec!["botmaster".into()],
        max_players: players + 8,
        ..Default::default()
    };
    let h = dyno_server::start(cfg).map_err(|e| e.to_string())?;
    Ok((format!("127.0.0.1:{}", h.local_addr.port()), h))
}

/// Run a swarm and write the report (JSON, and Markdown beside it).
pub fn run(o: Options) -> Result<report::Report, String> {
    let mut _server = None;
    let addr = match (&o.addr, o.local) {
        (_, true) => {
            let (a, h) = local_server(o.count)?;
            eprintln!("Started a local server on {a}");
            _server = Some(h);
            a
        }
        (Some(a), false) => a.clone(),
        (None, false) => return Err("give a server address or --local".into()),
    };
    let fixed = if o.brain == "mix" {
        None
    } else {
        Some(Personality::parse(&o.brain).ok_or_else(|| format!("unknown brain '{}'", o.brain))?)
    };
    let id = run_id();
    let stop = Arc::new(AtomicBool::new(false));
    let board: Board = Arc::new(Mutex::new(BTreeMap::new()));
    let start = Instant::now();
    let end = start + Duration::from_secs_f32(o.minutes * 60.0 + o.stagger * o.count as f32);
    let mut handles = Vec::new();
    for i in 0..o.count {
        let kind = fixed.unwrap_or_else(|| Personality::mix(i));
        let class = o
            .class
            .clone()
            .unwrap_or_else(|| CLASSES[i % CLASSES.len()].to_string());
        let mut name = format!("{}{}{:02}{}", o.prefix, id, i, &kind.key()[..3]);
        name.truncate(dyno_protocol::MAX_NAME);
        let addr = addr.clone();
        let stop = stop.clone();
        let board = board.clone();
        // each bot plays the same length of time, whenever it joined
        let my_end = Instant::now() + Duration::from_secs_f32(o.minutes * 60.0);
        let my_end = my_end.min(end);
        handles.push(
            std::thread::Builder::new()
                .name(name.clone())
                .spawn(move || run_bot(&addr, &name, kind, &class, i, my_end, stop, Some(board)))
                .map_err(|e| e.to_string())?,
        );
        if i + 1 < o.count {
            std::thread::sleep(Duration::from_secs_f32(o.stagger));
        }
    }
    let mut last = Instant::now();
    while handles.iter().any(|h| !h.is_finished()) {
        std::thread::sleep(Duration::from_millis(250));
        if !o.quiet && last.elapsed() >= Duration::from_secs(30) {
            last = Instant::now();
            let b = board.lock().unwrap();
            let n = b.len().max(1);
            let lv: f32 = b.values().map(|l| l.level as f32).sum::<f32>() / n as f32;
            let q: usize = b.values().map(|l| l.quests).sum();
            let k: u32 = b.values().map(|l| l.kills).sum();
            let d: usize = b.values().map(|l| l.deaths).sum();
            eprintln!(
                "[{:>4.1} min] {} bots: mean level {lv:.2}, {q} quests, {k} kills, {d} deaths",
                start.elapsed().as_secs_f32() / 60.0,
                b.len()
            );
            for (name, l) in b.iter().take(40) {
                eprintln!(
                    "    {name:<22} lv{} q{} k{} d{} {}",
                    l.level, l.quests, l.kills, l.deaths, l.doing
                );
            }
        }
    }
    let tels: Vec<Telemetry> = handles
        .into_iter()
        .map(|h| h.join().unwrap_or_default())
        .collect();
    let rep = report::build(
        &format!(
            "Hearthblock playtest: {} bots, {} min, brain {}",
            o.count, o.minutes, o.brain
        ),
        if o.local { "local" } else { &addr },
        tels,
    );
    write_report(&rep, &o.report)?;
    Ok(rep)
}

pub fn write_report(rep: &report::Report, path: &std::path::Path) -> Result<(), String> {
    if let Some(dir) = path.parent()
        && !dir.as_os_str().is_empty()
    {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(rep).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| format!("{}: {e}", path.display()))?;
    let md = path.with_extension("md");
    std::fs::write(&md, report::markdown(rep)).map_err(|e| format!("{}: {e}", md.display()))?;
    eprintln!("Wrote {} and {}", path.display(), md.display());
    Ok(())
}
