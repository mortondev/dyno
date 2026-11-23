# Hearthblock playtest: 4 bots, 10 min, brain party

4 bots on `local`, 40.0 bot-minutes (longest session 10.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 3 fail, 3 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 43.22 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 8.09 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 116.12 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 2.31 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.64  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 1.10 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 7.42 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 15.00 /h | <= 1.5 pass, > 4 fail |
| WARN | Fights won | 0.84  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 4.96 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.04  | <= 0.15 pass, > 0.3 fail |
| WARN | Share of time travelling | 0.45  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 21.82 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.00  | >= 2 pass, < 1.5 fail |
| FAIL | Share of casts that are the most used ability | 0.86  | <= 0.7 pass, > 0.85 fail |
| WARN | Kills lost to someone else's last hit | 0.05  | <= 0.05 pass, > 0.15 fail |
| FAIL | Stuck events per bot-hour | 39.00 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 2544.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 43 s; first quest turned in: median 8 s; level 2: median 116 s (100% of level-1 starters reached it)
- XP per hour: median 14770 (design pace 6400); 64% of XP from quests; 39 quests turned in, median 1.1 min each
- Rewards: median gap 7 s; median longest drought 57 s; 0% of bots had a drought over 5 min
- Combat: 133 fights, 84% won, median 5.0 s; 92 kills, 5 without credit (2 of them with no other player within 20 blocks); 10 deaths (15.0/h)
- Travel: 22 blocks per reward; money earned 2544 c, spent 0 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 39.0 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| dead | 1.2% |
| fight | 34.6% |
| gather | 5.3% |
| idle | 2.9% |
| loot | 1.6% |
| rest | 2.7% |
| town | 6.4% |
| travel | 45.2% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 4 | 1.0 | 0 |
| 1 | 4 | 1.0 | 232 |
| 2 | 4 | 1.5 | 468 |
| 3 | 4 | 2.0 | 765 |
| 4 | 4 | 2.5 | 1216 |
| 5 | 4 | 3.0 | 1395 |
| 6 | 4 | 3.0 | 1554 |
| 7 | 4 | 3.0 | 1710 |
| 8 | 4 | 3.0 | 2040 |
| 9 | 4 | 3.0 | 2205 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 4 | 3 | 4.1 |
| coal_for_the_watch | 4 | 2 | 2.7 |
| cold_hearth | 4 | 4 | 0.1 |
| down_to_water | 1 | 0 | - |
| eyes_in_the_dark | 4 | 0 | - |
| garricks_camp | 4 | 4 | 0.5 |
| iron_for_hollis | 2 | 0 | - |
| kindling | 4 | 4 | 2.0 |
| meat_for_the_pot | 4 | 4 | 3.0 |
| north_watch | 4 | 4 | 0.8 |
| report_to_holt | 4 | 4 | 0.2 |
| silk_for_sails | 4 | 0 | - |
| stolen_grain | 1 | 0 | - |
| thieves_in_fields | 1 | 0 | - |
| thin_the_herd | 4 | 4 | 2.6 |
| tusks_and_tempers | 4 | 0 | - |
| watchfire | 4 | 3 | 0.0 |
| windmill_breadcrumb | 2 | 1 | 1.5 |
| wool_for_winter | 4 | 2 | 3.6 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x4
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x4
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x1
- watchfire: collect Coal: no source in reach x1

## Deaths by cause

- You fell to a Websnare. x6
- You fell to a Ashcloak Bandit. x3
- You fell to a Thornboar. x1

## Fights by creature and outcome

- Grazer: won x60
- Thornboar: won x29
- Websnare: won x18
- Websnare: lost (died) x6
- Websnare: lost (it fled home and healed) x6
- Ashcloak Bandit: won x5
- Thornboar: lost (it fled home and healed) x4
- Ashcloak Bandit: lost (died) x2
- Thornboar: lost (died) x2
- Thornboar: lost (could not reach or hurt it) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 40 | Quick Shot 98%, Barbed Arrow 2% |
| Paladin | 18 | Judging Blow 78%, Holy Mend 22% |
| Rogue | 43 | Vicious Strike 88%, Finishing Cut 12% |
| Warrior | 20 | Heroic Blow 80%, Battle Cry 20% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [0, -16]: 5 events
  - Botnny00par at [0, 0, -3] going to [0, -6, -3] (no path; iron_for_hollis: gather Raw iron)
  - Botnny00par at [1, 0, -4] going to [0, -13, -6] (no path; watchfire: gather Coal)
  - Botnny01par at [0, 0, -3] going to [0, -6, -3] (no path; coal_for_the_watch: gather Coal)
  - Botnny02par at [13, 0, -1] going to [13, -12, -1] (no path; iron_for_hollis: gather Raw iron)
- cell [-64, -112]: 4 events
  - Botnny00par at [-51, -4, -97] going to [-51, -21, -96] (no path; watchfire: gather Coal)
  - Botnny01par at [-52, -4, -97] going to [-51, -21, -96] (no path; coal_for_the_watch: gather Coal)
  - Botnny02par at [-51, -4, -97] going to [-51, -21, -96] (no path; coal_for_the_watch: gather Coal)
  - Botnny03par at [-51, -4, -97] going to [-51, -21, -96] (no path; watchfire: gather Coal)
- cell [-16, -48]: 3 events
  - Botnny00par at [-5, -6, -44] going to [-5, -14, -44] (no path; watchfire: gather Coal)
  - Botnny00par at [-2, -5, -36] going to [-2, -15, -36] (no path; iron_for_hollis: gather Raw iron)
  - Botnny03par at [-7, -7, -48] going to [-7, -14, -48] (no path; coal_for_the_watch: gather Coal)
- cell [0, -64]: 2 events
  - Botnny00par at [10, -9, -57] going to [12, -20, -56] (no path; iron_for_hollis: gather Raw iron)
  - Botnny01par at [15, -9, -52] going to [17, -17, -52] (no path; coal_for_the_watch: gather Coal)
- cell [0, 0]: 2 events
  - Botnny00par at [13, 0, 1] going to [13, -8, 1] (no path; iron_for_hollis: gather Raw iron)
  - Botnny03par at [15, 0, 4] going to [15, -10, 4] (no path; coal_for_the_watch: gather Coal)
- cell [-112, -64]: 1 events
  - Botnny01par at [-97, -10, -49] going to [-97, -18, -49] (no path; coal_for_the_watch: gather Coal)
- cell [-112, -48]: 1 events
  - Botnny00par at [-104, -10, -47] going to [-104, -21, -47] (no path; iron_for_hollis: gather Raw iron)
- cell [-64, -80]: 1 events
  - Botnny00par at [-56, -4, -73] going to [-56, -23, -72] (no path; iron_for_hollis: gather Raw iron)
- cell [-64, -64]: 1 events
  - Botnny03par at [-60, -5, -62] going to [-60, -11, -62] (no path; watchfire: gather Coal)
- cell [-64, -48]: 1 events
  - Botnny02par at [-50, -4, -45] going to [-50, -22, -45] (no path; coal_for_the_watch: gather Coal)
- cell [-32, -96]: 1 events
  - Botnny02par at [-20, -6, -88] going to [-20, -22, -88] (no path; iron_for_hollis: gather Raw iron)
- cell [-16, -64]: 1 events
  - Botnny00par at [-7, -7, -49] going to [-7, -17, -47] (no path; iron_for_hollis: gather Raw iron)

## Notes

- grouped in a party of two x4

## Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)

- Shade lv3 at -42,8,2 (ground -1) x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| party | 4 | 14770 | 39 | 15.0 | 84% | 7 | 4% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 1 | 16472 | 12 | 18.0 | 81% | 5 | 2% |
| Paladin | 1 | 11161 | 8 | 12.0 | 86% | 12 | 4% |
| Rogue | 1 | 15050 | 8 | 18.0 | 82% | 8 | 6% |
| Warrior | 1 | 14491 | 11 | 12.0 | 90% | 7 | 4% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botnny00par | Warrior | party | 10.0 | 1->3 | 14491 | 11/16 | 22 | 2 | 7 | 52 |
| Botnny01par | Paladin | party | 10.0 | 1->3 | 11161 | 8/14 | 18 | 2 | 12 | 64 |
| Botnny02par | Hunter | party | 10.0 | 1->4 | 16472 | 12/19 | 30 | 3 | 5 | 45 |
| Botnny03par | Rogue | party | 10.0 | 1->3 | 15050 | 8/14 | 22 | 3 | 8 | 62 |
