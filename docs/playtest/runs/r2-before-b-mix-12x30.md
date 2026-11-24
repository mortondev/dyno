# Hearthblock playtest: 12 bots, 30 min, brain mix

12 bots on `local`, 344.7 bot-minutes (longest session 30.0 min). Thresholds: docs/playtest/fun-metrics.md.

Machine: load average 10.3 at the start, 8.3 mean, 16.2 at most, on 16 cores. Server tick: mean 0.47 ms, p95 1.25 ms, p99 2.75 ms, worst 58 ms over 35995 ticks; the worst minute's p95 was 1.75 ms (budget 50 ms).

## Fun checks: 2 fail, 3 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 75.00 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 8.26 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 191.46 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.13 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.64  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 3.21 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 10.89 s | <= 60 pass, > 90 fail |
| WARN | Share of bots with a reward drought over 5 minutes | 0.25  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 4.35 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.96  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 6.31 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.09  | <= 0.15 pass, > 0.3 fail |
| WARN | Share of time travelling | 0.53  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 37.10 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.92  | >= 2 pass, < 1.5 fail |
| PASS | Share of casts that are the most used ability | 0.70  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.22  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 1.22 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 298.82 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 75 s; first quest turned in: median 8 s; level 2: median 191 s (100% of level-1 starters reached it)
- XP per hour: median 7257 (design pace 6400); 64% of XP from quests; 129 quests turned in, median 3.2 min each
- Rewards: median gap 11 s; median longest drought 161 s; 25% of bots had a drought over 5 min
- Combat: 745 fights, 96% won, median 6.3 s; 521 kills, 147 without credit (10 of them with no other player within 20 blocks); 25 deaths (4.4/h)
- Travel: 37 blocks per reward; money earned 13447 c, spent 45 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 1.2 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 1.4% |
| dead | 0.4% |
| fight | 28.2% |
| gather | 1.8% |
| idle | 2.4% |
| loot | 2.1% |
| rest | 2.0% |
| town | 1.9% |
| travel | 53.5% |
| wait_spawn | 6.3% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 12 | 1.0 | 0 |
| 2 | 12 | 1.0 | 259 |
| 4 | 12 | 2.0 | 552 |
| 6 | 12 | 2.0 | 826 |
| 8 | 12 | 2.0 | 1089 |
| 10 | 12 | 3.0 | 1349 |
| 12 | 12 | 3.0 | 1586 |
| 14 | 12 | 3.0 | 2044 |
| 16 | 11 | 3.0 | 2365 |
| 18 | 11 | 4.0 | 2596 |
| 20 | 11 | 4.0 | 2851 |
| 22 | 11 | 4.0 | 3038 |
| 24 | 11 | 4.0 | 3238 |
| 26 | 11 | 4.0 | 3402 |
| 28 | 11 | 4.0 | 3643 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 8 | 7 | 12.4 |
| broken_tower | 9 | 9 | 5.8 |
| coal_for_the_watch | 9 | 8 | 3.5 |
| cold_hearth | 12 | 12 | 0.1 |
| down_to_water | 3 | 0 | - |
| dust_of_shadows | 6 | 6 | 0.0 |
| eyes_in_the_dark | 8 | 6 | 4.8 |
| forgotten_graves | 7 | 3 | 2.1 |
| garricks_camp | 8 | 4 | 3.0 |
| hearth_stirs | 1 | 0 | - |
| into_barrowwood | 1 | 0 | - |
| iron_for_hollis | 8 | 3 | 1.9 |
| kindling | 9 | 9 | 0.6 |
| know_the_vale | 6 | 0 | - |
| lay_of_the_land | 9 | 4 | 15.3 |
| lookouts_eye | 8 | 6 | 10.3 |
| meat_for_the_pot | 9 | 9 | 3.1 |
| north_watch | 9 | 8 | 3.8 |
| old_tusk | 7 | 1 | 2.3 |
| silk_for_sails | 4 | 0 | - |
| stolen_grain | 3 | 0 | - |
| thieves_in_fields | 3 | 0 | - |
| thin_the_herd | 12 | 12 | 4.2 |
| tusks_and_tempers | 4 | 1 | 0.0 |
| watchfire | 8 | 7 | 8.6 |
| webs_between_roots | 2 | 1 | 2.4 |
| windmill_breadcrumb | 8 | 3 | 5.0 |
| wool_for_winter | 9 | 8 | 2.7 |
| word_for_rysa | 4 | 2 | 1.1 |
| word_from_east | 2 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x9
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x7
- watchfire: collect Coal: no source in reach x7
- iron_for_hollis: collect Raw iron: no source in reach x3
- boar_trouble: goto failed: too many repaths x1
- eyes_in_the_dark: goto failed: too many repaths x1
- hearth_stirs: collect Shade dust: no source in reach x1
- hearth_stirs: kill Shade: only at night: waiting for dark x1
- iron_for_hollis: collect Raw iron: Requires mining 5 (have 4) x1
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x1

## Deaths by cause

- You fell to a Grazer. x13
- You fell to a Shade. x6
- You fell to a Barrow Warden. x3
- You fell to a Thornboar. x2
- You fell to a Old Tusk. x1

## Fights by creature and outcome

- Grazer: won x491
- Shade: won x159
- Thornboar: won x57
- Grazer: lost (died) x13
- Websnare: won x10
- Shade: lost (died) x6
- Barrow Warden: lost (died) x3
- Thornboar: lost (died) x2
- Old Tusk: lost (died) x1
- Old Tusk: won x1
- Thornboar: lost (it fled home and healed) x1
- Websnare: lost (it fled home and healed) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 77 | Quick Shot 58%, Barbed Arrow 38%, Recall 4% |
| Mage | 244 | Fireball 60%, Fire Blast 39%, Recall 1%, Scholar's Focus 1% |
| Paladin | 211 | Judging Blow 80%, Rebuke 12%, Holy Mend 6%, Valour 2% |
| Priest | 243 | Smite 70%, Withering Word 25%, Mend 2%, Recall 2% |
| Rogue | 59 | Vicious Strike 78%, Finishing Cut 19%, Recall 3% |
| Warrior | 86 | Heroic Blow 52%, Rending Cut 27%, Battle Cry 15%, Recall 3%, Onrush 2% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-176, -208]: 1 events
  - Botcpy09par at [-162, -15, -195] going to [-159, -15, -197] (repaths; following Botcpy08par)
- cell [-160, 32]: 1 events
  - Botcpy06com at [-156, -9, 40] going to [-150, -3, 40] (repaths; boar_trouble: go to the Thornboar camp)
- cell [-144, -160]: 1 events
  - Botcpy02exp at [-138, -13, -150] going to [-150, -10, -150] (repaths; exploring The Old Quarry)
- cell [-144, -32]: 1 events
  - Botcpy11gri at [-136, -10, -25] going to [-130, -10, -25] (repaths; going to grind)
- cell [-144, 288]: 1 events
  - Botcpy02exp at [-130, -18, 288] going to [-130, -10, 300] (repaths; exploring Southfold Farm)
- cell [0, 32]: 1 events
  - Botcpy10com at [0, 0, 45] going to [0, -14, 49] (repaths; eyes_in_the_dark: look for Shade in the dark)
- cell [624, 32]: 1 events
  - Botcpy02exp at [628, -14, 45] going to [640, -33, 45] (repaths; exploring Miner's Rest)

## Notes

- grouped in a party of two x2
- casual session ended x1
- tried /invite Botcpy03com x1

## Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)

- Grazer lv1 at -76,-5,-70 (ground -7) x3
- Grazer lv1 at -97,-6,-81 (ground -8) x3
- Thornboar lv2 at -150,-7,41 (ground -9) x3
- Grazer lv1 at -98,-21,288 (ground -23) x2
- Grazer lv1 at -98,-7,-76 (ground -9) x2
- Grazer lv1 at -99,-9,-59 (ground -11) x2
- Grazer lv2 at -137,-9,-16 (ground -11) x2
- Shade lv2 at -70,-9,104 (ground -11) x2
- Thornboar lv3 at -84,3,-392 (ground 1) x2
- Grazer lv1 at -144,-10,-11 (ground -12) x1
- Grazer lv1 at -74,-7,-95 (ground -9) x1
- Grazer lv1 at -87,-21,300 (ground -23) x1
- Grazer lv1 at -93,-21,285 (ground -23) x1
- Grazer lv1 at -96,-8,-68 (ground -10) x1
- Grazer lv1 at 126,-8,19 (ground -10) x1
- Grazer lv1 at 70,-13,-204 (ground -15) x1
- Grazer lv1 at 77,-10,98 (ground -12) x1
- Grazer lv2 at -97,-6,-81 (ground -8) x1
- Grazer lv2 at -98,-6,-85 (ground -8) x1
- Grazer lv2 at 121,-8,9 (ground -10) x1
- Grazer lv2 at 142,-9,16 (ground -11) x1
- Grazer lv2 at 77,-4,-70 (ground -6) x1
- Grazer lv4 at 6,-22,401 (ground -24) x1
- Thornboar lv2 at -55,-6,-329 (ground -8) x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 1 | 8295 | 8 | 4.1 | 95% | 8 | 2% |
| completionist | 4 | 7839 | 58 | 3.5 | 95% | 13 | 15% |
| explorer | 1 | 6407 | 11 | 0.0 | 94% | 13 | 1% |
| grinder | 3 | 7366 | 6 | 10.7 | 97% | 9 | 2% |
| party | 2 | 5518 | 26 | 0.0 | 98% | 14 | 18% |
| social | 1 | 11531 | 20 | 2.0 | 98% | 10 | 1% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 2 | 6329 | 25 | 0.0 | 95% | 14 | 9% |
| Mage | 2 | 7722 | 10 | 8.0 | 96% | 7 | 2% |
| Paladin | 2 | 7415 | 4 | 11.0 | 97% | 10 | 2% |
| Priest | 2 | 10157 | 35 | 2.0 | 98% | 9 | 5% |
| Rogue | 2 | 5949 | 26 | 2.0 | 95% | 12 | 20% |
| Warrior | 2 | 7619 | 29 | 4.0 | 94% | 16 | 15% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botcpy00com | Warrior | completionist | 30.0 | 1->4 | 6675 | 13/18 | 21 | 3 | 18 | 258 |
| Botcpy01gri | Paladin | grinder | 30.0 | 1->4 | 7465 | 2/2 | 109 | 5 | 9 | 100 |
| Botcpy02exp | Hunter | explorer | 30.0 | 1->4 | 6407 | 11/22 | 16 | 0 | 13 | 184 |
| Botcpy03com | Rogue | completionist | 30.0 | 1->4 | 7115 | 14/20 | 21 | 2 | 11 | 450 |
| Botcpy04soc | Priest | social | 30.0 | 1->5 | 11531 | 20/28 | 36 | 1 | 10 | 93 |
| Botcpy05cas | Mage | casual | 14.7 | 1->3 | 8295 | 8/11 | 16 | 1 | 8 | 97 |
| Botcpy06com | Warrior | completionist | 30.0 | 1->4 | 8563 | 16/23 | 21 | 1 | 15 | 233 |
| Botcpy07gri | Paladin | grinder | 30.0 | 1->4 | 7366 | 2/2 | 106 | 6 | 11 | 138 |
| Botcpy08par | Hunter | party | 30.0 | 1->4 | 6252 | 14/23 | 21 | 0 | 14 | 304 |
| Botcpy09par | Rogue | party | 30.0 | 1->3 | 4784 | 12/20 | 7 | 0 | 13 | 306 |
| Botcpy10com | Priest | completionist | 30.0 | 1->4 | 8783 | 15/25 | 28 | 1 | 7 | 137 |
| Botcpy11gri | Mage | grinder | 30.0 | 1->4 | 7149 | 2/2 | 119 | 5 | 5 | 82 |
