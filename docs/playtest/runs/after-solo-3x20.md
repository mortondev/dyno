# Hearthblock playtest: 3 bots, 20 min, brain completionist

3 bots on `local`, 60.0 bot-minutes (longest session 20.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 3 fail, 2 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 50.88 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 8.41 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 62.72 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 2.45 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.58  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 1.45 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 8.90 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 30.00 /h | <= 1.5 pass, > 4 fail |
| FAIL | Fights won | 0.66  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 8.47 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.04  | <= 0.15 pass, > 0.3 fail |
| PASS | Share of time travelling | 0.27  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 20.44 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.67  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.76  | <= 0.7 pass, > 0.85 fail |
| WARN | Kills lost to someone else's last hit | 0.08  | <= 0.05 pass, > 0.15 fail |
| FAIL | Stuck events per bot-hour | 12.00 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 4787.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 51 s; first quest turned in: median 8 s; level 2: median 63 s (100% of level-1 starters reached it)
- XP per hour: median 15684 (design pace 6400); 58% of XP from quests; 49 quests turned in, median 1.4 min each
- Rewards: median gap 9 s; median longest drought 88 s; 0% of bots had a drought over 5 min
- Combat: 178 fights, 66% won, median 8.5 s; 110 kills, 10 without credit (3 of them with no other player within 20 blocks); 30 deaths (30.0/h)
- Travel: 20 blocks per reward; money earned 4787 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 12.0 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| dead | 2.6% |
| fight | 55.8% |
| gather | 5.4% |
| idle | 0.4% |
| loot | 1.5% |
| rest | 5.3% |
| stuck | 1.0% |
| town | 0.9% |
| travel | 26.8% |
| wait_spawn | 0.3% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 3 | 1.0 | 0 |
| 1 | 3 | 1.0 | 284 |
| 2 | 3 | 2.0 | 570 |
| 3 | 3 | 2.0 | 1011 |
| 4 | 3 | 3.0 | 1592 |
| 5 | 3 | 3.0 | 2016 |
| 6 | 3 | 3.0 | 2234 |
| 7 | 3 | 3.0 | 2428 |
| 8 | 3 | 3.0 | 2505 |
| 9 | 3 | 4.0 | 2926 |
| 10 | 3 | 4.0 | 3137 |
| 11 | 3 | 4.0 | 3244 |
| 12 | 3 | 4.0 | 3532 |
| 13 | 3 | 4.0 | 4005 |
| 14 | 3 | 4.0 | 4196 |
| 15 | 3 | 4.0 | 4330 |
| 16 | 3 | 4.0 | 4465 |
| 17 | 3 | 4.0 | 4638 |
| 18 | 3 | 5.0 | 4901 |
| 19 | 3 | 5.0 | 4931 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| ambush_east_road | 1 | 0 | - |
| boar_trouble | 3 | 3 | 4.5 |
| burn_stores | 1 | 1 | 1.4 |
| candles_in_dark | 1 | 0 | - |
| coal_for_the_watch | 3 | 3 | 1.0 |
| cold_hearth | 3 | 3 | 0.1 |
| down_to_water | 3 | 2 | 9.3 |
| dust_of_shadows | 2 | 1 | 0.2 |
| eyes_in_the_dark | 3 | 2 | 9.5 |
| garricks_camp | 3 | 3 | 0.5 |
| into_barrowwood | 1 | 0 | - |
| iron_for_hollis | 3 | 1 | 5.2 |
| kindling | 3 | 3 | 1.8 |
| meat_for_the_pot | 3 | 3 | 1.5 |
| north_watch | 3 | 3 | 0.3 |
| pincher_supper | 2 | 0 | - |
| report_to_holt | 3 | 3 | 0.1 |
| rook_varn | 1 | 0 | - |
| shells_for_nets | 2 | 0 | - |
| silk_for_sails | 3 | 0 | - |
| stolen_grain | 3 | 1 | 6.1 |
| thieves_in_fields | 3 | 3 | 7.0 |
| thin_the_herd | 3 | 3 | 3.2 |
| tusks_and_tempers | 3 | 1 | 9.7 |
| watchfire | 3 | 3 | 0.1 |
| windmill_breadcrumb | 3 | 3 | 2.4 |
| wool_for_winter | 3 | 3 | 1.1 |
| word_from_east | 1 | 1 | 6.6 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x3
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x3
- dust_of_shadows: collect Shade dust: no source in reach x1
- dust_of_shadows: kill Shade: only at night: waiting for dark x1
- into_barrowwood: goto failed: stuck x1
- rook_varn: goto failed: stuck x1

## Deaths by cause

- You fell to a Ashcloak Bandit. x13
- You fell to a Ashcloak Cutthroat. x4
- You fell to a Thornboar. x4
- You fell to a Websnare. x3
- You fell to a Gloomwisp. x2
- You fell to a Shade. x2
- You fell to a Mirepincher. x1
- You fell to a Rook Varn. x1

## Fights by creature and outcome

- Grazer: won x37
- Ashcloak Bandit: won x28
- Thornboar: won x25
- Shade: won x16
- Ashcloak Bandit: lost (died) x14
- Ashcloak Bandit: lost (it fled home and healed) x6
- Websnare: won x5
- Ashcloak Bandit: lost (could not reach or hurt it) x4
- Mirepincher: won x4
- Shade: lost (died) x4
- Thornboar: lost (died) x4
- Candlekin: won x3
- Thornboar: lost (it fled home and healed) x3
- Gloomwisp: lost (could not reach or hurt it) x2
- Gloomwisp: lost (died) x2
- Thornboar: lost (could not reach or hurt it) x2
- Websnare: lost (cannot reach it: stuck) x2
- Websnare: lost (died) x2
- Websnare: lost (it fled home and healed) x2
- Ashcloak Bandit: lost (cannot reach it: stuck) x1
- Ashcloak Cutthroat: lost (died) x1
- Ashcloak Cutthroat: lost (it fled home and healed) x1
- Candlekin: lost (cannot reach it: stuck) x1
- Candlekin: lost (could not reach or hurt it) x1
- Gloomwisp: lost (cannot reach it: stuck) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 54 | Quick Shot 81%, Barbed Arrow 19% |
| Paladin | 82 | Judging Blow 71%, Holy Mend 27%, Valour 2% |
| Warrior | 111 | Heroic Blow 75%, Onrush 18%, Battle Cry 7% |

## Unreachable NPCs

- Captain Maren (cannot reach NPC: stuck) x1

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-32, -80]: 4 events
  - Botrsd00com at [-27, -4, -68] going to [2, 0, -12] (stuck; visiting Captain Maren: hand in eyes_in_the_dark)
  - Botrsd00com at [-27, -4, -68] going to [-83, -12, -29] (stuck; silk_for_sails: kill Websnare for Websnare Silk)
  - Botrsd00com at [-27, -4, -68] going to [-116, -4, 10] (stuck; into_barrowwood: explore the Old Barrow)
  - Botrsd00com at [-27, -4, -68] going to [0, 0, 0] (gave up: /spawn; into_barrowwood: explore the Old Barrow)
- cell [144, -48]: 4 events
  - Botrsd00com at [158, -12, -47] going to [167, -14, -51] (stuck; candles_in_dark: kill Candlekin)
  - Botrsd00com at [158, -12, -47] going to [85, -6, -14] (stuck; ambush_east_road: kill Ashcloak Bandit)
  - Botrsd00com at [158, -12, -47] going to [62, -10, 61] (stuck; rook_varn: go to the Rook Varn camp)
  - Botrsd00com at [158, -12, -47] going to [157, -14, -58] (stuck; defending against Gloomwisp)
- cell [-16, -80]: 3 events
  - Botrsd00com at [-1, -6, -79] going to [-38, -4, -58] (stuck; eyes_in_the_dark: kill Shade)
  - Botrsd00com at [-1, -6, -79] going to [-32, -5, -115] (stuck; tusks_and_tempers: kill Thornboar for Boar Tusk)
  - Botrsd00com at [-1, -6, -79] going to [-78, -10, -51] (stuck; silk_for_sails: kill Websnare for Websnare Silk)
- cell [-16, 128]: 1 events
  - Botrsd02com at [-14, -13, 134] going to [-14, -16, 134] (repaths; iron_for_hollis: gather Raw iron)

## Notes

- used /spawn to get unstuck x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| completionist | 3 | 15684 | 49 | 30.0 | 66% | 9 | 4% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 1 | 11574 | 15 | 30.0 | 62% | 6 | 3% |
| Paladin | 1 | 15684 | 16 | 30.0 | 73% | 10 | 3% |
| Warrior | 1 | 18049 | 18 | 30.0 | 63% | 9 | 7% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botrsd00com | Warrior | completionist | 20.0 | 1->5 | 18049 | 18/25 | 43 | 10 | 9 | 88 |
| Botrsd01com | Paladin | completionist | 20.0 | 1->5 | 15684 | 16/22 | 40 | 10 | 10 | 65 |
| Botrsd02com | Hunter | completionist | 20.0 | 1->4 | 11574 | 15/22 | 27 | 10 | 6 | 240 |
