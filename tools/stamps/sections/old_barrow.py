# The Old Barrow and Barrowwood (world-detail pass). Build with:
#   python3 tools/stamps/build.py old_barrow_<name> ...
from build import recipe, R
from kit import *
import math

# ---------------------------------------------------------------- shared pieces
# Copied verbatim from the director's shared_pieces.py (only KEY changed), so
# the same piece looks the same everywhere.

KEY = "old_barrow"   # <- your section key


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


# ---------------------------------------------------------------- the barrow
# Coordinates of the pieces placed at the barrow's own anchor ([0, 0] at
# old_barrow, default pad fit) are the baked old_barrow stamp's: the passage
# runs east, the shed-like porch is x 7..13, z -2..2, its mouth at x 13 and
# the stamp's "door" point at (14, 4, 0). The barrow_crypt camera stands at
# (17.5, -2.5) with its eye at (19, -11): nothing over half a metre within
# 4 m of either, so the porch is re-faced in place rather than built out.


@recipe
def old_barrow_crown():
    """The barrow's crest marker, seen over the canopy: a stone_bricks
    trilithon facing the trail (east), a fire on a stepped plinth framed in
    its gate. Placed at [0, 0] with dy 8, its floor slab (y = -1) lies on the
    mound's top course (barrow y 7). 3 x 9 x 7."""
    v = Vol("landmark")
    # the bedding slab on the crest, so nothing stands on air
    for x in range(-1, 2):
        for z in range(-3, 4):
            v.set(x, -1, z, "mossy_cobble" if (x + z) % 3 == 0 else "stone_bricks")
    # two 2 x 1 posts with a 3 m gate between them, 7 m tall
    for z in (-3, -2, 2, 3):
        for y in range(0, 7):
            v.set(0, y, z, "mossy_cobble" if y < 2 and z in (-3, 3) else "stone_bricks")
    # the lintel, overhanging the posts by a metre each side
    for z in range(-4, 5):
        v.set(0, 7, z, "stone_bricks")
    # a stepped plinth in the gate and the fire on it
    for z in (-1, 0, 1):
        v.set(-1, 0, z, "stone_bricks")
        v.set(0, 0, z, "stone_bricks")
        v.set(1, 0, z, "stone_bricks")
    v.col(0, 0, 1, 2, "stone_bricks")
    v.set(0, 3, 0, "brazier")
    # lanterns hung under the lintel ends
    v.set(0, 6, -4, "lantern")
    v.set(0, 6, 4, "lantern")
    return v


@recipe
def old_barrow_dromos():
    """A façade of standing stones across the barrow's east face, the porch
    re-cut as a 3 m door under a 2 m lintel, braziers and the hung lantern.
    Placed at [0, 0] (the barrow's anchor) on the pad, after the baked stamp.
    The approach stays at grade: the threshold and the forecourt are made
    flush (the stray earth heap in front of the door goes)."""
    v = Vol("ruin")
    # -- the door: 3 wide (z -1..1), 3 clear (y 0..2), open from x 12 out
    for x in range(12, 18):
        for z in (-1, 0, 1):
            v.set(x, -1, z, "flagstones" if x < 16 else "dirt")
            for y in range(0, 3):
                v.set(x, y, z, "air")
    # the porch floor's hole at its mouth is filled; the steps down start
    # inside, where the passage was
    v.set(11, -1, 0, "flagstones")
    # jambs and the 2 m lintel over the door (x 12..13)
    for x in (12, 13):
        for z in (-2, 2):
            for y in range(0, 3):
                v.set(x, y, z, "stone_bricks" if y else "mossy_cobble")
        for z in range(-2, 3):
            v.set(x, 3, z, "stone_bricks")
            v.set(x, 4, z, "mossy_cobble" if z in (-2, 2) else "stone_bricks")
    # the lantern hung from the lintel's face (the baked one, kept) and the
    # door point's column left open above it
    v.set(14, 3, 0, "lantern")
    v.set(14, 4, 0, "air")
    # the porch roof is turfed into the mound so it reads as a passage
    for x in range(7, 12):
        for z in range(-2, 3):
            v.set(x, 3, z, "dirt" if x < 10 and abs(z) < 2 else "grass")
    for x in range(7, 10):
        for z in range(-1, 2):
            v.set(x, 4, z, "grass")
    # -- the façade: orthostats either side, 4 m and more off the axis,
    # stepping down outward; the horns of the mound are built up behind them
    north = [(-4, 5), (-5, 5), (-6, 4), (-7, 4), (-8, 3)]
    south = [(4, 5), (5, 5), (6, 4), (7, 4), (8, 3)]
    for z, h in north + south:
        for y in range(0, h):
            v.set(13, y, z, "mossy_cobble" if (y + z) % 3 == 0 or y == 0 else "stone")
        v.set(13, h - 1, z, "mossy_cobble")
    # dry-stone between the jambs and the first orthostats
    for z in (-3, 3):
        for y in range(0, 4):
            v.set(13, y, z, pick(13, y, z, ["cobble", "mossy_cobble", "stone_bricks"]))
        for y in range(0, 4):
            v.set(12, y, z, "cobble")
    # the horns: earth stepped up behind the stones to the mound
    for z in list(range(-8, -2)) + list(range(3, 9)):
        top = 4 if abs(z) <= 5 else 3 if abs(z) <= 7 else 2
        for x in range(9, 13):
            h = top if x >= 11 else top - 1
            for y in range(0, h):
                if v.get(x, y, z) is None:
                    v.set(x, y, z, "cobble")
            if v.get(x, h, z) is None:
                v.set(x, h, z, "grass")
    # braziers on plinths at the ends of the façade
    for (x, z) in ((15, -7), (15, 7)):
        v.set(x, -1, z, "cobble")
        v.set(x, 0, z, "stone_bricks")
        v.set(x, 1, z, "brazier")
    return v


@recipe
def old_barrow_vigil():
    """Brother Calder's and Anwen's vigil kit by the tent: lamp-oil barrels,
    a crate of candles, a lectern of books under a lantern on a post, a
    bedroll. 4 x 3, at the tent's side."""
    v = Vol("prop")
    v.set(0, 0, 0, "barrel")
    v.set(1, 0, 0, "barrel")
    v.set(0, 1, 0, "barrel")
    v.set(0, 0, 1, "crate")
    v.set(2, 0, 2, "bookshelf")
    v.col(3, 2, 0, 2, "timber_beam")
    v.set(3, 3, 2, "lantern")
    v.set(-1, 0, 2, "bed")
    return v


# ---------------------------------------------------------------- the graves


@recipe
def old_barrow_graves():
    """A grave plot: two rows of headstones (stone_bricks, 2 m, with a mossy
    shoulder), a dirt bed before each, ledger slabs, one open grave, and a
    broken boundary wall along the back (north) with a gap. 13 x 9."""
    v = Vol("ruin")
    stones = [(-5, -2), (-2, -2), (1, -2), (4, -2), (-4, 2), (2, 2), (5, 2)]
    for i, (x, z) in enumerate(stones):
        v.set(x, 0, z, "stone_bricks")
        v.set(x, 1, z, "mossy_cobble" if i % 3 == 1 else "stone_bricks")
        for dz in (1, 2):
            v.set(x, -1, z + dz, "dirt")
    # ledger slabs (a flat stone over the bed)
    for (x, z) in ((-1, 2), (-1, 3)):
        v.set(x, 0, z, "stone_bricks")
    v.set(-1, 0, 1, "mossy_cobble")
    # the open grave and its spoil
    for z in (0, 1):
        v.set(4, -1, z, "air")
        v.set(4, -2, z, "air")
        v.set(4, -3, z, "dirt")
    v.set(5, 0, 0, "dirt")
    # the broken boundary along the back
    for x in range(-6, 7):
        if x in (-1, 0, 1):
            continue   # the gap
        h = 2 if x in (-6, -5, 4, 6) else 1 if x not in (2, 3) else 0
        for y in range(h):
            v.set(x, y, -4, "mossy_cobble" if (x + y) % 2 else "stone_bricks")
    v.set(-6, 0, -3, "mossy_cobble")
    v.set(-6, 0, -2, "mossy_cobble")
    v.set(6, 0, -3, "cobble")
    # a grave candle
    v.set(-5, 0, 0, "lantern")
    return v


@recipe
def old_barrow_tomb():
    """The family tomb, the graves' focal point: a chest tomb of
    stone_bricks (3 x 2 x 2) on a flagstone step, a mossy lid, a lantern at
    its head and flower beds at its foot. 5 x 4."""
    v = Vol("ruin")
    for x in range(-2, 3):
        for z in range(-1, 3):
            v.set(x, -1, z, "flagstones")
    for x in range(-1, 2):
        for z in range(0, 2):
            v.set(x, 0, z, "stone_bricks")
            v.set(x, 1, z, "mossy_cobble" if x == 0 else "stone_bricks")
    v.set(0, 2, 0, "stone_bricks")
    v.set(0, 3, 0, "lantern")
    v.set(-2, 0, 2, "flower_bed")
    v.set(2, 0, 2, "flower_bed")
    return v


@recipe
def old_barrow_gatepost():
    """A lych post where the trail enters the graves: a 3 m stone_bricks
    pier on a mossy footing with a lantern on top. 1 x 1. Two of them make
    the gate, set clear of the trail either side; no lintel crosses it."""
    v = Vol("ruin")
    v.set(0, -1, 0, "mossy_cobble")
    v.set(0, 0, 0, "mossy_cobble")
    v.col(0, 0, 1, 2, "stone_bricks")
    v.set(0, 3, 0, "lantern")
    return v


# ---------------------------------------------------------------- waysides


@recipe
def old_barrow_cairn_tall():
    """The Barrowwood waymark: a 6 m stacked cairn, a 3 x 3 base, a stepped
    shaft with a lantern niche on the trail side, a stone_bricks capstone."""
    v = Vol("prop")
    for x in range(-1, 2):
        for z in range(-1, 2):
            for y in (0, 1):
                v.set(x, y, z, pick(x, y, z, ["cobble", "mossy_cobble", "cobble", "stone"]))
    for (x, z) in ((0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)):
        for y in (2, 3):
            v.set(x, y, z, pick(x, y, z, ["cobble", "mossy_cobble", "stone"], 3))
    v.set(0, 2, 1, "lantern")   # the niche faces +z (rotate it to the trail)
    v.set(0, 4, 0, "stone_bricks")
    v.set(0, 5, 0, "stone_bricks")
    for (x, z) in ((1, 0), (-1, 0)):
        v.set(x, 4, z, "stone_bricks")
    return v


@recipe
def old_barrow_mound():
    """A lesser barrow: a terraced grass mound 11 m across and 3 m high with
    mossy kerbstones round its foot and a blocked stone mouth on the east."""
    v = Vol("ruin")
    for x in range(-6, 7):
        for z in range(-6, 7):
            d = math.hypot(x, z)
            h = 3 if d <= 2.5 else 2 if d <= 4.0 else 1 if d <= 5.2 else 0
            for y in range(h):
                top = y == h - 1
                v.set(x, y, z, "grass" if top else "dirt" if y == h - 2 else "cobble")
            if 5.2 < d <= 6.2 and rnd(x, 0, z, 7) % 3 == 0:
                v.set(x, 0, z, "mossy_cobble")
    # the blocked mouth: two jambs and a cap slab, sealed
    for z in (-1, 1):
        v.col(5, z, 0, 1, "mossy_cobble")
    v.set(5, 0, 0, "stone")
    v.set(5, 1, 0, "stone")
    for z in (-1, 0, 1):
        v.set(5, 2, z, "stone_bricks")
    return v


@recipe
def old_barrow_menhirs():
    """A pair of leaning-free menhirs (2 x 1 slabs, 5 and 4 m) and a fallen
    third, a barrow-age alignment. 7 x 5."""
    v = Vol("ruin")
    for z in (0, 1):
        for y in range(5):
            v.set(-2, y, z, "mossy_cobble" if y == 0 or (y + z) % 3 == 0 else "stone")
        for y in range(4):
            v.set(2, y, z, "mossy_cobble" if y == 0 or (y + z) % 3 == 1 else "stone")
    for x in range(-1, 4):
        v.set(x, 0, 3, "mossy_cobble" if x % 2 else "stone")
        v.set(x, 0, 4, "stone")
    return v


# ---------------------------------------------------------------- beasts


@recipe
def old_barrow_wallow():
    """The thornboars' wallow: a churned mud hollow with a skin of water no
    more than a metre deep, rooted-up earth round it. 11 x 9."""
    v = Vol("prop")
    for x in range(-5, 6):
        for z in range(-4, 5):
            d = (x / 5) ** 2 + (z / 4) ** 2
            if d <= 0.55:
                v.set(x, -1, z, "water")
                v.set(x, -2, z, "clay")
            elif d <= 1.0:
                v.set(x, -1, z, "clay" if rnd(x, 0, z, 51) % 3 else "dirt")
            elif d <= 1.4:
                v.set(x, -1, z, "dirt")
                if rnd(x, 0, z, 52) % 7 == 0:
                    v.set(x, 0, z, "dirt")   # a rooted-up clod
    return v


@recipe
def old_barrow_bones():
    """A scatter of bones (white blocks) and an old cocoon on bare dirt, the
    floor of a websnare lair. 5 x 4."""
    v = Vol("prop")
    for x in range(-2, 3):
        for z in range(-1, 3):
            v.set(x, -1, z, "dirt")
    for (x, z) in ((-2, 0), (0, 1), (1, -1), (2, 2)):
        v.set(x, 0, z, "white")
    v.set(-1, 0, 2, "white")
    v.set(-1, 1, 2, "white")
    return v
