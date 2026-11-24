# Hearthblock playtest: 4 bots, 10 min, brain party

4 bots on `127.0.0.1:7713`, 40.0 bot-minutes (longest session 10.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 2 fail, 1 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 48.85 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.84 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 89.54 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.79 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.73  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.20 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 10.47 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| PASS | Deaths per hour of play | 0.00 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 1.00  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 4.26 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.02  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.79  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 47.53 blocks | <= 60 pass, > 120 fail |
| WARN | Distinct abilities used per fighting bot | 1.75  | >= 2 pass, < 1.5 fail |
| FAIL | Share of casts that are the most used ability | 0.88  | <= 0.7 pass, > 0.85 fail |
| PASS | Kills lost to someone else's last hit | 0.04  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 1.50 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 1485.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 49 s; first quest turned in: median 8 s; level 2: median 90 s (100% of level-1 starters reached it)
- XP per hour: median 11448 (design pace 6400); 73% of XP from quests; 28 quests turned in, median 2.2 min each
- Rewards: median gap 10 s; median longest drought 73 s; 0% of bots had a drought over 5 min
- Combat: 53 fights, 100% won, median 4.3 s; 44 kills, 2 without credit (0 of them with no other player within 20 blocks); 0 deaths (0.0/h)
- Travel: 48 blocks per reward; money earned 1485 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 1.5 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| fight | 12.7% |
| gather | 2.8% |
| idle | 1.8% |
| loot | 0.2% |
| town | 3.8% |
| travel | 78.7% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 4 | 1.0 | 0 |
| 1 | 4 | 1.0 | 224 |
| 2 | 4 | 2.0 | 574 |
| 3 | 4 | 2.0 | 888 |
| 4 | 4 | 2.0 | 1105 |
| 5 | 4 | 2.0 | 1222 |
| 6 | 4 | 2.5 | 1302 |
| 7 | 4 | 3.0 | 1452 |
| 8 | 4 | 3.0 | 1582 |
| 9 | 4 | 3.0 | 1670 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 2 | 0 | - |
| broken_tower | 4 | 4 | 4.6 |
| coal_for_the_watch | 2 | 1 | 6.0 |
| cold_hearth | 4 | 4 | 0.1 |
| eyes_in_the_dark | 2 | 0 | - |
| garricks_camp | 2 | 2 | 2.1 |
| iron_for_hollis | 1 | 0 | - |
| kindling | 4 | 2 | 1.0 |
| lay_of_the_land | 4 | 1 | 4.4 |
| lookouts_eye | 2 | 0 | - |
| meat_for_the_pot | 4 | 4 | 1.6 |
| north_watch | 4 | 2 | 4.3 |
| silk_for_sails | 2 | 0 | - |
| thin_the_herd | 4 | 4 | 2.2 |
| tusks_and_tempers | 2 | 0 | - |
| watchfire | 2 | 0 | - |
| webs_between_roots | 1 | 0 | - |
| windmill_breadcrumb | 2 | 0 | - |
| wool_for_winter | 4 | 3 | 2.3 |
| word_for_rysa | 2 | 1 | 1.0 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x2
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x2
- watchfire: collect Coal: no source in reach x2
- iron_for_hollis: collect Raw iron: Requires mining 5 (have 4) x1
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x1

## Fights by creature and outcome

- Grazer: won x53

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 16 | Quick Shot 69%, Barbed Arrow 25%, Recall 6% |
| Paladin | 4 | Judging Blow 100% |
| Rogue | 13 | Vicious Strike 92%, Finishing Cut 8% |
| Warrior | 9 | Heroic Blow 78%, Battle Cry 11%, Recall 11% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-48, -48]: 1 events
  - Botnfw01par at [-45, 0, -46] going to [-42, 0, -43] (repaths; following Botnfw00par)

## Notes

- grouped in a party of two x4

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| party | 4 | 11448 | 28 | 0.0 | 100% | 10 | 2% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 1 | 12039 | 8 | 0.0 | 100% | 9 | 2% |
| Paladin | 1 | 7551 | 5 | 0.0 | 100% | 16 | 2% |
| Rogue | 1 | 11583 | 7 | 0.0 | 100% | 12 | 1% |
| Warrior | 1 | 11314 | 8 | 0.0 | 100% | 8 | 3% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botnfw00par | Warrior | party | 10.0 | 1->3 | 11314 | 8/11 | 11 | 0 | 8 | 65 |
| Botnfw01par | Paladin | party | 10.0 | 1->2 | 7551 | 5/8 | 7 | 0 | 16 | 81 |
| Botnfw02par | Hunter | party | 10.0 | 1->3 | 12039 | 8/18 | 15 | 0 | 9 | 85 |
| Botnfw03par | Rogue | party | 10.0 | 1->3 | 11583 | 7/17 | 11 | 0 | 12 | 51 |
