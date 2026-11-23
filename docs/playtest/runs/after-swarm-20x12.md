# Hearthblock playtest: 20 bots, 12 min, brain mix

20 bots on `127.0.0.1:7542`, 234.5 bot-minutes (longest session 12.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 4 fail, 3 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 99.71 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 8.42 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 217.80 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.30 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.51  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 1.20 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 9.09 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 11.00 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.94  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 4.96 s | <= 25 pass, > 40 fail (and >= 5) |
| WARN | Share of time idle, stuck, waiting for spawns or dead | 0.16  | <= 0.15 pass, > 0.3 fail |
| PASS | Share of time travelling | 0.26  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 27.36 blocks | <= 60 pass, > 120 fail |
| WARN | Distinct abilities used per fighting bot | 1.75  | >= 2 pass, < 1.5 fail |
| FAIL | Share of casts that are the most used ability | 0.91  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.26  | <= 0.05 pass, > 0.15 fail |
| FAIL | Stuck events per bot-hour | 19.96 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 7163.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 100 s; first quest turned in: median 8 s; level 2: median 218 s (100% of level-1 starters reached it)
- XP per hour: median 8346 (design pace 6400); 51% of XP from quests; 118 quests turned in, median 1.2 min each
- Rewards: median gap 9 s; median longest drought 102 s; 0% of bots had a drought over 5 min
- Combat: 925 fights, 94% won, median 5.0 s; 319 kills, 111 without credit (7 of them with no other player within 20 blocks); 43 deaths (11.0/h)
- Travel: 27 blocks per reward; money earned 7163 c, spent 0 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 20.0 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 2.0% |
| dead | 1.0% |
| fight | 44.0% |
| gather | 5.2% |
| idle | 2.8% |
| loot | 3.7% |
| rest | 2.4% |
| town | 1.3% |
| travel | 25.7% |
| wait_spawn | 11.9% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 20 | 1.0 | 0 |
| 1 | 20 | 1.0 | 145 |
| 2 | 20 | 1.0 | 266 |
| 3 | 20 | 1.0 | 391 |
| 4 | 20 | 2.0 | 495 |
| 5 | 20 | 2.0 | 636 |
| 6 | 20 | 2.0 | 801 |
| 7 | 19 | 2.0 | 924 |
| 8 | 19 | 2.0 | 1077 |
| 9 | 19 | 2.0 | 1230 |
| 10 | 19 | 3.0 | 1440 |
| 11 | 19 | 3.0 | 1658 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 6 | 0 | - |
| coal_for_the_watch | 16 | 12 | 3.5 |
| cold_hearth | 20 | 20 | 0.1 |
| down_to_water | 7 | 2 | 1.2 |
| dust_of_shadows | 2 | 1 | 0.0 |
| eyes_in_the_dark | 6 | 2 | 4.6 |
| garricks_camp | 6 | 6 | 0.3 |
| iron_for_hollis | 11 | 0 | - |
| kindling | 16 | 16 | 1.6 |
| meat_for_the_pot | 16 | 12 | 6.5 |
| north_watch | 8 | 6 | 0.3 |
| pincher_supper | 2 | 0 | - |
| report_to_holt | 6 | 6 | 0.1 |
| silk_for_sails | 6 | 0 | - |
| stolen_grain | 7 | 0 | - |
| thieves_in_fields | 7 | 1 | 1.2 |
| thin_the_herd | 20 | 12 | 7.5 |
| tusks_and_tempers | 6 | 0 | - |
| watchfire | 6 | 5 | 0.0 |
| windmill_breadcrumb | 10 | 7 | 0.8 |
| wool_for_winter | 12 | 10 | 1.9 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x12
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x6
- thin_the_herd: goto failed: too many repaths x2
- watchfire: collect Coal: no source in reach x2
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x1
- north_watch: goto failed: dead x1

## Deaths by cause

- You fell to a Shade. x12
- You fell to a Websnare. x10
- You fell to a Ashcloak Bandit. x8
- You fell to a Thornboar. x5
- You fell to a Ashcloak Cutthroat. x4
- You fell to a Rook Varn. x4

## Fights by creature and outcome

- Grazer: won x689
- Thornboar: won x82
- Shade: won x63
- Ashcloak Bandit: won x24
- Websnare: lost (died) x11
- Rook Varn: lost (died) x10
- Shade: lost (died) x9
- Ashcloak Bandit: lost (died) x7
- Ashcloak Bandit: lost (it fled home and healed) x7
- Websnare: won x6
- Thornboar: lost (died) x4
- Websnare: lost (could not reach or hurt it) x4
- Ashcloak Cutthroat: won x2
- Websnare: lost (it fled home and healed) x2
- Ashcloak Bandit: lost (could not reach or hurt it) x1
- Grazer: lost (could not reach or hurt it) x1
- Rook Varn: lost (could not reach or hurt it) x1
- Rook Varn: lost (it fled home and healed) x1
- Thornboar: lost (it fled home and healed) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 94 | Quick Shot 96%, Barbed Arrow 4% |
| Mage | 190 | Fireball 98%, Scholar's Focus 2% |
| Paladin | 99 | Judging Blow 84%, Holy Mend 16% |
| Priest | 180 | Smite 99%, Mend 1% |
| Rogue | 95 | Vicious Strike 85%, Finishing Cut 15% |
| Warrior | 70 | Heroic Blow 84%, Battle Cry 16% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [0, -16]: 15 events
  - Botuux00com at [0, 0, -3] going to [0, -6, -3] (no path; coal_for_the_watch: gather Coal)
  - Botuux02exp at [0, 0, -3] going to [0, -6, -3] (no path; coal_for_the_watch: gather Coal)
  - Botuux03com at [2, 0, -3] going to [2, -6, -3] (no path; iron_for_hollis: gather Raw iron)
  - Botuux04soc at [0, 0, -3] going to [0, -6, -3] (no path; coal_for_the_watch: gather Coal)
- cell [16, -96]: 6 events
  - Botuux03com at [21, -8, -86] going to [22, -17, -85] (no path; iron_for_hollis: gather Raw iron)
  - Botuux03com at [25, -7, -91] going to [25, -14, -91] (no path; watchfire: gather Coal)
  - Botuux10com at [25, -7, -91] going to [25, -14, -91] (no path; coal_for_the_watch: gather Coal)
  - Botuux10com at [20, -6, -92] going to [20, -15, -92] (no path; coal_for_the_watch: gather Coal)
- cell [-16, -48]: 5 events
  - Botuux03com at [-5, -6, -44] going to [-5, -14, -44] (no path; watchfire: gather Coal)
  - Botuux03com at [-2, -5, -38] going to [-2, -15, -38] (no path; iron_for_hollis: gather Raw iron)
  - Botuux10com at [-5, -6, -44] going to [-5, -14, -44] (no path; coal_for_the_watch: gather Coal)
  - Botuux14soc at [-5, -6, -44] going to [-5, -14, -44] (no path; watchfire: gather Coal)
- cell [0, -96]: 5 events
  - Botuux00com at [9, -6, -94] going to [9, -17, -94] (no path; coal_for_the_watch: gather Coal)
  - Botuux00com at [1, -5, -85] going to [1, -21, -85] (no path; coal_for_the_watch: gather Coal)
  - Botuux00com at [6, -7, -86] going to [6, -21, -86] (no path; coal_for_the_watch: gather Coal)
  - Botuux13com at [6, -7, -86] going to [6, -21, -86] (no path; coal_for_the_watch: gather Coal)
- cell [0, 80]: 5 events
  - Botuux02exp at [11, -10, 88] going to [11, -18, 88] (no path; coal_for_the_watch: gather Coal)
  - Botuux05cas at [12, -10, 90] going to [12, -17, 90] (no path; coal_for_the_watch: gather Coal)
  - Botuux05cas at [1, -7, 84] going to [2, -21, 84] (no path; coal_for_the_watch: gather Coal)
  - Botuux13com at [9, -10, 87] going to [10, -18, 86] (no path; iron_for_hollis: gather Raw iron)
- cell [-16, 64]: 4 events
  - Botuux02exp at [-6, -8, 67] going to [-6, -14, 67] (no path; coal_for_the_watch: gather Coal)
  - Botuux13com at [-6, -8, 67] going to [-6, -14, 67] (no path; coal_for_the_watch: gather Coal)
  - Botuux14soc at [-4, -9, 72] going to [-4, -13, 71] (no path; iron_for_hollis: gather Raw iron)
  - Botuux19par at [-4, -9, 71] going to [-4, -13, 71] (no path; coal_for_the_watch: gather Coal)
- cell [0, 96]: 4 events
  - Botuux08par at [3, -9, 101] going to [3, -15, 102] (no path; iron_for_hollis: gather Raw iron)
  - Botuux08par at [13, -10, 96] going to [13, -17, 96] (no path; iron_for_hollis: gather Raw iron)
  - Botuux14soc at [13, -10, 100] going to [13, -16, 100] (no path; iron_for_hollis: gather Raw iron)
  - Botuux16com at [3, -9, 102] going to [3, -15, 102] (no path; iron_for_hollis: gather Raw iron)
- cell [16, -64]: 4 events
  - Botuux03com at [24, -7, -52] going to [30, -5, -52] (repaths; thin_the_herd: go to the Grazer camp)
  - Botuux10com at [17, -8, -52] going to [17, -17, -52] (no path; coal_for_the_watch: gather Coal)
  - Botuux18par at [17, -8, -52] going to [17, -17, -52] (no path; coal_for_the_watch: gather Coal)
  - Botuux19par at [17, -8, -52] going to [17, -17, -52] (no path; coal_for_the_watch: gather Coal)
- cell [32, 16]: 4 events
  - Botuux04soc at [41, -7, 21] going to [41, -13, 21] (no path; coal_for_the_watch: gather Coal)
  - Botuux05cas at [40, -7, 20] going to [40, -15, 20] (no path; coal_for_the_watch: gather Coal)
  - Botuux14soc at [39, -7, 20] going to [39, -16, 20] (no path; iron_for_hollis: gather Raw iron)
  - Botuux16com at [39, -7, 18] going to [39, -17, 18] (no path; iron_for_hollis: gather Raw iron)
- cell [-64, -112]: 3 events
  - Botuux03com at [-52, -6, -101] going to [-52, -20, -101] (no path; watchfire: gather Coal)
  - Botuux03com at [-63, -5, -100] going to [-63, -17, -100] (no path; iron_for_hollis: gather Raw iron)
  - Botuux03com at [-57, -7, -109] going to [-57, -17, -109] (no path; watchfire: gather Coal)
- cell [0, -48]: 3 events
  - Botuux04soc at [0, -7, -42] going to [0, -14, -42] (no path; iron_for_hollis: gather Raw iron)
  - Botuux09par at [1, -7, -41] going to [3, -13, -40] (no path; coal_for_the_watch: gather Coal)
  - Botuux14soc at [0, -7, -42] going to [0, -14, -42] (no path; iron_for_hollis: gather Raw iron)
- cell [16, 32]: 3 events
  - Botuux00com at [30, -7, 47] going to [30, -15, 47] (no path; iron_for_hollis: gather Raw iron)
  - Botuux05cas at [30, -6, 45] going to [30, -14, 45] (no path; coal_for_the_watch: gather Coal)
  - Botuux14soc at [30, -7, 46] going to [30, -15, 46] (no path; iron_for_hollis: gather Raw iron)

## Notes

- grouped in a party of two x4
- casual session ended x1
- tried /invite Botuux00com x1
- tried /invite Botuux02exp x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 2 | 5418 | 7 | 0.0 | 98% | 12 | 17% |
| completionist | 6 | 8664 | 44 | 10.8 | 94% | 8 | 17% |
| explorer | 2 | 7764 | 11 | 17.5 | 86% | 11 | 13% |
| grinder | 4 | 8178 | 8 | 7.5 | 98% | 11 | 12% |
| party | 4 | 7801 | 25 | 12.5 | 91% | 12 | 20% |
| social | 2 | 14584 | 23 | 17.5 | 86% | 6 | 11% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 3 | 13436 | 32 | 20.0 | 80% | 6 | 12% |
| Mage | 3 | 6981 | 9 | 1.7 | 99% | 7 | 11% |
| Paladin | 4 | 7823 | 12 | 11.3 | 96% | 14 | 18% |
| Priest | 3 | 10891 | 27 | 8.3 | 96% | 7 | 17% |
| Rogue | 3 | 10916 | 21 | 15.7 | 90% | 10 | 15% |
| Warrior | 4 | 6848 | 17 | 10.0 | 93% | 15 | 19% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botuux00com | Warrior | completionist | 12.0 | 1->3 | 8106 | 7/12 | 11 | 3 | 8 | 108 |
| Botuux01gri | Paladin | grinder | 12.0 | 1->3 | 7771 | 2/2 | 24 | 1 | 14 | 83 |
| Botuux02exp | Hunter | explorer | 12.0 | 1->3 | 8681 | 7/12 | 11 | 4 | 7 | 75 |
| Botuux03com | Rogue | completionist | 12.0 | 1->4 | 13786 | 10/19 | 25 | 4 | 8 | 92 |
| Botuux04soc | Priest | social | 12.0 | 1->3 | 12871 | 10/16 | 19 | 2 | 6 | 79 |
| Botuux05cas | Mage | casual | 12.0 | 1->3 | 6981 | 5/8 | 9 | 0 | 10 | 102 |
| Botuux06com | Warrior | completionist | 12.0 | 1->3 | 6851 | 4/7 | 11 | 1 | 15 | 102 |
| Botuux07gri | Paladin | grinder | 12.0 | 1->3 | 8586 | 2/2 | 26 | 4 | 16 | 91 |
| Botuux08par | Hunter | party | 12.0 | 1->3 | 13436 | 12/19 | 25 | 3 | 6 | 79 |
| Botuux09par | Rogue | party | 12.0 | 1->3 | 10916 | 9/15 | 12 | 4 | 10 | 163 |
| Botuux10com | Priest | completionist | 12.0 | 1->3 | 10891 | 9/15 | 16 | 2 | 7 | 103 |
| Botuux11gri | Mage | grinder | 12.0 | 1->3 | 9616 | 2/2 | 30 | 1 | 5 | 140 |
| Botuux12exp | Warrior | explorer | 12.0 | 1->3 | 6846 | 4/6 | 12 | 3 | 15 | 102 |
| Botuux13com | Paladin | completionist | 12.0 | 1->3 | 7876 | 6/11 | 11 | 2 | 10 | 129 |
| Botuux14soc | Hunter | social | 12.0 | 1->4 | 16297 | 13/20 | 21 | 5 | 5 | 73 |
| Botuux15cas | Rogue | casual | 6.5 | 1->2 | 3855 | 2/5 | 3 | 0 | 14 | 79 |
| Botuux16com | Priest | completionist | 12.0 | 1->3 | 9221 | 8/13 | 13 | 1 | 8 | 66 |
| Botuux17gri | Mage | grinder | 12.0 | 1->2 | 6000 | 2/2 | 16 | 0 | 7 | 234 |
| Botuux18par | Warrior | party | 12.0 | 1->2 | 4685 | 2/5 | 14 | 1 | 15 | 114 |
| Botuux19par | Paladin | party | 12.0 | 1->2 | 3800 | 2/5 | 10 | 2 | 14 | 118 |
