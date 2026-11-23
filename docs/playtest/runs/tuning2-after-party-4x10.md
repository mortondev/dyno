# Hearthblock playtest: 4 bots, 10 min, brain party

4 bots on `local`, 40.0 bot-minutes (longest session 10.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 0 fail, 2 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 42.93 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 8.02 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 73.92 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 2.23 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.60  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 1.56 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 7.79 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| WARN | Deaths per hour of play | 1.50 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.95  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 5.54 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.06  | <= 0.15 pass, > 0.3 fail |
| WARN | Share of time travelling | 0.50  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 21.48 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.50  | >= 2 pass, < 1.5 fail |
| PASS | Share of casts that are the most used ability | 0.70  | <= 0.7 pass, > 0.85 fail |
| PASS | Kills lost to someone else's last hit | 0.00  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 1.50 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 2080.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 43 s; first quest turned in: median 8 s; level 2: median 74 s (100% of level-1 starters reached it)
- XP per hour: median 14256 (design pace 6400); 60% of XP from quests; 38 quests turned in, median 1.6 min each
- Rewards: median gap 8 s; median longest drought 67 s; 0% of bots had a drought over 5 min
- Combat: 86 fights, 95% won, median 5.5 s; 79 kills, 0 without credit (0 of them with no other player within 20 blocks); 1 deaths (1.5/h)
- Travel: 21 blocks per reward; money earned 2080 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 1.5 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| dead | 0.1% |
| fight | 23.8% |
| gather | 5.8% |
| idle | 4.9% |
| loot | 0.3% |
| rest | 6.6% |
| town | 7.4% |
| travel | 49.7% |
| wait_spawn | 1.3% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 4 | 1.0 | 0 |
| 1 | 4 | 1.0 | 319 |
| 2 | 4 | 2.0 | 678 |
| 3 | 4 | 2.0 | 940 |
| 4 | 4 | 2.5 | 1244 |
| 5 | 4 | 3.0 | 1538 |
| 6 | 4 | 3.0 | 1608 |
| 7 | 4 | 3.0 | 1766 |
| 8 | 4 | 3.0 | 2113 |
| 9 | 4 | 3.0 | 2167 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 2 | 1 | 1.6 |
| coal_for_the_watch | 4 | 4 | 2.6 |
| cold_hearth | 4 | 4 | 0.1 |
| down_to_water | 4 | 2 | 0.2 |
| eyes_in_the_dark | 2 | 0 | - |
| garricks_camp | 2 | 2 | 2.8 |
| iron_for_hollis | 4 | 0 | - |
| kindling | 4 | 4 | 5.0 |
| meat_for_the_pot | 4 | 4 | 1.7 |
| north_watch | 4 | 2 | 1.6 |
| pincher_supper | 2 | 0 | - |
| report_to_holt | 2 | 2 | 0.1 |
| silk_for_sails | 2 | 0 | - |
| stolen_grain | 4 | 0 | - |
| thieves_in_fields | 4 | 0 | - |
| thin_the_herd | 4 | 4 | 2.4 |
| tusks_and_tempers | 2 | 0 | - |
| watchfire | 2 | 1 | 0.1 |
| windmill_breadcrumb | 4 | 4 | 0.5 |
| wool_for_winter | 4 | 4 | 2.5 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x4
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x2
- iron_for_hollis: collect Raw iron: no source in reach x2
- iron_for_hollis: collect Raw iron: Requires mining 5 (have 4) x1
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x1
- watchfire: collect Coal: no source in reach x1

## Deaths by cause

- You fell to a Thornboar. x1

## Fights by creature and outcome

- Grazer: won x48
- Thornboar: won x18
- Websnare: won x10
- Ashcloak Bandit: won x6
- Ashcloak Bandit: lost (it fled home and healed) x2
- Thornboar: lost (died) x1
- Thornboar: lost (it fled home and healed) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 37 | Quick Shot 57%, Barbed Arrow 41%, Recall 3% |
| Paladin | 19 | Judging Blow 84%, Rebuke 11%, Holy Mend 5% |
| Rogue | 36 | Vicious Strike 83%, Finishing Cut 17% |
| Warrior | 22 | Heroic Blow 50%, Rending Cut 27%, Battle Cry 18%, Recall 5% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-80, -64]: 1 events
  - Botcgt01par at [-77, -10, -52] going to [-81, -10, -53] (repaths; following Botcgt00par)

## Notes

- grouped in a party of two x4

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| party | 4 | 14256 | 38 | 1.5 | 95% | 8 | 6% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 1 | 14871 | 11 | 0.0 | 91% | 7 | 3% |
| Paladin | 1 | 13018 | 8 | 0.0 | 100% | 8 | 2% |
| Rogue | 1 | 13729 | 8 | 6.0 | 91% | 8 | 18% |
| Warrior | 1 | 14784 | 11 | 0.0 | 100% | 8 | 2% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botcgt00par | Warrior | party | 10.0 | 1->3 | 14784 | 11/19 | 19 | 0 | 8 | 65 |
| Botcgt01par | Paladin | party | 10.0 | 1->3 | 13018 | 8/13 | 19 | 0 | 8 | 95 |
| Botcgt02par | Hunter | party | 10.0 | 1->3 | 14871 | 11/19 | 21 | 0 | 7 | 51 |
| Botcgt03par | Rogue | party | 10.0 | 1->3 | 13729 | 8/13 | 20 | 1 | 8 | 69 |
