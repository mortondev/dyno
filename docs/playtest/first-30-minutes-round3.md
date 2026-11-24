# The first 30 minutes, round 3

24 November 2026. Sections 3 to 5 and 12 of
[the quality bar](../design/quality-bar.md). Round 2 is
[first-30-minutes-round2.md](first-30-minutes-round2.md). This round
teaches the bots what round 2 found them doing wrong, freezes them, and
measures again. It then moves content so a walk to a talk quest has
something on the way, and adds a way to find a second player for Old Tusk.

## The goals, and where each stands

| | Goal | Result |
|---|---|---|
| 1 | Teach the bots (rest to 80% health and mana before pulling; leave group quests and elites alone unless grouped to size; judge camps by what stands there; no gear-swap loop, with a test), then freeze them | **Pass.** Bot build `7a4b4de`, the same binary for all four counted runs. Chosen pulls under 80% health in the 8-minute check: 0 of 161. Grinder deaths: 10.7 an hour (round 2) to 0. Test `a_two_handed_weapon_and_a_shield_do_not_swap_for_ever` (fails on the old code). One bot rule was added after the counted runs and is not in any table (section 3) |
| 2 | Two 12-bot 30-minute mixed swarms before and two after, tick p95 under 50 ms, medians per type and class | **Pass.** Tick p95 1.00 to 1.25 ms in all four. Tables in section 2. Before: v0.11.3's rules + the `group` tag; after: the same + this round's content |
| 3 | Deaths under 1.5 an hour for completionist, casual, explorer and social; every death attributed; my causes fixed | **Pass (small samples).** Before / after: completionist 0.8 / 0.8, casual 0 / 0 (0.2 and 0.3 bot-hours), explorer 0 / 0, social 1.0 / 0. All 25 deaths attributed. No deaths to adds, night, respawns or low pulls. Fixed: Silkmother, the Warden's distance, and (after the runs) Know the Vale's Shrine Shore. Not mine: the party pair died 18 times to Old Tusk at levels 2 and 3, a bot fault, fixed after the freeze |
| 4 | Travel 50% or less for completionist, casual and social; talk-quest walks carry something; five longest runs before and after | **Fail.** Completionist 69% → 66%, social 61% → 58%, casual 28% → 53% (it plays about 10 minutes). Five talk quests now have a step on the way, and two quests are handed out before the walk they belong to. The Garrick talk walk went from 11.2 bot-minutes to under 4.2, but a new hand-in walk to Garrick (12.8) ate it. The five longest runs, before and after, are none of them talk walks (section 4) |
| 5 | Questing bots: median longest gap under 2:00, worst under 3:30; every gap over 2:00 listed | **Pass, narrowly.** 2:24 → 1:59, worst 3:22 → 2:51. All 26 listed with cause (section 5) |
| 6 | A light, data-driven way to find a group for Old Tusk, proven by two bots or two `dyno play` sessions | **Pass.** `Quest::group` in the rules; `ServerMsg::Seekers` once a second to carriers (protocol 26, appended); a Looking for group box with Invite and Whisper, and green dots on the map; the LFG line in `dyno play`. Proof: a bot test (two strangers group and kill him), two `dyno play` sessions (whisper, invite, kill in 16 s), and 6 pairs formed in the swarms, 5 of which killed him. The client box was not looked at in a running client |
| 7 | One 30-minute hand run as a class round 2 did not play | **Done:** a hunter, level 4 at 29:04, 13 quests, 2 deaths (section 7) |
| 8 | Gates: fmt, clippy with warnings as errors, the workspace's tests once, the client check | **Pass**, on the tree merged with main (`10b49c2`, protocol 26, with the camera lane's merge): `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo test --workspace` 292 passed and 0 failed, `check-client.sh` 9.25 MB with no content strings. An earlier full run, before that merge, lost the LFG test once: on a loaded machine, a few of its staging `/finish` lines landed late and the bots went off to other quests. The test now repeats any that are missing |

## How it was measured

- **One bot build for every counted swarm:** `7a4b4de`. The binary was
  copied before the first counted run and every counted swarm ran on that
  copy. Since then `crates/dyno-bot/src` changed twice: `cargo fmt`
  (whitespace), and one rule added after the last counted run (a bot leaves
  a group's creature alone below the quest's level; section 2). Neither is
  in any table below.
- **Rules, like for like.** Every counted run uses the rules of v0.11.3
  (`bdad493`), not main's later rules (main has since merged feel F2's
  creature changes), so before and after differ only by this round's
  changes:
  - *before*: v0.11.3's rules plus one tag, `group: 2` on Old Tusk's quest
    and `group: 3` on the Barrow Warden's
    ([runs/r3-before-rules.json.gz](runs/r3-before-rules.json.gz)).
    Without the tag the bots cannot tell a group quest, and round 2's first
    pass showed what follows: 15 of 23 completionist deaths to Old Tusk;
  - *after*: the same plus the `content` stage of
    `tools/playtest/round3_rules.py`
    ([runs/r3-after-rules.json.gz](runs/r3-after-rules.json.gz)).

  The hand-in's rules are main's (with F2) plus the `group`, `content` and
  `vale` stages. `vale` came after the counted runs (section 2).
- **Runs:** `dyno bots --local --port 7710 --count 12 --minutes 30
  --content RULES.json`, one at a time. `--content` is new this round: a
  `--local` swarm can run on a rules file other than the built-in one.
- **Analysis:** `tools/playtest/round2.py --rules RULES.json REPORT...`
  for round 2's tables, with round 2's definitions unchanged, and
  `tools/playtest/round3.py` for the questing bots' gaps, their longest
  runs and looking for group. Per type and per class:
  `tools/playtest/compare.py`.
- **Small samples.** A 12-bot mix has one casual, one explorer and one
  social bot per run. Pooled over two runs that is 0.2 casual bot-hours
  and one explorer and one social bot-hour, so one death reads as 5, 1
  and 1 an hour. Counts and hours are shown.
- **"Resource"** in "rest to 80% health and resource" is mana. Rage starts
  each fight empty and energy refills in seconds, so only mana is waited
  for.

| | run | report | load: start / mean / max (16 cores) | server tick: mean / p95 / p99 / worst | worst minute's p95 | counts (p95 < 50 ms) |
|---|---|---|---|---|---|---|
| before | A | r3-before-a-mix-12x30.json.gz | 8.0 / 8.4 / 16.3 | 0.44 / 1.25 / 2.25 / 41 ms | 1.75 ms | yes |
| before | B | r3-before-b-mix-12x30.json.gz | 17.1 / 8.2 / 22.0 | 0.39 / 1.00 / 2.00 / 52 ms | 1.50 ms | yes |
| after | A | r3-after-a-mix-12x30.json.gz | 6.3 / 7.1 / 14.7 | 0.43 / 1.25 / 2.25 / 20 ms | 1.75 ms | yes |
| after | B | r3-after-b-mix-12x30.json.gz | 11.4 / 5.9 / 11.4 | 0.38 / 1.00 / 1.75 / 19 ms | 1.25 ms | yes |

## 1. The bots, taught and frozen

| What round 2 found | What the bot does now | Where |
|---|---|---|
| It rested only out of combat. The combat flag outlasts a kill by a few seconds, so it pulled the next creature first: 28 of the grinders' 35 deaths | Before choosing anything, it rests if health or mana is under 80%, unless something hit it in the last 2 s. It rests to 85% (at 95% the grinders spent half their time resting at level 2) | `brain.rs` `ready_to_pull`, `choose` step 2; the social bot's assist checks it too |
| It took every quest and walked into Old Tusk alone | A creature that a group quest asks for (`Quest::group`, new in the rules) is left alone unless at least that many of its party stand within 40 m. Its camp counts as dangerous, so the bot does not path through it. It still takes the quest: carrying it is what tells the server it is looking | `brain.rs` `needs_group`, `live_target`, `hunt`, `dangerous` |
| It chose camps by the creature kind's level range. The server spawns a camp at the camp's band, else the area's, so a level 4 grinder walked between flocks of level 1 Grazers | Camps are judged by `camp.levels` or the area's levels, as the server spawns them. A camp whose band starts above our level + 2 is skipped, one that tops out above our level + 1 costs 400 m, and a grinder skips camps whose top is below our level − 1. Explorers visit zones whose band starts at our level + 1 at most (was + 2: an explorer died in the Ashcloak Stockade in round 2) | `brain.rs` `camp_levels`, `hunt`, `grind`, `explore`, `dangerous` |
| A warrior with the Watch Greataxe and a buckler swapped them for ever | The bot scored each item in the place it chose, with the other hand untouched. The server puts an item in the first empty matching place, else the first, takes the off hand off for a two-handed weapon, and takes a two-handed weapon off for an off-hand item. Both now use one function, `dyno_sim::character::equip_place`. The same fault could loop on rings (two places) | `character.rs` `equip_place`; `gear.rs` `equip`; `body.rs` `upgrade_place` |

Test: `crates/dyno-bot/tests/gear_and_groups.rs`,
`a_two_handed_weapon_and_a_shield_do_not_swap_for_ever`. A level 4 warrior
is given both items and wears upgrades as the brain does for 8 s; at most
three things go on, and then nothing. On the old `upgrade_place` it fails:
after 8 s the buckler is on and the axe is in the bags. Its waits scale by
`DYNO_TEST_TIME_SCALE`.

Checked before freezing: one 8-minute 12-bot swarm. 0 of 161 chosen pulls
were under 80% health, no deaths, no equip loops.

What the freeze kept in: the party brain's stalls (one of the pair idles
for minutes in town; section 4), and a rule I added only after the counted
runs (section 2).

## 2. Like for like: the frozen bots, before and after

Two 12-bot 30-minute mixed swarms on each rules file, all four on the
frozen build. Medians over the bots of both runs. Deaths an hour are
pooled over the hours played, and time shares are of the time played.
Each run's report and analysis is in `runs/`: `r3-before-*`,
`r3-after-*`, `r3-before-analysis.md`, `r3-after-analysis.md`,
`r3-before-round3.md` and `r3-after-round3.md`.

### Per player type

| brain | | bots | level 2 | level 3 | level 4 | level at end | reached 5 | quests done | XP earned | longest gap (median) | longest gap (worst) | travel | fighting | waiting for spawns | deaths | deaths an hour | kills lost to a tag |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| completionist | before | 8 | 2:11 | 7:55 | 14:57 | 5 | 6 of 8 | 19 | 4964 | 2:27 | 3:22 | 69% | 17% | 2% | 3 in 4.00 h | 0.8 | 0% |
|  | after | 8 | 2:03 | 7:48 | 15:13 | 5 | 7 of 8 | 19.5 | 5544 | 2:08 | 2:51 | 66% | 19% | 2% | 3 in 4.00 h | 0.8 | 0% |
| casual | before | 2 | 4:13 | - | - | 2 | 0 of 2 | 4.5 | 885 | 0:48 | 0:52 | 28% | 33% | 0% | 0 in 0.20 h | 0.0 | 0% |
|  | after | 2 | 3:37 | - | - | 2.5 | 0 of 2 | 8.5 | 1549 | 1:12 | 1:28 | 53% | 20% | 1% | 0 in 0.31 h | 0.0 | 0% |
| explorer | before | 2 | 4:57 | 11:59 | 24:55 | 4 | 0 of 2 | 11.5 | 2914 | 2:15 | 2:34 | 75% | 7% | 0% | 0 in 1.00 h | 0.0 | 0% |
|  | after | 2 | 3:44 | 12:38 | - | 3.5 | 0 of 2 | 9.5 | 2634 | 2:38 | 2:42 | 78% | 7% | 0% | 0 in 1.00 h | 0.0 | 0% |
| social | before | 2 | 2:13 | 8:53 | 17:22 | 4 | 0 of 2 | 17 | 4196 | 2:06 | 2:24 | 61% | 16% | 1% | 1 in 1.00 h | 1.0 | 2% |
|  | after | 2 | 2:51 | 9:12 | 17:14 | 4.5 | 1 of 2 | 18 | 4994 | 1:32 | 1:38 | 58% | 19% | 0% | 0 in 1.00 h | 0.0 | 0% |
| party | before | 4 | 3:15 | 12:42 | - | 3 | 0 of 4 | 11 | 2328 | 6:29 | 10:46 | 53% | 5% | 0% | 0 in 2.00 h | 0.0 | 0% |
|  | after | 4 | 3:10 | 12:12 | - | 3 | 0 of 4 | 9.5 | 2206 | 8:37 | 9:55 | 49% | 10% | 0% | 18 in 2.00 h | 9.0 | 17% |
| grinder | before | 6 | 3:12 | 9:30 | 21:17 | 4 | 0 of 6 | 2 | 3478 | 0:44 | 0:51 | 3% | 38% | 0% | 0 in 3.00 h | 0.0 | 0% |
|  | after | 6 | 3:00 | 9:45 | 21:39 | 4 | 0 of 6 | 2 | 3504 | 0:49 | 1:06 | 4% | 37% | 0% | 0 in 3.00 h | 0.0 | 0% |
| all | before | 24 | 2:52 | 9:18 | 20:16 | 4 | 6 of 24 | 11 | 3478 | 2:20 | 10:46 | 48% | 20% | 1% | 4 in 11.20 h | 0.4 | 0% |
|  | after | 24 | 2:53 | 9:42 | 17:55 | 4 | 8 of 24 | 11.5 | 3540 | 1:45 | 9:55 | 47% | 21% | 1% | 21 in 11.30 h | 1.9 | 1% |

### Per class

| class | | bots | level 2 | level 3 | level 4 | level at end | reached 5 | quests done | XP earned | longest gap (median) | longest gap (worst) | travel | fighting | waiting for spawns | deaths | deaths an hour | kills lost to a tag |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Hunter | before | 4 | 4:17 | 11:42 | 25:35 | 4 | 0 of 4 | 11.5 | 2834 | 2:47 | 3:40 | 64% | 8% | 0% | 0 in 2.00 h | 0.0 | 0% |
|  | after | 4 | 3:02 | 12:25 | - | 3.5 | 0 of 4 | 11.5 | 2641 | 4:36 | 8:46 | 63% | 10% | 0% | 9 in 2.00 h | 4.5 | 3% |
| Mage | before | 4 | 4:11 | - | - | 3 | 0 of 4 | 3 | 2266 | 0:47 | 0:52 | 8% | 33% | 0% | 0 in 1.20 h | 0.0 | 0% |
|  | after | 4 | 3:37 | 9:46 | - | 3.5 | 0 of 4 | 4 | 2748 | 1:01 | 1:28 | 16% | 28% | 0% | 0 in 1.31 h | 0.0 | 0% |
| Paladin | before | 4 | 2:59 | 9:06 | 21:02 | 4 | 0 of 4 | 2 | 3488 | 0:44 | 0:46 | 3% | 40% | 0% | 0 in 2.00 h | 0.0 | 0% |
|  | after | 4 | 2:55 | 9:42 | 21:36 | 4 | 0 of 4 | 2 | 3540 | 0:49 | 0:55 | 3% | 41% | 0% | 0 in 2.00 h | 0.0 | 0% |
| Priest | before | 4 | 2:33 | 8:53 | 17:22 | 4 | 1 of 4 | 17 | 4196 | 2:20 | 3:07 | 62% | 16% | 1% | 3 in 2.00 h | 1.5 | 2% |
|  | after | 4 | 2:51 | 9:37 | 17:19 | 4.5 | 2 of 4 | 18 | 4994 | 1:34 | 2:15 | 60% | 17% | 0% | 1 in 2.00 h | 0.5 | 0% |
| Rogue | before | 4 | 2:38 | 11:19 | - | 4 | 2 of 4 | 14.5 | 3500 | 6:20 | 10:46 | 64% | 8% | 0% | 1 in 2.00 h | 0.5 | 0% |
|  | after | 4 | 2:52 | 7:49 | - | 4 | 2 of 4 | 13.5 | 3580 | 5:38 | 9:55 | 59% | 15% | 1% | 10 in 2.00 h | 5.0 | 10% |
| Warrior | before | 4 | 1:55 | 7:44 | 15:13 | 5 | 3 of 4 | 19 | 5270 | 2:25 | 2:28 | 69% | 19% | 2% | 0 in 2.00 h | 0.0 | 0% |
|  | after | 4 | 2:00 | 7:16 | 14:17 | 5 | 4 of 4 | 19.5 | 5544 | 1:59 | 2:51 | 68% | 20% | 2% | 1 in 2.00 h | 0.5 | 0% |
| all | before | 24 | 2:52 | 9:18 | 20:16 | 4 | 6 of 24 | 11 | 3478 | 2:20 | 10:46 | 48% | 20% | 1% | 4 in 11.20 h | 0.4 | 0% |
|  | after | 24 | 2:53 | 9:42 | 17:55 | 4 | 8 of 24 | 11.5 | 3540 | 1:45 | 9:55 | 47% | 21% | 1% | 21 in 11.30 h | 1.9 | 1% |

Bots 1, 7 and 11 of every 12 are grinders (paladin, paladin, mage).
Paladin and mage rows are therefore the grinder brain, as in round 2.
Hunter and rogue are the party pair (bots 8 and 9) plus one completionist
or explorer each. Their after rows carry the party's 18 deaths to Old
Tusk (section 3).

### What the bot fixes did on their own

Round 2's last tables were taken on round 2's bots, so they are not
comparable with this round's in every column. In deaths the bots alone
moved the most:

| deaths an hour | round 2, after (round 2's bots, Old Tusk hidden) | round 3, before (frozen bots, v0.11.3 rules + `group`) |
|---|---|---|
| completionist | 1.8 | 0.8 |
| explorer | 4.0 | 0.0 |
| social | 2.0 | 1.0 |
| grinder | 10.7 | 0.0 |
| chosen pulls under 60% health, grinders | 22% | 0% |

The grinders' deaths (10.7 an hour in round 2) were the brain, as round 2 said. Resting
before pulling took them to none, at the price of half their time spent
resting (51%).


## 3. Deaths

Goal: completionist, casual, explorer and social each under 1.5 an hour.

| brain | before: deaths in bot-hours | an hour | after: deaths in bot-hours | an hour |
|---|---|---|---|---|
| completionist | 3 in 4.00 | 0.8 | 3 in 4.00 | 0.8 |
| casual | 0 in 0.20 | 0.0 | 0 in 0.31 | 0.0 |
| explorer | 0 in 1.00 | 0.0 | 0 in 1.00 | 0.0 |
| social | 1 in 1.00 | 1.0 | 0 in 1.00 | 0.0 |
| party (not in the goal) | 0 in 2.00 | 0.0 | 18 in 2.00 | 9.0 |
| grinder (not in the goal) | 0 in 3.00 | 0.0 | 0 in 3.00 | 0.0 |

Every death, with round 2's definitions (killer two or more levels above:
over-level; a Shade: night; two or more engaged: adds; attacked under 60%
health: jumped; chose to pull under 60%: low pull; else one-on-one).
"Health at pull" is at the start of the fight that killed.

### Before (frozen bots, v0.11.3 rules + `group`)

| run | bot | brain | class | at | lv | killer (gap) | engaged | day/night | health at pull | pulled | where | cause | whose, and what was done |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| A | 04soc | social | Priest | 29:12 | 4 | Silkmother, Websnare 5 (+1) | Silkmother | night | 68% | attacked | The Hollow Oak | one-on-one (a rare elite a level up) | Mine. Silkmother, the Oak's rare, stood a level above the level 4s that Rysa sends there and hit for 2.5 times a Websnare; she killed two in round 2 as well. Now at her camp's level, 2.0 times |
| B | 10com | completionist | Priest | 18:51 | 3 | Old Tusk 4 (+1) | Old Tusk | day | 100% | helping its LFG leader | Old Tusk's hollow | one-on-one | The bot: a level 3 priest grouped with a social priest through looking for group, and the pair lost |
| B | 10com | completionist | Priest | 20:53 | 3 | Old Tusk 4 (+1) | Old Tusk | day | 100% | Old Tusk | Old Tusk's hollow | one-on-one | The same pair again. The bot's group rule counted heads, not levels |
| B | 03com | completionist | Rogue | 28:23 | 4 | Barrow Warden 6 (+2) | Barrow Warden | night | 19% | attacked | The Old Barrow | over-level | Mine. Into Barrowwood asked you to stand 20 m from a level 6 elite. Now 37 m, at the barrow's far edge |

No deaths at all to adds, the night, respawns or low pulls.

### After (frozen bots, after rules)

| run | bot | brain | class | at | lv | killer (gap) | engaged | day/night | health at pull | pulled | where | cause | whose, and what was done |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| A | 03com | completionist | Rogue | 26:06 | 5 | Gloomwisp 7 (+2) | Gloomwisp | night | 87% | attacked | the Shrine Shore | over-level | Mine. Know the Vale (level 5) sends you to stand at the Shrine Shore, which is a camp of level 7 to 8 Gloomwisps |
| B | 00com | completionist | Warrior | 25:31 | 5 | Gloomwisp 8 (+3) | Gloomwisp | day | 87% | attacked | the Shrine Shore | over-level | The same |
| B | 10com | completionist | Priest | 25:35 | 4 | Gloomwisp 8 (+4) | none left | night | 34% | attacked | the Shrine Shore | over-level | The same |
| A, B | 08par, 09par | party | Hunter, Rogue | 19:58 to 29:35 | 2 to 3 | Old Tusk 4 (+1, +2) | Old Tusk | day and night | 100% | Old Tusk, and helping | Old Tusk's hollow | one-on-one / over-level | The bot: 18 deaths, 9 trips of the pair, at levels 2 and 3. The leader's party of two met the head count, and it walked back after each wipe. Before, the pair never got that far; now Tusks and Tempers comes from Holt, so it reached Old Tusk's quest at level 3 (the engine offers a quest at its level − 1) |

What was done after the counted runs (in the hand-in, in no table above):

- **Rules, `vale` stage.** Know the Vale stands at Emberrun Ford (levels
  4 to 5) instead of the Shrine Shore. All three questing deaths of the
  after runs were there.
- **Bot.** A group's creature is also left alone below the group quest's
  level (`needs_group`). This is a bot change after the freeze. I made it
  because the party pair's loop at Old Tusk is a fault in the frozen
  build, not in the game: two level 2 and 3 characters lost to a level 4
  elite every time they tried. It is not measured by a swarm.

So: **completionist 0.8, casual 0, explorer 0, social 0 deaths an hour
after; 0.8, 0, 0, 1.0 before. Pass, with the sample caveat**: 0.2 to 0.3
casual bot-hours and 1 explorer and 1 social bot-hour over two runs.

- Not one death in the 48 bot-runs was to adds, to a Shade at night, to a
  creature come back on top of someone, or to a pull under 60%. These were
  the causes round 2 named as the goal's.
- What is left are places that sent a level 4 or 5 to stand beside a
  level 6 to 8 creature (both fixed), and the bots' Old Tusk pairs.


## 4. Travel

Goal: completionist, casual and social travel 50% of their time or less.

| brain | before | after |
|---|---|---|
| completionist (8) | 69% | 66% |
| casual (2) | 28% | 53% |
| social (2) | 61% | 58% |
| explorer (2, not in the goal) | 75% | 78% |

**Fail** for completionists and social. Casual is under the line before
and over it after, which says little: it plays 5 to 15 minutes, about 10
minutes a run. Before this round's bot changes (round 2's last tables) it
was completionist 66%, casual 60% and social 74%.

### What changed on the walks that were for a talk

Bot-minutes of travel, all bots but grinders, by what the bot was on its
way to (`round2.py`, "Travel by purpose"), both runs pooled:

| on the way to | before | after | |
|---|---|---|---|
| Garrick's Camp: talk to Garrick | 11.2 | under 4.2 | Now the Woodcutters' Well first (5.3), a discovery and a quest step 70 m out |
| Tusks and Tempers: boars for tusks | 5.8 | 4.8 | Taken from Holt; the boars at Holt and on the trail drop them |
| Visiting Garrick to hand in Tusks and Tempers | (part of the visit for Garrick's Camp) | 12.8 | New, and it eats what was saved. A bot that takes both quests at Holt finishes Garrick's Camp first and walks back with the tusks later |
| Word for the Hunter: talk to Rysa | 9.4 | 6.4 | Hunter's Rest on the way |
| Bread for the Mill: talk to Ostrin | (the two longest runs of the baseline, 7:44 and 7:51) | the Listening Stones 4.3, Ostrin under 4.2 | No run to Ostrin is in the after runs' longest twelve |

The arithmetic, done before choosing the content:
the walk to Garrick was 11 of about 300 bot-minutes of questing travel.
Fixing every talk walk named in round 2 could not take a completionist
from 69% to 50%. The travel is in the shape of the zone: two hubs 255 m
apart, the Broken Tower 225 m south of the gate that hands it out (10.3
and 10.9 bot-minutes on Tolan's hand-ins, before and after, untouched),
the Lay of the Land round a wall with no west gate, and a third hub
(Garrick) 285 m past the second. Moving whole hubs is the world lane's and
a layout change; I did not.

### The five longest runs of travel, questing bots

Before:

| | run | bot | at | length | from | to | for |
|---|---|---|---|---|---|---|---|
| 1 | A | 02exp | 18:29 | 7:43 | Beacon Rocks | 35 m from ford_trail_ruin_20 | defending against Ashcloak Thief |
| 2 | B | 03com | 16:15 | 6:16 | Beacon Rocks | oak_webs | webs_between_roots: kill Websnare |
| 3 | B | 02exp | 10:02 | 6:07 | 60 m from North Gate Rocks | 86 m from east_road_glade_6 | boar_trouble: kill Thornboar |
| 4 | A | 02exp | 1:10 | 5:39 | 35 m from North Gate Rocks | meadow_flock_5 | thin_the_herd: kill Grazer |
| 5 | A | 06com | 15:14 | 5:29 | Beacon Rocks | birchrise_boars_2 | tusks_and_tempers: kill Thornboar for Boar Tusk |

After:

| | run | bot | at | length | from | to | for |
|---|---|---|---|---|---|---|---|
| 1 | A | 00com | 19:02 | 9:05 | Hearthstead | The Old Barrow | eyes_in_the_dark: wait for a Shade to rise |
| 2 | B | 02exp | 9:04 | 7:47 | 99 m from Hearthstead | 64 m from north_road_glade_1 | defending against Thornboar |
| 3 | B | 10com | 4:22 | 6:31 | 108 m from Hearthstead | 45 m from west_trail_camp_11 | boar_trouble: kill Thornboar |
| 4 | B | 03com | 22:27 | 5:44 | barrow_webs | The Barrowwood Graves | dust_of_shadows: kill Shade for Shade dust |
| 5 | A | 06com | 5:12 | 5:26 | 57 m from east_road_wood_5 | Beacon Rocks | watchfire: gather Coal |

- None of the five is a walk for a talk quest, before or after. They are
  bots crossing between camps: Boar Trouble from town, coal from
  Hearthstead to Beacon Rocks, a Shade hunt.
- After run A's 9:05 is a completionist that walked from Hearthstead to
  the Old Barrow at night to wait for a Shade for Eyes in the Dark. The
  bot picks the nearest dark area at or under its level + 1, and at level
  5 that is the barrow, not the Dimwood. Mine to point at: Eyes in the
  Dark is Holt's and names the Dimwood, but a kill objective cannot say
  where (round 2's "kill quests with a place" is still open).


## 5. Reward gaps

Goal: among the questing bots (completionist, casual, explorer, social),
the median longest gap is under 2:00 and the worst under 3:30.

| | before | after |
|---|---|---|
| median of each questing bot's longest gap (14 bots) | 2:24 | **1:59** |
| worst | 3:22 | **2:51** |
| gaps over 2:00 | 14 | 12 |
| all 24 bots: median / worst | 2:20 / 10:46 | 1:45 / 9:55 |

**Pass, narrowly**: the median is a second under. Round 2's questing bots
were at 2:04 and 15:48 (3:12 without the gear loop).

### Every questing gap over 2:00, before

| run | bot | brain | from | gap | where: from, to | doing |
|---|---|---|---|---|---|---|
| B | 03com | completionist | 25:56 | 3:22 | The Logging Camp → 38 m from south_road_ruin_8 | visiting Anwen: accept forgotten_graves; forgotten_graves: talk to Brother Calder; dust_of_shadows: kill Shade for Shade dust |
| A | 10com | completionist | 14:44 | 3:07 | The North Watch → 80 m from meadow_flock_4 | recalling to town; following a party member; helping a party member |
| A | 03com | completionist | 23:44 | 2:49 | The North Pass → oak_trail_view_19 | know_the_vale: explore the North Pass; know_the_vale: explore the Barrow Overlook |
| A | 02exp | explorer | 19:42 | 2:34 | 61 m from meadow_flock_5 → The Toll Hut | exploring The East Road; boar_trouble: go to the Thornboar camp; exploring Emberford Bridge |
| A | 06com | completionist | 7:39 | 2:28 | Beacon Rocks → North Gate Rocks | visiting Tilda: accept windmill_breadcrumb; boar_trouble: go to the Thornboar camp; coal_for_the_watch: gather Coal |
| B | 06com | completionist | 15:17 | 2:26 | The North Watch → The Logging Camp | visiting Old Brannoc: accept know_the_vale; garricks_camp: talk to Foreman Garrick |
| A | 04soc | social | 18:31 | 2:24 | The North Watch → 33 m from The Logging Camp | visiting Old Brannoc: accept know_the_vale; garricks_camp: talk to Foreman Garrick |
| B | 00com | completionist | 15:09 | 2:23 | The North Watch → The Logging Camp | visiting Old Brannoc: accept know_the_vale; garricks_camp: talk to Foreman Garrick |
| B | 10com | completionist | 25:25 | 2:17 | birchrise_boars_3 → birchrise_boars_3 | following a party member; recalling to town; defending against Thornboar |
| A | 04soc | social | 27:48 | 2:11 (to the end) | birchrise_boars_2 → 45 m from west_trail_camp_11 | resting; webs_between_roots: go to the Websnare camp; defending against Websnare |
| A | 04soc | social | 15:54 | 2:10 | 31 m from south_road_ruin_8 → 81 m from meadow_flock_4 | old_tusk: go to the Old Tusk camp; old_tusk: kill Old Tusk |
| A | 00com | completionist | 19:05 | 2:08 | The Barrowwood Graves → 45 m from south_road_stones_9 | resting; webs_between_roots: kill Websnare; silk_for_sails: kill Websnare for Websnare Silk |
| B | 03com | completionist | 23:47 | 2:08 | 36 m from The Old Barrow → The Logging Camp | into_barrowwood: explore the Old Barrow; visiting Foreman Garrick: hand in into_barrowwood |
| B | 10com | completionist | 18:20 | 2:06 | 82 m from meadow_flock_4 → 78 m from The Old Quarry | helping a party member; following a party member |

### Every questing gap over 2:00, after

| run | bot | brain | from | gap | where: from, to | doing |
|---|---|---|---|---|---|---|
| A | 00com | completionist | 20:24 | 2:51 | The North Pass → 41 m from oak_trail_view_19 | know_the_vale: explore the North Pass; know_the_vale: explore the Barrow Overlook |
| B | 03com | completionist | 25:03 | 2:48 | The North Pass → 43 m from oak_trail_view_19 | know_the_vale: explore the Barrow Overlook |
| B | 02exp | explorer | 22:50 | 2:42 | birchrise_boars_3 → 39 m from oak_webs | resting; exploring The Forgotten Graves |
| A | 02exp | explorer | 26:37 | 2:34 | Emberford Bridge → 64 m from The Ashcloak Stockade | exploring Emberford Bridge; boar_trouble: go to the Thornboar camp; exploring Emberrun Ford |
| A | 00com | completionist | 25:54 | 2:29 | The Logging Camp → 33 m from The Old Barrow | visiting Anwen: accept forgotten_graves; eyes_in_the_dark: wait for a Shade to rise; forgotten_graves: talk to Brother Calder |
| A | 02exp | explorer | 22:56 | 2:16 | west_trail_glade_13 → Hearthstead | exploring Bluebell Glade; boar_trouble: go to the Thornboar camp; broken_tower: explore the Broken Tower |
| A | 10com | completionist | 25:47 | 2:15 | barrow_boars → 37 m from The Logging Camp | resting; visiting Foreman Garrick: hand in tusks_and_tempers |
| A | 03com | completionist | 7:47 | 2:12 | west_trail_camp_11 → 56 m from North Gate Rocks | visiting Tilda: accept windmill_breadcrumb; visiting Hollis: accept iron_for_hollis; iron_for_hollis: gather Raw iron |
| B | 02exp | explorer | 25:46 | 2:10 | oak_trail_view_19 → 63 m from south_road_stones_9 | exploring Barrow Overlook; exploring The Listening Stones |
| B | 06com | completionist | 14:36 | 2:04 | 50 m from meadow_flock_3 → 78 m from The Old Quarry | helping a party member; following a party member |
| B | 03com | completionist | 15:05 | 2:02 | The North Watch → 88 m from meadow_flock_4 | visiting Old Brannoc: accept dust_of_shadows; old_tusk: go to the Old Tusk camp; old_tusk: kill Old Tusk |
| A | 00com | completionist | 23:52 | 2:02 | 42 m from The Old Barrow → The Logging Camp | visiting Foreman Garrick: hand in into_barrowwood |

By cause, after:

| cause | gaps | longest | whose |
|---|---|---|---|
| Know the Vale: the North Pass to the Barrow Overlook, a 600 m walk between two of its four places | 2 | 2:51 | mine; a level 5 quest that is four discoveries across the whole vale. Its places pay XP each, but the walk between the North Pass and the Overlook has nothing. Not changed |
| The explorer bot walking between named places | 4 | 2:42 | the brain: an explorer is travel by design |
| Garrick to Anwen and the Old Barrow and back (Into Barrowwood, the Forgotten Graves) | 3 | 2:29 | mine, not changed: Into Barrowwood is a 450 m walk out and back, and the place to stand is now 17 m further |
| From the Trapper's Camp to town and out for iron | 1 | 2:12 | the hub loop |
| Following a looking-for-group leader | 2 | 2:04 | the bot |

The longest gaps of all are the party brain's: the follower sits in town
"resting with the party" for 8 to 10 minutes. This was in round 2 too, it
is a stall of the party brain, and the freeze kept it.


## 6. Finding a group

**Data.** A quest says how many it is made for: `"group": 2` on Old
Tusk's, `3` on the Barrow Warden's. `Quest::group` defaults to 0 (alone).
Old Tusk's text asked for three, but a pair beats him (round 2: in 17 s,
nobody died), so he is made for two and the text says "do not go alone.
Find someone to go with you".

**Server.** Every second, each player who carries an unfinished group
quest is sent `ServerMsg::Seekers { quest, players }`: the others in that
shard who carry it, with name, level and position, leaving out their own
party (`rpg.rs` `tick_seekers`). It is one small message a second, and
only to those carrying such a quest. Nothing is kept between ticks.

**Wire.** `Seekers` and the `Seeker` struct are appended after main's
`Pong` and `Refused`, so they ride on protocol 26. v0.11.3 shipped on 25,
and 26 is on main and unreleased. No version bump: an older client never
sees the message, because the server it talks to is older too.

**Client** (each part in its own function):

- `hud.rs` `looking_for_group`: a small box at the lower right while you
  carry a group quest your party is too small for. It reads "Looking for
  group: Old Tusk (Group)", then one line per player who carries it ("Bo,
  level 4, 35 m"), each with **Invite** and **Whisper** buttons. Whisper
  sends "/w Bo Shall we take on Old Tusk (Group) together?". With nobody
  else carrying it, the box says so and that they will show here and on
  the map. The strings are in `content/ui/strings.json`
  (`social.lfg_*`).
- `hud.rs` `seekers` feeds `worldmap.rs`: each seeker is a green dot on the
  world map, named "(looking for group)".
- `dyno play` prints the same under the quest: `Looking for group (made
  for 2): Ana lv4 0m here (say /invite NAME or /w NAME ...)`.

**Bots.** `brain.rs` `lfg_tick`. A bot that carries a group quest and is
in no party looks at the `Seekers` list:

- if another carrier stands within 60 m, the one whose name sorts first
  invites, and the other accepts an invitation only from a seeker;
- once grouped, the invited one follows the leader and helps its fights;
  the leader puts the group objective first while the party is near;
- when the group quest is done, both leave the party.

Nobody waits for a partner: a meeting happens only when two pass close.

**Proof:**

- **Two bots.** `two_strangers_find_each_other_for_old_tusk` in
  `gear_and_groups.rs`. A warrior and a priest who never met, level 5,
  each walk to Holt and take the quest. Each is told of the other. Their
  brains group them and they kill Old Tusk together; both get the credit.
  The test takes 2 minutes.
- **Two `dyno play` sessions** on a server of the after rules
  ([Ana](runs/r3-lfg-ana.txt), [Bo](runs/r3-lfg-bo.txt)). Both were staged
  with operator commands to level 4 in starter gear, with Boar Trouble
  finished.
  - Both took Old Tusk. Each one's log read `Looking for group (made for
    2): Bo lv4 0m here` (and Ana for Bo).
  - Bo whispered, Ana invited, Bo accepted, and the line went away.
  - First attempt: they walked out one command apart. Ana arrived alone,
    he took her, then Bo. Both died.
  - Second attempt: they waited 23 m from the hollow and attacked
    together. He died in 16 s; Ana ended at 60 of 75 health, Bo took 21
    damage. Both credited.
- **In the swarms**, from the `Seekers` message alone, with no party
  brain: across the four counted runs, six pairs of strangers
  formed (two in each baseline run, one in each after run), each when one
  bot passed within 60 m of another who carried the quest. Five pairs
  killed Old Tusk: ten bots credited, handed in between 14:21 and 20:28.
  One pair, a level 3 priest following a level 4 priest, lost to him
  twice (section 3). Before this round, with round 2's bots, Old Tusk had
  to be hidden from the bots for a run to count.

Not verified: the client's box and map dots were built and pass clippy
and the client check (`hearthblock` 9.25 MB, no content strings), but I did
not look at them in a running client. A `dyno shot --ui` of the box, and a
look at where it sits beside the chat at 1080p, are for the UI lane.

## 7. What I played: Wrenna, a hunter, 30 minutes

A new character on a fresh server of the after rules (the counted runs'
rules, before `vale`), driven through `dyno play` command by command, no
operator commands. Transcript: [runs/r3-hand-wrenna.txt](runs/r3-hand-wrenna.txt).
Round 2 played a mage and a rogue; this is the first hunter.

| | Wrenna (hunter) |
|---|---|
| Level 2 / 3 / 4 | about 8:00 / 18:40 / 29:04 |
| At 30:00 | level 4, 2 of 2,100 |
| Quests done | 13 |
| Named places found | 10 |
| Deaths | 2: a Grazer at 5:00 (my fault), a Thornboar at 21:45 (part mine, part the pack's) |
| Gear | leggings at 8, the Ashwood Longbow at 9 (Maren's reward), a woolen cloak at 13, Watchman's Mail Vest at 29 |
| Money at the end | 11 s 32 c |

**Minutes 0 to 5.** I woke facing Brannoc and the hearth. His quest and
Tilda's took 30 seconds, and Maren was 22 m away. The first Grazers were
south-west, 25 s from town. My first lesson as a hunter: four Grazers
emptied my mana (52 of 52 at the start), and after that I was meleeing
with a bow. I chained `attack` commands without looking and pulled the
sixth at 1 health of 50. That death is mine. The game never told me I was
out of mana; the status line did, and I didn't read it. A player at a
keyboard would have seen the blue bar empty.

**Minutes 5 to 13.** I respawned at the hearth, rested 20 s in the safe
town, then did Kindling (eight trees beside the hearth, 30 seconds) and
the last two Grazers. Level 2 came at about 8:00, three minutes behind a
bot because of the death. Wool came from the flock at the north gate and
coal from the North Gate Rocks. Coal for the Watch's tracker still says
"dropped by Candlekin, 746 m east" until you stand on the seam (round 2's
note, still open). Maren's reward is the first time the hunter is a
hunter: the Ashwood Longbow.

**Minutes 13 to 19: the south road and back.** Tolan's Broken Tower is a
225 m walk south for one discovery and the hand-in, 3½ minutes in all. The
Lay of the Land went west and north from there and paid level 3 on the
way back. The walk felt long because the tower is the only thing on it.
It is the one stretch of the half hour that is a walk and nothing else.

**Minutes 19 to 25: the North Watch.** Holt hands you six quests at once
now, Tusks and Tempers among them, and five of them are within 150 m.
The boar pack east of the tower is where the hunter's half hour went
wrong. Three boars took my mana again. I sat down to rest in the middle
of the pack (that part is mine), and a boar that came back in 60 s
interrupted the rest twice. The third fight ended with me at 0 and him at
3 of 75. Respawning means 255 m back from the hearth. On the way back,
straight across the meadow, I walked into the west pack. Lesson: a hunter
at level 3 has mana for two fights, and Holt's packs come back fast
enough to find you sitting.

**Minutes 25 to 30: the logging trail.** This is where this round's change
shows:

- The walk to the Woodcutters' Well paid three things at once: Birchrise
  discovered, the Well discovered, and the quest step (+70 XP).
- I took Old Tusk from Holt. The quest log said `Looking for group (made
  for 2): nobody else here carries it yet`, which is true: I was alone on
  the server. It is the first time the game told me what a group quest
  wants of me before I walked into it.
- The walk to Garrick was 61 s. I arrived with 4 of 5 tusks (one boar
  short) and talked to him (my `goto` had only walked me there).
- `accept all` then walked me 150 m back toward Hollis, the nearest giver
  with something to offer, which is not what a player means by it. On
  the way I took the fifth tusk off a boar.
- Garrick took both quests and I was level 4 at 29:04, in mail.

What I would tell the next designer:

- The opening up to the North Watch is good. The hunter fails the bar's
  "a single same-level mob is safe" twice in twenty minutes, both times
  after its mana ran out (for the class lane).
- The walk to Garrick is better: two discoveries and a quest step on the
  way, and tusks in the bags before I got there.
- The Broken Tower, alone at the end of the south road, is now the
  emptiest walk of the half hour.


## What changed

### Rules (content/rules/hearthblock.json; the record is tools/playtest/round3_rules.py)

| Stage | Change | Why |
|---|---|---|
| group | Old Tusk's quest `group: 2` and new text; the Barrow Warden's `group: 3` | Section 6 |
| content | Garrick's Camp: explore the Woodcutters' Well, then talk to Garrick | 285 m for a hello (round 2); baseline: 3 gaps of 2:23 to 2:26 on that walk |
| content | Tusks and Tempers given by Holt (after The North Watch), handed in to Garrick | Garrick sent you back out for tusks: "go to the Thornboar camp for Boar Tusk" was 10 bot-minutes in round 2; now the boars on the way (Holt's and the trail's) pay for it before you arrive |
| content | Bread for the Mill: the Listening Stones on the way to Ostrin | 490 m for a hello |
| content | Down to the Water: the Wayshrine of the Mere on the way to Wenna; Pincher Supper given by Ostrin with it, handed in to Wenna | The same walk, and the claw meat from the pinchers on that shore |
| content | Word for the Hunter: Hunter's Rest on the way to Rysa | 300 m for a hello (5.2 bot-minutes of baseline A's travel) |
| content | Silkmother: her camp's level (was +1) and 2.0 times a Websnare (was 2.5) | 2 of round 2's 13 deaths and baseline A's one questing death |
| content | Into Barrowwood: stand at the barrow's far edge, 37 m from the Warden (was 20 m) | Baseline B lost a level 4 to him there |
| vale (after the counted runs) | Know the Vale: Emberrun Ford (levels 4 to 5) instead of the Shrine Shore, which is a gloomwisp camp of levels 7 to 8 | After run A lost a level 5 there |

### Engine (small, general)

| Change | Where |
|---|---|
| `Quest::group` | `dyno-world` `content.rs` |
| `ServerMsg::Seekers`, `Seeker` (protocol 26) | `dyno-protocol` |
| `tick_seekers`, once a second | `dyno-server` `rpg.rs`, `game.rs` |
| `equip_place`, the one rule for where gear goes, used by the server and the bots | `dyno-sim` `character.rs`, `dyno-server` `gear.rs` |
| The Looking for group box, the map's seekers | `dyno-client` `app/hud.rs`, `worldmap.rs`, `app.rs` (message), `content/ui/strings.json` |

### The bots and tools

- Section 1's four behaviours, looking for group, `--content`, and the
  LFG line in `dyno play`.
- `tools/playtest/round3.py`, `tools/playtest/round3_rules.py`.

## For the other lanes

**Class lane:**

- A hunter at level 1 has mana for four Grazers. At level 3 it has mana
  for two Thornboars, and with no mana it fights in melee with a bow and
  loses to a same-level boar. Twice in my 30 minutes. The bar's "a single
  same-level mob is safe" fails for a hunter who keeps going.
- Two fresh level 4s in starter gear (warrior, priest) beat Old Tusk in
  16 s when they engaged together. When the warrior arrived a few seconds
  first, he killed her and then the priest. Pairs of healers (two priests)
  lost to him in the swarm.
- Paladins and mages (the grinders) rest 51% of their time when resting
  to 85% health and mana. The out-of-combat regeneration is slow next to
  the cost of a fight.

**UI lane:**

- The Looking for group box (`hud.rs` `looking_for_group`) and the map's
  green seeker dots are new and not looked at in a running client.
  Please shoot them and place the box.
- Round 2's tracker note is still open: Coal for the Watch and Feed the
  Watchfire point at "Candlekin, 746 m east" until you stand on a seam.

**`dyno play` (tools):**

- `goto <npc>` walks there and reports "arrived" but does not talk; a
  talk objective stays open until `talk`.
- `accept all` walks to the nearest giver with anything to offer, which
  was 150 m back the way I came.

**World lane:**

- The Broken Tower, 225 m south of the gate whose guard sends you there,
  is the emptiest walk of the half hour. A pack or a node on the south
  road would carry it.
- The Shrine Shore is a level 7 to 8 camp. A level 5 quest named it as a
  place to stand; it is out of that quest now, but the name still reads
  like a sightseeing stop on the map.

**Coordinator:**

- `Seekers` rides on protocol 26, after `Pong` and `Refused`, with no
  bump.
- `dyno bots --content` lets a `--local` swarm run on any rules file.
- This branch merged main twice (voxel rules; then feel F2 at protocol
  26). The counted runs are on v0.11.3's rules on purpose (see "How it was
  measured"). The hand-in's rules are main's with this round's stages on
  top.


## Still open, and what I would do next

1. **Travel is the zone's shape now, not the bots'.** Two hubs 255 m
   apart, a third 285 m on, the Broken Tower 225 m south of its giver,
   and a wall with no west gate. A completionist bot travels 66% of its
   time. Content moved within a hub's ring will not reach 50%; moving
   hubs or adding a gate will.
2. **Kill quests with a place** (round 2's item 2, still open). It would
   have kept the after run's level 5 from walking to the Old Barrow for
   Eyes in the Dark, and Thieves in the Fields would not need its own
   creature.
3. **A quest's hand-ins should gather.** Taking Tusks and Tempers from
   Holt moved the boars onto the way, but a bot (and a player who finishes
   Garrick's Camp first) walks back to Garrick with the tusks. Garrick's
   Camp could take the tusks as its own step ("Talk to Garrick, bring five
   tusks").
4. **Bots, next build.** Two things, both seen in this round's runs:
   - the party brain's follower idles in town for 8 to 10 minutes;
   - a pair that wipes on an elite should not walk back for a second try
     at the same level.
5. **Know the Vale** has four places across the whole vale and two
   2:50 gaps between them. Two places, or a reason to stop between them.
