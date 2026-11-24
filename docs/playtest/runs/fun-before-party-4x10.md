# Hearthblock playtest: 4 bots, 10 min, brain party

4 bots on `127.0.0.1:7713`, 40.0 bot-minutes (longest session 10.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 1 fail, 4 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 111.85 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.84 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 206.26 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.17 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.65  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.48 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 9.19 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| PASS | Deaths per hour of play | 0.00 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 1.00  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 3.80 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.02  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.82  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 58.64 blocks | <= 60 pass, > 120 fail |
| WARN | Distinct abilities used per fighting bot | 1.75  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.81  | <= 0.7 pass, > 0.85 fail |
| PASS | Kills lost to someone else's last hit | 0.00  | <= 0.05 pass, > 0.15 fail |
| WARN | Stuck events per bot-hour | 4.50 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 932.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 112 s; first quest turned in: median 8 s; level 2: median 206 s (100% of level-1 starters reached it)
- XP per hour: median 7473 (design pace 6400); 65% of XP from quests; 19 quests turned in, median 2.5 min each
- Rewards: median gap 9 s; median longest drought 97 s; 0% of bots had a drought over 5 min
- Combat: 38 fights, 100% won, median 3.8 s; 36 kills, 0 without credit (0 of them with no other player within 20 blocks); 0 deaths (0.0/h)
- Travel: 59 blocks per reward; money earned 932 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 4.5 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| fight | 6.7% |
| gather | 2.7% |
| idle | 2.4% |
| loot | 0.2% |
| rest | 1.0% |
| town | 5.4% |
| travel | 81.5% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 4 | 1.0 | 0 |
| 1 | 4 | 1.0 | 60 |
| 2 | 4 | 1.0 | 266 |
| 3 | 4 | 1.5 | 362 |
| 4 | 4 | 1.5 | 451 |
| 5 | 4 | 2.0 | 601 |
| 6 | 4 | 2.0 | 739 |
| 7 | 4 | 2.0 | 798 |
| 8 | 4 | 2.0 | 954 |
| 9 | 4 | 2.0 | 1090 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 1 | 0 | - |
| coal_for_the_watch | 3 | 0 | - |
| cold_hearth | 4 | 4 | 0.1 |
| eyes_in_the_dark | 2 | 0 | - |
| garricks_camp | 1 | 0 | - |
| kindling | 4 | 3 | 0.9 |
| meat_for_the_pot | 4 | 4 | 4.6 |
| north_watch | 3 | 3 | 0.9 |
| report_to_holt | 2 | 1 | 0.7 |
| thin_the_herd | 4 | 3 | 7.4 |
| watchfire | 1 | 0 | - |
| windmill_breadcrumb | 1 | 0 | - |
| wool_for_winter | 4 | 1 | 3.7 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x3
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x2
- kindling: collect Log: no source in reach x2
- watchfire: collect Coal: no source in reach x1

## Fights by creature and outcome

- Grazer: won x38

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 15 | Quick Shot 67%, Barbed Arrow 27%, Recall 7% |
| Paladin | 6 | Judging Blow 100% |
| Rogue | 9 | Vicious Strike 78%, Finishing Cut 22% |
| Warrior | 5 | Heroic Blow 60%, Battle Cry 20%, Recall 20% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [128, 64]: 1 events
  - Botcfx01par at [140, -5, 78] going to [144, -5, 78] (repaths; following Botcfx00par)
- cell [192, 96]: 1 events
  - Botcfx03par at [203, -6, 99] going to [207, -6, 99] (repaths; following Botcfx02par)
- cell [224, 0]: 1 events
  - Botcfx03par at [235, -7, 10] going to [236, -7, 6] (repaths; following Botcfx02par)

## Notes

- grouped in a party of two x4

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| party | 4 | 7473 | 19 | 0.0 | 100% | 9 | 2% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 1 | 8841 | 6 | 0.0 | 100% | 9 | 1% |
| Paladin | 1 | 7354 | 5 | 0.0 | 100% | 8 | 2% |
| Rogue | 1 | 4053 | 2 | 0.0 | 100% | 34 | 6% |
| Warrior | 1 | 7592 | 6 | 0.0 | 100% | 10 | 0% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botcfx00par | Warrior | party | 10.0 | 1->2 | 7592 | 6/12 | 9 | 0 | 10 | 98 |
| Botcfx01par | Paladin | party | 10.0 | 1->2 | 7354 | 5/7 | 10 | 0 | 8 | 107 |
| Botcfx02par | Hunter | party | 10.0 | 1->3 | 8841 | 6/10 | 10 | 0 | 9 | 62 |
| Botcfx03par | Rogue | party | 10.0 | 1->2 | 4053 | 2/5 | 7 | 0 | 34 | 96 |
