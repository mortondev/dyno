# Hearthblock playtest: 4 bots, 10 min, brain completionist

4 bots on `local`, 40.0 bot-minutes (longest session 10.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 1 fail, 2 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 28.02 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 8.36 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 82.02 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 2.26 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.65  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 1.04 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 7.80 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 6.00 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.90  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 8.51 s | <= 25 pass, > 40 fail (and >= 5) |
| WARN | Share of time idle, stuck, waiting for spawns or dead | 0.26  | <= 0.15 pass, > 0.3 fail |
| PASS | Share of time travelling | 0.26  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 14.51 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.75  | >= 2 pass, < 1.5 fail |
| PASS | Share of casts that are the most used ability | 0.69  | <= 0.7 pass, > 0.85 fail |
| WARN | Kills lost to someone else's last hit | 0.14  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 0.00 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 2588.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 28 s; first quest turned in: median 8 s; level 2: median 82 s (100% of level-1 starters reached it)
- XP per hour: median 14469 (design pace 6400); 65% of XP from quests; 44 quests turned in, median 1.0 min each
- Rewards: median gap 8 s; median longest drought 92 s; 0% of bots had a drought over 5 min
- Combat: 92 fights, 90% won, median 8.5 s; 65 kills, 11 without credit (0 of them with no other player within 20 blocks); 4 deaths (6.0/h)
- Travel: 15 blocks per reward; money earned 2588 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 0.0 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| dead | 0.5% |
| fight | 37.5% |
| gather | 5.9% |
| idle | 0.8% |
| loot | 1.1% |
| rest | 2.5% |
| town | 1.2% |
| travel | 25.9% |
| wait_spawn | 24.5% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 4 | 1.0 | 0 |
| 1 | 4 | 1.0 | 232 |
| 2 | 4 | 2.0 | 614 |
| 3 | 4 | 2.0 | 1052 |
| 4 | 4 | 2.5 | 1296 |
| 5 | 4 | 3.0 | 1806 |
| 6 | 4 | 3.0 | 1979 |
| 7 | 4 | 3.0 | 2052 |
| 8 | 4 | 3.0 | 2238 |
| 9 | 4 | 3.0 | 2252 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 4 | 2 | 3.9 |
| coal_for_the_watch | 4 | 4 | 1.3 |
| cold_hearth | 4 | 4 | 0.1 |
| down_to_water | 1 | 1 | 4.4 |
| eyes_in_the_dark | 4 | 0 | - |
| garricks_camp | 4 | 4 | 1.0 |
| into_barrowwood | 1 | 0 | - |
| iron_for_hollis | 4 | 0 | - |
| kindling | 4 | 4 | 1.3 |
| meat_for_the_pot | 4 | 4 | 1.9 |
| north_watch | 4 | 4 | 0.3 |
| pincher_supper | 1 | 0 | - |
| report_to_holt | 4 | 4 | 0.1 |
| silk_for_sails | 4 | 0 | - |
| stolen_grain | 1 | 0 | - |
| thieves_in_fields | 1 | 0 | - |
| thin_the_herd | 4 | 4 | 4.3 |
| tusks_and_tempers | 4 | 1 | 2.7 |
| watchfire | 4 | 3 | 0.1 |
| windmill_breadcrumb | 4 | 1 | 2.1 |
| wool_for_winter | 4 | 4 | 0.5 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x4
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x4
- iron_for_hollis: collect Raw iron: no source in reach x3
- watchfire: collect Coal: no source in reach x2
- iron_for_hollis: collect Raw iron: Requires mining 5 (have 4) x1
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x1

## Deaths by cause

- You fell to a Ashcloak Bandit. x1
- You fell to a Grazer. x1
- You fell to a Thornboar. x1
- You fell to a Websnare. x1

## Fights by creature and outcome

- Grazer: won x49
- Thornboar: won x23
- Websnare: won x10
- Ashcloak Bandit: lost (it fled home and healed) x2
- Ashcloak Bandit: lost (could not reach or hurt it) x1
- Ashcloak Bandit: lost (died) x1
- Ashcloak Bandit: won x1
- Grazer: lost (could not reach or hurt it) x1
- Grazer: lost (died) x1
- Grazer: lost (it fled home and healed) x1
- Thornboar: lost (died) x1
- Websnare: lost (died) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 39 | Quick Shot 64%, Barbed Arrow 36% |
| Paladin | 28 | Judging Blow 79%, Rebuke 14%, Holy Mend 7% |
| Rogue | 36 | Vicious Strike 78%, Finishing Cut 22% |
| Warrior | 40 | Heroic Blow 55%, Rending Cut 30%, Battle Cry 10%, Onrush 5% |

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| completionist | 4 | 14469 | 44 | 6.0 | 90% | 8 | 26% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 1 | 14034 | 12 | 12.0 | 77% | 8 | 23% |
| Paladin | 1 | 14904 | 11 | 0.0 | 100% | 8 | 19% |
| Rogue | 1 | 11846 | 9 | 6.0 | 91% | 6 | 38% |
| Warrior | 1 | 17654 | 12 | 6.0 | 92% | 8 | 22% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botyus00com | Warrior | completionist | 10.0 | 1->4 | 17654 | 12/17 | 20 | 1 | 8 | 83 |
| Botyus01com | Paladin | completionist | 10.0 | 1->3 | 14904 | 11/16 | 16 | 0 | 8 | 101 |
| Botyus02com | Hunter | completionist | 10.0 | 1->3 | 14034 | 12/20 | 16 | 2 | 8 | 80 |
| Botyus03com | Rogue | completionist | 10.0 | 1->3 | 11846 | 9/16 | 13 | 1 | 6 | 120 |
