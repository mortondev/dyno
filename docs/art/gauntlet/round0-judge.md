# Gauntlet round 0: judge's report

Judged from the 18 pair images only (left: game, right: target), each half viewed at 1280×720.

**Overall average: 2.65 / 10.** The game reads as a different renderer from the target: smooth, texture-mapped surfaces in midday light with no inhabitants, against a target of crisp per-voxel colour, dense ground cover, layered mountains and a populated scene.

Scoring rules I applied:
- **Layout and camera distance are not penalised.** Look and proportion are.
- **Water is n/a where the game half shows no water,** even if the concept has a pond. Averages use scored cells only.
- **The stand-in figure is not scored** under Life or Scale.
- **Stills only:** water motion and NPC routines cannot be seen, so Water is scored on colour, reflections and foam, and Life on what is present in the frame.

## 1. Scores

| Location | Lighting | Atmosphere | Voxel quality | Water | Vegetation | Scale | Life | Composition | Avg |
|---|---|---|---|---|---|---|---|---|---|
| ashcloak_stockade | 2 | 2 | 2 | n/a | 2 | 3 | 2 | 2 | 2.14 |
| barrow_crypt | 3 | 3 | 2 | n/a | 2 | 3 | 1 | 3 | 2.43 |
| emberford_bridge | 4 | 3 | 3 | 2 | 2 | 3 | 2 | 4 | 2.88 |
| fishers_landing | 3 | 3 | 3 | 3 | 2 | 4 | 1 | 4 | 2.88 |
| hearthstead_dusk | 4 | 4 | 3 | n/a | 2 | 4 | 1 | 3 | 3.00 |
| hearthstead_plaza | 4 | 3 | 3 | n/a | 2 | 4 | 1 | 4 | 3.00 |
| hearthstead_street | 4 | 3 | 3 | n/a | 2 | 4 | 1 | 4 | 3.00 |
| hollow_mines | 3 | 2 | 2 | n/a | 1 | 2 | 1 | 2 | 1.86 |
| hollow_oak | 4 | 3 | 2 | 2 | 1 | 3 | 1 | 2 | 2.25 |
| logging_camp | 4 | 3 | 2 | n/a | 2 | 3 | 1 | 3 | 2.57 |
| miners_rest | 4 | 3 | 3 | n/a | 2 | 3 | 1 | 3 | 2.71 |
| mirrormere | 3 | 4 | 3 | 3 | 2 | 4 | 1 | 3 | 2.88 |
| north_watch | 4 | 3 | 3 | n/a | 2 | 3 | 1 | 4 | 2.86 |
| old_barrow | 4 | 2 | 2 | n/a | 1 | 3 | 1 | 3 | 2.29 |
| sunken_shrine | 3 | 4 | 3 | 3 | 2 | 3 | 1 | 3 | 2.75 |
| watch_tower | 4 | 3 | 3 | n/a | 2 | 4 | 1 | 3 | 2.86 |
| windmill_farm | 3 | 4 | 3 | 2 | 2 | 3 | 1 | 3 | 2.62 |
| wrecked_caravan | 4 | 3 | 3 | n/a | 2 | 3 | 1 | 3 | 2.71 |
| **Average** | **3.56** | **3.06** | **2.67** | **2.50** | **1.83** | **3.28** | **1.11** | **3.11** | **2.65** |

## 2. Gaps per criterion

### Lighting (3.56)
Daytime sun shadows exist and are the strongest thing in the game shots, but they are large soft dark blobs with no visible ambient occlusion under eaves, at wall bases or between props. The target has warm sunlight against cool blue shadow; the game has one neutral white light and saturated flat green. Lanterns in the target are bright warm cubes that light the ground and walls around them; in the game they are small pale rectangles with a faint halo and light nothing (hearthstead_dusk, hearthstead_street). The two night shots are close to black, with no moon and no blue moonlight fill, where the target nights are fully readable in blue with orange pools at torches.

### Atmosphere (3.06)
The target always closes the view with forest, rolling hills and then mountains fading into blue haze; the game horizon is a bare line of grass with a few lollipop trees, and no hills or mountains in any shot. Game clouds are thin smeared streaks; the target has chunky cubic cumulus. Time of day is wrong in several shots: fishers_landing, mirrormere, watch_tower and miners_rest are sunset or dawn in the target and plain midday in the game, and old_barrow is purple dusk in the target and midday in the game. Distance haze is visible only in mirrormere, sunken_shrine and windmill_farm, and there it is a flat white wash rather than blue aerial layers.

### Voxel quality (2.67)
This is the core identity gap. The target is built from small cubes, each with its own flat colour; the game shows large smooth surfaces carrying tiled photographic-style textures (cobble, brick, bark, shingle, sand ripple) that stretch and blur, most visibly on the ground in every shot and on the cliff in hollow_mines and miners_rest. The ground is a smooth sloped mesh with a blurry grass texture, not stepped voxels. Path edges are soft smeared blends between textures (emberford_bridge, wrecked_caravan), where the target has a stepped cube edge with scattered loose stones. Texel scale is inconsistent: the mound in old_barrow and barrow_crypt is studded with metre-wide cubes each wearing a whole stone-wall texture.

### Water (2.50, six shots)
Game water is a flat, opaque pale blue sheet with darker square patches and no reflections of sky, trees or buildings; the target is deep blue (or clear turquoise at the shrine, with the bed visible) with sun or moon glitter paths and mirrored lantern light. The game does have a white foam band at the shore in fishers_landing, which is the closest match, but it is a soft smear instead of stepped white voxel foam. There are no lilies, reeds or cattails, no ripples around objects and no waterfalls. Artefacts: bright blue columns hang under the dock in mirrormere and sunken_shrine, and the shore in windmill_farm ends in a grey smeared puddle.

### Vegetation (1.83)
The lowest-scoring visual criterion. Leaf crowns are smooth balloon shapes wrapped in one repeating leaf texture, some stretched into long sausages (emberford_bridge, wrecked_caravan); the target has chunky cube clumps with lighter tips, visible branches and gaps of sky. Trunks are thin poles under those crowns, where the target has thick, flared, buttressed trunks. In hollow_oak, old_barrow and barrow_crypt most trees are bare trunks with stub branches and detached pieces floating in the air, and the hero oak has no crown at all. Ground cover is almost absent: a few thin dark blades on a flat texture against the target's dense tufted grass, flower clusters in four or five colours, mossy boulders, reeds and bushes. There is effectively one tree species and one size per shot, with no birch, pine, willow or blossom variety mixed in.

### Scale and proportion (3.28)
Buildings are the best part: the town tower and gabled houses have plausible heroic bulk beside the figure. Roads are the main failure; the game paths are 1–2 m ribbons (emberford_bridge, north_watch, wrecked_caravan, miners_rest), where the target roads are 4–6 m with kerbs. Tree proportions are inverted: crowns are huge and trunks spindly, where the target has trunks one to two people thick. The landscape has no breadth: no rolling hills or far ridges, and hollow_mines is a slot between two walls. Outlying landmarks are undersized next to the target: the north_watch tower is a slim shaft about two people wide, the windmill has short vertical sails, and the "caravan" is a pile of crates.

### Life (1.11)
Apart from the stand-in there is essentially nobody: I can make out one possible figure on the bridge in emberford_bridge and nothing else in 18 shots. The target has, per shot, two to six NPCs or players, a quest giver with a gold "!", one or two animals (deer, sheep, ducks, crabs, a heron, a spider), chimney smoke, birds, and at night fireflies and wisps. The game has no chimney smoke, no birds, no campfires burning, no motes. The only ambient motion cue is the glow from scattered teal cubes.

### Composition and readability (3.11)
Town shots have a clear landmark (the tower), but most outlying shots do not lead the eye: there is no road from the figure's feet to the landmark, no framing trunk or canopy at the frame edge, and no foreground of rocks and flowers. Large empty planes dominate the lower half of the frame (grey sand in ashcloak_stockade, flat green in logging_camp and north_watch, sand in windmill_farm). Clutter and artefacts: glowing teal cubes and flower planter boxes dropped in open fields in nine or more shots, purple-grey patches on the sand in fishers_landing, a translucent grey-blue smear in front of the crypt door in barrow_crypt, a wall cutting across the foreground in hearthstead_dusk and watch_tower, and a field of identical posts in the town garden.

## 3. The ten changes, ranked by impact

1. **[life] Populate every landmark.** Add 2–5 NPCs with a "!" quest giver, a creature pack or grazing animals, chimney smoke, birds by day, and fireflies and wisps by night. Life is 1.1 and is empty in all 18 shots; this is the single largest gap.
2. **[vegetation] Replace the leaf material and crown surface with chunky leaf clumps.** About 50 cm cubes with per-clump colour variation, lighter tips and gaps showing branches and sky, instead of smooth blobs with a repeating texture. Affects all 16 shots with trees.
3. **[terrain] Make the ground read as voxels.** A stepped surface with per-voxel colour variation for grass, dirt, sand and rock in place of smooth slopes with blurred, stretched textures; hard stepped edges where path meets grass. Affects all 18 shots, and the cliff faces in hollow_mines and miners_rest.
4. **[vegetation] Dense ground cover and foreground dressing.** Grass tufts everywhere, flower clusters in several colours, mossy boulders, bushes, reeds and cattails at shores, loose stones along path edges. Affects all 18; the target foreground is never bare.
5. **[light] Distant backdrop and sky.** Layered forest, hills and mountains fading into blue haze on every horizon, plus cubic cumulus clouds in place of smeared streaks. Affects all 16 outdoor shots with a visible horizon.
6. **[world] Tree shapes, sizes and species.** Thick flared trunks (1–2 m), spreading branch structure, mixed oak, birch, pine, willow and blossom trees in clumps, one hero tree per landmark, and repair the bare-trunk trees with floating pieces in hollow_oak, old_barrow and barrow_crypt. Affects 16 shots and is critical for 3.
7. **[light] Per-shot time of day and colour grade, and readable night.** Sunset and dusk grading where the target calls for it (5 shots), a moon with blue fill at night (2 shots), warm sun against cool shadow by day, ambient occlusion in corners, and lanterns and windows that bloom and cast warm light on their surroundings.
8. **[world] Roads and landmark dressing.** 4–6 m roads with kerbs leading from the viewpoint to each landmark, fences, lantern posts and banners along them; bulk up the outlying landmarks (north_watch tower, windmill sails, a real wagon for the caravan, a headframe and rails at the mine, a gatehouse on the stockade) and remove the stray teal cubes and planter boxes from open ground. Affects about 14 shots.
9. **[water] Water surface.** Deeper, more saturated colour, reflections of sky, sun or moon and lights, stepped white foam at shores and around posts, lilies; clear turquoise shallows at the shrine; remove the square dark patches and the blue columns under docks. Affects 6 shots now, more once ponds and rivers are added.
10. **[characters] The player figure.** A heroic silhouette with coloured tabard, oversized shoulders and a weapon that reads from behind, in place of the small beige figure, plus a held light source at night. Present in all 18 shots but not scored this round, hence last.

## 4. The three worst locations

1. **hollow_mines (1.86).** The frame is two blurred grey cliff walls with a small patch of grass and a plain door frame between them. There is no sky to speak of, no vista, no headframe, rails, carts or miners, and the rock is the most smeared texture in the set. The target is an open hillside mine with a valley view.
2. **ashcloak_stockade (2.14).** Nearly black: no moon, no moonlight, and the lower half of the frame is a featureless grey sand plane with one crate. The stockade is a dark slab of bark texture with no gate, torches or banners readable, and nobody is there. The target is a clearly lit blue night with a torch-lit gate, bandits, a river and a farmhouse.
3. **hollow_oak (2.25).** The landmark, a giant oak, appears as a bare brown shaft with detached brown and white pieces floating around it, surrounded by more leafless poles with stub branches. It reads as broken rather than as a tree. The target is a huge crowned oak with a doorway, banners, mushrooms, webs, guards and a pond.

Close behind: old_barrow (2.29, same bare trunks, midday instead of dusk) and barrow_crypt (2.43, dark, with a translucent smear over the path at the crypt door).
