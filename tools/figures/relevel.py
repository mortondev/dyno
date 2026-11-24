"""Held items on the cube ladder (docs/art/voxel-rule.md).

A held item's voxels are `held.kinds.<kind>.unit` blocks across, and the
items were tuned at sizes off the ladder (0.014 to 0.039 blocks). This
re-voxels each held model at 1/32 of a block (3.125 cm, the ladder's finest
step) at the same size: every model in its .vox file is resampled about
its origin by `unit * 32` (nearest voxel), and the unit is set to 1/32, so
the item's size, grip and first-person place are unchanged.

usage: python3 tools/figures/relevel.py [PACK_DIR]   (default content/)
Idempotent: a kind already at 1/32 is left alone.
"""
import json, math, os, sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'shapes'))
import voxio

PACK = sys.argv[1] if len(sys.argv) > 1 else 'content'
STEP = 1 / 32


def resample(cells, r):
    """Voxels at `r` times the count along each axis (r < 1: coarser)."""
    if not cells:
        return cells
    lo = [math.floor(min(p[a] for p in cells) * r) - 1 for a in range(3)]
    hi = [math.ceil((max(p[a] for p in cells) + 1) * r) + 1 for a in range(3)]
    out = {}
    for x in range(lo[0], hi[0]):
        for y in range(lo[1], hi[1]):
            for z in range(lo[2], hi[2]):
                src = tuple(math.floor((v + 0.5) / r) for v in (x, y, z))
                c = cells.get(src)
                if c is not None:
                    out[(x, y, z)] = c
    return out


def main():
    path = os.path.join(PACK, 'figures.json')
    figs = json.load(open(path))
    done = []
    for kind, k in figs['held']['kinds'].items():
        unit = k['unit']
        r = unit / STEP
        if abs(r - 1) < 1e-6:
            continue
        fig = figs['figures'][k['figure']]
        vox = os.path.join(PACK, fig['file'])
        models, palette = voxio.read(vox)
        voxio.write(vox, [(n, resample(c, r)) for n, c in models], palette)
        for g in fig['grids']:
            g['min'] = [math.floor(v * r) - 1 for v in g['min']]
            g['max'] = [math.ceil(v * r) + 1 for v in g['max']]
        # every kind drawn with this figure takes the new unit
        for k2 in figs['held']['kinds'].values():
            if k2['figure'] == k['figure']:
                k2['unit'] = STEP
        done.append(f'{kind} x{r:.3f}')
    text = json.dumps(figs, indent=1)
    open(path, 'w').write(text + '\n')
    print('re-voxelled at 1/32:', ', '.join(done) or 'nothing')


if __name__ == '__main__':
    main()
