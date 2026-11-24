# Classes at levels 1 to 5: the swarm, after (this branch, protocol 24)

12 bots (two per class, mixed brains) for 30 minutes on a fresh world, one server, port 7740. The per-class tables count only fights and deaths at character level 5 or below (the bots end at level 3 to 4). "Presses per 10 s" is ability presses per 10 seconds of fighting. The full fun report follows.

## By class

| Class | Bots | Fights (won) | Kill time, median s | same-level kill s | Damage taken / fight | Casts / fight | Presses per 10 s | Deaths (per hour) | Level after run | XP / hour |
|---|---|---|---|---|---|---|---|---|---|---|
| Warrior | 2 | 64 (95%) | 5.0 | 4.5 | 7.9 | 2.2 | 3.6 | 0 (0.0) | 4/4 | 2826 |
| Paladin | 2 | 100 (98%) | 5.9 | 6.3 | 10.5 | 2.0 | 3.0 | 1 (1.0) | 3/4 | 2659 |
| Hunter | 2 | 43 (100%) | 4.9 | 4.9 | 3.3 | 2.5 | 3.6 | 0 (0.0) | 3/4 | 2573 |
| Rogue | 2 | 48 (98%) | 4.4 | 4.6 | 4.7 | 1.8 | 3.5 | 1 (1.0) | 3/3 | 2606 |
| Priest | 2 | 68 (100%) | 4.7 | 4.7 | 2.1 | 2.7 | 5.4 | 0 (0.0) | 4/4 | 3379 |
| Mage | 2 | 44 (95%) | 5.4 | 5.7 | 5.1 | 2.5 | 4.3 | 2 (2.8) | 3/3 | 2570 |

| Class | Abilities (share of presses) | Deaths by cause |
|---|---|---|
| Warrior | Heroic Blow 47%, Onrush 24%, Rending Cut 22%, Ground Slam 3%, Second Wind 3%, Recall 2% | none |
| Paladin | Judging Blow 63%, Seal of Dawn 17%, Rebuke 10%, Holy Mend 6%, Valour 4% | Shade (lv2) x1 |
| Hunter | Quick Shot 57%, Barbed Arrow 33%, Marked Shot 4%, Recall 4%, Hamstring 2% | none |
| Rogue | Vicious Strike 43%, Finishing Cut 28%, Ambush 13%, Low Blow 13%, Recall 2% | Shade (lv2) x1 |
| Priest | Smite 64%, Withering Word 29%, Ward of Light 4%, Recall 3% | none |
| Mage | Fireball 58%, Frostbolt 22%, Fire Blast 19%, Recall 1% | Shade (lv3) x1, Shade (lv2) x1 |

Kill time spread vs mean (5.1 s): Warrior -2%, Paladin +17%, Hunter -3%, Rogue -12%, Priest -7%, Mage +8%

| Class | L1 kill s (n) | L2 kill s (n) | L3 kill s (n) | L4 kill s (n) | L5 kill s (n) |
|---|---|---|---|---|---|
| Warrior | 3.3 (5) | 3.2 (23) | 5.7 (24) | 5.9 (9) | - |
| Paladin | 5.5 (14) | 4.2 (40) | 8.0 (33) | 7.8 (11) | - |
| Hunter | 4.8 (8) | 4.7 (18) | 6.8 (17) | - | - |
| Rogue | 3.7 (5) | 3.0 (17) | 5.1 (25) | - | - |
| Priest | 3.3 (7) | 4.7 (21) | 4.7 (35) | 4.7 (5) | - |
| Mage | 5.8 (11) | 5.1 (19) | 3.9 (12) | - | - |

---

# Hearthblock playtest: 12 bots, 30 min, brain mix

12 bots on `127.0.0.1:7740`, 343.5 bot-minutes (longest session 30.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 1 fail, 5 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 109.26 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 8.63 s | <= 180 s pass, > 300 s fail |
| WARN | Time to level 2 from level 1 (median) | 498.77 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 0.87 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.50  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.83 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 21.07 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| PASS | Deaths per hour of play | 0.70 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.98  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 4.85 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.02  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.82  | <= 0.4 pass, > 0.55 fail |
| WARN | Blocks walked per reward | 72.01 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 4.00  | >= 2 pass, < 1.5 fail |
| PASS | Share of casts that are the most used ability | 0.57  | <= 0.7 pass, > 0.85 fail |
| WARN | Kills lost to someone else's last hit | 0.10  | <= 0.05 pass, > 0.15 fail |
| WARN | Stuck events per bot-hour | 4.89 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 56.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.17 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 6910.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 109 s; first quest turned in: median 9 s; level 2: median 499 s (100% of level-1 starters reached it)
- XP per hour: median 5586 (design pace 6400); 50% of XP from quests; 90 quests turned in, median 2.8 min each
- Rewards: median gap 21 s; median longest drought 202 s; 0% of bots had a drought over 5 min
- Combat: 367 fights, 98% won, median 4.8 s; 308 kills, 34 without credit (2 of them with no other player within 20 blocks); 4 deaths (0.7/h)
- Travel: 72 blocks per reward; money earned 6910 c, spent 0 c
- Server: action round trip p50 50 ms, p95 56 ms; 0 disconnects; 4.9 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 0.8% |
| dead | 0.1% |
| fight | 10.8% |
| gather | 0.9% |
| idle | 1.9% |
| loot | 0.8% |
| rest | 0.3% |
| stuck | 0.5% |
| town | 1.6% |
| travel | 82.5% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 12 | 1.0 | 0 |
| 2 | 12 | 1.0 | 126 |
| 4 | 12 | 1.0 | 227 |
| 6 | 12 | 1.0 | 325 |
| 8 | 12 | 1.5 | 437 |
| 10 | 12 | 2.0 | 542 |
| 12 | 12 | 2.0 | 850 |
| 14 | 11 | 2.0 | 1059 |
| 16 | 11 | 2.0 | 1259 |
| 18 | 11 | 3.0 | 1534 |
| 20 | 11 | 3.0 | 1710 |
| 22 | 11 | 3.0 | 1836 |
| 24 | 11 | 3.0 | 1976 |
| 26 | 11 | 3.0 | 2125 |
| 28 | 11 | 3.0 | 2366 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 8 | 3 | 7.4 |
| coal_for_the_watch | 8 | 0 | - |
| cold_hearth | 12 | 12 | 0.1 |
| down_to_water | 2 | 0 | - |
| dust_of_shadows | 5 | 5 | 0.0 |
| eyes_in_the_dark | 8 | 5 | 8.8 |
| garricks_camp | 8 | 7 | 1.2 |
| into_barrowwood | 1 | 0 | - |
| kindling | 9 | 8 | 4.9 |
| meat_for_the_pot | 9 | 9 | 10.6 |
| north_watch | 9 | 8 | 0.9 |
| report_to_holt | 8 | 8 | 0.7 |
| silk_for_sails | 7 | 1 | 8.1 |
| stolen_grain | 2 | 0 | - |
| thieves_in_fields | 2 | 0 | - |
| thin_the_herd | 12 | 12 | 16.2 |
| tusks_and_tempers | 7 | 2 | 4.8 |
| watchfire | 8 | 0 | - |
| windmill_breadcrumb | 8 | 2 | 3.8 |
| wool_for_winter | 9 | 8 | 3.6 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x8
- watchfire: collect Coal: no source in reach x8
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x7
- thin_the_herd: goto failed: too many repaths x5
- silk_for_sails: goto failed: stuck x2
- tusks_and_tempers: goto failed: stuck x2
- boar_trouble: goto failed: stuck x1
- dust_of_shadows: collect Shade dust: no source in reach x1
- dust_of_shadows: kill Shade: only at night: waiting for dark x1
- eyes_in_the_dark: goto failed: stuck x1
- kindling: collect Log: no source in reach x1
- report_to_holt: talk failed: cannot reach NPC: stuck x1
- tusks_and_tempers: goto failed: too many repaths x1
- windmill_breadcrumb: talk failed: cannot reach NPC: stuck x1

## Deaths by cause

- You fell to a Shade. x4

## Fights by creature and outcome

- Grazer: won x170
- Shade: won x121
- Thornboar: won x47
- Ashcloak Bandit: won x13
- Websnare: won x8
- Shade: lost (died) x4
- Thornboar: lost (cannot reach it: stuck) x3
- Ashcloak Bandit: lost (it fled home and healed) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 112 | Quick Shot 57%, Barbed Arrow 33%, Marked Shot 4%, Recall 4%, Hamstring 2% |
| Mage | 117 | Fireball 58%, Frostbolt 22%, Fire Blast 19%, Recall 1% |
| Paladin | 202 | Judging Blow 63%, Seal of Dawn 17%, Rebuke 10%, Holy Mend 6%, Valour 4% |
| Priest | 192 | Smite 64%, Withering Word 29%, Ward of Light 4%, Recall 3% |
| Rogue | 90 | Vicious Strike 43%, Finishing Cut 28%, Ambush 13%, Low Blow 13%, Recall 2% |
| Warrior | 144 | Heroic Blow 47%, Onrush 24%, Rending Cut 22%, Ground Slam 3%, Second Wind 3%, Recall 2% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-96, -416]: 8 events
  - Boterw00com at [-84, 4, -408] going to [-85, -2, -408] (stuck; boar_trouble: kill Thornboar)
  - Boterw00com at [-84, 4, -408] going to [-102, 5, -383] (stuck; tusks_and_tempers: kill Thornboar for Boar Tusk)
  - Boterw00com at [-84, 4, -408] going to [-480, 4, -266] (stuck; silk_for_sails: go to the Websnare camp for Websnare Silk)
  - Boterw00com at [-84, 4, -408] going to [-90, 6, -380] (stuck; boar_trouble: go to the Thornboar camp)
- cell [0, -80]: 8 events
  - Boterw00com at [8, 0, -78] going to [-41, 0, 27] (stuck; eyes_in_the_dark: look for Shade in the dark)
  - Boterw00com at [8, 0, -78] going to [2, 0, -251] (stuck; report_to_holt: talk to Sergeant Holt)
  - Boterw00com at [8, 0, -78] going to [66, 0, 486] (stuck; windmill_breadcrumb: talk to Farmer Ostrin)
  - Boterw00com at [8, 0, -78] going to [4, 0, -82] (stuck; looting)
- cell [288, -144]: 5 events
  - Boterw01gri at [290, -10, -144] going to [290, -9, -150] (repaths; going to grind)
  - Boterw04soc at [290, -10, -144] going to [290, -9, -150] (repaths; thin_the_herd: go to the Grazer camp)
  - Boterw07gri at [290, -10, -144] going to [290, -9, -150] (repaths; going to grind)
  - Boterw10com at [290, -10, -144] going to [290, -9, -150] (repaths; thin_the_herd: go to the Grazer camp)
- cell [144, -272]: 2 events
  - Boterw03com at [156, -9, -260] going to [150, -7, -260] (repaths; thin_the_herd: go to the Grazer camp)
  - Boterw07gri at [156, -9, -260] going to [150, -10, -260] (repaths; thin_the_herd: go to the Grazer camp)
- cell [-48, -592]: 1 events
  - Boterw04soc at [-40, -2, -584] going to [-40, 6, -590] (repaths; tusks_and_tempers: go to the Thornboar camp for Boar Tusk)
- cell [16, 432]: 1 events
  - Boterw07gri at [25, -22, 434] going to [25, -22, 440] (repaths; going to grind)
- cell [144, -256]: 1 events
  - Boterw01gri at [150, -11, -254] going to [150, -10, -260] (repaths; thin_the_herd: go to the Grazer camp)
- cell [240, 96]: 1 events
  - Boterw07gri at [240, -9, 104] going to [240, -6, 110] (repaths; thin_the_herd: go to the Grazer camp)
- cell [288, -176]: 1 events
  - Boterw00com at [289, -6, -165] going to [3, -6, -15] (stuck; visiting Captain Maren: hand in thin_the_herd)

## Rubber-bands (moves the server refused; hearth-relative)

- Boterw00com at [-11, 0, -94] sent back to [8, 0, -78]: moved back 23.8 blocks (on ground true, in water false) (eyes_in_the_dark: kill Shade)

## Notes

- grouped in a party of two x2
- casual session ended x1
- tried /invite Boterw10com x1
- used /spawn to get unstuck x1

## Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)

- Shade lv2 at 142,-8,-280 (ground -10) x1
- Shade lv3 at 86,-8,-395 (ground -10) x1
- Thornboar lv3 at -42,-4,-607 (ground -6) x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 1 | 8455 | 6 | 4.4 | 93% | 7 | 2% |
| completionist | 4 | 5652 | 40 | 0.5 | 96% | 20 | 2% |
| explorer | 1 | 4699 | 8 | 0.0 | 100% | 21 | 0% |
| grinder | 3 | 4087 | 6 | 1.3 | 98% | 36 | 0% |
| party | 2 | 5391 | 19 | 0.0 | 100% | 24 | 8% |
| social | 1 | 6980 | 11 | 0.0 | 100% | 26 | 1% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 2 | 5145 | 18 | 0.0 | 100% | 18 | 1% |
| Mage | 2 | 6048 | 8 | 2.8 | 95% | 22 | 1% |
| Paladin | 2 | 5318 | 4 | 1.0 | 98% | 26 | 1% |
| Priest | 2 | 6758 | 22 | 0.0 | 100% | 22 | 1% |
| Rogue | 2 | 5212 | 19 | 1.0 | 98% | 27 | 8% |
| Warrior | 2 | 5652 | 19 | 0.0 | 95% | 22 | 3% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Boterw00com | Warrior | completionist | 30.0 | 1->4 | 5581 | 9/15 | 28 | 0 | 18 | 204 |
| Boterw01gri | Paladin | grinder | 30.0 | 1->3 | 4087 | 2/2 | 30 | 0 | 38 | 143 |
| Boterw02exp | Hunter | explorer | 30.0 | 1->3 | 4699 | 8/13 | 17 | 0 | 21 | 200 |
| Boterw03com | Rogue | completionist | 30.0 | 1->3 | 5234 | 10/18 | 19 | 1 | 22 | 226 |
| Boterw04soc | Priest | social | 30.0 | 1->4 | 6980 | 11/16 | 26 | 0 | 26 | 185 |
| Boterw05cas | Mage | casual | 13.6 | 1->3 | 8455 | 6/11 | 13 | 1 | 7 | 134 |
| Boterw06com | Warrior | completionist | 30.0 | 1->4 | 5723 | 10/16 | 26 | 0 | 27 | 174 |
| Boterw07gri | Paladin | grinder | 30.0 | 1->4 | 6549 | 2/2 | 53 | 1 | 13 | 151 |
| Boterw08par | Hunter | party | 30.0 | 1->4 | 5591 | 10/16 | 24 | 0 | 16 | 274 |
| Boterw09par | Rogue | party | 30.0 | 1->3 | 5190 | 9/15 | 17 | 0 | 33 | 212 |
| Boterw10com | Priest | completionist | 30.0 | 1->4 | 6536 | 11/16 | 26 | 0 | 19 | 205 |
| Boterw11gri | Mage | grinder | 30.0 | 1->3 | 3640 | 2/2 | 29 | 1 | 36 | 284 |
