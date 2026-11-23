#!/usr/bin/env python3
"""Write the release manifest for the binaries in a folder.

usage: manifest.py <dist-dir> <version>
Binaries are named <program>-<target-triple>[.exe], where program is
`dyno` (servers, tools, dev client) or `hearthblock` (the player client).

`assets` lists the dyno binaries only: every updater released before the
split reads just that key (the live server updates itself as dyno).
`programs` lists every program; updaters from 0.7 pick the one named like
their own executable.
"""
import hashlib, json, os, re, sys

PROGRAMS = ("dyno", "hearthblock")
TARGETS = 4

dist, version = sys.argv[1], sys.argv[2]
src = open("crates/dyno-protocol/src/lib.rs").read()
protocol = int(re.search(r"PROTOCOL_VERSION: u32 = (\d+);", src).group(1))
programs = {p: {} for p in PROGRAMS}
for name in sorted(os.listdir(dist)):
    m = re.fullmatch(r"(dyno|hearthblock)-(.+?)(\.exe)?", name)
    if not m or name.startswith("dyno-update"):
        continue
    data = open(os.path.join(dist, name), "rb").read()
    programs[m.group(1)][m.group(2)] = {
        "name": name,
        "size": len(data),
        "sha256": hashlib.sha256(data).hexdigest(),
    }
for p in PROGRAMS:
    if len(programs[p]) < TARGETS:
        sys.exit(f"expected {TARGETS} {p} binaries, found {sorted(programs[p])}")
json.dump(
    {"version": version, "protocol": protocol, "assets": programs["dyno"], "programs": programs},
    sys.stdout,
    indent=1,
)
