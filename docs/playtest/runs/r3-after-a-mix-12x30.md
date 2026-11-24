# Hearthblock playtest: 12 bots, 30 min, brain mix

12 bots on `local`, 344.2 bot-minutes (longest session 30.0 min). Thresholds: docs/playtest/fun-metrics.md.

Machine: load average 6.3 at the start, 7.1 mean, 14.7 at most, on 16 cores. Server tick: mean 0.43 ms, p95 1.25 ms, p99 2.25 ms, worst 20 ms over 36062 ticks; the worst minute's p95 was 1.75 ms (budget 50 ms).

## Fun checks: 0 fail, 5 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 62.69 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 6.66 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 174.08 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.24 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.60  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 3.07 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 13.34 s | <= 60 pass, > 90 fail |
| WARN | Share of bots with a reward drought over 5 minutes | 0.17  | <= 0.1 pass, > 0.25 fail |
| WARN | Deaths per hour of play | 1.92 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.98  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 5.51 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.04  | <= 0.15 pass, > 0.3 fail |
| WARN | Share of time travelling | 0.49  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 30.60 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 4.33  | >= 2 pass, < 1.5 fail |
| PASS | Share of casts that are the most used ability | 0.49  | <= 0.7 pass, > 0.85 fail |
| PASS | Kills lost to someone else's last hit | 0.02  | <= 0.05 pass, > 0.15 fail |
| WARN | Stuck events per bot-hour | 3.14 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 164.75 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 63 s; first quest turned in: median 7 s; level 2: median 174 s (100% of level-1 starters reached it)
- XP per hour: median 7913 (design pace 6400); 60% of XP from quests; 139 quests turned in, median 3.1 min each
- Rewards: median gap 13 s; median longest drought 123 s; 17% of bots had a drought over 5 min
- Combat: 634 fights, 98% won, median 5.5 s; 567 kills, 11 without credit (4 of them with no other player within 20 blocks); 11 deaths (1.9/h)
- Travel: 31 blocks per reward; money earned 14004 c, spent 85 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 3.1 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 0.6% |
| dead | 0.2% |
| fight | 21.4% |
| gather | 2.0% |
| idle | 2.8% |
| loot | 2.1% |
| rest | 20.0% |
| stuck | 0.2% |
| town | 1.5% |
| travel | 48.6% |
| wait_spawn | 0.7% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 12 | 1.0 | 0 |
| 2 | 12 | 1.0 | 281 |
| 4 | 12 | 2.0 | 534 |
| 6 | 12 | 2.0 | 839 |
| 8 | 12 | 2.0 | 1029 |
| 10 | 12 | 2.5 | 1322 |
| 12 | 12 | 3.0 | 1633 |
| 14 | 12 | 3.0 | 1994 |
| 16 | 11 | 3.0 | 2281 |
| 18 | 11 | 3.0 | 2490 |
| 20 | 11 | 3.0 | 2738 |
| 22 | 11 | 4.0 | 3095 |
| 24 | 11 | 4.0 | 3330 |
| 26 | 11 | 4.0 | 3564 |
| 28 | 11 | 4.0 | 3729 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 9 | 7 | 6.2 |
| broken_tower | 9 | 8 | 6.9 |
| coal_for_the_watch | 9 | 8 | 6.1 |
| cold_hearth | 12 | 12 | 0.1 |
| down_to_water | 1 | 1 | 2.2 |
| drowned_light | 1 | 1 | 1.8 |
| dust_of_shadows | 5 | 4 | 0.0 |
| eyes_in_the_dark | 9 | 5 | 2.8 |
| forgotten_graves | 1 | 0 | - |
| garricks_camp | 9 | 5 | 10.9 |
| hearth_stirs | 2 | 0 | - |
| into_barrowwood | 4 | 1 | 10.7 |
| iron_for_hollis | 8 | 3 | 4.9 |
| kindling | 9 | 9 | 0.5 |
| know_the_vale | 5 | 0 | - |
| lay_of_the_land | 8 | 7 | 1.7 |
| lookouts_eye | 9 | 4 | 5.2 |
| meat_for_the_pot | 9 | 9 | 2.8 |
| north_watch | 9 | 9 | 6.8 |
| old_tusk | 7 | 2 | 2.9 |
| pincher_supper | 1 | 1 | 2.2 |
| shells_for_nets | 1 | 1 | 2.9 |
| silk_for_sails | 5 | 3 | 4.9 |
| stolen_grain | 1 | 0 | - |
| thieves_in_fields | 1 | 1 | 9.5 |
| thin_the_herd | 12 | 12 | 3.7 |
| tusks_and_tempers | 9 | 5 | 10.9 |
| watchfire | 9 | 6 | 0.9 |
| webs_between_roots | 4 | 2 | 2.8 |
| windmill_breadcrumb | 8 | 1 | 12.0 |
| wool_for_winter | 9 | 8 | 3.3 |
| word_for_rysa | 5 | 4 | 1.1 |
| word_from_east | 3 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x9
- iron_for_hollis: collect Raw iron: no source in reach x7
- tusks_and_tempers: kill Old Tusk: a group quest: waiting for a party x7
- old_tusk: kill Old Tusk: a group quest: waiting for a party x5
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x4
- watchfire: collect Coal: no source in reach x4
- boar_trouble: goto failed: too many repaths x2
- dust_of_shadows: collect Shade dust: no source in reach x1
- dust_of_shadows: kill Barrow Warden: a group quest: waiting for a party x1
- dust_of_shadows: kill Shade: only at night: waiting for dark x1
- forgotten_graves: talk failed: cannot reach NPC: no path x1
- hearth_stirs: collect Shade dust: no source in reach x1
- hearth_stirs: kill Barrow Warden: a group quest: waiting for a party x1
- hearth_stirs: kill Shade: only at night: waiting for dark x1
- tusks_and_tempers: goto failed: too many repaths x1
- webs_between_roots: goto failed: stuck x1

## Deaths by cause

- You fell to a Old Tusk. x10
- You fell to a Gloomwisp. x1

## Fights by creature and outcome

- Grazer: won x394
- Thornboar: won x118
- Shade: won x47
- Websnare: won x39
- Mirepincher: won x14
- Old Tusk: lost (died) x10
- Ashcloak Thief: won x5
- Old Tusk: won x2
- Thornboar: lost (it fled home and healed) x2
- Gloomwisp: lost (died) x1
- Grazer: lost (cannot reach it: too many repaths) x1
- Grazer: lost (could not reach or hurt it) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 149 | Quick Shot 49%, Barbed Arrow 28%, Marked Shot 11%, Hamstring 9%, Recall 3% |
| Mage | 313 | Frostbolt 36%, Fireball 33%, Fire Blast 29%, Recall 1%, Ice Ring 1% |
| Paladin | 382 | Judging Blow 62%, Rebuke 15%, Seal of Dawn 14%, Holy Mend 6%, Valour 3%, Recall 0% |
| Priest | 241 | Smite 66%, Withering Word 25%, Ward of Light 5%, Mend 2%, Recall 2% |
| Rogue | 197 | Vicious Strike 34%, Finishing Cut 24%, Low Blow 19%, Ambush 15%, Sneak 7%, Recall 1% |
| Warrior | 193 | Heroic Blow 38%, Rending Cut 23%, Onrush 23%, Ground Slam 11%, Second Wind 3%, Recall 2% |

## Unreachable NPCs

- Foreman Garrick (cannot reach NPC: stuck) x1
- Rysa (cannot reach NPC: stuck) x1

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-64, -224]: 4 events
  - Botegf01gri at [-60, -13, -219] going to [-60, -8, -225] (repaths; going to grind)
  - Botegf03com at [-60, -13, -219] going to [-60, -14, -225] (repaths; tusks_and_tempers: go to the Thornboar camp for Boar Tusk)
  - Botegf04soc at [-60, -13, -219] going to [-60, -8, -225] (repaths; boar_trouble: go to the Thornboar camp)
  - Botegf11gri at [-60, -13, -219] going to [-60, -14, -225] (repaths; going to grind)
- cell [-560, -160]: 3 events
  - Botegf00com at [-547, 2, -146] going to [-486, -9, -232] (stuck; visiting Rysa: hand in webs_between_roots)
  - Botegf00com at [-547, 2, -146] going to [-223, 2, -426] (stuck; visiting Foreman Garrick: hand in silk_for_sails)
  - Botegf00com at [-547, 2, -146] going to [0, 0, 0] (gave up: recall; visiting Foreman Garrick: hand in silk_for_sails)
- cell [-656, 48]: 1 events
  - Botegf00com at [-651, -2, 53] going to [-649, 6, 58] (no path; forgotten_graves: talk to Brother Calder)
- cell [-496, -288]: 1 events
  - Botegf06com at [-482, -8, -274] going to [-490, -6, -185] (stuck; webs_between_roots: go to the Websnare camp)
- cell [-304, -432]: 1 events
  - Botegf02exp at [-295, -6, -419] going to [-295, -6, -407] (repaths; exploring Hunter's Rest)
- cell [-160, 32]: 1 events
  - Botegf02exp at [-156, -9, 40] going to [-150, -6, 40] (repaths; boar_trouble: go to the Thornboar camp)
- cell [-80, -16]: 1 events
  - Botegf09par at [-69, 0, -4] going to [-72, 0, -6] (repaths; following Botegf08par)
- cell [-80, 80]: 1 events
  - Botegf06com at [-70, -5, 89] going to [-69, -11, 89] (repaths; thin_the_herd: kill Grazer)
- cell [-48, -256]: 1 events
  - Botegf09par at [-35, -14, -249] going to [-39, -14, -250] (repaths; following Botegf08par)
- cell [-16, -32]: 1 events
  - Botegf09par at [-2, 0, -21] going to [-3, 0, -17] (repaths; following Botegf08par)
- cell [0, 160]: 1 events
  - Botegf02exp at [10, -18, 168] going to [10, 0, 180] (repaths; exploring The Broken Tower)
- cell [48, -80]: 1 events
  - Botegf09par at [49, 0, -75] going to [52, 0, -73] (repaths; following Botegf08par)

## Notes

- grouped in a party of two x2
- casual session ended x1
- looking for group: invited Botegf06com x1
- looking for group: invited Botegf08par x1
- looking for group: joined Botegf04soc x1
- recalled home to get unstuck x1
- tried /invite Botegf03com x1

## Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)

- Grazer lv1 at 137,-11,-252 (ground -13) x4
- Grazer lv1 at -137,-9,-16 (ground -11) x3
- Grazer lv1 at -76,-5,-70 (ground -7) x3
- Grazer lv1 at 11,-11,-150 (ground -13) x3
- Grazer lv1 at 165,-7,-255 (ground -9) x2
- Grazer lv1 at 74,-6,-105 (ground -8) x2
- Thornboar lv2 at -150,-7,41 (ground -9) x2
- Thornboar lv2 at -161,-8,48 (ground -10) x2
- Websnare lv3 at -491,-6,-197 (ground -8) x2
- Grazer lv1 at 172,-7,-253 (ground -9) x1
- Grazer lv1 at 63,-7,-151 (ground -9) x1
- Grazer lv1 at 71,-6,-87 (ground -8) x1
- Grazer lv2 at -137,-9,-16 (ground -11) x1
- Grazer lv2 at -182,-11,-64 (ground -13) x1
- Mirepincher lv3 at -15,-29,542 (ground -31) x1
- Thornboar lv1 at 142,-5,-138 (ground -7) x1
- Thornboar lv2 at -291,-9,-538 (ground -11) x1
- Thornboar lv2 at 142,-5,-102 (ground -7) x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 1 | 8875 | 11 | 0.0 | 93% | 9 | 3% |
| completionist | 4 | 10576 | 77 | 0.5 | 99% | 11 | 4% |
| explorer | 1 | 5058 | 7 | 0.0 | 95% | 15 | 1% |
| grinder | 3 | 6796 | 6 | 0.0 | 100% | 14 | 3% |
| party | 2 | 4411 | 20 | 10.0 | 78% | 18 | 7% |
| social | 1 | 10959 | 18 | 0.0 | 100% | 17 | 1% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 2 | 5072 | 19 | 5.0 | 88% | 15 | 2% |
| Mage | 2 | 7836 | 13 | 0.0 | 99% | 10 | 2% |
| Paladin | 2 | 6815 | 4 | 0.0 | 100% | 15 | 4% |
| Priest | 2 | 9876 | 36 | 0.0 | 100% | 14 | 2% |
| Rogue | 2 | 8075 | 28 | 6.0 | 91% | 16 | 9% |
| Warrior | 2 | 10576 | 39 | 0.0 | 99% | 10 | 4% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botegf00com | Warrior | completionist | 30.0 | 1->5 | 9609 | 18/25 | 32 | 0 | 11 | 172 |
| Botegf01gri | Paladin | grinder | 30.0 | 1->4 | 7034 | 2/2 | 113 | 0 | 14 | 52 |
| Botegf02exp | Hunter | explorer | 30.0 | 1->3 | 5058 | 7/16 | 20 | 0 | 15 | 154 |
| Botegf03com | Rogue | completionist | 30.0 | 1->5 | 12415 | 20/26 | 47 | 1 | 12 | 132 |
| Botegf04soc | Priest | social | 30.0 | 1->5 | 10959 | 18/24 | 41 | 0 | 17 | 87 |
| Botegf05cas | Mage | casual | 14.2 | 1->3 | 8875 | 11/18 | 13 | 0 | 9 | 89 |
| Botegf06com | Warrior | completionist | 30.0 | 1->5 | 11543 | 21/26 | 38 | 0 | 9 | 113 |
| Botegf07gri | Paladin | grinder | 30.0 | 1->4 | 6595 | 2/2 | 98 | 0 | 15 | 56 |
| Botegf08par | Hunter | party | 30.0 | 1->3 | 5086 | 12/18 | 19 | 5 | 16 | 527 |
| Botegf09par | Rogue | party | 30.0 | 1->3 | 3736 | 8/18 | 7 | 5 | 20 | 596 |
| Botegf10com | Priest | completionist | 30.0 | 1->4 | 8792 | 18/26 | 30 | 0 | 12 | 136 |
| Botegf11gri | Mage | grinder | 30.0 | 1->4 | 6796 | 2/2 | 109 | 0 | 10 | 45 |
