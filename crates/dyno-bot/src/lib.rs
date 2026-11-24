//! Headless playtesting for Hearthblock Online.
//!
//! - `body`: a headless player (world copy, real physics, what the server says)
//! - `nav`: path finding over the block grid
//! - `agent`: a body carrying out one task at a time, with telemetry
//! - `brain`: scripted players with personalities
//! - `swarm`: many bots at once (`dyno bots`)
//! - `report`: the fun report (JSON and Markdown)
//! - `play`: a text interface for AI playtesters (`dyno play`)

pub mod agent;
pub mod body;
pub mod brain;
pub mod nav;
pub mod play;
pub mod report;
pub mod swarm;
pub mod telemetry;

/// `dyno bots <addr> --count N --minutes M [--brain mix|completionist|grinder|explorer|social|casual]
/// [--class KEY] [--report out.json] [--stagger SECS] [--prefix NAME] [--local] [--quiet]`
pub fn cli_bots(args: &[String]) -> Result<(), String> {
    let mut o = swarm::Options::default();
    let mut i = 0;
    while i < args.len() {
        let val = |i: usize| {
            args.get(i + 1)
                .cloned()
                .ok_or(format!("{} needs a value", args[i]))
        };
        match args[i].as_str() {
            "--count" | "-n" => {
                o.count = val(i)?.parse().map_err(|_| "--count needs a number")?;
                i += 1;
            }
            "--minutes" | "-m" => {
                o.minutes = val(i)?.parse().map_err(|_| "--minutes needs a number")?;
                i += 1;
            }
            "--brain" => {
                o.brain = val(i)?;
                i += 1;
            }
            "--class" => {
                o.class = Some(val(i)?);
                i += 1;
            }
            "--report" => {
                o.report = val(i)?.into();
                i += 1;
            }
            "--stagger" => {
                o.stagger = val(i)?.parse().map_err(|_| "--stagger needs seconds")?;
                i += 1;
            }
            "--prefix" => {
                o.prefix = val(i)?;
                i += 1;
            }
            "--local" => o.local = true,
            "--port" => {
                o.port = val(i)?.parse().map_err(|_| "--port needs a number")?;
                i += 1;
            }
            "--party-size" => {
                o.party_size = val(i)?.parse().map_err(|_| "--party-size needs a number")?;
                i += 1;
            }
            "--setup" => {
                o.setup = val(i)?.split(';').map(|s| s.trim().to_string()).collect();
                i += 1;
            }
            "--content" => {
                o.content = Some(val(i)?.into());
                i += 1;
            }
            "--quiet" => o.quiet = true,
            a if !a.starts_with("--") && o.addr.is_none() => o.addr = Some(a.to_string()),
            other => return Err(format!("unknown bots option {other}")),
        }
        i += 1;
    }
    let r = swarm::run(o)?;
    let fails: Vec<&str> = r
        .checks
        .iter()
        .filter(|c| c.status == "fail")
        .map(|c| c.key.as_str())
        .collect();
    println!(
        "{} bots, {:.0} bot-minutes. Failed fun checks: {}",
        r.aggregate.bots,
        r.aggregate.bot_minutes,
        if fails.is_empty() {
            "none".into()
        } else {
            fails.join(", ")
        }
    );
    Ok(())
}

/// `dyno play <addr> --as NAME [--class KEY] [--script FILE]`
pub fn cli_play(args: &[String]) -> Result<(), String> {
    let mut o = play::Options {
        addr: String::new(),
        name: "Playtester".into(),
        class: None,
        script: None,
    };
    let mut i = 0;
    while i < args.len() {
        let val = |i: usize| {
            args.get(i + 1)
                .cloned()
                .ok_or(format!("{} needs a value", args[i]))
        };
        match args[i].as_str() {
            "--as" | "--name" => {
                o.name = val(i)?;
                i += 1;
            }
            "--class" => {
                o.class = Some(val(i)?);
                i += 1;
            }
            "--script" => {
                o.script = Some(val(i)?.into());
                i += 1;
            }
            a if !a.starts_with("--") && o.addr.is_empty() => o.addr = a.to_string(),
            other => return Err(format!("unknown play option {other}")),
        }
        i += 1;
    }
    if o.addr.is_empty() {
        return Err("usage: dyno play <address> --as NAME [--class KEY] [--script FILE]".into());
    }
    play::run(o)
}
