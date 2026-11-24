#!/usr/bin/env bash
# The netcode session (docs/design/netcode.md, goal 2): the real client,
# off screen, on a poor line, played by tools/netcode/lap.drive lap after
# lap (about 50 s each; 12 laps is the 10-minute session) on a test server
# of the feel pack (tools/feel/make_pack.py). The overlay's movement
# corrections are reported at the end; a snap fails the run.
#
# usage: tools/netcode/session.sh PATH-TO-DYNO PORT [LAPS]
# The line is DYNO_NET_SIM (default lat=150,jitter=30,loss=2: 150 ms round
# trip, 30 ms jitter, 2% loss). Needs a GPU or a software Vulkan, and python3.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
bin="$1"
port="$2"
laps="${3:-12}"

work=$(mktemp -d)
python3 "$root/tools/feel/make_pack.py" "$work/pack" >&2
"$bin" server --port "$port" --world "$work/world.dynw" --content "$work/pack" --op Netcode >"$work/server.log" 2>&1 &
server=$!
cleanup() {
  kill "$server" 2>/dev/null || true
  wait "$server" 2>/dev/null || true
  if [ -n "${NETCODE_KEEP:-}" ]; then echo "kept $work" >&2; else rm -rf "$work"; fi
}
trap cleanup EXIT

for _ in $(seq 600); do
  grep -q "listening" "$work/server.log" && break
  kill -0 "$server" 2>/dev/null || { cat "$work/server.log" >&2; exit 1; }
  sleep 1
done
grep -q "listening" "$work/server.log" || { echo "the server did not start" >&2; exit 1; }

{
  echo "key F3"
  for _ in $(seq "$laps"); do cat "$root/tools/netcode/lap.drive"; done
  echo "expect snaps 0"
} >"$work/session.drive"

started=$(date +%s)
DYNO_NET_SIM="${DYNO_NET_SIM:-lat=150,jitter=30,loss=2}" "$bin" drive "127.0.0.1:$port" \
  --script "$work/session.drive" --as Netcode --class warrior \
  --size "${NETCODE_SIZE:-960x540}" --shots "$work/shots"
echo "session: $laps laps in $(( $(date +%s) - started )) s on ${DYNO_NET_SIM:-lat=150,jitter=30,loss=2}" >&2
