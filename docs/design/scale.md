# Scale: World of Warcraft proportions

Hearthblock should *feel* like a WoW zone: heroic architecture, big trees,
wide roads, open landscapes, a camera that shows a lot of world. This spec
sets those proportions as **ratios to a person (H = 1.78 m)** and **running
times**, so every number can be measured in-engine and checked by tests and
the visual gauntlet.

## How this fits the earlier real-life scale pass

The earlier pass made **bodies** life-size, and they stay that way: people
1.78 m, sheep-sized grazers, boar-sized boars, run 4.4 m/s, jump 0.6 m.
WoW characters are about two yards tall too.

What WoW exaggerates is **everything built around the body** and **the
land**:

- **Architecture is heroic:** about 1.3–1.5× real, so a third-person camera fits indoors and buildings read from afar. This replaces the real-life targets of 3 m storeys and 2 m doors.
- **Hero gear:** shoulders, weapons and helmets oversized (the character work).
- **Trees are big and thick-trunked.**
- **Roads are wide.**
- **Landscape distances are compressed** compared with reality, but much larger than our 384 m map.

## Targets (H = 1.78 m)

| Thing | Target | In metres | Ours (measured, layout v1) |
|---|---|---|---|
| Door | 1.6–1.8 H | 2.8–3.2 m | 3 m |
| Storey, floor to floor | 2.2–2.5 H | 4–4.5 m | 4 m |
| Roof | ridge rise about equal to wall height; eaves ≥ 1 m | | 45° gables: a 9 m house rises 6 m over 4 m walls; eaves 1 m |
| Table top | about 0.5 H | 0.9 m | 1 m (one block) |
| Hero tree (one per glade or landmark) | 10–15 H tall | 18–27 m, trunk 2–3 m thick, crown 14–18 m wide | 18–27 m, trunk 2–3 m, crown 14–18 m (`hero_oak`), planted at landmarks and glades; the Hollow Oak is 26 m |
| Common oak or birch | 6–9 H tall | 11–16 m, trunk 1–2 m thick, crown 8–12 m wide | 11–16 m, trunks 1–2 m, crowns 8–12 m of several leaf clumps |
| Pine | 8–12 H tall | 15–21 m | 15–21 m |
| Main road | 3–4 H wide | 5–7 m | 6 m, kerbed (narrowed from 6–8 m after judge round 2: the concepts' roads are 4–6 m) |
| Trail | 1–1.5 H wide | 2–3 m | 3 m |
| Town plaza | 15–25 H across | 25–45 m | 38 m |
| Rolling hills | rise of 8–20 H | 15–35 m | 16–35 m |
| Ridges at the zone edge | rise of 50–100 H | 90–180 m | 134 m (the world is 256 m tall) |

## Distances as running time (run 4.4 m/s)

WoW paces a starting zone in minutes of running, not metres. Targets:

| Trip | Target | Distance | Ours (measured, layout v1) |
|---|---|---|---|
| Hub to its nearby quest spots (camps, farms, ruins) | 30–60 s | 130–260 m | 38–58 s (Hearthstead to the North Watch 58 s) |
| Hub to the next hub | 2–3 min | 530–800 m | Hearthstead to Miner's Rest 2.6 min, to Fisher's Landing 2.9 min |
| Across the starting zone | 6–10 min | 1.6–2.6 km | 2048 m map; the North Pass to the Sunken Shrine by road is 7.0 min |

So the starting zone should be roughly **2 km × 2 km**, about **5× our map
in each direction**. Streaming (v0.8.0) makes that feasible. Mounts come in
around level 20, as in WoW.

## Density

- **Open meadows between clumps of wood,** not an even carpet of trees. Forests are dense where they are forests, and then they stop.
- **Something every 100–150 m along a road:** a camp, a ruin, a farm, a creature pack or a view. Never more than about 30 s of running without one.
- **Creature packs of 2–5** spaced 30–60 m apart within a camp area.
- **Props cluster at hubs and landmarks;** emptiness elsewhere is deliberate.

## View and camera

- **Landmarks are visible from 400–800 m:** the watchtower, the windmill, the mine headframe. That needs far terrain and landmark level of detail beyond the 160 m streamed view.
- **Gameplay camera** (WoW's retail default zoom is about 28.5 yd, and its default field of view is commonly given as 90° horizontal):
  - default 9 m behind the character, zoom 2–25 m;
  - pitch 10–20° down;
  - about 75° vertical (90°+ horizontal) at 16:9.

  Ours: default 5 m, maximum 12 m, field of view capped at 62°.
- **The character fills about 1/5 of the screen height** at default zoom; the horizon sits in the upper third.

## Where the numbers live

Every proportion above is data: buildings are stamps
(`content/stamps/`, drawn by the recipes in `tools/stamps/`), tree species
and their size ranges, road widths, places and the distances between them
are in `content/world/layout.json` (docs/design/world-layout.md). The
"ours" columns are what `crates/dyno-server/tests/world_scale.rs` measures
in the generated world.

## How it is checked

- **Pack and world tests** measure doors, storeys, road widths, tree heights and trunks, and the running time between named places against these tables.
- **The visual gauntlet** (docs/art/gauntlet.md) judges in-game shots from the gameplay camera against the concept targets, including scale.

## Sources for the WoW figures

- [Eastern Kingdoms length (~26,312 yd)](https://warcraft.wiki.gg/wiki/Eastern_kingdom)
- [Camera zoom cvar (default factor 1.9 ≈ 28.5 yd, max 39 yd)](https://warcraft.wiki.gg/wiki/CVar_cameraDistanceMaxFactor)
- [Field of view (default 90 per Wowhead, adjustable 50–120)](https://www.wowhead.com/news/change-field-of-view-in-wow-with-new-console-command-in-patch-9-2-5-327456)

The proportions in the targets table are estimates from screenshots and
play, not official figures. The running-time targets are the binding ones.
