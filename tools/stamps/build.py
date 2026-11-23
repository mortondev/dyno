"""Recipes for the stamps of the Hearthblock pack. Run from the workspace:

    python3 tools/stamps/build.py [name ...]

Each recipe returns a Vol; its file is content/stamps/<name>.dstamp. The
stamps baked from the 0.8.0 world builders (the Hollow Mines, the Old
Barrow and its crypt, the Sunken Shrine, the wrecked caravan) have no
recipe: their files are the source.
"""
import math, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kit import *

R = {}


def recipe(f):
    R[f.__name__] = f
    return f


def furnish_home(v, x0, z0, x1, z1, hearth_side="n"):
    """A bed, a table and a fire for a one-room house."""
    cz = z0 + 1 if hearth_side == "n" else z1 - 1
    cx = (x0 + x1) // 2
    v.set(cx, 0, cz, "brazier")
    v.set(cx - 1, 0, cz, "stone_bricks")
    v.set(cx + 1, 0, cz, "stone_bricks")
    v.set(x0 + 1, 0, z0 + 3, "bed")
    v.set(x0 + 1, 0, z0 + 4, "bed")
    table(v, x1 - 2, 0, (z0 + z1) // 2)
    v.set(x1 - 1, 0, z0 + 1, "chest")
    v.set(x0 + 1, 2, z1 - 1, "lantern")


def cottage(style, w=4, d=5, ridge_x=False):
    v = house(-w, -d, w, d, 1, style, ridge_x=ridge_x, door=("s", -1), chimney=(0, -d))
    furnish_home(v, -w, -d, w, d)
    return v


@recipe
def cottage_tile():
    return cottage("tile")


@recipe
def cottage_thatch():
    return cottage("thatch", 4, 4)


@recipe
def cottage_slate():
    return cottage("slate", 5, 4, ridge_x=True)


@recipe
def farmhouse():
    v = house(-5, -4, 5, 4, 1, "thatch", ridge_x=True, door=("s", -1), chimney=(-5, 0))
    furnish_home(v, -5, -4, 5, 4)
    return v


@recipe
def inn():
    """The Wayfarer's Rest: two storeys under red tiles, a common room
    below, beds above."""
    x0, z0, x1, z1 = -7, -10, 7, 10
    v = house(x0, z0, x1, z1, 2, "tile", ridge_x=False, door=("s", -1), chimney=(0, z0))
    # the fire and the bar
    for x in (-1, 0, 1):
        v.col(x, z0 + 1, 0, 2, "stone_bricks")
    v.set(0, 0, z0 + 1, "brazier")
    v.set(0, 1, z0 + 1, "air")
    for z in range(z0 + 4, z0 + 9):
        v.set(x0 + 3, 0, z, "planks")
    v.set(x0 + 3, 0, z0 + 3, "barrel")
    v.set(x0 + 1, 0, z0 + 2, "barrel")
    v.set(x0 + 1, 1, z0 + 2, "barrel")
    v.set(x0 + 1, 0, z0 + 3, "barrel")
    v.points["bar"] = (x0 + 2, 0, z0 + 6)
    for (x, z) in ((3, -4), (3, 1), (-2, 5), (4, 6)):
        table(v, x, 0, z)
        v.set(x, 1, z, "torch") if (x + z) % 2 else None
    for (x, z) in ((x0 + 1, 3), (x1 - 1, -7), (x1 - 1, 4), (x0 + 1, 8)):
        v.set(x, 2, z, "lantern")
    # stairs along the east wall, and the floor above
    stairs(v, x1 - 1, 0, z1 - 2, 0, -1)
    v.points["upstairs"] = (x1 - 1, STOREY, z1 - 7)
    for z in (z0 + 2, z0 + 6, z0 + 10, z0 + 14):
        v.set(x0 + 1, STOREY, z, "bed")
        v.set(x0 + 2, STOREY, z, "bed")
        v.set(x0 + 1, STOREY, z + 1, "chest")
    v.set(0, STOREY + 2, 0, "lantern")
    # the sign and a bench outside
    v.col(3, z1 + 2, 0, 3, "timber_beam")
    v.set(3, 3, z1 + 1, "red_banner")
    v.set(-4, 0, z1 + 1, "planks")
    v.set(-5, 0, z1 + 1, "planks")
    return v


@recipe
def smithy():
    """A stone forge with an open lean-to: furnace, anvil, quench barrel."""
    v = house(-4, -4, 4, 4, 1, "slate", ridge_x=True, door=("s", 0), chimney=(-4, -4))
    v.set(-3, 0, -3, "furnace")
    v.set(-2, 0, -3, "furnace")
    v.set(3, 0, -3, "chest")
    v.set(3, 0, 0, "bed")
    v.set(0, 2, -3, "lantern")
    # the lean-to on the east side
    for z in range(-4, 5):
        for x in range(5, 10):
            v.set(x, -1, z, "flagstones")
            v.set(x, 3 if x < 8 else 2, z, "slate_roof")
    for (x, z) in ((9, -4), (9, 4), (9, 0)):
        v.col(x, z, 0, 1, "timber_beam")
    v.set(6, 0, -2, "anvil")
    v.set(7, 0, -3, "furnace")
    v.set(8, 0, -3, "brazier")
    v.set(6, 0, 2, "barrel")
    v.set(8, 0, 3, "crate")
    v.set(8, 1, 3, "crate")
    v.set(5, 2, 0, "lantern")
    v.points["forge"] = (7, 0, 0)
    return v


@recipe
def library():
    """The Lorehouse: two storeys of stone with shelves to the ceiling."""
    x0, z0, x1, z1 = -5, -6, 5, 6
    v = house(x0, z0, x1, z1, 2, "slate", ridge_x=False, door=("s", -1))
    for z in range(z0 + 2, z1 - 2, 3):
        for x in (x0 + 1, x1 - 1):
            v.col(x, z, 0, 2, "bookshelf")
            v.col(x, z, STOREY, STOREY + 2, "bookshelf")
    table(v, 0, 0, -2)
    v.set(0, 1, -2, "torch")
    stairs(v, x0 + 1, 0, z1 - 1, 1, 0)
    v.set(0, STOREY + 2, 0, "lantern")
    v.set(0, 2, 3, "lantern")
    v.set(2, 0, z1 + 1, "blue_banner")
    return v


@recipe
def barn():
    v = house(-5, -7, 5, 7, 1, "plank", ridge_x=False, door=("s", -1), windows=False, attic=False)
    # a hay loft over the back half
    v.fill((-4, 3, -6), (4, 3, -1), "planks")
    for (x, y, z) in ((-4, 0, -6), (-3, 0, -6), (-4, 1, -6), (4, 0, -5), (3, 0, -6), (4, 0, -6),
                      (-4, 4, -6), (-3, 4, -6), (-4, 4, -5), (2, 4, -6), (3, 4, -6)):
        v.set(x, y, z, "hay_bale")
    stairs(v, 4, 0, 3, 0, -1)
    v.set(-4, 0, 4, "barrel")
    v.set(-4, 0, 5, "crate")
    v.set(0, 2, 0, "lantern")
    return v


def tower(half, storeys, banner=None, beacon=True, style_wall=None):
    """A square stone tower with a crenellated top."""
    v = Vol("building")
    v.foundation = "stone_bricks"
    h = storeys * STOREY
    a = half
    for y in range(-1, h + 1):
        for x in range(-a, a + 1):
            for z in range(-a, a + 1):
                edge = abs(x) == a or abs(z) == a
                if y == -1:
                    v.set(x, y, z, "flagstones")
                elif edge:
                    v.set(x, y, z, pick(x, y, z, MASONRY))
                elif y % STOREY == STOREY - 1:
                    v.set(x, y, z, "planks")
                else:
                    v.set(x, y, z, "air")
    v.storeys = [k * STOREY for k in range(storeys + 1)]
    # arrow slits
    for k in range(1, storeys):
        for (x, z) in ((0, -a), (0, a), (-a, 0), (a, 0)):
            v.col(x, z, k * STOREY + 1, k * STOREY + 2, "leaded_window")
    # stairs round the walls, each flight on the next side
    flights = [((a - 1, a - 1), (0, -1)), ((a - 1, -a + 1), (-1, 0)), ((-a + 1, -a + 1), (0, 1)), ((-a + 1, a - 1), (1, 0))]
    for k in range(storeys):
        (x, z), (dx, dz) = flights[k % 4]
        stairs(v, x, k * STOREY, z, dx, dz, block="stone_bricks")
    # the door, south
    v.col(0, a, 0, DOOR - 1, "air")
    v.col(1, a, 0, DOOR - 1, "wooden_door")
    v.set(0, -1, a + 1, "flagstones")
    v.set(1, -1, a + 1, "flagstones")
    v.doors.append({"at": [0, 0, a], "height": DOOR})
    v.points["door"] = (0, 0, a + 2)
    # the fighting top: a parapet a block out, with merlons
    top = h
    for x in range(-a - 1, a + 2):
        for z in range(-a - 1, a + 2):
            edge = abs(x) == a + 1 or abs(z) == a + 1
            v.set(x, top - 1, z, "stone_bricks")
            if edge:
                v.set(x, top, z, "stone_bricks")
                if (x + z) % 2 == 0:
                    v.set(x, top + 1, z, "stone_bricks")
    v.points["top"] = (0, top, 0)
    v.points["floor"] = (0, 0, -1)
    if beacon:
        v.set(0, top, 0, "stone_bricks")
        v.set(0, top + 1, 0, "brazier")
        for (x, z) in ((-1, -1), (1, -1), (-1, 1), (1, 1)):
            v.col(x * (a - 1), z * (a - 1), top, top + 2, "timber_beam")
        for x in range(-a, a + 1):
            for z in range(-a, a + 1):
                d = max(abs(x), abs(z))
                if d == a:
                    v.set(x, top + 3, z, "slate_roof")
                elif d == a - 1:
                    v.set(x, top + 4, z, "slate_roof")
                elif d <= a - 2:
                    v.set(x, top + 5, z, "slate_roof")
        v.set(0, top + 6, 0, "lantern")
    if banner:
        for (x, z, dx, dz) in ((-1, a, 0, 1), (2, a, 0, 1)):
            for y in range(h - 7, h - 3):
                v.set(x + dx, y, z + dz, banner)
    v.set(a - 1, 2, 0, "lantern")
    return v


@recipe
def watch_tower():
    """The Watch's tower in Hearthstead: five storeys, seen from afar."""
    return tower(4, 5, banner="blue_banner")


@recipe
def north_watch():
    """The beacon tower on the north road."""
    v = tower(3, 4, banner="red_banner")
    return v


@recipe
def great_hearth():
    """The Great Hearth: a ring of cold stones in the middle of the plaza."""
    v = Vol("landmark")
    for x in range(-6, 7):
        for z in range(-6, 7):
            d = math.hypot(x, z)
            if d <= 6.3:
                v.set(x, -1, z, pick(x, 0, z, ["cobble", "flagstones", "mossy_cobble", "flagstones"]))
            if 2.6 <= d <= 3.4 and not (abs(x) <= 0 or abs(z) <= 0):
                v.set(x, 0, z, pick(x, 1, z, ["stone_bricks", "cobble", "mossy_cobble"]))
    v.set(0, 0, 0, "hearth")
    v.points["hearth"] = (0, 0, 0)
    # broken pillars of the old hall
    for i, (x, z) in enumerate(((-5, -5), (5, -5), (-5, 5), (5, 5))):
        h = (5, 2, 3, 6)[i]
        for y in range(h):
            v.set(x, y, z, "mossy_cobble" if y == h - 1 and i % 2 == 0 else "stone_bricks")
    for (x, z) in ((-2, 5), (-1, 5), (1, 5), (2, 5), (-2, -5), (-1, -5), (1, -5), (2, -5)):
        v.set(x, 0, z, "planks")
    return v


def stall(cloth, goods):
    v = Vol("prop")
    for (x, z) in ((-2, -1), (2, -1), (-2, 1), (2, 1)):
        v.col(x, z, 0, 2, "timber_beam")
    for x in range(-3, 4):
        for z in range(-2, 3):
            v.set(x, 3 if abs(z) < 2 else 2, z, cloth if x % 2 == 0 else "white")
    for i, x in enumerate((-1, 0, 1)):
        v.set(x, 0, 1, "planks")
        v.set(x, 1, 1, goods[i])
    v.set(-1, 0, -1, "crate")
    v.points["keeper"] = (0, 0, 0)
    return v


@recipe
def stall_red():
    return stall("red", ["hay_bale", "barrel", "crate"])


@recipe
def stall_blue():
    return stall("blue", ["crate", "flower_bed", "barrel"])


@recipe
def stall_yellow():
    return stall("yellow", ["barrel", "crate", "hay_bale"])


@recipe
def well():
    v = Vol("prop")
    for x in range(-1, 2):
        for z in range(-1, 2):
            if (x, z) == (0, 0):
                v.col(x, z, -4, -2, "water")
                v.set(x, -5, z, "cobble")
                v.set(x, -1, z, "air")
                v.set(x, 0, z, "air")
            else:
                v.col(x, z, -5, 0, pick(x, 0, z, ["cobble", "mossy_cobble", "stone_bricks"]))
    for x in (-1, 1):
        v.col(x, 0, 1, 2, "timber_beam")
    for x in range(-2, 3):
        for z in (-1, 0, 1):
            v.set(x, 3 if z == 0 else 2 + 0, z, "slate_roof") if abs(x) <= 1 or z == 0 else None
    v.set(0, 2, 0, "timber_beam")
    return v


@recipe
def gate():
    """Two gate towers and a timber arch over a main road (the road runs
    north-south through the middle)."""
    v = Vol("building")
    v.foundation = "stone_bricks"
    for side in (-1, 1):
        for x in range(5, 8):
            for z in range(-1, 2):
                for y in range(0, 7):
                    v.set(side * x, y, z, pick(x * side, y, z, MASONRY))
                if abs(z) == 1 or x != 6:
                    v.set(side * x, 7, z, "stone_bricks")
        v.set(side * 6, 7, 0, "brazier")
        v.col(side * 4, 0, 3, 5, "blue_banner")
    for x in range(-4, 5):
        v.set(x, 6, 0, "timber_beam")
    v.set(0, 5, 0, "lantern")
    v.points["through"] = (0, 0, 0)
    return v


@recipe
def lamp_post_():
    v = Vol("prop")
    lamp_post(v, 0, 0, 0, 4)
    return v


@recipe
def signpost():
    v = Vol("prop")
    v.col(0, 0, 0, 2, "timber_beam")
    v.set(1, 2, 0, "planks")
    v.set(-1, 2, 0, "planks")
    v.set(0, 3, 0, "lantern")
    return v


@recipe
def hut():
    v = house(-3, -3, 3, 3, 1, "log", ridge_x=True, door=("s", -1), windows=False)
    v.set(-2, 0, -2, "bed")
    v.set(2, 0, -2, "crate")
    v.set(0, 2, -2, "lantern")
    return v


@recipe
def fisher_hut():
    v = house(-3, -4, 3, 4, 1, "plank", ridge_x=False, door=("s", -1), windows=True)
    v.set(-2, 0, -3, "bed")
    v.set(2, 0, -3, "barrel")
    v.set(2, 0, -2, "barrel")
    v.set(0, 2, -3, "lantern")
    v.set(3, 0, 5, "barrel")
    return v


def ridge_tent(length, half, cloth):
    v = Vol("prop")
    for l in range(length):
        for c in range(-half, half + 1):
            top = half - abs(c)
            v.set(c, top, l, cloth[(l // 2) % len(cloth)])
            for y in range(0, top):
                # closed at the back, open at the front
                v.set(c, y, l, cloth[0] if l == 0 else "air")
    v.set(0, half + 1, 0, "timber_beam")
    v.set(0, half + 1, length - 1, "timber_beam")
    v.points["door"] = (0, 0, length)
    return v


@recipe
def tent_white():
    return ridge_tent(6, 3, ["white", "white", "yellow"])


@recipe
def tent_red():
    return ridge_tent(7, 3, ["red", "black"])


@recipe
def tent_blue():
    return ridge_tent(6, 3, ["blue", "white"])


@recipe
def campfire():
    v = Vol("prop")
    for (x, z) in ((-1, 0), (1, 0), (0, -1), (0, 1)):
        v.set(x, -1, z, "cobble")
    v.set(0, -1, 0, "cobble")
    v.set(0, 0, 0, "brazier")
    v.set(-2, 0, 1, "log")
    v.set(-2, 0, 0, "log")
    v.set(1, 0, 2, "log")
    v.set(2, 0, -1, "crate")
    return v


@recipe
def log_pile():
    v = Vol("prop")
    for x in range(-3, 4):
        for z, top in ((-1, 0), (0, 1), (1, 0)):
            for y in range(top + 1):
                v.set(x, y, z, "log")
    v.set(-4, 0, 0, "timber_beam")
    v.set(4, 0, 0, "timber_beam")
    return v


@recipe
def birch_pile():
    v = Vol("prop")
    for x in range(-2, 3):
        for z, top in ((0, 1), (1, 0)):
            for y in range(top + 1):
                v.set(x, y, z, "birch_log")
    return v


@recipe
def saw_pit():
    """A lean-to over a saw bench, with a stack of planks."""
    v = Vol("prop")
    for x in range(-3, 4):
        for z in range(-2, 3):
            v.set(x, 4 - (z + 2) // 2, z, "thatch")
    for (x, z) in ((-3, -2), (3, -2)):
        v.col(x, z, 0, 3, "timber_beam")
    for (x, z) in ((-3, 2), (3, 2)):
        v.col(x, z, 0, 1, "timber_beam")
    for x in range(-1, 2):
        v.set(x, 0, 0, "planks")
    v.set(2, 0, -1, "planks")
    v.set(2, 1, -1, "planks")
    v.set(-2, 0, 1, "crate")
    v.set(0, 2, -1, "lantern")
    return v


@recipe
def stumps():
    v = Vol("prop")
    for (x, z) in ((0, 0), (3, 2), (-2, 4), (5, -3), (-4, -2), (1, -5)):
        v.set(x, 0, z, "log")
    v.set(2, 0, -1, "birch_log")
    v.set(3, 0, -1, "birch_log")
    return v


@recipe
def windmill():
    """A stone mill tower with four cloth sails on its south face."""
    v = tower(4, 3, beacon=False)
    h = 3 * STOREY
    # a round-ish shingled cap in place of the fighting top
    v.drop((-6, h, -6), (6, h + 3, 6))
    for k in range(0, 5):
        a = 5 - k
        for x in range(-a, a + 1):
            for z in range(-a, a + 1):
                if max(abs(x), abs(z)) == a or k == 4:
                    v.set(x, h + k, z, "thatch")
    # the sails: a cross in the plane just south of the wall
    hub = (0, h - 2, 6)
    v.set(0, h - 2, 5, "timber_beam")
    v.set(*hub, "timber_beam")
    for (dx, dy) in ((1, 1), (-1, 1), (1, -1), (-1, -1)):
        for i in range(1, 8):
            v.set(dx * i, hub[1] + dy * i, 6, "timber_beam")
            if i >= 2:
                # cloth on the trailing side of each arm
                v.set(dx * i - dy, hub[1] + dy * i + dx, 6, "white")
                if i >= 3:
                    v.set(dx * i - 2 * dy, hub[1] + dy * i + 2 * dx, 6, "white")
    v.set(-3, 0, -3, "hay_bale")
    v.set(-3, 1, -3, "hay_bale")
    v.set(3, 0, -3, "barrel")
    v.set(-2, 0, 5, "hay_bale")
    v.points["sails"] = hub
    return v


@recipe
def field():
    """A fenced field of grain in rows."""
    v = Vol("prop")
    fence(v, -8, -6, 8, 6, gate=[(0, 6), (1, 6)])
    for x in range(-7, 8):
        for z in range(-5, 6):
            if x % 2 == 0:
                v.set(x, -1, z, "dirt")
                if rnd(x, 0, z) % 5 != 0:
                    v.set(x, 0, z, "hay_bale" if rnd(x, 1, z) % 9 == 0 else "flower_bed" if rnd(x, 2, z) % 17 == 0 else "skip")
    # sheaves standing in the stubble
    for (x, z) in ((-6, -4), (-2, 2), (4, -2), (6, 4), (0, -4), (-4, 4)):
        v.set(x, 0, z, "hay_bale")
    v.c = {k: b for k, b in v.c.items() if b != "skip"}
    return v


@recipe
def pasture():
    """A sheep field: a rail fence with a gate on the south side."""
    v = Vol("prop")
    fence(v, -9, -7, 9, 7, gate=[(0, 7), (1, 7)])
    v.set(-8, 0, -6, "hay_bale")
    v.set(-7, 0, -6, "hay_bale")
    v.set(7, 0, -6, "barrel")
    v.points["flock"] = (0, 0, 0)
    return v


@recipe
def haystack():
    v = Vol("prop")
    for (x, y, z) in ((0, 0, 0), (1, 0, 0), (0, 0, 1), (1, 0, 1), (0, 1, 0), (1, 1, 1), (-1, 0, 0), (0, 2, 0)):
        v.set(x, y, z, "hay_bale")
    return v


@recipe
def dock():
    """A pier of planks on posts, running south over the water."""
    v = Vol("prop")
    v.foundation = "timber_beam"
    for z in range(-2, 15):
        for x in range(-1, 2):
            v.set(x, -1, z, "planks")
    for z in range(10, 15):
        for x in range(-4, 5):
            v.set(x, -1, z, "planks")
    for (x, z) in ((-1, 2), (1, 2), (-1, 8), (1, 8), (-4, 10), (4, 10), (-4, 14), (4, 14)):
        v.col(x, z, -1, 1, "timber_beam")
    v.set(-4, 2, 14, "lantern")
    v.set(4, 2, 10, "lantern")
    v.set(3, 0, 13, "barrel")
    v.set(-3, 0, 11, "crate")
    v.set(-3, 0, 12, "crate")
    v.points["end"] = (0, 0, 13)
    return v


@recipe
def boat():
    v = Vol("prop")
    for z in range(-2, 3):
        for x in (-1, 0, 1):
            if abs(z) == 2 and x != 0:
                continue
            v.set(x, 0 if x == 0 else 1, z, "dark_planks")
        v.set(0, 1, z, "air") if abs(z) < 2 else v.set(0, 1, z, "dark_planks")
    return v


@recipe
def drying_rack():
    v = Vol("prop")
    for x in (-2, 2):
        v.col(x, 0, 0, 2, "timber_beam")
    for x in range(-2, 3):
        v.set(x, 3, 0, "timber_beam")
    for x in (-1, 0, 1):
        v.set(x, 2, 0, "white")
    return v


@recipe
def toll_hut():
    v = house(-2, -2, 2, 2, 1, "plank", ridge_x=True, door=("s", -1), windows=False)
    v.set(1, 0, -1, "chest")
    # the bar across the road
    v.col(4, 3, 0, 1, "timber_beam")
    for x in range(4, 9):
        v.set(x, 2, 3, "timber_beam")
    v.set(-3, 0, 3, "red_banner")
    return v


@recipe
def bridge():
    """Emberford Bridge: stone arches carrying a seven-metre road east to
    west. The anchor is the middle of the river, level with the banks."""
    v = Vol("building")
    v.foundation = "stone_bricks"
    L = 15
    for x in range(-L, L + 1):
        rise = 1 if abs(x) <= 9 else 0
        for z in range(-4, 5):
            edge = abs(z) == 4
            v.set(x, rise, z, pick(x, 0, z, ["flagstones", "cobble", "flagstones", "stone_bricks"]))
            if rise:
                v.set(x, 0, z, "stone_bricks")
            v.col(x, z, rise + 1, rise + 5, "air")
            if edge:
                v.set(x, rise + 1, z, pick(x, 1, z, MASONRY))
        # piers and the arch soffit
        if x % 6 == 0:
            for z in range(-4, 5):
                v.col(x, z, -6, -1, pick(x, 2, z, MASONRY))
        else:
            for z in range(-4, 5):
                v.set(x, -1, z, "stone_bricks") if abs(x % 6 - 3) >= 2 else None
    for x in (-L, -9, 0, 9, L):
        for z in (-4, 4):
            r = 1 if abs(x) <= 9 else 0
            v.col(x, z, r + 1, r + 3, "stone_bricks")
            v.set(x, r + 4, z, "lantern")
    v.points["middle"] = (0, 2, 0)
    v.points["west"] = (-L - 1, 1, 0)
    v.points["east"] = (L + 1, 1, 0)
    return v


@recipe
def miners_lodge():
    """Miner's Rest: a two-storey lodge of stone under slate."""
    x0, z0, x1, z1 = -6, -5, 6, 5
    v = house(x0, z0, x1, z1, 2, "slate", ridge_x=True, door=("s", -1), chimney=(x0, 0))
    v.set(x0 + 1, 0, 0, "brazier")
    for z in (-3, 3):
        table(v, 2, 0, z)
    v.set(x1 - 1, 0, z0 + 1, "barrel")
    v.set(x1 - 1, 1, z0 + 1, "barrel")
    v.set(x1 - 2, 0, z0 + 1, "crate")
    stairs(v, x1 - 1, 0, z1 - 1, -1, 0)
    for x in (x0 + 2, x0 + 5, x0 + 8):
        v.set(x, STOREY, z0 + 1, "bed")
        v.set(x, STOREY, z0 + 2, "bed")
    v.set(0, 2, 0, "lantern")
    v.set(0, STOREY + 2, 0, "lantern")
    v.set(3, 0, z1 + 1, "red_banner")
    return v


@recipe
def ore_cart():
    v = Vol("prop")
    for x in range(-1, 2):
        v.set(x, 0, 0, "dark_planks")
        v.set(x, 0, 1, "dark_planks")
    v.set(-1, 1, 0, "coal_ore")
    v.set(0, 1, 0, "iron_ore")
    v.set(0, 1, 1, "coal_ore")
    v.set(3, 0, 1, "barrel")
    v.set(3, 0, -1, "crate")
    return v


@recipe
def mine_head():
    """The headframe over the Hollow Mines' shaft: a timber tower with a
    winding wheel, and the cage hanging where the hoist stopped."""
    v = Vol("landmark")
    for (x, z) in ((-2, -2), (2, -2), (-2, 2), (2, 2)):
        v.col(x, z, 0, 13, "timber_beam")
    for y in (4, 9, 13):
        for i in range(-2, 3):
            for (x, z) in ((i, -2), (i, 2), (-2, i), (2, i)):
                v.set(x, y, z, "timber_beam")
    for i in range(-3, 4):
        v.set(i, 14, 0, "timber_beam")
        v.set(0, 14, i, "dark_planks")
    for (x, y) in ((-1, 15), (0, 16), (1, 15), (0, 15)):
        v.set(x, y, 0, "dark_planks")
    for y in range(5, 14):
        v.set(0, y, 0, "timber_beam" if y != 8 else "lantern")
    for x in range(-1, 2):
        for z in range(-1, 2):
            v.set(x, 4, z, "planks")
    v.fill((-2, -1, -2), (2, -1, 2), "planks")
    v.set(-3, 0, 1, "crate")
    v.set(3, 0, -1, "barrel")
    return v


@recipe
def mine_portal():
    """The timbered mouth of an adit, facing west; the tunnel runs east."""
    v = Vol("building")
    for x in range(0, 9):
        for z in range(-2, 3):
            for y in range(0, 4):
                edge = abs(z) == 2 or y == 3
                if edge:
                    if x % 3 == 0:
                        v.set(x, y, z, "timber_beam")
                else:
                    v.set(x, y, z, "air")
            v.set(x, -1, z, "planks" if abs(z) < 2 and x % 2 == 0 else "cobble")
    for z in range(-3, 4):
        for y in range(0, 6):
            if abs(z) == 3 or y >= 4:
                v.set(-1, y, z, pick(0, y, z, MASONRY))
    v.set(-1, 4, 0, "lantern")
    v.set(-2, 0, 3, "crate")
    v.set(-2, 0, -3, "barrel")
    v.doors.append({"at": [0, 0, 0], "height": 3})
    return v


@recipe
def stockade():
    """The Ashcloak Stockade: a palisade of sharpened logs, the gate to the
    south, tents and stolen stores inside."""
    v = Vol("building")
    a = 13
    for i in range(-a, a + 1):
        for (x, z) in ((i, -a), (i, a), (-a, i), (a, i)):
            if z == a and abs(x) <= 2:
                continue
            h = 5 if (x + z) % 2 == 0 else 4
            v.col(x, z, 0, h, "log")
    # gate posts and the walk over the gate
    for x in (-3, 3):
        v.col(x, a, 0, 7, "log")
        v.set(x, 8, a, "brazier")
    for x in range(-3, 4):
        v.set(x, 6, a, "timber_beam")
    v.col(-2, a + 1, 3, 5, "red_banner")
    v.col(2, a + 1, 3, 5, "red_banner")
    # corner platforms
    for (sx, sz) in ((-1, -1), (1, -1)):
        for x in range(a - 3, a):
            for z in range(a - 3, a):
                v.set(sx * x, 3, sz * z, "dark_planks")
        v.col(sx * (a - 3), sz * (a - 3), 0, 2, "log")
    # the leader's tent, north; two more; the stores under a lean-to
    v.merge(ridge_tent(8, 4, ["black", "red"]), at=(0, 0, -11))
    v.merge(ridge_tent(6, 3, ["red", "black"]), at=(-8, 0, -6), rot=3)
    v.merge(ridge_tent(6, 3, ["black", "black", "red"]), at=(8, 0, -6), rot=1)
    for x in range(5, 12):
        for z in range(5, 10):
            v.set(x, 4 - (z - 5) // 2, z, "thatch")
    for (x, z) in ((5, 5), (11, 5)):
        v.col(x, z, 0, 3, "timber_beam")
    for (x, y, z) in ((6, 0, 7), (7, 0, 7), (6, 1, 7), (8, 0, 8), (9, 0, 8), (10, 0, 7), (10, 1, 7), (9, 0, 6), (7, 0, 9)):
        v.set(x, y, z, pick(x, y, z, ["crate", "barrel", "crate", "hay_bale"]))
    v.points["stores"] = (8, 0, 4)
    v.set(0, -1, 0, "cobble")
    v.set(0, 0, 0, "brazier")
    v.points["yard"] = (0, 0, 2)
    v.points["gate"] = (0, 0, a + 3)
    for (x, z) in ((-6, 4), (-7, 5), (-9, 9)):
        v.set(x, 0, z, "log")
    v.set(-10, 0, 8, "crate")
    return v


def blob(v, cx, cy, cz, rx, ry, block, salt=0, only_air=True):
    for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
        for z in range(int(cz - rx) - 1, int(cz + rx) + 2):
            for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
                d = ((x - cx) ** 2 + (z - cz) ** 2) / (rx * rx) + ((y - cy) / ry) ** 2
                if d <= 0.8 + 0.3 * (rnd(x, y, z, salt) % 100) / 100:
                    if not only_air or v.get(x, y, z) is None:
                        v.set(x, y, z, block)


@recipe
def hollow_oak():
    """The Hollow Oak: a giant of the old wood, 26 m tall, its trunk
    rotted open and hung with webs."""
    v = Vol("landmark")
    H = 26
    for y in range(0, H - 6):
        r = 3.4 if y < 3 else 2.9 if y < 12 else 2.3
        for x in range(-4, 5):
            for z in range(-4, 5):
                d = math.hypot(x, z)
                if d <= r and (d > r - 1.6 or y > 11):
                    v.set(x, y, z, "log")
                elif d <= r and y <= 11:
                    v.set(x, y, z, "air")
    # roots
    for i, (dx, dz) in enumerate(((1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1))):
        n = 5 if i < 4 else 4
        for k in range(3, n + 1):
            v.col(dx * k if abs(dx) else 0, dz * k if abs(dz) else 0, 0, max(0, 2 - (k - 3)), "log")
    # the rotted doorway, south
    for x in (-1, 0, 1):
        v.col(x, 3, 0, 3 if x == 0 else 2, "air")
        v.col(x, 2, 0, 3 if x == 0 else 2, "air")
    v.points["hollow"] = (0, 0, 0)
    v.points["door"] = (0, 0, 5)
    v.fill((-1, -1, -1), (1, -1, 1), "dirt")
    # limbs and the crown
    limbs = [(1, 0.3), (-1, 0.5), (0.4, -1), (-0.5, 1), (0.9, 0.9), (-0.9, -0.8)]
    for i, (dx, dz) in enumerate(limbs):
        n = math.hypot(dx, dz)
        y0 = 13 + (i % 3) * 2
        for s in range(2, 8):
            v.set(round(dx / n * s), y0 + s // 2, round(dz / n * s), "log")
        blob(v, dx / n * 6.5, y0 + 5, dz / n * 6.5, 3.6, 2.8, "leaves", i)
    blob(v, 0, H - 4, 0, 6.5, 3.6, "leaves", 9)
    blob(v, 0, H - 8, 0, 8.4, 3.0, "leaves", 10)
    # webs in the hollow and under the limbs
    for (x, y, z) in ((1, 6, 1), (-1, 8, 0), (0, 10, -1), (1, 9, -1), (-3, 12, 3), (4, 13, -2), (-4, 14, -3), (2, 4, 4), (-2, 3, 4), (3, 12, 4)):
        v.set(x, y, z, "white")
    for (x, z) in ((5, 2), (-5, -1), (2, -5), (-3, 5), (6, -4)):
        v.set(x, 0, z, "glowshroom")
    return v


@recipe
def graves():
    """A grave cluster: leaning stones, sunken beds, an open pit."""
    v = Vol("ruin")
    for i, (x, z) in enumerate(((-4, -3), (-1, -3), (2, -3), (5, -3), (-4, 1), (-1, 1), (5, 1), (-3, 4), (1, 4))):
        h = 1 + (i % 3 == 0)
        v.col(x, z, 0, h - 1, "mossy_cobble" if i % 2 else "cobble")
        v.set(x, -1, z + 1, "dirt")
        v.set(x, -1, z + 2, "dirt")
    # the open one
    for z in (2, 3):
        v.set(2, -1, z, "air")
        v.set(2, -2, z, "air")
        v.set(2, -3, z, "dirt")
    v.set(3, 0, 2, "dirt")
    v.set(-6, 0, 0, "lantern")
    return v


@recipe
def ruin_wall():
    v = Vol("ruin")
    hs = [3, 4, 4, 2, 1, 0, 0, 2, 5, 5, 3]
    for i, h in enumerate(hs):
        for y in range(h):
            v.set(i - 5, y, 0, pick(i, y, 0, MASONRY))
    for z, h in enumerate([5, 4, 2, 2, 1]):
        for y in range(h):
            v.set(5, y, z, pick(5, y, z, MASONRY))
    v.col(3, 0, 1, 3, "air")   # a window that outlived the wall
    v.set(-3, 0, 2, "cobble")
    v.set(1, 0, 3, "mossy_cobble")
    v.set(2, 0, 3, "mossy_cobble")
    v.set(0, 0, 1, "flower_bed")
    return v


@recipe
def ruin_tower():
    v = Vol("ruin")
    for y in range(0, 9):
        for x in range(-3, 4):
            for z in range(-3, 4):
                if max(abs(x), abs(z)) != 3:
                    continue
                # it broke on a slant
                if y > 3 + (x + 3) * 0.9 + (rnd(x, 0, z) % 2):
                    continue
                v.set(x, y, z, pick(x, y, z, MASONRY))
    v.col(0, 3, 0, 2, "air")
    v.doors.append({"at": [0, 0, 3], "height": 3})
    v.fill((-2, -1, -2), (2, -1, 2), "flagstones")
    for (x, y, z) in ((-5, 0, 1), (-4, 0, -2), (-5, 0, -1), (-4, 1, -2), (1, 0, 1), (-1, 0, -1)):
        v.set(x, y, z, "cobble" if (x + z) % 2 else "mossy_cobble")
    v.set(2, 0, -2, "chest")
    return v


@recipe
def standing_stones():
    v = Vol("ruin")
    for i in range(7):
        a = i / 7 * math.tau
        x, z = round(math.cos(a) * 6), round(math.sin(a) * 6)
        h = 3 + rnd(i, 0, 0) % 3
        if i == 4:
            # one fell
            for k in range(3):
                v.set(x + k, 0, z, "mossy_cobble")
            continue
        v.col(x, z, 0, h, "stone" if i % 2 else "mossy_cobble")
    v.set(0, 0, 0, "stone_bricks")
    v.set(0, 1, 0, "glowshroom")
    return v


@recipe
def wayshrine():
    v = Vol("prop")
    for x in range(-1, 2):
        for z in range(-1, 2):
            v.set(x, -1, z, "flagstones")
    v.col(0, 0, 0, 1, "stone_bricks")
    v.set(0, 2, 0, "lantern")
    for (x, z) in ((-1, 0), (1, 0), (0, 1)):
        v.set(x, 0, z, "flower_bed")
    v.set(0, 0, -1, "blue_banner")
    return v


@recipe
def cairn():
    v = Vol("prop")
    for (x, y, z) in ((0, 0, 0), (1, 0, 0), (0, 0, 1), (-1, 0, 0), (0, 0, -1), (0, 1, 0), (1, 1, 0), (0, 2, 0)):
        v.set(x, y, z, "cobble" if (x + y + z) % 2 else "mossy_cobble")
    return v


@recipe
def milestone():
    v = Vol("prop")
    v.set(0, 0, 0, "stone_bricks")
    v.set(0, 1, 0, "mossy_cobble")
    return v


@recipe
def viewpoint():
    """A lookout: a low stone platform with a banner and a bench."""
    v = Vol("prop")
    v.foundation = "stone_bricks"
    for x in range(-3, 4):
        for z in range(-3, 4):
            if math.hypot(x, z) <= 3.6:
                v.set(x, 0, z, pick(x, 0, z, ["flagstones", "cobble", "stone_bricks"]))
                if math.hypot(x, z) > 2.7 and z < 2:
                    v.set(x, 1, z, "stone_bricks")
    v.col(0, -3, 2, 5, "timber_beam")
    v.col(1, -3, 3, 5, "blue_banner")
    v.set(-1, 1, 0, "planks")
    v.set(0, 1, 0, "planks")
    v.set(2, 1, -1, "lantern")
    return v


@recipe
def hunter_camp():
    v = Vol("prop")
    v.merge(ridge_tent(5, 2, ["yellow", "white"]), at=(-4, 0, -3))
    v.set(1, -1, 1, "cobble")
    v.set(1, 0, 1, "brazier")
    v.set(3, 0, 0, "log")
    v.set(3, 0, 1, "log")
    v.set(0, 0, 3, "crate")
    v.col(4, -3, 0, 2, "timber_beam")
    v.set(3, 2, -3, "white")
    return v


@recipe
def cart():
    """A farm cart with a load of hay."""
    v = Vol("prop")
    for x in range(-2, 2):
        for z in (0, 1):
            v.set(x, 1, z, "planks")
    for (x, z) in ((-2, -1), (1, -1), (-2, 2), (1, 2)):
        v.set(x, 0, z, "dark_planks")
    v.set(-1, 2, 0, "hay_bale")
    v.set(0, 2, 1, "hay_bale")
    v.set(-1, 2, 1, "hay_bale")
    v.set(2, 1, 0, "timber_beam")
    v.set(3, 0, 0, "timber_beam")
    return v


@recipe
def wrecked_caravan():
    """Two wagons run off the road: one on its side, its load spilled."""
    v = Vol("ruin")
    # the wagon that stands, its cover torn
    for x in range(-6, -1):
        for z in (0, 1, 2):
            v.set(x, 1, z, "planks")
        v.set(x, 2, 0, "dark_planks")
        v.set(x, 2, 2, "dark_planks")
        if x % 2 == 0:
            v.set(x, 3, 0, "white")
            v.set(x, 4, 1, "white")
    for (x, z) in ((-6, -1), (-2, -1), (-6, 3), (-2, 3)):
        v.set(x, 0, z, "dark_planks")
    v.set(-1, 1, 1, "timber_beam")
    v.set(0, 0, 1, "timber_beam")
    # the one on its side
    for x in range(3, 8):
        for y in (0, 1, 2):
            v.set(x, y, -3, "planks")
        v.set(x, 0, -2, "dark_planks")
    v.set(3, 3, -3, "dark_planks")
    v.set(7, 3, -3, "dark_planks")
    for (x, y, z) in ((2, 0, 0), (4, 0, 1), (5, 0, -1), (4, 1, 1), (8, 0, 0), (1, 0, 3), (6, 0, 3), (-4, 2, 1)):
        v.set(x, y, z, pick(x, y, z, ["crate", "barrel", "crate"]))
    v.set(5, 0, 2, "black")
    v.set(0, 0, -2, "black")
    v.set(-8, 0, 2, "lantern")
    v.points["wreck"] = (1, 0, 1)
    return v


@recipe
def boulders():
    v = Vol("prop")
    blob(v, 0, 0.5, 0, 2.2, 1.6, "stone", 1)
    blob(v, 3, 0, 2, 1.4, 1.1, "stone", 2)
    blob(v, -3, 0, -1, 1.2, 0.9, "mossy_cobble", 3)
    v.c = {k: b for k, b in v.c.items() if k[1] >= 0}
    return v


@recipe
def ore_outcrop():
    v = Vol("prop")
    blob(v, 0, 0.5, 0, 2.4, 1.8, "stone", 4)
    v.c = {k: b for k, b in v.c.items() if k[1] >= 0}
    for (x, y, z) in ((2, 0, 0), (1, 1, 1), (-2, 0, 1), (0, 0, -2), (-1, 1, -1)):
        v.set(x, y, z, "coal_ore" if (x + z) % 2 else "iron_ore")
    return v


@recipe
def bench_lamp():
    v = Vol("prop")
    lamp_post(v, 0, 0, 0, 4)
    v.set(1, 0, 0, "planks")
    v.set(2, 0, 0, "planks")
    return v


@recipe
def flower_garden():
    v = Vol("prop")
    for x in range(-2, 3):
        for z in range(-1, 2):
            if (x + z) % 2 == 0:
                v.set(x, 0, z, "flower_bed")
    v.set(-3, 0, 0, "timber_beam")
    v.set(3, 0, 0, "timber_beam")
    return v


@recipe
def pond():
    v = Vol("prop")
    for x in range(-5, 6):
        for z in range(-4, 5):
            d = (x / 5) ** 2 + (z / 4) ** 2
            if d <= 1.0:
                v.set(x, -1, z, "water")
                v.set(x, -2, z, "water" if d < 0.5 else "sand")
                v.set(x, -3, z, "sand")
            elif d <= 1.35:
                v.set(x, -1, z, "sand")
    v.set(5, 0, 2, "flower_bed")
    v.set(-5, 0, -2, "flower_bed")
    return v


def main():
    root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "content", "stamps")
    names = sys.argv[1:] or sorted(R)
    for n in names:
        v = R[n]()
        name = n.rstrip("_")
        size = v.write(os.path.join(root, name + ".dstamp"))
        print(f"{name}: {size[0]}x{size[1]}x{size[2]}, {len(v.c)} cells")


if __name__ == "__main__":
    main()
