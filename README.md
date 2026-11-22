# Dyno

Dyno is an open-source multiplayer engine for block worlds and 3D games. The goal is that all game logic is Luau code and data in a live content store, so a game can change while players are connected. The full design is in [docs/spec/index.html](docs/spec/index.html).

**Status: bootstrap build.** This build proves the release pipeline and the signed auto-updater, and gives a small shared block world to build in together.

## Play

1. Download the file for your system from the [latest release](https://github.com/mortondev/dyno/releases/latest):

   | System | File |
   |---|---|
   | Windows (64-bit) | `dyno-x86_64-pc-windows-msvc.exe` |
   | macOS (Apple silicon) | `dyno-aarch64-apple-darwin` |
   | macOS (Intel) | `dyno-x86_64-apple-darwin` |
   | Linux (x86_64) | `dyno-x86_64-unknown-linux-gnu` |

2. Start it.
   - **Windows:** double-click. If SmartScreen appears, choose *More info*, then *Run anyway*.
   - **macOS:** in Terminal: `chmod +x dyno-aarch64-apple-darwin && xattr -d com.apple.quarantine dyno-aarch64-apple-darwin; ./dyno-aarch64-apple-darwin`
   - **Linux:** `chmod +x dyno-x86_64-unknown-linux-gnu && ./dyno-x86_64-unknown-linux-gnu`
3. Enter your name and the server address, then press **Join**. The address is saved for next time.

The address can be `192.168.1.20`, `192.168.1.20:7400`, `[::1]:7400`, a host name, or a tunnel address such as `https://name.trycloudflare.com`.

### Controls

| Key | Action |
|---|---|
| Mouse, W A S D, Space | Look, move, jump |
| Ctrl or R | Sprint |
| Left mouse | Attack the creature you look at; hold to gather from trees, ore and bushes |
| Right mouse | Talk to people (a yellow **!** means they have a quest for you, **?** means you can hand one in); eat; open chests |
| Tab | Target the nearest creature in front of you |
| L | Quest log |
| E | Pack and recipes |
| 1 to 9, wheel | Hotbar |
| Q / Ctrl+Q | Drop one / the stack |
| T or Enter, / | Chat, commands (`/help`) |
| Esc | Menu, or close a window |

Operators building the world use `/gamemode creative` for flight, free blocks and the fill tool (G, V, X, Ctrl+Z).

### Hearthblock Online in short

You wake by the cold ruins of the Great Hearth in Hearthstead. Talk to Old Brannoc, Captain Maren, Tilda and Hollis to pick up quests: hunt Grazers, gather wood, coal and iron, find the old North Watch beacon, and fight the Shades that rise at night. Quests and kills give experience; you level up from 1 to 5 in the first chain, with gold and gear as rewards. The land is protected, but trees, ore and bushes grow back after you gather them. You never lose items when you die. The plan for classes, gear, zones and a dungeon is in [docs/design/rpg.md](docs/design/rpg.md).

## Host a world

**From the game:** press *Host a world on this computer* on the connect screen. Friends on the same network join with the address shown in the menu. For friends on the internet, press *Share over the internet*. This needs [cloudflared](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/) and gives you an `https://…trycloudflare.com` address to send them. No port forwarding is needed.

**Dedicated server:**

```sh
dyno server --port 7400 --world ./world/world.dynw --motd "Welcome!"
# in another terminal, for internet players without port forwarding:
cloudflared tunnel --url http://localhost:7400
```

The server saves the world every 10 seconds. `--auto-update` makes it install new releases and restart.

## Updates

Dyno checks GitHub Releases each time it starts. Each release has a manifest signed with an Ed25519 key, and each binary is checked with SHA-256 before it replaces the old one. If anything fails, Dyno keeps running the version you have. Set `DYNO_NO_UPDATE=1` to turn updates off, or run `dyno update` to check by hand.

Releases are built by [.github/workflows/release.yml](.github/workflows/release.yml) when a `v*` tag is pushed.

## Commands

```text
dyno                     Open the game
dyno connect <address>   Open the game with the address filled in
dyno server [options]    Run a dedicated server
dyno ping <address>      Join, print server information, leave (for tests and agents)
dyno say <address> <text>  Join, send chat or a /command, print replies, leave
dyno shot <address> [--pos x,y,z] [--look yaw,pitch] [--time 0-1] [--size WxH] [--out f.ppm]
                         Join, render one frame off-screen to an image, leave
dyno fill <address> x,y,z x,y,z <block>   Fill a box with a block (Air clears it)
dyno bot <address> [name] [seconds]      A bot that walks in a circle (demos, tests)
dyno update              Check for a new release and install it
dyno version             Print the version
```

`DYNO_AUTOJOIN=1 dyno connect <address>` joins without a click, and `DYNO_AUTOJOIN=host dyno` hosts and joins. `DYNO_SCREENSHOT=file.ppm` saves the view every 3 seconds while playing. These are for scripted tests and AI agents.

## What is in the world

- A 256 × 96 × 256 world: meadows, oak and birch forests, pine-covered hills, snowy mountains and lakes with sandy shores.
- A shared day and night cycle (20 minutes). Drifting clouds, sunsets, stars, fireflies and crickets, and a dark night where glowing blocks matter.
- The server remembers where you were, so you rejoin where you left off.
- 36 blocks: natural blocks, building blocks, 10 colours, glass, ice, and light sources (Lantern, Emberstone, Glowshroom) that light up their surroundings.
- Sounds for breaking, placing and footsteps, and particles when blocks break. All sounds are generated in code.

## Where this build differs from the spec

This is a bootstrap slice. These parts of [the spec](docs/spec/index.html) are not built yet:

- **No Luau yet.** Game rules are Rust (`crates/dyno-sim`, `crates/dyno-server`). Items, recipes, block drops, creatures and milestones are data in [content/hearthblock.json](content/hearthblock.json), sent to players on join; `/reload` changes them live. This is the first step toward the spec's content store.
- **Transport is WebSocket over TCP**, not QUIC (NET-001), so the server also works through HTTPS tunnels.
- **Movement is client-authoritative.** The server checks reach, break time, item use and crafting; falls and drowning are reported by the client (spec PRED-001 to PRED-016).
- **One fixed-size world** (256 × 96 × 256) sent whole on join, not streamed chunks.
- **One release signing key**, not the root key list of REL-013. The private key stays offline with the maintainer; CI holds a copy as an Actions secret.
- **No accounts.** Anyone who knows the address can join. Names are not reserved.

## Licence

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
