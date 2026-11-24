# Hearthblock playtest: 12 bots, 30 min, brain mix

12 bots on `127.0.0.1:7710`, 339.9 bot-minutes (longest session 30.0 min). Thresholds: docs/playtest/fun-metrics.md.

Machine: load average 15.5 at the start, 14.2 mean, 23.7 at most, on 16 cores.

## Fun checks: 0 fail, 4 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 69.54 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.12 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 144.30 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.31 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.58  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 3.38 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 8.77 s | <= 60 pass, > 90 fail |
| WARN | Share of bots with a reward drought over 5 minutes | 0.17  | <= 0.1 pass, > 0.25 fail |
| WARN | Deaths per hour of play | 3.35 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.96  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 6.41 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.08  | <= 0.15 pass, > 0.3 fail |
| WARN | Share of time travelling | 0.55  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 35.96 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.83  | >= 2 pass, < 1.5 fail |
| PASS | Share of casts that are the most used ability | 0.67  | <= 0.7 pass, > 0.85 fail |
| PASS | Kills lost to someone else's last hit | 0.03  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 1.77 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 13280.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 70 s; first quest turned in: median 7 s; level 2: median 144 s (100% of level-1 starters reached it)
- XP per hour: median 8406 (design pace 6400); 58% of XP from quests; 143 quests turned in, median 3.4 min each
- Rewards: median gap 9 s; median longest drought 146 s; 17% of bots had a drought over 5 min
- Combat: 737 fights, 96% won, median 6.4 s; 627 kills, 17 without credit (6 of them with no other player within 20 blocks); 19 deaths (3.4/h)
- Travel: 36 blocks per reward; money earned 13280 c, spent 0 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 1.8 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 0.5% |
| dead | 0.3% |
| fight | 28.6% |
| gather | 1.8% |
| idle | 6.9% |
| loot | 1.6% |
| rest | 2.6% |
| town | 2.4% |
| travel | 54.6% |
| wait_spawn | 0.5% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 12 | 1.0 | 0 |
| 2 | 12 | 1.0 | 313 |
| 4 | 12 | 2.0 | 661 |
| 6 | 12 | 2.0 | 934 |
| 8 | 12 | 2.0 | 1211 |
| 10 | 11 | 3.0 | 1498 |
| 12 | 11 | 3.0 | 1796 |
| 14 | 11 | 3.0 | 2028 |
| 16 | 11 | 3.0 | 2275 |
| 18 | 11 | 4.0 | 2455 |
| 20 | 11 | 4.0 | 2697 |
| 22 | 11 | 4.0 | 2883 |
| 24 | 11 | 4.0 | 3141 |
| 26 | 11 | 4.0 | 3357 |
| 28 | 11 | 4.0 | 3662 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 9 | 9 | 6.1 |
| broken_tower | 9 | 7 | 5.9 |
| coal_for_the_watch | 9 | 6 | 5.1 |
| cold_hearth | 12 | 12 | 0.1 |
| down_to_water | 2 | 1 | 3.7 |
| dust_of_shadows | 5 | 3 | 16.4 |
| eyes_in_the_dark | 9 | 5 | 2.4 |
| forgotten_graves | 1 | 0 | - |
| garricks_camp | 9 | 7 | 7.6 |
| hearth_stirs | 1 | 0 | - |
| into_barrowwood | 2 | 1 | 5.5 |
| iron_for_hollis | 6 | 3 | 1.7 |
| kindling | 9 | 9 | 0.6 |
| know_the_vale | 5 | 0 | - |
| lay_of_the_land | 7 | 7 | 1.8 |
| lookouts_eye | 9 | 7 | 5.2 |
| meat_for_the_pot | 9 | 9 | 2.4 |
| north_watch | 9 | 9 | 5.6 |
| old_tusk | 9 | 0 | - |
| pincher_supper | 1 | 0 | - |
| shells_for_nets | 1 | 1 | 5.5 |
| silk_for_sails | 7 | 3 | 6.9 |
| stolen_grain | 2 | 1 | 7.9 |
| thieves_in_fields | 2 | 1 | 7.9 |
| thin_the_herd | 12 | 12 | 3.5 |
| tusks_and_tempers | 7 | 2 | 4.3 |
| watchfire | 9 | 8 | 2.8 |
| webs_between_roots | 7 | 3 | 5.5 |
| windmill_breadcrumb | 8 | 2 | 13.7 |
| wool_for_winter | 9 | 8 | 2.2 |
| word_for_rysa | 7 | 7 | 2.1 |
| word_from_east | 3 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x9
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x8
- old_tusk: kill Old Tusk: creature far above our level x8
- dust_of_shadows: collect Shade dust: no source in reach x5
- dust_of_shadows: kill Shade: only at night: waiting for dark x5
- watchfire: collect Coal: no source in reach x5
- iron_for_hollis: collect Raw iron: no source in reach x3
- boar_trouble: goto failed: too many repaths x2
- down_to_water: talk failed: cannot reach NPC: stuck x1
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x1
- thieves_in_fields: goto failed: too many repaths x1

## Deaths by cause

- You fell to a Grazer. x10
- You fell to a Thornboar. x6
- You fell to a Mirepincher. x1
- You fell to a Shade. x1
- You fell to a Websnare. x1

## Fights by creature and outcome

- Grazer: won x469
- Thornboar: won x122
- Shade: won x52
- Websnare: won x44
- Mirepincher: won x13
- Grazer: lost (died) x10
- Thornboar: lost (it fled home and healed) x7
- Ashcloak Thief: won x6
- Thornboar: lost (died) x6
- Ashcloak Thief: lost (cannot reach it: stuck) x3
- Ashcloak Thief: lost (could not reach or hurt it) x1
- Grazer: lost (it fled home and healed) x1
- Mirepincher: lost (died) x1
- Shade: lost (died) x1
- Websnare: lost (died) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 88 | Quick Shot 59%, Barbed Arrow 39%, Recall 2% |
| Mage | 201 | Fireball 60%, Fire Blast 39%, Scholar's Focus 1%, Recall 0% |
| Paladin | 252 | Judging Blow 78%, Rebuke 15%, Holy Mend 4%, Valour 2% |
| Priest | 238 | Smite 71%, Withering Word 27%, Recall 2% |
| Rogue | 92 | Vicious Strike 74%, Finishing Cut 24%, Recall 2% |
| Warrior | 178 | Heroic Blow 50%, Rending Cut 27%, Onrush 12%, Battle Cry 8%, Recall 3% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [112, 400]: 4 events
  - Botzmq00com at [113, -19, 411] going to [122, -19, 443] (stuck; stolen_grain: kill Ashcloak Thief for Sack of Stolen Grain)
  - Botzmq00com at [113, -19, 411] going to [-148, -19, 683] (stuck; down_to_water: talk to Old Wenna)
  - Botzmq00com at [113, -19, 411] going to [144, -17, 438] (stuck; thieves_in_fields: kill Ashcloak Thief)
  - Botzmq00com at [113, -19, 411] going to [0, 0, 0] (gave up: /spawn; stolen_grain: kill Ashcloak Thief for Sack of Stolen Grain)
- cell [-176, -48]: 1 events
  - Botzmq09par at [-166, -11, -43] going to [-166, -11, -47] (repaths; following Botzmq08par)
- cell [-160, 32]: 1 events
  - Botzmq11gri at [-150, -8, 34] going to [-150, -8, 40] (repaths; going to grind)
- cell [-80, -80]: 1 events
  - Botzmq11gri at [-80, -7, -74] going to [-80, -7, -80] (repaths; going to grind)
- cell [-64, -224]: 1 events
  - Botzmq04soc at [-60, -13, -219] going to [-60, -8, -225] (repaths; boar_trouble: go to the Thornboar camp)
- cell [144, -128]: 1 events
  - Botzmq00com at [144, -5, -120] going to [150, 0, -120] (repaths; boar_trouble: go to the Thornboar camp)
- cell [144, 416]: 1 events
  - Botzmq02exp at [144, -18, 425] going to [150, -17, 425] (repaths; thieves_in_fields: go to the Ashcloak Thief camp)

## Notes

- grouped in a party of two x2
- casual session ended x1
- tried /invite Botzmq03com x1
- used /spawn to get unstuck x1

## Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)

- Grazer lv1 at -90,-7,-71 (ground -9) x3
- Grazer lv2 at -74,-7,-95 (ground -9) x2
- Grazer lv2 at -87,-12,178 (ground -14) x2
- Thornboar lv1 at -166,-8,46 (ground -10) x2
- Thornboar lv3 at -93,-14,-218 (ground -16) x2
- Grazer lv1 at -71,-9,87 (ground -11) x1
- Grazer lv1 at -76,-5,-70 (ground -7) x1
- Grazer lv1 at -97,-6,-81 (ground -8) x1
- Grazer lv1 at 121,-11,-271 (ground -13) x1
- Grazer lv1 at 146,-11,-246 (ground -13) x1
- Grazer lv1 at 161,-8,-262 (ground -10) x1
- Grazer lv1 at 187,-10,52 (ground -12) x1
- Grazer lv2 at 144,-11,-252 (ground -13) x1
- Grazer lv2 at 74,-6,-105 (ground -8) x1
- Grazer lv4 at 13,-22,410 (ground -24) x1
- Thornboar lv2 at -25,-5,-605 (ground -7) x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 1 | 9334 | 8 | 0.0 | 100% | 7 | 0% |
| completionist | 4 | 10115 | 75 | 2.0 | 95% | 10 | 2% |
| explorer | 1 | 7280 | 15 | 4.0 | 77% | 18 | 2% |
| grinder | 3 | 7567 | 6 | 8.7 | 97% | 8 | 2% |
| party | 2 | 4305 | 22 | 0.0 | 94% | 11 | 35% |
| social | 1 | 9035 | 17 | 0.0 | 100% | 9 | 2% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 2 | 6069 | 28 | 2.0 | 85% | 13 | 12% |
| Mage | 2 | 8405 | 10 | 4.5 | 98% | 6 | 1% |
| Paladin | 2 | 7722 | 4 | 10.0 | 97% | 8 | 2% |
| Priest | 2 | 8985 | 35 | 0.0 | 100% | 9 | 2% |
| Rogue | 2 | 6900 | 28 | 0.0 | 96% | 11 | 25% |
| Warrior | 2 | 10184 | 38 | 4.0 | 92% | 12 | 2% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botzmq00com | Warrior | completionist | 30.0 | 1->5 | 10181 | 19/25 | 37 | 2 | 14 | 193 |
| Botzmq01gri | Paladin | grinder | 30.0 | 1->4 | 7567 | 2/2 | 139 | 5 | 8 | 75 |
| Botzmq02exp | Hunter | explorer | 30.0 | 1->4 | 7280 | 15/24 | 23 | 2 | 18 | 105 |
| Botzmq03com | Rogue | completionist | 30.0 | 1->5 | 10049 | 19/25 | 35 | 0 | 8 | 120 |
| Botzmq04soc | Priest | social | 30.0 | 1->4 | 9035 | 17/23 | 38 | 0 | 9 | 147 |
| Botzmq05cas | Mage | casual | 9.9 | 1->3 | 9334 | 8/15 | 13 | 0 | 7 | 45 |
| Botzmq06com | Warrior | completionist | 30.0 | 1->5 | 10187 | 19/24 | 38 | 2 | 10 | 145 |
| Botzmq07gri | Paladin | grinder | 30.0 | 1->4 | 7877 | 2/2 | 139 | 5 | 9 | 62 |
| Botzmq08par | Hunter | party | 30.0 | 1->3 | 4858 | 13/21 | 15 | 0 | 8 | 652 |
| Botzmq09par | Rogue | party | 30.0 | 1->3 | 3752 | 9/18 | 6 | 0 | 14 | 313 |
| Botzmq10com | Priest | completionist | 30.0 | 1->4 | 8935 | 18/25 | 30 | 0 | 9 | 173 |
| Botzmq11gri | Mage | grinder | 30.0 | 1->4 | 7477 | 2/2 | 114 | 3 | 5 | 191 |
