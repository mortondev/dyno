# Fun review: the first hour on the world at WoW proportions

23 November 2026, on main after the merge of the 2048 m world (layout v1,
`477996b`) and the life lane (protocol 22). Measured with the swarm
(`dyno bots`, 20 bots for 12 minutes, 4 solo completionists and 2 parties of
two for 10 minutes, each on its own fresh world) and played by hand through
`dyno play` as four characters: a warrior and a mage before any change, a
hunter and a priest after. How the numbers are graded:
[fun-metrics.md](fun-metrics.md). Reports: `runs/fun-*.md`.

All runs shared a machine with other builds (load average about 70 on 16
cores), so read differences of 10% as noise. The swarm's own spread between
two identical runs is in the table.

## The short version

The bigger world broke the first hour, and not because the world is wrong.
Everything that the old 384 m map kept within 60 m of the hearth (the first
flock, the trees, the coal) had moved out with the map, while quests, XP and
camp sizes were still tuned for the small one.

- **89% of a new player's time was running.** The nearest Grazer stood 260 m
  from the hearth: a minute's run before the first swing, and the flock was
  three or four animals for a quest that asks for ten.
- **Pace fell to 0.45× the design** in the swarm (it was 1.3–1.4× on the old
  map). The first kill came at 177 s.
- **The Hearthstead story chain was a dead end at level 2.** There was no
  coal and no iron anywhere but inside the Hollow Mines, the Old Barrow and
  the Sunken Shrine (level 5 to 10 places, 650 m and more away), and four
  quests need them.
- **Breadcrumbs led to hubs with nothing to give.** "Bread for the Mill" is
  offered at level 2; Ostrin's quests need level 3. I ran 490 m to be told
  nothing, then 500 m back.
- **The North Watch was three one-minute runs** for two "go and talk" quests
  (there, back to Maren, there again).
- Nothing happened on the road. No XP for finding a place, no reason to
  step off the path, no way back but walking.
- The level 8 armour sets dropped nowhere, and no class could wear the
  plate or the mail ones. The three rare elites dropped nothing of their own.
- Bosses took a random level from their area's band (the level 10 Hollow
  Foreman could appear at level 5).

After the changes below the same swarm passes the pace, first-kill and
reward checks, travel is down from 89% to about half, and the hour has a
shape: a ring of things to do round each hub, then a road with places on it
to the next.

## Before and after

20 bots, 12 minutes, mixed brains, fresh world each time. "After" is the
final content, run twice at once on two servers.

| Check (swarm, 20 × 12 min) | Before | After, run A / run B | Target |
|---|---|---|---|
| Time to first kill (median) | 177 s | 42 s / 53 s | <= 60 s |
| Time to level 2 (median; share who got there) | 361 s (55%) | 171 s (95%) / 176 s (100%) | about 300 s |
| XP per hour against the design pace | 0.45× (2,909) | 1.31× (8,412) / 1.44× (9,190) | >= 0.8× |
| Mean XP earned by minute 5 / minute 11 | 216 / 542 | 716 / 1,422 and 704 / 1,575 | |
| Share of XP from quests | 42% | 61% / 62% | about 60% |
| Quests turned in | 37 | 132 / 138 | |
| Fights | 187 | 678 / 703 | |
| Seconds between rewards (median) | 18.6 | 11.7 / 8.2 | <= 60 |
| Blocks walked per reward | 98 | 32 / 31 | <= 60 |
| Share of time travelling | 89% | 49% / 53% | <= 40% (fail > 55%) |
| Share of time fighting | 6% | 30% / 34% | |
| Deaths per hour | 0.8 | 2.8 / 3.7 | <= 1.5 (fail > 4) |
| of which Shades / Grazers / Thornboars | 2 / 0 / 0 (and a Websnare) | 6 / 3 / 2 and 5 / 6 / 3 | |
| Fights won | 97% | 98% / 98% | >= 90% |
| Distinct abilities per fighting bot | 1.7 | 2.3 / 2.5 | >= 2 |
| Most-used ability's share of casts | 90% | 79% / 74% | <= 70% |
| Kills that went to someone else's tag | 19% | 31% / 27% | <= 5% |
| Stuck events per bot-hour | 1.8 | 0.8 / 1.3 | <= 2 |
| Fun checks failing / warning | 5 / 4 | 1 / 3 and 1 / 3 | |

| Solo and party (4 bots × 10 min, own worlds) | Before | After |
|---|---|---|
| Solo: first kill / level 2 / pace / travel / deaths per hour | 95 s / 374 s / 0.96× / 86% / 0 | 61 s / 117 s / 1.54× / 68% / 0 |
| Party: first kill / level 2 / pace / travel / deaths per hour | 112 s / 206 s / 1.17× / 82% / 0 | 50 s / 141 s / 1.29× / 78% / 0 |
| Party: kills lost to another tag | 0% | 0% |

Three things in these tables need saying plainly.

- **The pace is now above the design, on purpose not cut further.** Bots
  never read a quest and always know where the camp is; by hand I reached
  level 2 in under 3 minutes and level 3 at 9 to 10.5 minutes on the final
  content (the design says 5 and about 12). Before the changes I reached
  level 3 by hand at 12.7 minutes, so for one player alone on the server the
  old pace was close to the design: what the swarm's 0.45× measured was
  twenty players sharing six flocks of three or four. Intermediate rounds ran
  at 1.6× to 2.2×; kill XP at 70% and smaller opening quest rewards brought
  it to 1.3× to 1.5×. I would look at real players before taking more.
- **Deaths went up, and most of the rise is what you would expect once
  players reach fights at all.** Before, a bot spent 89% of its time running
  and 6% fighting. The grinder bots, which pull again at 60% health,
  account for most Grazer deaths. The round before this one had 11 and 19
  Grazer deaths a run; lowering Grazer damage from 3 to 2.5 and the bots'
  rest threshold from 50% to 60% brought that to 3 and 6. Shades are still
  the largest single cause, all in the last 2.5 minutes.
- **Kills lost to another player's tag got worse** (19% to about 29%),
  because the players are now all at the flocks instead of strung out along
  the roads. Parties lose none. See "What I would do next".

Travel is still over the 40% target and I do not think the target is right
for this map: the solo run, with no competition, travels 68% because a
completionist bot walks the Lay of the Land and the North Watch chain,
which are walks by design, and bots cannot buy rides.

Tick cost (`DYNO_TICK_STATS=1`, new: mean and worst over each minute; 50 ms
budget): 1.0 ms mean before, 1.4 to 1.5 ms after with 20 bots (the bots now
fight three times as often); worst single ticks are 100 to 260 ms in both,
at joins and when a far section is first built, on a machine at load 70.

## What I played (first person)

### Before: Bryn the warrior, Isolde the mage

I woke by the hearth, took the Cold Hearth and Meat for the Pot, and the
tracker said the nearest Grazer was **268 m south-east**. I ran for 46
seconds through an empty meadow. When I got there the banner read **"The
East Road, level 4–5"**: the first flock stands inside the next zone's
circle. For a level 1 that reads as "you are in the wrong place".

Four Grazers. I killed them in a minute and the meadow was empty, with the
quest at 4/10. I waited for the respawn, got four more, levelled (level 2 at
about four minutes, which is the one thing that was on pace), and it was
empty again at 8/10. I gave up and ran 270 m home with the quest unfinished.
As the mage I found a flock of three.

Kindling was fine: trees 40 m from the hearth, ten logs in half a minute.
Then Brannoc asked for coal and the tracker pointed at **Candlekin, 763 m
east, in the Hollow Mines**. `nodes coal` found none. I never finished a coal
quest as either character, and Iron for Hollis and The Hearth Stirs hang off
it.

Tilda sent me to Ostrin for flour. That is 490 m. I ran it in 75 seconds,
passing nothing I could interact with, arrived at dusk, and Ostrin had no
quest for a level 2. So I ran 500 m back to hand in the Grazers (96 seconds
with the detour round the flock), and was level 3 at minute 17. The North
Watch then cost three more minutes of road for two conversations. When I
went back to Ostrin's at level 3 it was night, I found no Ashcloak in his
fields in the minute I looked (they were there by day in the later run), and
I walked on to Fisher's Landing with two untouched quests in my log.

After half an hour I had been in four fights that I remember and I knew the
south road very well.

What was good even then: the first two levels come at a satisfying rhythm
once you are at a flock; loot goes straight to the bags and a pair of
Mud-caked Boots off my third Grazer felt like a present; the Nightfall
Incursion announcement made me look up from what I was doing.

### After: Tess the hunter, Odile the priest

Out of the north gate and the flock is right there, twelve seconds from
Captain Maren. "Discovered: Hearthmeadow" paid a little XP on the way. Six
Grazers, another flock in sight beyond them, boars past that. I did Meat for
the Pot and Thinning the Herd without ever standing in an empty field.

Then I learned things about the game that the running had hidden:

- **Two players on one flock is miserable for one of them.** Odile and Tess
  attacked the same animals; Tess shot first every time. Odile fought eight
  Grazers, spent all her mana, and got no XP, no loot and no quest credit.
  When I grouped them (`/invite`) it was the opposite: animals died in three
  seconds and both got 85% of the XP. This is the tag system working as
  written, and it is the swarm's "kills lost" number (19% before, 27–31%
  after, none in parties). It is the worst thing left in the first ten minutes.
- **I died four times as the hunter in 25 minutes.** Once because Quick Shot
  had emptied my 52 mana by the fourth Grazer and I kept pulling; once to my
  own new rare, Greatfleece, which `attack grazer` picked as the nearest
  Grazer (it had 6 health left; I came back and it dropped a cloak and
  leggings, the best moment of the hour); once to a Shade at the west gate at
  night, walking home at 16 health; once to a pack of boars at Old Tusk's
  camp. Two of those are on me. The Shade and the boars are the game.
- **The night is where it turns from tense to unfair.** It comes every 16
  minutes and lasts five. Odile was killing boars in Birchrise when it fell;
  three Shades walked in while she was mid-fight and that was that. At level
  3 with 50 health there is no answer except to already be standing at a
  fire.

The quarry worked as a place: "Discovered: The Old Quarry", learned Mining
on the first seam, ten coal in a minute, with the Trapper's Camp and the
wrecked caravan as the next two stops of the same quest. The Lay of the Land
is a walking quest, but each leg is 45 seconds, ends at something with a
name, pays on arrival, and crosses two flocks and a boar pack.

The North Watch is now one trip: find the beacon, hand in to Holt, and he has
four things to do within 100 m of his fire (the lookout, the boars, the coal
in the Beacon Rocks, the Shades at night).

At Garrick's camp I bought a **cart ride to Hearthstead for 15 copper** and
was standing beside Carter Jeb's wagon. I tried to buy one to Miner's Rest
and was told to walk there first. That is the right rule: the first trip to
each hub is still a journey, the fifth is not.

### After, final content: Bryn and Isolde again

The same two classes as the first run, on the final numbers, each at their
own flock.

| | Before | After |
|---|---|---|
| First kill | 1 min 32 s | 50 s (both) |
| Level 2 | 3 min 56 s | under 3 min (both, at the first hand-in) |
| Level 3 | 12 min 40 s | 8 min 40 s (Bryn), 10 min 40 s (Isolde) |
| Quests done by minute 15 | 7 | 9 (Bryn), 9 (Isolde) |
| Named places found by minute 15 | 5, unrewarded | 9 or 10, each paying XP |
| Deaths in the first 15 minutes | 0 | 0 (the hunter, a round earlier: 4 in 25) |
| Longest stretch with nothing but running | 96 s | 76 s (Trapper's Camp to the wrecked caravan, across the town) |

Fights cost a warrior 6 to 10 health and a mage 0 to 3 now, so I could take
four or five Grazers before eating. The mage's first three buttons felt
right: Fireball to open, Fire Blast as it arrives. The warrior is duller:
Heroic Blow when rage allows, and Battle Cry every two minutes; boars cost
her 17 to 21 health each, so that is where she has to eat between pulls.

Isolde finished at Garrick's camp after dark, having already fought two
Shades on the way there, paid Carter Nan 15 copper and was in Hearthstead.
That was the first time in any of the six runs that the night made me make
a decision and not just lose health.

One honest gap: `dyno play`'s `gather log` matched my new "Cart ride to
Garrick's Logging Camp" item, so neither character did Kindling or the coal
chain in this last run (the hunter did, a round earlier, before the ride
existed in her world's rules: ten coal at the quarry in a minute). The item
is renamed and the command now prefers an exact key; I did not replay it.

## Critique by heading

| | Before | After | Still open |
|---|---|---|---|
| **First 5 minutes** | Two conversations, then a minute's run to a flock of four in a field labelled level 4–5 | First swing 12 s after leaving Maren; six to a flock, four flocks at the town's edge, eight more further out | A second player on your flock takes every kill (see grouping) |
| **Pace to 2 and 5** | Swarm: level 2 at 361 s, 0.45× the design. By hand, alone: level 2 at 4 min, level 3 at 12.7 | Swarm: level 2 at about 175 s, 1.3× to 1.4×. By hand: under 3 min and 9 to 10.5 min. Level 5 not reached in any run (the longest was 26 minutes, level 4) | Bots are an optimistic bound; watch real players before cutting more |
| **Quest variety and clarity** | 34 quests; objectives: 16 collect, 11 kill, 6 talk, 4 explore. Kill and collect quests do not say where | 40 quests; 14 explore objectives; texts name the place ("outside the west gate", "up the north road, on the left") | No use-item, escort or defend objectives: those need engine work. Kill quests still count any creature of the kind anywhere |
| **Travel against things to do** | 89% of time; hub to hub is 2.5 minutes of nothing | About half; 23 named places pay XP when found; packs within sight of every road out of Hearthstead; carts between the five hubs once found | Past the first ring the roads are still quiet. No wandering patrols or roadside events (engine) |
| **Combat feel and class identity** | Most-used ability 90% of casts | 74–79%: fights are frequent enough that the second button comes out | Hunter ran dry in four fights (costs cut 30%); Rogue Low Blow still never chosen; party bots press one button (88%) |
| **Danger** | 0.8 deaths an hour, because nobody reached a fight | 2.8 to 3.7 an hour in the swarm, none in the solo and party runs or in my last two characters' first 15 minutes; four for my hunter in 25 | Mages and hunters die most. Boar packs add a second boar often |
| **Grouping** | A pair ran at 1.17× the design against 0.96× solo | A pair gets 85% each and loses almost no kills (0% against 27–31%) | The elites (Rook Varn, the Barrow Warden, the Foreman) now keep their level and say "bring a friend", but nothing in the first hour needs a group |
| **Rewards** | Green quest rewards to level 5; level 8 sets unobtainable; plate and mail sets unwearable; rares drop nothing | Level 8 pieces from seven quests, belts from mine and shrine creatures and from Bree; six rares each drop their own item; warriors and paladins wear plate, hunters mail | Money has nothing to buy before the carts and Bree's belts. No treasure chests (needs a lootable-object block) |
| **Exploration** | Nothing | XP for every named place, once (15 + 10 a level); "Know the Vale" sends you to the four corners for a level 8 set piece | The map does not show what you have found (client) |
| **The night** | Half of all deaths in the last 2.5 minutes of every run | Shades at 40 health and 3 damage (were 44 and 4); incursion waves of 3, 4 and 5 unless level 3+ players are in town | Still the main killer. A night every 16 minutes is too often for a zone this size |

## What changed

### Engine (small, general, all driven by data)

| Change | Where |
|---|---|
| `world.discovery_xp: [base, per level]`: XP the first time you walk into a named area (never for where you wake up). The areas found are saved with the character | `content.rs` `WorldRules::discovery`, `camps.rs` `tick_zones`, `store.rs` `PlayerData.discovered` |
| `world.kill_xp`: kill XP as a share of the design formula | `rpg.rs` `credit_kill` |
| An item with `"ride": {"place", "at", "area"}`: buying it from a merchant takes you there if you have found that area; nothing goes into the bags. No new message: it is a `Buy` and a `Teleport`, so protocol 22 clients can ride | `content.rs` `Ride`, `loot.rs` `take_ride` |
| A camp's rare may have `drops` of its own | `content.rs` `Rare`, `camps.rs`, `creatures.rs` `kill_mob` |
| A camp may set `levels`, narrower than its area's band (bosses) | `content.rs` `Camp`, `camps.rs` |
| Nightfall Incursion waves grow with the defenders of level 3 and up near the town, not with everyone | `events.rs` |
| `DYNO_TICK_STATS=1` also logs the whole tick: mean and worst each minute | `lib.rs` `tick_loop` |
| Bots and `dyno play` list resource nodes round where they stand and again as they move (they listed them once at join, when only the 160 m round the hearth had been streamed, so every gather quest outside the town showed as "no source in reach") | `dyno-bot` `brain.rs` `Nodes` |

No protocol change: the rules reach clients as JSON, and the new fields are
ignored by a client that does not know them. Test:
`crates/dyno-bot/tests/discovery_and_rides.rs`.

### World (layout.json, additive; for the world lane to reconcile)

- `version` 1 → 2 (three new places carry stamps, so a saved world must be
  rebuilt to have its ore).
- New places, appended to `places`: `old_quarry` (site, [-150, -150]: three
  coal seams, an iron vein, boulders, an ore cart), `gate_rocks` (wayside,
  [-96, 26]: a coal seam outside the west gate), `beacon_rocks` (wayside,
  [45, -275]: a coal seam by the North Watch).
- New stamps with recipes in `tools/stamps/build.py`: `coal_seam` (6 coal, 2
  iron at walking height) and `iron_vein` (6 iron, 2 coal).
- Nothing existing was moved. New camps are placed in the rules as offsets
  from `hearthstead`, `north_watch` and `windmill_farm`, so they follow those
  places if they move; if the relief round Hearthstead changes, check the
  twelve flocks between 100 and 200 m of the hearth still stand on open
  ground.
- Quest beacons (the pillar `build_landmarks` raises at each explore
  objective) now also stand at: the Broken Tower, the Old Quarry, the
  Trapper's Camp, the Wrecked Caravan, Watch Hill Lookout, the Forgotten
  Graves, the North Pass, the Barrow Overlook and the Ridge Cut, each 6 to
  10 m off the road.

### Rules (content/rules/hearthblock.json)

The three scripts `tools/playtest/fun_review_rules*.py` are the exact
record. In summary:

- **Flocks:** twelve new Grazer flocks between 100 and 200 m of the hearth
  (four of six at the town's edge), the six old ones raised to five; three
  small boar packs in the meadow, one by the North Watch, Mirepinchers and
  Grazers by Ostrin's farm.
- **Zones:** the East Road's circle drawn in to 190 m (it covered the
  south-east meadow). 23 small named areas on existing waysides, farms,
  ruins and viewpoints, each with the level band of the land round it.
- **Quests:** Thinning the Herd 10 → 8, Kindling 10 → 8; The North Watch is
  handed in to Holt and Report to the Watch is gone; Eyes in the Dark is
  Holt's; Bread for the Mill and Word from the East are offered a level
  later, when their hub has work; breadcrumbs pay 100–120 XP for the walk.
  Seven new quests: The Broken Tower, The Lay of the Land, A Lookout's Eye,
  Word for the Hunter, Webs Between the Roots, The Forgotten Graves, Know
  the Vale.
- **Pace:** kill XP at 70%, the opening quests' XP cut by a third to a half;
  Grazer damage 3 → 2.5.
- **Rewards:** level 8 set pieces as quest choices (shoulders from the
  Barrow Warden, gloves and boots from the mine quests, helms from Wisps on
  the Water, chests from the Foreman, legs from Know the Vale), belts as
  drops and at Bree's; rare drops for Old Tusk, Silkmother, the Drowned Lamp
  and three new rares (Greatfleece, Old Snapper, Quick Fenna).
- **Classes:** warrior and paladin wear plate, hunter mail; Quick Shot 10 →
  7 mana, Barbed Arrow 12 → 9.
- **Bosses:** the Foreman is level 10, the Warden and Rook Varn 6, always.
- **Travel:** five carters (Hearthstead, Garrick's camp, Ostrin's farm,
  Fisher's Landing, Miner's Rest) sell rides for 15 to 40 copper.
- **Night:** Shade 44 → 40 health, 4 → 3 damage.

## What I would do next

1. **Shared tags in the open world** (or an automatic "join their fight?"
   prompt): everyone who damages a creature gets credit and their own loot.
   It is the largest unfairness left and it hits exactly when a launch-day
   crowd is on the first flock.
2. **The night.** Make it rarer (one night in three days of the 16-minute
   clock, or a longer day), let lamps on the roads keep Shades off the road
   itself, and keep the incursion at the gate. Then give the night its own
   rewards so players choose to go out in it.
3. **New objective kinds** in the engine, in this order: use an item at a
   place (light the watchfire, plant the lamp at the graves), collect from a
   world object (grain sacks in the stockade, not only from corpses), defend
   a point for a time, escort. Each one opens five or six quests that are
   not "kill" or "walk".
4. **Treasure:** a lootable object (a chest block with a loot table and a
   respawn) at the viewpoints and behind the elites, and wandering patrols on
   the East Road.
5. **A second carter line** worth paying for: the cart is instant; a visible
   ride along the road would be a sight in itself.
6. **Kill quests with a place**, so "Thieves in the Fields" means Ostrin's
   fields. Bots and the tracker both walked to the wrong Ashcloaks.
7. **Bots:** teach them to rest before the next pull, to buy rides, and to
   read a quest's place; today they overstate deaths and travel.
