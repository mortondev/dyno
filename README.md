# Dyno

Dyno is an open-source multiplayer engine. Its flagship game is **Hearthblock Online**, a classic fantasy MMORPG: six classes, quests and level progression, gear, loot, merchants, phasing and one shared world that shards itself automatically. The engine's original design is in [docs/spec/index.html](docs/spec/index.html); how it is actually built is in **[docs/architecture.md](docs/architecture.md)**.

**Status: playable alpha.** Ashfall Vale (levels 1–10) is open, with the signed auto-updater keeping players and servers current.

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
3. Press **Play**, create a character (class, appearance, name) and press **Enter World**. There are no realms to choose: there is one world, and the game places you in it.

Builds made without a world address built in (`DYNO_DEFAULT_SERVER`) ask for one under **Options**. It can be `192.168.1.20`, `192.168.1.20:7400`, a host name, or a tunnel address such as `https://name.trycloudflare.com`.

### Controls

Classic MMO controls: the cursor is free, and you hold a mouse button to steer.

| Input | Action |
|---|---|
| Hold right mouse | Steer: turn your character and camera together |
| Hold left mouse | Look around your character without turning |
| Hold both mouse buttons | Run forward |
| W / S | Run forward / back up |
| A / D | Turn (strafe while holding right mouse) |
| Q / E | Strafe |
| R | Autorun |
| Space | Jump (you walk up one-block steps without jumping) |
| Mouse wheel | Zoom the camera; all the way in is first person |
| Left click | Select the creature or person under the cursor |
| Right click | Attack a creature, talk to a person, gather from a tree, ore vein or bush |
| Tab | Target the nearest creature ahead |
| 1 to 0, -, = | Action bar: class abilities; Recall on -, eat on = |
| C / B / P / K / L | Character, Bags, Spellbook, Skills, Quest log (press again to close) |
| Esc | Close windows, then clear your target, then the menu |
| T or Enter, / | Chat and commands (`/help`) |

Operators building the world use `/gamemode creative`, which switches back to free-flying build controls.

### Hearthblock Online in short

Pick a class (Warrior, Paladin, Hunter, Rogue, Priest or Mage) and wake in Hearthstead, a village around the cold ruins of the Great Hearth. Talk to Old Brannoc, Captain Maren, Tilda and Hollis to pick up quests: hunt Grazers, gather wood, coal and iron, find the old North Watch beacon, and fight the Shades that rise at night. Quests and kills give experience; you level up from 1 to 5 in the first chain, with gold and gear as rewards. The land is protected, but trees, ore and bushes grow back after you gather them. You never lose items when you die. The plan for classes, gear, zones and a dungeon is in [docs/design/rpg.md](docs/design/rpg.md).

## Host a world

**From the game (developers):** *Options → Host a local world* runs a world on your computer. *Share over the internet* needs [cloudflared](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/) and gives an `https://…trycloudflare.com` address. No port forwarding is needed.

**Dedicated server:**

```sh
dyno server --port 7400 --world ./world/world.dynw --motd "Welcome!"
# in another terminal, for internet players without port forwarding:
cloudflared tunnel --url http://localhost:7400
```

The server saves the world every 10 seconds. `--auto-update` makes it install new releases and restart.

The server splits the world into shards by itself: `--shard-cap 40` players per shard, `--area-cap 16` players in one area of a shard before newcomers are spread out, `--max-shards 16`. Parties are kept together. See [docs/architecture.md](docs/architecture.md).

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

- **Ashfall Vale**, 384 × 96 × 384 blocks: Hearthstead and its market, the North Watch, Birchrise hills, the Hollow Oak, Barrowwood and its crypt, Mirrormere lake, the Emberrun river and its ford, the windmill farm, Fisher's Landing, the Sunken Shrine, the East Road and the Ashcloak Stockade, Miner's Rest and the three-level Hollow Mines.
- **Six classes** (Warrior, Paladin, Hunter, Rogue, Priest, Mage) with rage, mana or energy, a spellbook and an action bar; levels 1–20; 35 quests in chains; gear in 17 slots; professions.
- **Classic loop:** loot corpses by hand, sell to and buy from merchants, rest in town for rested XP, group up in parties.
- **A living world:** camps that respawn, rare elites, weather, the Nightfall Incursion, and phasing (the world changes for you as your story moves on).
- **Classic controls:** hold the right mouse button to steer, both to run, A/D to turn, Q/E to strafe; a minimap; character, bags, spellbook and skills windows.

## Where this build differs from the spec

These parts of [the spec](docs/spec/index.html) are not built yet:

- **No Luau yet.** Game rules are Rust (`crates/dyno-sim`, `crates/dyno-server`); content is data in [content/hearthblock.json](content/hearthblock.json), sent to players on join; `/reload` changes it live.
- **Transport is WebSocket over TCP**, not QUIC (NET-001), so the server also works through HTTPS tunnels.
- **Movement is client-authoritative**, checked by the server.
- **The world is sent whole on join**, not streamed in chunks.
- **One server instance today.** Running shards across many instances with a quorum is designed and in progress ([docs/architecture.md](docs/architecture.md), section 5).
- **One release signing key**, not the root key list of REL-013. The private key stays offline with the maintainer; CI holds a copy as an Actions secret.
- **No accounts.** A character belongs to the install that made it.

## Licence

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
