"""Put the gauntlet's landmarks (tools/concepts/landmarks.json) where the
layout has their places. Run it when places move or the world's size
changes, then run `dyno scout` again to choose the cameras:

    python3 tools/world/landmarks.py
    dyno scout ADDRESS

Each landmark is a place of content/world/layout.json, the middle of what
is looked at and a spot to stand, both as offsets from the place in metres
(things at a place do not move when the world grows; places do).
"""
import json, os

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..")

# name: place, middle (x, z), anchor (x, z) [, the anchor's place]
WHERE = {
    "hearthstead_plaza": ("hearthstead", (0, 0), (0, 8)),
    "hearthstead_street": ("hearthstead", (0, -40), (0, -22)),
    "hearthstead_dusk": ("hearthstead", (4, -8), (0, 8)),
    "watch_tower": ("hearthstead", (-24, -26), (0, -22)),
    "north_watch": ("north_watch", (13, -5), (0, 10)),
    "logging_camp": ("logging_camp", (2, -20), (1, 6)),
    "hollow_oak": ("hollow_oak", (0, 0), (26, 8)),
    "old_barrow": ("old_barrow", (0, 0), (21, 0)),
    "barrow_crypt": ("old_barrow", (12, 0), (17, 0)),
    "wrecked_caravan": ("wrecked_caravan", (0, -8), (0, 0)),
    "emberford_bridge": ("emberford_bridge", (0, 0), (-22, 0)),
    "windmill_farm": ("windmill_farm", (-10, -3), (-4, 10)),
    "fishers_landing": ("fishers_landing", (0, 0), (2, 3)),
    "mirrormere": ("fishers_landing", (40, 0), (4, -6)),
    "sunken_shrine": ("sunken_shrine", (0, 0), (0, 0), "shrine_shore"),
    "ashcloak_stockade": ("ashcloak_stockade", (0, 15), (0, 28)),
    "miners_rest": ("miners_rest", (0, -5), (-3, 8)),
    "hollow_mines": ("hollow_mines", (3, -1), (-13, 7)),
}
# the boxes the landmarks fill now: half-sizes and height
BOX = {
    "hearthstead_plaza": ((9, 9), 6), "hearthstead_street": ((4, 20), 9), "hearthstead_dusk": ((22, 22), 28),
    "watch_tower": ((5, 5), 26), "north_watch": ((4, 4), 22), "logging_camp": ((14, 9), 13),
    "hollow_oak": ((10, 10), 29), "old_barrow": ((17, 15), 12), "barrow_crypt": ((5, 4), 5),
    "wrecked_caravan": ((9, 4), 5), "emberford_bridge": ((16, 5), 6), "windmill_farm": ((9, 6), 26),
    "fishers_landing": ((12, 12), 9), "mirrormere": ((30, 26), 3), "sunken_shrine": ((8, 8), 8),
    "ashcloak_stockade": ((7, 5), 11), "miners_rest": ((14, 10), 14), "hollow_mines": ((13, 13), 24),
}


# tall or far-off landmarks need more room to stand back
FAR = {"watch_tower": 60, "hollow_oak": 56, "ashcloak_stockade": 26, "hollow_mines": 44, "sunken_shrine": 42, "north_watch": 44, "windmill_farm": 44}


def rnd(v):
    """Round half away from zero, as the engine does."""
    return int(v + 0.5) if v >= 0 else -int(-v + 0.5)


def main():
    layout = json.load(open(os.path.join(ROOT, "content", "world", "layout.json")))
    spread = layout["size"][0] / layout.get("design", [2048])[0]
    at = {p["key"]: (rnd(p["at"][0] * spread), rnd(p["at"][1] * spread)) for p in layout["places"]}
    path = os.path.join(ROOT, "tools", "concepts", "landmarks.json")
    data = json.load(open(path))
    for lm in data["landmarks"]:
        place, mid, anchor, *rest = WHERE[lm["name"]]
        px, pz = at[place]
        ax, az = at[rest[0]] if rest else (px, pz)
        lm["at"] = [px + mid[0], pz + mid[1]]
        lm["anchor"] = [ax + anchor[0], az + anchor[1]]
        half, height = BOX[lm["name"]]
        lm["half"], lm["height"] = list(half), height
        if place != "hearthstead":
            lm.pop("base", None)
        if lm["name"] in FAR:
            lm["far"] = FAR[lm["name"]]
    lines = ["{", f' "note": {json.dumps(data["note"])},', f' "paths": {json.dumps(data["paths"])},',
             f' "defaults": {json.dumps(data["defaults"])},', ' "landmarks": [']
    lines += ["  " + json.dumps(lm) + ("," if i + 1 < len(data["landmarks"]) else "")
              for i, lm in enumerate(data["landmarks"])]
    lines += [" ]", "}"]
    open(path, "w").write("\n".join(lines) + "\n")
    print(f"{len(data['landmarks'])} landmarks at spread {spread:g}")


if __name__ == "__main__":
    main()
