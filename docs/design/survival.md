# Hearthblock survival design

Hearthblock is the first game built on Dyno: a block survival game for a small group of friends on one server. This document is the living design. The tracker shows progress against it.

## Theme: light is safety

- Shades rise from darkness. They spawn only where it is dark, hunt at night and fade in daylight.
- Torches, lanterns and hearths push the dark back. A **Hearth** claims safe ground around it (no spawns within its radius) and is a respawn point.
- The long-term shared goal: **rekindle the Great Hearth**. It needs emberstone from deep caves, iron and shade dust. When lit, nights in the valley are safer for everyone.

## Fun review outcomes (22 November 2026)

A design review found these problems and decisions:

- **Nights were 9 minutes, and late joiners arrived in the dark.** Night is now 5 minutes of a 16-minute day. The clock stops when nobody is online and every session starts at dawn.
- **Coal was the hidden gate.** The Journal now points to the furnace and charcoal (smelt a log into coal). One log makes four torches.
- **Torches alone would end the threat.** Light has rules: hearths are sanctuary, strong light burns Shades, and torches outside a hearth's reach can be snuffed (v0.1.2).
- **Hunger was a tax.** One point per 60 seconds; health regenerates at food 10 or more; no starvation damage; you respawn with food 10.
- **Death piles vanished in 5 minutes.** You keep your hotbar; the rest stays where you fell for 30 minutes.
- **The wrong pickaxe deleted ore.** The wrong tier now mines slowly, drops nothing and says which tool you need.
- **Ember was the cheapest tier.** Emberstone is deep only; ember tools are upgrades of iron tools with shade dust.
- **Purpose ran out after about 90 minutes.** The Great Hearth becomes a three-stage shared goal that needs at least two players.

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

- **Health** 20. Fall damage is one point per block beyond three. Death drops everything except your hotbar; the pile lasts 30 minutes.
- **Hunger** 20. Drains one point per 60 seconds of activity. At 10 or more, health regenerates. At 0 you cannot sprint, but you do not starve.
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
| Shade | Hostile, 20 health, 3 damage every 1.5 s, speed 3.8. Spawns 18 to 32 blocks from a player where block light < 4 and it is night or underground, never inside a hearth's reach, at most one every 20 seconds and 2 per player on the first night (then 3). Light 10 or more halves its speed and burns it; daylight destroys it | Shade dust (1 to 2) |
| Crawler (v0.2) | Hostile, caves only, fast, low health | Shade dust, string |
| Wisp (v0.2) | Neutral glowing orb near glowshrooms at night | Wisp light |

## Stations

- **Crafting table**: tools, furnace, chest, bed, door, lantern, hearth. Needed within 5 blocks.
- **Furnace**: smelting. Needs fuel: coal 8 units, log 4, planks 2, stick 1. Each smelt uses 1 unit.
- **Hearth** (v0.1.2): safe radius 16, respawn point. **Great Hearth** (v0.2): radius 48.

## Night rules (light is safety)

- **Hearths are sanctuary.** No Shade spawns or enters within a hearth's radius. A ruined hearth burns at the world spawn (radius 16). A crafted hearth (8 cobble, 4 logs, 2 coal) has radius 16.
- **Strong light burns.** In block light 10 or more (about 3 blocks from a torch, 5 from a lantern), Shades move at half speed and lose 1 health per second.
- **Torches are temporary** (v0.1.2). Outside a hearth's reach, a Shade next to a torch snuffs it after 3 seconds; the torch drops.
- **Held light** (v0.1.2). A torch in hand gives light 10 around you.
- **Away players** are not targeted and do not count for spawning.
- **Sleep.** Right click a bed at night to lie down. When half of the active players sleep, the night runs ten times faster.

## The Great Hearth (v0.1.2)

Three stages at the spawn ruin, each lit at the next dusk once at least two different players have contributed:

| Stage | Needs | Radius |
|---|---|---|
| Kindle | 20 logs, 10 coal | 16 |
| Stoke | 16 iron ingots, 20 shade dust | 32 |
| Rekindle | 24 ember shards, 40 shade dust | 48 |

## Open questions for the first playtest

- Shades wait at a hearth's edge and strike within 1.7 blocks; players reach 4.5. Fighting from just inside the edge is nearly safe. Is that a fair "dust harvest", or too easy?
- The live world was generated before v0.1.1, so it still has shallow emberstone. Only new worlds place it deep.
- The spawn hearth cannot be mined in survival. Crafted hearths can.

## Co-op

- Shared chests, beds (half of the players asleep speeds the night), `/tp` to friends.
- No player damage between players.
- An optional Discord webhook posts milestones, deaths and hearth progress.
- On join you see the last 10 events you missed.
- The first player on a new world becomes its owner and can use operator commands.
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
| v0.1.1 | The night: 5-minute nights, Grazers, Shades, combat, hearths, light rules, sleeping, juice, owner, webhook, new textures |
| v0.1.2 | Together: Great Hearth stages, torch snuffing, held light, signs |
| v0.1.3 | Direction: journal screen, day counter, farming, breeding |
| v0.2 | Depth: Crawlers, Wisps, ember tier, armour, Great Hearth, signs |
