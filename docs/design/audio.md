# Sound

What the game sounds like, how it is built, and how to check it. This is
the listening guide for section 11 of the quality bar
(docs/design/quality-bar.md): music per zone and for combat, ambient
sound per zone, footsteps per ground, and a UI sound for every action.

Nobody has listened to the music or the ambience yet. They were made and
checked by measurement only (levels, spectra, loudness, seams), and the
tunes were written by rule, not by ear. Listen to the montages in
`docs/audio/montages/` before trusting any of it.

## How it works

- **The pack describes, the engine mixes.** `content/audio.json` lists
  every *event*: a key such as `ui.open` or `step.grass`, its variant files
  (`<file>_<n>.wav`, 16-bit PCM), its bus, its level, how far its pitch may
  wander, and the least time between two plays. The client holds no sound
  and no synthesis: `audio.rs` decodes WAV, mixes, and nothing else.
- **Game code says what happened, by key.** `Audio::play("ui.buy", 1.0)`,
  `Audio::play_at("combat.hit", place, 1.0)` for a sound with a place in the
  world, or `audio::emit("ui.loot")` from code with no `Audio` to hand (the
  HUD, panels). An event the pack does not describe is silent, and a test
  fails on it.
- **Variants.** A repeated event has three or more variants. One is picked
  at random each time, never the same one twice running, and its pitch
  wanders by a few percent, so a run of steps or blows does not repeat.
  Three cues are marked `once` and have one variant, because each is a
  signature heard a handful of times in the first half hour: the level-up,
  the new ability, and your own death.
- **Sounds the rules name.** An ability names its sound in the rules
  (`"sound": "frost"`, the class lane's data and files:
  `sounds/abilities/<name>_<n>.wav`). `Audio::play_named_at` plays it
  through the same mixer as `audio.json`'s `named` entry says (bus, level,
  pitch); until the file has arrived the plain bolt or burst plays instead.
- **Streaming.** Steps, the interface and blows (`sounds/step/`,
  `sounds/ui/`, `sounds/combat/`) come with the join. Everything else
  (`sounds/world/`, `sounds/amb/`) streams: the first time an event plays
  with files missing, the client asks for every variant
  (`assets::request`) and plays whichever have arrived. `Audio::want(key)`
  asks ahead of time; digging does this when you start on a block, so the
  break is heard.
- **The mixer** (`audio::Mixer`) runs on the audio thread, fed by messages.
  It has four buses (music, effects, ambience, interface) under a master
  level, then a limiter. With no device it renders on request
  (`Audio::offline`): the tests and the montages hear exactly what a player
  would.
- **Placed sounds** fade to nothing at `reach` metres (32 by default) and
  lean to the side they are on (equal power, never wholly in one ear).

## Ambience

The sound of a place is three kinds of thing, all described in one
streamed file, `sounds/amb/ambience.json`, with rules written as the life
lane writes its own (`life::Where`: areas of the rules and a window of the
day), plus height above the hearth, weather, and whether there is sky
overhead:

- **Beds** go round for as long as their rule holds, and several lie over
  each other: the air of the day, the night's crickets, leaves in the
  birch woods, the Barrow wood's moan, a town's murmur by day, the mine,
  wind on high ground, rain. A bed comes and goes over 3 s as you cross
  into its place or its hour (day and night overlap at dawn and dusk).
- **Voices** are heard now and then from somewhere round about, each from
  its own side: birds by day, an owl and near crickets by night, frogs and
  gulls by the lake, crows in the Barrow wood, drips down the mine, a gust.
- **Things that stand** sound where they are: the nearest hearth, brazier
  or furnace crackles; an anvil rings every few seconds by day; water with
  air above it laps (in the lake's areas) or runs (elsewhere). The client
  looks for them in the blocks within 24 m, a slab of the neighbourhood a
  frame (`ambience::Search`), and the nearest of each kind is heard, to
  its side and fading with distance. This does not depend on the Wildlife
  setting.

What each of the gauntlet's 18 places resolves to (a test prints this and
fails if any place has no bed at any of nine hours):

| Place | Beds at noon | Beds at midnight | Theme |
|---|---|---|---|
| hearthstead_plaza | amb.air + amb.town | amb.night | hearthstead |
| hearthstead_street | amb.air + amb.town | amb.night | hearthstead |
| hearthstead_dusk | amb.air + amb.town | amb.night | hearthstead |
| watch_tower | amb.air + amb.town | amb.night | hearthstead |
| north_watch | amb.air + amb.leaves | amb.night + amb.leaves | birchrise |
| logging_camp | amb.air + amb.leaves + amb.town | amb.night + amb.leaves | birchrise |
| hollow_oak | amb.air + amb.leaves | amb.night + amb.leaves | barrowwood |
| old_barrow | amb.air + amb.eerie | amb.night + amb.eerie | barrowwood |
| barrow_crypt | amb.air + amb.eerie | amb.night + amb.eerie | barrowwood |
| wrecked_caravan | amb.air | amb.night | hearthstead |
| emberford_bridge | amb.air | amb.night | hearthstead |
| windmill_farm | amb.air | amb.night | mirrormere |
| fishers_landing | amb.air + amb.town | amb.night | mirrormere |
| mirrormere | amb.air | amb.night | mirrormere |
| sunken_shrine | amb.air | amb.night | mirrormere |
| ashcloak_stockade | amb.air | amb.night | hearthstead |
| miners_rest | amb.air + amb.town | amb.night | mines |
| hollow_mines | amb.air | amb.night | mines |

The camera called `hollow_mines` stands at the mine head, outdoors, so it
hears the valley; down the mine itself (inside the rules' area, no sky)
the only bed is `amb.mine`, at every hour.

## Music

**Why a tracker.** The client has no decoder for compressed sound and
should not grow one (it is held under 10 MB, and every theme as a
recording would be megabytes on the wire even compressed). So music is
data the client plays: a song is a few kilobytes of patterns (which
instrument plays which note on which row), and the instruments are 17
short samples shared by every song, pitched by playing them faster or
slower (`tracker.rs`). Nine songs and all their instruments are 0.9 MB on
disk, streamed when first wanted; a new theme costs about 5 KB.

- **Instruments** (`music/inst/*.wav`, `tools/audio/instruments.py`):
  strings, flute, horn and voices are each one seamless loop (every
  partial makes a whole number of cycles in the sample), held for as long
  as a note lasts with a swell and a release; lute, bell and drums ring
  out once. Each has two or three recordings across its range.
- **Songs** (`music/*.json`, `tools/audio/compose.py`): tempo, the
  instruments with their levels and places left to right, patterns of
  notes, and the order of the patterns. A small room (reverb) is part of
  the player.
- **Which plays where** (`music/music.json`): a theme per area of the
  rules (the smallest area you stand in that has one), a few spots round
  points, the wilds' theme, the title's, the combat layer and the
  stingers.

| Theme | Where | What it is |
|---|---|---|
| `title` | the title screen and its menus | the valley's tune in D with horns, strings, lute, flute and drums |
| `hearthstead` | Hearthstead, its meadow, the windmill, the east road, the wilds | the same tune on lute and flute over strings, 84 beats a minute |
| `birchrise` | Birchrise, the North Watch, the logging camp | a slow waltz in E minor: lute, low flute, a bell |
| `barrowwood` | the Barrow wood, the Hollow Oak | voices on a drone that will not resolve, a bell out of key, no pulse |
| `mirrormere` | Mirrormere, Fisher's Landing, the Sunken Shrine | slow harp rising and falling in F with a raised fourth, long flute notes |
| `mines` | the Hollow Mines, the mine head, Miner's Rest | C minor: picks with their echoes, a low horn, voices |
| `combat` | over the theme while you fight | drums only, so it sits over any theme in any key |
| `combat_sting` | as a fight starts | two drum blows and a low horn |
| `discovery` | a named place entered for the first time | the first notes of the valley's tune, opening upward |

- **Crossing.** When you have been in another theme's place for a second
  and a half (so a border walked along does not flip the music), the new
  theme rises as the old one falls, over 4 s. Each deck's level moves a
  little every sample, and a song put off its deck by another fades in
  six hundredths of a second rather than stopping, so there is no step to
  click however restless the player.
- **A fight.** When you attack, or are hurt with a hostile creature within
  24 m (a fall is not a fight), the drums come in over 1.2 s, the theme
  steps back to 45%, and the sting plays. Five seconds after the last blow
  the drums leave and the theme returns.
- **Ducking.** While a stinger sounds (until its last note has died away),
  and for the length of the level-up and quest-complete fanfares (their
  `duck` seconds in `audio.json`), the themes and the drums dip to 35%
  (`audio.json`, `duck`), down in a fifth of a second and back over a
  second and a half. The discovery stinger holds them down for 8 s in all.
- **Discovery** is the client's: the first time in a session that you
  enter a named place, other than the one you arrive in. It is not tied to
  the server's exploration reward.

The level-up and quest-complete moments keep their cues from the first
hand-in (on the interface bus) rather than gaining a second, musical
stinger; the music ducks under them instead.

## The mix

**Sliders.** Options has five: Master, Music, Effects, Ambience and
Interface. They are heard while dragged and saved when let go
(`settings.json`: `volume`, `volume_music`, `volume_effects`,
`volume_ambience`, `volume_ui`). A slider is loudness, not signal: the
mixer gets its square, so half way is a quarter of the signal. A new
player has Master 80%, Music 70%, Effects 100%, Ambience 80%, Interface
90%.

**The limiter** looks 1.5 ms ahead: it sees each peak before it plays,
turns down smoothly in that moment and lets go over 0.12 s. Its ceiling is
0.89 (-1 dBFS): nothing leaves louder, so nothing clips, and no peak is
cut flat. It is a guard and not part of the sound: at a new player's
levels it is idle even in the worst case, and at full sliders the worst
case barely reaches it.

**Targets** (`audio.json`, `targets`), with what was measured. The
targets were chosen after the first measurements, as the levels to hold
from here on, not before them. "Peak" is
the loudest single event on the bus with every slider at full; a test
holds every event to it. "Loudness" is the level (RMS) of the bus's
loudest 3 seconds in the worst-case scene; a test holds it within 4 dB.
It is not a broadcast loudness meter (no ear weighting).

| Bus | Peak target (dBFS) | Loudest event, measured | Loudness target (dB) | Measured |
|---|---|---|---|---|
| Effects | -5 | -5.3 (`combat.crit`) | -20 | -19.2 |
| Interface | -7 | -7.1 (`ui.level_up`) | -21 | -19.1 |
| Ambience | -12 | -15.4 (`amb.fire`, beside it) | -28 | -26.0 |
| Music | -10 | (songs, not events) -11.6 in the worst case | -24 | -24.2 |

The worst case is the fight of `6_everything` (three foes, the interface
busy, a level-up and a quest handed in at once, your death) set in
Hearthstead's plaza in heavy rain, beside the hearth fire and the anvil,
with the town's theme, the combat drums, the combat sting and the
discovery stinger all playing. Measured through the mixer:

| Sliders | Peak (dBFS) | Loudness (dB) | Limiter | Clipped samples |
|---|---|---|---|---|
| All at full | -1.0 | -16.1 | took off 1.3 dB at most | 0 |
| A new player's | -4.0 | -21.4 | idle | 0 |

Footsteps and routine interface sounds sit well under a fight on purpose:
a walk measures -35 dB and an errand at a merchant -33 dB against a
fight's -24 dB.

## Listening guide: what plays when

### Footsteps

One step every 2 m you run, chosen by the block under your feet
(`audio.json`, `footsteps.blocks`, by block key; `default` for a block not
listed; `wade` when your feet are in water).

| Ground | Event | Blocks | What it should sound like |
|---|---|---|---|
| Grass | `step.grass` | grass, leaves, thatch, hay, flower beds | a soft thump and a rustle of blades |
| Dirt | `step.dirt` | dirt, clay, anything not listed | a dull, low pad with a little grit |
| Stone | `step.stone` | stone, cobble, flagstones, bricks, roofs, ore, ice | a hard, short click with a ring |
| Wood | `step.wood` | planks, logs, beams, furniture | a hollow knock on a board |
| Sand | `step.sand` | sand, cloth blocks | a two-part hiss, heel then toe |
| Water | `step.water` | water, and wading | a splash with small bubbles |
| Snow | `step.snow` | snow | a crunch of many small cracks, with a squeak |

### Interface

| When | Event | What it should sound like |
|---|---|---|
| Any button, tab or slot is clicked | `ui.click` | a small wooden tick |
| A panel opens (bags, character, quest log, merchant, loot, trade, menu) | `ui.open` | parchment drawn out and a clasp |
| A panel closes | `ui.close` | the page falling back and a soft thud |
| You pick an item up from the ground, or craft one | `ui.item_pick` | cloth and a small clink |
| You move, equip, sort or throw an item | `ui.item_drop` | a leather thud |
| You buy | `ui.buy` | four coins counted out, falling |
| You sell | `ui.sell` | four coins dropped in, rising |
| You are given money | `ui.coin` | two coins |
| New items reach your bags | `ui.loot` | two plucked notes and a sparkle |
| You press an ability you cannot use (still cooling down, too little rage or mana, or the target out of range) | `ui.error` | two dull knocks on wood |
| A quest is taken | `ui.quest_accept` | two plucked notes rising a fifth |
| A quest objective moves on; a journal milestone | `ui.quest_step` | one small bell note |
| A quest is ready to hand in | `ui.quest_ready` | three plucked notes up the chord |
| A quest is handed in | `ui.quest_complete` | a rolled chord, bells, a warm horn fifth |
| You reach a level | `ui.level_up` | a two-octave rise into a held chord with horns and a drum |
| A level brings a new ability (2.4 s after the level's fanfare) | `ui.ability_new` | a glassy rise and one sure plucked note |
| Chat, a trade request, a party invitation | `ui.chat` | (the earlier ping) |
| You or another player joins | `ui.join` | (the earlier chime) |

When several quest changes arrive together, only the weightiest is heard:
handed in, then ready, then taken, then a step.

### Combat

| When | Event | Placed | What it should sound like |
|---|---|---|---|
| Your weapon swings (your auto-attack, an instant weapon ability) | `combat.swing` | no | a short rush of air |
| Another player or a creature swings | `combat.swing` | at them | the same, quieter with distance |
| Anyone looses an arrow | `combat.shoot` | at them | a bowstring and the arrow leaving |
| A creature is hurt | `combat.hit` | at it | a thud and a smack, a trace of blade |
| Your blow is a critical | `combat.crit` | at it | a bigger blow, a low drop and a metal ring |
| You are hurt | `combat.hurt` | no | a dull blow close to, with the rattle of your gear |
| You are healed (a heal from the server, or 3 health or more at once) | `combat.heal` | no | a soft rising chord with air |
| A creature dies (a hostile one, or any you were fighting) | `combat.kill` | at it | the fall and a breath out |
| You die | `combat.death` | no | a low drum, three falling notes |
| An ability goes off | the ability's own sound (the rules' `sound`) | at the caster | the class lane's library, `sounds/abilities/` |
| ...and its sound has not arrived, or it has none: a bolt | `combat.bolt` | at the caster | a falling zip of air |
| ...or a burst | `combat.burst` | at the caster | a low boom with a shimmer |

Creatures have no voices: the server does not say that a creature has
noticed you (see "Not yet").

### The world (earlier sounds, now streamed)

Breaking, placing and digging by material (`break.*`, `place.*`, `dig.*`),
the low-health heartbeat, the dusk horn and the dawn chime are the sounds
of the first bake, in `sounds/world/`, played through the new mixer a few
dB under their old levels at a new player's sliders. Two of the breaking
sounds (glass and stone) had clipped samples; those and foliage are made
anew. The first bake's bird, cricket, rain, wind and hammer are gone:
the ambience above replaces them.

### Ambience

| Sound | Heard | What it should sound like |
|---|---|---|
| `amb.air` (bed) | outdoors by day, everywhere but the mine | the least breeze, grass moving |
| `amb.night` (bed) | outdoors by night | a field of crickets over still air |
| `amb.leaves` (bed) | Birchrise, the logging camp, the North Watch, the Hollow Oak | leaves hissing as the air moves |
| `amb.eerie` (bed) | the Barrow wood, indoors and out | a low moan that slides and never settles |
| `amb.town` (bed) | Hearthstead and the other settlements by day (Hearthstead a little into the evening) | people talking somewhere near, never a word |
| `amb.mine` (bed) | down the Hollow Mines | the weight of the hill, air in a shaft |
| `amb.wind` (bed) | outdoors 14 m or more above the hearth, and on the named heights (Watch Hill, Beacon Rocks, the North Pass, the Barrow Overlook, the Ridge Cut, the Broken Tower) | wind with nothing in its way |
| `amb.rain` (bed) | outdoors in rain, as strong as the rain | steady rain on leaves and ground |
| `amb.bird` (6 songs) | by day, every 3 to 9 s, 10 to 36 m off | falling notes, a trill, two notes, a warble, rising whistles, a twitter |
| `amb.owl` | by night, every 16 to 40 s | two or three hoots, far off |
| `amb.cricket` | by night, every 4 to 11 s, close | one cricket near by |
| `amb.frog` | by the lake from late afternoon to dawn | a croak or two |
| `amb.crow` | the Barrow wood and the Hollow Oak | two or three caws |
| `amb.gull` | by the lake by day | a cry, repeated |
| `amb.drip` | down the mine, every 3 to 9 s | a drip with a long echo |
| `amb.gust` | outdoors, every 22 to 50 s | a gust rising and falling |
| `amb.fire` (placed loop) | a hearth, brazier or furnace within 14 m | a low breath and wood cracking |
| `amb.anvil` (placed) | an anvil within 40 m, by day, every 1.3 to 3.6 s | a hammer blow, its ring, a small bounce |
| `amb.lap` (placed loop) | water in the lake's areas within 22 m | small waves coming in |
| `amb.river` (placed loop) | water anywhere else within 22 m | a stream over stones |

## The montages

`tools/audio/montage.sh` renders twelve scripted situations through the
real mixer into `docs/audio/montages/` (WAV, mono, 22050 Hz: the game
itself is stereo). The scripts are in
`crates/dyno-client/examples/audio_montage/`: `scenes.rs` (1 to 6) and
`places.rs` (7 to 12: a place of the gauntlet at an hour, with the real
ambience and music; no world is loaded, so the fire, anvil or water near
by is written into each).

| File | What happens | Seconds | Peak (dBFS) | Loudness (dB) | Limiter |
|---|---|---|---|---|---|
| `1_footsteps.wav` | grass, dirt, stone, wood, sand, water, snow: five running steps on each | 16.5 | -16.8 | -35.4 | idle |
| `2_interface.wav` | bags open, an item moved, closed; a merchant: two bought, two sold, a refusal | 10.8 | -13.5 | -32.6 | idle |
| `3_quest.wav` | a quest taken, a run, three things gathered, the run back, handed in | 16.0 | -8.0 | -22.9 | idle |
| `4_fight.wav` | one foe ahead: blows both ways, a crit, a bolt, a heal, a burst, the kill, the loot | 11.5 | -6.0 | -24.0 | idle |
| `5_level_up.wav` | the last blows, the kill, coin, the level, the new ability | 8.5 | -7.1 | -20.9 | idle |
| `6_everything.wav` | the worst case of the first hand-in: three foes, the interface, a level-up, your death (no ambience or music) | 13.0 | -1.0 | -17.3 | took off 0.9 dB at most |
| `7_town_morning.wav` | Hearthstead's plaza in the morning: the town's murmur, birds, the hearth fire to the right, the anvil off to the left, steps on the flagstones at 13 s, the town's theme coming in | 24.0 | -11.2 | -23.3 | idle |
| `8_barrowwood_night.wav` | the old barrow at midnight: the wood's moan, crickets, an owl, a brazier ahead, the barrow's theme | 26.0 | -13.6 | -26.3 | idle |
| `9_lakeside_sunset.wav` | Mirrormere's shore from late afternoon into dusk: the lake lapping to the right, gulls then frogs, day's air giving way to crickets from 8 s, steps on sand and into the shallows, the lake's theme | 26.0 | -12.1 | -24.9 | idle |
| `10_mines.wav` | down the Hollow Mines: the hill's weight, drips with long echoes, a brazier behind to the left, steps on stone at 10 s, the mines' theme | 24.0 | -12.9 | -26.0 | idle |
| `11_fight_ducking.wav` | the east road, the valley's theme; at 6 s a fight (the sting, the theme steps back, drums over it); the kill at 13.7 s; at 15 s the level-up fanfare with the music dipping under it; from about 19 s the drums leave and the theme returns | 28.0 | -5.7 | -20.5 | idle |
| `12_title_to_world.wav` | the title theme; at 10 s you arrive in the plaza and the town comes up; from 11.5 s the title crosses into the town's theme over 4 s; at 20 s the discovery stinger, the theme dipping under it | 28.0 | -10.7 | -23.7 | idle |

The montages are rendered with every slider at full. They are 10.3 MB in
all (6.9 MB of it the six new ones, which need their length to show a
theme arriving).

## Making the sounds

Four tools, standard library only, a few seconds each:
`tools/audio/feedback.py` (steps, interface, combat), `ambience.py` (beds,
voices, fires and water; every loop closes on itself), `instruments.py`
(the music's samples) and `compose.py` (the songs and `music.json`).
 `tools/audio/synth.py` is the toolkit: struck things as
sums of decaying sines, plucked strings (Karplus-Strong), soft horns,
filtered noise for air, cloth and grit, and a small room. Every random
choice comes from a generator seeded by the sound's name, so the same
source writes the same bytes; `tools/audio/check.sh` regenerates twice and
compares both runs with `content/` (184 files, on one machine: the
arithmetic is the platform's). The earlier sounds in `sounds/world/` have
no generator any more, the ability sounds have their own
(`tools/abilities/sounds.py`), and `sounds/amb/ambience.json` is written
by hand.

To add a sound: write a recipe in `feedback.py`, add its line to `SOUNDS`,
run the tool, add the event to `content/audio.json`, and play it by key.

## Tests

| Test | What it holds |
|---|---|
| `tests/pack.rs` `every_sound_event_has_its_files` | every event's files exist, decode, are not silent or clipped; a repeated event has 3 or more variants; every ability's sound has its file; no sound file is left unplayed |
| `tests/pack.rs` `every_event_the_client_plays_is_described` | every key in the client's code, every block's material and every block's footstep is in `audio.json`; the events the quality bar needs exist and are played |
| `tests/pack.rs` `sounds_for_the_first_minute_come_with_the_join_and_the_rest_stream` | which sounds join and which stream |
| `tests/pack.rs` `each_event_keeps_to_the_level_of_its_bus` | each event's peak through the mixer, and each ability sound's, is under its bus's target |
| `tests/audio_mix.rs` `the_worst_case_never_clips_and_each_bus_keeps_its_level` | the worst case (with the real ambience and music) has no clipped sample; each bus is within 4 dB of its target; the limiter is idle at a new player's levels; a slider at nothing silences only its bus |
| `tests/audio_mix.rs` `themes_cross_without_a_click_or_a_hole` | all 30 crossings between the title and the five zone themes: no sample-to-sample step larger than 1.25 times what the themes' own notes make (the largest measured is 100% of them), no half second quieter than a quarter of the quieter theme, one deck left playing after |
| `tests/audio_mix.rs` `a_restless_player_makes_no_clicks` | 20 s of crossing a border every 2 s, fights starting and stopping inside the drums' fade, and stingers on top of stingers (five songs at once at most): the largest step is 0.083 against 0.101 for the same music played calmly, and one theme is left when the player stands still |
| `tests/audio_mix.rs` `the_music_dips_under_a_stinger_and_a_fanfare_and_under_the_drums` | under the level-up fanfare the theme measures 35% (the pack asks 35%) and is back to 100% six seconds later, with no step; under the discovery stinger the mixer's dip reaches 35% and is gone 8 s after it starts; the drums and the sting come and go with a fight |
| `tests/pack.rs` `every_gauntlet_place_has_a_bed_at_every_hour_and_a_theme` | the 18 gauntlet places at nine hours each hear a bed; each has a theme with a track; the moods are where they should be; every rule names sounds, areas and blocks that exist |
| `tests/pack.rs` `beds_and_loops_go_round_without_a_seam` | each bed and placed loop steps no further from its last sample to its first than it does anywhere else |
| `tests/pack.rs` `every_song_plays_on_instruments_the_pack_has` | every song parses and builds from the pack's samples; held instruments' samples are seamless; themes loop and stingers end; nothing under `music/` is unused |
| `tracker.rs` and `ambience.rs` unit tests | notes at their rows and pitch, a theme going round, a fade with no step, the room; rules by hour, height, weather and sky; the search for sounding blocks |
| `tests/streaming.rs` `a_streamed_sound_is_fetched_when_first_played` | against a real server: a streamed sound is not in the join, is asked for when first played, and every variant arrives |
| `audio.rs` unit tests | the limiter's ceiling, and that a rising peak is turned down and not cut flat; the path the device is fed by; each slider's bus; resampling; variants taking turns; named sounds; distance and side; the event queue |
| `options.rs` `sound_levels_go_into_settings_and_back` | the five levels save and load, and an older settings file keeps its volume as the master |

## Sizes

| | Before any of this (4de5b40) | After the feedback sounds (de581d0) | With ambience and music |
|---|---|---|---|
| First join, bytes on the wire (`tests/streaming.rs`) | 3,746,571 | 2,947,175 | 2,947,128 |
| Player client (`check-client.sh`) | 9,507,656 | 9,564,344 | CLIENT_NOW |

Everything of the ambience and the music streams, their descriptions
included, so the join did not grow (it lost 47 bytes: the first bake's
ambience events left `audio.json`).

| Sounds | Files | Bytes on disk | Arrives |
|---|---|---|---|
| `sounds/step/` | 28 | 206,382 | with the join |
| `sounds/ui/` | 52 | 1,407,810 | with the join |
| `sounds/combat/` | 30 | 681,102 | with the join |
| `sounds/world/` | 96 | 2,022,078 | streamed when first needed |
| `sounds/amb/` (43 sounds and `ambience.json`) | 44 | 4,803,873 | streamed when first needed |
| `music/inst/` | 17 | 877,022 | streamed with the first song |
| `music/*.json` (nine songs and `music.json`) | 10 | 35,677 | streamed when first wanted |
| `sounds/abilities/` (the class lane's) | 31 | 437,844 | streamed when first needed |
| `audio.json` | 1 | 9,512 | with the join |

Before, all 136 sound files (4.1 MB) came with the join.

The new sounds are 22050 Hz mono; the earlier ones are 44100 Hz. No
decoder was added: the client still reads 16-bit WAV only, and files
travel with the lossless `PCM16` filter.

## Not yet

- **The first seconds are quiet.** Ambience and music stream when first
  wanted, so on a first join the beds and the theme arrive a moment after
  you do (how long was not measured), and the title screen has music
  only once a join has cached it.
- **No theme by night**, and themes loop without rests; a long stay in one
  place hears the same minute round and round.
- **Beds ignore walls.** A bed marked outdoors is heard wherever a fair
  share of sky light reaches (from a fifth of full to a half it fades in),
  but placed sounds are not muffled by what is between.
- **Not tried in the running game.** The rules were checked against the
  rules' areas and the gauntlet's camera positions, and the search for
  sounding blocks against a small test world; the stamps do contain the
  blocks (hearth, brazier, furnace, anvil). Nobody has walked the valley
  with this playing: how much sky light a canopy lets through, how high
  the hills really are, and whether a fight's start and end feel right
  are for a hand run.
- **The combat sting is in D**, like the valley's theme; over the other
  themes it may sit badly.
- **Indoors by night away from the Barrow wood and the mine there is no
  bed** (the night's is outdoors only), only what stands near.
- **Creature voices** (aggro, hurt, death by family): aggro needs the
  server to say when a creature notices you, which is a wire change.
- **A fuller ability library.** The class lane's ability sounds play
  through the mixer, but they are one or two variants each at 16 kHz, with
  one sound per ability (no separate cast, travel and impact).
- **Click sounds in live play.** A click sounds when egui reports a widget
  clicked while the pointer is over the interface. This was not tried with
  a mouse (the checks here have no window): listen for stray ticks while
  fighting, and for missing ones on painted parts of the HUD.
- **The whisper ping** has no sound of its own (whispers arrive as chat).
- **Server refusals** ("out of range", "not enough money") arrive as plain
  text, so only the refusals the client can see for itself sound as errors.
- **Other players' footsteps** are not heard.
