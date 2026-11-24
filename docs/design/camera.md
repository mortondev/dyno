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
| 5 | No pops through a door, under a crown, up stairs, along a cliff | `tools/drive/camera.sh` |
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
- **Eased:** the boom's wanted length is found by marching out from the
  follow point in 5 cm steps. Where the boom passes within 1.5 m of
  something it is already shortened, in proportion, so it comes in as an
  edge *nears* the boom instead of all at once when the edge crosses it
  (near the head this counts only in part, so a doorway's sides do not pull
  it in); and something closing on the eye's end from the side draws the
  eye in up to 1.5 units ahead of the touch. That length changes smoothly as
  the character and the view move. The boom follows it on one critically
  damped spring, so its speed never jumps: in at 18 rad/s (45 rad/s while
  the eye is cut off from the character, so what it then must cross is
  still thin), at most 50 units/s, and back out settling in about 0.3 s.

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
and back, looking up and down, and it walks back.

**The pop threshold:** `camera check 30` fails if in any frame the camera
moved faster than **30 m/s** (0.5 m in a 60 Hz frame; a frame shorter than
1/60 s counts as 1/60 s), or if the eye came within 0.05 m of a solid cell.
30 m/s is above what a hand makes (running at 6.2 m/s while turning at
150°/s on a 9.6 m boom is about 25 m/s at the eye) and far below a pop (an
instant pull-in at 9 m moves the eye 4–7 m in a frame, 240+ m/s).

RESULTS
