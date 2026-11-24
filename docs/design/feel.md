# Moment-to-moment feel: critique and plan

"Feel" is what a player notices in the first ten seconds, before any quest,
loot or level-up: how the character moves, how the camera follows, and how a
hit lands. Nobody has yet played Hearthblock in a real window, so this
critique comes from the code and the numbers. Every point needs confirming by
hand play: the first goal below is that hand play.

## What the code does today

| Area | Today | Feels like |
|---|---|---|
| **Movement authority** | The client moves itself; the server only checks moves (speed budget, flying, teleports) and snaps a player back with `Teleport` when a check fails (`movement.rs`). | Responsive locally. On a lag spike the speed budget is exceeded and the player is yanked back, which reads as a bug. |
| **Server tick** | 50 ms (20 Hz) loop; poses fan out each tick. | Other players and creatures update 20 times a second. |
| **Remote players and creatures** | Linear interpolation between the last two positions (`lerp`). There's no buffer and no fixed delay, so jitter in arrival shows as speed wobble. | Slight stutter, and other characters slide rather than step. |
| **Global cooldown** | 1.5 s (1.0 s for combo classes), with a 0.4 s ability queue from the class pass. | Correct for a classic MMO, but long if nothing marks the press. |
| **Hit feedback** | Damage numbers and a one-shot flinch clip, plus hit and crit sounds since audio hand-in 1. No hit-stop, screen shake, recoil, knockback on players, impact particles at the point of contact, or enemy wind-up tells. | Blows register as numbers, not as impacts. This is the biggest gap. |
| **Animation** | Clips with a mirrored flinch; locomotion clips with `stride` so feet don't skate. Blending between states not reviewed; no turn-in-place, landing or stopping clips. | Likely snappy but stiff; turning and stopping pop. |
| **Camera** | Third-person; default zoom 5 m in play, while the scale spec and judge cameras use about 9 m; `pull_back` stops the camera inside walls. No smoothing on collision, and no lag or lead on movement. | Close and twitchy, and it pops forward when a wall comes between camera and character. |
| **Input** | Click-to-target and hotkeys; the action bar and drag-and-drop were proven by `dyno drive`. No auto-run toggle, no click-to-move. | Fine for WoW players; strafe and turn keys need checking by hand. |
| **Jump and fall** | Gravity 15.7, jump about 0.6 m, auto-step on. | Probably right; check landing and edge drops by hand. |

## Plan, in order of impact

### F1. Hand-play baseline: measure before changing
- **Who plays:** a person, the user, plays 15 minutes of each of two classes in a real window on the live server through the tunnel, and once on the local network.
- **What is recorded:** where it feels bad, plus the frame time and network round trip shown on screen. That means building an F3 debug overlay with fps, frame ms, ping, tick and correction count.
- **Before you play:** an agent first builds the overlay and a feel test map (a flat field, stairs, a ledge, a narrow corridor, a dummy creature) as data.

### F2. Combat impact: the biggest gap
Every hit should land in the hands and eyes.
- **Hit-stop:** a 40–80 ms freeze of both attacker and target animation on a landed melee hit; longer on crits and kills.
- **Screen shake and recoil:** small, short, directional camera shake on the player's own crits and on heavy hits taken. Capped, and with an option to turn it off.
- **Impact particles at the contact point:** sparks on metal, blood-free dust and leaves on beasts, magic bursts per school. Data in life.json, triggered by `Effect` and `Damage`.
- **Enemy tells:** creatures telegraph big attacks (wind-up pose, glow or ground marker) 0.5–1 s ahead, so dodging and interrupting exist.
- **Knockback and stagger:** a crit staggers small creatures; a stun visibly dazes them.
- **Kill moment:** a short death animation with a pop of particles and loot sparkle. No body vanishing on the same frame.
- **Ability press acknowledgement:** a press during the GCD queues with a visible pulse on the button; a pressed ability starts its cast animation the same frame, before the server answers. The result still comes from the server.

**Goal:** a blind side-by-side of 3 s clips, before and after, where the judge picks the "after" clip as "hits land" for every class. Hand play agrees.

### F3. Movement that never fights you
- **Server-simulated movement with client prediction and reconciliation.** Inputs are sent with sequence numbers. The server simulates with the same `dyno_sim::movement` code, and the client predicts and replays from the last acknowledged state. Corrections blend over 100–150 ms instead of snapping.
- **Remote smoothing:** an interpolation buffer of about 100 ms with timestamps; extrapolation capped at 150 ms.
- **Tick rate:** consider 30 Hz for poses near the player, measured against the server budget.

**Goals:**
- with 150 ms of simulated latency and 2% loss, no visible snap in 10 minutes of play (counted by the overlay's correction count);
- remote characters move without wobble (measured as jitter in drawn speed);
- speed hacks are still refused (the existing tests stay green).

### F4. Animation polish
- **Transitions:** blend times between locomotion states.
- **New clips:** turn-in-place, starting and stopping, landing (soft and hard), strafe and backpedal with the right clips, and running up and down stairs.
- **Combat stance:** a "combat idle" while in combat, and a visible weapon draw and sheathe.
- **Data:** all of this lives in rigs and anims, covered by the existing animation contract test.

**Goal:** no clip snaps visibly in a hand-play recording of walk, run, stop, turn, jump, land and fight.

### F5. Camera that feels like a WoW camera
- **Default zoom:** 9 m, matching the scale spec, with the wheel range 2–25 m.
- **Smoothing:** smooth pull-in on collision and a smooth return; light follow lag (critically damped); a slight look-ahead when running.
- **Behaviour:** mouse-look hold, plus auto-follow behind the character while moving if the player hasn't turned the camera recently (an option).
- **Rules:** never inside geometry, and never clipping the character's own head.

**Goal:** the camera never pops (no frame-to-frame jump over a threshold) in a scripted run through doors, under trees and along cliffs, measured by `dyno drive`.

### F6. Responsiveness budget
- **Input to on-screen response:** under 1 frame locally for movement and ability press feedback.
- **Frame pacing:** steady, with no hitch over 20 ms while streaming (already measured by the performance lane).
- **Audio:** click to sound under 30 ms.

## Ownership and order
1. **F1 baseline tools and test map.** One agent, small. Then the user plays; the session lead writes up the notes.
2. **F2 combat impact.** The biggest win, and possible without a wire change for most of it (client side, driven by existing messages). Telegraphs need one server message, so they ride on the next protocol bump.
3. **F5 camera.** Small, client only.
4. **F3 prediction and reconciliation.** The largest engineering piece: server simulation, input sequence numbers, a protocol bump. Do it after F2 and F5 so the game feels better while it's under way.
5. **F4 animation polish,** alongside F3.

Each piece is a Fable agent with written goals; every hand-in reports pass or fail against them.
