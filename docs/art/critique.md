# Concept critique (round 2, voxel set) and what changed in round 3

The goal: Ashfall Vale must look **fun and beautiful**, a world you
instantly want to adventure in. Round 2 (the first Cube World × World of
Warcraft voxel set) was judged against that.

## What worked
- Cube World's clean, colourful voxel charm.
- WoW-style timber houses, the watchtower and arched bridges.
- Golden light, bright water and cosy settlements.

## What held it back (most important first)
1. **No life.** There were no characters, creatures, quest givers or spells, and almost nothing moving. In an MMO, fun reads through activity.
2. **Every zone looked alike.** It was mostly midday green. WoW's zones each have a signature palette and mood.
3. **One tree repeated.** Forests read as green carpet; there was little species, size or colour variety.
4. **Weak composition.** Most shots were high oblique views with no focal point, and some had a large blurred bush filling the foreground.
5. **Little wonder.** Most places had no fantastical element, and some read as Minecraft with shaders rather than a fantasy world.
6. **Flat terrain.** There were empty, level meadows without relief, outcrops or falls.
7. **Inconsistent voxel scale.** Huge bricks sat next to fine leaves.
8. **Murky nights.** Night scenes had low contrast.

## Round 3 rules (now in `tools/concepts/style.txt`)
- **Life in every scene:**
  - adventurers in colourful class gear;
  - townsfolk;
  - quest givers with a gold "!";
  - creatures that fit the place;
  - smoke, birds, butterflies, fireflies and motes.
- **A mood per zone:**

  | Place | Mood |
  |---|---|
  | Hearthstead | Golden mornings |
  | Birchrise | Autumn |
  | Hollow Oak | Teal dapple |
  | Barrowwood | Purple-blue dusk and moonlit nights |
  | Mirrormere | Misty pastel dawn |
  | Fisher's Landing | Sunset |
  | Sunken Shrine | Turquoise |
  | Mines | Dusty orange |
  | The stockade | Firelit night |

- **One wonder per place:** glowing runestones, giant mushrooms, statues, small waterfalls, crystals.
- **Variety:** oaks, birches, pines, blossom trees, autumn tints, flower meadows, outcrops and gentle hills.
- **Composition:** a focal landmark, an interesting foreground, a beautiful vista; the camera may come lower for a cinematic third-person view, but the layout stays.
- **One voxel scale:** about 8 voxels per metre for everything, characters included.
- **Night:** moonlight rim light and bright warm or magical glows for contrast.

## What this means for the game, beyond the art
- **Ambient life systems:** birds, butterflies, fireflies, smoke and motes, placed as data per zone.
- **NPCs with things to do:** a lumberjack chopping, a farmer with a dog.
- **Zone palettes and times of day:** per-zone grading in `render.json`.
- **Wonder props per landmark:** runestones, glowing mushrooms, falls.
- **Tree variety:** the species tables in the vegetation phase.

# Round 3 review (adopted as the reference set)

Round 3 meets the brief: the world is alive (parties, quest givers,
townsfolk, wildlife, spirits), each zone has its own mood, and the
framing is cinematic. Notes for the next pass:

1. **Too busy in places.** The plaza, Logging Camp and Barrowwood are maximalist. Keep calm space and one clear focal point; let the landmark breathe.
2. **The same "wonder" everywhere.** Glowing runestones appear in nearly every scene. Give each place its own wonder, and use runestones sparingly.
3. **The same party in every scene.** The warrior, mage and hunter trio repeats. Vary classes, numbers and activities: a lone fisher, a duel, a group at a quest giver.
4. **Distant castles and cities** are not places in Ashfall Vale. Keep them only as beyond-the-edge scenery (the horizon panorama), never as promised content.
5. **Characters in the concepts are chibi** (Cube World proportions). The character direction stays slender, heroic, WoW-like with exaggerated gear; treat concept figures as placeholders for activity and colour, not proportions.
6. **Seasons mix** within one scene. Each zone keeps one coherent season: spring blossom in Hearthstead, autumn in Birchrise, high summer at the farm, moody late autumn in Barrowwood.
