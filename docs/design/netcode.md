# Netcode: movement that never fights you (feel F3)

The plan is `docs/design/feel.md`, F3. This is the design, the goals with
their pass lines (written before measuring), and the results.

## What changed

| | Before | Now |
|---|---|---|
| **Who moves you** | The client moved itself and sent its pose 20 times a second; the server checked it (a speed budget, a hang timer) and snapped you back with `Teleport` when a check failed. | The client sends **steps**: how long (whole ms, 1–50), the movement keys, and the facing (`ClientMsg::Steps`, numbered). The server runs them with the same `dyno_sim::movement` code and is the authority (`dyno-server/src/movement.rs`). |
| **Cheats** | Caught by checks with slack for lag (8.5 m/s, 14 m bursts, 1.6 s of hang). | Impossible by construction: a position never comes from the client. Steps spend a real-time budget, so sending them faster only queues them (and drops them beyond 2 s). Holding jump is a jump. |
| **Your own movement** | Moved at once, at the frame rate. | Predicted at once with the same steps (`dyno-client/src/predict.rs`). The server's answer (`ServerMsg::Moved`: the body after step `seq`) is taken as the truth: the client puts its body there and runs its unanswered steps again. A difference is eased away over about 120 ms; over 2 m it snaps. |
| **Others** | Drawn between the last two poses received, by arrival time. | Drawn from a buffer of positions stamped with the mover's own clock, 100 ms behind, carried on at most 150 ms past the newest (`dyno-client/src/interp.rs`). |
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
run while their real-time budget lasts (`State::tick_moves`). The budget
banks up to 1 s while no steps come, so a burst of late steps after a stall
runs at once and the client's prediction stays right. Past that bank a
silent client (a stall over a second, a minimised window) is stood still
by the server: its body falls and takes knockback like everyone's. At the
end of the tick each player whose body changed gets `Moved` (so knockback
dealt during the tick is in it).

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
last one's size, and the teleports apart from them.

### Remote smoothing

`Poses` carry, for each player, the time of their movement clock: the
milliseconds of steps the server has run for them. So a player's position
is stamped with *when they made that move*, not when the server happened
to run it (steps arrive at the server in bursts on a poor line). Creatures
are stamped with the server's clock (`EntityMoves`). A `Clock` maps the
sender's time to ours by the quickest arrival of the last 2 s, moving that
mapping no faster than 2% (so time on screen never jumps); a `Track` draws
100 ms behind that, between two positions, or carried on at most 150 ms
past the newest. A creature that stops is held, not carried on: the next
batch comes without it.

### The network conditioner

`DYNO_NET_SIM=lat=150,jitter=30,loss=2` (client and bots;
`JoinOptions::net_sim` in tests) holds back what the client sends and
receives:

- `lat` is the round trip added: half each way;
- `jitter` is up to that much more each way, at random;
- `loss` is the share of packets lost each way.

The game runs over TCP (WebSocket), so a lost packet is never missing: it
comes again after a retransmission timeout, and everything behind it waits
(head-of-line blocking). The conditioner does the same: a lost packet costs
the larger of 200 ms (Linux's least timeout) and the round trip plus four
times the jitter (270 ms here), and nothing overtakes it. Messages that
leave within 3 ms of each other share a packet.

## Goals and results

Pass lines were set before measuring. The measurements are in the next
section.

| # | Goal | Pass line | Result |
|---|---|---|---|
| 1 | Server-authoritative movement | Inputs with sequence numbers; the server simulates with `dyno_sim::movement`; a pose from a player in play is ignored; steps sent 10× faster than real time move no faster than running; holding jump never rises more than a jump; the existing movement tests adapted and green; tick cost with 40 players well inside 50 ms | |
| 2 | Prediction and reconciliation | 150 ms latency, 30 ms jitter, 2% loss: zero snaps in a 10-minute scripted session (walk, run, sprint, jump, stairs, a ledge, a doorway, the /feel yard) | |
| 3 | Remote smoothing | Same line: the watched runner's drawn speed on a straight run is within 0.5 m/s RMS of its true speed (before: measured), with no frame off by more than 2 m/s outside loss stalls | |
| 4 | Pose rate | Bandwidth per player with 40 in one area measured before and after; under 64 KB/s received | |
| 5 | Everything that moves players | Shard moves, cluster relays (cluster tests green), rides, knockback, recall and teleports, anti-cheat rubber-band, bots (their tests, including the real-time ones), `dyno drive` (`tools/drive/run.sh`) | |
| 6 | Protocol | Still 26; a copy of the live world's saves loads unchanged | |
| 7 | Gates | fmt, clippy `-D warnings`, `cargo test --workspace`, `check-client.sh` | |

## Measurements

(filled in as measured)
