# Content is data; the server streams it; the client is tiny

**Core principle.** All content is data, not code. The server is fully
authoritative and streams the world and every asset to clients as they need
them. The client is an engine: a renderer, an interpreter for the data
formats below, input, audio and the protocol. It carries no game content.

This document is the contract every part of the migration builds to.

## Where we start (inventory, v0.6.0)

| Decided or held by the client today | Becomes |
|---|---|
| Movement (client simulates and reports; the server checks) | Server simulates from inputs; client predicts and reconciles |
| Fall and drowning damage (client reports) | Server computes from its own simulation |
| Block textures (`textures.rs`, procedural) | PNG assets in the pack |
| Character and creature models (`models.rs`, `voxmodel.rs`) | `.vox` models plus rig JSON |
| Animation poses (`anim.rs`, procedural per skeleton) | Animation clip JSON on named bones; checked by a pack test, not the compiler |
| Detail shapes (`detail.rs`: roofs, timber, props, windows, cloth) | `.vox` shape assets with variant rules in block definitions |
| Terrain tags (`terrain_mesh.rs`: terrain, paving, chamfer) | Tags in block definitions |
| Sounds (`audio.rs`, synthesised) | WAV assets |
| Fonts (embedded TTFs) | Font assets |
| Render look (shader constants: leaf, relief, grade) | A render-settings asset |
| The content pack (`include_str!` of hearthblock.json) | Sent by the server only |
| Title-screen stage (client calls `World::generate` and `build_hearthstead`) | A small baked stage scene streamed like any other asset |
| Block table (`dyno-world/src/lib.rs`) | `blocks.json` |
| World builders (`village.rs`, `poi.rs`, `zones.rs`, worldgen trees) | Stamps (block volumes) and a layout file; terrain shaping as data operations |

Size floor: a stripped, size-optimised program with only wgpu, egui, winit,
rodio and TLS WebSockets is **5.7 MB**. Today's `dyno` is 20.6 MB with the
server, tools and content in it. Target for the client: **under 8 MB**, with
no content strings in it (a test greps the binary for names like
`thornboar` and `Hearthstead`).

## 1. The pack

A pack is a directory the server loads at start:

```
content/
  pack.json            # name, version, entry points (rules, blocks, render, stage)
  rules/hearthblock.json   # today's content: items, creatures, quests, classes…
  blocks.json          # block definitions (§3)
  render.json          # shader settings
  textures/*.png
  models/*.vox         # characters, creatures, held items
  rigs/*.json          # skeletons: bones, parents, pivots, part → model mapping
  anims/*.json         # clips per skeleton
  shapes/*.vox         # detail shapes for blocks (fine voxels)
  sounds/*.wav
  fonts/*.ttf
  stamps/*.dstamp      # block volumes for buildings and sites (§5)
  world/layout.json    # where stamps go; terrain operations; vegetation tables
  stage/               # the title-screen scene
```

**Status: built** (protocol 17). Today's pack: `pack.json`, the rules
(`rules/hearthblock.json`), `blocks.json`, `render.json`, `fonts/`,
`textures/`, `icons/`, `sounds/` (from the textures and sounds bake),
`ui/strings.json` (loading screen, intro, class prompt, news) and
`stage/title.dynw` (the title stage): 328 files, 5.3 MB, 4.1 MB of it sounds.

`pack.json`:

```json
{ "name": "hearthblock", "version": 1,
  "entries": { "rules": "rules/hearthblock.json", "blocks": "blocks.json",
               "render": "render.json", "fonts": "fonts/fonts.json",
               "strings": "ui/strings.json", "stage": "stage/title.dynw" },
  "stream": [] }
```

**Loading** (`dyno-server/src/pack.rs`). `--content` takes a pack
directory, or a rules JSON file (the built-in pack with those rules: tests
and custom rules). Without it the server runs the **built-in pack**: the
workspace's `content/` directory, embedded in the server by its build script,
because the live server is a lone release binary with no files beside it. The
old `--content content/hearthblock.json` still works: when that file is
missing, the pack around it is loaded. `/reload` reloads the whole pack.

**Manifest.** At start the server hashes every file (SHA-256) and builds:

```rust
pub struct Manifest { pub pack: String, pub version: u32, pub files: Vec<AssetRef>, pub stream: Vec<String> }
pub struct AssetRef { pub path: String, pub hash: [u8; 32], pub size: u32 }
```

`files[0]` is the rules JSON; the rest follow in path order, so the same
files give the same manifest. The pack hash is the SHA-256 of the encoded
manifest. `stream` lists path prefixes fetched when first needed (§2);
`pack.json`, `blocks.json` and the rules are always fetched before play,
because texture layer numbers depend on `blocks.json` and nothing remeshes
if it arrives late (the client also reloads textures before meshing a
joined world).

`Welcome` carries the manifest instead of the whole content struct. The
rules JSON is an asset like any other: the client parses `Content` from
the bytes it received on that connection. After `/reload` the server sends
`ServerMsg::Manifest`; the client fetches what changed, and when new rules
arrive the game gets `NetEvent::Content`.

**Transfer** runs on the same WebSocket, because the public address is one
port through a tunnel.

- `ClientMsg::Want { hashes: Vec<[u8; 32]> }` (at most 1024 per message)
- `ServerMsg::Asset { hash, offset: u32, total: u32, data: Vec<u8> }` in chunks of up to 64 KB.

Each session has an `Outbox` (a queue and a token bucket). Every pass of the
session loop writes the game messages waiting for it first, then the asset
chunks the budget allows: 256 KB/s by default once the player plays
(`--asset-rate KB`, `Config::asset_rate`), and 2 MB/s before that, while
the loading screen is up and nothing competes (`--join-asset-rate KB`); a
64 KB burst, chunks of 16–64 KB. The rate drops at the first message from
the client that is not a `Want`. Measured: a
640 KB streamed file took 2.5 s (budget 2.25 s) while `/who` was answered
in at most 11 ms throughout.

**Relayed sessions** (cluster): the edge a client reached answers `Want`
itself from its own pack and never relays it; it rewrites the host's
`Welcome` to name its own manifest. Mismatched instances drain, so the
packs agree anyway.

**Cache** (`dyno-client/src/cache.rs`). The client stores files by hash
under the platform cache directory (`directories`; `DYNO_CACHE_DIR`
overrides it), checking each file's hash when it is written and when it is
read. On join it asks only for hashes it lacks, keeps playing messages that
arrive during the download, and returns when the files needed before play
are present (`net::connect_with`; `Joined::downloaded` counts the bytes). It
pings the server during long downloads and gives up after 30 s with no
progress. A second join with nothing changed downloads nothing: 4,290,437
bytes on the first join, 0 on the second (`tests/streaming.rs`). It also keeps
the last manifest, so the title screen shows the last world's stage, news
and strings before any join. The loading screen shows the download.

**Cluster.** The pack hash joins the protocol and content versions in the
instance handshake (the member's content identity is
`"<rules name> v<version> pack <hash>"`); a mismatched instance drains, even
when only one non-rules file differs.

**Tools and tests** load a local pack with `assets::load_dir` or
`assets::load_local_pack` (working directory, next to the executable, then
the source tree), or the built-in one with
`dyno_client::load_builtin_pack()`. `load_local_pack` does nothing in the
player client (a release build without `host`): it works from the
streamed pack and its cache alone. Before its first join ever it shows a
plain title backdrop, egui's own fonts and flat block colours.

## 2. Assets needed before play vs streamed

- **Before entering the world:** rules, blocks, render settings, textures
  for blocks, the player's own and nearby figures' models, rigs and clips,
  fonts, UI sounds.
- **Streamed as needed:** models and sounds for creatures and items when
  first seen (the engine draws a neutral placeholder until they arrive),
  detail shapes for blocks in streamed sections.

**Built:** a pack lists streamed prefixes in `pack.json` (`"stream":
["models/creatures/", …]`); the join fetches everything else. Engine code
that finds `assets::get(path)` empty calls `assets::request(path)`; the
connection sends `Want` and the file arrives through `assets::insert`
during play, within the session's budget. Today's pack streams nothing;
the asset bakers choose prefixes as their files land.

## 3. Formats

- **Blocks** (`blocks.json`): per block id
  - name, solid, opaque, light emitted, material (sounds)
  - textures per face (pack paths)
  - tags: `terrain`, `paving`, `leaves`, `water`, `chamfer`
  - optional `shape`: a list of rules from a neighbour pattern to a `.vox`
    shape and rotation, which replaces the code in `detail.rs`.

  IDs stay stable; saves store IDs.
- **Models:** MagicaVoxel `.vox`. One file per figure, with named models per
  part (`head`, `torso`, `arm_l`, …) so artists can edit them in MagicaVoxel.
  The voxel size comes from the rig (1/32 m for figures; heads may declare
  their own scale).
- **Rigs** (`rigs/<skeleton>.json`): bones with name, parent, pivot in
  voxels, and the part model drawn on each; grip and attachment points.
- **Clips** (`anims/<skeleton>.json`): for each animation name, a duration
  or loop, keyframes per bone (rotation in degrees, offset), and
  `stride: metres per cycle` for locomotion so feet don't skate. The engine
  blends and layers as `anim.rs` does today. Bake today's procedural poses
  into keyframes by sampling them. A **pack test** fails if any figure lacks
  any standard animation, replacing the compile-time check.
- **Sounds:** WAV (16-bit PCM). Bake today's synthesised sounds to files.
- **Textures:** PNG, 32 px per block face, atlases built by the client at load.
- **Stamps** (`.dstamp`): a header (size, anchor) and RLE block IDs.

## 4. World streaming

- The world is stored in sections of 16 × 16 × 16. The server sends a
  client the sections within its view radius (about 160 m) around the
  player, nearest first, RLE compressed, and section edits as they happen.
  Sections leaving the radius are dropped from client memory.
- Entities and players use **area of interest**: a client hears about
  entities and poses within about 100 m only. This also fixes the
  quadratic pose fan-out.
- Code that assumes a whole `World` on the client moves to a sparse store:
  light, minimap, ground height, picking, the terrain mesher, and the
  shard-move block diff (which becomes "resend the sections you have").
- The title screen's stage is a small baked scene in the pack.

## 5. World building as data

The live world contains migrated player builds, so it is never regenerated.
For new worlds and rebuilds:

- `World::generate` (terrain from a seed) stays code: it is an engine
  generator driven by parameters in `layout.json`.
- Every building, site and town is a **stamp**, placed by `layout.json`
  with position, rotation and a ground-fitting rule.
- Today's builders are run once by a bake tool to produce the stamps, and
  are then deleted.
- Vegetation comes from species tables (size ranges and crown shapes) in
  `layout.json`.

## 6. Authoritative movement (last)

1. Move player physics from `dyno-client/src/game.rs` into `dyno-sim` on a
   fixed timestep (60 Hz), deterministic for the same inputs.
2. The client sends `Input { seq, dt_ticks, forward, right, jump, down, sprint, yaw, pitch }`
   instead of `Pose`.
3. The server simulates each player from inputs, computes falls, drowning
   and environment damage itself, and sends each client its own
   authoritative state with the last input `seq` applied.
4. The client predicts with the same code and replays unacknowledged inputs
   on correction; other players are interpolated.
5. Keep today's checked `Pose` path until bots under injected latency
   (100–200 ms, jitter) show almost no corrections; then remove it.

## 7. Binaries and updates

- `dyno`: server, tools, bots and a dev client. It keeps its name and update
  channel, because the live server updates itself as `dyno`.
- `hearthblock`: the player client. It has no server, world builders or
  content, and hosting a local world is a dev-binary feature.
- Releases publish both. The updater picks the artifact that matches its
  own executable's name. Rehearse the live server's update on a copy before
  shipping.

**Built:**

- `crates/hearthblock` links `dyno-client` without its `host` feature, so
  it has no `dyno-server`, no world builders and no rules JSON. `dyno`
  turns `host` on (local hosting, the built-in pack for its title screen
  and `dyno shot --front`).
- Content strings left the client's code: the loading screen, intro and
  news come from `ui/strings.json`; the title stage is `stage/title.dynw`
  (128 × 64 × 128, 124 KB), baked with `dyno bake-stage`. Before the first
  join ever, the title screen has a plain backdrop and no news.
- `.github/scripts/check-client.sh` builds the client as it ships
  (`--profile client-release`: `opt-level = "z"`, fat LTO, one codegen
  unit, stripped, `panic = "abort"`) and fails if the binary contains
  `thornboar`, `Hearthstead` or `Brannoc`, or is 10 MB or more (a ceiling
  against regressions). CI runs it, with `cargo clippy -p hearthblock`,
  because `--workspace` turns `host` on everywhere.
- **Size: 9.32 MB** (Linux x86-64, 0.6.2 with the textures and sounds
  bake): over the 8 MB target. It was 10.89 MB before the bake merge, and
  9.54 MB after it with `panic = "abort"`; dropping egui-winit's `links`
  feature (webbrowser) took off 0.22 MB. In a symbol breakdown of the
  pre-merge build, unwind tables were 1.4 MB, the client's own code about
  0.85 MB and naga 0.73 MB; the
  rest is engine libraries: naga (0.73 MB), wgpu core/hal, winit with its
  Wayland and X11 stacks and tiny-skia window decorations, rustls and ring
  (WebSockets and the updater share one copy), egui, and egui's built-in
  fonts (needed: before its first join the client has no pack fonts).
  Next steps are product calls, not code: one GPU backend per platform
  (drop GL where Vulkan or Metal exists: `glow` and naga's GLSL output come
  from wgpu's default `gles`), winit without the Adwaita window decorations
  on Wayland (tiny-skia), a smaller fallback font. The size gate fails only
  on Linux, where it was measured; other targets warn.
- The release manifest keeps `assets` as the `dyno` binaries, which every
  updater up to 0.6 reads (the live server among them), and adds
  `programs: { dyno: {…}, hearthblock: {…} }`. From 0.7 the updater picks
  by its executable's file name: `hearthblock*` gets hearthblock, anything
  else gets dyno; a missing hearthblock build is an error, never the dyno
  binary (`dyno-update` unit tests).

## 8. Order of work

1. Pack, manifest, transfer and cache; the binary split. The content pack
   becomes the first streamed asset. **(Done.)**
2. In parallel, bake each asset class and delete its generator once an
   image diff (`dyno shot`, the animation sheet) matches:
   - textures, sounds, fonts and render settings;
   - models, rigs, clips and detail shapes.
3. World streaming and area of interest; stamps and layout.
4. Authoritative movement.

Every step keeps `dyno shot`, the bots and the protocol tests working.
