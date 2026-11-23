"""Make the ground cover models of the pack: content/cover/*.vox.

Small voxel plants and stones (grass tufts, flowers, ferns, reeds, pebbles,
rocks), a few variants of each in one MagicaVoxel file per kind. They are
ordinary .vox files: open and edit them in MagicaVoxel, or change the
recipes here and run this again. Where they grow is in content/cover.json.

usage: python3 tools/cover/make.py [OUT_DIR]   (default content/cover)
"""
import os, random, struct, sys

GREENS = [(38, 92, 30), (52, 116, 34), (72, 142, 40), (98, 168, 48), (130, 190, 58), (162, 208, 72)]
STEM = (48, 108, 36)


def chunk(cid, body):
    return cid + struct.pack('<ii', len(body), 0) + body


def write_vox(path, models):
    """models: a list of {(x, y, z): (r, g, b)} in game axes (y up)."""
    colours = []
    for m in models:
        for c in m.values():
            if c not in colours:
                colours.append(c)
    assert len(colours) <= 255, (path, len(colours))
    body = b''
    for m in models:
        xs, ys, zs = zip(*m.keys())
        x0, y0, z0 = min(xs), min(ys), min(zs)
        sx, sy, sz = max(xs) - x0 + 1, max(ys) - y0 + 1, max(zs) - z0 + 1
        # the game's (x, y, z) is stored at (x, -z - 1, y)
        body += chunk(b'SIZE', struct.pack('<iii', sx, sz, sy))
        cells = b''.join(struct.pack('<BBBB', x - x0, sz - 1 - (z - z0), y - y0, colours.index(c) + 1)
                         for (x, y, z), c in sorted(m.items()))
        body += chunk(b'XYZI', struct.pack('<i', len(m)) + cells)
    pal = b''.join(struct.pack('<BBBB', *c, 255) for c in colours)
    pal += b'\0\0\0\xff' * (256 - len(colours))
    body += chunk(b'RGBA', pal)
    with open(path, 'wb') as f:
        f.write(b'VOX ' + struct.pack('<i', 150) + b'MAIN' + struct.pack('<ii', 0, len(body)) + body)


def blade(m, rnd, x, z, tall, lean=None, tip=None):
    """One blade of grass: a column that may lean a voxel near the top."""
    dx, dz = lean or (0, 0)
    for y in range(tall):
        k = y / max(tall - 1, 1)
        shade = min(len(GREENS) - 1, int(k * (len(GREENS) - 1) + rnd.random() * 0.9))
        bend = 1 if y >= tall * 0.6 else 0
        m[(x + dx * bend, y, z + dz * bend)] = GREENS[shade]
    if tip:
        m[(x + dx, tall, z + dz)] = tip


def grass(seed, blades, lo, hi, spread, seeds=False):
    rnd = random.Random(seed)
    m = {}
    for _ in range(blades):
        x, z = rnd.randint(-spread, spread), rnd.randint(-spread, spread)
        lean = rnd.choice([None, None, (1, 0), (-1, 0), (0, 1), (0, -1)])
        tip = (206, 196, 110) if seeds and rnd.random() < 0.5 else None
        blade(m, rnd, x, z, rnd.randint(lo, hi), lean, tip)
    return m


def stem(m, x, z, tall):
    for y in range(tall):
        m[(x, y, z)] = STEM if y % 2 else GREENS[2]


def lupins(seed, spikes):
    rnd = random.Random(seed)
    purple = [(96, 60, 168), (128, 82, 200), (158, 112, 226), (196, 164, 240)]
    m = {}
    for _ in range(spikes):
        x, z = rnd.randint(-4, 4), rnd.randint(-4, 4)
        foot, tall = rnd.randint(2, 4), rnd.randint(5, 8)
        stem(m, x, z, foot)
        for d in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            if rnd.random() < 0.6:
                m[(x + d[0], 0, z + d[1])] = GREENS[rnd.randint(1, 3)]
        for i in range(tall):
            k = i / (tall - 1)
            c = purple[min(3, int(k * 3 + rnd.random() * 0.8))]
            m[(x, foot + i, z)] = c
            # the spike is fuller low down
            if k < 0.6:
                for d in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    if rnd.random() < 0.55 - k * 0.5:
                        m[(x + d[0], foot + i, z + d[1])] = purple[rnd.randint(0, 2)]
    blade(m, rnd, rnd.randint(-3, 3), rnd.randint(-3, 3), rnd.randint(3, 5))
    return m


def heads(seed, count, petal, heart, lo=2, hi=5, spread=4, cup=False):
    """A clump of small flowers: a stem with a flat four-petalled head."""
    rnd = random.Random(seed)
    m = {}
    for _ in range(count):
        x, z = rnd.randint(-spread, spread), rnd.randint(-spread, spread)
        tall = rnd.randint(lo, hi)
        stem(m, x, z, tall)
        p = petal[rnd.randrange(len(petal))]
        for d in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            m[(x + d[0], tall + (1 if cup else 0), z + d[1])] = p
        m[(x, tall, z)] = heart
    for _ in range(2):
        blade(m, rnd, rnd.randint(-spread, spread), rnd.randint(-spread, spread), rnd.randint(2, 4))
    return m


def fern(seed, fronds):
    rnd = random.Random(seed)
    dark, mid, light = (30, 84, 34), (44, 112, 40), (70, 140, 48)
    m = {}
    dirs = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1)]
    rnd.shuffle(dirs)
    for dx, dz in dirs[:fronds]:
        length = rnd.randint(5, 8)
        px, pz = -dz, dx
        for i in range(length):
            k = i / (length - 1)
            # rises, then droops at the tip
            y = round(1 + 3.2 * (1 - (2 * k - 0.9) ** 2))
            x, z = dx * (i + 1), dz * (i + 1)
            m[(x, max(y, 0), z)] = mid if i % 2 else dark
            wide = 2 if 0.15 < k < 0.7 else 1
            for s in range(1, wide + 1):
                if dx and dz and s > 1:
                    continue
                c = light if s == wide else mid
                m[(x + px * s, max(y - (s - 1), 0), z + pz * s)] = c
                m[(x - px * s, max(y - (s - 1), 0), z - pz * s)] = c
    m[(0, 0, 0)] = dark
    m[(0, 1, 0)] = dark
    return m


def reeds(seed, stalks):
    rnd = random.Random(seed)
    green = [(74, 110, 44), (96, 134, 52), (122, 156, 62), (150, 176, 78)]
    brown = [(104, 66, 36), (126, 82, 44)]
    m = {}
    for _ in range(stalks):
        x, z = rnd.randint(-3, 3), rnd.randint(-3, 3)
        tall = rnd.randint(9, 17)
        lean = rnd.choice([(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)])
        for y in range(tall):
            bend = 1 if y > tall * 0.7 else 0
            m[(x + lean[0] * bend, y, z + lean[1] * bend)] = green[min(3, int(y / tall * 4))]
        if rnd.random() < 0.45:
            for y in range(tall, tall + 3):
                m[(x + lean[0], y, z + lean[1])] = brown[y % 2]
            m[(x + lean[0], tall + 3, z + lean[1])] = green[3]
    return m


def pebbles(seed, count, spread, greys, big=2):
    rnd = random.Random(seed)
    m = {}
    for _ in range(count):
        x, z = rnd.randint(-spread, spread), rnd.randint(-spread, spread)
        w, d = rnd.randint(1, big), rnd.randint(1, big)
        c = greys[rnd.randrange(len(greys))]
        for i in range(w):
            for j in range(d):
                m[(x + i, 0, z + j)] = c
        if w > 1 and d > 1 and rnd.random() < 0.5:
            m[(x, 1, z)] = tuple(min(255, v + 18) for v in c)
    return m


def slabs(seed, count, spread, tones):
    """Flat worn stones set in a path."""
    rnd = random.Random(seed)
    m = {}
    for _ in range(count):
        x, z = rnd.randint(-spread, spread), rnd.randint(-spread, spread)
        w, d = rnd.randint(2, 4), rnd.randint(2, 4)
        c = tones[rnd.randrange(len(tones))]
        for i in range(w):
            for j in range(d):
                if (i in (0, w - 1) and j in (0, d - 1)) and w > 2 and d > 2 and rnd.random() < 0.6:
                    continue
                v = rnd.randint(-8, 8)
                m[(x + i, 0, z + j)] = tuple(max(0, min(255, a + v)) for a in c)
    return m


def rock(seed, cells, moss=0.3):
    """A lump of stone built from quarter-metre cubes (two voxels a side),
    mossy on top. `cells` is its rough size in those cubes: (wide, high, deep)."""
    rnd = random.Random(seed)
    greys = [(104, 106, 112), (124, 126, 130), (144, 146, 148), (164, 164, 162)]
    mossy = [(86, 122, 54), (104, 140, 60)]
    w, h, d = cells
    solid = set()
    for x in range(w):
        for z in range(d):
            # an uneven dome
            fx, fz = (x + 0.5) / w * 2 - 1, (z + 0.5) / d * 2 - 1
            top = h * (1 - 0.55 * (fx * fx + fz * fz)) + rnd.uniform(-0.6, 0.4)
            for y in range(max(1, round(top))):
                if y < h:
                    solid.add((x, y, z))
    m = {}
    for (x, y, z) in solid:
        base = greys[min(3, max(0, int(1.2 + y / max(h - 1, 1) * 1.6 + rnd.uniform(-1, 1))))]
        if (x, y + 1, z) not in solid and rnd.random() < moss:
            base = mossy[rnd.randrange(2)]
        for i in range(2):
            for j in range(2):
                for k in range(2):
                    v = rnd.randint(-5, 5)
                    m[(x * 2 + i, y * 2 + j, z * 2 + k)] = tuple(max(0, min(255, a + v)) for a in base)
    return m


def litter(seed, count):
    rnd = random.Random(seed)
    tones = [(214, 132, 38), (196, 84, 34), (226, 176, 52), (170, 96, 40)]
    m = {}
    for _ in range(count):
        x, z = rnd.randint(-6, 6), rnd.randint(-6, 6)
        c = tones[rnd.randrange(len(tones))]
        m[(x, 0, z)] = c
        if rnd.random() < 0.5:
            dx, dz = rnd.choice([(1, 0), (0, 1)])
            m[(x + dx, 0, z + dz)] = c
    return m


def mushrooms(seed, count, cap, spots):
    rnd = random.Random(seed)
    stalk = (226, 216, 196)
    m = {}
    for _ in range(count):
        x, z = rnd.randint(-4, 4), rnd.randint(-4, 4)
        tall = rnd.randint(1, 3)
        for y in range(tall):
            m[(x, y, z)] = stalk
        for i in (-1, 0, 1):
            for j in (-1, 0, 1):
                m[(x + i, tall, z + j)] = spots if (i + j) % 2 and rnd.random() < 0.3 else cap
        m[(x, tall + 1, z)] = cap
    return m


KINDS = {
    'grass_short': [grass(s, 5, 2, 4, 3) for s in range(11, 15)],
    'grass': [grass(s, 6, 3, 7, 3) for s in range(21, 27)],
    'grass_tall': [grass(s, 6, 6, 11, 2, seeds=True) for s in range(31, 35)],
    'lupin': [lupins(s, n) for s, n in ((41, 3), (42, 4), (43, 5), (44, 2))],
    'daisy': [heads(s, n, [(244, 244, 236), (232, 236, 244)], (240, 196, 40)) for s, n in ((51, 3), (52, 5), (53, 4))],
    'buttercup': [heads(s, n, [(246, 208, 48), (250, 224, 80)], (232, 150, 30), 2, 4) for s, n in ((61, 3), (62, 5), (63, 4))],
    'poppy': [heads(s, n, [(214, 44, 38), (232, 70, 48)], (50, 30, 34), 3, 6, cup=True) for s, n in ((71, 3), (72, 4), (73, 2))],
    'pink': [heads(s, n, [(240, 150, 190), (248, 186, 214)], (250, 232, 150), 2, 5) for s, n in ((81, 3), (82, 5), (83, 4))],
    'fern': [fern(s, n) for s, n in ((91, 5), (92, 6), (93, 4))],
    'reeds': [reeds(s, n) for s, n in ((101, 5), (102, 7), (103, 4))],
    'pebbles': [pebbles(s, n, 5, [(112, 114, 120), (138, 138, 140), (160, 158, 154), (92, 94, 100)]) for s, n in ((111, 4), (112, 6), (113, 3), (114, 5))],
    'cobbles': [slabs(s, n, 5, [(128, 120, 112), (146, 138, 128), (110, 106, 104)]) for s, n in ((121, 2), (122, 3), (123, 2))],
    'path_stones': [slabs(s, n, 6, [(132, 128, 124), (152, 146, 138), (116, 112, 110)]) for s, n in ((131, 2), (132, 3), (133, 1))],
    'rock_small': [rock(s, c) for s, c in ((141, (2, 1, 2)), (142, (3, 2, 2)), (143, (2, 2, 3)), (144, (3, 1, 2)))],
    'rock': [rock(s, c, 0.4) for s, c in ((151, (4, 2, 3)), (152, (3, 3, 4)), (153, (5, 3, 4)))],
    'boulder': [rock(s, c, 0.45) for s, c in ((161, (6, 4, 5)), (162, (5, 5, 6)), (163, (7, 4, 6)))],
    'stepping_stone': [rock(s, c, 0.25) for s, c in ((171, (4, 1, 4)), (172, (5, 1, 4)), (173, (4, 1, 3)))],
    'leaf_litter': [litter(s, n) for s, n in ((181, 5), (182, 7), (183, 4))],
    'mushroom_red': [mushrooms(s, n, (196, 48, 40), (240, 236, 226)) for s, n in ((191, 2), (192, 3))],
    'mushroom_brown': [mushrooms(s, n, (150, 104, 66), (176, 132, 90)) for s, n in ((201, 3), (202, 2))],
}

if __name__ == '__main__':
    out = sys.argv[1] if len(sys.argv) > 1 else 'content/cover'
    os.makedirs(out, exist_ok=True)
    for name, models in KINDS.items():
        write_vox(f'{out}/{name}.vox', models)
        print(name, len(models), 'models,', sum(len(m) for m in models), 'voxels')
