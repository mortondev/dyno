# Hearthblock playtest: 20 bots, 12 min, brain mix

20 bots on `127.0.0.1:7711`, 229.9 bot-minutes (longest session 12.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 1 fail, 3 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 53.03 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.77 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 176.39 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.44 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.62  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.72 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 8.17 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| WARN | Deaths per hour of play | 3.65 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.98  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 5.18 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.02  | <= 0.15 pass, > 0.3 fail |
| WARN | Share of time travelling | 0.53  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 30.82 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.50  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.74  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.27  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 1.31 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 10673.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 53 s; first quest turned in: median 8 s; level 2: median 176 s (100% of level-1 starters reached it)
- XP per hour: median 9190 (design pace 6400); 62% of XP from quests; 138 quests turned in, median 2.7 min each
- Rewards: median gap 8 s; median longest drought 64 s; 0% of bots had a drought over 5 min
- Combat: 703 fights, 98% won, median 5.2 s; 412 kills, 155 without credit (7 of them with no other player within 20 blocks); 14 deaths (3.7/h)
- Travel: 31 blocks per reward; money earned 10673 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 1.3 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 2.0% |
| dead | 0.3% |
| fight | 34.1% |
| gather | 2.8% |
| idle | 2.0% |
| loot | 2.6% |
| rest | 1.5% |
| town | 1.6% |
| travel | 53.1% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 20 | 1.0 | 0 |
| 1 | 20 | 1.0 | 120 |
| 2 | 20 | 1.0 | 255 |
| 3 | 20 | 2.0 | 424 |
| 4 | 20 | 2.0 | 566 |
| 5 | 20 | 2.0 | 704 |
| 6 | 20 | 2.0 | 862 |
| 7 | 19 | 2.0 | 992 |
| 8 | 18 | 2.0 | 1141 |
| 9 | 18 | 3.0 | 1283 |
| 10 | 18 | 3.0 | 1426 |
| 11 | 18 | 3.0 | 1575 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 13 | 0 | - |
| broken_tower | 16 | 14 | 4.9 |
| coal_for_the_watch | 16 | 7 | 4.7 |
| cold_hearth | 20 | 20 | 0.1 |
| dust_of_shadows | 2 | 2 | 0.0 |
| eyes_in_the_dark | 13 | 4 | 4.3 |
| garricks_camp | 13 | 3 | 1.1 |
| iron_for_hollis | 7 | 0 | - |
| kindling | 16 | 16 | 1.8 |
| lay_of_the_land | 14 | 4 | 4.0 |
| lookouts_eye | 13 | 4 | 0.6 |
| meat_for_the_pot | 16 | 16 | 3.0 |
| north_watch | 15 | 13 | 3.3 |
| silk_for_sails | 3 | 0 | - |
| thin_the_herd | 20 | 19 | 4.2 |
| tusks_and_tempers | 3 | 0 | - |
| watchfire | 13 | 2 | 0.4 |
| windmill_breadcrumb | 11 | 0 | - |
| wool_for_winter | 16 | 14 | 1.4 |
| word_for_rysa | 3 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x15
- watchfire: collect Coal: no source in reach x9
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x7
- iron_for_hollis: collect Raw iron: no source in reach x6
- iron_for_hollis: collect Raw iron: Requires mining 5 (have 4) x1
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x1

## Deaths by cause

- You fell to a Grazer. x6
- You fell to a Shade. x5
- You fell to a Thornboar. x3

## Fights by creature and outcome

- Grazer: won x567
- Shade: won x93
- Thornboar: won x27
- Grazer: lost (died) x6
- Shade: lost (died) x5
- Thornboar: lost (died) x3
- Grazer: lost (it fled home and healed) x1
- Thornboar: lost (it fled home and healed) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 70 | Quick Shot 63%, Barbed Arrow 31%, Recall 6% |
| Mage | 231 | Fireball 66%, Fire Blast 32%, Scholar's Focus 1% |
| Paladin | 112 | Judging Blow 88%, Rebuke 5%, Holy Mend 4%, Recall 2% |
| Priest | 177 | Smite 73%, Withering Word 23%, Recall 4% |
| Rogue | 79 | Vicious Strike 78%, Finishing Cut 18%, Recall 4% |
| Warrior | 84 | Heroic Blow 52%, Rending Cut 24%, Battle Cry 14%, Recall 10% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-272, 112]: 1 events
  - Botwbj12exp at [-258, -5, 120] going to [-270, -3, 120] (repaths; exploring Westfold Farm)
- cell [-192, -64]: 1 events
  - Botwbj09par at [-183, -6, -60] going to [-180, -6, -58] (repaths; following Botwbj08par)
- cell [-176, 0]: 1 events
  - Botwbj12exp at [-168, -5, 6] going to [-180, -6, 6] (repaths; exploring Trapper's Camp)
- cell [-160, 0]: 1 events
  - Botwbj09par at [-147, -6, 7] going to [-151, -6, 7] (repaths; following Botwbj08par)
- cell [0, 160]: 1 events
  - Botwbj02exp at [10, -9, 168] going to [10, 0, 180] (repaths; exploring The Broken Tower)

## Notes

- grouped in a party of two x4
- casual session ended x2
- tried /invite Botwbj00com x1
- tried /invite Botwbj13com x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 2 | 7523 | 11 | 0.0 | 100% | 10 | 1% |
| completionist | 6 | 10975 | 56 | 5.0 | 97% | 8 | 3% |
| explorer | 2 | 7450 | 12 | 0.0 | 100% | 13 | 2% |
| grinder | 4 | 9614 | 8 | 10.0 | 97% | 7 | 2% |
| party | 4 | 8353 | 33 | 0.0 | 100% | 11 | 3% |
| social | 2 | 8109 | 18 | 0.0 | 100% | 9 | 2% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 3 | 9167 | 23 | 0.0 | 100% | 8 | 2% |
| Mage | 3 | 9222 | 9 | 11.6 | 95% | 6 | 2% |
| Paladin | 4 | 9387 | 22 | 2.5 | 99% | 10 | 3% |
| Priest | 3 | 8967 | 26 | 3.3 | 98% | 8 | 2% |
| Rogue | 3 | 7943 | 22 | 3.9 | 97% | 11 | 3% |
| Warrior | 4 | 9493 | 36 | 2.5 | 98% | 9 | 2% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botwbj00com | Warrior | completionist | 12.0 | 1->3 | 9212 | 8/16 | 15 | 1 | 8 | 75 |
| Botwbj01gri | Paladin | grinder | 12.0 | 1->3 | 10007 | 2/2 | 46 | 1 | 8 | 67 |
| Botwbj02exp | Hunter | explorer | 12.0 | 1->2 | 6371 | 4/7 | 10 | 0 | 13 | 61 |
| Botwbj03com | Rogue | completionist | 12.0 | 1->3 | 11457 | 9/16 | 24 | 2 | 8 | 62 |
| Botwbj04soc | Priest | social | 12.0 | 1->3 | 7052 | 8/15 | 11 | 0 | 10 | 147 |
| Botwbj05cas | Mage | casual | 6.9 | 1->2 | 7103 | 5/8 | 9 | 0 | 8 | 53 |
| Botwbj06com | Warrior | completionist | 12.0 | 1->4 | 13623 | 10/16 | 24 | 1 | 8 | 55 |
| Botwbj07gri | Paladin | grinder | 12.0 | 1->3 | 8767 | 2/2 | 50 | 1 | 8 | 71 |
| Botwbj08par | Hunter | party | 12.0 | 1->3 | 9235 | 9/16 | 13 | 0 | 7 | 57 |
| Botwbj09par | Rogue | party | 12.0 | 1->3 | 7337 | 7/17 | 6 | 0 | 14 | 82 |
| Botwbj10com | Priest | completionist | 12.0 | 1->3 | 8967 | 9/16 | 13 | 2 | 8 | 62 |
| Botwbj11gri | Mage | grinder | 12.0 | 1->3 | 9222 | 2/2 | 51 | 2 | 4 | 67 |
| Botwbj12exp | Warrior | explorer | 12.0 | 1->3 | 8528 | 8/16 | 10 | 0 | 13 | 62 |
| Botwbj13com | Paladin | completionist | 12.0 | 1->3 | 11858 | 11/17 | 14 | 0 | 12 | 55 |
| Botwbj14soc | Hunter | social | 12.0 | 1->3 | 9167 | 10/15 | 14 | 0 | 8 | 88 |
| Botwbj15cas | Rogue | casual | 7.0 | 1->2 | 7943 | 6/9 | 8 | 0 | 11 | 70 |
| Botwbj16com | Priest | completionist | 12.0 | 1->3 | 10493 | 9/15 | 15 | 0 | 8 | 110 |
| Botwbj17gri | Mage | grinder | 12.0 | 1->3 | 11177 | 2/2 | 55 | 4 | 6 | 55 |
| Botwbj18par | Warrior | party | 12.0 | 1->3 | 9773 | 10/19 | 11 | 0 | 10 | 55 |
| Botwbj19par | Paladin | party | 12.0 | 1->3 | 7471 | 7/17 | 13 | 0 | 12 | 98 |
