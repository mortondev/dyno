# Animation: the contract every character and creature conforms to

Hearthblock's figures are fine-voxel models (1/32 block) built in code,
meshed once per look and posed part by part every frame
(`crates/dyno-client/src/models.rs`). This document records what was animated
before the animation contract existed (the audit), the contract itself
(`crates/dyno-client/src/anim.rs`), how the client drives it, and what the
server would need to send so other players' actions read as well as your own.

## 1. Audit (before the contract)

Everything was ad-hoc trigonometry on two numbers passed in by the caller:
`phase` (a clock, usually `clock * 9`) and `walk` (0 standing to 1 moving).
There was no notion of state, no transitions and no one-shot actions.

| Model | What moved | Missing |
|---|---|---|
| **Person** (players, NPCs) `push_person` | Walk: thighs swing ±0.7 rad from `sin(phase)`, knees bend on the forward swing, arms counter-swing, body bobs `abs(cos)`, tunic flaps lift, three-segment cape streams with speed. Idle: a breath term (`sin(phase/2)`) nudges arms by 0.025 rad and the head by 0.15 voxel. A held weapon is carried at a fixed angle in the right hand. Head pitches with the camera. Robes damp the leg swing. | No run (walk cycle just runs faster, set by clock not distance, so feet skate); no backpedal or strafe (legs swing forward while moving sideways or backward); no jump, fall or swim pose; no attack of any kind (melee, bow, unarmed); no cast; no combat stance; no hit react; no death or lying pose; no sit, kneel, talk or emotes; no idle fidget. Legs hang from the root, not a pelvis, so the body cannot crouch, sit or fall. NPCs were drawn with `walk = 0` always, even when moving. |
| **Ashcloak / leader / Candlekin / Hollow foreman** (humanoid variants 1–3) | Same as Person, with a dagger, battle axe or pickaxe held. | Same gaps; a bandit chasing you plays the same walk as a villager; it never swings the weapon it holds. |
| **Shade / Barrow warden** (humanoid variant 0) | Hovers with a slow bob, leans with `walk`, rags trail and sway, hood nods, eyes blink, long arms reach forward and sway. | No attack, no cast, no hit react (only the white flash), no death; corpse rolled the whole hovering model on its side. |
| **Grazer, Thornboar** (quadruped 0, 1) | Trot: diagonal leg pairs swing ±0.55 from `sin(phase)`, body bobs and rocks; grazer dips its head to graze when standing; boar roots and tosses its head. | No gallop (the charge is a fast trot); no attack (gore/butt); no hit react; no death; corpse rolled sideways with legs still in the trot pose. |
| **Mirepincher** (crab) | Legs lift and swing in alternating sets (sideways gait), twitch at rest; claws weigh the air and rise with speed. | No claw strike, no hit react, no death (curl-up), corpse rolled. |
| **Websnare** (spider) | Eight legs step in alternating tetrapods (ry swing + rz lift), twitch at rest. | No bite/rear-up, no web special, no hit react, no death curl. |
| **Gloomwisp** (orb) | Core spins and pulses, veil breathes, five motes orbit, four-puff tail hangs or streams with speed, slow bob. | No bolt cast (its special), no hit react, no death (it just vanished or rolled over as a corpse). |
| **Corpses** (`push_corpses`) | The live model at phase 0, darkened, rotated 90° about the facing axis over 0.45 s, then still. | Not a death animation: legs and arms keep their standing pose, floaters lie on their side in mid-air, the drop replays whenever a corpse comes into view. |
| **Local player (3rd person), remotes** | Person walk with `walk = speed / 4`. | Everything above; plus remote players' casts were received (`ServerMsg::Casting` is broadcast) but ignored. |
| **Front-end preview, `dyno shot`** | Person at phase 0, walk 0: frozen. | (Left as is: the front end is being reworked separately; it can call `push_person_anim` with an `Animator` when that work lands.) |

Signals the client already had but did not use for animation: own velocity,
`on_ground`, water at the feet, `ServerMsg::Casting` for every player,
`ServerMsg::Damage` (your hits), `ServerMsg::Hurt` (you were hit),
`EntityInfo` health (true hits; `EntityHurt` also fires on regen/evade
resets), `ServerMsg::Effect` bolts (start point = the caster),
`Died`, corpse spawns, the open loot window, the NPC dialog, remote players'
vertical motion.

## 2. The contract (`crates/dyno-client/src/anim.rs`)

### Animations

One enum, `Anim`, for everyone (`Anim::ALL` lists them; it is generated with
the enum so tests cannot miss one). One-shots have a `duration()`; the rest
loop. `Death` is a one-shot that holds its last frame.

| Group | Animations | One-shot? |
|---|---|---|
| Locomotion | `Idle` (breathing, weight shift, a fidget every ~11 s: look round, stretch, glance down), `Walk`, `Run`, `Backpedal`, `StrafeLeft`, `StrafeRight`, `Swim` | loops |
| Air | `JumpStart` (0.18 s), `JumpAir`, `Fall`, `JumpLand` (0.32 s) | start/land are one-shots |
| Combat | `Ready` (combat idle), `Attack1H` (0.7 s), `Attack2H` (1.0 s), `AttackUnarmed` (0.5 s; beasts' natural attack), `Shoot` (1.0 s), `Special` (1.2 s) | attacks are one-shots |
| Magic | `CastDirected`, `CastOmni`, `Channel` (loops while the cast bar runs), `CastRelease` (0.5 s) | |
| Reactions | `HitReact` (0.4 s), `Stun`, `Death` (1.2 s, holds), `Dead` | |
| Social | `Sit`, `Kneel` (looting, gathering), `Talk`, `Wave` (2 s), `Bow` (2 s), `Cheer` (1.8 s), `Point` (1.6 s) | emotes are one-shots |

### Skeletons

`Skeleton::of(model, variant)` gives each creature its rig; people are
`Biped`. Bone indices are named constants in `anim::biped`, `anim::wraith`,
`anim::quad`, `anim::multi`, `anim::wisp`.

| Skeleton | Bones (hierarchy) | Used by |
|---|---|---|
| **Biped** | `pelvis` → (`thigh.r/l` → `shin.r/l`), skirts, `spine` → (`head`, `upper_arm.r/l` → `forearm.r/l` → `grip` (weapon), `cape` → 3 cape lengths) | players, NPCs, Ashcloak bandit, Ashcloak leader/Rook Varn, Candlekin, Hollow foreman |
| **Wraith** | `body` (hover) → (`hood` → eyes, `upper_arm` → `forearm`, 5 rags) | Shade, Barrow warden |
| **Quadruped** | `body` (turns about mid-back at hip height) → (`head`, `leg.fr/fl/br/bl`) | Grazer, Thornboar |
| **Multileg** | `body` → (`claw.r/l`, `leg.0..7`) | Mirepincher (6 legs, 2 claws, travels sideways, turns its face to you when it stops), Websnare (8 legs) |
| **Wisp** | `root` → `core`, `veil`, 5 motes, 4-puff tail | Gloomwisp |

A `Pose` is a rotation and an offset per bone plus four shape keys (`FLOW`:
capes/rags/tail streaming; `SHAPE`: tunic flap lift / wraith eyes / wisp
core size; `SPREAD`: wisp motes; `CLOCK`: the animator's time for secondary
motion). Poses blend by per-bone nlerp under a bone mask.

Each skeleton has one pose function — `biped(anim, at, rig)`,
`wraith(anim, at)`, `quadruped(anim, at, rig)`, `multileg(anim, at, rig)`,
`wisp(anim, at)` — and each is a single exhaustive `match` over `Anim` with
no wildcard arm: a new animation does not compile until every skeleton says
what it does. Pose functions are pure functions of time (seconds into the
animation), stride phase and a per-figure seed, so any frame can be frozen.
Keyframed motion uses `track(u, &[(time, value), ...])` (eased, held at the
ends). Rigs carry what changes the motion: what a biped holds (`Grip`:
empty, one hand, two hands, bow), robes (short strides; the pelvis tilts so
the robe sweeps forward to sit or kneel), a beast's hip height, width and
length (how it lies on its side), a crawler's legs (side, gait phase, front
pair) and shell height (how it lies on its back).

How the creature equivalents map: beasts use `AttackUnarmed` for their
natural attack (gore and head toss, claw snap, spider bite, wraith rake,
wisp spit) and `Special` for their special (rear up, threat display with
claws clacking / spinnerets raised, wraith scream, wisp flare). Floaters
hover for `Idle` and fly for `Walk`/`Run` (rags and tail stream behind).
Animations a beast cannot do literally get its nearest equivalent (`Sit`
is a dog-sit for beasts and a hunker for crawlers, `Kneel` lies down, `Bow`
is a play-bow, `Wave` a head toss or a raised claw), and a test asserts that
no animation is a silent copy of `Idle` for any skeleton.

### The animator

`Animator` holds a looping base state and an optional one-shot action:

- Changing the base crossfades from the last pose over `FADE` = 0.2 s.
- An action fades in over 0.08 s and out over 0.18 s. While the base is a
  moving one (`Anim::moving`) the action takes only the skeleton's upper
  body (`Skeleton::upper`), so you can swing while running; otherwise it
  takes the whole body.
- The stride phase advances by ground covered (`drive_at` measures it from
  the figure's position), at `Skeleton::step(speed, scale)` blocks a step,
  so feet do not skate whatever the speed.
- `Book` keeps one animator per figure (you, players, entities), on one
  clock, pruned when not drawn for 5 s.

## 3. Rig

`models.rs` draws parts through the pose: `bone(parent, pivot, pose, i)`
composes the bone's rotation and offset onto its parent frame. People hang
legs, skirts and the body from the pelvis (the skirt mesh is now pivoted at
the hip), arms from the spine, the weapon from the forearm's grip. Creature
scale (content `scale`, elites ×1.25) goes into the root transform instead
of rescaling vertices afterwards. Meshes, voxel detail and LODs are
unchanged. `push_person` and `push_avatar` keep their old signatures (idle
or walk, no state) for the front end and `dyno shot`; animated figures use
`push_person_anim` and `push_creature(.., &mut Animator, now)`.

## 4. What drives it (`app.rs`)

| Figure | State (base) | One-shots |
|---|---|---|
| You | dead → `Death`; deep water → `Swim`; off the ground → `JumpAir`, then `Fall` after 0.9 s or below −7 b/s; casting → `CastDirected` (damage), `CastOmni` (heals, HoTs, recall), `Channel` (other), `Kneel` (gathering); loot window open → `Kneel`; moving → `Walk`/`Run`/`Backpedal`/`Strafe*` from velocity against facing; held emote (`/sit`, `/kneel`, `/playdead`) until you move; auto-attacking → `Ready`; else `Idle` | `JumpStart`/`JumpLand` on leaving/touching ground; your attack (by what you hold) on each `Damage` you deal; `CastRelease` when an `Effect` leaves your hands; `HitReact` on `Hurt`; emotes `/wave /bow /cheer /point /roar`; a swing every 0.7 s while digging |
| Other players | in the air (above the ground by 0.45) → `JumpAir`/`Fall`; casting (`ServerMsg::Casting` is broadcast) → as yours; moving → as yours from pose velocity; else `Idle` | `CastRelease` when an `Effect` starts at them; a swing every 0.7 s while `Digging` |
| Creatures | moving → `Walk`/`Run` (by speed against the creature's speed); hit in the last 8 s or your auto-attack target → `Ready`; else `Idle` | `HitReact` when `EntityInfo` health drops (not on `EntityHurt`, which also fires on regen and evade resets); its attack when you take a `Hurt` and it is the nearest hostile in reach; its attack when an `Effect` bolt starts at it |
| NPCs | your dialog or vendor window is open with them → `Talk`; moving → `Walk`/`Run`; else `Idle` (fidgets) | |
| Corpses | `Death` if a creature vanished there within the last second (it just died), else `Dead` (it was already lying there when it came into view) | |

## 5. What the server would need to send

Everything above works from existing messages, but some of it is a guess
from your own point of view. To make every player see every other figure's
actions, the lead would add (no changes were made to `dyno-protocol` or
`dyno-server` here):

1. **`ServerMsg::Anim { entity: Option<u32>, player: Option<PlayerId>, anim: u8, target: Option<u32> }`**,
   broadcast to players in range, for one-shots the client cannot infer:
   - a creature's melee swing, on any player (today only the player hit sees
     it, inferred from `Hurt`, and only for the nearest hostile);
   - a creature's special (`CreatureAbility` charge start, flee) — `Special`;
   - a player's melee swing and instant abilities (auto-attack `Damage` goes
     only to the attacker; others see nothing) — the attack kind can come
     from the player's equipped weapon, so `anim` could be just
     "attack/special/release";
   - player emotes (`/wave` etc. are local today) — `ClientMsg::Emote { anim: u8 }`
     relayed as the above.
   Encoding `anim` as the index into `Anim::ALL` keeps it one byte.
2. **Player state flags in `Pose`** (or a `PlayerState { player, flags }` on
   change): `dead`, `sitting`, `in_combat` (for `Ready`), `swimming`,
   `stunned`. Remote deaths and sits are invisible today.
3. **`EntityState { id, flags }`** on change: `stunned`/`rooted` (for
   `Stun` on mobs; ability `stun`/`root` exist), `in_combat` (`Ready` for mobs
   fighting other players), `evading`.
4. **Hit reacts on players**: `EntityHurt`'s player twin, `PlayerHurt { id }`,
   so other players flinch when hit.
5. **The remote player's weapon** (`PlayerInfo.look` has no weapon): an
   equipped-weapon item id in `PlayerJoined`/on change, so remote players
   carry and swing what they wield.

## 6. Checking it

- `cargo test -p dyno-client anim`: every creature in
  `content/hearthblock.json` and seven kinds of player (unarmed, sword, axe,
  staff, bow, robed, NPC) in every animation at six moments each build
  geometry that is finite, stays within 2.6 blocks sideways and 3 up and
  goes no more than 0.13 blocks under the ground (swimmers half a block);
  every pair of animations crossfading with a hit react on top stays
  finite and in its box; no animation is a copy of idle on any skeleton.
- `dyno shot --anim-sheet out.png` (no server needed): `out.png` is every
  figure in eight key poses (idle, walk mid-stride, run, attack mid-swing,
  cast or special, hit, death mid-fall, dead) from a third-person camera;
  `out-all-<figure>.png` is every animation for one figure of each
  skeleton; `out-motion.png` plays the one-shots out frame by frame. Writes
  PPM, or PNG through ImageMagick when the name ends in `.png`.
