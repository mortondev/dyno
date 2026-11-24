#!/usr/bin/env bash
# The feel checks a picture can show (tools/feel/check.drive) on the feel
# pack (tools/feel/make_pack.py).
#
# usage: tools/feel/check.sh PATH-TO-DYNO PORT OUT_DIR
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
bin="$1"
port="$2"
out="$3"
work=$(mktemp -d)
python3 "$root/tools/feel/make_pack.py" "$work/pack" >&2
"$bin" server --port "$port" --world "$work/world.dynw" --content "$work/pack" --op FeelCheck >"$work/server.log" 2>&1 &
server=$!
cleanup() {
  kill "$server" 2>/dev/null || true
  wait "$server" 2>/dev/null || true
  rm -rf "$work"
}
trap cleanup EXIT
for _ in $(seq 600); do
  grep -q "listening" "$work/server.log" && break
  kill -0 "$server" 2>/dev/null || { cat "$work/server.log" >&2; exit 1; }
  sleep 1
done
mkdir -p "$out"
"$bin" drive "127.0.0.1:$port" --script "$root/tools/feel/check.drive" --as FeelCheck --class warrior \
  --size "${FEEL_SIZE:-1280x720}" --shots "$out"
