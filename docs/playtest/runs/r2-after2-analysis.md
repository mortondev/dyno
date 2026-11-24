## Machine and tick

| run | report | load: start / mean / max (cores) | server tick: mean / p95 / p99 / worst | worst minute's p95 | counts (p95 < 50 ms) |
|---|---|---|---|---|---|
| A | r2-after2-a-mix-12x30.json.gz | 10.3 / 10.4 / 17.9 (16) | 0.47 / under 1.75 / not measured / 34 ms | 1.75 ms | yes |
| B | r2-after2-b-mix-12x30.json.gz | 15.7 / 14.1 / 27.9 (16) | 0.54 / under 3.00 / not measured / 31 ms | 3.00 ms | yes |

## Per player type (medians; deaths per hour pooled over the bot-hours)

| brain (bots) | first kill | level 2 | level 3 | level 4 | level 5 | level at end | quests done | gear reward | first upgrade | median gap | longest gap | worst gap | travel % | fight % | deaths | bot-hours | deaths/h | lost kills % | money |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| completionist (8) | 1:10 | 2:18 | 8:10 | 15:09 | 28:44 | 5 | 18.5 | 3:31 | 3:30 | 11 s | 2:35 | 15:48 | 66% | 19% | 7 | 4.00 | 1.8 | 1% | 1924 |
| casual (2) | 1:37 | 2:50 | - | - | - | 2.5 | 6 | 4:41 | 4:40 | 7 s | 0:40 | 0:44 | 60% | 24% | 0 | 0.22 | 0.0 | 0% | 450 |
| explorer (2) | 3:22 | 4:06 | 10:10 | 23:51 | - | 4 | 13.5 | 7:56 | 7:56 | 15 s | 1:46 | 1:47 | 77% | 16% | 4 | 1.00 | 4.0 | 2% | 1006 |
| social (2) | 1:05 | 2:17 | 8:42 | 16:01 | - | 4 | 17.5 | 3:47 | 3:48 | 9 s | 2:18 | 2:27 | 74% | 18% | 2 | 1.00 | 2.0 | 1% | 1487.5 |
| party (4) | 1:16 | 2:25 | 16:44 | - | - | 3 | 11 | 4:16 | 4:17 | 10 s | 8:02 | 15:41 | 47% | 7% | 0 | 2.00 | 0.0 | 0% | 778.5 |
| grinder (6) | 0:25 | 2:15 | 7:17 | 17:15 | - | 4 | 2 | 2:31 | 2:31 | 8 s | 2:07 | 5:05 | 25% | 64% | 32 | 3.00 | 10.7 | 3% | 173 |
| all (24) | 1:09 | 2:24 | 8:29 | 16:08 | - | 4 | 13 | 3:33 | 3:32 | 10 s | 2:25 | 15:48 | 54% | 29% | 45 | 11.21 | 4.0 | 2% | 1089 |

## Per class

| class (bots) | first kill | level 2 | level 3 | level 4 | level 5 | level at end | quests done | gear reward | first upgrade | median gap | longest gap | worst gap | travel % | fight % | deaths | bot-hours | deaths/h | lost kills % | money |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Hunter (4) | 1:56 | 3:07 | 10:10 | 24:15 | - | 4 | 13 | 5:52 | 5:52 | 12 s | 3:10 | 10:52 | 63% | 13% | 4 | 2.00 | 2.0 | 1% | 1089 |
| Mage (4) | 1:33 | 2:50 | 7:25 | - | - | 3.5 | 3 | 3:35 | 4:40 | 6 s | 1:50 | 3:11 | 40% | 46% | 6 | 1.22 | 4.9 | 2% | 212.5 |
| Paladin (4) | 0:24 | 2:06 | 7:18 | 17:42 | - | 4 | 2 | 2:31 | 2:31 | 9 s | 1:16 | 5:05 | 20% | 71% | 26 | 2.00 | 13.0 | 3% | 173 |
| Priest (4) | 1:11 | 2:17 | 8:42 | 15:40 | - | 4 | 18 | 3:38 | 3:38 | 9 s | 2:18 | 2:52 | 73% | 18% | 2 | 2.00 | 1.0 | 1% | 1487.5 |
| Rogue (4) | 1:15 | 2:27 | 10:29 | - | - | 3.5 | 13 | 4:24 | 4:24 | 10 s | 3:36 | 15:41 | 60% | 10% | 1 | 2.00 | 0.5 | 0% | 1194.5 |
| Warrior (4) | 1:10 | 2:23 | 8:09 | 15:37 | 29:05 | 5 | 19 | 3:31 | 3:25 | 12 s | 2:59 | 15:48 | 59% | 21% | 6 | 2.00 | 3.0 | 2% | 2019 |
| all (24) | 1:09 | 2:24 | 8:29 | 16:08 | - | 4 | 13 | 3:33 | 3:32 | 10 s | 2:25 | 15:48 | 54% | 29% | 45 | 11.21 | 4.0 | 2% | 1089 |

## Deaths, each one

| run | bot | brain | class | at | lv | killer (gap) | engaged at death | day/night | health at pull | pulled | where | quest area | road | cause |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| A | 03com | completionist | Rogue | 20:33 | 4 | Websnare 4 (+0) | Silkmother 4 | day | 12% | Websnare 4 (attacked) | The Hollow Oak | websnare camp | no | jumped |
| A | 00com | completionist | Warrior | 20:45 | 4 | Websnare 5 (+1) | Silkmother 5 | day | 82% | Websnare 5 (attacked) | The Hollow Oak | websnare camp | no | one-on-one |
| A | 00com | completionist | Warrior | 22:56 | 4 | Ashcloak Thief 4 (+0) | Ashcloak Thief 4 | day | 18% | Ashcloak Thief 4 | 57 m from ford_trail_ruin_20 | ashcloak_thief camp | no | low pull |
| B | 00com | completionist | Warrior | 5:32 | 2 | Thornboar 3 (+1) | Thornboar 3 | day | 12% | Thornboar 3 | 64 m from north_road_glade_1 | thornboar camp | no | low pull |
| B | 06com | completionist | Warrior | 14:49 | 3 | Thornboar 3 (+0) | Thornboar 3 | day | 4% | Thornboar 3 | 68 m from north_road_glade_1 | thornboar camp | no | low pull |
| B | 06com | completionist | Warrior | 24:25 | 4 | Websnare 4 (+0) | Websnare 4 | day | 11% | Websnare 4 | barrow_webs | websnare camp | no | low pull |
| B | 00com | completionist | Warrior | 25:00 | 4 | Mirepincher 4 (+0) | Mirepincher 4 | day | 14% | Mirepincher 4 | mere_pinchers_1 | mirepincher camp | yes | low pull |
| A | 02exp | explorer | Hunter | 13:10 | 3 | Websnare 3 (+0) | Websnare 3 | night | 7% | Websnare 3 (attacked) | The Hollow Oak | websnare camp | no | jumped |
| A | 02exp | explorer | Hunter | 20:55 | 3 | Websnare 3 (+0) | Websnare 3 | day | 2% | Websnare 3 (attacked) | oak_webs | websnare camp | yes | jumped |
| B | 02exp | explorer | Hunter | 11:15 | 3 | Thornboar 3 (+0) | Thornboar 3 | night | 54% | Thornboar 3 | 69 m from The North Watch | thornboar camp | no | low pull |
| B | 02exp | explorer | Hunter | 27:31 | 4 | Shade 5 (+1) | Shade 5 | night | 9% | Shade 5 | The Barrowwood Graves | forgotten_graves | yes | night |
| A | 04soc | social | Priest | 11:47 | 3 | Thornboar 3 (+0) | Thornboar 3 | night | 55% | Thornboar 3 | 74 m from Beacon Rocks | thornboar camp | no | low pull |
| A | 04soc | social | Priest | 18:25 | 4 | Ashcloak Thief 3 (-1) | Ashcloak Thief 3 | day | 36% | Ashcloak Thief 3 | 39 m from ford_trail_ruin_20 | ashcloak_thief camp | no | low pull |
| A | 01gri | grinder | Paladin | 3:43 | 2 | Grazer 1 (-1) | Grazer 1 | day | 8% | Grazer 1 | 83 m from north_road_glade_1 | grazer camp | no | low pull |
| A | 07gri | grinder | Paladin | 4:42 | 2 | Thornboar 3 (+1) | Thornboar 3 | day | 30% | Thornboar 3 | 74 m from north_road_glade_1 | grazer camp | no | low pull |
| A | 01gri | grinder | Paladin | 5:18 | 2 | Grazer 1 (-1) | Grazer 1 | day | 17% | Grazer 1 | 61 m from south_road_tower_7 | grazer camp | no | low pull |
| A | 11gri | grinder | Mage | 5:51 | 2 | Grazer 2 (+0) | Grazer 2 | day | 24% | Grazer 2 | 43 m from west_trail_camp_11 | grazer camp | no | low pull |
| A | 07gri | grinder | Paladin | 5:57 | 2 | Grazer 2 (+0) | Grazer 2 | day | 18% | Grazer 2 | 71 m from east_road_wood_5 | grazer camp | no | low pull |
| A | 01gri | grinder | Paladin | 6:59 | 2 | Grazer 2 (+0) | Grazer 2 | day | 15% | Grazer 2 | 98 m from south_road_tower_7 | grazer camp | no | low pull |
| A | 01gri | grinder | Paladin | 8:43 | 3 | Grazer 2 (-1) | Grazer 2 | day | 14% | Grazer 2 | 96 m from south_road_tower_7 | grazer camp | no | low pull |
| A | 01gri | grinder | Paladin | 10:04 | 3 | Thornboar 2 (-1) | Thornboar 2 | night | 20% | Thornboar 2 | 65 m from west_trail_camp_11 | thornboar camp | no | low pull |
| A | 11gri | grinder | Mage | 10:53 | 3 | Grazer 2 (-1) | Grazer 2 | night | 26% | Grazer 2 | meadow_flock_6 | grazer camp | no | low pull |
| A | 01gri | grinder | Paladin | 11:55 | 3 | Grazer 2 (-1) | Grazer 2 | night | 9% | Grazer 2 | 48 m from west_trail_camp_11 | grazer camp | no | low pull |
| A | 07gri | grinder | Paladin | 12:18 | 3 | Grazer 2 (-1) | Grazer 2 | night | 6% | Grazer 2 | 84 m from south_road_tower_7 | grazer camp | no | low pull |
| A | 11gri | grinder | Mage | 12:30 | 3 | Grazer 1 (-2) | Grazer 1 | night | 29% | Grazer 1 (attacked) | 50 m from east_road_wood_5 | grazer camp | no | jumped |
| A | 01gri | grinder | Paladin | 13:37 | 3 | Grazer 1 (-2) | Grazer 1 | night | 8% | Grazer 1 | 38 m from west_trail_camp_11 | grazer camp | no | low pull |
| A | 01gri | grinder | Paladin | 15:17 | 3 | Grazer 2 (-1) | Grazer 2 | day | 1% | Grazer 2 | 38 m from North Gate Rocks | grazer camp | no | low pull |
| A | 07gri | grinder | Paladin | 15:23 | 3 | Grazer 2 (-1) | Grazer 2 | day | 18% | Grazer 2 | meadow_flock_6 | grazer camp | no | low pull |
| A | 01gri | grinder | Paladin | 16:40 | 3 | Grazer 1 (-2) | Grazer 1 | day | 5% | Grazer 1 | 79 m from east_road_wood_5 | grazer camp | no | low pull |
| A | 01gri | grinder | Paladin | 22:21 | 4 | Thornboar 3 (-1) | Thornboar 3 | day | 21% | Thornboar 3 | 72 m from north_road_glade_1 | grazer camp | no | low pull |
| A | 07gri | grinder | Paladin | 27:33 | 4 | Shade 2 (-2) | Shade 2×2, Shade 1×3 | night | 6% | Shade 2 | Hearthstead | Hearthstead | yes | night |
| A | 07gri | grinder | Paladin | 28:04 | 4 | Shade 1 (-3) | Shade 1×3 | night | 18% | Shade 1 (attacked) | Hearthstead | Hearthstead | yes | night |
| B | 01gri | grinder | Paladin | 4:04 | 2 | Thornboar 1 (-1) | Thornboar 1 | day | 29% | Thornboar 1 (attacked) | 81 m from east_road_glade_6 | thornboar camp | no | jumped |
| B | 11gri | grinder | Mage | 4:23 | 2 | Grazer 2 (+0) | Grazer 2 | day | 10% | Grazer 2 | meadow_flock_3 | grazer camp | no | low pull |
| B | 01gri | grinder | Paladin | 5:09 | 2 | Grazer 2 (+0) | Grazer 2 | day | 7% | Grazer 2 | 114 m from Hearthstead | grazer camp | no | low pull |
| B | 07gri | grinder | Paladin | 5:13 | 2 | Thornboar 3 (+1) | Thornboar 3 | day | 3% | Thornboar 3 | 72 m from Beacon Rocks | thornboar camp | no | low pull |
| B | 07gri | grinder | Paladin | 6:28 | 2 | Grazer 1 (-1) | Grazer 1 | day | 6% | Grazer 1 | 86 m from south_road_tower_7 | grazer camp | no | low pull |
| B | 07gri | grinder | Paladin | 8:10 | 3 | Grazer 1 (-2) | Grazer 1 | day | 6% | Grazer 1 | 58 m from west_trail_camp_11 | grazer camp | no | low pull |
| B | 07gri | grinder | Paladin | 10:15 | 3 | Grazer 2 (-1) | Grazer 2 | night | 4% | Grazer 2 | 71 m from south_road_tower_7 | grazer camp | no | low pull |
| B | 01gri | grinder | Paladin | 10:28 | 3 | Thornboar 1 (-2) | Thornboar 1 | night | 13% | Thornboar 1 | 90 m from east_road_glade_6 | thornboar camp | no | low pull |
| B | 11gri | grinder | Mage | 11:08 | 3 | Grazer 1 (-2) | Grazer 1 | night | 5% | Grazer 1 | meadow_flock_1 | grazer camp | no | low pull |
| B | 07gri | grinder | Paladin | 11:52 | 3 | Grazer 2 (-1) | Grazer 2 | night | 1% | Grazer 2 | 70 m from south_road_tower_7 | grazer camp | no | low pull |
| B | 01gri | grinder | Paladin | 12:02 | 3 | Grazer 2 (-1) | Grazer 2 | night | 32% | Grazer 2 | 113 m from east_road_glade_6 | grazer camp | no | low pull |
| B | 11gri | grinder | Mage | 13:06 | 3 | Grazer 2 (-1) | Grazer 2 | night | 5% | Grazer 2 | 42 m from south_road_tower_7 | grazer camp | no | low pull |
| B | 01gri | grinder | Paladin | 19:26 | 4 | Grazer 2 (-2) | Grazer 2 | day | 13% | Grazer 2 | 57 m from north_road_glade_1 | grazer camp | no | low pull |

| brain | bot-hours | deaths | per hour | over-level | night | adds | jumped | low pull | one-on-one | deaths at night at level 1-4 on a road or in a quest area | of those, to a Shade |
|---|---|---|---|---|---|---|---|---|---|---|---|
| completionist | 4.00 | 7 | 1.8 | 0 | 0 | 0 | 1 | 5 | 1 | 0 | 0 |
| casual | 0.22 | 0 | 0.0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| explorer | 1.00 | 4 | 4.0 | 0 | 1 | 0 | 2 | 1 | 0 | 3 | 1 |
| social | 1.00 | 2 | 2.0 | 0 | 0 | 0 | 0 | 2 | 0 | 1 | 0 |
| party | 2.00 | 0 | 0.0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| grinder | 3.00 | 32 | 10.7 | 0 | 2 | 0 | 2 | 28 | 0 | 14 | 2 |
| all | 11.21 | 45 | 4.0 | 0 | 3 | 0 | 5 | 36 | 1 | 18 | 3 |

One-on-one losses at the same level or below (for the class lane):

- Warrior 4 (completionist) to Websnare 5, pulled at 82% health, day, The Hollow Oak

## How the bots pull

| brain | fights | pulled under 60% health | under 40% | fights with an add | with two or more | at night |
|---|---|---|---|---|---|---|
| completionist | 345 | 13% | 6% | 5% | 0% | 35% |
| casual | 30 | 0% | 0% | 0% | 0% | 0% |
| explorer | 58 | 12% | 7% | 3% | 0% | 38% |
| social | 95 | 3% | 2% | 2% | 0% | 37% |
| party | 82 | 4% | 0% | 5% | 0% | 23% |
| grinder | 810 | 22% | 12% | 2% | 1% | 29% |
| all | 1420 | 17% | 9% | 3% | 1% | 30% |

## Time and travel

| brain | travel | fight | gather | town | rest | loot | waiting for spawns | dead | idle + stuck |
|---|---|---|---|---|---|---|---|---|---|
| completionist (8) | 66% | 19% | 3% | 8% | 1% | 1% | 1% | 0% | 1% |
| casual (2) | 60% | 24% | 6% | 2% | 0% | 7% | 0% | 0% | 0% |
| explorer (2) | 77% | 16% | 1% | 1% | 1% | 2% | 0% | 0% | 1% |
| social (2) | 74% | 18% | 2% | 1% | 0% | 3% | 0% | 0% | 1% |
| party (4) | 47% | 7% | 2% | 19% | 1% | 0% | 0% | 0% | 23% |
| grinder (6) | 25% | 64% | 0% | 0% | 5% | 3% | 0% | 1% | 1% |
| all (24) | 54% | 29% | 2% | 6% | 2% | 2% | 1% | 0% | 5% |

Travel by purpose (all but grinders), share of travel time:

| what the bot was on its way to | minutes | share |
|---|---|---|
| webs_between_roots: go to the Websnare camp | 20.5 | 7% |
| following a party member | 17.2 | 5% |
| boar_trouble: go to the Thornboar camp | 12.9 | 4% |
| garricks_camp: talk to Foreman Garrick | 10.8 | 3% |
| visiting Guard Tolan: hand in broken_tower | 10.5 | 3% |
| tusks_and_tempers: go to the Thornboar camp for Boar Tusk | 10.0 | 3% |
| visiting Guard Tolan: hand in lay_of_the_land | 9.6 | 3% |
| taking a party member to an NPC | 9.5 | 3% |
| coal_for_the_watch: gather Coal | 9.1 | 3% |
| north_watch: explore the North Watch beacon | 8.7 | 3% |
| word_for_rysa: talk to Rysa | 8.4 | 3% |
| visiting Sergeant Holt: hand in boar_trouble | 8.2 | 3% |
| visiting Foreman Garrick: hand in silk_for_sails | 8.2 | 3% |
| visiting Tilda: hand in meat_for_the_pot | 7.9 | 3% |
| visiting Captain Maren: hand in thin_the_herd | 7.2 | 2% |
| visiting Old Brannoc: hand in dust_of_shadows | 7.1 | 2% |
| visiting Tilda: hand in wool_for_winter | 6.7 | 2% |
| lay_of_the_land: explore the Trapper's Camp | 6.7 | 2% |
| windmill_breadcrumb: talk to Farmer Ostrin | 6.5 | 2% |
| broken_tower: explore the Broken Tower | 6.2 | 2% |
| lookouts_eye: explore Watch Hill Lookout | 5.8 | 2% |
| lay_of_the_land: explore the Old Quarry | 5.6 | 2% |

The longest runs (unbroken travel), by route:

| from | to | for | longest | median | times | bots |
|---|---|---|---|---|---|---|
| 62 m from south_road_tower_7 | birchrise_boars_1 | boar_trouble: kill Thornboar | 18:56 | 18:56 | 1 | 1 |
| Hearthstead | The Logging Camp | boar_trouble: kill Thornboar | 6:41 | 6:41 | 1 | 1 |
| birchrise_boars_2 | The North Watch | tusks_and_tempers: kill Thornboar for Boar Tusk | 6:36 | 6:36 | 1 | 1 |
| North Gate Rocks | birchrise_boars_1 | helping a party member | 5:33 | 5:33 | 1 | 1 |
| 39 m from oak_webs | The Old Barrow | dust_of_shadows: wait for a Shade to rise for Shade dust | 5:27 | 5:27 | 1 | 1 |
| North Gate Rocks | 35 m from Hearthstead | with a party member | 5:12 | 5:12 | 1 | 1 |
| 54 m from south_road_tower_7 | Beacon Rocks | watchfire: gather Coal | 4:32 | 4:32 | 1 | 1 |
| 118 m from Hearthstead | The North Watch | boar_trouble: kill Thornboar | 4:28 | 4:28 | 1 | 1 |
| 59 m from west_trail_camp_11 | The Old Quarry | recalling to town | 4:09 | 4:09 | 1 | 1 |
| 57 m from North Gate Rocks | North Gate Rocks | recalling to town | 3:57 | 3:57 | 1 | 1 |
| 43 m from south_road_tower_7 | Beacon Rocks | watchfire: gather Coal | 3:52 | 3:52 | 1 | 1 |
| 112 m from south_road_tower_7 | North Gate Rocks | coal_for_the_watch: gather Coal | 3:51 | 3:51 | 1 | 1 |

## Reward gaps over 2:00

| run | bot | brain | from | gap | where: from, to | mostly | doing |
|---|---|---|---|---|---|---|---|
| A | 06com | completionist | 14:11 | 15:48 (to the end) | Hearthstead → Hearthstead | town 100% | visiting Old Brannoc: hand in dust_of_shadows |
| A | 09par | party | 13:13 | 15:41 | 83 m from oak_webs → Hearthstead | town 88%, travel 12% | with a party member; quest business with Hollis |
| B | 08par | party | 6:30 | 10:52 | Hearthstead → south_road_tower_7 | town 50%, idle 48% | waiting a party member; broken_tower: explore the Broken Tower |
| B | 09par | party | 5:39 | 5:12 | North Gate Rocks → North Gate Rocks | idle 98%, gather 2% | with a party member; coal_for_the_watch: gather Coal |
| B | 09par | party | 10:52 | 5:11 | North Gate Rocks → North Gate Rocks | idle 98%, gather 2% | with a party member; coal_for_the_watch: gather Coal |
| A | 07gri | grinder | 21:06 | 5:05 | 79 m from west_trail_camp_11 → 43 m from Hearthstead | travel 87%, fight 11% | going to grind; grinding |
| A | 08par | party | 20:18 | 4:33 | The North Watch → north_road_view_2 | idle 60%, town 29% | waiting a party member; lookouts_eye: explore Watch Hill Lookout |
| B | 09par | party | 16:03 | 3:20 | North Gate Rocks → North Gate Rocks | idle 98%, gather 2% | with a party member; coal_for_the_watch: gather Coal |
| B | 00com | completionist | 19:49 | 3:12 | 62 m from ford_trail_ruin_20 → 40 m from Fisher's Landing | travel 66%, fight 32% | down_to_water: talk to Old Wenna; thieves_in_fields: kill Ashcloak Thief |
| B | 11gri | grinder | 20:20 | 3:11 | 58 m from west_trail_camp_11 → 38 m from west_trail_camp_11 | travel 97%, fight 3% | going to grind; defending against Thornboar |
| A | 11gri | grinder | 27:03 | 2:56 (to the end) | 70 m from south_road_tower_7 → 64 m from south_road_tower_7 | travel 100% | going to grind |
| B | 10com | completionist | 15:26 | 2:52 | birchrise_boars_1 → The Logging Camp | travel 100% | garricks_camp: talk to Foreman Garrick; visiting Old Brannoc: accept know_the_vale |
| A | 00com | completionist | 14:53 | 2:47 | Beacon Rocks → 34 m from The Logging Camp | travel 100% | garricks_camp: talk to Foreman Garrick; visiting Hollis: accept word_from_east |
| B | 10com | completionist | 24:49 | 2:28 | The Logging Camp → The Old Barrow | travel 90%, wait_spawn 7% | visiting Anwen: accept forgotten_graves; dust_of_shadows: wait for a Shade to rise for Shade dust |
| B | 04soc | social | 11:43 | 2:27 | The North Watch → North Gate Rocks | travel 100% | boar_trouble: go to the Thornboar camp; visiting Old Brannoc: accept dust_of_shadows |
| B | 06com | completionist | 16:20 | 2:24 | The North Watch → 68 m from The North Watch | travel 93%, fight 7% | boar_trouble: go to the Thornboar camp; visiting Hollis: accept word_from_east |
| A | 09par | party | 8:33 | 2:20 | The Old Quarry → The North Watch | travel 79%, idle 14% | following a party member; with a party member |
| B | 06com | completionist | 12:14 | 2:16 | The North Watch → 68 m from The North Watch | travel 96%, fight 4% | boar_trouble: go to the Thornboar camp; visiting Old Brannoc: accept dust_of_shadows |
| A | 04soc | social | 25:11 | 2:10 | The Logging Camp → barrow_webs | travel 81%, fight 19% | webs_between_roots: go to the Websnare camp; tusks_and_tempers: go to the Thornboar camp for Boar Tusk |
| B | 11gri | grinder | 27:49 | 2:09 (to the end) | 41 m from west_trail_camp_11 → 38 m from west_trail_camp_11 | travel 100% | going to grind |
| B | 10com | completionist | 22:40 | 2:08 | The Old Barrow → The Logging Camp | travel 100% | visiting Foreman Garrick: hand in into_barrowwood |
| B | 08par | party | 17:22 | 2:04 | south_road_tower_7 → Hearthstead | idle 54%, travel 46% | waiting a party member; visiting Guard Tolan: hand in broken_tower |
| A | 08par | party | 25:30 | 2:01 | The North Watch → 55 m from logging_trail_well_14 | idle 67%, fight 33% | waiting a party member; tusks_and_tempers: kill Thornboar for Boar Tusk |

Longest gap per bot: median 2:25, worst 15:48; 23 gaps over 2:00 among 24 bots.

## Quests

| quest | level | accepted | done | done at (median) |
|---|---|---|---|---|
| cold_hearth | 1 | 24 | 24 | 0:07 |
| meat_for_the_pot | 1 | 18 | 18 | 2:29 |
| broken_tower | 2 | 18 | 13 | 6:24 |
| kindling | 2 | 18 | 18 | 0:45 |
| lay_of_the_land | 2 | 13 | 13 | 8:56 |
| thin_the_herd | 2 | 24 | 24 | 3:33 |
| wool_for_winter | 2 | 18 | 15 | 5:11 |
| boar_trouble | 3 | 17 | 16 | 14:00 |
| coal_for_the_watch | 3 | 18 | 14 | 7:35 |
| eyes_in_the_dark | 3 | 17 | 8 | 12:09 |
| garricks_camp | 3 | 17 | 14 | 16:59 |
| lookouts_eye | 3 | 17 | 14 | 13:06 |
| north_watch | 3 | 18 | 17 | 9:58 |
| tusks_and_tempers | 3 | 14 | 4 | 21:24 |
| watchfire | 3 | 17 | 14 | 13:27 |
| down_to_water | 4 | 5 | 1 | 23:05 |
| dust_of_shadows | 4 | 8 | 6 | 28:57 |
| iron_for_hollis | 4 | 14 | 5 | 10:48 |
| old_tusk | 4 | 16 | 0 | - |
| pincher_supper | 4 | 1 | 0 | - |
| silk_for_sails | 4 | 14 | 7 | 24:57 |
| stolen_grain | 4 | 5 | 1 | 27:14 |
| thieves_in_fields | 4 | 5 | 1 | 27:14 |
| webs_between_roots | 4 | 12 | 6 | 26:14 |
| windmill_breadcrumb | 4 | 15 | 5 | 22:16 |
| word_for_rysa | 4 | 14 | 12 | 20:14 |
| forgotten_graves | 5 | 1 | 0 | - |
| hearth_stirs | 5 | 1 | 0 | - |
| into_barrowwood | 5 | 4 | 1 | 24:49 |
| know_the_vale | 5 | 12 | 0 | - |
| shells_for_nets | 5 | 1 | 1 | 28:36 |
| word_from_east | 5 | 5 | 0 | - |

