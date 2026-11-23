# Hearthblock playtest: 4 bots, 10 min, brain party

4 bots on `local`, 40.0 bot-minutes (longest session 10.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 2 fail, 3 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 39.06 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.57 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 78.66 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 3.14 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.79  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 1.08 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 6.25 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 4.50 /h | <= 1.5 pass, > 4 fail |
| WARN | Fights won | 0.86  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 4.16 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.05  | <= 0.15 pass, > 0.3 fail |
| PASS | Share of time travelling | 0.36  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 19.44 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.25  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.80  | <= 0.7 pass, > 0.85 fail |
| PASS | Kills lost to someone else's last hit | 0.04  | <= 0.05 pass, > 0.15 fail |
| FAIL | Stuck events per bot-hour | 37.50 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| WARN | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 1.50 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 2223.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 39 s; first quest turned in: median 8 s; level 2: median 79 s (100% of level-1 starters reached it)
- XP per hour: median 20075 (design pace 6400); 79% of XP from quests; 36 quests turned in, median 1.1 min each
- Rewards: median gap 6 s; median longest drought 73 s; 0% of bots had a drought over 5 min
- Combat: 116 fights, 86% won, median 4.2 s; 81 kills, 3 without credit (0 of them with no other player within 20 blocks); 3 deaths (4.5/h)
- Travel: 19 blocks per reward; money earned 2223 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 37.5 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| dead | 0.4% |
| fight | 45.4% |
| gather | 5.5% |
| idle | 4.3% |
| loot | 1.0% |
| rest | 4.4% |
| stuck | 0.3% |
| town | 2.8% |
| travel | 35.7% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 4 | 1.0 | 0 |
| 1 | 4 | 1.0 | 295 |
| 2 | 4 | 2.0 | 652 |
| 3 | 4 | 2.5 | 1352 |
| 4 | 4 | 3.0 | 1941 |
| 5 | 4 | 3.0 | 2176 |
| 6 | 4 | 3.5 | 2481 |
| 7 | 4 | 3.5 | 2481 |
| 8 | 4 | 3.5 | 2959 |
| 9 | 4 | 4.0 | 3151 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 4 | 1 | 5.1 |
| coal_for_the_watch | 4 | 2 | 3.8 |
| cold_hearth | 4 | 4 | 0.1 |
| down_to_water | 1 | 0 | - |
| eyes_in_the_dark | 4 | 0 | - |
| garricks_camp | 4 | 4 | 0.4 |
| iron_for_hollis | 2 | 0 | - |
| kindling | 4 | 4 | 2.4 |
| meat_for_the_pot | 4 | 3 | 1.4 |
| north_watch | 4 | 4 | 0.6 |
| report_to_holt | 4 | 4 | 0.1 |
| silk_for_sails | 4 | 0 | - |
| stolen_grain | 1 | 0 | - |
| thieves_in_fields | 1 | 0 | - |
| thin_the_herd | 4 | 4 | 2.3 |
| tusks_and_tempers | 4 | 0 | - |
| watchfire | 4 | 3 | 1.1 |
| windmill_breadcrumb | 2 | 1 | 5.4 |
| wool_for_winter | 3 | 2 | 1.8 |

## Blockers (bots that could not progress an objective)

- eyes_in_the_dark: kill Shade: only at night: waiting for dark x4
- coal_for_the_watch: collect Coal: no source in reach x3
- watchfire: collect Coal: no source in reach x3
- iron_for_hollis: gather Raw iron: needs mining 25: raising it on easier nodes x2
- windmill_breadcrumb: talk failed: cannot reach NPC: stuck x1

## Deaths by cause

- You fell to a Gloomwisp. x2
- You fell to a Websnare. x1

## Fights by creature and outcome

- Grazer: won x51
- Thornboar: won x24
- Websnare: won x15
- Ashcloak Bandit: won x10
- Thornboar: lost (no progress: it hardly took damage) x7
- Thornboar: lost (cannot reach it: stuck) x3
- Ashcloak Bandit: lost (died) x2
- Ashcloak Bandit: lost (no progress: it hardly took damage) x2
- Websnare: lost (cannot reach it: stuck) x1
- Websnare: lost (died) x1

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-16, -96]: 7 events
  - Botkut00par at [-5, -4, -85] going to [-75, -5, -112] (stuck; boar_trouble: kill Thornboar)
  - Botkut00par at [-5, -4, -85] going to [-98, -13, -29] (stuck; silk_for_sails: kill Websnare for Websnare Silk)
  - Botkut00par at [-5, -4, -85] going to [12, -10, 97] (stuck; windmill_breadcrumb: talk to Farmer Ostrin)
  - Botkut00par at [-5, -4, -85] going to [-42, -3, -84] (stuck; boar_trouble: kill Thornboar)
- cell [0, 0]: 3 events
  - Botkut00par at [13, 0, 1] going to [13, -10, 1] (no path; iron_for_hollis: gather Raw iron)
  - Botkut02par at [12, 0, 1] going to [12, -7, 1] (no path; iron_for_hollis: gather Raw iron)
  - Botkut03par at [12, 0, 1] going to [12, -7, 1] (no path; coal_for_the_watch: gather Coal)
- cell [-16, -48]: 2 events
  - Botkut02par at [-7, -6, -47] going to [-7, -14, -47] (no path; coal_for_the_watch: gather Coal)
  - Botkut02par at [-2, -5, -41] going to [-2, -14, -41] (no path; coal_for_the_watch: gather Coal)
- cell [-16, -16]: 2 events
  - Botkut00par at [-2, 0, -8] going to [-2, -12, -8] (no path; iron_for_hollis: gather Raw iron)
  - Botkut01par at [-14, 0, -10] going to [-10, 0, -10] (repaths; following Botkut00par)
- cell [0, -96]: 2 events
  - Botkut01par at [3, -6, -90] going to [3, -21, -90] (no path; coal_for_the_watch: gather Coal)
  - Botkut01par at [2, -5, -86] going to [2, -21, -86] (no path; coal_for_the_watch: gather Coal)
- cell [0, -48]: 2 events
  - Botkut00par at [2, -6, -43] going to [2, -13, -43] (no path; iron_for_hollis: gather Raw iron)
  - Botkut02par at [0, -6, -45] going to [1, -13, -43] (no path; coal_for_the_watch: gather Coal)
- cell [0, -16]: 2 events
  - Botkut02par at [0, 0, -8] going to [-2, -12, -8] (no path; coal_for_the_watch: gather Coal)
  - Botkut02par at [10, 0, -4] going to [10, -11, -4] (no path; iron_for_hollis: gather Raw iron)
- cell [16, -48]: 2 events
  - Botkut00par at [25, -5, -48] going to [27, -1, -48] (no path (target on a tree canopy); wool_for_winter: kill Grazer for White)
  - Botkut01par at [23, -5, -48] going to [27, -1, -48] (no path (target on a tree canopy); looting)
- cell [-16, -32]: 1 events
  - Botkut03par at [-3, 0, -29] going to [-1, -16, -30] (no path; watchfire: gather Coal)
- cell [0, 80]: 1 events
  - Botkut02par at [10, -10, 87] going to [10, -17, 87] (no path; iron_for_hollis: gather Raw iron)
- cell [16, 32]: 1 events
  - Botkut02par at [29, -6, 41] going to [28, -13, 41] (no path; coal_for_the_watch: gather Coal)

## Rubber-bands (moves the server refused; hearth-relative)

- Botkut00par at [-17, -2, -67] sent back to [-37, -2, -78]: moved back 23.7 blocks (on ground false, in water false) (tusks_and_tempers: kill Thornboar for Boar Tusk)

## Notes

- grouped in a party of two x4
- used /spawn to get unstuck x1

## Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)

- Grazer lv1 at 27,-1,-50 (ground -6) x4
- Grazer lv1 at 28,-1,-45 (ground -5) x4
- Grazer lv1 at 28,0,-48 (ground -6) x4
- Grazer lv2 at 10,0,-102 (ground -8) x4
- Grazer lv2 at 10,0,-103 (ground -8) x4
- Grazer lv2 at 10,1,-101 (ground -8) x4
- Grazer lv2 at 11,0,-103 (ground -8) x4
- Grazer lv2 at 11,1,-100 (ground -8) x4
- Grazer lv2 at 11,1,-101 (ground -8) x4
- Grazer lv2 at 11,1,-102 (ground -8) x4
- Grazer lv2 at 13,-1,-92 (ground -7) x4
- Grazer lv2 at 13,-1,-95 (ground -7) x4
- Grazer lv2 at 13,0,-100 (ground -7) x4
- Grazer lv2 at 13,0,-103 (ground -8) x4
- Grazer lv2 at 23,0,-95 (ground -7) x4
- Grazer lv2 at 23,1,-93 (ground -7) x4
- Grazer lv2 at 24,0,-94 (ground -7) x4
- Grazer lv2 at 25,1,-48 (ground -6) x4
- Grazer lv2 at 26,-1,-95 (ground -7) x4
- Grazer lv2 at 26,0,-93 (ground -7) x4
- Grazer lv2 at 26,1,-45 (ground -5) x4
- Grazer lv2 at 28,0,-50 (ground -6) x4
- Grazer lv2 at 30,-3,-90 (ground -8) x4
- Grazer lv2 at 30,-3,-91 (ground -8) x4
- Grazer lv2 at 30,-3,-92 (ground -7) x4

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| party | 4 | 20075 | 36 | 4.5 | 86% | 6 | 5% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 1 | 22093 | 11 | 6.0 | 83% | 5 | 2% |
| Paladin | 1 | 18056 | 8 | 0.0 | 97% | 7 | 10% |
| Rogue | 1 | 13693 | 6 | 6.0 | 83% | 7 | 6% |
| Warrior | 1 | 23698 | 11 | 6.0 | 82% | 5 | 3% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botkut00par | Warrior | party | 10.0 | 1->4 | 23698 | 11/16 | 28 | 1 | 5 | 64 |
| Botkut01par | Paladin | party | 10.0 | 1->4 | 18056 | 8/14 | 20 | 0 | 7 | 75 |
| Botkut02par | Hunter | party | 10.0 | 1->4 | 22093 | 11/19 | 18 | 1 | 5 | 70 |
| Botkut03par | Rogue | party | 10.0 | 1->3 | 13693 | 6/13 | 15 | 1 | 7 | 120 |
