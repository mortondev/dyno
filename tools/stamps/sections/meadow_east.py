from build import recipe, R
from kit import *

# ---------------------------------------------------------------------------
# Shared pieces (director/tools/shared_pieces.py), copied verbatim.
# ---------------------------------------------------------------------------

KEY = "meadow_east"   # <- your section key


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


# ---------------------------------------------------------------------------
# East meadow pieces
# ---------------------------------------------------------------------------

def wheel_upright(v, x, y, z, along_x=True):
    """A cart wheel standing on its rim, in one vertical plane (along x, or
    along z): a 3 x 3 ring of timber with an open hub. Free timber draws as
    thin joined sticks, so the ring reads as an open rim with sky through
    it. (Planks in a 3 x 3 read as a box or a wall with a window, and a
    timber ring with a hub cell as a gate.)"""
    for a in (-1, 0, 1):
        for b in (0, 1, 2):
            if (a, b) == (0, 1):
                continue
            if along_x:
                v.set(x + a, y + b, z, "timber_beam")
            else:
                v.set(x, y + b, z + a, "timber_beam")


def wheel_flat(v, x, y, z):
    """A wheel lying flat: a 3 x 3 timber ring round an open hub."""
    for a in (-1, 0, 1):
        for b in (-1, 0, 1):
            if (a, b) != (0, 0):
                v.set(x + a, y, z + b, "timber_beam")


@recipe
def meadow_east_caravan():
    """The Wrecked Caravan after the Ashcloak ambush. Anchor between the two
    wagons; place it at the caravan's (-3, -12).
    West: a covered wagon slewed off the road (a rut runs to it from the
    paving), its rear axle broken so the back sags onto the grass. One rear
    wheel lies flat beside it. The canvas still covers the front half; the
    back half is torn away down to the bare timber hoops.
    East: the second wagon flipped onto its back, its crushed canvas under
    it and its four wheels in the air, with an Ashcloak flag planted in its
    upturned bed (the same flag as meadow_east_ash_flag, set 3 m up).
    The load is spilled between the two."""
    v = Vol("ruin")
    # -- the slewed wagon: bed x -13..-7, z -8..-6, tongue east -------------
    z0, z1 = -8, -6
    for x in range(-13, -6):
        sag = x <= -12              # the back two rows rest on the ground
        y = 0 if sag else 1
        for z in range(z0, z1 + 1):
            v.set(x, y, z, "planks")
        v.set(x, y + 1, z0, "dark_planks")
        v.set(x, y + 1, z1, "dark_planks")
        if x >= -10:
            # canvas over the front half: a flat-topped hood, continuous runs
            for z in (z0, z1):
                v.set(x, y + 2, z, "white")
                v.set(x, y + 3, z, "white")
            for z in range(z0, z1 + 1):
                v.set(x, y + 4, z, "white")
        elif x in (-13, -11):
            # bare hoops where the canvas was torn away
            for z in (z0, z1):
                v.set(x, y + 2, z, "timber_beam")
                v.set(x, y + 3, z, "timber_beam")
            for z in range(z0, z1 + 1):
                v.set(x, y + 4, z, "timber_beam")
    # front wheels standing; the rear ones gone
    for wz in (z0 - 1, z1 + 1):
        wheel_upright(v, -8, 0, wz)
    # the tongue, east
    v.set(-6, 1, -7, "timber_beam")
    v.set(-5, 1, -7, "timber_beam")
    v.set(-4, 0, -7, "timber_beam")
    # a rear wheel lying flat beside the sagging end
    wheel_flat(v, -12, 0, -11)

    # -- the flipped wagon: x 4..10, z -10..-8, on its back ------------------
    for x in range(4, 11):
        for z in (-10, -9, -8):
            torn = x in (8, 9) and z == -8
            if not torn:
                v.set(x, 0, z, "white")           # the crushed canvas
            v.set(x, 2, z, "planks")              # the bed, upside down
        v.set(x, 1, -10, "dark_planks")           # side boards
        v.set(x, 1, -8, "dark_planks")
        v.set(x, 1, -9, "white")
    for wx in (5, 9):
        for wz in (-10, -8):
            wheel_upright(v, wx, 3, wz)           # wheels in the air
    # the Ashcloak flag planted in the upturned bed (cloth to the south)
    # (the cloth flies above the wheels, so daylight shows between them)
    v.col(7, -9, 3, 9, "timber_beam")
    for y in range(6, 10):
        v.set(7, y, -8, "black")
        v.set(7, y, -7, "red")
    v.set(7, 10, -9, "lantern")
    # the tongue, snapped and thrown down
    for x in (11, 12, 13):
        v.set(x, 0, -9, "timber_beam")

    # -- the load, spilled between the wagons --------------------------------
    for (x, y, z, b) in ((-2, 0, -4, "crate"), (-2, 1, -4, "crate"), (-1, 0, -5, "barrel"),
                         (0, 0, -3, "crate"), (1, 0, -6, "hay_bale"), (2, 0, -5, "crate"),
                         (3, 0, -6, "barrel"), (-3, 0, -2, "hay_bale"), (12, 0, -6, "crate"),
                         (-1, 0, -8, "crate"), (0, 0, -9, "planks")):
        v.set(x, y, z, b)
    v.points["wreck"] = (0, 0, -4)
    return v


@recipe
def meadow_east_spill():
    """Goods spilled on the road verge: broken crates, a barrel on its side,
    a hay bale and a loose wagon wheel standing on its rim. 7 x 3 x 3."""
    v = Vol("prop")
    wheel_upright(v, -2, 0, 0)
    for (x, y, z, b) in ((1, 0, -1, "crate"), (1, 1, -1, "crate"), (2, 0, 0, "crate"),
                         (3, 0, 1, "barrel"), (0, 0, 1, "hay_bale"), (3, 0, -1, "planks")):
        v.set(x, y, z, b)
    return v


@recipe
def meadow_east_dropped():
    """Loot dropped by raiders in a hurry: a crate split on the grass, a
    barrel, a sack of hay. 3 x 2 x 2."""
    v = Vol("prop")
    for (x, y, z, b) in ((0, 0, 0, "crate"), (1, 0, 0, "planks"), (-1, 0, 1, "barrel"),
                         (0, 0, 1, "hay_bale"), (0, 1, 0, "crate")):
        v.set(x, y, z, b)
    return v


@recipe
def meadow_east_rut():
    """A wheel rut two wide, torn through the turf from the road edge to the
    slewed wagon. 2 x 14, flush with the ground."""
    v = Vol("prop")
    for x in (0, 1):
        for z in range(-7, 7):
            v.set(x, -1, z, "dirt")
    return v


@recipe
def meadow_east_lookout():
    """An Ashcloak lookout on the crest: a 3 x 3 timber platform 4 m up on
    four posts, braced by a ring beam, a ladder on its south face, a rail,
    and a black-and-red flag in one vertical plane up its north-east post.
    Its top is 10 m."""
    v = Vol("prop")
    for x in (-1, 1):
        for z in (-1, 1):
            v.col(x, z, 0, 4, "timber_beam")
    # ring beam at 2 m (whole blocks, face to face)
    for a in (-1, 0, 1):
        for (x, z) in ((a, -1), (a, 1), (-1, a), (1, a)):
            v.set(x, 2, z, "timber_beam")
    # deck
    for x in (-1, 0, 1):
        for z in (-1, 0, 1):
            v.set(x, 4, z, "planks")
    # rail on three sides, open over the ladder
    for a in (-1, 0, 1):
        v.set(a, 5, -1, "timber_beam")
        v.set(-1, 5, a, "timber_beam")
        v.set(1, 5, a, "timber_beam")
    # ladder up the south face
    for y in range(0, 4):
        v.set(0, y, 2, "timber_beam" if y % 2 == 0 else "planks")
    # the flag: pole up the north-east post, cloth to the east
    v.col(1, -1, 6, 9, "timber_beam")
    for y in range(6, 9):
        v.set(2, y, -1, "black")
        v.set(3, y, -1, "red")
    v.set(1, 10, -1, "lantern")
    v.set(-1, 6, 1, "lantern")
    return v


@recipe
def meadow_east_pen():
    """A small stock pen of post and rail, 7 x 6, with a 3 m gate on its east
    side, a haystack in the corner and a plank trough of water."""
    v = Vol("prop")
    fence(v, -3, -3, 3, 2, gate=[(3, -1), (3, 0), (3, 1)])
    for (x, y, z) in ((-2, 0, -2), (-1, 0, -2), (-2, 0, -1), (-2, 1, -2)):
        v.set(x, y, z, "hay_bale")
    for x in range(-1, 3):
        v.set(x, 0, 0, "planks")
        v.set(x, -1, 1, "planks")
    for x in (0, 1):
        v.set(x, 0, 1, "water")
    v.set(-1, 0, 1, "planks")
    v.set(2, 0, 1, "planks")
    return v


def _dry_wall(v, a0, a1, fixed, along_x, joins):
    """A dry-stone field wall, one block high and continuous, with a capped
    pier two high at each join so it reads as built segments."""
    for a in range(a0, a1 + 1):
        x, z = (a, fixed) if along_x else (fixed, a)
        v.set(x, 0, z, pick(x, 0, z, ["cobble", "stone", "cobble", "stone_bricks"], 21))
        if a in joins:
            v.set(x, 1, z, "stone_bricks")


def _hedge_run(v, a0, a1, fixed, along_x, joins, gate=()):
    """A hedgerow two high with timber posts at the joins."""
    for a in range(a0, a1 + 1):
        if a in gate:
            continue
        x, z = (a, fixed) if along_x else (fixed, a)
        if a in joins:
            v.col(x, z, 0, 1, "timber_beam")
        else:
            v.set(x, 0, z, "leaves")
            if rnd(x, 0, z, 23) % 5:
                v.set(x, 1, z, "leaves")


@recipe
def meadow_east_farm_bounds():
    """Eastfold Farm's closed boundary, 39 x 35: dry-stone walls on the north
    and west (uphill), hedgerows on the east and south, each side in three
    segments, and a 6 m field gate in the south hedge at x -8..-3 where the
    Eastfold trail comes in, hung between two timber gateposts. Placed at the
    farm's centre at pad level."""
    v = Vol("prop")
    X, Z = 19, 17
    _dry_wall(v, -X, X, -Z, True, joins={-X, -6, 6, X})       # north
    _dry_wall(v, -Z, Z, -X, False, joins={-Z, -6, 6, Z})      # west
    _hedge_run(v, -Z + 1, Z - 1, X, False, joins={-6, 6})    # east
    gate = set(range(-8, -2))
    _hedge_run(v, -X + 1, X - 1, Z, True, joins={-9, 6}, gate=gate)   # south
    v.col(X, Z, 0, 1, "stone_bricks")                         # south-east corner pier
    v.col(X, -Z, 0, 1, "stone_bricks")
    v.col(-X, Z, 0, 1, "stone_bricks")
    # gateposts with a lantern, and the field gate swung open against the hedge
    v.col(-9, Z, 0, 2, "timber_beam")
    v.col(-2, Z, 0, 2, "timber_beam")
    v.set(-2, 3, Z, "lantern")
    return v
