# Hearthblock playtest: 20 bots, 12 min, brain mix

20 bots on `127.0.0.1:7711`, 229.5 bot-minutes (longest session 12.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 2 fail, 2 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 54.62 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.76 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 173.77 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.38 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.60  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.67 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 8.69 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 6.54 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.96  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 6.09 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.04  | <= 0.15 pass, > 0.3 fail |
| WARN | Share of time travelling | 0.53  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 31.66 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.25  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.76  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.21  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 1.83 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 9380.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 55 s; first quest turned in: median 8 s; level 2: median 174 s (100% of level-1 starters reached it)
- XP per hour: median 8827 (design pace 6400); 60% of XP from quests; 128 quests turned in, median 2.7 min each
- Rewards: median gap 9 s; median longest drought 73 s; 0% of bots had a drought over 5 min
- Combat: 605 fights, 96% won, median 6.1 s; 380 kills, 102 without credit (6 of them with no other player within 20 blocks); 25 deaths (6.5/h)
- Travel: 32 blocks per reward; money earned 9380 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 1.8 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 2.0% |
| dead | 0.6% |
| fight | 31.8% |
| gather | 3.0% |
| idle | 3.4% |
| loot | 2.2% |
| rest | 1.9% |
| town | 1.9% |
| travel | 53.1% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 20 | 1.0 | 0 |
| 1 | 20 | 1.0 | 103 |
| 2 | 20 | 1.0 | 225 |
| 3 | 20 | 2.0 | 392 |
| 4 | 20 | 2.0 | 564 |
| 5 | 20 | 2.0 | 705 |
| 6 | 19 | 2.0 | 854 |
| 7 | 19 | 2.0 | 988 |
| 8 | 19 | 2.0 | 1072 |
| 9 | 18 | 2.5 | 1231 |
| 10 | 18 | 3.0 | 1396 |
| 11 | 18 | 3.0 | 1530 |
| 12 | 1 | 3.0 | 1769 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 12 | 0 | - |
| broken_tower | 16 | 14 | 5.3 |
| coal_for_the_watch | 14 | 6 | 4.7 |
| cold_hearth | 20 | 20 | 0.1 |
| down_to_water | 3 | 0 | - |
| eyes_in_the_dark | 12 | 1 | 3.5 |
| garricks_camp | 12 | 1 | 1.0 |
| iron_for_hollis | 6 | 0 | - |
| kindling | 16 | 15 | 2.0 |
| lay_of_the_land | 14 | 6 | 3.3 |
| lookouts_eye | 12 | 3 | 0.9 |
| meat_for_the_pot | 16 | 15 | 2.5 |
| north_watch | 13 | 12 | 3.7 |
| silk_for_sails | 1 | 0 | - |
| stolen_grain | 3 | 0 | - |
| thieves_in_fields | 3 | 0 | - |
| thin_the_herd | 20 | 17 | 4.0 |
| tusks_and_tempers | 1 | 0 | - |
| watchfire | 12 | 3 | 0.4 |
| windmill_breadcrumb | 11 | 3 | 2.2 |
| wool_for_winter | 15 | 12 | 1.4 |
| word_for_rysa | 1 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x13
- watchfire: collect Coal: no source in reach x8
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x6
- iron_for_hollis: collect Raw iron: no source in reach x4
- iron_for_hollis: collect Raw iron: Requires mining 5 (have 4) x2
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x2
- kindling: collect Log: no source in reach x2

## Deaths by cause

- You fell to a Grazer. x19
- You fell to a Shade. x3
- You fell to a Thornboar. x3

## Fights by creature and outcome

- Grazer: won x463
- Shade: won x87
- Thornboar: won x30
- Grazer: lost (died) x19
- Shade: lost (died) x3
- Thornboar: lost (died) x3

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 82 | Quick Shot 61%, Barbed Arrow 34%, Recall 5% |
| Mage | 193 | Fireball 67%, Fire Blast 32%, Scholar's Focus 1% |
| Paladin | 105 | Judging Blow 83%, Rebuke 10%, Holy Mend 7%, Recall 1% |
| Priest | 190 | Smite 76%, Withering Word 21%, Recall 2%, Mend 1% |
| Rogue | 67 | Vicious Strike 76%, Finishing Cut 19%, Recall 4% |
| Warrior | 69 | Heroic Blow 58%, Rending Cut 20%, Battle Cry 12%, Recall 10% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-272, 112]: 1 events
  - Botenq02exp at [-258, -5, 120] going to [-270, -4, 120] (repaths; exploring Westfold Farm)
- cell [-96, -112]: 1 events
  - Botenq19par at [-85, -6, -97] going to [-83, -6, -94] (repaths; following Botenq18par)
- cell [-96, 16]: 1 events
  - Botenq02exp at [-90, -4, 18] going to [-96, -1, 26] (repaths; exploring West Gate Rocks)
- cell [-32, -16]: 1 events
  - Botenq09par at [-24, 0, -1] going to [-28, 0, -1] (repaths; following Botenq08par)
- cell [-16, -128]: 1 events
  - Botenq09par at [-3, -6, -126] going to [-3, -6, -130] (repaths; following Botenq08par)
- cell [0, -256]: 1 events
  - Botenq02exp at [0, -4, -246] going to [0, 0, -255] (repaths; exploring The North Watch)
- cell [0, 160]: 1 events
  - Botenq02exp at [10, -9, 168] going to [10, 0, 180] (repaths; exploring The Broken Tower)

## Notes

- grouped in a party of two x4
- casual session ended x2
- tried /invite Botenq00com x1
- tried /invite Botenq13com x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 2 | 6519 | 9 | 0.0 | 100% | 11 | 1% |
| completionist | 6 | 9337 | 55 | 1.7 | 99% | 9 | 2% |
| explorer | 2 | 7583 | 11 | 0.0 | 100% | 14 | 2% |
| grinder | 4 | 9954 | 8 | 28.8 | 92% | 7 | 3% |
| party | 4 | 6822 | 27 | 0.0 | 100% | 12 | 10% |
| social | 2 | 9095 | 18 | 0.0 | 100% | 10 | 2% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 3 | 8818 | 26 | 0.0 | 100% | 12 | 2% |
| Mage | 3 | 9621 | 7 | 24.7 | 92% | 6 | 3% |
| Paladin | 4 | 8670 | 16 | 13.8 | 94% | 10 | 9% |
| Priest | 3 | 9372 | 25 | 1.7 | 99% | 8 | 2% |
| Rogue | 3 | 7117 | 24 | 0.0 | 100% | 10 | 4% |
| Warrior | 4 | 7902 | 30 | 1.2 | 99% | 14 | 2% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botenq00com | Warrior | completionist | 12.0 | 1->3 | 9796 | 10/18 | 17 | 1 | 8 | 79 |
| Botenq01gri | Paladin | grinder | 12.0 | 1->3 | 8520 | 2/2 | 46 | 5 | 12 | 62 |
| Botenq02exp | Hunter | explorer | 12.0 | 1->3 | 9508 | 8/15 | 13 | 0 | 12 | 71 |
| Botenq03com | Rogue | completionist | 12.0 | 1->3 | 10906 | 11/16 | 16 | 0 | 9 | 55 |
| Botenq04soc | Priest | social | 12.0 | 1->3 | 9372 | 9/16 | 15 | 0 | 8 | 54 |
| Botenq05cas | Mage | casual | 5.1 | 1->2 | 5921 | 3/6 | 6 | 0 | 6 | 80 |
| Botenq06com | Warrior | completionist | 12.0 | 1->3 | 8834 | 9/15 | 16 | 0 | 14 | 51 |
| Botenq07gri | Paladin | grinder | 12.0 | 1->3 | 11102 | 2/2 | 53 | 6 | 8 | 59 |
| Botenq08par | Hunter | party | 12.0 | 1->3 | 8272 | 9/16 | 12 | 0 | 8 | 60 |
| Botenq09par | Rogue | party | 12.0 | 1->3 | 6674 | 7/17 | 5 | 0 | 10 | 98 |
| Botenq10com | Priest | completionist | 12.0 | 1->3 | 9440 | 8/15 | 13 | 0 | 7 | 73 |
| Botenq11gri | Mage | grinder | 12.0 | 1->3 | 9621 | 2/2 | 53 | 6 | 6 | 72 |
| Botenq12exp | Warrior | explorer | 12.0 | 1->2 | 5658 | 3/7 | 7 | 0 | 15 | 83 |
| Botenq13com | Paladin | completionist | 12.0 | 1->3 | 8821 | 9/18 | 14 | 0 | 9 | 106 |
| Botenq14soc | Hunter | social | 12.0 | 1->3 | 8818 | 9/19 | 14 | 0 | 12 | 73 |
| Botenq15cas | Rogue | casual | 8.5 | 1->2 | 7117 | 6/13 | 9 | 0 | 16 | 81 |
| Botenq16com | Priest | completionist | 12.0 | 1->3 | 9235 | 8/16 | 13 | 1 | 9 | 79 |
| Botenq17gri | Mage | grinder | 12.0 | 1->3 | 10287 | 2/2 | 45 | 6 | 6 | 56 |
| Botenq18par | Warrior | party | 12.0 | 1->3 | 6971 | 8/11 | 8 | 0 | 14 | 143 |
| Botenq19par | Paladin | party | 12.0 | 1->2 | 3276 | 3/7 | 5 | 0 | 19 | 97 |
