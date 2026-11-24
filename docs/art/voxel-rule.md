# The voxel rule (hard rule)

**Everything drawn in Hearthblock is made of cubes. No curves, no diagonals,
no slopes, no bevels.** Detail comes from smaller cubes, never from smooth or
angled surfaces.

This is a hard rule for every lane: terrain, buildings, props, trees, ground
cover, characters, creatures, items, water, particles, the far view and the
sky's clouds. A change that breaks it does not merge.

## The cube ladder

Cubes come in a fixed range of sizes, each half the one before:

| Step | Edge | Used for |
|---|---|---|
| 1 | 1 m | world blocks at a distance, far view columns (4 m and 8 m cells are stacks of these) |
| 2 | 50 cm | leaf clumps, coarse terrain rings, large props |
| 3 | 25 cm | ground colour cells, building detail, boulders |
| 4 | 12.5 cm | near terrain, props, ground cover, tree bark |
| 5 | 6.25 cm | small props, held items, creature detail |
| 6 | 3.125 cm | characters and faces, the finest step |

Nothing is drawn with a cube off this ladder. A model picks one step (or two
adjacent steps) and stays on it; cubes of one object share one grid.

## What the rule allows

- **Rigid motion:** a whole voxel part (a limb, a door, a windmill sail, a
  thrown item) may move and rotate as one rigid piece, because animation needs
  it. Its cubes stay cubes on the part's own grid.
- **Shading:** lighting, ambient occlusion, colour variation per cube, fog and
  post effects are free. Shading must not fake a slope or a curve: one flat
  colour per cube face, no smooth normals across steps.
- **Screen-space effects:** bloom, haze, water reflections and the HUD are
  not geometry.

## What the rule forbids

- Sloped or diagonal faces (roof planes, ramps, "one quad over a stair").
- Round solids (round trunks, posts, barrels, pillars, domes).
- Bevels, chamfers and tapers; non-uniform scaling of a voxel part; scaling a
  part off the ladder.
- Smooth meshes (surface nets, marching cubes), smooth or bent normals that
  make stepped ground read as a slope.
- Textures that draw curves or diagonal lines across a face larger than their
  cube step: a face's picture is itself a grid of ladder cubes.
- Billboards and flat cards for plants, particles or clouds.

Roofs are stairs of cubes. Round things are stepped circles. Hills are
terraces. Water is a flat cube face with cube-stepped ripples and foam.

## How it is enforced

- **Mesh test:** every mesher's output is checked: every face normal is one of
  the six axis directions in its part's frame, and every vertex lies on the
  ladder grid of its object.
- **Pack test:** rigs carry no taper, bevel or non-uniform scale; every model
  names a ladder step; shape rules carry no `round`, `slope` or chamfer.
- **The visual judge:** "voxel quality" fails any scene with a visible curve
  or diagonal.
- **Concepts:** the art brief (`tools/concepts/style.txt`) states the rule;
  concept images that show slopes or curves are regenerated.
