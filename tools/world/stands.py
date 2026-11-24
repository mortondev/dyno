"""Where each gauntlet camera stands and looks, as offsets from its place
of the layout: for putting things in (or out of) a camera's view.

    python3 tools/world/stands.py [name ...]
"""
import json, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from landmarks import WHERE

ROOT = os.path.join(HERE, "..", "..")

if __name__ == "__main__":
    cams = {c["name"]: c for c in json.load(open(os.path.join(ROOT, "tools", "concepts", "cameras.json")))["cameras"]}
    layout = json.load(open(os.path.join(ROOT, "content", "world", "layout.json")))
    spread = layout["size"][0] / layout.get("design", [2048])[0]
    at = {p["key"]: (p["at"][0] * spread, p["at"][1] * spread) for p in layout["places"]}
    for name in sys.argv[1:] or sorted(cams):
        c, p = cams[name], at[WHERE[name][0]]
        rel = lambda v: [round(v[0] - p[0], 1), round(v[2] - p[1], 1)]
        print(f"{name}: at {WHERE[name][0]}, stands {rel(c['stand'])}, looks at {rel(c['face'])}, pitch {c['pitch']}")
