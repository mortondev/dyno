# Hearthblock playtest: 12 bots, 30 min, brain mix

12 bots on `local`, 338.2 bot-minutes (longest session 30.0 min). Thresholds: docs/playtest/fun-metrics.md.

Machine: load average 8.0 at the start, 8.4 mean, 16.3 at most, on 16 cores. Server tick: mean 0.44 ms, p95 1.25 ms, p99 2.25 ms, worst 41 ms over 36049 ticks; the worst minute's p95 was 1.75 ms (budget 50 ms).

## Fun checks: 0 fail, 3 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 64.05 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 6.66 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 178.52 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.15 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.60  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.82 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 14.91 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.08  | <= 0.1 pass, > 0.25 fail |
| PASS | Deaths per hour of play | 0.18 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.99  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 5.21 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.04  | <= 0.15 pass, > 0.3 fail |
| WARN | Share of time travelling | 0.47  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 30.34 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 4.25  | >= 2 pass, < 1.5 fail |
| PASS | Share of casts that are the most used ability | 0.53  | <= 0.7 pass, > 0.85 fail |
| PASS | Kills lost to someone else's last hit | 0.00  | <= 0.05 pass, > 0.15 fail |
| WARN | Stuck events per bot-hour | 2.13 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 421.20 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 64 s; first quest turned in: median 7 s; level 2: median 179 s (100% of level-1 starters reached it)
- XP per hour: median 7380 (design pace 6400); 60% of XP from quests; 139 quests turned in, median 2.8 min each
- Rewards: median gap 15 s; median longest drought 146 s; 8% of bots had a drought over 5 min
- Combat: 614 fights, 99% won, median 5.2 s; 558 kills, 1 without credit (1 of them with no other player within 20 blocks); 1 deaths (0.2/h)
- Travel: 30 blocks per reward; money earned 12636 c, spent 30 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 2.1 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 0.5% |
| fight | 20.3% |
| gather | 1.4% |
| idle | 2.8% |
| loot | 1.8% |
| rest | 19.6% |
| stuck | 0.2% |
| town | 5.3% |
| travel | 47.2% |
| wait_spawn | 0.9% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 12 | 1.0 | 0 |
| 2 | 12 | 1.0 | 287 |
| 4 | 12 | 2.0 | 554 |
| 6 | 12 | 2.0 | 831 |
| 8 | 12 | 2.0 | 1090 |
| 10 | 11 | 3.0 | 1374 |
| 12 | 11 | 3.0 | 1707 |
| 14 | 11 | 3.0 | 1947 |
| 16 | 11 | 3.0 | 2243 |
| 18 | 11 | 3.0 | 2474 |
| 20 | 11 | 3.0 | 2784 |
| 22 | 11 | 4.0 | 3078 |
| 24 | 11 | 4.0 | 3311 |
| 26 | 11 | 4.0 | 3504 |
| 28 | 11 | 4.0 | 3741 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 8 | 6 | 3.3 |
| broken_tower | 9 | 7 | 6.0 |
| coal_for_the_watch | 9 | 7 | 4.9 |
| cold_hearth | 12 | 12 | 0.1 |
| down_to_water | 4 | 2 | 2.6 |
| drowned_light | 1 | 0 | - |
| dust_of_shadows | 4 | 0 | - |
| eyes_in_the_dark | 8 | 4 | 4.9 |
| garricks_camp | 8 | 8 | 9.1 |
| into_barrowwood | 4 | 0 | - |
| iron_for_hollis | 6 | 1 | 1.7 |
| kindling | 9 | 9 | 0.6 |
| know_the_vale | 5 | 0 | - |
| lay_of_the_land | 7 | 6 | 1.8 |
| lookouts_eye | 8 | 6 | 2.2 |
| meat_for_the_pot | 9 | 9 | 3.3 |
| north_watch | 9 | 8 | 5.6 |
| old_tusk | 6 | 4 | 5.0 |
| pincher_supper | 2 | 1 | 3.0 |
| shells_for_nets | 1 | 1 | 2.5 |
| silk_for_sails | 8 | 4 | 5.9 |
| stolen_grain | 4 | 0 | - |
| thieves_in_fields | 4 | 1 | 2.2 |
| thin_the_herd | 12 | 12 | 3.6 |
| tusks_and_tempers | 8 | 4 | 1.0 |
| watchfire | 8 | 6 | 0.3 |
| webs_between_roots | 6 | 3 | 4.4 |
| windmill_breadcrumb | 8 | 4 | 12.9 |
| wool_for_winter | 9 | 8 | 2.2 |
| word_for_rysa | 8 | 6 | 1.7 |
| word_from_east | 1 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x7
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x6
- old_tusk: kill Old Tusk: a group quest: waiting for a party x6
- tusks_and_tempers: kill Old Tusk: a group quest: waiting for a party x6
- iron_for_hollis: collect Raw iron: no source in reach x5
- dust_of_shadows: collect Shade dust: no source in reach x4
- dust_of_shadows: kill Shade: only at night: waiting for dark x4
- watchfire: collect Coal: no source in reach x3
- dust_of_shadows: kill Barrow Warden: a group quest: waiting for a party x2
- boar_trouble: goto failed: too many repaths x1
- know_the_vale: goto failed: stuck x1
- old_tusk: goto failed: too many repaths x1
- silk_for_sails: goto failed: stuck x1
- webs_between_roots: goto failed: stuck x1
- webs_between_roots: goto failed: too many repaths x1

## Deaths by cause

- You fell to a Websnare. x1

## Fights by creature and outcome

- Grazer: won x397
- Thornboar: won x90
- Websnare: won x52
- Shade: won x43
- Ashcloak Thief: won x9
- Mirepincher: won x8
- Old Tusk: won x4
- Gloomwisp: won x3
- Thornboar: lost (it fled home and healed) x3
- Websnare: lost (cannot reach it: stuck) x2
- Ashcloak Thief: lost (cannot reach it: too many repaths) x1
- Grazer: lost (cannot reach it: too many repaths) x1
- Websnare: lost (died) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 95 | Quick Shot 46%, Barbed Arrow 31%, Marked Shot 15%, Hamstring 6%, Recall 2% |
| Mage | 308 | Frostbolt 38%, Fireball 32%, Fire Blast 29%, Recall 1%, Ice Ring 0% |
| Paladin | 412 | Judging Blow 61%, Rebuke 16%, Seal of Dawn 14%, Holy Mend 5%, Valour 3%, Recall 0% |
| Priest | 250 | Smite 66%, Withering Word 24%, Ward of Light 6%, Mend 2%, Recall 2% |
| Rogue | 114 | Vicious Strike 32%, Finishing Cut 29%, Ambush 16%, Low Blow 15%, Sneak 5%, Recall 2%, Slip Away 1% |
| Warrior | 237 | Heroic Blow 38%, Rending Cut 24%, Onrush 19%, Ground Slam 12%, Second Wind 4%, Recall 2% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-528, -144]: 4 events
  - Botthk00com at [-523, 3, -130] going to [-555, 3, -129] (stuck; webs_between_roots: kill Websnare)
  - Botthk00com at [-523, 3, -130] going to [-557, 1, -149] (stuck; silk_for_sails: kill Websnare for Websnare Silk)
  - Botthk00com at [-523, 3, -130] going to [-496, -5, -60] (stuck; know_the_vale: explore the Barrow Overlook)
  - Botthk00com at [-523, 3, -130] going to [0, 0, 0] (gave up: recall; silk_for_sails: go to the Websnare camp for Websnare Silk)
- cell [-480, -272]: 1 events
  - Botthk06com at [-474, -9, -266] going to [-480, -2, -266] (repaths; webs_between_roots: go to the Websnare camp)
- cell [-384, -16]: 1 events
  - Botthk02exp at [-372, -3, -1] going to [-384, -8, -1] (repaths; exploring Bluebell Glade)
- cell [-144, 288]: 1 events
  - Botthk02exp at [-130, -18, 288] going to [-130, -10, 300] (repaths; exploring Southfold Farm)
- cell [-64, -224]: 1 events
  - Botthk06com at [-60, -13, -219] going to [-60, -8, -225] (repaths; boar_trouble: go to the Thornboar camp)
- cell [-32, 112]: 1 events
  - Botthk06com at [-27, -6, 115] going to [-26, -15, 115] (repaths; thin_the_herd: kill Grazer)
- cell [-16, -32]: 1 events
  - Botthk09par at [-2, 0, -25] going to [1, 0, -22] (repaths; following Botthk08par)
- cell [32, 224]: 1 events
  - Botthk10com at [44, -17, 233] going to [40, -16, 231] (repaths; following Botthk04soc for a group quest)
- cell [128, 416]: 1 events
  - Botthk00com at [130, -1, 421] going to [128, -2, 421] (repaths; stolen_grain: kill Ashcloak Thief for Sack of Stolen Grain)

## Notes

- grouped in a party of two x2
- looking for group: invited Botthk10com x2
- casual session ended x1
- looking for group: invited Botthk03com x1
- looking for group: invited Botthk08par x1
- looking for group: joined Botthk00com x1
- looking for group: joined Botthk04soc x1
- recalled home to get unstuck x1
- tried /invite Botthk03com x1

## Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)

- Grazer lv2 at -76,-5,-70 (ground -7) x3
- Thornboar lv1 at -166,-8,59 (ground -10) x3
- Thornboar lv1 at 163,-4,-140 (ground -6) x2
- Grazer lv1 at -137,-9,-16 (ground -11) x1
- Grazer lv1 at -182,-11,-64 (ground -13) x1
- Grazer lv1 at -97,-6,-81 (ground -8) x1
- Grazer lv1 at 122,-6,13 (ground -8) x1
- Grazer lv1 at 147,-5,-280 (ground -7) x1
- Grazer lv1 at 158,-6,-266 (ground -8) x1
- Grazer lv1 at 71,-6,-87 (ground -8) x1
- Grazer lv1 at 88,-6,-89 (ground -8) x1
- Grazer lv2 at -104,-7,-91 (ground -9) x1
- Grazer lv2 at -71,-9,87 (ground -11) x1
- Grazer lv2 at 11,-10,-119 (ground -12) x1
- Grazer lv2 at 122,-6,13 (ground -8) x1
- Grazer lv2 at 124,-10,-269 (ground -12) x1
- Grazer lv2 at 57,-7,-152 (ground -9) x1
- Thornboar lv1 at -173,-8,41 (ground -10) x1
- Thornboar lv1 at 142,-5,-138 (ground -7) x1
- Thornboar lv1 at 144,-5,-137 (ground -7) x1
- Thornboar lv3 at -293,-9,-525 (ground -11) x1
- Thornboar lv3 at -75,-13,-215 (ground -15) x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 1 | 7692 | 5 | 0.0 | 100% | 15 | 2% |
| completionist | 4 | 9884 | 78 | 0.0 | 97% | 13 | 4% |
| explorer | 1 | 5722 | 11 | 0.0 | 100% | 17 | 1% |
| grinder | 3 | 6960 | 6 | 0.0 | 100% | 15 | 4% |
| party | 2 | 4923 | 22 | 0.0 | 97% | 18 | 6% |
| social | 1 | 8431 | 17 | 2.0 | 97% | 16 | 3% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 2 | 5668 | 22 | 0.0 | 97% | 18 | 4% |
| Mage | 2 | 7326 | 7 | 0.0 | 99% | 13 | 2% |
| Paladin | 2 | 6977 | 4 | 0.0 | 100% | 15 | 5% |
| Priest | 2 | 9214 | 36 | 1.0 | 98% | 16 | 3% |
| Rogue | 2 | 7001 | 29 | 0.0 | 100% | 15 | 4% |
| Warrior | 2 | 11177 | 41 | 0.0 | 95% | 13 | 5% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botthk00com | Warrior | completionist | 30.0 | 1->5 | 12861 | 22/30 | 47 | 0 | 12 | 129 |
| Botthk01gri | Paladin | grinder | 30.0 | 1->4 | 6887 | 2/2 | 110 | 0 | 15 | 43 |
| Botthk02exp | Hunter | explorer | 30.0 | 1->4 | 5722 | 11/21 | 14 | 0 | 17 | 155 |
| Botthk03com | Rogue | completionist | 30.0 | 1->5 | 9771 | 18/23 | 33 | 0 | 14 | 170 |
| Botthk04soc | Priest | social | 30.0 | 1->4 | 8431 | 17/23 | 29 | 1 | 16 | 144 |
| Botthk05cas | Mage | casual | 8.2 | 1->2 | 7692 | 5/8 | 13 | 0 | 15 | 44 |
| Botthk06com | Warrior | completionist | 30.0 | 1->4 | 9493 | 19/25 | 32 | 0 | 13 | 148 |
| Botthk07gri | Paladin | grinder | 30.0 | 1->4 | 7068 | 2/2 | 112 | 0 | 15 | 42 |
| Botthk08par | Hunter | party | 30.0 | 1->4 | 5614 | 11/22 | 19 | 0 | 19 | 220 |
| Botthk09par | Rogue | party | 30.0 | 1->3 | 4232 | 11/23 | 7 | 0 | 17 | 558 |
| Botthk10com | Priest | completionist | 30.0 | 1->5 | 9997 | 19/24 | 31 | 0 | 16 | 188 |
| Botthk11gri | Mage | grinder | 30.0 | 1->4 | 6960 | 2/2 | 111 | 0 | 11 | 51 |
