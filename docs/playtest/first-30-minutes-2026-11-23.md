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
QB_ROWS

## What was changed for the first 30 minutes

All data, in `content/rules/hearthblock.json` (scripts
`tools/playtest/fun_review_rules*.py` are the record) and three additive
places in `content/world/layout.json`, except where marked engine.

- **Something to hit within 100 m of the plaza:** four flocks of six Grazers
  at the town's edge and eight more out to 200 m; boar packs at 150 m; the
  first quests ask for 8, not 10.
- **Everything a Hearthstead quest needs is within a minute's run:** coal at
  the West Gate Rocks (95 m) and the Old Quarry (210 m, with the iron), coal
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

HAND_NOTES

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
  (mage 12 to 25 an hour in the grinder brain, which pulls again at 60%
  health), priests almost never. A Grazer costs a mage a third of her health
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
