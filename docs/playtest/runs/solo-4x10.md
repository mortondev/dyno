# Hearthblock playtest: 4 bots, 10 min, brain completionist

4 bots on `local`, 40.0 bot-minutes (longest session 10.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 4 fail, 1 warn

| | Check | Value | Target |
|---|---|---|---|
| PASS | Time to first kill (median) | 39.78 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.86 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 42.20 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 3.28 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.78  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 0.79 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 7.11 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 13.50 /h | <= 1.5 pass, > 4 fail |
| FAIL | Fights won | 0.69  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 7.54 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.03  | <= 0.15 pass, > 0.3 fail |
| PASS | Share of time travelling | 0.27  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 17.65 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.50  | >= 2 pass, < 1.5 fail |
| WARN | Share of casts that are the most used ability | 0.79  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.15  | <= 0.05 pass, > 0.15 fail |
| FAIL | Stuck events per bot-hour | 61.51 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 2686.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 40 s; first quest turned in: median 8 s; level 2: median 42 s (100% of level-1 starters reached it)
- XP per hour: median 20975 (design pace 6400); 78% of XP from quests; 43 quests turned in, median 0.8 min each
- Rewards: median gap 7 s; median longest drought 79 s; 0% of bots had a drought over 5 min
- Combat: 107 fights, 69% won, median 7.5 s; 61 kills, 11 without credit (2 of them with no other player within 20 blocks); 9 deaths (13.5/h)
- Travel: 18 blocks per reward; money earned 2686 c, spent 0 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 61.5 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| dead | 1.2% |
| fight | 60.7% |
| gather | 5.9% |
| idle | 0.5% |
| loot | 1.1% |
| rest | 1.5% |
| stuck | 1.2% |
| town | 1.3% |
| travel | 26.6% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 4 | 1.0 | 0 |
| 1 | 4 | 2.0 | 526 |
| 2 | 4 | 2.0 | 955 |
| 3 | 4 | 3.0 | 1522 |
| 4 | 4 | 3.0 | 2273 |
| 5 | 4 | 3.5 | 2717 |
| 6 | 4 | 4.0 | 3120 |
| 7 | 4 | 4.0 | 3378 |
| 8 | 4 | 4.0 | 3406 |
| 9 | 4 | 4.0 | 3460 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 4 | 2 | 4.0 |
| coal_for_the_watch | 4 | 2 | 3.1 |
| cold_hearth | 4 | 4 | 0.1 |
| down_to_water | 3 | 0 | - |
| eyes_in_the_dark | 4 | 0 | - |
| garricks_camp | 4 | 4 | 0.6 |
| iron_for_hollis | 2 | 0 | - |
| kindling | 4 | 4 | 1.0 |
| meat_for_the_pot | 4 | 4 | 1.1 |
| north_watch | 4 | 4 | 0.5 |
| report_to_holt | 4 | 4 | 0.1 |
| silk_for_sails | 4 | 0 | - |
| stolen_grain | 3 | 0 | - |
| thieves_in_fields | 3 | 1 | 6.0 |
| thin_the_herd | 4 | 4 | 2.7 |
| tusks_and_tempers | 4 | 0 | - |
| watchfire | 4 | 3 | 3.1 |
| windmill_breadcrumb | 4 | 3 | 2.3 |
| wool_for_winter | 4 | 4 | 0.7 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x4
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x4
- iron_for_hollis: gather Raw iron: needs mining 25: raising it on easier nodes x2
- down_to_water: talk failed: cannot reach NPC: stuck x1
- watchfire: collect Coal: no source in reach x1

## Deaths by cause

- You fell to a Ashcloak Bandit. x4
- You fell to a Thornboar. x3
- You fell to a Rook Varn. x1
- You fell to a Websnare. x1

## Fights by creature and outcome

- Grazer: won x42
- Thornboar: won x25
- Ashcloak Bandit: won x7
- Thornboar: lost (no progress: it hardly took damage) x6
- Ashcloak Bandit: lost (died) x5
- Ashcloak Bandit: lost (no progress: it hardly took damage) x5
- Websnare: lost (cannot reach it: stuck) x4
- Thornboar: lost (died) x3
- Grazer: lost (cannot reach it: no path) x2
- Thornboar: lost (cannot reach it: stuck) x2
- Websnare: lost (no progress: it hardly took damage) x2
- Ashcloak Bandit: lost (cannot reach it: stuck) x1
- Ashcloak Bandit: lost (could not reach or hurt it) x1
- Ashcloak Cutthroat: lost (could not reach or hurt it) x1
- Websnare: lost (died) x1

## Unreachable NPCs

- Sergeant Holt (cannot reach NPC: stuck) x1

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-80, -96]: 6 events
  - Botmvt00com at [-66, -3, -95] going to [-96, -10, -55] (stuck; defending against Websnare)
  - Botmvt00com at [-66, -3, -95] going to [-74, -8, -57] (stuck; silk_for_sails: kill Websnare for Websnare Silk)
  - Botmvt00com at [-66, -3, -95] going to [-70, -22, -100] (stuck; coal_for_the_watch: gather Coal)
  - Botmvt00com at [-66, -3, -95] going to [-23, -6, -72] (stuck; boar_trouble: kill Thornboar)
- cell [-64, -128]: 6 events
  - Botmvt00com at [-57, -6, -113] going to [-57, -17, -113] (no path; coal_for_the_watch: gather Coal)
  - Botmvt00com at [-53, -3, -113] going to [-59, -16, -115] (stuck; coal_for_the_watch: gather Coal)
  - Botmvt00com at [-53, -3, -113] going to [-74, -8, -57] (stuck; silk_for_sails: kill Websnare for Websnare Silk)
  - Botmvt00com at [-53, -3, -113] going to [-117, -6, 7] (stuck; boar_trouble: kill Thornboar)
- cell [0, -48]: 6 events
  - Botmvt00com at [1, -5, -43] going to [1, -13, -43] (no path; watchfire: gather Coal)
  - Botmvt01com at [0, -6, -45] going to [1, -13, -43] (no path; coal_for_the_watch: gather Coal)
  - Botmvt01com at [7, -6, -46] going to [7, -12, -46] (no path; coal_for_the_watch: gather Coal)
  - Botmvt02com at [0, -6, -45] going to [1, -13, -43] (no path; iron_for_hollis: gather Raw iron)
- cell [16, 112]: 4 events
  - Botmvt02com at [19, -13, 123] going to [2, -5, -49] (stuck; visiting Sergeant Holt: hand in watchfire)
  - Botmvt02com at [19, -13, 123] going to [45, -12, 106] (stuck; thieves_in_fields: kill Ashcloak Bandit)
  - Botmvt02com at [19, -13, 123] going to [-26, -11, 138] (stuck; down_to_water: talk to Old Wenna)
  - Botmvt02com at [19, -13, 123] going to [0, 0, 0] (gave up: /spawn; down_to_water: talk to Old Wenna)
- cell [0, -16]: 3 events
  - Botmvt01com at [11, 0, -2] going to [11, -8, -2] (no path; coal_for_the_watch: gather Coal)
  - Botmvt01com at [10, 1, -7] going to [10, -13, -6] (no path; coal_for_the_watch: gather Coal)
  - Botmvt03com at [11, 0, -2] going to [11, -8, -2] (no path; coal_for_the_watch: gather Coal)
- cell [-64, -112]: 2 events
  - Botmvt00com at [-53, -5, -99] going to [-53, -21, -99] (no path; coal_for_the_watch: gather Coal)
  - Botmvt00com at [-54, -7, -104] going to [-54, -20, -104] (no path; watchfire: gather Coal)
- cell [-64, -64]: 2 events
  - Botmvt01com at [-64, -6, -60] going to [-64, -12, -60] (no path; coal_for_the_watch: gather Coal)
  - Botmvt03com at [-64, -6, -60] going to [-64, -12, -60] (no path; iron_for_hollis: gather Raw iron)
- cell [-16, -48]: 2 events
  - Botmvt01com at [-2, -5, -41] going to [-2, -14, -41] (no path; watchfire: gather Coal)
  - Botmvt02com at [-2, -5, -41] going to [-2, -14, -41] (no path; watchfire: gather Coal)
- cell [-16, -16]: 2 events
  - Botmvt00com at [-2, 0, -8] going to [-2, -12, -8] (no path; coal_for_the_watch: gather Coal)
  - Botmvt01com at [-2, 0, -8] going to [-2, -12, -8] (no path; coal_for_the_watch: gather Coal)
- cell [0, 80]: 2 events
  - Botmvt02com at [10, -10, 87] going to [10, -17, 87] (no path; watchfire: gather Coal)
  - Botmvt03com at [10, -10, 87] going to [10, -17, 87] (no path; iron_for_hollis: gather Raw iron)
- cell [-16, -64]: 1 events
  - Botmvt01com at [-9, -6, -51] going to [-9, -14, -51] (no path; watchfire: gather Coal)
- cell [0, -112]: 1 events
  - Botmvt01com at [7, -6, -98] going to [7, -17, -98] (no path; coal_for_the_watch: gather Coal)

## Notes

- used /spawn to get unstuck x2

## Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)

- Grazer lv1 at 53,-1,-24 (ground -7) x4
- Grazer lv1 at 53,-1,-25 (ground -6) x4
- Grazer lv2 at 13,-1,-94 (ground -7) x4
- Grazer lv2 at 16,-1,-95 (ground -7) x4
- Grazer lv2 at 25,1,-45 (ground -6) x4
- Grazer lv2 at 25,1,-48 (ground -6) x4
- Grazer lv2 at 26,1,-45 (ground -5) x4
- Grazer lv2 at 27,-1,-48 (ground -6) x4
- Grazer lv2 at 27,-1,-50 (ground -6) x4
- Grazer lv2 at 27,0,-47 (ground -6) x4
- Grazer lv2 at 27,0,-62 (ground -7) x4
- Grazer lv2 at 27,1,-59 (ground -7) x4
- Grazer lv2 at 28,-1,-45 (ground -5) x4
- Grazer lv2 at 28,0,-46 (ground -6) x4
- Grazer lv2 at 28,0,-47 (ground -6) x4
- Grazer lv2 at 28,0,-48 (ground -6) x4
- Grazer lv2 at 28,0,-50 (ground -6) x4
- Grazer lv2 at 28,0,-63 (ground -8) x4
- Grazer lv2 at 29,0,-47 (ground -6) x4
- Grazer lv2 at 29,0,-49 (ground -6) x4
- Grazer lv2 at 29,0,-50 (ground -6) x4
- Grazer lv2 at 30,1,-61 (ground -7) x4
- Grazer lv2 at 31,-1,-47 (ground -5) x4
- Grazer lv2 at 31,0,-49 (ground -6) x4
- Grazer lv2 at 31,0,-60 (ground -6) x4

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| completionist | 4 | 20975 | 43 | 13.5 | 69% | 7 | 3% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 1 | 20222 | 10 | 12.0 | 62% | 6 | 6% |
| Paladin | 1 | 21728 | 10 | 12.0 | 80% | 9 | 2% |
| Rogue | 1 | 28898 | 13 | 24.0 | 74% | 5 | 3% |
| Warrior | 1 | 19742 | 10 | 6.0 | 59% | 8 | 1% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botmvt00com | Warrior | completionist | 10.0 | 1->4 | 19742 | 10/18 | 12 | 1 | 8 | 99 |
| Botmvt01com | Paladin | completionist | 10.0 | 1->4 | 21728 | 10/15 | 18 | 2 | 9 | 60 |
| Botmvt02com | Hunter | completionist | 10.0 | 1->4 | 20222 | 10/19 | 11 | 2 | 6 | 122 |
| Botmvt03com | Rogue | completionist | 10.0 | 1->5 | 28898 | 13/19 | 20 | 4 | 5 | 50 |
