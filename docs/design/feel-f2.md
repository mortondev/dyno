# Feel, rounds F1 and F2: baseline tools and combat impact

The plan is `docs/design/feel.md`. This note holds the goals of the first
two rounds, what was built for each, how each was checked, and the result.

## Goals

**F1: baseline tools**

1. **A debug overlay on F3:** fps, frame ms (mean and worst over the last
   second), ping (round trip), the server tick the client sees, the count of
   movement corrections (server `Teleport` snaps) this session, and entities
   drawn. Themed, off by default, its text in `content/ui/strings.json`.
2. **A feel test area,** reachable by an operator command (`/feel`) on a
   test server: a flat field, stairs, a ledge, a narrow corridor, and a
   training dummy that never dies and hits back lightly on a timer. Built as
   data, not in the live zone.
3. **A drive script** that records a 3-second clip, as a frame sequence, of
   each class landing a basic hit, a crit and a kill on the dummy. The
   "before" clips are kept in `docs/design/feel/before/` as contact strips.

**F2: combat impact**

4. **Hit-stop:** a 40–80 ms freeze of the attacker's and the target's
   animation when a melee hit lands, longer on crits and kills. Only
   animation freezes, never movement or input. All timings are data.
5. **Shake and recoil:** small, short, directional camera shake on your own
   crits and on heavy hits you take. Capped, with an Options toggle.
6. **Impact particles at the contact point:** per weapon or school and per
   target material. Data in `life.json`, triggered from the existing damage
   and effect messages.
7. **Enemy tells:** creatures telegraph a heavy attack 0.5–1 s ahead with a
   wind-up pose and a visible marker, so a player can step away or
   interrupt. Which attacks are heavy is server data; two or three early
   creatures at most have one; a 12-bot, 15-minute swarm shows deaths per
   hour do not rise.
8. **Stagger and stun reads:** a crit visibly staggers small creatures; a
   stunned creature shows a daze effect.
9. **Kill moment:** a short death animation plus a particle pop and a loot
   sparkle. The body does not vanish on the same frame.
10. **Instant press feedback:** a pressed ability starts its cast or swing
    animation and a button pulse on the same frame, before the server
    answers. A refusal cancels it cleanly with the existing error sound. A
    press during the global cooldown shows the queued pulse.
11. **Proof:** "after" clips from the same drive script, in
    `docs/design/feel/after/`, and a blind judge who picks "after" as "hits
    land" for all six classes.
12. **Cost and gates:** `dyno bench` within the budgets of
    `docs/design/performance.md`; fmt, clippy, the tests and
    `check-client.sh` pass; `tools/drive/run.sh` still passes.
13. **Obeys the voxel rule** (`docs/art/voxel-rule.md`): impact particles,
    tell markers, daze effects, loot sparkles and the test yard are cubes on
    the size ladder; a ground marker is a ring of cubes.

## Results

(Filled in at each hand-in.)
