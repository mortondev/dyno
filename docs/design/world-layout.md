# The world as data: layout and stamps

Ashfall Vale is built from two kinds of data in the content pack. The
engine (`dyno-world`) only knows how to shape land from primitives, lay
roads, put stamps down and scatter vegetation by rules.

| File | What it holds |
|---|---|
| `content/world/layout.json` | The size of the world, the shape of the land, the river and the lake, zones and what grows in them, tree species, road kinds, roads, and every place with the stamps that stand there |
| `content/stamps/*.dstamp` | Voxel structures: every building, prop, ruin and dug-out place |
| `content/rules/hearthblock.json` | Areas, camps, NPCs and quest objectives, each placed **at a place of the layout** |

Both world files stay on the server (they are not in the manifest):
clients are streamed the world itself.

## Size: one value

`"size"` in `layout.json` is the size of the world. Positions are written
for `"design"` (2048 × 256 × 2048, the size docs/design/scale.md asks for).
When `size` is smaller, every distance between places is drawn in by
`spread = size.x / design.x`, and hills and ridges are lowered to fit under
the sky; the sizes of things (stamps, road widths, trees, pads) never
change.

Today `size` is **1024 × 192 × 1024** (`spread` 0.5), because the server's
world is still dense and saves stop at 1024. When the sparse server world
and the new save format land, set `"size": [2048, 256, 2048]` and bump
`"version"`: the live world is rebuilt once and every character moves to
its place (see "Rebuilding" below). Nothing else changes.

A server asked for a smaller world (`Config::world_size`, used by tests and
the title stage) builds the window of that size round the spawn; places
that do not fit are left out.

## layout.json

Positions are `[x, z]` offsets from the spawn hearth in metres at design
size; north is −z, east is +x.

| Key | Meaning |
|---|---|
| `version` | Bump it when the world must be rebuilt from the layout |
| `size`, `design`, `seed` | See above |
| `terrain` | `sea` (water level), `ground` (open ground), `rolling` (noise layers), `hills` (ovals that rise), `ridges` (mountain lines), `rim` (the mountains that close the zone), `lakes`, `rivers` (a path, width, depth and valley), `rock_line` (bare rock and snow heights) |
| `zones` | Ovals with a vegetation mix; the first that holds a column wins |
| `flora.species` | Every tree: trunk and leaf blocks, `height`, `girth` (trunk thickness) and `crown` width as ranges in metres, crown shape (`round`, `cone`, `none`), limbs, roots |
| `flora.mixes` | What grows in a zone: `cover` (share of the land that is wood), `clump` (size of the woods), `spacing`, species weights, lone trees per hectare, small plants per hectare, patches of other ground |
| `ways` | Road kinds: `width`, surface blocks with weights, lamp stamp and spacing, `ford` |
| `roads` | A way and a path of stops: a place key, or `[x, z]` |
| `places` | `key`, `name`, `kind` (`hub`, `site`, `wayside`, `camp`, `farm`, `view`, `zone`), `at`, `radius`, `pad` (levelled ground), `plaza` (paving), `level` (`water` for landings), `stamps`, `trees` (planted by hand: hero trees), `was` (where it was before this layout, for the rebuild) |

A stamp at a place is `{"stamp": "inn", "at": [36, 24], "rot": 1}`: an
offset from the place in metres (never scaled), quarter turns clockwise,
optional `dy`, and a ground-fitting rule `fit`:

- `pad`: at the place's level (the default at a place with a pad);
- `ground`: on the land under it, which is levelled under its footprint;
- `float`: on the land under it, as it is.

## Stamps

`content/stamps/<name>.dstamp` (`crates/dyno-world/src/stamp.rs`):

```text
DSTP1
{"size":[x,y,z],"anchor":[x,y,z],"palette":["skip","air","planks",…],"points":{…},"doors":[…],"storeys":[…],"kind":"building"}
run-length cells: (palette index, varint count), y then z then x
```

`skip` leaves the world alone, `air` carves (mines, crypts). `points` name
cells (`hearth`, `door`, `floor`…); `doors` and `storeys` let the tests
measure a building where it stands.

Stamps are written by `tools/stamps/build.py` from short recipes
(`tools/stamps/kit.py`: houses, roofs, towers, stairs). The game never runs
those; any tool that writes the format will do. Four stamps were baked
from the 0.8.0 world builders and have no recipe: `hollow_mines`,
`old_barrow`, `sunken_shrine` (their files are the source).

## Rules stand on places

Anything in the rules with a position names a place:

```json
{"key": "miners_rest", "name": "Miner's Rest", "place": "miners_rest", "radius": "place", "levels": [5, 5], "safe": true}
{"creature": "candlekin", "count": 5, "place": "hollow_mines", "at": [18, -30], "below": 7, "spread": 10, "respawn": 120}
{"key": "hollis", "name": "Hollis", "place": "hearthstead", "at": [36, 0, -22]}
```

`at` is an offset from the place (`[x, z]` or `[x, y, z]`), `"radius":
"place"` takes the place's reach, and `"below": n` puts an underground camp
n metres under the place's ground. The pack build
(`dyno_server::pack`, `Layout::resolve_rules`) turns these into the offsets
from the spawn hearth that the server and client have always read, so
moving a place in the layout moves its creatures, people and objectives.

## Rebuilding a saved world

`server.json` records the layout version the saved world was built from
(`world_layout`). When a server starts with a pack whose layout has another
version (or with a world from before layouts), it
(`dyno_server::migrate::rebuild`):

1. copies `world.dynw`, `players.json`, `chests.json` and `server.json` to
   `*.before-layout-N` (never overwriting a backup);
2. builds the new world from the layout;
3. moves every character: someone within a place's old reach (`was`) goes to
   the same place, as far from its middle in proportion; someone
   underground goes to the nearest place's entrance; anyone else moves with
   the map. Each lands on open ground. Only `pos` and `spawn` change:
   inventories, quests, money and everything else are untouched;
4. resets the spawn hearth and the regrowing nodes, and records the version.

Blocks of the old world are not carried over; the backup keeps them.

## Tools

```sh
python3 tools/stamps/build.py            # write the stamps from their recipes
dyno map --out map.ppm                   # build the world, draw it from above, list the places
dyno map --design --out map.ppm          # the same at design size (2048 m; a gigabyte of blocks)
python3 tools/world/shots.py OUT map.places.json ADDR   # gameplay-camera shots of every place
dyno bake-stage                          # the title stage: the middle of the world
```

## Tests

`crates/dyno-server/tests/world_scale.rs` builds the pack's world and
measures doors, storeys, road widths, trees, the plaza, running times
between named places, the longest stretch of road with nothing to see, the
hills and the rim against docs/design/scale.md. The pack test in the same
file validates the layout and every stamp.
