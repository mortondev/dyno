# Hearthblock playtest: 20 bots, 12 min, brain mix

20 bots on `127.0.0.1:7548`, 239.3 bot-minutes (longest session 12.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 2 fail, 2 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 49.50 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 8.41 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 190.65 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.39 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.58  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 1.39 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 8.54 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 4.26 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.97  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 5.52 s | <= 25 pass, > 40 fail (and >= 5) |
| WARN | Share of time idle, stuck, waiting for spawns or dead | 0.16  | <= 0.15 pass, > 0.3 fail |
| PASS | Share of time travelling | 0.24  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 26.83 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.45  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.71  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.36  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 1.00 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 7399.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 49 s; first quest turned in: median 8 s; level 2: median 191 s (100% of level-1 starters reached it)
- XP per hour: median 8899 (design pace 6400); 58% of XP from quests; 120 quests turned in, median 1.4 min each
- Rewards: median gap 9 s; median longest drought 134 s; 0% of bots had a drought over 5 min
- Combat: 872 fights, 97% won, median 5.5 s; 282 kills, 158 without credit (10 of them with no other player within 20 blocks); 17 deaths (4.3/h)
- Travel: 27 blocks per reward; money earned 7399 c, spent 0 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 1.0 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 3.6% |
| dead | 0.4% |
| fight | 41.0% |
| gather | 7.8% |
| idle | 2.2% |
| loot | 3.3% |
| rest | 3.4% |
| town | 1.2% |
| travel | 23.6% |
| wait_spawn | 13.5% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 20 | 1.0 | 0 |
| 1 | 20 | 1.0 | 152 |
| 2 | 20 | 1.0 | 274 |
| 3 | 20 | 1.0 | 384 |
| 4 | 20 | 2.0 | 500 |
| 5 | 20 | 2.0 | 614 |
| 6 | 20 | 2.0 | 811 |
| 7 | 20 | 2.0 | 936 |
| 8 | 20 | 2.0 | 1066 |
| 9 | 20 | 2.0 | 1218 |
| 10 | 20 | 3.0 | 1348 |
| 11 | 20 | 3.0 | 1520 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 7 | 0 | - |
| coal_for_the_watch | 16 | 14 | 1.7 |
| cold_hearth | 20 | 20 | 0.1 |
| down_to_water | 6 | 0 | - |
| dust_of_shadows | 1 | 1 | 0.3 |
| eyes_in_the_dark | 7 | 1 | 3.0 |
| garricks_camp | 7 | 2 | 0.6 |
| iron_for_hollis | 13 | 0 | - |
| kindling | 16 | 16 | 1.4 |
| meat_for_the_pot | 16 | 15 | 5.7 |
| north_watch | 10 | 7 | 0.4 |
| report_to_holt | 7 | 7 | 0.1 |
| silk_for_sails | 2 | 0 | - |
| stolen_grain | 6 | 0 | - |
| thieves_in_fields | 6 | 0 | - |
| thin_the_herd | 20 | 14 | 8.9 |
| tusks_and_tempers | 2 | 0 | - |
| watchfire | 7 | 5 | 0.2 |
| windmill_breadcrumb | 12 | 6 | 1.3 |
| wool_for_winter | 15 | 12 | 2.4 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x16
- iron_for_hollis: collect Raw iron: no source in reach x11
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x4
- watchfire: collect Coal: no source in reach x4
- iron_for_hollis: collect Raw iron: Requires mining 5 (have 4) x2
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x2
- thin_the_herd: goto failed: too many repaths x2

## Deaths by cause

- You fell to a Shade. x6
- You fell to a Websnare. x3
- You fell to a Ashcloak Bandit. x2
- You fell to a Gloomwisp. x2
- You fell to a Thornboar. x2
- You fell to a Candlekin. x1
- You fell to a Grazer. x1

## Fights by creature and outcome

- Grazer: won x717
- Shade: won x59
- Thornboar: won x50
- Ashcloak Bandit: won x16
- Shade: lost (died) x5
- Mirepincher: won x4
- Websnare: won x4
- Websnare: lost (died) x3
- Gloomwisp: lost (died) x2
- Thornboar: lost (died) x2
- Ashcloak Bandit: lost (died) x1
- Ashcloak Cutthroat: lost (died) x1
- Ashcloak Cutthroat: lost (it fled home and healed) x1
- Candlekin: lost (died) x1
- Gloomwisp: lost (could not reach or hurt it) x1
- Grazer: lost (cannot reach it: too many repaths) x1
- Grazer: lost (died) x1
- Mirepincher: lost (died) x1
- Shade: lost (it fled home and healed) x1
- Websnare: lost (it fled home and healed) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 103 | Quick Shot 57%, Barbed Arrow 42%, Recall 1% |
| Mage | 202 | Fireball 79%, Fire Blast 20%, Scholar's Focus 1% |
| Paladin | 130 | Judging Blow 78%, Holy Mend 15%, Rebuke 8% |
| Priest | 216 | Smite 69%, Withering Word 31% |
| Rogue | 68 | Vicious Strike 81%, Finishing Cut 19% |
| Warrior | 45 | Heroic Blow 58%, Rending Cut 22%, Battle Cry 20% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [0, -48]: 1 events
  - Botzdn12exp at [0, -7, -46] going to [0, -7, -51] (repaths; exploring The North Watch)
- cell [0, 80]: 1 events
  - Botzdn02exp at [5, -8, 82] going to [5, -9, 88] (repaths; thin_the_herd: go to the Grazer camp)
- cell [16, -64]: 1 events
  - Botzdn13com at [24, -7, -52] going to [30, -5, -52] (repaths; thin_the_herd: go to the Grazer camp)
- cell [48, 16]: 1 events
  - Botzdn07gri at [48, -8, 23] going to [48, -5, 25] (repaths; grinding)

## Notes

- grouped in a party of two x4
- casual session ended x1
- tried /invite Botzdn05cas x1
- tried /invite Botzdn09par x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 2 | 8093 | 11 | 5.1 | 97% | 8 | 13% |
| completionist | 6 | 8435 | 36 | 1.7 | 99% | 8 | 24% |
| explorer | 2 | 9051 | 14 | 17.5 | 85% | 12 | 9% |
| grinder | 4 | 8072 | 8 | 7.5 | 97% | 11 | 9% |
| party | 4 | 9045 | 29 | 0.0 | 100% | 11 | 14% |
| social | 2 | 12720 | 22 | 0.0 | 100% | 6 | 20% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 3 | 9863 | 31 | 8.3 | 90% | 9 | 11% |
| Mage | 3 | 9026 | 12 | 3.3 | 99% | 8 | 12% |
| Paladin | 4 | 8241 | 15 | 5.0 | 97% | 13 | 14% |
| Priest | 3 | 11012 | 25 | 1.7 | 99% | 6 | 30% |
| Rogue | 3 | 7070 | 14 | 3.4 | 97% | 8 | 13% |
| Warrior | 4 | 8904 | 23 | 3.8 | 99% | 9 | 18% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botzdn00com | Warrior | completionist | 12.0 | 1->2 | 4682 | 3/5 | 8 | 1 | 7 | 169 |
| Botzdn01gri | Paladin | grinder | 12.0 | 1->3 | 8888 | 2/2 | 26 | 3 | 14 | 81 |
| Botzdn02exp | Hunter | explorer | 12.0 | 1->3 | 8909 | 8/13 | 12 | 5 | 9 | 50 |
| Botzdn03com | Rogue | completionist | 12.0 | 1->3 | 8973 | 6/11 | 15 | 0 | 8 | 171 |
| Botzdn04soc | Priest | social | 12.0 | 1->3 | 11012 | 10/16 | 15 | 0 | 5 | 164 |
| Botzdn05cas | Mage | casual | 12.0 | 1->3 | 9915 | 8/14 | 13 | 0 | 7 | 89 |
| Botzdn06com | Warrior | completionist | 12.0 | 1->3 | 8613 | 6/9 | 13 | 0 | 9 | 140 |
| Botzdn07gri | Paladin | grinder | 12.0 | 1->3 | 7255 | 2/2 | 22 | 1 | 13 | 135 |
| Botzdn08par | Hunter | party | 12.0 | 1->3 | 9863 | 11/19 | 11 | 0 | 9 | 142 |
| Botzdn09par | Rogue | party | 12.0 | 1->3 | 7070 | 5/8 | 1 | 0 | 12 | 133 |
| Botzdn10com | Priest | completionist | 12.0 | 1->3 | 7657 | 6/11 | 11 | 0 | 8 | 113 |
| Botzdn11gri | Mage | grinder | 12.0 | 1->3 | 7167 | 2/2 | 18 | 1 | 9 | 107 |
| Botzdn12exp | Warrior | explorer | 12.0 | 1->3 | 9194 | 6/11 | 12 | 2 | 15 | 115 |
| Botzdn13com | Paladin | completionist | 12.0 | 1->3 | 8256 | 6/11 | 13 | 0 | 12 | 111 |
| Botzdn14soc | Hunter | social | 12.0 | 1->4 | 14428 | 12/18 | 17 | 0 | 7 | 71 |
| Botzdn15cas | Rogue | casual | 11.4 | 1->2 | 6272 | 3/6 | 9 | 2 | 8 | 202 |
| Botzdn16com | Priest | completionist | 12.0 | 1->3 | 13498 | 9/14 | 19 | 1 | 6 | 76 |
| Botzdn17gri | Mage | grinder | 12.0 | 1->3 | 9026 | 2/2 | 22 | 1 | 8 | 205 |
| Botzdn18par | Warrior | party | 12.0 | 1->3 | 10032 | 8/14 | 11 | 0 | 9 | 135 |
| Botzdn19par | Paladin | party | 12.0 | 1->3 | 8226 | 5/8 | 14 | 0 | 12 | 164 |
