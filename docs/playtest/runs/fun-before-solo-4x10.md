# Hearthblock playtest: 4 bots, 10 min, brain completionist

4 bots on `127.0.0.1:7712`, 40.0 bot-minutes (longest session 10.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 2 fail, 4 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 94.55 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.71 s | <= 180 s pass, > 300 s fail |
| WARN | Time to level 2 from level 1 (median) | 373.95 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 0.96 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.60  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 2.05 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 15.62 s | <= 60 pass, > 90 fail |
| PASS | Share of bots with a reward drought over 5 minutes | 0.00  | <= 0.1 pass, > 0.25 fail |
| PASS | Deaths per hour of play | 0.00 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 1.00  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 4.49 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.01  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.86  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 58.64 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.00  | >= 2 pass, < 1.5 fail |
| FAIL | Share of casts that are the most used ability | 0.86  | <= 0.7 pass, > 0.85 fail |
| WARN | Kills lost to someone else's last hit | 0.13  | <= 0.05 pass, > 0.15 fail |
| WARN | Stuck events per bot-hour | 3.00 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 51.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 771.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 95 s; first quest turned in: median 8 s; level 2: median 374 s (100% of level-1 starters reached it)
- XP per hour: median 6123 (design pace 6400); 60% of XP from quests; 17 quests turned in, median 2.1 min each
- Rewards: median gap 16 s; median longest drought 89 s; 0% of bots had a drought over 5 min
- Combat: 40 fights, 100% won, median 4.5 s; 33 kills, 5 without credit (0 of them with no other player within 20 blocks); 0 deaths (0.0/h)
- Travel: 59 blocks per reward; money earned 771 c, spent 0 c
- Server: action round trip p50 50 ms, p95 51 ms; 0 disconnects; 3.0 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| fight | 8.1% |
| gather | 3.0% |
| idle | 1.0% |
| loot | 0.6% |
| town | 1.0% |
| travel | 86.2% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 4 | 1.0 | 0 |
| 1 | 4 | 1.0 | 110 |
| 2 | 4 | 1.0 | 176 |
| 3 | 4 | 1.0 | 254 |
| 4 | 4 | 1.0 | 336 |
| 5 | 4 | 1.0 | 408 |
| 6 | 4 | 1.5 | 514 |
| 7 | 4 | 2.0 | 684 |
| 8 | 4 | 2.0 | 830 |
| 9 | 4 | 2.0 | 1038 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 1 | 0 | - |
| coal_for_the_watch | 4 | 0 | - |
| cold_hearth | 4 | 4 | 0.1 |
| eyes_in_the_dark | 1 | 0 | - |
| garricks_camp | 1 | 0 | - |
| kindling | 4 | 4 | 6.4 |
| meat_for_the_pot | 4 | 4 | 7.6 |
| north_watch | 1 | 1 | 0.8 |
| report_to_holt | 1 | 1 | 0.8 |
| thin_the_herd | 4 | 1 | 7.8 |
| watchfire | 1 | 0 | - |
| windmill_breadcrumb | 2 | 0 | - |
| wool_for_winter | 4 | 2 | 1.0 |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x4
- kindling: collect Log: no source in reach x3
- thin_the_herd: goto failed: too many repaths x2
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x1
- watchfire: collect Coal: no source in reach x1

## Fights by creature and outcome

- Grazer: won x40

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 16 | Quick Shot 81%, Barbed Arrow 19% |
| Paladin | 13 | Judging Blow 85%, Holy Mend 8%, Recall 8% |
| Rogue | 22 | Vicious Strike 86%, Finishing Cut 9%, Recall 5% |
| Warrior | 6 | Heroic Blow 67%, Battle Cry 17%, Recall 17% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-272, 144]: 2 events
  - Botztw00com at [-260, -4, 144] going to [-260, -7, 150] (repaths; thin_the_herd: go to the Grazer camp)
  - Botztw02com at [-260, -4, 144] going to [-260, -7, 150] (repaths; thin_the_herd: go to the Grazer camp)

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| completionist | 4 | 6123 | 17 | 0.0 | 100% | 16 | 1% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 1 | 6092 | 3 | 0.0 | 100% | 8 | 0% |
| Paladin | 1 | 9136 | 7 | 0.0 | 100% | 10 | 0% |
| Rogue | 1 | 6154 | 4 | 0.0 | 100% | 23 | 2% |
| Warrior | 1 | 4766 | 3 | 0.0 | 100% | 21 | 2% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botztw00com | Warrior | completionist | 10.0 | 1->2 | 4766 | 3/6 | 6 | 0 | 21 | 106 |
| Botztw01com | Paladin | completionist | 10.0 | 1->3 | 9136 | 7/13 | 10 | 0 | 10 | 63 |
| Botztw02com | Hunter | completionist | 10.0 | 1->2 | 6092 | 3/6 | 10 | 0 | 8 | 88 |
| Botztw03com | Rogue | completionist | 10.0 | 1->2 | 6154 | 4/7 | 7 | 0 | 23 | 90 |
