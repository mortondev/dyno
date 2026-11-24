# Hearthblock playtest: 4 bots, 10 min, brain party

4 bots on `127.0.0.1:7713`, 40.0 bot-minutes (longest session 10.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 1 fail, 3 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 50.07 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.97 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 140.51 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.29 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.75  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.10 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 10.07 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| PASS | Deaths per hour of play | 0.00 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.96  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 4.46 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.02  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.78  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 46.14 blocks | <= 60 pass, > 120 fail |
| WARN | Distinct abilities used per fighting bot | 1.75  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.73  | <= 0.7 pass, > 0.85 fail |
| PASS | Kills lost to someone else's last hit | 0.00  | <= 0.05 pass, > 0.15 fail |
| WARN | Stuck events per bot-hour | 3.00 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 1551.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 50 s; first quest turned in: median 8 s; level 2: median 141 s (100% of level-1 starters reached it)
- XP per hour: median 8240 (design pace 6400); 75% of XP from quests; 30 quests turned in, median 2.1 min each
- Rewards: median gap 10 s; median longest drought 107 s; 0% of bots had a drought over 5 min
- Combat: 47 fights, 96% won, median 4.5 s; 43 kills, 0 without credit (0 of them with no other player within 20 blocks); 0 deaths (0.0/h)
- Travel: 46 blocks per reward; money earned 1551 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 3.0 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| fight | 11.3% |
| gather | 3.0% |
| idle | 2.1% |
| loot | 0.3% |
| town | 4.9% |
| travel | 78.5% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 4 | 1.0 | 0 |
| 1 | 4 | 1.0 | 132 |
| 2 | 4 | 1.0 | 328 |
| 3 | 4 | 2.0 | 544 |
| 4 | 4 | 2.0 | 707 |
| 5 | 4 | 2.0 | 842 |
| 6 | 4 | 2.0 | 981 |
| 7 | 4 | 2.0 | 1101 |
| 8 | 4 | 2.0 | 1213 |
| 9 | 4 | 2.5 | 1268 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 4 | 0 | - |
| broken_tower | 4 | 4 | 4.6 |
| coal_for_the_watch | 2 | 1 | 0.7 |
| cold_hearth | 4 | 4 | 0.1 |
| eyes_in_the_dark | 4 | 0 | - |
| garricks_camp | 4 | 3 | 1.0 |
| iron_for_hollis | 1 | 0 | - |
| kindling | 4 | 2 | 1.2 |
| lay_of_the_land | 4 | 0 | - |
| lookouts_eye | 4 | 0 | - |
| meat_for_the_pot | 4 | 4 | 1.6 |
| north_watch | 4 | 4 | 3.6 |
| silk_for_sails | 1 | 0 | - |
| thin_the_herd | 4 | 4 | 2.9 |
| tusks_and_tempers | 3 | 0 | - |
| watchfire | 4 | 0 | - |
| webs_between_roots | 1 | 0 | - |
| wool_for_winter | 4 | 3 | 3.2 |
| word_for_rysa | 1 | 1 | 1.0 |

## Blockers (bots that could not progress an objective)

- eyes_in_the_dark: kill Shade: only at night: waiting for dark x2
- coal_for_the_watch: collect Coal: no source in reach x1
- watchfire: collect Coal: no source in reach x1

## Fights by creature and outcome

- Grazer: won x41
- Thornboar: won x3
- Grazer: lost (it fled home and healed) x2
- Websnare: won x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 18 | Quick Shot 56%, Barbed Arrow 39%, Recall 6% |
| Paladin | 6 | Judging Blow 100% |
| Rogue | 25 | Vicious Strike 72%, Finishing Cut 28% |
| Warrior | 6 | Heroic Blow 50%, Battle Cry 33%, Recall 17% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-32, -16]: 1 events
  - Botpjr01par at [-19, 0, -3] going to [-22, 0, -1] (repaths; following Botpjr00par)
- cell [16, -48]: 1 events
  - Botpjr03par at [26, 0, -37] going to [29, 0, -40] (repaths; following Botpjr02par)

## Notes

- grouped in a party of two x4

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| party | 4 | 8240 | 30 | 0.0 | 96% | 10 | 2% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 1 | 8792 | 8 | 0.0 | 92% | 8 | 2% |
| Paladin | 1 | 6901 | 6 | 0.0 | 100% | 12 | 2% |
| Rogue | 1 | 9740 | 8 | 0.0 | 94% | 16 | 2% |
| Warrior | 1 | 7689 | 8 | 0.0 | 100% | 8 | 3% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botpjr00par | Warrior | party | 10.0 | 1->2 | 7689 | 8/15 | 9 | 0 | 8 | 130 |
| Botpjr01par | Paladin | party | 10.0 | 1->2 | 6901 | 6/14 | 9 | 0 | 12 | 119 |
| Botpjr02par | Hunter | party | 10.0 | 1->3 | 8792 | 8/15 | 12 | 0 | 8 | 95 |
| Botpjr03par | Rogue | party | 10.0 | 1->3 | 9740 | 8/17 | 13 | 0 | 16 | 60 |
