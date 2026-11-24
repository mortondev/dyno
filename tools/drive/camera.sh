#!/usr/bin/env bash
# The play camera's test (F5, docs/design/camera.md): a fresh world, a new
# character who is its operator, and the real client played off screen by
# tools/drive/camera.drive, which builds a yard (a house with doors, a tree,
# stairs, a cliff), runs through it and fails on any camera pop.
#
# usage: tools/drive/camera.sh [path-to-dyno] [port] [shots-dir]
# Builds the release `dyno` when no binary is given. Needs a GPU or a
# software Vulkan (lavapipe), as `dyno shot` does. Exits 0 when every step held.
# DYNO_CAMERA=instant runs it with the camera placed at once each frame (the
# camera before F5): that run is expected to fail.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
bin="${1:-}"
port="${2:-7758}"
shots="${3:-}"
if [ -z "$bin" ]; then
  (cd "$root" && cargo build --release --locked -p dyno >&2)
  bin="$root/target/release/dyno"
fi

work=$(mktemp -d)
# (the client's settings stay in the work dir, not the user's)
export DYNO_CONFIG_DIR="$work/config"
"$bin" server --port "$port" --world "$work/world.dynw" --op Driver >"$work/server.log" 2>&1 &
server=$!
cleanup() {
  kill "$server" 2>/dev/null || true
  wait "$server" 2>/dev/null || true
  rm -rf "$work"
}
trap cleanup EXIT

# the world is built on the first start: wait for the port
for _ in $(seq 240); do
  grep -q "listening" "$work/server.log" && break
  kill -0 "$server" 2>/dev/null || { cat "$work/server.log" >&2; exit 1; }
  sleep 1
done
grep -q "listening" "$work/server.log" || { echo "the server did not start" >&2; exit 1; }

args=("127.0.0.1:$port" --script "$root/tools/drive/camera.drive" --as Driver --class warrior --size "${DRIVE_SIZE:-1280x720}")
[ -z "$shots" ] && shots="$work/shots" && mkdir -p "$shots"
args+=(--shots "$shots")
"$bin" drive "${args[@]}"
