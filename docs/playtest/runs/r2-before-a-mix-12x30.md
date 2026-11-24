# Hearthblock playtest: 12 bots, 30 min, brain mix

12 bots on `local`, 343.0 bot-minutes (longest session 30.0 min). Thresholds: docs/playtest/fun-metrics.md.

Machine: load average 5.6 at the start, 6.6 mean, 17.4 at most, on 16 cores. Server tick: mean 0.45 ms, p95 1.00 ms, p99 2.50 ms, worst 67 ms over 36017 ticks; the worst minute's p95 was 2.75 ms (budget 50 ms).

## Fun checks: 3 fail, 2 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 77.72 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 8.26 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 159.02 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.19 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.67  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.95 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 10.17 s | <= 60 pass, > 90 fail |
| WARN | Share of bots with a reward drought over 5 minutes | 0.17  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 4.37 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.96  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 6.91 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.05  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.62  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 43.36 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 3.00  | >= 2 pass, < 1.5 fail |
| PASS | Share of casts that are the most used ability | 0.68  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.15  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 0.35 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 244.42 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 78 s; first quest turned in: median 8 s; level 2: median 159 s (100% of level-1 starters reached it)
- XP per hour: median 7621 (design pace 6400); 67% of XP from quests; 134 quests turned in, median 2.9 min each
- Rewards: median gap 10 s; median longest drought 167 s; 17% of bots had a drought over 5 min
- Combat: 649 fights, 96% won, median 6.9 s; 486 kills, 87 without credit (12 of them with no other player within 20 blocks); 25 deaths (4.4/h)
- Travel: 43 blocks per reward; money earned 14665 c, spent 60 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 0.3 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 0.8% |
| dead | 0.4% |
| fight | 25.6% |
| gather | 1.7% |
| idle | 1.0% |
| loot | 1.8% |
| rest | 0.8% |
| town | 1.8% |
| travel | 62.3% |
| wait_spawn | 3.9% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 12 | 1.0 | 0 |
| 2 | 12 | 1.0 | 286 |
| 4 | 12 | 2.0 | 586 |
| 6 | 12 | 2.0 | 876 |
| 8 | 12 | 2.0 | 1167 |
| 10 | 12 | 3.0 | 1363 |
| 12 | 12 | 3.0 | 1848 |
| 14 | 11 | 3.0 | 2241 |
| 16 | 11 | 3.0 | 2522 |
| 18 | 11 | 3.0 | 2703 |
| 20 | 11 | 4.0 | 2909 |
| 22 | 11 | 4.0 | 3020 |
| 24 | 11 | 4.0 | 3212 |
| 26 | 11 | 4.0 | 3389 |
| 28 | 11 | 4.0 | 3565 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 9 | 5 | 14.2 |
| broken_tower | 9 | 8 | 6.0 |
| coal_for_the_watch | 9 | 9 | 4.5 |
| cold_hearth | 12 | 12 | 0.1 |
| down_to_water | 8 | 1 | 0.9 |
| dust_of_shadows | 8 | 7 | 0.0 |
| eyes_in_the_dark | 9 | 9 | 2.7 |
| forgotten_graves | 8 | 6 | 1.9 |
| garricks_camp | 9 | 2 | 5.0 |
| hearth_stirs | 1 | 1 | 2.0 |
| iron_for_hollis | 9 | 1 | 1.7 |
| kindling | 9 | 9 | 0.6 |
| know_the_vale | 8 | 0 | - |
| lay_of_the_land | 8 | 2 | 24.5 |
| lookouts_eye | 9 | 6 | 7.5 |
| meat_for_the_pot | 9 | 9 | 2.6 |
| north_watch | 9 | 9 | 4.1 |
| old_tusk | 5 | 1 | 7.1 |
| pincher_supper | 1 | 0 | - |
| shells_for_nets | 1 | 0 | - |
| silk_for_sails | 2 | 0 | - |
| stolen_grain | 8 | 0 | - |
| thieves_in_fields | 8 | 0 | - |
| thin_the_herd | 12 | 12 | 3.7 |
| tusks_and_tempers | 2 | 0 | - |
| watchfire | 9 | 7 | 13.0 |
| webs_between_roots | 1 | 0 | - |
| windmill_breadcrumb | 9 | 8 | 5.9 |
| wool_for_winter | 9 | 9 | 2.3 |
| word_for_rysa | 2 | 1 | 1.2 |
| word_from_east | 1 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x9
- iron_for_hollis: collect Raw iron: no source in reach x5
- watchfire: collect Coal: no source in reach x5
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x4
- thieves_in_fields: goto failed: too many repaths x1

## Deaths by cause

- You fell to a Grazer. x12
- You fell to a Shade. x5
- You fell to a Thornboar. x4
- You fell to a Ashcloak Bandit. x3
- You fell to a Websnare. x1

## Fights by creature and outcome

- Grazer: won x413
- Shade: won x149
- Thornboar: won x49
- Grazer: lost (died) x12
- Shade: lost (died) x5
- Ashcloak Bandit: won x4
- Thornboar: lost (died) x4
- Websnare: won x4
- Ashcloak Bandit: lost (died) x3
- Grazer: lost (it fled home and healed) x3
- Old Tusk: won x1
- Thornboar: lost (it fled home and healed) x1
- Websnare: lost (died) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 92 | Quick Shot 54%, Barbed Arrow 37%, Recall 9% |
| Mage | 197 | Fireball 57%, Fire Blast 42%, Scholar's Focus 1%, Recall 1% |
| Paladin | 176 | Judging Blow 80%, Rebuke 12%, Holy Mend 5%, Valour 2% |
| Priest | 183 | Smite 69%, Withering Word 25%, Mend 3%, Recall 3% |
| Rogue | 87 | Vicious Strike 78%, Finishing Cut 20%, Recall 2% |
| Warrior | 92 | Heroic Blow 51%, Rending Cut 26%, Battle Cry 12%, Onrush 5%, Recall 5% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-144, -160]: 1 events
  - Botttc02exp at [-138, -13, -150] going to [-150, -10, -150] (repaths; exploring The Old Quarry)
- cell [224, 480]: 1 events
  - Botttc02exp at [224, -25, 495] going to [230, -24, 495] (repaths; thieves_in_fields: go to the Ashcloak Bandit camp)

## Notes

- grouped in a party of two x2
- casual session ended x1
- tried /invite Botttc03com x1

## Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)

- Grazer lv1 at -97,-6,-81 (ground -8) x3
- Grazer lv1 at 57,-7,-152 (ground -9) x3
- Grazer lv1 at 88,-6,-89 (ground -8) x3
- Grazer lv1 at 11,-10,-119 (ground -12) x2
- Grazer lv1 at 121,-8,9 (ground -10) x2
- Grazer lv2 at 91,-9,95 (ground -11) x2
- Thornboar lv1 at -161,-8,48 (ground -10) x2
- Grazer lv1 at -203,-15,-263 (ground -17) x1
- Grazer lv1 at -206,-16,-260 (ground -18) x1
- Grazer lv1 at -217,-16,-261 (ground -18) x1
- Grazer lv1 at -71,-9,87 (ground -11) x1
- Grazer lv1 at -74,-7,-95 (ground -9) x1
- Grazer lv1 at -80,-13,178 (ground -15) x1
- Grazer lv1 at 126,-8,19 (ground -10) x1
- Grazer lv1 at 187,-10,52 (ground -12) x1
- Grazer lv1 at 71,-6,-87 (ground -8) x1
- Grazer lv2 at -137,-9,-16 (ground -11) x1
- Grazer lv2 at -276,-8,139 (ground -11) x1
- Grazer lv2 at 57,-7,-152 (ground -9) x1
- Grazer lv3 at 13,-22,410 (ground -24) x1
- Shade lv1 at 220,-10,73 (ground -12) x1
- Thornboar lv2 at -103,0,-404 (ground -2) x1
- Thornboar lv2 at -64,-10,-300 (ground -12) x1
- Thornboar lv2 at 148,-4,-113 (ground -6) x1
- Thornboar lv3 at -67,-11,-299 (ground -13) x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 1 | 9256 | 9 | 0.0 | 100% | 8 | 1% |
| completionist | 4 | 8020 | 63 | 1.5 | 97% | 11 | 10% |
| explorer | 1 | 7800 | 15 | 4.0 | 87% | 12 | 2% |
| grinder | 3 | 6592 | 6 | 12.7 | 95% | 10 | 2% |
| party | 2 | 6943 | 26 | 0.0 | 100% | 16 | 1% |
| social | 1 | 7443 | 15 | 2.0 | 97% | 9 | 9% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 2 | 6904 | 26 | 2.0 | 93% | 15 | 1% |
| Mage | 2 | 8128 | 11 | 7.0 | 96% | 7 | 2% |
| Paladin | 2 | 6415 | 4 | 14.0 | 94% | 10 | 2% |
| Priest | 2 | 7812 | 30 | 1.0 | 99% | 9 | 15% |
| Rogue | 2 | 7574 | 30 | 1.0 | 98% | 14 | 2% |
| Warrior | 2 | 9578 | 33 | 2.0 | 96% | 11 | 9% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botttc00com | Warrior | completionist | 30.0 | 1->5 | 11298 | 18/25 | 24 | 1 | 9 | 105 |
| Botttc01gri | Paladin | grinder | 30.0 | 1->4 | 6238 | 2/2 | 86 | 8 | 10 | 379 |
| Botttc02exp | Hunter | explorer | 30.0 | 1->4 | 7800 | 15/22 | 23 | 2 | 12 | 134 |
| Botttc03com | Rogue | completionist | 30.0 | 1->4 | 7270 | 15/23 | 24 | 1 | 14 | 194 |
| Botttc04soc | Priest | social | 30.0 | 1->4 | 7443 | 15/23 | 25 | 1 | 9 | 177 |
| Botttc05cas | Mage | casual | 13.0 | 1->3 | 9256 | 9/16 | 13 | 0 | 8 | 62 |
| Botttc06com | Warrior | completionist | 30.0 | 1->4 | 7859 | 15/23 | 22 | 1 | 13 | 168 |
| Botttc07gri | Paladin | grinder | 30.0 | 1->4 | 6592 | 2/2 | 105 | 6 | 10 | 122 |
| Botttc08par | Hunter | party | 30.0 | 1->4 | 6008 | 11/25 | 18 | 0 | 17 | 165 |
| Botttc09par | Rogue | party | 30.0 | 1->4 | 7877 | 15/27 | 19 | 0 | 14 | 161 |
| Botttc10com | Priest | completionist | 30.0 | 1->4 | 8181 | 15/23 | 25 | 0 | 10 | 276 |
| Botttc11gri | Mage | grinder | 30.0 | 1->4 | 7001 | 2/2 | 102 | 5 | 6 | 372 |
