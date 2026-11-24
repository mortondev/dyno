# Survival audit

Step 1 of removing the survival game from Hearthblock Online: the audit, written read-only before anything was changed. The table was approved and acted on; what was decided and done is in "Outcome" at the end.

- **Audited at:** `9e1b755` (v0.10.0, protocol 23). All line numbers are from that commit.
- **"Live"** means reachable when the server runs the shipped pack (`content/rules/hearthblock.json`, `world.protected: true`, world built from `content/world/layout.json`) with the `hearthblock` client. Rows are classed by their callers, not their names.
- **Classes:** `dead` (unreachable in the live game), `live` (reachable; new name given), `player` (changes what players can do; recommendation only, not decided).
- **Wire** and **Save** columns say whether the item is part of the protocol-23 wire format or of `players.json` / `server.json` / `chests.json`. A "yes" means the data shape must stay even where the code behind it is dead.

## Read this first

1. **`dyno bench` is not in this branch.** The step 2 picture gate needs `dyno bench --diff`. It exists on `main` (`crates/dyno/src/bench.rs`, `main` is at `9b2faa4`), which is ahead of this worktree. Step 2 has to start by rebasing onto `main`; line numbers below will then shift.
2. **The switch between the two games is one flag.** `WorldRules.protected` (`content.rs:556`), read as `State::rpg()` (`rpg.rs:19`) on the server and `content.world.protected` on the client. Most dead code is the `!protected` arm of a branch on it. The flag itself is a key in the rules file that other lanes edit (row D-30).
3. **`Mode::Survival` is every normal player.** It is the default mode, it is what the live save stores (`"mode": "Survival"` in `~/dyno-world/players.json`, read-only check), and the Shade code tests for it. It is live and needs a new name, with the saved string kept (row L-3).
4. **Creative mode is live, for operators and tools.** Operators reach it with `/gamemode creative`; observers (`dyno shot`, `dyno scout`) are welcomed as `Mode::Creative` (`lib.rs:1516`), so the client's `creative()` branches run in every tool picture. It is not dead (rows P-9, P-10).
5. **Hunger is dead; food is live.** In a protected world food is pinned at 20 and eating heals instead. Hunger rows are therefore `dead`, not `player`.
6. **Night Shades are live**, not just the Incursion: the roaming dark spawn runs for every `Mode::Survival` player in the shipped pack, and `night_brings_shades` tests it against the shipped pack.
7. **Three findings that look like live bugs** came out of the audit and are listed at the end. They are not survival removal work.

## A. Changes what players can do (recommendation only)

| ID | Item | Where | Callers / how it is reached live | Wire | Save | Recommendation |
|---|---|---|---|---|---|---|
| P-1 | Night Shades: roaming dark spawn (18 to 32 blocks from a player, light under 4, at night or underground; not in a hearth's reach or a safe area) | `creatures.rs:432-460`, timers `SHADE_INTERVAL`, `Player.shade_timer`; creature `shade` (`spawn: dark`) in the rules | `spawn_creatures` each tick for every active player whose mode is `Survival`. Test: `survival_loop.rs:108 night_brings_shades` (shipped pack) | no | no | Keep. It is the game's "the dark is dangerous" rule and the Incursion reuses the creature. Lead decides whether a WoW-like world should have unscheduled night spawns in levelled zones. |
| P-2 | Light rules: hearths are sanctuary, light 10 or more burns Shades, daylight unmakes them | `creatures.rs:1-10` (module doc), `find_hearths`, `in_sanctuary`, `w::HEARTH_RADIUS` | `tick_creatures`; `State.hearths` built at start (`lib.rs:422`). The Great Hearth stamp places a hearth block | no | no | Keep with P-1; they are one feature. |
| P-3 | Nightfall Incursion: Shade waves at a town gate after dusk | `events.rs` (whole file), `/event` op command (`lib.rs:1861`) | `maybe_start_incursion` at dusk, requires `rpg()`. Test: `survival_loop.rs:782 nightfall_incursion` | `ServerMsg::WorldEvent` | no | Keep. It was written for the RPG (`rpg.md`, "Dynamic world"). |
| P-4 | Beds: right click sets your spawn point; at night you lie down, and when half the players sleep the night runs fast | server `game.rs:1773-1798`, `284-330`; client `app.rs:1842`, `4011-4017`; `Player.sleeping`, `PlayerData.spawn` | Live: cottages, the inn and the smithy stamps contain beds (`tools/stamps/build.py:29,91,110,363,715,1174`); the WoW click path sends `Interact` | `ClientMsg::Interact`, `ServerMsg::Time` | `spawn` | Decide. A bed in any cottage rebinding your respawn, and players speeding the shared clock, are survival co-op rules. Options: keep; keep only the inn as a bind point; make beds decoration. |
| P-5 | Chests: shared storage, open to anyone in reach | server `game.rs:1764-1771`, `move_items`, `throw`, `send_container`, `store::load_chests`; client `app.rs:1842`, `2263-2267`, the pack window's chest grid | Live: cottages and the smithy contain chests (`build.py:32,93,109,658,975,1176`). Live save has none in use (`chests.json` is `{}`) | `Interact`, `Move`, `Throw`, `CloseContainer`, `Container`, `SlotRef::Chest` | `chests.json` | Decide. Unowned shared storage in a public town is a survival feature; in an MMO it is a free bank with no owner. Options: decoration only (server ignores `Interact` on chests), or keep until a bank exists. |
| P-6 | Crafting at stations (the pack window's recipe list) | server `game.rs:1653-1725 craft`, `stations_near`; `dyno_sim::crafting`; client `survival.rs:406 pack_window`, `518 side_panel`; rules `recipes` (46) | Live: right click a crafting table, furnace or anvil (`app.rs:1844`) opens the window; `Craft` is sent from `app.rs:3659`. The smithy stamp has furnaces and an anvil. Recipes carry `blacksmithing` and `cooking` skills (`professions.rs`) | `ClientMsg::Craft`, `Status.fuel` | `fuel`, `skills` | Keep as profession crafting; rename the UI (L-1). Lead decides the two points in P-7. |
| P-7 | Recipes that cannot be finished or used in the live world | rules `recipes`: 34 with `station: "table"`; outputs that are blocks (crafting table, furnace, chest, bed, hearth, torch, lantern, building blocks, 9 dyes) | No stamp contains a crafting table (`grep` of `content/stamps`), the layout places none, and players cannot place blocks, so every `table` recipe is unreachable, including all tools. The anvil opens the window but the server counts only table and furnace as stations (`game.rs:1666`). Block outputs cannot be placed (P-8) | no | item keys | Decide. Either (a) put a crafting table in the smithy, or make the anvil count as the table, so blacksmithing works, or (b) cut the unreachable recipes. Either way the block and dye recipes make items with no use; recommend cutting those. The file belongs to the class and flow lanes. |
| P-8 | Players digging and placing blocks | server `game.rs:1304-1393 dig`, `1401-1450 place`, `rpg.rs:491 may_harvest`; client `app.rs:2942-3006` (hold to mine), `4036-4066` (place) | Shipped client: never. `can_mine` needs `!wow()` (`app.rs:2942`), placing returns at `app.rs:4037`, the server refuses `Place` at `game.rs:1402`. But the server still accepts `DigStart` + `Dig` on a resource node from any non-creative player (`may_harvest` passes when `respawn > 0`), with drops and regrowth, bypassing the `Gather` cast | `DigStart`, `Dig`, `Place`, `Digging` | no | Recommend: the server refuses `Dig` and `Place` unless the player is in creative mode, and the client's hold-to-mine code goes (D-12). This removes an ability only a modified client has. Keep the refusal test (T-2). Lead decides because it is a rule change. |
| P-9 | Creative mode for operators: fly (F), break and place freely, the item palette (E, middle click), box fill (G, G, V or X), undo (Ctrl+Z), no damage, no movement checks | server `lib.rs:1999-2012 /gamemode`, `game.rs:1163-1188`, `1452-1520 fill`, `movement.rs:69`, `game.rs:1809`, `2165`; client every `creative()` branch in `app.rs` (23 sites), `survival.rs:528-560` palette | Live for operators (`may_op`: `--op` names, else the world's owner). Live for tools: observers are `Mode::Creative` (`lib.rs:1516`). Test `world_streaming.rs:220` uses it to change blocks | `Mode`, `SetBlock`, `Fill` (both ways), `CreativeSet`, `MAX_FILL` | `mode` | Decide. Keep it as the operator build mode and rename it for what it is (L-3), or remove it now that the world is built from the layout. If removed, the observer path and `world_streaming.rs` need another way to fly and to change a block. I have not checked whether creative edits survive a layout rebuild (`migrate::rebuild`). |
| P-10 | Starting every new player in creative: `dyno server --creative`, `Config.default_mode` | `main.rs:15`, `618`; `lib.rs:79`, `116`, `462`; `game.rs:194`, `758`; `shard.rs:134-192`; `clustered.rs:184` | The live server does not pass the flag (Dockerfile, `docs/hosting.md`). `default_mode` itself is read on every first join | no | sets `mode` | Recommend deleting the flag and the field, and using `Mode::default()`; operators still have `/gamemode`. Depends on P-9. |
| P-11 | `dyno fill <address> ...` | `main.rs:44`, `338-406` | Joins as the player "builder" and sends `Fill`. The server answers "Fill works in creative mode only" unless that character is already creative, so on the live server it only works after an operator sets it up. Listed in `README.md:90` | `Fill` | no | Recommend deleting with P-10, whatever P-9 decides: operators can fill in game. |
| P-12 | Throwing items on the ground (Q), items lying in the world, picking them up | client `app.rs:3824-3833` (no mode guard), server `game.rs:1606-1651 throw`, `spawn_item`, `EntityKind::Item` | Live for every player. `spawn_item` also serves craft overflow and gathering drops | `Throw`, `EntityKind::Item` | no | Decide. It is the survival way to hand items over; the RPG has trade. Keep ground items (drops use them); lead decides whether Q stays. |
| P-13 | Fall and drowning damage measured by the client | client `app.rs:2900-2935`, bots `body.rs:1279,1300`; server `game.rs:1204-1212` | Live | `ClientMsg::Hurt`, `EnvDamage` | no | Keep. Listed only because it comes from the survival stats. See finding F-2. |
| P-14 | `/kill`, `/spawn`, `/tp <player>` for every player | `lib.rs:1833-1860` | Live, no operator check on these three | no | no | Decide. They are survival co-op conveniences; `/tp` to any player is unusual for an MMO. Out of scope unless the lead wants them in. |
| P-15 | `/gamemode survival` as the command's wording | `lib.rs:2005-2011`, `/help` at `lib.rs:1828` | Live for operators | no | no | With P-9: if creative stays, the argument and message need the new name (L-3). The old argument can stay as a silent alias, or go. |

## B. Dead in the live game

"Dead" here means no caller in the live game. Step 2 deletes only rows the lead approves.

### Server

| ID | Item | Where | Callers | Wire | Save | Note |
|---|---|---|---|---|---|---|
| D-1 | Hunger drain and survival health regeneration | `game.rs:2200-2220`, `Player.hunger_clock` | Only when `!rpg` in `tick_players` | no | no | The `rpg` arm pins `food` at 20 (`game.rs:2190`). |
| D-2 | Eating fills hunger | `game.rs:1750-1756` (the non-protected tail of `use_held`) | Only when `!protected` | no | no | The protected arm (food heals) is live through `EatBest` and `UseHeld`. |
| D-3 | Death pile: keep the hotbar, drop the rest | `game.rs:1838-1846`, `1855-1857` | Only when `!protected` | no | no | |
| D-4 | Respawn at 20 health, food at least 10 | `game.rs:1870-1875` (else arm and `p.food` line) | Only when `!protected` | no | no | |
| D-5 | The Journal: `State::milestone`, the "first night" check | `game.rs:1912-1935`, `2163`, `2168-2174`, `2221-2223`, `Player.saw_night`; calls at `game.rs:1449`, `1723`, `2119`, `creatures.rs:1003` | `milestone` returns at once when `rpg()` | `ServerMsg::Milestone` stays | `PlayerData.milestones` stays | Delete the body and the calls; keep the wire variant and the saved field (D-28, D-29). |
| D-6 | Direct melee `State::attack` (knockback, held tool damage, tool wear) | `creatures.rs:1008-1097` | One caller: `game.rs:1197`, the `!rpg` arm of `ClientMsg::Attack`. The live arm calls `set_auto_attack`; auto-attack swings do their own damage in `gear.rs:169-240 tick_auto_attacks` and never call `attack` (checked) | `ClientMsg::Attack` stays | no | The `rpg` branches inside it (`1017`, `1026`) die with it. `Player.held_tool` (`game.rs:140`) then has one caller left, in `dig` (P-8). |
| D-7 | Creature level and health without areas | `creatures.rs:292`, `307` (else arms) | Only when `!rpg` | no | no | |
| D-8 | Free meadow spawning of Grazers | `creatures.rs:462-484`, `GRAZER_INTERVAL`, `Player.grazer_timer` | Needs `!has_areas`; the shipped pack has areas and camps | no | no | |
| D-9 | `load_or_generate`: a world from the seed, with a spawn hearth and beacons | `lib.rs:303-345` and what follows it | `lib.rs:416`, only when the pack has no layout or is not protected. Used by `survival_loop()` (T-1) | no | no | |
| D-10 | `migrate::expand`: growing an old seed world | `migrate.rs:37-187`, its test `growing_keeps_builds_and_moves_saves` (`migrate.rs:396`) | One caller: `load_or_generate` (`lib.rs:320`) | no | reads old saves | It ran on the live save on 22 November (the `.before-384` files). The live save now loads through `load_or_build` and `migrate::rebuild`, which stay. Check `natural`, `floor`, `standing` for use by `rebuild` before deleting. |
| D-11 | `DigStart` handling and the dig crack shown to others | `game.rs:1150-1160`, `Player.dig`, the `Digging` broadcast at `game.rs:1386-1392` | The shipped client sends `DigStart` only from hold-to-mine (D-12) | `DigStart`, `Digging` stay | no | Goes with P-8. |
| D-31 | Tool wear and breaking ("Your tool broke.") | `game.rs:1378-1390` (in `dig`), `creatures.rs:1076-1091` (in `attack`); `Tool.durability` in the rules | Only the non-creative `dig` path (P-8) and `attack` (D-6). Gathering does not wear tools (`professions.rs` has no wear) | `Stack.wear` stays | `SavedStack` wear stays | Goes with P-8 and D-6. The `durability` numbers in the rules become unused data (content lanes). |

### Client

| ID | Item | Where | Callers | Wire | Save | Note |
|---|---|---|---|---|---|---|
| D-12 | Hold-to-mine: timing, particles, the wrong-tool hint | `app.rs:2941-3008`, `Play.dig`, `Play.tier_hint`, `app.rs:3382`, the crack overlay at `app.rs:4739` | `can_mine` needs `!creative && !wow()`, which is never true in a protected world | sends `Dig`, `DigStart` | no | In `app.rs` (other lanes). Goes with P-8. |
| D-13 | Hunger and air bars | `survival.rs:199-231 bars`; call `app.rs:4712-4730` | Only when `!creative && !protected` | no | no | See finding F-2: the air meter dies here too. |
| D-14 | The survival welcome line | `app.rs:1970-1972` (the non-creative text) | Only when `!protected` | no | no | The creative text in the same line is unreachable too: the guard is `!protected`. |
| D-15 | Right click to eat when hungry | `app.rs:4024-4034` | Needs `food < 20`; food is always 20 live | sends `UseHeld` | no | `UseHeld` stays live from the bags (`app.rs:3552`). |
| D-16 | Left click sends `Attack` | `app.rs:3981-3983` | Only when `!protected` | `Attack` stays | no | Goes with D-6. |
| D-17 | Others' dig crack, the "Journal" toast | `app.rs:2633-2640`, `6290` | The server never sends `Milestone` or `Digging` live | variants stay | no | Keep empty match arms. |
| D-18 | Sprint needs food | `app.rs:2863` (`p.food > 0.0`) | Always true live | no | no | One condition. |
| D-32 | The 9-slot hotbar as the player's bar: number keys pick a slot, the hotbar strip | `app.rs:3944-3946` (else arm), `4650` onwards (the `!rpg_bar` arm) | Only when `!protected` or creative. Live players get the action bar. Live for operators in creative (P-9) | no | no | Dies only if P-9 removes creative. `ClientMsg::Held` and the server's `held` slot stay live either way: the bags send `Held` then `UseHeld` (`app.rs:3552-3553`) and `eat_best` sets it (`abilities.rs:660`). |
| D-19 | `dyno shot --ui dig` | `app.rs:6574-6578`, help text `main.rs:25`, `shot.rs:27`, `app.rs:6102` | Sets a dig crack that no live player can see | no | no | `--ui pack` is live (it shows the crafting window): keep it, see L-6. `--ui dead` is live. |

### World and tools

| ID | Item | Where | Callers | Wire | Save | Note |
|---|---|---|---|---|---|---|
| D-20 | `World::generate`: the hard-coded seed terrain | `dyno-world/src/lib.rs:1294-1600` or so | Live server: none (only D-9 and D-10). Tests of live code use it as a cheap world: `farview.rs:280`, `minimap.rs:785,812,837`, `render.rs:2567`, `terrain_mesh.rs:1624`, `worldwork.rs:618`, `stream.rs:506,549,615`, `migrate.rs:401`, `light.rs:597,722,794`, `far.rs:203`, `dyno-world/src/lib.rs:1924,1940,1970,2015,2045,2100`, `world_scale.rs:914` | no | no | **Lead's choice.** Dead in the server, but about 22 tests of live code stand on it. Options: (a) keep it as a test fixture under a new name (`World::test_terrain`) behind a `test-fixtures` feature, so it leaves the shipped binaries; (b) move those tests to `worldgen::build` on the pack layout (slower, and a rewrite of live tests). I recommend (a). `world_scale.rs:914` uses it as "an old world" for `migrate::rebuild`, which the live save still needs once. |
| D-21 | The `village` shim | `dyno-world/src/lib.rs:21-25` | Tests only: `render.rs:2569`, `terrain_mesh.rs:1626` | no | no | Delete; the two tests call `worldgen::dev::hub` directly. |
| D-22 | The `zones` shim (does nothing, returns false) | `dyno-world/src/lib.rs:30-34` | One test: `render.rs:2570` | no | no | Delete with its call. |
| D-23 | `mining::break_time` | `dyno-sim/src/mining.rs:9-26` | `game.rs:1338` (dig), `app.rs:2965` (hold-to-mine), `survival_loop.rs:246`, `rules.rs:144-154` | no | no | Dead once P-8 is approved, except in creative (which skips it). `mining::drops` and `missing_tool` are live through `professions.rs:113,156`, and `drops` calls `harvests` (`mining.rs:38`), so `harvests` stays: only `break_time` can go. |

### Content and docs

| ID | Item | Where | Callers | Wire | Save | Note |
|---|---|---|---|---|---|---|
| D-24 | The 16 Journal milestones in the rules ("Punch a tree", "Survive until morning", ...) | `content/rules/hearthblock.json` `milestones`; `content.rs` `Milestone`, `Content.milestones` (`595`, `684`, `1426`) | Read only by `State::milestone` (D-5) | sent in the rules file: first-join bytes | no | The rules file belongs to other lanes: one mechanical deletion of the array, or hand it to them. The authored field is already `#[serde(default)]` (`content.rs:683-684`), and content reaches clients as the rules file, not as an encoded struct, so removing the array changes no format. |
| D-25 | Block rules that only timed digging reads: `hardness` and `tier` on blocks that are not resource nodes | rules `blocks` | `mining::break_time` only (D-23) | first-join bytes | no | Leave to the content lanes; listed for size. Nodes still use `tool` and `drops`. |
| D-26 | `docs/design/survival.md` | whole file | Cited by `creatures.rs:3` ("Night rules") and `rpg.md:3` ("stays as the record") | no | no | Recommend: move the Night rules and creature table that P-1 and P-2 keep into `rpg.md`, then delete the file. Git keeps the record. |
| D-27 | Survival wording in docs | `README.md:49`, `:90`; `docs/hosting.md:23`; `docs/architecture.md:416`; `docs/spec/index.html:272`, `4290`, `4306`, `4324`, `5127`, `5149`; `main.rs:7` ("live multiplayer block world") | n/a | no | no | README and hosting follow P-9 to P-11. The spec is the original engine spec: allowlist it or reword six lines (lead's choice). |

### Dead code whose data shape must stay

| ID | Item | Where | Why it stays |
|---|---|---|---|
| D-28 | `ServerMsg::Milestone`, `ServerMsg::Digging`, `ClientMsg::DigStart`, `ClientMsg::Attack`; and, if P-8 to P-11 are approved, `Dig`, `Place`, `SetBlock`, `Fill`, `CreativeSet` | `dyno-protocol/src/lib.rs:297-350`, `369`, `602`, `643`, `662` | Postcard encodes variants by position. Removing or reordering any changes the wire format, and protocol 24 is claimed. Handlers can go; variants stay, marked unused. |
| D-29 | `Status.food`, `PlayerData.food`, `PlayerData.milestones`, `Player.food` | `dyno-protocol/src/lib.rs:610`, `store.rs:21`, `30` | `food` is on the wire and in saves; `milestones` is in saves. The save gate needs every field identical. Keep the fields; stop reading them where dead. |
| D-30 | `WorldRules.protected` | `content.rs:552-556`; 31 server sites (`rpg()` or `world.protected`), 18 in the client's `app.rs`, 1 in the bots | A key in the rules file, which other lanes edit and the pack ships. Recommend: delete the `!protected` arms row by row above, keep the key parsed, and decide later whether the field becomes ignored. Not a step 2 deletion. **Arms of this flag that have no row above are not proposed for deletion.** All sites, for checking: server `creatures.rs:292,307,492,983,1017,1026,1056`; `events.rs:50`; `game.rs:776,786,919,977,1194,1328,1369,1402,1738,1839,1870,1913,2153`; `lib.rs:410,492`; `loot.rs:122`; `professions.rs:26,51,90`; `rpg.rs:20,24,358,492`; client `app.rs:350,1605,1970,2960,2966,3351,3836,3852,3937,3978,4037,4510,4620,4650,4700,4712,5287,5892`; bots `body.rs:278`. Most of the unlisted ones are guards that turn an RPG feature on (`if !rpg { return }`), where the dead arm is one line. |

## C. Live: rename for what it is

| ID | Item | Where | Callers | Proposed name | Risk |
|---|---|---|---|---|---|
| L-1 | `crates/dyno-client/src/survival.rs`: item and block icons, the crafting and chest window | `lib.rs:39`; `app.rs:14`, `567`, `4676`, `4713`, `5118`, `5575`, `5586`; `rpg_ui.rs:201`, `1092`, `1234`; `screens.rs:4`, `265`, `554`; read by path in `tests/pack.rs:147-150` | `item_ui.rs` (module `item_ui`); `pack_window` to `craft_window`; `PackView` and `PackCmd` keep their names. After D-13 nothing in it is survival | Ten one-token edits in `app.rs`, `rpg_ui.rs` and `screens.rs`, which other lanes are in. One mechanical commit; rebase just before. |
| L-2 | `crates/dyno-client/tests/survival_loop.rs` | `docs/architecture.md:416` | `tests/protocol.rs` (the architecture doc already calls them "Protocol tests"). See section D for the tests inside | None; whole file. |
| L-3 | `Mode::Survival`, `Mode::Creative`, the doc comment "Survival or creative" | `dyno-protocol/src/lib.rs:15-21`; uses at `store.rs:18,109`, `lib.rs:116,2005`, `creatures.rs:415,503`, `app.rs:2253`, `cluster/machine.rs:336`, `cluster/tests.rs:45`, `survival_loop.rs:223` | `Mode::Play` and `Mode::Build` (if P-9 keeps the build mode). **Wire:** safe, variants keep their positions. **Save:** `players.json` stores the variant name (`"mode": "Survival"` in the live save), so the variant needs `#[serde(rename = "Survival")]`, which puts one "Survival" on the allowlist. An alias would rewrite the saved value and fail the save gate | The serde attribute must be exact; covered by the save gate. |
| L-4 | `fn survival()` (serde default for `mode`) | `store.rs:18`, `109-111` | `fn default_mode()` | None. |
| L-5 | The local `survival` and the comment "alive survival players" | `creatures.rs:412-415`, `432`, `498` | `playing`; "alive players not in build mode" | None. |
| L-6 | `dyno shot --ui pack` | `app.rs:6469-6470`, `main.rs:25`, `shot.rs:27`, `docs/architecture.md:381` | Keep `pack` (it is the field and message name); no rename proposed | Listed because the brief names it. |
| L-7 | Module and item doc comments | `game.rs:1` ("survival rules on the server"), `game.rs:1838`, `1914`; `dyno-sim/src/lib.rs:1`; `content.rs:1`, `63` ("hunger points restored"), `552`; `creatures.rs:1-10`; `survival.rs:1` | Reword: "game state and rules", "health restored when eaten", and so on | Comments only. |
| L-8 | The death screen line "Your items are where you fell. Friends can pick them up too." | `app.rs:5611-5616` | Live and wrong (finding F-1). New text, for example "You will wake at your hearth." | One string in `app.rs`; the UI lane may prefer to own it. |
| L-9 | `docs/architecture.md:416` | | | Follows L-2 | None. |
| L-10 | The "Survival mode" / "Creative mode" toast when an operator changes mode | `app.rs:2253-2260` | `ServerMsg::Status` handler; live for operators | Follows L-3 ("Play mode" / "Build mode"), or goes with P-9 | Two strings in `app.rs`. |
| L-11 | Food: eating heals 5% of maximum health per food point, out of combat | `game.rs:1738-1749`, `abilities.rs:650-662 eat_best`, `inventory::best_food`, `Item.food`; client action slot `Eat` (`app.rs:494`), bots `agent.rs:1261` | Live | No rename of the `food` field (it is in the rules file, on the wire and in saves). Only the doc comment "hunger points restored when eaten" (`content.rs:63`) changes, under L-7 | None. |

### Proposed allowlist for `grep -ri survival crates content docs tools` after step 3

1. `crates/dyno-protocol/src/lib.rs`: the one `#[serde(rename = "Survival")]` line (L-3).
2. `docs/design/rpg.md:3`, `9`, `21`, `159`: the record of the pivot.
3. `docs/spec/index.html`: six lines of the original engine spec, unless the lead wants them reworded.
4. `docs/design/survival-audit.md`: this file, unless it is deleted at the end of step 3.

`content` and `tools` have no matches today.

## D. Tests

No test is deleted without a reason in the step 2 report. Tests of live behaviour move.

| ID | Test | Where | Server it runs | Class | Plan |
|---|---|---|---|---|---|
| T-1 | `survival_loop` | `survival_loop.rs:217-461`, helper `start_server` (`:8-27`, rewrites the rules to `protected: false`) | Unprotected, seed world: the only test that does (checked: `streaming.rs` and `cluster.rs` pass the built-in pack or its rules, protected and with the layout) | Mixed | Parts 1 and 2 (dig a log, craft planks, sticks, a table, place it, craft a pickaxe at it) test dead behaviour, except that crafting is live. Part 3 (`Move`, `Throw`), part 4a (a fall hurts), part 5 (the same name with another token is refused) are live and move to a test on the shipped pack. Part 4b (death drops all but the hotbar; respawn at 20) is dead and is replaced by "death keeps everything". A live crafting test at the smithy furnace replaces the table crafting. |
| T-2 | "the world is protected: digging the ground is refused" | `survival_loop.rs:765-775` inside `quests_give_xp_and_rewards` | Shipped pack | Live | Keep. If P-8 is approved, extend it to a resource node. |
| T-3 | `night_brings_shades` | `survival_loop.rs:108` | Shipped pack | Live (P-1) | Keep; moves with the file rename. |
| T-4 | `nightfall_incursion`, `quests_give_xp_and_rewards`, `characters_are_created_listed_and_deleted`, `corpses_are_looted_and_merchants_sell`, `phasing_shows_each_player_their_own_world`, `shards_fill_split_and_parties_meet`, `crowded_areas_are_spread_across_shards`, `cheating_movement_is_put_back`, `parties_share_kills_and_loot`, `players_trade_items_and_coins` | `survival_loop.rs:466-1475` | Shipped pack | Live | Keep; move with L-2. |
| T-5 | `stacking_and_overflow`, `crafting_planks_sticks_and_tools`, `smelting_uses_fuel`, `item_count_invariant` | `dyno-sim/tests/rules.rs` | none | Live | Keep. |
| T-6 | `mining_needs_the_right_tool` | `rules.rs:122-171` | none | Mixed | The `drops` and `missing_tool` assertions are live (gathering) and stay. The `break_time` assertions go only if D-23 is approved. |
| T-7 | `growing_keeps_builds_and_moves_saves` | `migrate.rs:396` | none | Dead (D-10) | Delete with `expand`. |
| T-8 | The block-change streaming test | `world_streaming.rs:196-250` | Shipped pack; uses `/gamemode creative` and `Fill` | Live, stands on P-9 | Keep. If creative is removed it needs another way to change blocks. |
| T-9 | `pack.rs` icon check | `tests/pack.rs:145-151` | none | Live | Update the path with L-1. |
| T-10 | The 22 tests listed in D-20 | | none | Live | Unchanged under option (a). |

## E. Findings outside the brief

| ID | Finding | Where |
|---|---|---|
| F-1 | The live death screen tells players their items are where they fell. In the RPG they keep everything. | `app.rs:5611-5616` |
| F-2 | Drowning is live (`app.rs:2900-2935`) but the air meter is drawn only in unprotected worlds (`app.rs:4712-4730`), so live players drown with no warning. Deleting D-13 as written keeps it that way; the lead may want the meter kept and shown. | |
| F-3 | Blacksmithing cannot make anything in the live world: every blacksmithing recipe but smelting needs a crafting table and there is none (P-7). | rules `recipes` |

## What step 2 would touch in other lanes' files

- `app.rs`: D-12 to D-18 (deletions inside existing functions), L-1 (6 tokens), L-8 (one string).
- `rpg_ui.rs`, `screens.rs`: L-1 only (3 tokens each).
- `content/rules/hearthblock.json`: D-24 (one array), and P-7 if the lead chooses to cut recipes. Nothing else.

## The save gate needs a shared baseline

`~/dyno-world/server.json` has no `world_layout` (read-only check), so the first time that save is loaded by this code `load_or_build` rebuilds the world and `migrate::rebuild` moves every saved position. That happens on the "before" binary as much as on the "after" one. For "every character field identical", step 2 will load one copy with the unchanged binary and another copy with the changed binary, and compare those two results, not the raw files.

## Not verified

- Whether blocks changed in creative mode survive a layout rebuild.
- How the WoW-mode `click` path at `app.rs:3951-4080` is reached while steering with a held button. I treated its non-creative block arms as dead on the strength of the guards at `4028`, `4037` and the server refusals.
- Sizes. No build was run, so client bytes and first-join bytes have no "before" figure yet. Step 2 starts by measuring both.

## Outcome (steps 2 and 3)

The table above was approved and acted on. This section records what was decided and what was done; the rows above keep their step 1 wording and line numbers.

### Decisions

| Row | Decision | Done |
|---|---|---|
| P-1 to P-3 | Keep night Shades, the light rules and the Incursion | Kept. The rules are now written in `rpg.md`, "Night and light" |
| P-4 | Home is made only at the inn, through Hearthstead's innkeeper. Other beds are decoration. No sleeping through the night | An NPC can be an `innkeeper` (rules data); talking to one makes that spot your home. The pack has one at the Wayfarer's Rest. The server ignores `Interact`; the sleep code is gone |
| P-5 | Chests are decoration until a bank exists | The server ignores `Interact` and refuses chest slots. `chests.json` is still loaded, saved and migrated untouched |
| P-6, P-7 | Profession crafting. Stations are data and a recipe names its station. Cut the block and dye recipes. Every recipe left can be made in the live world, with a test | `stations` in the rules: `table` (Workbench: crafting table, anvil, furnace) and `furnace` (Forge). 18 recipes remain of 46. Test: `every_recipe_can_be_made_in_the_live_world` |
| P-8 | The server refuses `Dig` and `Place` outside build mode; hold-to-mine goes | Done; `DigStart` is ignored |
| P-9 to P-11 | Keep the operator build mode under a new name; delete `--creative`, `default_mode` and `dyno fill` | `Mode::Build`, `/gamemode build` and `/gamemode play`, operators only. Observers still use it |
| P-12 | No throwing with Q; ground items stay for drops | Q only strafes; the window's "Throw selected" button is gone too and the server ignores `Throw` |
| P-14 | `/kill`, `/spawn`, `/tp` are for operators | Done. Bots that are stuck recall instead of using `/spawn` |
| D-20 | Keep the seed terrain as a renamed test fixture behind a feature | `World::test_terrain` in `fixture.rs`, feature `test-fixtures`, enabled only by dev-dependencies |
| D-26, D-27 | Move the night rules to `rpg.md`, delete `survival.md`, reword the spec | Done |
| D-30 | Delete only the dead arms; keep the key parsed | `protected` is still read. A pack without a layout is now refused at start |
| F-1, F-2 | Fix the death text; show the air meter | Done |

### Deleted without a row of their own

- The item-drop arm of a kill (`creatures.rs`, the `!rpg` arm beside the corpse): creatures always leave a body that holds the loot.
- `rpg::build_landmarks`: the beacons of seed worlds, called only from `load_or_generate` (D-9).
- `Play.mouse_left` in the client: read only by hold-to-mine (D-12).

### Recipes cut beyond blocks and dyes

The four stone tools were cut as well: they need cobble, and nothing in the live world gives cobble (stone is not a resource node). They would have failed the "can be made" test. Hollis still sells the stone pickaxe and axe, and the blacksmithing ladder still climbs without them (wooden tools at 1, ingots at 15, iron tools at 25, ember tools at 60).

### Wire and saves

- Protocol 23 is unchanged: no variant was added, removed or reordered. `DigStart`, `Throw`, `Interact`, `CloseContainer`, `Container`, `Milestone` and `Digging` stay as unused variants.
- `Mode` is saved by name. The variants carry `#[serde(rename = "Survival")]` and `#[serde(rename = "Creative")]`, so saves read and write as before.
- `PlayerData` has every field it had, including `food` and `milestones`.

### A correction to the audit

T-1 said `survival_loop()` was the only test on a seed world. That was wrong. Three more reached `load_or_generate`: `a_second_join_with_nothing_changed_downloads_nothing` and `streamed_files_arrive_during_play_without_holding_it_up` (`tests/streaming.rs`) and `an_instance_with_a_different_pack_drains` (`tests/cluster.rs`). Each wrote the pack as clients receive it, which has no layout, into a directory and started a server on it. They now write the pack as it is authored (`Pack::builtin_source`), layout included, and test what they tested before.

### Checked against main as it moved

The work was merged with main twice (the flow designer's rules, quest chain and layout 3; then the UI branch).

- **Quests and the cut recipes.** No quest step asks for anything to be crafted. Two quests give a stone pickaxe and torches as rewards, and Hollis sells the stone pickaxe and axe: the items are all still there. Only the recipes that made them are gone.
- **Bots and playtests.** `dyno bots` and `dyno play` gather with the `Gather` cast. Nothing in `dyno-bot` sends `Dig`, `DigStart`, `Place`, `Interact` or `Throw`.
- **The UI branch.** The air meter is `hud::air_meter` in `app/hud.rs`. The death text is the UI branch's; its unprotected-world arm and the two strings only that arm used (`death.hint_survival`, `death.respawn`) were removed. The crafting window is one function, `item_ui::craft_window`, with plain widgets.

### Results

Before is main at `4de5b40`; after is this branch on top of it.

| Check | Before | After |
|---|---|---|
| Client (`hearthblock`, client-release) | 9,507,656 bytes | 9,500,064 bytes |
| First join | 3,746,571 bytes | 3,745,680 bytes |
| Pictures (`dyno bench --diff`, high tier, six cameras) | | Three identical; three with a largest difference of 3, 18 and 69 on a few pixels: mean 0.000, 0.000% of pixels over 2. The floor in `performance.md` is 0.000 to 0.001% |
| Live save (two characters, 24 fields each) | loaded | every field identical; chests and world bytes the same |
| Fun report, 12 bots for 10 minutes | fails: deaths, kill credit | fails: kill credit |

The fun reports differ in two checks, both of them counts of rare events in two bot-hours: deaths per hour went from 5.52 (fail) to 3.02 (warn), and stuck events per bot-hour from 1.00 (pass) to 2.01 (warn, the limit is 2). The other twenty checks kept their result. No bot used the recall that replaces `/spawn`.

Gates on the last code commit: `cargo fmt --all --check` clean, `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo test --workspace` 216 passed and none failed (12 ignored), `check-client.sh` passed.

### What `grep -ri survival crates content docs tools` returns now

1. `crates/dyno-protocol/src/lib.rs`: the `#[serde(rename = "Survival")]` line.
2. `docs/design/rpg.md`: the record of the pivot (the lines that say what was built for survival and what replaced it).
3. `docs/design/survival-audit.md`: this file.
