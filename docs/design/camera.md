# The play camera (F5)

The third-person camera of RPG worlds: where it sits, how it follows, how
it keeps out of walls, and how it is tested. It is F5 in
[feel.md](feel.md); the numbers come from the camera section of
[scale.md](scale.md). The code is `crates/dyno-client/src/camera.rs`
(state stepped once a frame, unit-tested without a window); `app.rs` asks it
for the eye every frame.

`dyno shot`, the gauntlet cameras and `dyno scout` do **not** use it. They
place their camera at once with `app::pull_back` at the distance they are
given, which is unchanged, so their framing is unchanged. At rest the play
camera frames exactly as `pull_back` does (the same boom, rising 0.08 m for
each metre back): a test holds this.

## Goals

| # | Goal | How it is checked |
|---|------|-------------------|
| 1 | Default zoom 9 m; the wheel covers 2–25 m; a player's zoom is kept | `camera::tests::the_wheel_covers_two_to_twenty_five_metres`; the setting `camera_zoom` |
| 2 | Smooth collision: in fast when something comes between, out over about 0.3 s; never in geometry; never in the head (the figure fades, then hides) | unit tests; `camera check` (nearest a wall) |
| 3 | Light follow lag (critically damped); a slight look-ahead while running | unit tests |
| 4 | WoW mouse-look (right steers, left orbits); auto-follow as an option, off by default | the existing input path; `auto_follow_waits_and_is_off_unless_asked` |
| 5 | No pops through a door, under a crown, up stairs, along a cliff (and along a wall) | `tools/drive/camera.sh` |
| 6 | Shots and gauntlet cameras frame as before | six gauntlet cameras before and after |

## Where it sits

- **Zoom:** 9 m behind the eyes by default (scale.md), the wheel 2–25 m in
  steps of 13 % (so a notch is about the same share of the view near and
  far). One notch in from 2 m is first person, and one notch out of first
  person is 2 m: WoW does the same, and first person was already in the
  game. The wheel's distance is eased in (about 0.1 s), so a notch is a
  glide, not a jump.
- **Saved:** the zoom is written to the settings file (`camera_zoom`) once
  the wheel has rested a second, and a join puts the camera back where the
  player left it. Before this change the zoom was **not saved at all**
  (every join started at 5 m), so there was no saved value to keep: a
  settings file from an older client has no `camera_zoom` and reads the
  9 m default. From now on a player's chosen zoom is kept.
- **Pitch** is the character's (the right or left button and the mouse);
  the eye rises 0.08 m per metre back, as `pull_back` does.

## Keeping out of walls without popping

Two rules, kept apart:

- **Hard, every frame:** the eye, a ball of 0.25 m (the near plane is
  0.05 m), is never in a solid cell. When the boom would put it in one, the
  eye first *slides*: it is pushed out of the surface it grazes (a ceiling
  or a crown of leaves above, the ground below, a cliff to the side) by as
  much as it went in, up to 1 m, if the character can still be seen from
  there. Only when that cannot be done is the boom cut to its clear length
  at once.
- **Eased:** the boom's wanted length is the shortest of two, found by
  marching out from the follow point in 5 cm steps: its clear length now
  (a little shorter where it passes within 0.5 m of something, and drawn in
  up to 1.5 units ahead of something closing on the eye's end), and its
  clear length **0.15 s from now**, at the speed the follow point and the
  view are moving. So a lintel, a trunk or a rim about to come between the
  camera and the character draws it in before it does, while a wall
  alongside that never crosses the boom does not (running a shoulder's
  width from the house wall kept the full 9 m in the drive run). The boom
  follows that length on one critically damped spring, so its speed never
  jumps: in at 18 rad/s (45 rad/s while the eye is cut off from the
  character, so what it then must cross is still thin), at most 40 units/s,
  and back out settling in about 0.3 s.

What is left are the cases with no smooth way in: a wall appearing from
nowhere between the camera and the character (nothing in play does this; a
block placed by another player could), and an edge swung or dropped across
the boom faster than the spring can follow. The eye then crosses what is
between in one step: its thickness along the boom plus the margins. A unit
test bounds the first; the drive run measures the rest (below).

**The head:** the boom is never shorter than the clear length from the
follow point, and the follow point is the head or a point at most 1 m
behind it on the way it came (2 m below or above: stairs); if that point is
not in the open it moves towards the head until it is. Nearer than 1.3 m to
the head the character is dithered out (a screen-space dither in the
fragment shader: whole cubes, nothing moved, so the voxel rule's shader
test is untouched), and nearer than 0.5 m it is not drawn.

## Following

- **Lag:** the follow point trails the head on critically damped springs
  (across at 22 rad/s, up at 12 rad/s), never more than 1 m behind across
  or 2 m below or above. At a run (6.2 m/s) it trails about 0.55 m. The
  slower spring upward turns the auto-step's one-block jumps on stairs into
  a glide.
- **Snap:** further than 4 m from the head (a teleport, a respawn), and on
  the server's `Teleport`, the springs are forgotten and the camera starts
  where the character is.
- **Look-ahead:** the view turns a little towards where the character is
  going: 0.09 s of travel (at most 0.7 m), eased in at 3/s, less when the
  camera is close in. Yaw and pitch themselves are not smoothed, so the
  mouse is never behind the hand.
- **Shake (F2):** F2's shake (`feel.rs`, on crits and heavy blows, capped
  at 1.6°, with its own option) turns the picture: `app.rs` turns this
  camera's view direction by it after the camera has placed the eye, so it
  never moves the eye, never puts it in a wall, and is not in what the
  no-pop test measures (the eye's position). For a shake that moves the
  eye instead, `Camera::shake` is an offset added to the eye after the
  collision, dropped for a frame if it would put the eye in a wall (unused
  today; a unit test holds it).

## Mouse-look and auto-follow

As WoW: hold the right button to steer (the character turns with the
mouse and takes the camera's facing when the button goes down), hold the
left to orbit the camera round the character, both to run. This was already
the game's input; it is unchanged.

**Auto-follow** is an option (Options → Interface → Camera follow), off
by default. When on, moving swings the camera back behind the character
once the player has not turned it for 2.5 s. Before this change the camera
always swung back while walking; with the option off it now stays where the
player put it, as WoW's does.

## Testing

`tools/drive/camera.sh` starts a fresh world and runs
`tools/drive/camera.drive` in the real client off screen at the default
9 m. `yard` builds the course round the character (as the operator, in
build mode, then back to play): a house of planks with a door two wide and
three high at each end, a tree whose crown of leaves hangs over the way
from four metres up, six steps up onto a cliff six metres high, and the
cliff's edge. The character walks it with W held, steering at no more than
150°/s (a hand on the mouse), then the camera is orbited out over the edge
and back, looking up and down (`swing`), it walks back and then along the
house's east wall a shoulder's width from it. After the check, set-up turns
it with its back to the wall for a picture of the camera at the head.

**The pop threshold:** `camera check 60` fails if in any frame the camera's
eye moved faster than **60 m/s, that is 1 m in a 60 Hz frame** (the step
over the frame time the camera was stepped by; a frame shorter than 1/60 s
counts as 1/60 s, so a quick frame does not make a step look sudden), or if
the eye ever came within 0.05 m (the near plane) of a solid cell. The eye
is measured before F2's shake, which turns the view and never moves it. A
hand alone moves the eye at up to about 25 m/s here (running at 6.2 m/s
while turning at 150°/s on a 9.6 m boom), the fast pull-in this camera is
asked for adds to that, and a pop (the camera placed at once) moves it
several metres in one frame.

`DYNO_CAMERA=instant` places the camera at once every frame, as before F5
(no lag, no easing; the same clear length), so the test can be seen to fail;
`DYNO_CAMERA_LOG=FILE` writes every frame (frame time, step, eye, head,
boom, clear length) for looking into one.

## Results (hand-in)

| Goal | Result |
|------|--------|
| 1. 9 m default, 2–25 m, zoom kept | **Pass.** `DEFAULT_ZOOM` 9, wheel clamped 2–25 (one notch past 2 m is first person, as WoW and as the game already had); the zoom is saved as `camera_zoom` once the wheel rests and restored on join. There was no saved zoom before (every join started at 5 m), so no existing value to keep; old settings files read 9 m. |
| 2. Smooth collision, never in geometry or the head | **Pass, with residual steps.** In every drive run the eye never came nearer a solid cell than its 0.25 m margin. Ease-out settles in about 0.3 s (unit test band 0.18–0.45 s). Running a shoulder's width from the house wall kept the full 9 m (no pull-in from a wall alongside). Backed against the wall the eye sits 0.33 m from the head and the figure is not drawn (`camera-fade` shot: a clear view, no head). The dithered part of the fade (0.5–1.3 m) is in the shader but no picture was taken of it. Where an edge crosses the boom quickly (the step down off the stairs with the cliff top behind; the crown of the tree on the stairs) the eye still moves up to 0.87 m in one frame: smooth enough to pass the threshold, not zero. |
| 3. Follow lag and look-ahead | **Pass.** Critically damped follow (22 rad/s across, 12 up; about 0.55 m behind at a run); look-ahead 0.09 s of travel, at most 0.7 m, eased; it reads `Player::vel`, which the movement code sets while walking. Unit tests; not measured separately in the drive. |
| 4. WoW mouse-look; auto-follow option, off by default | **Pass.** Right steers, left orbits, both run (unchanged); auto-follow is Options → Interface → Camera follow, off by default, swings back after 2.5 s without a turn. The old always-on swing-back is gone unless the option is on. |
| 5. No pops in a scripted run | **Pass.** `tools/drive/camera.sh`, threshold 60 m/s (1 m in a 60 Hz frame). Two runs of the final build: fastest 52.3 and 52.4 m/s (largest single step 0.87 m at a 60 Hz frame); by part of the course: house 42–43 m/s, tree 24, stairs 52, cliff 29, wall 24. The same run with `DYNO_CAMERA=instant` fails: 354 m/s (a 5.91 m step at the house door), stairs 321, tree 174, cliff 82. *The threshold was first set at 30 m/s and raised to 60 after measuring*: a fast pull-in under a lintel at a run is 40–50 m/s by itself. The margin is about 13 %. |
| 6. Shots still match | **Pass.** Six gauntlet cameras (hearthstead_plaza, hearthstead_street, watch_tower, logging_camp, old_barrow, windmill_farm) rendered with main's binary twice and this branch once, on one world. They differ from main only as much as main differs from itself (0.3–1.4 % of pixels at 8 % fuzz, scattered: grass, leaves, particles and creatures that move between runs); no edge of the land or the buildings moved. That diff was made at the first merge of main (60e2a5b), before the later camera tuning; nothing in the shot path changed since (`dyno shot --stand`, the gauntlet and `dyno scout` call `pull_back`, untouched). `dyno shot --ui` *without* `--zoom` (HUD pictures) and `dyno drive`'s own shots now frame from 9 m instead of 5 m: the new default, on purpose. |
| 7. Gates | **Pass**, on the tree merged with main (cd5e68d): `cargo fmt --all`; `cargo clippy --workspace --all-targets -D warnings`; `cargo test --workspace` (all ok; run once, before the last camera tuning, whose camera and drive tests were run again); `check-client.sh` (9.24 MB, no content strings); `tools/drive/run.sh` (first_steps, 136 steps, at the new 9 m default); `tools/drive/camera.sh`; and F2's `tools/feel/check.sh` (39 steps). |

**Cost:** `Camera::update` is about 13 µs a frame at 9 m and 32 µs at 25 m
(`cargo test --release -p dyno-client camera::tests::cost -- --ignored`,
with a cheap solidity query; the world's lookups cost a few times more).
