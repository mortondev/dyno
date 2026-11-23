# Hearthblock playtest: 20 bots, 12 min, brain mix

20 bots on `127.0.0.1:7710`, 232.1 bot-minutes (longest session 12.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 2 fail, 2 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 49.01 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.77 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 171.30 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.42 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.63  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.56 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 9.39 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 4.14 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.98  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 5.03 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.03  | <= 0.15 pass, > 0.3 fail |
| WARN | Share of time travelling | 0.55  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 33.24 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.40  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.74  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.30  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 0.00 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 9720.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 49 s; first quest turned in: median 8 s; level 2: median 171 s (100% of level-1 starters reached it)
- XP per hour: median 9072 (design pace 6400); 63% of XP from quests; 137 quests turned in, median 2.6 min each
- Rewards: median gap 9 s; median longest drought 65 s; 0% of bots had a drought over 5 min
- Combat: 694 fights, 98% won, median 5.0 s; 377 kills, 164 without credit (3 of them with no other player within 20 blocks); 16 deaths (4.1/h)
- Travel: 33 blocks per reward; money earned 9720 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 0.0 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 0.8% |
| dead | 0.3% |
| fight | 32.6% |
| gather | 2.8% |
| idle | 2.0% |
| loot | 2.2% |
| rest | 2.2% |
| stuck | 0.5% |
| town | 1.9% |
| travel | 54.7% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 20 | 1.0 | 0 |
| 1 | 20 | 1.0 | 106 |
| 2 | 20 | 1.0 | 258 |
| 3 | 20 | 2.0 | 402 |
| 4 | 20 | 2.0 | 559 |
| 5 | 20 | 2.0 | 716 |
| 6 | 20 | 2.0 | 835 |
| 7 | 20 | 2.0 | 975 |
| 8 | 19 | 2.0 | 1103 |
| 9 | 18 | 2.5 | 1243 |
| 10 | 18 | 3.0 | 1339 |
| 11 | 18 | 3.0 | 1534 |
| 12 | 1 | 2.0 | 1236 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 12 | 0 | - |
| broken_tower | 16 | 15 | 5.8 |
| coal_for_the_watch | 14 | 5 | 4.3 |
| cold_hearth | 20 | 20 | 0.1 |
| down_to_water | 2 | 0 | - |
| dust_of_shadows | 1 | 0 | - |
| eyes_in_the_dark | 12 | 4 | 3.6 |
| garricks_camp | 12 | 4 | 1.2 |
| iron_for_hollis | 4 | 0 | - |
| kindling | 16 | 14 | 1.8 |
| lay_of_the_land | 15 | 3 | 3.5 |
| lookouts_eye | 12 | 6 | 0.8 |
| meat_for_the_pot | 16 | 15 | 2.5 |
| north_watch | 15 | 12 | 3.7 |
| silk_for_sails | 2 | 0 | - |
| stolen_grain | 2 | 0 | - |
| thieves_in_fields | 2 | 0 | - |
| thin_the_herd | 20 | 19 | 4.1 |
| tusks_and_tempers | 4 | 0 | - |
| watchfire | 12 | 4 | 0.4 |
| windmill_breadcrumb | 12 | 2 | 2.3 |
| wool_for_winter | 15 | 14 | 2.0 |
| word_for_rysa | 2 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x12
- watchfire: collect Coal: no source in reach x9
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x8
- iron_for_hollis: collect Raw iron: no source in reach x3
- iron_for_hollis: collect Raw iron: Requires mining 5 (have 4) x1
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x1
- kindling: collect Log: no source in reach x1

## Deaths by cause

- You fell to a Grazer. x11
- You fell to a Shade. x3
- You fell to a Thornboar. x2

## Fights by creature and outcome

- Grazer: won x563
- Shade: won x86
- Thornboar: won x29
- Grazer: lost (died) x11
- Shade: lost (died) x3
- Thornboar: lost (died) x2

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 80 | Quick Shot 55%, Barbed Arrow 38%, Recall 8% |
| Mage | 228 | Fireball 67%, Fire Blast 32%, Scholar's Focus 1%, Recall 0% |
| Paladin | 110 | Judging Blow 90%, Holy Mend 6%, Rebuke 3%, Recall 1% |
| Priest | 172 | Smite 73%, Withering Word 24%, Recall 3% |
| Rogue | 66 | Vicious Strike 85%, Finishing Cut 12%, Recall 3% |
| Warrior | 60 | Heroic Blow 57%, Battle Cry 18%, Rending Cut 15%, Recall 10% |

## Notes

- grouped in a party of two x4
- casual session ended x2
- tried /invite Botnup00com x1
- tried /invite Botnup13com x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 2 | 8464 | 13 | 0.0 | 100% | 9 | 2% |
| completionist | 6 | 9491 | 56 | 0.8 | 99% | 9 | 2% |
| explorer | 2 | 6889 | 9 | 0.0 | 100% | 12 | 2% |
| grinder | 4 | 9517 | 8 | 16.3 | 96% | 8 | 2% |
| party | 4 | 8129 | 30 | 1.3 | 99% | 11 | 6% |
| social | 2 | 9844 | 21 | 2.5 | 98% | 10 | 2% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 3 | 9023 | 26 | 0.0 | 100% | 11 | 2% |
| Mage | 3 | 9121 | 11 | 18.4 | 94% | 5 | 3% |
| Paladin | 4 | 9452 | 17 | 5.0 | 98% | 12 | 6% |
| Priest | 3 | 8838 | 30 | 1.7 | 99% | 8 | 2% |
| Rogue | 3 | 7807 | 21 | 1.9 | 98% | 12 | 2% |
| Warrior | 4 | 8465 | 32 | 0.0 | 100% | 9 | 2% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botnup00com | Warrior | completionist | 12.0 | 1->3 | 10455 | 11/16 | 13 | 0 | 9 | 53 |
| Botnup01gri | Paladin | grinder | 12.0 | 1->3 | 9461 | 2/2 | 40 | 1 | 12 | 61 |
| Botnup02exp | Hunter | explorer | 12.0 | 1->3 | 9023 | 6/10 | 12 | 0 | 11 | 77 |
| Botnup03com | Rogue | completionist | 12.0 | 1->3 | 7007 | 8/16 | 12 | 0 | 9 | 98 |
| Botnup04soc | Priest | social | 12.0 | 1->3 | 8838 | 10/18 | 14 | 1 | 8 | 63 |
| Botnup05cas | Mage | casual | 8.7 | 1->3 | 9121 | 7/10 | 14 | 0 | 4 | 53 |
| Botnup06com | Warrior | completionist | 12.0 | 1->3 | 9539 | 10/18 | 16 | 0 | 9 | 81 |
| Botnup07gri | Paladin | grinder | 12.0 | 1->3 | 9573 | 2/2 | 42 | 2 | 10 | 71 |
| Botnup08par | Hunter | party | 12.0 | 1->3 | 8867 | 9/18 | 16 | 0 | 8 | 119 |
| Botnup09par | Rogue | party | 12.0 | 1->3 | 9814 | 7/14 | 10 | 1 | 12 | 116 |
| Botnup10com | Priest | completionist | 12.0 | 1->3 | 7713 | 9/15 | 13 | 0 | 7 | 91 |
| Botnup11gri | Mage | grinder | 12.0 | 1->3 | 8128 | 2/2 | 43 | 5 | 7 | 64 |
| Botnup12exp | Warrior | explorer | 12.0 | 1->2 | 4755 | 3/7 | 7 | 0 | 13 | 54 |
| Botnup13com | Paladin | completionist | 12.0 | 1->3 | 9444 | 7/15 | 19 | 1 | 12 | 65 |
| Botnup14soc | Hunter | social | 12.0 | 1->3 | 10851 | 11/17 | 14 | 0 | 13 | 54 |
| Botnup15cas | Rogue | casual | 7.5 | 1->2 | 7807 | 6/9 | 8 | 0 | 14 | 50 |
| Botnup16com | Priest | completionist | 12.0 | 1->3 | 11090 | 11/16 | 14 | 0 | 9 | 54 |
| Botnup17gri | Mage | grinder | 12.0 | 1->3 | 11403 | 2/2 | 54 | 5 | 5 | 51 |
| Botnup18par | Warrior | party | 12.0 | 1->3 | 7391 | 8/18 | 10 | 0 | 10 | 89 |
| Botnup19par | Paladin | party | 12.0 | 1->2 | 6174 | 6/13 | 6 | 0 | 14 | 135 |
