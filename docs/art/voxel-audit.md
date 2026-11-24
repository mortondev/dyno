# The voxel rule: audit

What breaks the voxel rule (`docs/art/voxel-rule.md`), where, who fixes it,
and what has been fixed. The tests that hold the rule are in
`crates/dyno-client/src/voxel_rule.rs`; the cube ladder is
`crates/dyno-client/src/ladder.rs`. Written 2026-11-24.

## Goals

### Hand-in 1

| # | Goal | Result |
|---|---|---|
| 1 | **Audit:** every violation with its file, what it draws, the ladder step, the fix and the owner | **pass**: the tables below |
| 2 | **Mesh test:** representative world sections plus every shape, cover model, critter and particle primitive; every face normal one of the six axis directions in its object's frame, every vertex on the object's ladder grid | **pass**: `everything_drawn_is_cubes_on_the_ladder` meshes the sections round the six bench cameras at every terrain level of detail (with skirts, leaves in clumps and as whole cubes), every shape model, cover model (both levels), critter frame, figure part (every level, a dozen looks), held item, particle size, far-view tile and the target ring. It checks that each face is an axis-aligned rectangle with its own normal, and that its corners lie on the object's grid and that grid is a ladder step. It passes, with the allowlist below |
| 3 | **Pack test:** fails on any rig taper, bevel or non-uniform scale, any model without a ladder step, any shape rule using `round`, sloped quads or chamfers; dated allowlist entries naming the owner; the allowlist only shrinks | **pass**: `the_pack_keeps_the_voxel_rule`, plus `the_shaders_keep_cubes_whole` for what shaders do to vertices and normals. Entries that match nothing fail the test (`allowlists_only_shrink` tests that) |
| 4 | **Fix what's yours:** stepped cubes in the shapes mesher; flat per-face normals on fine terrain; dead smooth-terrain code removed; particles, sky shapes and water surface obey the rule; `tests/budget.rs` passes; `dyno bench` within performance.md | **pass**: see "Fixed" below. Budget counts were recorded again, with the reason in the commit (b4c644c: stairs and stepped circles where one sloped polygon or a 12-sided lathe stood). Bench at the default tier: every scene within every budget (`docs/art/voxel-rule/bench-high.md`) |
| 5 | **Before and after shots** of six gauntlet cameras, nothing curved or diagonal visible after except allowlisted items | **pass, with notes**: `docs/art/voxel-rule/before/`, `after/`. Roofs, awnings, logs, posts, the brazier, the sun and moon are stepped or square. Still visible: 1 m block stairs in stamps (the windmill's sails, the mine head's braces, gables), which are cubes but read as diagonals at a distance (world lane, listed below), and the round tree-ring picture on log ends (a texture: listed below) |
| 6 | **Gates:** fmt, clippy `-D warnings`, `cargo test --workspace`, `check-client.sh`, `tools/drive/run.sh` | **pass** on the tree merged with main v0.11.3: fmt clean; clippy clean (this machine's clippy is 1.98, which flagged `chunks_exact` with a constant size, now `as_chunks`); `cargo test --workspace` 31 test binaries ok (an earlier run before the merge hit main's since-fixed `shared_tags` flake); check-client 9.23 MB with no content strings; drive PASS, 136 steps |

### Hand-in 2

| # | Goal | Result |
|---|---|---|
| 7 | Figures sculpted at 3.125 cm (32 a block), no taper, bevel or non-uniform scale, via `dyno bake-figure`; people stay 1.78 m; held items and creatures name a ladder step | **pass**. The person's grids are sculpted at `detail` 32/34.63 voxels a rig unit, so its cubes are exactly 1/32 of a block (the lead agreed: rig and clip numbers stand, `people_are_life_size` passes). Builds and the 1.14 head are sculpted into the voxels (`sculpt/mod.rs` `Variant`), the rig's part scales are gone, and so are the taper and the shaped mesher (`smooth.rs`). Held items are re-voxelled at 1/32 (`tools/figures/relevel.py`). Creatures were already at 32 a block; a creature drawn at a rules `scale` is re-voxelled at that size (`build_at`), and a bone's `scale` op draws its part whole or hidden. The mesh test checks every figure part on its own grid at sizes 0.7, 1, 1.2 and 1.6 |
| 8 | Silhouettes, armour sets and NPC looks still read; outfit sheets before and after | **pass, coarser**: `docs/art/voxel-rule/figures/outfits-before*.jpg` and `outfits-after*.jpg` (plus `outfits-after-plate-ironbound-level-8.jpg`). Silhouettes, builds, pauldrons, helms, tabards, capes, shields and the class looks read as before. Faces are simpler at 3.125 cm: eyes are one or two cubes, mouths and brows on the woman's face fall below a cube and vanish (bake warns of four empty layers), and close-up portraits are blocky. Shells, sheets and trim bands were made at least a voxel thick so nothing shows through |
| 9 | Figure vertex cost within 10% at each LOD; first join under 4 MB | **pass**: the person (12 looks summed) costs 371k, 371k, 371k, 138k and 40k vertices at its five levels, against 1,817k, 704k, 500k, 192k and 48k before (−80%, −47%, −26%, −28%, −18%); held items 29k against 44k; creatures unchanged at life size (`cost-before.tsv`, `cost-after.tsv`). `person.vox` 2.5 MB to 0.83 MB; a first join downloads 2,432,906 bytes (2,964,228 before) |
| 10 | The figure entries are gone from the allowlist | **pass**: none left; the halos the lead ruled out are removed and guarded by the shader test |

## How the tests decide

- **A face** is an axis-aligned rectangle when each triangle has a right
  angle between two axis-aligned edges and pairs with its twin into a
  rectangle. A sloped quad, a gable triangle, a lathe's side and a turned
  cube all fail.
- **A grid** is the object's own: world things in world metres from 0,
  a shape in its cell's 16 voxels, a figure part or critter frame in its
  own voxels (any whole-part motion is the rule's allowed rigid motion, so
  the part is checked before it is posed).
- **The ladder:** the object's cubes a metre must be 1, 2, 4, 8, 16 or 32.
- **A stored normal** must be the face's own (for streams that carry one).
- `tests-before.txt` is what the three tests reported before any fix.

## Violations

Ladder step: 1 = 1 m, 2 = 50 cm, 3 = 25 cm, 4 = 12.5 cm, 5 = 6.25 cm,
6 = 3.125 cm; "off" for a size off the ladder.

### Fixed (this lane)

| File | What it drew | Step | Fix |
|---|---|---|---|
| `shapes.rs` (`mesh_model`, `slopes`, `faces`) | one sloped polygon over every 45° run of voxels (roofs, awnings, cloth, the brazier, lanterns, flower mounds, the hay bale); trapezoids cut to match | 5 | the mesher draws each voxel's open faces as greedy rectangles, nothing else |
| `shapes.rs` (`lathe_model`), `blocks.json` `"round"` | solids of revolution (8 to 12 sides) for trunks, posts, barrels, pillar shafts, the anvil's stump, the ridge roll | 5 | lathe removed; `round` gone from the data; the models redrawn as clean stepped circles slice by slice (`tools/shapes/rounds.py`) so they merge into tall faces |
| `shapes.rs` `jitter`, `spin` | props turned a few degrees, trunks twisted by 0.025 rad steps | 5 | jitter removed; `spin` is now extra quarter turns |
| `blocks.json` `lift` 0.16 voxels | flower beds lifted off the grid | 5 | lifts are whole voxels (0 or 1) |
| `blocks.json` `shape_files` | no ladder step named | — | `"step": 5` on each |
| `blocks.json` palette `"slope": false` | (kept steps out of slopes) | 5 | removed with the slopes |
| `content/shapes/roof_*.vox`, `cloth.vox` | 45° runs of single voxels (a slope once meshed) | 5 | restepped to 25 cm treads (`tools/shapes/stairs.py`): a roof is a stair |
| `terrain_mesh.rs` bevels, `"chamfer"` tag on 40 blocks | built cubes with 5 cm chamfers and slanted corner triangles | off | chamfers removed; the tag retired |
| `fine.rs` `smooth`, `shade_gentle`; `blocks.json` `terrain.smooth`, `terrain.blocks.{grass,sand}.smooth` | the ground's normals bent towards the slope the steps make; creases softened on gentle slopes | 4 (and 3, 2, 1 by distance) | one flat normal per face; the keys and `slope_at` removed |
| `terrain_mesh.rs` `Nets` | a smooth leaf surface (surface nets) when the pack has no `foliage.json` | — | removed; such leaves are whole cubes (`foliage::plain`) |
| `terrain_mesh.rs` `WATER_DROP` 0.2, `water.rs` fall foot 0.06 | the water surface and the foot of falls off the grid | off | 0.1875 and 0.0625 |
| `render.rs`, `water.rs` `swell` | the water surface rising and falling vertex by vertex (a flat face no more) | — | removed: the surface is a flat face; its ripples stay drawn on it in squares |
| `render.rs` `push_post` | torch posts 18.75 cm across, off the 6.25 cm grid, with no normals | off | 12.5 cm (two cubes), faces carry their normals |
| `render.rs` `ring` | a 40-segment circle round the target | — | a stepped circle of 6.25 cm cells |
| `sky.rs` | round sun and moon discs | — | squares, the moon's seas in cells as before |
| `particles.rs` `edge` | cubes of any size as an effect grows and fades | off | sizes snap to the ladder (`ladder::snap`); smaller than two thirds of 3.125 cm is not drawn |
| `app.rs` `spawn_bolt`, `spawn_burst`, `spawn_motes`, block chips | sparks, motes and chips shrinking smoothly as they die | off | drawn at ladder edges (`particle_edge`, `ladder::snap`); in the mesh test |
| `render.rs` `push_glow`, `particles.rs` `halo` | camera-facing glow quads round lamps and glowing particles (billboards) | — | removed (the lead's ruling): the glow is the emissive cubes' and the bloom's |
| `content/figures.json` person (`per_block` 34.63 × detail 2 and 2.5) | people at 1.44 cm and 1.16 cm voxels | off | sculpted at 32/34.63 a rig unit: 3.125 cm (hand-in 2) |
| `content/rigs/biped.json` part `scale` (builds 0.93–1.1, head 1.14); `figure.rs` `reshape` taper | parts stretched unevenly off their grid | off | builds and the head sculpted into the voxels; `shape`, `reshape` and the taper removed |
| `voxmodel` `smooth.rs` (surface nets, `bevel`, `soft`) | the shaped mesher | — | removed |
| `content/figures.json` `held.kinds.*.unit` (0.014–0.039 blocks) | held items at 29 off-ladder voxel sizes | off | re-voxelled at 1/32 (`tools/figures/relevel.py`) |
| `content/rules/hearthblock.json` creature `scale` (0.7–1.6) | creatures drawn larger or smaller than modelled | off | re-voxelled at their size when built (`figure.rs` `build_at`, `Vox::resample`); what they hold is drawn at its own size |
| `content/rigs/wisp.json`, `wraith.json` bone `scale` ops | the wisp's core, tail and veil, the shade's eyes squashed as they move | off | the engine draws a part whole or hidden; the tails' pulse removed from the data |

### Allowlisted (other lanes)

All dated 2026-11-24 in `voxel_rule.rs`.

| File | What it draws | Step | Fix | Owner |
|---|---|---|---|---|
| `content/life.json` `critters.*.unit` (0.035 to 0.18 m) | butterflies, birds, bats, crabs, deer, ducks, gulls, herons, rabbits, sheep, stags at 13 off-ladder sizes | off | re-voxel each at the nearest step (1/32, 1/16 or 1/8) | life lane (ambient critters) |
| `cover.rs` sway (`let s = v.sway * (sin(…`); `cover.json` `sway` on 20 kinds | grass and flowers bending vertex by vertex (cubes shear) | 4–5 | sway each plant as one rigid piece about its root | vegetation lane |
| `cover.rs` fade (`* fade`) | cover shrinking into the ground at the end of its reach (cubes leave the ladder) | 4–5 | sink it whole instead of scaling, or a dither | vegetation lane |
| `cover.json` `soft` on 26 kinds (`cover.rs` `soft`) | plants' side faces lit with a normal leaned to the sky (a side lit as a slope) | 4–5 | flat face normals; brighten with colour instead | vegetation lane |
| `render.rs` leaf stir (`pos.x = pos.x + sin(t * 1.2 + ph)…`) | leaves moved vertex by vertex in the wind (cubes shear) | 2–4 | stir whole clumps rigidly, or not at all | vegetation lane |
| `render.rs` painted relief (`bent = normalize(abs(det) * n - g * k)`) | built blocks' normals bent inside a face by the texture's brightness | — | light by the face; relief as colour only | light lane |
| `render.rs` block sampler (`mag_filter: Linear`) | block textures smoothed up close: a face's picture is not a grid of cubes | — | nearest filtering when magnified | light lane |

### Listed, not edited (world lane: stamps)

The stamps (`tools/stamps/build.py`, `content/stamps/*.dstamp`) are built
of 1 m blocks, so they are cubes on step 1 and the tests pass them. These
still read as diagonals or curves at play distance, and are the world
engineer's to judge:

- **Roofs** (`kit.roof`: every cottage, the farmhouse, inn, smithy,
  library, barn, huts, townhouses, toll hut, miners' lodge): 1 m stairs of
  roof blocks. The roof blocks' shapes are now 25 cm stairs (fixed above),
  so the roof surface is stepped, but the gable outline still climbs 45°.
- **The windmill's sails** (`windmill`): four arms of 1 m blocks on the
  diagonals, a lattice that reads as a diagonal cross.
- **The sawmill's wheel** (`sawmill`): a 1 m stepped circle of radius 5
  with diagonal spokes.
- **The mine head** (`mine_head`): diagonal braces of 1 m blocks.
- **Ridge tents** (`ridge_tent`): cloth in 1 m steps either side of the
  ridge.
- **The Hollow Oak** (`hollow_oak`, `blob`): ellipsoid blobs of leaves
  and wood: stepped at 1 m, round at a distance.
- **The covered wagon** (`wagon`): canvas hoops and spoked wheels in 1 m
  blocks.

### Listed, not edited (textures)

- **Log ends** (`content/textures`, the `log` and `birch_log` top faces):
  a picture of round tree rings across a whole 1 m face. The rule asks a
  face's picture to be a grid of ladder cubes. Owner: whoever owns block
  textures (light or world lane).

### Judged within the rule

- **Water ripples and foam** (`water.rs`): drawn on the flat surface in
  squares, one height a cell; the reflection leans by a cell's ripple. A
  reflection is a screen effect, and the cells are squares.
- **Clouds** (`sky.rs`): heaped square cells already.
- **The far view** (`farview.rs`): columns of 4 m and 8 m cells, stacks of
  1 m cubes; passes the mesh test.
- **Lily pads** (`water.rs`): "a disc of cells", so a stepped circle.

## Cost

The rule costs triangles where one sloped polygon or a lathe stood. On the
budget test's six cameras, world triangles grew 16% to 43% (the plaza
most, for its roofs) before they were recorded again. The default-tier
bench stays within every budget in `docs/design/performance.md`: GPU
1.3–1.9 ms a frame, 2.4–5.7 M triangles, 63–148 MB of vertices, 1132–1392
draw calls, meshing 0.8–1.4 ms a section (mean). Two tools keep the count
down: `stairs.py` (25 cm treads, a quarter of the steps of the 6.25 cm
diagonal) and `rounds.py` (a clean circle per slice merges into tall
faces), and round parts join from cell to cell (`"joins": true`).
`tools/bench/reference.json` was recorded again for the same reason
(the plaza 28–34% and Miner's Rest 18% over the old reference).
