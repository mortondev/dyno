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
| You | dead → `Death`; deep water → `Swim`; off the ground → `JumpAir`, then `Fall` after 0.9 s or below −7 b/s; moving → `Walk`/`Run`/`Backpedal`/`Strafe*` from velocity against facing (moving also cancels casts on the server); casting, while the cast bar runs → `CastDirected` (damage), `CastOmni` (heals, HoTs, recall), `Channel` (other), `Kneel` (gathering); loot window open → `Kneel`; held emote (`/sit`, `/kneel`, `/playdead`) until you move; auto-attacking → `Ready`; else `Idle` | `JumpStart`/`JumpLand` on leaving/touching ground; your attack (by what you hold) on a `Damage` to your auto-attack target (at most every 0.75 s, so damage-over-time ticks do not swing) and at once when you use an instant weapon ability; your ranged auto-shot (an `Effect` in the auto-shot colour from you) plays your attack; `CastRelease` when a spell `Effect` leaves your hands or you use an instant spell; `HitReact` on `Hurt`; emotes `/wave /bow /cheer /point /roar`; swing after swing while digging |
| Other players | in the air (no block under the feet and moving up or down) → `JumpAir`/`Fall`; moving → as yours from pose velocity; casting (`ServerMsg::Casting` is broadcast; expires with the cast bar) → as yours; else `Idle` | `Shoot` for an auto-shot `Effect` from them, `CastRelease` for any other `Effect` from them; swing after swing while `Digging` |
| Creatures | moving → `Walk` (wandering, at 0.45 of its speed) or `Run` (chasing, charging, fleeing, evading: 0.62 of its speed or more); hurt (below full health and hit in the last 8 s) or your auto-attack target → `Ready`; else `Idle` (a crab turns its face to the front after standing 0.35 s) | `HitReact` when `EntityInfo` health drops (not on `EntityHurt`, which also fires on regen and evade resets); its attack when you take a `Hurt` and it is the nearest hostile in reach; its attack when an `Effect` bolt starts at it |
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

## 7. Figures, rigs and clips as pack data (step 2 of data-and-streaming)

The builders in `models.rs` and the pose functions in `anim.rs` were run
once by a bake (in git history: `crates/dyno-client/src/bake.rs`, commit
"Bake figures, rigs and clips into the pack") and deleted. The client now
carries no figure content; everything below comes through the asset store.

- **`figures.json`**
  - `appearance`: the character creator's options (`skin`, `hair`, `eyes`
    colour lists, keyed by the protocol's `Appearance` fields) and the
    features it sets (`hair_style`, `beard`, `build`, `mood`, `ears`).
  - `player`: the player figure (`person`) and how a traveller is drawn
    from a seed: `{"pick": [...], "shift": n}` picks by bits of the hash,
    `{"pick": [...], "by": "armor"}` by another feature, `{"from": "color",
    "scale": k}` follows the signature colour.
  - `npcs`: `*` for all townsfolk, then named overrides (features, colours,
    `null` to take one off, `remap` of values, a held `weapon`).
  - `creatures`: `"<Model>/<variant>"` (or `/*`) → figure, an optional
    look (as for NPCs, colours may follow the creature's `color`) and what
    it holds.
  - `held`: held kinds (figure, `unit` blocks per voxel, `carry_turn`,
    `grip`, first-person `fp` = size, tilt, roll, lift) and how items map
    to them (by weapon kind, tool kind, gear slot, icon; the torch; the
    empty hand; `glow_names`).
  - `figures`: per figure its `.vox` file, `skeleton`, `rig`, clip set
    (`"name"`, or `{"features": ["robe"]}` for grip + features), LODs,
    grids (bounds, part names, groups of layers with `when` conditions and
    value `map`s) and the palette: each index is a fixed colour (`c`) or
    follows a look colour (`slot`, per-channel `k` and `c`:
    `clamp(k * colour + c)`), optionally glowing.
- **`models/<figure>.vox`** (MagicaVoxel, one palette per file): models
  named `<group>/<layer>/<part>/<mode>`. A look is composed group by group
  (legs, torso, arms, cape, head for people): the base layer, then each
  layer whose `when` matches, overriding cell by cell; then the group is
  applied with each cell's mode: `set`, `add` (only into empty space),
  `erase`, `dye` (recolour what is there), `add-dye`, or `drop` (undo an
  earlier layer of the group). Layers are per feature value (a hair style,
  a beard, plate armour) and name only the earlier features they depend on.
- **`rigs/<skeleton>.json`**: the skeleton's pose bones, Euler orders
  (default `yxz`), the upper-body mask, the step formula (ground per step
  from speed), and per figure rig: bones (`parent`, `at` in voxels, which
  may vary by a feature, `ops`: `"pose"`, `rx/ry/rz`, `move`, `scale`, with
  expressions over `clock`, `flow`, `shape`, `spread`, `pitch` and bone
  offsets — capes, rags, wisp motes and tail are written this way) and the
  parts drawn in order on them (mesh pivot, `glow`, `light`, `held`).
  People are 1.78 m with heads at 0.55 scale (`scale` op on `head`).
- **`anims/<skeleton>.json`**: clip sets (`default` plus sets holding only
  what differs: `one_hand+robe`, `thornboar`...). A clip has `duration`
  (one-shot, `hold` for death) or `loop`, `time` channels and, for
  locomotion, `stride` channels over one stride cycle that add to them.
  Channels are `<bone>.rx|ry|rz` (degrees), `<bone>.ox|oy|oz` (voxels),
  `flow|shape|spread`; a number or linear `[t, v]` keys. `seed` is the
  figure seed the loops were sampled at; other seeds start further in.
  - **Compact form.** The pack build compiles each file into
    `anims/<skeleton>.clips` (`dyno_protocol::clips`), which is what the
    server streams: the JSON tree in binary with a string table, and each
    channel thinned (keys that linear interpolation reproduces within 0.1°,
    0.02 voxels or 0.002 of a shape key are dropped), quantised (times on a
    1/1920 s or 1/4096-of-a-stride grid, values in 0.01°, 0.002 voxel or
    0.0005 steps, delta coded) and shared (a channel equal to one already
    written is a reference). 4.0 MB of JSON becomes 178 KB (106 KB
    deflated on the wire; the biped's 34 KB). A test plays every channel
    back against its source within tolerance; edit the JSON and the
    change flows through the next server start or `/reload`.
  - **Loops** are 6 to 16 s. The baked ones ran 20 to 36 s (periods that
    did not divide); they were cut once, keeping each clip's most
    distinctive moment (the fidget), at the window whose ends match best in
    value and speed, and the last 1.5 s blend into the start so the seam is
    exact (the biped's idle: 26.2 s to 12.0 s, ends 0.09° apart before the
    blend). A test checks every loop meets itself.
  - **Mirroring.** `mirror_pairs` names the left and right bones; a clip
    with `"mirror": true` may play mirrored (bones swap, turns about y and
    z and moves along x change sign). The animator picks the side each
    time an action starts, from the figure's seed and its count of plays,
    so the same history poses the same. `HitReact` flinches either way on
    bipeds, wraiths and quadrupeds.
  - **Hover** is explicit in the wraith rig: the body bone sits at the
    waist (`at` `[0,20,0]`, where its mesh pivots) and a `move` op lifts it
    4 voxels before the pose, rather than a pivot 4 voxels above its mesh.
- **Engine** (`figure.rs`, `clip.rs`, `expr.rs`, `vox.rs`, `voxmodel.rs`,
  `anim.rs`): reads it all, caches meshes per look, blends, layers and
  crossfades as before; a grey stand-in is drawn until a figure's files
  arrive. `Anim` stays the list of standard names; the pack test
  `every_figure_has_every_animation` replaces the compile-time check.

## Proportions are data (part shapes)

A rig part may carry a `shape`, applied to its mesh about the part's pivot:

```json
{"part": "thigh.r", "bone": "thigh.r", "pivot": [5, 26, 0],
 "shape": {"scale": [1, 1.15, 1], "taper": [0.80, 0.62], "taper_depth": true}}
```

- `scale`: across, along and front to back.
- `taper`: width at the part's top and bottom (x, and z unless
  `taper_depth` is false), linear in between.

A shape may vary by features, like any rig vector: `{"by": "build",
"values": [...]}`, or `{"by": ["body", "build"], "map": {"1,2": ...,
"*": ...}}` (several features' values joined with commas; `*` matches any).

Human proportions now live in the sculpt (see docs/design/characters.md).
The person is modelled at heroic proportions in `models/person.vox`, so its
rig carries no tapers. What remains in the rig:
- `build` (slight, average, burly) is one girth `scale` for every part
  (0.93, 1, 1.1 across), so the seams between parts stay closed; the
  shoulder and hip `at` change with body and build;
- `body` (masculine, feminine) picks each bone's `at` and each part's `pivot`;
- the head is drawn at true size (no `scale` op);
- `per_block` (34.19) keeps the overall height at 1.78 m;
- the figure is modelled at twice its rig's density (`detail` 2, and 4 for
  the head grid), so the rig and clip numbers are unchanged;
- sockets (`socket.offhand`, `socket.shield`, `socket.back`,
  `socket.helm`, `socket.shoulder.r/l`, `socket.cloak`, `socket.buckle`)
  are rig-only bones; a part `"held": "<socket>"` draws what the look
  carries there (`Look::carry`, set by an item's `carry` in `equip`).

The rig, the layers and the figure entry are generated by
`dyno bake-figure` (crates/dyno/src/sculpt/).
