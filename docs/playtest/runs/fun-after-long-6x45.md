# Hearthblock playtest: 6 bots, 45 min, brain completionist

6 bots on `127.0.0.1:7711`, 270.0 bot-minutes (longest session 45.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 3 fail, 1 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 42.01 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.70 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 150.62 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.24 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.76  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 4.35 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 10.57 s | <= 60 pass, > 90 fail |
| WARN | Share of bots with a reward drought over 5 minutes | 0.17  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 4.22 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.93  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 6.42 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.01  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.72  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 57.24 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 3.00  | >= 2 pass, < 1.5 fail |
| PASS | Share of casts that are the most used ability | 0.67  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.18  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 1.56 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 13658.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 42 s; first quest turned in: median 8 s; level 2: median 151 s (100% of level-1 starters reached it)
- XP per hour: median 7931 (design pace 6400); 76% of XP from quests; 124 quests turned in, median 4.4 min each
- Rewards: median gap 11 s; median longest drought 140 s; 17% of bots had a drought over 5 min
- Combat: 327 fights, 93% won, median 6.4 s; 239 kills, 53 without credit (10 of them with no other player within 20 blocks); 19 deaths (4.2/h)
- Travel: 57 blocks per reward; money earned 13658 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 1.6 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| dead | 0.4% |
| fight | 15.2% |
| gather | 2.4% |
| idle | 0.6% |
| loot | 1.2% |
| rest | 0.3% |
| town | 7.1% |
| travel | 72.5% |
| wait_spawn | 0.4% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 6 | 1.0 | 0 |
| 3 | 6 | 2.0 | 487 |
| 6 | 6 | 2.0 | 972 |
| 9 | 6 | 3.0 | 1428 |
| 12 | 6 | 3.0 | 1813 |
| 15 | 6 | 3.0 | 2431 |
| 18 | 6 | 4.0 | 2729 |
| 21 | 6 | 4.0 | 3036 |
| 24 | 6 | 4.0 | 3540 |
| 27 | 6 | 4.0 | 3944 |
| 30 | 6 | 4.0 | 4403 |
| 33 | 6 | 5.0 | 4744 |
| 36 | 6 | 5.0 | 5085 |
| 39 | 6 | 5.0 | 5295 |
| 42 | 6 | 5.0 | 5600 |
| 45 | 1 | 5.0 | 5758 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| barrow_warden | 1 | 0 | - |
| boar_trouble | 6 | 6 | 14.8 |
| broken_tower | 6 | 6 | 5.1 |
| coal_for_the_watch | 6 | 5 | 11.1 |
| cold_hearth | 6 | 6 | 0.1 |
| down_to_water | 6 | 1 | 21.9 |
| dust_of_shadows | 6 | 6 | 0.5 |
| eyes_in_the_dark | 6 | 6 | 7.0 |
| forgotten_graves | 6 | 4 | 4.4 |
| garricks_camp | 6 | 6 | 16.6 |
| hearth_stirs | 3 | 0 | - |
| into_barrowwood | 5 | 1 | 4.8 |
| iron_for_hollis | 5 | 3 | 27.3 |
| kindling | 6 | 6 | 1.6 |
| know_the_vale | 5 | 0 | - |
| lay_of_the_land | 6 | 5 | 9.8 |
| lookouts_eye | 6 | 6 | 0.8 |
| meat_for_the_pot | 6 | 6 | 2.4 |
| north_watch | 6 | 6 | 3.1 |
| pincher_supper | 1 | 0 | - |
| shells_for_nets | 1 | 0 | - |
| silk_for_sails | 6 | 4 | 8.0 |
| stolen_grain | 6 | 0 | - |
| thieves_in_fields | 6 | 2 | 16.0 |
| thin_the_herd | 6 | 6 | 3.6 |
| tusks_and_tempers | 6 | 5 | 0.9 |
| watchfire | 6 | 6 | 2.3 |
| webs_between_roots | 6 | 4 | 6.5 |
| windmill_breadcrumb | 6 | 6 | 9.8 |
| wool_for_winter | 6 | 6 | 0.8 |
| word_for_rysa | 6 | 6 | 1.7 |
| word_from_east | 3 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x6
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x6
- watchfire: collect Coal: no source in reach x4
- iron_for_hollis: collect Raw iron: no source in reach x3
- dust_of_shadows: collect Shade dust: no source in reach x2
- dust_of_shadows: kill Shade: only at night: waiting for dark x2
- eyes_in_the_dark: goto failed: too many repaths x2
- thieves_in_fields: goto failed: too many repaths x2
- hearth_stirs: kill Shade: only at night: waiting for dark x1
- webs_between_roots: goto failed: too many repaths x1

## Deaths by cause

- You fell to a Barrow Warden. x5
- You fell to a Websnare. x5
- You fell to a Shade. x3
- You fell to a Ashcloak Bandit. x2
- You fell to a Thornboar. x2
- You fell to a Mirepincher. x1
- You fell to a Rook Varn. x1

## Fights by creature and outcome

- Grazer: won x87
- Shade: won x66
- Websnare: won x66
- Thornboar: won x58
- Ashcloak Bandit: won x21
- Mirepincher: won x6
- Barrow Warden: lost (died) x5
- Websnare: lost (died) x5
- Shade: lost (died) x3
- Websnare: lost (it fled home and healed) x3
- Ashcloak Bandit: lost (died) x2
- Thornboar: lost (died) x2
- Mirepincher: lost (died) x1
- Rook Varn: lost (died) x1
- Websnare: lost (could not reach or hurt it) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 92 | Quick Shot 59%, Barbed Arrow 39%, Recall 2% |
| Mage | 111 | Fireball 71%, Fire Blast 24%, Recall 3%, Scholar's Focus 2% |
| Paladin | 93 | Judging Blow 65%, Rebuke 18%, Holy Mend 10%, Valour 5%, Recall 2% |
| Priest | 174 | Smite 71%, Withering Word 25%, Mend 3%, Recall 2% |
| Rogue | 124 | Vicious Strike 79%, Finishing Cut 18%, Recall 3% |
| Warrior | 86 | Heroic Blow 50%, Rending Cut 31%, Battle Cry 9%, Onrush 7%, Recall 2% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [224, 480]: 2 events
  - Botnze03com at [224, -9, 495] going to [230, -6, 495] (repaths; thieves_in_fields: go to the Ashcloak Bandit camp)
  - Botnze04com at [224, -9, 495] going to [230, -6, 495] (repaths; thieves_in_fields: go to the Ashcloak Bandit camp)
- cell [-496, -192]: 1 events
  - Botnze03com at [-490, 1, -179] going to [-490, 0, -185] (repaths; webs_between_roots: go to the Websnare camp)
- cell [-48, -32]: 1 events
  - Botnze02com at [-46, 0, -25] going to [-42, 0, -25] (repaths; eyes_in_the_dark: look for Shade in the dark)
- cell [-16, 32]: 1 events
  - Botnze02com at [-16, 0, 45] going to [-20, 4, 45] (repaths; eyes_in_the_dark: look for Shade in the dark)
- cell [32, 32]: 1 events
  - Botnze04com at [33, 0, 40] going to [29, 0, 40] (repaths; eyes_in_the_dark: look for Shade in the dark)
- cell [48, 0]: 1 events
  - Botnze04com at [48, 0, 14] going to [48, 0, 10] (repaths; eyes_in_the_dark: look for Shade in the dark)

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| completionist | 6 | 7931 | 124 | 4.2 | 93% | 11 | 1% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 1 | 7677 | 20 | 6.7 | 86% | 11 | 1% |
| Mage | 1 | 7764 | 21 | 2.7 | 97% | 11 | 3% |
| Paladin | 1 | 8555 | 23 | 2.7 | 95% | 15 | 1% |
| Priest | 1 | 8098 | 21 | 4.0 | 93% | 10 | 1% |
| Rogue | 1 | 9022 | 22 | 5.3 | 94% | 10 | 2% |
| Warrior | 1 | 6439 | 17 | 4.0 | 93% | 10 | 1% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botnze00com | Warrior | completionist | 45.0 | 1->5 | 6439 | 17/24 | 37 | 3 | 10 | 1007 |
| Botnze01com | Paladin | completionist | 45.0 | 1->5 | 8555 | 23/30 | 35 | 2 | 15 | 123 |
| Botnze02com | Hunter | completionist | 45.0 | 1->5 | 7677 | 20/29 | 37 | 5 | 11 | 140 |
| Botnze03com | Rogue | completionist | 45.0 | 1->5 | 9022 | 22/31 | 49 | 4 | 10 | 124 |
| Botnze04com | Priest | completionist | 45.0 | 1->5 | 8098 | 21/27 | 44 | 3 | 10 | 150 |
| Botnze05com | Mage | completionist | 45.0 | 1->5 | 7764 | 21/27 | 37 | 2 | 11 | 141 |
