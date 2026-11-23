# Hearthblock playtest: 20 bots, 12 min, brain mix

20 bots on `127.0.0.1:7541`, 238.5 bot-minutes (longest session 12.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 4 fail, 1 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 51.43 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 8.43 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 224.63 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.25 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.55  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 1.64 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 8.53 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 8.05 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.96  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 5.05 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.14  | <= 0.15 pass, > 0.3 fail |
| PASS | Share of time travelling | 0.29  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 30.00 blocks | <= 60 pass, > 120 fail |
| WARN | Distinct abilities used per fighting bot | 1.80  | >= 2 pass, < 1.5 fail |
| FAIL | Share of casts that are the most used ability | 0.90  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.28  | <= 0.05 pass, > 0.15 fail |
| FAIL | Stuck events per bot-hour | 22.14 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 7822.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 51 s; first quest turned in: median 8 s; level 2: median 225 s (100% of level-1 starters reached it)
- XP per hour: median 7994 (design pace 6400); 55% of XP from quests; 124 quests turned in, median 1.6 min each
- Rewards: median gap 9 s; median longest drought 100 s; 0% of bots had a drought over 5 min
- Combat: 993 fights, 96% won, median 5.0 s; 304 kills, 117 without credit (5 of them with no other player within 20 blocks); 32 deaths (8.1/h)
- Travel: 30 blocks per reward; money earned 7822 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 22.1 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 2.1% |
| dead | 0.7% |
| fight | 44.5% |
| gather | 3.8% |
| idle | 2.3% |
| loot | 3.7% |
| rest | 1.8% |
| town | 1.5% |
| travel | 28.6% |
| wait_spawn | 11.1% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 20 | 1.0 | 0 |
| 1 | 20 | 1.0 | 146 |
| 2 | 20 | 1.0 | 231 |
| 3 | 20 | 1.0 | 385 |
| 4 | 20 | 2.0 | 483 |
| 5 | 20 | 2.0 | 568 |
| 6 | 20 | 2.0 | 740 |
| 7 | 20 | 2.0 | 858 |
| 8 | 20 | 2.0 | 989 |
| 9 | 20 | 2.0 | 1156 |
| 10 | 20 | 2.5 | 1352 |
| 11 | 19 | 3.0 | 1574 |
| 12 | 1 | 3.0 | 2177 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 8 | 0 | - |
| coal_for_the_watch | 16 | 15 | 3.2 |
| cold_hearth | 20 | 20 | 0.1 |
| dust_of_shadows | 3 | 2 | 0.0 |
| eyes_in_the_dark | 9 | 4 | 2.2 |
| garricks_camp | 8 | 7 | 0.4 |
| iron_for_hollis | 12 | 0 | - |
| kindling | 16 | 16 | 2.0 |
| meat_for_the_pot | 16 | 13 | 6.2 |
| north_watch | 12 | 9 | 0.3 |
| report_to_holt | 9 | 8 | 0.1 |
| silk_for_sails | 7 | 0 | - |
| thin_the_herd | 20 | 16 | 9.3 |
| tusks_and_tempers | 7 | 0 | - |
| watchfire | 8 | 4 | 1.2 |
| windmill_breadcrumb | 10 | 0 | - |
| wool_for_winter | 13 | 10 | 0.8 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x10
- iron_for_hollis: collect Raw iron: no source in reach x5
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x4
- watchfire: collect Coal: no source in reach x3
- windmill_breadcrumb: talk failed: cannot reach NPC: no path x3
- thin_the_herd: goto failed: too many repaths x1

## Deaths by cause

- You fell to a Shade. x12
- You fell to a Thornboar. x11
- You fell to a Websnare. x5
- You fell to a Ashcloak Bandit. x4

## Fights by creature and outcome

- Grazer: won x786
- Shade: won x79
- Thornboar: won x66
- Shade: lost (died) x11
- Thornboar: lost (died) x11
- Ashcloak Bandit: won x10
- Websnare: lost (died) x6
- Websnare: won x6
- Ashcloak Bandit: lost (died) x4
- Thornboar: lost (could not reach or hurt it) x3
- Thornboar: lost (it fled home and healed) x3
- Websnare: lost (could not reach or hurt it) x3
- Websnare: lost (it fled home and healed) x2
- Ashcloak Bandit: lost (it fled home and healed) x1
- Ashcloak Cutthroat: won x1
- Mirepincher: won x1

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [0, -16]: 16 events
  - Botvsc00com at [0, 0, -3] going to [0, -6, -3] (no path; coal_for_the_watch: gather Coal)
  - Botvsc02exp at [0, 0, -3] going to [0, -6, -3] (no path; coal_for_the_watch: gather Coal)
  - Botvsc02exp at [11, 0, -8] going to [11, -15, -8] (no path; iron_for_hollis: gather Raw iron)
  - Botvsc03com at [0, 0, -3] going to [0, -6, -3] (no path; coal_for_the_watch: gather Coal)
- cell [-16, -48]: 12 events
  - Botvsc00com at [-5, -6, -43] going to [-5, -14, -44] (no path; coal_for_the_watch: gather Coal)
  - Botvsc03com at [-5, -4, -36] going to [-5, -13, -36] (no path; iron_for_hollis: gather Raw iron)
  - Botvsc03com at [-5, -6, -44] going to [-5, -14, -44] (no path; watchfire: gather Coal)
  - Botvsc04soc at [-2, -5, -36] going to [-2, -15, -36] (no path; iron_for_hollis: gather Raw iron)
- cell [0, 80]: 6 events
  - Botvsc08par at [13, -10, 89] going to [11, -18, 88] (no path; coal_for_the_watch: gather Coal)
  - Botvsc10com at [11, -10, 95] going to [12, -17, 97] (no path; windmill_breadcrumb: talk to Farmer Ostrin)
  - Botvsc12exp at [12, -10, 91] going to [10, -10, 95] (repaths; exploring Ostrin's Farm)
  - Botvsc14soc at [11, -10, 87] going to [10, -18, 87] (no path; coal_for_the_watch: gather Coal)
- cell [16, -64]: 6 events
  - Botvsc00com at [17, -8, -52] going to [17, -17, -52] (no path; coal_for_the_watch: gather Coal)
  - Botvsc01gri at [24, -7, -52] going to [30, -5, -52] (repaths; going to grind)
  - Botvsc05cas at [24, -7, -52] going to [30, -5, -52] (repaths; thin_the_herd: go to the Grazer camp)
  - Botvsc06com at [19, -8, -56] going to [19, -14, -56] (no path; coal_for_the_watch: gather Coal)
- cell [-64, -112]: 5 events
  - Botvsc03com at [-55, -7, -110] going to [-55, -17, -110] (no path; watchfire: gather Coal)
  - Botvsc03com at [-60, -5, -112] going to [-60, -16, -112] (no path; iron_for_hollis: gather Raw iron)
  - Botvsc06com at [-51, -4, -97] going to [-51, -21, -96] (no path; iron_for_hollis: gather Raw iron)
  - Botvsc15cas at [-51, -4, -97] going to [-51, -21, -96] (no path; coal_for_the_watch: gather Coal)
- cell [0, -96]: 5 events
  - Botvsc02exp at [4, -7, -95] going to [4, -22, -95] (no path; coal_for_the_watch: gather Coal)
  - Botvsc03com at [5, -6, -87] going to [5, -21, -87] (no path; coal_for_the_watch: gather Coal)
  - Botvsc04soc at [5, -6, -87] going to [5, -21, -87] (no path; coal_for_the_watch: gather Coal)
  - Botvsc06com at [4, -7, -94] going to [4, -22, -94] (no path; iron_for_hollis: gather Raw iron)
- cell [-64, -64]: 3 events
  - Botvsc16com at [-61, -6, -58] going to [-62, -12, -57] (no path; coal_for_the_watch: gather Coal)
  - Botvsc16com at [-60, -5, -62] going to [-60, -11, -62] (no path; coal_for_the_watch: gather Coal)
  - Botvsc16com at [-57, -6, -55] going to [-57, -12, -55] (no path; coal_for_the_watch: gather Coal)
- cell [-32, -112]: 3 events
  - Botvsc04soc at [-25, -4, -100] going to [-25, -20, -100] (no path; coal_for_the_watch: gather Coal)
  - Botvsc14soc at [-23, -6, -111] going to [-23, -19, -111] (no path; iron_for_hollis: gather Raw iron)
  - Botvsc14soc at [-25, -4, -103] going to [-25, -22, -103] (no path; watchfire: gather Coal)
- cell [-32, -96]: 3 events
  - Botvsc03com at [-20, -6, -88] going to [-20, -22, -88] (no path; iron_for_hollis: gather Raw iron)
  - Botvsc03com at [-22, -5, -92] going to [-22, -25, -92] (no path; watchfire: gather Coal)
  - Botvsc10com at [-20, -6, -88] going to [-20, -22, -88] (no path; iron_for_hollis: gather Raw iron)
- cell [0, -48]: 3 events
  - Botvsc06com at [4, -5, -38] going to [3, -13, -39] (no path; coal_for_the_watch: gather Coal)
  - Botvsc14soc at [1, -7, -42] going to [1, -14, -42] (no path; iron_for_hollis: gather Raw iron)
  - Botvsc16com at [1, -5, -39] going to [0, -14, -38] (no path; coal_for_the_watch: gather Coal)
- cell [32, 16]: 3 events
  - Botvsc03com at [39, -7, 18] going to [39, -17, 18] (no path; iron_for_hollis: gather Raw iron)
  - Botvsc10com at [39, -7, 20] going to [39, -16, 20] (no path; iron_for_hollis: gather Raw iron)
  - Botvsc15cas at [40, -7, 20] going to [40, -15, 20] (no path; coal_for_the_watch: gather Coal)
- cell [-48, -64]: 2 events
  - Botvsc06com at [-42, -4, -58] going to [-42, -22, -58] (no path; iron_for_hollis: gather Raw iron)
  - Botvsc14soc at [-42, -4, -58] going to [-42, -22, -58] (no path; iron_for_hollis: gather Raw iron)

## Notes

- grouped in a party of two x4
- casual session ended x1
- tried /invite Botvsc05cas x1
- tried /invite Botvsc15cas x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 2 | 8432 | 11 | 5.3 | 97% | 10 | 12% |
| completionist | 6 | 10607 | 46 | 10.0 | 95% | 9 | 17% |
| explorer | 2 | 7791 | 11 | 10.0 | 91% | 9 | 11% |
| grinder | 4 | 7546 | 8 | 12.5 | 95% | 10 | 10% |
| party | 4 | 7504 | 27 | 1.3 | 99% | 12 | 16% |
| social | 2 | 13268 | 21 | 7.5 | 95% | 8 | 16% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 3 | 10882 | 29 | 6.7 | 94% | 7 | 13% |
| Mage | 3 | 7606 | 9 | 7.0 | 97% | 7 | 12% |
| Paladin | 4 | 6518 | 11 | 10.0 | 96% | 13 | 13% |
| Priest | 3 | 14123 | 30 | 10.0 | 94% | 8 | 14% |
| Rogue | 3 | 9071 | 22 | 6.7 | 97% | 12 | 17% |
| Warrior | 4 | 7919 | 23 | 7.5 | 96% | 12 | 15% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botvsc00com | Warrior | completionist | 12.0 | 1->3 | 8151 | 5/8 | 12 | 2 | 12 | 100 |
| Botvsc01gri | Paladin | grinder | 12.0 | 1->3 | 7351 | 2/2 | 21 | 3 | 17 | 118 |
| Botvsc02exp | Hunter | explorer | 12.0 | 1->3 | 9747 | 7/11 | 12 | 2 | 8 | 68 |
| Botvsc03com | Rogue | completionist | 12.0 | 1->3 | 11452 | 9/16 | 17 | 2 | 9 | 232 |
| Botvsc04soc | Priest | social | 12.0 | 1->3 | 11402 | 9/14 | 16 | 2 | 8 | 62 |
| Botvsc05cas | Mage | casual | 10.5 | 1->3 | 7793 | 5/8 | 9 | 0 | 7 | 108 |
| Botvsc06com | Warrior | completionist | 12.0 | 1->3 | 9762 | 8/15 | 16 | 2 | 11 | 79 |
| Botvsc07gri | Paladin | grinder | 12.0 | 1->3 | 7837 | 2/2 | 22 | 3 | 13 | 131 |
| Botvsc08par | Hunter | party | 12.0 | 1->3 | 10882 | 10/16 | 17 | 1 | 5 | 82 |
| Botvsc09par | Rogue | party | 12.0 | 1->3 | 7321 | 7/14 | 5 | 0 | 14 | 101 |
| Botvsc10com | Priest | completionist | 12.0 | 1->4 | 15227 | 11/17 | 19 | 1 | 8 | 67 |
| Botvsc11gri | Mage | grinder | 12.0 | 1->3 | 7606 | 2/2 | 22 | 3 | 7 | 68 |
| Botvsc12exp | Warrior | explorer | 12.0 | 1->2 | 5836 | 4/6 | 8 | 2 | 10 | 158 |
| Botvsc13com | Paladin | completionist | 12.0 | 1->2 | 5021 | 3/5 | 9 | 2 | 8 | 105 |
| Botvsc14soc | Hunter | social | 12.0 | 1->4 | 15133 | 12/17 | 19 | 1 | 7 | 81 |
| Botvsc15cas | Rogue | casual | 12.0 | 1->3 | 9071 | 6/9 | 14 | 2 | 12 | 68 |
| Botvsc16com | Priest | completionist | 12.0 | 1->4 | 14123 | 10/15 | 22 | 3 | 8 | 58 |
| Botvsc17gri | Mage | grinder | 12.0 | 1->3 | 7486 | 2/2 | 24 | 1 | 6 | 214 |
| Botvsc18par | Warrior | party | 12.0 | 1->3 | 7686 | 6/9 | 12 | 0 | 12 | 121 |
| Botvsc19par | Paladin | party | 12.0 | 1->2 | 5686 | 4/6 | 8 | 0 | 12 | 101 |
