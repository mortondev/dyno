#!/usr/bin/env python3
"""Write the release manifest for the binaries in a folder.

usage: manifest.py <dist-dir> <version>
Binaries are named dyno-<target-triple>[.exe].
"""
import hashlib, json, os, re, sys

dist, version = sys.argv[1], sys.argv[2]
src = open("crates/dyno-protocol/src/lib.rs").read()
protocol = int(re.search(r"PROTOCOL_VERSION: u32 = (\d+);", src).group(1))
assets = {}
for name in sorted(os.listdir(dist)):
    m = re.fullmatch(r"dyno-(.+?)(\.exe)?", name)
    if not m or name.startswith("dyno-update"):
        continue
    data = open(os.path.join(dist, name), "rb").read()
    assets[m.group(1)] = {"name": name, "size": len(data), "sha256": hashlib.sha256(data).hexdigest()}
if len(assets) < 4:
    sys.exit(f"expected 4 platform binaries, found {sorted(assets)}")
json.dump({"version": version, "protocol": protocol, "assets": assets}, sys.stdout, indent=1)
