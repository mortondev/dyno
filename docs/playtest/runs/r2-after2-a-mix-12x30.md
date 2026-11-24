# Hearthblock playtest: 12 bots, 30 min, brain mix

12 bots on `127.0.0.1:7710`, 335.0 bot-minutes (longest session 30.0 min). Thresholds: docs/playtest/fun-metrics.md.

Machine: load average 8.9 at the start, 10.3 mean, 16.3 at most, on 16 cores.

## Fun checks: 1 fail, 3 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 70.55 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.11 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 143.63 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.11 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.58  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.91 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 9.93 s | <= 60 pass, > 90 fail |
| WARN | Share of bots with a reward drought over 5 minutes | 0.25  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 4.66 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.95  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 7.07 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.04  | <= 0.15 pass, > 0.3 fail |
| WARN | Share of time travelling | 0.52  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 37.23 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.92  | >= 2 pass, < 1.5 fail |
| PASS | Share of casts that are the most used ability | 0.67  | <= 0.7 pass, > 0.85 fail |
| PASS | Kills lost to someone else's last hit | 0.04  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 0.90 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 189.05 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 71 s; first quest turned in: median 7 s; level 2: median 144 s (100% of level-1 starters reached it)
- XP per hour: median 7128 (design pace 6400); 58% of XP from quests; 129 quests turned in, median 2.9 min each
- Rewards: median gap 10 s; median longest drought 149 s; 25% of bots had a drought over 5 min
- Combat: 684 fights, 95% won, median 7.1 s; 570 kills, 22 without credit (5 of them with no other player within 20 blocks); 26 deaths (4.7/h)
- Travel: 37 blocks per reward; money earned 11343 c, spent 60 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 0.9 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 0.1% |
| dead | 0.4% |
| fight | 28.9% |
| gather | 1.6% |
| idle | 2.6% |
| loot | 1.9% |
| rest | 1.3% |
| town | 10.5% |
| travel | 52.2% |
| wait_spawn | 0.5% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 12 | 1.0 | 0 |
| 2 | 12 | 1.0 | 318 |
| 4 | 12 | 2.0 | 611 |
| 6 | 11 | 2.0 | 902 |
| 8 | 11 | 2.0 | 1176 |
| 10 | 11 | 3.0 | 1510 |
| 12 | 11 | 3.0 | 1869 |
| 14 | 11 | 3.0 | 2195 |
| 16 | 11 | 4.0 | 2471 |
| 18 | 11 | 4.0 | 2673 |
| 20 | 11 | 4.0 | 2858 |
| 22 | 11 | 4.0 | 2993 |
| 24 | 11 | 4.0 | 3130 |
| 26 | 11 | 4.0 | 3283 |
| 28 | 11 | 4.0 | 3467 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 8 | 7 | 3.0 |
| broken_tower | 9 | 6 | 6.6 |
| coal_for_the_watch | 9 | 8 | 5.0 |
| cold_hearth | 12 | 12 | 0.1 |
| down_to_water | 3 | 0 | - |
| dust_of_shadows | 3 | 3 | 14.3 |
| eyes_in_the_dark | 8 | 3 | 4.7 |
| garricks_camp | 8 | 7 | 6.0 |
| into_barrowwood | 2 | 0 | - |
| iron_for_hollis | 8 | 2 | 2.5 |
| kindling | 9 | 9 | 0.6 |
| know_the_vale | 7 | 0 | - |
| lay_of_the_land | 6 | 6 | 1.8 |
| lookouts_eye | 8 | 7 | 1.6 |
| meat_for_the_pot | 9 | 9 | 2.4 |
| north_watch | 9 | 8 | 6.0 |
| old_tusk | 7 | 0 | - |
| silk_for_sails | 7 | 4 | 8.1 |
| stolen_grain | 3 | 0 | - |
| thieves_in_fields | 3 | 0 | - |
| thin_the_herd | 12 | 12 | 3.4 |
| tusks_and_tempers | 7 | 2 | 6.8 |
| watchfire | 8 | 6 | 1.4 |
| webs_between_roots | 5 | 3 | 8.5 |
| windmill_breadcrumb | 7 | 3 | 7.0 |
| wool_for_winter | 9 | 7 | 3.5 |
| word_for_rysa | 7 | 5 | 1.5 |
| word_from_east | 2 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x7
- old_tusk: kill Old Tusk: creature far above our level x7
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x6
- iron_for_hollis: collect Raw iron: no source in reach x6
- watchfire: collect Coal: no source in reach x5
- dust_of_shadows: collect Shade dust: no source in reach x2
- dust_of_shadows: kill Shade: only at night: waiting for dark x2
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x2
- iron_for_hollis: collect Raw iron: Requires mining 5 (have 4) x1

## Deaths by cause

- You fell to a Grazer. x14
- You fell to a Thornboar. x4
- You fell to a Websnare. x4
- You fell to a Ashcloak Thief. x2
- You fell to a Shade. x2

## Fights by creature and outcome

- Grazer: won x476
- Thornboar: won x62
- Shade: won x56
- Websnare: won x48
- Grazer: lost (died) x14
- Ashcloak Thief: won x7
- Thornboar: lost (it fled home and healed) x5
- Thornboar: lost (died) x4
- Websnare: lost (died) x4
- Grazer: lost (it fled home and healed) x3
- Ashcloak Thief: lost (died) x2
- Shade: lost (died) x2
- Shade: lost (could not reach or hurt it) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 103 | Quick Shot 57%, Barbed Arrow 38%, Recall 5% |
| Mage | 193 | Fireball 59%, Fire Blast 40%, Scholar's Focus 1% |
| Paladin | 175 | Judging Blow 79%, Rebuke 14%, Holy Mend 5%, Valour 2% |
| Priest | 267 | Smite 69%, Withering Word 25%, Mend 4%, Recall 1% |
| Rogue | 92 | Vicious Strike 78%, Finishing Cut 21%, Recall 1% |
| Warrior | 109 | Heroic Blow 50%, Rending Cut 30%, Battle Cry 10%, Onrush 7%, Recall 3% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-384, -16]: 1 events
  - Botyhm02exp at [-372, -3, -1] going to [-384, 0, -1] (repaths; exploring Bluebell Glade)
- cell [-192, -80]: 1 events
  - Botyhm07gri at [-190, -12, -76] going to [-190, -12, -70] (repaths; going to grind)
- cell [-160, 32]: 1 events
  - Botyhm07gri at [-150, -8, 34] going to [-150, -8, 40] (repaths; going to grind)
- cell [-32, -32]: 1 events
  - Botyhm09par at [-17, 0, -20] going to [-21, 0, -20] (repaths; following Botyhm08par)
- cell [624, 32]: 1 events
  - Botyhm02exp at [628, -14, 45] going to [640, -33, 45] (repaths; exploring Miner's Rest)

## Notes

- grouped in a party of two x2
- casual session ended x1
- tried /invite Botyhm03com x1

## Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)

- Grazer lv1 at -177,-11,-81 (ground -13) x2
- Grazer lv1 at -201,-12,-47 (ground -14) x2
- Grazer lv1 at -86,-6,-100 (ground -8) x2
- Grazer lv1 at 25,-10,-162 (ground -12) x2
- Grazer lv2 at -97,-6,-81 (ground -8) x2
- Grazer lv2 at 140,-9,19 (ground -11) x2
- Thornboar lv2 at -137,-6,42 (ground -8) x2
- Thornboar lv2 at 43,-12,-218 (ground -14) x2
- Ashcloak Thief lv3 at 157,-15,404 (ground -17) x1
- Grazer lv1 at -113,-8,-72 (ground -10) x1
- Grazer lv1 at -164,-10,-27 (ground -12) x1
- Grazer lv1 at -182,-11,-64 (ground -13) x1
- Grazer lv1 at -202,-12,-60 (ground -14) x1
- Grazer lv1 at -96,-8,-68 (ground -10) x1
- Grazer lv1 at -97,-6,-81 (ground -8) x1
- Grazer lv1 at -98,-7,-76 (ground -9) x1
- Grazer lv1 at 110,-8,9 (ground -10) x1
- Grazer lv2 at -137,-9,-16 (ground -11) x1
- Grazer lv2 at -79,-16,212 (ground -18) x1
- Grazer lv2 at -81,-20,308 (ground -22) x1
- Grazer lv2 at -90,-21,303 (ground -23) x1
- Grazer lv2 at 114,-8,12 (ground -10) x1
- Grazer lv2 at 137,-12,-252 (ground -14) x1
- Grazer lv2 at 239,-8,115 (ground -10) x1
- Grazer lv2 at 46,-5,-129 (ground -7) x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 1 | 8562 | 4 | 0.0 | 100% | 6 | 0% |
| completionist | 4 | 9349 | 68 | 1.5 | 98% | 10 | 2% |
| explorer | 1 | 6424 | 12 | 4.0 | 88% | 13 | 1% |
| grinder | 3 | 7049 | 6 | 12.7 | 95% | 9 | 2% |
| party | 2 | 5243 | 21 | 0.0 | 87% | 9 | 11% |
| social | 1 | 8843 | 18 | 4.0 | 96% | 8 | 1% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 2 | 6397 | 25 | 2.0 | 86% | 12 | 10% |
| Mage | 2 | 7734 | 6 | 5.1 | 97% | 6 | 1% |
| Paladin | 2 | 7128 | 4 | 16.0 | 93% | 10 | 2% |
| Priest | 2 | 9437 | 36 | 2.0 | 98% | 9 | 2% |
| Rogue | 2 | 6527 | 25 | 1.0 | 97% | 9 | 2% |
| Warrior | 2 | 7915 | 33 | 2.0 | 97% | 11 | 2% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botyhm00com | Warrior | completionist | 30.0 | 1->5 | 9758 | 19/27 | 36 | 2 | 13 | 167 |
| Botyhm01gri | Paladin | grinder | 30.0 | 1->4 | 7049 | 2/2 | 114 | 10 | 10 | 77 |
| Botyhm02exp | Hunter | explorer | 30.0 | 1->4 | 6424 | 12/23 | 23 | 2 | 13 | 108 |
| Botyhm03com | Rogue | completionist | 30.0 | 1->4 | 8939 | 17/23 | 34 | 1 | 11 | 102 |
| Botyhm04soc | Priest | social | 30.0 | 1->4 | 8843 | 18/26 | 34 | 2 | 8 | 130 |
| Botyhm05cas | Mage | casual | 5.1 | 1->2 | 8562 | 4/8 | 9 | 0 | 6 | 36 |
| Botyhm06com | Warrior | completionist | 30.0 | 1->4 | 6072 | 14/19 | 20 | 0 | 10 | 948 |
| Botyhm07gri | Paladin | grinder | 30.0 | 1->4 | 7206 | 2/2 | 107 | 6 | 9 | 306 |
| Botyhm08par | Hunter | party | 30.0 | 1->4 | 6370 | 13/22 | 23 | 0 | 12 | 274 |
| Botyhm09par | Rogue | party | 30.0 | 1->3 | 4116 | 8/18 | 8 | 0 | 7 | 942 |
| Botyhm10com | Priest | completionist | 30.0 | 1->5 | 10031 | 18/23 | 44 | 0 | 10 | 107 |
| Botyhm11gri | Mage | grinder | 30.0 | 1->4 | 6906 | 2/2 | 118 | 3 | 5 | 177 |
