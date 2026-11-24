"""Mirrormere shore: Fisher's Landing, the Sunken Shrine, the crab camps and
the lake's flanks. Build with

    python3 tools/stamps/build.py mirrormere_<name> ...

Coordinates as in kit.py: x east, z south, y up; y = 0 is the first layer
above the ground the stamp stands on (y = -1 replaces the ground's top).
Everything is whole 1 m cells; finer detail comes from the blocks' shapes.
"""
from build import recipe, R
from kit import *

KEY = "mirrormere"


# ---------------------------------------------------------------- shared
# (verbatim from the world-detail brief's shared pieces, KEY set)

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


# ---------------------------------------------------------------- the landing

def _net(v, x0, x1, y0, y1, z=0, torn=()):
    """A net hung in one vertical plane: white cloth as an orthogonal lattice
    (every other column full, the top row full), never a checkerboard."""
    for x in range(x0, x1 + 1):
        for y in range(y0, y1 + 1):
            if (x - x0) % 2 == 0 or y == y1:
                if (x, y) not in torn:
                    v.set(x, y, z, "white")


@recipe
def mirrormere_drying_rack():
    """A fisher's net rack, the size of the old drying rack: two posts and a
    beam, two strips of net hung from it with the gap between them open, a
    basket of the catch under one end. 5 x 4 x 2."""
    v = Vol("prop")
    for x in (-2, 2):
        v.col(x, 0, 0, 2, "timber_beam")
    for x in range(-2, 3):
        v.set(x, 3, 0, "timber_beam")
    for x in (-1, 1):
        v.col(x, 0, 1, 2, "white")
    v.set(1, 0, 1, "barrel")
    return v


@recipe
def mirrormere_net_torn():
    """A crab-wrecked net rack: one post down, the beam slumped onto it, the
    net hanging in rags."""
    v = Vol("prop")
    v.col(-3, 0, 0, 3, "timber_beam")
    for x in range(-3, 1):
        v.set(x, 4, 0, "timber_beam")
    v.col(0, 0, 0, 3, "timber_beam")
    # the fallen post and its stretch of beam on the ground
    for x in range(1, 5):
        v.set(x, 0, 1, "timber_beam")
    _net(v, -2, -1, 2, 3)
    v.set(-1, 1, 0, "white")
    v.set(2, 0, 0, "white")
    v.set(3, 0, 0, "white")
    return v


@recipe
def mirrormere_fish_stall():
    """The fishmonger's stall: a striped yellow awning on four posts, a
    counter of fish on ice, a barrel of the day's catch. The counter faces
    +z (turn it to face the plaza). 7 x 4 x 5."""
    v = Vol("prop")
    for (x, z) in ((-2, -1), (2, -1), (-2, 1), (2, 1)):
        v.col(x, z, 0, 2, "timber_beam")
    for x in range(-3, 4):
        for z in range(-2, 3):
            v.set(x, 3 if abs(z) < 2 else 2, z, "yellow" if x % 2 == 0 else "white")
    for x in (-1, 0, 1):
        v.set(x, 0, 1, "planks")
    v.set(-1, 1, 1, "ice")
    v.set(0, 1, 1, "ice")
    v.set(1, 1, 1, "barrel")
    v.set(-1, 0, -1, "crate")
    v.set(1, 0, -1, "barrel")
    v.set(0, 2, 0, "lantern")
    v.points["keeper"] = (0, 0, 0)
    return v


@recipe
def mirrormere_pier_goods():
    """Barrels and crates stacked at a pier head, with a coil of rope."""
    v = Vol("prop")
    for (x, y, z, b) in ((0, 0, 0, "barrel"), (1, 0, 0, "barrel"), (0, 1, 0, "barrel"),
                         (0, 0, 1, "crate"), (1, 0, 1, "crate"), (1, 1, 1, "crate"),
                         (-1, 0, 1, "crate"), (2, 0, 1, "barrel"), (-1, 0, -1, "hay_bale")):
        v.set(x, y, z, b)
    return v


@recipe
def mirrormere_fish_table():
    """A gutting table: a plank bench with fish on ice, a barrel for the
    guts, a crate, and a beam over it hung with the day's best and a lantern."""
    v = Vol("prop")
    for x in (-1, 0, 1):
        v.set(x, 0, 0, "planks")
    v.set(-1, 1, 0, "ice")
    v.set(0, 1, 0, "planks")
    v.set(2, 0, 0, "barrel")
    v.set(-2, 0, 0, "crate")
    for x in (-2, 2):
        v.col(x, -1, 0, 3, "timber_beam")
    for x in range(-2, 3):
        v.set(x, 4, -1, "timber_beam")
    v.set(-1, 3, -1, "cyan")
    v.set(1, 3, -1, "cyan")
    v.set(0, 3, -1, "lantern")
    return v


@recipe
def mirrormere_lantern_string():
    """Two tall posts and a line between them hung with lanterns. 9 x 6 x 1."""
    v = Vol("prop")
    for x in (-4, 4):
        v.col(x, 0, 0, 4, "timber_beam")
    for x in range(-4, 5):
        v.set(x, 5, 0, "timber_beam")
    for x in (-2, 0, 2):
        v.set(x, 4, 0, "lantern")
    return v


@recipe
def mirrormere_boat_trestle():
    """A boat turned keel-up on two trestles for tarring, a barrel of pitch."""
    v = Vol("prop")
    for z in (-2, 2):
        for x in (-1, 1):
            v.set(x, 0, z, "timber_beam")
        v.set(0, 0, z, "planks")
    for z in range(-3, 4):
        for x in (-1, 0, 1):
            if abs(z) == 3 and x != 0:
                continue
            v.set(x, 2 if x == 0 else 1, z, "dark_planks")
    v.set(3, 0, 0, "barrel")
    v.set(3, 0, 1, "brazier")
    return v


@recipe
def mirrormere_boat_capsized():
    """A boat left keel-up on the bank. 3 x 2 x 5."""
    v = Vol("prop")
    for z in range(-2, 3):
        for x in (-1, 0, 1):
            if abs(z) == 2 and x != 0:
                continue
            v.set(x, 1 if x == 0 else 0, z, "dark_planks")
    return v


@recipe
def mirrormere_wreck():
    """A wrecked boat, stern sunk and bow up the bank: a broken keel, sides
    stove in, ribs standing out of the planking. The bow is at -z (point it
    up the bank). Place it at the waterline with fit float."""
    v = Vol("ruin")
    # keel stepping up from the stern (+z, in the water) to the bow
    for z in range(-4, 5):
        y = -1 if z >= 2 else (0 if z >= -2 else 1)
        v.set(0, y, z, "dark_planks")
        if z <= -3:
            v.set(0, y - 1, z, "dark_planks")
    v.col(0, -5, 1, 3, "dark_planks")      # the stem post
    for z in range(-3, 4):
        y = 0 if z >= 1 else 1
        for x in (-1, 1):
            # stove in: planks gone on the port side amidships
            if x == 1 and z in (-1, 0, 1):
                continue
            v.set(x, y, z, "dark_planks" if (z + x) % 3 else "planks")
    # ribs where the planking went
    v.col(1, -1, 1, 2, "timber_beam")
    v.col(1, 1, 0, 1, "timber_beam")
    v.col(-1, -2, 2, 2, "timber_beam")
    v.set(-2, 0, 2, "planks")   # a loose board
    return v


@recipe
def mirrormere_shells():
    """A crab midden: a spill of shell and bone on churned sand and clay, a
    smashed lobster pot."""
    v = Vol("prop")
    for x in range(-2, 3):
        for z in range(-2, 3):
            if abs(x) + abs(z) <= 3:
                v.set(x, -1, z, pick(x, -1, z, ["sand", "sand", "clay", "sand"], 31))
    for (x, y, z, b) in ((0, 0, 0, "marble"), (1, 0, 0, "sand"), (0, 0, 1, "marble"),
                         (-1, 0, 0, "snow"), (0, 1, 0, "snow"), (1, 0, -1, "marble"),
                         (-2, 0, 1, "snow"), (2, 0, 2, "crate")):
        v.set(x, y, z, b)
    v.set(-1, 0, -2, "timber_beam")
    return v


# ---------------------------------------------------------------- the shrine

@recipe
def mirrormere_shrine_heart():
    """The heart of the Sunken Shrine, built on the baked stamp (same anchor,
    placed at [0, 0] with dy -1, so y = -1 is the water's top layer and y = -2
    the drowned floor): a stepped dais rising out of the water to an
    emberstone altar ringed with lanterns; one tall broken arch behind it on
    the north side (13 m over the water); a toppled pillar lying in the
    shallows to the west and a leaning one stepped against the east; rubble
    breaking the island's straight south edge."""
    v = Vol("ruin")
    v.foundation = "stone_bricks"
    # the dais, stepped 9-7-5-3
    for y, r in ((-1, 4), (0, 3), (1, 2), (2, 1)):
        for x in range(-r, r + 1):
            for z in range(-r, r + 1):
                if y <= 0 and max(abs(x), abs(z)) == r:
                    b = pick(x, y, z, ["mossy_cobble", "stone_bricks", "cobble"], 21)
                else:
                    b = "stone_bricks"
                v.set(x, y, z, b)
    v.set(0, 3, 0, "emberstone")
    for (x, z) in ((-1, -1), (1, -1), (-1, 1), (1, 1)):
        v.set(x, 3, z, "lantern")
    # the broken arch on the north side, two cells deep (z = -6, -5)
    for z in (-6, -5):
        # the west pier stands whole, 14 cells from the water's top layer
        for x in (-5, -4):
            for y in range(-1, 13):
                v.set(x, y, z, pick(x, y, z, MASONRY if y < 2 else ["stone_bricks"] * 4 + ["cobble"], 22))
        # the east pier broke off at the spring of the arch
        for x, top in ((4, 7), (5, 6)):
            for y in range(-1, top + 1):
                v.set(x, y, z, pick(x, y, z, MASONRY if y < 2 else ["stone_bricks"], 23))
        v.set(3, 7, z, "stone_bricks")          # the east springer
        # the arch from the west pier, stepped corbels then the lintel,
        # snapped off past the middle
        for y, x1 in ((8, -3), (9, -2), (10, -1), (11, 1), (12, 0)):
            for x in range(-3, x1 + 1):
                v.set(x, y, z, "stone_bricks")
        v.set(-5, 13, z, "stone_bricks")
    v.set(-4, 13, -6, "stone_bricks")
    v.set(-1, 9, -5, "lantern")                 # a lamp still hung in the arch
    # the toppled pillar in the west shallows: a 2 x 2 drum lying east-west
    # with its capital at the far end
    for x in range(-10, -5):
        for z in (2, 3):
            for y in (-1, 0):
                v.set(x, y, z, pick(x, y, z, ["stone_bricks", "stone_bricks", "mossy_cobble"], 24))
    for z in range(1, 5):
        for y in (-1, 0, 1):
            v.set(-11, y, z, "stone_bricks")
    # the leaning pillar on the east: a 2-wide stair of drums, each step
    # resting face to face on the one below
    for i, x0 in enumerate((6, 7, 8)):
        for x in (x0, x0 + 1):
            for z in (-3, -2):
                for y in (2 * i - 1, 2 * i):
                    v.set(x, y, z, pick(x, y, z, ["stone_bricks", "mossy_cobble"], 25))
    # rubble breaking the island's south edge (clear of the keeper at [3, 8])
    for (x, y, z) in ((-7, 0, 7), (-6, 0, 7), (-6, 1, 7), (-5, 0, 8), (-7, 0, 6),
                      (7, 0, 7), (8, 0, 6), (8, 1, 6), (9, 0, 7), (8, 0, 8)):
        v.set(x, y, z, pick(x, y, z, ["mossy_cobble", "cobble", "stone"], 26))
    v.points["altar"] = (0, 3, 0)
    return v


@recipe
def mirrormere_causeway():
    """Stepping stones across the last channel to the shrine island: 2 x 1
    stones at every other metre, each a solid column carried down to the bed
    (foundation). Place in the shrine's place with fit pad, dy -1: the stones'
    tops are the first layer above the water. Runs from z = 0 to -2."""
    v = Vol("prop")
    v.foundation = "stone_bricks"
    for z in (0, -2):
        for x in (0, 1):
            v.set(x, -1, z, "stone_bricks")
            v.set(x, 0, z, pick(x, 0, z, ["cobble", "mossy_cobble", "stone_bricks"], 27))
    return v


@recipe
def mirrormere_step_slab():
    """A flush 2 x 2 flagstone slab: a stepping stone across sand or grass
    (place with fit float, dy -1, so it replaces the ground's top layer)."""
    v = Vol("prop")
    for x in (0, 1):
        for z in (0, 1):
            v.set(x, 0, z, "flagstones" if (x + z) % 2 == 0 else "cobble")
    return v


@recipe
def mirrormere_causeway_posts():
    """Two short lantern posts flanking the start of the causeway (4 m apart)."""
    v = Vol("prop")
    for x in (-2, 2):
        v.set(x, 0, 0, "mossy_cobble")
        v.set(x, 1, 0, "timber_beam")
        v.set(x, 2, 0, "lantern")
    return v


# ---------------------------------------------------------------- the lake

@recipe
def mirrormere_isle_rocks():
    """The rocky crown of the islet in the lake: a stepped stone outcrop on
    its west shoulder, a broken stretch of old wall, a fisher's lantern post
    and a flat landing stone on the east. Stands on the islet's 1 m ground
    (fit pad at the isle place, which is levelled at the water)."""
    v = Vol("prop")
    v.foundation = "stone"
    # stepped rock on the west, highest at the back
    for x in range(-7, -1):
        for z in range(-3, 4):
            d = abs(z) + max(0, x + 4) + max(0, -6 - x)
            h = 5 - d
            if d > 4:
                continue
            for y in range(-1, max(h, 0) + 1):
                v.set(x, y, z, pick(x, y, z, ["stone", "stone", "mossy_cobble", "cobble"], 31))
    # an old wall, broken in steps, on the north
    for x, h in zip(range(-1, 5), (3, 3, 2, 1, 0, 1)):
        for y in range(h):
            v.set(x, y, -4, pick(x, y, -4, MASONRY, 32))
    # a lantern post for the night fishers
    v.col(4, 2, 0, 2, "timber_beam")
    v.set(4, 3, 2, "lantern")
    # the landing stone at the water on the east
    for z in (-1, 0, 1):
        v.set(6, -1, z, "flagstones")
        v.set(7, -1, z, "flagstones")
    return v


@recipe
def mirrormere_boathouse():
    """The old boathouse at the river mouth: a plank shed on a stone footing,
    its water end open, half the thatch fallen in, a slip of planks on posts
    running out over the water. The water end is -z. 9 x 7 x 17."""
    v = Vol("ruin")
    x0, z0, x1, z1 = -3, -3, 3, 4
    v.fill((x0, -1, z0), (x1, -1, z1), "planks")
    for x in range(x0, x1 + 1):
        for z in range(z0, z1 + 1):
            v.col(x, z, -3, -2, "cobble")     # the footing, into the bank
    for x in range(x0, x1 + 1):
        for z in range(z0, z1 + 1):
            if x in (x0, x1) or z == z1:
                for y in range(0, 4):
                    b = "cobble" if y == 0 else ("timber_beam" if (x in (x0, x1) and z in (z0, z1)) or y == 3 else "dark_planks")
                    v.set(x, y, z, b)
    # the door on the land end, and a window gone
    v.col(0, z1, 0, 2, "air")
    v.col(1, z1, 0, 2, "wooden_door")
    v.set(x1, 2, 1, "air")
    v.set(x0, 1, -1, "air")
    v.set(x0, 2, -1, "air")
    top = roof(v, x0, z0, x1, z1, 3, "plank", ridge_x=False)
    # half the roof fell in: the east slope is bare rafters, and the
    # thatch of the west slope is holed
    for (x, y, z) in list(v.c):
        if v.c[(x, y, z)] != "thatch":
            continue
        if x > 0:
            if z % 2 == 0 and z > z0 - 1:
                v.c[(x, y, z)] = "timber_beam"
            else:
                del v.c[(x, y, z)]
        elif x < 0 and z in (z0, 1) and rnd(x, y, z, 33) % 2:
            del v.c[(x, y, z)]
    # planks gone from the land-end wall
    for (x, y) in ((-2, 2), (2, 1), (2, 2), (-1, 3)):
        v.c.pop((x, y, z1), None)
    # the slip: planks on posts over the water
    for z in range(-11, z0):
        for x in (-1, 0, 1):
            if (x, z) in ((1, -11), (-1, -9)):
                continue      # rotten boards
            v.set(x, -1, z, "planks")
    for z in (-6, -10):
        for x in (-2, 2):
            v.col(x, z, -7, 1, "timber_beam")   # driven into the bed
    v.set(-2, 2, -10, "lantern")
    # inside: a boat half-built, a barrel, oars
    for z in (0, 1, 2):
        v.set(0, 0, z, "dark_planks")
    v.set(-2, 0, 3, "barrel")
    v.set(2, 0, 3, "crate")
    v.col(-2, -2, 0, 1, "timber_beam")
    return v


@recipe
def mirrormere_paving():
    """A 7 x 5 patch of worn cobble and flagstones laid flush in the ground
    (it replaces the top layer), a bucket-barrel at one corner: ties a well
    and a bench into one stop."""
    v = Vol("prop")
    for x in range(-3, 4):
        for z in range(-2, 3):
            if abs(x) == 3 and abs(z) == 2:
                continue
            v.set(x, -1, z, pick(x, 0, z, ["cobble", "cobble", "flagstones", "mossy_cobble"], 41))
    v.set(3, 0, -1, "barrel")
    return v


@recipe
def mirrormere_shoal_rocks():
    """Rocks breaking the water: two stepped stacks carried down to the bed
    (foundation), standing 1-2 m out of the water. Place with fit pad, dy -1,
    at a place levelled at the water, so y = -1 is the water's top layer."""
    v = Vol("prop")
    v.foundation = "stone"
    for (x, y, z) in ((-1, -1, -1), (0, -1, -1), (1, -1, -1), (-1, -1, 0), (0, -1, 0), (1, -1, 0),
                      (0, -1, 1), (-1, -1, 1), (0, 0, 0), (-1, 0, 0), (0, 0, -1), (0, 1, 0),
                      (3, -1, 2), (4, -1, 2), (3, -1, 3), (3, 0, 2)):
        v.set(x, y, z, pick(x, y, z, ["stone", "stone", "mossy_cobble"], 51))
    return v
