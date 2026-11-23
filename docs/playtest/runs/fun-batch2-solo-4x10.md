# Hearthblock playtest: 4 bots, 10 min, brain completionist

4 bots on `127.0.0.1:7712`, 40.0 bot-minutes (longest session 10.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 2 fail, 2 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 51.45 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.71 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 97.77 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 2.15 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.72  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.45 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 12.47 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| WARN | Deaths per hour of play | 3.00 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.97  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 7.09 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.02  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.66  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 34.50 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.25  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.74  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.22  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 0.00 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 2405.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 51 s; first quest turned in: median 8 s; level 2: median 98 s (100% of level-1 starters reached it)
- XP per hour: median 13736 (design pace 6400); 72% of XP from quests; 36 quests turned in, median 2.4 min each
- Rewards: median gap 12 s; median longest drought 50 s; 0% of bots had a drought over 5 min
- Combat: 68 fights, 97% won, median 7.1 s; 49 kills, 14 without credit (4 of them with no other player within 20 blocks); 2 deaths (3.0/h)
- Travel: 34 blocks per reward; money earned 2405 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 0.0 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| dead | 0.3% |
| fight | 26.3% |
| gather | 3.5% |
| idle | 2.1% |
| loot | 0.9% |
| town | 1.1% |
| travel | 65.8% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 4 | 1.0 | 0 |
| 1 | 4 | 1.0 | 210 |
| 2 | 4 | 2.0 | 476 |
| 3 | 4 | 2.0 | 682 |
| 4 | 4 | 2.0 | 947 |
| 5 | 4 | 2.0 | 1188 |
| 6 | 4 | 2.5 | 1314 |
| 7 | 4 | 3.0 | 1455 |
| 8 | 4 | 3.0 | 1612 |
| 9 | 4 | 3.0 | 1936 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 4 | 0 | - |
| broken_tower | 4 | 4 | 5.1 |
| coal_for_the_watch | 4 | 1 | 5.4 |
| cold_hearth | 4 | 4 | 0.1 |
| down_to_water | 1 | 0 | - |
| eyes_in_the_dark | 4 | 0 | - |
| garricks_camp | 4 | 0 | - |
| iron_for_hollis | 1 | 0 | - |
| kindling | 4 | 4 | 1.2 |
| lay_of_the_land | 4 | 2 | 2.8 |
| lookouts_eye | 4 | 3 | 0.7 |
| meat_for_the_pot | 4 | 4 | 2.6 |
| north_watch | 4 | 4 | 4.1 |
| stolen_grain | 1 | 0 | - |
| thieves_in_fields | 1 | 0 | - |
| thin_the_herd | 4 | 4 | 3.7 |
| watchfire | 4 | 1 | 0.3 |
| windmill_breadcrumb | 4 | 1 | 2.6 |
| wool_for_winter | 4 | 4 | 1.7 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x4
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x4
- watchfire: collect Coal: no source in reach x3
- iron_for_hollis: collect Raw iron: Requires mining 5 (have 4) x1
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x1

## Deaths by cause

- You fell to a Grazer. x1
- You fell to a Thornboar. x1

## Fights by creature and outcome

- Grazer: won x55
- Thornboar: won x11
- Grazer: lost (died) x1
- Thornboar: lost (died) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 15 | Quick Shot 60%, Barbed Arrow 33%, Recall 7% |
| Paladin | 15 | Judging Blow 80%, Recall 13%, Holy Mend 7% |
| Rogue | 45 | Vicious Strike 78%, Finishing Cut 20%, Recall 2% |
| Warrior | 23 | Heroic Blow 57%, Rending Cut 26%, Battle Cry 13%, Recall 4% |

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| completionist | 4 | 13736 | 36 | 3.0 | 97% | 12 | 2% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 1 | 14720 | 10 | 6.0 | 92% | 8 | 2% |
| Paladin | 1 | 12983 | 9 | 0.0 | 100% | 16 | 3% |
| Rogue | 1 | 14488 | 9 | 6.0 | 95% | 10 | 2% |
| Warrior | 1 | 11433 | 8 | 0.0 | 100% | 15 | 2% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botbnw00com | Warrior | completionist | 10.0 | 1->3 | 11433 | 8/18 | 13 | 0 | 15 | 79 |
| Botbnw01com | Paladin | completionist | 10.0 | 1->3 | 12983 | 9/15 | 9 | 0 | 16 | 52 |
| Botbnw02com | Hunter | completionist | 10.0 | 1->3 | 14720 | 10/16 | 10 | 1 | 8 | 48 |
| Botbnw03com | Rogue | completionist | 10.0 | 1->3 | 14488 | 9/15 | 17 | 1 | 10 | 44 |
