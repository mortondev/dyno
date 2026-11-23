# Hearthblock playtest: 20 bots, 12 min, brain mix

20 bots on `127.0.0.1:7543`, 236.8 bot-minutes (longest session 12.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 3 fail, 2 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 48.23 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 8.44 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 251.08 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.35 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.55  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 1.39 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 8.48 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 9.12 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.95  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 4.85 s | <= 25 pass, > 40 fail (and >= 5) |
| WARN | Share of time idle, stuck, waiting for spawns or dead | 0.17  | <= 0.15 pass, > 0.3 fail |
| PASS | Share of time travelling | 0.24  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 26.75 blocks | <= 60 pass, > 120 fail |
| WARN | Distinct abilities used per fighting bot | 1.85  | >= 2 pass, < 1.5 fail |
| FAIL | Share of casts that are the most used ability | 0.91  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.29  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 1.01 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 8234.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 48 s; first quest turned in: median 8 s; level 2: median 251 s (100% of level-1 starters reached it)
- XP per hour: median 8624 (design pace 6400); 55% of XP from quests; 119 quests turned in, median 1.4 min each
- Rewards: median gap 8 s; median longest drought 104 s; 0% of bots had a drought over 5 min
- Combat: 941 fights, 95% won, median 4.8 s; 310 kills, 128 without credit (6 of them with no other player within 20 blocks); 36 deaths (9.1/h)
- Travel: 27 blocks per reward; money earned 8234 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 1.0 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 2.0% |
| dead | 0.8% |
| fight | 43.2% |
| gather | 5.8% |
| idle | 2.8% |
| loot | 4.3% |
| rest | 2.5% |
| town | 1.3% |
| travel | 24.1% |
| wait_spawn | 13.2% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 20 | 1.0 | 0 |
| 1 | 20 | 1.0 | 151 |
| 2 | 20 | 1.0 | 210 |
| 3 | 20 | 1.0 | 314 |
| 4 | 20 | 1.0 | 389 |
| 5 | 20 | 2.0 | 586 |
| 6 | 20 | 2.0 | 744 |
| 7 | 20 | 2.0 | 936 |
| 8 | 20 | 2.0 | 1107 |
| 9 | 19 | 2.0 | 1271 |
| 10 | 19 | 3.0 | 1447 |
| 11 | 19 | 3.0 | 1657 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 6 | 1 | 1.5 |
| coal_for_the_watch | 16 | 14 | 1.4 |
| cold_hearth | 20 | 20 | 0.1 |
| down_to_water | 5 | 2 | 0.8 |
| dust_of_shadows | 2 | 0 | - |
| eyes_in_the_dark | 7 | 2 | 2.1 |
| garricks_camp | 6 | 3 | 0.3 |
| iron_for_hollis | 13 | 0 | - |
| kindling | 16 | 16 | 3.0 |
| meat_for_the_pot | 16 | 14 | 6.1 |
| north_watch | 10 | 7 | 0.3 |
| pincher_supper | 2 | 0 | - |
| report_to_holt | 7 | 6 | 0.1 |
| silk_for_sails | 3 | 0 | - |
| stolen_grain | 5 | 0 | - |
| thieves_in_fields | 5 | 0 | - |
| thin_the_herd | 20 | 14 | 8.8 |
| tusks_and_tempers | 3 | 0 | - |
| watchfire | 6 | 4 | 0.1 |
| windmill_breadcrumb | 11 | 5 | 1.0 |
| wool_for_winter | 14 | 11 | 1.3 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x16
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x4
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x3
- watchfire: collect Coal: no source in reach x3
- thin_the_herd: goto failed: too many repaths x2
- kindling: collect Log: no source in reach x1

## Deaths by cause

- You fell to a Thornboar. x10
- You fell to a Ashcloak Bandit. x7
- You fell to a Shade. x7
- You fell to a Rook Varn. x5
- You fell to a Ashcloak Cutthroat. x2
- You fell to a Gloomwisp. x2
- You fell to a Websnare. x2
- You fell to a Candlekin. x1

## Fights by creature and outcome

- Grazer: won x721
- Shade: won x73
- Thornboar: won x47
- Ashcloak Bandit: won x25
- Websnare: won x21
- Thornboar: lost (died) x10
- Ashcloak Bandit: lost (died) x7
- Shade: lost (died) x6
- Rook Varn: lost (died) x5
- Mirepincher: won x4
- Gloomwisp: lost (could not reach or hurt it) x2
- Gloomwisp: lost (died) x2
- Rook Varn: lost (it fled home and healed) x2
- Thornboar: lost (it fled home and healed) x2
- Websnare: lost (died) x2
- Ashcloak Bandit: lost (it fled home and healed) x1
- Ashcloak Cutthroat: lost (could not reach or hurt it) x1
- Ashcloak Cutthroat: lost (died) x1
- Candlekin: lost (could not reach or hurt it) x1
- Candlekin: lost (died) x1
- Gloomwisp: won x1
- Grazer: lost (could not reach or hurt it) x1
- Grazer: lost (died) x1
- Shade: lost (could not reach or hurt it) x1
- Thornboar: lost (could not reach or hurt it) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 98 | Quick Shot 93%, Barbed Arrow 7% |
| Mage | 210 | Fireball 98%, Scholar's Focus 1%, Recall 0% |
| Paladin | 77 | Judging Blow 90%, Holy Mend 10% |
| Priest | 191 | Smite 97%, Mend 2%, Withering Word 1% |
| Rogue | 88 | Vicious Strike 78%, Finishing Cut 22% |
| Warrior | 45 | Heroic Blow 76%, Battle Cry 24% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [0, 80]: 2 events
  - Botgyd03com at [5, -8, 82] going to [5, -9, 88] (repaths; thin_the_herd: go to the Grazer camp)
  - Botgyd16com at [5, -8, 82] going to [5, -9, 88] (repaths; thin_the_herd: go to the Grazer camp)
- cell [-16, 48]: 1 events
  - Botgyd16com at [-14, -7, 60] going to [-20, -5, 60] (repaths; thin_the_herd: go to the Grazer camp)
- cell [-16, 128]: 1 events
  - Botgyd13com at [-14, -13, 134] going to [-14, -16, 134] (repaths; iron_for_hollis: gather Raw iron)

## Notes

- grouped in a party of two x4
- casual session ended x1
- tried /invite Botgyd05cas x1
- tried /invite Botgyd16com x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 2 | 5946 | 8 | 0.0 | 100% | 11 | 19% |
| completionist | 6 | 7865 | 43 | 7.5 | 96% | 9 | 19% |
| explorer | 2 | 8249 | 13 | 12.5 | 88% | 10 | 13% |
| grinder | 4 | 8137 | 8 | 17.5 | 93% | 9 | 12% |
| party | 4 | 8877 | 25 | 6.3 | 96% | 10 | 20% |
| social | 2 | 13596 | 22 | 7.5 | 92% | 6 | 16% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 3 | 13768 | 32 | 11.7 | 88% | 5 | 15% |
| Mage | 3 | 9018 | 10 | 15.0 | 93% | 8 | 13% |
| Paladin | 4 | 6569 | 15 | 6.3 | 97% | 11 | 17% |
| Priest | 3 | 12392 | 24 | 5.0 | 98% | 8 | 18% |
| Rogue | 3 | 8631 | 12 | 12.8 | 89% | 8 | 19% |
| Warrior | 4 | 6825 | 26 | 6.3 | 97% | 11 | 19% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botgyd00com | Warrior | completionist | 12.0 | 1->3 | 7088 | 7/12 | 10 | 0 | 9 | 186 |
| Botgyd01gri | Paladin | grinder | 12.0 | 1->2 | 6041 | 2/2 | 17 | 2 | 10 | 127 |
| Botgyd02exp | Hunter | explorer | 12.0 | 1->3 | 10201 | 9/14 | 13 | 3 | 6 | 97 |
| Botgyd03com | Rogue | completionist | 12.0 | 1->3 | 8631 | 6/11 | 13 | 4 | 8 | 202 |
| Botgyd04soc | Priest | social | 12.0 | 1->3 | 12392 | 10/16 | 18 | 1 | 6 | 77 |
| Botgyd05cas | Mage | casual | 12.0 | 1->3 | 9018 | 6/9 | 13 | 0 | 8 | 94 |
| Botgyd06com | Warrior | completionist | 12.0 | 1->3 | 12222 | 9/14 | 17 | 3 | 9 | 91 |
| Botgyd07gri | Paladin | grinder | 12.0 | 1->3 | 8617 | 2/2 | 28 | 3 | 10 | 168 |
| Botgyd08par | Hunter | party | 12.0 | 1->4 | 13768 | 11/20 | 25 | 2 | 5 | 80 |
| Botgyd09par | Rogue | party | 12.0 | 1->3 | 11192 | 4/7 | 18 | 3 | 7 | 111 |
| Botgyd10com | Priest | completionist | 12.0 | 1->3 | 6841 | 5/8 | 9 | 1 | 9 | 119 |
| Botgyd11gri | Mage | grinder | 12.0 | 1->3 | 12424 | 2/2 | 33 | 6 | 6 | 59 |
| Botgyd12exp | Warrior | explorer | 12.0 | 1->2 | 6297 | 4/6 | 9 | 2 | 14 | 118 |
| Botgyd13com | Paladin | completionist | 12.0 | 1->3 | 7098 | 7/12 | 8 | 0 | 12 | 91 |
| Botgyd14soc | Hunter | social | 12.0 | 1->4 | 14801 | 12/19 | 21 | 2 | 5 | 81 |
| Botgyd15cas | Rogue | casual | 8.9 | 1->2 | 2873 | 2/5 | 3 | 0 | 15 | 204 |
| Botgyd16com | Priest | completionist | 12.0 | 1->4 | 14688 | 9/15 | 18 | 1 | 8 | 78 |
| Botgyd17gri | Mage | grinder | 12.0 | 1->3 | 7658 | 2/2 | 23 | 3 | 8 | 79 |
| Botgyd18par | Warrior | party | 12.0 | 1->3 | 6562 | 6/10 | 8 | 0 | 13 | 161 |
| Botgyd19par | Paladin | party | 12.0 | 1->2 | 4866 | 4/7 | 6 | 0 | 14 | 135 |
