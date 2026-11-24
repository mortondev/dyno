#!/usr/bin/env python3
"""Draw the ability icons of the pack: content/icons/abilities/<key>.png,
32 x 32, one glyph each on a tile in the ability's own colour, so every
ability of a class reads apart at 40 px. Standard library only.

    python3 tools/abilities/icons.py            # every ability in the rules
    python3 tools/abilities/icons.py --sheet out.png   # a contact sheet too

The glyph for an ability is chosen by its key in GLYPHS below; an ability
without an entry gets a plain gem in its colour (and a warning).
"""
import json, math, os, struct, sys, zlib

ROOT = os.path.join(os.path.dirname(__file__), '..', '..')
RULES = os.path.join(ROOT, 'content', 'rules', 'hearthblock.json')
OUT = os.path.join(ROOT, 'content', 'icons', 'abilities')
N = 32   # icon size
SS = 4   # supersampling


class Canvas:
    """An RGBA float canvas, N*SS square; shapes are in icon units (0..32)."""

    def __init__(self):
        self.n = N * SS
        self.px = [[0.0, 0.0, 0.0, 0.0] for _ in range(self.n * self.n)]

    def _blend(self, i, c, a):
        if a <= 0:
            return
        p = self.px[i]
        na = a + p[3] * (1 - a)
        for k in range(3):
            p[k] = (c[k] * a + p[k] * p[3] * (1 - a)) / na
        p[3] = na

    def shape(self, inside, c, a=1.0, box=None):
        """Fill where inside(x, y) is true (x, y in icon units)."""
        x0, y0, x1, y1 = box or (0, 0, N, N)
        for py in range(max(0, int(y0 * SS)), min(self.n, int(y1 * SS) + 1)):
            y = (py + 0.5) / SS
            for px in range(max(0, int(x0 * SS)), min(self.n, int(x1 * SS) + 1)):
                x = (px + 0.5) / SS
                v = inside(x, y)
                if v:
                    self._blend(py * self.n + px, c, a * (v if isinstance(v, float) else 1.0))

    def circle(self, cx, cy, r, c, a=1.0):
        self.shape(lambda x, y: (x - cx) ** 2 + (y - cy) ** 2 <= r * r, c, a, (cx - r, cy - r, cx + r, cy + r))

    def ring(self, cx, cy, r, w, c, a=1.0, a0=0.0, a1=360.0):
        def f(x, y):
            d = math.hypot(x - cx, y - cy)
            if abs(d - r) > w / 2:
                return False
            ang = math.degrees(math.atan2(y - cy, x - cx)) % 360
            return a0 <= ang <= a1 if a0 <= a1 else (ang >= a0 or ang <= a1)
        self.shape(f, c, a, (cx - r - w, cy - r - w, cx + r + w, cy + r + w))

    def line(self, x0, y0, x1, y1, w, c, a=1.0):
        dx, dy = x1 - x0, y1 - y0
        l2 = dx * dx + dy * dy or 1e-9

        def f(x, y):
            t = max(0.0, min(1.0, ((x - x0) * dx + (y - y0) * dy) / l2))
            return math.hypot(x - x0 - t * dx, y - y0 - t * dy) <= w / 2
        self.shape(f, c, a, (min(x0, x1) - w, min(y0, y1) - w, max(x0, x1) + w, max(y0, y1) + w))

    def poly(self, pts, c, a=1.0):
        def f(x, y):
            ins = False
            j = len(pts) - 1
            for i in range(len(pts)):
                xi, yi = pts[i]
                xj, yj = pts[j]
                if (yi > y) != (yj > y) and x < (xj - xi) * (y - yi) / (yj - yi) + xi:
                    ins = not ins
                j = i
            return ins
        xs = [p[0] for p in pts]
        ys = [p[1] for p in pts]
        self.shape(f, c, a, (min(xs), min(ys), max(xs), max(ys)))

    def down(self):
        out = bytearray()
        for y in range(N):
            out.append(0)
            for x in range(N):
                acc = [0.0, 0.0, 0.0, 0.0]
                for sy in range(SS):
                    for sx in range(SS):
                        p = self.px[(y * SS + sy) * self.n + x * SS + sx]
                        for k in range(3):
                            acc[k] += p[k] * p[3]
                        acc[3] += p[3]
                a = acc[3] / (SS * SS)
                rgb = [acc[k] / acc[3] if acc[3] > 0 else 0 for k in range(3)]
                out += bytes(int(max(0, min(1, v)) * 255 + 0.5) for v in rgb + [a])
        return bytes(out)


def png(raw, w=N, h=N):
    def chunk(t, d):
        return struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b''))


def mix(a, b, t):
    return [a[k] + (b[k] - a[k]) * t for k in range(3)]


INK = [0.07, 0.06, 0.08]
WHITE = [1.0, 0.98, 0.92]
STEEL = [0.82, 0.84, 0.9]
WOOD = [0.55, 0.38, 0.2]
GOLD = [1.0, 0.82, 0.3]
BLOOD = [0.85, 0.1, 0.1]


def tile(cv, col):
    """The background: a rounded tile, dark at the foot, in the ability's colour."""
    def rr(x, y, inset):
        r = 5.0
        lo, hi = inset, N - inset
        if not (lo <= x <= hi and lo <= y <= hi):
            return False
        cx = min(max(x, lo + r), hi - r)
        cy = min(max(y, lo + r), hi - r)
        return (x - cx) ** 2 + (y - cy) ** 2 <= r * r
    cv.shape(lambda x, y: rr(x, y, 0.0), mix(col, INK, 0.75))
    for band in range(8):
        y0 = 1.5 + band * 3.7
        t = band / 7
        c = mix(mix(col, WHITE, 0.08), mix(col, INK, 0.72), 0.35 + 0.55 * t)
        cv.shape(lambda x, y, y0=y0: rr(x, y, 1.5) and y0 <= y < y0 + 3.8, c)
    # a soft light from the top left
    cv.shape(lambda x, y: rr(x, y, 1.5) and max(0.0, 1 - math.hypot(x - 9, y - 8) / 16) * 0.999, mix(col, WHITE, 0.5), 0.35)


class G:
    """Glyph drawing with an ink outline: every stroke is laid twice."""

    def __init__(self, cv):
        self.cv = cv

    def line(self, x0, y0, x1, y1, w, c):
        self.cv.line(x0, y0, x1, y1, w + 1.6, INK)
        self.cv.line(x0, y0, x1, y1, w, c)

    def lines(self, segs, w, c):
        for s in segs:
            self.cv.line(*s, w + 1.6, INK)
        for s in segs:
            self.cv.line(*s, w, c)

    def poly(self, pts, c, edge=1.0):
        n = len(pts)
        for i in range(n):
            self.cv.line(*pts[i], *pts[(i + 1) % n], edge * 1.6, INK)
        self.cv.poly(pts, c)

    def circle(self, x, y, r, c):
        self.cv.circle(x, y, r + 0.8, INK)
        self.cv.circle(x, y, r, c)

    def ring(self, x, y, r, w, c, a0=0.0, a1=360.0):
        self.cv.ring(x, y, r, w + 1.6, INK, 1.0, a0, a1)
        self.cv.ring(x, y, r, w, c, 1.0, a0, a1)


def rot(pts, ang, cx=16, cy=16):
    s, c = math.sin(math.radians(ang)), math.cos(math.radians(ang))
    return [(cx + (x - cx) * c - (y - cy) * s, cy + (x - cx) * s + (y - cy) * c) for x, y in pts]


def blade(g, ang=45, length=20, width=4.2, col=STEEL, hilt=GOLD, cx=16, cy=16):
    """A sword or dagger, point up before turning."""
    top = cy - length / 2
    b = [(cx, top), (cx + width / 2, top + 4), (cx + width / 2, top + length * 0.66), (cx - width / 2, top + length * 0.66), (cx - width / 2, top + 4)]
    g.poly(rot(b, ang, cx, cy), col)
    hl = [(cx - 0.4, top + 3), (cx - 0.4, top + length * 0.6)]
    p = rot(hl, ang, cx, cy)
    g.cv.line(*p[0], *p[1], 0.9, WHITE, 0.8)
    gy = top + length * 0.66
    p = rot([(cx - width * 1.1, gy + 0.6), (cx + width * 1.1, gy + 0.6), (cx, gy + 1), (cx, top + length)], ang, cx, cy)
    g.line(*p[2], *p[3], 2.2, WOOD)
    g.line(*p[0], *p[1], 2.2, hilt)


def arrow(g, x0, y0, x1, y1, col=WOOD, head=STEEL, w=1.8):
    g.line(x0, y0, x1, y1, w, col)
    ang = math.atan2(y1 - y0, x1 - x0)
    hx = [(x1 + 3 * math.cos(ang), y1 + 3 * math.sin(ang))]
    for d in (2.5, -2.5):
        hx.append((x1 - 3 * math.cos(ang) + 3.4 * math.cos(ang + math.pi / 2) * d / 2.5, y1 - 3 * math.sin(ang) + 3.4 * math.sin(ang + math.pi / 2) * d / 2.5))
    g.poly(hx, head)
    for k in (0.0, 2.6):  # fletching
        bx, by = x0 + k * math.cos(ang), y0 + k * math.sin(ang)
        for d in (1, -1):
            g.cv.line(bx, by, bx - 2.6 * math.cos(ang) + d * 2.4 * math.cos(ang + math.pi / 2), by - 2.6 * math.sin(ang) + d * 2.4 * math.sin(ang + math.pi / 2), 1.2, WHITE)


def star(cx, cy, r0, r1, n, turn=-90):
    pts = []
    for i in range(n * 2):
        r = r0 if i % 2 == 0 else r1
        a = math.radians(turn + i * 180 / n)
        pts.append((cx + r * math.cos(a), cy + r * math.sin(a)))
    return pts


def flame(g, cx, cy, s, outer, inner):
    o = [(cx, cy - 9 * s), (cx + 3 * s, cy - 4 * s), (cx + 6.5 * s, cy - 1 * s), (cx + 6 * s, cy + 4 * s), (cx + 3 * s, cy + 7.5 * s), (cx - 3 * s, cy + 7.5 * s),
         (cx - 6 * s, cy + 4 * s), (cx - 6 * s, cy - 1 * s), (cx - 3 * s, cy - 3 * s), (cx - 2.5 * s, cy - 6.5 * s)]
    g.poly(o, outer)
    i = [(cx + 0.5 * s, cy - 2.5 * s), (cx + 3 * s, cy + 2 * s), (cx + 2 * s, cy + 5.5 * s), (cx - 2 * s, cy + 5.5 * s), (cx - 3 * s, cy + 2 * s)]
    g.cv.poly(i, inner)


def shield(g, cx, cy, s, col, trim):
    pts = [(cx - 8 * s, cy - 8 * s), (cx + 8 * s, cy - 8 * s), (cx + 8 * s, cy + 1 * s), (cx + 5 * s, cy + 6.5 * s), (cx, cy + 10 * s), (cx - 5 * s, cy + 6.5 * s), (cx - 8 * s, cy + 1 * s)]
    g.poly(pts, trim)
    k = 0.72
    g.cv.poly([(cx + (x - cx) * k, cy + 0.4 + (y - cy) * k) for x, y in pts], col)


def heart(g, cx, cy, s, col):
    g.cv.circle(cx - 3.6 * s, cy - 2.5 * s, 4.4 * s + 0.8, INK)
    g.cv.circle(cx + 3.6 * s, cy - 2.5 * s, 4.4 * s + 0.8, INK)
    g.poly([(cx - 7.6 * s, cy - 0.6 * s), (cx + 7.6 * s, cy - 0.6 * s), (cx, cy + 8.5 * s)], col)
    g.cv.circle(cx - 3.6 * s, cy - 2.5 * s, 4.4 * s, col)
    g.cv.circle(cx + 3.6 * s, cy - 2.5 * s, 4.4 * s, col)
    g.cv.circle(cx - 4.2 * s, cy - 3.6 * s, 1.4 * s, WHITE, 0.7)


def plus(g, cx, cy, s, col):
    g.lines([(cx, cy - 7 * s, cx, cy + 7 * s), (cx - 7 * s, cy, cx + 7 * s, cy)], 4.6 * s, col)


def drop(g, cx, cy, s, col):
    g.cv.circle(cx, cy + 2.5 * s, 4.6 * s + 0.8, INK)
    g.poly([(cx, cy - 8 * s), (cx + 4 * s, cy + 1 * s), (cx - 4 * s, cy + 1 * s)], col)
    g.cv.circle(cx, cy + 2.5 * s, 4.6 * s, col)
    g.cv.circle(cx - 1.5 * s, cy + 1.5 * s, 1.2 * s, WHITE, 0.7)


def fist(g, cx, cy, s, col):
    g.poly([(cx - 6 * s, cy - 4 * s), (cx + 6 * s, cy - 4 * s), (cx + 6.5 * s, cy + 3 * s), (cx + 3 * s, cy + 6 * s), (cx - 5 * s, cy + 6 * s), (cx - 6.5 * s, cy + 2 * s)], col)
    for k in (-3, 0, 3):
        g.cv.line(cx + k * s, cy - 3.6 * s, cx + k * s, cy + 0.5 * s, 0.9, INK, 0.7)
    g.cv.line(cx - 6 * s, cy + 1 * s, cx + 2 * s, cy + 1.4 * s, 0.9, INK, 0.7)


def eye(g, cx, cy, s, iris):
    pts = [(cx - 10 * s, cy), (cx - 5 * s, cy - 5 * s), (cx, cy - 6.4 * s), (cx + 5 * s, cy - 5 * s), (cx + 10 * s, cy), (cx + 5 * s, cy + 5 * s), (cx, cy + 6.4 * s), (cx - 5 * s, cy + 5 * s)]
    g.poly(pts, WHITE)
    g.circle(cx, cy, 3.8 * s, iris)
    g.cv.circle(cx, cy, 1.7 * s, INK)
    g.cv.circle(cx - 1.2 * s, cy - 1.3 * s, 0.9 * s, WHITE)


def snow(g, cx, cy, r, col):
    segs = []
    for i in range(3):
        a = math.radians(i * 60 + 30)
        dx, dy = r * math.cos(a), r * math.sin(a)
        segs.append((cx - dx, cy - dy, cx + dx, cy + dy))
        for sgn in (1, -1):
            bx, by = cx + sgn * dx * 0.62, cy + sgn * dy * 0.62
            for t in (50, -50):
                b = a + math.radians(t) + (0 if sgn > 0 else math.pi)
                segs.append((bx, by, bx + r * 0.36 * math.cos(b), by + r * 0.36 * math.sin(b)))
    g.lines(segs, 1.7, col)


# ---------------- the glyphs, by ability key ----------------
def heroic_blow(g, c):
    g.cv.ring(16, 16, 10.5, 2.2, WHITE, 0.55, 200, 300)
    blade(g, 45, 23, 4.6)


def onrush(g, c):
    for k, a in ((0, 0.45), (6, 0.75), (12, 1.0)):
        col = mix(INK, WHITE, a)
        g.lines([(5 + k, 8, 12 + k, 16), (12 + k, 16, 5 + k, 24)], 3.0, col)
    g.cv.line(3, 16, 9, 16, 1.2, WHITE, 0.6)


def battle_cry(g, c):
    g.poly([(5, 13), (11, 13), (17, 8), (17, 24), (11, 19), (5, 19)], GOLD)
    for r in (5, 9, 13):
        g.ring(17, 16, r, 1.9, WHITE, 325, 35)


def rending_cut(g, c):
    for k in (-6, 0, 6):
        g.poly([(9 + k, 5), (11.4 + k, 5), (22 + k * 0.6, 25), (20.6 + k * 0.6, 26)], BLOOD)
    drop(g, 24, 24, 0.55, BLOOD)


def ground_slam(g, c):
    fist(g, 16, 10, 0.95, [0.86, 0.66, 0.5])
    g.lines([(4, 24, 28, 24)], 2.2, [0.5, 0.4, 0.3])
    g.lines([(16, 19, 13, 24), (13, 24, 15, 29), (9, 20, 6, 23), (23, 20, 26, 23)], 1.5, WHITE)


def second_wind(g, c):
    heart(g, 16, 16, 1.0, [0.95, 0.3, 0.28])
    g.ring(16, 15.5, 12.5, 1.8, WHITE, 200, 330)


def last_stand(g, c):
    shield(g, 16, 15, 1.05, [0.55, 0.57, 0.66], STEEL)
    g.lines([(16, 9, 16, 21), (11, 13, 21, 13)], 2.0, WHITE)


def judging_blow(g, c):
    g.cv.circle(21, 10, 8, WHITE, 0.35)
    g.line(8, 26, 19, 13, 2.6, WOOD)
    g.poly(rot([(13, 5), (27, 5), (27, 14), (13, 14)], 42, 20, 10), GOLD)
    g.cv.line(17, 6.5, 24.5, 13, 1.1, WHITE, 0.8)


def holy_mend(g, c):
    g.cv.circle(16, 16, 12, WHITE, 0.3)
    plus(g, 16, 16, 1.25, GOLD)
    g.cv.line(16, 9, 16, 23, 1.2, WHITE, 0.8)


def seal_of_dawn(g, c):
    segs = []
    for i in range(8):
        a = math.radians(i * 45)
        segs.append((16 + 8.5 * math.cos(a), 16 + 8.5 * math.sin(a), 16 + 13 * math.cos(a), 16 + 13 * math.sin(a)))
    g.lines(segs, 2.2, GOLD)
    g.circle(16, 16, 6.6, GOLD)
    g.cv.circle(14.4, 14.4, 2.8, WHITE, 0.8)


def rebuke(g, c):
    g.poly(star(16, 16, 13.5, 6, 8), WHITE)
    fist(g, 16, 16.5, 0.8, GOLD)


def devout_ward(g, c):
    shield(g, 16, 15, 1.05, [0.95, 0.9, 0.7], GOLD)
    plus(g, 16, 14.5, 0.62, [0.8, 0.45, 0.1])


def valour(g, c):
    g.line(9, 5, 9, 28, 2.0, WOOD)
    g.poly([(10, 6), (26, 8), (21, 12.5), (26, 17), (10, 17)], GOLD)
    g.cv.circle(15, 11.5, 2.2, WHITE, 0.9)


def hallowed_ground(g, c):
    g.ring(16, 22, 10.5, 2.4, GOLD, 180, 360)
    g.lines([(4, 22, 28, 22)], 2.4, GOLD)
    g.lines([(16, 18, 16, 5), (10, 18, 7, 9), (22, 18, 25, 9)], 2.0, WHITE)


def quick_shot(g, c):
    arrow(g, 6, 26, 24, 8)


def barbed_arrow(g, c):
    arrow(g, 5, 24, 21, 8)
    g.lines([(14, 15, 12, 10.5), (17.5, 11.5, 15.5, 7)], 1.4, STEEL)
    drop(g, 24.5, 23, 0.62, BLOOD)


def marked_shot(g, c):
    g.ring(16, 16, 9.5, 2.0, WHITE)
    g.lines([(16, 3, 16, 10), (16, 22, 16, 29), (3, 16, 10, 16), (22, 16, 29, 16)], 2.0, WHITE)
    g.circle(16, 16, 2.6, BLOOD)


def snare_shot(g, c):
    up, lo = [(4, 17)], [(4, 19)]
    for i in range(6):
        x = 6 + i * 4
        up += [(x, 9.5 if i % 2 == 0 else 15)]
        lo += [(x, 27 if i % 2 == 0 else 21)]
    up += [(28, 17)]
    lo += [(28, 19)]
    g.poly(up, STEEL)
    g.poly(lo, STEEL)
    g.circle(16, 18, 2.6, [0.75, 0.6, 0.3])
    g.lines([(16, 3, 16, 8)], 1.4, WHITE)


def hamstring(g, c):
    g.line(8, 27, 20, 9, 2.4, WOOD)
    g.poly([(16, 6), (25, 5), (28, 13), (22, 17), (19, 12)], STEEL)
    g.cv.line(24, 7, 26.5, 12.5, 1.1, WHITE, 0.8)
    g.lines([(4, 19, 10, 19), (3, 23, 8, 23)], 1.5, BLOOD)


def hawks_eye(g, c):
    eye(g, 16, 17, 1.05, [0.95, 0.75, 0.15])
    g.lines([(6, 8, 13, 10.5), (26, 8, 19, 10.5)], 1.8, WOOD)


def volley(g, c):
    arrow(g, 4, 22, 14, 6, w=1.5)
    arrow(g, 10, 26, 20, 10, w=1.5)
    arrow(g, 16, 28, 26, 12, w=1.5)


def vicious_strike(g, c):
    blade(g, 45, 19, 3.6)
    g.cv.ring(14, 18, 11, 1.6, WHITE, 0.6, 180, 260)


def finishing_cut(g, c):
    g.poly([(5, 6), (8, 5), (27, 24), (26, 27)], WHITE)
    g.poly([(27, 6), (24, 5), (5, 24), (6, 27)], GOLD)
    g.circle(16, 15.5, 2.4, BLOOD)


def ambush(g, c):
    g.cv.poly([(0, 0), (20, 0), (0, 20)], INK, 0.7)
    blade(g, 135, 20, 3.8, cx=17, cy=17)
    g.lines([(5, 7, 9, 7), (12, 7, 16, 7)], 2.0, [0.95, 0.3, 0.3])


def low_blow(g, c):
    fist(g, 14, 20, 0.85, [0.86, 0.66, 0.5])
    g.poly(star(23, 9, 6, 2.5, 5), GOLD)
    g.poly(star(11, 7, 3.6, 1.5, 5), WHITE)


def sneak(g, c):
    hood = [(16, 4), (23, 8), (26, 17), (25, 27), (7, 27), (6, 17), (9, 8)]
    g.poly(hood, [0.2, 0.18, 0.28])
    g.cv.poly([(16, 10), (21, 13), (21.5, 21), (16, 24), (10.5, 21), (11, 13)], INK)
    g.cv.line(12.5, 16.5, 14.8, 17, 1.5, WHITE)
    g.cv.line(17.2, 17, 19.5, 16.5, 1.5, WHITE)


def flurry(g, c):
    for ang in (5, 45, 85):
        blade(g, ang, 17, 3.0, cx=14 + ang * 0.03, cy=18)


def open_vein(g, c):
    blade(g, 45, 15, 3.0, cx=12, cy=12)
    drop(g, 21, 21, 0.9, BLOOD)


def slip_away(g, c):
    g.ring(16, 16, 10.5, 2.4, WHITE, 120, 400 % 360)
    g.ring(16, 16, 5.5, 2.2, mix(c, WHITE, 0.6), 300, 200)
    g.poly([(22, 3), (28, 9), (20, 10)], WHITE)


def smite(g, c):
    g.cv.poly([(12, 0), (20, 0), (23, 32), (9, 32)], WHITE, 0.3)
    g.poly(star(16, 15, 13, 3.6, 4), GOLD)
    g.cv.circle(16, 15, 3, WHITE)


def mend(g, c):
    heart(g, 16, 16, 1.0, [0.35, 0.8, 0.4])
    plus(g, 16, 15.5, 0.5, WHITE)


def withering_word(g, c):
    pts = []
    for i in range(60):
        a = math.radians(i * 14)
        r = 1.5 + i * 0.19
        pts.append((16 + r * math.cos(a), 16 + r * math.sin(a)))
    g.lines([(*pts[i], *pts[i + 1]) for i in range(len(pts) - 1)], 2.0, [0.72, 0.45, 0.95])
    g.circle(16, 16, 2.2, INK)


def ward_of_light(g, c):
    g.cv.circle(16, 16, 12, WHITE, 0.28)
    g.ring(16, 16, 11.5, 2.4, GOLD)
    g.cv.ring(16, 16, 8.5, 1.4, WHITE, 0.9, 190, 280)
    g.poly(star(16, 16, 6, 2, 4), WHITE)


def renewal(g, c):
    leaf = [(16, 5), (23, 11), (24, 19), (16, 28), (8, 19), (9, 11)]
    g.poly(leaf, [0.4, 0.82, 0.36])
    g.cv.line(16, 8, 16, 26, 1.3, WHITE, 0.85)
    for y in (13, 18):
        g.cv.line(16, y + 2, 20.5, y - 1.5, 1.1, WHITE, 0.8)
        g.cv.line(16, y + 2, 11.5, y - 1.5, 1.1, WHITE, 0.8)


def fortify(g, c):
    g.poly([(7, 28), (7, 11), (10, 11), (10, 7), (13, 7), (13, 11), (19, 11), (19, 7), (22, 7), (22, 11), (25, 11), (25, 28)], [0.72, 0.72, 0.8])
    g.cv.poly([(14, 28), (14, 20), (16, 18), (18, 20), (18, 28)], INK)
    g.cv.line(8.5, 16, 23.5, 16, 0.9, INK, 0.5)


def radiance(g, c):
    g.poly(star(16, 16, 14, 6.2, 12), GOLD)
    g.circle(16, 16, 5.4, WHITE)


def fireball(g, c):
    g.cv.poly([(2, 30), (10, 16), (18, 24)], [1.0, 0.75, 0.2], 0.75)
    g.cv.poly([(1, 24), (9, 14), (12, 20)], [1.0, 0.45, 0.1], 0.6)
    flame_c = [1.0, 0.42, 0.08]
    g.circle(19, 13, 8.2, flame_c)
    g.cv.circle(19.5, 12.5, 5.2, [1.0, 0.8, 0.25])
    g.cv.circle(20.5, 11.5, 2.4, WHITE)


def frostbolt(g, c):
    g.poly([(27, 4), (22, 15), (8, 28), (5, 27), (4, 24), (17, 10)], [0.72, 0.9, 1.0])
    g.cv.line(24, 7, 8, 24, 1.3, WHITE, 0.9)
    g.lines([(7, 9, 11, 9), (9, 7, 9, 11), (22, 23, 26, 23), (24, 21, 24, 25)], 1.3, WHITE)


def fire_blast(g, c):
    g.poly(star(16, 16, 14, 6, 9), [1.0, 0.4, 0.08])
    g.cv.poly(star(16, 16, 9, 4, 9, -70), [1.0, 0.8, 0.25])
    g.cv.circle(16, 16, 3, WHITE)


def ice_ring(g, c):
    g.ring(16, 16, 12, 2.2, [0.72, 0.9, 1.0])
    snow(g, 16, 16, 7.5, WHITE)


def scholars_focus(g, c):
    g.poly([(4, 9), (15, 11), (15, 26), (4, 24)], WHITE)
    g.poly([(28, 9), (17, 11), (17, 26), (28, 24)], WHITE)
    for y in (14, 17.5, 21):
        g.cv.line(6.5, y - 1, 12.5, y, 1.0, [0.35, 0.45, 0.8])
        g.cv.line(19.5, y, 25.5, y - 1, 1.0, [0.35, 0.45, 0.8])
    g.poly(star(16, 6, 4.4, 1.7, 4), GOLD)


def blink(g, c):
    g.cv.circle(8, 22, 5, WHITE, 0.3)
    g.ring(8, 22, 4.5, 1.6, mix(c, WHITE, 0.5))
    g.lines([(11, 19, 15, 15), (17, 13, 20, 10)], 1.8, WHITE)
    g.poly(star(23, 8, 6.5, 2.4, 4), WHITE)


def flame_nova(g, c):
    g.ring(16, 17, 9, 2.6, [1.0, 0.75, 0.2])
    for i in range(6):
        a = math.radians(i * 60 - 90)
        flame(g, 16 + 10.5 * math.cos(a), 16 + 10.5 * math.sin(a), 0.45, [1.0, 0.42, 0.08], [1.0, 0.85, 0.3])


def gem(g, c):
    g.poly([(16, 5), (26, 14), (16, 27), (6, 14)], mix(c, WHITE, 0.4))
    g.cv.poly([(16, 5), (20, 14), (16, 27), (12, 14)], WHITE, 0.4)


GLYPHS = {f.__name__: f for f in [
    heroic_blow, onrush, battle_cry, rending_cut, ground_slam, second_wind, last_stand,
    judging_blow, holy_mend, seal_of_dawn, rebuke, devout_ward, valour, hallowed_ground,
    quick_shot, barbed_arrow, marked_shot, snare_shot, hamstring, hawks_eye, volley,
    vicious_strike, finishing_cut, ambush, low_blow, sneak, flurry, open_vein, slip_away,
    smite, mend, withering_word, ward_of_light, renewal, fortify, radiance,
    fireball, frostbolt, fire_blast, ice_ring, scholars_focus, blink, flame_nova]}


def draw(key, color):
    cv = Canvas()
    tile(cv, color)
    f = GLYPHS.get(key)
    if f is None:
        print(f'warning: no glyph for {key}; drawing a gem', file=sys.stderr)
        f = gem
    f(G(cv), color)
    return cv.down()


def main():
    rules = json.load(open(RULES))
    os.makedirs(OUT, exist_ok=True)
    raws = []
    for a in rules['abilities']:
        raw = draw(a['key'], a.get('color', [0.6, 0.6, 0.6]))
        raws.append((a['class'], raw))
        with open(os.path.join(OUT, a['key'] + '.png'), 'wb') as f:
            f.write(png(raw))
    print(f'{len(raws)} icons in {os.path.relpath(OUT, ROOT)}')
    if '--sheet' in sys.argv:
        out = sys.argv[sys.argv.index('--sheet') + 1]
        classes = []
        for c, _ in raws:
            if c not in classes:
                classes.append(c)
        cols = max(sum(1 for c, _ in raws if c == k) for k in classes)
        cell, scale = N + 4, 3
        w, h = cols * cell * scale, len(classes) * cell * scale
        img = bytearray([24, 22, 28, 255] * (w * h))
        for r, k in enumerate(classes):
            for i, raw in enumerate(x for c, x in raws if c == k):
                for y in range(N):
                    row = raw[y * (N * 4 + 1) + 1:(y + 1) * (N * 4 + 1)]
                    for x in range(N):
                        p = row[x * 4:x * 4 + 4]
                        al = p[3] / 255
                        for sy in range(scale):
                            for sx in range(scale):
                                o = (((r * cell + 2 + y) * scale + sy) * w + (i * cell + 2 + x) * scale + sx) * 4
                                for ch in range(3):
                                    img[o + ch] = int(p[ch] * al + img[o + ch] * (1 - al))
        data = b''.join(b'\x00' + bytes(img[y * w * 4:(y + 1) * w * 4]) for y in range(h))
        open(out, 'wb').write(png(data, w, h))
        print('sheet:', out)


if __name__ == '__main__':
    main()
