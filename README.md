# Dyno

Dyno is an open-source multiplayer engine for block worlds and 3D games. The goal is that all game logic is Luau code and data in a live content store, so a game can change while players are connected. The full design is in [docs/spec/index.html](docs/spec/index.html).

**Status: bootstrap build.** This early build proves the release pipeline and the signed auto-updater, and gives a small shared block world to build in together.

## Download and run

Download the file for your system from the [latest release](https://github.com/mortondev/dyno/releases/latest):

| System | File |
|---|---|
| Windows (64-bit) | `dyno-x86_64-pc-windows-msvc.exe` |
| macOS (Apple silicon) | `dyno-aarch64-apple-darwin` |
| macOS (Intel) | `dyno-x86_64-apple-darwin` |
| Linux (x86_64) | `dyno-x86_64-unknown-linux-gnu` |

Dyno updates itself at start from GitHub Releases. Each release manifest is signed with an Ed25519 key, and each binary is checked with SHA-256 before it replaces the old one. Set `DYNO_NO_UPDATE=1` to turn this off.

## Licence

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
