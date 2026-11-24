# Hearthblock playtest: 20 bots, 12 min, brain mix

20 bots on `127.0.0.1:7710`, 233.6 bot-minutes (longest session 12.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 2 fail, 3 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 54.93 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.77 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 145.99 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.63 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.57  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.81 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 10.03 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 4.62 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.97  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 5.76 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.02  | <= 0.15 pass, > 0.3 fail |
| WARN | Share of time travelling | 0.55  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 33.23 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.45  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.74  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.29  | <= 0.05 pass, > 0.15 fail |
| WARN | Stuck events per bot-hour | 2.57 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 8275.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 55 s; first quest turned in: median 8 s; level 2: median 146 s (100% of level-1 starters reached it)
- XP per hour: median 10445 (design pace 6400); 57% of XP from quests; 131 quests turned in, median 2.8 min each
- Rewards: median gap 10 s; median longest drought 75 s; 0% of bots had a drought over 5 min
- Combat: 637 fights, 97% won, median 5.8 s; 378 kills, 152 without credit (9 of them with no other player within 20 blocks); 18 deaths (4.6/h)
- Travel: 33 blocks per reward; money earned 8275 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 2.6 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 3.1% |
| dead | 0.4% |
| fight | 31.4% |
| gather | 2.3% |
| idle | 1.9% |
| loot | 2.5% |
| rest | 2.0% |
| town | 1.7% |
| travel | 54.7% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 20 | 1.0 | 0 |
| 1 | 20 | 1.0 | 154 |
| 2 | 20 | 1.0 | 315 |
| 3 | 20 | 2.0 | 485 |
| 4 | 20 | 2.0 | 712 |
| 5 | 20 | 2.0 | 928 |
| 6 | 20 | 2.0 | 1143 |
| 7 | 20 | 2.5 | 1277 |
| 8 | 19 | 3.0 | 1397 |
| 9 | 19 | 3.0 | 1525 |
| 10 | 18 | 3.0 | 1723 |
| 11 | 18 | 3.0 | 1864 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 12 | 0 | - |
| broken_tower | 16 | 16 | 5.4 |
| coal_for_the_watch | 16 | 4 | 3.9 |
| cold_hearth | 20 | 20 | 0.1 |
| down_to_water | 4 | 0 | - |
| dust_of_shadows | 1 | 1 | 0.0 |
| eyes_in_the_dark | 12 | 2 | 4.0 |
| garricks_camp | 12 | 0 | - |
| iron_for_hollis | 4 | 0 | - |
| kindling | 16 | 16 | 2.1 |
| know_the_vale | 1 | 0 | - |
| lay_of_the_land | 16 | 5 | 4.3 |
| lookouts_eye | 12 | 1 | 0.7 |
| meat_for_the_pot | 16 | 16 | 3.3 |
| north_watch | 15 | 12 | 4.5 |
| stolen_grain | 4 | 0 | - |
| thieves_in_fields | 4 | 0 | - |
| thin_the_herd | 20 | 19 | 3.9 |
| watchfire | 12 | 1 | 0.4 |
| windmill_breadcrumb | 12 | 4 | 2.2 |
| wool_for_winter | 16 | 14 | 1.3 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x15
- watchfire: collect Coal: no source in reach x9
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x7
- thieves_in_fields: goto failed: too many repaths x4
- iron_for_hollis: collect Raw iron: no source in reach x3
- boar_trouble: goto failed: too many repaths x1
- iron_for_hollis: collect Raw iron: Requires mining 5 (have 4) x1
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x1

## Deaths by cause

- You fell to a Shade. x8
- You fell to a Grazer. x7
- You fell to a Thornboar. x3

## Fights by creature and outcome

- Grazer: won x503
- Shade: won x83
- Thornboar: won x25
- Shade: lost (died) x8
- Grazer: lost (died) x7
- Ashcloak Bandit: won x4
- Thornboar: lost (it fled home and healed) x4
- Thornboar: lost (died) x3

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 71 | Quick Shot 58%, Barbed Arrow 37%, Recall 6% |
| Mage | 233 | Fireball 64%, Fire Blast 35%, Scholar's Focus 1%, Recall 0% |
| Paladin | 120 | Judging Blow 86%, Rebuke 9%, Holy Mend 3%, Recall 2% |
| Priest | 187 | Smite 73%, Withering Word 25%, Recall 2%, Mend 1% |
| Rogue | 77 | Vicious Strike 79%, Finishing Cut 17%, Recall 4% |
| Warrior | 74 | Heroic Blow 58%, Rending Cut 22%, Battle Cry 12%, Recall 8% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [224, 480]: 4 events
  - Botrtw00com at [224, -9, 495] going to [230, -6, 495] (repaths; thieves_in_fields: go to the Ashcloak Bandit camp)
  - Botrtw03com at [224, -9, 495] going to [230, -6, 495] (repaths; thieves_in_fields: go to the Ashcloak Bandit camp)
  - Botrtw06com at [224, -9, 495] going to [230, -6, 495] (repaths; thieves_in_fields: go to the Ashcloak Bandit camp)
  - Botrtw14soc at [224, -9, 495] going to [230, -6, 495] (repaths; thieves_in_fields: go to the Ashcloak Bandit camp)
- cell [-176, -128]: 1 events
  - Botrtw09par at [-162, -7, -126] going to [-160, -7, -129] (repaths; following Botrtw08par)
- cell [-160, 32]: 1 events
  - Botrtw00com at [-150, -5, 34] going to [-150, -4, 40] (repaths; boar_trouble: go to the Thornboar camp)
- cell [-64, -16]: 1 events
  - Botrtw19par at [-53, 0, -5] going to [-49, 0, -5] (repaths; following Botrtw18par)
- cell [0, -160]: 1 events
  - Botrtw09par at [7, -6, -154] going to [6, -6, -158] (repaths; following Botrtw08par)
- cell [0, 160]: 1 events
  - Botrtw12exp at [10, -9, 168] going to [10, 0, 180] (repaths; exploring The Broken Tower)
- cell [48, -32]: 1 events
  - Botrtw09par at [48, 0, -21] going to [52, 0, -22] (repaths; following Botrtw08par)

## Notes

- grouped in a party of two x4
- casual session ended x2
- tried /invite Botrtw00com x1
- tried /invite Botrtw13com x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 2 | 8414 | 12 | 0.0 | 100% | 12 | 2% |
| completionist | 6 | 10457 | 51 | 1.7 | 99% | 10 | 2% |
| explorer | 2 | 9271 | 12 | 5.0 | 90% | 15 | 2% |
| grinder | 4 | 11723 | 8 | 15.0 | 95% | 8 | 2% |
| party | 4 | 9185 | 30 | 0.0 | 97% | 17 | 3% |
| social | 2 | 11386 | 18 | 5.0 | 96% | 11 | 2% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 3 | 11063 | 23 | 6.7 | 92% | 10 | 2% |
| Mage | 3 | 12098 | 10 | 12.4 | 96% | 6 | 2% |
| Paladin | 4 | 10611 | 22 | 6.3 | 96% | 10 | 3% |
| Priest | 3 | 10897 | 24 | 0.0 | 100% | 11 | 2% |
| Rogue | 3 | 9094 | 20 | 1.9 | 98% | 18 | 3% |
| Warrior | 4 | 10409 | 32 | 1.3 | 97% | 13 | 2% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botrtw00com | Warrior | completionist | 12.0 | 1->3 | 10463 | 8/18 | 15 | 1 | 9 | 145 |
| Botrtw01gri | Paladin | grinder | 12.0 | 1->3 | 11349 | 2/2 | 47 | 3 | 10 | 58 |
| Botrtw02exp | Hunter | explorer | 12.0 | 1->3 | 8163 | 5/9 | 9 | 2 | 15 | 74 |
| Botrtw03com | Rogue | completionist | 12.0 | 1->3 | 10452 | 8/18 | 15 | 1 | 19 | 96 |
| Botrtw04soc | Priest | social | 12.0 | 1->3 | 10897 | 9/16 | 12 | 0 | 13 | 60 |
| Botrtw05cas | Mage | casual | 9.9 | 1->2 | 7734 | 6/9 | 8 | 0 | 16 | 73 |
| Botrtw06com | Warrior | completionist | 12.0 | 1->3 | 9437 | 8/18 | 13 | 0 | 10 | 152 |
| Botrtw07gri | Paladin | grinder | 12.0 | 1->3 | 9872 | 2/2 | 38 | 2 | 9 | 77 |
| Botrtw08par | Hunter | party | 12.0 | 1->3 | 11063 | 9/16 | 10 | 0 | 9 | 120 |
| Botrtw09par | Rogue | party | 12.0 | 1->3 | 7902 | 6/14 | 8 | 0 | 18 | 59 |
| Botrtw10com | Priest | completionist | 12.0 | 1->3 | 10912 | 8/15 | 18 | 0 | 8 | 69 |
| Botrtw11gri | Mage | grinder | 12.0 | 1->3 | 12098 | 2/2 | 50 | 4 | 6 | 73 |
| Botrtw12exp | Warrior | explorer | 12.0 | 1->3 | 10379 | 7/10 | 9 | 0 | 15 | 70 |
| Botrtw13com | Paladin | completionist | 12.0 | 1->4 | 15449 | 12/18 | 14 | 0 | 9 | 55 |
| Botrtw14soc | Hunter | social | 12.0 | 1->3 | 11875 | 9/18 | 16 | 2 | 10 | 78 |
| Botrtw15cas | Rogue | casual | 7.8 | 1->2 | 9094 | 6/9 | 8 | 0 | 9 | 81 |
| Botrtw16com | Priest | completionist | 12.0 | 1->3 | 8868 | 7/15 | 16 | 0 | 11 | 82 |
| Botrtw17gri | Mage | grinder | 12.0 | 1->3 | 13155 | 2/2 | 54 | 3 | 5 | 49 |
| Botrtw18par | Warrior | party | 12.0 | 1->3 | 10438 | 9/16 | 10 | 0 | 16 | 93 |
| Botrtw19par | Paladin | party | 12.0 | 1->3 | 7932 | 6/14 | 8 | 0 | 18 | 102 |
