# Hearthblock playtest: 12 bots, 30 min, brain mix

12 bots on `127.0.0.1:7710`, 339.9 bot-minutes (longest session 30.0 min). Thresholds: docs/playtest/fun-metrics.md.

## Fun checks: 3 fail, 2 warn

| | Check | Value | Target |
|---|---|---|---|
| WARN | Time to first kill (median) | 70.05 s | <= 60 s pass, > 120 s fail |
| PASS | Time to first quest turned in (median) | 7.66 s | <= 180 s pass, > 300 s fail |
| PASS | Time to level 2 from level 1 (median) | 145.20 s | <= 300 s (design) pass, > 600 s fail |
| PASS | XP per hour vs. the design pace (level 5 in 45 min) | 1.13 x design | >= 0.8 pass, < 0.5 fail |
| PASS | Share of XP from quests | 0.61  | >= 0.45 pass (design 0.6), < 0.3 fail |
| PASS | Minutes per quest (median, accept to turn-in) | 3.18 min | <= 10 pass, > 15 fail |
| PASS | Seconds between rewarding moments (median of bots' medians) | 11.70 s | <= 60 pass, > 90 fail |
| WARN | Share of bots with a reward drought over 5 minutes | 0.17  | <= 0.1 pass, > 0.25 fail |
| FAIL | Deaths per hour of play | 7.41 /h | <= 1.5 pass, > 4 fail |
| PASS | Fights won | 0.94  | >= 0.9 pass, < 0.75 fail |
| PASS | Seconds per won fight (median) | 7.11 s | <= 25 pass, > 40 fail (and >= 5) |
| PASS | Share of time idle, stuck, waiting for spawns or dead | 0.06  | <= 0.15 pass, > 0.3 fail |
| FAIL | Share of time travelling | 0.58  | <= 0.4 pass, > 0.55 fail |
| PASS | Blocks walked per reward | 37.78 blocks | <= 60 pass, > 120 fail |
| PASS | Distinct abilities used per fighting bot | 2.83  | >= 2 pass, < 1.5 fail |
| PASS | Share of casts that are the most used ability | 0.68  | <= 0.7 pass, > 0.85 fail |
| FAIL | Kills lost to someone else's last hit | 0.18  | <= 0.05 pass, > 0.15 fail |
| PASS | Stuck events per bot-hour | 1.41 /h | <= 2 pass, > 6 fail |
| PASS | Action round trip p95 | 50.00 ms | <= 150 pass, > 400 fail |
| PASS | Unexpected disconnects | 0.00  | 0 pass, > 1 fail |
| PASS | Moves the server refused (rubber-bands) per bot-hour, walking with real physics | 0.00 /h | <= 0.5 pass, > 2 fail |
| PASS | Copper earned per copper spent | 13428.00 x | >= 1.5 pass, < 1 fail |

## Pace

- First kill: median 70 s; first quest turned in: median 8 s; level 2: median 145 s (100% of level-1 starters reached it)
- XP per hour: median 7262 (design pace 6400); 61% of XP from quests; 129 quests turned in, median 3.2 min each
- Rewards: median gap 12 s; median longest drought 174 s; 17% of bots had a drought over 5 min
- Combat: 739 fights, 94% won, median 7.1 s; 555 kills, 123 without credit (18 of them with no other player within 20 blocks); 42 deaths (7.4/h)
- Travel: 38 blocks per reward; money earned 13428 c, spent 0 c
- Server: action round trip p50 50 ms, p95 50 ms; 0 disconnects; 1.4 stuck events per bot-hour

## Where the time goes

| Activity | Share |
|---|---|
| afk | 0.7% |
| dead | 0.6% |
| fight | 28.9% |
| gather | 2.2% |
| idle | 2.5% |
| loot | 1.6% |
| rest | 1.4% |
| town | 1.1% |
| travel | 57.7% |
| wait_spawn | 3.3% |

## Level curve

| Minute | Bots | Median level | Mean XP earned |
|---|---|---|---|
| 0 | 12 | 1.0 | 0 |
| 2 | 12 | 1.0 | 324 |
| 4 | 12 | 2.0 | 685 |
| 6 | 12 | 2.0 | 912 |
| 8 | 12 | 2.0 | 1208 |
| 10 | 11 | 3.0 | 1458 |
| 12 | 11 | 3.0 | 1868 |
| 14 | 11 | 3.0 | 2221 |
| 16 | 11 | 3.0 | 2493 |
| 18 | 11 | 4.0 | 2626 |
| 20 | 11 | 4.0 | 2775 |
| 22 | 11 | 4.0 | 3028 |
| 24 | 11 | 4.0 | 3244 |
| 26 | 11 | 4.0 | 3449 |
| 28 | 11 | 4.0 | 3597 |

## Quests

| Quest | Accepted | Done | Median min |
|---|---|---|---|
| boar_trouble | 9 | 5 | 12.9 |
| broken_tower | 9 | 8 | 5.7 |
| coal_for_the_watch | 9 | 7 | 4.0 |
| cold_hearth | 12 | 12 | 0.1 |
| down_to_water | 5 | 1 | 5.5 |
| dust_of_shadows | 7 | 6 | 0.0 |
| eyes_in_the_dark | 9 | 7 | 4.2 |
| forgotten_graves | 7 | 4 | 1.9 |
| garricks_camp | 9 | 4 | 4.7 |
| hearth_stirs | 2 | 1 | 12.9 |
| into_barrowwood | 1 | 0 | - |
| iron_for_hollis | 6 | 2 | 10.4 |
| kindling | 9 | 9 | 0.6 |
| know_the_vale | 6 | 0 | - |
| lay_of_the_land | 8 | 2 | 4.1 |
| lookouts_eye | 9 | 4 | 9.6 |
| meat_for_the_pot | 9 | 9 | 2.7 |
| north_watch | 9 | 9 | 3.6 |
| old_tusk | 5 | 1 | 1.1 |
| pincher_supper | 1 | 0 | - |
| shells_for_nets | 1 | 0 | - |
| silk_for_sails | 4 | 0 | - |
| stolen_grain | 5 | 1 | 4.7 |
| thieves_in_fields | 5 | 1 | 4.7 |
| thin_the_herd | 12 | 12 | 3.7 |
| tusks_and_tempers | 4 | 1 | 0.0 |
| watchfire | 9 | 7 | 0.4 |
| webs_between_roots | 2 | 1 | 3.0 |
| windmill_breadcrumb | 7 | 5 | 6.6 |
| wool_for_winter | 9 | 8 | 1.2 |
| word_for_rysa | 4 | 2 | 1.0 |
| word_from_east | 2 | 0 | - |

## Blockers (bots that could not progress an objective)

- coal_for_the_watch: collect Coal: no source in reach x7
- eyes_in_the_dark: kill Shade: only at night: waiting for dark x6
- iron_for_hollis: collect Raw iron: no source in reach x6
- hearth_stirs: collect Shade dust: no source in reach x2
- hearth_stirs: kill Shade: only at night: waiting for dark x2
- thieves_in_fields: goto failed: too many repaths x2
- watchfire: collect Coal: no source in reach x2
- dust_of_shadows: collect Shade dust: no source in reach x1
- dust_of_shadows: kill Shade: only at night: waiting for dark x1
- hearth_stirs: collect Log: no source in reach x1

## Deaths by cause

- You fell to a Grazer. x18
- You fell to a Shade. x11
- You fell to a Thornboar. x7
- You fell to a Ashcloak Bandit. x4
- You fell to a Ashcloak Cutthroat. x1
- You fell to a Websnare. x1

## Fights by creature and outcome

- Grazer: won x451
- Shade: won x151
- Thornboar: won x50
- Ashcloak Bandit: won x25
- Grazer: lost (died) x18
- Shade: lost (died) x12
- Websnare: won x11
- Thornboar: lost (died) x7
- Ashcloak Bandit: lost (died) x4
- Ashcloak Cutthroat: won x2
- Mirepincher: won x2
- Thornboar: lost (it fled home and healed) x2
- Grazer: lost (it fled home and healed) x1
- Old Tusk: won x1
- Websnare: lost (died) x1
- Websnare: lost (it fled home and healed) x1

## Casts by class and ability

| Class | Casts | Abilities (share of casts) |
|---|---|---|
| Hunter | 109 | Quick Shot 61%, Barbed Arrow 37%, Recall 3% |
| Mage | 223 | Fireball 51%, Fire Blast 48%, Scholar's Focus 1% |
| Paladin | 186 | Judging Blow 82%, Rebuke 12%, Holy Mend 4%, Valour 3% |
| Priest | 223 | Smite 73%, Withering Word 25%, Mend 2%, Recall 1% |
| Rogue | 125 | Vicious Strike 75%, Finishing Cut 24%, Recall 1% |
| Warrior | 80 | Heroic Blow 50%, Rending Cut 34%, Battle Cry 14%, Recall 2% |

## Stuck spots (16-block cells, hearth-relative x,z)

- cell [-48, 0]: 2 events
  - Botxpe09par at [-34, 0, 15] going to [-37, 0, 13] (repaths; following Botxpe08par)
  - Botxpe09par at [-46, 0, 6] going to [-50, 0, 6] (repaths; following Botxpe08par)
- cell [224, 480]: 2 events
  - Botxpe00com at [224, -9, 495] going to [230, -6, 495] (repaths; thieves_in_fields: go to the Ashcloak Bandit camp)
  - Botxpe08par at [224, -9, 495] going to [230, -6, 495] (repaths; thieves_in_fields: go to the Ashcloak Bandit camp)
- cell [-384, -16]: 1 events
  - Botxpe02exp at [-372, -4, -1] going to [-384, -5, -1] (repaths; exploring Bluebell Glade)
- cell [-176, 0]: 1 events
  - Botxpe02exp at [-168, -5, 6] going to [-180, 0, 6] (repaths; exploring Trapper's Camp)
- cell [-96, 0]: 1 events
  - Botxpe09par at [-92, -4, 15] going to [-95, -4, 17] (repaths; following Botxpe08par)
- cell [32, -144]: 1 events
  - Botxpe07gri at [36, -7, -130] going to [30, -6, -130] (repaths; going to grind)

## Notes

- grouped in a party of two x2
- casual session ended x1
- tried /invite Botxpe03com x1

## By brain

| brain | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| casual | 1 | 8059 | 8 | 0.0 | 100% | 10 | 0% |
| completionist | 4 | 8827 | 64 | 2.0 | 97% | 14 | 5% |
| explorer | 1 | 8632 | 16 | 4.0 | 93% | 15 | 1% |
| grinder | 3 | 6922 | 6 | 22.0 | 92% | 10 | 3% |
| party | 2 | 5787 | 20 | 2.0 | 97% | 15 | 12% |
| social | 1 | 7456 | 15 | 2.0 | 94% | 8 | 19% |

## By class

| class | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |
|---|---|---|---|---|---|---|---|
| Hunter | 2 | 7382 | 27 | 4.0 | 93% | 14 | 1% |
| Mage | 2 | 8200 | 10 | 15.0 | 93% | 8 | 2% |
| Paladin | 2 | 6516 | 4 | 23.0 | 92% | 10 | 3% |
| Priest | 2 | 9466 | 34 | 3.0 | 94% | 10 | 10% |
| Rogue | 2 | 8014 | 26 | 1.0 | 99% | 13 | 11% |
| Warrior | 2 | 6965 | 28 | 1.0 | 99% | 16 | 10% |

## Bots

| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |
|---|---|---|---|---|---|---|---|---|---|---|
| Botxpe00com | Warrior | completionist | 30.0 | 1->4 | 6862 | 14/23 | 22 | 0 | 15 | 373 |
| Botxpe01gri | Paladin | grinder | 30.0 | 1->4 | 6922 | 2/2 | 102 | 11 | 10 | 143 |
| Botxpe02exp | Hunter | explorer | 30.0 | 1->4 | 8632 | 16/27 | 28 | 2 | 15 | 116 |
| Botxpe03com | Rogue | completionist | 30.0 | 1->5 | 10586 | 17/25 | 28 | 1 | 9 | 94 |
| Botxpe04soc | Priest | social | 30.0 | 1->4 | 7456 | 15/22 | 23 | 1 | 8 | 206 |
| Botxpe05cas | Mage | casual | 9.9 | 1->3 | 8059 | 8/14 | 9 | 0 | 10 | 58 |
| Botxpe06com | Warrior | completionist | 30.0 | 1->4 | 7068 | 14/23 | 25 | 1 | 17 | 255 |
| Botxpe07gri | Paladin | grinder | 30.0 | 1->4 | 6110 | 2/2 | 101 | 12 | 10 | 211 |
| Botxpe08par | Hunter | party | 30.0 | 1->4 | 6132 | 11/22 | 23 | 2 | 13 | 324 |
| Botxpe09par | Rogue | party | 30.0 | 1->4 | 5442 | 9/18 | 27 | 0 | 17 | 207 |
| Botxpe10com | Priest | completionist | 30.0 | 1->5 | 11476 | 19/25 | 38 | 2 | 13 | 86 |
| Botxpe11gri | Mage | grinder | 30.0 | 1->4 | 8340 | 2/2 | 129 | 10 | 6 | 84 |
