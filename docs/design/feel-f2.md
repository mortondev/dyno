# Feel, rounds F1 and F2: baseline tools and combat impact

The plan is `docs/design/feel.md`. This note holds the goals of the first
two rounds, what was built for each, how each was checked, and the result.

## Goals

**F1: baseline tools**

1. **A debug overlay on F3:** fps, frame time, ping, the server tick, the
   count of movement corrections and the entities drawn. It is off by
   default and its words are in `content/ui/strings.json`.
2. **A feel test yard,** built as a test pack and reached with `/feel` on a
   test server. It has a flat field, stairs, a ledge, a narrow corridor and
   three dummies: one that cannot die, one that always takes a crit, and
   one that falls to any blow.
3. **"Before" clips:** a drive script records a 3-second clip of each class
   landing a basic hit, a crit and a kill on the dummies. The clips are kept
   as small JPG contact strips in `docs/design/feel/before/`, taken before
   any F2 change could affect them.

**F2: combat impact**

4. **Hit-stop:** a 40–80 ms freeze of the animation when a melee hit lands,
   longer on crits and kills. Only the animation freezes, never movement or
   input. The timings are data.
5. **Camera shake:** small, directional and capped, on your own crits and on
   heavy hits you take. An Options toggle turns it off.
6. **Impact particles:** cube particles at the contact point, by weapon or
   school and by the target's material. They are data in `content/life.json`,
   set off by the existing damage and effect messages.
7. **Enemy tells:**
   - two or three early creatures telegraph a heavy attack 0.5–1 s ahead,
     with a wind-up pose and a ring-of-cubes marker;
   - this is server behaviour, set in data;
   - a 12-bot, 15-minute swarm shows deaths per hour do not rise.
8. **Stagger and daze:** a crit staggers small creatures, and a stunned
   creature shows a daze effect made of cubes.
9. **Kill moment:** a short death animation, a cube pop and a loot sparkle.
   The body does not vanish on the same frame.
10. **Instant press feedback:**
    - the cast or swing animation and the button pulse start on the frame of
      the press;
    - a server refusal cancels them cleanly with the existing error sound;
    - a press during the global cooldown shows the queued pulse.
11. **Proof:**
    - "after" clips in `docs/design/feel/after/`;
    - a blind check: a fresh agent judge gets only the before and after
      strips per class, in random order, and picks which reads as "hits
      land";
    - it passes if the judge picks "after" for all six classes.
12. **Gates:**
    - `cargo fmt --all`;
    - `cargo clippy --workspace --all-targets -- -D warnings`;
    - `cargo test --workspace`;
    - `bash .github/scripts/check-client.sh`;
    - `tools/drive/run.sh`;
    - `dyno bench` within `docs/design/performance.md`.
13. **Obeys the voxel rule** (`docs/art/voxel-rule.md`): impact particles,
    tell markers, daze effects, loot sparkles and the test yard are cubes on
    the size ladder, and a ground marker is a ring of cubes.

## What was built

### F1
- **The overlay** is `crates/dyno-client/src/app/debug.rs`. Ping and Pong
  are protocol messages.
- **The yard** is made by `tools/feel/make_pack.py`, which writes a whole
  pack (the live pack plus the yard) for a test server. The live pack holds
  none of it.
- **Recording.** `dyno drive` gained:
  - `orbit DEGREES`;
  - `clip-start NAME` and `clip-end`, which save every frame, plus a
    `frames.txt` that says in which frame a blow of yours landed (hit, crit
    or kill) and where the target is in the picture;
  - `key f3`.
- **The clips** are recorded by `tools/feel/run.sh` running
  `tools/feel/clips.drive`, one class at a time. `tools/feel/strip.py`
  turns them into contact strips: each row is a hit, a crit or a kill, and
  its 11 frames are taken at fixed moments from 150 ms before the blow to
  1.1 s after, cut out round the target.

### F2
- **Data.** Every number is the pack's:
  - `content/feel.json` holds the hit-stop, shake, stagger, press, tell and
    kill timings;
  - `content/life.json` has a new `impacts` section: bursts by material,
    bursts by school (and `weapon`), the crit multiplier, the kill pop, the
    loot sparkle and the daze stars;
  - in the rules, creatures gain `material` and `heavy`, and abilities gain
    `school`.
- **Hit-stop.** `Animator::freeze` stops the animator's own clock and
  nothing else: the server position, the stride and the input go on (see
  `anim.rs` and its tests).
  - A melee blow stops the target and you at contact, a hit for 60 ms and a
    crit for 100 ms. A kill stops you, and holds the body's first pose, for
    140 ms.
  - A shot or a spell stops only the target, for 0.6 of that.
  - A blow you take stops you for 50 ms.
  - An auto-attack swing is drawn from its contact point, so the impact
    lands with the damage number.
- **Camera shake** (`feel::Camera`). Only the picture turns, never where
  you look or aim.
  - Your own crit throws it down and towards the target's side of the
    screen (0.9°, 0.22 s). A kill that is not a crit does not shake.
  - A heavy blow you take throws it away from whoever struck (1.3°, 0.3 s).
    A blow is heavy if it was wound up, or if it takes 15% of your health.
  - Everything at once is capped at 1.6°.
  - The toggle is Options, Interface, "Camera shake", saved as
    `camera_shake` in the settings.
- **Impact bursts** (`feel::Bits`). Cubes are thrown back off the struck
  face, by the target's material (wood, hide, shell, cloth, metal, bone,
  shadow, wax) and by the weapon or the spell's school (fire, frost, holy,
  shadow, arrow, earth).
  - A crit throws 1.8 times as many.
  - A bleed's tick throws a few and stops nothing.
  - Each cube is a whole ladder step, drawn upright. Late in its life it
    drops one step down the ladder, never to a size between.
- **Tells.** Heavy blows are server behaviour (`Mob::strike` in
  `creatures.rs`).
  - Every `heavy.every`th blow is wound up for `heavy.windup` seconds, sent
    as `Act::WindUp { time }`. It then lands as `Act::Special`, at
    `heavy.damage` times the ordinary blow, on a target still within
    `heavy.reach`; a target that stepped out is missed.
  - A stun breaks it off (`Act::CastEnd { done: false }`).
  - The Thornboar (0.8 s) and the Ashcloak (0.9 s) have one; so does the
    yard's Training Dummy.
  - The client stretches the creature's Special clip over the wind-up
    (`Animator::wind_up`). It draws a ring of 12.5 cm cubes at the reach,
    with a second ring closing in from the middle, re-snapped to whole
    cubes as it grows, and whitens the creature as the blow comes.
- **Stagger and daze.**
  - A crit pushes a creature of scale 1.1 or less 0.4 m back and returns it
    over 0.35 s. This is drawn only; the server has not moved it.
  - A creature under a stun aura has five 6.25 cm cubes going round its
    head.
- **Kill moment.**
  - The killing blow pops 22 cubes of 12.5 cm.
  - A creature that leaves no body (yet) is still drawn falling for 0.8 s
    (`feel::Fallen`), so nothing vanishes on the frame it dies.
  - A body comes in holding its first pose for the kill's stop, then plays
    its death.
  - A body with loot throws a sparkle 0.6 s after it falls.
- **Press.**
  - A pressed key starts its swing, or its cast pose for an ability with a
    cast time, on that frame, and its button flashes white and gold for
    0.3 s.
  - A press in the last 0.4 s of the global cooldown or a cast (the
    server's `QUEUE`) is queued: no animation yet, and the button beats
    gold until the cooldown ends.
  - A press the client knows will fail (cooling down, too far, not enough
    power, too early in the global cooldown) gets the existing `ui.error`
    sound and nothing else.
  - **A change of behaviour:** a press earlier than the last 0.4 s of the
    global cooldown was always dropped by the server. It now beeps, so
    mashing a key through the cooldown beeps until the queue window opens.
  - The server now sends `Refused { ability }` for every press it turns
    down, including one that was too early to queue. The client then takes
    back the animation, the predicted cast and the pulse, and plays
    `ui.error`.
- **Protocol 26.** `Act::WindUp` and `ServerMsg::Refused` are new server
  messages, as are F1's `Ping` and `Pong`. v0.11.2 and v0.11.3 shipped on protocol 25, and a client on 25
  drops the connection on a message it cannot read. These additions cannot
  ride on 25, so the protocol is 26. The lead may overrule this.

## Results

| # | Goal | Result | How it was checked |
|---|---|---|---|
| 1 | Overlay on F3 | **pass** (the first message to the lead claimed this before it was checked; `check.sh` checked it afterwards) | `tools/feel/check.sh`: `key f3` then a shot shows fps, frame time, ping, server tick, corrections and entities drawn (`docs/design/feel/checks/f1-overlay.jpg`) |
| 2 | Test yard | **pass** | the check run reaches it with `/feel`; the stairs to the deck, the ledge and the corridor are in `checks/f1-yard.jpg`; all three dummies are used by every clip run (the Keen Dummy crits every blow, the Straw Dummy falls) |
| 3 | Before clips | **pass** | `docs/design/feel/before/*.jpg`, six classes, recorded on the merged build before any F2 code was wired in. The rogue's first kill clip lost its target and was recorded again |
| 4 | Hit-stop | **pass** | timings in `feel.json`: hit 60 ms, crit 100 ms, kill 140 ms. `feel::tests::the_pack_keeps_to_the_design` holds the hit to 40–80 ms and crit < kill. `anim::tests::a_hit_stop_holds_the_pose_and_loses_no_animation` holds the pose and loses no animation. Movement and input are untouched by construction (only `Animator`'s clock stops) |
| 5 | Camera shake | **pass** (by test and code; not seen) | directional, capped at 1.6° (`feel::tests::the_shake_is_thrown_the_way_of_the_blow_capped_and_soon_over`); Options toggle "Camera shake". A turn of under a degree is not visible in a still, and neither the shake nor the Options row has been looked at in play |
| 6 | Impact particles | **pass** | bursts by material and school from `Damage` and `Effect`, data in `life.json` `impacts`; visible in every after strip |
| 7 | Enemy tells | **pass** | wind-up pose, ring and closing ring in `checks/tell.jpg` (0.8 s); server unit test `a_heavy_blow_is_wound_up_and_can_be_stepped_away_from_or_stunned`; swarm below |
| 8 | Stagger and daze | **pass** | crit stagger on the Keen Dummy in the after strips; daze stars on the Onrush-stunned dummy in `checks/daze.jpg` |
| 9 | Kill moment | **pass** | the kill rows of the after strips: death fall, a pop of cubes; the fallen figure covers any frames before the body comes. The loot sparkle is in `checks/loot.jpg`, from `FEEL_LOOT=1 tools/feel/check.sh ... tools/feel/loot.drive` (a scratch pack whose Straw Dummy drops something; the yard itself is as the before clips had it): a burst of gold cubes at the body about 0.6–0.8 s after it falls, among the corpse's existing loot motes |
| 10 | Instant press | **pass** | `checks/press.jpg`, top to bottom: Onrush's key lit on the frame of the press; Heroic Blow pressed straight after, refused (no pulse, error sound); Heroic Blow pressed in the last moment of the global cooldown, queued (gold frame) |
| 11 | Proof | **pass** | after strips in `docs/design/feel/after/`. A fresh agent judge got only the 12 strips, as `classN-X/Y` in an order drawn at random per class, and no hint of what changed. It picked "after" for all six classes on its first and only verdict, naming in each case "a burst of block-shaped bits thrown off the dummy in the frames right after the hit" |
| 12 | Gates | **pass**, with one inherited count failure | below |
| 13 | Voxel rule | **pass** | every burst cube is snapped to the ladder (`feel::on_ladder`) and drawn as an upright cube; it shrinks only by a whole ladder step. The tell is two rings of 12.5 cm cubes on one grid (`feel::ring_cells`, tested). The daze is 6.25 cm cubes. No billboards, cards, curves or diagonals |

### Swarm (goal 7)

Two swarms, run one after the other on the same build (`9fbfac0`). Each used a fresh world and
`dyno bots 127.0.0.1:PORT --count 12 --minutes 15`, with the server on
`DYNO_TICK_STATS=1`. The only difference between the two packs is the two
creatures' `heavy` entries: the control pack has them removed.

| | bot-minutes | fights | deaths | deaths an hour | deaths a fight | by cause |
|---|---|---|---|---|---|---|
| without heavy blows | 173 | 450 | 14 | 4.85 | 0.031 | Grazer 7, Thornboar 4, Old Tusk 3 |
| with heavy blows | 179 | 456 | 14 | 4.70 | 0.031 | Grazer 7, Thornboar 5, Old Tusk 2 |

- **Deaths an hour did not rise** (4.85 to 4.70; the same 14 deaths).
- **The heavy blows were real:** 24 were wound up and 21 landed. None
  missed, because the bots never step away, and none were broken off. The
  other 3 are not accounted for: most likely the creature died mid-wind-up,
  which no counter records. A bot stands in every one: this is the worst
  case.
- **Only the Thornboar's heavy blow was tested.** No Ashcloak was fought in
  either arm: 15 minutes of new characters do not reach them.
- **Small sample.** It was one run per arm, with 14 deaths each. The
  client-release build of `check-client.sh` ran during the control arm.
- **Why this does not raise deaths:** a heavy blow is 1.6 times an ordinary
  one, but it replaces a blow and is 0.8–0.9 s late. Over a round of three
  blows a creature deals about 4% more than before, and a player who steps
  out of the ring takes nothing.
- **The server's tick** was the same in both (p95 0.75–1.25 ms).
- **Deaths an hour fail the fun check in both arms** (it was failing
  before F2). The deaths are Grazers and Old Tusk, which have no heavy
  blow.

### Gates (goal 12)

All run on the branch merged with main (the voxel-rule tests included). The
check pictures in `docs/design/feel/checks/` are from the final binary.

| Gate | Result |
|---|---|
| `cargo fmt --all` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo test --workspace` | pass, run once. It includes the voxel rule's mesh test, with F2's bursts, rings, daze, pop and sparkle registered in it (`feel::voxel_objects`) |
| `bash .github/scripts/check-client.sh` | pass on the merged tree: 9.30 MB, no content strings |
| `tools/drive/run.sh` (port 7781) | pass: 136 steps |
| `dyno bench --local --tier high --check tools/bench/reference.json` | **times within budget; the count check fails, not because of F2** (below) |

**The bench's times** at the default tier:

| | Measured | Budget |
|---|---|---|
| GPU median, the six scenes | 1.1–2.1 ms | 8 ms |
| GPU median, the flight | 4.8 ms | 8 ms |
| Main thread, the six scenes | 1.3–4.3 ms | 8 ms |
| Main thread, the flight | 5.7 ms | 10 ms |

**The count check** fails on three numbers against
`tools/bench/reference.json`:

| Scene | Count | Over the reference |
|---|---|---|
| Hearthstead plaza | triangles | +34% |
| Hearthstead plaza | vertex memory | +16% |
| Miner's Rest | triangles | +18% |

These are world meshes. Main's voxel-rule work (`b4c644c`, roofs as stairs)
recorded +16% to +43% world triangles in `tests/budget.rs`, which passes,
but did not record the bench's `reference.json` again. Main's own bench at
the default tier, without F2 (`docs/art/voxel-rule/bench-high.md`), already
has the plaza at 3.71 M triangles (+28% on the reference's 2.89 M) and
Miner's Rest at 2.60 M (+15%). This run has 3.87 M and 2.67 M, on the later
main. F2 touches no mesher
(`git diff main -- render.rs terrain_mesh.rs shapes.rs worldwork.rs` is
empty), and the bench scenes have no fight in them. Recording the
reference again is the voxel-rule lane's or the lead's call.

### Not done, or left as it was
- **Hit-stop is hard to see in stills.** The judge saw the bursts. A stop of
  60–100 ms is two to six frames held, which only a hand-play recording
  shows well.
- **Bots do not dodge.** The swarm is the worst case: a bot that stands in
  every heavy blow.
- **The existing white flash** on a struck creature (0.12 s) and the old
  effects in `app.rs` (`spawn_bolt`, `spawn_burst`, `spawn_motes`, whose
  sizes are off the ladder) are not F2's. The voxel-rule lane owns them.
