# Art direction and references

Hearthblock Online is a **Cube World × World of Warcraft hybrid**, and the
world stays **voxel**: every surface reads as voxels, never smooth polygons.

- **From Cube World:** the clean, charming voxel look; chunky but finely
  shaped voxel forms (about 8 voxels per metre for terrain and buildings,
  finer for props and details); soft, bright, colourful light with gentle
  ambient occlusion; puffy canopies built from many voxel leaf clumps;
  voxel flowers, grass and mushrooms; a cosy adventure mood.
- **From World of Warcraft:** the art direction and world design: heroic,
  chunky fantasy architecture, readable landmarks and silhouettes, zone
  moods (golden meadows, eerie night woods, misty lakes), warm/cool
  contrast, light shafts, aerial haze and distant hills.

Every visual change and every new piece of content is judged against the
concept art in this folder.

- `concepts/` — what each existing location should look like: AI concept art
  generated from real game screenshots (`reference/`), keeping each
  location's layout, landmarks and camera but raising the fidelity of
  everything.
- `reference/` — the game as it looked when the concepts were made
  (v0.7.2, 23 November 2026), from the same cameras.

The visual upgrade plan built from these concepts is the "Visual upgrade
plan" page (also `.tracker/visual-plan.html`).

## What the concepts agree on (the style guide)

| Area | Target |
|---|---|
| Light | Warm golden key light, soft long shadows, light shafts through canopies, cool blue shade; warm lantern pools and lit windows at night |
| Atmosphere | Blue aerial haze with distance, layered clouds, distant hills and forests beyond the playable edge |
| Colour | Saturated but harmonious; warm/cool contrast; greens with yellow highlights and deep shadowed interiors |
| Voxels | Everything is voxels at a finer scale than today (about 8 per metre for terrain and buildings, finer for details); shapes read as sculpted but stay cubic |
| Foliage | Voxel canopies of many leaf clumps with visible voxel branches and gnarled trunks, several species per scene, dense voxel undergrowth: ferns, flowers, grass tufts, mushrooms |
| Terrain | Painterly ground with colour variation, worn dirt paths with pebbles and edges, wet sand at shores, boulders, layered rock strata on cliffs |
| Water | Deep blue with specular glints, foam at banks and rapids, reflections of sky and trees |
| Architecture | Timber framing, shingled and mossy roofs, chimneys, lit windows, arched stonework, wear and moss |
| Props | Dense dressing along paths and in settlements: lanterns, fences, barrels, carts, signs, crates, flowers |
| Readability | Strong silhouettes; landmarks visible from far away; paths lead the eye |

## Using the concepts for new content

1. Render reference shots with `tools/concepts/shots.py` (a running server and
   `dyno shot`); edit its list of cameras.
2. Generate concepts with `tools/concepts/generate.sh` (Codex CLI image
   generation; the art direction is `tools/concepts/style.txt`). The prompt
   must come before `-i` (that flag takes several files).
3. For a new zone or dungeon, write the location description first, generate
   a concept from a blockout screenshot, and build to the concept.
4. Keep accepted concepts here, one per location, named like the location.

These are references, not assets: nothing here is shipped in the content pack.
