# Hearthblock playtest: 4 bots, 10 min, brain completionist

4 bots on `local`, 40.0 bot-minutes (longest session 10.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 3 fail, 2 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 32.38 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 8.42 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 74.08 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 2.73 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.60  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 1.43 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 8.61 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 18.00 /h | <= 1.5 pass, > 4 fail |
| WARN | Fights won | 0.81  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 7.61 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.03  | <= 0.15 pass, > 0.3 fail |
| PASS | Share of time travelling | 0.32  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 18.56 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.50  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.85  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.15  | <= 0.05 pass, > 0.15 fail |
| FAIL | Stuck events per bot-hour | 45.00 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 3442.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 32 s; first quest turned in: median 8 s; level 2: median 74 s (100% of level-1 starters reached it)
- XP per hour: median 17474 (design pace 6400); 60% of XP from quests; 50 quests turned in, median 1.4 min each
- Rewards: median gap 9 s; median longest drought 54 s; 0% of bots had a drought over 5 min
- Combat: 129 fights, 81% won, median 7.6 s; 82 kills, 15 without credit (4 of them with no other player within 20 blocks); 12 deaths (18.0/h)
- Travel: 19 blocks per reward; money earned 3442 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 45.0 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| dead | 1.6% |
| fight | 54.9% |
| gather | 6.4% |
| idle | 1.0% |
| loot | 2.3% |
| rest | 0.4% |
| town | 1.4% |
| travel | 32.2% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 4 | 1.0 | 0 |
| 1 | 4 | 1.0 | 282 |
| 2 | 4 | 2.0 | 660 |
| 3 | 4 | 2.0 | 1180 |
| 4 | 4 | 3.0 | 1572 |
| 5 | 4 | 3.0 | 1884 |
| 6 | 4 | 3.0 | 1988 |
| 7 | 4 | 3.0 | 2208 |
| 8 | 4 | 3.0 | 2428 |
| 9 | 4 | 3.5 | 2595 |
| 10 | 1 | 4.0 | 2767 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 4 | 3 | 4.2 |
| coal_for_the_watch | 4 | 4 | 2.6 |
| cold_hearth | 4 | 4 | 0.1 |
| down_to_water | 4 | 1 | 3.0 |
| eyes_in_the_dark | 4 | 0 | - |
| garricks_camp | 4 | 4 | 0.4 |
| into_barrowwood | 1 | 0 | - |
| iron_for_hollis | 4 | 0 | - |
| kindling | 4 | 4 | 1.6 |
| meat_for_the_pot | 4 | 4 | 1.6 |
| north_watch | 4 | 4 | 0.3 |
| pincher_supper | 1 | 0 | - |
| report_to_holt | 4 | 4 | 0.1 |
| shells_for_nets | 1 | 0 | - |
| silk_for_sails | 4 | 0 | - |
| stolen_grain | 4 | 0 | - |
| thieves_in_fields | 4 | 1 | 2.8 |
| thin_the_herd | 4 | 4 | 3.5 |
| tusks_and_tempers | 4 | 1 | 4.9 |
| watchfire | 4 | 4 | 1.9 |
| windmill_breadcrumb | 4 | 4 | 2.8 |
| wool_for_winter | 4 | 4 | 1.2 |

## Blockers (bots that could not progress an objective)

- eyes_in_the_dark: kill Shade: only at night: waiting for dark x4
- coal_for_the_watch: collect Coal: no source in reach x2

## Deaths by cause

- You fell to a Ashcloak Bandit. x5
- You fell to a Websnare. x4
- You fell to a Thornboar. x3

## Fights by creature and outcome

- Grazer: won x55
- Thornboar: won x33
- Ashcloak Bandit: won x10
- Thornboar: lost (it fled home and healed) x5
- Websnare: lost (died) x5
- Ashcloak Bandit: lost (died) x4
- Websnare: won x4
- Thornboar: lost (died) x3
- Websnare: lost (it fled home and healed) x3
- Mirepincher: won x2
- Websnare: lost (could not reach or hurt it) x2
- Ashcloak Bandit: lost (cannot reach it: too many repaths) x1
- Ashcloak Bandit: lost (could not reach or hurt it) x1
- Thornboar: lost (could not reach or hurt it) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 41 | Quick Shot 98%, Barbed Arrow 2% |
| Paladin | 39 | Judging Blow 72%, Holy Mend 23%, Recall 3%, Valour 3% |
| Rogue | 65 | Vicious Strike 82%, Finishing Cut 18% |
| Warrior | 42 | Heroic Blow 86%, Battle Cry 10%, Onrush 5% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-16, -48]: 7 events
  - Botjay00com at [-2, -7, -43] going to [-2, -14, -43] (no path; watchfire: gather Coal)
  - Botjay00com at [-7, -7, -48] going to [-7, -14, -48] (no path; iron_for_hollis: gather Raw iron)
  - Botjay01com at [-2, -7, -43] going to [-2, -14, -43] (no path; coal_for_the_watch: gather Coal)
  - Botjay02com at [-5, -6, -44] going to [-5, -14, -44] (no path; coal_for_the_watch: gather Coal)
- cell [-64, -112]: 6 events
  - Botjay00com at [-63, -5, -100] going to [-63, -17, -100] (no path; iron_for_hollis: gather Raw iron)
  - Botjay00com at [-60, -5, -112] going to [-60, -16, -112] (no path; iron_for_hollis: gather Raw iron)
  - Botjay00com at [-51, -4, -97] going to [-51, -21, -96] (no path; iron_for_hollis: gather Raw iron)
  - Botjay01com at [-63, -5, -100] going to [-63, -17, -100] (no path; iron_for_hollis: gather Raw iron)
- cell [0, -16]: 4 events
  - Botjay00com at [0, 0, -3] going to [0, -6, -3] (no path; coal_for_the_watch: gather Coal)
  - Botjay00com at [10, 0, -3] going to [10, -8, -3] (no path; iron_for_hollis: gather Raw iron)
  - Botjay00com at [11, 0, -8] going to [11, -15, -8] (no path; iron_for_hollis: gather Raw iron)
  - Botjay03com at [0, 0, -3] going to [0, -6, -3] (no path; coal_for_the_watch: gather Coal)
- cell [-16, -16]: 2 events
  - Botjay01com at [-1, 0, -4] going to [-1, -8, -4] (no path; coal_for_the_watch: gather Coal)
  - Botjay02com at [-1, 0, -4] going to [-1, -8, -4] (no path; coal_for_the_watch: gather Coal)
- cell [0, -48]: 2 events
  - Botjay02com at [0, -7, -42] going to [0, -14, -42] (no path; iron_for_hollis: gather Raw iron)
  - Botjay03com at [0, -7, -42] going to [0, -14, -42] (no path; iron_for_hollis: gather Raw iron)
- cell [48, 64]: 2 events
  - Botjay02com at [58, -10, 70] going to [59, -8, 72] (repaths; thieves_in_fields: kill Ashcloak Bandit)
  - Botjay02com at [62, -10, 67] going to [62, -17, 67] (no path; iron_for_hollis: gather Raw iron)
- cell [-80, -112]: 1 events
  - Botjay00com at [-69, -3, -98] going to [-69, -22, -98] (no path; watchfire: gather Coal)
- cell [-32, 64]: 1 events
  - Botjay01com at [-26, -6, 65] going to [-26, -17, 65] (no path; iron_for_hollis: gather Raw iron)
- cell [0, -112]: 1 events
  - Botjay01com at [3, -9, -110] going to [3, -19, -110] (no path; coal_for_the_watch: gather Coal)
- cell [0, 80]: 1 events
  - Botjay03com at [9, -10, 85] going to [10, -18, 87] (no path; watchfire: gather Coal)
- cell [0, 96]: 1 events
  - Botjay03com at [3, -9, 101] going to [3, -15, 102] (no path; iron_for_hollis: gather Raw iron)
- cell [16, 32]: 1 events
  - Botjay02com at [30, -7, 46] going to [30, -15, 46] (no path; iron_for_hollis: gather Raw iron)

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| completionist | 4 | 17474 | 50 | 18.0 | 81% | 9 | 3% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 1 | 16699 | 12 | 18.0 | 69% | 6 | 2% |
| Paladin | 1 | 16601 | 12 | 12.0 | 88% | 9 | 3% |
| Rogue | 1 | 18248 | 13 | 24.0 | 80% | 8 | 3% |
| Warrior | 1 | 20017 | 13 | 18.0 | 85% | 9 | 2% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botjay00com | Warrior | completionist | 10.0 | 1->4 | 20017 | 13/20 | 25 | 3 | 9 | 32 |
| Botjay01com | Paladin | completionist | 10.0 | 1->4 | 16601 | 12/19 | 19 | 2 | 9 | 65 |
| Botjay02com | Hunter | completionist | 10.0 | 1->4 | 16699 | 12/19 | 19 | 3 | 6 | 80 |
| Botjay03com | Rogue | completionist | 10.0 | 1->4 | 18248 | 13/21 | 19 | 4 | 8 | 42 |
