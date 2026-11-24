# Netcode: movement that never fights you (feel F3)

The plan is `docs/design/feel.md`, F3. This is the design, the goals with
their pass lines (written before measuring), and the results.

## What changed

| | Before | Now |
|---|---|---|
| **Who moves you** | The client moved itself and sent its pose 20 times a second; the server checked it (a speed budget, a hang timer) and snapped you back with `Teleport` when a check failed. | The client sends **steps**: how long (whole ms, 1–50), the movement keys, and the facing (`ClientMsg::Steps`, numbered). The server runs them with the same `dyno_sim::movement` code and is the authority (`dyno-server/src/movement.rs`). |
| **Cheats** | Caught by checks with slack for lag (8.5 m/s, 14 m bursts, 1.6 s of hang). | Impossible by construction: a position never comes from the client. Steps spend a real-time budget, so sending them faster only queues them (and drops them beyond 2 s). Holding jump is a jump. |
| **Your own movement** | Moved at once, at the frame rate. | Predicted at once with the same steps (`dyno-client/src/predict.rs`). The server's answer (`ServerMsg::Moved`: the body after step `seq`) is taken as the truth: the client puts its body there and runs its unanswered steps again. A difference is eased away over about 120 ms; over 2 m it snaps. |
| **Others** | Drawn between the last two poses received, by arrival time. | Drawn from a buffer of positions stamped with the mover's own clock, 100 ms behind on a clean line (more, up to 200 ms, on one that keeps stalling), carried on at most 150 ms past the newest (`dyno-client/src/interp.rs`). |
| **Knockback** | The client added the push to its own velocity. | The server adds it to the body it runs; the client sees it in the next `Moved`. |
| **Teleports** (recall, rides, blink, charge, respawn, `/tp`, `/feel`) | `Teleport`. | `Teleport` still, through one door on the server (`Player::put`). The client counts them in its steps (`teleports`): steps sent before the latest one are answered but never run, so a teleport is never followed by a correction. |

### The steps

A frame of any length becomes steps of whole milliseconds (the fraction is
carried to the next frame, so the steps add up to the time that passed; a
frame over 50 ms is split). Each carries `forward` and `right` (-127..127),
the keys (jump, down, sprint) and the facing in 65536ths of a turn. The
client runs the step it sends, rounded as it travels, so both sides run the
very same numbers. Steps go out 30 times a second in one message.

Build mode is unchanged: operators who build fly and place themselves by
pose. A server started with `trust_movement` (the protocol tests, and
`dyno server --trust-movement` for hand measurements) also takes poses;
the live server never does.

### The server's side

Each tick (50 ms), before the creatures move, every player's queued steps
run while their real-time budget lasts (`State::tick_moves`):

- **The budget** grows with real time and is spent by the steps' time. It
  banks up to 1 s, so a burst of late steps after a stall (a lost packet
  holds a TCP line for a quarter of a second) runs at once and the client's
  prediction stays right. It starts fresh (200 ms) at a player's first
  steps, so time spent joining is not banked.
- **The queue** holds at most 2 s of steps; older ones beyond that are
  dropped (answered, never run). Only a client sending faster than real
  time fills it.
- **A silent client** (no steps for over a second: a long stall, a
  minimised window) is stood still by the server from then on, a tick at a
  time: its body falls and takes knockback like everyone's. When it comes
  back with the steps of that time in one burst (a client catches up on up
  to 2 s), what the bank cannot run at once, up to the time it was stood
  still, is answered unrun, so the queue never lags real time afterwards.
- **After a teleport, a death, build mode or a trusted pose** the queue,
  the budget and the silence start over.

At the end of the tick each player whose body changed gets `Moved` (so
knockback dealt during the tick is in it).

**The one allowance:** a player who has stood silent for a second can then
run a second of steps at once, as a stalled line's late steps must. That is
about 6 m in one tick, once; the average never beats running.

### The client's side

`Predictor::advance` turns the frame into steps and runs them on the
body; `flush` sends them; `ack` takes `Moved`, drops the answered steps,
puts the body where the server has it and runs the rest again. What was on
screen less where that lands is the correction: added to an offset eased
away with a 40 ms time constant (95% gone in 120 ms), or, over 2 m, shown
at once and counted as a snap. Where the world under the body has not
streamed in, the steps are idle (the server would otherwise walk you where
you cannot see).

The F3 overlay shows the corrections: snapped, eased (over 5 cm) and the
last one's size, and the teleports apart from them. `DYNO_PREDICT_LOG=1`
prints each correction (client, bots); `DYNO_MOVE_LOG=1` logs what the
server does that is not plain running (steps dropped, a silent player
stood still, steps answered unrun).

### Remote smoothing

`Poses` carry, for each player, the time of their movement clock: the
milliseconds of steps the server has run for them. So a player's position
is stamped with *when they made that move*, not when the server happened
to run it (steps arrive at the server in bursts on a poor line). Creatures
are stamped with the server's clock (`EntityMoves`, now every tick, 20 Hz,
and sent empty too, so an entity missing from it is known to stand still).

- **The clock** (`interp::Clock`, one per remote player and one for the
  creatures) is the time drawn, on the sender's clock. It aims 100 ms
  behind the newest position as positions usually arrive (the median
  arrival of the last 2 s). It gets there by running at most 3% fast or
  slow, never by a jump (except to start, or when it is more than 0.6 s
  off: a new shard, a long stall). When positions stop coming it eases to
  a stop before it is 150 ms past the newest, and catches up when they come.
- **Earned delay:** while it is drawing past the newest position the clock
  earns more delay, up to 100 ms more, and gives it back at 10 ms a second
  once the line is clean. On a clean line others are drawn 100 ms behind;
  on a TCP line losing 2% of its packets, a lost packet holds everything for
  about 270 ms, more than 100 ms of buffer and 150 ms of carrying on can
  cover, and the delay settles near 170–200 ms.
- **The track** (`interp::Track`, one per mover) draws between the two
  positions around that time, or past the newest at the last speed for at
  most 150 ms. A creature that stopped is held, not carried on; one that
  sets off after a rest is drawn setting off when the last batch without
  it came, not sliding from where it stopped. A jump of over 8 m is a
  teleport: drawn there at once.

The brief asked for "about 100 ms behind": that is what a clean line gets.
The earned delay is a deviation, taken because on this transport (TCP) a
lost packet's stall is longer than the brief's buffer and cap together.

### The network conditioner

`DYNO_NET_SIM=lat=150,jitter=30,loss=2` (client and bots;
`JoinOptions::net_sim` in tests) holds back what the client sends and
receives:

- `lat` is the round trip added: half each way;
- `jitter` is up to that much more each way, at random;
- `loss` is the share of packets lost each way;
- `seed` (optional) makes the same packets late and lost each run.

The game runs over TCP (WebSocket), so a lost packet is never missing: it
comes again after a retransmission timeout, and everything behind it waits
(head-of-line blocking). The conditioner does the same: a lost packet costs
the larger of 200 ms (Linux's least timeout) and the round trip plus four
times the jitter (270 ms here), and nothing overtakes it. Messages that
leave within 3 ms of each other share a packet.

### How it is checked

| Tool | What it does |
|---|---|
| `crates/dyno-client/tests/netcode.rs`, `prediction_holds_under_a_poor_line` | A real server and two clients through the conditioner (150 ms, 30 ms, 2%, fixed seeds). A runner laps a course built for it high over the town (walk, run, sprint, jump, stop and start, strafe, stairs to a 3 m deck, a 3 m ledge, a 1 m wide doorway, a 1 m step, a flat lane back); a watcher draws it, and the walking creatures and townsfolk it sees, from the buffer and as before, and measures drawn speed. 50 s in `cargo test`; `DYNO_NETCODE_SECS=600` for the 10-minute run; `DYNO_NETCODE_MODE=pose` for the old way. |
| the same file, `measure_crowd` (ignored) | 40 players in one area, all running circles: bytes received and sent a player a second by message kind, and the tick's cost. |
| `tools/netcode/session.sh` | The real client (`dyno drive`, off screen) on the poor line, lap after lap of `tools/netcode/lap.drive`: the feel yard (stairs, deck, ledges, the half-roofed corridor), open ground (jumping as it runs, sneaking, sprinting, strafing, backing), `/feel` and `/spawn` teleports, and a doorway in town walked in and out. Fails on a snap; prints the overlay's counts. |
| `crates/dyno-server/src/movement.rs` tests | Speed hacks queue, held jump is a jump, a silent body falls, stalls of any length leave no lag, steady steps after a silence all run, steps from before a teleport are skipped, no idle flood after a death. |
| `crates/dyno-client/tests/protocol.rs` | `cheating_movement_is_impossible` (the old movement test, adapted: a pose, a speed hack and a held jump get nowhere), `knockback_moves_the_body_on_the_server`. |
| `crates/dyno-client/tests/cluster.rs` | `running_through_a_move_between_instances_is_not_snapped`. |

Bots predict with the same code (`dyno_client::predict`) in 25 ms slices,
as they moved before.

## Goals and results

Pass lines were set before measuring. The measurements are in the next
section.

| # | Goal | Pass line | Result |
|---|---|---|---|
| 1 | Server-authoritative movement | Inputs with sequence numbers; the server simulates with `dyno_sim::movement`; a pose from a player in play is ignored; steps sent 10× faster than real time move no faster than running; holding jump never rises more than a jump; the existing movement tests adapted and green; tick cost with 40 players well inside 50 ms | **Pass.** `cheating_movement_is_impossible` (the old test, adapted) and 8 server unit tests green. 40 in one area: tick mean 0.46 ms, p95 0.75 ms, worst 4.6 ms (before 0.38 / 0.75 / 4.6); a 40-bot swarm: mean 1.0 ms, p95 2.5 ms, worst 19.7 ms. Allowance: one 1 s burst after a 1 s silence (see above). |
| 2 | Prediction and reconciliation | 150 ms latency, 30 ms jitter, 2% loss: zero snaps in a 10-minute scripted session (walk, run, sprint, jump, stairs, a ledge, a doorway, the /feel yard) | **Pass.** Real client, `tools/netcode/session.sh`, 14 laps in 632 s: 0 snapped, 0 eased, worst correction 0.00 m, 10,331 answers, 28 teleports. Harness, 600 s, 32 laps: 0 snapped, 0 eased. Also 0 at a 300 ms round trip. |
| 3 | Remote smoothing | Same line: the watched runner's drawn speed on a straight run is within 0.5 m/s RMS of its true speed (before: measured), with no frame off by more than 2 m/s outside loss stalls | **Pass on the RMS line; the second clause holds but for one frame.** 600 s: 0.477 m/s RMS over every frame (before 7.17); 0.118 on frames drawn from the buffer, of which 1 in 8,151 was off by over 2 m/s (5.7 m/s, at the end of a long stall). Walking creatures and townsfolk: 0.126 m/s RMS against their stamped speed; frame-to-frame change 0.19 m/s (before 0.43). Clean line (no loss): 0.033 m/s (before 3.38). Deviation: the delay grows past 100 ms on a stalling line. |
| 4 | Pose rate | Bandwidth per player with 40 in one area measured before and after; under 64 KB/s received | **Pass, at 20 Hz** (30 Hz not needed: 0.033 m/s on a clean line). Received 20.8 KB/s a player (before 17.2: stamps, 20 Hz creature moves, `Moved`); sent 0.6 KB/s (before 0.33). |
| 5 | Everything that moves players | Shard moves, cluster relays (cluster tests green), rides, knockback, recall and teleports, anti-cheat rubber-band, bots (their tests, including the real-time ones), `dyno drive` (`tools/drive/run.sh`) | **Pass.** Cluster tests green, with a new one running through a move between instances (no snap); the shard and crowding protocol tests green; rides (bots' `discovery_pays_once_and_rides_need_it`), `/feel`, `/spawn` (28 teleports in the session); `knockback_moves_the_body_on_the_server`; every `dyno-bot` test; a 40-bot swarm; `tools/drive/run.sh` PASS (0 snaps). |
| 6 | Protocol | Still 26; a copy of the live world's saves loads unchanged | **Pass.** 26. A copy of `~/dyno-world` loaded without a rebuild (layout v6), a stepping bot moved on it and was saved; the live folder untouched. |
| 7 | Gates | fmt, clippy `-D warnings`, `cargo test --workspace`, `check-client.sh` | See the hand-in. |

## Measurements

**The line** is the conditioner at `lat=150,jitter=30,loss=2` (150 ms round
trip). The brief's "150 ms of latency" could mean one way; at
`lat=300` (120 s): prediction still 0 corrections; the runner 0.90 m/s RMS
(before 10.8); creatures' frame-to-frame change 0.91 m/s against 0.69 as
before: on that line a lost packet holds the line for 420 ms, longer than
the earned delay covers, and they ease and catch up visibly.

**Before** is the old path run in the same harness (`DYNO_NETCODE_MODE=pose`:
poses 20 times a second, trusted, drawn between the last two by arrival).

| Measure | Before | After |
|---|---|---|
| Snaps, 10 minutes on the line (real client) | (corrections were teleports from failed checks) | 0 (0 eased) |
| Runner drawn speed, RMS error, every frame | 7.17 m/s (worst 64) | 0.477 m/s (worst 6.1) |
| Runner, frames drawn from the buffer | | 0.118 m/s |
| Runner, clean line (150/30/0) | 3.38 m/s | 0.033 m/s |
| Creatures and townsfolk, frame-to-frame speed change | 0.43 m/s | 0.19 m/s |
| Received a player, 40 in one area | 17.2 KB/s | 20.8 KB/s |
| Sent a player | 0.33 KB/s | 0.61 KB/s |
| Tick, 40 in one area (mean / p95 / worst) | 0.38 / 0.75 / 4.6 ms | 0.46 / 0.75 / 4.6 ms |

Received after, by kind (40 in one area): Poses 18.3 KB/s, EntityMoves
1.9 KB/s (20 Hz, was 10 Hz), Moved 0.55 KB/s.
