from build import recipe, R
from kit import *

# Ashcloak stockade and Emberrun Ford: the bandits' corner of the east frontier.
#
# Shared pieces every section copies VERBATIM into
# tools/stamps/sections/<key>.py (change only KEY), so the same piece looks the
# same everywhere. Build them with:
#   python3 tools/stamps/build.py <key>_clear_6 <key>_clear_10 <key>_clear_14 <key>_clear_20 <key>_ash_flag <key>_web_sheet <key>_cocoons

KEY = "ashcloak"


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


def _web_sheet():
    """Websnare silk hung between two dead posts: white cloth in ONE vertical
    plane (z = 0) as an orthogonal lattice (full columns and rows), never a
    checkerboard or a diagonal step (cloth joins diagonal neighbours into
    slanted ribbons). 7 x 5 x 1. Rotate it across the path or between roots."""
    v = Vol("prop")
    for x in (-3, 3):
        v.col(x, 0, 0, 4, "log")
    for x in range(-2, 3):
        for y in range(1, 5):
            if x % 2 == 0 or y % 2 == 0:
                v.set(x, y, 0, "white")
    return v


_named(f"{KEY}_web_sheet", _web_sheet)


def _cocoons():
    """Three silk-wrapped bundles (white cloth columns 1 x 2) and bones on dirt."""
    v = Vol("prop")
    for (x, z) in ((0, 0), (2, 1), (-1, 2)):
        v.set(x, -1, z, "dirt")
        v.col(x, 0, z, 1, "white")
    v.set(1, -1, -1, "dirt")
    v.set(1, 0, -1, "white")
    return v


_named(f"{KEY}_cocoons", _cocoons)


# ------------------------------------------------------------ Ashcloak pieces

# The ford stones' anchor is the trail's ford bed at the river's centre
# (place emberrun_ford [0,0], fit float). TOP is the layer of the stone tops
# above the anchor: one block above the water.
FORD_TOP = 4


@recipe
def ashcloak_ford_stones():
    """Emberrun Ford: the crossing. Stepping stones along the trail's line
    across the river (z = -x/2), each a solid 1 x 2 column from deep in the
    bed to one block above the water, one every 2 m, no two touching; a timber
    post on each side of the trail mouth on both banks with a short rail
    along the bank; and a gravel bar on the inside of the bend (the west
    bank, upstream of the trail). Place at the ford's centre with
    "fit": "float" (never ground: it would fill the river). Posts and bar are
    columns from deep down, so they stand on whatever the bank is."""
    v = Vol("prop")
    t = FORD_TOP
    for i, x in enumerate(range(-12, 13, 2)):
        z = round(-0.48 * x)
        for dz in (0, 1):
            v.col(x, z + dz, t - 9, t - 1, "stone")
            v.set(x, t, z + dz, "mossy_cobble" if (i + dz) % 3 == 1 else "stone")
    # posts flanking the trail mouths, rails running along the banks
    for (px, pz, dz, lamp) in ((-10, 0, -1, True), (-16, 11, 1, False),
                               (10, -1, 1, False), (17, -14, -1, True)):
        for k in (0, 3):
            v.col(px, pz + dz * k, t - 9, t + 2, "timber_beam")
        for k in (1, 2):
            v.set(px, t + 1, pz + dz * k, "timber_beam")
        if lamp:
            v.set(px, t + 3, pz, "lantern")
    # the gravel bar: a lens of sand, clay and cobble, flush with the water,
    # its bank side just breaking the surface
    for x in range(-8, -1):
        for z in range(-10, -3):
            if ((x + 5) / 3.4) ** 2 + ((z + 7) / 3.4) ** 2 > 1:
                continue
            top = t if x <= -6 else t - 1
            for y in range(t - 9, top + 1):
                v.set(x, y, z, pick(x, y, z, ["sand", "sand", "clay", "cobble", "stone"], 3))
    return v


@recipe
def ashcloak_stockade_keep():
    """The Ashcloak Stockade, faction-correct: the base stockade with its
    flame banners swapped for black-and-red ash cloth, the palisade made solid
    and two rows thick (a plank wall behind the log posts), a 50 cm-step
    sharpened timber tip on every other post, and a lived-in yard: a second
    brazier by the leader's tent, weapon racks and crate stacks."""
    v = R["stockade"]()
    a = 13
    # ash cloth instead of the red flame banners: two flat 2 x 3 sheets
    # hanging from the gate walk, black with a red ember hem
    for k, b in list(v.c.items()):
        if b == "red_banner":
            del v.c[k]
    for x in (-2, -1, 1, 2):
        for y in (4, 5, 6):
            v.set(x, y, a + 1, "black" if y > 4 else "red")
    # the inner row: a plank wall behind the log posts, 4 m
    b = a - 1
    for i in range(-b, b + 1):
        for (x, z) in ((i, -b), (i, b), (-b, i), (b, i)):
            if z == b and abs(x) <= 2:
                continue
            for y in range(0, 4):
                if (x, y, z) not in v.c:
                    v.set(x, y, z, "dark_planks")
    # sharpened tips on every other outer post
    for i in range(-a, a + 1):
        for (x, z) in ((i, -a), (i, a), (-a, i), (a, i)):
            if z == a and abs(x) <= 2:
                continue
            if (x + z) % 2 == 0 and v.get(x, 5, z) == "log" and (x, 6, z) not in v.c:
                v.set(x, 6, z, "timber_beam")
    # a second brazier by the leader's tent
    v.set(-6, -1, -3, "cobble")
    v.set(-6, 0, -3, "brazier")
    # weapon racks along the west wall
    for z0 in (-1, 3):
        v.merge(weapon_rack(), at=(-10, 0, z0), rot=1)
    # crate stacks by the east wall
    for (x, z, h) in ((10, -2, 2), (11, -2, 1), (10, -1, 1), (11, 0, 2), (9, 1, 1)):
        for y in range(h):
            v.set(x, y, z, pick(x, y, z, ["crate", "crate", "barrel"], 5))
    return v


def weapon_rack():
    """A spear rack: two posts, a top bar, three spears (timber poles) and a
    barrel of hafts. 5 x 3 x 2 along x."""
    v = Vol("prop")
    for x in (-2, 2):
        v.col(x, 0, 0, 2, "timber_beam")
    for x in (-1, 0, 1):
        v.set(x, 2, 0, "timber_beam")
    for x in (-1, 1):
        v.col(x, 0, 0, 1, "timber_beam")
    v.set(0, 0, 1, "barrel")
    return v


@recipe
def ashcloak_weapon_rack():
    return weapon_rack()


@recipe
def ashcloak_loot_cart():
    """A stolen cart: the farm cart loaded with crates and a barrel instead of
    hay, more crates and a sack of hay spilled round it."""
    v = R["cart"]()
    v.set(-1, 2, 0, "crate")
    v.set(0, 2, 1, "barrel")
    v.set(-1, 2, 1, "crate")
    v.set(0, 2, 0, "crate")
    for (x, y, z) in ((-4, 0, 0), (-4, 0, 1), (-4, 1, 0), (-3, 0, 3), (1, 0, 4), (-1, 0, -3)):
        v.set(x, y, z, pick(x, y, z, ["crate", "crate", "barrel", "hay_bale"], 7))
    return v


@recipe
def ashcloak_burnt_cart():
    """A burnt-out cart: blackened boards, one side down on its broken
    wheel, scorched ground and spilled crates round it."""
    v = Vol("prop")
    for x in range(-4, 5):
        for z in range(-3, 4):
            if (x / 4.5) ** 2 + (z / 3.5) ** 2 <= 1:
                v.set(x, -1, z, pick(x, -1, z, ["basalt", "dirt", "dirt", "basalt"], 9))
    for z in (0, 1):
        v.set(-2, 0, z, "dark_planks")
        v.set(-1, 0, z, "dark_planks")
        v.set(0, 1, z, "dark_planks")
        v.set(1, 1, z, "dark_planks")
    for (x, z) in ((1, -1), (1, 2)):
        v.set(x, 0, z, "dark_planks")
    v.set(-1, 1, 1, "basalt")
    v.set(2, 1, 0, "timber_beam")
    v.set(3, 0, 0, "timber_beam")
    for (x, y, z) in ((-3, 0, 2), (-3, 0, -1), (2, 0, 3), (-4, 0, 1)):
        v.set(x, y, z, pick(x, y, z, ["crate", "barrel", "crate"], 2))
    return v


@recipe
def ashcloak_scorch():
    """Burnt ground: a ragged stepped disc (radius 4) of basalt and dirt at
    y -1, two charred logs and a blackened stone. The Ashcloaks' mark."""
    v = Vol("prop")
    for x in range(-5, 6):
        for z in range(-5, 6):
            d = x * x + z * z
            if d <= 17 or (d <= 26 and rnd(x, 0, z, 4) % 3 == 0):
                v.set(x, -1, z, pick(x, -1, z, ["basalt", "dirt", "dirt", "basalt", "basalt"], 4))
    v.set(-1, 0, 1, "log")
    v.set(0, 0, 1, "log")
    v.set(2, 0, -2, "basalt")
    return v


@recipe
def ashcloak_charred_stumps():
    """Where the palisade logs came from: six stumps, two of them charred to
    the ground, a trimmed log waiting, chips and burnt dirt round them."""
    v = Vol("prop")
    for (x, z, h) in ((0, 0, 1), (3, 2, 0), (-3, 3, 0), (5, -3, 1), (-4, -2, 0), (1, -5, 0)):
        v.col(x, z, 0, h, "log")
        for (dx, dz) in ((0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)):
            v.set(x + dx, -1, z + dz, "basalt" if (dx + dz + x) % 2 else "dirt")
    for x in range(-1, 3):
        v.set(x, 0, 4, "log")
    v.set(-2, 0, 4, "timber_beam")
    return v


@recipe
def ashcloak_lookout():
    """The bandits' lookout on the north rise: a 3 x 3 plank platform 5 m up
    on four log legs braced at 2 m, a ladder pole, a rail with a gap at the
    ladder, a brazier and an ash flag on a pole: 11 m to the lantern."""
    v = Vol("prop")
    for (x, z) in ((-1, -1), (1, -1), (-1, 1), (1, 1)):
        v.col(x, z, 0, 4, "log")
    for x in (-1, 0, 1):
        for z in (-1, 0, 1):
            v.set(x, 5, z, "dark_planks")
    for z in (-1, 1):
        v.set(0, 2, z, "timber_beam")
    for x in (-1, 1):
        v.set(x, 2, 0, "timber_beam")
    v.col(0, 2, 0, 5, "timber_beam")          # the ladder pole, south
    for (x, z) in ((-1, -1), (0, -1), (1, -1), (-1, 0), (-1, 1), (1, 0)):
        v.set(x, 6, z, "timber_beam")
    v.set(0, 6, 0, "brazier")
    v.col(1, -1, 7, 10, "timber_beam")
    for y in (7, 8, 9):
        v.set(2, y, -1, "black")
        v.set(3, y, -1, "red")
    v.set(1, 11, -1, "lantern")
    return v
