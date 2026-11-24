"""Round parts as stepped circles (docs/art/voxel-rule.md).

Logs, posts, pillars and barrels were modelled as rough voxel cylinders that
the old mesher turned into smooth solids of revolution ("round" parts). The
voxel rule draws every voxel as a cube, so the rough outline itself would
show, a different wobble on every layer (and a rectangle for each). This
redraws each slice across the axis as a clean stepped circle with the
slice's own area and centre, so a log is a stack of a few circles that the
mesher merges into tall faces, and a flare steps out ring by ring.

usage: python3 tools/shapes/rounds.py FILE.vox AXIS PREFIX [PREFIX ...]

AXIS is 0 (x), 1 (y), 2 (z) or auto (each model's longest); a model is redrawn when its name starts with
one of the prefixes. The radius is taken to half a voxel and the centre to
half a voxel, so the circles are symmetric; each new voxel takes the colour
of the nearest old one in its slice. Run again, it changes nothing.
"""
import math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import voxio


def circle(cells, axis):
    slices = {}
    for p, c in cells.items():
        slices.setdefault(p[axis], []).append((p, c))
    ua, va = [a for a in range(3) if a != axis]
    out = {}
    for s, vs in slices.items():
        n = len(vs)
        cu = sum(p[ua] + 0.5 for p, _ in vs) / n
        cv = sum(p[va] + 0.5 for p, _ in vs) / n
        cu, cv = round(cu * 2) / 2, round(cv * 2) / 2
        r = round(math.sqrt(n / math.pi) * 2) / 2
        if r < 0.5:
            for p, c in vs:
                out[p] = c
            continue
        lo_u, hi_u = math.floor(cu - r) - 1, math.ceil(cu + r) + 1
        lo_v, hi_v = math.floor(cv - r) - 1, math.ceil(cv + r) + 1
        for u in range(lo_u, hi_u + 1):
            for v in range(lo_v, hi_v + 1):
                du, dv = u + 0.5 - cu, v + 0.5 - cv
                if du * du + dv * dv > r * r:
                    continue
                p = [0, 0, 0]
                p[axis], p[ua], p[va] = s, u, v
                near = min(vs, key=lambda q: (q[0][ua] - u) ** 2 + (q[0][va] - v) ** 2)
                out[tuple(p)] = near[1]
    return out


def main(path, axis, prefixes):
    models, palette = voxio.read(path)
    new, n = [], 0
    for name, cells in models:
        if any(name.startswith(p) for p in prefixes):
            a = axis
            if a is None:
                ext = [max(p[k] for p in cells) - min(p[k] for p in cells) for k in range(3)]
                a = max(range(3), key=lambda k: (ext[k], k == 1))
            c2 = circle(cells, a)
            n += c2 != cells
            cells = c2
        new.append((name, cells))
    voxio.write(path, new, palette)
    print(f'{path}: {n} models redrawn as stepped circles')


if __name__ == '__main__':
    if len(sys.argv) < 4:
        sys.exit(__doc__)
    main(sys.argv[1], None if sys.argv[2] == 'auto' else int(sys.argv[2]), sys.argv[3:])
