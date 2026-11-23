"""Render the reference cameras of every location from a running server.

    python3 tools/concepts/shots.py OUT_DIR [ADDRESS] [DYNO_BINARY] [NAME ...]

ADDRESS defaults to 127.0.0.1:7499 and DYNO_BINARY to ./target/debug/dyno.
Writes OUT_DIR/<name>.jpg (and the .ppm). The world is a fresh one from the
default seed: the hearth stands at H.
"""
import math, subprocess, sys, os

out_dir = sys.argv[1]
addr = sys.argv[2] if len(sys.argv) > 2 else '127.0.0.1:7499'
dyno = sys.argv[3] if len(sys.argv) > 3 else './target/debug/dyno'
only = set(sys.argv[4:])
H = (192, 37, 192)
OLD = (194, 37, 195)
def at(p): return (p[0] - OLD[0] + H[0], p[1], p[2] - OLD[2] + H[2])
def rel(dx, y, dz): return (OLD[0] + dx, y, OLD[2] + dz)
# name: (target, camera offset (dx, dy, dz), time of day)
spots = {
    'hearthstead_plaza': ((194, 38, 195), (-4, 20, 30), 0.30),
    'hearthstead_street': ((194, 38, 189), (10, 6, 16), 0.32),
    'hearthstead_dusk': ((194, 38, 195), (24, 26, 34), 0.73),
    'watch_tower': ((200, 40, 180), (-28, 16, 10), 0.32),
    'north_watch': ((194, 30, 144), (24, 22, 26), 0.30),
    'logging_camp': ((149, 34, 109), (26, 24, 24), 0.27),
    'hollow_oak': ((107, 27, 142), (28, 24, 22), 0.36),
    'old_barrow': (rel(-116, 28, 10), (20, 18, -22), 0.40),
    'barrow_crypt': ((70, 30, 205), (22, 20, 16), 0.62),
    'windmill_farm': (rel(14, 27, 96), (-22, 14, -18), 0.30),
    'mirrormere': (rel(0, 27, 118), (-14, 16, -26), 0.30),
    'fishers_landing': (rel(-28, 27, 136), (18, 18, -20), 0.32),
    'sunken_shrine': (rel(8, 26, 159), (-22, 14, -18), 0.33),
    'wrecked_caravan': ((244, 31, 186), (20, 18, 22), 0.30),
    'ashcloak_stockade': (rel(62, 28, 66), (-22, 18, -16), 0.80),
    'emberford_bridge': (rel(95, 28, 0), (-18, 12, 20), 0.31),
    'miners_rest': ((322, 33, 205), (-24, 20, 22), 0.31),
    'hollow_mines': ((351, 30, 176), (-26, 26, 26), 0.34),
}
os.makedirs(out_dir, exist_ok=True)
for name, (t, off, time) in spots.items():
    if only and name not in only:
        continue
    tx, ty, tz = at(t)
    cx, cy, cz = tx + off[0], ty + off[1], tz + off[2]
    dx, dy, dz = tx - cx, ty - cy, tz - cz
    yaw = math.degrees(math.atan2(-dx, -dz))
    pitch = math.degrees(math.atan2(dy, math.hypot(dx, dz)))
    out = f'{out_dir}/{name}.ppm'
    r = subprocess.run([dyno, 'shot', addr, '--pos', f'{cx},{cy},{cz}', '--look', f'{yaw:.1f},{pitch:.1f}',
                        '--time', str(time), '--size', '1280x720', '--out', out],
                       capture_output=True, text=True, timeout=600)
    subprocess.run(['magick', out, '-quality', '90', out.replace('.ppm', '.jpg')])
    lines = (r.stderr + r.stdout).strip().splitlines()
    stats = [l for l in lines if l.startswith('stats:')]
    print(name, stats[-1] if stats else (lines[-1][:80] if lines else ''), flush=True)
