# Hearthblock playtest: 20 bots, 12 min, brain mix

20 bots on `127.0.0.1:7710`, 239.9 bot-minutes (longest session 12.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 1 fail, 3 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 41.81 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.73 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 171.22 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.31 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.61  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.41 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 11.68 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.10  | <= 0.1 pass, > 0.25 fail |
| WARN | Deaths per hour of play | 2.75 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.98  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 4.98 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.06  | <= 0.15 pass, > 0.3 fail |
| WARN | Share of time travelling | 0.49  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 32.30 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.30  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.79  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.31  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 0.75 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 9512.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 42 s; first quest turned in: median 8 s; level 2: median 171 s (95% of level-1 starters reached it)
- XP per hour: median 8412 (design pace 6400); 61% of XP from quests; 132 quests turned in, median 2.4 min each
- Rewards: median gap 12 s; median longest drought 75 s; 10% of bots had a drought over 5 min
- Combat: 678 fights, 98% won, median 5.0 s; 383 kills, 175 without credit (4 of them with no other player within 20 blocks); 11 deaths (2.8/h)
- Travel: 32 blocks per reward; money earned 9512 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 0.8 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 1.7% |
| dead | 0.2% |
| fight | 30.2% |
| gather | 2.5% |
| idle | 6.2% |
| loot | 3.3% |
| rest | 1.8% |
| town | 4.7% |
| travel | 49.4% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 20 | 1.0 | 0 |
| 1 | 20 | 1.0 | 123 |
| 2 | 20 | 1.0 | 257 |
| 3 | 20 | 2.0 | 421 |
| 4 | 20 | 2.0 | 580 |
| 5 | 20 | 2.0 | 716 |
| 6 | 20 | 2.0 | 828 |
| 7 | 20 | 2.0 | 944 |
| 8 | 20 | 2.0 | 1068 |
| 9 | 20 | 2.0 | 1157 |
| 10 | 20 | 3.0 | 1278 |
| 11 | 20 | 3.0 | 1422 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 10 | 0 | - |
| broken_tower | 16 | 14 | 6.0 |
| coal_for_the_watch | 15 | 7 | 5.1 |
| cold_hearth | 20 | 20 | 0.1 |
| down_to_water | 3 | 0 | - |
| dust_of_shadows | 1 | 0 | - |
| eyes_in_the_dark | 10 | 2 | 2.7 |
| garricks_camp | 10 | 0 | - |
| iron_for_hollis | 6 | 0 | - |
| kindling | 16 | 15 | 1.8 |
| lay_of_the_land | 14 | 6 | 4.0 |
| lookouts_eye | 10 | 4 | 0.8 |
| meat_for_the_pot | 16 | 15 | 3.1 |
| north_watch | 14 | 10 | 2.9 |
| stolen_grain | 3 | 0 | - |
| thieves_in_fields | 3 | 0 | - |
| thin_the_herd | 20 | 18 | 4.1 |
| watchfire | 10 | 3 | 0.5 |
| windmill_breadcrumb | 13 | 3 | 2.0 |
| wool_for_winter | 15 | 15 | 0.9 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x14
- watchfire: collect Coal: no source in reach x7
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x6
- iron_for_hollis: collect Raw iron: no source in reach x4
- kindling: collect Log: no source in reach x4
- iron_for_hollis: collect Raw iron: Requires mining 5 (have 4) x2
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x2
- eyes_in_the_dark: goto failed: too many repaths x1
- thin_the_herd: goto failed: too many repaths x1

## Deaths by cause

- You fell to a Shade. x6
- You fell to a Grazer. x3
- You fell to a Thornboar. x2

## Fights by creature and outcome

- Grazer: won x540
- Shade: won x95
- Thornboar: won x31
- Shade: lost (died) x6
- Grazer: lost (died) x3
- Thornboar: lost (died) x2
- Thornboar: lost (it fled home and healed) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 66 | Quick Shot 59%, Barbed Arrow 35%, Recall 6% |
| Mage | 254 | Fireball 65%, Fire Blast 33%, Scholar's Focus 1%, Recall 1% |
| Paladin | 116 | Judging Blow 90%, Holy Mend 7%, Rebuke 2%, Recall 2% |
| Priest | 140 | Smite 79%, Withering Word 17%, Recall 4% |
| Rogue | 71 | Vicious Strike 83%, Finishing Cut 14%, Recall 3% |
| Warrior | 65 | Heroic Blow 63%, Rending Cut 15%, Battle Cry 12%, Recall 9% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-208, -80]: 1 events
  - Botkhj12exp at [-196, -6, -70] going to [-190, 1, -70] (repaths; thin_the_herd: go to the Grazer camp)
- cell [-96, 16]: 1 events
  - Botkhj02exp at [-90, -4, 18] going to [-96, -1, 26] (repaths; exploring West Gate Rocks)
- cell [32, 16]: 1 events
  - Botkhj16com at [36, 4, 30] going to [36, -4, 34] (repaths; eyes_in_the_dark: look for Shade in the dark)

## Notes

- grouped in a party of two x4
- tried /invite Botkhj00com x1
- tried /invite Botkhj13com x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 2 | 8426 | 18 | 0.0 | 100% | 12 | 2% |
| completionist | 6 | 9339 | 54 | 1.7 | 99% | 12 | 2% |
| explorer | 2 | 7149 | 13 | 0.0 | 100% | 14 | 3% |
| grinder | 4 | 9897 | 8 | 10.0 | 98% | 6 | 2% |
| party | 4 | 5937 | 20 | 1.3 | 98% | 14 | 24% |
| social | 2 | 9087 | 19 | 0.0 | 98% | 8 | 2% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 3 | 7541 | 22 | 1.7 | 96% | 7 | 3% |
| Mage | 3 | 10648 | 12 | 11.7 | 96% | 4 | 2% |
| Paladin | 4 | 8487 | 20 | 3.8 | 99% | 14 | 2% |
| Priest | 3 | 8692 | 28 | 0.0 | 100% | 9 | 2% |
| Rogue | 3 | 8687 | 20 | 0.0 | 100% | 14 | 30% |
| Warrior | 4 | 7252 | 30 | 0.0 | 100% | 13 | 2% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botkhj00com | Warrior | completionist | 12.0 | 1->3 | 10312 | 9/19 | 14 | 0 | 9 | 79 |
| Botkhj01gri | Paladin | grinder | 12.0 | 1->3 | 9146 | 2/2 | 52 | 1 | 8 | 106 |
| Botkhj02exp | Hunter | explorer | 12.0 | 1->3 | 7541 | 7/10 | 8 | 0 | 12 | 69 |
| Botkhj03com | Rogue | completionist | 12.0 | 1->3 | 8687 | 9/18 | 13 | 0 | 14 | 87 |
| Botkhj04soc | Priest | social | 12.0 | 1->3 | 8692 | 9/16 | 13 | 0 | 9 | 55 |
| Botkhj05cas | Mage | casual | 12.0 | 1->3 | 7721 | 8/15 | 15 | 0 | 9 | 57 |
| Botkhj06com | Warrior | completionist | 12.0 | 1->3 | 7742 | 8/16 | 18 | 0 | 13 | 99 |
| Botkhj07gri | Paladin | grinder | 12.0 | 1->3 | 7827 | 2/2 | 41 | 0 | 14 | 68 |
| Botkhj08par | Hunter | party | 12.0 | 1->2 | 5578 | 5/8 | 10 | 1 | 7 | 374 |
| Botkhj09par | Rogue | party | 12.0 | 1->1 | 1155 | 1/5 | 3 | 0 | 23 | 637 |
| Botkhj10com | Priest | completionist | 12.0 | 1->3 | 9992 | 10/16 | 13 | 0 | 11 | 56 |
| Botkhj11gri | Mage | grinder | 12.0 | 1->3 | 10662 | 2/2 | 49 | 3 | 4 | 48 |
| Botkhj12exp | Warrior | explorer | 12.0 | 1->3 | 6757 | 6/10 | 7 | 0 | 16 | 71 |
| Botkhj13com | Paladin | completionist | 12.0 | 1->3 | 11507 | 9/16 | 16 | 2 | 15 | 64 |
| Botkhj14soc | Hunter | social | 12.0 | 1->3 | 9482 | 10/18 | 14 | 0 | 7 | 88 |
| Botkhj15cas | Rogue | casual | 12.0 | 1->3 | 9132 | 10/16 | 11 | 0 | 14 | 59 |
| Botkhj16com | Priest | completionist | 12.0 | 1->3 | 8137 | 9/15 | 13 | 0 | 7 | 100 |
| Botkhj17gri | Mage | grinder | 12.0 | 1->3 | 10648 | 2/2 | 50 | 4 | 4 | 60 |
| Botkhj18par | Warrior | party | 12.0 | 1->3 | 6762 | 7/10 | 13 | 0 | 14 | 116 |
| Botkhj19par | Paladin | party | 12.0 | 1->2 | 6297 | 7/9 | 10 | 0 | 14 | 88 |
