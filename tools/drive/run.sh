#!/usr/bin/env bash
# The interface's input test: a fresh world, a new character, and the real
# client played off screen by tools/drive/first_steps.drive.
#
# usage: tools/drive/run.sh [path-to-dyno] [port] [shots-dir]
# Builds the release `dyno` when no binary is given. Needs a GPU or a
# software Vulkan (lavapipe), as `dyno shot` does. Exits 0 when every step held.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
bin="${1:-}"
port="${2:-7757}"
shots="${3:-}"  # defaults to a folder in the work dir (the script takes pictures)
if [ -z "$bin" ]; then
  (cd "$root" && cargo build --release --locked -p dyno >&2)
  bin="$root/target/release/dyno"
fi

work=$(mktemp -d)
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

# (a small picture: the frames stay quick on a busy machine)
args=("127.0.0.1:$port" --script "$root/tools/drive/first_steps.drive" --as Driver --class mage --size "${DRIVE_SIZE:-1280x720}")
[ -z "$shots" ] && shots="$work/shots" && mkdir -p "$shots"
args+=(--shots "$shots")
"$bin" drive "${args[@]}"
