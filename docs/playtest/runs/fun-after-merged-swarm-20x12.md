# Hearthblock playtest: 20 bots, 12 min, brain mix

20 bots on `127.0.0.1:7710`, 233.1 bot-minutes (longest session 12.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 1 fail, 3 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 50.06 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.72 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 176.13 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.37 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.59  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.79 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 11.66 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| WARN | Deaths per hour of play | 3.60 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.98  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 5.37 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.02  | <= 0.15 pass, > 0.3 fail |
| WARN | Share of time travelling | 0.54  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 31.25 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.40  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.74  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.21  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 1.80 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 9455.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 50 s; first quest turned in: median 8 s; level 2: median 176 s (100% of level-1 starters reached it)
- XP per hour: median 8774 (design pace 6400); 59% of XP from quests; 132 quests turned in, median 2.8 min each
- Rewards: median gap 12 s; median longest drought 71 s; 0% of bots had a drought over 5 min
- Combat: 654 fights, 98% won, median 5.4 s; 422 kills, 111 without credit (7 of them with no other player within 20 blocks); 14 deaths (3.6/h)
- Travel: 31 blocks per reward; money earned 9455 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 1.8 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 2.3% |
| dead | 0.3% |
| fight | 32.4% |
| gather | 2.8% |
| idle | 1.3% |
| loot | 3.1% |
| rest | 2.1% |
| town | 1.6% |
| travel | 54.1% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 20 | 1.0 | 0 |
| 1 | 20 | 1.0 | 132 |
| 2 | 20 | 1.0 | 241 |
| 3 | 20 | 2.0 | 424 |
| 4 | 20 | 2.0 | 563 |
| 5 | 20 | 2.0 | 713 |
| 6 | 19 | 2.0 | 844 |
| 7 | 19 | 2.0 | 971 |
| 8 | 19 | 2.0 | 1088 |
| 9 | 19 | 2.0 | 1236 |
| 10 | 19 | 3.0 | 1366 |
| 11 | 19 | 3.0 | 1542 |
| 12 | 2 | 3.0 | 1745 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 13 | 0 | - |
| broken_tower | 16 | 13 | 6.0 |
| coal_for_the_watch | 16 | 4 | 4.4 |
| cold_hearth | 20 | 20 | 0.1 |
| dust_of_shadows | 1 | 1 | 0.0 |
| eyes_in_the_dark | 13 | 2 | 2.0 |
| garricks_camp | 13 | 3 | 1.0 |
| iron_for_hollis | 3 | 0 | - |
| kindling | 16 | 16 | 2.5 |
| lay_of_the_land | 13 | 3 | 3.7 |
| lookouts_eye | 13 | 4 | 0.7 |
| meat_for_the_pot | 16 | 16 | 2.7 |
| north_watch | 15 | 13 | 3.8 |
| silk_for_sails | 2 | 0 | - |
| thin_the_herd | 20 | 19 | 4.1 |
| tusks_and_tempers | 3 | 0 | - |
| watchfire | 13 | 4 | 0.2 |
| windmill_breadcrumb | 10 | 0 | - |
| wool_for_winter | 16 | 14 | 2.0 |
| word_for_rysa | 2 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x14
- watchfire: collect Coal: no source in reach x9
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x8
- iron_for_hollis: collect Raw iron: no source in reach x3
- eyes_in_the_dark: goto failed: too many repaths x1

## Deaths by cause

- You fell to a Grazer. x7
- You fell to a Shade. x7

## Fights by creature and outcome

- Grazer: won x521
- Shade: won x86
- Thornboar: won x31
- Grazer: lost (died) x7
- Shade: lost (died) x7
- Grazer: lost (it fled home and healed) x1
- Thornboar: lost (it fled home and healed) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 82 | Quick Shot 61%, Barbed Arrow 34%, Recall 5% |
| Mage | 249 | Fireball 65%, Fire Blast 33%, Scholar's Focus 1%, Recall 1% |
| Paladin | 115 | Judging Blow 89%, Holy Mend 6%, Rebuke 4%, Recall 1% |
| Priest | 165 | Smite 75%, Withering Word 23%, Recall 2% |
| Rogue | 68 | Vicious Strike 82%, Finishing Cut 16%, Recall 1% |
| Warrior | 71 | Heroic Blow 58%, Rending Cut 27%, Battle Cry 13%, Recall 3% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-64, -16]: 1 events
  - Botbff19par at [-56, 0, -8] going to [-60, 0, -9] (repaths; following Botbff18par)
- cell [-32, -48]: 1 events
  - Botbff09par at [-28, 0, -43] going to [-27, 0, -39] (repaths; following Botbff08par)
- cell [-16, -176]: 1 events
  - Botbff19par at [-8, -6, -174] going to [-11, -6, -177] (repaths; following Botbff18par)
- cell [-16, -144]: 1 events
  - Botbff19par at [-2, -6, -129] going to [-1, -6, -133] (repaths; following Botbff18par)
- cell [0, 16]: 1 events
  - Botbff19par at [3, 0, 20] going to [4, 0, 16] (repaths; following Botbff18par)
- cell [0, 128]: 1 events
  - Botbff19par at [3, -8, 131] going to [3, -8, 135] (repaths; following Botbff18par)
- cell [16, 32]: 1 events
  - Botbff04soc at [29, 0, 36] going to [33, 0, 36] (repaths; eyes_in_the_dark: look for Shade in the dark)

## Notes

- grouped in a party of two x4
- casual session ended x1
- tried /invite Botbff03com x1
- tried /invite Botbff13com x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 2 | 7676 | 13 | 0.0 | 100% | 10 | 1% |
| completionist | 6 | 9123 | 53 | 0.0 | 99% | 11 | 1% |
| explorer | 2 | 6401 | 11 | 2.5 | 95% | 14 | 2% |
| grinder | 4 | 9943 | 8 | 13.8 | 96% | 7 | 2% |
| party | 4 | 7364 | 31 | 2.5 | 97% | 15 | 2% |
| social | 2 | 8714 | 16 | 0.0 | 100% | 11 | 2% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 3 | 7732 | 24 | 1.7 | 97% | 13 | 1% |
| Mage | 3 | 9981 | 13 | 10.0 | 97% | 6 | 2% |
| Paladin | 4 | 8801 | 20 | 6.3 | 97% | 11 | 2% |
| Priest | 3 | 8780 | 26 | 0.0 | 99% | 9 | 1% |
| Rogue | 3 | 6996 | 18 | 0.0 | 100% | 14 | 2% |
| Warrior | 4 | 8447 | 31 | 2.5 | 98% | 15 | 1% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botbff00com | Warrior | completionist | 12.0 | 1->3 | 9757 | 9/16 | 14 | 0 | 10 | 71 |
| Botbff01gri | Paladin | grinder | 12.0 | 1->3 | 8757 | 2/2 | 49 | 1 | 9 | 58 |
| Botbff02exp | Hunter | explorer | 12.0 | 1->2 | 5751 | 6/9 | 11 | 1 | 11 | 61 |
| Botbff03com | Rogue | completionist | 12.0 | 1->3 | 9401 | 7/15 | 21 | 0 | 14 | 82 |
| Botbff04soc | Priest | social | 12.0 | 1->3 | 8661 | 7/15 | 14 | 0 | 9 | 75 |
| Botbff05cas | Mage | casual | 12.0 | 1->3 | 9146 | 9/16 | 13 | 0 | 7 | 104 |
| Botbff06com | Warrior | completionist | 12.0 | 1->3 | 7951 | 9/16 | 15 | 0 | 13 | 100 |
| Botbff07gri | Paladin | grinder | 12.0 | 1->3 | 9904 | 2/2 | 57 | 4 | 8 | 44 |
| Botbff08par | Hunter | party | 12.0 | 1->3 | 7732 | 9/14 | 10 | 0 | 13 | 92 |
| Botbff09par | Rogue | party | 12.0 | 1->3 | 6996 | 7/16 | 4 | 0 | 14 | 79 |
| Botbff10com | Priest | completionist | 12.0 | 1->3 | 8780 | 9/15 | 14 | 0 | 9 | 72 |
| Botbff11gri | Mage | grinder | 12.0 | 1->3 | 9981 | 2/2 | 58 | 0 | 4 | 46 |
| Botbff12exp | Warrior | explorer | 12.0 | 1->3 | 7052 | 5/9 | 13 | 0 | 17 | 74 |
| Botbff13com | Paladin | completionist | 12.0 | 1->3 | 8844 | 9/16 | 12 | 0 | 13 | 58 |
| Botbff14soc | Hunter | social | 12.0 | 1->3 | 8767 | 9/15 | 17 | 0 | 13 | 58 |
| Botbff15cas | Rogue | casual | 5.1 | 1->2 | 6206 | 4/7 | 5 | 0 | 12 | 60 |
| Botbff16com | Priest | completionist | 12.0 | 1->3 | 9412 | 10/15 | 15 | 0 | 7 | 72 |
| Botbff17gri | Mage | grinder | 12.0 | 1->3 | 11363 | 2/2 | 56 | 6 | 6 | 34 |
| Botbff18par | Warrior | party | 12.0 | 1->3 | 8943 | 8/18 | 15 | 2 | 17 | 61 |
| Botbff19par | Paladin | party | 12.0 | 1->2 | 6412 | 7/14 | 9 | 0 | 19 | 82 |
