# Hearthblock playtest: 20 bots, 12 min, brain mix

20 bots on `127.0.0.1:7540`, 234.5 bot-minutes (longest session 12.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 4 fail, 1 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 52.70 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.79 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 97.02 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 2.14 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.71  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 1.11 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 9.51 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.10  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 8.96 /h | <= 1.5 pass, > 4 fail |
| WARN | Fights won | 0.82  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 5.81 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.09  | <= 0.15 pass, > 0.3 fail |
| PASS | Share of time travelling | 0.18  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 28.70 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.20  | >= 2 pass, < 1.5 fail |
| FAIL | Share of casts that are the most used ability | 0.87  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.20  | <= 0.05 pass, > 0.15 fail |
| FAIL | Stuck events per bot-hour | 29.17 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 7378.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 53 s; first quest turned in: median 8 s; level 2: median 97 s (95% of level-1 starters reached it)
- XP per hour: median 13694 (design pace 6400); 71% of XP from quests; 116 quests turned in, median 1.1 min each
- Rewards: median gap 10 s; median longest drought 131 s; 10% of bots had a drought over 5 min
- Combat: 732 fights, 82% won, median 5.8 s; 254 kills, 64 without credit (7 of them with no other player within 20 blocks); 35 deaths (9.0/h)
- Travel: 29 blocks per reward; money earned 7378 c, spent 0 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 29.2 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 1.2% |
| dead | 0.8% |
| fight | 56.7% |
| gather | 6.6% |
| idle | 2.6% |
| loot | 3.4% |
| rest | 3.5% |
| stuck | 2.8% |
| town | 1.4% |
| travel | 17.8% |
| wait_spawn | 3.3% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 20 | 1.0 | 0 |
| 1 | 20 | 1.0 | 193 |
| 2 | 20 | 2.0 | 358 |
| 3 | 20 | 2.0 | 510 |
| 4 | 20 | 2.0 | 701 |
| 5 | 20 | 2.0 | 872 |
| 6 | 20 | 2.0 | 1022 |
| 7 | 19 | 2.0 | 1215 |
| 8 | 19 | 3.0 | 1421 |
| 9 | 19 | 3.0 | 1634 |
| 10 | 19 | 3.0 | 1906 |
| 11 | 19 | 3.0 | 2126 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 10 | 0 | - |
| coal_for_the_watch | 14 | 10 | 2.0 |
| cold_hearth | 20 | 18 | 0.1 |
| down_to_water | 4 | 0 | - |
| dust_of_shadows | 1 | 1 | 0.3 |
| eyes_in_the_dark | 10 | 1 | 4.5 |
| garricks_camp | 10 | 7 | 1.2 |
| iron_for_hollis | 9 | 0 | - |
| kindling | 14 | 14 | 1.4 |
| meat_for_the_pot | 16 | 12 | 4.5 |
| north_watch | 10 | 10 | 0.3 |
| report_to_holt | 10 | 10 | 0.1 |
| silk_for_sails | 7 | 0 | - |
| stolen_grain | 4 | 0 | - |
| thieves_in_fields | 4 | 0 | - |
| thin_the_herd | 18 | 14 | 7.4 |
| tusks_and_tempers | 7 | 0 | - |
| watchfire | 10 | 5 | 0.0 |
| windmill_breadcrumb | 10 | 4 | 1.3 |
| wool_for_winter | 12 | 10 | 1.1 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x14
- iron_for_hollis: gather Raw iron: needs mining 25: raising it on easier nodes x9
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x7
- thin_the_herd: goto failed: stuck x3
- thin_the_herd: goto failed: too many repaths x3
- watchfire: collect Coal: no source in reach x3
- iron_for_hollis: collect Raw iron: Requires mining 25 (have 20) x1
- iron_for_hollis: collect Raw iron: Requires mining 25 (have 4) x1
- iron_for_hollis: collect Raw iron: Requires mining 25 (have 5) x1
- iron_for_hollis: collect Raw iron: Requires mining 25 (have 6) x1

## Deaths by cause

- You fell to a Shade. x20
- You fell to a Ashcloak Bandit. x5
- You fell to a Gloomwisp. x4
- You fell to a Websnare. x3
- You fell to a Candlekin. x1
- You fell to a Mirepincher. x1
- You fell to a Thornboar. x1

## Fights by creature and outcome

- Grazer: won x485
- Thornboar: lost (no progress: it hardly took damage) x58
- Shade: won x54
- Ashcloak Bandit: won x25
- Thornboar: won x25
- Shade: lost (died) x15
- Thornboar: lost (cannot reach it: stuck) x9
- Mirepincher: won x8
- Grazer: lost (cannot reach it: no path) x7
- Ashcloak Bandit: lost (died) x6
- Thornboar: lost (died) x5
- Websnare: won x5
- Shade: lost (cannot reach it: no path) x4
- Grazer: lost (cannot reach it: no path (target on a tree canopy)) x3
- Mirepincher: lost (died) x3
- Thornboar: lost (could not reach or hurt it) x3
- Websnare: lost (died) x3
- Ashcloak Bandit: lost (no progress: it hardly took damage) x2
- Gloomwisp: lost (could not reach or hurt it) x2
- Gloomwisp: lost (died) x2
- Mirepincher: lost (could not reach or hurt it) x2
- Ashcloak Bandit: lost (could not reach or hurt it) x1
- Candlekin: lost (could not reach or hurt it) x1
- Candlekin: lost (died) x1
- Grazer: lost (could not reach or hurt it) x1

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [0, -96]: 9 events
  - Bottbt04soc at [10, -7, -89] going to [10, -7, -95] (repaths; thin_the_herd: go to the Grazer camp)
  - Bottbt04soc at [3, -6, -90] going to [3, -21, -90] (no path; coal_for_the_watch: gather Coal)
  - Bottbt06com at [15, -7, -92] going to [17, -16, -93] (no path; coal_for_the_watch: gather Coal)
  - Bottbt06com at [3, -6, -90] going to [3, -21, -90] (no path; coal_for_the_watch: gather Coal)
- cell [0, -112]: 8 events
  - Bottbt04soc at [8, -7, -100] going to [9, 0, -102] (no path (target on a tree canopy); looting)
  - Bottbt04soc at [12, -6, -97] going to [13, -1, -95] (no path (target on a tree canopy); looting)
  - Bottbt06com at [10, -7, -101] going to [9, 0, -103] (no path; meat_for_the_pot: kill Grazer for Raw meat)
  - Bottbt06com at [7, -6, -97] going to [7, -17, -97] (no path; coal_for_the_watch: gather Coal)
- cell [0, -48]: 8 events
  - Bottbt02exp at [1, -5, -43] going to [1, -13, -43] (no path; iron_for_hollis: gather Raw iron)
  - Bottbt04soc at [1, -5, -43] going to [1, -13, -43] (no path; watchfire: gather Coal)
  - Bottbt05cas at [0, -6, -45] going to [1, -13, -43] (no path; watchfire: gather Coal)
  - Bottbt06com at [1, -5, -43] going to [1, -13, -43] (no path; watchfire: gather Coal)
- cell [16, -64]: 8 events
  - Bottbt01gri at [25, -6, -50] going to [26, 1, -50] (no path; grinding)
  - Bottbt05cas at [25, -6, -50] going to [27, 0, -49] (no path; looting)
  - Bottbt06com at [29, -6, -62] going to [30, 1, -62] (no path (target on a tree canopy); thin_the_herd: kill Grazer)
  - Bottbt07gri at [27, -6, -62] going to [29, 1, -62] (no path (target on a tree canopy); grinding)
- cell [48, -32]: 8 events
  - Bottbt01gri at [52, -5, -27] going to [51, 1, -26] (no path; grinding)
  - Bottbt02exp at [53, -6, -24] going to [52, 1, -26] (no path; looting)
  - Bottbt05cas at [49, -5, -23] going to [51, 1, -24] (no path; looting)
  - Bottbt05cas at [58, -6, -24] going to [58, -3, -30] (repaths; thin_the_herd: go to the Grazer camp)
- cell [-16, -96]: 7 events
  - Bottbt04soc at [-4, -4, -89] going to [-4, -22, -89] (no path; coal_for_the_watch: gather Coal)
  - Bottbt06com at [-4, -4, -89] going to [-4, -22, -89] (no path; coal_for_the_watch: gather Coal)
  - Bottbt10com at [-4, -4, -89] going to [-4, -22, -89] (no path; iron_for_hollis: gather Raw iron)
  - Bottbt12exp at [-4, -4, -89] going to [-4, -22, -89] (no path; coal_for_the_watch: gather Coal)
- cell [0, 0]: 7 events
  - Bottbt00com at [12, 0, 1] going to [12, -7, 1] (no path; iron_for_hollis: gather Raw iron)
  - Bottbt03com at [12, 0, 1] going to [12, -7, 1] (no path; iron_for_hollis: gather Raw iron)
  - Bottbt04soc at [13, 0, 1] going to [13, -10, 1] (no path; iron_for_hollis: gather Raw iron)
  - Bottbt05cas at [12, 0, 1] going to [12, -7, 1] (no path; iron_for_hollis: gather Raw iron)
- cell [96, -64]: 7 events
  - Bottbt03com at [102, -9, -55] going to [58, -3, -30] (stuck; thin_the_herd: go to the Grazer camp)
  - Bottbt03com at [102, -9, -55] going to [100, -10, -49] (stuck; meat_for_the_pot: kill Thornboar for Raw meat)
  - Bottbt03com at [99, -9, -56] going to [101, -10, -14] (stuck; meat_for_the_pot: kill Thornboar for Raw meat)
  - Bottbt03com at [98, -9, -57] going to [58, -3, -30] (stuck; thin_the_herd: go to the Grazer camp)
- cell [0, -16]: 6 events
  - Bottbt02exp at [11, 0, -2] going to [11, -8, -2] (no path; coal_for_the_watch: gather Coal)
  - Bottbt02exp at [10, 1, -7] going to [10, -13, -6] (no path; iron_for_hollis: gather Raw iron)
  - Bottbt03com at [0, 0, -8] going to [-2, -12, -8] (no path; iron_for_hollis: gather Raw iron)
  - Bottbt06com at [11, 0, -2] going to [11, -8, -2] (no path; coal_for_the_watch: gather Coal)
- cell [16, -96]: 6 events
  - Bottbt04soc at [16, -7, -89] going to [17, -1, -87] (no path; looting)
  - Bottbt04soc at [23, -6, -94] going to [23, -14, -94] (no path; coal_for_the_watch: gather Coal)
  - Bottbt04soc at [18, -6, -95] going to [18, -15, -95] (no path; coal_for_the_watch: gather Coal)
  - Bottbt06com at [18, -7, -87] going to [20, -1, -87] (no path (target on a tree canopy); thin_the_herd: kill Grazer)
- cell [-16, -16]: 5 events
  - Bottbt04soc at [-2, 0, -8] going to [-2, -12, -8] (no path; iron_for_hollis: gather Raw iron)
  - Bottbt06com at [-2, 0, -8] going to [-2, -12, -8] (no path; coal_for_the_watch: gather Coal)
  - Bottbt14soc at [-2, 0, -8] going to [-2, -12, -8] (no path; coal_for_the_watch: gather Coal)
  - Bottbt15cas at [-2, 0, -8] going to [-2, -12, -8] (no path; coal_for_the_watch: gather Coal)
- cell [-16, -80]: 4 events
  - Bottbt02exp at [-1, -7, -73] going to [-1, -25, -73] (no path; watchfire: gather Coal)
  - Bottbt13com at [-1, -7, -73] going to [-1, -25, -73] (no path; watchfire: gather Coal)
  - Bottbt14soc at [-1, -7, -73] going to [-1, -25, -73] (no path; watchfire: gather Coal)
  - Bottbt16com at [-12, -5, -76] going to [-14, 1, -77] (no path; tusks_and_tempers: kill Thornboar for Boar Tusk)

## Notes

- grouped in a party of two x4
- casual session ended x2
- tried /invite Bottbt00com x1
- tried /invite Bottbt13com x1
- used /spawn to get unstuck x1

## Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)

- Grazer lv1 at 25,1,-48 (ground -6) x20
- Grazer lv1 at 30,1,-59 (ground -6) x20
- Grazer lv1 at 53,0,-26 (ground -6) x20
- Grazer lv2 at 10,0,-99 (ground -8) x20
- Grazer lv2 at 13,-1,-95 (ground -7) x20
- Grazer lv2 at 31,-1,-47 (ground -5) x20
- Grazer lv2 at 31,1,-62 (ground -7) x20
- Grazer lv2 at 38,1,-57 (ground -6) x20
- Grazer lv2 at 53,-1,-25 (ground -6) x20
- Grazer lv3 at 13,-1,-92 (ground -7) x20
- Grazer lv3 at 20,-1,-86 (ground -8) x20
- Grazer lv4 at 17,-1,-96 (ground -8) x20
- Thornboar lv5 at -90,-6,25 (ground -9) x20
- Grazer lv1 at 25,1,-45 (ground -6) x19
- Grazer lv1 at 27,-1,-49 (ground -6) x19
- Grazer lv1 at 30,1,-62 (ground -7) x19
- Grazer lv1 at 50,-1,-26 (ground -6) x19
- Grazer lv2 at 10,0,-103 (ground -8) x19
- Grazer lv2 at 11,1,-102 (ground -8) x19
- Grazer lv2 at 9,0,-100 (ground -8) x19
- Grazer lv3 at 13,-1,-94 (ground -7) x19
- Grazer lv3 at 15,-1,-96 (ground -7) x19
- Shade lv4 at 21,-3,-88 (ground -7) x19
- Shade lv5 at -82,-2,-99 (ground -6) x19
- Shade lv6 at 117,-2,-19 (ground -8) x19

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 2 | 12768 | 12 | 3.2 | 92% | 10 | 7% |
| completionist | 6 | 16451 | 54 | 4.2 | 83% | 6 | 6% |
| explorer | 2 | 12519 | 12 | 22.5 | 70% | 11 | 3% |
| grinder | 4 | 10418 | 8 | 17.5 | 89% | 12 | 5% |
| party | 4 | 3473 | 12 | 6.3 | 66% | 28 | 25% |
| social | 2 | 14989 | 18 | 2.5 | 78% | 8 | 6% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 3 | 16461 | 28 | 15.0 | 68% | 7 | 5% |
| Mage | 3 | 12621 | 12 | 10.1 | 91% | 10 | 5% |
| Paladin | 4 | 10418 | 13 | 12.5 | 84% | 13 | 14% |
| Priest | 3 | 16896 | 30 | 5.0 | 84% | 6 | 4% |
| Rogue | 3 | 10545 | 14 | 7.8 | 79% | 10 | 12% |
| Warrior | 4 | 8271 | 19 | 3.8 | 78% | 10 | 14% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Bottbt00com | Warrior | completionist | 12.0 | 1->3 | 10001 | 6/11 | 9 | 0 | 6 | 432 |
| Bottbt01gri | Paladin | grinder | 12.0 | 1->3 | 10921 | 2/2 | 25 | 6 | 15 | 72 |
| Bottbt02exp | Hunter | explorer | 12.0 | 1->4 | 18497 | 9/17 | 14 | 6 | 9 | 66 |
| Bottbt03com | Rogue | completionist | 12.0 | 1->4 | 16906 | 10/17 | 12 | 2 | 4 | 300 |
| Bottbt04soc | Priest | social | 12.0 | 1->4 | 15211 | 9/16 | 11 | 0 | 9 | 141 |
| Bottbt05cas | Mage | casual | 11.7 | 1->4 | 14991 | 8/15 | 14 | 1 | 10 | 69 |
| Bottbt06com | Warrior | completionist | 12.0 | 1->4 | 16006 | 8/13 | 13 | 0 | 8 | 209 |
| Bottbt07gri | Paladin | grinder | 12.0 | 1->3 | 9916 | 2/2 | 23 | 3 | 11 | 121 |
| Bottbt08par | Hunter | party | 12.0 | 1->4 | 16461 | 10/16 | 13 | 2 | 4 | 97 |
| Bottbt09par | Rogue | party | 12.0 | 1->2 | 3755 | 0/2 | 4 | 2 | 41 | 218 |
| Bottbt10com | Priest | completionist | 12.0 | 1->4 | 16896 | 9/16 | 11 | 1 | 6 | 147 |
| Bottbt11gri | Mage | grinder | 12.0 | 1->3 | 12621 | 2/2 | 29 | 3 | 8 | 96 |
| Bottbt12exp | Warrior | explorer | 12.0 | 1->3 | 6541 | 3/6 | 9 | 3 | 12 | 242 |
| Bottbt13com | Paladin | completionist | 12.0 | 1->4 | 15381 | 9/16 | 12 | 0 | 10 | 95 |
| Bottbt14soc | Hunter | social | 12.0 | 1->4 | 14766 | 9/15 | 10 | 1 | 7 | 160 |
| Bottbt15cas | Rogue | casual | 6.8 | 1->2 | 10545 | 4/6 | 4 | 0 | 10 | 76 |
| Bottbt16com | Priest | completionist | 12.0 | 1->4 | 23712 | 12/19 | 16 | 2 | 6 | 101 |
| Bottbt17gri | Mage | grinder | 12.0 | 1->2 | 6385 | 2/2 | 15 | 2 | 14 | 96 |
| Bottbt18par | Warrior | party | 12.0 | 1->2 | 3190 | 2/5 | 7 | 0 | 14 | 145 |
| Bottbt19par | Paladin | party | 12.0 | 1->1 | 1290 | 0/2 | 3 | 1 | 75 | 254 |
