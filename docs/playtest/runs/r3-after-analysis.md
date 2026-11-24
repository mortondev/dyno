## Machine and tick

| run | report | load: start / mean / max (cores) | server tick: mean / p95 / p99 / worst | worst minute's p95 | counts (p95 < 50 ms) |
|---|---|---|---|---|---|
| A | r3-after-a.json | 6.3 / 7.1 / 14.7 (16) | 0.43 / 1.25 / 2.25 / 20 ms | 1.75 ms | yes |
| B | r3-after-b.json | 11.4 / 5.9 / 11.4 (16) | 0.38 / 1.00 / 1.75 / 19 ms | 1.25 ms | yes |

## Per player type (medians; deaths per hour pooled over the bot-hours)

| brain (bots) | first kill | level 2 | level 3 | level 4 | level 5 | level at end | quests done | gear reward | first upgrade | median gap | longest gap | worst gap | travel % | fight % | deaths | bot-hours | deaths/h | lost kills % | money |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| completionist (8) | 1:04 | 2:03 | 7:48 | 15:13 | 25:10 | 5 | 19.5 | 3:24 | 3:24 | 12 s | 2:08 | 2:51 | 66% | 19% | 3 | 4.00 | 0.8 | 0% | 1989.5 |
| casual (2) | 1:34 | 3:37 | - | - | - | 2.5 | 8.5 | 5:47 | 5:42 | 10 s | 1:12 | 1:28 | 53% | 20% | 0 | 0.31 | 0.0 | 0% | 634 |
| explorer (2) | 2:20 | 3:44 | 12:38 | - | - | 3.5 | 9.5 | 12:28 | 7:36 | 16 s | 2:38 | 2:42 | 78% | 7% | 0 | 1.00 | 0.0 | 0% | 579.5 |
| social (2) | 1:09 | 2:51 | 9:12 | 17:14 | - | 4.5 | 18 | 4:00 | 4:00 | 17 s | 1:32 | 1:38 | 58% | 19% | 0 | 1.00 | 0.0 | 0% | 1680.5 |
| party (4) | 1:25 | 3:10 | 12:12 | - | - | 3 | 9.5 | 5:09 | 5:09 | 17 s | 8:37 | 9:55 | 49% | 10% | 18 | 2.00 | 9.0 | 17% | 806 |
| grinder (6) | 0:28 | 3:00 | 9:45 | 21:39 | - | 4 | 2 | 3:07 | 2:55 | 14 s | 0:49 | 1:06 | 4% | 37% | 0 | 3.00 | 0.0 | 0% | 153 |
| all (24) | 1:04 | 2:53 | 9:42 | 17:55 | - | 4 | 11.5 | 4:09 | 3:51 | 14 s | 1:45 | 9:55 | 47% | 21% | 21 | 11.30 | 1.9 | 1% | 1039 |

## Per class

| class (bots) | first kill | level 2 | level 3 | level 4 | level 5 | level at end | quests done | gear reward | first upgrade | median gap | longest gap | worst gap | travel % | fight % | deaths | bot-hours | deaths/h | lost kills % | money |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Hunter (4) | 1:05 | 3:02 | 12:25 | - | - | 3.5 | 11.5 | 7:20 | 5:56 | 16 s | 4:36 | 8:46 | 63% | 10% | 9 | 2.00 | 4.5 | 3% | 942 |
| Mage (4) | 0:52 | 3:37 | 9:46 | - | - | 3.5 | 4 | 5:32 | 5:42 | 10 s | 1:01 | 1:28 | 16% | 28% | 0 | 1.31 | 0.0 | 0% | 273 |
| Paladin (4) | 0:30 | 2:55 | 9:42 | 21:36 | - | 4 | 2 | 2:55 | 2:55 | 14 s | 0:49 | 0:55 | 3% | 41% | 0 | 2.00 | 0.0 | 0% | 152 |
| Priest (4) | 1:09 | 2:51 | 9:37 | 17:19 | - | 4.5 | 18 | 4:14 | 4:14 | 16 s | 1:34 | 2:15 | 60% | 17% | 1 | 2.00 | 0.5 | 0% | 1875.5 |
| Rogue (4) | 1:26 | 2:52 | 7:49 | - | - | 4 | 13.5 | 3:33 | 3:33 | 15 s | 5:38 | 9:55 | 59% | 15% | 10 | 2.00 | 5.0 | 10% | 1250 |
| Warrior (4) | 1:01 | 2:00 | 7:16 | 14:17 | 25:17 | 5 | 19.5 | 3:15 | 3:13 | 10 s | 1:59 | 2:51 | 68% | 20% | 1 | 2.00 | 0.5 | 0% | 2112.5 |
| all (24) | 1:04 | 2:53 | 9:42 | 17:55 | - | 4 | 11.5 | 4:09 | 3:51 | 14 s | 1:45 | 9:55 | 47% | 21% | 21 | 11.30 | 1.9 | 1% | 1039 |

## Deaths, each one

| run | bot | brain | class | at | lv | killer (gap) | engaged at death | day/night | health at pull | pulled | where | quest area | road | cause |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| A | 03com | completionist | Rogue | 26:06 | 5 | Gloomwisp 7 (+2) | Gloomwisp 7 | night | 87% | Gloomwisp 7 (attacked) | The Sunken Shrine | know_the_vale | yes | over-level |
| B | 00com | completionist | Warrior | 25:31 | 5 | Gloomwisp 8 (+3) | Gloomwisp 8 | day | 87% | Gloomwisp 8 (attacked) | The Sunken Shrine | know_the_vale | yes | over-level |
| B | 10com | completionist | Priest | 25:35 | 4 | Gloomwisp 8 (+4) | none in reach | night | 34% | Gloomwisp 8 (attacked) | The Sunken Shrine | know_the_vale | yes | over-level |
| A | 08par | party | Hunter | 21:59 | 3 | Old Tusk 4 (+1) | Old Tusk 4 | day | 100% | Old Tusk 4 | 81 m from meadow_flock_4 | old_tusk camp | no | one-on-one |
| A | 09par | party | Rogue | 22:09 | 3 | Old Tusk 4 (+1) | Old Tusk 4 | day | 100% | Old Tusk 4 | 80 m from meadow_flock_4 | old_tusk camp | no | one-on-one |
| A | 08par | party | Hunter | 23:55 | 3 | Old Tusk 4 (+1) | Old Tusk 4 | day | 100% | Old Tusk 4 | 86 m from meadow_flock_4 | old_tusk camp | no | one-on-one |
| A | 09par | party | Rogue | 24:02 | 3 | Old Tusk 4 (+1) | Old Tusk 4 | day | 100% | Old Tusk 4 | 83 m from meadow_flock_4 | old_tusk camp | no | one-on-one |
| A | 08par | party | Hunter | 25:44 | 3 | Old Tusk 4 (+1) | Old Tusk 4 | night | 100% | Old Tusk 4 | 81 m from meadow_flock_4 | old_tusk camp | no | one-on-one |
| A | 09par | party | Rogue | 25:51 | 3 | Old Tusk 4 (+1) | Old Tusk 4 | night | 100% | Old Tusk 4 | 82 m from meadow_flock_4 | old_tusk camp | no | one-on-one |
| A | 08par | party | Hunter | 27:33 | 3 | Old Tusk 4 (+1) | Old Tusk 4 | night | 100% | Old Tusk 4 | 81 m from meadow_flock_4 | old_tusk camp | no | one-on-one |
| A | 09par | party | Rogue | 27:43 | 3 | Old Tusk 4 (+1) | Old Tusk 4 | night | 100% | Old Tusk 4 | 82 m from meadow_flock_4 | old_tusk camp | no | one-on-one |
| A | 08par | party | Hunter | 29:28 | 3 | Old Tusk 4 (+1) | Old Tusk 4 | night | 100% | Old Tusk 4 | 81 m from meadow_flock_4 | old_tusk camp | no | one-on-one |
| A | 09par | party | Rogue | 29:35 | 3 | Old Tusk 4 (+1) | Old Tusk 4 | night | 100% | Old Tusk 4 | 79 m from meadow_flock_4 | old_tusk camp | no | one-on-one |
| B | 08par | party | Hunter | 19:58 | 3 | Old Tusk 4 (+1) | Old Tusk 4 | day | 100% | Old Tusk 4 | 84 m from meadow_flock_4 | old_tusk camp | no | one-on-one |
| B | 09par | party | Rogue | 20:03 | 2 | Old Tusk 4 (+2) | Old Tusk 4 | day | 100% | Old Tusk 4 | 81 m from meadow_flock_4 | old_tusk camp | no | over-level |
| B | 08par | party | Hunter | 21:48 | 3 | Old Tusk 4 (+1) | Old Tusk 4 | day | 100% | Old Tusk 4 | 81 m from meadow_flock_4 | old_tusk camp | no | one-on-one |
| B | 09par | party | Rogue | 21:53 | 2 | Old Tusk 4 (+2) | Old Tusk 4 | day | 100% | Old Tusk 4 | 80 m from meadow_flock_4 | old_tusk camp | no | over-level |
| B | 08par | party | Hunter | 23:26 | 3 | Old Tusk 4 (+1) | Old Tusk 4 | day | 100% | Old Tusk 4 | 82 m from meadow_flock_4 | old_tusk camp | no | one-on-one |
| B | 09par | party | Rogue | 23:32 | 2 | Old Tusk 4 (+2) | Old Tusk 4 | day | 100% | Old Tusk 4 | 85 m from meadow_flock_4 | old_tusk camp | no | over-level |
| B | 08par | party | Hunter | 25:19 | 3 | Old Tusk 4 (+1) | Old Tusk 4 | day | 100% | Old Tusk 4 | 81 m from meadow_flock_4 | old_tusk camp | no | one-on-one |
| B | 09par | party | Rogue | 25:24 | 2 | Old Tusk 4 (+2) | Old Tusk 4 | day | 100% | Old Tusk 4 | 82 m from meadow_flock_4 | old_tusk camp | no | over-level |

| brain | bot-hours | deaths | per hour | over-level | night | adds | jumped | low pull | one-on-one | deaths at night at level 1-4 on a road or in a quest area | of those, to a Shade |
|---|---|---|---|---|---|---|---|---|---|---|---|
| completionist | 4.00 | 3 | 0.8 | 3 | 0 | 0 | 0 | 0 | 0 | 1 | 0 |
| casual | 0.31 | 0 | 0.0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| explorer | 1.00 | 0 | 0.0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| social | 1.00 | 0 | 0.0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| party | 2.00 | 18 | 9.0 | 4 | 0 | 0 | 0 | 0 | 14 | 6 | 0 |
| grinder | 3.00 | 0 | 0.0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| all | 11.30 | 21 | 1.9 | 7 | 0 | 0 | 0 | 0 | 14 | 7 | 0 |

One-on-one losses at the same level or below (for the class lane):

- Hunter 3 (party) to Old Tusk 4, pulled at 100% health, day, 81 m from meadow_flock_4
- Hunter 3 (party) to Old Tusk 4, pulled at 100% health, day, 86 m from meadow_flock_4
- Hunter 3 (party) to Old Tusk 4, pulled at 100% health, night, 81 m from meadow_flock_4
- Hunter 3 (party) to Old Tusk 4, pulled at 100% health, night, 81 m from meadow_flock_4
- Hunter 3 (party) to Old Tusk 4, pulled at 100% health, night, 81 m from meadow_flock_4
- Rogue 3 (party) to Old Tusk 4, pulled at 100% health, day, 80 m from meadow_flock_4
- Rogue 3 (party) to Old Tusk 4, pulled at 100% health, day, 83 m from meadow_flock_4
- Rogue 3 (party) to Old Tusk 4, pulled at 100% health, night, 82 m from meadow_flock_4
- Rogue 3 (party) to Old Tusk 4, pulled at 100% health, night, 82 m from meadow_flock_4
- Rogue 3 (party) to Old Tusk 4, pulled at 100% health, night, 79 m from meadow_flock_4
- Hunter 3 (party) to Old Tusk 4, pulled at 100% health, day, 84 m from meadow_flock_4
- Hunter 3 (party) to Old Tusk 4, pulled at 100% health, day, 81 m from meadow_flock_4
- Hunter 3 (party) to Old Tusk 4, pulled at 100% health, day, 82 m from meadow_flock_4
- Hunter 3 (party) to Old Tusk 4, pulled at 100% health, day, 81 m from meadow_flock_4

## How the bots pull

| brain | fights | pulled under 60% health | under 40% | fights with an add | with two or more | at night |
|---|---|---|---|---|---|---|
| completionist | 340 | 1% | 1% | 4% | 2% | 32% |
| casual | 27 | 0% | 0% | 0% | 0% | 22% |
| explorer | 33 | 0% | 0% | 3% | 0% | 27% |
| social | 94 | 0% | 0% | 3% | 0% | 34% |
| party | 92 | 0% | 0% | 3% | 1% | 17% |
| grinder | 681 | 0% | 0% | 0% | 0% | 32% |
| all | 1267 | 0% | 0% | 2% | 1% | 31% |

## Time and travel

| brain | travel | fight | gather | town | rest | loot | waiting for spawns | dead | idle + stuck |
|---|---|---|---|---|---|---|---|---|---|
| completionist (8) | 66% | 19% | 3% | 1% | 6% | 1% | 2% | 0% | 2% |
| casual (2) | 53% | 20% | 5% | 1% | 13% | 5% | 1% | 0% | 2% |
| explorer (2) | 78% | 7% | 1% | 1% | 10% | 2% | 0% | 0% | 1% |
| social (2) | 58% | 19% | 2% | 1% | 17% | 2% | 0% | 0% | 2% |
| party (4) | 49% | 10% | 1% | 18% | 11% | 1% | 0% | 1% | 9% |
| grinder (6) | 4% | 37% | 0% | 0% | 51% | 4% | 0% | 0% | 3% |
| all (24) | 47% | 21% | 2% | 4% | 20% | 2% | 1% | 0% | 3% |

Travel by purpose (all but grinders), share of travel time:

| what the bot was on its way to | minutes | share |
|---|---|---|
| following a party member | 26.5 | 9% |
| visiting Foreman Garrick: hand in tusks_and_tempers | 12.8 | 4% |
| know_the_vale: explore the Barrow Overlook | 11.0 | 4% |
| boar_trouble: go to the Thornboar camp | 11.0 | 4% |
| visiting Guard Tolan: hand in broken_tower | 10.9 | 4% |
| north_watch: explore the North Watch beacon | 10.2 | 3% |
| visiting Guard Tolan: hand in lay_of_the_land | 10.0 | 3% |
| old_tusk: go to the Old Tusk camp | 9.2 | 3% |
| lay_of_the_land: explore the Trapper's Camp | 7.4 | 2% |
| broken_tower: explore the Broken Tower | 7.2 | 2% |
| coal_for_the_watch: gather Coal | 6.8 | 2% |
| lay_of_the_land: explore the Old Quarry | 6.5 | 2% |
| word_for_rysa: talk to Rysa | 6.4 | 2% |
| visiting Foreman Garrick: hand in silk_for_sails | 6.3 | 2% |
| visiting Captain Maren: hand in thin_the_herd | 5.9 | 2% |
| taking a party member to an NPC | 5.6 | 2% |
| visiting Tilda: hand in meat_for_the_pot | 5.3 | 2% |
| garricks_camp: explore Woodcutters' Well | 5.3 | 2% |
| tusks_and_tempers: go to the Thornboar camp for Boar Tusk | 4.8 | 2% |
| visiting Tilda: hand in wool_for_winter | 4.7 | 2% |
| windmill_breadcrumb: explore the Listening Stones | 4.3 | 1% |
| visiting Sergeant Holt: hand in boar_trouble | 4.2 | 1% |

The longest runs (unbroken travel), by route:

| from | to | for | longest | median | times | bots |
|---|---|---|---|---|---|---|
| Hearthstead | The Old Barrow | eyes_in_the_dark: wait for a Shade to rise | 9:05 | 9:05 | 1 | 1 |
| Hearthstead | The North Watch | boar_trouble: kill Thornboar | 8:00 | 4:38 | 2 | 2 |
| The Old Quarry | 34 m from The North Watch | helping a party member | 7:51 | 7:51 | 1 | 1 |
| 99 m from Hearthstead | 64 m from north_road_glade_1 | defending against Thornboar | 7:47 | 7:47 | 1 | 1 |
| 108 m from Hearthstead | 45 m from west_trail_camp_11 | boar_trouble: kill Thornboar | 6:31 | 6:31 | 1 | 1 |
| barrow_webs | The Barrowwood Graves | dust_of_shadows: kill Shade for Shade dust | 5:44 | 5:44 | 1 | 1 |
| 57 m from east_road_wood_5 | Beacon Rocks | watchfire: gather Coal | 5:26 | 5:26 | 1 | 1 |
| 63 m from north_road_glade_1 | birchrise_boars_3 | boar_trouble: kill Thornboar | 5:18 | 5:18 | 1 | 1 |
| 53 m from north_road_glade_1 | The North Watch | with a party member | 5:07 | 5:07 | 1 | 1 |
| Hearthstead | Beacon Rocks | watchfire: gather Coal | 4:52 | 4:52 | 1 | 1 |
| 109 m from south_road_tower_7 | Beacon Rocks | watchfire: gather Coal | 4:36 | 4:36 | 1 | 1 |
| 65 m from North Gate Rocks | The North Watch | boar_trouble: kill Thornboar | 4:28 | 4:28 | 1 | 1 |

## Reward gaps over 2:00

| run | bot | brain | from | gap | where: from, to | mostly | doing |
|---|---|---|---|---|---|---|---|
| A | 09par | party | 20:03 | 9:55 (to the end) | 49 m from logging_trail_well_14 → Hearthstead | travel 52%, town 20% | following a party member; helping a party member |
| A | 08par | party | 21:13 | 8:46 (to the end) | The Logging Camp → Hearthstead | travel 50%, rest 33% | old_tusk: go to the Old Tusk camp; resting |
| B | 09par | party | 10:46 | 8:27 | The North Watch → The North Watch | town 90%, travel 9% | with a party member; resting with the party |
| B | 08par | party | 19:28 | 6:30 | 59 m from logging_trail_well_14 → Hearthstead | travel 49%, rest 33% | old_tusk: go to the Old Tusk camp; resting |
| B | 09par | party | 19:28 | 6:28 | 56 m from logging_trail_well_14 → Hearthstead | travel 45%, town 26% | following a party member; helping a party member |
| B | 09par | party | 25:57 | 4:02 (to the end) | Hearthstead → The North Watch | town 82%, travel 18% | with a party member; resting with the party |
| B | 09par | party | 1:42 | 3:49 | 48 m from north_road_glade_1 → 75 m from North Gate Rocks | idle 96%, fight 4% | with a party member; resting with the party |
| B | 08par | party | 11:37 | 3:14 | Hearthstead → 66 m from west_trail_camp_11 | town 87%, fight 8% | waiting a party member; boar_trouble: kill Thornboar |
| A | 00com | completionist | 20:24 | 2:51 | The North Pass → 41 m from oak_trail_view_19 | travel 100% | know_the_vale: explore the Barrow Overlook; know_the_vale: explore the North Pass |
| B | 03com | completionist | 25:03 | 2:48 | The North Pass → 43 m from oak_trail_view_19 | travel 100% | know_the_vale: explore the Barrow Overlook |
| B | 02exp | explorer | 22:50 | 2:42 | birchrise_boars_3 → 39 m from oak_webs | travel 84%, rest 16% | exploring The Forgotten Graves; resting |
| A | 02exp | explorer | 26:37 | 2:34 | Emberford Bridge → 64 m from The Ashcloak Stockade | travel 100% | boar_trouble: go to the Thornboar camp; exploring Emberrun Ford |
| A | 00com | completionist | 25:54 | 2:29 | The Logging Camp → 33 m from The Old Barrow | travel 90%, fight 7% | visiting Anwen: accept forgotten_graves; eyes_in_the_dark: kill Shade |
| A | 02exp | explorer | 22:56 | 2:16 | west_trail_glade_13 → Hearthstead | travel 100% | boar_trouble: go to the Thornboar camp; visiting Guard Tolan: hand in broken_tower |
| A | 10com | completionist | 25:47 | 2:15 | barrow_boars → 37 m from The Logging Camp | travel 89%, rest 11% | visiting Foreman Garrick: hand in tusks_and_tempers; resting |
| A | 09par | party | 15:28 | 2:13 | 62 m from oak_webs → North Gate Rocks | travel 96%, town 4% | quest business with Hollis; following a party member |
| A | 03com | completionist | 7:47 | 2:12 | west_trail_camp_11 → 56 m from North Gate Rocks | travel 96%, gather 4% | visiting Guard Tolan: hand in lay_of_the_land; visiting Tilda: accept windmill_breadcrumb |
| B | 02exp | explorer | 25:46 | 2:10 | oak_trail_view_19 → 63 m from south_road_stones_9 | travel 100% | exploring The Listening Stones; exploring Barrow Overlook |
| A | 08par | party | 15:34 | 2:05 | 60 m from oak_webs → North Gate Rocks | travel 100% | taking a party member to an NPC; watchfire: gather Coal |
| B | 06com | completionist | 14:36 | 2:04 | 50 m from meadow_flock_3 → 78 m from The Old Quarry | travel 96%, idle 4% | following a party member; helping a party member |
| B | 03com | completionist | 15:05 | 2:02 | The North Watch → 88 m from meadow_flock_4 | travel 84%, fight 16% | old_tusk: go to the Old Tusk camp; visiting Old Brannoc: accept dust_of_shadows |
| A | 00com | completionist | 23:52 | 2:02 | 42 m from The Old Barrow → The Logging Camp | travel 100% | visiting Foreman Garrick: hand in into_barrowwood |
| B | 08par | party | 27:59 | 2:00 (to the end) | Hearthstead → 34 m from Hearthstead | town 100% | waiting a party member |

Longest gap per bot: median 1:45, worst 9:55; 23 gaps over 2:00 among 24 bots.

## Quests

| quest | level | accepted | done | done at (median) |
|---|---|---|---|---|
| cold_hearth | 1 | 24 | 24 | 0:06 |
| meat_for_the_pot | 1 | 18 | 18 | 2:44 |
| broken_tower | 2 | 18 | 15 | 7:10 |
| kindling | 2 | 18 | 18 | 0:44 |
| lay_of_the_land | 2 | 15 | 14 | 9:05 |
| thin_the_herd | 2 | 24 | 23 | 4:09 |
| wool_for_winter | 2 | 18 | 16 | 5:47 |
| boar_trouble | 3 | 17 | 12 | 16:45 |
| coal_for_the_watch | 3 | 18 | 14 | 8:50 |
| eyes_in_the_dark | 3 | 17 | 10 | 13:38 |
| garricks_camp | 3 | 17 | 11 | 20:10 |
| lookouts_eye | 3 | 17 | 8 | 16:33 |
| north_watch | 3 | 17 | 17 | 10:29 |
| tusks_and_tempers | 3 | 17 | 10 | 20:20 |
| watchfire | 3 | 17 | 12 | 11:12 |
| down_to_water | 4 | 4 | 4 | 21:50 |
| dust_of_shadows | 4 | 10 | 8 | 23:08 |
| iron_for_hollis | 4 | 13 | 4 | 11:23 |
| old_tusk | 4 | 12 | 4 | 19:08 |
| pincher_supper | 4 | 4 | 2 | 23:15 |
| silk_for_sails | 4 | 11 | 6 | 26:51 |
| stolen_grain | 4 | 4 | 0 | - |
| thieves_in_fields | 4 | 4 | 2 | 24:37 |
| webs_between_roots | 4 | 7 | 4 | 25:22 |
| windmill_breadcrumb | 4 | 15 | 4 | 19:39 |
| word_for_rysa | 4 | 11 | 7 | 22:37 |
| forgotten_graves | 5 | 1 | 0 | - |
| hearth_stirs | 5 | 3 | 0 | - |
| into_barrowwood | 5 | 9 | 1 | 25:54 |
| know_the_vale | 5 | 10 | 0 | - |
| shells_for_nets | 5 | 4 | 3 | 24:57 |
| word_from_east | 5 | 4 | 0 | - |
| drowned_light | 6 | 2 | 2 | 27:44 |

