# The first 30 minutes, against the quality bar

23 November 2026. The checklist is [quality-bar.md](../design/quality-bar.md);
sections 3 (arrival), 4 (quests and flow), 5 (pace and reward) and 12
(social) are this lane's. The wider critique of the first hour, and every
change made, is in [fun-review-2026-11-23.md](fun-review-2026-11-23.md);
this page is the measurement of minutes 0 to 30 per class and per player
type, and what was changed for them.

## How it was measured

- **Swarm:** one mixed swarm of 12 bots for 30 minutes on a fresh world, once
  on main's content ("before") and once on this branch ("after"), with the
  same bot build. 12 bots is two of each class and every player type (4
  completionists, 3 grinders, a party of two, an explorer, a social and a
  casual player). The request was a swarm per class and per brain; the
  compute budget changed to one swarm at a time of at most 12 bots, so the
  per-class and per-brain rows below are slices of one run each, of 1 to 4
  bots. Read a row of one or two bots as an example, not a median.
- **Beats:** `python3 tools/playtest/beats.py --by class|brain REPORT.json`
  (new) reads the swarm report and prints, per group, the median time to the
  first kill, first loot, first step out of town, first discovery, first
  gear reward (a quest that pays a choice of gear), first upgrade put on,
  second ability used, each level, the longest stretch without a reward, and
  so on.
- **By hand:** three characters through `dyno play` on the branch, at the
  same time on one server (a paladin, a hunter and a priest), 20 minutes
  each, scripted phase by phase and read back afterwards; and six earlier
  characters of all six classes, 15 to 38 minutes each (the fun review has
  their notes).

## Before and after, per class

BEATS_CLASS

## Before and after, per player type

BEATS_BRAIN

Targets: first kill within 1:00, level 2 at about 5:00, level 3 at about
12:00, level 4 to 5 at 30:00, a gear upgrade by about 10:00, something
rewarding every 60 to 90 s, no stretch over 2:00.

## Against the quality bar

| Line | Before | After | |
|---|---|---|---|
| **3. First quest accepted within 30 s, starts within sight** | The first quest is handed in within 10 s of joining (the bots' median; Brannoc stands 10 m from where you wake, Maren 15 m). The first quest with anything to do sent you 260 m | The same 10 s; the first flock is 113 m from the hearth, just outside the wall | Met for the quest; "within sight" is the world and client lanes' (a "!" in view, the gate framed) |
| **3. Spawn facing the hearth and a quest giver, tips, music** | not this lane | not this lane | |
| **4. A clear next step** | Coal for the Watch pointed at Candlekin 763 m away in a level 5 to 10 mine; Bread for the Mill led to a farmer with nothing for you | Every Hearthstead quest names its place and that place is under a minute away; breadcrumbs are offered a level later, when their hub has work | Met in `dyno play`'s tracker. Kill quests still count any creature of the kind, so the tracker can point at the wrong camp |
| **4. Variety: four kinds of objective, kill-10 never twice running** | talk, kill, collect, explore exist; the first six quests were talk, collect, kill 10, collect, collect, collect | The first ten are talk, collect, kill 8, explore, collect, collect, explore x3, explore, collect, kill 5 | Four kinds, met. No use, interact, escort or defend objective exists in the engine |
| **4. Breadcrumb chain, first trip out under 60 s, each place introduced by a quest** | First trip 46 s of empty meadow, to a flock in another zone's circle | First trip 12 to 25 s; the Broken Tower, the Quarry, the Trapper's Camp, the caravan, the North Watch, the lookout and Garrick's camp each have a quest that sends you there | Met |
| **4. Short quest text** | 2 to 4 sentences mostly | The three longest of mine cut to 3 sentences | Met for the quests of the first 30 minutes |
| **5. Something rewarding every 60 to 90 s, never over 2 minutes** | Median gap 19 s; median longest stretch 3:27, worst 4:01 | Median gap 12 s; median longest stretch 2:54, worst 6:12 | **Not met.** The long stretches are the hub-to-hub runs (Hearthstead to Garrick's camp is 480 m) and bots waiting for night for Eyes in the Dark |
| **5. Level 2 at about 5, level 3 at about 12, level 4 to 5 at 30** | 5:32, 15:52, 29:26 (half the bots never reached 3 in some classes) | 2:25, 8:28, 16:51; median level at 30 minutes is 4, the best bots 5 | Level 4 by 30 is met with room; **levels 2 and 3 come early** (the bar says 5 and 12). By hand: 3 to 4 minutes and 9 to 12 |
| **5. First upgrade by about 10 minutes** | First gear reward at 15:34 | First gear reward at 3:53, on the character at 3:38 (a drop came first) | Met |
| **5. A visible armour change by about 30** | Chest reward only from a night-only quest | Chest reward from Boar Trouble by day (5 boars 80 m from Holt) | Reward is there by about minute 15 to 20; whether it shows is the character lane's |
| **5. Money for one meaningful purchase by 30** | 6 s 60 c, and nothing to buy but bread and picks | 10 s 90 c; a cart ride is 15 to 40 c, a stone pick or axe 20 c | Met |
| **5. Discovery gives XP and a banner; a treasure or a view off the road** | A banner only | 15 XP + 10 a level for each of 37 named places; three viewpoints are quest objectives | XP met. No treasure: there is no lootable world object |
| **12. A reason to group in the first 30 minutes** | None before level 6 | Old Tusk (Group), from Holt at level 3 to 4, 140 m from the North Watch | Present. Not yet fought by a party: my three characters reached him ungrouped and at different times |
| **12. Shared tags, XP and quest credit, group bonus** | Work in a party (85% of kill XP each for two) | The same | Outside a party the first to hit takes everything: 21 to 31% of kills in a crowd |

## What was changed for the first 30 minutes

All data, in `content/rules/hearthblock.json` (scripts
`tools/playtest/fun_review_rules*.py` are the record) and three additive
places in `content/world/layout.json`, except where marked engine.

- **Something to hit just outside the wall:** four flocks of six Grazers
  113 m from the hearth and eight more out to 200 m; boar packs at 150 m; the
  first quests ask for 8, not 10.
- **Everything a Hearthstead quest needs is within a minute's run:** coal at
  the North Gate Rocks (120 m) and the Old Quarry (210 m, with the iron), coal
  again at the Beacon Rocks beside the North Watch.
- **Each hub is one trip:** The North Watch is handed in where it is found;
  Holt then has five things within 150 m of his fire.
- **A reason to step off the road:** XP for each of 37 named places, three
  explore quests in the first twenty minutes.
- **A gear reward on the clock:** weapon choice from Thinning the Herd (about
  minute 4), boots or cloak from The Lay of the Land (about 10), a chest
  piece from Boar Trouble by day (about 15 to 20; it was only on the
  night-only Eyes in the Dark).
- **A reason to group:** Old Tusk now always stands in a hollow 140 m west of
  the North Watch (level 4, 276 health: too much for one level 3 or 4, a fair
  fight for two), with a group quest from Holt that pays the chest piece and
  a sure drop of his mantle.
- **Starter kit:** six Hearth Bread on top of the berries.
- **Nodes rest for the player who gathered them, not for everyone** (engine,
  `professions.rs`): a second player at the same tree was told "gathered
  recently" with nothing on the tree to show why. Ore still visibly turns to
  stone and regrows for all, now in two minutes.
- **Pace:** kill XP at 70% of the formula (`world.kill_xp`, engine) and
  smaller opening quest rewards, so level 2 does not come in the second
  minute now that the flock is at the gate.

## What I saw by hand (three at once, 20 minutes each)

A paladin, a hunter and a priest, started together, each at a different
gate flock, same quests in the same order, resting to full between pulls.

| | Paladin | Hunter | Priest |
|---|---|---|---|
| Played | 20 min | 19 min | 20 min |
| Level at the end | 3 (920/1400) | 3 (332/1400) | 3 (541/1400) |
| Quests done | 11 | 5 | 9 |
| Named places found | 9 | 9 | 10 |
| Deaths | 0 | 1 (a Shade) | 0 |
| Money | 7 s 19 c | 3 s 48 c | 7 s 80 c |

- The opening is right now. Each of us was fighting within a minute and
  never stood in an empty field; phase one (seven quests' worth of Grazers,
  logs and coal) took seven minutes with honest rests.
- **The paladin did the whole Hearthstead chain** for the first time in any
  run: Kindling, Coal for the Watch, the three places of The Lay of the Land,
  Mining to 5 on quarry coal, six iron, Iron for Hollis handed in at minute
  20.
- **Three people doing the same quests starve each other.** The hunter and
  the priest were told "This has been gathered recently" twelve times at
  trees the paladin had just cut, with nothing on the tree to say so. The
  hunter never finished Kindling and so never got the coal quest. That is
  why nodes now rest per player (the change was made after this run and is
  in the swarm numbers, not in these three).
- The same at Holt's boars: packs of three, three players, two boars each of
  the six we needed. The pack by the watch is now four and returns in 75 s,
  and the quest asks for five.
- **Old Tusk did not appear** for us: his camp's four-metre scatter found no
  open ground in the wood. With a 14 m scatter he stands there (checked on
  the next server: level 4, 276 health, hostile). My party invite also went
  out before the others arrived, so the three reached his hollow alone and
  found it empty. The group fight itself is untested by hand.
- Nobody was confused about where to go except the hunter at the trees.

## For the class-combat lane

Found while playing; none of it is changed on this branch (classes and
abilities are as on main).

- **No class can wear plate, and the hunter cannot wear the mail set made
  for it.** Warrior and paladin are `"armor": "mail"`, hunter `"leather"`;
  the Ironbound, Stormguard and Dawnforged sets are plate, and Wildstalker,
  Thornwatch and Skyhunter are mail. The level 8 quest rewards on this
  branch offer one piece of each of the four level 8 sets, so today the
  plate piece is a choice nobody can use.
- **The hunter runs out of mana in four fights.** 52 mana at level 1, Quick
  Shot 10 on a 6 s cooldown; after the fourth Grazer she is auto-shooting
  only and it takes over a minute to come back. Two of my hunter's five
  deaths started there. Quick Shot at 7 and Barbed Arrow at 9 felt right in
  one swarm (hunter deaths 6.7 an hour to 0), then I reverted them.
- **Rogue:** Low Blow (level 3, 40 energy for 1 damage and a stun) is never
  worth pressing; bots never choose it and neither did I.
- **Warrior:** before level 3 there is one button and a two-minute buff.
- **Deaths by class, final 20-bot swarms:** mages and paladins die most
  (mages 12 and 25 an hour in two of the swarms, paladins 23 in the last
  one, nearly all of it bots of the grinder type, which pull again at 60%
  health); priests almost never. A Grazer costs a mage a third of her health
  if it reaches her; she has no way to stop it reaching her before level 4.
- **Fights are short:** median 5 s. That is why the second ability is 25% of
  casts and no more.

## Still open

- **Tags.** Three players on one flock or one boar pack get a third of the
  kills each and the slowest gets none. Parties fix it; nothing tells a new
  player so. Shared tags in the open world would.
- **The night** (every 16 minutes, 5 minutes long) is still where most
  deaths that feel unfair happen: a Shade arriving in the middle of a fight
  at level 3.
- **No "use" or "escort" objectives:** four kinds of objective exist (talk,
  kill, collect, explore), which meets the bar's count but not its spirit.
- **A visible armour-set change by minute 30** depends on the chest reward
  having its own look; the level 8 sets are out of reach until level 8.
- **Travel share** is still above the 40% target.
