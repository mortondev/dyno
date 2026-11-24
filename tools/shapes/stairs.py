"""Roofs and awnings as stairs of cubes (docs/art/voxel-rule.md).

The detail shapes of roofs and cloth were modelled as 45-degree runs of
single voxels (16 a block), which the old mesher drew as sloped polygons.
The voxel rule has no slopes: a roof is a stair. This turns every such run
in a shape file into a stair with treads and risers of STEP voxels (4:
25 cm, the ladder's step for building detail), keeping everything else of
the model (courses of shingles, fascia boards, barge boards, gable walls).

usage: python3 tools/shapes/stairs.py content/shapes/roof_red.vox [more.vox ...]

How: in each model, a column of voxels (x, z) whose top rises or falls by
one voxel a column along x or z, over three columns or more, is on a slope. The whole column is moved up
so that its top is level over each run of STEP columns; the first column of
each tread is carried down to the tread below, which closes the riser. A
column that stands on a level floor (a gable wall under the slope) keeps
its foot. The move depends only on where the column is in the cell, so the
parts of a roof (tiles, boards under them, barge boards, gables) and the
cells of a long roof still meet. Run again, it changes nothing.
"""
import os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import voxio

STEP = 4


def columns(cells):
    cols = {}
    for (x, y, z), c in cells.items():
        cols.setdefault((x, z), {})[y] = c
    return cols


def restep(cells):
    cols = columns(cells)
    top = {k: max(v) for k, v in cols.items()}
    bottom = {k: min(v) for k, v in cols.items()}

    def slope(x, z):
        """(axis, sign) of the 45-degree slope the column lies on, if any:
        three columns in a row, this one among them, each a voxel higher
        (or lower) than the one before."""
        for axis, (dx, dz) in enumerate([(1, 0), (0, 1)]):
            t = [top.get((x + k * dx, z + k * dz)) for k in range(-2, 3)]
            for first in range(3):
                run = t[first:first + 3]
                if None in run:
                    continue
                d = (run[1] - run[0], run[2] - run[1])
                if d == (1, 1):
                    return axis, 1
                if d == (-1, -1):
                    return axis, -1
        return None

    def level_foot(x, z, axis):
        """True when the column's foot does not follow the slope."""
        dx, dz = [(1, 0), (0, 1)][axis]
        b = bottom[(x, z)]
        near = [bottom.get((x - dx, z - dz)), bottom.get((x + dx, z + dz))]
        near = [n for n in near if n is not None]
        return bool(near) and all(n == b for n in near)

    out = {}
    changed = False
    for (x, z), col in cols.items():
        s = slope(x, z)
        if s is None:
            for y, c in col.items():
                out[(x, y, z)] = c
            continue
        axis, sign = s
        along = (x, z)[axis] % STEP
        # rising: the tread's last column keeps its height; falling: its first
        up = STEP - 1 - along if sign > 0 else along
        # the column at the riser reaches down to the tread below
        riser = along == 0 if sign > 0 else along == STEP - 1
        keep_foot = level_foot(x, z, axis)
        lo = min(col)
        for y, c in col.items():
            out[(x, y + up, z)] = c
        if keep_foot:
            for y in range(lo, lo + up):
                out[(x, y, z)] = col[lo]
        elif riser:
            for y in range(lo + up - STEP, lo + up):
                out[(x, y, z)] = col[lo]
        changed = changed or up != 0 or riser
    return out, changed


def main(paths):
    for path in paths:
        models, palette = voxio.read(path)
        n = 0
        new = []
        for name, cells in models:
            cells2, changed = restep(cells)
            n += changed
            new.append((name, cells2))
        voxio.write(path, new, palette)
        print(f'{path}: {n} of {len(models)} models stepped')


if __name__ == '__main__':
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    main(sys.argv[1:])
