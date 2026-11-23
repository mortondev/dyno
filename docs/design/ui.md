# The interface

How the HUD, the panels and the tutorial tips are built, the rules they
follow, and what is still missing. The acceptance lines are sections 1–3, 8
and 9 of `quality-bar.md`; the table at the end reports against them.

## The system

| Part | Where | What it is |
|---|---|---|
| Theme | `dyno-client/src/theme.rs` | The palette, the three fonts (title, body, bold) and painter pieces: windows, tabs, parchment, leather, slots, bars, the action-bar frame, tooltips. Everything is drawn with shapes; there are no interface images. |
| Words | `content/ui/strings.json`, read by `uitext.rs` | Every word the HUD and panels show. `uitext::t("hud.recall")` reads a dotted key; `uitext::f(key, &[("n", "3")])` fills `{n}` places. The file is parsed once and again when the pack changes. A missing key is empty text. |
| HUD | `app/hud.rs` (a child of `app.rs`, so it sees the play state) | Unit frames, action bar, cast bar, the moments (level gained, quest step, loot), the death screen, the game menu, and the tips' events. |
| Panels | `screens.rs`, `rpg_ui.rs` | Character, bags, spellbook, skills (one window with tabs); quest dialogue, quest log, tracker, merchant, loot. |
| Tips | `tips.rs` | The tutorial layer (below). |
| Options | `options.rs` | Graphics, and the interface's size and tips. What the interface remembers (size, tips seen, each character's action bar) is saved in the settings file. |
| Front end | `front.rs` | Title, options, character select and creation. |

The client holds no content. The interface knows *kinds* of things (an
ability, an item, a quest objective) and reads their names, numbers and
descriptions from the rules; it reads its own words from the strings file.
A new pack can rename every label without a client release.

### Size

The interface is laid out for a window 1000 points tall
(`options::zoom_for`). A 1080p window draws it at 1.08 device pixels a
point and a 1440p window at 1.44, so every frame, slot and letter takes the
same share of the screen on both. The player's Size option (80% to 130%)
multiplies that. Headless shots (`dyno shot`) use the same rule, so a shot
is what a player sees.

### Layout of the HUD

```
 player  target  (combo)                          buffs   zone name
 party                                                    minimap, clock
                    title / level banner                  quest tracker
                    toasts
                    quest-step lines

                              floating combat text
                                 XP and coins
 chat                                                     things received
                         cast bar
                         XP bar
                 [ brazier | 12 action slots | brazier ]
```

Nothing overlaps at 1080p or 1440p: each region is anchored to its own
corner or to the action bar. F3 puts the debug line under the unit frames.

## Style rules

1. **One material per job.** Windows are dark carved wood with gold trim
   (`theme::window`); text to read at length is on parchment
   (`theme::parchment`); bags are stitched leather (`theme::leather`); a slot
   is a well with a bronze rim (`slot_well`, `slot_rim`, `item_slot`).
   Do not invent a new box: use these.
2. **Three fonts.** The title face for names and headings (small caps), the
   body face for sentences, the bold body face for numbers. Never
   `FontId::proportional` or `monospace` in the RPG interface.
3. **Gold means "yours to act on".** Bright gold: titles, the primary
   button (one per window), quest marks. Dim text is help. Green is done or
   better, red is worse or cannot. Item names take their quality colour and
   nothing else does.
4. **Class resources keep their colours everywhere:** rage red, energy
   yellow, mana blue, health green (`theme::RAGE` and friends).
5. **Words are data.** A new label is a new key in `strings.json`. Numbers
   go in through `{n}` places, so a translator can move them.
6. **Text over the world has a shadow** (`theme::shadow_text`,
   `title_text`), and a banner that must be read has a dark band behind it.
7. **A panel explains itself in one dim line** at its top, and every icon
   has a tooltip. Tooltips are built from the thing's data, never typed.
8. **Moments are brief and never block.** Banners, toasts and tips are
   drawn over the world and take no input (a tip can be clicked away).
   Only a window the player opened, death and the game menu take the mouse.
9. **Every panel opens on its key, closes on the same key and on Esc,** can
   be dragged, and stays where it was put for the session.

## The HUD, piece by piece

- **Unit frames** (`rpg_ui::unit_frame`): a portrait medallion with the
  level, the name, health with numbers, and for the player the class
  resource in its colour. The target sits beside the player's frame: a
  creature's level is coloured by danger, an elite says so under its name,
  and an NPC shows its title. Rogue combo points are five diamonds on the
  target's right edge. Party members are small frames under the player's.
- **Buffs** are a row left of the minimap with the time left under each;
  pointing at one names it.
- **Action bar** (`hud::action_bar`): twelve slots on keys 1–0, - and =.
  Each shows its key, a clockwise cooldown sweep with seconds on long
  cooldowns, a red tint when you lack the resource or the target is out of
  range, and a gold rim while it is being cast. Pointing shows the ability
  tooltip. Click to use; drag to another slot to swap; drag off the bar to
  clear; drag from the spellbook to bind. A newly learned ability goes on
  the first free key by itself and pulses gold for a while. The bar is
  saved per character.
- **Cast bar** above the XP bar; **XP bar** with the rested band above the
  action bar.
- **Floating combat text** (`hud::combat_text`): white damage, a critical
  hit lands large and settles, damage you take is red, a heal is green with
  a plus. Numbers drift apart so a flurry stays readable.
- **Moments** (`hud::moments`):
  - *Level gained:* a warm flash, the level in the title face over a dark
    band, the health gained, and each new ability with its icon and the key
    it was put on.
  - *Quest steps:* a bright line in the upper middle for each objective
    count ("Grazer slain: 5/10"), for a quest taken, ready and handed in.
  - *Things received:* icon and name in the quality colour at the lower
    right, worked out from what you carry (no message needed).
  - *Experience and coins* rise beside the character.
  - *Toasts and the big title* (a place discovered, dusk) share the top
    middle.
- **Death:** what killed you, what it costs (nothing you carry), and one
  gold button.
- **Game menu (Esc):** return, options, controls, leave, quit. Options has
  graphics and interface (size, tips). Controls lists every key from
  `keys.rpg` in the strings.

## Panels

- **Character (C):** the paper doll's 17 places around the stat sheet. Each
  stat says what it does when pointed at (`sheet.*.text`).
- **Bags (B):** 36 slots on leather with quality borders. Right-click
  wears, eats or uses; at a merchant it sells. *Sort* joins stacks and
  orders gear, food, tools, other things, junk. *Sell junk* (at a merchant)
  sells every grey item and shows the sum first.
- **Item tooltips** (`screens::item_tooltip_vs`): the classic layout, then,
  for gear you can wear, a comparison with what is worn in that place:
  a green up arrow or a red down arrow for damage per second, armour and
  each stat. Bags, merchants, loot and quest rewards all show it.
- **Spellbook (P):** every ability of the class in level order with its
  icon, one-line effect, and either the key it is on, "not on your bar" or
  the level it arrives. The next one to learn is named at the top. Drag a
  known ability to the action bar.
- **Ability tooltips** (`screens::ability_tooltip`): name; cost and range;
  cast time and cooldown; the effect line. All from the ability's data.
- **Skills (K):** each profession with a bar.
- **Quest dialogue, log (L), tracker, merchant, loot:** `rpg_ui.rs`.

## The tutorial tips

A tip is a short panel with a gold `!`, a title and two sentences, shown
once per install the first time something happens. An arrow points from it
to the part of the HUD it is about. Tips are data: `tips.list` in
`strings.json`.

```json
{ "id": "bar", "on": "combat", "anchor": "bar", "until": "", "secs": 0,
  "title": "Your abilities",
  "text": "You swing by yourself. Press 1, 2, 3 (or click) to use an ability. …" }
```

| Field | Meaning |
|---|---|
| `id` | Kept in the settings once shown, so it never shows again |
| `on` | The event that queues it |
| `until` | An event that puts it away early (the move tip goes when you move) |
| `anchor` | The HUD part it points at |
| `secs` | How long it stays (default 14) |

**Events** the engine fires: `spawn`, `moved`, `dialog`, `quest_accepted`,
`quest_step`, `quest_ready`, `quest_done`, `target`, `combat`,
`combat_rage`, `combat_energy`, `combat_mana`, `combo`, `xp`, `loot`,
`item`, `gear`, `bags`, `spellbook`, `character`, `skills`, `vendor`,
`level`, `ability`, `low_health`, `death`, `rested`, `party`.

**Anchors:** `frame` (player), `target`, `bar`, `xp`, `minimap`,
`tracker`, and `center` (above the action bar, no arrow).

Rules for the layer:
- one tip at a time, with a pause between two; later ones wait in a queue;
- none during the intro, a level banner, the game menu or while a quest is
  being read;
- a tip whose `until` event has already happened is skipped and counts as
  seen;
- a click closes it; with the mouse held for steering it leaves by itself;
- Options → Interface → Tips turns them off, or shows them all again.

Adding a tip for an existing event needs only the pack. A new event is one
line in `hud::tips` (or where the message arrives).

The first half hour as the tips tell it: move → a gold ! is a quest → the
map → your quests → your target → your abilities → your resource →
experience → loot → into your bags → something to wear → ready to hand in
→ a new level → a new ability.

## Looking at it

`dyno shot ADDR --ui NAME --as CHARACTER --size WxH` draws one state. Beside
the older names (`hud`, `talk`, `log`, `char`, `bags`, `spells`, `skills`,
`vendor`, `loot`, `dead`) there are:

| Name | Shows |
|---|---|
| `combat` | Target, cast bar, cooldown, buff, combo points, floating text, a quest step, things received |
| `levelup` | The level banner with a new ability |
| `tooltip` | An item compared with what is worn, and an ability tooltip |
| `party` | Party frames |
| `menu`, `menu-options`, `menu-controls` | The game menu's pages |
| `tip:EVENT` | The tip for an event, for example `tip:combat` |

`--as NAME` also finds a character that `dyno play` or a bot made.

## Against the quality bar

| Line | State |
|---|---|
| 1 Title looks finished, no debug text | Met (unchanged; now sized for the screen) |
| 2 Class cards: fantasy line, role, difficulty | Met. Roles and difficulty are still a table in `front.rs` keyed by class: they need to come in `ClassInfo` |
| 2 Portrait and preview in the starter kit | Not met: the preview wears a tunic in the class colour. Needs the starter items in `ClassInfo` |
| 2 Names: random button, clear rule, no profanity | Met on the client (`names.blocked`); the server should refuse the same list |
| 3 Tip on moving, talking and the gold ! | Met (`move`, `talk`) |
| 8 Unit frames: health, resource, level, name | Met for player and target; party frames have health only (the party message has no resource) |
| 8 Buffs with timers | Met for the player. Target debuffs need the server to send a creature's auras |
| 8 Action bar: keys, drag and drop, new abilities placed | Met |
| 8 Cast bar, XP bar with rested, class resource | Met |
| 8 Minimap: zone, clock, quest marks, zoom | Met (unchanged) |
| 8 Floating text: damage, crits, heals, XP, loot | Met; heals are read from health rising, so a small heal over time is not shown |
| 8 Quest-progress toast, level-up banner | Met |
| 8 Nothing overlaps at 1080p or 1440p; can be scaled | Met |
| 9 Same theme on every panel, key and Esc, remembers its place | Met for the session; the class picker shown to a character with no class, the trade window's buttons and the chat box still use plain widgets |
| 9 Quest log: objectives, rewards, story, abandon | Met; share is missing (needs a message) |
| 9 Bags: sorting, stacking, quality colours, sell junk | Met |
| 9 Item tooltips compared with worn gear | Met |
| 9 Character sheet: gear, stats explained | Met; the doll shows icons, not a 3D figure |
| 9 Spellbook: drag to bar, next ability | Met |
| 9 Merchant: buy, sell, buyback | Buy and sell; no buyback (server) |
| 9 Death: release and timer | Release; no timer (the server has none) |
| 9 Chat channels, combat log | Present; plain styling |
| 9 Map | Missing: only the minimap |
| 9 Options with every setting and keybinds | Graphics, interface size, tips; the keys are listed but cannot be rebound |

## What is next

1. A world map (M) with quest pins (the minimap renderer can draw a larger
   area; it needs land the client has not streamed yet).
2. Rebinding keys (the Controls page is a list today).
3. Wire additions, to do together in one protocol bump: `ClassInfo` with
   roles, difficulty and starter items; a target's auras; party members'
   resource; quest share; merchant buyback; a release timer.
4. The front end's own words into the strings file (it runs before the
   first pack arrives, so it needs a fallback first), and the remaining
   hard-coded lines in `rpg_ui.rs` (quest dialogue, merchant, loot) and the
   combat log.
5. The class picker, trade window and chat box on the theme.
