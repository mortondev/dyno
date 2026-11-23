# The quality bar: the first 30 minutes

The first 30 minutes of Hearthblock Online should feel like a polished
classic MMO opening. This is the checklist every lane is judged against: by
the swarm (`dyno bots`), by hand play (`dyno play`), by the visual gauntlet,
and by the reviewers. Each line is meant to be checkable.

## 1. Launch and first impression
- The client downloads and starts in under 10 s on a normal connection. The first join's download is under 4 MB, and the second join's is 0.
- The title screen looks finished: logo, music, a living backdrop, and no debug text.
- The game picks a graphics tier for the machine; nobody's first frame is a slideshow.
- No crash, hang, or "connection lost" in 30 minutes of play, on any of the four platforms.

## 2. Character creation
- Each of the six classes has a card that sells its fantasy in one line, with a portrait in its starter kit and its role (tank, healer or damage) and difficulty.
- Appearance: body, face, hair and colours, with a live preview in the class starter kit, not the bare body.
- Names: a random name button, a clear rule when a name is refused, and no profanity.
- It takes under 60 s from title to in-world for a player who doesn't linger.

## 3. Arrival (minutes 0–2)
- You spawn in Hearthstead's plaza, facing the hearth and a quest giver with a gold "!". The music swells and the town is alive: townsfolk at work, smoke, birds.
- A short contextual tip says how to move, how to talk (right-click) and that "!" means a quest.
- The first quest is accepted within 30 s, needs no reading to understand, and starts within sight.

## 4. Quests and flow (minutes 0–30)
- **A clear next step:** you always know what to do next. The quest tracker on the HUD shows objectives and progress, and the map and minimap mark targets.
- **Variety:** in the first 30 minutes there are at least four kinds of objective (talk, kill, collect, use or interact, explore, escort or defend), and kill-10 never appears twice in a row.
- **A breadcrumb chain:** it leads you out of town hub by hub. The first trip out takes under 60 s of running, and each new place is introduced by a quest.
- **Short quest text:** two to four sentences, with the objective restated in one line and a little character in every NPC.
- **Turn-ins feel good:** a sound, an XP flash, and a reward choice where it makes sense.

## 5. Pace and reward (the rhythm)
- **Frequent rewards:** something rewarding at least every 60–90 s (XP tick, loot, quest step, discovery, new ability, gear). Never more than 2 minutes of dead time.
- **Level milestones:** level 2 at about 5 minutes, level 3 at about 12, and level 4 to 5 at 30. Each level-up is a moment: a flash, a sound, the stats and any new ability shown.
- **Gear:** the first upgrade by about 10 minutes, and a visible change on the character (the armour sets) by about 30.
- **Money:** enough for one meaningful vendor purchase by 30 minutes.
- **Exploration:** discovering a named place gives XP and a banner; a treasure or a view rewards going off the road.

## 6. Combat and classes
- **The fantasy reads within a minute:** warrior rage and charge, paladin seals and heals, hunter ranged and traps, rogue stealth openers and combo points, priest heals and shields, mage frost and fire control.
- **Distinct abilities:** two to three by level 5, each with a clear purpose, and one new ability at level 2–3 and another at 4–5.
- **Feel:** a responsive global cooldown, and hits that land. You get a swing animation, an impact effect, a sound, the target reacting, floating damage, and crits that are visibly bigger.
- **Resources matter:** rage builds and decays, energy ticks, mana must be budgeted, and combo points show.
- **Fair difficulty:**
  - a single same-level mob is safe;
  - two mobs are dangerous but winnable with your kit;
  - deaths are rare (under 1.5 an hour) and always explicable;
  - the classes are within about 20% of each other in kill time and deaths.
- **Tagging, loot and XP shared in a party** work without explanation.

## 7. Abilities (presentation)
- Icons read at 40 px and are distinct within a class.
- Tooltips give the name, cost, cooldown, range, cast time and a one-line effect, worded consistently and generated from data.
- Every ability has a cast effect, an impact effect and a sound.
- Cooldowns sweep on the button, out-of-range or out-of-resource buttons tint, and keybind numbers show.

## 8. HUD
- **Unit frames:** player, target and party, with health and resource, level, name, and buffs and debuffs with timers.
- **Action bar:** 1–0 and -=, drag and drop from the spellbook, and new abilities placed automatically.
- **Bars:** cast bar, XP bar with rested XP, and the class resource bar.
- **Minimap:** zone name, clock, quest markers, and zoom.
- **Combat feedback:** floating combat text (damage, crits, heals, XP, loot), the quest-progress toast, and the level-up banner.
- **Uncluttered:** nothing overlaps at 1080p or 1440p, and the UI can be scaled.

## 9. Menus and panels
- Every panel uses the same theme (gold frames, title font, dark parchment), opens and closes on its key and Esc, and remembers where it was.
- **Quest log:** objectives, reward preview and the story text, with abandon and share.
- **Bags:** sorting, stacking, item quality colours and a sell-junk option.
- **Item tooltips:** stats compared with what you're wearing, with green and red arrows.
- **Character sheet:** a paper doll showing the real gear, plus stats with explanations.
- **Spellbook:** every learned ability, with drag to the bar, and the next ability shown with its level.
- **Other panels:** skills, talents (when they arrive), merchant (buy, sell, buyback), loot window, trade, death screen (release and timer), chat with channels, combat log, map, and the options with every setting and keybinds.

## 10. World and look (the visual gauntlet)
- Every scene meets the concept rubric (docs/art/gauntlet.md) at 7 or above: lighting, atmosphere, voxel quality, water, vegetation, scale, life and composition.
- The scale feels like WoW (docs/design/scale.md): roads, doors and trees in proportion, and hubs 2–3 minutes apart with something every 100–150 m.
- Every landmark is recognisable from its approach, and every zone has its own mood and colour.

## 11. Life and sound
- Townsfolk work, wildlife moves, smoke rises, and nights have fireflies and wisps.
- Music per zone and for combat, ambient sound per zone, footsteps per ground, and a UI sound for every action.

## 12. Social
- **Other players are visible** in their gear, with name plates, and the town feels busy.
- **Grouping:** invite, party frames, shared tags, XP and quest credit, and the group bonus all work. There is at least one reason to group in the first 30 minutes (an elite, or a group quest).
- **Chat:** General and Trade channels, whispers and emotes.

## 13. Performance and stability
- **Frame rate:** 60 fps at 1080p on a mid-range GPU at the default tier, with no hitch over 20 ms while streaming or joining (docs/design/performance.md).
- **Time to play:** under 1 s on rejoin. Land streams around you before you see empty space, and the horizon fills within about a second.
- **Server:** the tick stays under budget with 40 players in an area, and movement feels local, with no rubber-banding.

## How it is measured
| Area | How it is checked |
|---|---|
| Pace, rewards, deaths, idle time | `dyno bots` fun report per class and per brain (docs/playtest/fun-metrics.md) |
| Flow, clarity, feel | `dyno play` hand runs, written up in docs/playtest/ |
| Look | the visual gauntlet judge (docs/art/gauntlet/) |
| HUD and menus | per-screen shots (`dyno shot --ui …`) reviewed against sections 7–9 |
| Performance | `dyno bench` and the budgets in docs/design/performance.md |
| Stability | CI on four platforms, cluster tests, and soak runs with the swarm |
