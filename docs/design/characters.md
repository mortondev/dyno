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

## 2. Direction

The art direction (docs/art/README.md) is a Cube World × World of Warcraft
hybrid that **stays voxel**. Characters are therefore clearly voxel, in
Cube World's clean, charming and colourful style, with WoW's heroic
proportions and readable gear. The answer to "less blocky" is not smooth
polygons. It is finer voxels (twice the density for the body, 2.5 times
for the head), real anatomy sculpted into them, clean flat colours that
merge into large faces, and soft ambient occlusion.

A first pass meshed the figures as smooth surfaces (surface nets). It read
as sculpted, but it lost the voxel look and cost 3–8× the vertices. That
mesher stays in the engine as an option (see 3), with a `bevel` setting
that keeps the voxel look; the person does not use it.

## 3. Engine (generic)

| Feature | Data | Code |
|---|---|---|
| **Voxel density** per figure and per grid: a figure can be modelled finer than its rig, so the rig and clip numbers (pivots, offsets in rig units) stay unchanged. | figure `"detail": 2`; grid `"detail": 3` | figure.rs (`Vox::set_unit`) |
| **Levels of detail as data**: how many times each level halves the voxels, how it is meshed, the distance it is drawn to, and how much it is simplified. Each level is meshed the first time it is drawn. | `"lods": [{"until": 5}, {"half": 1, "until": 16}, ...]` | figure.rs `LodSpec`, `Built::level` |
| **Shaped meshing** (optional): surface nets over the whole figure, so parts meet without seams. `bevel` 0 gives exact cubes, a little gives a gentle bevel, 1 gives smooth surfaces. Also soft normals, ambient occlusion from a summed occupancy table, painted colour blending, and joints capped and shaded where parts meet (interior caps are merged into rectangles). Quadric edge-collapse simplification locks part seams and the borders of crisp inks. | `"smooth": {"bevel", "soft", "relax", "blend", "occlusion"}`; per level `"smooth": true, "simplify": 0.5`; per part `"smooth": false` | smooth.rs, voxmodel.rs (`Mesh` normals, `emit` with per-triangle camera/sun culling) |
| **Crisp inks**: palette entries whose colour never blends and whose corners never move (eyes, trim, emblems). | palette `"crisp": true` | voxmodel.rs |
| **Values by several features**: rig `at`, `pivot` and part `shape` can be a list by one feature or a map keyed by several (`"1,2"`, `"*"`). | `{"by": ["body","build"], "map": {...}}` | figure.rs `By<T>` |
| **Derived features**: rules that set features and colours a look does not set itself. This is how old NPC looks (`armor`, `pauldrons`, `gloves`, `helmet`) keep composing as the new slot features. | figure `"derive": [{"if": {...}, "set": {...}, "colors": {...}}]` | figure.rs `Figure::derived` |
| **Equipment to look**: each item key sets features, colours and carried things. Your own figure wears your paper doll; others' figures wear what the server sends in `ServerMsg::Wearing` (new, protocol 19, sent with `Wielding`). | `"equip": {"items": {"ironbound_helm": {...}}}` | figure.rs `Look::wear`, app.rs, server game.rs/shard.rs |
| **Sockets**: rig-only bones for the off hand, a shield on the arm, the back, the helm, the shoulders, the cloak and the belt buckle. A part `"held": "<socket>"` draws what the look carries there. | rig bones `socket.*`; equip `"carry": {"shield": ["shield", colour]}` | figure.rs `HeldAt`, `Look::carry` |
| **Creator from data**: the steppers (label, value names, the field they edit, several features packed in one field) and what a new character wears before any gear. **Body** (masculine or feminine) is packed into the `build` field (`build / 3`), so the protocol's `Appearance` and saved characters are unchanged. | `"appearance": {"features": {"body": {"field": "build", "div": 3, "names": [...]}}, "base": {...}}` | figure.rs `Look::steppers`, front.rs |
| **Outfit sheet**: every showcase set on its people, from the front, three-quarter and back, ready and mid-attack, plus close-up portraits and a level-of-detail strip. | `"showcase": {"people": [...], "sets": [...]}` | `dyno shot --outfit-sheet out.png` (sheet.rs) |

## 4. Art (data, generated by `dyno bake-figure`)

`dyno bake-figure [content]` (crates/dyno/src/sculpt/, a dev tool in the
`dyno` binary, not in the client) sculpts the person from signed distance
fields and writes three things: `models/person.vox` (144 layers), the
person's entry in `figures.json` (grids, groups, palette and LODs) and the
`person` rig in `rigs/biped.json` (bones and pivots for each body, build
widths, sockets). The shapes are parametric, so a change of proportion
re-flows every garment and armour piece.

- **Anatomy** (`body.rs`), for two bodies:
  - the torso is built from a ribcage, pectorals or bust, abdomen, pelvis, glutes, lats, trapezius, clavicles and a strong neck;
  - each arm has a deltoid cap, a biceps swell and a forearm that narrows to a flat wrist;
  - big hands have a palm, a curled finger mass and a thumb;
  - thighs have a quadriceps mass, shins a calf, and the feet a heel.
  Heroic proportions: shoulders about 2.8 heads wide, a narrow waist, legs half the height, and a head about 1/7 of the height (slightly large). The feminine body has narrower shoulders, a bust, a narrower waist, wider hips and a smaller head, with its own joints.
- **Head** (head grid, 2.5× detail): cranium, jaw and chin, cheekbones, a brow ridge, a wedge nose with nostrils, lips, eye sockets, and eyes with white, iris, pupil and a glint (crisp).
  - Ears per the `ears` feature: round, pointed, or large.
  - Expressions (`mood`): brows that arch, frown or sit level; a smiling, set or calm mouth; and lids.
  - The `elder` lines.
- **Hair** (`hair_style`): short with a swept fringe and crown volume, long with side locks, a hood, and a bun. Hair thins towards the hairline so it lies on the head. There is also a beard.
- **Everyday clothes**: trousers, boots with sole, strap and folded cuff, a tunic with a V neck and trimmed sleeves, front and back flaps, and a belt with buckle. Also a robe with standing collar and flared skirt, an apron, a sash, emblems, a cape (three segments, folds, trim, clasped collar), a scarf and a candle.
- **Armour** (`gear.rs`): every slot in four kinds, coloured by the item through `<slot>.main/trim/accent/gem`.

| Slot | Cloth (1) | Leather (2) | Mail (3) | Plate (4) |
|---|---|---|---|---|
| chest | high-collared robe with bell sleeves and a scalloped, banded skirt | jerkin over dark sleeves, crossed straps and buckle, leather flaps | mail hauberk under a sleeveless surcoat, mail skirt | cuirass with keel, gorget and lames; mail sleeves and couters; tabard with emblem; plate faulds |
| shoulders | scalloped mantle with gems | two overlapping leather caps | scaled mantle | big domed pauldrons in three lames with a ridge and gem |
| head | circlet with gem | cowl with mask | pointed hood over a mail coif | open helm: dome, crest, cheek guards, nasal, rivets |
| hands | cloth wraps | gloves with flared cuffs | mail gloves, leather cuffs | gauntlets with knuckle plates, finger lames and big cuffs |
| waist | wide sash with hanging tails | belt with pouches | studded belt | girdle with a gem buckle |
| legs | cloth trousers | breeches with knee pads | mail leggings | cuisses, domed knee cops, greaves |
| feet | slippers | tall soft boots with straps | mail boots with fur cuffs | sabatons with toe lames and flared cuffs |
| back | | | quiver of arrows | |

## 5. Item sets

There are four sets, each in three tiers (levels 8, 14 and 20; uncommon, rare and epic). All 87 items are in `rules/hearthblock.json` with stats, armour and icons, and wearable on the paper doll. Their looks are in `figures.json` `equip`, and the existing armour items now change the figure too.

| Kind | Tier 1 | Tier 2 | Tier 3 |
|---|---|---|---|
| Plate (helm, pauldrons, cuirass, gauntlets, girdle, legplates, sabatons) | Ironbound: iron, bronze, red tabard | Stormguard: steel, silver, blue | Dawnforged: gilded white, gold, purple |
| Mail (hood, mantle, hauberk, gauntlets, belt, leggings, boots, quiver) | Wildstalker: green and brown | Thornwatch: teal and bronze | Skyhunter: blue and gold |
| Leather (cowl, spaulders, jerkin, gloves, belt, breeches, boots) | Nightstalker: black and purple | Bloodfang: dark red | Duskwhisper: midnight and silver |
| Cloth (circlet, mantle, robe, wraps, sash, trousers, slippers) | Apprentice's: blue and white | Arcanist's: purple and gold | Magister's: crimson and gold |

Shots are in `docs/design/characters/sets/`: `outfits.jpg` (every set, front
and three-quarter), one sheet per set (`outfits-<set>.jpg`: front,
three-quarter, back, ready, attack, and two portraits, on masculine and
feminine bodies with different builds, skins, hair and expressions),
`outfits-townsfolk.jpg` (everyday clothes) and `outfits-lods.jpg`.

## 6. Before and after

| | Before | After |
|---|---|---|
| Key poses (anim sheet) | `before/sheet.jpg`, `before/people.jpg` | `after/sheet.jpg`, `after/people.jpg` |
| Close-ups and portraits | `before/people-closeup.jpg` | `sets/outfits-townsfolk.jpg` and the set sheets |
| Character creator | `before/creator-*.jpg` | `after/creator-*.jpg` (with the Body stepper) |

## 7. Cost

Measured with `cargo test -p dyno-client --release animated_crowd
crowd_is_cheap figure_budget -- --nocapture` on a shared machine under heavy
load from other builds (load average 13–56). Times move by up to 3× from
run to run, so the vertex counts are the reliable comparison. Cost per
vertex is the same before and after, so time follows the vertex count.

Levels of detail for the person (all crisp voxels):

| To | Voxels | Vertices |
|---|---|---|
| 3.5 blocks (close-ups, the creator) | body 2×, head 2.5× | 152k |
| 5.5 blocks (your own figure at the default zoom) | body 1×, head 2.5× | 57k |
| 8 blocks | body 1×, head 1.25× | 41k |
| 40 blocks | body 0.5×, head 1.25× | 16k |
| beyond | body 0.25×, head 0.6× | 4k |

| | Before (main) | After |
|---|---|---|
| 20 animated people (`animated_crowd`), A/B with the same binary and load | 269k vertices; 2.8, 4.2, 8.1 ms | 357k vertices (1.33×); 3.1, 4.4, 15.9 ms (earlier LOD set, 450k vertices) |
| 20 people within 14 blocks (`crowd_is_cheap`) | 264k | 356k |
| 20 people spread over 60 blocks | 103k | 163k |
| A new look (compose and mesh its first level) | 340–420 ms | 250–670 ms (other levels are meshed when first drawn) |
| `models/person.vox` | 1,055 KB (159 KB deflated) | 2,214 KB (about 940 KB deflated); a first join stays under the 4 MB test |

The 2.5 ms target for 20 people is not met on this machine as loaded today.
Main's own figures measure 2.8–8.1 ms here, and ours cost about a third
more vertices. The LOD distances are data: moving the 5.5-block level to
4.5 and the 8-block level to 6 should bring the crowd close to parity (an
estimate from the per-level counts, not measured), at the cost of
coarser faces on your own figure at the default zoom.

The smooth/bevelled variant of the mesher costs far more: 80–160k vertices
at its finest level even after simplification. That is why the person uses
crisp cubes.

## 8. What is left

- **Other races and creatures** next: dwarf, elf and orc bodies are new
  `Body` parameter sets in `body.rs` (each gets its own rig `at` values
  through `by: body`), and the creatures can be re-sculpted with the same
  tool and their own anatomy modules.
- Faces: closer to Cube World's big, simple, charming eyes; mouth shapes
  per expression read only up close.
- Wire size: the person has two bodies × every garment. If the first join
  needs to shrink, its layers could stream per feature (body, slot) as
  creatures already do.
- Remote players' gear needs protocol 19 (`ServerMsg::Wearing`); older
  clients will not join a new server.
- Held items and shields are still the old models; the shield socket is
  ready for an off-hand item in the paper doll (`carry`).

## 9. The read from the play camera (visual gauntlet, round 1)

The gauntlet's judge sees the figure from the play camera: 9 m behind and
15° down, where it is about 90 pixels tall and drawn at the 40-block level
(6 cm voxels). The concepts show a hero there whose class reads at a glance.
What changed, all of it data or the sculpt tool:

| What | Where |
|---|---|
| **Pauldrons a third bigger** (each wider than the head), with rims thick enough to survive the far levels; bigger leather caps and fuller mantles. | `gear.rs` `shoulders` |
| **A tabard over anything**: panels on chest and back and skirts to the knees, in its own colours, with a broad border. Feature `tabard`, colours `tabard.main` and `tabard.trim`. | `gear.rs` `tabard` |
| **Big devices** (sword or cross, sun, chevron, diamond) on the tabard, on the back of a cloth robe and on a cape, chosen by the `emblem` feature. | `gear.rs` `device`, `robe_device`; `clothes.rs` `cape` |
| **A thicker cape with a broad border**, so it holds together at a distance. | `clothes.rs` `cape` |
| **A slightly large head** (1.14, the rig's `shape` on the head part) and **bigger hands**. | `person.rs` `rig`, `body.rs` `hand` |
| **Hats**: `helm` 5 is a wide straw hat, 6 a miner's hard hat with a lit lamp. | `gear.rs` `helm` |
| **Carried weapons 1.2 to 1.45 times life size** (the first-person size is unchanged). | `figures.json` `held.kinds.*.unit`, `fp` |
| **Starter kits per class**: two garments each, in place of the worn tunic. Warrior: red tabard over mail, iron pauldrons. Paladin: blue tabard with a golden sun, bright pauldrons. Hunter: green tabard over a jerkin, a hood and a quiver. Rogue: dark jerkin, red cowl and scarf. Priest: white and gold robe and hood. Mage: blue and gold robe and mantle. | `rules/hearthblock.json` items and `classes[].starter`; `figures.json` `equip` |
| **Townsfolk dressed for their trades**: the Watch (Holt, Ansel) in mail, blue tabards and helms with spears; Captain Maren in plate with a cape; Hollis in a leather apron with a hammer; Ostrin and Wenna in straw hats; Dunmore in a hard hat with a lamp and a pickaxe; Garrick with a felling axe; Bree with a ledger; Brannoc with a staff. | `figures.json` `npcs` |
| **A hero per class** for the stand shots: a showcase set named by the class key (`dyno shot --stand --hero KEY`; the landmark list names each place's hero, as its concept shows). | `figures.json` `showcase`; `tools/concepts/landmarks.json` `hero` |

`tools/figures/dress.py` holds the kits, the heroes and the townsfolk as
tables and writes them into the pack; `dyno bake-figure` then lays
`figures.json` out again.

Engine (generic, small):
- a look's `light` feature (0 to 15) is a floor under the light on its
  figure, so someone carrying a light is lit at night; `weapon_glow` shows
  the held weapon's glowing layer (models.rs);
- when voxels are halved for a far level, a crisp ink wins a tie against a
  plain one, so trim and devices keep their width (voxmodel.rs);
- `--hero` on `dyno shot --stand`, carried through `dyno scout`;
- the outfit sheet's level-of-detail strip also shows the last six sets
  (the heroes) from behind.

Not done: the character creator still previews the bare look (the class
list sent before joining does not carry the starter kit); a carried light
lights the figure only, not the ground round it (the lighting lane's).
