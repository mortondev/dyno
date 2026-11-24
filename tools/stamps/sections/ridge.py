"""The Ridge: Miner's Rest, the Ridge Cut and the Hollow Mines.

Build with:
    python3 tools/stamps/build.py ridge_crag_wall ridge_falls ridge_winch_tower \
        ridge_ore_chute ridge_mine_head ridge_cliff_face ridge_adit_brazier \
        ridge_rock_cut ridge_lookout ridge_ore_pile ridge_cart_wreck \
        ridge_intruders_camp ridge_barricade ridge_boarded_adit ridge_beacon \
        ridge_prospect ridge_trial_shaft ridge_rail_spur ridge_clear_10 ridge_clear_14

Everything is whole cubes: rock is layered columns stepped into ledges,
braces are stairs of blocks touching face to face, round things are
stepped circles. Coordinates: x east, z south, y up; y = 0 is the first
layer above the ground the stamp stands on.
"""
from build import recipe, R
from kit import *
import math

KEY = "ridge"


# ------------------------------------------------ shared pieces (verbatim)

def _named(name, f):
    f.__name__ = name
    return recipe(f)


def _clear(r):
    """A clearing: one layer of air cells in a stepped circle of radius r.
    Place with "fit": "float", "dy": 40. It draws nothing and levels nothing;
    wild trees, strewn stamps (boulders, logs, hedges) and the 1 m ground
    blocks stay off its footprint. Size it to the feature's radius + 6 m so
    crowns do not overhang. (Grass tufts and flowers still grow.)"""
    v = Vol("prop")
    for x in range(-r, r + 1):
        for z in range(-r, r + 1):
            if x * x + z * z <= r * r + r:
                v.set(x, 0, z, "air")
    return v


for _r in (6, 10, 14, 20):
    _named(f"{KEY}_clear_{_r}", lambda r=_r: _clear(r))


def _ash_flag():
    """Ashcloak war flag (hostile camps only): a 6 m pole, a black-and-red
    cloth flag on the +x side, a lantern on top. 3 x 7 x 1."""
    v = Vol("prop")
    v.col(0, 0, 0, 5, "timber_beam")
    for y in range(2, 6):
        v.set(1, y, 0, "black")
        v.set(2, y, 0, "red")
    v.set(0, 6, 0, "lantern")
    return v


_named(f"{KEY}_ash_flag", _ash_flag)


# ------------------------------------------------ rock

def strata(x, y, z, salt=0):
    """Layered ridge rock: talus at the foot, grey stone with a cobbled
    bed every five metres and a pale clay seam."""
    if y <= 0:
        return pick(x, y, z, ["stone", "stone", "stone", "cobble", "stone"], salt)
    if y % 5 == 2:
        return "stone" if rnd(x // 2, y, z // 2, salt) % 4 else "cobble"
    if y % 7 == 4:
        return "clay"
    return "stone"


def rock(v, x, z, h, salt=0, soil=0, base=-1):
    """One column of layered rock from the foundation course up to h - 1:
    turf on the ledges (soil 1), and on the top tier (soil 2) turf over a
    band of earth."""
    if h <= 0:
        return
    for y in range(base, h):
        v.set(x, y, z, strata(x, y, z, salt))
    if soil and h >= 3:
        v.set(x, h - 1, z, "grass")
        if soil > 1:
            v.set(x, h - 2, z, "dirt")


def tier(d, steps):
    """Stepped profile: steps = [(limit, height), ...]; the first limit that
    d is under gives the height."""
    for lim, h in steps:
        if d < lim:
            return h
    return steps[-1][1]


def jit(x, z, salt, cell=3, amp=1.0):
    """Coarse jitter (constant over cell x cell metres): breaks a ruled
    edge into steps."""
    return ((rnd(x // cell, 0, z // cell, salt) % 5) - 2) * 0.5 * amp


def massif(anchor, span, height, salt=0, soil_min=6):
    """Rock from a height function over place coordinates; the stamp's
    anchor is `anchor` (place coordinates). Turf on ledges from soil_min
    up, earth under the turf on the top tier."""
    v = Vol("landmark")
    v.foundation = "stone"
    ax, az = anchor
    (x0, x1), (z0, z1) = span
    hs = {(x, z): height(x, z) for z in range(z0, z1 + 1) for x in range(x0, x1 + 1)}
    top = max(hs.values())
    for (x, z), h in hs.items():
        if h > 0:
            rock(v, x - ax, z - az, h, salt, soil=0 if h < soil_min else 2 if h == top else 1)
    return v


FRONT = [(0, 0), (1.5, 4), (3, 7), (5, 10), (1e9, 13)]
BACK = [(0, 0), (2, 3), (4, 6), (6, 9), (1e9, 13)]
ENDS = [(0, 0), (2, 3), (4, 6), (6, 9), (8, 11), (1e9, 13)]

# Miner's Rest crag: an arc of cliff round the hub's east side, 23 m out,
# 18 m deep, 13 m at the top; from bearing -40 (NE, clear of the mine
# trail) to +48 (SE, behind the falls). Anchor at place (27, -2).
CRAG_ANCHOR = (27, -2)


def crag_height(x, z):
    r = math.hypot(x, z)
    th = math.degrees(math.atan2(z, x))
    if th < -40 or th > 48 or r < 18:
        return 0
    R0 = 23 + jit(x, z, 41)
    d = r - R0
    b = R0 + 18 + jit(x, z, 42, 4) - r
    e = min(th + 40, 48 - th) * math.pi / 180 * r
    return min(tier(d, FRONT), tier(b, BACK), tier(e, ENDS))


@recipe
def ridge_crag_wall():
    """The cliff Miner's Rest stands against: stepped ledges of layered
    grey rock with red earth and turf on the ledges, 13 m high."""
    return massif(CRAG_ANCHOR, ((14, 46), (-34, 34)), crag_height, 43)


@recipe
def ridge_falls():
    """A terraced cascade five blocks wide, cut into a cliff face that
    faces west (-x): four steps of water from a spring 12 m up, a pool at
    the foot and a stony channel running out west then south (to meet the
    hub brook). x = 0 is the cliff's front."""
    v = Vol("landmark")
    W = 2                       # half width: 5 blocks
    steps = [(0, 3), (2, 6), (4, 9), (6, 12)]   # (lip x, water level)
    # carve the notch and the air in front of it
    for z in range(-W - 1, W + 2):
        for x in range(-2, 10):
            for y in range(0, 16):
                v.set(x, y, z, "air")
    # the rock under each step and the wet lips either side
    for i, (lx, wy) in enumerate(steps):
        nx = steps[i + 1][0] if i + 1 < len(steps) else 10
        for x in range(lx, nx):
            for z in range(-W, W + 1):
                for y in range(-1, wy):
                    v.set(x, y, z, pick(x, y, z, ["stone", "stone", "mossy_cobble", "cobble"], 51))
                v.set(x, wy, z, "water")
            for z in (-W - 1, W + 1):
                for y in range(-1, wy + 2):
                    v.set(x, y, z, pick(x, y, z, ["mossy_cobble", "stone", "cobble"], 52))
        # the curtain falling from this step's lip to the step below
        below = steps[i - 1][1] if i > 0 else -1
        for z in range(-W, W + 1):
            for y in range(below + 1, wy + 1):
                v.set(lx - 1, y, z, "water")
    # the spring: rock round the top pool
    for z in range(-W - 1, W + 2):
        for y in range(-1, 14):
            v.set(10, y, z, strata(10, y, z, 53))
        v.set(10, 13, z, "grass")
    # pool at the foot, x -8..-1
    for x in range(-9, 1):
        for z in range(-5, 6):
            d = ((x + 4.5) / 4.5) ** 2 + (z / 4.2) ** 2
            if d <= 1.0:
                v.set(x, -1, z, "water")
                v.set(x, -2, z, "water" if d < 0.6 else "sand")
                v.set(x, -3, z, "sand")
                for y in range(0, 3):
                    v.set(x, y, z, "air")
            elif d <= 1.45:
                v.set(x, -1, z, pick(x, 0, z, ["sand", "cobble", "mossy_cobble", "sand", "stone"], 54))
    # the channel: west along z 0..2, then south along x -15..-13 to z 9
    chan = [(x, z) for x in range(-13, -8) for z in range(0, 3)] + \
           [(x, z) for x in range(-15, -12) for z in range(0, 10)]
    for (x, z) in chan:
        v.set(x, -1, z, "water")
        v.set(x, -2, z, "water")
        v.set(x, -3, z, "cobble")
        v.set(x, 0, z, "air")
    for (x, z) in chan:
        for (dx, dz) in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            p = (x + dx, z + dz)
            if p not in chan and v.get(p[0], -1, p[1]) is None:
                v.set(p[0], -1, p[1], pick(p[0], 0, p[1], ["cobble", "stone", "sand", "cobble"], 55))
    v.set(-6, 0, 4, "stone")
    v.set(-2, 0, -4, "cobble")
    v.points["pool"] = (-4, 0, 0)
    return v


def timber_tower(v, half, H, bands, block="dark_planks"):
    """A square timber frame: corner posts of beams, whole-block rings at
    the bands, and stair braces (blocks touching face to face) rising from
    each bottom corner to the middle of the band above, on every face."""
    a = half
    for (x, z) in ((-a, -a), (a, -a), (-a, a), (a, a)):
        v.col(x, z, 0, H, "timber_beam")
    for y in bands:
        for i in range(-a, a + 1):
            for (x, z) in ((i, -a), (i, a), (-a, i), (a, i)):
                v.set(x, y, z, block)
    levels = [0] + list(bands)
    for y0, y1 in zip(levels, levels[1:]):
        rise = y1 - y0 - 1
        if rise < 2:
            continue
        for s in (-1, 1):            # from each corner towards the middle
            cells = []
            x, y = -a * s + s, y0 + (1 if y0 > 0 else 0)
            while y < y1 and abs(x) <= a - 1 and (x * s) <= 0:
                cells.append((x, y))
                if len(cells) % 2:
                    y += 1
                else:
                    x += s
            # a king post from the apex up to the band above
            top = max(yy for (_, yy) in cells)
            cells += [(0, yy) for yy in range(top + 1, y1)]
            for (i, yy) in cells:
                for (px, pz) in ((i, -a), (i, a), (-a, i), (a, i)):
                    v.set(px, yy, pz, block)
    return v


def wheel(v, cx, cy, z, r=2, block="dark_planks", axis="z"):
    """A winding wheel in the plane z = const (axis "z") or x = const
    (axis "x", then cx is that x and z the wheel's centre): a stepped ring
    with a cross of spokes and a hub."""
    for dx in range(-r, r + 1):
        for dy in range(-r, r + 1):
            ring = max(abs(dx), abs(dy)) == r and not (abs(dx) == r and abs(dy) == r)
            spoke = dx == 0 or dy == 0
            if ring or spoke:
                if axis == "z":
                    v.set(cx + dx, cy + dy, z, block)
                else:
                    v.set(cx, cy + dy, z + dx, block)


@recipe
def ridge_winch_tower():
    """Miner's Rest's winch tower on the crag top: a 15 m frame of five by
    five with braced bands, a winding wheel on its deck, and a jib reaching
    out west over the cliff with a bucket on its rope."""
    v = Vol("landmark")
    H = 12
    timber_tower(v, 2, H, (4, 8, H))
    v.fill((-2, H, -2), (2, H, 2), "planks")
    v.fill((-2, -1, -2), (2, -1, 2), "planks")
    # the winch house roof: a stepped slate cap over the deck
    for (k, y) in ((0, H + 3), (1, H + 4)):
        for x in range(-2 + k, 3 - k):
            for z in range(-2 + k, 3 - k):
                if k == 1 or abs(x) == 2 or abs(z) == 2:
                    v.set(x, y, z, "slate_roof")
    for (x, z) in ((-2, -2), (2, -2), (-2, 2), (2, 2)):
        v.col(x, z, H + 1, H + 2, "timber_beam")
    wheel(v, 0, H + 2, 0, 1)
    # the jib: a beam out west, a rope (beams) down to a bucket
    for x in range(-6, -2):
        v.set(x, H, 0, "dark_planks")
    for y in range(H - 6, H):
        v.set(-6, y, 0, "timber_beam")
    v.set(-6, H - 7, 0, "barrel")
    v.set(-6, H + 1, 0, "lantern")
    v.set(3, 1, 0, "crate")
    v.points["top"] = (0, H + 4, 0)
    return v


@recipe
def ridge_ore_chute():
    """An ore chute down the crag face to a bin over the rail end: stairs
    of planks in a groove cut into the rock, side boards, the bin on
    timber legs with coal and iron in it. x = 0 is the cliff's front,
    rock is +x."""
    v = Vol("prop")
    for x in range(0, 6):
        top = 2 + 2 * x
        for z in range(-1, 2):
            v.set(x, top - 1, z, "dark_planks")
            v.set(x, top, z, "planks")
            for y in range(top + 1, top + 4):
                v.set(x, y, z, "air")
        for z in (-2, 2):
            v.set(x, top, z, "dark_planks")
            v.set(x, top + 1, z, "dark_planks")
    # the bin
    for (x, z) in ((-3, -1), (-1, -1), (-3, 1), (-1, 1)):
        v.col(x, z, 0, 1, "timber_beam")
    for x in range(-3, 0):
        for z in range(-1, 2):
            v.set(x, 2, z, "dark_planks")
            edge = x in (-3, -1) or abs(z) == 1
            v.set(x, 3, z, "planks" if edge else ("coal_ore" if (x + z) % 2 else "iron_ore"))
    v.set(-2, 3, 0, "iron_ore")
    v.set(-1, 3, 0, "coal_ore")
    return v


@recipe
def ridge_mine_head():
    """The Hollow Mines headframe: an 18 m frame of seven by seven with
    whole-block rings at the storey bands and stair braces, back legs
    raking down behind (-x) as stairs of whole blocks, a winding house
    with its wheel on top under a stepped slate roof, and a lantern beacon
    on the ridge. Red flame banners: the miners' own."""
    v = Vol("landmark")
    H = 17
    timber_tower(v, 3, H, (5, 11, H))
    v.fill((-3, -1, -3), (3, -1, 3), "planks")
    v.fill((-3, H, -3), (3, H, 3), "planks")
    # the cable down the shaft and the cage where the hoist stopped
    for y in range(6, H):
        v.set(0, y, 0, "timber_beam")
    v.fill((-1, 5, -1), (1, 5, 1), "planks")
    v.set(0, 9, 0, "lantern")
    # back legs: stairs of whole blocks, three tall, one out per two down
    for z in (-2, 2):
        k = 0
        while True:
            hi = H - 1 - 2 * k
            lo = max(hi - 2, -1)
            v.col(-4 - k, z, lo, hi, "dark_planks")
            if lo <= -1:
                break
            k += 1
        # ties between the two legs at the storey bands
    for k in range(0, 9):
        hi = H - 1 - 2 * k
        if hi in (11, 5) or hi - 1 in (11, 5):
            for z in range(-1, 2):
                v.set(-4 - k, hi - 1, z, "dark_planks")
    # the winding house on the deck
    hx0, hx1, hz0, hz1 = -3, 3, -2, 2
    for y in range(H + 1, H + 4):
        for x in range(hx0, hx1 + 1):
            for z in range(hz0, hz1 + 1):
                if x in (hx0, hx1) or z in (hz0, hz1):
                    corner = x in (hx0, hx1) and z in (hz0, hz1)
                    v.set(x, y, z, "timber_beam" if corner else "planks")
    v.set(0, H + 2, hz0, "leaded_window")
    v.set(0, H + 2, hz1, "leaded_window")
    v.col(hx0, 0, H + 1, H + 2, "air")          # door onto the deck
    top = roof(v, hx0, hz0, hx1, hz1, H + 3, "slate", True)
    # the beacon: lanterns at both ends of the ridge and a brazier between
    v.set(hx0, top + 1, 0, "lantern")
    v.set(hx1, top + 1, 0, "lantern")
    v.set(0, top + 1, 0, "brazier")
    # the big wheel on the shaft side (+x, away from the legs)
    wheel(v, hx1 + 1, H + 3, 0, 2, axis="x")
    # red flame banners on the winding house's -z wall
    v.col(-2, -3, H + 1, H + 2, "red_banner")
    v.col(2, -3, H + 1, H + 2, "red_banner")
    v.set(-4, 0, 3, "crate")
    v.set(4, 0, -3, "barrel")
    v.points["top"] = (0, top + 1, 0)
    return v


# Hollow Mines cliff: round the yard's east and south, in front of the
# hill; the portal (place (12, -3), facade at x 11) sits in its face.
# Anchor at the portal, place (12, -3). Front tiers 5/8/11; round the
# portal itself the rock stands 9 then 11 so the facade is set into it.
CLIFF_ANCHOR = (12, -3)
CLIFF_FRONT = [(0, 0), (1.5, 5), (3, 8), (1e9, 11)]
PORTAL_FRONT = [(0, 0), (1, 9), (1e9, 11)]
HEAD_AT = (6, 25)                # the headframe's plateau (place)


def cliff_height(x, z):
    r = math.hypot(x, z)
    th = math.degrees(math.atan2(z, x))
    best = 0
    if 12 <= x <= 30 and -10 <= z <= 4:
        best = tier(x - 12, PORTAL_FRONT)
    if -40 <= th <= 104 and r >= 13:
        R0 = 15.5 + jit(x, z, 61)
        e = min(th + 40, 104 - th) * math.pi / 180 * r
        b = R0 + 15 + jit(x, z, 63, 4) - r
        h = min(tier(r - R0, CLIFF_FRONT), tier(e, ENDS[:-1] + [(1e9, 11)]),
                tier(b, BACK[:-1] + [(1e9, 11)]))
        best = max(best, h)
    # the plateau round the headframe is flat at 11
    if math.hypot(x - HEAD_AT[0], z - HEAD_AT[1]) <= 13 and -40 <= th <= 110 and r >= 18.5:
        best = 11
    return best


@recipe
def ridge_cliff_face():
    """The stepped cliff the Hollow Mines adit is cut into: layered grey
    rock 11 m high with turf ledges, wrapping the yard from the portal
    round to the south; a flat plateau on top for the headframe, carved
    clear of the hill, with a timber fence along its edge. Place it BEFORE
    the hollow_mines dig and the mine_portal so they cut the adit back
    through it."""
    ax, az = CLIFF_ANCHOR
    v = massif(CLIFF_ANCHOR, ((-10, 36), (-14, 40)), cliff_height, 62)
    # carve the hill above the plateau so the headframe stands clear
    hx, hz = HEAD_AT
    for z in range(hz - 14, hz + 15):
        for x in range(hx - 14, hx + 15):
            if math.hypot(x - hx, z - hz) <= 13 and cliff_height(x, z) >= 11:
                for y in range(11, 26):
                    v.set(x - ax, y, z - az, "air")
    # the fence along the plateau's west and north edges
    for z in range(hz - 10, hz + 10):
        for x in range(hx - 10, hx + 10):
            if cliff_height(x, z) == 11 and (cliff_height(x - 1, z) < 11 or cliff_height(x, z - 1) < 11):
                if math.hypot(x - hx, z - hz) <= 9:
                    v.set(x - ax, 11, z - az, "timber_beam")
    v.points["plateau"] = (hx - ax, 11, hz - az)
    return v


@recipe
def ridge_adit_brazier():
    """A brazier on a two-block stone plinth, lighting a mine mouth."""
    v = Vol("prop")
    v.set(0, -1, 0, "cobble")
    v.col(0, 0, 0, 1, "stone_bricks")
    v.set(0, 2, 0, "brazier")
    return v


# ------------------------------------------------ the Ridge Cut

CUT_A, CUT_O, CUT_B = (-50, 65), (0, 0), (26, -64)


def _seg(p, a, b):
    dx, dz = b[0] - a[0], b[1] - a[1]
    L = math.hypot(dx, dz)
    ux, uz = dx / L, dz / L
    px, pz = p[0] - a[0], p[1] - a[1]
    t = max(0.0, min(L, px * ux + pz * uz))
    qx, qz = a[0] + ux * t, a[1] + uz * t
    side = ux * pz - uz * px          # < 0 west, > 0 east
    return math.hypot(p[0] - qx, p[1] - qz), side, t, L


def cut_frame(x, z):
    """Distance from the trail, side (-1 west, +1 east) and the distance
    along it from the cut's centre (negative toward the hub)."""
    d1, s1, t1, L1 = _seg((x, z), CUT_A, CUT_O)
    d2, s2, t2, L2 = _seg((x, z), CUT_O, CUT_B)
    if d1 <= d2:
        return d1, (-1 if s1 < 0 else 1), t1 - L1
    return d2, (-1 if s2 < 0 else 1), t2


WALL_IN = [(0, 0), (1.5, 4), (3, 7), (1e9, 10)]


def cut_height(x, z):
    d, side, t = cut_frame(x, z)
    if not (-17 <= t <= 19):
        return 0
    if side < 0 and -5 <= t <= 4:
        return 0                     # the gap for the lookout
    inner = 4.5 + jit(x, z, 71, 2, 0.6)
    width = 7 + jit(x, z, 72, 4)
    a = d - inner
    b = inner + width - d
    e = min(t + 17, 19 - t)
    if side < 0:
        e = min(e, abs(t + 5) if t < -5 else abs(t - 4))
    return min(tier(a, WALL_IN), tier(b, BACK[:-1] + [(1e9, 10)]),
               tier(e, ENDS[:-2] + [(8, 9), (1e9, 10)]))


@recipe
def ridge_rock_cut():
    """The Ridge Cut: stepped walls of layered rock 4-10 m high either
    side of the mine trail where it climbs through the ridge's shoulder,
    with a gap on the west side where the lookout faces the valley.
    Anchored at the cut's centre; place with fit float."""
    return massif((0, 0), ((-30, 30), (-30, 30)), cut_height, 73)


@recipe
def ridge_lookout():
    """A miners' lookout over the Emberrun valley: a stepped stone
    platform with a parapet on the valley side (-x), a bench and a lantern
    on a post. No banner (the trail is the miners')."""
    v = Vol("prop")
    v.foundation = "stone_bricks"
    for x in range(-3, 4):
        for z in range(-3, 4):
            if x * x + z * z <= 12:
                v.set(x, -1, z, "stone_bricks")
                v.set(x, 0, z, pick(x, 0, z, ["cobble", "flagstones", "flagstones"], 81))
                if x <= -2 or (abs(z) == 3 and x <= 0):
                    if x * x + z * z >= 7:
                        v.set(x, 1, z, "stone_bricks")
    for z in range(-1, 2):
        v.set(-1, 1, z, "planks")
    v.col(2, -2, 1, 3, "timber_beam")
    v.set(2, 4, -2, "lantern")
    # steps up from the trail side
    for z in range(-1, 2):
        v.set(4, -1, z, "cobble")
        v.set(4, 0, z, "air")
    return v


# ------------------------------------------------ props and story

@recipe
def ridge_ore_pile():
    """Heaps of ore waiting for the carts: stone only at the foot, the
    tops coal and iron, a crate and a barrel beside."""
    v = Vol("prop")
    for (cx, cz, r, h, salt) in ((0, 0, 2.6, 3, 31), (3, 2, 1.8, 2, 32), (-3, -2, 1.6, 2, 33)):
        for x in range(int(cx - r) - 1, int(cx + r) + 2):
            for z in range(int(cz - r) - 1, int(cz + r) + 2):
                d = math.hypot(x - cx, z - cz) / r
                if d > 1.0:
                    continue
                top = max(1, round(h * (1 - d * d) + 0.4))
                for y in range(0, top):
                    if y == 0 and d > 0.7:
                        b = "stone"
                    else:
                        b = "coal_ore" if rnd(x, y, z, salt) % 3 else "iron_ore"
                    if v.get(x, y, z) is None:
                        v.set(x, y, z, b)
    v.set(4, 0, -2, "barrel")
    v.set(-2, 0, 3, "crate")
    v.set(-1, 0, 3, "crate")
    return v


@recipe
def ridge_cart_wreck():
    """An ore cart off its rails on its side, its load spilled: the bed
    stood on edge, a wheel off, ore across the ground, the bent track
    ending under it."""
    v = Vol("prop")
    # the cart on its side: the bed is a wall of planks one wide (x),
    # the sides stand out from it
    for z in range(-1, 2):
        for y in range(0, 2):
            v.set(0, y, z, "dark_planks")
    for y in range(0, 2):
        v.set(1, y, -1, "dark_planks")
        v.set(1, y, 1, "dark_planks")
    v.set(-1, 0, 2, "timber_beam")
    # the spill
    for (x, z) in ((2, 0), (3, 0), (2, 1), (3, -1), (4, 1), (3, 2), (5, 0)):
        v.set(x, 0, z, "coal_ore" if (x + z) % 3 else "iron_ore")
    v.set(2, 1, 0, "coal_ore")
    # the track it came off: a short bent length, then broken sleepers
    for z in range(-7, -1):
        v.set(-1, 0, z, "rail")
    v.set(0, 0, -3, "timber_beam")
    v.set(-2, 0, 3, "crate")
    v.set(-3, 0, 4, "planks")
    return v


@recipe
def ridge_intruders_camp():
    """Where the Candlekin came up into the yard: the miners' fire gone to
    embers in a scorched ring, their gear thrown about, a ring of guttered
    torches stuck in the ground. Open in the middle; nothing over 1 m but
    the torches."""
    v = Vol("prop")
    for x in range(-3, 4):
        for z in range(-3, 4):
            if x * x + z * z <= 10:
                v.set(x, -1, z, pick(x, 0, z, ["basalt", "dirt", "dirt", "basalt", "cobble"], 91))
    for (x, z) in ((-1, 0), (1, 0), (0, -1), (0, 1)):
        v.set(x, -1, z, "cobble")
    v.set(0, -1, 0, "emberstone")
    v.set(0, 0, 1, "log")
    v.set(-1, 0, -1, "log")
    for (x, z) in ((4, 0), (-4, 1), (1, 4), (-1, -4), (3, -3), (-3, 3)):
        v.set(x, 0, z, "torch")
    v.set(3, 0, 2, "crate")
    v.set(-3, 0, -2, "barrel")
    v.set(2, 0, -2, "planks")
    return v


@recipe
def ridge_barricade():
    """The miners' barricade, broken: a run of posts and rails across the
    track, smashed open in the middle (6 m gap), planks thrown down."""
    v = Vol("prop")
    for z in list(range(-8, -4)) + list(range(5, 9)):
        v.set(0, 0, z, "timber_beam")
        if abs(z) in (5, 8):
            v.col(0, z, 1, 2, "timber_beam")
        elif abs(z) != 7:
            v.set(0, 1, z, "timber_beam")
    for (x, z) in ((-1, -2), (-2, -1), (-1, 2), (1, 3)):
        v.set(x, 0, z, "planks")
    return v


@recipe
def ridge_boarded_adit():
    """The old upper adit, given up: the mine portal's mouth with its
    track torn up, planks nailed across the opening (one hanging loose,
    a gap at the bottom), no lights."""
    v = R["mine_portal"]()
    for (x, y, z), b in list(v.c.items()):
        if b == "lantern":
            del v.c[(x, y, z)]
        if b == "rail" and (x < -3 or (x + z) % 3 == 0):
            del v.c[(x, y, z)]
    for z in range(-1, 2):
        v.set(0, 1, z, "dark_planks")
    v.set(0, 2, -1, "dark_planks")
    v.set(0, 2, 0, "dark_planks")
    v.col(0, 1, 2, 2, "timber_beam")
    v.set(-1, 0, 1, "planks")
    v.doors = []
    return v


@recipe
def ridge_beacon():
    """The miners' beacon on the crest: a stepped cairn tower of stone,
    7 x 7 at the foot to 3 x 3 at the head, 13 m, a brazier on top and a
    lantern on a beam: the mark of the Hollow Mines on the skyline."""
    v = Vol("landmark")
    v.foundation = "stone"
    for y in range(0, 13):
        a = 3 if y < 4 else 2 if y < 9 else 1
        for x in range(-a, a + 1):
            for z in range(-a, a + 1):
                v.set(x, y, z, pick(x, y, z, ["stone_bricks", "cobble", "stone_bricks", "stone"], 101))
    for x in range(-3, 4):
        for z in range(-3, 4):
            v.set(x, -1, z, "cobble")
    v.set(0, 13, 0, "brazier")
    for (x, z) in ((-1, -1), (1, -1), (-1, 1), (1, 1)):
        v.set(x, 13, z, "cobble")
    v.col(2, 0, 9, 11, "timber_beam")
    v.set(3, 11, 0, "timber_beam")
    v.set(3, 10, 0, "lantern")
    # a red flame banner down the west face
    v.col(-3, 0, 5, 8, "red_banner")
    return v


@recipe
def ridge_prospect():
    """A prospectors' working: a trial pit with a windlass over it (posts,
    a beam, a bucket on its rope), a spoil heap of earth and ore, and a
    sluice box of planks."""
    v = Vol("prop")
    for x in range(-1, 2):
        for z in range(-1, 2):
            v.set(x, -1, z, "cobble" if (x, z) != (0, 0) else "air")
    v.set(0, -2, 0, "air")
    v.set(0, -3, 0, "dirt")
    for z in (-2, 2):
        v.col(0, z, 0, 2, "timber_beam")
    for z in range(-1, 2):
        v.set(0, 2, z, "dark_planks")
    v.set(0, 1, 0, "timber_beam")
    v.set(0, 0, 0, "barrel")
    for (x, z, h) in ((3, -1, 2), (3, 0, 2), (4, 0, 1), (3, 1, 1), (4, -1, 1), (2, -2, 1)):
        for y in range(h):
            v.set(x, y, z, "dirt" if y == 0 else ("iron_ore" if (x + z) % 2 else "dirt"))
    for x in range(-5, -2):
        v.set(x, 0, 2, "planks")
        v.set(x, 0, 3, "water" if x != -5 else "planks")
        v.set(x, 0, 4, "planks")
    v.set(-2, 0, -2, "crate")
    return v


@recipe
def ridge_trial_shaft():
    """An old trial shaft, worked out: a stepped stone collar round a
    boarded hole, a fallen windlass, a heap of spoil."""
    v = Vol("prop")
    for x in range(-2, 3):
        for z in range(-2, 3):
            ring = max(abs(x), abs(z)) == 2
            if ring and not (abs(x) == 2 and abs(z) == 2):
                v.set(x, 0, z, pick(x, 0, z, ["cobble", "stone", "cobble"], 111))
            elif not ring:
                v.set(x, -1, z, "dark_planks" if (x + z) % 2 == 0 else "air")
    v.set(3, 0, 0, "timber_beam")
    v.set(4, 0, 0, "timber_beam")
    v.set(4, 0, 1, "timber_beam")
    for (x, z) in ((-4, 1), (-4, 2), (-5, 1), (-3, 3)):
        v.set(x, 0, z, "dirt")
    v.set(-4, 1, 1, "dirt")
    return v


@recipe
def ridge_rail_spur():
    """Eight metres of old track along x, the last two sleepers bare (an
    abandoned spur): lay beside the mine trail with fit ground."""
    v = Vol("prop")
    for x in range(-4, 4):
        if x != 2:
            v.set(x, 0, 0, "rail")
        v.set(x, -1, 0, "dirt" if x % 3 else "cobble")
    v.set(2, 0, 0, "timber_beam")
    return v
