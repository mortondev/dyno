# Hearthblock playtest: 12 bots, 30 min, brain mix

12 bots on `127.0.0.1:7710`, 339.3 bot-minutes (longest session 30.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 4 fail, 2 warn

| | Check | Value | Target |
|---|---|---|---|
| FAIL | Time to first kill (median) | 145.68 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.76 s | <= 180 s pass, > 300 s fail |
| WARN | Time to level 2 from level 1 (median) | 332.32 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 0.85 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.49  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.52 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 18.91 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 4.77 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.93  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 5.21 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.01  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.84  | <= 0.4 pass, > 0.55 fail |
| WARN | Blocks walked per reward | 77.99 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.75  | >= 2 pass, < 1.5 fail |
| PASS | Share of casts that are the most used ability | 0.68  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.16  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 1.77 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 7185.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 146 s; first quest turned in: median 8 s; level 2: median 332 s (100% of level-1 starters reached it)
- XP per hour: median 5452 (design pace 6400); 49% of XP from quests; 95 quests turned in, median 2.5 min each
- Rewards: median gap 19 s; median longest drought 207 s; 0% of bots had a drought over 5 min
- Combat: 378 fights, 93% won, median 5.2 s; 305 kills, 56 without credit (21 of them with no other player within 20 blocks); 27 deaths (4.8/h)
- Travel: 78 blocks per reward; money earned 7185 c, spent 0 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 1.8 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 0.3% |
| dead | 0.4% |
| fight | 11.5% |
| gather | 0.9% |
| idle | 0.8% |
| loot | 0.7% |
| rest | 0.5% |
| stuck | 0.1% |
| town | 1.2% |
| travel | 83.6% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 12 | 1.0 | 0 |
| 2 | 12 | 1.0 | 142 |
| 4 | 12 | 1.0 | 270 |
| 6 | 12 | 2.0 | 448 |
| 8 | 12 | 2.0 | 554 |
| 10 | 11 | 2.0 | 635 |
| 12 | 11 | 2.0 | 852 |
| 14 | 11 | 2.0 | 1080 |
| 16 | 11 | 3.0 | 1326 |
| 18 | 11 | 3.0 | 1495 |
| 20 | 11 | 3.0 | 1710 |
| 22 | 11 | 3.0 | 1891 |
| 24 | 11 | 3.0 | 2042 |
| 26 | 11 | 3.0 | 2168 |
| 28 | 11 | 3.0 | 2404 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 8 | 4 | 6.8 |
| coal_for_the_watch | 9 | 1 | 15.9 |
| cold_hearth | 12 | 12 | 0.1 |
| down_to_water | 3 | 2 | 1.6 |
| dust_of_shadows | 6 | 5 | 0.0 |
| eyes_in_the_dark | 8 | 6 | 9.7 |
| garricks_camp | 8 | 7 | 1.4 |
| into_barrowwood | 2 | 0 | - |
| iron_for_hollis | 1 | 0 | - |
| kindling | 9 | 9 | 2.7 |
| meat_for_the_pot | 9 | 8 | 7.7 |
| north_watch | 8 | 8 | 0.8 |
| pincher_supper | 2 | 0 | - |
| report_to_holt | 8 | 8 | 0.7 |
| shells_for_nets | 1 | 0 | - |
| silk_for_sails | 7 | 0 | - |
| stolen_grain | 3 | 0 | - |
| thieves_in_fields | 3 | 0 | - |
| thin_the_herd | 12 | 11 | 14.0 |
| tusks_and_tempers | 7 | 3 | 1.1 |
| watchfire | 8 | 0 | - |
| windmill_breadcrumb | 8 | 3 | 1.5 |
| wool_for_winter | 8 | 8 | 7.1 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x9
- watchfire: collect Coal: no source in reach x8
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x6
- eyes_in_the_dark: goto failed: too many repaths x3
- kindling: collect Log: no source in reach x2
- eyes_in_the_dark: goto failed: stuck x1
- into_barrowwood: goto failed: stuck x1
- iron_for_hollis: collect Raw iron: no source in reach x1
- thin_the_herd: goto failed: too many repaths x1

## Deaths by cause

- You fell to a Ashcloak Bandit. x10
- You fell to a Shade. x7
- You fell to a Ashcloak Cutthroat. x5
- You fell to a Websnare. x3
- You fell to a Mirepincher. x1
- You fell to a Rook Varn. x1

## Fights by creature and outcome

- Grazer: won x179
- Shade: won x102
- Thornboar: won x43
- Ashcloak Bandit: won x15
- Ashcloak Bandit: lost (died) x10
- Mirepincher: won x6
- Shade: lost (died) x6
- Ashcloak Cutthroat: lost (died) x4
- Websnare: won x4
- Websnare: lost (died) x3
- Rook Varn: lost (died) x2
- Ashcloak Cutthroat: won x1
- Mirepincher: lost (died) x1
- Shade: lost (could not reach or hurt it) x1
- Websnare: lost (cannot reach it: stuck) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 94 | Quick Shot 57%, Barbed Arrow 38%, Recall 4% |
| Mage | 108 | Fireball 72%, Fire Blast 25%, Scholar's Focus 2%, Recall 1% |
| Paladin | 138 | Judging Blow 64%, Holy Mend 20%, Rebuke 15% |
| Priest | 184 | Smite 70%, Withering Word 27%, Recall 2%, Mend 1% |
| Rogue | 95 | Vicious Strike 81%, Finishing Cut 17%, Recall 2% |
| Warrior | 155 | Heroic Blow 53%, Rending Cut 27%, Battle Cry 12%, Onrush 6%, Recall 2% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-496, -272]: 3 events
  - Botwkd00com at [-487, -6, -270] going to [48, -6, -10] (stuck; eyes_in_the_dark: look for Shade in the dark)
  - Botwkd00com at [-487, -6, -270] going to [-502, 1, -193] (stuck; silk_for_sails: kill Websnare for Websnare Silk)
  - Botwkd00com at [-487, -6, -270] going to [-633, -6, 50] (stuck; into_barrowwood: explore the Old Barrow)
- cell [-64, 0]: 1 events
  - Botwkd08par at [-49, 0, 2] going to [-49, 0, 6] (repaths; eyes_in_the_dark: look for Shade in the dark)
- cell [-48, -16]: 1 events
  - Botwkd00com at [-45, 0, -5] going to [-49, 0, -5] (repaths; eyes_in_the_dark: look for Shade in the dark)
- cell [-48, 0]: 1 events
  - Botwkd02exp at [-45, 0, 8] going to [-49, 0, 8] (repaths; eyes_in_the_dark: look for Shade in the dark)
- cell [-16, 336]: 1 events
  - Botwkd09par at [-3, -9, 339] going to [-4, -9, 335] (repaths; following Botwkd08par)
- cell [240, 96]: 1 events
  - Botwkd07gri at [240, -6, 104] going to [240, -5, 110] (repaths; thin_the_herd: go to the Grazer camp)
- cell [288, -144]: 1 events
  - Botwkd07gri at [290, -7, -144] going to [290, -6, -150] (repaths; going to grind)
- cell [304, 304]: 1 events
  - Botwkd11gri at [310, -8, 311] going to [310, -6, 305] (repaths; going to grind)

## Notes

- grouped in a party of two x2
- casual session ended x1
- tried /invite Botwkd03com x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 1 | 4150 | 2 | 0.0 | 100% | 18 | 2% |
| completionist | 4 | 6043 | 45 | 2.5 | 95% | 16 | 1% |
| explorer | 1 | 4437 | 8 | 4.0 | 90% | 18 | 1% |
| grinder | 3 | 4144 | 6 | 13.3 | 85% | 32 | 1% |
| party | 2 | 5821 | 23 | 0.0 | 100% | 19 | 2% |
| social | 1 | 6262 | 11 | 0.0 | 100% | 22 | 1% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 2 | 4943 | 18 | 2.0 | 96% | 18 | 1% |
| Mage | 2 | 4169 | 4 | 4.6 | 95% | 25 | 1% |
| Paladin | 2 | 4088 | 4 | 17.0 | 82% | 29 | 2% |
| Priest | 2 | 5858 | 21 | 0.0 | 100% | 19 | 1% |
| Rogue | 2 | 6043 | 23 | 0.0 | 98% | 29 | 2% |
| Warrior | 2 | 7266 | 25 | 5.0 | 92% | 15 | 1% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botwkd00com | Warrior | completionist | 30.0 | 1->4 | 6192 | 11/19 | 26 | 3 | 16 | 240 |
| Botwkd01gri | Paladin | grinder | 30.0 | 1->3 | 4032 | 2/2 | 32 | 7 | 40 | 241 |
| Botwkd02exp | Hunter | explorer | 30.0 | 1->3 | 4437 | 8/14 | 18 | 2 | 18 | 131 |
| Botwkd03com | Rogue | completionist | 30.0 | 1->4 | 5894 | 10/16 | 24 | 0 | 39 | 123 |
| Botwkd04soc | Priest | social | 30.0 | 1->4 | 6262 | 11/16 | 23 | 0 | 22 | 208 |
| Botwkd05cas | Mage | casual | 9.4 | 1->2 | 4150 | 2/5 | 7 | 0 | 18 | 97 |
| Botwkd06com | Warrior | completionist | 30.0 | 1->4 | 8340 | 14/22 | 37 | 2 | 13 | 133 |
| Botwkd07gri | Paladin | grinder | 30.0 | 1->3 | 4144 | 2/2 | 37 | 10 | 19 | 207 |
| Botwkd08par | Hunter | party | 30.0 | 1->4 | 5450 | 10/16 | 26 | 0 | 19 | 154 |
| Botwkd09par | Rogue | party | 30.0 | 1->4 | 6192 | 13/20 | 22 | 0 | 19 | 231 |
| Botwkd10com | Priest | completionist | 30.0 | 1->4 | 5454 | 10/16 | 21 | 0 | 16 | 212 |
| Botwkd11gri | Mage | grinder | 30.0 | 1->3 | 4188 | 2/2 | 32 | 3 | 32 | 222 |
