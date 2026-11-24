## Machine and tick

| run | report | load: start / mean / max (cores) | server tick: mean / p95 / p99 / worst | worst minute's p95 | counts (p95 < 50 ms) |
|---|---|---|---|---|---|
| A | r2-before-a-mix-12x30.json.gz | 5.6 / 6.6 / 17.4 (16) | 0.45 / 1.00 / 2.50 / 67 ms | 2.75 ms | yes |
| B | r2-before-b-mix-12x30.json.gz | 10.3 / 8.3 / 16.2 (16) | 0.47 / 1.25 / 2.75 / 58 ms | 1.75 ms | yes |

## Per player type (medians; deaths per hour pooled over the bot-hours)

| brain (bots) | first kill | level 2 | level 3 | level 4 | level 5 | level at end | quests done | gear reward | first upgrade | median gap | longest gap | worst gap | travel % | fight % | deaths | bot-hours | deaths/h | lost kills % | money |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| completionist (8) | 1:14 | 2:47 | 9:02 | 16:04 | - | 4 | 15 | 4:31 | 3:53 | 12 s | 3:33 | 7:30 | 68% | 14% | 10 | 4.00 | 2.5 | 17% | 1733.5 |
| casual (2) | 2:03 | 3:25 | 11:19 | - | - | 3 | 8.5 | 5:20 | 7:10 | 8 s | 1:19 | 1:36 | 65% | 21% | 1 | 0.34 | 3.0 | 0% | 876.5 |
| explorer (2) | 3:48 | 4:39 | 11:22 | 21:01 | - | 4 | 13 | 9:47 | 8:36 | 13 s | 2:38 | 3:03 | 82% | 12% | 2 | 1.00 | 2.0 | 7% | 1358 |
| social (2) | 1:13 | 2:53 | 8:56 | 17:46 | - | 4.5 | 17.5 | 5:17 | 5:17 | 9 s | 2:14 | 2:56 | 68% | 19% | 2 | 1.00 | 2.0 | 21% | 2219.5 |
| party (4) | 1:21 | 2:21 | 8:52 | 18:13 | - | 4 | 13 | 3:35 | 3:35 | 15 s | 3:54 | 5:05 | 73% | 7% | 0 | 2.00 | 0.0 | 2% | 959.5 |
| grinder (6) | 0:33 | 2:58 | 8:13 | 18:43 | - | 4 | 2 | 3:07 | 3:25 | 10 s | 2:09 | 6:19 | 24% | 66% | 35 | 3.00 | 11.7 | 20% | 269.5 |
| all (24) | 1:16 | 2:51 | 8:59 | 17:22 | - | 4 | 13.5 | 4:00 | 4:02 | 10 s | 2:46 | 7:30 | 59% | 27% | 50 | 11.34 | 4.4 | 17% | 1080.5 |

## Per class

| class (bots) | first kill | level 2 | level 3 | level 4 | level 5 | level at end | quests done | gear reward | first upgrade | median gap | longest gap | worst gap | travel % | fight % | deaths | bot-hours | deaths/h | lost kills % | money |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Hunter (4) | 2:12 | 3:26 | 10:08 | 21:01 | - | 4 | 12.5 | 6:29 | 5:57 | 14 s | 2:54 | 5:03 | 77% | 12% | 2 | 2.00 | 1.0 | 5% | 1010 |
| Mage (4) | 1:24 | 3:21 | 9:38 | - | - | 3.5 | 5 | 4:03 | 11:48 | 7 s | 1:29 | 6:11 | 38% | 47% | 11 | 1.34 | 8.2 | 11% | 572.5 |
| Paladin (4) | 0:30 | 2:32 | 8:09 | 19:02 | - | 4 | 2 | 2:46 | 2:32 | 10 s | 2:09 | 6:19 | 22% | 71% | 25 | 2.00 | 12.5 | 23% | 269.5 |
| Priest (4) | 1:12 | 3:15 | 9:08 | 15:38 | - | 4 | 15 | 5:00 | 5:00 | 9 s | 2:37 | 4:35 | 65% | 16% | 3 | 2.00 | 1.5 | 21% | 1859.5 |
| Rogue (4) | 1:21 | 2:21 | 8:15 | 17:16 | - | 4 | 14.5 | 3:38 | 3:23 | 15 s | 4:09 | 7:30 | 74% | 8% | 3 | 2.00 | 1.5 | 9% | 1221.5 |
| Warrior (4) | 1:15 | 2:47 | 9:15 | 16:37 | - | 4 | 15.5 | 4:53 | 3:53 | 15 s | 3:20 | 4:17 | 68% | 16% | 6 | 2.00 | 3.0 | 17% | 1787.5 |
| all (24) | 1:16 | 2:51 | 8:59 | 17:22 | - | 4 | 13.5 | 4:00 | 4:02 | 10 s | 2:46 | 7:30 | 59% | 27% | 50 | 11.34 | 4.4 | 17% | 1080.5 |

## Deaths, each one

| run | bot | brain | class | at | lv | killer (gap) | engaged at death | day/night | health at pull | pulled | where | quest area | road | cause |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| A | 06com | completionist | Warrior | 11:15 | 3 | Shade 1 (-2) | Shade 1×3, Shade 2×2 | night | 9% | Shade 2 (attacked) | Hearthstead | Hearthstead | yes | night |
| A | 03com | completionist | Rogue | 18:59 | 3 | Ashcloak Bandit 4 (+1) | Ashcloak Bandit 4×2 | day | 79% | Ashcloak Bandit 4 | mere_ashcloaks | ashcloak camp | no | adds |
| A | 00com | completionist | Warrior | 23:45 | 4 | Thornboar 3 (-1) | Thornboar 3 | day | 16% | Thornboar 3 (attacked) | 30 m from logging_trail_well_14 | thornboar camp | no | jumped |
| B | 00com | completionist | Warrior | 9:09 | 2 | Thornboar 2 (+0) | Thornboar 2 | day | 11% | Thornboar 2 | 50 m from logging_trail_well_14 | thornboar camp | no | low pull |
| B | 00com | completionist | Warrior | 11:05 | 3 | Shade 1 (-2) | Shade 1×4, Shade 2×4 | night | 100% | Shade 2 | 87 m from Hearthstead | outside | yes | night |
| B | 06com | completionist | Warrior | 11:19 | 3 | Shade 2 (-1) | Shade 2 | night | 0% | Shade 2 | 95 m from Hearthstead | grazer camp | no | night |
| B | 03com | completionist | Rogue | 11:42 | 3 | Shade 2 (-1) | Shade 2×2, Shade 1×3 | night | 29% | Shade 2 (attacked) | Hearthstead | Hearthstead | yes | night |
| B | 10com | completionist | Priest | 18:38 | 4 | Barrow Warden 6 (+2) | Barrow Warden 6 | day | 61% | Barrow Warden 6 (attacked) | The Old Barrow | into_barrowwood | no | over-level |
| B | 00com | completionist | Warrior | 19:17 | 4 | Barrow Warden 6 (+2) | Barrow Warden 6 | day | 79% | Barrow Warden 6 (attacked) | The Old Barrow | into_barrowwood | no | over-level |
| B | 03com | completionist | Rogue | 20:59 | 4 | Old Tusk 4 (+0) | Old Tusk 4 | day | 100% | Old Tusk 4 | 74 m from logging_trail_cairn_15 | old_tusk camp | no | one-on-one |
| B | 05cas | casual | Mage | 10:14 | 2 | Shade 2 (+0) | Shade 2×2 | night | 11% | Shade 2 (attacked) | 68 m from Hearthstead | Hearthstead | yes | night |
| A | 02exp | explorer | Hunter | 23:03 | 4 | Websnare 4 (+0) | Websnare 4 | day | 29% | Websnare 4 (attacked) | The Barrowwood Graves | forgotten_graves | no | jumped |
| A | 02exp | explorer | Hunter | 27:52 | 4 | Ashcloak Bandit 4 (+0) | Ashcloak Bandit 4 | night | 4% | Ashcloak Bandit 4 (attacked) | mere_ashcloaks | ashcloak camp | no | jumped |
| A | 04soc | social | Priest | 16:17 | 3 | Ashcloak Bandit 3 (+0) | Ashcloak Bandit 3 | day | 100% | Ashcloak Bandit 3 | mere_ashcloaks | ashcloak camp | no | one-on-one |
| B | 04soc | social | Priest | 19:57 | 4 | Barrow Warden 6 (+2) | Barrow Warden 6 | day | 61% | Barrow Warden 6 (attacked) | The Old Barrow | into_barrowwood | no | over-level |
| A | 01gri | grinder | Paladin | 1:45 | 1 | Grazer 2 (+1) | Grazer 2 | day | 38% | Grazer 2 | 70 m from south_road_tower_7 | grazer camp | no | low pull |
| A | 11gri | grinder | Mage | 4:41 | 2 | Grazer 2 (+0) | Grazer 2 | day | 46% | Grazer 2 | 64 m from north_road_glade_1 | grazer camp | no | low pull |
| A | 07gri | grinder | Paladin | 6:24 | 2 | Grazer 2 (+0) | Grazer 2 | day | 27% | Grazer 2 | 78 m from The Old Quarry | grazer camp | no | low pull |
| A | 01gri | grinder | Paladin | 6:51 | 2 | Grazer 2 (+0) | Grazer 2 | day | 8% | Grazer 2 | 104 m from Hearthstead | grazer camp | no | low pull |
| A | 11gri | grinder | Mage | 7:25 | 2 | Grazer 1 (-1) | Grazer 1 | day | 11% | Grazer 1 | 108 m from east_road_glade_6 | grazer camp | no | low pull |
| A | 07gri | grinder | Paladin | 9:19 | 3 | Grazer 2 (-1) | Grazer 2 | day | 19% | Grazer 2 | 106 m from south_road_tower_7 | grazer camp | no | low pull |
| A | 01gri | grinder | Paladin | 9:37 | 3 | Grazer 2 (-1) | Grazer 2 | day | 18% | Grazer 2 | 45 m from south_road_tower_7 | grazer camp | no | low pull |
| A | 11gri | grinder | Mage | 9:40 | 3 | Grazer 2 (-1) | Grazer 2 | night | 12% | Grazer 2 | 74 m from North Gate Rocks | grazer camp | no | low pull |
| A | 07gri | grinder | Paladin | 10:51 | 3 | Shade 2 (-1) | Shade 2 | night | 33% | Shade 2 | 66 m from east_road_glade_6 | outside | no | night |
| A | 11gri | grinder | Mage | 11:17 | 3 | Grazer 1 (-2) | Grazer 1 | night | 1% | Grazer 1 | 57 m from west_trail_camp_11 | grazer camp | no | low pull |
| A | 01gri | grinder | Paladin | 11:21 | 3 | Shade 2 (-1) | Shade 2 | night | 29% | Shade 2 | 48 m from south_road_tower_7 | grazer camp | no | night |
| A | 01gri | grinder | Paladin | 11:43 | 3 | Shade 1 (-2) | Shade 1×3, Shade 2 | night | 35% | Shade 1 (attacked) | Hearthstead | Hearthstead | yes | night |
| A | 07gri | grinder | Paladin | 12:09 | 3 | Thornboar 1 (-2) | Thornboar 1 | night | 3% | Thornboar 1 | 100 m from east_road_glade_6 | grazer camp | no | low pull |
| A | 01gri | grinder | Paladin | 14:41 | 3 | Shade 2 (-1) | Shade 2 | night | 14% | Shade 2 | 62 m from north_road_glade_1 | grazer camp | no | night |
| A | 07gri | grinder | Paladin | 15:34 | 3 | Grazer 2 (-1) | Grazer 2 | day | 9% | Grazer 2 | 54 m from north_road_glade_1 | grazer camp | no | low pull |
| A | 01gri | grinder | Paladin | 15:58 | 3 | Thornboar 2 (-1) | Thornboar 2 | day | 26% | Thornboar 2 | 86 m from east_road_glade_6 | thornboar camp | no | low pull |
| A | 11gri | grinder | Mage | 16:35 | 4 | Grazer 2 (-2) | Grazer 2 | day | 5% | Grazer 2 | meadow_flock_1 | grazer camp | no | low pull |
| A | 07gri | grinder | Paladin | 18:10 | 3 | Grazer 1 (-2) | Grazer 1 | day | 7% | Grazer 1 (attacked) | 69 m from east_road_wood_5 | grazer camp | no | jumped |
| A | 01gri | grinder | Paladin | 29:20 | 4 | Thornboar 3 (-1) | Thornboar 3 | night | 15% | Thornboar 3 | logging_trail_well_14 | thornboar camp | no | low pull |
| B | 11gri | grinder | Mage | 4:54 | 2 | Thornboar 2 (+0) | Thornboar 2 | day | 29% | Thornboar 2 | 76 m from east_road_glade_6 | thornboar camp | no | low pull |
| B | 07gri | grinder | Paladin | 5:26 | 2 | Grazer 2 (+0) | Grazer 2 | day | 21% | Grazer 2 | meadow_flock_6 | grazer camp | no | low pull |
| B | 01gri | grinder | Paladin | 6:07 | 2 | Grazer 2 (+0) | Grazer 2 | day | 6% | Grazer 2 | 74 m from west_trail_camp_11 | grazer camp | no | low pull |
| B | 11gri | grinder | Mage | 6:13 | 2 | Grazer 2 (+0) | Grazer 2 | day | 16% | Grazer 2 | 81 m from south_road_tower_7 | grazer camp | no | low pull |
| B | 07gri | grinder | Paladin | 7:11 | 2 | Grazer 2 (+0) | Grazer 2 | day | 12% | Grazer 2 | 52 m from north_road_glade_1 | grazer camp | no | low pull |
| B | 11gri | grinder | Mage | 7:41 | 2 | Grazer 2 (+0) | Grazer 2 | day | 41% | Grazer 2 | 33 m from south_road_tower_7 | grazer camp | no | low pull |
| B | 01gri | grinder | Paladin | 8:31 | 2 | Grazer 1 (-1) | Grazer 1 | day | 15% | Grazer 1 | 35 m from west_trail_camp_11 | grazer camp | no | low pull |
| B | 07gri | grinder | Paladin | 9:18 | 3 | Grazer 1 (-2) | Grazer 1 | day | 9% | Grazer 1 | 109 m from Hearthstead | grazer camp | no | low pull |
| B | 11gri | grinder | Mage | 9:27 | 3 | Grazer 2 (-1) | Grazer 2 | day | 37% | Grazer 2 | 58 m from south_road_tower_7 | grazer camp | no | low pull |
| B | 01gri | grinder | Paladin | 9:55 | 3 | Grazer 2 (-1) | Grazer 2 | day | 30% | Grazer 2 | 48 m from North Gate Rocks | grazer camp | no | low pull |
| B | 07gri | grinder | Paladin | 10:33 | 3 | Shade 1 (-2) | Shade 1 | night | 6% | Shade 1 | 126 m from east_road_wood_5 | grazer camp | no | night |
| B | 11gri | grinder | Mage | 12:28 | 3 | Grazer 1 (-2) | Grazer 1 | night | 11% | Grazer 1 | meadow_flock_6 | grazer camp | no | low pull |
| B | 07gri | grinder | Paladin | 12:40 | 3 | Grazer 2 (-1) | Grazer 2 | night | 8% | Grazer 2 | 99 m from east_road_glade_6 | grazer camp | no | low pull |
| B | 01gri | grinder | Paladin | 13:01 | 3 | Shade 2 (-1) | Shade 2 | night | 2% | Shade 2 | 96 m from east_road_glade_6 | grazer camp | no | night |
| B | 07gri | grinder | Paladin | 20:24 | 3 | Grazer 1 (-2) | Grazer 1 | day | 5% | Grazer 1 | 114 m from east_road_glade_6 | grazer camp | no | low pull |
| B | 01gri | grinder | Paladin | 21:00 | 4 | Grazer 2 (-2) | Grazer 2 | day | 20% | Grazer 2 | 49 m from north_road_glade_1 | grazer camp | no | low pull |

| brain | bot-hours | deaths | per hour | over-level | night | adds | jumped | low pull | one-on-one | deaths at night at level 1-4 on a road or in a quest area | of those, to a Shade |
|---|---|---|---|---|---|---|---|---|---|---|---|
| completionist | 4.00 | 10 | 2.5 | 2 | 4 | 1 | 1 | 1 | 1 | 4 | 4 |
| casual | 0.34 | 1 | 3.0 | 0 | 1 | 0 | 0 | 0 | 0 | 1 | 1 |
| explorer | 1.00 | 2 | 2.0 | 0 | 0 | 0 | 2 | 0 | 0 | 1 | 0 |
| social | 1.00 | 2 | 2.0 | 1 | 0 | 0 | 0 | 0 | 1 | 0 | 0 |
| party | 2.00 | 0 | 0.0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| grinder | 3.00 | 35 | 11.7 | 0 | 6 | 0 | 1 | 28 | 0 | 11 | 5 |
| all | 11.34 | 50 | 4.4 | 3 | 11 | 1 | 4 | 29 | 2 | 17 | 10 |

One-on-one losses at the same level or below (for the class lane):

- Priest 3 (social) to Ashcloak Bandit 3, pulled at 100% health, day, mere_ashcloaks
- Rogue 4 (completionist) to Old Tusk 4, pulled at 100% health, day, 74 m from logging_trail_cairn_15

## How the bots pull

| brain | fights | pulled under 60% health | under 40% | fights with an add | with two or more | at night |
|---|---|---|---|---|---|---|
| completionist | 279 | 7% | 4% | 9% | 6% | 38% |
| casual | 36 | 6% | 6% | 11% | 8% | 33% |
| explorer | 48 | 4% | 4% | 4% | 0% | 35% |
| social | 92 | 4% | 1% | 5% | 1% | 48% |
| party | 90 | 2% | 0% | 4% | 1% | 29% |
| grinder | 850 | 22% | 15% | 4% | 2% | 36% |
| all | 1395 | 16% | 10% | 6% | 3% | 37% |

## Time and travel

| brain | travel | fight | gather | town | rest | loot | waiting for spawns | dead | idle + stuck |
|---|---|---|---|---|---|---|---|---|---|
| completionist (8) | 68% | 14% | 3% | 1% | 0% | 1% | 12% | 0% | 1% |
| casual (2) | 65% | 21% | 6% | 1% | 0% | 5% | 0% | 0% | 2% |
| explorer (2) | 82% | 12% | 2% | 0% | 0% | 2% | 0% | 0% | 1% |
| social (2) | 68% | 19% | 2% | 1% | 2% | 3% | 4% | 0% | 1% |
| party (4) | 73% | 7% | 2% | 7% | 1% | 0% | 4% | 0% | 6% |
| grinder (6) | 24% | 66% | 0% | 0% | 4% | 4% | 0% | 1% | 1% |
| all (24) | 59% | 27% | 2% | 2% | 1% | 2% | 5% | 0% | 2% |

Travel by purpose (all but grinders), share of travel time:

| what the bot was on its way to | minutes | share |
|---|---|---|
| boar_trouble: go to the Thornboar camp | 33.6 | 9% |
| visiting Anwen: accept forgotten_graves | 28.6 | 8% |
| taking a party member to an NPC | 19.8 | 6% |
| following a party member | 18.7 | 5% |
| visiting Guard Tolan: hand in broken_tower | 11.8 | 3% |
| visiting Sergeant Holt: hand in boar_trouble | 11.1 | 3% |
| visiting Sergeant Holt: hand in watchfire | 10.6 | 3% |
| old_tusk: go to the Old Tusk camp | 9.8 | 3% |
| visiting Sergeant Holt: hand in eyes_in_the_dark | 9.6 | 3% |
| north_watch: explore the North Watch beacon | 9.5 | 3% |
| visiting Captain Maren: hand in thin_the_herd | 8.0 | 2% |
| lay_of_the_land: explore the Trapper's Camp | 7.9 | 2% |
| coal_for_the_watch: gather Coal | 7.9 | 2% |
| visiting Tilda: hand in meat_for_the_pot | 7.8 | 2% |
| lay_of_the_land: explore the Old Quarry | 7.3 | 2% |
| broken_tower: explore the Broken Tower | 7.2 | 2% |
| visiting Hollis: accept iron_for_hollis | 6.5 | 2% |
| visiting Tilda: hand in wool_for_winter | 6.4 | 2% |
| visiting Brother Calder: hand in forgotten_graves | 6.0 | 2% |
| eyes_in_the_dark: look for Shade in the dark | 5.3 | 2% |
| with a party member | 5.3 | 2% |
| windmill_breadcrumb: talk to Farmer Ostrin | 5.2 | 1% |

The longest runs (unbroken travel), by route:

| from | to | for | longest | median | times | bots |
|---|---|---|---|---|---|---|
| 47 m from The Logging Camp | The Barrowwood Graves | helping a party member | 8:46 | 8:46 | 1 | 1 |
| 50 m from The Logging Camp | The Barrowwood Graves | defending against Websnare | 8:44 | 8:44 | 1 | 1 |
| 110 m from Hearthstead | 34 m from Hearthstead | with a party member | 8:32 | 8:32 | 1 | 1 |
| Hearthstead | The Logging Camp | eyes_in_the_dark: kill Shade | 7:51 | 7:51 | 1 | 1 |
| 45 m from west_trail_camp_11 | 77 m from logging_trail_cairn_15 | old_tusk: kill Old Tusk | 6:55 | 6:55 | 1 | 1 |
| Hearthstead | logging_trail_well_14 | defending against Thornboar | 6:38 | 6:38 | 1 | 1 |
| 47 m from south_road_ruin_8 | Beacon Rocks | coal_for_the_watch: gather Coal | 6:33 | 6:33 | 1 | 1 |
| 84 m from west_trail_camp_11 | barrow_boars | boar_trouble: kill Thornboar | 6:23 | 6:23 | 1 | 1 |
| 107 m from east_road_glade_6 | 67 m from north_road_glade_1 | looting | 6:10 | 6:10 | 1 | 1 |
| Hearthstead | 72 m from logging_trail_cairn_15 | old_tusk: wait for a Old Tusk to spawn | 5:37 | 5:37 | 1 | 1 |
| The Hollow Mines | logging_trail_well_14 | boar_trouble: kill Thornboar | 5:18 | 5:18 | 1 | 1 |
| 110 m from Hearthstead | 60 m from Hearthstead | eyes_in_the_dark: kill Shade | 5:17 | 5:17 | 1 | 1 |

## Reward gaps over 2:00

| run | bot | brain | from | gap | where: from, to | mostly | doing |
|---|---|---|---|---|---|---|---|
| B | 03com | completionist | 22:29 | 7:30 (to the end) | 56 m from North Gate Rocks → 72 m from birchrise_boars_1 | wait_spawn 85%, travel 13% | old_tusk: wait for a Old Tusk to spawn; old_tusk: go to the Old Tusk camp |
| A | 01gri | grinder | 19:33 | 6:19 | 117 m from east_road_glade_6 → 59 m from north_road_glade_1 | travel 96%, fight 4% | going to grind; grinding |
| A | 11gri | grinder | 20:21 | 6:11 | 108 m from east_road_glade_6 → 61 m from north_road_glade_1 | travel 99%, fight 1% | going to grind; grinding |
| B | 09par | party | 24:54 | 5:05 (to the end) | logging_trail_well_14 → 71 m from birchrise_boars_1 | idle 92%, travel 8% | with a party member; following a party member |
| B | 08par | party | 24:56 | 5:03 (to the end) | logging_trail_well_14 → 72 m from birchrise_boars_1 | wait_spawn 92%, travel 7% | tusks_and_tempers: wait for a Old Tusk to spawn for Boar Tusk; tusks_and_tempers: go to the Old Tusk camp for Boar Tusk |
| B | 09par | party | 10:16 | 4:35 | 95 m from The Old Quarry → North Gate Rocks | town 60%, travel 40% | with a party member; following a party member |
| A | 10com | completionist | 23:11 | 4:35 | The North Watch → 72 m from birchrise_boars_1 | wait_spawn 85%, travel 13% | old_tusk: wait for a Old Tusk to spawn; old_tusk: go to the Old Tusk camp |
| B | 00com | completionist | 23:25 | 4:17 | oak_trail_view_19 → 72 m from logging_trail_cairn_15 | wait_spawn 63%, travel 35% | old_tusk: wait for a Old Tusk to spawn; old_tusk: go to the Old Tusk camp |
| B | 06com | completionist | 26:06 | 3:53 (to the end) | logging_trail_well_14 → 72 m from birchrise_boars_1 | wait_spawn 91%, travel 9% | old_tusk: wait for a Old Tusk to spawn; old_tusk: go to the Old Tusk camp |
| A | 03com | completionist | 14:34 | 3:13 | Hearthstead → 43 m from south_road_ruin_8 | travel 97%, fight 3% | boar_trouble: go to the Thornboar camp; boar_trouble: kill Thornboar |
| B | 02exp | explorer | 21:52 | 3:03 | birchrise_boars_2 → 82 m from west_trail_wood_12 | travel 100% | visiting Hollis: accept word_from_east; visiting Anwen: accept forgotten_graves |
| B | 00com | completionist | 19:06 | 3:02 | The Old Barrow → 49 m from Barrowwood | travel 94%, fight 3% | visiting Anwen: accept forgotten_graves; forgotten_graves: explore the Forgotten Graves |
| B | 09par | party | 18:02 | 3:01 | logging_trail_well_14 → 45 m from The Old Barrow | travel 95%, idle 3% | following a party member; with a party member |
| A | 04soc | social | 15:58 | 2:56 | mere_ashcloaks → 50 m from west_trail_camp_11 | travel 83%, fight 17% | boar_trouble: go to the Thornboar camp; thieves_in_fields: kill Ashcloak Bandit |
| A | 04soc | social | 27:08 | 2:50 (to the end) | logging_trail_well_14 → 72 m from birchrise_boars_1 | wait_spawn 83%, travel 9% | old_tusk: wait for a Old Tusk to spawn; old_tusk: go to the Old Tusk camp |
| A | 06com | completionist | 27:11 | 2:48 (to the end) | 72 m from birchrise_boars_1 → 72 m from birchrise_boars_1 | wait_spawn 100% | old_tusk: wait for a Old Tusk to spawn |
| A | 08par | party | 19:34 | 2:45 | Fisher's Landing → 61 m from The Old Barrow | travel 100% | taking a party member to an NPC |
| B | 08par | party | 18:13 | 2:44 | 30 m from The North Watch → 39 m from west_trail_cairn | travel 100% | visiting Anwen: accept forgotten_graves; visiting Old Brannoc: accept know_the_vale |
| A | 06com | completionist | 24:29 | 2:42 | logging_trail_well_14 → 72 m from birchrise_boars_1 | wait_spawn 84%, travel 12% | old_tusk: wait for a Old Tusk to spawn; old_tusk: go to the Old Tusk camp |
| A | 09par | party | 19:35 | 2:41 | Fisher's Landing → 58 m from The Old Barrow | travel 100% | quest business with Anwen |
| A | 09par | party | 27:20 | 2:38 (to the end) | The North Watch → The North Watch | town 100% | with a party member |
| A | 08par | party | 15:48 | 2:36 | 78 m from oak_webs → 45 m from south_road_stones_9 | travel 100% | taking a party member to an NPC |
| B | 02exp | explorer | 14:36 | 2:28 | The Hollow Mines → 45 m from Hearthstead | travel 100% | visiting Hollis: hand in iron_for_hollis |
| A | 08par | party | 27:34 | 2:25 (to the end) | south_road_stones_9 → 80 m from oak_webs | travel 100% | visiting Rysa: hand in word_for_rysa |
| B | 06com | completionist | 20:36 | 2:24 | barrow_boars → east_road_glade_6 | travel 100% | boar_trouble: go to the Thornboar camp; lay_of_the_land: explore the Wrecked Caravan |
| B | 07gri | grinder | 23:43 | 2:17 | 54 m from north_road_glade_1 → 56 m from North Gate Rocks | travel 67%, fight 33% | going to grind; grinding |
| B | 10com | completionist | 27:42 | 2:17 (to the end) | 73 m from birchrise_boars_1 → 73 m from birchrise_boars_1 | wait_spawn 96%, idle 4% | old_tusk: wait for a Old Tusk to spawn |
| B | 00com | completionist | 27:43 | 2:16 (to the end) | 72 m from logging_trail_cairn_15 → 72 m from logging_trail_cairn_15 | wait_spawn 100% | old_tusk: wait for a Old Tusk to spawn |
| A | 02exp | explorer | 22:52 | 2:13 | barrow_webs → The Old Barrow | travel 93%, fight 4% | visiting Brother Calder: hand in forgotten_graves; defending against Websnare |
| A | 10com | completionist | 19:46 | 2:13 | The Old Barrow → The North Watch | travel 100% | visiting Sergeant Holt: hand in watchfire |
| A | 04soc | social | 23:55 | 2:12 | The Old Barrow → The North Watch | travel 100% | visiting Sergeant Holt: hand in watchfire |
| A | 02exp | explorer | 20:42 | 2:09 | 36 m from The Old Barrow → barrow_webs | travel 92%, fight 8% | exploring The Hollow Oak; coal_for_the_watch: gather Coal |
| A | 04soc | social | 8:33 | 2:06 | The North Watch → 58 m from Hearthstead | travel 96%, fight 4% | visiting Hollis: accept iron_for_hollis; eyes_in_the_dark: look for Shade in the dark |
| A | 07gri | grinder | 26:16 | 2:02 | meadow_flock_1 → 45 m from meadow_flock_1 | travel 96%, idle 4% | going to grind |
| B | 08par | party | 9:42 | 2:01 | 55 m from oak_webs → 35 m from Hearthstead | travel 100% | taking a party member to an NPC; visiting Tilda: accept windmill_breadcrumb |

Longest gap per bot: median 2:46, worst 7:30; 35 gaps over 2:00 among 24 bots.

## Quests

| quest | level | accepted | done | done at (median) |
|---|---|---|---|---|
| cold_hearth | 1 | 24 | 24 | 0:08 |
| meat_for_the_pot | 1 | 18 | 18 | 2:51 |
| broken_tower | 2 | 18 | 17 | 6:17 |
| kindling | 2 | 18 | 18 | 0:48 |
| lay_of_the_land | 2 | 17 | 6 | 23:10 |
| thin_the_herd | 2 | 24 | 24 | 4:00 |
| wool_for_winter | 2 | 18 | 17 | 5:09 |
| boar_trouble | 3 | 17 | 12 | 21:54 |
| coal_for_the_watch | 3 | 18 | 17 | 7:42 |
| eyes_in_the_dark | 3 | 17 | 15 | 13:08 |
| garricks_camp | 3 | 17 | 6 | 14:17 |
| lookouts_eye | 3 | 17 | 12 | 17:37 |
| north_watch | 3 | 18 | 17 | 8:58 |
| tusks_and_tempers | 3 | 6 | 1 | 25:24 |
| watchfire | 3 | 17 | 14 | 18:13 |
| down_to_water | 4 | 11 | 1 | 19:35 |
| dust_of_shadows | 4 | 14 | 13 | 13:58 |
| iron_for_hollis | 4 | 17 | 4 | 19:29 |
| old_tusk | 4 | 12 | 2 | 24:13 |
| pincher_supper | 4 | 1 | 0 | - |
| silk_for_sails | 4 | 6 | 0 | - |
| stolen_grain | 4 | 11 | 0 | - |
| thieves_in_fields | 4 | 11 | 0 | - |
| webs_between_roots | 4 | 3 | 1 | 28:56 |
| windmill_breadcrumb | 4 | 17 | 11 | 15:33 |
| word_for_rysa | 4 | 6 | 3 | 15:33 |
| forgotten_graves | 5 | 15 | 9 | 21:09 |
| hearth_stirs | 5 | 2 | 1 | 28:54 |
| into_barrowwood | 5 | 1 | 0 | - |
| know_the_vale | 5 | 14 | 0 | - |
| shells_for_nets | 5 | 1 | 0 | - |
| word_from_east | 5 | 3 | 0 | - |

