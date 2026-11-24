# Hearthblock playtest: 12 bots, 30 min, brain mix

12 bots on `local`, 344.0 bot-minutes (longest session 30.0 min). Thresholds: docs/playtest/fun-metrics.md.

Machine: load average 18.4 at the start, 31.0 mean, 74.0 at most, on 16 cores. Server tick: mean 1.60 ms, p95 5.50 ms, p99 27.00 ms, worst 389 ms over 33347 ticks; the worst minute's p95 was 27.75 ms (budget 50 ms).

## Fun checks: 2 fail, 3 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 65.57 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 6.69 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 146.78 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.00 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.59  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.66 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 9.04 s | <= 60 pass, > 90 fail |
| FAIL | Share of bots with a reward drought over 5 minutes | 0.50  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 4.01 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.96  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 6.41 s | <= 25 pass, > 40 fail (and >= 5) |
| WARN | Share of time idle, stuck, waiting for spawns or dead | 0.20  | <= 0.15 pass, > 0.3 fail |
| WARN | Share of time travelling | 0.49  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 35.55 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 4.00  | >= 2 pass, < 1.5 fail |
| PASS | Share of casts that are the most used ability | 0.57  | <= 0.7 pass, > 0.85 fail |
| PASS | Kills lost to someone else's last hit | 0.04  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 1.57 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 11339.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 66 s; first quest turned in: median 7 s; level 2: median 147 s (100% of level-1 starters reached it)
- XP per hour: median 6421 (design pace 6400); 59% of XP from quests; 128 quests turned in, median 2.7 min each
- Rewards: median gap 9 s; median longest drought 428 s; 50% of bots had a drought over 5 min
- Combat: 613 fights, 96% won, median 6.4 s; 526 kills, 22 without credit (11 of them with no other player within 20 blocks); 23 deaths (4.0/h)
- Travel: 36 blocks per reward; money earned 11339 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 1.6 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 0.4% |
| dead | 0.3% |
| fight | 24.8% |
| gather | 1.6% |
| idle | 4.9% |
| loot | 1.5% |
| rest | 1.6% |
| town | 1.1% |
| travel | 49.4% |
| wait_spawn | 14.3% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 12 | 1.0 | 0 |
| 2 | 12 | 1.0 | 302 |
| 4 | 12 | 2.0 | 609 |
| 6 | 12 | 2.0 | 936 |
| 8 | 12 | 2.0 | 1196 |
| 10 | 12 | 3.0 | 1486 |
| 12 | 12 | 3.0 | 1922 |
| 14 | 12 | 3.0 | 2202 |
| 16 | 11 | 3.0 | 2461 |
| 18 | 11 | 4.0 | 2716 |
| 20 | 11 | 4.0 | 2912 |
| 22 | 11 | 4.0 | 3068 |
| 24 | 11 | 4.0 | 3267 |
| 26 | 11 | 4.0 | 3325 |
| 28 | 11 | 4.0 | 3351 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 8 | 7 | 4.7 |
| broken_tower | 9 | 8 | 6.5 |
| coal_for_the_watch | 9 | 8 | 5.5 |
| cold_hearth | 12 | 12 | 0.1 |
| down_to_water | 1 | 1 | 3.2 |
| drowned_light | 1 | 0 | - |
| dust_of_shadows | 7 | 1 | 0.0 |
| eyes_in_the_dark | 8 | 7 | 2.8 |
| garricks_camp | 8 | 3 | 8.9 |
| into_barrowwood | 1 | 0 | - |
| iron_for_hollis | 7 | 2 | 1.7 |
| kindling | 9 | 9 | 0.5 |
| know_the_vale | 5 | 0 | - |
| lay_of_the_land | 8 | 8 | 2.1 |
| lookouts_eye | 8 | 7 | 5.6 |
| meat_for_the_pot | 9 | 9 | 2.3 |
| north_watch | 8 | 8 | 5.8 |
| old_tusk | 7 | 2 | 2.9 |
| pincher_supper | 1 | 1 | 2.3 |
| shells_for_nets | 1 | 1 | 2.3 |
| silk_for_sails | 3 | 1 | 5.3 |
| stolen_grain | 1 | 1 | 2.2 |
| thieves_in_fields | 1 | 1 | 1.3 |
| thin_the_herd | 12 | 11 | 3.4 |
| tusks_and_tempers | 3 | 1 | 0.0 |
| watchfire | 8 | 8 | 3.7 |
| webs_between_roots | 1 | 1 | 2.6 |
| windmill_breadcrumb | 8 | 1 | 9.9 |
| wool_for_winter | 9 | 8 | 2.1 |
| word_for_rysa | 3 | 1 | 1.1 |
| word_from_east | 2 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x8
- dust_of_shadows: collect Shade dust: no source in reach x6
- dust_of_shadows: kill Shade: only at night: waiting for dark x6
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x5
- watchfire: collect Coal: no source in reach x5
- iron_for_hollis: collect Raw iron: no source in reach x4
- boar_trouble: goto failed: too many repaths x1
- iron_for_hollis: collect Raw iron: Requires mining 5 (have 4) x1
- iron_for_hollis: gather Raw iron: needs mining 5: raising it on easier nodes x1

## Deaths by cause

- You fell to a Grazer. x14
- You fell to a Old Tusk. x4
- You fell to a Thornboar. x3
- You fell to a Ashcloak Bandit. x1
- You fell to a Websnare. x1

## Fights by creature and outcome

- Grazer: won x457
- Thornboar: won x55
- Shade: won x50
- Grazer: lost (died) x14
- Websnare: won x10
- Ashcloak Thief: won x7
- Mirepincher: won x7
- Old Tusk: lost (died) x4
- Thornboar: lost (died) x3
- Old Tusk: won x2
- Ashcloak Bandit: lost (died) x1
- Gloomwisp: won x1
- Grazer: lost (it fled home and healed) x1
- Websnare: lost (died) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 89 | Quick Shot 45%, Barbed Arrow 39%, Marked Shot 7%, Hamstring 6%, Recall 3% |
| Mage | 263 | Frostbolt 54%, Fireball 26%, Fire Blast 20%, Recall 0% |
| Paladin | 292 | Judging Blow 69%, Seal of Dawn 13%, Rebuke 12%, Valour 3%, Holy Mend 2% |
| Priest | 199 | Smite 69%, Withering Word 24%, Ward of Light 4%, Mend 2%, Recall 2% |
| Rogue | 61 | Vicious Strike 52%, Finishing Cut 25%, Ambush 15%, Low Blow 7%, Recall 2% |
| Warrior | 185 | Heroic Blow 43%, Rending Cut 21%, Onrush 21%, Ground Slam 9%, Second Wind 4%, Recall 2% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-160, 32]: 1 events
  - Botwja07gri at [-150, -8, 34] going to [-150, -8, 40] (repaths; going to grind)
- cell [-144, -160]: 1 events
  - Botwja02exp at [-138, -13, -150] going to [-150, -10, -150] (repaths; exploring The Old Quarry)
- cell [-144, 288]: 1 events
  - Botwja02exp at [-130, -18, 288] going to [-130, -10, 300] (repaths; exploring Southfold Farm)
- cell [-64, -224]: 1 events
  - Botwja04soc at [-60, -13, -219] going to [-60, -8, -225] (repaths; boar_trouble: go to the Thornboar camp)
- cell [-64, 0]: 1 events
  - Botwja09par at [-52, 0, 10] going to [-56, 0, 10] (repaths; following Botwja08par)
- cell [-32, -192]: 1 events
  - Botwja09par at [-22, -13, -184] going to [-20, -14, -187] (repaths; following Botwja08par)
- cell [-16, 48]: 1 events
  - Botwja09par at [-8, 0, 58] going to [-8, 0, 54] (repaths; following Botwja08par)
- cell [32, -272]: 1 events
  - Botwja09par at [34, -12, -269] going to [37, -12, -272] (repaths; following Botwja08par)
- cell [304, 304]: 1 events
  - Botwja02exp at [310, -21, 312] going to [310, -9, 300] (repaths; exploring The Ashcloak Stockade)

## Notes

- grouped in a party of two x2
- casual session ended x1
- tried /invite Botwja03com x1

## Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)

- Grazer lv2 at 158,-8,-266 (ground -10) x6
- Grazer lv1 at 121,-8,9 (ground -10) x2
- Grazer lv1 at 122,-8,13 (ground -10) x2
- Grazer lv1 at 231,-9,122 (ground -11) x2
- Grazer lv2 at -76,-5,-70 (ground -7) x2
- Shade lv2 at 100,-14,-280 (ground -16) x2
- Thornboar lv1 at -174,-8,58 (ground -10) x2
- Grazer lv1 at -108,-7,-93 (ground -9) x1
- Grazer lv1 at -87,-6,-95 (ground -8) x1
- Grazer lv1 at 144,-9,10 (ground -11) x1
- Grazer lv1 at 220,-9,75 (ground -12) x1
- Grazer lv1 at 77,-4,-70 (ground -6) x1
- Grazer lv2 at -247,-16,-248 (ground -18) x1
- Grazer lv2 at 71,-6,-87 (ground -8) x1
- Grazer lv3 at 13,-22,410 (ground -24) x1
- Thornboar lv3 at -279,-8,-555 (ground -10) x1
- Thornboar lv3 at -84,0,-400 (ground -2) x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 1 | 10377 | 12 | 0.0 | 100% | 7 | 3% |
| completionist | 4 | 8228 | 66 | 2.0 | 97% | 11 | 22% |
| explorer | 1 | 6088 | 12 | 4.0 | 87% | 13 | 1% |
| grinder | 3 | 6507 | 6 | 10.7 | 95% | 7 | 2% |
| party | 2 | 3894 | 17 | 1.0 | 98% | 10 | 46% |
| social | 1 | 6794 | 15 | 0.0 | 100% | 8 | 37% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 2 | 5701 | 25 | 2.0 | 95% | 11 | 23% |
| Mage | 2 | 8442 | 14 | 12.3 | 93% | 7 | 3% |
| Paladin | 2 | 6926 | 4 | 7.0 | 97% | 7 | 1% |
| Priest | 2 | 8603 | 34 | 0.0 | 100% | 9 | 20% |
| Rogue | 2 | 4158 | 18 | 2.0 | 95% | 12 | 44% |
| Warrior | 2 | 8602 | 33 | 3.0 | 95% | 12 | 23% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botwja00com | Warrior | completionist | 30.0 | 1->4 | 6045 | 13/19 | 19 | 2 | 13 | 750 |
| Botwja01gri | Paladin | grinder | 30.0 | 1->4 | 6335 | 2/2 | 104 | 2 | 6 | 596 |
| Botwja02exp | Hunter | explorer | 30.0 | 1->4 | 6088 | 12/19 | 13 | 2 | 13 | 124 |
| Botwja03com | Rogue | completionist | 30.0 | 1->4 | 5842 | 14/20 | 18 | 1 | 12 | 764 |
| Botwja04soc | Priest | social | 30.0 | 1->4 | 6794 | 15/22 | 23 | 0 | 8 | 681 |
| Botwja05cas | Mage | casual | 14.0 | 1->3 | 10377 | 12/17 | 14 | 0 | 7 | 61 |
| Botwja06com | Warrior | completionist | 30.0 | 1->5 | 11159 | 20/26 | 40 | 1 | 11 | 219 |
| Botwja07gri | Paladin | grinder | 30.0 | 1->4 | 7516 | 2/2 | 125 | 5 | 8 | 178 |
| Botwja08par | Hunter | party | 30.0 | 1->3 | 5314 | 13/18 | 22 | 0 | 8 | 813 |
| Botwja09par | Rogue | party | 30.0 | 1->2 | 2474 | 4/7 | 7 | 1 | 12 | 827 |
| Botwja10com | Priest | completionist | 30.0 | 1->5 | 10411 | 19/24 | 34 | 0 | 10 | 177 |
| Botwja11gri | Mage | grinder | 30.0 | 1->4 | 6507 | 2/2 | 107 | 9 | 7 | 259 |
