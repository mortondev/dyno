# Classes at levels 1 to 5: the swarm, before (main 0f561ab, protocol 23)

12 bots (two per class, mixed brains) for 30 minutes on a fresh world, one server, port 7740. The per-class tables count only fights and deaths at character level 5 or below (the bots end at level 3 to 4). "Presses per 10 s" is ability presses per 10 seconds of fighting. The full fun report follows.

## By class

| Class | Bots | Fights (won) | Kill time, median s | same-level kill s | Damage taken / fight | Casts / fight | Presses per 10 s | Deaths (per hour) | Level after run | XP / hour |
|---|---|---|---|---|---|---|---|---|---|---|
| Warrior | 2 | 72 (94%) | 6.8 | 7.3 | 12.1 | 1.7 | 2.4 | 2 (2.0) | 4/4 | 3444 |
| Paladin | 2 | 120 (83%) | 4.7 | 6.1 | 9.5 | 1.3 | 2.0 | 20 (20.0) | 3/4 | 2650 |
| Hunter | 2 | 34 (94%) | 5.3 | 5.2 | 7.1 | 1.7 | 2.4 | 2 (2.0) | 3/3 | 2115 |
| Rogue | 2 | 43 (100%) | 4.8 | 3.6 | 7.9 | 2.2 | 4.4 | 0 (0.0) | 3/4 | 2266 |
| Priest | 2 | 71 (99%) | 4.7 | 4.6 | 3.3 | 2.4 | 4.9 | 1 (1.0) | 4/4 | 2970 |
| Mage | 2 | 66 (86%) | 3.6 | 4.2 | 2.5 | 2.0 | 4.7 | 9 (15.1) | 1/4 | 2411 |

| Class | Abilities (share of presses) | Deaths by cause |
|---|---|---|
| Warrior | Heroic Blow 54%, Rending Cut 28%, Battle Cry 14%, Onrush 4%, Recall 1% | Websnare (lv4) x1, Websnare (lv3) x1 |
| Paladin | Judging Blow 63%, Holy Mend 19%, Rebuke 17% | Rook Varn (lv3) x6, Ashcloak Bandit (lv3) x6, Ashcloak Cutthroat (lv3) x4, Shade (lv2) x2, Shade (lv3) x2 |
| Hunter | Quick Shot 57%, Barbed Arrow 40%, Recall 3% | Websnare (lv1) x1, Mirepincher (lv3) x1 |
| Rogue | Vicious Strike 79%, Finishing Cut 20%, Recall 1% | none |
| Priest | Smite 76%, Withering Word 21%, Recall 2%, Mend 1% | Shade (lv3) x1 |
| Mage | Fireball 69%, Fire Blast 30%, Scholar's Focus 1% | Ashcloak Bandit (lv3) x6, Shade (lv2) x1, Ashcloak Cutthroat (lv3) x1, Rook Varn (lv3) x1 |

Kill time spread vs mean (5.0 s): Warrior +36%, Paladin -6%, Hunter +6%, Rogue -4%, Priest -6%, Mage -27%

| Class | L1 kill s (n) | L2 kill s (n) | L3 kill s (n) | L4 kill s (n) | L5 kill s (n) |
|---|---|---|---|---|---|
| Warrior | 3.9 (10) | 5.8 (16) | 7.4 (31) | 9.4 (11) | - |
| Paladin | 3.5 (14) | 4.6 (31) | 6.2 (54) | 6.7 (1) | - |
| Hunter | 4.8 (7) | 4.9 (13) | 6.5 (12) | - | - |
| Rogue | 3.2 (8) | 3.6 (15) | 6.8 (14) | 8.8 (6) | - |
| Priest | 3.2 (23) | 4.7 (17) | 4.9 (25) | 4.7 (5) | - |
| Mage | 3.5 (11) | 4.1 (11) | 3.6 (32) | 6.1 (3) | - |

---

# Hearthblock playtest: 12 bots, 30 min, brain mix

12 bots on `127.0.0.1:7740`, 335.7 bot-minutes (longest session 30.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 4 fail, 4 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 79.23 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.68 s | <= 180 s pass, > 300 s fail |
| WARN | Time to level 2 from level 1 (median) | 379.67 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 0.86 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.49  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 3.48 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 20.26 s | <= 60 pass, > 90 fail |
| FAIL | Share of bots with a reward drought over 5 minutes | 0.33  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 6.08 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.91  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 4.75 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.04  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.76  | <= 0.4 pass, > 0.55 fail |
| WARN | Blocks walked per reward | 71.86 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.58  | >= 2 pass, < 1.5 fail |
| PASS | Share of casts that are the most used ability | 0.69  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.20  | <= 0.05 pass, > 0.15 fail |
| WARN | Stuck events per bot-hour | 2.32 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 6888.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 79 s; first quest turned in: median 8 s; level 2: median 380 s (92% of level-1 starters reached it)
- XP per hour: median 5507 (design pace 6400); 49% of XP from quests; 83 quests turned in, median 3.5 min each
- Rewards: median gap 20 s; median longest drought 206 s; 33% of bots had a drought over 5 min
- Combat: 406 fights, 91% won, median 4.8 s; 303 kills, 76 without credit (18 of them with no other player within 20 blocks); 34 deaths (6.1/h)
- Travel: 72 blocks per reward; money earned 6888 c, spent 0 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 2.3 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 0.3% |
| dead | 0.5% |
| fight | 11.9% |
| gather | 0.7% |
| idle | 2.9% |
| loot | 0.7% |
| rest | 0.5% |
| stuck | 0.1% |
| town | 6.6% |
| travel | 75.7% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 12 | 1.0 | 0 |
| 2 | 12 | 1.0 | 141 |
| 4 | 12 | 1.0 | 241 |
| 6 | 11 | 1.0 | 392 |
| 8 | 11 | 2.0 | 526 |
| 10 | 11 | 2.0 | 709 |
| 12 | 11 | 2.0 | 972 |
| 14 | 11 | 3.0 | 1337 |
| 16 | 11 | 3.0 | 1511 |
| 18 | 11 | 3.0 | 1634 |
| 20 | 11 | 3.0 | 1741 |
| 22 | 11 | 3.0 | 1904 |
| 24 | 11 | 3.0 | 1983 |
| 26 | 11 | 3.0 | 2139 |
| 28 | 11 | 3.0 | 2416 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 6 | 4 | 6.6 |
| coal_for_the_watch | 7 | 0 | - |
| cold_hearth | 12 | 12 | 0.1 |
| down_to_water | 4 | 2 | 1.9 |
| dust_of_shadows | 4 | 4 | 0.0 |
| eyes_in_the_dark | 6 | 4 | 7.8 |
| garricks_camp | 6 | 5 | 1.0 |
| into_barrowwood | 2 | 0 | - |
| kindling | 9 | 7 | 4.8 |
| meat_for_the_pot | 9 | 8 | 7.9 |
| north_watch | 8 | 6 | 0.8 |
| pincher_supper | 2 | 0 | - |
| report_to_holt | 6 | 6 | 0.7 |
| silk_for_sails | 5 | 0 | - |
| stolen_grain | 4 | 0 | - |
| thieves_in_fields | 4 | 0 | - |
| thin_the_herd | 12 | 11 | 13.5 |
| tusks_and_tempers | 5 | 2 | 6.6 |
| watchfire | 6 | 0 | - |
| windmill_breadcrumb | 8 | 4 | 9.7 |
| wool_for_winter | 8 | 8 | 3.8 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x7
- kindling: collect Log: no source in reach x7
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x6
- watchfire: collect Coal: no source in reach x6
- down_to_water: talk failed: cannot reach NPC: stuck x1
- eyes_in_the_dark: goto failed: too many repaths x1
- thieves_in_fields: goto failed: stuck x1
- thieves_in_fields: goto failed: too many repaths x1
- thin_the_herd: goto failed: too many repaths x1
- tusks_and_tempers: goto failed: too many repaths x1

## Deaths by cause

- You fell to a Ashcloak Bandit. x12
- You fell to a Rook Varn. x7
- You fell to a Shade. x6
- You fell to a Ashcloak Cutthroat. x5
- You fell to a Websnare. x3
- You fell to a Mirepincher. x1

## Fights by creature and outcome

- Grazer: won x186
- Shade: won x113
- Thornboar: won x43
- Ashcloak Bandit: won x17
- Ashcloak Bandit: lost (died) x12
- Websnare: won x9
- Ashcloak Cutthroat: lost (died) x6
- Rook Varn: lost (died) x6
- Shade: lost (died) x6
- Websnare: lost (died) x3
- Shade: lost (cannot reach it: stuck) x2
- Ashcloak Cutthroat: won x1
- Mirepincher: lost (died) x1
- Mirepincher: won x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 60 | Quick Shot 57%, Barbed Arrow 40%, Recall 3% |
| Mage | 129 | Fireball 69%, Fire Blast 30%, Scholar's Focus 1% |
| Paladin | 161 | Judging Blow 63%, Holy Mend 19%, Rebuke 17% |
| Priest | 176 | Smite 76%, Withering Word 21%, Recall 2%, Mend 1% |
| Rogue | 98 | Vicious Strike 79%, Finishing Cut 20%, Recall 1% |
| Warrior | 123 | Heroic Blow 54%, Rending Cut 28%, Battle Cry 14%, Onrush 4%, Recall 1% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [48, 16]: 4 events
  - Botuqk06com at [52, 0, 20] going to [74, -1, 0] (stuck; eyes_in_the_dark: kill Shade)
  - Botuqk06com at [52, 0, 20] going to [420, 0, -80] (stuck; thieves_in_fields: go to the Ashcloak Bandit camp)
  - Botuqk06com at [52, 0, 20] going to [-148, 0, 683] (stuck; down_to_water: talk to Old Wenna)
  - Botuqk06com at [52, 0, 20] going to [77, -4, 34] (stuck; eyes_in_the_dark: kill Shade)
- cell [304, 304]: 2 events
  - Botuqk07gri at [310, -8, 311] going to [310, -6, 305] (repaths; going to grind)
  - Botuqk11gri at [310, -8, 311] going to [310, -6, 305] (repaths; going to grind)
- cell [-272, 144]: 1 events
  - Botuqk06com at [-260, -4, 144] going to [-260, -7, 150] (repaths; thin_the_herd: go to the Grazer camp)
- cell [-240, -432]: 1 events
  - Botuqk02exp at [-225, 3, -418] going to [-225, -8, -430] (repaths; exploring Garrick's Logging Camp)
- cell [-48, -592]: 1 events
  - Botuqk06com at [-40, 6, -584] going to [-40, 5, -590] (repaths; tusks_and_tempers: go to the Thornboar camp for Boar Tusk)
- cell [-16, -48]: 1 events
  - Botuqk00com at [-4, 0, -45] going to [-4, 0, -49] (repaths; eyes_in_the_dark: look for Shade in the dark)
- cell [160, 64]: 1 events
  - Botuqk09par at [174, -6, 71] going to [171, -6, 69] (repaths; following Botuqk08par)
- cell [224, 480]: 1 events
  - Botuqk08par at [224, -9, 495] going to [230, -6, 495] (repaths; thieves_in_fields: go to the Ashcloak Bandit camp)
- cell [288, -144]: 1 events
  - Botuqk01gri at [290, -7, -144] going to [290, -6, -150] (repaths; going to grind)

## Notes

- grouped in a party of two x2
- casual session ended x1
- tried /invite Botuqk00com x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 1 | 631 | 1 | 0.0 | 100% | 46 | 0% |
| completionist | 4 | 6310 | 44 | 1.5 | 96% | 21 | 1% |
| explorer | 1 | 4746 | 9 | 2.0 | 94% | 36 | 1% |
| grinder | 3 | 5448 | 6 | 19.3 | 84% | 14 | 2% |
| party | 2 | 3481 | 13 | 1.0 | 97% | 15 | 14% |
| social | 1 | 5566 | 10 | 0.0 | 100% | 28 | 1% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 2 | 4230 | 16 | 2.0 | 94% | 24 | 14% |
| Mage | 2 | 3126 | 3 | 15.1 | 86% | 26 | 2% |
| Paladin | 2 | 5300 | 4 | 20.0 | 83% | 18 | 2% |
| Priest | 2 | 5940 | 21 | 1.0 | 99% | 21 | 1% |
| Rogue | 2 | 4531 | 16 | 0.0 | 100% | 20 | 1% |
| Warrior | 2 | 6888 | 23 | 2.0 | 94% | 22 | 1% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botuqk00com | Warrior | completionist | 30.0 | 1->4 | 6306 | 11/19 | 29 | 1 | 20 | 123 |
| Botuqk01gri | Paladin | grinder | 30.0 | 1->4 | 5448 | 2/2 | 45 | 10 | 14 | 195 |
| Botuqk02exp | Hunter | explorer | 30.0 | 1->3 | 4746 | 9/14 | 15 | 1 | 36 | 201 |
| Botuqk03com | Rogue | completionist | 30.0 | 1->4 | 5814 | 10/16 | 26 | 0 | 21 | 162 |
| Botuqk04soc | Priest | social | 30.0 | 1->4 | 5566 | 10/16 | 22 | 0 | 28 | 171 |
| Botuqk05cas | Mage | casual | 5.7 | 1->1 | 631 | 1/4 | 0 | 0 | 46 | 216 |
| Botuqk06com | Warrior | completionist | 30.0 | 1->4 | 7470 | 12/19 | 27 | 1 | 24 | 141 |
| Botuqk07gri | Paladin | grinder | 30.0 | 1->3 | 5152 | 2/2 | 33 | 10 | 21 | 315 |
| Botuqk08par | Hunter | party | 30.0 | 1->3 | 3714 | 7/12 | 17 | 1 | 11 | 919 |
| Botuqk09par | Rogue | party | 30.0 | 1->3 | 3248 | 6/11 | 16 | 0 | 18 | 923 |
| Botuqk10com | Priest | completionist | 30.0 | 1->4 | 6314 | 11/16 | 25 | 1 | 15 | 211 |
| Botuqk11gri | Mage | grinder | 30.0 | 1->4 | 5620 | 2/2 | 48 | 9 | 6 | 367 |
