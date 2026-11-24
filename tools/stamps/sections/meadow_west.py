# West meadow and farms (meadow_west): Westfold and Southfold farms, the
# pastures, the west_trail stops and the waysides off the trail.
# Build with:  python3 tools/stamps/build.py meadow_west_<name> ...
from build import recipe, R
from kit import *

KEY = "meadow_west"   # <- your section key


# ---------------------------------------------------------------- shared pieces
# (copied verbatim from director/tools/shared_pieces.py)

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


# ---------------------------------------------------------------- field bounds

def _drywall(n):
    """A dry-stone field wall n cells long along x (centred), tumbled in
    places. Sunlit meadow stone only (cobble and stone, no mossy_cobble,
    which reads navy in the sun)."""
    v = Vol("prop")
    h0 = -(n // 2)
    for i in range(n):
        x = h0 + i
        h = 1 + (rnd(x, 0, 0, 9 + n) % 3 > 0)
        if 0 < i < n - 1 and rnd(x, 0, 0, 10 + n) % 9 == 0:
            h = 0
            v.set(x, 0, 1, "cobble")
        for y in range(h):
            v.set(x, y, 0, pick(x, y, 0, ["stone", "stone", "cobble", "stone"], n))
    # a capstone course at both ends so a run reads as built, not strewn
    for x in (h0, h0 + n - 1):
        v.col(x, 0, 0, 1, "stone")
    return v


def _hedgerow(n):
    """A length of hedgerow n cells along x, with a timber post at each end."""
    v = Vol("prop")
    h0 = -(n // 2)
    for i in range(1, n - 1):
        x = h0 + i
        v.set(x, 0, 0, "leaves")
        if rnd(x, 0, 0, 7 + n) % 4:
            v.set(x, 1, 0, "leaves")
        if rnd(x, 0, 0, 8 + n) % 5 == 0:
            v.set(x, 0, 1, "leaves")
    v.set(h0, 0, 0, "timber_beam")
    v.set(h0 + n - 1, 0, 0, "timber_beam")
    return v


def _rails(n):
    """A post-and-rail fence n cells along x: a rail of beams on the ground
    and a post standing a metre higher every 3 m."""
    v = Vol("prop")
    h0 = -(n // 2)
    for i in range(n):
        x = h0 + i
        v.set(x, 0, 0, "timber_beam")
        if i % 3 == 0 or i == n - 1:
            v.set(x, 1, 0, "timber_beam")
    return v


for _n in (9, 13, 17):
    _named(f"{KEY}_drywall_{_n}", lambda n=_n: _drywall(n))
    _named(f"{KEY}_hedgerow_{_n}", lambda n=_n: _hedgerow(n))
for _n in (7, 11):
    _named(f"{KEY}_rails_{_n}", lambda n=_n: _rails(n))


def _farm_gate(post):
    """A field gate on a farm lane: two 2 m posts 6 m apart (a 5 m clear
    opening, x -2..2) and the gate leaf swung open inward (+z) against the
    right post. Low, so a camera sees the farm through it."""
    v = Vol("prop")
    for x in (-3, 3):
        if post == "stone":
            v.col(x, 0, 0, 1, "stone_bricks")
            v.set(x, 2, 0, "cobble")
        else:
            v.col(x, 0, 0, 2, "timber_beam")
    # the leaf: a 3 x 2 timber panel hinged on the +x post, swung into the farm
    for z in (1, 2, 3):
        v.set(3, 0, z, "timber_beam")
        v.set(3, 1, z, "timber_beam")
    v.set(-3, 3 if post == "stone" else 3, 0, "lantern")
    return v


_named(f"{KEY}_gate_stone", lambda: _farm_gate("stone"))
_named(f"{KEY}_gate_timber", lambda: _farm_gate("timber"))


@recipe
def meadow_west_gap():
    """Widens a 2 m pen gate to 3 m: carves the fence cell (and its post)
    where it stands. Place it on the fence line beside the gate."""
    v = Vol("prop")
    v.col(0, 0, 0, 1, "air")
    return v


# ---------------------------------------------------------------- crops and yards

def _plot(kind):
    """A small fenced plot, 9 x 7: furrows of dirt and a crop, a scarecrow
    in the middle, a 2 m gap in the fence on the -z side."""
    v = Vol("prop")
    fence(v, -4, -3, 4, 3, gate=[(0, -3), (1, -3)])
    for x in range(-3, 4):
        for z in range(-2, 3):
            v.set(x, -1, z, "dirt")
            if kind == "barley":
                if z % 2 == 0 and rnd(x, 0, z, 31) % 3:
                    v.set(x, 0, z, "hay_bale")
            else:
                if x % 2 == 0:
                    v.set(x, 0, z, "flower_bed")
    # a scarecrow: a post with a cross-arm and a straw head
    v.col(0, 0, 0, 2, "timber_beam")
    v.set(-1, 2, 0, "timber_beam")
    v.set(1, 2, 0, "timber_beam")
    v.set(0, 3, 0, "hay_bale")
    return v


_named(f"{KEY}_plot_barley", lambda: _plot("barley"))
_named(f"{KEY}_plot_veg", lambda: _plot("veg"))


@recipe
def meadow_west_yard():
    """A worn farmyard: dirt with a few grass cells, 11 x 7, ragged at the
    edge. Lies in the ground (y = -1)."""
    v = Vol("prop")
    for x in range(-5, 6):
        for z in range(-3, 4):
            edge = abs(x) == 5 or abs(z) == 3
            if edge and rnd(x, 0, z, 41) % 2:
                continue
            v.set(x, -1, z, "grass" if rnd(x, 0, z, 42) % 5 == 0 else "dirt")
    return v


@recipe
def meadow_west_log_cart():
    """A cart loaded with felled logs, its shafts down."""
    v = Vol("prop")
    for x in range(-2, 2):
        for z in (0, 1):
            v.set(x, 1, z, "planks")
    for (x, z) in ((-2, -1), (1, -1), (-2, 2), (1, 2)):
        v.set(x, 0, z, "dark_planks")
    for x in range(-2, 2):
        v.set(x, 2, 0, "log")
        v.set(x, 2, 1, "log")
    v.set(-1, 3, 0, "log")
    v.set(0, 3, 0, "log")
    v.set(2, 1, 0, "timber_beam")
    v.set(3, 0, 0, "timber_beam")
    return v


@recipe
def meadow_west_chopping():
    """A chopping block with the axe in it and split wood round it."""
    v = Vol("prop")
    v.set(0, 0, 0, "log")
    v.set(0, 1, 0, "timber_beam")      # the axe haft
    v.set(1, 0, 1, "birch_log")
    v.set(-1, 0, 1, "birch_log")
    v.set(2, 0, 0, "crate")
    return v


# ---------------------------------------------------------------- the glade

@recipe
def meadow_west_bed():
    """A wild flower bed for Bluebell Glade: an irregular 5 x 3 drift of
    flower_bed with no posts, stepped at the ends."""
    v = Vol("prop")
    for x in range(-2, 3):
        for z in range(-1, 2):
            if abs(x) == 2 and z != 0 and rnd(x, 0, z, 51) % 2:
                continue
            v.set(x, 0, z, "flower_bed")
    v.set(3, 0, 0, "flower_bed")
    return v


@recipe
def meadow_west_log_seat():
    """A fallen trunk dragged under the oak for a seat, a flat stone beside it."""
    v = Vol("prop")
    for x in range(-3, 3):
        v.set(x, 0, 0, "log")
    v.set(3, 0, 0, "stone")
    v.set(-1, 0, 2, "stone")
    v.set(0, 0, 2, "planks")
    return v


# ---------------------------------------------------------------- the shepherd

@recipe
def meadow_west_sheepfold():
    """A shepherd's round fold: a dry-stone ring (a stepped circle of radius
    4, every cell joined face to face) a metre high with capstones, a 3 m
    gap on the +z side, and a hay rack inside."""
    v = Vol("prop")
    ring = set()
    for x in range(-5, 6):
        for z in range(-5, 6):
            d = (x * x + z * z) ** 0.5
            if 3.5 <= d <= 4.7:
                ring.add((x, z))
    for (x, z) in sorted(ring):
        if z >= 3 and abs(x) <= 1:
            continue  # the gap
        v.set(x, 0, z, pick(x, 0, z, ["cobble", "stone", "cobble"], 61))
        if rnd(x, 0, z, 62) % 3:
            v.set(x, 1, z, pick(x, 1, z, ["cobble", "stone"], 63))
    v.set(-1, 0, -2, "hay_bale")
    v.set(0, 0, -2, "hay_bale")
    v.set(0, 1, -2, "hay_bale")
    v.set(1, 0, -2, "barrel")
    return v


# ---------------------------------------------------------------- outbuildings

@recipe
def meadow_west_byre():
    """Westfold's byre: a 7 x 11 plank cow-house under thatch, the second
    roofline behind the farmhouse (smaller than the stock barn, so the farm
    stays inside its vertex share). Door on +z, hay inside."""
    v = house(-3, -5, 3, 5, 1, "plank", ridge_x=False, door=("s", 0), windows=False)
    for (x, z) in ((-2, -4), (-1, -4), (-2, -3), (2, -4)):
        v.set(x, 0, z, "hay_bale")
    v.set(-2, 1, -4, "hay_bale")
    v.set(2, 0, 3, "barrel")
    v.set(0, 2, 0, "lantern")
    return v
