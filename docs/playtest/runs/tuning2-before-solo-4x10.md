# Hearthblock playtest: 4 bots, 10 min, brain completionist

4 bots on `local`, 40.0 bot-minutes (longest session 10.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 2 fail, 2 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 29.49 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 8.41 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 85.67 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 2.50 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.62  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 1.01 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 6.89 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 22.50 /h | <= 1.5 pass, > 4 fail |
| WARN | Fights won | 0.75  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 8.69 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.03  | <= 0.15 pass, > 0.3 fail |
| PASS | Share of time travelling | 0.27  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 17.51 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.00  | >= 2 pass, < 1.5 fail |
| FAIL | Share of casts that are the most used ability | 0.86  | <= 0.7 pass, > 0.85 fail |
| WARN | Kills lost to someone else's last hit | 0.13  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 0.00 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 52.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 3361.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 29 s; first quest turned in: median 8 s; level 2: median 86 s (100% of level-1 starters reached it)
- XP per hour: median 16017 (design pace 6400); 62% of XP from quests; 46 quests turned in, median 1.0 min each
- Rewards: median gap 7 s; median longest drought 64 s; 0% of bots had a drought over 5 min
- Combat: 110 fights, 75% won, median 8.7 s; 71 kills, 11 without credit (2 of them with no other player within 20 blocks); 15 deaths (22.5/h)
- Travel: 18 blocks per reward; money earned 3361 c, spent 0 c
- Server: action round trip p50 50 ms, p95 52 ms; 0 disconnects; 0.0 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| dead | 2.0% |
| fight | 49.9% |
| gather | 16.0% |
| idle | 0.9% |
| loot | 1.8% |
| rest | 1.7% |
| town | 1.4% |
| travel | 26.5% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 4 | 1.0 | 0 |
| 1 | 4 | 1.0 | 298 |
| 2 | 4 | 2.0 | 622 |
| 3 | 4 | 2.0 | 978 |
| 4 | 4 | 3.0 | 1492 |
| 5 | 4 | 3.0 | 1898 |
| 6 | 4 | 3.0 | 2023 |
| 7 | 4 | 3.0 | 2302 |
| 8 | 4 | 3.0 | 2505 |
| 9 | 4 | 3.0 | 2570 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 4 | 2 | 3.8 |
| coal_for_the_watch | 4 | 4 | 1.2 |
| cold_hearth | 4 | 4 | 0.1 |
| down_to_water | 3 | 0 | - |
| eyes_in_the_dark | 4 | 0 | - |
| garricks_camp | 4 | 4 | 0.4 |
| into_barrowwood | 1 | 0 | - |
| iron_for_hollis | 4 | 1 | 2.9 |
| kindling | 4 | 4 | 1.5 |
| meat_for_the_pot | 4 | 4 | 2.4 |
| north_watch | 4 | 4 | 0.3 |
| report_to_holt | 4 | 4 | 0.1 |
| silk_for_sails | 4 | 0 | - |
| stolen_grain | 3 | 0 | - |
| thieves_in_fields | 3 | 0 | - |
| thin_the_herd | 4 | 4 | 3.9 |
| tusks_and_tempers | 4 | 1 | 2.8 |
| watchfire | 4 | 3 | 1.7 |
| windmill_breadcrumb | 4 | 3 | 4.7 |
| wool_for_winter | 4 | 4 | 0.8 |
| word_from_east | 1 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x4
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x4
- watchfire: collect Coal: no source in reach x3

## Deaths by cause

- You fell to a Thornboar. x5
- You fell to a Websnare. x4
- You fell to a Ashcloak Bandit. x3
- You fell to a Ashcloak Cutthroat. x2
- You fell to a Grazer. x1

## Fights by creature and outcome

- Grazer: won x48
- Thornboar: won x25
- Websnare: won x6
- Thornboar: lost (died) x5
- Ashcloak Bandit: lost (died) x4
- Ashcloak Bandit: won x4
- Thornboar: lost (it fled home and healed) x4
- Websnare: lost (died) x4
- Websnare: lost (it fled home and healed) x3
- Ashcloak Bandit: lost (it fled home and healed) x2
- Thornboar: lost (could not reach or hurt it) x2
- Ashcloak Cutthroat: lost (died) x1
- Grazer: lost (died) x1
- Websnare: lost (could not reach or hurt it) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 38 | Quick Shot 100% |
| Paladin | 35 | Judging Blow 71%, Holy Mend 23%, Recall 3%, Valour 3% |
| Rogue | 54 | Vicious Strike 85%, Finishing Cut 15% |
| Warrior | 14 | Heroic Blow 86%, Battle Cry 14% |

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| completionist | 4 | 16017 | 46 | 22.5 | 75% | 7 | 3% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 1 | 14721 | 11 | 42.0 | 53% | 5 | 4% |
| Paladin | 1 | 21874 | 14 | 18.0 | 82% | 9 | 3% |
| Rogue | 1 | 17313 | 12 | 30.0 | 81% | 7 | 3% |
| Warrior | 1 | 10994 | 9 | 0.0 | 94% | 7 | 1% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botvsd00com | Warrior | completionist | 10.0 | 1->3 | 10994 | 9/16 | 11 | 0 | 7 | 281 |
| Botvsd01com | Paladin | completionist | 10.0 | 1->4 | 21874 | 14/21 | 22 | 3 | 9 | 59 |
| Botvsd02com | Hunter | completionist | 10.0 | 1->3 | 14721 | 11/19 | 17 | 7 | 5 | 67 |
| Botvsd03com | Rogue | completionist | 10.0 | 1->4 | 17313 | 12/19 | 21 | 5 | 7 | 62 |
