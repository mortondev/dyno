# Hearthblock Online: the RPG direction

Decision (22 November 2026): Hearthblock becomes a small online RPG in the style of World of Warcraft, on the same Dyno engine. This document replaced the survival design as the main game plan. The systems kept from it are written down here: night and light (below), bags, and profession crafting.

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
| Players break and place any block | The world is a designed place. Players gather from **resource nodes** that respawn (ore, herbs, trees). Building moves to **housing plots** later. Operators have a build mode (`/gamemode build`) to work on the world. |
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

## Classes

The six traditional classes, chosen when you first enter the world (`/class <name>` also works):

| Class | Resource | Armour | Weapons | Abilities to level 10 (level learned) |
|---|---|---|---|---|
| Warrior | Rage (from hitting and being hit; fades out of combat) | Plate, shield | Sword, axe, mace, polearm | Heroic Blow (1), Onrush (1, charge), Second Wind (2, heal), Rending Cut (3, bleed), Ground Slam (4, area and slow), Battle Cry (6), Last Stand (10) |
| Paladin | Mana | Plate, shield | Mace, sword | Judging Blow (1), Holy Mend (1), Seal of Dawn (1, weapon seal), Valour (2), Rebuke (3, stun), Devout Ward (5), Hallowed Ground (10) |
| Hunter | Mana | Mail | Bow, axe, sword, dagger | Auto-shot, Quick Shot (1), Barbed Arrow (1, bleed), Marked Shot (2, opener), Hamstring (3, melee slow), Snare Trap (4, root), Hawk's Eye (8), Volley (10) |
| Rogue | Energy (10 a second), 5 combo points | Leather | Dagger, sword | Vicious Strike (1), Finishing Cut (1), Ambush (2, opener), Low Blow (3, stun), Slip Away (4), Sneak (5, stealth), Flurry (6), Open Vein (8) |
| Priest | Mana | Cloth | Mace, staff, wand | Smite (1), Mend (1), Withering Word (2), Ward of Light (3, shield), Renewal (5), Fortify (8), Radiance (10) |
| Mage | Mana | Cloth | Staff, dagger, wand | Fireball (1), Frostbolt (1, slow), Fire Blast (2), Ice Ring (4, root), Scholar's Focus (5), Blink (8), Flame Nova (10) |

Every class has two or three buttons at level 1, a new ability at level 2 or 3 and another at 4 or 5 (a test in the content loader holds this). The numbers, the play notes and the measurements are in [docs/playtest/classes-1-5.md](../playtest/classes-1-5.md).

Effects available to abilities (all data, in the rules):
- direct damage (optionally plus a weapon swing);
- healing;
- area damage around you or your target;
- root, stun and slow (`slow`: percent and seconds);
- damage over time and healing over time (ticks every 3 seconds);
- timed buffs: damage, damage taken, attack speed, stats;
- an absorb shield (`absorb`: soaks up damage, then breaks);
- a weapon seal (`strike`: extra damage on every auto-attack);
- stealth (`stealth`: creatures notice you from less far, until you strike or are struck);
- an opener (`opener`: a multiplier against a creature that is not yet fighting you, or was first hit within the last 1.5 seconds);
- Rogue combo points: builders add one or two, finishers spend all;
- charge to the target;
- blink;
- a bonus against Shades.

Each ability names its picture (`image`, under `icons/abilities/`, in the pack before play) and its sound (`sound`, under `sounds/abilities/`, streamed when first needed); `tools/abilities/icons.py` and `tools/abilities/sounds.py` make them.

The global cooldown is 1.5 seconds (Rogues 1.0). An ability pressed up to 0.4 seconds before the global cooldown or a cast ends is queued, and goes off the moment it can.

## Design review (22 November 2026)

An independent review against classic MMO conventions found these problems, and they were fixed:
- the first-person-only camera;
- click-spam combat with knockback;
- mobs that always chased the nearest player;
- no equipment, stats, quality or tooltips;
- no character screen, bags, spellbook or professions;
- tool and bed quest rewards;
- loot on the ground;
- generous health regeneration.

Now:
- **Camera:** third person, with the mouse wheel zooming in and out (fully in is first person).
- **Combat:** clicking a creature starts auto-attack at weapon speed.
- **Creatures:**
  - Threat: they attack whoever has hurt them most.
  - Aggro and leashing: they aggro at about 10 blocks (more for higher levels) and give up, heal and walk home if pulled 30 blocks.
  - Neutral creatures fight back.
- **Gear:** equipment with quality colours and class armour and weapon limits. Quests reward green gear choices.
- **Loot:** loot goes straight into your bags.
- **Regeneration:** health returns slowly out of combat (Spirit), and food heals.

Still to do from the review: corpse loot windows, vendors that sell, repairs and durability, ghost runs, a minimap, rested XP, chat channels, a combat log and talents.

## Stats and gear

Formulas (`crates/dyno-sim/src/character.rs`):
- Health is 20 + 5 × Stamina, and mana is 20 + 8 × Intellect.
- Attack power is 2 × Strength (Rogues and Hunters: Strength + Agility). A swing does weapon damage + attack power / 14 × weapon speed.
- Armour reduces damage by armour / (armour + 100 + 50 × attacker level), at most 75%.
- Crit is 5% + Agility / 10 (×2 damage), or 5% + Intellect / 15 for spells (×1.5).
- Class base stats grow each level.

## Screens

| Key | Screen |
|---|---|
| C | Character: paper doll with 17 places (head to ranged), and the stat sheet |
| B or E | Bags: 36 slots, quality borders, tooltips. Right click equips, eats, or sells at a merchant |
| P | Spellbook: every class ability, greyed until learned. Bind any ability to any of the 12 bar keys |
| K | Skills: professions (Mining, Herbalism, Woodcutting, Blacksmithing, Cooking), 1 to 150, rising orange, yellow, green, grey |
| L | Quest log |

The action bar has 12 keys (1 to 0, -, =). By default, Recall is on - and Eat is on =.

## More on gear

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

## Night and light

The dark is dangerous and light is safety. These rules run in every zone (`creatures.rs`, `events.rs`):

- **Shades rise in the dark.** A Shade spawns 18 to 32 blocks from an active player where block light is below 4, at night on the surface or at any time underground. At most one every 20 seconds for each player, and never where players are standing close by. It takes the level of the area it rises in.
- **Hearths are sanctuary.** No Shade rises or enters within 16 blocks of a hearth, or inside a safe area such as Hearthstead.
- **Strong light burns.** In block light 10 or more a Shade moves at half speed and loses health each second.
- **Daylight unmakes them.** A Shade under the open sky at dawn burns away.
- **Away players** are not hunted and do not count for spawning. The clock stops while nobody plays. A night lasts 5 minutes of a 16-minute day; nobody sleeps through it.
- **Nightfall Incursion.** After dusk, when players are near Hearthstead, Shades assault one of its gates in three waves; the defenders share the reward.

## Home

- You make your home at an inn by talking to its innkeeper (Hearthstead's is at the Wayfarer's Rest). Home is where you wake after dying and where Recall takes you. Until then it is the hearth of Hearthstead.
- Beds and chests in the towns are decoration. A bank will come later.

## Crafting

- A recipe names the **station** it is made at. Stations are data (`stations` in the rules): each lists the blocks that serve as it, so the forge and anvil of a smithy are a workbench as well.
- Recipes carry a profession and a skill. Every recipe in the pack can be made in the live world: its station stands in the built world, its ingredients come from nodes, creatures, merchants or other recipes, and its skill can be reached. A test holds this (`world_scale.rs`, `every_recipe_can_be_made_in_the_live_world`).
- Nothing is dug, placed or thrown in play. Players gather from resource nodes; operators build in build mode.

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
