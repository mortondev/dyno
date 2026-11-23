# Hearthblock playtest: 20 bots, 12 min, brain mix

20 bots on `127.0.0.1:7549`, 236.8 bot-minutes (longest session 12.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 2 fail, 2 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 44.70 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 8.45 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 239.02 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.41 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.57  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 1.49 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 8.67 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 5.32 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.97  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 5.11 s | <= 25 pass, > 40 fail (and >= 5) |
| WARN | Share of time idle, stuck, waiting for spawns or dead | 0.16  | <= 0.15 pass, > 0.3 fail |
| PASS | Share of time travelling | 0.23  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 30.93 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.35  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.75  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.36  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 1.27 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 6857.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 45 s; first quest turned in: median 8 s; level 2: median 239 s (100% of level-1 starters reached it)
- XP per hour: median 9016 (design pace 6400); 57% of XP from quests; 109 quests turned in, median 1.5 min each
- Rewards: median gap 9 s; median longest drought 124 s; 0% of bots had a drought over 5 min
- Combat: 1045 fights, 97% won, median 5.1 s; 280 kills, 157 without credit (7 of them with no other player within 20 blocks); 21 deaths (5.3/h)
- Travel: 31 blocks per reward; money earned 6857 c, spent 0 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 1.3 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 3.2% |
| dead | 0.5% |
| fight | 46.8% |
| gather | 3.4% |
| idle | 3.2% |
| loot | 3.3% |
| rest | 2.5% |
| town | 1.0% |
| travel | 23.5% |
| wait_spawn | 12.7% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 20 | 1.0 | 0 |
| 1 | 20 | 1.0 | 137 |
| 2 | 20 | 1.0 | 227 |
| 3 | 20 | 1.0 | 368 |
| 4 | 20 | 1.5 | 500 |
| 5 | 20 | 2.0 | 610 |
| 6 | 20 | 2.0 | 695 |
| 7 | 20 | 2.0 | 858 |
| 8 | 20 | 2.0 | 978 |
| 9 | 19 | 2.0 | 1027 |
| 10 | 19 | 2.0 | 1156 |
| 11 | 19 | 3.0 | 1353 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 5 | 0 | - |
| coal_for_the_watch | 16 | 12 | 1.9 |
| cold_hearth | 20 | 20 | 0.1 |
| down_to_water | 3 | 2 | 1.2 |
| eyes_in_the_dark | 8 | 0 | - |
| garricks_camp | 5 | 4 | 0.5 |
| iron_for_hollis | 8 | 0 | - |
| kindling | 16 | 16 | 2.0 |
| meat_for_the_pot | 16 | 13 | 4.4 |
| north_watch | 9 | 8 | 0.3 |
| pincher_supper | 2 | 0 | - |
| report_to_holt | 8 | 5 | 0.1 |
| silk_for_sails | 4 | 0 | - |
| stolen_grain | 3 | 0 | - |
| thieves_in_fields | 3 | 1 | 4.7 |
| thin_the_herd | 20 | 12 | 9.7 |
| tusks_and_tempers | 4 | 0 | - |
| watchfire | 5 | 3 | 0.1 |
| windmill_breadcrumb | 10 | 3 | 0.5 |
| wool_for_winter | 13 | 10 | 3.4 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x16
- iron_for_hollis: collect Raw iron: no source in reach x7
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x3
- thin_the_herd: goto failed: too many repaths x2
- watchfire: collect Coal: no source in reach x2
- iron_for_hollis: collect Raw iron: Requires mining 5 (have 4) x1
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x1
- kindling: collect Log: no source in reach x1

## Deaths by cause

- You fell to a Shade. x10
- You fell to a Websnare. x4
- You fell to a Ashcloak Bandit. x2
- You fell to a Mirepincher. x2
- You fell to a Candlekin. x1
- You fell to a Grazer. x1
- You fell to a Thornboar. x1

## Fights by creature and outcome

- Grazer: won x885
- Shade: won x76
- Thornboar: won x41
- Shade: lost (died) x9
- Ashcloak Bandit: won x5
- Mirepincher: won x4
- Websnare: lost (died) x4
- Ashcloak Bandit: lost (died) x2
- Ashcloak Bandit: lost (it fled home and healed) x2
- Mirepincher: lost (died) x2
- Shade: lost (it fled home and healed) x2
- Thornboar: lost (died) x2
- Websnare: lost (could not reach or hurt it) x2
- Websnare: won x2
- Candlekin: lost (could not reach or hurt it) x1
- Candlekin: lost (died) x1
- Grazer: lost (cannot reach it: no path) x1
- Grazer: lost (died) x1
- Mirepincher: lost (could not reach or hurt it) x1
- Mirepincher: lost (it fled home and healed) x1
- Thornboar: lost (could not reach or hurt it) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 100 | Quick Shot 57%, Barbed Arrow 43% |
| Mage | 163 | Fireball 74%, Fire Blast 25%, Scholar's Focus 2% |
| Paladin | 101 | Judging Blow 90%, Holy Mend 8%, Rebuke 2% |
| Priest | 238 | Smite 65%, Withering Word 34%, Mend 1% |
| Rogue | 97 | Vicious Strike 80%, Finishing Cut 19%, Recall 1% |
| Warrior | 35 | Heroic Blow 77%, Battle Cry 20%, Rending Cut 3% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [48, 16]: 2 events
  - Botpkn07gri at [49, -9, 24] going to [48, -5, 26] (no path; looting)
  - Botpkn18par at [48, -6, 16] going to [48, -7, 22] (repaths; thin_the_herd: go to the Grazer camp)
- cell [0, -112]: 1 events
  - Botpkn03com at [10, -8, -101] going to [10, -7, -95] (repaths; thin_the_herd: go to the Grazer camp)
- cell [32, -16]: 1 events
  - Botpkn19par at [41, -5, -8] going to [41, -4, -12] (repaths; following Botpkn18par)
- cell [32, 16]: 1 events
  - Botpkn07gri at [46, -9, 27] going to [47, -5, 26] (no path; thin_the_herd: kill Grazer)

## Notes

- grouped in a party of two x4
- casual session ended x1
- tried /invite Botpkn00com x1
- tried /invite Botpkn13com x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 2 | 8888 | 12 | 0.0 | 100% | 9 | 11% |
| completionist | 6 | 8617 | 34 | 7.5 | 97% | 8 | 16% |
| explorer | 2 | 7164 | 12 | 12.5 | 88% | 8 | 18% |
| grinder | 4 | 7816 | 7 | 5.0 | 97% | 14 | 11% |
| party | 4 | 7983 | 26 | 2.5 | 98% | 11 | 22% |
| social | 2 | 11621 | 18 | 2.5 | 98% | 5 | 20% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 3 | 12055 | 31 | 5.0 | 93% | 5 | 21% |
| Mage | 3 | 9305 | 8 | 7.3 | 97% | 8 | 10% |
| Paladin | 4 | 6748 | 10 | 2.5 | 98% | 13 | 18% |
| Priest | 3 | 9461 | 20 | 5.0 | 98% | 6 | 11% |
| Rogue | 3 | 9375 | 25 | 6.7 | 94% | 10 | 16% |
| Warrior | 4 | 5244 | 15 | 6.3 | 98% | 9 | 20% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botpkn00com | Warrior | completionist | 12.0 | 1->2 | 2966 | 2/5 | 6 | 0 | 7 | 237 |
| Botpkn01gri | Paladin | grinder | 12.0 | 1->3 | 8727 | 2/2 | 28 | 0 | 14 | 66 |
| Botpkn02exp | Hunter | explorer | 12.0 | 1->3 | 10186 | 9/14 | 13 | 2 | 8 | 65 |
| Botpkn03com | Rogue | completionist | 12.0 | 1->4 | 15571 | 11/19 | 19 | 4 | 9 | 111 |
| Botpkn04soc | Priest | social | 12.0 | 1->3 | 9461 | 8/15 | 15 | 0 | 5 | 90 |
| Botpkn05cas | Mage | casual | 8.9 | 1->3 | 9305 | 5/8 | 10 | 0 | 6 | 81 |
| Botpkn06com | Warrior | completionist | 12.0 | 1->3 | 10438 | 6/10 | 14 | 1 | 12 | 109 |
| Botpkn07gri | Paladin | grinder | 12.0 | 1->3 | 6904 | 2/2 | 15 | 0 | 16 | 114 |
| Botpkn08par | Hunter | party | 12.0 | 1->3 | 12055 | 12/20 | 19 | 0 | 5 | 125 |
| Botpkn09par | Rogue | party | 12.0 | 1->3 | 9375 | 7/12 | 12 | 0 | 10 | 130 |
| Botpkn10com | Priest | completionist | 12.0 | 1->3 | 11545 | 7/11 | 16 | 2 | 7 | 129 |
| Botpkn11gri | Mage | grinder | 12.0 | 1->2 | 2642 | 1/2 | 8 | 0 | 14 | 195 |
| Botpkn12exp | Warrior | explorer | 12.0 | 1->2 | 4141 | 3/5 | 6 | 3 | 7 | 201 |
| Botpkn13com | Paladin | completionist | 12.0 | 1->2 | 3956 | 3/5 | 6 | 1 | 10 | 200 |
| Botpkn14soc | Hunter | social | 12.0 | 1->4 | 13782 | 10/16 | 22 | 1 | 5 | 132 |
| Botpkn15cas | Rogue | casual | 12.0 | 1->3 | 8471 | 7/11 | 10 | 0 | 13 | 142 |
| Botpkn16com | Priest | completionist | 12.0 | 1->3 | 6796 | 5/7 | 10 | 1 | 6 | 123 |
| Botpkn17gri | Mage | grinder | 12.0 | 1->3 | 10741 | 2/2 | 30 | 4 | 8 | 83 |
| Botpkn18par | Warrior | party | 12.0 | 1->2 | 6346 | 4/6 | 10 | 1 | 14 | 95 |
| Botpkn19par | Paladin | party | 12.0 | 1->3 | 6591 | 3/6 | 11 | 1 | 13 | 203 |
