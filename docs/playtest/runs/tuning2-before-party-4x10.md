# Hearthblock playtest: 4 bots, 10 min, brain party

4 bots on `local`, 40.0 bot-minutes (longest session 10.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 4 fail, 1 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 42.52 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 8.09 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 95.78 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 2.22 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.68  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 1.57 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 6.81 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 6.00 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.93  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 4.78 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.03  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.57  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 23.68 blocks | <= 60 pass, > 120 fail |
| WARN | Distinct abilities used per fighting bot | 1.75  | >= 2 pass, < 1.5 fail |
| FAIL | Share of casts that are the most used ability | 0.88  | <= 0.7 pass, > 0.85 fail |
| PASS | Kills lost to someone else's last hit | 0.00  | <= 0.05 pass, > 0.15 fail |
| FAIL | Stuck events per bot-hour | 6.00 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 2554.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 43 s; first quest turned in: median 8 s; level 2: median 96 s (100% of level-1 starters reached it)
- XP per hour: median 14178 (design pace 6400); 68% of XP from quests; 40 quests turned in, median 1.6 min each
- Rewards: median gap 7 s; median longest drought 79 s; 0% of bots had a drought over 5 min
- Combat: 94 fights, 93% won, median 4.8 s; 81 kills, 0 without credit (0 of them with no other player within 20 blocks); 4 deaths (6.0/h)
- Travel: 24 blocks per reward; money earned 2554 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 6.0 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| dead | 0.5% |
| fight | 25.2% |
| gather | 7.6% |
| idle | 2.1% |
| loot | 1.1% |
| rest | 1.3% |
| town | 4.9% |
| travel | 57.2% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 4 | 1.0 | 0 |
| 1 | 4 | 1.0 | 232 |
| 2 | 4 | 2.0 | 543 |
| 3 | 4 | 2.0 | 863 |
| 4 | 4 | 2.0 | 1211 |
| 5 | 4 | 2.5 | 1348 |
| 6 | 4 | 3.0 | 1698 |
| 7 | 4 | 3.0 | 1858 |
| 8 | 4 | 3.0 | 2020 |
| 9 | 4 | 3.0 | 2233 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 3 | 1 | 2.2 |
| coal_for_the_watch | 4 | 3 | 1.9 |
| cold_hearth | 4 | 4 | 0.1 |
| down_to_water | 3 | 3 | 1.1 |
| eyes_in_the_dark | 3 | 0 | - |
| garricks_camp | 3 | 2 | 1.0 |
| iron_for_hollis | 3 | 0 | - |
| kindling | 4 | 4 | 3.2 |
| meat_for_the_pot | 4 | 4 | 2.8 |
| north_watch | 4 | 3 | 1.1 |
| pincher_supper | 3 | 0 | - |
| report_to_holt | 3 | 3 | 0.1 |
| silk_for_sails | 2 | 0 | - |
| stolen_grain | 3 | 0 | - |
| thieves_in_fields | 3 | 2 | 2.8 |
| thin_the_herd | 4 | 4 | 2.6 |
| tusks_and_tempers | 2 | 0 | - |
| watchfire | 3 | 1 | 1.4 |
| windmill_breadcrumb | 3 | 3 | 0.9 |
| wool_for_winter | 4 | 3 | 0.5 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x3
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x3
- watchfire: collect Coal: no source in reach x1

## Deaths by cause

- You fell to a Ashcloak Bandit. x3
- You fell to a Ashcloak Cutthroat. x1

## Fights by creature and outcome

- Grazer: won x52
- Ashcloak Bandit: won x15
- Thornboar: won x12
- Mirepincher: won x6
- Ashcloak Bandit: lost (died) x3
- Ashcloak Bandit: lost (it fled home and healed) x2
- Gloomwisp: won x2
- Mirepincher: lost (no progress: it hardly took damage) x2

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 35 | Quick Shot 100% |
| Paladin | 16 | Judging Blow 94%, Holy Mend 6% |
| Rogue | 44 | Vicious Strike 75%, Finishing Cut 25% |
| Warrior | 18 | Heroic Blow 83%, Battle Cry 17% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-16, -16]: 1 events
  - Botyde01par at [-3, 0, -6] going to [-7, 1, -6] (repaths; following Botyde00par)
- cell [-16, 128]: 1 events
  - Botyde03par at [-14, -13, 134] going to [-14, -16, 134] (repaths; iron_for_hollis: gather Raw iron)
- cell [0, 48]: 1 events
  - Botyde03par at [2, -5, 48] going to [1, -5, 44] (repaths; following Botyde02par)
- cell [0, 112]: 1 events
  - Botyde01par at [13, -13, 125] going to [13, -16, 125] (repaths; coal_for_the_watch: gather Coal)

## Notes

- grouped in a party of two x4

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| party | 4 | 14178 | 40 | 6.0 | 93% | 7 | 3% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 1 | 15975 | 12 | 12.0 | 88% | 6 | 1% |
| Paladin | 1 | 9812 | 7 | 0.0 | 100% | 8 | 5% |
| Rogue | 1 | 14247 | 9 | 12.0 | 85% | 9 | 3% |
| Warrior | 1 | 14109 | 12 | 0.0 | 100% | 6 | 1% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botyde00par | Warrior | party | 10.0 | 1->3 | 14109 | 12/20 | 21 | 0 | 6 | 89 |
| Botyde01par | Paladin | party | 10.0 | 1->3 | 9812 | 7/14 | 19 | 0 | 8 | 78 |
| Botyde02par | Hunter | party | 10.0 | 1->3 | 15975 | 12/18 | 23 | 2 | 6 | 78 |
| Botyde03par | Rogue | party | 10.0 | 1->3 | 14247 | 9/13 | 18 | 2 | 9 | 80 |
