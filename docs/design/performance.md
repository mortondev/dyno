# Performance: budgets, the benchmark and what a frame costs

The game must run smoothly on a mid-range graphics card at 1080p with the
settings a new player gets, without losing the look of the visual gauntlet
(`docs/art/gauntlet.md`). This note sets the budgets, describes the
benchmark that holds them (`dyno bench`) and the test that guards the
counts in CI, and records what was measured before and after the first
round of work.

## Budgets

The default tier is **high** (what a new player on a discrete card gets:
`options::Graphics::preset(2)`), the picture 1920 × 1080. The reference
card is an RTX 3080; an RTX 3060-class card is taken as half as fast, so
8 ms on the reference card leaves it 60 frames a second with headroom.

| What | Budget (default tier) | Measured now | How it is measured |
|---|---|---|---|
| GPU time a frame, median, every scene | 8 ms on the reference card | 1.1–2.3 ms (the least of any frame 0.9–2.3 ms) | timestamps between passes |
| Main thread's time a frame, median | 8 ms standing, 10 ms in the flight | 4.3–6.2 ms standing, 7.1 ms in the flight | the thread's own processor time |
| Vertex memory (world, far view, cover) | 300 MB at its peak in the flight; 200 MB standing | 59 MB in the flight; 37–84 MB standing | buffer sizes |
| Texture memory (block textures, shadow maps, the picture's targets) | 256 MB | 103 MB (ultra: 247 MB) | texture sizes |
| World thread, meshing a section (release) | 5 ms mean, 16 ms for 95% | 0.96–1.42 ms mean, 3.3–5.1 ms for 95% | the meshing thread's own processor time |
| Draw calls a frame | 3000 | 1153–1972 | counted |
| Triangles a frame, all passes | 10 M | 2.4–6.5 M | counted |
| Counts on the fixed scenes | no more than 15% over the recorded ones | held by a test | `tests/budget.rs` |

The other tiers have no budgets of their own; the report shows them so a
change that only hurts low or ultra is seen. Ultra is allowed to cost more.

## The benchmark: `dyno bench`

```sh
cargo build --release -p dyno
target/release/dyno bench --local --out bench            # every tier, about ten minutes
target/release/dyno bench --local --tier high --no-flight --out bench
target/release/dyno bench --diff bench-before/high bench/high --out bench/diff
target/release/dyno bench --local --check tools/bench/reference.json    # counts against the recorded run
```

- **Scenes** (`tools/bench/scenes.json`): six of the gauntlet's gameplay
  cameras (`tools/concepts/cameras.json`: the plaza, the logging camp, the
  hollow oak, the windmill farm, Mirrormere, Miner's Rest) and a 60-second
  flight round Hearthstead at 8 m/s, a few metres over the ground, with the
  world streaming in as play has it.
- **World:** `--local` builds a fresh world from the pack's layout in this
  process; an address measures a running server instead.
- **Tiers:** the options panel's presets (`options.rs`): the quality of the
  costly passes, water, ground cover, leaf detail and ambient life together.
- **A still scene** is the stand shot of `dyno shot --stand`. Its saved
  picture has nothing that moves in it (no creatures or ambient life, the
  shaders' clock held), so two builds' pictures can be compared pixel by
  pixel; its frames are then measured with everything in: creatures, the
  ambient life made anew each frame, the clock running.
- **It reports**, a scene and a tier (`report.json`, `report.md`):
  - GPU time a frame: the median, the 95th percentile and the least of any
    frame, and the median of each pass (three shadow cascades, the world,
    occlusion, sky, water, translucent, post, interface);
  - the main thread's time a frame, of its own processor time and on the
    wall, and where it went: the renderer's setup, each pass's commands,
    handing the frame over, the ambient life; in the flight also what
    arrived, world and far-view uploads, creatures;
  - draw calls and triangles by pass, the cover's triangles and instances,
    the vertices made anew each frame (figures, effects);
  - vertex memory (world meshes, far view, cover, the quad index) and
    texture memory (block textures, shadow maps, the picture's targets);
  - meshing: sections, and the mean, median and 95th percentile of a
    section's time on its thread.
- **`--check REPORT.json`** fails when a still scene's vertex memory or
  triangles a frame grew more than 15% (`--tolerance`) over an earlier
  report. Times are never compared. `tools/bench/reference.json` is the
  "after" run recorded below (RTX 3080), to check against on that card.
- **`--diff A B`** compares two runs' pictures: the largest and the mean
  difference of a channel (of 255) and the share of pixels more than 2 and
  more than 8 apart, with a picture of each difference (twenty times as
  bright) when `--out` is given.

### Reading the times

The machine these numbers come from is shared: sixteen cores at a load
average of 30 to 60, and a graphics card other work also draws on.

- **GPU time** is the card's clock between passes, so a frame that waited
  behind someone else's work reads long, and an idle card that has dropped
  its clocks reads slow. The same build measured twice gave medians a
  factor of two apart. The least of the frames ("best") is what the card
  can do; the median is what it did that minute. Compare builds on runs
  taken back to back, and trust counts over times.
- **CPU time** is the thread's own processor time, which a busy machine
  does not stretch (the time on the wall is reported beside it).
- **Counts and memory** are exact, and repeat.

## The check in CI: `tests/budget.rs`

CI has no graphics card, so the test counts on the processor: it builds
the layout's world, takes the sections a server would send for each of the
six cameras, lights and meshes them with the world worker for a camera
there, and adds up the bytes of their vertices, their triangles and the
triangles of the ground cover round the camera (all the sections held,
not only those in view: culling is the benchmark's to show). It fails when any of them
is more than 15% over `tools/bench/counts.json`. Two runs give the same
counts to the byte. It takes about a minute in a debug build on sixteen
cores (most of it building the 2 km world); on a CI runner's two to four
it has not been timed.

When a change is meant to add that much (new vegetation, a rebuilt world),
record the counts again and say why in the commit:

```sh
DYNO_BLESS=1 cargo test -p dyno-client --test budget
```

## Measured: before and after

Release builds, 1920 × 1080, RTX 3080 (Vulkan), a fresh world of the
pack's layout (2048 × 256 × 2048). "Before" is main at c507290 with only
the benchmark added; "after" is this branch as merged with that main (it
has since merged main again, which changed figures, not the world). Each
cell is before → after.
GPU is the median (after: the least of any frame in brackets); CPU the
median of the main thread's own time; vertex memory the peak.

### high (the default)

| scene | GPU ms | CPU ms | draw calls | triangles | cover triangles | vertex memory MB | mesh ms a section (mean / 95%) |
|---|---|---|---|---|---|---|---|
| hearthstead_plaza | 1.99 → 1.09 (1.08) | 6.7 → 6.2 | 1347 → 1544 | 3.12 M → 2.43 M | 1.35 M → 0.93 M | 108 → 47 | 0.83 / 3.14 → 0.99 / 4.01 |
| logging_camp | 1.77 → 1.32 (1.31) | 4.1 → 4.3 | 1412 → 1639 | 6.01 M → 4.50 M | 3.08 M → 1.98 M | 182 → 70 | 1.14 / 3.68 → 1.38 / 5.02 |
| hollow_oak | 2.35 → 2.33 (2.31) | 5.1 → 5.3 | 1714 → 1972 | 8.30 M → 6.51 M | 4.90 M → 3.62 M | 234 → 84 | 1.12 / 3.48 → 1.42 / 5.10 |
| windmill_farm | 1.29 → 1.79 (1.77) | 3.3 → 5.8 | 1185 → 1287 | 4.23 M → 3.12 M | 2.80 M → 2.02 M | 91 → 42 | 0.84 / 2.65 → 1.15 / 3.91 |
| mirrormere | 3.85 → 1.76 (1.74) | 5.8 → 5.5 | 1158 → 1247 | 3.48 M → 2.47 M | 1.80 M → 1.36 M | 101 → 50 | 0.92 / 3.10 → 1.24 / 4.27 |
| miners_rest | 3.33 → 1.96 (1.64) | 8.5 → 5.0 | 1269 → 1328 | 4.26 M → 3.38 M | 2.86 M → 2.25 M | 77 → 37 | 0.89 / 2.74 → 0.97 / 3.26 |
| flight | 5.39 → 2.20 (0.85) | 10.9 → 7.1 | 1338 → 1153 | 4.60 M → 2.92 M | 3.07 M → 2.07 M | 133 → 59 | 0.83 / 3.05 → 0.96 / 3.61 |

### low

| scene | GPU ms | CPU ms | draw calls | triangles | cover triangles | vertex memory MB | mesh ms a section (mean / 95%) |
|---|---|---|---|---|---|---|---|
| hearthstead_plaza | 1.82 → 0.50 (0.50) | 3.7 → 2.4 | 1320 → 1368 | 2.00 M → 1.60 M | 0.32 M → 0.18 M | 107 → 47 | 0.82 / 2.72 → 1.04 / 3.57 |
| logging_camp | 1.05 → 0.67 (0.66) | 4.2 → 2.1 | 1388 → 1483 | 3.65 M → 2.88 M | 0.90 M → 0.49 M | 172 → 68 | 1.14 / 3.89 → 1.29 / 4.75 |
| hollow_oak | 1.29 → 1.22 (0.92) | 3.3 → 2.2 | 1671 → 1815 | 4.49 M → 3.59 M | 1.41 M → 0.99 M | 217 → 80 | 0.93 / 3.07 → 1.31 / 4.69 |
| windmill_farm | 1.65 → 1.55 (1.49) | 6.4 → 2.3 | 1160 → 1159 | 2.15 M → 1.54 M | 0.80 M → 0.50 M | 88 → 40 | 0.95 / 3.00 → 0.99 / 3.38 |
| mirrormere | 0.86 → 0.57 (0.56) | 4.5 → 2.9 | 1128 → 1064 | 2.14 M → 1.37 M | 0.52 M → 0.31 M | 98 → 49 | 1.08 / 3.55 → 1.21 / 4.06 |
| miners_rest | 1.34 → 0.55 (0.55) | 5.6 → 3.0 | 1259 → 1190 | 2.16 M → 1.67 M | 0.85 M → 0.61 M | 72 → 36 | 0.91 / 2.74 → 0.94 / 3.10 |
| flight | 4.15 → 0.57 (0.39) | 13.4 → 6.2 | 1142 → 1059 | 2.33 M → 1.44 M | 0.93 M → 0.61 M | 133 → 58 | 1.02 / 3.52 → 0.98 / 3.79 |

### medium ("Before" ran medium with ground cover at 0.75; the preset is 1.0, which "after" uses: its cover triangles are not comparable.)

| scene | GPU ms | CPU ms | draw calls | triangles | cover triangles | vertex memory MB | mesh ms a section (mean / 95%) |
|---|---|---|---|---|---|---|---|
| hearthstead_plaza | 5.97 → 0.83 (0.82) | 7.6 → 6.1 | 1338 → 1542 | 2.49 M → 2.42 M | 0.73 M → 0.93 M | 108 → 47 | 0.85 / 3.17 → 0.96 / 3.75 |
| logging_camp | 2.73 → 1.12 (1.11) | 9.4 → 4.4 | 1405 → 1637 | 4.75 M → 4.50 M | 1.81 M → 1.98 M | 181 → 70 | 1.18 / 3.86 → 1.32 / 4.71 |
| hollow_oak | 3.22 → 1.82 (1.43) | 6.2 → 3.4 | 1689 → 1970 | 6.29 M → 6.51 M | 2.90 M → 3.62 M | 233 → 84 | 1.11 / 3.48 → 1.34 / 4.80 |
| windmill_farm | 3.91 → 0.87 (0.86) | 8.4 → 3.4 | 1176 → 1285 | 3.10 M → 3.11 M | 1.66 M → 2.02 M | 90 → 42 | 1.01 / 3.10 → 1.06 / 3.49 |
| mirrormere | 1.91 → 1.95 (1.36) | 8.7 → 3.4 | 1142 → 1245 | 2.74 M → 2.47 M | 1.06 M → 1.36 M | 100 → 49 | 1.10 / 3.70 → 1.08 / 3.61 |
| miners_rest | 2.46 → 2.00 (1.03) | 7.2 → 4.2 | 1265 → 1309 | 3.11 M → 3.38 M | 1.71 M → 2.25 M | 76 → 37 | 0.91 / 2.74 → 0.85 / 2.82 |
| flight | 4.20 → 2.32 (0.70) | 12.7 → 8.0 | 1334 → 1173 | 3.39 M → 2.95 M | 1.86 M → 2.05 M | 132 → 58 | 0.88 / 3.16 → 0.97 / 3.72 |

### ultra ("Before" ran ultra with ground cover at 2.0; the preset is 1.5, which "after" uses: part of the fall in triangles is that.)

| scene | GPU ms | CPU ms | draw calls | triangles | cover triangles | vertex memory MB | mesh ms a section (mean / 95%) |
|---|---|---|---|---|---|---|---|
| hearthstead_plaza | 2.58 → 1.62 (1.61) | 5.5 → 3.1 | 1351 → 1592 | 4.83 M → 3.18 M | 3.02 M → 1.64 M | 113 → 49 | 0.83 / 3.16 → 0.93 / 3.64 |
| logging_camp | 5.61 → 1.93 (1.91) | 7.1 → 6.5 | 1412 → 1685 | 8.90 M → 5.80 M | 5.64 M → 3.05 M | 217 → 80 | 1.18 / 4.03 → 1.45 / 5.75 |
| hollow_oak | 5.95 → 2.72 (2.71) | 8.5 → 5.4 | 1696 → 1984 | 13.11 M → 8.57 M | 9.08 M → 5.19 M | 290 → 99 | 1.15 / 3.65 → 1.54 / 6.13 |
| windmill_farm | 3.21 → 1.42 (1.41) | 7.4 → 3.5 | 1186 → 1327 | 6.68 M → 4.20 M | 5.12 M → 3.02 M | 106 → 46 | 1.02 / 3.27 → 1.13 / 4.03 |
| mirrormere | 4.20 → 2.44 (2.41) | 9.4 → 5.4 | 1153 → 1272 | 5.43 M → 3.12 M | 3.67 M → 2.02 M | 115 → 53 | 1.11 / 3.71 → 1.22 / 4.43 |
| miners_rest | 5.94 → 5.12 (3.26) | 6.9 → 3.8 | 1269 → 1348 | 6.64 M → 4.56 M | 5.12 M → 3.31 M | 86 → 40 | 0.95 / 3.02 → 0.98 / 3.32 |
| flight | 8.10 → 6.74 (1.18) | 16.5 → 9.9 | 1124 → 1172 | 6.63 M → 3.83 M | 5.18 M → 2.94 M | 138 → 60 | 0.99 / 3.45 → 1.02 / 3.95 |

Shadow-pass triangles on the default tier (the three cascades together), before → after: hearthstead_plaza 1.22 M → 0.96 M, logging_camp 2.06 M → 1.67 M, hollow_oak 2.42 M → 1.94 M, windmill_farm 0.94 M → 0.63 M, mirrormere 1.13 M → 0.59 M, miners_rest 0.94 M → 0.70 M, flight 1.12 M → 0.50 M.

The flight on the default tier: 2003 → 2633 frames drawn in its 60 seconds (the loop asks for 60 a second); frames over 16.7 ms on the wall 1028 → 978; the main thread's 95th percentile 17.4 → 11.9 ms; its worst frame 554 → 562 ms (the first sight of the creatures' models, see below).

Where the main thread's 7.1 ms a frame go in that flight (medians): what arrived 0.00 ms (95% 0.01); world uploads 0.00 ms (95% 0.36); far view 0.00 ms (95% 1.49); camera 0.07 ms (95% 0.10); creatures 1.52 ms (95% 5.10); ambient life 2.52 ms; the renderer's setup 1.24 ms (of which uploading 55,000 moving vertices 0.66 ms), its passes 0.60 ms, handing the frame over 1.17 ms.

The card's frame by pass on the default tier, after (median ms):

| scene | shadow near | shadow mid | shadow far | world | occlusion | sky | water | translucent | post |
|---|---|---|---|---|---|---|---|---|---|
| hearthstead_plaza | 0.05 | 0.13 | 0.08 | 0.53 | 0.13 | 0.05 | 0.03 | 0.01 | 0.08 |
| logging_camp | 0.08 | 0.13 | 0.13 | 0.70 | 0.13 | 0.04 | 0.03 | 0.01 | 0.07 |
| hollow_oak | 0.15 | 0.28 | 0.24 | 1.23 | 0.18 | 0.04 | 0.08 | 0.01 | 0.12 |
| windmill_farm | 0.07 | 0.11 | 0.11 | 1.02 | 0.22 | 0.07 | 0.06 | 0.01 | 0.13 |
| mirrormere | 0.08 | 0.10 | 0.09 | 0.77 | 0.19 | 0.05 | 0.34 | 0.03 | 0.12 |
| miners_rest | 0.09 | 0.12 | 0.08 | 1.19 | 0.18 | 0.05 | 0.10 | 0.01 | 0.13 |
| flight | 0.14 | 0.10 | 0.06 | 1.20 | 0.24 | 0.14 | 0.07 | 0.01 | 0.14 |

## What changed

Each step was measured on the benchmark before the next.

1. **World meshes packed for the card** (`pack.rs`). The meshers still
   write plain 60-byte vertices; a section's solid mesh is packed on the
   thread that meshed it to 24 bytes a vertex: positions in 1/2048 m from
   the section's corner (the grid is the world's, so sections agree at
   their borders; the ground's voxels are kept exactly), normals in bytes,
   colour in ten bits a part of its square root, brightness in ten bits,
   light and sky in ten bits each, texture coordinates in 1/2048 tile.
   Two triangles on an edge become one quad on the shared quad index, so
   cubes and shapes take four vertices a quad as the ground does (92% of
   their triangles paired up on the world first measured). The vertex shader unpacks; a buffer of
   section corners is read by instance. Translucent faces and halos are
   few and keep the plain format. Vertex memory fell by 50 to 64% on every
   scene (and on the 384 world main had when this began: 328–419 MB to
   114–142 MB). Packing adds 0.1–0.3 ms to a section's meshing. A test packs
   every vertex the meshers make for a stretch of land and holds each
   field to its tolerance (`the_worlds_meshes_pack_within_tolerance`),
   and the budget test holds that no value of any vertex of the layout
   world's six scenes was cut to fit its field (`pack::cut`). A mesher
   that starts using a field differently should fail there, not on
   screen: that happened once in this work (main began to keep a ground face's
   direction in a field the packing had sized for less).
2. **What moves goes into buffers kept from frame to frame.** Figures,
   effects and lines are made anew every frame (60,000 to 250,000 plain
   vertices: 4 to 15 MB); each frame made a new buffer for them. Kept
   buffers stop that churn, but the time did not change measurably:
   uploading costs the main thread about 10 µs a thousand vertices either
   way (the copy, not the buffer). The cost is the vertices themselves:
   see what is left.
3. **Shadow casters that cannot shadow the picture are left out.** A
   cascade already left out sections beside its square; now also those
   outside one side of the view whose shadow, falling away from the light,
   only goes further out (`render::casts_into`). Shadow triangles fell by
   a fifth to a half (table below); the picture is the same.
4. **Ground cover out of view is not drawn.** The instances round the
   camera are sorted into eight directions and a near ring; a batch is
   drawn only when its ball touches the view. Cover triangles a frame fell
   by a fifth to a third, for 100 to 250 more draw calls.
5. **Sections are drawn nearest first**, so what is hidden is not shaded,
   and the translucent farthest first; both in the same order every frame
   (they were drawn in a hash map's order).
6. **Creatures neither in the picture nor casting a shadow into it are
   not made.** Every creature and townsperson is built into vertices on
   the main thread every frame; in the flight that was 4.0 ms a frame, the
   largest single cost. With those out of sight left out (the same test
   as the shadow casters') it is 1.3–1.5 ms, and the flight draws a third
   more frames in its minute. The still pictures hold no creatures, so
   this step rests on the test being conservative, not on a picture.

Tried, measured and left alone:

- **Occlusion and bloom at lower resolution.** Occlusion already runs at
  half size and bloom from half size down; together they take 0.2–0.3 ms
  of the card's frame. Nothing to win that would not show.
- **Plane-wide greedy merging of the ground.** The ground's faces already
  merge within a block and across blocks where their extents line up. A
  wider merge changes where the vertices lie, and the ground's slow tone
  is worked out at the vertices, so its look would shift; it needs the
  tone worked out per pixel first. Not done.

## The pictures: the same to the eye, nearly to the pixel

`dyno bench --diff`, before against after (the packing as it now is, on
the main the tables were measured on), the six cameras. The floor is
the "before" build against itself: its pictures repeat (0.000–0.001% of
pixels more than 2 of 255 apart, a mean difference of 0.000). Against it,
with the occlusion pass off (low) the pictures differ in at most 0.11% of
pixels by more than 2; with it on (high) in 0.3 to 5.4%, on wall panels
and window panes (see below: an open deviation). The mean difference of a
channel is at most 0.3 of 255 on any picture.

High (the default tier: shadows, occlusion, bloom, lamps as lights):

| picture | largest | mean | pixels over 2 | pixels over 8 |
|---|---|---|---|---|
| hearthstead_plaza | 226 | 0.232 | 3.715% | 0.438% |
| hollow_oak | 190 | 0.122 | 2.163% | 0.208% |
| logging_camp | 203 | 0.075 | 0.909% | 0.053% |
| miners_rest | 217 | 0.302 | 5.408% | 0.971% |
| mirrormere | 220 | 0.022 | 0.262% | 0.020% |
| windmill_farm | 160 | 0.047 | 0.590% | 0.046% |

Low (no occlusion or bloom):

| picture | largest | mean | pixels over 2 | pixels over 8 |
|---|---|---|---|---|
| hearthstead_plaza | 235 | 0.046 | 0.111% | 0.078% |
| hollow_oak | 244 | 0.029 | 0.063% | 0.032% |
| logging_camp | 248 | 0.030 | 0.043% | 0.030% |
| miners_rest | 232 | 0.034 | 0.086% | 0.042% |
| mirrormere | 236 | 0.013 | 0.007% | 0.004% |
| windmill_farm | 174 | 0.017 | 0.029% | 0.017% |

The largest differences (over 150) are single pixels on edges. Where the
pictures differ:

- **Edges.** A position moves by at most a quarter of a millimetre, so a
  few pixels on the edges of shapes fall the other side.
- **The occlusion pass (high and ultra): an open deviation.** With it on,
  0.3 to 5.4% of pixels differ by more than 2, most on recessed wall
  panels between beams and on window panes; to the eye the pictures are
  the same, to a difference picture they are not. What is known:
  - it is the occlusion pass: with its strength set to 0 in the pack, the
    worst camera (Miner's Rest) differs in 0.08% of pixels instead of 5.4%;
  - it is not the order a quad's corners are sent in: packed with the
    meshers' own triangles instead of quads, the same camera differs from
    the quads' picture in 0.03% of pixels;
  - the pictures repeat: the same build twice gives the same picture.

  What is supposed, not shown: the pass works a surface's normal out from
  the depth of neighbouring pixels, and the depth buffer (near plane at
  5 cm, not reversed) resolves about 0.3 mm at 15 m, so that normal is
  already at the limit of what the depth holds; a beam standing 1/16 m
  proud of a panel subtends exactly the pass's threshold half a metre
  away. A change of a fraction of a millimetre in a vertex (the packed
  positions' steps are 0.49 mm) or of the last bit of a depth then tips
  samples over the threshold. If so the pass wants a steadier normal (a
  reversed depth buffer, or the normal the geometry already has); until
  that is tried, the default tier's pictures are close, not identical.

## What a frame costs now, and what is left

On the default tier the card's frame is 1–2.3 ms on the reference card, of
which the world pass is half or more and the three shadow cascades a fifth
to a quarter; occlusion, sky, water and post are 0.05–0.35 ms each. The
main thread spends more than the card: 4–6 ms standing and 7 ms in the
flight on the run in the tables (a busy hour: a quieter run gave 3–4 ms
and 6 ms).

- **Figures are the main thread's largest cost.** Creatures and people are
  built into plain vertices on the processor every frame and uploaded
  (1.3–1.5 ms for those in sight in the flight, 0.6–1.2 ms more to upload
  them, and they are drawn four times: three cascades and the picture).
  The first time a model is drawn it is built on the main thread: one
  frame of about half a second in the flight. Figures want meshes kept on
  the card with a transform a part, built off the main thread. (Character
  lane: `figure.rs`, `voxmodel.rs`, `models.rs`.)
- **Ambient life** takes 1–2.5 ms of the main thread a frame.
- **Handing the frame over** takes about 1 ms for 1200–2000 draw calls:
  one buffer and one or two draws a section and pass. Sections of a column
  could share a buffer and a draw.
- **The far shadow cascade** draws every section within 124 m at full
  detail. It could draw the ground's coarser levels; that changes shadows
  at a distance slightly, so it needs the gauntlet's eye.
- **Meshing** is within budget (1–1.4 ms a section, 5 ms for 95%).
  Packing adds 0.1–0.3 ms a section; the meshers could write packed
  vertices directly. No hot spot was hunted further: the budget is met
  five times over.
- **The occlusion pass** tips on sub-millimetre changes (above): the one
  place the packed pictures are not the plain ones. Its normal should not
  come from a depth buffer at the limit of its precision.
- **GPU times on a shared machine** are noisy; a quiet reference machine
  (or the card's clocks pinned) would let the report hold times as well as
  counts.
- **Not measured:** how many quads a plane-wide merge of the ground would
  save (`ground_levels_cost` in `terrain_mesh.rs` counts them); where
  within a section's millisecond the meshing time goes; the counts test's
  time on a CI runner.
- **Not run:** the packed pipeline on the GL fallback (ten-bit colour,
  whole-number attributes, a first instance other than zero). A headless
  GL device does not start on the machine this was written on, with or
  without these changes.
