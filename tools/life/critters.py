"""The ambient critters' voxel models (content/models/critters/*.vox).

Each is a MagicaVoxel file with a few named frames: `idle`, `move1`,
`move2` and `alt` (feeding, head down). A model faces +z, stands on y = 0
and is centred on x = 0, z = 0 (see crates/dyno-client/src/critters.rs).
Edit the .vox files in MagicaVoxel, or change the shapes here and run

    python3 tools/life/critters.py

from the repository root. What the critters do and where they live is in
content/life.json.
"""
import os, struct

OUT = os.path.join(os.path.dirname(__file__), '..', '..', 'content', 'models', 'critters')


class Model:
    def __init__(self):
        self.v = {}

    def box(self, x, y, z, c):
        """x, y, z: a number or an inclusive (lo, hi) range"""
        r = lambda a: range(a, a + 1) if isinstance(a, int) else range(a[0], a[1] + 1)
        for i in r(x):
            for j in r(y):
                for k in r(z):
                    self.v[(i, j, k)] = c
        return self

    def mirror(self, x, y, z, c):
        """a box and its twin on the other side (x and -x - 1)"""
        self.box(x, y, z, c)
        if isinstance(x, int):
            x = (x, x)
        return self.box((-x[1] - 1, -x[0] - 1), y, z, c)


def chunk(cid, body):
    return cid + struct.pack('<ii', len(body), 0) + body


def string(s):
    b = s.encode()
    return struct.pack('<i', len(b)) + b


def dict_(d):
    return struct.pack('<i', len(d)) + b''.join(string(k) + string(v) for k, v in d.items())


def write(name, palette, frames):
    """palette: list of (r, g, b), index 1 first; frames: {name: Model}"""
    body = b''
    places = []
    for m in frames.values():
        # game (x, y, z) is stored at (x, -z - 1, y)
        ps = [((x, -z - 1, y), c) for (x, y, z), c in m.v.items()]
        lo = [min(p[0][a] for p in ps) for a in range(3)]
        hi = [max(p[0][a] for p in ps) for a in range(3)]
        size = [hi[a] - lo[a] + 1 for a in range(3)]
        body += chunk(b'SIZE', struct.pack('<iii', *size))
        xyzi = struct.pack('<i', len(ps))
        for p, c in ps:
            xyzi += struct.pack('<BBBB', p[0] - lo[0], p[1] - lo[1], p[2] - lo[2], c)
        body += chunk(b'XYZI', xyzi)
        places.append([lo[a] + size[a] // 2 for a in range(3)])
    body += chunk(b'nTRN', struct.pack('<i', 0) + dict_({}) + struct.pack('<iiii', 1, -1, -1, 1) + dict_({}))
    n = len(frames)
    body += chunk(b'nGRP', struct.pack('<i', 1) + dict_({}) + struct.pack('<i', n)
                  + b''.join(struct.pack('<i', 2 + 2 * i) for i in range(n)))
    for i, (fname, at) in enumerate(zip(frames, places)):
        nid = 2 + 2 * i
        body += chunk(b'nTRN', struct.pack('<i', nid) + dict_({'_name': fname})
                      + struct.pack('<iiii', nid + 1, -1, 0, 1) + dict_({'_t': '%d %d %d' % tuple(at)}))
        body += chunk(b'nSHP', struct.pack('<i', nid + 1) + dict_({}) + struct.pack('<ii', 1, i) + dict_({}))
    pal = b''
    for i in range(255):
        r, g, b = palette[i] if i < len(palette) else (0, 0, 0)
        pal += struct.pack('<BBBB', r, g, b, 255)
    pal += b'\0\0\0\0'
    body += chunk(b'RGBA', pal)
    data = b'VOX ' + struct.pack('<i', 200) + b'MAIN' + struct.pack('<ii', 0, len(body)) + body
    os.makedirs(OUT, exist_ok=True)
    with open(os.path.join(OUT, name + '.vox'), 'wb') as f:
        f.write(data)
    print(name, {k: len(m.v) for k, m in frames.items()})


# ---------------------------------------------------------------- shapes

def bird(body, wing, beak):
    pal = [body, wing, beak]

    def frame(tip):
        """tip: how high the wing tips are against the shoulders"""
        m = Model()
        m.mirror(0, (0, 1), (-3, 2), 1)          # body
        m.mirror(0, 1, (3, 4), 1)                # head
        m.mirror(0, 1, 5, 3)                     # beak
        m.mirror((0, 1), 1, (-5, -4), 2)         # tail
        for i in range(1, 7):                    # wing, root to tip
            y = 1 + round(tip * i / 6)
            m.mirror(i, y, (-1 if i < 5 else 0, 1), 2)
        return m
    return pal, {'idle': frame(0), 'move1': frame(3), 'move2': frame(-3)}


def butterfly(wing, spot):
    pal = [(40, 34, 30), wing, spot]

    def frame(up):
        m = Model()
        m.box((-1, 0), 0, (-2, 1), 1)            # body
        m.box((-1, 0), 0, 2, 1)
        if up == 2:                              # wings closed over the back
            m.mirror(1, (1, 4), (-2, 1), 2)
            m.mirror(1, (2, 3), (-1, 0), 3)
        elif up == 1:                            # half open
            for i in range(1, 4):
                m.mirror(i, i, (-2, 1), 2)
            m.mirror(2, 2, (-1, 0), 3)
        else:                                    # flat
            m.mirror((1, 4), 0, (-2, 1), 2)
            m.mirror((2, 3), 0, (-1, 0), 3)
        return m
    return pal, {'idle': frame(1), 'move1': frame(0), 'move2': frame(2)}


def duck(body, head, wing):
    pal = [body, head, (232, 170, 40), wing, (30, 26, 24)]

    def frame(wag):
        m = Model()
        m.mirror((0, 1), (0, 2), (-3, 2), 1)     # body
        m.mirror((0, 1), 3, (-2, 1), 4)          # back and wings
        m.mirror(2, (1, 2), (-2, 1), 4)
        m.box((-1 + wag, 0 + wag), (2, 3), (-5, -4), 4)   # tail
        m.mirror(0, (3, 5), (2, 3), 2)           # neck and head
        m.mirror(0, 6, (2, 3), 2)
        m.mirror(0, 5, (4, 5), 3)                # bill
        m.mirror(0, 6, 3, 5)                     # eyes (the head's sides)
        return m
    return pal, {'idle': frame(0), 'move1': frame(-1), 'move2': frame(1)}


def heron():
    pal = [(158, 172, 190), (236, 238, 240), (226, 176, 52), (58, 62, 78), (120, 112, 96)]

    def frame(down):
        m = Model()
        m.box(-2, (0, 7), 0, 5)                  # legs
        m.box(1, (0, 7), -1, 5)
        m.mirror((0, 1), (8, 11), (-4, 2), 1)    # body
        m.mirror((0, 1), (8, 9), (-6, -5), 4)    # tail feathers
        m.mirror(1, (10, 11), (-3, 1), 4)        # folded wings
        if down:                                 # neck stretched to the water
            m.mirror(0, (9, 11), (3, 4), 2)
            m.mirror(0, (6, 9), (5, 6), 2)
            m.mirror(0, (4, 6), (6, 7), 2)
            m.mirror(0, 6, 5, 4)
            m.mirror(0, (2, 4), 7, 3)
        else:                                    # neck in an S, head high
            m.mirror(0, (11, 13), (2, 3), 2)
            m.mirror(0, (13, 16), (3, 4), 2)
            m.mirror(0, (16, 18), (4, 5), 2)
            m.mirror(0, 18, (2, 3), 4)           # crest
            m.mirror(0, 17, (6, 9), 3)           # bill
        return m
    return pal, {'idle': frame(False), 'alt': frame(True)}


def rabbit(fur, belly):
    pal = [fur, belly, (245, 242, 235), (28, 22, 20), (224, 150, 150)]

    def frame(pose):
        m = Model()
        if pose == 'up':                         # sitting up, ears pricked
            m.mirror((0, 1), (0, 4), (-2, 0), 1)
            m.mirror(0, (1, 4), 1, 2)
            m.mirror((0, 1), (5, 7), (-1, 1), 1)
            m.mirror(1, (8, 10), 0, 1)
            m.mirror(1, 6, 1, 4)
            m.mirror(0, 5, 2, 5)
            m.mirror(0, (1, 2), -3, 3)
            return m
        stretch = 1 if pose == 'leap' else 0
        m.mirror((0, 1), (1, 3), (-3 - stretch, 1), 1)    # body
        m.mirror(0, 1, (-2, 0), 2)
        m.mirror((0, 1), (3, 5), (1, 3), 1)      # head
        m.mirror(1, (6, 8), 1 - stretch, 1)      # ears (laid back in a leap)
        m.mirror(1, 4, 3, 4)                     # eyes
        m.mirror(0, 3, 4, 5)                     # nose
        m.mirror(0, (2, 3), -4 - stretch, 3)     # scut
        if pose == 'leap':
            m.mirror(1, 0, (-6, -5), 1)          # hind legs out behind
            m.mirror(1, 1, (2, 3), 1)            # forelegs reaching
        else:
            m.mirror(1, 0, (-3, -1), 1)          # haunches
            m.mirror(1, 0, 1, 1)
        return m
    return pal, {'idle': frame('sit'), 'move1': frame('leap'), 'move2': frame('sit'), 'alt': frame('up')}


def deer(antlers):
    pal = [(178, 124, 74), (222, 190, 150), (64, 44, 34), (240, 236, 226), (26, 20, 18), (206, 190, 160)]

    def frame(step, down):
        m = Model()
        m.mirror((0, 1), (9, 13), (-6, 5), 1)    # body
        m.mirror((0, 1), 9, (-5, 4), 2)          # belly
        m.mirror(0, (11, 13), -7, 4)             # tail
        # legs: (z of the leg, which diagonal pair it belongs to)
        for x, z, pair in [(1, 4, 0), (-2, 4, 1), (1, -6, 1), (-2, -6, 0)]:
            swing = step if pair == 0 else -step
            m.box(x, (5, 8), (z, z + 1), 1)
            m.box(x, (1, 4), z + swing, 1)
            m.box(x, 0, z + swing, 3)
        if down:                                 # grazing
            m.mirror((0, 0), (9, 12), (6, 7), 1)
            m.mirror((0, 0), (5, 9), (7, 8), 1)
            m.mirror((0, 0), (2, 5), (8, 10), 1)
            m.mirror(0, 2, 11, 5)
            m.mirror(1, 6, 8, 1)
            hy, hz = 6, 8
        else:
            m.mirror((0, 0), (12, 17), (5, 6), 1)   # neck
            m.mirror((0, 0), (11, 14), 6, 2)
            m.mirror((0, 1), (17, 19), (5, 8), 1)   # head
            m.mirror(0, (17, 18), (9, 10), 1)       # muzzle
            m.mirror(0, 18, 11, 5)                  # nose
            m.mirror(1, 19, 8, 5)                   # eyes
            m.mirror(2, (19, 20), 5, 1)             # ears
            hy, hz = 20, 6
        if antlers:
            m.mirror(1, (hy, hy + 3), hz, 6)
            m.mirror(2, hy + 2, (hz, hz + 1), 6)
            m.mirror(2, hy + 4, hz, 6)
            m.mirror(3, (hy + 3, hy + 5), hz - 1, 6)
        return m
    return pal, {'idle': frame(0, False), 'move1': frame(1, False), 'move2': frame(-1, False),
                 'alt': frame(0, True)}


def sheep():
    pal = [(236, 230, 218), (72, 62, 56), (210, 202, 188), (24, 20, 18)]

    def frame(step, down):
        m = Model()
        m.mirror((0, 2), (5, 10), (-5, 4), 1)    # wool
        m.mirror((0, 1), 11, (-4, 3), 1)
        m.mirror((0, 2), (5, 6), (-5, 4), 3)
        for x, z, pair in [(1, 3, 0), (-2, 3, 1), (1, -5, 1), (-2, -5, 0)]:
            swing = step if pair == 0 else -step
            m.box(x, (0, 4), z + swing, 2)
        if down:
            m.mirror((0, 1), (2, 5), (5, 7), 2)
            m.mirror(1, 5, 5, 2)
        else:
            m.mirror((0, 1), (8, 11), (5, 7), 2)
            m.mirror(2, 10, 5, 2)                # ears
            m.mirror(1, 10, 7, 4)
            m.mirror((0, 1), (11, 12), (4, 5), 1)   # woolly brow
        return m
    return pal, {'idle': frame(0, False), 'move1': frame(1, False), 'move2': frame(-1, False),
                 'alt': frame(0, True)}


def crab():
    pal = [(216, 98, 58), (240, 150, 96), (24, 20, 18), (170, 66, 44)]

    def frame(step):
        m = Model()
        m.mirror((0, 2), (1, 2), (-2, 1), 1)     # shell
        m.mirror((0, 1), 3, (-1, 0), 2)
        m.mirror(1, (3, 4), 2, 4)                # eye stalks
        m.mirror(1, 5, 2, 3)
        m.mirror((3, 4), (1, 3), (2, 3), 1)      # claws
        m.mirror(4, (3, 4), (3, 4), 2)
        for i, z in enumerate([-2, -1, 0]):      # legs
            lift = 1 if (i + step) % 2 == 0 else 0
            m.mirror(3, 1, z, 4)
            m.mirror(4, lift, z, 4)
        return m
    return pal, {'idle': frame(0), 'move1': frame(0), 'move2': frame(1)}


if __name__ == '__main__':
    write('bird', *bird((52, 50, 60), (38, 36, 46), (230, 160, 40)))
    write('gull', *bird((240, 240, 236), (196, 200, 208), (236, 180, 48)))
    write('butterfly_orange', *butterfly((240, 140, 30), (60, 40, 30)))
    write('butterfly_blue', *butterfly((70, 150, 240), (220, 240, 255)))
    write('butterfly_white', *butterfly((248, 244, 220), (250, 210, 80)))
    write('duck', *duck((150, 112, 78), (40, 112, 76), (112, 84, 60)))
    write('heron', *heron())
    write('rabbit', *rabbit((156, 130, 104), (214, 196, 172)))
    write('deer', *deer(False))
    write('stag', *deer(True))
    write('sheep', *sheep())
    write('crab', *crab())
