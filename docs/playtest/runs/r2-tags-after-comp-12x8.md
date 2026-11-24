# Hearthblock playtest: 12 bots, 8 min, brain completionist

12 bots on `local`, 96.0 bot-minutes (longest session 8.0 min). Thresholds: docs/playtest/fun-metrics.md.

Machine: load average 7.1 at the start, 5.5 mean, 8.9 at most, on 16 cores. Server tick: mean 0.31 ms, p95 1.00 ms, p99 1.50 ms, worst 18 ms over 9963 ticks; the worst minute's p95 was 1.25 ms (budget 50 ms).

## Fun checks: 1 fail, 2 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 66.47 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.11 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 143.01 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.41 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.73  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.44 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 7.94 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| PASS | Deaths per hour of play | 0.00 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.98  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 6.91 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.01  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.63  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 36.53 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.08  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.81  | <= 0.7 pass, > 0.85 fail |
| PASS | Kills lost to someone else's last hit | 0.00  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 0.00 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 5972.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 66 s; first quest turned in: median 7 s; level 2: median 143 s (100% of level-1 starters reached it)
- XP per hour: median 9022 (design pace 6400); 73% of XP from quests; 89 quests turned in, median 2.4 min each
- Rewards: median gap 8 s; median longest drought 49 s; 0% of bots had a drought over 5 min
- Combat: 169 fights, 98% won, median 6.9 s; 122 kills, 0 without credit (0 of them with no other player within 20 blocks); 0 deaths (0.0/h)
- Travel: 37 blocks per reward; money earned 5972 c, spent 0 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 0.0 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| fight | 25.0% |
| gather | 4.9% |
| idle | 1.3% |
| loot | 3.7% |
| rest | 0.5% |
| town | 1.5% |
| travel | 63.1% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 12 | 1.0 | 0 |
| 1 | 12 | 1.0 | 152 |
| 2 | 12 | 1.0 | 322 |
| 3 | 12 | 2.0 | 482 |
| 4 | 12 | 2.0 | 632 |
| 5 | 12 | 2.0 | 806 |
| 6 | 12 | 2.0 | 959 |
| 7 | 12 | 2.0 | 1114 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 4 | 0 | - |
| broken_tower | 12 | 10 | 5.5 |
| coal_for_the_watch | 12 | 7 | 3.0 |
| cold_hearth | 12 | 12 | 0.1 |
| eyes_in_the_dark | 4 | 0 | - |
| garricks_camp | 4 | 1 | 2.4 |
| iron_for_hollis | 2 | 0 | - |
| kindling | 12 | 12 | 0.5 |
| lay_of_the_land | 10 | 5 | 3.1 |
| lookouts_eye | 4 | 1 | 1.3 |
| meat_for_the_pot | 12 | 12 | 2.3 |
| north_watch | 12 | 4 | 3.5 |
| silk_for_sails | 1 | 0 | - |
| thin_the_herd | 12 | 12 | 4.0 |
| tusks_and_tempers | 1 | 0 | - |
| watchfire | 4 | 1 | 0.6 |
| windmill_breadcrumb | 3 | 0 | - |
| wool_for_winter | 12 | 12 | 2.6 |
| word_for_rysa | 1 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x12
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x4
- iron_for_hollis: collect Raw iron: no source in reach x2
- watchfire: collect Coal: no source in reach x1

## Fights by creature and outcome

- Grazer: won x166
- Thornboar: lost (it fled home and healed) x3

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 28 | Quick Shot 61%, Barbed Arrow 32%, Recall 7% |
| Mage | 52 | Fireball 75%, Fire Blast 17%, Recall 4%, Scholar's Focus 4% |
| Paladin | 19 | Judging Blow 95%, Holy Mend 5% |
| Priest | 70 | Smite 77%, Withering Word 19%, Recall 3%, Mend 1% |
| Rogue | 30 | Vicious Strike 73%, Finishing Cut 20%, Recall 7% |
| Warrior | 15 | Heroic Blow 73%, Battle Cry 13%, Recall 13% |

## Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)

- Grazer lv1 at -182,-11,-64 (ground -13) x2
- Grazer lv2 at -137,-9,-16 (ground -11) x1
- Grazer lv2 at -87,-6,-95 (ground -8) x1
- Thornboar lv1 at -162,-8,50 (ground -10) x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| completionist | 12 | 9022 | 89 | 0.0 | 98% | 8 | 1% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 2 | 9452 | 15 | 0.0 | 100% | 7 | 0% |
| Mage | 2 | 9513 | 16 | 0.0 | 90% | 8 | 1% |
| Paladin | 2 | 8736 | 13 | 0.0 | 100% | 10 | 0% |
| Priest | 2 | 8804 | 13 | 0.0 | 100% | 8 | 2% |
| Rogue | 2 | 11226 | 18 | 0.0 | 100% | 8 | 2% |
| Warrior | 2 | 8700 | 14 | 0.0 | 100% | 7 | 2% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botchk00com | Warrior | completionist | 8.0 | 1->2 | 8417 | 7/9 | 9 | 0 | 7 | 50 |
| Botchk01com | Paladin | completionist | 8.0 | 1->2 | 9250 | 7/9 | 11 | 0 | 11 | 49 |
| Botchk02com | Hunter | completionist | 8.0 | 1->2 | 9062 | 7/9 | 10 | 0 | 7 | 49 |
| Botchk03com | Rogue | completionist | 8.0 | 1->3 | 11021 | 9/16 | 8 | 0 | 8 | 58 |
| Botchk04com | Priest | completionist | 8.0 | 1->2 | 8800 | 7/9 | 10 | 0 | 7 | 48 |
| Botchk05com | Mage | completionist | 8.0 | 1->2 | 8485 | 7/9 | 9 | 0 | 9 | 48 |
| Botchk06com | Warrior | completionist | 8.0 | 1->2 | 8983 | 7/9 | 11 | 0 | 7 | 59 |
| Botchk07com | Paladin | completionist | 8.0 | 1->2 | 8222 | 6/9 | 14 | 0 | 9 | 48 |
| Botchk08com | Hunter | completionist | 8.0 | 1->3 | 9842 | 8/15 | 8 | 0 | 8 | 49 |
| Botchk09com | Rogue | completionist | 8.0 | 1->3 | 11432 | 9/16 | 8 | 0 | 8 | 57 |
| Botchk10com | Priest | completionist | 8.0 | 1->2 | 8807 | 6/8 | 16 | 0 | 8 | 46 |
| Botchk11com | Mage | completionist | 8.0 | 1->3 | 10541 | 9/16 | 8 | 0 | 8 | 51 |
