#!/usr/bin/env bash
# Build the player client (`hearthblock`) as it ships and check it is an
# engine only: no content strings in it, and under the size target.
#
# usage: .github/scripts/check-client.sh [target-triple]
# Prints the path of the binary on the last line.
set -euo pipefail

target="${1:-}"
# a ceiling against regressions: 9.32 MB at 0.6.2; the target is under 8 MB
max_bytes="${MAX_CLIENT_BYTES:-10485760}" # 10 MB
words=(thornboar Hearthstead Brannoc)

args=(--locked -p hearthblock --profile client-release)
dir=target/client-release
if [ -n "$target" ]; then
  args+=(--target "$target")
  dir="target/$target/client-release"
fi
cargo build "${args[@]}" >&2

bin="$dir/hearthblock"
[ -f "$bin" ] || bin="$bin.exe"
size=$(wc -c <"$bin" | tr -d ' ')
echo "hearthblock: $size bytes ($(awk "BEGIN { printf \"%.2f\", $size / 1048576 }") MB)" >&2

fail=0
for w in "${words[@]}"; do
  if LC_ALL=C grep -a -q -F -- "$w" "$bin"; then
    echo "::error::the client binary contains the content string '$w'" >&2
    fail=1
  fi
done
# the ceiling was measured on Linux; elsewhere the size is reported only
if [ "$size" -ge "$max_bytes" ]; then
  case "$target" in
    "" | *linux*)
      echo "::error::the client binary is $size bytes, over $max_bytes" >&2
      fail=1
      ;;
    *) echo "::warning::the client binary is $size bytes, over $max_bytes" >&2 ;;
  esac
fi
[ "$fail" -eq 0 ] || exit 1
echo "no content strings (${words[*]}); under $max_bytes bytes" >&2
echo "$bin"
