"""Canonical cameras for the concept references and the visual gauntlet.

Gameplay framing (docs/design/scale.md): about 5 m above the ground, about
30 m from the landmark, looking slightly down at it, as a WoW-style third
person camera sees a place while you walk up to it.

usage: python3 tools/concepts/shots.py OUT_DIR [ADDRESS] [name ...]
"""
import math, subprocess, sys, os

# world placement: the test world's hearth is at H; the spots were measured
# in a world whose hearth was at OLD
H = (192, 37, 192); OLD = (194, 37, 195)
def at(p): return (p[0] - OLD[0] + H[0], p[1], p[2] - OLD[2] + H[2])

# name: (landmark (ground), direction to the camera (dx, dz), distance m,
#        camera height above the landmark ground m, time of day 0-1)
SPOTS = {
 'hearthstead_plaza': ((194,38,195), (14,16), 28, 5, 0.30),
 'hearthstead_street': ((194,38,180), (6,22), 26, 4, 0.28),
 'hearthstead_dusk': ((194,40,195), (-26,24), 34, 7, 0.47),
 'watch_tower': ((200,40,180), (-14,20), 32, 4, 0.32),
 'north_watch': ((194,30,144), (16,18), 30, 5, 0.30),
 'logging_camp': ((149,34,109), (14,16), 30, 5, 0.27),
 'hollow_oak': ((107,27,142), (16,14), 30, 5, 0.36),
 'old_barrow': ((78,30,205), (16,12), 28, 5, 0.40),
 'barrow_crypt': ((61,26,205), (8,6), 22, 4, 0.62),
 'wrecked_caravan': ((244,31,186), (12,12), 24, 4, 0.30),
 'emberford_bridge': ((289,31,195), (14,18), 30, 5, 0.29),
 'windmill_farm': ((204,27,290), (18,16), 32, 5, 0.26),
 'fishers_landing': ((166,26,331), (16,14), 30, 5, 0.45),
 'mirrormere': ((185,26,300), (-22,24), 34, 6, 0.25),
 'sunken_shrine': ((202,28,354), (14,12), 26, 5, 0.33),
 'ashcloak_stockade': ((256,28,267), (18,18), 30, 5, 0.60),
 'miners_rest': ((322,33,205), (16,16), 30, 5, 0.31),
 'hollow_mines': ((351,30,176), (14,16), 30, 5, 0.34),
}

def camera(name):
    t, (dx, dz), dist, up, time = SPOTS[name]
    tx, ty, tz = at(t)
    n = math.hypot(dx, dz)
    cx, cz = tx + dx / n * dist, tz + dz / n * dist
    cy = ty + up
    # look at a point 3 m above the landmark's ground
    lx, ly, lz = tx - cx, ty + 3 - cy, tz - cz
    yaw = math.degrees(math.atan2(-lx, -lz))
    pitch = math.degrees(math.atan2(ly, math.hypot(lx, lz)))
    return (cx, cy, cz), (yaw, pitch), time

if __name__ == '__main__':
    out_dir = sys.argv[1]
    addr = sys.argv[2] if len(sys.argv) > 2 else '127.0.0.1:7499'
    names = sys.argv[3:] or list(SPOTS)
    os.makedirs(out_dir, exist_ok=True)
    exe = os.environ.get('DYNO', './target/debug/dyno')
    for name in names:
        (cx, cy, cz), (yaw, pitch), time = camera(name)
        out = f'{out_dir}/{name}.ppm'
        r = subprocess.run([exe, 'shot', addr, '--pos', f'{cx:.1f},{cy:.1f},{cz:.1f}',
                            '--look', f'{yaw:.1f},{pitch:.1f}', '--time', str(time),
                            '--size', '1280x720', '--out', out],
                           capture_output=True, text=True, timeout=300)
        subprocess.run(['magick', out, '-quality', '90', out[:-4] + '.jpg'])
        os.remove(out) if os.path.exists(out) else None
        print(name, (r.stdout or r.stderr).strip().splitlines()[-1][:70])
