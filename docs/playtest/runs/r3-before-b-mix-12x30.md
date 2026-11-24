# Hearthblock playtest: 12 bots, 30 min, brain mix

12 bots on `local`, 337.2 bot-minutes (longest session 30.0 min). Thresholds: docs/playtest/fun-metrics.md.

Machine: load average 17.1 at the start, 8.2 mean, 22.0 at most, on 16 cores. Server tick: mean 0.39 ms, p95 1.00 ms, p99 2.00 ms, worst 52 ms over 36091 ticks; the worst minute's p95 was 1.50 ms (budget 50 ms).

## Fun checks: 0 fail, 3 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 63.65 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 6.67 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 172.68 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.08 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.58  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 3.59 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 15.05 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.08  | <= 0.1 pass, > 0.25 fail |
| PASS | Deaths per hour of play | 0.53 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.98  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 5.21 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.04  | <= 0.15 pass, > 0.3 fail |
| WARN | Share of time travelling | 0.48  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 31.17 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 4.42  | >= 2 pass, < 1.5 fail |
| PASS | Share of casts that are the most used ability | 0.49  | <= 0.7 pass, > 0.85 fail |
| PASS | Kills lost to someone else's last hit | 0.01  | <= 0.05 pass, > 0.15 fail |
| WARN | Stuck events per bot-hour | 3.91 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 100.92 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 64 s; first quest turned in: median 7 s; level 2: median 173 s (100% of level-1 starters reached it)
- XP per hour: median 6896 (design pace 6400); 58% of XP from quests; 131 quests turned in, median 3.6 min each
- Rewards: median gap 15 s; median longest drought 127 s; 8% of bots had a drought over 5 min
- Combat: 599 fights, 98% won, median 5.2 s; 542 kills, 5 without credit (3 of them with no other player within 20 blocks); 3 deaths (0.5/h)
- Travel: 31 blocks per reward; money earned 12110 c, spent 120 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 3.9 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 0.5% |
| fight | 19.2% |
| gather | 1.5% |
| idle | 2.6% |
| loot | 1.9% |
| rest | 20.0% |
| stuck | 0.9% |
| town | 4.9% |
| travel | 47.8% |
| wait_spawn | 0.6% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 12 | 1.0 | 0 |
| 2 | 12 | 1.0 | 294 |
| 4 | 12 | 2.0 | 526 |
| 6 | 12 | 2.0 | 765 |
| 8 | 11 | 2.0 | 1073 |
| 10 | 11 | 3.0 | 1311 |
| 12 | 11 | 3.0 | 1668 |
| 14 | 11 | 3.0 | 1959 |
| 16 | 11 | 3.0 | 2236 |
| 18 | 11 | 3.0 | 2417 |
| 20 | 11 | 3.0 | 2672 |
| 22 | 11 | 4.0 | 2951 |
| 24 | 11 | 4.0 | 3156 |
| 26 | 11 | 4.0 | 3342 |
| 28 | 11 | 4.0 | 3480 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 8 | 7 | 5.4 |
| broken_tower | 9 | 7 | 5.8 |
| coal_for_the_watch | 9 | 8 | 5.0 |
| cold_hearth | 12 | 12 | 0.1 |
| down_to_water | 3 | 1 | 0.9 |
| dust_of_shadows | 5 | 2 | 16.6 |
| eyes_in_the_dark | 8 | 5 | 2.8 |
| forgotten_graves | 1 | 0 | - |
| garricks_camp | 8 | 7 | 8.9 |
| into_barrowwood | 4 | 1 | 4.7 |
| iron_for_hollis | 8 | 2 | 1.7 |
| kindling | 9 | 9 | 0.5 |
| know_the_vale | 5 | 0 | - |
| lay_of_the_land | 7 | 6 | 1.8 |
| lookouts_eye | 8 | 5 | 5.1 |
| meat_for_the_pot | 9 | 9 | 2.6 |
| north_watch | 9 | 8 | 6.0 |
| old_tusk | 7 | 2 | 4.5 |
| pincher_supper | 1 | 0 | - |
| silk_for_sails | 7 | 2 | 6.8 |
| stolen_grain | 3 | 0 | - |
| thieves_in_fields | 3 | 0 | - |
| thin_the_herd | 12 | 12 | 4.0 |
| tusks_and_tempers | 7 | 4 | 2.5 |
| watchfire | 8 | 4 | 5.2 |
| webs_between_roots | 5 | 2 | 2.7 |
| windmill_breadcrumb | 8 | 3 | 10.7 |
| wool_for_winter | 9 | 8 | 3.5 |
| word_for_rysa | 7 | 5 | 2.0 |
| word_from_east | 2 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x9
- old_tusk: kill Old Tusk: a group quest: waiting for a party x7
- iron_for_hollis: collect Raw iron: no source in reach x6
- watchfire: collect Coal: no source in reach x6
- dust_of_shadows: collect Shade dust: no source in reach x5
- dust_of_shadows: kill Shade: only at night: waiting for dark x5
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x5
- tusks_and_tempers: kill Old Tusk: a group quest: waiting for a party x5
- boar_trouble: goto failed: too many repaths x1
- broken_tower: goto failed: stuck x1
- dust_of_shadows: kill Barrow Warden: a group quest: waiting for a party x1
- forgotten_graves: talk failed: cannot reach NPC: no path x1
- north_watch: goto failed: stuck x1
- silk_for_sails: goto failed: too many repaths x1

## Deaths by cause

- You fell to a Old Tusk. x2
- You fell to a Barrow Warden. x1

## Fights by creature and outcome

- Grazer: won x385
- Thornboar: won x113
- Shade: won x42
- Websnare: won x35
- Grazer: lost (cannot reach it: no path) x7
- Ashcloak Bandit: won x5
- Ashcloak Thief: won x3
- Old Tusk: lost (died) x2
- Old Tusk: lost (it fled home and healed) x2
- Old Tusk: won x2
- Barrow Warden: lost (died) x1
- Grazer: lost (cannot reach it: stuck) x1
- Shade: lost (cannot reach it: no path) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 104 | Quick Shot 48%, Barbed Arrow 32%, Marked Shot 10%, Hamstring 7%, Recall 3%, Snare Trap 1% |
| Mage | 307 | Frostbolt 38%, Fire Blast 33%, Fireball 28%, Ice Ring 0%, Recall 0% |
| Paladin | 411 | Judging Blow 62%, Rebuke 16%, Seal of Dawn 14%, Holy Mend 6%, Valour 3% |
| Priest | 241 | Smite 67%, Withering Word 22%, Ward of Light 6%, Mend 2%, Recall 2% |
| Rogue | 71 | Vicious Strike 34%, Finishing Cut 30%, Ambush 18%, Low Blow 14%, Recall 3%, Slip Away 1% |
| Warrior | 213 | Heroic Blow 31%, Rending Cut 28%, Onrush 22%, Ground Slam 12%, Recall 3%, Second Wind 3% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [128, 16]: 6 events
  - Bottjp06com at [133, 2, 20] going to [137, -3, 36] (no path; grinding)
  - Bottjp06com at [133, 2, 20] going to [129, -5, 38] (no path; grinding)
  - Bottjp06com at [133, 2, 20] going to [171, -9, 72] (no path; grinding)
  - Bottjp06com at [133, 2, 20] going to [166, -10, 77] (no path; grinding)
- cell [112, 0]: 3 events
  - Bottjp06com at [117, -2, 4] going to [118, -8, 4] (stuck; wool_for_winter: kill Grazer for White)
  - Bottjp06com at [117, -2, 4] going to [17, -2, 183] (stuck; broken_tower: explore the Broken Tower)
  - Bottjp06com at [117, -2, 4] going to [0, -2, -253] (stuck; north_watch: explore the North Watch beacon)
- cell [128, 0]: 3 events
  - Bottjp06com at [130, -2, 11] going to [131, -6, 12] (no path; wool_for_winter: kill Grazer for White)
  - Bottjp06com at [137, 3, 15] going to [147, -5, 15] (no path; grinding)
  - Bottjp06com at [137, 3, 15] going to [0, 0, 0] (gave up: recall; grinding)
- cell [-656, 48]: 1 events
  - Bottjp03com at [-646, -2, 52] going to [-646, 6, 56] (no path; forgotten_graves: talk to Brother Calder)
- cell [-640, 48]: 1 events
  - Bottjp03com at [-636, 2, 48] going to [-625, -1, 37] (no path; dust_of_shadows: kill Shade for Shade dust)
- cell [-480, -272]: 1 events
  - Bottjp02exp at [-474, -9, -266] going to [-480, -9, -266] (repaths; silk_for_sails: go to the Websnare camp for Websnare Silk)
- cell [-384, -16]: 1 events
  - Bottjp02exp at [-372, -3, -1] going to [-384, -9, -1] (repaths; exploring Bluebell Glade)
- cell [-160, -160]: 1 events
  - Bottjp09par at [-152, -13, -146] going to [-149, -13, -144] (repaths; following Bottjp08par)
- cell [-144, -160]: 1 events
  - Bottjp02exp at [-138, -13, -150] going to [-150, -10, -150] (repaths; exploring The Old Quarry)
- cell [-96, -272]: 1 events
  - Bottjp03com at [-93, -16, -257] going to [-97, -16, -257] (repaths; following Bottjp00com for a group quest)
- cell [-64, -224]: 1 events
  - Bottjp06com at [-60, -13, -219] going to [-60, -8, -225] (repaths; boar_trouble: go to the Thornboar camp)
- cell [0, 160]: 1 events
  - Bottjp02exp at [10, -18, 168] going to [10, 0, 180] (repaths; exploring The Broken Tower)

## Notes

- grouped in a party of two x2
- looking for group: invited Bottjp10com x2
- casual session ended x1
- looking for group: invited Bottjp03com x1
- looking for group: invited Bottjp04soc x1
- looking for group: joined Bottjp00com x1
- looking for group: joined Bottjp04soc x1
- recalled home to get unstuck x1
- tried /invite Bottjp03com x1

## Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)

- Grazer lv1 at 142,-4,16 (ground -6) x3
- Grazer lv1 at 122,-6,13 (ground -8) x2
- Grazer lv2 at -97,-6,-81 (ground -8) x2
- Grazer lv2 at 52,-5,-132 (ground -7) x2
- Thornboar lv1 at -162,-8,50 (ground -10) x2
- Thornboar lv2 at 57,-13,-218 (ground -15) x2
- Ashcloak Thief lv3 at 142,-10,457 (ground -12) x1
- Grazer lv1 at -54,-14,167 (ground -16) x1
- Grazer lv1 at -98,-6,-85 (ground -8) x1
- Grazer lv1 at 109,-6,-1 (ground -8) x1
- Grazer lv1 at 121,-11,-271 (ground -13) x1
- Grazer lv1 at 121,-7,9 (ground -9) x1
- Grazer lv1 at 144,-5,10 (ground -7) x1
- Grazer lv2 at -185,-11,-46 (ground -13) x1
- Grazer lv2 at -61,-8,-115 (ground -10) x1
- Grazer lv2 at -90,-7,-71 (ground -9) x1
- Mirepincher lv3 at -185,-19,744 (ground -21) x1
- Shade lv2 at 99,-11,-303 (ground -13) x1
- Thornboar lv2 at -93,5,-370 (ground 3) x1
- Thornboar lv2 at 64,-13,-214 (ground -15) x1
- Thornboar lv3 at -106,6,-372 (ground 4) x1
- Thornboar lv3 at -293,-9,-525 (ground -11) x1
- Thornboar lv3 at -32,-3,-592 (ground -5) x1
- Websnare lv3 at -485,-5,-169 (ground -7) x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 1 | 5908 | 4 | 0.0 | 100% | 7 | 2% |
| completionist | 4 | 10058 | 69 | 1.5 | 92% | 14 | 3% |
| explorer | 1 | 5934 | 12 | 0.0 | 100% | 17 | 2% |
| grinder | 3 | 6953 | 6 | 0.0 | 100% | 15 | 3% |
| party | 2 | 4547 | 23 | 0.0 | 100% | 18 | 10% |
| social | 1 | 8357 | 17 | 0.0 | 98% | 16 | 3% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 2 | 5508 | 24 | 0.0 | 100% | 17 | 8% |
| Mage | 2 | 6430 | 6 | 0.0 | 100% | 9 | 1% |
| Paladin | 2 | 7312 | 4 | 0.0 | 100% | 15 | 4% |
| Priest | 2 | 6966 | 28 | 2.0 | 95% | 16 | 3% |
| Rogue | 2 | 6937 | 31 | 1.0 | 95% | 17 | 4% |
| Warrior | 2 | 10541 | 38 | 0.0 | 92% | 12 | 4% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Bottjp00com | Warrior | completionist | 30.0 | 1->5 | 10829 | 19/24 | 37 | 0 | 13 | 144 |
| Bottjp01gri | Paladin | grinder | 30.0 | 1->4 | 6840 | 2/2 | 110 | 0 | 15 | 47 |
| Bottjp02exp | Hunter | explorer | 30.0 | 1->4 | 5934 | 12/21 | 21 | 0 | 17 | 117 |
| Bottjp03com | Rogue | completionist | 30.0 | 1->5 | 9863 | 20/29 | 22 | 1 | 15 | 202 |
| Bottjp04soc | Priest | social | 30.0 | 1->4 | 8357 | 17/25 | 32 | 0 | 16 | 109 |
| Bottjp05cas | Mage | casual | 7.2 | 1->2 | 5908 | 4/8 | 8 | 0 | 7 | 53 |
| Bottjp06com | Warrior | completionist | 30.0 | 1->5 | 10253 | 19/24 | 43 | 0 | 11 | 147 |
| Bottjp07gri | Paladin | grinder | 30.0 | 1->4 | 7783 | 2/2 | 113 | 0 | 15 | 47 |
| Bottjp08par | Hunter | party | 30.0 | 1->3 | 5082 | 12/23 | 15 | 0 | 17 | 180 |
| Bottjp09par | Rogue | party | 30.0 | 1->3 | 4012 | 11/23 | 4 | 0 | 20 | 646 |
| Bottjp10com | Priest | completionist | 30.0 | 1->4 | 5574 | 11/18 | 25 | 2 | 17 | 137 |
| Bottjp11gri | Mage | grinder | 30.0 | 1->4 | 6953 | 2/2 | 112 | 0 | 12 | 32 |
