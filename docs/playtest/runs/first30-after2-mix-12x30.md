# Hearthblock playtest: 12 bots, 30 min, brain mix

12 bots on `127.0.0.1:7710`, 338.4 bot-minutes (longest session 30.1 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 3 fail, 2 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 74.98 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 8.43 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 184.52 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.06 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.57  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 3.59 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 10.41 s | <= 60 pass, > 90 fail |
| WARN | Share of bots with a reward drought over 5 minutes | 0.17  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 4.43 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.95  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 6.78 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.05  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.55  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 35.43 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.92  | >= 2 pass, < 1.5 fail |
| PASS | Share of casts that are the most used ability | 0.70  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.20  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 1.06 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 77.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 11160.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 75 s; first quest turned in: median 8 s; level 2: median 185 s (100% of level-1 starters reached it)
- XP per hour: median 6758 (design pace 6400); 57% of XP from quests; 108 quests turned in, median 3.6 min each
- Rewards: median gap 10 s; median longest drought 205 s; 17% of bots had a drought over 5 min
- Combat: 786 fights, 95% won, median 6.8 s; 543 kills, 137 without credit (16 of them with no other player within 20 blocks); 25 deaths (4.4/h)
- Travel: 35 blocks per reward; money earned 11160 c, spent 0 c
- Server: action round trip p50 50 ms, p95 77 ms; 0 disconnects; 1.1 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 0.6% |
| dead | 0.4% |
| fight | 31.5% |
| gather | 1.3% |
| idle | 4.6% |
| loot | 2.3% |
| rest | 1.7% |
| town | 2.0% |
| travel | 55.4% |
| wait_spawn | 0.2% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 12 | 1.0 | 0 |
| 2 | 12 | 1.0 | 250 |
| 4 | 12 | 2.0 | 508 |
| 6 | 12 | 2.0 | 809 |
| 8 | 12 | 2.0 | 1009 |
| 10 | 11 | 3.0 | 1319 |
| 12 | 11 | 3.0 | 1722 |
| 14 | 11 | 3.0 | 1969 |
| 16 | 11 | 3.0 | 2247 |
| 18 | 11 | 3.0 | 2455 |
| 20 | 11 | 3.0 | 2582 |
| 22 | 11 | 4.0 | 2672 |
| 24 | 11 | 4.0 | 2854 |
| 26 | 11 | 4.0 | 2952 |
| 28 | 11 | 4.0 | 3150 |
| 30 | 2 | 3.5 | 3358 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 6 | 4 | 17.6 |
| broken_tower | 9 | 8 | 7.7 |
| coal_for_the_watch | 9 | 7 | 6.6 |
| cold_hearth | 12 | 12 | 0.1 |
| down_to_water | 7 | 1 | 1.1 |
| dust_of_shadows | 5 | 5 | 0.0 |
| eyes_in_the_dark | 6 | 5 | 3.5 |
| forgotten_graves | 4 | 3 | 2.8 |
| garricks_camp | 6 | 0 | - |
| iron_for_hollis | 7 | 0 | - |
| kindling | 9 | 9 | 0.6 |
| know_the_vale | 5 | 0 | - |
| lay_of_the_land | 8 | 1 | 21.1 |
| lookouts_eye | 6 | 3 | 18.8 |
| meat_for_the_pot | 9 | 9 | 3.1 |
| north_watch | 9 | 6 | 5.0 |
| old_tusk | 4 | 1 | 2.2 |
| pincher_supper | 1 | 0 | - |
| stolen_grain | 7 | 0 | - |
| thieves_in_fields | 7 | 1 | 6.8 |
| thin_the_herd | 12 | 12 | 4.1 |
| watchfire | 6 | 5 | 15.2 |
| windmill_breadcrumb | 8 | 7 | 6.3 |
| wool_for_winter | 9 | 9 | 2.5 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x8
- iron_for_hollis: collect Raw iron: no source in reach x5
- watchfire: collect Coal: no source in reach x4
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x2
- iron_for_hollis: collect Raw iron: Requires mining 5 (have 4) x2
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x2
- dust_of_shadows: goto failed: too many repaths x1
- thieves_in_fields: goto failed: dead x1
- thieves_in_fields: goto failed: too many repaths x1

## Deaths by cause

- You fell to a Grazer. x12
- You fell to a Shade. x7
- You fell to a Ashcloak Bandit. x3
- You fell to a Thornboar. x2
- You fell to a Old Tusk. x1

## Fights by creature and outcome

- Grazer: won x556
- Shade: won x137
- Thornboar: won x49
- Grazer: lost (died) x12
- Grazer: lost (could not reach or hurt it) x8
- Ashcloak Bandit: won x7
- Shade: lost (died) x7
- Ashcloak Bandit: lost (died) x2
- Grazer: lost (it fled home and healed) x2
- Thornboar: lost (died) x2
- Ashcloak Bandit: lost (it fled home and healed) x1
- Old Tusk: lost (died) x1
- Old Tusk: won x1
- Thornboar: lost (it fled home and healed) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 73 | Quick Shot 58%, Barbed Arrow 37%, Recall 5% |
| Mage | 264 | Fireball 59%, Fire Blast 40%, Scholar's Focus 1%, Recall 0% |
| Paladin | 216 | Judging Blow 80%, Rebuke 13%, Holy Mend 5%, Valour 2% |
| Priest | 188 | Smite 72%, Withering Word 22%, Recall 3%, Mend 2% |
| Rogue | 83 | Vicious Strike 77%, Finishing Cut 19%, Recall 4% |
| Warrior | 107 | Heroic Blow 51%, Rending Cut 27%, Battle Cry 13%, Recall 7%, Onrush 2% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [128, 16]: 2 events
  - Botgvx01gri at [141, -10, 20] going to [135, -9, 20] (repaths; going to grind)
  - Botgvx11gri at [141, -10, 20] going to [135, -9, 20] (repaths; going to grind)
- cell [-144, -160]: 1 events
  - Botgvx02exp at [-138, -13, -150] going to [-150, -10, -150] (repaths; exploring The Old Quarry)
- cell [16, 0]: 1 events
  - Botgvx09par at [25, 0, 7] going to [21, 1, 7] (repaths; following Botgvx08par)
- cell [32, 0]: 1 events
  - Botgvx10com at [45, 0, 2] going to [49, 0, 2] (repaths; dust_of_shadows: look for Shade in the dark for Shade dust)
- cell [224, 480]: 1 events
  - Botgvx00com at [224, -25, 495] going to [230, -24, 495] (repaths; thieves_in_fields: go to the Ashcloak Bandit camp)

## Notes

- grouped in a party of two x2
- casual session ended x1
- tried /invite Botgvx03com x1

## Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)

- Grazer lv2 at -97,-6,-81 (ground -8) x8
- Grazer lv1 at -97,-6,-81 (ground -8) x2
- Grazer lv2 at 126,-8,19 (ground -10) x2
- Grazer lv1 at 60,-7,-97 (ground -9) x1
- Grazer lv1 at 75,-10,98 (ground -12) x1
- Grazer lv1 at 88,-6,-89 (ground -8) x1
- Grazer lv2 at -164,-10,-27 (ground -12) x1
- Shade lv1 at 220,-10,76 (ground -12) x1
- Shade lv3 at 38,-20,383 (ground -22) x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 1 | 6818 | 5 | 0.0 | 100% | 8 | 2% |
| completionist | 4 | 7405 | 56 | 3.5 | 96% | 15 | 2% |
| explorer | 1 | 4282 | 8 | 2.0 | 95% | 14 | 1% |
| grinder | 3 | 7589 | 6 | 11.3 | 95% | 9 | 2% |
| party | 2 | 4436 | 17 | 0.0 | 100% | 10 | 22% |
| social | 1 | 9137 | 16 | 0.0 | 95% | 10 | 1% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 2 | 4328 | 16 | 1.0 | 98% | 11 | 1% |
| Mage | 2 | 7203 | 7 | 6.3 | 97% | 7 | 2% |
| Paladin | 2 | 7306 | 4 | 13.0 | 94% | 10 | 2% |
| Priest | 2 | 7532 | 29 | 3.0 | 92% | 9 | 1% |
| Rogue | 2 | 6496 | 25 | 1.0 | 98% | 14 | 22% |
| Warrior | 2 | 7405 | 27 | 3.0 | 98% | 17 | 2% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botgvx00com | Warrior | completionist | 30.0 | 1->4 | 6699 | 12/22 | 27 | 2 | 19 | 125 |
| Botgvx01gri | Paladin | grinder | 30.0 | 1->4 | 6434 | 2/2 | 104 | 6 | 9 | 147 |
| Botgvx02exp | Hunter | explorer | 30.1 | 1->3 | 4282 | 8/15 | 18 | 1 | 14 | 253 |
| Botgvx03com | Rogue | completionist | 30.0 | 1->4 | 8495 | 16/23 | 22 | 1 | 15 | 203 |
| Botgvx04soc | Priest | social | 30.0 | 1->4 | 9137 | 16/23 | 23 | 0 | 10 | 207 |
| Botgvx05cas | Mage | casual | 8.4 | 1->2 | 6818 | 5/8 | 12 | 0 | 8 | 59 |
| Botgvx06com | Warrior | completionist | 30.0 | 1->4 | 8112 | 15/23 | 26 | 1 | 15 | 215 |
| Botgvx07gri | Paladin | grinder | 30.0 | 1->4 | 8177 | 2/2 | 124 | 7 | 11 | 62 |
| Botgvx08par | Hunter | party | 30.0 | 1->3 | 4374 | 8/14 | 24 | 0 | 7 | 869 |
| Botgvx09par | Rogue | party | 30.0 | 1->3 | 4497 | 9/15 | 14 | 0 | 13 | 753 |
| Botgvx10com | Priest | completionist | 30.0 | 1->4 | 5927 | 13/22 | 23 | 3 | 8 | 300 |
| Botgvx11gri | Mage | grinder | 30.0 | 1->4 | 7589 | 2/2 | 126 | 4 | 5 | 63 |
