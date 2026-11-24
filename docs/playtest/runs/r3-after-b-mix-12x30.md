# Hearthblock playtest: 12 bots, 30 min, brain mix

12 bots on `local`, 338.1 bot-minutes (longest session 30.0 min). Thresholds: docs/playtest/fun-metrics.md.

Machine: load average 11.4 at the start, 5.9 mean, 11.4 at most, on 16 cores. Server tick: mean 0.38 ms, p95 1.00 ms, p99 1.75 ms, worst 19 ms over 36106 ticks; the worst minute's p95 was 1.25 ms (budget 50 ms).

## Fun checks: 0 fail, 5 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 67.41 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 6.66 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 172.54 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.15 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.60  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 3.13 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 14.42 s | <= 60 pass, > 90 fail |
| WARN | Share of bots with a reward drought over 5 minutes | 0.17  | <= 0.1 pass, > 0.25 fail |
| WARN | Deaths per hour of play | 1.77 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.97  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 5.16 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.04  | <= 0.15 pass, > 0.3 fail |
| WARN | Share of time travelling | 0.44  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 27.87 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 4.17  | >= 2 pass, < 1.5 fail |
| PASS | Share of casts that are the most used ability | 0.52  | <= 0.7 pass, > 0.85 fail |
| PASS | Kills lost to someone else's last hit | 0.02  | <= 0.05 pass, > 0.15 fail |
| WARN | Stuck events per bot-hour | 2.31 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 188.76 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 67 s; first quest turned in: median 7 s; level 2: median 173 s (100% of level-1 starters reached it)
- XP per hour: median 7362 (design pace 6400); 60% of XP from quests; 134 quests turned in, median 3.1 min each
- Rewards: median gap 14 s; median longest drought 94 s; 17% of bots had a drought over 5 min
- Combat: 630 fights, 97% won, median 5.2 s; 577 kills, 9 without credit (4 of them with no other player within 20 blocks); 10 deaths (1.8/h)
- Travel: 28 blocks per reward; money earned 13213 c, spent 70 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 2.3 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 0.6% |
| dead | 0.2% |
| fight | 20.9% |
| gather | 1.5% |
| idle | 3.5% |
| loot | 2.0% |
| rest | 20.5% |
| stuck | 0.2% |
| town | 6.0% |
| travel | 44.1% |
| wait_spawn | 0.6% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 12 | 1.0 | 0 |
| 2 | 12 | 1.0 | 295 |
| 4 | 12 | 2.0 | 579 |
| 6 | 12 | 2.0 | 808 |
| 8 | 12 | 2.0 | 1050 |
| 10 | 11 | 3.0 | 1411 |
| 12 | 11 | 3.0 | 1708 |
| 14 | 11 | 3.0 | 1963 |
| 16 | 11 | 3.0 | 2191 |
| 18 | 11 | 3.0 | 2450 |
| 20 | 11 | 3.0 | 2727 |
| 22 | 11 | 4.0 | 2971 |
| 24 | 11 | 4.0 | 3226 |
| 26 | 11 | 4.0 | 3473 |
| 28 | 11 | 4.0 | 3729 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 8 | 5 | 4.2 |
| broken_tower | 9 | 7 | 6.1 |
| coal_for_the_watch | 9 | 6 | 4.2 |
| cold_hearth | 12 | 12 | 0.1 |
| down_to_water | 3 | 3 | 1.8 |
| drowned_light | 1 | 1 | 1.2 |
| dust_of_shadows | 5 | 4 | 0.5 |
| eyes_in_the_dark | 8 | 5 | 6.3 |
| garricks_camp | 8 | 6 | 9.4 |
| hearth_stirs | 1 | 0 | - |
| into_barrowwood | 5 | 0 | - |
| iron_for_hollis | 5 | 1 | 1.6 |
| kindling | 9 | 9 | 0.6 |
| know_the_vale | 5 | 0 | - |
| lay_of_the_land | 7 | 7 | 1.7 |
| lookouts_eye | 8 | 4 | 7.0 |
| meat_for_the_pot | 9 | 9 | 2.4 |
| north_watch | 8 | 8 | 5.6 |
| old_tusk | 5 | 2 | 7.0 |
| pincher_supper | 3 | 1 | 5.0 |
| shells_for_nets | 3 | 2 | 3.6 |
| silk_for_sails | 6 | 3 | 6.1 |
| stolen_grain | 3 | 0 | - |
| thieves_in_fields | 3 | 1 | 2.3 |
| thin_the_herd | 12 | 11 | 3.2 |
| tusks_and_tempers | 8 | 5 | 10.5 |
| watchfire | 8 | 6 | 0.3 |
| webs_between_roots | 3 | 2 | 2.3 |
| windmill_breadcrumb | 7 | 3 | 9.9 |
| wool_for_winter | 9 | 8 | 2.5 |
| word_for_rysa | 6 | 3 | 1.1 |
| word_from_east | 1 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x9
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x8
- tusks_and_tempers: kill Old Tusk: a group quest: waiting for a party x7
- old_tusk: kill Old Tusk: a group quest: waiting for a party x5
- iron_for_hollis: collect Raw iron: no source in reach x4
- watchfire: collect Coal: no source in reach x3
- dust_of_shadows: collect Shade dust: no source in reach x2
- dust_of_shadows: kill Shade: only at night: waiting for dark x2
- webs_between_roots: goto failed: too many repaths x2
- dust_of_shadows: kill Barrow Warden: a group quest: waiting for a party x1
- hearth_stirs: collect Shade dust: no source in reach x1
- hearth_stirs: kill Barrow Warden: a group quest: waiting for a party x1
- hearth_stirs: kill Shade: only at night: waiting for dark x1
- into_barrowwood: goto failed: stuck x1
- word_for_rysa: goto failed: stuck x1

## Deaths by cause

- You fell to a Old Tusk. x8
- You fell to a Gloomwisp. x2

## Fights by creature and outcome

- Grazer: won x408
- Thornboar: won x82
- Shade: won x56
- Websnare: won x30
- Mirepincher: won x28
- Old Tusk: lost (died) x8
- Ashcloak Thief: won x5
- Websnare: lost (it fled home and healed) x4
- Gloomwisp: lost (died) x2
- Old Tusk: won x2
- Grazer: lost (cannot reach it: too many repaths) x1
- Grazer: lost (it fled home and healed) x1
- Thornboar: lost (it fled home and healed) x1
- Websnare: lost (cannot reach it: stuck) x1
- Websnare: lost (could not reach or hurt it) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 115 | Quick Shot 44%, Barbed Arrow 26%, Marked Shot 16%, Hamstring 10%, Recall 4% |
| Mage | 308 | Frostbolt 39%, Fire Blast 32%, Fireball 28%, Ice Ring 1% |
| Paladin | 405 | Judging Blow 63%, Rebuke 17%, Seal of Dawn 14%, Holy Mend 3%, Valour 3%, Recall 0% |
| Priest | 241 | Smite 65%, Withering Word 26%, Ward of Light 6%, Recall 3%, Mend 1% |
| Rogue | 134 | Vicious Strike 43%, Finishing Cut 24%, Ambush 16%, Low Blow 13%, Recall 2%, Sneak 1%, Slip Away 1% |
| Warrior | 243 | Heroic Blow 37%, Rending Cut 23%, Onrush 23%, Ground Slam 13%, Second Wind 3%, Recall 1% |

## Unreachable NPCs

- Sergeant Holt (cannot reach NPC: stuck) x1

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-512, -208]: 3 events
  - Botvvx00com at [-499, -7, -198] going to [-474, -7, -177] (stuck; silk_for_sails: kill Websnare for Websnare Silk)
  - Botvvx00com at [-499, -7, -198] going to [-616, -7, 56] (stuck; into_barrowwood: explore the Old Barrow)
  - Botvvx00com at [-499, -7, -198] going to [-295, -7, -407] (stuck; word_for_rysa: explore Hunter's Rest)
- cell [-480, -272]: 2 events
  - Botvvx04soc at [-474, -9, -266] going to [-480, -2, -266] (repaths; webs_between_roots: go to the Websnare camp)
  - Botvvx06com at [-474, -9, -266] going to [-480, -9, -266] (repaths; webs_between_roots: go to the Websnare camp)
- cell [-384, -16]: 1 events
  - Botvvx02exp at [-372, -3, -1] going to [-384, -9, -1] (repaths; exploring Bluebell Glade)
- cell [-144, -160]: 1 events
  - Botvvx02exp at [-138, -13, -150] going to [-150, -10, -150] (repaths; exploring The Old Quarry)
- cell [-16, 112]: 1 events
  - Botvvx06com at [-10, -11, 125] going to [-10, -18, 125] (repaths; thin_the_herd: kill Grazer)
- cell [0, -256]: 1 events
  - Botvvx09par at [0, -14, -243] going to [1, -14, -247] (repaths; following Botvvx08par)
- cell [0, 160]: 1 events
  - Botvvx02exp at [10, -18, 168] going to [10, 0, 180] (repaths; exploring The Broken Tower)
- cell [80, -288]: 1 events
  - Botvvx06com at [89, -16, -280] going to [85, -16, -279] (repaths; following Botvvx03com for a group quest)
- cell [128, -144]: 1 events
  - Botvvx00com at [130, -5, -134] going to [2, -14, -251] (stuck; visiting Sergeant Holt: hand in boar_trouble)
- cell [208, -48]: 1 events
  - Botvvx02exp at [208, -12, -42] going to [220, 0, -42] (repaths; exploring The Wrecked Caravan)

## Notes

- grouped in a party of two x2
- casual session ended x1
- looking for group: invited Botvvx06com x1
- looking for group: invited Botvvx08par x1
- looking for group: joined Botvvx03com x1
- tried /invite Botvvx03com x1

## Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)

- Grazer lv1 at -108,-7,-83 (ground -9) x2
- Grazer lv1 at 77,-4,-70 (ground -6) x2
- Grazer lv2 at -137,-9,-16 (ground -11) x2
- Grazer lv2 at 134,-7,-1 (ground -9) x2
- Grazer lv1 at -104,-7,-91 (ground -9) x1
- Grazer lv1 at -208,-12,-62 (ground -14) x1
- Grazer lv1 at -209,-12,-49 (ground -14) x1
- Grazer lv1 at -97,-6,-81 (ground -8) x1
- Grazer lv1 at 142,-4,16 (ground -6) x1
- Grazer lv1 at 161,-6,-262 (ground -8) x1
- Grazer lv1 at 46,-5,-129 (ground -7) x1
- Grazer lv1 at 87,-9,95 (ground -11) x1
- Grazer lv2 at -45,-14,158 (ground -16) x1
- Grazer lv2 at 127,-7,-12 (ground -9) x1
- Grazer lv2 at 140,-4,19 (ground -6) x1
- Grazer lv2 at 52,-5,-132 (ground -7) x1
- Grazer lv2 at 63,-7,-151 (ground -9) x1
- Grazer lv2 at 92,-8,78 (ground -10) x1
- Grazer lv4 at 4,-17,417 (ground -19) x1
- Shade lv3 at 78,-13,-300 (ground -15) x1
- Shade lv4 at -545,1,-146 (ground -1) x1
- Thornboar lv2 at -90,4,-394 (ground 2) x1
- Thornboar lv2 at 146,-5,-137 (ground -7) x1
- Thornboar lv2 at 57,-13,-218 (ground -15) x1
- Thornboar lv3 at 57,-13,-218 (ground -15) x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 1 | 7366 | 6 | 0.0 | 92% | 10 | 0% |
| completionist | 4 | 11090 | 77 | 1.0 | 96% | 12 | 4% |
| explorer | 1 | 5478 | 12 | 0.0 | 100% | 16 | 1% |
| grinder | 3 | 7124 | 6 | 0.0 | 100% | 14 | 3% |
| party | 2 | 4352 | 15 | 8.0 | 83% | 17 | 11% |
| social | 1 | 9020 | 18 | 0.0 | 95% | 17 | 2% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 2 | 5985 | 23 | 4.0 | 91% | 16 | 4% |
| Mage | 2 | 7173 | 8 | 0.0 | 98% | 10 | 1% |
| Paladin | 2 | 7241 | 4 | 0.0 | 100% | 14 | 4% |
| Priest | 2 | 10398 | 39 | 1.0 | 96% | 16 | 2% |
| Rogue | 2 | 6397 | 23 | 4.0 | 92% | 15 | 9% |
| Warrior | 2 | 11090 | 37 | 1.0 | 94% | 11 | 5% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botvvx00com | Warrior | completionist | 30.0 | 1->5 | 10637 | 16/26 | 48 | 1 | 12 | 84 |
| Botvvx01gri | Paladin | grinder | 30.0 | 1->4 | 7359 | 2/2 | 111 | 0 | 14 | 47 |
| Botvvx02exp | Hunter | explorer | 30.0 | 1->4 | 5478 | 12/23 | 11 | 0 | 16 | 162 |
| Botvvx03com | Rogue | completionist | 30.0 | 1->5 | 10583 | 19/24 | 36 | 0 | 11 | 168 |
| Botvvx04soc | Priest | social | 30.0 | 1->4 | 9020 | 18/24 | 32 | 0 | 17 | 99 |
| Botvvx05cas | Mage | casual | 8.2 | 1->2 | 7366 | 6/14 | 10 | 0 | 10 | 57 |
| Botvvx06com | Warrior | completionist | 30.0 | 1->5 | 11543 | 21/26 | 36 | 0 | 9 | 125 |
| Botvvx07gri | Paladin | grinder | 30.0 | 1->4 | 7124 | 2/2 | 117 | 0 | 14 | 37 |
| Botvvx08par | Hunter | party | 30.0 | 1->4 | 6492 | 11/18 | 23 | 4 | 16 | 390 |
| Botvvx09par | Rogue | party | 30.0 | 1->2 | 2212 | 4/7 | 5 | 4 | 18 | 508 |
| Botvvx10com | Priest | completionist | 30.0 | 1->5 | 11775 | 21/29 | 34 | 1 | 15 | 90 |
| Botvvx11gri | Mage | grinder | 30.0 | 1->4 | 6981 | 2/2 | 114 | 0 | 11 | 66 |
