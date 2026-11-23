# Hearthblock playtest: 20 bots, 12 min, brain mix

20 bots on `127.0.0.1:7540`, 229.1 bot-minutes (longest session 12.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 5 fail, 2 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 56.12 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.82 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 113.77 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.51 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.68  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 1.42 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 7.59 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.10  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 4.45 /h | <= 1.5 pass, > 4 fail |
| WARN | Fights won | 0.83  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 5.76 s | <= 25 pass, > 40 fail (and >= 5) |
| FAIL | Share of time idle, stuck, waiting for spawns or dead | 0.31  | <= 0.15 pass, > 0.3 fail |
| PASS | Share of time travelling | 0.20  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 23.59 blocks | <= 60 pass, > 120 fail |
| WARN | Distinct abilities used per fighting bot | 1.90  | >= 2 pass, < 1.5 fail |
| FAIL | Share of casts that are the most used ability | 0.90  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.22  | <= 0.05 pass, > 0.15 fail |
| FAIL | Stuck events per bot-hour | 32.74 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Copper earned per copper spent | 6446.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 56 s; first quest turned in: median 8 s; level 2: median 114 s (100% of level-1 starters reached it)
- XP per hour: median 9676 (design pace 6400); 68% of XP from quests; 107 quests turned in, median 1.4 min each
- Rewards: median gap 8 s; median longest drought 187 s; 10% of bots had a drought over 5 min
- Combat: 578 fights, 83% won, median 5.8 s; 231 kills, 64 lost to another player's last hit; 17 deaths (4.5/h)
- Travel: 24 blocks per reward; money earned 6446 c, spent 0 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 32.7 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| dead | 0.4% |
| fight | 38.9% |
| gather | 5.1% |
| idle | 4.4% |
| loot | 2.5% |
| rest | 1.3% |
| stuck | 1.2% |
| town | 1.9% |
| travel | 19.8% |
| wait_spawn | 24.6% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 20 | 1.0 | 0 |
| 1 | 20 | 1.0 | 199 |
| 2 | 20 | 2.0 | 368 |
| 3 | 20 | 2.0 | 542 |
| 4 | 20 | 2.0 | 687 |
| 5 | 20 | 2.0 | 864 |
| 6 | 20 | 2.0 | 983 |
| 7 | 20 | 2.0 | 1125 |
| 8 | 19 | 2.0 | 1253 |
| 9 | 18 | 2.0 | 1413 |
| 10 | 17 | 3.0 | 1593 |
| 11 | 17 | 3.0 | 1809 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 6 | 0 | - |
| coal_for_the_watch | 16 | 13 | 3.3 |
| cold_hearth | 20 | 20 | 0.1 |
| down_to_water | 4 | 0 | - |
| eyes_in_the_dark | 6 | 0 | - |
| garricks_camp | 6 | 2 | 0.2 |
| iron_for_hollis | 12 | 1 | 7.0 |
| kindling | 16 | 16 | 1.7 |
| meat_for_the_pot | 16 | 12 | 6.3 |
| north_watch | 9 | 6 | 0.4 |
| report_to_holt | 6 | 6 | 0.1 |
| silk_for_sails | 2 | 0 | - |
| stolen_grain | 4 | 0 | - |
| thieves_in_fields | 4 | 0 | - |
| thin_the_herd | 20 | 13 | 9.9 |
| tusks_and_tempers | 2 | 0 | - |
| watchfire | 6 | 3 | 3.0 |
| windmill_breadcrumb | 11 | 4 | 1.2 |
| wool_for_winter | 12 | 11 | 0.6 |
| word_from_east | 1 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x15
- iron_for_hollis: gather Raw iron: needs mining 25: raising it on easier nodes x12
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x2
- thin_the_herd: goto failed: too many repaths x2
- boar_trouble: goto failed: stuck x1
- iron_for_hollis: collect Raw iron: Requires mining 25 (have 5) x1
- iron_for_hollis: collect Raw iron: Requires mining 25 (have 7) x1
- thin_the_herd: goto failed: stuck x1

## Deaths by cause

- You fell to a Shade. x11
- You drowned. x4
- You fell to a Thornboar. x2

## Fights by creature and outcome

- Grazer: won x397
- Shade: won x47
- Thornboar: won x22
- Grazer: lost (cannot reach it: no path) x21
- Ashcloak Bandit: lost (no progress: it hardly took damage) x16
- Grazer: lost (cannot reach it: no path (target on a tree canopy)) x14
- Shade: lost (died) x9
- Grazer: lost (cannot reach it: stuck) x6
- Mirepincher: won x5
- Shade: lost (could not reach or hurt it) x5
- Ashcloak Bandit: won x4
- Websnare: lost (no progress: it hardly took damage) x4
- Ashcloak Bandit: lost (died) x3
- Ashcloak Cutthroat: lost (no progress: it hardly took damage) x3
- Grazer: lost (could not reach or hurt it) x3
- Grazer: lost (died) x3
- Shade: lost (cannot reach it: stuck) x3
- Mirepincher: lost (could not reach or hurt it) x2
- Mirepincher: lost (no progress: it hardly took damage) x2
- Thornboar: lost (no progress: it hardly took damage) x2
- Ashcloak Cutthroat: lost (died) x1
- Candlekin: won x1
- Mirepincher: lost (it fled home and healed) x1
- Thornboar: lost (cannot reach it: stuck) x1
- Thornboar: lost (could not reach or hurt it) x1

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [0, 0]: 12 events
  - Botvvm00com at [12, 0, 1] going to [12, -7, 1] (no path; iron_for_hollis: gather Raw iron)
  - Botvvm02exp at [12, 0, 1] going to [12, -7, 1] (no path; iron_for_hollis: gather Raw iron)
  - Botvvm03com at [12, 0, 1] going to [12, -7, 1] (no path; iron_for_hollis: gather Raw iron)
  - Botvvm04soc at [13, 0, 1] going to [13, -10, 1] (no path; iron_for_hollis: gather Raw iron)
- cell [0, -112]: 9 events
  - Botvvm01gri at [7, -7, -100] going to [9, 1, -101] (no path; thin_the_herd: kill Grazer)
  - Botvvm04soc at [9, -7, -98] going to [10, 0, -100] (no path (target on a tree canopy); looting)
  - Botvvm04soc at [15, -6, -97] going to [16, 0, -99] (no path; looting)
  - Botvvm05cas at [9, -7, -98] going to [10, 1, -100] (no path; looting)
- cell [48, -32]: 9 events
  - Botvvm01gri at [54, -6, -23] going to [52, 1, -22] (no path (target on a tree canopy); grinding)
  - Botvvm01gri at [48, -5, -24] going to [49, 1, -25] (no path (target on a tree canopy); grinding)
  - Botvvm06com at [53, -6, -24] going to [52, 1, -25] (no path (target on a tree canopy); meat_for_the_pot: kill Grazer for Raw meat)
  - Botvvm07soc at [51, -6, -22] going to [52, 1, -24] (no path (target on a tree canopy); thin_the_herd: kill Grazer)
- cell [16, -96]: 8 events
  - Botvvm01gri at [24, -7, -90] going to [23, 1, -91] (no path; grinding)
  - Botvvm03com at [21, -8, -86] going to [21, -3, -87] (no path; thin_the_herd: kill Grazer)
  - Botvvm08gri at [22, -6, -90] going to [21, 0, -90] (no path (target on a tree canopy); grinding)
  - Botvvm13com at [22, -6, -93] going to [23, 1, -91] (no path; meat_for_the_pot: kill Grazer for Raw meat)
- cell [16, -64]: 7 events
  - Botvvm00com at [29, -6, -61] going to [28, 1, -62] (no path; thin_the_herd: kill Grazer)
  - Botvvm01gri at [29, -6, -61] going to [28, 1, -62] (no path; grinding)
  - Botvvm06com at [31, -5, -60] going to [30, 1, -61] (no path (target on a tree canopy); grinding)
  - Botvvm08gri at [24, -6, -52] going to [25, 1, -50] (no path; looting)
- cell [32, -32]: 7 events
  - Botvvm01gri at [46, -5, -20] going to [48, 0, -20] (no path; grinding)
  - Botvvm03com at [39, -4, -30] going to [39, -9, -30] (no path; iron_for_hollis: gather Raw iron)
  - Botvvm06com at [40, -4, -30] going to [39, -9, -30] (no path; iron_for_hollis: gather Raw iron)
  - Botvvm07soc at [39, -4, -31] going to [38, -13, -29] (no path; iron_for_hollis: gather Raw iron)
- cell [64, 80]: 7 events
  - Botvvm03com at [74, -13, 83] going to [75, -17, 65] (stuck; iron_for_hollis: gather Raw iron)
  - Botvvm11gri at [67, -13, 86] going to [63, -8, 18] (stuck; grinding)
  - Botvvm11gri at [67, -13, 86] going to [56, -10, 29] (stuck; thin_the_herd: kill Grazer)
  - Botvvm11gri at [67, -13, 86] going to [58, -5, -33] (stuck; grinding)
- cell [0, -96]: 6 events
  - Botvvm01gri at [15, -6, -95] going to [13, -1, -96] (no path; grinding)
  - Botvvm03com at [14, -6, -93] going to [13, 0, -95] (no path; thin_the_herd: kill Grazer)
  - Botvvm05cas at [3, -6, -90] going to [3, -21, -90] (no path; iron_for_hollis: gather Raw iron)
  - Botvvm06com at [3, -6, -90] going to [3, -21, -90] (no path; iron_for_hollis: gather Raw iron)
- cell [0, -48]: 6 events
  - Botvvm00com at [1, -5, -43] going to [1, -13, -43] (no path; watchfire: gather Coal)
  - Botvvm02exp at [1, -5, -43] going to [1, -13, -43] (no path; watchfire: gather Coal)
  - Botvvm06com at [0, -6, -45] going to [1, -13, -43] (no path; iron_for_hollis: gather Raw iron)
  - Botvvm10com at [0, -6, -45] going to [1, -13, -43] (no path; iron_for_hollis: gather Raw iron)
- cell [16, -48]: 6 events
  - Botvvm01gri at [28, -4, -44] going to [27, 1, -45] (no path; grinding)
  - Botvvm06com at [29, -4, -44] going to [28, 0, -46] (no path; thin_the_herd: kill Grazer)
  - Botvvm08gri at [26, -4, -43] going to [27, 1, -45] (no path; looting)
  - Botvvm11gri at [25, -5, -46] going to [27, 0, -47] (no path; looting)
- cell [96, -32]: 6 events
  - Botvvm00com at [107, -7, -25] going to [106, -13, -25] (no path; coal_for_the_watch: gather Coal)
  - Botvvm06com at [107, -7, -25] going to [106, -13, -25] (no path; coal_for_the_watch: gather Coal)
  - Botvvm10com at [107, -7, -25] going to [106, -13, -25] (no path; coal_for_the_watch: gather Coal)
  - Botvvm12exp at [107, -7, -25] going to [106, -13, -25] (no path; coal_for_the_watch: gather Coal)
- cell [32, -48]: 4 events
  - Botvvm00com at [42, -3, -34] going to [39, -9, -30] (stuck; watchfire: gather Coal)
  - Botvvm00com at [42, -3, -34] going to [101, -10, -14] (stuck; boar_trouble: kill Thornboar)
  - Botvvm00com at [42, -3, -34] going to [119, -29, -14] (stuck; eyes_in_the_dark: kill Shade)
  - Botvvm00com at [42, -3, -34] going to [0, 0, 0] (gave up: /spawn; watchfire: gather Coal)

## Notes

- casual session ended x3
- used /spawn to get unstuck x3
- tried /invite Botvvm06com x1
- tried /invite Botvvm08gri x1
- tried /invite Botvvm13com x1
- tried /invite Botvvm16com x1

## Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)

- Grazer lv1 at 25,1,-48 (ground -6) x20
- Grazer lv1 at 27,1,-60 (ground -7) x20
- Grazer lv1 at 32,0,-59 (ground -6) x20
- Grazer lv2 at 53,0,-26 (ground -6) x20
- Grazer lv3 at 14,-1,-93 (ground -7) x20
- Grazer lv3 at 22,0,-95 (ground -7) x20
- Grazer lv4 at 11,0,-103 (ground -8) x20
- Grazer lv4 at 11,0,-99 (ground -8) x20
- Grazer lv4 at 11,1,-102 (ground -8) x20
- Grazer lv4 at 12,0,-103 (ground -8) x20
- Grazer lv4 at 12,0,-99 (ground -7) x20
- Grazer lv4 at 13,0,-99 (ground -7) x20
- Grazer lv4 at 17,-1,-94 (ground -8) x20
- Grazer lv4 at 17,-1,-96 (ground -8) x20
- Grazer lv4 at 9,0,-100 (ground -8) x20
- Grazer lv4 at 9,0,-102 (ground -8) x20
- Thornboar lv4 at -12,-1,-118 (ground -7) x20
- Thornboar lv4 at -13,3,-114 (ground -6) x20
- Thornboar lv4 at -17,0,-84 (ground -6) x20
- Grazer lv1 at 27,-2,-63 (ground -8) x19
- Grazer lv1 at 28,-1,-45 (ground -5) x19
- Grazer lv1 at 48,0,-21 (ground -6) x19
- Grazer lv1 at 51,0,-26 (ground -6) x19
- Grazer lv1 at 53,-1,-24 (ground -7) x19
- Grazer lv1 at 53,-1,-25 (ground -6) x19

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 4 | 6564 | 15 | 0.0 | 90% | 10 | 55% |
| completionist | 6 | 12826 | 45 | 1.7 | 81% | 8 | 27% |
| explorer | 2 | 9264 | 11 | 2.5 | 94% | 8 | 40% |
| grinder | 4 | 7743 | 8 | 10.0 | 80% | 9 | 9% |
| social | 4 | 12293 | 28 | 7.5 | 83% | 6 | 34% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 3 | 13622 | 20 | 6.7 | 85% | 7 | 21% |
| Mage | 3 | 9596 | 16 | 6.7 | 85% | 5 | 42% |
| Paladin | 4 | 7163 | 15 | 6.7 | 70% | 11 | 33% |
| Priest | 3 | 8456 | 19 | 0.0 | 93% | 5 | 45% |
| Rogue | 3 | 9986 | 13 | 4.2 | 74% | 9 | 24% |
| Warrior | 4 | 11956 | 24 | 2.5 | 87% | 10 | 20% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botvvm00com | Warrior | completionist | 12.0 | 1->4 | 16721 | 10/17 | 12 | 0 | 8 | 152 |
| Botvvm01gri | Paladin | grinder | 12.0 | 1->3 | 6900 | 2/2 | 18 | 3 | 12 | 296 |
| Botvvm02exp | Hunter | explorer | 12.0 | 1->4 | 13622 | 8/14 | 11 | 0 | 7 | 290 |
| Botvvm03com | Rogue | completionist | 12.0 | 1->3 | 9986 | 6/11 | 9 | 2 | 4 | 109 |
| Botvvm04soc | Priest | social | 12.0 | 1->3 | 8456 | 4/7 | 12 | 0 | 5 | 256 |
| Botvvm05cas | Mage | casual | 12.0 | 1->3 | 9596 | 6/9 | 8 | 0 | 4 | 332 |
| Botvvm06com | Warrior | completionist | 12.0 | 1->4 | 15667 | 9/17 | 13 | 0 | 11 | 116 |
| Botvvm07soc | Paladin | social | 12.0 | 1->3 | 9981 | 6/11 | 7 | 2 | 6 | 186 |
| Botvvm08gri | Hunter | grinder | 12.0 | 1->3 | 9756 | 2/2 | 30 | 1 | 7 | 61 |
| Botvvm09cas | Rogue | casual | 7.1 | 1->3 | 12832 | 5/8 | 5 | 0 | 12 | 43 |
| Botvvm10com | Priest | completionist | 12.0 | 1->4 | 19506 | 11/17 | 15 | 0 | 3 | 167 |
| Botvvm11gri | Mage | grinder | 12.0 | 1->3 | 7240 | 2/2 | 17 | 3 | 5 | 472 |
| Botvvm12exp | Warrior | explorer | 12.0 | 1->2 | 4905 | 3/5 | 5 | 1 | 9 | 187 |
| Botvvm13com | Paladin | completionist | 12.0 | 1->3 | 7425 | 5/8 | 5 | 0 | 11 | 292 |
| Botvvm14soc | Hunter | social | 12.0 | 1->4 | 16966 | 10/16 | 13 | 3 | 6 | 102 |
| Botvvm15cas | Rogue | casual | 9.4 | 1->2 | 3532 | 2/5 | 3 | 0 | 9 | 167 |
| Botvvm16com | Priest | completionist | 12.0 | 1->3 | 7511 | 4/7 | 11 | 0 | 8 | 256 |
| Botvvm17soc | Mage | social | 12.0 | 1->4 | 14606 | 8/14 | 14 | 1 | 5 | 254 |
| Botvvm18gri | Warrior | grinder | 12.0 | 1->3 | 8246 | 2/2 | 22 | 1 | 14 | 93 |
| Botvvm19cas | Paladin | casual | 8.6 | 1->2 | 3163 | 2/5 | 1 | 0 | 12 | 231 |
