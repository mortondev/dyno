# The first 30 minutes, round 2

24 November 2026. Sections 3 to 5 and 12 of
[the quality bar](../design/quality-bar.md). Round 1 is
[first-30-minutes-2026-11-23.md](first-30-minutes-2026-11-23.md) and
[fun-review-2026-11-23.md](fun-review-2026-11-23.md); this round measures
what it left, attributes every death, and changes what the tables asked for.

## The goals, and where each stands

| | Goal | Result |
|---|---|---|
| 1 | A like-for-like baseline on main: one bot build, two 12-bot 30-minute mixed swarms, tick p95 under 50 ms, medians per player type and class | **Pass.** Bot build `2b98591`, runs A and B on main `60a0d69`, tick p95 1.00 and 1.25 ms at load 6.6 and 8.3 on 16 cores. Tables below |
| 2 | Every death attributed; completionist, casual, explorer and social under 1.5 deaths an hour each; no night deaths at levels 1 to 4 on roads or in quest areas | **Attributed: pass. Rates: TBD2** |
| 3 | Completionist, casual and social travel 50% of their time or less; the five longest runs fixed by moving content | **TBD3** |
| 4 | Longest reward gap: median under 2:00, worst under 3:30; every gap over 2:00 listed with its cause | **TBD4** |
| 5 | Old Tusk falls to three and not to one level 4; all five cart drop-offs ridden; two 30-minute hand runs on the final build | **Pass**, after changing him: on main one geared level 4 warrior or priest killed him alone. Now 0 of 9 solo attempts, a three in 10 s, a pair in 17 s. Five drop-offs ridden. Two hand runs (mage, rogue) on the merged build, no deaths |
| 6 | Shared tags: a crowd at one camp no longer starves | **Pass.** Kills lost to another player's tag, 12 bots on the gate flocks: 26% to 0%. Meat for the Pot done at 2:21, was 3:39 |
| 7 | Enough quest content to reach level 5 by 30 minutes without grinding | **The content is there; the bots do not get through it in time.** TBD7 |

Also asked for during the round: a new character wakes facing the hearth
and the first quest giver with his "!" in view. **Done**, on top of world
round 3's wake point ([picture](runs/r2-wake-view.jpg)).

## How it was measured

- **One bot build for every counted swarm:** `2b98591` (`crates/dyno-bot/src`
  is unchanged from there to the last counted run). Against round 1's bots
  it adds measurement (each fight's start with health and what stood near,
  each death's surroundings, unbroken stretches of travel, a 5-second trail)
  and two behaviours, both in the baseline too: a bot takes a cart to a hub
  it has walked to when that saves 200 m, and at night looks for Shades in
  a place the rules mark as the dark's (there are none on main, so there it
  does what it did).
- **Before:** two swarms, `dyno bots --local --port 7710 --count 12 --minutes 30`,
  on main's rules (`60a0d69`).
- **After, first pass:** one swarm on the first set of changes (`0ef9486`).
  It showed two things that swamped everything else (below), so it is
  reported but not pooled.
- **After:** two swarms on the final rules of this branch as they stood
  before merging main (the same engine and classes as the baseline), with
  one difference from what ships: Old Tusk's kind is marked level 8 in the
  rules file the server was given, so the bots, which skip creatures four
  levels above them and cannot read "(Group)", leave him alone. His camp
  still sets his real level; players see no difference. Why: next section.
- **Tick and load** are printed beside each table. A `--local` swarm now
  records the server's tick cost itself (mean, p95, p99, worst, and the
  worst minute's p95) and `/proc/loadavg` each minute; for a swarm against a
  separate server, `DYNO_TICK_STATS=1` logs the p95 each minute and
  `tools/playtest/attach_tick.py` puts it in the report.
- **Analysis:** `tools/playtest/round2.py --rules RULES REPORT...`. The bots
  record facts; the script decides what they mean, with the rules the run
  was made on. Definitions: *on a road* is within 10 m of a road or trail
  of the layout; *in a quest area* is within 45 m of a camp of a creature
  that a quest of level 5 or under asks for, in an explore objective's
  circle, by a quest ore place, or in a safe hub; *engaged* is every
  creature within 6 m at the death that is hostile or that the bot had hit.
  A death's cause is the first that fits of: *over-level* (the killer two or
  more levels above), *night* (a Shade), *adds* (two or more engaged),
  *jumped* (attacked while under 60% health), *low pull* (the bot chose to
  pull under 60% health), *one-on-one*.
- **Small samples.** A 12-bot mix has one casual, one explorer and one
  social bot, and the casual one plays 5 to 15 minutes. One death there
  reads as 2 to 6 an hour. Counts and bot-hours are shown; rates are pooled
  over both runs.
- **Hand play** through `dyno play`, driven command by command.

## What the first pass taught

The first set of changes made Old Tusk a real elite (he could be soloed on
main) and fixed a rule that had hidden him: a creature could not come back
within 12 m of a player, so one bot waiting in his hollow kept him from
ever returning. With both fixed, he was always there and never soloable,
and the bots, which take every quest and cannot read "(Group)", walked into
him alone one after another: 20 of that run's 49 deaths, and 18 bot-minutes
of walking back from the hearth to die again, the largest single use of
travel time. The second thing: Thieves in the Fields counts any Ashcloak,
and from the hearth the level 5 scouts on the East Road are nearer than the
new camp in Ostrin's field, so bots went there and died (5 deaths).

| First pass, one run (12 bots, 30 min; tick p95 1.00 ms, load 7.5) | completionist | social | explorer |
|---|---|---|---|
| Deaths | 23, of which 15 to Old Tusk alone and 5 at Ashcloak camps | 4, of which 2 to Old Tusk | 1, to Old Tusk |
| Travel | 71% | 65% | 81% |

The thieves were fixed in the rules (they are their own creature now, in
Ostrin's east field only). Old Tusk is right for players and wrong only for
bots that cannot read, so for the counted after runs the bots are kept off
him and he is tested on his own (below). The bot should learn to leave
group quests alone unless it is in a party; that is a change to the bot
build, so it is for the next round.

TBDBODY
