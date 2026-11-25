from build import recipe, R
from kit import *

# Hearthstead: the starting town (places hearthstead, gate_rocks and the
# hearthstead_* outskirts). Build with
#   python3 tools/stamps/build.py hearthstead_great_hearth ...
# Coordinates as in kit.py: x east, z south, y up, y = 0 the first layer
# above the ground, y = -1 the ground layer (paving).

import math

# ---------------------------------------------------------------- shared
# Copied verbatim from the world-detail brief's shared pieces (KEY set).

KEY = "hearthstead"   # <- your section key


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


# ---------------------------------------------------------------- helpers

def pave(v, x0, z0, x1, z1, mix=("cobble",) * 12 + ("mossy_cobble",), salt=0):
    """Paving in the ground layer (y = -1) over a box."""
    for x in range(min(x0, x1), max(x0, x1) + 1):
        for z in range(min(z0, z1), max(z0, z1) + 1):
            v.set(x, -1, z, pick(x, 0, z, list(mix), salt))


# ---------------------------------------------------------------- the Great Hearth

@recipe
def hearthstead_great_hearth():
    """The Great Hearth: the town's open fire at ground level, at the foot of
    a stepped stone monument. A plinth of three tiers (7x7, 5x5, 3x3) rises
    behind the fire (north of it, so the people round the plaza keep their
    ground and the fire faces the square); a fire bowl burns on the top tier
    3 m up; four pillars carry a stepped stone hood with a beacon 11 m up,
    and red flame banners hang from the front pillars. The hearth cell
    (0, 0, 0) stays in open air, nothing above it."""
    v = Vol("landmark")
    # the apron: flagstones round the monument
    for x in range(-6, 7):
        for z in range(-9, 3):
            if abs(x) == 6 and (z in (-9, 2)):
                continue
            v.set(x, -1, z, "flagstones")
    # the plinth: tiers centred 3 m north of the fire
    cz = -3
    for (half, y, block) in ((3, 0, "stone_bricks"), (2, 1, "stone_bricks"), (1, 2, "stone_bricks")):
        for x in range(-half, half + 1):
            for z in range(cz - half, cz + half + 1):
                v.set(x, y, z, block)
    # the bottom tier's edge in cobble, a kerb that reads as a step
    for x in range(-3, 4):
        for z in range(cz - 3, cz + 4):
            if max(abs(x), abs(z - cz)) == 3:
                v.set(x, 0, z, "cobble" if (x + z) % 3 else "stone_bricks")
    # the fire's niche in the front of the bottom tier: the hearth itself
    for x in (-1, 0, 1):
        v.set(x, 0, 0, "air")
    v.set(0, 0, 0, "hearth")
    v.points["hearth"] = (0, 0, 0)
    # the raised fire bowl on the top tier (3 m up)
    for x in (-1, 0, 1):
        for z in (cz - 1, cz, cz + 1):
            v.set(x, 3, z, "stone_bricks")
    v.set(0, 3, cz, "hearth")
    for (x, z) in ((-1, cz), (1, cz), (0, cz - 1), (0, cz + 1)):
        v.set(x, 4, z, "brazier")
    # four pillars on the middle tier's corners, and the stepped hood
    for (x, z) in ((-2, cz - 2), (2, cz - 2), (-2, cz + 2), (2, cz + 2)):
        v.col(x, z, 2, 7, "stone_bricks")
    ring = lambda r: [(x, z) for x in range(-r, r + 1) for z in range(cz - r, cz + r + 1)
                      if max(abs(x), abs(z - cz)) == r]
    for (x, z) in ring(2):
        v.set(x, 8, z, "stone_bricks")
        v.set(x, 9, z, "slate_roof")
    for (x, z) in ring(3):
        v.set(x, 8, z, "slate_roof")
    for (x, z) in ring(1) + [(0, cz)]:
        v.set(x, 10, z, "slate_roof")
    v.set(0, 10, cz, "stone_bricks")
    v.set(0, 11, cz, "stone_bricks")
    v.set(0, 12, cz, "brazier")
    # lanterns under the hood's corners, banners on the front pillars
    for (x, z) in ((-2, cz - 2), (2, cz - 2), (-2, cz + 2), (2, cz + 2)):
        v.set(x + (1 if x > 0 else -1), 6, z, "lantern")
    for x in (-2, 2):
        for y in range(3, 7):
            v.set(x, y, cz + 3, "red_banner")
    # benches either side, facing the square, and two lamps behind
    for x in (-6, 6):
        for z in (-5, -4, -3):
            v.set(x, 0, z, "planks")
    for x in (-5, 5):
        lamp_post(v, x, 0, -8, 4)
    return v


# ---------------------------------------------------------------- signs

@recipe
def hearthstead_inn_sign():
    """The Wayfarer's Rest's sign: a brown board with a yellow tankard,
    hung from an arm, and lanterns at the post and the arm's end."""
    v = Vol("prop")
    v.col(0, 0, 0, 4, "timber_beam")
    for x in range(1, 5):
        v.set(x, 4, 0, "timber_beam")
    for x in (1, 2, 3):
        for y in (2, 3):
            v.set(x, y, 0, "planks")
    # the tankard: a yellow column and its handle, framed in dark wood
    v.set(2, 2, 0, "yellow")
    v.set(2, 3, 0, "yellow")
    v.set(1, 1, 0, "dark_planks")
    v.set(2, 1, 0, "dark_planks")
    v.set(3, 1, 0, "dark_planks")
    v.set(4, 3, 0, "lantern")
    v.set(0, 5, 0, "lantern")
    v.set(-1, 0, 0, "barrel")
    return v


@recipe
def hearthstead_smithy_sign():
    """The smithy's sign: an anvil on a stone post, a red flame banner on an
    arm above it, a lantern on top."""
    v = Vol("prop")
    v.col(0, 0, 0, 1, "stone_bricks")
    v.set(0, 2, 0, "anvil")
    v.col(0, 0, 1, 4, "timber_beam")
    v.set(1, 4, 1, "timber_beam")
    v.set(2, 4, 1, "timber_beam")
    v.set(2, 3, 1, "red_banner")
    v.set(2, 2, 1, "red_banner")
    v.set(0, 5, 1, "lantern")
    return v


# ---------------------------------------------------------------- the smithy

@recipe
def hearthstead_smithy():
    """The smithy turned to the street: the stone forge house east, its
    open lean-to west (furnace, anvil, brazier, crates), and before it a
    cobbled yard with an outdoor anvil, a quench barrel, a brazier and a
    weapon rack."""
    v = house(-4, -4, 4, 4, 1, "slate", ridge_x=True, door=("s", 0), chimney=(4, -4))
    v.set(3, 0, -3, "furnace")
    v.set(2, 0, -3, "furnace")
    v.set(-3, 0, -3, "chest")
    v.set(3, 0, 1, "bed")
    v.set(0, 2, -3, "lantern")
    # an opening in the west wall into the lean-to
    for z in (-1, 0, 1):
        v.col(-4, z, 0, 2, "air")
    # the lean-to on the west side
    for z in range(-4, 5):
        for x in range(-9, -4):
            v.set(x, -1, z, "cobble")
            v.set(x, 3 if x > -8 else 2, z, "slate_roof")
    for (x, z) in ((-9, -4), (-9, 4)):
        v.col(x, z, 0, 1, "timber_beam")
    # no window boxes under the lean-to: Hollis works there
    for z in range(-4, 5):
        if v.get(-5, 0, z) == "flower_bed":
            del v.c[(-5, 0, z)]
    v.set(-8, 0, -2, "anvil")
    v.set(-6, 0, -3, "furnace")
    v.set(-7, 0, -3, "furnace")
    v.set(-8, 0, -3, "brazier")
    v.set(-6, 0, 3, "crate")
    v.set(-6, 1, 3, "crate")
    v.set(-7, 0, 3, "barrel")
    v.set(-5, 2, 0, "lantern")
    # the yard before it
    pave(v, -15, -5, -10, 5, salt=3)
    v.set(-11, 0, -2, "anvil")
    v.set(-11, 0, 0, "barrel")
    v.set(-12, 0, -4, "brazier")
    # a weapon rack: two posts and a rail
    v.col(-13, 3, 0, 1, "timber_beam")
    v.col(-13, 5, 0, 1, "timber_beam")
    v.set(-13, 1, 4, "timber_beam")
    v.set(-10, 0, 4, "crate")
    v.points["forge"] = (-7, 0, 0)
    return v


# ---------------------------------------------------------------- walls and gates

@recipe
def hearthstead_town_wall():
    """The town wall with the west opening widened to |z| <= 4 for the
    postern."""
    v = R["town_wall"]()
    v.c = {k: b for k, b in v.c.items() if not (k[0] < -40 and abs(k[2]) <= 4)}
    return v


@recipe
def hearthstead_postern():
    """The west postern: two 3x3 towers 7 m high with crenels, a timber
    lintel over a 9 m opening, a lantern under it, blue banners on both
    faces. The road runs through along z (place it rot 1 for an east-west
    road); the outer face is +z."""
    v = Vol("building")
    v.foundation = "stone_bricks"
    for side in (-1, 1):
        for x in range(5, 8):
            for z in range(-1, 2):
                for y in range(0, 7):
                    v.set(side * x, y, z, pick(x * side, y, z, MASONRY))
                if (x + z) % 2 == 0:
                    v.set(side * x, 7, z, "stone_bricks")
        v.col(side * 6, 2, 2, 5, "blue_banner")
        v.col(side * 6, -2, 2, 5, "blue_banner")
        v.set(side * 6, 7, 0, "lantern")
    for x in range(-4, 5):
        v.set(x, 5, 0, "timber_beam")
    v.set(-4, 4, 0, "timber_beam")
    v.set(4, 4, 0, "timber_beam")
    v.set(0, 4, 0, "lantern")
    v.points["through"] = (0, 0, 0)
    return v


@recipe
def hearthstead_gatehouse():
    """A main gate: two 5x5 towers 10 m high with crenels (11 m), a 2 m stone
    band over a 9 m opening with a timber beam under it, a lantern, braziers
    on the towers and two blue banners on each tower's outer face (-z). The
    road runs north-south through the middle."""
    v = Vol("building")
    v.foundation = "stone_bricks"
    for side in (-1, 1):
        for x in range(5, 10):
            for z in range(-2, 3):
                for y in range(0, 10):
                    v.set(side * x, y, z, pick(x * side, y, z, MASONRY))
                if (x in (5, 9) or abs(z) == 2) and (x + z) % 2 == 0:
                    v.set(side * x, 10, z, "stone_bricks")
        v.set(side * 7, 10, 0, "brazier")
        for x in (6, 8):
            v.col(side * x, -3, 4, 7, "blue_banner")
        # arrow slits
        v.col(side * 7, -2, 5, 6, "leaded_window")
        v.col(side * 7, 2, 5, 6, "leaded_window")
    for x in range(-4, 5):
        for z in (-1, 0, 1):
            v.set(x, 7, z, "stone_bricks")
            v.set(x, 8, z, "stone_bricks")
        v.set(x, 6, 0, "timber_beam")
        if x % 2 == 0:
            v.set(x, 9, -1, "stone_bricks")
            v.set(x, 9, 1, "stone_bricks")
    v.set(0, 5, 0, "lantern")
    v.points["through"] = (0, 0, 0)
    return v


# ---------------------------------------------------------------- the library

@recipe
def hearthstead_library():
    """The Lorehouse with a face of its own: an all-stone front (no timber),
    tall coloured windows, a bell-cote over the door gable, a flagstone
    forecourt with a lectern, book crates, blue banners and two lamps."""
    v = R["library"]()
    x0, z0, x1, z1 = -5, -6, 5, 6
    # the front all of stone
    for (x, y, z), b in list(v.c.items()):
        if z == z1 and b == "timber_beam" and y >= 0:
            v.c[(x, y, z)] = "stone_bricks"
    # tall coloured windows on the front
    # (coloured cloth panes set flat in the wall's plane, one column each)
    for x, cloth in ((-3, "blue"), (3, "red")):
        for y in range(1, 7):
            v.set(x, y, z1, cloth)
    # a cross-shaped window in the gable, yellow
    for (x, y) in ((0, 9), (-1, 10), (0, 10), (1, 10), (0, 11)):
        v.set(x, y, z1, "yellow")
    # the bell-cote on the gable's peak
    top = max(y for (x, y, z) in v.c if z == z1 and x == 0)
    for x in (-1, 0, 1):
        for z in (z1 - 1, z1):
            v.col(x, z, top, top + 2, "stone_bricks")
    for x in (-1, 1):
        for z in (z1 - 1, z1):
            v.col(x, z, top + 3, top + 4, "stone_bricks")
    v.set(0, top + 4, z1, "lantern")
    v.set(0, top + 4, z1 - 1, "lantern")
    for x in (-1, 0, 1):
        for z in (z1 - 1, z1):
            v.set(x, top + 5, z, "slate_roof")
    v.set(0, top + 6, z1, "slate_roof")
    v.set(0, top + 6, z1 - 1, "slate_roof")
    # the forecourt
    for x in range(-4, 5):
        for z in range(z1 + 1, z1 + 6):
            v.set(x, -1, z, "flagstones" if (x + z) % 4 else "stone_bricks")
    v.set(2, 0, z1 + 1, "blue_banner")
    v.set(-3, 0, z1 + 1, "blue_banner")
    v.set(2, 0, z1 + 3, "planks")
    v.set(2, 1, z1 + 3, "bookshelf")
    v.set(-3, 0, z1 + 3, "crate")
    v.set(-3, 1, z1 + 3, "crate")
    v.set(-2, 0, z1 + 3, "crate")
    for x in (-4, 4):
        lamp_post(v, x, 0, z1 + 5, 4)
    return v


# ---------------------------------------------------------------- props

@recipe
def hearthstead_goods():
    """Market goods on the cobbles: crates, barrels and sacks of hay."""
    v = Vol("prop")
    for (x, y, z, b) in ((0, 0, 0, "crate"), (1, 0, 0, "crate"), (0, 1, 0, "crate"), (0, 0, 1, "barrel"),
                         (1, 0, 1, "barrel"), (-1, 0, 1, "hay_bale"), (2, 0, 0, "barrel")):
        v.set(x, y, z, b)
    return v


@recipe
def hearthstead_lane():
    """A 3 m dirt lane, 12 m long (x), worn into the grass, with a few
    cobbles: back lanes and paths to doors."""
    v = Vol("prop")
    for x in range(-6, 6):
        for z in (-1, 0, 1):
            v.set(x, -1, z, "dirt")
    return v


@recipe
def hearthstead_yard():
    """A house's back yard on worn ground: a rail along the back, a
    woodpile, a water butt and a row of greens. 8 x 6."""
    v = Vol("prop")
    for x in range(-4, 4):
        for z in range(-3, 3):
            v.set(x, -1, z, "dirt")
    for x in range(-4, 4):
        v.set(x, 0, 2, "timber_beam")
    v.set(-4, 1, 2, "timber_beam")
    v.set(3, 1, 2, "timber_beam")
    v.set(2, 0, -1, "log")
    v.set(2, 0, 0, "log")
    v.set(2, 0, 1, "barrel")
    for x in (-2, -1, 0):
        v.set(x, 0, 0, "flower_bed")
    return v


@recipe
def hearthstead_training():
    """The Watch's training yard: two straw dummies on posts, a weapon rack
    and a crate, on trodden dirt. 7 x 5."""
    v = Vol("prop")
    for x in range(-3, 4):
        for z in range(-2, 3):
            v.set(x, -1, z, "dirt")
    for x in (-2, 1):
        v.col(x, 0, 0, 1, "timber_beam")
        v.set(x, 2, 0, "hay_bale")
        v.set(x, 3, 0, "hay_bale")
    v.col(3, -2, 0, 1, "timber_beam")
    v.col(3, 0, 0, 1, "timber_beam")
    v.set(3, 1, -1, "timber_beam")
    v.set(-3, 0, 2, "crate")
    v.set(-3, 0, -2, "barrel")
    return v


@recipe
def hearthstead_beer_garden():
    """The inn's garden: two tables with benches, a barrel table, lanterns on
    posts, on flagstones. 9 x 7."""
    v = Vol("prop")
    for x in range(-4, 5):
        for z in range(-3, 4):
            v.set(x, -1, z, "flagstones" if (x * 7 + z) % 6 else "cobble")
    for cx in (-2, 2):
        v.set(cx, 0, 0, "planks")
        v.set(cx, 0, -1, "planks")
    for cx in (-2, 2):
        for z in (0, -1):
            v.set(cx - 1, 0, z, "dark_planks")
            v.set(cx + 1, 0, z, "dark_planks")
    v.set(0, 0, 2, "barrel")
    v.set(0, 1, 2, "barrel")
    for (x, z) in ((-4, -3), (4, -3), (-4, 3), (4, 3)):
        lamp_post(v, x, 0, z, 3)
    return v


@recipe
def hearthstead_retaining():
    """A stone retaining wall on the town pad's crest, its
    face going down 4 m into the slope (on the downhill side it shows), a
    parapet course above the pad. 9 m long (x)."""
    v = Vol("prop")
    for x in range(-4, 5):
        for y in range(-5, 1):
            if y == 0 and rnd(x, 0, 0, 51) % 4 == 0:
                continue
            v.set(x, y, 0, pick(x, y, 0, ["stone_bricks", "stone_bricks", "cobble", "stone"], 52))
    return v


@recipe
def hearthstead_lych_gate():
    """A graveyard's roofed gate: two posts, a little stepped roof of
    slate over a 3 m way, a lantern."""
    v = Vol("prop")
    for x in (-2, 2):
        v.col(x, 0, 0, 2, "timber_beam")
    for x in range(-3, 4):
        v.set(x, 3, 0, "slate_roof")
        v.set(x, 3, -1, "slate_roof")
        v.set(x, 3, 1, "slate_roof")
    for x in range(-3, 4):
        v.set(x, 4, 0, "slate_roof")
    v.set(0, 2, 0, "lantern")
    return v


@recipe
def hearthstead_shelter():
    """A field shelter for the flock: a 3x2 lean-to of planks under thatch,
    a trough beside it."""
    v = Vol("prop")
    for x in range(-1, 2):
        v.set(x, 0, -1, "planks")
        v.set(x, 1, -1, "planks")
        v.set(x, 2, -1, "thatch")
        v.set(x, 2, 0, "thatch")
    for x in (-1, 1):
        v.col(x, 0, 0, 1, "timber_beam")
    v.set(0, 0, -1, "hay_bale")
    for x in range(-1, 2):
        v.set(x, 0, 2, "dark_planks")
    v.set(2, 0, 2, "barrel")
    return v


# Walls of plaster in timber frames under slate: the old quarters' roofs
# without the cost of all-stone walls (a runtime style, kit.py untouched).
STYLES["hearthstead_slate"] = dict(base="stone_bricks", wall="plaster", frame="timber_beam", roof="slate_roof", floor="planks")


@recipe
def hearthstead_cottage_slate():
    """A cottage of plaster and timber under slate (the Watch, lore and
    craft quarters)."""
    from build import furnish_home
    v = house(-5, -4, 5, 4, 1, "hearthstead_slate", ridge_x=True, door=("s", -1), chimney=(0, -4))
    furnish_home(v, -5, -4, 5, 4)
    return v


@recipe
def hearthstead_townhouse_slate():
    """A two-storey town house of plaster and timber under slate, its gable
    to the street."""
    from build import furnish_home
    v = house(-4, -5, 4, 5, 2, "hearthstead_slate", ridge_x=False, door=("s", -1), chimney=(0, -5))
    furnish_home(v, -4, -5, 4, 5)
    stairs(v, 3, 0, 3, 0, -1)
    v.set(-3, STOREY, -3, "bed")
    v.set(-3, STOREY, -2, "bed")
    v.set(3, STOREY, -4, "chest")
    v.set(0, STOREY + 2, 0, "lantern")
    v.set(2, 3, 6, "lantern")
    return v


# ---------------------------------------------------------------- the east quarter's lots
# Each lot is one house with what its people keep round it (a garden, a
# fence, a woodpile, a lean-to), so no two lots in the quarter repeat. Roofs
# cluster: red tile next to the inn, slate on the stone house, thatch toward
# the wall. The house is the stamp's base, so its doors, storeys and
# foundation stand as house() made them.

STYLES["hearthstead_brick"] = dict(base="cobble", wall="brick", frame="timber_beam", roof="red_roof_tiles", floor="planks")
STYLES["hearthstead_thatch"] = dict(base="cobble", wall="plaster", frame="timber_beam", roof="thatch", floor="planks")
STYLES["hearthstead_stone"] = dict(base="stone_bricks", wall="stone_bricks", frame="dark_planks", roof="slate_roof", floor="flagstones")


def _rail(v, x0, z0, x1, z1, gaps=()):
    """A post-and-rail fence along a straight run (x or z), posts every 3 m
    a metre higher, open at the cells in `gaps`."""
    pts = [(x, z0) for x in range(x0, x1 + 1)] if z0 == z1 else [(x0, z) for z in range(z0, z1 + 1)]
    for i, (x, z) in enumerate(pts):
        if (x, z) in gaps:
            continue
        v.set(x, 0, z, "timber_beam")
        if i % 3 == 0:
            v.set(x, 1, z, "timber_beam")


def _beds(v, x0, z0, x1, z1):
    """Kitchen-garden rows: flower beds on tilled dirt, a path of dirt
    between every pair of rows (rows run along x)."""
    for z in range(z0, z1 + 1):
        for x in range(x0, x1 + 1):
            v.set(x, -1, z, "dirt")
            if (z - z0) % 2 == 0:
                v.set(x, 0, z, "flower_bed")


def _woodpile(v, x, z, n=3):
    for i in range(n):
        v.set(x + i, 0, z, "log")
    v.set(x, 1, z, "log")


@recipe
def hearthstead_lot_brick():
    """A low brick cottage under red tiles, its door on the long side to
    the lane, with a fenced kitchen garden behind it and a woodpile by the
    gable. House 9 x 7."""
    from build import furnish_home
    v = house(-4, -3, 4, 3, 1, "hearthstead_brick", ridge_x=True, door=("s", -1), chimney=(3, -3))
    furnish_home(v, -4, -3, 4, 3)
    # the back garden (north), fenced, a gate on the east
    _beds(v, -3, -9, 3, -6)
    _rail(v, -5, -10, 5, -10)
    _rail(v, -5, -9, -5, -5)
    _rail(v, 5, -9, 5, -5, gaps={(5, -7), (5, -8)})
    _woodpile(v, 6, -1)
    v.set(6, 0, 1, "barrel")
    return v


@recipe
def hearthstead_lot_thatch():
    """A small square cottage of plaster under thatch, out toward the wall:
    a fenced dooryard in front with a bench and flower beds, a hay bale and
    a lean-to woodshed on its side. House 7 x 7."""
    from build import furnish_home
    v = house(-3, -3, 3, 3, 1, "hearthstead_thatch", ridge_x=False, door=("s", -1), chimney=(0, -3))
    furnish_home(v, -3, -3, 3, 3)
    # the dooryard: worn ground, a low rail with a gate in line with the door
    for x in range(-4, 5):
        for z in range(4, 8):
            if (x, z) not in ((-1, 4), (0, 4)):
                v.set(x, -1, z, "dirt" if (x + z) % 3 else "grass")
    _rail(v, -4, 8, 4, 8, gaps={(-1, 8), (0, 8)})
    _rail(v, -4, 5, -4, 7)
    _rail(v, 4, 5, 4, 7)
    v.set(2, 0, 6, "planks")
    v.set(3, 0, 6, "planks")
    v.set(-3, 0, 6, "flower_bed")
    v.set(-2, 0, 7, "flower_bed")
    # the lean-to on the east wall: posts, a thatch slab, logs under it
    for z in (-2, 1):
        v.col(5, z, 0, 1, "timber_beam")
    for z in range(-2, 2):
        v.set(4, 2, z, "thatch")
        v.set(5, 2, z, "thatch")
        v.set(5, 0, z, "log" if z < 1 else "hay_bale")
    return v


@recipe
def hearthstead_lot_longhouse():
    """A long, low house of dark planks under thatch along the wall, a
    store at one end: a cart and hay at the barn door, a water butt by the
    house door, a dirt yard before it. House 13 x 7."""
    from build import furnish_home
    v = house(-6, -3, 6, 3, 1, "plank", ridge_x=True, door=("s", 1), chimney=(-6, 0))
    furnish_home(v, -6, -3, 0, 3)
    for x in range(-6, 7):
        for z in range(4, 8):
            v.set(x, -1, z, "dirt" if rnd(x, 0, z, 61) % 5 else "cobble")
    # the store's wide door on the west end of the south wall
    for x in (-5, -4, -3):
        v.col(x, 3, 0, 2, "air")
    v.set(-4, 0, 5, "hay_bale")
    v.set(-5, 0, 5, "hay_bale")
    v.set(-5, 1, 5, "hay_bale")
    v.set(-3, 0, 6, "crate")
    v.set(4, 0, 4, "barrel")
    _woodpile(v, 7, -2)
    return v


@recipe
def hearthstead_lot_stone():
    """A tall square house of stone under slate, a merchant's: two storeys,
    a walled front garden on flagstones with a bench and a clipped hedge.
    House 9 x 9."""
    from build import furnish_home
    v = house(-4, -4, 4, 4, 2, "hearthstead_stone", ridge_x=True, door=("s", -1), chimney=(-4, 0))
    furnish_home(v, -4, -4, 4, 4)
    stairs(v, 3, 0, 2, 0, -1)
    v.set(0, STOREY + 2, 0, "lantern")
    for x in range(-5, 6):
        for z in range(5, 9):
            v.set(x, -1, z, "flagstones" if (x * 3 + z) % 5 else "cobble")
    for x in range(-5, 6):
        if x not in (-1, 0):
            v.set(x, 0, 9, "stone_bricks")
    for z in range(5, 9):
        v.set(-5, 0, z, "stone_bricks")
        v.set(5, 0, z, "stone_bricks")
    for x in (-4, -3, 3, 4):
        v.set(x, 0, 8, "leaves")
        if x in (-4, 4):
            v.set(x, 1, 8, "leaves")
    v.set(-3, 0, 6, "planks")
    v.set(-2, 0, 6, "planks")
    return v


@recipe
def hearthstead_lot_tall():
    """A narrow two-storey house of plaster under red tiles, its gable to
    the lane, a lean-to scullery under thatch at the back and a drying line
    of washing in the yard. House 7 x 9."""
    from build import furnish_home
    v = house(-3, -4, 3, 4, 2, "tile", ridge_x=False, door=("s", -1), chimney=(0, -4))
    furnish_home(v, -3, -4, 3, 4)
    stairs(v, 2, 0, 3, 0, -1)
    # the scullery on the north: low walls, a thatch slab
    for x in range(-2, 3):
        for z in (-7, -6, -5):
            if x in (-2, 2) or z == -7:
                v.col(x, z, 0, 1, "planks" if (x + z) % 4 else "timber_beam")
            v.set(x, 2, z, "thatch")
    v.set(0, 0, -6, "barrel")
    # the drying line: two posts, a white sheet hung flat between them
    v.col(-6, -3, 0, 2, "timber_beam")
    v.col(-6, 3, 0, 2, "timber_beam")
    for z in range(-2, 3):
        v.set(-6, 2, z, "white")
        if abs(z) < 2:
            v.set(-6, 1, z, "white")
    _woodpile(v, 4, 2, 2)
    return v


@recipe
def hearthstead_lane_bend():
    """A 3 m dirt lane that turns a corner: 8 m east from the anchor, then
    8 m south, the corner's inside cut in two steps."""
    v = Vol("prop")
    for x in range(-8, 2):
        for z in (-1, 0, 1):
            v.set(x, -1, z, "dirt")
    for z in range(-1, 9):
        for x in (-1, 0, 1):
            v.set(x, -1, z, "dirt")
    for (x, z) in ((-2, 2), (-3, 2), (-2, 3)):
        v.set(x, -1, z, "dirt")
    for (x, z) in ((-4, 1), (-6, -1), (0, 5), (1, 7)):
        v.set(x, -1, z, "cobble")
    return v


# ---------------------------------------------------------------- the plaza's rim

@recipe
def hearthstead_plaza_tree():
    """A little tree in a stone ring at the plaza's edge: a stepped ring of
    stone bricks a metre high round a bed of grass and flowers, the tree
    built in the stamp. 7 x 7."""
    v = Vol("prop")
    ring = [(x, z) for x in range(-3, 4) for z in range(-3, 4)
            if max(abs(x), abs(z)) == 3 and not (abs(x) == 3 and abs(z) == 3)]
    for (x, z) in ring:
        v.set(x, 0, z, "stone_bricks")
    for (x, z) in ((-3, -3), (3, -3), (-3, 3), (3, 3)):
        v.set(x, -1, z, "cobble")
    for x in range(-2, 3):
        for z in range(-2, 3):
            v.set(x, -1, z, "grass")
            if (x + z) % 2 and abs(x) + abs(z) > 1:
                v.set(x, 0, z, "flower_bed")
    # the tree (built here: the plaza is paved, and planted trees keep off
    # paving): a trunk with two boughs, a crown of leaves 4-8 m up, wide in
    # the middle, ragged at the edge, a few cells of blossom
    v.col(0, 0, 0, 5, "log")
    v.set(1, 4, 0, "log")
    v.set(-1, 5, 1, "log")
    for (y, r) in ((4, 2), (5, 3), (6, 3), (7, 2), (8, 1)):
        for x in range(-r, r + 1):
            for z in range(-r, r + 1):
                d = abs(x) + abs(z)
                if d > r + 1 or (d == r + 1 and rnd(x, y, z, 72) % 3 == 0):
                    continue
                if y == 4 and d < 2:
                    continue
                v.set(x, y, z, "blossom_leaves" if rnd(x, y, z, 71) % 5 == 0 else "leaves")
    return v


@recipe
def hearthstead_planter():
    """A long planter on the plaza: a kerb of stone bricks round a bed of
    flowers and two clipped shrubs, a bench along its front. 6 x 3."""
    v = Vol("prop")
    for x in range(-3, 3):
        for z in (-1, 0, 1):
            edge = x in (-3, 2) or z in (-1, 1)
            v.set(x, 0, z, "stone_bricks" if edge else "flower_bed")
    v.set(1, 1, 0, "leaves")
    for x in (-2, -1, 0, 1):
        v.set(x, 0, 2, "planks")
    return v


# ---------------------------------------------------------------- Hearth Farm's track

@recipe
def hearthstead_track():
    """A 6 m length of farm track (z), two wheel ruts of dirt with grass
    between: laid in short lengths, each fitted to the ground, it steps
    down a slope in terraces instead of cutting one long trench."""
    v = Vol("prop")
    for z in range(-3, 3):
        for x in (-1, 1):
            v.set(x, -1, z, "dirt")
        v.set(0, -1, z, "dirt" if rnd(0, 0, z, 81) % 3 == 0 else "grass")
    return v


@recipe
def hearthstead_farm_gate():
    """A farm gate on the track: two tall posts, a lantern on one, a
    rail swung open against the fence line."""
    v = Vol("prop")
    for x in (-2, 2):
        v.col(x, 0, 0, 2, "timber_beam")
    v.set(-2, 3, 0, "lantern")
    for z in (1, 2, 3):
        v.set(2, 1, z, "timber_beam")
    return v
