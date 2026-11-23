# Playtest findings: Hearthblock Online, the first hour

Found by headless bots (`dyno bots`) and by playing through the text
interface (`dyno play`) on 23 November 2026, against a local server on a
fresh world (`dyno server --port 7540 --op botmaster`, protocol 14, main at
84da9c4 merged). Nothing was changed on the server, protocol or content side;
each item says where to look.

Evidence in `runs/`:
- `swarm-20x12.md` / `.json`: the main run, 20 bots (mixed brains, rotating
  classes) for 12 minutes each, 234 bot-minutes, after the merge of first-hit
  tagging, parties and movement checks.
- `earlier-swarm-20x12.md`: the same before that merge (last-hit credit).
- `solo-4x10.md` and `party-4x10.md`: 4 solo completionists vs. 2 parties of
  two, each on its own fresh world for 10 minutes.
- `play-transcript.txt`: a priest played by hand through `dyno play` from
  level 1 to 3 (Cold Hearth, Meat for the Pot, Thinning the Herd, Kindling,
  Wool for Winter, The North Watch).

Coordinates are x,y,z blocks from the spawn hearth (north is -z).

## After the fixes (23 November, second pass)

Fixed since the first pass (main c535f75 and the `playtest-fixes` branch).
Every content change is in `content/hearthblock.json`; the code changes are
for engine bugs only.

| Problem | Fix | Kind |
|---|---|---|
| 1. Shades of level 9 in the meadow | creatures take the level band of the area they appear in | engine (coordinator) |
| 2. Creatures on tree canopies | spawn on `World::ground`, not on leaves | engine (coordinator) |
| 3. Thornboar (and bandit) stalls | evading creatures take no damage and are put home if they cannot walk there for 3 s; a creature that cannot get at its target for 6 s gives up and evades; it heals and loses its tag on arriving home | engine, `creatures.rs` + `abilities.rs` (`Mob::evade`, `arrive_home`, `note_motion`, `take_hit`); regression tests in `creatures.rs` |
| 4. Iron for Hollis hidden grind | iron ore needs Mining 5, not 25; the quest text says how (stone pickaxe from Brannoc's Kindling or Hollis, practise on coal); Ashcloak Bandits drop raw iron (35%), "they rob the miners' carts" | content |
| 5. Bandits beside the level 1 grazers | the East Road camp at (50,-9) moved to (84,-16) | content |
| 6. Level-ups heal above the maximum | the sheet's maximum for level-ups, food and respawns | engine (coordinator) |
| 9. Pace | talk quests give 40 to 60 XP (was 120 or about 300); first quests ask for more (meat 6, grazers 10, wool 8); explicit XP for the level 1 to 3 quests (120 to 260) | content |
| 10. Bed reward | Wool for Winter gives 5 Hearth Bread instead of a Bed | content |
| 10. World saved every 10 s | working out an ore node and its regrowth no longer mark the world dirty (the regrow list is in the meta already) | engine, `game.rs` `set_block_as`, used by `professions.rs` and `rpg.rs` |
| new: Farmer Ostrin unreachable | his spot was inside the windmill, so the server put him in the cave under it (y -17; `dyno play` `npcs`: "partial path"); moved to the mill's west side (8,-10,97) | content |

Before: `runs/swarm-20x12.md` (main 6e9d11a). After: `runs/after-swarm-20x12.md`
(this branch, fresh world, 20 bots x 12 min), with the solo and party runs
beside it. `runs/after-fixes-without-iron-drop-swarm-20x12.md` is the same run
before the bandit iron drop and the Ostrin move.

| | Before | After | Design / target |
|---|---|---|---|
| XP per hour (median, 20 bots) | 13,694 (2.1x design) | 8,346 (1.3x) | 6,400 |
| XP per hour, 4 solo bots on an empty world | 20,975 | 17,474 | 6,400 (bots are faster than people) |
| Level 2 (median) | 97 s | 218 s | about 300 s |
| XP from quests | 71% | 51% | about 60% |
| Minutes per quest (median) | 1.1 | 1.2 | 3 to 10 |
| Reward gap / bots with a 5-minute drought | 9.5 s / 10% | 9.1 s / 0% | <= 60 s / <= 10% |
| Fights won | 82% | 94% | >= 90% |
| Fights that stalled ("no progress", creature never hurt) | 60 (58 Thornboar) | 0 | 0 |
| Thornboar fights won | 25 of 100 | 82 of 87 | |
| Deaths per hour | 9.0 | 11.0 | <= 1.5 |
| Waiting for spawns | 3% | 12% | |
| Kills credited to someone else's tag | 20% | 26% | <= 5% |
| Rubber-bands | 0 | 0 | 0 |
| "Saved world" lines while ore regrows | 168 in 50 min | 0 in the whole run | only on real changes |
| windmill_breadcrumb (accepted / done) | 10 / 4, then 10 / 0 on a fresh world | 10 / 7 (solo 4 / 4) | |
| iron_for_hollis | 9 / 0 | 11 / 0 in 12 min; 3 / 1 in a 20-minute solo run (5.2 min), which unlocked word_from_east (done) | completable |

Still open:
- **Deaths went up**, from 9 to 11 an hour. Shade deaths fell (20 to 12), but
  level 3 bots now go after Ashcloak Bandits (for raw iron and the level 4
  `thieves_in_fields`) and walk into the stockade at (62,67): Rook Varn
  (elite, level 6) and Cutthroats killed 8, bandits 8. In the 20-minute solo
  run, Ashcloak Bandits caused 13 of 34 deaths. Bandits and the stockade sit
  next to the quests that send level 3 to 4 players to them. The bots overreach
  (they take level 4 quests at level 3 and fight what is near), but players
  will do the same.
- **Pace is still fast for bots** (1.3x the design in a crowd, 2.7x alone);
  people read, explore and get lost, so the crowded number is the closer
  estimate. Kill XP is in code (`progression::kill_xp`), and 20 to 25 kills in
  10 minutes give about the whole design pace on their own. The next lever in
  the data is creature health (grazers die in about 5 s).
- **Ore near the surface is scarce.** `dyno play` `nodes coal`: 81,431 coal
  nodes, of which 2,472 can be gathered while standing in the open, and 4,578
  are exposed in caves. For iron the numbers are 50,799, 470 and 2,995. Coal
  ran out for 12 bots ("no source in reach"). This is world generation, not data.
- **Tag contention** (26%) was not looked at (as agreed).

### Casts by class (after; one-button combat, not redesigned)

From `runs/after-swarm-20x12.md` (levels 1 to 4):

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 94 | Quick Shot 96%, Barbed Arrow 4% |
| Mage | 190 | Fireball 98%, Scholar's Focus 2% |
| Paladin | 99 | Judging Blow 84%, Holy Mend 16% |
| Priest | 180 | Smite 99%, Mend 1% |
| Rogue | 95 | Vicious Strike 85%, Finishing Cut 15% |
| Warrior | 70 | Heroic Blow 84%, Battle Cry 16% |

In the first levels every class has one damaging button. The second
ability is a heal, a buff or a finisher that comes up rarely, because
grazers die in 1 to 3 casts.

### Party vs. solo (after)

| | Party (2 pairs) | Solo (4) |
|---|---|---|
| XP per hour (median) | 14,770 | 17,474 |
| Quests turned in | 39 | 50 |
| Fights won | 84% | 81% |
| Deaths per hour | 15.0 | 18.0 |

Grouping is about 15% slower than solo now (it was 4% before). Solo got
safer and the quests need more kills, which a pair shares (one corpse's loot
for two players). The group bonus (15% per extra member) does not yet make
up for it.

## First pass

### Headline numbers (main run, 20 bots x 12 min)

| | Value | Target |
|---|---|---|
| First kill (median) | 53 s | <= 60 s |
| First quest turned in | 8 s | <= 180 s |
| Level 2 (median) | 97 s | design: about 300 s |
| XP per hour (median) | 13,694 | design pace 6,400 (2.1x) |
| XP from quests | 71% | design: about 60% |
| Minutes per quest (median) | 1.1 | design: 3 to 10 |
| Median gap between rewarding moments | 9.5 s | <= 60 s |
| Bots with a reward drought over 5 minutes | 10% | <= 10% |
| Deaths per hour | 9.0 | <= 1.5 (FAIL) |
| Fights won | 82% | >= 90% (warn) |
| Kills credited to another player's tag | 20% (64 of 318) | <= 5% (FAIL) |
| Share of casts that are the most-used ability | 87% | <= 70% (FAIL) |
| Time: fight / travel / gather / waiting for spawns / stuck / idle / dead | 57 / 18 / 7 / 3 / 3 / 3 / 1 % | dead time <= 15% |
| Rubber-bands from the movement checks | 0 in 234 bot-minutes | 0 |
| Action round trip p95 | 50 ms (the bots' tick; the real value is lower) | <= 150 ms |
| Disconnects | 0 | 0 |

The first ten minutes reward often (a rewarding moment every ~10 s), but the
pace is double the design, death is routine, fights are one button, and two
quest lines stall.

## Problems, most important first

### 1. Night is lethal for new players (9 deaths an hour; 20 of 35 deaths are Shades)

Deaths by cause in the main run: Shade 20, Ashcloak Bandit 5, Gloomwisp 4,
Websnare 3, others 3. Shades (levels 2 to 9) rise 18 to 32 blocks from players
in the dark, and the Nightfall Incursion world event sends waves at the town.
Level 1 to 3 characters in Hearthmeadow lose 15 of 69 fights against Shades
(`Shade: lost (died) x15`). In the earlier run Shades were 11 of 14 deaths.
Explorer and grinder bots died 22 and 17 times an hour.
Look at: Shade level near the hearth (cap it at the zone's 1 to 2 in
Hearthmeadow) and the incursion's strength against low levels.

### 2. Creatures spawn and stand on tree canopies

`crates/dyno-server/src/camps.rs:59` places camp creatures at
`world.surface(x, z) + 1`, and `surface` counts leaves as ground. Grazers
(and their corpses) end up 6 to 8 blocks above the ground on oak canopies,
out of melee reach (4.5) and loot reach (6). One 4-minute run saw 25 distinct
grazers on canopies, e.g. "Grazer lv2 at 25,1,-45 (ground -6)", "Grazer lv2 at
50,2,-24 (ground -6)", "Grazer lv3 at 20,0,-94 (ground -7)" (the bots now skip
them; the report lists them under "Creatures seen on tree canopies"). Before
the bots learned to skip them: 37 corpse-loot attempts with no path to a
canopy and 14 fights lost with "target on a tree canopy".
Look at: spawn on the ground under the canopy (skip leaves and logs), as
`ground()` in `crates/dyno-bot/src/body.rs` does.

### 3. Thornboar fights stall: the boar neither fights back nor dies

58 of 100 Thornboar fights ended with the boar above 85% health after 45
seconds, having dealt **0 damage** to the bot (`Thornboar: lost (no progress:
it hardly took damage) x58`; telemetry `fights[].damage_taken == 0`, 0 to 22
casts each), for every class at levels 1 to 4 against boars of level 3 to 5.
Only 25 boar fights were won. `boar_trouble` and `tusks_and_tempers` were
accepted 17 times and finished 0 times. In a party run a follower spent over
4 minutes in one such fight. Likely cause: the Charge (range 14, three times
speed) carries the boar past the 30-block leash, it evades (heals to full,
keeps no threat), walks home and charges again; damage taken while evading is
healed. Reproduce with `dyno play`: `goto thornboar`, `attack thornboar`,
`look`.
Look at: leash/evade in `creatures.rs` (~line 512) with the Charge ability;
Birchrise camps at (-18,-76), (-58,-104), (-8,-118).

### 4. Iron for Hollis is a hidden grind that gates the east of the map

`iron_for_hollis` (level 4) asks for 6 Raw iron. Iron ore needs Mining 25 and
a stone pickaxe (tier 2); characters start with a wooden pickaxe and no
Mining. Mining rises by one per gather of easier nodes, and the coal quests
before it need only 9 coal, so players arrive with Mining 5 to 9 and must
gather coal about 16 more times (nodes regrow in 4 minutes), with no hint
anywhere. The other source is Candlekin (30% drop, levels 5 to 8, Hollow
Mines). Main run: 9 bots took it, none finished ("Requires mining 25 (have
4 / 5 / 6 / 20)"); in the earlier run one bot finished it after 7 minutes of
raising Mining on coal. Its reward is an iron pickaxe. It gates
`word_from_east` (the whole Miner's Rest chain) and `hearth_stirs`.
Look at: a lower skill need or a level 3 to 4 source of raw iron, and quest
text that names the stone pickaxe (Hollis, 20c) and Mining.

### 5. The starter grazers share the meadow with level 4 to 6 bandits

Hearthmeadow (levels 1 to 2) has grazer camps at (48,22) and (58,-30); an
Ashcloak Bandit camp (levels 4 to 6, aggro radius 10 + level difference, about
15 blocks against a level 1) is at (50,-9), and the East Road area (centre
86,5, radius 52) reaches x = 34. Playing by hand, my level 1 priest was killed
by an Ashcloak Bandit while looting grazers at (55,14) (`play-transcript.txt`,
"DIED: You fell to a Ashcloak Bandit"). Bots died to bandits 5 times.
Look at: move the (50,-9) camp east of x = 70, or the grazer camps west.

### 6. Level-ups (and food, and respawns) heal above the real maximum health

`crates/dyno-server/src/rpg.rs:73-75` sets health to
`progression::max_health(level)` (50 + 12 per level) on a level-up, but the
character's maximum is the sheet's (20 + 5 per Stamina). A priest showed
**HP 62/45** at level 2 and **74/50** at level 3 (`play-transcript.txt`); the
extra lasts until damage. Eating (`game.rs` `use_held`) and respawning (60%
of the progression maximum) use the same wrong maximum.
Look at: use `max_health(id)` (the sheet) in all three places.

### 7. Launch-day contention: kills tagged by others, camps and nodes run dry

With 20 players the first quests compete for about 21 grazers. Thinning the
Herd took a median 7.4 minutes in the swarm and 1 to 2 for a few players;
bots waited for spawns 3% of the time (25% in the earlier run under the old
last-hit rule). 64 of the 318 creatures a bot hit were credited to someone
who tagged them first; nothing tells a player a creature is tagged
(`EntityInfo` has no owner), so attacks are wasted. (7 of the 64 had no other
player within 20 blocks.) Coal ran out: "coal_for_the_watch: collect Coal: no
source in reach" for 14 bots (nodes regrow in 240 s).
Look at: a "tagged" flag for grey nameplates; more grazer spawn points or
faster respawns near the hearth; more coal near the North Watch.

### 8. Fights are one button in the first levels

87% of all casts were each bot's most-used ability. Warriors, mages and
hunters have one damaging ability until level 2 to 4 (Heroic Blow, Fireball,
Quick Shot), and grazers die in 1 to 3 casts (median fight 5.8 s).
Look at: a second ability at level 1 for every class.

### 9. The pace is double the design

Median 13,694 XP an hour (2.1x the design's level 5 in 45 minutes), level 2
in 97 s (design about 300 s); 4 solo completionists on an empty world reached
level 4 to 5 in 10 minutes (about 21,000 XP an hour). Talk and breadcrumb
quests carry much of it: Cold Hearth gives 120 XP for a 20-block walk (median
0.1 min), Report to the Watch 0.1 min, Garrick's Camp 0.3 min. The median
quest takes 1.1 minutes against a design of 3 to 10.
Look at: XP of talk-only quests, or kill and collect counts.

### 10. Smaller things

- **Wool for Winter gives a Bed** that cannot be placed in the protected
  world (`game.rs` `place` refuses non-creative players) and sells for
  nothing (block items have value 0): dead weight in the bags.
- **The world is saved every 10 seconds** while gathered ore regrows (168
  "Saved world" lines in 50 minutes; each save writes the whole 14 MB world).
- **Quests nobody finished in 12 minutes** (accepted / done): boar_trouble
  10/0 and tusks_and_tempers 7/0 (item 3), iron_for_hollis 9/0 (item 4),
  silk_for_sails 7/0 (Websnares at the Hollow Oak are level 4 to 5),
  eyes_in_the_dark 10/1 (night only), stolen_grain and thieves_in_fields 4/0
  (bandits level 4 to 6), down_to_water 4/0.

## Things that work

- **Movement checks: no false positives.** 234 bot-minutes of real-physics
  movement (walking, auto-stepping, jumping, swimming, drops of up to 3
  blocks, warrior charges, recalls, respawns) produced zero rubber-bands.
  Every unasked-for `Teleport` is recorded with where it happened; none came.
- **Parties form**: `/invite` and `/accept` worked every time ("grouped in a
  party of two" for every party bot). Shared kill credit works: in the party
  runs no party kill was lost to another tag.
- **Server health**: no disconnects, action round trips within the bots' 50 ms
  tick, with 20 to 24 bots on one instance.

## Grouping vs. solo

Two parties of two (`--brain party`: the leader invites, leads and goes
where the follower has quest business; the follower accepts, joins every
fight and talks to the same NPCs) against four solo completionists, each
group alone on a fresh world for 10 minutes (`runs/party-4x10.md`,
`runs/solo-4x10.md`):

| | Party (4 bots in 2 pairs) | Solo (4 bots) |
|---|---|---|
| XP per hour (median) | 20,075 | 20,975 |
| Levels after 10 min | 4, 4, 4, 3 | 4, 4, 4, 5 |
| Quests turned in | 36 | 43 |
| Fights won | 86% | 69% |
| Deaths per hour | 4.5 | 13.5 |
| Median fight | 4.2 s | 7.5 s |
| Longest reward drought (median) | 73 s | 79 s |
| Kills lost to someone else's tag | 3 of 84 | 11 of 72 |

Grouping is now about as fast as playing alone (4% less XP an hour), much
safer (a third of the deaths) and the only way the bots beat Thornboars
(24 won, 7 stalled, against 58 stalled solo in the main run). Followers lag
their leaders a little (8 and 6 quests against 11 and 11) because collect
objectives are per player: a corpse's loot is taken once, so the second
member needs more kills for meat, wool and tusks. The group XP bonus (15% per
extra member, split) does not quite pay for that on collect-heavy quests.
Look at: per-member quest drops (each member who shares the kill rolls the
quest item), the common MMO answer.

## Not game problems (fixed in the bots)

Bots drowned while standing still in deep water during fights (players hold
jump), walked to iron nodes under the town, and kept fighting mobs on cliffs
they could not reach. These were bot faults and are fixed in
`crates/dyno-bot`.
