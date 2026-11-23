//! Signed self-updater for Dyno.
//!
//! Each GitHub release of `mortondev/dyno` carries `manifest.json` and
//! `manifest.json.sig` (a raw 64-byte Ed25519 signature of the manifest bytes).
//! The updater reads both from the `releases/latest/download/` URL, verifies the
//! signature with the public key built into this binary, checks the SHA-256 of
//! the platform binary, and replaces the running executable.
//!
//! Every failure is soft: the caller logs it and keeps running the current version.

use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

/// GitHub repository that publishes releases.
pub const REPO: &str = "mortondev/dyno";

/// Ed25519 public key of the release signing key (raw 32 bytes).
pub const PUBLIC_KEY: [u8; 32] = [
    13, 17, 204, 185, 54, 111, 121, 205, 167, 229, 78, 134, 163, 107, 202, 20, 81, 76, 39, 152,
    208, 24, 71, 233, 5, 114, 130, 170, 229, 250, 147, 166,
];

/// Rust target triple this binary was built for, for example `x86_64-unknown-linux-gnu`.
pub const TARGET: &str = env!("DYNO_TARGET");

/// Version of this binary.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Environment variable set on the process that starts after an update.
/// That process does not check again, so a bad release cannot cause a loop.
pub const JUST_UPDATED_ENV: &str = "DYNO_JUST_UPDATED";

/// Set this environment variable to any value to disable automatic updates.
pub const DISABLE_ENV: &str = "DYNO_NO_UPDATE";

const MAX_MANIFEST: u64 = 256 * 1024;
const MAX_BINARY: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Release version without the leading `v`, for example `0.0.2`.
    pub version: String,
    /// Network protocol version of this release.
    pub protocol: u32,
    /// The `dyno` binary, one entry per target triple. Every updater before
    /// 0.7 reads only this, so it stays the `dyno` binary for good.
    pub assets: BTreeMap<String, Asset>,
    /// Every program by name (`dyno`, `hearthblock`), then target triple.
    #[serde(default)]
    pub programs: BTreeMap<String, BTreeMap<String, Asset>>,
}

/// The program an executable is, from its file name: `hearthblock` for the
/// player client, else `dyno` (servers, tools, the dev client), whatever
/// it was renamed to.
pub fn program_of(exe: &Path) -> &'static str {
    // the last part of the path, split on either separator
    let path = exe.to_string_lossy().to_lowercase();
    let name = path.rsplit(['/', '\\']).next().unwrap_or_default();
    if name.starts_with("hearthblock") {
        "hearthblock"
    } else {
        "dyno"
    }
}

/// The program this executable is.
pub fn program() -> &'static str {
    exe_path().map(|p| program_of(&p)).unwrap_or("dyno")
}

/// The release file for a program on a target. A player client never falls
/// back to the `dyno` binary.
pub fn pick<'a>(m: &'a Manifest, program: &str, target: &str) -> Result<&'a Asset, Error> {
    let found = if program == "dyno" {
        m.assets.get(target)
    } else {
        m.programs.get(program).and_then(|t| t.get(target))
    };
    found.ok_or_else(|| {
        err(format!(
            "release {} has no {program} build for {target}",
            m.version
        ))
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug)]
pub enum Outcome {
    /// Updates are disabled, or this is the process that started after an update.
    Skipped(&'static str),
    /// The installed version is the newest.
    UpToDate(String),
    /// The executable was replaced. The caller should restart.
    Updated { from: String, to: String },
}

#[derive(Debug)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}

fn err(msg: impl Into<String>) -> Error {
    Error(msg.into())
}

/// Verify a manifest signature with the built-in public key.
pub fn verify_manifest(manifest: &[u8], signature: &[u8]) -> Result<Manifest, Error> {
    let key =
        VerifyingKey::from_bytes(&PUBLIC_KEY).map_err(|e| err(format!("bad public key: {e}")))?;
    let sig: [u8; 64] = signature.try_into().map_err(|_| {
        err(format!(
            "signature must be 64 bytes, got {}",
            signature.len()
        ))
    })?;
    let sig = Signature::from_bytes(&sig);
    key.verify_strict(manifest, &sig)
        .map_err(|_| err("manifest signature is not valid"))?;
    serde_json::from_slice(manifest).map_err(|e| err(format!("manifest is not valid JSON: {e}")))
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .user_agent(format!("dyno/{VERSION}"))
        .build()
        .into()
}

fn get(agent: &ureq::Agent, url: &str, limit: u64) -> Result<Vec<u8>, Error> {
    let mut resp = agent
        .get(url)
        .call()
        .map_err(|e| err(format!("GET {url}: {e}")))?;
    resp.body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .map_err(|e| err(format!("reading {url}: {e}")))
}

/// Download and verify the manifest of the latest release.
pub fn fetch_latest(timeout: Duration) -> Result<Manifest, Error> {
    let agent = agent(timeout);
    let base = format!("https://github.com/{REPO}/releases/latest/download");
    let manifest = get(&agent, &format!("{base}/manifest.json"), MAX_MANIFEST)?;
    let sig = get(&agent, &format!("{base}/manifest.json.sig"), 1024)?;
    verify_manifest(&manifest, &sig)
}

/// True when `candidate` is a newer version than `current`.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (
        semver::Version::parse(candidate),
        semver::Version::parse(current),
    ) {
        (Ok(a), Ok(b)) => a > b,
        _ => false,
    }
}

/// Check for a newer release and install it.
///
/// `force` ignores the disable variable and debug builds (used by `dyno update`).
pub fn check_and_update(force: bool, timeout: Duration) -> Result<Outcome, Error> {
    // Right after an update-restart, do not check again (no update loops). The
    // variable stays set for the life of the process, so this only applies to
    // checks in the first few minutes; long-running servers check again later.
    static FIRST_CHECK: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    let first = *FIRST_CHECK.get_or_init(std::time::Instant::now);
    if std::env::var_os(JUST_UPDATED_ENV).is_some() && first.elapsed() < Duration::from_secs(4 * 60)
    {
        return Ok(Outcome::Skipped("this process started after an update"));
    }
    if !force {
        if std::env::var_os(DISABLE_ENV).is_some() {
            return Ok(Outcome::Skipped("DYNO_NO_UPDATE is set"));
        }
        if cfg!(debug_assertions) {
            return Ok(Outcome::Skipped("development build"));
        }
    }
    let manifest = fetch_latest(timeout)?;
    if !is_newer(&manifest.version, VERSION) {
        return Ok(Outcome::UpToDate(VERSION.to_string()));
    }
    // the artifact named like this executable: `dyno` stays `dyno` (the live
    // server updates itself this way), `hearthblock` stays `hearthblock`
    let asset = pick(&manifest, program(), TARGET)?;
    if asset.size > MAX_BINARY {
        return Err(err("release binary is too large"));
    }
    let url = format!(
        "https://github.com/{REPO}/releases/download/v{}/{}",
        manifest.version, asset.name
    );
    let bytes = get(&agent(Duration::from_secs(600)), &url, MAX_BINARY)?;
    if bytes.len() as u64 != asset.size {
        return Err(err(format!(
            "size mismatch: expected {}, got {}",
            asset.size,
            bytes.len()
        )));
    }
    let digest = hex(&Sha256::digest(&bytes));
    if !digest.eq_ignore_ascii_case(&asset.sha256) {
        return Err(err(
            "SHA-256 of the downloaded binary does not match the manifest",
        ));
    }
    install(&bytes)?;
    Ok(Outcome::Updated {
        from: VERSION.to_string(),
        to: manifest.version,
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// SHA-256 of a file, as lowercase hex.
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    Ok(hex(&Sha256::digest(std::fs::read(path)?)))
}

static EXE_PATH: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

/// Path of this executable. On Linux, once the file is replaced, `current_exe`
/// reports "<path> (deleted)", so the path is remembered before replacing.
fn exe_path() -> Option<std::path::PathBuf> {
    if let Some(p) = EXE_PATH.get() {
        return Some(p.clone());
    }
    let p = std::env::current_exe().ok()?;
    let s = p.to_string_lossy();
    Some(match s.strip_suffix(" (deleted)") {
        Some(t) => std::path::PathBuf::from(t),
        None => p,
    })
}

fn install(bytes: &[u8]) -> Result<(), Error> {
    let exe = exe_path().ok_or_else(|| err("cannot find own executable"))?;
    let _ = EXE_PATH.set(exe.clone());
    let dir = exe
        .parent()
        .ok_or_else(|| err("executable has no parent folder"))?;
    // Write next to the executable so the final rename stays on one file system.
    let tmp = dir.join(format!(".dyno-update-{}.tmp", std::process::id()));
    let result = (|| {
        let mut f = std::fs::File::create(&tmp)
            .map_err(|e| err(format!("cannot write to {}: {e}", dir.display())))?;
        f.write_all(bytes)
            .map_err(|e| err(format!("write failed: {e}")))?;
        f.sync_all().ok();
        drop(f);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| err(format!("chmod failed: {e}")))?;
        }
        self_replace::self_replace(&tmp).map_err(|e| err(format!("replace failed: {e}")))
    })();
    let _ = std::fs::remove_file(&tmp);
    result
}

/// Start the new executable with the same arguments, then exit this process.
pub fn restart() -> ! {
    let exe = exe_path().expect("current executable");
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let mut cmd = std::process::Command::new(exe);
    cmd.args(args).env(JUST_UPDATED_ENV, "1");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let e = cmd.exec();
        eprintln!("restart failed: {e}");
        std::process::exit(1);
    }
    #[cfg(not(unix))]
    {
        match cmd.status() {
            Ok(s) => std::process::exit(s.code().unwrap_or(0)),
            Err(e) => {
                eprintln!("restart failed: {e}");
                std::process::exit(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare() {
        assert!(is_newer("0.0.2", "0.0.1"));
        assert!(!is_newer("0.0.1", "0.0.1"));
        assert!(!is_newer("0.0.1", "0.0.2"));
        assert!(!is_newer("garbage", "0.0.1"));
    }

    #[test]
    fn rejects_bad_signature() {
        let m = br#"{"version":"9.9.9","protocol":1,"assets":{}}"#;
        assert!(verify_manifest(m, &[0u8; 64]).is_err());
        assert!(verify_manifest(m, &[0u8; 3]).is_err());
    }

    /// A manifest as .github/scripts/manifest.py writes it from 0.7 on.
    const BOTH: &str = r#"{
 "version": "0.7.0", "protocol": 16,
 "assets": {
  "x86_64-unknown-linux-gnu": {"name": "dyno-x86_64-unknown-linux-gnu", "size": 10, "sha256": "aa"},
  "x86_64-pc-windows-msvc": {"name": "dyno-x86_64-pc-windows-msvc.exe", "size": 11, "sha256": "bb"}
 },
 "programs": {
  "dyno": {
   "x86_64-unknown-linux-gnu": {"name": "dyno-x86_64-unknown-linux-gnu", "size": 10, "sha256": "aa"},
   "x86_64-pc-windows-msvc": {"name": "dyno-x86_64-pc-windows-msvc.exe", "size": 11, "sha256": "bb"}
  },
  "hearthblock": {
   "x86_64-unknown-linux-gnu": {"name": "hearthblock-x86_64-unknown-linux-gnu", "size": 5, "sha256": "cc"},
   "x86_64-pc-windows-msvc": {"name": "hearthblock-x86_64-pc-windows-msvc.exe", "size": 6, "sha256": "dd"}
  }
 }
}"#;

    #[test]
    fn picks_the_artifact_named_like_the_executable() {
        let m: Manifest = serde_json::from_str(BOTH).unwrap();
        let linux = "x86_64-unknown-linux-gnu";
        let win = "x86_64-pc-windows-msvc";
        let name = |exe: &str, target: &str| {
            pick(&m, program_of(Path::new(exe)), target)
                .map(|a| a.name.clone())
                .unwrap_or_default()
        };
        // the live server runs `dyno server --auto-update`: it stays dyno
        assert_eq!(
            name("/usr/local/bin/dyno", linux),
            "dyno-x86_64-unknown-linux-gnu"
        );
        assert_eq!(
            name(r"C:\Games\dyno.exe", win),
            "dyno-x86_64-pc-windows-msvc.exe"
        );
        // a downloaded or renamed dyno is still dyno
        assert_eq!(
            name("/opt/dyno-x86_64-unknown-linux-gnu", linux),
            "dyno-x86_64-unknown-linux-gnu"
        );
        assert_eq!(
            name("/home/a/bin/my-server", linux),
            "dyno-x86_64-unknown-linux-gnu"
        );
        // the player client updates as the player client
        assert_eq!(
            name("/home/a/bin/hearthblock", linux),
            "hearthblock-x86_64-unknown-linux-gnu"
        );
        assert_eq!(
            name(r"C:\Games\Hearthblock.exe", win),
            "hearthblock-x86_64-pc-windows-msvc.exe"
        );
        assert_eq!(
            name("/tmp/hearthblock-x86_64-unknown-linux-gnu", linux),
            "hearthblock-x86_64-unknown-linux-gnu"
        );
        // no hearthblock build for a target: an error, never the dyno binary
        assert!(pick(&m, "hearthblock", "aarch64-apple-darwin").is_err());
        let mut old = m.clone();
        old.programs.clear();
        assert!(pick(&old, "hearthblock", linux).is_err());
        assert!(pick(&old, "dyno", linux).is_ok());
    }

    #[test]
    fn updaters_before_the_split_still_get_dyno() {
        // the shape every released updater up to 0.6 parses (serde ignores `programs`)
        #[derive(Deserialize)]
        struct OldManifest {
            version: String,
            assets: BTreeMap<String, Asset>,
        }
        let old: OldManifest = serde_json::from_str(BOTH).unwrap();
        assert_eq!(old.version, "0.7.0");
        assert_eq!(
            old.assets["x86_64-unknown-linux-gnu"].name,
            "dyno-x86_64-unknown-linux-gnu"
        );
        // and a manifest from before the split parses with today's struct
        let m: Manifest = serde_json::from_str(
            r#"{"version":"0.6.0","protocol":15,"assets":{"t":{"name":"dyno-t","size":1,"sha256":"x"}}}"#,
        )
        .unwrap();
        assert_eq!(pick(&m, "dyno", "t").unwrap().name, "dyno-t");
    }

    #[test]
    fn public_key_is_valid() {
        assert!(VerifyingKey::from_bytes(&PUBLIC_KEY).is_ok());
    }
}
