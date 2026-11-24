#!/bin/bash
# Stop leftover development processes that are no longer needed.
#
# - test servers (dyno server) on agent ports 7450-7799 running for over
#   3 hours; never the live server on 7400
# - stray `dyno shot`, `dyno bots` and `dyno play` runs older than 1 hour
# - scratch folders untouched for 12 hours
# - shell polling loops (`until ...; do sleep`) older than 2.5 hours, which wait
#   for builds that never end while other builds keep starting
#
# Only this user's processes; each is checked through /proc before it is
# stopped. Prints what it stopped. Usage: tools/dev/reap.sh [--dry-run]
dry=0; [ "$1" = "--dry-run" ] && dry=1
me=$(id -u)
age() { ps -o etimes= -p "$1" 2>/dev/null | tr -d ' '; }
stop() {
  if [ $dry = 1 ]; then echo "would stop $1 ($2)"; else kill "$1" 2>/dev/null && echo "stopped $1 ($2)"; fi
}
for pid in $(pgrep -u "$me" -x dyno); do
  cmd=$(tr '\0' ' ' < /proc/$pid/cmdline 2>/dev/null) || continue
  a=$(age $pid); [ -z "$a" ] && continue
  case "$cmd" in
    *" server "*)
      port=$(sed -n 's/.*--port \([0-9]*\).*/\1/p' <<<"$cmd")
      [ -z "$port" ] && continue
      [ "$port" = 7400 ] && continue
      case "$cmd" in */dyno-world/*) continue;; esac
      if [ "$port" -ge 7450 ] && [ "$port" -le 7799 ] && [ "$a" -gt 10800 ]; then
        stop $pid "test server on $port, up $((a / 60)) min"
      fi;;
    *" shot "*|*" bots "*|*" play "*)
      [ "$a" -gt 3600 ] && stop $pid "stray run, $((a / 60)) min";;
  esac
done
for pid in $(pgrep -u "$me" -f "until .*do sleep"); do
  # only loops started from Claude's shells or task files, never anyone else's
  grep -q "claude" /proc/$pid/cmdline 2>/dev/null || continue
  a=$(age $pid); [ -z "$a" ] && continue
  [ "$a" -gt 9000 ] && stop $pid "polling loop, $((a / 60)) min"
done
# scratch folders no one has written to for 12 hours (session scratchpads only;
# concept sources, the gauntlet record and the integration tree are kept)
for scratch in /tmp/claude-$me/-home-james-dyno/*/scratchpad; do
  [ -d "$scratch" ] || continue
  for d in "$scratch"/*/; do
    d=${d%/}; n=$(basename "$d")
    case "$n" in concepts|gauntlet|int) continue;; esac
    newest=$(find "$d" -printf '%T@\n' 2>/dev/null | sort -n | tail -1 | cut -d. -f1)
    [ -z "$newest" ] && continue
    if [ $(( $(date +%s) - newest )) -gt 43200 ]; then
      if [ $dry = 1 ]; then echo "would remove $d ($(du -sh "$d" | cut -f1))"
      else echo "removed $d ($(du -sh "$d" | cut -f1))"; rm -rf "$d"; fi
    fi
  done
done
exit 0
