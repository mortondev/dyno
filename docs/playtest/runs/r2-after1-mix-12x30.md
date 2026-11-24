# Hearthblock playtest: 12 bots, 30 min, brain mix

12 bots on `local`, 338.8 bot-minutes (longest session 30.0 min). Thresholds: docs/playtest/fun-metrics.md.

Machine: load average 11.3 at the start, 7.5 mean, 16.8 at most, on 16 cores. Server tick: mean 0.42 ms, p95 1.00 ms, p99 2.00 ms, worst 127 ms over 36048 ticks; the worst minute's p95 was 1.75 ms (budget 50 ms).

## Fun checks: 2 fail, 4 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 71.88 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.12 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 129.55 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.17 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.58  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.95 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 10.46 s | <= 60 pass, > 90 fail |
| WARN | Share of bots with a reward drought over 5 minutes | 0.17  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 8.68 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.93  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 6.57 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.05  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.57  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 38.62 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 3.00  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.70  | <= 0.7 pass, > 0.85 fail |
| WARN | Kills lost to someone else's last hit | 0.07  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 0.71 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 304.04 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 72 s; first quest turned in: median 7 s; level 2: median 130 s (100% of level-1 starters reached it)
- XP per hour: median 7481 (design pace 6400); 58% of XP from quests; 134 quests turned in, median 3.0 min each
- Rewards: median gap 10 s; median longest drought 193 s; 17% of bots had a drought over 5 min
- Combat: 734 fights, 93% won, median 6.6 s; 611 kills, 49 without credit (30 of them with no other player within 20 blocks); 49 deaths (8.7/h)
- Travel: 39 blocks per reward; money earned 13682 c, spent 45 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 0.7 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 0.8% |
| dead | 0.7% |
| fight | 30.5% |
| gather | 1.5% |
| idle | 3.3% |
| loot | 2.0% |
| rest | 1.6% |
| town | 1.9% |
| travel | 56.9% |
| wait_spawn | 0.6% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 12 | 1.0 | 0 |
| 2 | 12 | 1.0 | 328 |
| 4 | 12 | 2.0 | 651 |
| 6 | 12 | 2.0 | 951 |
| 8 | 12 | 2.5 | 1210 |
| 10 | 11 | 3.0 | 1522 |
| 12 | 11 | 3.0 | 1959 |
| 14 | 11 | 3.0 | 2306 |
| 16 | 11 | 3.0 | 2524 |
| 18 | 11 | 4.0 | 2703 |
| 20 | 11 | 4.0 | 2966 |
| 22 | 11 | 4.0 | 3178 |
| 24 | 11 | 4.0 | 3338 |
| 26 | 11 | 4.0 | 3527 |
| 28 | 11 | 4.0 | 3644 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 8 | 7 | 3.1 |
| broken_tower | 9 | 8 | 6.9 |
| coal_for_the_watch | 9 | 6 | 5.0 |
| cold_hearth | 12 | 12 | 0.1 |
| down_to_water | 6 | 2 | 2.4 |
| drowned_light | 1 | 1 | 1.3 |
| dust_of_shadows | 5 | 1 | 0.0 |
| eyes_in_the_dark | 8 | 5 | 4.6 |
| garricks_camp | 8 | 4 | 9.5 |
| into_barrowwood | 2 | 0 | - |
| iron_for_hollis | 5 | 3 | 15.9 |
| kindling | 9 | 9 | 0.6 |
| know_the_vale | 5 | 0 | - |
| lay_of_the_land | 8 | 6 | 1.8 |
| lookouts_eye | 8 | 6 | 2.7 |
| meat_for_the_pot | 9 | 9 | 2.3 |
| north_watch | 9 | 8 | 5.7 |
| old_tusk | 7 | 1 | 17.1 |
| pincher_supper | 2 | 1 | 3.0 |
| shells_for_nets | 1 | 1 | 3.0 |
| silk_for_sails | 4 | 0 | - |
| stolen_grain | 6 | 1 | 3.0 |
| thieves_in_fields | 6 | 4 | 2.1 |
| thin_the_herd | 12 | 12 | 3.6 |
| tusks_and_tempers | 4 | 3 | 6.4 |
| watchfire | 8 | 8 | 2.2 |
| webs_between_roots | 1 | 0 | - |
| windmill_breadcrumb | 8 | 6 | 9.4 |
| wool_for_winter | 9 | 9 | 2.4 |
| word_for_rysa | 4 | 1 | 1.0 |
| word_from_east | 3 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x8
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x7
- watchfire: collect Coal: no source in reach x5
- iron_for_hollis: collect Raw iron: no source in reach x4
- dust_of_shadows: collect Shade dust: no source in reach x3
- dust_of_shadows: kill Shade: only at night: waiting for dark x3

## Deaths by cause

- You fell to a Old Tusk. x21
- You fell to a Grazer. x14
- You fell to a Thornboar. x7
- You fell to a Ashcloak Bandit. x6
- You fell to a Shade. x1

## Fights by creature and outcome

- Grazer: won x487
- Thornboar: won x102
- Ashcloak Bandit: won x43
- Shade: won x35
- Old Tusk: lost (died) x21
- Grazer: lost (died) x14
- Mirepincher: won x7
- Thornboar: lost (died) x7
- Ashcloak Bandit: lost (died) x6
- Gloomwisp: won x3
- Grazer: lost (it fled home and healed) x2
- Ashcloak Bandit: lost (it fled home and healed) x1
- Barrow Warden: lost (it fled home and healed) x1
- Mirepincher: lost (it fled home and healed) x1
- Old Tusk: won x1
- Shade: lost (died) x1
- Thornboar: lost (it fled home and healed) x1
- Websnare: won x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 89 | Quick Shot 65%, Barbed Arrow 33%, Recall 2% |
| Mage | 248 | Fireball 58%, Fire Blast 41%, Scholar's Focus 1%, Recall 0% |
| Paladin | 188 | Judging Blow 80%, Rebuke 12%, Holy Mend 5%, Valour 3% |
| Priest | 299 | Smite 72%, Withering Word 20%, Mend 6%, Recall 1% |
| Rogue | 116 | Vicious Strike 78%, Finishing Cut 22%, Recall 1% |
| Warrior | 191 | Heroic Blow 56%, Rending Cut 24%, Battle Cry 10%, Onrush 7%, Recall 2% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-144, -160]: 1 events
  - Botwpk02exp at [-138, -13, -150] going to [-150, -10, -150] (repaths; exploring The Old Quarry)
- cell [-112, 288]: 1 events
  - Botwpk07gri at [-106, -18, 300] going to [-100, -21, 300] (repaths; going to grind)
- cell [-16, -96]: 1 events
  - Botwpk09par at [-11, 0, -82] going to [-8, 0, -79] (repaths; following Botwpk08par)
- cell [64, -224]: 1 events
  - Botwpk01gri at [76, -15, -210] going to [70, -14, -210] (repaths; going to grind)

## Notes

- grouped in a party of two x2
- casual session ended x1
- tried /invite Botwpk03com x1

## Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)

- Grazer lv1 at -98,-7,-76 (ground -9) x4
- Grazer lv1 at -76,-5,-70 (ground -7) x2
- Grazer lv1 at 146,-11,-246 (ground -13) x2
- Grazer lv2 at -97,-6,-81 (ground -8) x2
- Grazer lv2 at 71,-6,-87 (ground -8) x2
- Grazer lv1 at -87,-21,300 (ground -23) x1
- Grazer lv1 at -94,-20,313 (ground -22) x1
- Grazer lv1 at -96,-8,-68 (ground -10) x1
- Grazer lv1 at 114,-8,12 (ground -10) x1
- Grazer lv1 at 126,-8,19 (ground -10) x1
- Grazer lv1 at 134,-8,-1 (ground -10) x1
- Grazer lv1 at 140,-9,19 (ground -11) x1
- Grazer lv1 at 144,-11,-252 (ground -13) x1
- Grazer lv1 at 171,-6,-267 (ground -8) x1
- Grazer lv1 at 239,-8,115 (ground -10) x1
- Grazer lv1 at 85,-8,-155 (ground -10) x1
- Grazer lv1 at 92,-8,78 (ground -10) x1
- Grazer lv2 at -100,-20,282 (ground -22) x1
- Grazer lv2 at -110,-16,300 (ground -19) x1
- Grazer lv2 at -144,-10,-11 (ground -12) x1
- Grazer lv2 at -80,-13,178 (ground -15) x1
- Grazer lv2 at -90,-7,-71 (ground -9) x1
- Grazer lv2 at -98,-21,287 (ground -23) x1
- Grazer lv2 at -98,-6,-85 (ground -8) x1
- Grazer lv2 at -98,-7,-76 (ground -9) x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 1 | 7143 | 6 | 0.0 | 100% | 6 | 2% |
| completionist | 4 | 8966 | 70 | 11.5 | 85% | 11 | 3% |
| explorer | 1 | 6835 | 12 | 2.0 | 95% | 17 | 2% |
| grinder | 3 | 7549 | 6 | 12.7 | 95% | 8 | 2% |
| party | 2 | 4864 | 24 | 2.0 | 95% | 14 | 15% |
| social | 1 | 8811 | 16 | 8.0 | 89% | 8 | 3% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 2 | 6037 | 24 | 2.0 | 96% | 15 | 3% |
| Mage | 2 | 7346 | 8 | 13.9 | 94% | 6 | 2% |
| Paladin | 2 | 7235 | 4 | 10.0 | 96% | 8 | 2% |
| Priest | 2 | 10586 | 37 | 5.0 | 91% | 9 | 3% |
| Rogue | 2 | 6839 | 30 | 8.0 | 87% | 14 | 14% |
| Warrior | 2 | 8079 | 31 | 15.0 | 80% | 11 | 4% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botwpk00com | Warrior | completionist | 30.0 | 1->4 | 8743 | 16/23 | 29 | 5 | 10 | 269 |
| Botwpk01gri | Paladin | grinder | 30.0 | 1->4 | 7817 | 2/2 | 143 | 7 | 8 | 92 |
| Botwpk02exp | Hunter | explorer | 30.0 | 1->4 | 6835 | 12/20 | 20 | 1 | 17 | 159 |
| Botwpk03com | Rogue | completionist | 30.0 | 1->4 | 9189 | 18/26 | 35 | 7 | 14 | 182 |
| Botwpk04soc | Priest | social | 30.0 | 1->4 | 8811 | 16/23 | 29 | 4 | 8 | 164 |
| Botwpk05cas | Mage | casual | 8.8 | 1->2 | 7143 | 6/8 | 12 | 0 | 6 | 70 |
| Botwpk06com | Warrior | completionist | 30.0 | 1->4 | 7414 | 15/22 | 27 | 10 | 11 | 259 |
| Botwpk07gri | Paladin | grinder | 30.0 | 1->4 | 6654 | 2/2 | 116 | 3 | 8 | 319 |
| Botwpk08par | Hunter | party | 30.0 | 1->3 | 5238 | 12/19 | 21 | 1 | 13 | 224 |
| Botwpk09par | Rogue | party | 30.0 | 1->3 | 4490 | 12/23 | 12 | 1 | 14 | 444 |
| Botwpk10com | Priest | completionist | 30.0 | 1->5 | 12361 | 21/26 | 37 | 1 | 11 | 198 |
| Botwpk11gri | Mage | grinder | 30.0 | 1->4 | 7549 | 2/2 | 130 | 9 | 6 | 189 |
