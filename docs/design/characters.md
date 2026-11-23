# Characters: from voxel toys to stylised MMO heroes

Target: stylised World of Warcraft humans. Heroic proportions (broad
shoulders, narrow waist, long legs), defined but simplified anatomy, faces
that read from the game camera (brow, nose, cheekbones, jaw, ears), hands
with a thumb and a finger mass, boots and gloves with cuffs, and armour with
layered plates and trim. Painterly, not photoreal. Voxels stay the medium,
but from the game camera they must read as sculpted surfaces, not boxes.

## 1. Critique of the figures before this work (v0.7.1 + slender pass)

Renders: `docs/design/characters/before/` (`sheet.png`, `people.png`,
`sheet-all-warrior.png`, `sheet-motion.png`, `creator-*.png`).

The person was 1/32-block voxels, composed from 295 combinatorial layers
baked from the old code builders, meshed as greedy-merged axis-aligned
quads, then squeezed by the rig's per-part `shape` (scale and a linear
taper). The taper pass made them slimmer, but every part is still a box.

| Area | What reads from the game camera | Against the target |
|---|---|---|
| **Silhouette** | A stack of boxes: square shoulders, a rectangular torso, legs as two posts. The taper narrows the waist but leaves hard vertical sides and square corners, so the V shape reads as a trapezoid. | WoW humans are all curves at silhouette scale: deltoid caps, lats flaring from a pinched waist, a hip line, calf and forearm swells. Nothing here is round. |
| **Proportions** | ~6.5 heads; the head is a scaled-down cube (0.47×0.52×0.50 of the modelled head); legs 15% longer by stretch, so knees and boots are elongated rather than drawn longer. Arms hang straight from the shoulder corners. | Heroic 7–7.5 heads with a slightly large head; long shins; the shoulder mass sits *above* the chest line and is wider than the hips by half again. |
| **Face** | A flat front plane with two coloured voxels for eyes, a one-voxel nose bump and a coloured mouth line. The scale op on the head halves the face detail, so at game distance the face is a skin-coloured square with dots. Mood (expressions) is a 1-voxel change of the mouth. | Needs a brow ridge that shades the eyes, a wedge nose, cheekbones, a squared jaw and chin, lips; ears on the side of the head per the `ears` feature (hidden, round, pointed). |
| **Hands** | A box at the end of the forearm, sometimes a different colour (gloves). No thumb, no knuckles; the held weapon pierces the box. | A large readable hand: palm and finger mass curled to grip, a separate thumb, a glove cuff that flares. |
| **Feet** | Brown boxes, flat toe, no sole, no heel; the shin taper makes them narrower than the calf. | Big boots with a turned-up toe, a darker sole, a heel and a folded cuff below the knee. |
| **Hair** | A cap of voxels on the head cube; long hair is a slab down the back; the bun a cube; the hood a box over the head. Stair-steps everywhere. | Chunky stylised locks: a swept fringe, volume over the crown, long hair falling in tapered strands over the shoulders. |
| **Armour** | Mail and plate are grey cubes with alternating light/dark voxel stripes; pauldrons are cubes stuck on the shoulder corners; trim is a coloured voxel ring. The chest piece is a box the same shape as the tunic. | Oversized rounded pauldrons in two or three layered lames with a gold rim, a cuirass that follows the pecs and abdomen, tassets over the hips, a tabard hanging in front, gauntlets and sabatons. |
| **Cloth** | Robes are a box from chest to ankle with a stepped hem; the cape is three flat 1-voxel-thick slabs whose edges stair-step when they swing; tunic flaps are thin boxes. | Robes that flare from the waist with a weighted, scalloped hem and a thick trim band; a cape with a collar at the shoulders, folds, and a hem. |
| **Blockiness** | Everything: axis-aligned faces, flat face shading (one shade per side), greedy-merged quads that show every step. Even at 1/32 block a stair-step is visible on every diagonal. | Smooth shading across the body, crisp only where the art wants it (eyes, trim, emblem edges). |

**In short:** the figures read as Roblox characters with a waist. The
remaining problems are the medium (boxy meshing and flat shading) and the
sculpt (box parts with no anatomy), not the colours or the rig.

## 2. What changes

See section 3 (engine) and 4 (art) below, filled in as the work lands.
