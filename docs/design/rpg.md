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

## Classes

The six traditional classes, chosen when you first enter the world (`/class <name>` also works):

| Class | Resource | Armour | Weapons | Abilities to level 10 |
|---|---|---|---|---|
| Warrior | Rage (from hitting and being hit) | Mail, shield | Sword, axe, mace, polearm | Heroic Blow, Battle Cry, Onrush (charge), Rending Cut (bleed), Ground Slam, Last Stand |
| Paladin | Mana | Mail, shield | Mace, sword | Judging Blow, Holy Mend, Valour, Rebuke (stun), Devout Ward, Hallowed Ground |
| Hunter | Mana | Leather | Bow, axe, sword, dagger | Auto-shot, Quick Shot, Barbed Arrow, Snare Shot, Hawk's Eye, Volley |
| Rogue | Energy (10 a second), 5 combo points | Leather | Dagger, sword | Vicious Strike, Finishing Cut, Low Blow, Flurry, Open Vein, Slip Away |
| Priest | Mana | Cloth | Mace, staff, wand | Smite, Mend, Withering Word, Renewal, Fortify, Radiance |
| Mage | Mana | Cloth | Staff, dagger, wand | Fireball, Scholar's Focus, Fire Blast, Ice Ring, Blink, Flame Nova |

Effects available to abilities:
- direct damage (optionally plus a weapon swing);
- healing;
- area damage around you or your target;
- root and stun;
- damage over time and healing over time (ticks every 3 seconds);
- timed buffs: damage, damage taken, attack speed, stats;
- Rogue combo points: builders add one, finishers spend all;
- charge to the target;
- blink;
- a bonus against Shades.

The global cooldown is 1.5 seconds (Rogues 1.0).

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
