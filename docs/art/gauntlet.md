# The visual gauntlet

The gauntlet is a loop that drives the game's real rendering toward the concept
targets in `docs/art/concepts/` (round 4 for now; round 5 later, shot from
gameplay cameras at WoW proportions). It keeps going until the target is met
or the round cap is reached.

## Loop

1. **Shoot.** Render the 18 locations in the game, from the canonical gameplay cameras (`tools/concepts/cameras.json`), at the location's time of day: `python3 tools/concepts/shots.py OUT_DIR ADDRESS`. Each shot is `dyno shot --stand`: a stand-in player in starter gear on the ground, facing the landmark, through the play camera of the scale spec (9 m behind, 10–20° down, 75° field of view), with no interface. When the world changes where things stand, choose the cameras again with `dyno scout ADDRESS` (it reads `tools/concepts/landmarks.json`) before shooting.
2. **Judge.** A fresh judge agent sees only pairs of images (the in-game shot beside its target) and the rubric below. It never sees the implementers' notes, diffs or claims. It scores each location and returns the five changes that would close the most gap.
3. **Implement.** One implementer per lane works on its own branch in a worktree from current main, taking the judge's fixes for its lane.
4. **Gate.**
   - Automated checks: `cargo fmt`, `cargo clippy --workspace --all-targets -D warnings`, `cargo test --workspace` and `check-client.sh` (no content in the client).
   - Frame time and memory: within the budgets of docs/design/performance.md on the benchmark's scenes (`dyno bench`), and the counts test passes (`cargo test -p dyno-client --test budget`; a lane that means to add more than 15% records the counts again and says why).
   - Content as data: looks live in the pack (`render.json`, rules, models, textures), not in code; engine code only adds capabilities.
5. **Merge.** The lead merges passing lanes into main and re-shoots. Releases stay with the lead, after a rehearsal on a copy of the live world.
6. **Repeat** until the pass bar is met or four rounds have run.

## Rubric (each criterion 1–10 per location)

1. **Lighting:**
   - sun shadows (crisp, correct length);
   - ambient occlusion in corners and under canopies;
   - bloom on lights;
   - warm/cool contrast.
2. **Atmosphere:** aerial haze and distance fog, the sky, clouds, the zone's colour mood and time of day.
3. **Voxel quality:** crisp voxel surfaces at about 8 per metre; per-voxel colour variation; no smeared textures or blobby smooth shapes.
4. **Water:** colour and clarity, reflections, shoreline foam, motion cues.
5. **Vegetation:**
   - species and size variety;
   - thick trunks and full crowns;
   - meadows of grass tufts and flowers;
   - clumped woods with open space between.
6. **Scale and proportion** (docs/design/scale.md): doors, storeys, road widths and tree sizes relative to the people; distances that read like a WoW zone.
7. **Life:** a believable population, ambient creatures, smoke, birds, fireflies, magical motes.
8. **Composition and readability:** the landmark reads clearly, and the frame is free of clutter and artefacts.

**Pass bar:** the average of all criteria across locations is at least 7, and no location scores below 5 on any criterion.

## Lanes

| Lane | Owns | Does not touch |
|---|---|---|
| Light and atmosphere | render.rs lighting, shadows, AO, bloom, fog, sky, grading; zone moods in render.json | terrain mesher internals, figures |
| Water | the water shader and water data | |
| Vegetation | tree species as data, trunks and crowns, grass tufts, flowers, density rules | world size |
| Life | ambient critters and particle emitters as data, NPC idle routines | |
| Architecture and scale | heroic doors, storeys and roofs, road widths, through world builders and stamps | |

The architecture and scale lane waits for the user's go-ahead on resizing the world, because that means a one-time rebuild of the live world.

The other live work keeps its own lanes:
- **Fine voxel terrain:** the terrain mesher.
- **Worker-thread meshing:** scheduling.
- **Character re-sculpt:** figures and rigs.

## Agents

- **Implementers:** Fable agents, one per lane, in their own worktrees.
- **Judge:** a fresh Fable agent each round, with no memory of earlier rounds beyond the scores table.
- **Lead:** the main session. It shoots, merges, keeps the tracker and releases.
