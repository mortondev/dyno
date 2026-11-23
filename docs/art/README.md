# Art direction and references

Hearthblock Online aims for a **hand-painted, stylised high-fantasy look** in
the spirit of classic World of Warcraft zones (Elwynn Forest, Westfall,
Duskwood). Every visual change and every new piece of content is judged
against the concept art in this folder.

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
| Foliage | Distinct leaf clumps with visible branches and trunks, several species per scene, dense undergrowth: ferns, flowers, long grass, mushrooms |
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
