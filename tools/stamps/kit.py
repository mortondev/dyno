"""Authoring kit for stamps (content/stamps/*.dstamp): voxel structures as
data. The game never runs this; it loads the stamp files. Edit a recipe in
build.py (or draw a stamp with any other tool that writes the format) and
run `python3 tools/stamps/build.py`.

Format: see crates/dyno-world/src/stamp.rs. Coordinates here: x east,
z south, y up; y = 0 is the first layer above the ground the stamp stands
on (its floor slab is y = -1), and (0, 0, 0) is the anchor unless a recipe
says otherwise.

Proportions (docs/design/scale.md): doors 3 m clear, storeys 4 m floor to
floor, roofs as tall as the walls with eaves a metre out.
"""
import json, os

STOREY = 4   # floor to floor
DOOR = 3     # clear height of a doorway


class Vol:
    def __init__(self, kind="building"):
        self.c = {}
        self.points = {}
        self.doors = []
        self.storeys = []
        self.kind = kind
        self.foundation = None

    def set(self, x, y, z, b):
        self.c[(x, y, z)] = b

    def get(self, x, y, z):
        return self.c.get((x, y, z))

    def fill(self, a, b, block):
        for y in range(min(a[1], b[1]), max(a[1], b[1]) + 1):
            for z in range(min(a[2], b[2]), max(a[2], b[2]) + 1):
                for x in range(min(a[0], b[0]), max(a[0], b[0]) + 1):
                    self.c[(x, y, z)] = block

    def col(self, x, z, y0, y1, block):
        for y in range(y0, y1 + 1):
            self.c[(x, y, z)] = block

    def clear(self, a, b):
        """Cells the stamp carves to air."""
        self.fill(a, b, "air")

    def drop(self, a, b):
        """Forget cells (the stamp leaves the world alone there)."""
        for y in range(min(a[1], b[1]), max(a[1], b[1]) + 1):
            for z in range(min(a[2], b[2]), max(a[2], b[2]) + 1):
                for x in range(min(a[0], b[0]), max(a[0], b[0]) + 1):
                    self.c.pop((x, y, z), None)

    def merge(self, other, at=(0, 0, 0), rot=0):
        for (x, y, z), b in other.c.items():
            x, z = turn(x, z, rot)
            self.c[(x + at[0], y + at[1], z + at[2])] = b
        for k, (x, y, z) in other.points.items():
            x, z = turn(x, z, rot)
            self.points.setdefault(k, (x + at[0], y + at[1], z + at[2]))
        for d in other.doors:
            x, z = turn(d["at"][0], d["at"][2], rot)
            self.doors.append({"at": [x + at[0], d["at"][1] + at[1], z + at[2]], "height": d["height"]})

    def bounds(self):
        pts = list(self.c) + list(self.points.values()) + [tuple(d["at"]) for d in self.doors] + [(0, 0, 0)]
        xs = [p[0] for p in pts]
        ys = [p[1] for p in pts]
        zs = [p[2] for p in pts]
        return (min(xs), min(ys), min(zs)), (max(xs), max(ys), max(zs))

    def write(self, path):
        lo, hi = self.bounds()
        size = [hi[i] - lo[i] + 1 for i in range(3)]
        palette = ["skip"]
        index = {}
        cells = bytearray(size[0] * size[1] * size[2])
        for (x, y, z), b in self.c.items():
            if b not in index:
                palette.append(b)
                index[b] = len(palette) - 1
            cells[((y - lo[1]) * size[2] + (z - lo[2])) * size[0] + (x - lo[0])] = index[b]
        shift = lambda p: [p[0] - lo[0], p[1] - lo[1], p[2] - lo[2]]
        head = {
            "size": size,
            "anchor": shift((0, 0, 0)),
            "palette": palette,
            "kind": self.kind,
        }
        if self.points:
            head["points"] = {k: shift(p) for k, p in sorted(self.points.items())}
        if self.doors:
            head["doors"] = [{"at": shift(d["at"]), "height": d["height"]} for d in self.doors]
        if self.storeys:
            head["storeys"] = [y - lo[1] for y in self.storeys]
        if self.foundation:
            head["foundation"] = self.foundation
        out = bytearray(b"DSTP1\n")
        out += json.dumps(head, separators=(",", ":")).encode() + b"\n"
        i = 0
        n = len(cells)
        while i < n:
            v = cells[i]
            j = i
            while j < n and cells[j] == v:
                j += 1
            run = j - i
            out.append(v)
            while True:
                byte = run & 0x7F
                run >>= 7
                if run:
                    out.append(byte | 0x80)
                else:
                    out.append(byte)
                    break
            i = j
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "wb") as f:
            f.write(out)
        return size


def turn(x, z, rot):
    for _ in range(rot % 4):
        x, z = -z, x
    return x, z


def rnd(x, y, z, salt=0):
    h = (x * 0x27D4EB2D ^ y * 0x165667B1 ^ z * 0x9E3779B9 ^ salt * 0x85EBCA6B) & 0xFFFFFFFF
    h ^= h >> 15
    h = (h * 0x2C1B3C6D) & 0xFFFFFFFF
    h ^= h >> 12
    h = (h * 0x297A2D39) & 0xFFFFFFFF
    return h ^ (h >> 15)


def pick(x, y, z, of, salt=0):
    return of[rnd(x, y, z, salt) % len(of)]


# ---------------------------------------------------------------- styles

STYLES = {
    # base course, wall, frame, roof, floor
    "tile": dict(base="stone_bricks", wall="plaster", frame="timber_beam", roof="red_roof_tiles", floor="planks"),
    "thatch": dict(base="cobble", wall="plaster", frame="timber_beam", roof="thatch", floor="planks"),
    "slate": dict(base="stone_bricks", wall="stone_bricks", frame="timber_beam", roof="slate_roof", floor="flagstones"),
    "plank": dict(base="cobble", wall="dark_planks", frame="timber_beam", roof="thatch", floor="planks"),
    "log": dict(base="cobble", wall="planks", frame="log", roof="thatch", floor="planks"),
}

MASONRY = ["stone_bricks", "stone_bricks", "stone_bricks", "cobble", "mossy_cobble"]


def roof(v, x0, z0, x1, z1, y, style, ridge_x, eave=1):
    """A pitched roof over the box, eaves a metre out, with filled gables.
    `y` is the wall's top course; the eaves hang beside it."""
    s = STYLES[style] if isinstance(style, str) else style
    lo, hi, la, lb = (z0, z1, x0, x1) if ridge_x else (x0, x1, z0, z1)
    k = 0
    while True:
        a, b = lo - eave + k, hi + eave - k
        if a > b:
            break
        yy = y + k
        for l in range(la - eave, lb + eave + 1):
            for c in ([a, b] if a != b else [a]):
                p = (l, yy, c) if ridge_x else (c, yy, l)
                v.set(*p, s["roof"])
        if k > 0:
            for c in range(a + 1, b):
                block = s["frame"] if c - lo == hi - c else s["wall"]
                for l in (la, lb):
                    p = (l, yy, c) if ridge_x else (c, yy, l)
                    v.set(*p, block)
        k += 1
    return y + k - 1   # the ridge's height


def house(x0, z0, x1, z1, storeys=1, style="tile", ridge_x=None, door=("s", None), windows=True,
          chimney=None, attic=False, kind="building", name="door"):
    """A timber-framed house. The floor slab is y = -1; each storey is
    STOREY high. `door` is (side, position along the wall or None for the
    middle); the doorway is two wide, one leaf open."""
    s = STYLES[style]
    v = Vol(kind)
    v.foundation = s["base"]
    h = storeys * STOREY          # walls are y = 0 .. h-1
    if ridge_x is None:
        ridge_x = (x1 - x0) >= (z1 - z0)
    v.fill((x0, -1, z0), (x1, -1, z1), s["floor"])
    v.clear((x0 + 1, 0, z0 + 1), (x1 - 1, h - 1, z1 - 1))
    sides = {"n": (x0, x1, lambda i: (i, z0), (0, -1)), "s": (x0, x1, lambda i: (i, z1), (0, 1)),
             "w": (z0, z1, lambda i: (x0, i), (-1, 0)), "e": (z0, z1, lambda i: (x1, i), (1, 0))}
    for side, (lo, hi, cell, out) in sides.items():
        for i in range(lo, hi + 1):
            x, z = cell(i)
            d = min(i - lo, hi - i)
            for y in range(0, h):
                storey_y = y % STOREY
                glass = windows and d % 4 == 2 and storey_y in (1, 2)
                if d == 0 or y == h - 1 or (storey_y == STOREY - 1):
                    b = s["frame"]
                elif y == 0:
                    b = s["base"]
                elif glass:
                    b = "leaded_window"
                elif d % 4 == 0:
                    b = s["frame"]
                else:
                    b = s["wall"]
                v.set(x, y, z, b)
            if windows and d % 4 == 2 and d > 0:
                v.set(x + out[0], 0, z + out[1], "flower_bed")
    # upper floors
    for k in range(1, storeys):
        v.fill((x0 + 1, k * STOREY - 1, z0 + 1), (x1 - 1, k * STOREY - 1, z1 - 1), s["floor"])
    v.storeys = [k * STOREY for k in range(storeys)]
    if attic:
        v.fill((x0 + 1, h - 1, z0 + 1), (x1 - 1, h - 1, z1 - 1), s["floor"])
    if door:
        side, pos = door
        lo, hi, cell, out = sides[side]
        if pos is None:
            pos = (lo + hi) // 2
        for j, i in enumerate((pos, pos + 1)):
            x, z = cell(i)
            v.col(x, z, 0, DOOR - 1, "air" if j == 0 else "wooden_door")
            v.set(x, DOOR, z, s["frame"])
            v.set(x + out[0], 0, z + out[1], "air")
            v.set(x + out[0], -1, z + out[1], "flagstones")
        for i in (pos - 1, pos + 2):
            x, z = cell(i)
            v.col(x, z, 0, DOOR, s["frame"])
        x, z = cell(pos)
        v.doors.append({"at": [x, 0, z], "height": DOOR})
        v.points[name] = (x + out[0] * 2, 0, z + out[1] * 2)
        # something left by the door
        x, z = cell(pos - 2)
        v.set(x + out[0], 0, z + out[1], pick(x, 0, z, ["crate", "barrel"]))
    top = roof(v, x0, z0, x1, z1, h - 1, s, ridge_x)
    if chimney:
        cx, cz = chimney
        v.col(cx, cz, -1, top + 2, "stone_bricks")
        v.set(cx, top + 3, cz, "cobble")
    # a cell that is open floor on every storey (tests measure storeys here)
    v.points["floor"] = ((x0 + x1) // 2, 0, (z0 + z1) // 2)
    return v


def stairs(v, x, y, z, dx, dz, rise=STOREY, block="planks"):
    """Steps of rising height from floor y up `rise`, and the hole above."""
    for i in range(rise):
        sx, sz = x + dx * i, z + dz * i
        v.col(sx, sz, y, y + i, block)
        v.col(sx, sz, y + i + 1, y + rise + 2, "air")
    # the landing's hole: one more cell of head room
    v.col(x + dx * rise, z + dz * rise, y + rise, y + rise + 2, "air")


def table(v, x, y, z):
    v.set(x, y, z, "planks")


def lamp_post(v, x, y, z, h=4):
    v.col(x, z, y, y + h - 2, "timber_beam")
    v.set(x, y + h - 1, z, "lantern")


def fence(v, x0, z0, x1, z1, y=0, gate=None, block="timber_beam"):
    for x in range(x0, x1 + 1):
        for z in (z0, z1):
            v.set(x, y, z, block)
    for z in range(z0, z1 + 1):
        for x in (x0, x1):
            v.set(x, y, z, block)
    for (x, z) in ((x0, z0), (x1, z0), (x0, z1), (x1, z1)):
        v.set(x, y + 1, z, block)
    if gate:
        for p in gate:
            v.set(p[0], y, p[1], "air")
