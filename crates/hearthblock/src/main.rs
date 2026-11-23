//! `hearthblock`: the player client. An engine (renderer, UI, input, audio,
//! protocol) with no game content: the world's server streams the content
//! pack (docs/design/data-and-streaming.md §7). It updates itself as
//! `hearthblock`; servers and tools are the `dyno` binary.

use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str).unwrap_or("") {
        "version" | "--version" | "-V" => {
            println!(
                "hearthblock {} ({})",
                dyno_update::VERSION,
                dyno_update::TARGET
            );
        }
        "help" | "--help" | "-h" => {
            println!(
                "hearthblock - Hearthblock Online\n\nUSAGE:\n  hearthblock                   Play\n  hearthblock connect <address> Play on another world (development)\n  hearthblock update            Install the newest release\n  hearthblock version           Print the version\n\nSet DYNO_NO_UPDATE=1 to turn off automatic updates."
            );
        }
        "update" => match dyno_update::check_and_update(true, Duration::from_secs(30)) {
            Ok(dyno_update::Outcome::Updated { from, to }) => println!("Updated {from} -> {to}"),
            Ok(other) => println!("{other:?}"),
            Err(e) => {
                eprintln!("update failed: {e}");
                std::process::exit(1);
            }
        },
        cmd => {
            let note = auto_update();
            let prefill = if cmd == "connect" {
                args.get(1).cloned()
            } else {
                None
            };
            if let Err(e) = dyno_client::run(prefill, note) {
                eprintln!("hearthblock could not start the game: {e}");
                std::process::exit(1);
            }
        }
    }
}

/// Check for updates at start. Every failure is soft. Returns a short status for the UI.
fn auto_update() -> String {
    if std::env::var_os(dyno_update::JUST_UPDATED_ENV).is_some() {
        return "just updated".into();
    }
    match dyno_update::check_and_update(false, Duration::from_secs(8)) {
        Ok(dyno_update::Outcome::Updated { from, to }) => {
            println!("Updated {from} -> {to}. Restarting...");
            dyno_update::restart();
        }
        Ok(dyno_update::Outcome::UpToDate(_)) => "up to date".into(),
        Ok(dyno_update::Outcome::Skipped(why)) => format!("updates off: {why}"),
        Err(e) => {
            eprintln!(
                "update check failed, continuing with {}: {e}",
                dyno_update::VERSION
            );
            "update check failed".into()
        }
    }
}
