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

**Manifest.** At start the server hashes every file (SHA-256) and builds:

```rust
pub struct Manifest { pub pack: String, pub version: u32, pub files: Vec<AssetRef> }
pub struct AssetRef { pub path: String, pub hash: [u8; 32], pub size: u32 }
```

`Welcome` carries the manifest instead of the whole content struct. The
rules JSON is an asset like any other.

**Transfer** runs on the same WebSocket, because the public address is one
port through a tunnel.

- `ClientMsg::Want { hashes: Vec<[u8; 32]> }`
- `ServerMsg::Asset { hash, offset: u32, total: u32, data: Vec<u8> }` in chunks of up to 64 KB.

The session writer sends game messages first and asset chunks only with the
bandwidth left over each tick (a budget per session, e.g. 256 KB/s), so play
never waits behind a download.

**Cache.** The client stores files by hash under the platform cache
directory (`directories`). On join it asks only for hashes it lacks. A
second join with nothing changed downloads nothing. A loading screen shows
progress for the first join.

**Cluster.** The pack hash joins the protocol and content versions in the
instance handshake; a mismatched instance drains.

## 2. Assets needed before play vs streamed

- **Before entering the world:** rules, blocks, render settings, textures
  for blocks, the player's own and nearby figures' models, rigs and clips,
  fonts, UI sounds.
- **Streamed as needed:** models and sounds for creatures and items when
  first seen (the engine draws a neutral placeholder until they arrive),
  detail shapes for blocks in streamed sections.

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

## 8. Order of work

1. Pack, manifest, transfer and cache; the binary split. The content pack
   becomes the first streamed asset.
2. In parallel, bake each asset class and delete its generator once an
   image diff (`dyno shot`, the animation sheet) matches:
   - textures, sounds, fonts and render settings;
   - models, rigs, clips and detail shapes.
3. World streaming and area of interest; stamps and layout.
4. Authoritative movement.

Every step keeps `dyno shot`, the bots and the protocol tests working.
