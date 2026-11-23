# Hearthblock playtest: 20 bots, 12 min, brain mix

20 bots on `127.0.0.1:7712`, 231.8 bot-minutes (longest session 12.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 2 fail, 2 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 47.04 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.78 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 201.93 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.32 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.64  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.75 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 9.57 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| WARN | Deaths per hour of play | 3.88 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.97  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 5.00 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.02  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.58  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 34.15 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.35  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.76  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.27  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 0.78 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 9274.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 47 s; first quest turned in: median 8 s; level 2: median 202 s (100% of level-1 starters reached it)
- XP per hour: median 8461 (design pace 6400); 64% of XP from quests; 134 quests turned in, median 2.8 min each
- Rewards: median gap 10 s; median longest drought 71 s; 0% of bots had a drought over 5 min
- Combat: 671 fights, 97% won, median 5.0 s; 366 kills, 132 without credit (10 of them with no other player within 20 blocks); 15 deaths (3.9/h)
- Travel: 34 blocks per reward; money earned 9274 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 0.8 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 1.3% |
| dead | 0.3% |
| fight | 30.2% |
| gather | 3.3% |
| idle | 1.4% |
| loot | 2.3% |
| rest | 2.0% |
| town | 1.6% |
| travel | 57.5% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 20 | 1.0 | 0 |
| 1 | 20 | 1.0 | 116 |
| 2 | 20 | 1.0 | 256 |
| 3 | 20 | 1.0 | 396 |
| 4 | 20 | 2.0 | 563 |
| 5 | 20 | 2.0 | 712 |
| 6 | 19 | 2.0 | 852 |
| 7 | 19 | 2.0 | 982 |
| 8 | 19 | 2.0 | 1098 |
| 9 | 19 | 3.0 | 1224 |
| 10 | 19 | 3.0 | 1351 |
| 11 | 18 | 3.0 | 1462 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 13 | 0 | - |
| broken_tower | 16 | 13 | 5.5 |
| coal_for_the_watch | 16 | 11 | 2.3 |
| cold_hearth | 20 | 20 | 0.1 |
| down_to_water | 1 | 0 | - |
| eyes_in_the_dark | 13 | 2 | 4.3 |
| garricks_camp | 13 | 1 | 1.0 |
| iron_for_hollis | 8 | 0 | - |
| kindling | 16 | 16 | 2.7 |
| lay_of_the_land | 13 | 3 | 4.1 |
| lookouts_eye | 13 | 2 | 1.0 |
| meat_for_the_pot | 16 | 16 | 2.6 |
| north_watch | 13 | 13 | 4.1 |
| silk_for_sails | 1 | 0 | - |
| stolen_grain | 1 | 0 | - |
| thieves_in_fields | 1 | 0 | - |
| thin_the_herd | 20 | 17 | 3.9 |
| tusks_and_tempers | 1 | 0 | - |
| watchfire | 13 | 7 | 0.4 |
| windmill_breadcrumb | 9 | 1 | 2.3 |
| wool_for_winter | 16 | 12 | 1.3 |
| word_for_rysa | 1 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x12
- iron_for_hollis: collect Raw iron: no source in reach x7
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x6
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x1
- kindling: collect Log: no source in reach x1
- thin_the_herd: goto failed: too many repaths x1
- watchfire: collect Coal: no source in reach x1

## Deaths by cause

- You fell to a Grazer. x8
- You fell to a Shade. x7

## Fights by creature and outcome

- Grazer: won x566
- Shade: won x69
- Thornboar: won x17
- Grazer: lost (died) x8
- Shade: lost (died) x7
- Grazer: lost (it fled home and healed) x2
- Thornboar: lost (it fled home and healed) x2

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 69 | Quick Shot 58%, Barbed Arrow 38%, Recall 4% |
| Mage | 198 | Fireball 62%, Fire Blast 36%, Scholar's Focus 1%, Recall 1% |
| Paladin | 112 | Judging Blow 90%, Rebuke 7%, Holy Mend 3% |
| Priest | 149 | Smite 76%, Withering Word 20%, Mend 2%, Recall 2% |
| Rogue | 56 | Vicious Strike 82%, Finishing Cut 14%, Recall 4% |
| Warrior | 54 | Heroic Blow 61%, Rending Cut 19%, Battle Cry 17%, Recall 4% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [32, -64]: 1 events
  - Botvxd19par at [39, 0, -57] going to [43, 0, -58] (repaths; following Botvxd18par)
- cell [48, -48]: 1 events
  - Botvxd09par at [53, 0, -35] going to [49, 0, -36] (repaths; following Botvxd08par)
- cell [240, 96]: 1 events
  - Botvxd02exp at [246, -6, 110] going to [240, -7, 110] (repaths; thin_the_herd: go to the Grazer camp)

## Notes

- grouped in a party of two x4
- casual session ended x2
- tried /invite Botvxd06com x1
- tried /invite Botvxd13com x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 2 | 6763 | 11 | 0.0 | 100% | 10 | 1% |
| completionist | 6 | 9077 | 56 | 2.5 | 98% | 8 | 1% |
| explorer | 2 | 6084 | 9 | 0.0 | 100% | 13 | 1% |
| grinder | 4 | 9189 | 8 | 13.8 | 96% | 8 | 2% |
| party | 4 | 8203 | 30 | 0.0 | 100% | 10 | 3% |
| social | 2 | 10095 | 20 | 2.5 | 96% | 9 | 1% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 3 | 8422 | 25 | 0.0 | 98% | 9 | 2% |
| Mage | 3 | 9788 | 7 | 12.3 | 95% | 7 | 2% |
| Paladin | 4 | 8400 | 21 | 6.3 | 98% | 10 | 2% |
| Priest | 3 | 10272 | 28 | 6.7 | 95% | 8 | 2% |
| Rogue | 3 | 7858 | 23 | 0.0 | 100% | 12 | 1% |
| Warrior | 4 | 8264 | 30 | 0.0 | 100% | 10 | 1% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botvxd00com | Warrior | completionist | 12.0 | 1->3 | 9556 | 9/16 | 14 | 0 | 8 | 74 |
| Botvxd01gri | Paladin | grinder | 12.0 | 1->3 | 8591 | 2/2 | 47 | 3 | 10 | 53 |
| Botvxd02exp | Hunter | explorer | 12.0 | 1->2 | 6491 | 5/8 | 9 | 0 | 10 | 74 |
| Botvxd03com | Rogue | completionist | 12.0 | 1->3 | 8597 | 10/18 | 11 | 0 | 13 | 108 |
| Botvxd04soc | Priest | social | 12.0 | 1->3 | 10053 | 9/15 | 13 | 1 | 10 | 78 |
| Botvxd05cas | Mage | casual | 5.3 | 1->2 | 5669 | 3/7 | 6 | 0 | 8 | 53 |
| Botvxd06com | Warrior | completionist | 12.0 | 1->3 | 8456 | 9/16 | 13 | 0 | 8 | 77 |
| Botvxd07gri | Paladin | grinder | 12.0 | 1->3 | 8466 | 2/2 | 46 | 2 | 10 | 59 |
| Botvxd08par | Hunter | party | 12.0 | 1->3 | 8422 | 9/16 | 10 | 0 | 9 | 97 |
| Botvxd09par | Rogue | party | 12.0 | 1->2 | 5486 | 5/13 | 5 | 0 | 10 | 105 |
| Botvxd10com | Priest | completionist | 12.0 | 1->3 | 10272 | 9/16 | 13 | 2 | 8 | 93 |
| Botvxd11gri | Mage | grinder | 12.0 | 1->3 | 10641 | 2/2 | 52 | 3 | 6 | 37 |
| Botvxd12exp | Warrior | explorer | 12.0 | 1->2 | 5676 | 4/7 | 4 | 0 | 17 | 105 |
| Botvxd13com | Paladin | completionist | 12.0 | 1->3 | 7817 | 9/16 | 13 | 0 | 17 | 73 |
| Botvxd14soc | Hunter | social | 12.0 | 1->3 | 10138 | 11/16 | 15 | 0 | 8 | 62 |
| Botvxd15cas | Rogue | casual | 10.5 | 1->3 | 7858 | 8/15 | 11 | 0 | 12 | 67 |
| Botvxd16com | Priest | completionist | 12.0 | 1->3 | 10828 | 10/16 | 11 | 1 | 8 | 63 |
| Botvxd17gri | Mage | grinder | 12.0 | 1->3 | 9788 | 2/2 | 44 | 3 | 7 | 69 |
| Botvxd18par | Warrior | party | 12.0 | 1->3 | 8072 | 8/14 | 18 | 0 | 11 | 61 |
| Botvxd19par | Paladin | party | 12.0 | 1->3 | 8333 | 8/17 | 11 | 0 | 10 | 58 |
