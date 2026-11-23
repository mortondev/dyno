"""Gameplay-camera shots of the places of a world built from the layout.

    dyno map --out /tmp/w/map.ppm           # also writes map.places.json
    dyno server --port 7670 --world /tmp/w/world.dynw &
    python3 tools/world/shots.py OUT_DIR /tmp/w/map.places.json [ADDRESS] [name ...]

Each camera is a place of the layout, a direction to stand in, a distance
and a height above the place's ground (docs/design/scale.md: about 9 m
behind where a character would stand, looking 10 to 20 degrees down), so
the same list works whatever the size of the world.
"""
import json, math, os, subprocess, sys

# name: (place, offset of the thing looked at (x, z), direction to the
#        camera (dx, dz), distance m, camera height m, time of day 0-1)
SPOTS = {
    "hearthstead_plaza": ("hearthstead", (0, 0), (10, 16), 30, 7, 0.30),
    "hearthstead_street": ("hearthstead", (0, -40), (2, 22), 26, 5, 0.28),
    "hearthstead_dusk": ("hearthstead", (10, 6), (-26, 24), 44, 10, 0.47),
    "watch_tower": ("hearthstead", (-24, -26), (14, 22), 34, 6, 0.32),
    "hearthstead_inn": ("hearthstead", (30, 22), (-18, 6), 26, 5, 0.30),
    "north_watch": ("north_watch", (0, -5), (10, 22), 32, 6, 0.30),
    "north_road": ("north_watch", (0, 60), (2, 30), 30, 5, 0.30),
    "logging_camp": ("logging_camp", (0, 0), (14, 20), 34, 7, 0.27),
    "hollow_oak": ("hollow_oak", (0, 0), (8, 26), 38, 7, 0.36),
    "old_barrow": ("old_barrow", (8, 0), (24, 10), 32, 6, 0.40),
    "wrecked_caravan": ("wrecked_caravan", (0, -6), (10, 16), 24, 5, 0.30),
    "emberford_bridge": ("emberford_bridge", (0, 0), (-22, 16), 36, 7, 0.29),
    "windmill_farm": ("windmill_farm", (0, 0), (-12, 24), 40, 7, 0.26),
    "fishers_landing": ("fishers_landing", (2, 0), (-18, 16), 34, 7, 0.45),
    "mirrormere": ("fishers_landing", (30, 0), (-30, -8), 40, 9, 0.25),
    "sunken_shrine": ("shrine_shore", (0, -16), (4, 22), 26, 5, 0.33),
    "ashcloak_stockade": ("ashcloak_stockade", (0, 6), (10, 26), 40, 8, 0.60),
    "miners_rest": ("miners_rest", (0, -2), (-12, 22), 34, 7, 0.31),
    "hollow_mines": ("hollow_mines", (8, -2), (-20, 12), 28, 6, 0.34),
    "meadow_farm": ("meadow_farm_1", (0, 0), (14, 20), 36, 7, 0.30),
    "east_road": ("wrecked_caravan", (-40, 6), (-30, 4), 30, 5, 0.31),
}


def camera(name, places):
    place, (ox, oz), (dx, dz), dist, up, time = SPOTS[name]
    px, py, pz = places[place]
    tx, tz = px + ox, pz + oz
    n = math.hypot(dx, dz)
    cx, cz = tx + dx / n * dist, tz + dz / n * dist
    cy = py + up
    lx, ly, lz = tx - cx, py + 3 - cy, tz - cz
    yaw = math.degrees(math.atan2(-lx, -lz))
    pitch = math.degrees(math.atan2(ly, math.hypot(lx, lz)))
    return (cx, cy, cz), (yaw, pitch), time


if __name__ == "__main__":
    out_dir, listing = sys.argv[1], json.load(open(sys.argv[2]))
    addr = sys.argv[3] if len(sys.argv) > 3 else "127.0.0.1:7670"
    names = sys.argv[4:] or list(SPOTS)
    os.makedirs(out_dir, exist_ok=True)
    exe = os.environ.get("DYNO", "./target/release/dyno")
    for name in names:
        if SPOTS[name][0] not in listing["places"]:
            print(name, "is outside this world")
            continue
        (cx, cy, cz), (yaw, pitch), time = camera(name, listing["places"])
        out = f"{out_dir}/{name}.ppm"
        r = subprocess.run([exe, "shot", addr, "--pos", f"{cx:.1f},{cy:.1f},{cz:.1f}",
                            "--look", f"{yaw:.1f},{pitch:.1f}", "--time", str(time),
                            "--size", "1280x720", "--out", out],
                           capture_output=True, text=True, timeout=300)
        subprocess.run(["magick", out, "-quality", "90", out[:-4] + ".jpg"])
        if os.path.exists(out):
            os.remove(out)
        print(name, (r.stdout or r.stderr).strip().splitlines()[-1][:70])
