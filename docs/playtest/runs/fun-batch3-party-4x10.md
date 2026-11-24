# Hearthblock playtest: 4 bots, 10 min, brain party

4 bots on `127.0.0.1:7713`, 40.0 bot-minutes (longest session 10.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 1 fail, 3 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 59.71 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.91 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 152.10 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.20 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.73  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.24 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 9.50 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| PASS | Deaths per hour of play | 0.00 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 1.00  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 4.92 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.13  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.57  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 37.35 blocks | <= 60 pass, > 120 fail |
| WARN | Distinct abilities used per fighting bot | 1.75  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.80  | <= 0.7 pass, > 0.85 fail |
| WARN | Kills lost to someone else's last hit | 0.11  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 1.50 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 1414.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 60 s; first quest turned in: median 8 s; level 2: median 152 s (100% of level-1 starters reached it)
- XP per hour: median 7666 (design pace 6400); 73% of XP from quests; 27 quests turned in, median 2.2 min each
- Rewards: median gap 9 s; median longest drought 173 s; 0% of bots had a drought over 5 min
- Combat: 56 fights, 100% won, median 4.9 s; 42 kills, 5 without credit (0 of them with no other player within 20 blocks); 0 deaths (0.0/h)
- Travel: 37 blocks per reward; money earned 1414 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 1.5 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| fight | 14.1% |
| gather | 3.1% |
| idle | 13.3% |
| loot | 0.7% |
| town | 12.0% |
| travel | 56.8% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 4 | 1.0 | 0 |
| 1 | 4 | 1.0 | 146 |
| 2 | 4 | 1.0 | 332 |
| 3 | 4 | 2.0 | 472 |
| 4 | 4 | 2.0 | 673 |
| 5 | 4 | 2.0 | 869 |
| 6 | 4 | 2.0 | 959 |
| 7 | 4 | 2.0 | 1006 |
| 8 | 4 | 2.0 | 1101 |
| 9 | 4 | 2.5 | 1146 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 2 | 0 | - |
| broken_tower | 4 | 3 | 5.6 |
| coal_for_the_watch | 3 | 1 | 0.8 |
| cold_hearth | 4 | 4 | 0.1 |
| eyes_in_the_dark | 2 | 0 | - |
| garricks_camp | 2 | 2 | 1.6 |
| iron_for_hollis | 1 | 0 | - |
| kindling | 4 | 3 | 0.5 |
| lay_of_the_land | 3 | 0 | - |
| lookouts_eye | 2 | 0 | - |
| meat_for_the_pot | 4 | 4 | 2.5 |
| north_watch | 4 | 2 | 3.6 |
| silk_for_sails | 2 | 0 | - |
| thin_the_herd | 4 | 4 | 3.7 |
| tusks_and_tempers | 2 | 0 | - |
| watchfire | 2 | 0 | - |
| webs_between_roots | 1 | 0 | - |
| wool_for_winter | 4 | 3 | 2.9 |
| word_for_rysa | 2 | 1 | 1.0 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x2
- kindling: collect Log: no source in reach x1

## Fights by creature and outcome

- Grazer: won x56

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 19 | Quick Shot 58%, Barbed Arrow 37%, Recall 5% |
| Paladin | 7 | Judging Blow 100% |
| Rogue | 11 | Vicious Strike 73%, Finishing Cut 27% |
| Warrior | 8 | Heroic Blow 75%, Battle Cry 12%, Recall 12% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-64, -16]: 1 events
  - Botvap01par at [-51, 0, -5] going to [-47, 0, -4] (repaths; following Botvap00par)

## Notes

- grouped in a party of two x4

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| party | 4 | 7666 | 27 | 0.0 | 100% | 9 | 13% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 1 | 9277 | 9 | 0.0 | 100% | 9 | 2% |
| Paladin | 1 | 4990 | 4 | 0.0 | 100% | 9 | 45% |
| Rogue | 1 | 9215 | 8 | 0.0 | 100% | 11 | 4% |
| Warrior | 1 | 6116 | 6 | 0.0 | 100% | 10 | 2% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botvap00par | Warrior | party | 10.0 | 1->2 | 6116 | 6/9 | 13 | 0 | 10 | 254 |
| Botvap01par | Paladin | party | 10.0 | 1->2 | 4990 | 4/7 | 9 | 0 | 9 | 268 |
| Botvap02par | Hunter | party | 10.0 | 1->3 | 9277 | 9/18 | 12 | 0 | 9 | 92 |
| Botvap03par | Rogue | party | 10.0 | 1->3 | 9215 | 8/18 | 8 | 0 | 11 | 52 |
