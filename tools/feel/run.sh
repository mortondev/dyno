#!/usr/bin/env bash
# The feel clips: a test server on the feel pack (tools/feel/make_pack.py),
# each class played off screen by tools/feel/clips.drive, and a contact
# strip a class (tools/feel/strip.py).
#
# usage: tools/feel/run.sh PATH-TO-DYNO PORT OUT_DIR [class ...]
# OUT_DIR gets <class>.jpg; the frames stay in a work folder that is removed
# (FEEL_KEEP=1 keeps it).
# Needs a GPU or a software Vulkan, python3 and ImageMagick.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
bin="$1"
port="$2"
out="$3"
shift 3
classes=("$@")
[ ${#classes[@]} -gt 0 ] || classes=(warrior paladin hunter rogue priest mage)

work=$(mktemp -d)
python3 "$root/tools/feel/make_pack.py" "$work/pack" >&2
ops=()
for c in "${classes[@]}"; do ops+=(--op "Feel$c"); done
"$bin" server --port "$port" --world "$work/world.dynw" --content "$work/pack" "${ops[@]}" >"$work/server.log" 2>&1 &
server=$!
cleanup() {
  kill "$server" 2>/dev/null || true
  wait "$server" 2>/dev/null || true
  if [ -n "${FEEL_KEEP:-}" ]; then echo "kept $work" >&2; else rm -rf "$work"; fi
}
trap cleanup EXIT

for _ in $(seq 600); do
  grep -q "listening" "$work/server.log" && break
  kill -0 "$server" 2>/dev/null || { cat "$work/server.log" >&2; exit 1; }
  sleep 1
done
grep -q "listening" "$work/server.log" || { echo "the server did not start" >&2; exit 1; }

mkdir -p "$out"
for c in "${classes[@]}"; do
  case "$c" in
    hunter | priest | mage) stand=7 ;;
    *) stand=2.2 ;;
  esac
  sed -e "s/CLASS/$c/g" -e "s/STAND/$stand/g" "$root/tools/feel/clips.drive" >"$work/$c.drive"
  "$bin" drive "127.0.0.1:$port" --script "$work/$c.drive" --as "Feel$c" --class "$c" \
    --size "${FEEL_SIZE:-1280x720}" --shots "$work/clips"
  python3 "$root/tools/feel/strip.py" "$work/clips" "$c" "$out/$c.jpg"
  [ -n "${FEEL_KEEP:-}" ] || rm -rf "$work/clips"
done
