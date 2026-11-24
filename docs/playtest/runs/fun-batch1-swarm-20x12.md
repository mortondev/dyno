# Hearthblock playtest: 20 bots, 12 min, brain mix

20 bots on `127.0.0.1:7710`, 235.7 bot-minutes (longest session 12.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 2 fail, 3 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 92.08 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.80 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 190.25 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.63 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.65  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 3.73 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 14.45 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| WARN | Deaths per hour of play | 3.31 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.97  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 5.17 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.02  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.62  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 44.50 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.40  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.72  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.27  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 1.78 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 6755.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 92 s; first quest turned in: median 8 s; level 2: median 190 s (100% of level-1 starters reached it)
- XP per hour: median 10410 (design pace 6400); 65% of XP from quests; 110 quests turned in, median 3.7 min each
- Rewards: median gap 14 s; median longest drought 82 s; 0% of bots had a drought over 5 min
- Combat: 575 fights, 97% won, median 5.2 s; 306 kills, 114 without credit (7 of them with no other player within 20 blocks); 13 deaths (3.3/h)
- Travel: 44 blocks per reward; money earned 6755 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 1.8 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 2.0% |
| dead | 0.3% |
| fight | 27.2% |
| gather | 1.5% |
| idle | 1.6% |
| loot | 2.5% |
| rest | 1.2% |
| town | 1.3% |
| travel | 62.5% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 20 | 1.0 | 0 |
| 1 | 20 | 1.0 | 137 |
| 2 | 20 | 1.0 | 238 |
| 3 | 20 | 1.0 | 358 |
| 4 | 20 | 2.0 | 546 |
| 5 | 20 | 2.0 | 723 |
| 6 | 20 | 2.0 | 871 |
| 7 | 20 | 2.0 | 1080 |
| 8 | 20 | 2.0 | 1282 |
| 9 | 19 | 3.0 | 1458 |
| 10 | 19 | 3.0 | 1719 |
| 11 | 19 | 3.0 | 1936 |
| 12 | 1 | 3.0 | 1705 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 5 | 0 | - |
| broken_tower | 16 | 14 | 7.2 |
| coal_for_the_watch | 10 | 0 | - |
| cold_hearth | 20 | 20 | 0.1 |
| down_to_water | 3 | 3 | 1.1 |
| eyes_in_the_dark | 5 | 0 | - |
| garricks_camp | 5 | 0 | - |
| kindling | 16 | 10 | 3.3 |
| lay_of_the_land | 14 | 3 | 6.3 |
| lookouts_eye | 5 | 3 | 0.6 |
| meat_for_the_pot | 16 | 16 | 4.5 |
| north_watch | 15 | 5 | 1.6 |
| pincher_supper | 3 | 0 | - |
| stolen_grain | 3 | 0 | - |
| thieves_in_fields | 3 | 0 | - |
| thin_the_herd | 20 | 19 | 6.0 |
| watchfire | 5 | 0 | - |
| windmill_breadcrumb | 13 | 3 | 1.7 |
| wool_for_winter | 16 | 14 | 2.5 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x10
- watchfire: collect Coal: no source in reach x5
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x4
- kindling: collect Log: no source in reach x4
- thin_the_herd: goto failed: too many repaths x2
- cold_hearth: talk failed: NPC is not here x1
- eyes_in_the_dark: goto failed: too many repaths x1

## Deaths by cause

- You fell to a Shade. x8
- You fell to a Thornboar. x2
- You fell to a Ashcloak Cutthroat. x1
- You fell to a Grazer. x1
- You fell to a Mirepincher. x1

## Fights by creature and outcome

- Grazer: won x437
- Shade: won x67
- Thornboar: won x52
- Shade: lost (died) x8
- Mirepincher: lost (it fled home and healed) x3
- Thornboar: lost (died) x2
- Thornboar: lost (it fled home and healed) x2
- Ashcloak Bandit: lost (died) x1
- Grazer: lost (died) x1
- Mirepincher: lost (died) x1
- Mirepincher: won x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 75 | Quick Shot 61%, Barbed Arrow 35%, Recall 4% |
| Mage | 193 | Fireball 67%, Fire Blast 32%, Scholar's Focus 2% |
| Paladin | 97 | Judging Blow 90%, Rebuke 6%, Holy Mend 3%, Recall 1% |
| Priest | 163 | Smite 69%, Withering Word 29%, Recall 2% |
| Rogue | 81 | Vicious Strike 79%, Finishing Cut 19%, Recall 2% |
| Warrior | 57 | Heroic Blow 51%, Rending Cut 26%, Battle Cry 16%, Recall 7% |

## Unreachable NPCs

- Tilda (NPC is not here) x1

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-112, -96]: 1 events
  - Botrwx08par at [-104, -6, -92] going to [-98, -5, -92] (repaths; thin_the_herd: go to the Grazer camp)
- cell [-64, -64]: 1 events
  - Botrwx19par at [-57, -1, -52] going to [-60, -2, -55] (repaths; following Botrwx18par)
- cell [-32, 16]: 1 events
  - Botrwx09par at [-22, 0, 16] going to [-24, 0, 19] (repaths; following Botrwx08par)
- cell [0, 160]: 1 events
  - Botrwx12exp at [10, -9, 168] going to [10, 0, 180] (repaths; exploring The Broken Tower)
- cell [32, -48]: 1 events
  - Botrwx13com at [36, 0, -38] going to [36, 0, -34] (repaths; eyes_in_the_dark: look for Shade in the dark)
- cell [48, -160]: 1 events
  - Botrwx05cas at [60, -7, -159] going to [60, -7, -165] (repaths; thin_the_herd: go to the Grazer camp)
- cell [96, -80]: 1 events
  - Botrwx07gri at [108, -3, -78] going to [108, -3, -72] (repaths; going to grind)

## Notes

- grouped in a party of two x4
- casual session ended x2
- tried /invite Botrwx05cas x1
- tried /invite Botrwx16com x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 2 | 8466 | 10 | 0.0 | 100% | 14 | 1% |
| completionist | 6 | 11655 | 44 | 2.5 | 98% | 16 | 2% |
| explorer | 2 | 9743 | 7 | 5.0 | 88% | 16 | 1% |
| grinder | 4 | 9147 | 8 | 8.8 | 97% | 10 | 2% |
| party | 4 | 10410 | 27 | 1.3 | 94% | 25 | 3% |
| social | 2 | 11193 | 14 | 0.0 | 98% | 13 | 2% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 3 | 11612 | 19 | 1.7 | 91% | 13 | 2% |
| Mage | 3 | 9701 | 8 | 1.8 | 99% | 6 | 1% |
| Paladin | 4 | 9555 | 19 | 7.5 | 96% | 16 | 2% |
| Priest | 3 | 9018 | 19 | 0.0 | 100% | 18 | 2% |
| Rogue | 3 | 10304 | 21 | 3.4 | 94% | 24 | 2% |
| Warrior | 4 | 10967 | 24 | 3.8 | 96% | 20 | 2% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botrwx00com | Warrior | completionist | 12.0 | 1->3 | 12977 | 8/15 | 15 | 2 | 14 | 86 |
| Botrwx01gri | Paladin | grinder | 12.0 | 1->3 | 8279 | 2/2 | 27 | 2 | 18 | 76 |
| Botrwx02exp | Hunter | explorer | 12.0 | 1->3 | 8219 | 3/7 | 7 | 1 | 15 | 91 |
| Botrwx03com | Rogue | completionist | 12.0 | 1->4 | 16381 | 8/15 | 18 | 1 | 10 | 59 |
| Botrwx04soc | Priest | social | 12.0 | 1->3 | 9018 | 6/10 | 10 | 0 | 13 | 87 |
| Botrwx05cas | Mage | casual | 8.8 | 1->2 | 7693 | 4/7 | 8 | 0 | 5 | 114 |
| Botrwx06com | Warrior | completionist | 12.0 | 1->3 | 10668 | 7/10 | 8 | 0 | 24 | 93 |
| Botrwx07gri | Paladin | grinder | 12.0 | 1->3 | 8593 | 2/2 | 30 | 4 | 13 | 69 |
| Botrwx08par | Hunter | party | 12.0 | 1->3 | 11612 | 8/14 | 14 | 0 | 8 | 90 |
| Botrwx09par | Rogue | party | 12.0 | 1->3 | 10304 | 7/13 | 8 | 1 | 24 | 64 |
| Botrwx10com | Priest | completionist | 12.0 | 1->3 | 8524 | 6/10 | 10 | 0 | 18 | 136 |
| Botrwx11gri | Mage | grinder | 12.0 | 1->3 | 10549 | 2/2 | 39 | 0 | 6 | 78 |
| Botrwx12exp | Warrior | explorer | 12.0 | 1->3 | 11267 | 4/12 | 10 | 1 | 17 | 87 |
| Botrwx13com | Paladin | completionist | 12.0 | 1->3 | 12284 | 8/15 | 13 | 0 | 12 | 97 |
| Botrwx14soc | Hunter | social | 12.0 | 1->3 | 13368 | 8/15 | 16 | 0 | 13 | 58 |
| Botrwx15cas | Rogue | casual | 11.0 | 1->3 | 9239 | 6/10 | 8 | 0 | 24 | 77 |
| Botrwx16com | Priest | completionist | 12.0 | 1->3 | 11026 | 7/10 | 11 | 0 | 21 | 105 |
| Botrwx17gri | Mage | grinder | 12.0 | 1->3 | 9701 | 2/2 | 37 | 1 | 7 | 74 |
| Botrwx18par | Warrior | party | 12.0 | 1->3 | 9458 | 5/9 | 9 | 0 | 27 | 79 |
| Botrwx19par | Paladin | party | 12.0 | 1->3 | 10516 | 7/13 | 8 | 0 | 32 | 73 |
