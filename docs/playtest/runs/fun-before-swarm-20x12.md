# Hearthblock playtest: 20 bots, 12 min, brain mix

20 bots on `127.0.0.1:7710`, 233.5 bot-minutes (longest session 12.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 5 fail, 4 warn

| | Check | Value | Target |
|---|---|---|---|
| FAIL | Time to first kill (median) | 176.90 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.82 s | <= 180 s pass, > 300 s fail |
| WARN | Time to level 2 from level 1 (median) | 360.64 s | <= 300 s (design) pass, > 600 s fail |
| FAIL | XP per hour vs. the design pace (level 5 in 45 min) | 0.45 x design | >= 0.8 pass, < 0.5 fail |
| WARN | Share of XP from quests | 0.42  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 0.09 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 18.60 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.05  | <= 0.1 pass, > 0.25 fail |
| PASS | Deaths per hour of play | 0.77 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.97  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 4.16 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.01  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.89  | <= 0.4 pass, > 0.55 fail |
| WARN | Blocks walked per reward | 97.75 blocks | <= 60 pass, > 120 fail |
| WARN | Distinct abilities used per fighting bot | 1.70  | >= 2 pass, < 1.5 fail |
| FAIL | Share of casts that are the most used ability | 0.90  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.19  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 1.80 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 1668.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 177 s; first quest turned in: median 8 s; level 2: median 361 s (55% of level-1 starters reached it)
- XP per hour: median 2909 (design pace 6400); 42% of XP from quests; 37 quests turned in, median 0.1 min each
- Rewards: median gap 19 s; median longest drought 180 s; 5% of bots had a drought over 5 min
- Combat: 187 fights, 97% won, median 4.2 s; 136 kills, 31 without credit (0 of them with no other player within 20 blocks); 3 deaths (0.8/h)
- Travel: 98 blocks per reward; money earned 1668 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 1.8 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 0.6% |
| dead | 0.1% |
| fight | 6.4% |
| gather | 1.4% |
| idle | 0.8% |
| loot | 0.8% |
| town | 0.5% |
| travel | 89.5% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 20 | 1.0 | 0 |
| 1 | 20 | 1.0 | 56 |
| 2 | 20 | 1.0 | 103 |
| 3 | 20 | 1.0 | 137 |
| 4 | 20 | 1.0 | 171 |
| 5 | 20 | 1.0 | 216 |
| 6 | 19 | 1.0 | 254 |
| 7 | 19 | 1.0 | 293 |
| 8 | 19 | 1.0 | 332 |
| 9 | 19 | 2.0 | 370 |
| 10 | 19 | 2.0 | 466 |
| 11 | 19 | 2.0 | 542 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| coal_for_the_watch | 4 | 0 | - |
| cold_hearth | 20 | 20 | 0.1 |
| kindling | 16 | 5 | 5.0 |
| meat_for_the_pot | 16 | 5 | 7.5 |
| thin_the_herd | 20 | 2 | 8.4 |
| windmill_breadcrumb | 5 | 0 | - |
| wool_for_winter | 5 | 5 | 1.9 |

## Blockers (bots that could not progress an objective)

- kindling: collect Log: no source in reach x14
- coal_for_the_watch: collect Coal: no source in reach x4
- thin_the_herd: goto failed: too many repaths x4

## Deaths by cause

- You fell to a Shade. x2
- You fell to a Websnare. x1

## Fights by creature and outcome

- Grazer: won x157
- Shade: won x23
- Shade: lost (could not reach or hurt it) x2
- Shade: lost (died) x2
- Thornboar: won x1
- Websnare: lost (died) x1
- Websnare: won x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 33 | Quick Shot 82%, Barbed Arrow 15%, Recall 3% |
| Mage | 68 | Fireball 76%, Fire Blast 19%, Scholar's Focus 3%, Recall 1% |
| Paladin | 46 | Judging Blow 93%, Holy Mend 4%, Rebuke 2% |
| Priest | 58 | Smite 84%, Withering Word 14%, Mend 2% |
| Rogue | 30 | Vicious Strike 87%, Finishing Cut 13% |
| Warrior | 25 | Heroic Blow 88%, Battle Cry 8%, Recall 4% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-272, 144]: 3 events
  - Botkmw05cas at [-260, -4, 144] going to [-260, -7, 150] (repaths; thin_the_herd: go to the Grazer camp)
  - Botkmw16com at [-260, -4, 144] going to [-260, -7, 150] (repaths; thin_the_herd: go to the Grazer camp)
  - Botkmw18par at [-260, -4, 144] going to [-260, -7, 150] (repaths; thin_the_herd: go to the Grazer camp)
- cell [32, -480]: 1 events
  - Botkmw11gri at [44, 4, -475] going to [50, 6, -475] (repaths; thin_the_herd: go to the Grazer camp)
- cell [48, -496]: 1 events
  - Botkmw18par at [50, 4, -481] going to [50, 3, -475] (repaths; thin_the_herd: go to the Grazer camp)
- cell [144, -240]: 1 events
  - Botkmw09par at [147, -4, -236] going to [146, -4, -232] (repaths; following Botkmw08par)
- cell [240, 96]: 1 events
  - Botkmw05cas at [240, -6, 104] going to [240, -5, 110] (repaths; thin_the_herd: go to the Grazer camp)

## Notes

- grouped in a party of two x4
- tried /invite Botkmw15cas x2
- casual session ended x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 2 | 1647 | 3 | 0.0 | 100% | 28 | 1% |
| completionist | 6 | 1696 | 12 | 0.0 | 100% | 19 | 0% |
| explorer | 2 | 4224 | 6 | 2.5 | 93% | 40 | 0% |
| grinder | 4 | 4082 | 6 | 0.0 | 97% | 16 | 0% |
| party | 4 | 2464 | 7 | 2.5 | 94% | 20 | 3% |
| social | 2 | 1946 | 3 | 0.0 | 100% | 31 | 0% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 3 | 3367 | 6 | 1.7 | 96% | 35 | 0% |
| Mage | 3 | 3302 | 5 | 0.0 | 97% | 9 | 1% |
| Paladin | 4 | 3959 | 8 | 0.0 | 98% | 21 | 2% |
| Priest | 3 | 1721 | 4 | 0.0 | 100% | 53 | 0% |
| Rogue | 3 | 1703 | 5 | 4.1 | 90% | 24 | 1% |
| Warrior | 4 | 2859 | 9 | 0.0 | 100% | 15 | 1% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botkmw00com | Warrior | completionist | 12.0 | 1->2 | 5972 | 4/7 | 10 | 0 | 18 | 126 |
| Botkmw01gri | Paladin | grinder | 12.0 | 1->2 | 4862 | 2/2 | 13 | 0 | 15 | 228 |
| Botkmw02exp | Hunter | explorer | 12.0 | 1->2 | 4401 | 3/6 | 7 | 1 | 35 | 80 |
| Botkmw03com | Rogue | completionist | 12.0 | 1->1 | 1005 | 1/4 | 2 | 0 | 20 | 277 |
| Botkmw04soc | Priest | social | 12.0 | 1->2 | 2761 | 2/5 | 5 | 0 | 17 | 175 |
| Botkmw05cas | Mage | casual | 12.0 | 1->1 | 1591 | 2/4 | 1 | 0 | 9 | 238 |
| Botkmw06com | Warrior | completionist | 12.0 | 1->1 | 1671 | 1/4 | 5 | 0 | 12 | 298 |
| Botkmw07gri | Paladin | grinder | 12.0 | 1->2 | 3056 | 1/2 | 10 | 0 | 43 | 136 |
| Botkmw08par | Hunter | party | 12.0 | 1->2 | 3367 | 2/5 | 7 | 0 | 16 | 160 |
| Botkmw09par | Rogue | party | 12.0 | 1->2 | 5632 | 3/6 | 8 | 2 | 24 | 171 |
| Botkmw10com | Priest | completionist | 12.0 | 1->1 | 1051 | 1/4 | 3 | 0 | 63 | 213 |
| Botkmw11gri | Mage | grinder | 12.0 | 1->2 | 5767 | 2/2 | 17 | 0 | 9 | 96 |
| Botkmw12exp | Warrior | explorer | 12.0 | 1->2 | 4047 | 3/6 | 6 | 0 | 44 | 163 |
| Botkmw13com | Paladin | completionist | 12.0 | 1->3 | 7384 | 4/7 | 11 | 0 | 8 | 190 |
| Botkmw14soc | Hunter | social | 12.0 | 1->1 | 1131 | 1/4 | 3 | 0 | 44 | 206 |
| Botkmw15cas | Rogue | casual | 5.6 | 1->1 | 1703 | 1/4 | 2 | 0 | 47 | 169 |
| Botkmw16com | Priest | completionist | 12.0 | 1->1 | 1721 | 1/4 | 4 | 0 | 53 | 304 |
| Botkmw17gri | Mage | grinder | 12.0 | 1->2 | 3302 | 1/2 | 12 | 0 | 16 | 119 |
| Botkmw18par | Warrior | party | 12.0 | 1->1 | 1561 | 1/4 | 5 | 0 | 13 | 185 |
| Botkmw19par | Paladin | party | 12.0 | 1->1 | 1560 | 1/4 | 5 | 0 | 27 | 237 |
