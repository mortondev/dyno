# Hearthblock playtest: 4 bots, 10 min, brain completionist

4 bots on `127.0.0.1:7712`, 40.0 bot-minutes (longest session 10.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 2 fail, 1 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 48.24 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.75 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 131.88 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.57 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.73  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.22 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 8.38 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| PASS | Deaths per hour of play | 0.00 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 1.00  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 5.94 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.02  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.67  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 31.92 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.50  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.76  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.22  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 0.00 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 2656.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 48 s; first quest turned in: median 8 s; level 2: median 132 s (100% of level-1 starters reached it)
- XP per hour: median 10018 (design pace 6400); 73% of XP from quests; 37 quests turned in, median 2.2 min each
- Rewards: median gap 8 s; median longest drought 66 s; 0% of bots had a drought over 5 min
- Combat: 71 fights, 100% won, median 5.9 s; 52 kills, 15 without credit (0 of them with no other player within 20 blocks); 0 deaths (0.0/h)
- Travel: 32 blocks per reward; money earned 2656 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 0.0 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| fight | 23.4% |
| gather | 5.0% |
| idle | 1.7% |
| loot | 0.9% |
| rest | 0.5% |
| town | 1.2% |
| travel | 67.2% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 4 | 1.0 | 0 |
| 1 | 4 | 1.0 | 174 |
| 2 | 4 | 1.0 | 321 |
| 3 | 4 | 2.0 | 619 |
| 4 | 4 | 2.0 | 831 |
| 5 | 4 | 2.0 | 969 |
| 6 | 4 | 2.0 | 1081 |
| 7 | 4 | 2.0 | 1240 |
| 8 | 4 | 3.0 | 1430 |
| 9 | 4 | 3.0 | 1550 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 4 | 0 | - |
| broken_tower | 4 | 4 | 4.7 |
| coal_for_the_watch | 4 | 3 | 3.8 |
| cold_hearth | 4 | 4 | 0.1 |
| eyes_in_the_dark | 4 | 0 | - |
| garricks_camp | 4 | 0 | - |
| iron_for_hollis | 3 | 0 | - |
| kindling | 4 | 4 | 1.3 |
| lay_of_the_land | 4 | 1 | 3.3 |
| lookouts_eye | 4 | 2 | 0.8 |
| meat_for_the_pot | 4 | 4 | 2.0 |
| north_watch | 4 | 4 | 3.4 |
| thin_the_herd | 4 | 4 | 3.0 |
| watchfire | 4 | 3 | 0.4 |
| windmill_breadcrumb | 4 | 0 | - |
| wool_for_winter | 4 | 4 | 1.6 |

## Blockers (bots that could not progress an objective)

- eyes_in_the_dark: kill Shade: only at night: waiting for dark x4
- coal_for_the_watch: collect Coal: no source in reach x3
- iron_for_hollis: collect Raw iron: no source in reach x3
- watchfire: collect Coal: no source in reach x2

## Fights by creature and outcome

- Grazer: won x58
- Thornboar: won x13

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 21 | Quick Shot 71%, Barbed Arrow 24%, Recall 5% |
| Paladin | 21 | Judging Blow 76%, Holy Mend 14%, Rebuke 5%, Recall 5% |
| Rogue | 36 | Vicious Strike 81%, Finishing Cut 17%, Recall 3% |
| Warrior | 16 | Heroic Blow 62%, Rending Cut 19%, Battle Cry 12%, Recall 6% |

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| completionist | 4 | 10018 | 37 | 0.0 | 100% | 8 | 2% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 1 | 10511 | 10 | 0.0 | 100% | 8 | 2% |
| Paladin | 1 | 9524 | 8 | 0.0 | 100% | 11 | 2% |
| Rogue | 1 | 9306 | 9 | 0.0 | 100% | 8 | 2% |
| Warrior | 1 | 11639 | 10 | 0.0 | 100% | 9 | 2% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botrfq00com | Warrior | completionist | 10.0 | 1->3 | 11639 | 10/16 | 11 | 0 | 9 | 51 |
| Botrfq01com | Paladin | completionist | 10.0 | 1->3 | 9524 | 8/15 | 16 | 0 | 11 | 69 |
| Botrfq02com | Hunter | completionist | 10.0 | 1->3 | 10511 | 10/16 | 12 | 0 | 8 | 64 |
| Botrfq03com | Rogue | completionist | 10.0 | 1->3 | 9306 | 9/16 | 13 | 0 | 8 | 79 |
