# Hearthblock survival design

Hearthblock is the first game built on Dyno: a block survival game for a small group of friends on one server. This document is the living design. The tracker shows progress against it.

## Theme: light is safety

- Shades rise from darkness. They spawn only where it is dark, hunt at night and fade in daylight.
- Torches, lanterns and hearths push the dark back. A **Hearth** claims safe ground around it (no spawns within its radius) and is a respawn point.
- The long-term shared goal: **rekindle the Great Hearth**. It needs emberstone from deep caves, iron and shade dust. When lit, nights in the valley are safer for everyone.

## Core loop

1. Gather wood by hand. Craft planks, sticks and a crafting table.
2. Make wood tools, then stone tools. Make torches from sticks and coal.
3. Survive the first night: light, shelter, a door.
4. Go underground for coal and iron. Smelt iron in a furnace.
5. Go deeper for emberstone. Make ember tools, which glow in your hand.
6. Build a home and a village with friends: chests, beds, farms, hearths.

## Progression tiers

| Tier | Head material | Mines | Speed | Durability | Sword damage |
|---|---|---|---|---|---|
| 0 Hand | none | dirt, wood, leaves (slowly) | 1 | none | 1 |
| 1 Wood | planks | stone, coal | 2 | 60 | 4 |
| 2 Stone | cobble | iron | 4 | 130 | 5 |
| 3 Iron | iron ingot | emberstone | 6 | 250 | 6 |
| 4 Ember | ember shard | everything | 9 | 1000 | 8 |

Break time: with the right tool, `hardness × 1.5 / speed` seconds. Without it: `hardness × 1.5` if the block needs no tool, else `hardness × 5` and nothing drops.

## Survival stats

- **Health** 20. Fall damage is one point per block beyond three. Death drops your items where you fell.
- **Hunger** 20. Drains about one point per 45 seconds of activity. When at 18 or more, health regenerates. At 0, you take damage down to 1 health.
- **Air** 10 seconds under water, then one point per second (v0.1.2).

## Food

| Food | Hunger | Source |
|---|---|---|
| Berries | 2 | Berry bushes at forest edges |
| Raw meat | 3 | Grazers |
| Cooked meat | 8 | Furnace |
| Bread | 5 | Farming (v0.1.3) |

## Creatures

| Creature | Behaviour | Drops |
|---|---|---|
| Grazer | Passive, wanders meadows by day, flees when hit | Raw meat, wool (White) |
| Shade | Hostile. Spawns where block light < 4 and it is night or underground. Burns in daylight | Shade dust |
| Crawler (v0.2) | Hostile, caves only, fast, low health | Shade dust, string |
| Wisp (v0.2) | Neutral glowing orb near glowshrooms at night | Wisp light |

## Stations

- **Crafting table**: tools, furnace, chest, bed, door, lantern, hearth. Needed within 5 blocks.
- **Furnace**: smelting. Needs fuel: coal 8 units, log 4, planks 2, stick 1. Each smelt uses 1 unit.
- **Hearth** (v0.1.2): safe radius 16, respawn point. **Great Hearth** (v0.2): radius 48.

## Co-op

- Shared chests, beds that skip the night when everyone sleeps, `/tp` to friends.
- No player damage between players.
- Death drops items, so friends can recover them.

## Modes

- **Survival** is the default. **Creative** (`/gamemode creative`) gives free blocks, flight and the fill tool.

## Content as data

Items, recipes, block rules (hardness, tool, drops), creatures and milestones live in `content/hearthblock.json`. The server loads it and sends it to players when they join, so new items and recipes need no client release. `/reload` re-reads the file while people play. This is the step toward the spec's Luau content store.

## Release slices

| Release | Contents |
|---|---|
| v0.0.8 | World format: caves, ores, bedrock, sky light, new blocks |
| v0.1.0 | Mine, collect, craft: inventory, timed mining, drops, recipes, tools, torches, health, death, modes |
| v0.1.1 | The night: hunger, food, Grazers, Shades, combat |
| v0.1.2 | Home: chests, beds, doors, smelting, hearths, drowning |
| v0.1.3 | Direction: journal milestones, day counter, farming, breeding |
| v0.2 | Depth: Crawlers, Wisps, ember tier, armour, Great Hearth, signs |
