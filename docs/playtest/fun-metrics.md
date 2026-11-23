# Measuring fun

"Fun" is not one number, but the parts of it that an MMO's first hour lives
or dies by can be measured: how often the game rewards you, how long you go
without anything happening, how much of your time is spent doing things
rather than waiting, whether fights are winnable and varied, and whether you
can get where you need to go. The swarm playtester (`dyno bots`) measures all
of these and grades them against the thresholds below. The thresholds come
from the design pace in [docs/design/rpg.md](../design/rpg.md) ("level 2 in
about 5 minutes, level 5 in about 45 minutes, about 60% of XP from quests,
a quest takes 3 to 10 minutes") and from what classic MMOs aim for.

Each check is **pass**, **warn** (between the two limits) or **fail**. A
check with no measurements says **no data**, never pass.

## Running it

```sh
# 20 bots for 10 minutes against a server; the report is JSON plus Markdown beside it
dyno bots 127.0.0.1:7400 --count 20 --minutes 10 --report playtest/report.json
# the same on a fresh in-process server and world
dyno bots --local --count 20 --minutes 10
# one personality or class only
dyno bots 127.0.0.1:7400 --count 6 --brain grinder --class mage
# an AI (or a person) plays through a text interface: one command per line
dyno play 127.0.0.1:7400 --as Wren --class priest            # stdin
dyno play 127.0.0.1:7400 --as Wren --script first-quests.txt # a script
```

Brains (`--brain`): `mix` (default, per 10 bots: 3 completionists, 2
grinders, an explorer, a social and a casual player, and a party of two),
`completionist`, `grinder`, `explorer`, `social`, `casual`, `party` (bots
pair up: the first of each pair leads and `/invite`s, the second `/accept`s,
follows, joins its fights and talks to the same NPCs). Compare `party` with
`completionist` in the "By brain" table to see whether grouping pays. Bots join with fresh characters (a run id in
their names), rotating through the classes.

In `dyno play`, type `help` for the commands. Every answer ends with a
description of the situation and a line `[ready]`. Positions are x,z blocks
from the spawn hearth (north is -z). The body keeps living between commands
and defends itself if attacked; `auto <minutes>` hands control to a scripted
player for a while.

## Rewarding moments

A rewarding moment is any of: a kill that gave XP or quest credit, loot
taken from a corpse, a quest turned in, a level gained, an item you never
had before, a named area you had not been to, a successful gather. Moments
within 2 seconds of each other count as one (a kill, its loot and a level-up
are one moment).

| Check | Pass | Fail | Why |
|---|---|---|---|
| `reward_gap`: median seconds between moments (median over bots) | <= 60 | > 90 | A player should feel progress at least every minute or so. |
| `drought`: share of bots with a gap over 5 minutes | <= 10% | > 25% | Five minutes with nothing is where players quit. The gap to the end of the session counts. |
| `distance_per_reward`: blocks walked per moment | <= 60 | > 120 | Walking is not a reward; long walks between rewards feel like chores. |

## Pace

| Check | Pass | Fail | Why |
|---|---|---|---|
| `first_kill`: seconds to the first kill (median) | <= 60 | > 120 | The first minute should have action in it. |
| `first_quest`: seconds to the first quest turned in | <= 180 | > 300 | The first quest teaches the loop; it must be quick. |
| `level_2`: seconds from level 1 to level 2 | <= 300 | > 600 | Design pace: level 2 in about 5 minutes. |
| `xp_rate`: XP per hour / design pace (4800 XP in 45 min = 6400/h) | >= 0.8 | < 0.5 | Level 5 in about 45 minutes. |
| `quest_share`: share of XP from quests | >= 0.45 | < 0.3 | Design: about 60% from quests; questing should beat grinding. |
| `quest_time`: minutes from accept to turn-in (median) | <= 10 | > 15 | Design: a quest takes 3 to 10 minutes. |

## Time

Every tick of every bot is put in one bucket: travel, fight, loot, gather,
town (talking, vending, standing in a safe hub), rest, wait_spawn (at a camp
with nothing alive to fight), stuck, dead, idle.

| Check | Pass | Fail | Why |
|---|---|---|---|
| `dead_time`: share idle + stuck + waiting for spawns + dead | <= 15% | > 30% | Time where the game gives you nothing to do. |
| `travel`: share of time travelling | <= 40% | > 55% | Some travel is exploring; most of the hour should not be walking. |
| `stuck`: stuck events per bot-hour | <= 2 | > 6 | Terrain that traps players. The report lists where (heatmap). |

## Combat

| Check | Pass | Fail | Why |
|---|---|---|---|
| `win_rate`: fights won | >= 90% | < 75% | Quest mobs at your level should be beatable. |
| `fight_length`: seconds per won fight (median) | <= 25 | > 40 | Long fights against trash mobs drag. |
| `deaths`: deaths per hour | <= 1.5 | > 4 | Death should be a warning, not a routine. |
| `variety`: distinct abilities used per fighting bot | >= 2 | < 1.5 | A class with one button is not a class. |
| `one_button`: share of casts that are the most used ability | <= 70% | > 85% | Same. |
| `kill_credit`: kills that went to someone else's last hit | <= 5% | > 15% | Losing a kill you fought for to a passer-by feels unfair. |

## Economy and health

| Check | Pass | Fail | Why |
|---|---|---|---|
| `economy`: copper earned per copper spent | >= 1.5 | < 1 | New players should be able to afford food and repairs. |
| `latency`: action round trip p95 (talk, start attack) | <= 150 ms | > 400 ms | The server must feel instant. |
| `disconnects`: unexpected disconnects | 0 | > 1 | |
| `rubberband`: moves the server refused per bot-hour | <= 0.5 | > 2 | Bots walk with the real client physics, so every rubber-band (an unasked-for `Teleport`) is a movement-check false positive. The report lists each one with where it happened. |

## What else the report lists

- **Quests**: how many bots accepted and finished each quest, and the median
  minutes. A quest many accept and none finish is a dead end.
- **Blockers**: objectives a bot could not progress, and why (no camp, needs
  a profession skill, a tool, night time, creature far above level).
- **Deaths by cause**, **unreachable NPCs**, and a **stuck heatmap** in
  16-block cells (hearth-relative x, z) with examples.
- **Level curve**: median level and mean XP per minute of play.
- Breakdowns **by brain** (personality) and **by class**.

## Caveats

- Bots are better than new players at some things (they know where every
  camp is and never read quest text) and worse at others (simple pathing,
  no tactics beyond a rotation). Pace numbers are an optimistic bound for
  questing and a pessimistic one for anything that needs judgement.
- Many bots in one zone compete for the same camps. That is realistic for a
  launch day, but it lowers the pace; compare runs with the same bot count.
- The action round trip is measured at the bots' 50 ms tick, so it cannot
  read below about 50 ms.
- A 10-minute run never sees night (a day is about 16 minutes: 10.8 day and 5
  night), so night-only quests show up as blockers unless the run is longer
  or the time is set with an operator command (label such runs).
