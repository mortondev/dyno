# Hearthblock Online: the RPG direction

Decision (22 November 2026): Hearthblock becomes a small online RPG in the style of World of Warcraft, on the same Dyno engine. This document replaces the survival design as the main game plan. `survival.md` stays as the record of the systems we keep.

## Assessment

### What carries over

| Built for survival | Use in the RPG |
|---|---|
| Server-authoritative world, 20 Hz tick, WebSocket, saves | Unchanged |
| Content as data (`content/hearthblock.json`), `/reload` | Grows to hold quests, NPCs, classes, loot, zones |
| Creatures with AI, spawning, combat, knockback, drops | Become levelled mobs with loot tables |
| Day and night, light, Shades, hearths as sanctuary | World flavour: zones are more dangerous at night; towns are hearth-safe |
| Pack, stacks, crafting stations, recipes | Bags and professions |
| Milestones, toasts, webhook, away replay | Achievements and guild-style news |
| Headless `dyno shot`, scripted end-to-end tests | How every feature is checked |

### What changes

| Survival | RPG |
|---|---|
| Players break and place any block | The world is a designed place. Players gather from **resource nodes** that respawn (ore, herbs, trees). Building moves to **housing plots** later. Operators keep creative mode to build the world. |
| Progress is tools and tiers | Progress is **character level** (1 to 20 at first), **gear** and **quests** |
| Hunger and starving | Removed. Food and drink restore health and mana out of combat. |
| Death drops items | Death sends you to the nearest **hearth** as a spirit; walk back to your body or take a durability penalty. You never lose items. |
| One procedural valley | **Zones** with level ranges, towns, quest hubs and a dungeon. Designed with tools, saved as data. |
| Click to swing | **Target and abilities**: Tab to target, number keys cast class abilities with cooldowns and a global cooldown. |

### Engine gaps (in order of need)

1. **Character model**: level, XP, class, stats, equipment, gold, quest state, saved per player.
2. **NPCs**: friendly entities with names, dialogue, quests and vendors.
3. **Targeting and abilities**: target selection, nameplates, cast times, cooldowns, resources (mana, rage, energy), threat.
4. **Protected world** with respawning resource nodes.
5. **World streaming**: the whole world is sent at join today (256 × 96 × 256). Zones need streaming by chunk (spec WORLD-STREAM) before the map can grow past about 512 × 512.
6. **Parties** and shared credit.
7. **Luau** for quest and boss scripts, once the data model has settled (spec SCRIPT-*).

## Progression (the rate)

The pace follows classic WoW, scaled for a small world of 20 levels.

- **XP to the next level**: 400, 900, 1400, 2100, 2800, 3600, 4500, 5400, 6500, 7600 for levels 1 to 10, then +1300 each level to 20.
- **Kill XP** for a mob of the same level: `45 + 5 × level`. Each level above you adds 5%, to a maximum of 20%. Mobs 1 to 4 levels below give less; grey mobs (5 or more below) give none.
- **Quest XP**: about 5 kills' worth by default (more for long or story quests). A quest takes 3 to 10 minutes.
- **Target pace**: level 2 in about 5 minutes, level 5 in about 45 minutes, level 10 in about 4 hours, level 20 in about 20 hours. About 60% of XP comes from quests.
- **Rested XP** (from the hearth review): logging out at a hearth doubles kill XP for a while.

## Classes (three to start)

| Class | Resource | Role | First abilities |
|---|---|---|---|
| Warden (warrior) | Rage, built by hitting and being hit | Tank, melee | Strike, Shield Bash (interrupt), Taunt |
| Emberwright (mage) | Mana | Ranged damage | Ember Bolt (cast 1.5 s), Frost Snare, Blink |
| Lightbearer (healer) | Mana | Heals, holy damage, strong against Shades | Mend, Radiance (hurts Shades, heals friends nearby), Smite |

New abilities arrive at levels 2, 4, 6, 8, 10, 14 and 20. Talents come later.

## The spell bar

The bar at the bottom of the screen holds abilities, not items:

- Keys 1 to 7: your class abilities, in the order you learn them.
- Key 8: Recall. An 8-second cast that returns you to your bed, or to Hearthstead. It is ready again after 5 minutes. You cannot use it in combat.
- Key 9: eat the most filling food in your bags (out of combat).

Your weapon and gathering tools are chosen from your bags automatically: the hardest-hitting weapon, and the best tool for each node. Items live in your bags (E).

Casting: abilities with a cast time are cancelled by moving. Every ability triggers a 1.2-second global cooldown. Click a creature or press Tab to target it; abilities use that target.

Rage (Warden) builds by hitting (8 a blow) and by being hit, and fades out of combat. Mana refills at 3% a second out of combat and 0.6% in combat.

## Look and feel

The goal is a warm, hand-painted fantasy look:

- 32 × 32 painted textures.
- Sun shadows, cool blue shade and warm light.
- A colour grade with soft highlight roll-off.
- Painted clouds.
- Chunky heroic characters with outfits.
- A gilded interface: portrait frames, parchment quest windows and a carved action bar.
- Hearthstead is a timber-framed village with an inn, a smithy, a watch tower and the Hearthkeeper's library.

## Stats and gear

- **Stats**: Strength, Agility, Intellect, Stamina, Spirit. Health = base for class and level + 10 × Stamina.
- **Slots**: head, chest, legs, feet, hands, main hand, off hand, two rings, trinket.
- **Quality colours**: poor (grey), common (white), uncommon (green), rare (blue), epic (purple).
- **Sources**: quest rewards (choose one of two or three), mob drops, dungeon bosses, crafting.
- **Gold**: copper, silver and gold. Vendors buy junk and sell basics.

## Quests

- Givers show **!** (available) and **?** (ready to hand in) above their heads.
- Objective types: kill N of a creature, collect N items (drops with a chance), talk to an NPC, explore a place, use an object.
- Quests have a level, prerequisites (chains) and rewards (XP, gold, item choice, reputation later).
- A quest log (L) holds up to 20 quests. The HUD tracks objectives on the right.
- All quests are data in the content file, so new quests need no client release.

## The world

| Zone | Levels | Theme | Hub |
|---|---|---|---|
| Ashfall Vale | 1 to 6 | Meadows and birch woods around the ruined Great Hearth | Hearthstead (the spawn) |
| The Hollow Mines | 5 to 10 | Abandoned mines; Shades in the deep | Miner's Rest |
| Gloamwood | 9 to 15 | A forest where night never quite lifts | Lantern Post |
| The Ember Deep (dungeon) | 12 to 15 | Five players, three bosses, under the mountain | |
| Cinder Reach | 15 to 20 | Volcanic highlands where emberstone surfaces | Forge Camp |

The story: the Great Hearth that kept the valley safe has gone out. Shades rise each night. Players rekindle it stage by stage (the staged Great Hearth from the fun review becomes the main story line, and the final stage is a server-wide event).

## Death

You become a spirit at the nearest hearth. Walk back to your body to revive with half health, or revive at the hearth with 10 minutes of weakness. Gear loses 10% durability. No items are lost.

## Release plan

| Release | Contents |
|---|---|
| v0.1.1 | The night (done): creatures, hearths, light, new textures. The last survival release. |
| v0.2.0 Adventurer | Levels and XP, mob levels by zone, NPC quest givers, quest data, quest log and tracker, gold, rewards, nameplates and Tab targeting, protected world with resource nodes, spirit death, the first quest chain in Ashfall Vale (levels 1 to 5) |
| v0.2.1 Classes | Three classes, abilities with cooldowns, mana and rage, cast bars, threat |
| v0.2.2 Gear | Equipment slots and stats, quality colours, loot tables, vendors |
| v0.3 Zones | World streaming, a larger map, Hollow Mines and Gloamwood, flight-free travel paths |
| v0.4 Together | Parties, shared credit, the Ember Deep dungeon, the Great Hearth story event |
| later | Professions, talents, housing plots, Luau quest scripts |

## How we check it is fun

- A scripted bot plays the first quest chain end to end in CI.
- Pace is measured, not guessed: the server logs time to each level, and the playtest target is level 2 in 5 minutes and level 5 in 45.
- Every release gets a headless screenshot tour of the new content.
