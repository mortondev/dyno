# Sound

What the game sounds like, how it is built, and how to check it. This is
the listening guide for section 11 of the quality bar
(docs/design/quality-bar.md): "footsteps per ground, and a UI sound for
every action" are here; music and the ambience of each zone are the next
hand-in and are not built yet (see "Not yet" at the end).

Nobody has listened to these sounds yet. They were made and checked by
measurement only (levels, spectra, loudness). Listen to the montages in
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
| Ambience | -12 | -16.7 (`amb.hammer`) | -28 | -28.2 |
| Music | -10 | none yet | -24 | -26.7 (a stand-in: the dusk horn on the music bus, in the test only) |

The worst case (`6_everything`: three foes, rain and wind, the interface,
a level-up and a quest handed in at once, your death; plus the music
stand-in in the test), measured through the mixer:

| Sliders | Peak (dBFS) | Loudness (dB) | Limiter | Clipped samples |
|---|---|---|---|---|
| All at full | -1.1 | -16.3 | took off under 0.1 dB | 0 |
| A new player's | -5.3 | -21.4 | idle | 0 |

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
the low-health heartbeat, the dusk horn and the dawn chime, and the
ambience one-shots (bird, cricket, rain, wind, a far hammer) are the
sounds of the first bake, moved to `sounds/world/` and `sounds/amb/` and
played through the new mixer. Breaking, placing, digging and the
heartbeat are a few dB under their old levels at a new player's sliders;
rain and wind are about 16 dB under, cut to fit the ambience target until
the ambience is rebuilt. Two of the breaking sounds (glass and stone) had
clipped samples; those and foliage are made anew.

## The montages

`tools/audio/montage.sh` renders six scripted situations through the real
mixer into `docs/audio/montages/` (WAV, mono, 22050 Hz: the game itself is
stereo). The scripts are
`crates/dyno-client/examples/audio_montage/scenes.rs`.

| File | What happens | Seconds | Peak (dBFS) | Loudness (dB) | Limiter |
|---|---|---|---|---|---|
| `1_footsteps.wav` | grass, dirt, stone, wood, sand, water, snow: five running steps on each | 16.5 | -16.8 | -35.4 | idle |
| `2_interface.wav` | bags open, an item moved, closed; a merchant: two bought, two sold, a refusal | 10.8 | -13.5 | -32.6 | idle |
| `3_quest.wav` | a quest taken, a run, three things gathered, the run back, handed in | 16.0 | -8.0 | -22.9 | idle |
| `4_fight.wav` | one foe ahead: blows both ways, a crit, a bolt, a heal, a burst, the kill, the loot | 11.5 | -6.0 | -24.0 | idle |
| `5_level_up.wav` | the last blows, the kill, coin, the level, the new ability | 8.5 | -7.1 | -20.9 | idle |
| `6_everything.wav` | the worst case: three foes, rain, the interface, a level-up, your death | 13.0 | -1.0 | -16.9 | took off 0.1 dB at most |

The montages are rendered with every slider at full. They are 3.4 MB in
all.

## Making the sounds

`python3 tools/audio/feedback.py` writes every new sound (standard library
only; about 8 s). `tools/audio/synth.py` is the toolkit: struck things as
sums of decaying sines, plucked strings (Karplus-Strong), soft horns,
filtered noise for air, cloth and grit, and a small room. Every random
choice comes from a generator seeded by the sound's name, so the same
source writes the same bytes; `tools/audio/check.sh` regenerates twice and
compares both runs with `content/sounds/` (114 files, on one machine: the
arithmetic is the platform's). The earlier sounds in `sounds/world/` and
`sounds/amb/` have no generator any more, and the ability sounds have
their own (`tools/abilities/sounds.py`).

To add a sound: write a recipe in `feedback.py`, add its line to `SOUNDS`,
run the tool, add the event to `content/audio.json`, and play it by key.

## Tests

| Test | What it holds |
|---|---|
| `tests/pack.rs` `every_sound_event_has_its_files` | every event's files exist, decode, are not silent or clipped; a repeated event has 3 or more variants; every ability's sound has its file; no sound file is left unplayed |
| `tests/pack.rs` `every_event_the_client_plays_is_described` | every key in the client's code, every block's material and every block's footstep is in `audio.json`; the events the quality bar needs exist and are played |
| `tests/pack.rs` `sounds_for_the_first_minute_come_with_the_join_and_the_rest_stream` | which sounds join and which stream |
| `tests/pack.rs` `each_event_keeps_to_the_level_of_its_bus` | each event's peak through the mixer, and each ability sound's, is under its bus's target |
| `tests/audio_mix.rs` | the worst case never clips; each bus is within 4 dB of its target; the limiter is idle at a new player's levels; a slider at nothing silences only its bus |
| `tests/streaming.rs` `a_streamed_sound_is_fetched_when_first_played` | against a real server: a streamed sound is not in the join, is asked for when first played, and every variant arrives |
| `audio.rs` unit tests | the limiter's ceiling, and that a rising peak is turned down and not cut flat; the path the device is fed by; each slider's bus; resampling; variants taking turns; named sounds; distance and side; the event queue |
| `options.rs` `sound_levels_go_into_settings_and_back` | the five levels save and load, and an older settings file keeps its volume as the master |

## Sizes

| | Before (main at 4de5b40) | This work alone | With main merged (43b696c) |
|---|---|---|---|
| First join, bytes on the wire (`tests/streaming.rs`) | 3,746,571 | 2,657,074 | 2,947,175 |
| Player client (`check-client.sh`) | 9,507,656 | 9,558,728 | 9,564,344 |

"This work alone" was measured before the three quest cues went from one
variant to three (about 0.4 MB more on disk) and before main's ability
icons and sounds were merged; the last column has both.

| Sounds | Files | Bytes on disk | Arrives |
|---|---|---|---|
| `sounds/step/` | 28 | 206,382 | with the join |
| `sounds/ui/` | 52 | 1,407,810 | with the join |
| `sounds/combat/` | 30 | 681,102 | with the join |
| `sounds/world/` | 96 | 2,022,078 | streamed when first needed |
| `sounds/amb/` | 20 | 1,518,418 | streamed when first needed |
| `sounds/abilities/` (the class lane's) | 31 | 437,844 | streamed when first needed |
| `audio.json` | 1 | 9,512 | with the join |

Before, all 136 sound files (4.1 MB) came with the join.

The new sounds are 22050 Hz mono; the earlier ones are 44100 Hz. No
decoder was added: the client still reads 16-bit WAV only, and files
travel with the lossless `PCM16` filter.

## Not yet

- **Music** (zone themes, title, combat layer, stingers) and **ambience
  beds per zone and time** (birdsong, night, wind, water, town, forge,
  fires): the next hand-in. The music and ambience buses and their sliders
  exist; nothing new plays on them yet, and nothing ducks.
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
