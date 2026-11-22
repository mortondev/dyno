//! The `dyno` executable: client, server and updater in one binary.

use std::time::Duration;

const HELP: &str = "\
dyno - live multiplayer block world (bootstrap build)

USAGE:
  dyno                         Open the game (connect screen)
  dyno server [--port 7400] [--bind 0.0.0.0] [--world PATH] [--auto-update]
                               Run a dedicated game server
  dyno update                  Check for a new release and install it
  dyno version                 Print version, target and protocol
  dyno verify-manifest <manifest.json> <manifest.json.sig>
                               Verify a release manifest with the built-in key

Set DYNO_NO_UPDATE=1 to turn off automatic updates.
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("");
    match cmd {
        "version" | "--version" | "-V" => {
            println!("dyno {} ({})", dyno_update::VERSION, dyno_update::TARGET);
        }
        "help" | "--help" | "-h" => print!("{HELP}"),
        "update" => match dyno_update::check_and_update(true, Duration::from_secs(30)) {
            Ok(dyno_update::Outcome::Updated { from, to }) => println!("Updated {from} -> {to}"),
            Ok(other) => println!("{other:?}"),
            Err(e) => {
                eprintln!("update failed: {e}");
                std::process::exit(1);
            }
        },
        "verify-manifest" => {
            let (Some(m), Some(s)) = (args.get(1), args.get(2)) else {
                eprintln!("usage: dyno verify-manifest <manifest.json> <manifest.json.sig>");
                std::process::exit(2);
            };
            let mb = std::fs::read(m).expect("read manifest");
            let sb = std::fs::read(s).expect("read signature");
            match dyno_update::verify_manifest(&mb, &sb) {
                Ok(man) => println!(
                    "OK: version {} with {} assets",
                    man.version,
                    man.assets.len()
                ),
                Err(e) => {
                    eprintln!("FAILED: {e}");
                    std::process::exit(1);
                }
            }
        }
        _ => {
            auto_update();
            println!(
                "dyno {} - the game client is not built yet.",
                dyno_update::VERSION
            );
        }
    }
}

/// Check for updates at start. Every failure is soft.
fn auto_update() {
    match dyno_update::check_and_update(false, Duration::from_secs(8)) {
        Ok(dyno_update::Outcome::Updated { from, to }) => {
            println!("Updated {from} -> {to}. Restarting...");
            dyno_update::restart();
        }
        Ok(_) => {}
        Err(e) => eprintln!(
            "update check failed, continuing with {}: {e}",
            dyno_update::VERSION
        ),
    }
}
