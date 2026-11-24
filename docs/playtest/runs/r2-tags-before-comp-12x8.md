# Hearthblock playtest: 12 bots, 8 min, brain completionist

12 bots on `local`, 96.0 bot-minutes (longest session 8.0 min). Thresholds: docs/playtest/fun-metrics.md.

Machine: load average 16.8 at the start, 11.9 mean, 16.8 at most, on 16 cores. Server tick: mean 0.45 ms, p95 1.25 ms, p99 3.50 ms, worst 30 ms over 9909 ticks; the worst minute's p95 was 1.50 ms (budget 50 ms).

## Fun checks: 3 fail, 1 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 74.00 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 8.23 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 184.69 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.36 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.70  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.23 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 8.06 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| PASS | Deaths per hour of play | 0.63 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.99  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 6.25 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.01  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.59  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 36.83 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.00  | >= 2 pass, < 1.5 fail |
| FAIL | Share of casts that are the most used ability | 0.85  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.27  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 0.63 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 5108.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 74 s; first quest turned in: median 8 s; level 2: median 185 s (100% of level-1 starters reached it)
- XP per hour: median 8695 (design pace 6400); 70% of XP from quests; 83 quests turned in, median 2.2 min each
- Rewards: median gap 8 s; median longest drought 53 s; 0% of bots had a drought over 5 min
- Combat: 216 fights, 99% won, median 6.3 s; 117 kills, 44 without credit (2 of them with no other player within 20 blocks); 1 deaths (0.6/h)
- Travel: 37 blocks per reward; money earned 5108 c, spent 0 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 0.6 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| dead | 0.1% |
| fight | 30.7% |
| gather | 4.8% |
| idle | 1.2% |
| loot | 3.3% |
| town | 1.2% |
| travel | 58.8% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 12 | 1.0 | 0 |
| 1 | 12 | 1.0 | 130 |
| 2 | 12 | 1.0 | 267 |
| 3 | 12 | 1.0 | 384 |
| 4 | 12 | 2.0 | 546 |
| 5 | 12 | 2.0 | 726 |
| 6 | 12 | 2.0 | 877 |
| 7 | 12 | 2.0 | 1023 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 4 | 0 | - |
| broken_tower | 12 | 10 | 4.1 |
| coal_for_the_watch | 12 | 5 | 3.2 |
| cold_hearth | 12 | 12 | 0.1 |
| eyes_in_the_dark | 4 | 0 | - |
| garricks_camp | 4 | 0 | - |
| iron_for_hollis | 2 | 0 | - |
| kindling | 12 | 12 | 0.6 |
| lay_of_the_land | 10 | 1 | 4.9 |
| lookouts_eye | 4 | 2 | 0.8 |
| meat_for_the_pot | 12 | 12 | 3.6 |
| north_watch | 12 | 4 | 1.9 |
| thin_the_herd | 12 | 12 | 4.7 |
| watchfire | 4 | 1 | 0.4 |
| windmill_breadcrumb | 2 | 0 | - |
| wool_for_winter | 12 | 12 | 2.1 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x12
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x3
- watchfire: collect Coal: no source in reach x2
- iron_for_hollis: collect Raw iron: no source in reach x1
- thin_the_herd: goto failed: too many repaths x1

## Deaths by cause

- You fell to a Thornboar. x1

## Fights by creature and outcome

- Grazer: won x209
- Thornboar: won x5
- Grazer: lost (could not reach or hurt it) x1
- Thornboar: lost (died) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 27 | Quick Shot 74%, Barbed Arrow 22%, Recall 4% |
| Mage | 59 | Fireball 80%, Fire Blast 15%, Scholar's Focus 3%, Recall 2% |
| Paladin | 27 | Judging Blow 96%, Recall 4% |
| Priest | 72 | Smite 83%, Withering Word 17% |
| Rogue | 43 | Vicious Strike 81%, Finishing Cut 16%, Recall 2% |
| Warrior | 22 | Heroic Blow 82%, Battle Cry 9%, Recall 9% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-80, 176]: 1 events
  - Botyas04com at [-76, -15, 190] going to [-70, -15, 190] (repaths; thin_the_herd: go to the Grazer camp)

## Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)

- Grazer lv1 at -97,-6,-81 (ground -8) x1
- Grazer lv2 at -137,-9,-16 (ground -11) x1
- Grazer lv2 at 71,-6,-87 (ground -8) x1
- Grazer lv2 at 92,-8,78 (ground -10) x1
- Thornboar lv3 at -47,-9,-311 (ground -11) x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| completionist | 12 | 8695 | 83 | 0.6 | 99% | 8 | 1% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 2 | 9539 | 15 | 0.0 | 100% | 9 | 1% |
| Mage | 2 | 8857 | 14 | 0.0 | 100% | 5 | 1% |
| Paladin | 2 | 7971 | 13 | 0.0 | 100% | 11 | 1% |
| Priest | 2 | 8211 | 13 | 3.8 | 95% | 8 | 1% |
| Rogue | 2 | 8732 | 14 | 0.0 | 100% | 9 | 1% |
| Warrior | 2 | 8988 | 14 | 0.0 | 100% | 9 | 2% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botyas00com | Warrior | completionist | 8.0 | 1->2 | 8456 | 7/9 | 8 | 0 | 10 | 55 |
| Botyas01com | Paladin | completionist | 8.0 | 1->2 | 7232 | 6/9 | 9 | 0 | 15 | 53 |
| Botyas02com | Hunter | completionist | 8.0 | 1->3 | 10398 | 8/11 | 8 | 0 | 6 | 52 |
| Botyas03com | Rogue | completionist | 8.0 | 1->2 | 7157 | 6/9 | 9 | 0 | 11 | 54 |
| Botyas04com | Priest | completionist | 8.0 | 1->2 | 7022 | 6/9 | 9 | 0 | 8 | 51 |
| Botyas05com | Mage | completionist | 8.0 | 1->3 | 9866 | 8/15 | 9 | 0 | 5 | 51 |
| Botyas06com | Warrior | completionist | 8.0 | 1->2 | 9519 | 7/9 | 14 | 0 | 8 | 48 |
| Botyas07com | Paladin | completionist | 8.0 | 1->2 | 8711 | 7/9 | 10 | 0 | 7 | 70 |
| Botyas08com | Hunter | completionist | 8.0 | 1->2 | 8680 | 7/14 | 8 | 0 | 13 | 50 |
| Botyas09com | Rogue | completionist | 8.0 | 1->3 | 10307 | 8/14 | 11 | 0 | 7 | 86 |
| Botyas10com | Priest | completionist | 8.0 | 1->2 | 9400 | 7/13 | 10 | 1 | 8 | 52 |
| Botyas11com | Mage | completionist | 8.0 | 1->2 | 7849 | 6/9 | 12 | 0 | 5 | 86 |
