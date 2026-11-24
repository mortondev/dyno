#!/usr/bin/env bash
# The sounds and the music in the pack are exactly what the tools write:
# regenerate them into a scratch directory, twice, and compare all three
# byte for byte. (Pack files are named by their hash: a sound that drifted
# between runs would make every client download it again.)
#
# usage: tools/audio/check.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
pack="$here/../../content"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

for run in a b; do
  python3 "$here/feedback.py" --out "$tmp/$run/sounds" >/dev/null
  python3 "$here/ambience.py" --out "$tmp/$run/sounds/amb" >/dev/null
  python3 "$here/instruments.py" --out "$tmp/$run/music/inst" >/dev/null
  python3 "$here/compose.py" --out "$tmp/$run/music" >/dev/null
done
diff -r "$tmp/a" "$tmp/b" >/dev/null || {
  echo "two runs differ" >&2
  exit 1
}

n=0
while IFS= read -r f; do
  rel="${f#"$tmp/a/"}"
  cmp -s "$f" "$pack/$rel" || {
    echo "content/$rel is not what the tool writes" >&2
    exit 1
  }
  n=$((n + 1))
done < <(find "$tmp/a" -type f | sort)
sum="$(cd "$tmp/a" && find . -type f | sort | xargs cat | sha256sum | cut -c1-16)"
echo "$n files: two runs agree with each other and with content/ ($sum)"
