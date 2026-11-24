# Old Tusk: staged runs (round 2)

Every bot is level 4 in the quest gear of a level 4 (the quest weapons, boots, cloak and the chest piece from Boar Trouble), at full health, with every quest but Old Tusk finished (`dyno bots --setup`; table by `tools/playtest/elite.py`). One bot and one server for each solo run. "solo" is each class once, "again" a second attempt by the three that came closest.

## The hand-in build (after merging main's class pass): level 4, 442 health, 12 damage

| run | bot | class | level | brain | fights | won | lost | longest fight | damage taken (most) | deaths to it | quest done |
|---|---|---|---|---|---|---|---|---|---|---|---|
| r2-tusk-solo-hunter | 00com | Hunter | 4 | completionist | 1 | 0 | 1 | 25 s | 87 | 1 | no |
|  |  |  |  |  | at 73 s | lost |  | 25 s | 87 |  | died |
| r2-tusk-solo-mage | 00com | Mage | 4 | completionist | 1 | 0 | 1 | 7 s | 66 | 1 | no |
|  |  |  |  |  | at 83 s | lost |  | 7 s | 66 |  | died |
| r2-tusk-solo-paladin | 00com | Paladin | 4 | completionist | 1 | 0 | 1 | 38 s | 239 | 1 | no |
|  |  |  |  |  | at 73 s | lost |  | 38 s | 239 |  | died |
| r2-tusk-solo-priest | 00com | Priest | 4 | completionist | 1 | 0 | 1 | 26 s | 186 | 1 | no |
|  |  |  |  |  | at 89 s | lost |  | 26 s | 186 |  | died |
| r2-tusk-solo-rogue | 00com | Rogue | 4 | completionist | 1 | 0 | 1 | 31 s | 82 | 1 | no |
|  |  |  |  |  | at 73 s | lost |  | 31 s | 82 |  | died |
| r2-tusk-solo-warrior | 00com | Warrior | 4 | completionist | 1 | 0 | 1 | 16 s | 130 | 1 | no |
|  |  |  |  |  | at 82 s | lost |  | 16 s | 130 |  | died |
| r2-tusk-again-paladin | 00com | Paladin | 4 | completionist | 1 | 0 | 1 | 24 s | 191 | 1 | no |
|  |  |  |  |  | at 83 s | lost |  | 24 s | 191 |  | died |
| r2-tusk-again-priest | 00com | Priest | 4 | completionist | 1 | 0 | 1 | 27 s | 186 | 1 | no |
|  |  |  |  |  | at 82 s | lost |  | 27 s | 186 |  | died |
| r2-tusk-again-warrior | 00com | Warrior | 4 | completionist | 1 | 0 | 1 | 16 s | 130 | 1 | no |
|  |  |  |  |  | at 82 s | lost |  | 16 s | 130 |  | died |
| r2-tusk-party3 | 00par | Warrior | 4 | party | 1 | 1 | 0 | 10 s | 58 | 0 | yes |
|  |  |  |  |  | at 81 s | won |  | 10 s | 58 |  | it died |
| r2-tusk-party3 | 01par | Paladin | 4 | party | 1 | 1 | 0 | 10 s | 0 | 0 | yes |
|  |  |  |  |  | at 80 s | won |  | 10 s | 0 |  | it died |
| r2-tusk-party3 | 02par | Hunter | 4 | party | 1 | 1 | 0 | 10 s | 0 | 0 | yes |
|  |  |  |  |  | at 79 s | won |  | 10 s | 0 |  | it died |
| r2-tusk-party2 | 00par | Warrior | 4 | party | 1 | 1 | 0 | 17 s | 101 | 0 | yes |
|  |  |  |  |  | at 84 s | won |  | 17 s | 101 |  | it died |
| r2-tusk-party2 | 01par | Paladin | 4 | party | 1 | 1 | 0 | 17 s | 0 | 0 | yes |
|  |  |  |  |  | at 83 s | won |  | 17 s | 0 |  | it died |

## The same Old Tusk before the merge (round 1's classes)

| run | bot | class | level | brain | fights | won | lost | longest fight | damage taken (most) | deaths to it | quest done |
|---|---|---|---|---|---|---|---|---|---|---|---|
| r2-tusk-solo-hunter | 00com | Hunter | 4 | completionist | 1 | 0 | 1 | 11 s | 89 | 1 | no |
|  |  |  |  |  | at 85 s | lost |  | 11 s | 89 |  | died |
| r2-tusk-solo-mage | 00com | Mage | 4 | completionist | 1 | 0 | 1 | 5 s | 50 | 1 | no |
|  |  |  |  |  | at 83 s | lost |  | 5 s | 50 |  | died |
| r2-tusk-solo-paladin | 00com | Paladin | 4 | completionist | 1 | 0 | 1 | 28 s | 210 | 1 | no |
|  |  |  |  |  | at 84 s | lost |  | 28 s | 210 |  | died |
| r2-tusk-solo-priest | 00com | Priest | 4 | completionist | 1 | 0 | 1 | 26 s | 249 | 1 | no |
|  |  |  |  |  | at 84 s | lost |  | 26 s | 249 |  | died |
| r2-tusk-solo-rogue | 00com | Rogue | 4 | completionist | 1 | 0 | 1 | 9 s | 74 | 1 | no |
|  |  |  |  |  | at 83 s | lost |  | 9 s | 74 |  | died |
| r2-tusk-solo-warrior | 00com | Warrior | 4 | completionist | 1 | 0 | 1 | 12 s | 103 | 1 | no |
|  |  |  |  |  | at 84 s | lost |  | 12 s | 103 |  | died |
| r2-tusk-again-paladin | 00com | Paladin | 4 | completionist | 1 | 0 | 1 | 25 s | 210 | 1 | no |
|  |  |  |  |  | at 84 s | lost |  | 25 s | 210 |  | died |
| r2-tusk-again-priest | 00com | Priest | 4 | completionist | 1 | 0 | 1 | 27 s | 232 | 1 | no |
|  |  |  |  |  | at 84 s | lost |  | 27 s | 232 |  | died |
| r2-tusk-again-warrior | 00com | Warrior | 4 | completionist | 2 | 0 | 2 | 13 s | 103 | 2 | no |
|  |  |  |  |  | at 84 s | lost |  | 11 s | 103 |  | died |
|  |  |  |  |  | at 163 s | lost |  | 13 s | 103 |  | died |
| r2-tusk-party3 | 00par | Warrior | 4 | party | 1 | 1 | 0 | 18 s | 0 | 0 | yes |
|  |  |  |  |  | at 85 s | won |  | 18 s | 0 |  | it died |
| r2-tusk-party3 | 01par | Paladin | 4 | party | 1 | 1 | 0 | 18 s | 48 | 0 | yes |
|  |  |  |  |  | at 84 s | won |  | 18 s | 48 |  | it died |
| r2-tusk-party3 | 02par | Hunter | 4 | party | 1 | 1 | 0 | 18 s | 59 | 0 | yes |
|  |  |  |  |  | at 83 s | won |  | 18 s | 59 |  | it died |
| r2-tusk-party2 | 00par | Warrior | 4 | party | 1 | 0 | 1 | 16 s | 103 | 1 | no |
|  |  |  |  |  | at 84 s | lost |  | 16 s | 103 |  | died |
| r2-tusk-party2 | 01par | Paladin | 4 | party | 1 | 1 | 0 | 23 s | 65 | 0 | yes |
|  |  |  |  |  | at 83 s | won |  | 23 s | 65 |  | it died |

## Main's Old Tusk: 276 health, 9 damage (before the merge, with the heal on reset already in)

| run | bot | class | level | brain | fights | won | lost | longest fight | damage taken (most) | deaths to it | quest done |
|---|---|---|---|---|---|---|---|---|---|---|---|
| r2-tusk-solo-hunter | 00com | Hunter | 4 | completionist | 2 | 0 | 2 | 22 s | 89 | 2 | no |
|  |  |  |  |  | at 78 s | lost |  | 22 s | 66 |  | died |
|  |  |  |  |  | at 194 s | lost |  | 16 s | 89 |  | died |
| r2-tusk-solo-mage | 00com | Mage | 4 | completionist | 2 | 0 | 2 | 11 s | 62 | 2 | no |
|  |  |  |  |  | at 84 s | lost |  | 11 s | 62 |  | died |
|  |  |  |  |  | at 193 s | lost |  | 10 s | 62 |  | died |
| r2-tusk-solo-paladin | 00com | Paladin | 4 | completionist | 1 | 0 | 1 | 32 s | 194 | 1 | no |
|  |  |  |  |  | at 84 s | lost |  | 32 s | 194 |  | died |
| r2-tusk-solo-priest | 00com | Priest | 4 | completionist | 1 | 1 | 0 | 16 s | 87 | 0 | yes |
|  |  |  |  |  | at 84 s | won |  | 16 s | 87 |  | it died |
| r2-tusk-solo-rogue | 00com | Rogue | 4 | completionist | 1 | 0 | 1 | 14 s | 89 | 1 | no |
|  |  |  |  |  | at 84 s | lost |  | 14 s | 89 |  | died |
| r2-tusk-solo-warrior | 00com | Warrior | 4 | completionist | 1 | 1 | 0 | 15 s | 89 | 0 | yes |
|  |  |  |  |  | at 83 s | won |  | 15 s | 89 |  | it died |
