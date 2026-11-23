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

**Status: built** (protocol 18: compressed transfer). Today's pack: `pack.json`, the rules
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
  "stream": ["models/creatures/", "models/held/", "rigs/multileg", "anims/multileg", …] }
```

**The pack build** (`dyno_server::pack::compile`, run by `Pack::from_files`
on every load, so the embedded pack, a pack directory and `/reload` all get
it): source files become what is streamed. Today one rule: each
`anims/<skeleton>.json` (clips as artists edit them) becomes
`anims/<skeleton>.clips` (`dyno_protocol::clips`: keys thinned within a
tolerance, quantised, shared, in binary; 4.0 MB of JSON becomes 178 KB,
106 KB on the wire). The source is not in the manifest. The client reads
either: the compact form from the server, the source from a local pack.

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

Files travel compressed (`dyno_protocol::wire`). The server encodes each
file once (per process, by hash, on every core) with the smallest of:
`RAW` (PNGs, which do not shrink), `DEFLATE` (raw deflate via
`miniz_oxide`, which the client already links for PNG) and `PCM16` (a 16-bit
WAV's samples less their order-2 prediction, split into low and high byte
planes, then deflated: lossless, 2.3 MB for 4.0 MB of sounds where deflate
alone gives 3.0 MB). `AssetRef` carries the `codec` and the `wire` size
beside the original `size` and hash; chunks add up to `wire`. The client
inflates with a limit of `size`, checks the hash of the original bytes and
caches the original, so files are always named by what they are.

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
progress. A second join with nothing changed downloads nothing
(`tests/streaming.rs`, which also holds a first join under 4 MB).
`Joined::downloaded` and the loading bar count bytes on the wire.

| First join | Bytes |
|---|---|
| 0.7.0, before figures and shapes moved to the pack | 4,290,976 |
| figures, clips and shapes as pack data, uncompressed | 12,485,452 |
| compressed, compact clips, creatures and held items streamed | 3,037,774 |

The cache also keeps
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
during play, within the session's budget. Today's pack streams creature
models (`models/creatures/`), held items (`models/held/`) and the rigs and
clips of every skeleton but the biped: `figure.rs` asks for a figure's
`.vox`, rig and clips when it first draws one, and draws the stand-in until
they arrive. The person figure, its rig and clips come before play. Files
arriving in play do not throw away figures already built: the figure
library keeps the addresses of the files it used and starts again only
when one of them is replaced.

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
- **Clips** (`anims/<skeleton>.json`, compiled to `.clips` by the pack
  build): for each animation name, a duration
  or loop, keyframes per bone (rotation in degrees, offset), and
  `stride: metres per cycle` for locomotion so feet don't skate. The engine
  blends and layers as `anim.rs` does today. Bake today's procedural poses
  into keyframes by sampling them. A **pack test** fails if any figure lacks
  any standard animation, replacing the compile-time check.
- **Sounds:** WAV (16-bit PCM). Bake today's synthesised sounds to files.
- **Textures:** PNG, 32 px per block face, atlases built by the client at load.
- **Stamps** (`.dstamp`): a header (size, anchor) and RLE block IDs.
- **Water** (`render.json`, `water`; drawn by `water.rs`):
  - the base look: `deep`, `shallow` and `foam` colours, `depth` (metres of
    water at which the colour is fully deep), `clarity` (metres you see
    through), `foam_width`, `reflect`, `ripple`, `sparkle`, `flow` (metres a
    second along a course), `fog` with `fog_density` for eyes under water,
    and `lilies` (how thick lily pads lie on calm shallows) with the `lily`
    and `lily_flower` colours;
  - `kinds`: named kinds of water, each changing what it names of the base
    (the river, the shrine's turquoise, Barrowwood's dark water);
  - `areas`: still water of a kind, an ellipse `at` a place with a `radius`,
    from the spawn hearth;
  - `courses`: running water of a kind along `points` (x, z, half width, from
    the spawn hearth); it flows in the order of the points.

  The shader holds the base look and seven kinds, eight areas and 32 course
  points. The courses repeat the river layout that the world builder still
  has in code; they move to the world layout file with it (§5).
- **Ground cover** (`cover.json`, `cover/*.vox`; `dyno-client/src/cover.rs`):
  grass tufts, flowers, ferns, reeds, bushes, pebbles and small rocks.
  - `kinds`: a `.vox` file each, every model in it a variant; `voxel`
    (metres per voxel), `sway` (wind) and `soft` (0 lit as a solid, 1 lit
    evenly like the ground).
  - `layers`: scatter rules. Each names its `kinds`, the blocks it stands
    `on`, `per_block` (how many per block top), and what thins or gathers
    it: `patch` (noise: none below `from`, all above `to`), `near` and
    `away` (blocks within 1–3 blocks: water, paving, trunks), `sky` (a range
    of sky light), `tint`, and `areas` (density and tint per area key of the
    rules, `elsewhere` for the rest of the map).
  - `range`, `near`, `far` (per layer) and `detail`: everything grows within
    `near` metres of the camera; beyond it each layer thins to its `far`
    share at `range`; beyond `detail` a model is drawn column by column.
  - The mesher hands over each open top of the ground; the scatter runs on
    the world thread; the renderer draws the spots near the camera
    instanced, sorted by model. The player's `ground_cover` setting (0–2;
    `DYNO_COVER` for tools) scales density and range.
  - The models are made by `tools/cover/make.py` and may be edited in
    MagicaVoxel. `boulder.vox` and `stepping_stone.vox` are in the pack for
    the world builders: cover has no collision, so anything a player should
    stand on or walk round belongs in the world as blocks.
- **Foliage** (`foliage.json`; `dyno-client/src/foliage.rs`): leaf crowns as
  clumps of cubes. `cells` cubes along a block (0: the smooth canopy), `erode`
  (how much of a crown's outside is worn away), `crease`, `under`, `depth`
  and `vary` (shading), `leaves` (colours per leaf block key), `trunks` (the
  blocks trees are made of) and `areas` (per area key: the `blocks` that
  turn, and `tones` with the `share` of trees each takes: autumn, blossom).
  Which cells hold leaves stays the world's business.

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

**Status: built** (protocol 19; the 2 km world: 21).

- **Sections on the wire.** `ServerMsg::Sections(Vec<(pos, data)>)`:
  section positions with each section's cells run-length encoded
  (`World::encode_section`: (block, varint count) pairs over the 4096
  cells; open air is 3 bytes). `SectionsGone(Vec<pos>)` drops them.
  `Welcome` carries the spawn `hearth` (content positions are relative to
  it, and the client can no longer find it by scanning a world it does not
  hold) and the `view` radius. `ServerMsg::World` is gone.
- **Server** (`dyno-server/src/stream.rs`). Each player (and observer) has
  a `View`: the sections sent, a list of those still wanted (nearest
  first, the layer you stand in before those above and below; remade only
  when you cross into another column) and a token bucket. At the end of
  each tick the shard sends each view one `Sections` batch within its
  budget, on the player's own ordered queue, so it works unchanged for
  relayed players (the edge holds no world). A section counts as held when
  it is queued, and the queue is ordered, so an edit can never overtake
  its section; `broadcast` sends `Block` and `Fill` only to views that hold
  the section. Budgets: `join_rate` (2 MB/s) until the first view is
  complete, then `rate` (256 KB/s); at most a tick's worth plus 0.1 s
  banked, so a game message waits behind at most one tick's batch. Columns
  past `view + keep` (160 + 24 m) are dropped. Flags: `--view`,
  `--section-rate`, `--join-section-rate`.
- **Area of interest** (`--aoi`, 100 m, forgotten past 112 m). Each
  player has the set of entities its client knows. The interest pass
  (every tick) introduces entities that come within range and are visible
  in the player's phases (`entity_intro`), and sends `EntityGone` for those
  past the margin. `broadcast` routes every message about an entity
  (spawn, hurt, info, name, damage, gone, its actions, its moves) only to
  players who know it, so phasing applies on top; a player's actions,
  casts and digging, and spell effects, go only to players within range.
  Poses go only to players within range; `PlayersFar` tells a client to
  stop drawing those who left it. The roster (`PlayerJoined`/`PlayerLeft`,
  `Wielding`, level-ups, parties, chat) stays world-wide. Observers (tools)
  hear every entity and pose, as before.
- **Shard moves** keep the client's sections: the `Shard` message carries
  only the blocks that differ in the sections the client holds. Moves
  between instances pass the held set in the cluster's `Adopt`.
- **Client.** `World::sparse` holds only the sections that arrived (open
  air and solid rock as one block per section). A cell whose section has
  not arrived is *unknown*: `get` answers air (nothing drawn, rays pass
  through, the minimap shows it dark), `solid` answers true and movement
  stops in place where the ground has not arrived (`dyno-sim` movement), so
  nobody falls through land that is still on its way. `Light` is stored
  per section, both levels in one byte, open sky not stored.
- **The world thread** (`dyno-client/src/worldwork.rs`). Lighting and
  meshing never run on the main thread in play. The game keeps its own
  copy of the world (movement, picking, minimap) and of its light (figures
  are lit by it); a worker thread keeps another copy of both and is sent
  every section that arrives or leaves, every block edit and, when it moves
  or turns, where the camera is. It lights whole columns once they are here
  (`Light::arrived`, scattered columns on every core), meshes the sections
  that are ready (a section and its 26 neighbours are here: the mesh and
  the light read past its edge), edits first, then nearest the camera and
  in front of it, 16 at a time, and sends back the meshes and copies of the
  light sections it changed (`Light::take_changed`). An arrival meshes only
  the neighbours it made ready (and remeshes those below it, whose sky it
  may shade). The main thread copies the light in and uploads meshes until
  3 ms of the frame are spent (`world_uploads` in `app.rs`); each section is
  one vertex buffer (opaque, translucent, then lamp halos) filled through
  the queue's staging memory. Meshes for a section dropped since are
  ignored. `DYNO_WORLD_INLINE=1` does the same work on the main thread, a
  step a frame (as before the worker), to compare. The mesher's interface
  (`terrain_mesh::mesh`) is unchanged. The draw distance is capped at the
  view less a section; the minimap redraws as land arrives.
- **Loading veil.** After a join, and after any teleport (recall, respawn,
  `/tp`) into land the client does not hold, a dark veil with "Loading the
  land around you" covers the game until every section within 24 m of the
  player is here and drawn. The server streams that land at the join rate:
  a view whose own column is not held goes back to the join budget until
  it is full again (`View::replan`).
- **Join.** `net::connect_with` returns once the pack files and every
  section of the columns within `LOAD_RADIUS` (40 m) of the spawn are here
  (the loading screen shows "Loading the land around you"); the rest
  arrives during play. Tools: `dyno shot` asks for the world round its
  camera with `JoinOptions::view_at` (`ClientMsg::ViewAt`), as an observer
  or as a player (`--as NAME --pos …`: the server then streams the world
  and what is near round that point instead of the character; the world and
  what stands in it were never secret, since before streaming every client
  heard of everything), lights and meshes it through the world thread as
  play does, and waits until the view stops arriving. `DYNO_SHOT_STATS=1`
  prints frame times; `DYNO_SHOT_RUN="x0,z0,x1,z1[,speed]"` flies the camera
  across the map while frames are measured. Bots apply sections like the game
  (`net::apply_world`); their path finder treats unknown land as not
  standable and walks partial paths towards far goals, and since they hear
  of creatures only within 100 m, a bot that finds a camp empty tries the
  next one before waiting for spawns.

**Measured** (the full 384 × 96 × 384 world, release builds on one
machine, `tests/world_measure.rs` against a real server; before is 0.7.1,
after is 0.7.2 with this work):

| | Before (whole world) | After (sections) |
|---|---|---|
| World bytes before play (fresh join) | 1,239,684 (`World`) | 127,516 (the land within 40 m) |
| Join until the world round the spawn is here | 148 ms | 49–66 ms |
| World bytes for the whole view (160 m, from the spawn) | — | 731,059 (1,890 of 3,456 sections) |
| `dyno shot --ui hud` (process start to saved frame; a proxy for the first frame, with fixed waits) | 8.3–10.9 s | 2.2–3.3 s (and it waits for the whole view) |
| One player running across the map (502 m at 7 m/s), ten others wandering | 8.6 KB/s | 12.5 KB/s: sections 6.7, poses 3.1, entity moves 2.6 |
| The same, everything but sections | 8.6 KB/s | 5.9 KB/s |
| The wanderers | 8.7 KB/s | 7.2 KB/s |
| Server RSS with 11 players (idle before they joined) | 132 MB (89) | 83 MB (78) |
| Client world + light | 13.5 + 27.0 MB | 3.9 + 4.3 MB |

The view is about 60% of this small map, so the whole view's bytes fall
only to 59% of the old world message; the gain is in what must arrive
before play and in memory, and it grows with the map. Running costs more
bandwidth than before (the land ahead streams in), though every other
kind of traffic fell.

- A `/who` sent over and over while a view streamed in at a deliberately
  slow 48 KB/s was answered in at most 12 ms (`tests/streaming.rs`).
- Forty players joining at once (a restart) on the full world: a tick of
  section streaming takes 11 ms on average and 36 ms at worst (the tick is
  50 ms), because each section is encoded once per shard and kept until it
  changes (`World::section_rev`); without the cache it was 384 ms.
- Client: light for a column that arrives takes 2.9 ms (the whole world:
  109 ms, from 374 ms before the fill read its box once). Meshes built on
  the world thread bit by bit equal those of the whole view built at once
  (`DYNO_SHOT_VERIFY`: 4,092,207 + 32,922 vertices round the spawn).
- Frames (`dyno shot` with `DYNO_SHOT_STATS`, frames rendered at about
  60 Hz, 960 × 540, one machine shared with other builds; "world work" is
  the main thread's time on sections, light copies and uploads):

  | | World work on the main thread, a frame (median / 95% / max) | Frames over 20 ms | Land round the camera drawn after |
  |---|---|---|---|
  | Join, inline (as before) | 8–13 / 13–55 / 62–156 ms | 1–54 | 0.9–3.0 s |
  | Join, world thread | 0.3–0.7 / 3.0–3.4 / 3.5–10.5 ms | 0–1 | 0.16–0.41 s |
  | Running across the map (304 m at 7 m/s), inline | 0 / 3.0 / 75.7 ms | 30 of 2,575 | |
  | The same, world thread | 0 / 0 / 3.2 ms | 0 of 2,700 | |

  Under heavy load (other builds, load average 25–60 on 16 cores) single
  frames still rose to 20–40 ms in a few join runs while their world work
  stayed under 10 ms: the rest is rendering and scheduling.

### The 2 km world (protocol 21)

For a starting zone of about 2048 × 256 × 2048 (docs/design/scale.md):

- **Storage** (`dyno_world::World`). Every world, server and client, is a
  grid of 16³ sections behind shared pointers: a section all one block
  (the sky, solid rock) costs nothing; others are packed with a palette
  and 1, 2 or 4 bits a cell (`World::compact`, after building and on
  load), and unpacked to a byte a cell only while being built or edited.
  Copies of the world (shards) share every section until they change one,
  so `diff` between shards skips shared sections. `positions_of` finds a
  block without scanning the sky and rock (hearths). Limits: `MAX_SIDE`
  4096, `MAX_HEIGHT` 512.
- **Saves** are `DYNW2`: the size, then each section as one block or its
  cells run-length encoded. `DYNW1` saves (every cell in one stream) still
  load, a row at a time, so the live world migrates on its next save.
- **Light** is stored the same way (one level, packed, or a byte a cell,
  copy on write); a whole world is lit in tiles of 128 × 128 columns on
  every core; shards start from a copy of the base world's light.
- **Renderer.** Only resident sections are held (a map, not a slot for
  each of the 262,144 places); each frame culls them once against the view
  and the three passes draw that list. Sections that are all sky (or rock)
  with the same all round are marked drawn without meshing, and sections
  are decoded and packed on the network thread (`NetEvent::Sections`;
  the game and the world thread share them).
- **Far view** (`dyno_world::far`, `dyno-client/src/farview.rs`). Beyond
  the streamed 160 m, the server sends a coarse heightmap in tiles of
  64 m: for each 4 × 4 m cell, the top of its highest block and that
  block's kind (`ServerMsg::Far`, about 300 bytes a tile), after the near
  view and within the same budget, out to 800 m (`--far`), remade and sent
  again only when a block in the tile changed (the sum of its sections'
  counts of changes). The client draws each cell as a column (a top, and
  the sides it shows above its neighbours) coloured by its block, outside
  the near view; the fog then ends at 900 m. A tower, a windmill, a
  headframe or a hero tree wins its cell, so landmarks stand out.
  `--world-size WxHxD` makes a new world of that size.

Measured on 2048 × 256 × 2048 (today's generator; release; a machine shared
with other builds, load average 30–50 on 16 cores):

| | |
|---|---|
| Sections | 262,144: 207,521 all one block, 54,623 packed, none raw |
| World in memory | 99 MB (94 MB of cells, 5 MB of table) |
| Light in memory | 60 MB (was 165 MB as a byte a cell; lit in 18 s) |
| Each further shard | about 10 MB (tables) plus what it changes |
| Save | 38 MB `DYNW2`, encoded in 0.9 s, loaded in 0.7 s |
| Server start from the save | 22 s; 234 MB resident, the whole process |
| Join on the 2 km world, a camera 46 m up | the land round it drawn after 1.3–1.5 s; world work on the main thread a frame: median 0, 95% 4–5 ms, max 7–9 ms (one run had a single far tile upload of 44 ms under that load) |
| Meshes streamed bit by bit against the whole view at once | identical (4,286,718 + 42,768 vertices) |

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
- **Size: 8.57 MB** (Linux x86-64, 0.9 with water: 8,989,888 bytes, from
  10,403,304). egui's four built-in fonts (1.4 MB) gave way to one,
  Ubuntu-Light, which stands in before the pack's fonts arrive
  (`theme::base_fonts`; the UI's few symbols became plain text), and the
  clipboard (arboard, smithay-clipboard and a second Wayland toolkit, about
  200 KB) is only in the `dyno` dev binary (`host`), which copies a hosted
  world's address. Tried and dropped: `-C force-unwind-tables=no` saves
  nothing (the standard library brings its unwind tables in; removing them
  needs a nightly `build-std`). Left for product calls: Wayland window
  decorations (sctk-adwaita and tiny-skia, about 290 KB; without them GNOME
  on Wayland shows no title bar), the GL backend (about 30 KB and part of
  naga's 700 KB), and the operating system's root certificates in place of
  the built-in ones.
- **Earlier: 9.32 MB** (Linux x86-64, 0.6.2 with the textures and sounds
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
3. World streaming and area of interest **(done)**; stamps and layout.
4. Authoritative movement.

Every step keeps `dyno shot`, the bots and the protocol tests working.
