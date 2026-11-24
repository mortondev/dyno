"""The feel test pack: the game's pack with a test yard added.

    python3 tools/feel/make_pack.py OUT_DIR

Writes a whole pack to OUT_DIR (run a test server on it with
`dyno server --content OUT_DIR --op NAME`, then `/feel` takes an operator
to the yard). The live pack in content/ is not touched and holds none of
this: the yard is a stamp, a place of the layout, an area and three
creatures of the rules, all added here.

The yard (docs/design/feel.md, F1): a flat field of bare earth, stairs up to a 4 m deck,
a 2 m ledge beside it, a corridor 2 m wide (half of it roofed), and three
dummies on the field:
  - Training Dummy: never dies, strikes back lightly, and winds up a heavy
    blow now and then;
  - Keen Dummy: never dies, and every blow on it is a critical one;
  - Straw Dummy: falls to any blow and is back in a few seconds.
"""
import json, os, shutil, sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..")
sys.path.insert(0, os.path.join(ROOT, "tools", "stamps"))
from kit import Vol

# where the yard lies (metres from the hearth): open meadow south-east of
# Hearthstead, clear of every place and camp of the live layout
AT = [140, 150]
# where things stand in the yard (x east, z south of its middle)
ARRIVE = [0, -4]
DUMMIES = {"training_dummy": [-8, 12], "keen_dummy": [0, 12], "straw_dummy": [8, 12]}


def yard():
    v = Vol("prop")
    # the deck (4 m) with stairs up its west side, and a 2 m ledge south of it
    v.fill((10, 0, -8), (18, 3, 0), "stone_bricks")
    for i in range(4):
        v.fill((6 + i, 0, -8), (6 + i, i, -5), "stone_bricks")
    v.fill((10, 0, 1), (18, 1, 4), "cobble")
    # the corridor: two walls 2 m apart, the west half roofed
    v.fill((-20, -1, -9), (-8, -1, -4), "cobble")
    v.fill((-20, 0, -9), (-8, 3, -9), "stone_bricks")
    v.fill((-20, 0, -6), (-8, 3, -6), "stone_bricks")
    v.fill((-20, 4, -9), (-14, 4, -6), "planks")
    # a paved square under each dummy, and where you arrive
    for x, z in list(DUMMIES.values()) + [ARRIVE]:
        v.fill((x - 1, -1, z - 1), (x + 1, -1, z + 1), "cobble")
    v.points = {"arrive": (ARRIVE[0], 0, ARRIVE[1])}
    for k, (x, z) in DUMMIES.items():
        v.points[k] = (x, 0, z)
    return v


def dummy(key, name, color, **more):
    c = {
        "key": key, "name": name, "model": "humanoid", "health": 4000, "speed": 0,
        "hostile": False, "damage": 1, "drops": [], "color": color, "spawn": "meadow",
        "cap": 0, "burns_in_daylight": False, "levels": [1, 1], "attack_speed": 2.0,
        "anchored": True, "immortal": True, "material": "wood",
    }
    c.update(more)
    return c


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else sys.exit(__doc__)
    if os.path.exists(out):
        shutil.rmtree(out)
    shutil.copytree(os.path.join(ROOT, "content"), out)
    yard().write(os.path.join(out, "stamps", "feel_yard.dstamp"))

    path = os.path.join(out, "world", "layout.json")
    layout = json.load(open(path))
    layout["places"].append({
        "key": "feel_yard", "kind": "site", "at": AT, "name": "The Feel Yard", "radius": 34,
        "pad": {"radius": [34, 34], "blend": 8, "surface": {"dirt": 1}}, "sight": False,
        "stamps": [{"stamp": "feel_yard", "at": [0, 0]}],
    })
    json.dump(layout, open(path, "w"), indent=1)

    path = os.path.join(out, "rules", "hearthblock.json")
    rules = json.load(open(path))
    rules["creatures"] += [
        dummy("training_dummy", "Training Dummy", [0.62, 0.47, 0.28],
              heavy={"every": 3, "windup": 0.8, "damage": 2.5, "reach": 2.6}),
        dummy("keen_dummy", "Keen Dummy", [0.75, 0.62, 0.3], damage=0, crit_taken=100),
        dummy("straw_dummy", "Straw Dummy", [0.85, 0.76, 0.42], damage=0, health=1, immortal=False),
    ]
    rules["areas"].append({
        "key": "feel_yard", "name": "The Feel Yard", "place": "feel_yard", "radius": "place",
        "levels": [1, 1], "safe": True,
        "camps": [
            {"creature": k, "count": 1, "place": "feel_yard", "at": at, "spread": 0, "respawn": 3}
            for k, at in DUMMIES.items()
        ],
    })
    rules["world"]["test_place"] = {"place": "feel_yard", "at": ARRIVE}
    json.dump(rules, open(path, "w"), indent=1)
    print(f"feel pack written to {out}")


if __name__ == "__main__":
    main()
