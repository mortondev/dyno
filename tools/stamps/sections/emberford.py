from build import recipe, R
from kit import *
import math

# Emberford: the bridge over the Emberrun, the toll point, the Ashcloak
# scouts who raid the east road, the boar ground and the river life.
#
# Shared pieces (director/tools/shared_pieces.py, copied verbatim; KEY set).

KEY = "emberford"   # <- your section key


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


# ------------------------------------------------------------------ the bridge

def _rise(x):
    """The deck's height over the ends, stepping up a metre at a time to the
    middle of the river (|x| 12..17 stays one up: the bridge camera stands
    there)."""
    a = abs(x)
    return 3 if a <= 6 else 2 if a <= 11 else 1 if a <= 17 else 0


@recipe
def emberford_bridge_arch():
    """Emberford Bridge as the place-name landmark: a hump deck stepped
    1/2/3 m to the middle on two river arches 3-4 m over the water, a
    cutwater centre pier carrying a pair of tall pylons with ember braziers,
    lantern pillars at the ends, a red-ochre (terracotta) string course along
    both faces, and black-and-red Ashcloak rags tied to the pillars. The
    anchor is the middle of the river, level with the banks (as `bridge`):
    y = -1 is the bank top, y = -2 the water's top."""
    v = Vol("building")
    v.foundation = "stone_bricks"
    L = 20
    piers = {-6: (-6, -6), 0: (-1, 1), 6: (6, 6)}
    pier_x = {x for lo, hi in piers.values() for x in range(lo, hi + 1)}

    def soffit(x):
        """The lowest deck-structure cell over x (None: solid to the
        footing). The river arches open at |x| 2..5."""
        a = abs(x)
        if a in (3, 4):
            return 3          # 4 m clear over the water at the crown
        if a in (2, 5):
            return 2          # 3 m at the springing
        return None

    for x in range(-L, L + 1):
        r = _rise(x)
        s = soffit(x)
        for z in range(-4, 5):
            edge = abs(z) == 4
            # the walking surface
            # the old bridge's paving: cobbles with a stone-brick setts mix
            # (the shaped cobbles give the deck its relief), a terracotta
            # course under the parapets
            top = "terracotta" if edge else pick(x, 0, z, ["cobble", "cobble", "cobble", "stone_bricks"])
            v.set(x, r, z, top)
            # the body under it: arches open to the water, piers to the bed,
            # abutments down into the banks
            if x in pier_x:
                lo = -7
            elif s is not None:
                lo = s
            else:
                lo = -3
            for y in range(lo, r):
                b = pick(x, y, z, MASONRY) if (edge or y < 0) else "stone_bricks"
                v.set(x, y, z, b)
            if s is not None:
                for y in range(-1, s):
                    v.set(x, y, z, "air")
            # headroom over the deck, and the parapet
            v.col(x, z, r + 1, r + 5, "air")
            if edge:
                v.set(x, r + 1, z, pick(x, 1, z, MASONRY))
    # cutwaters: the piers run out past the faces, stepped to a point
    # upstream (north, -z) and downstream
    for sz in (-1, 1):
        for x in (-1, 0, 1):
            v.col(x, 5 * sz, -7, 1, pick(x, 3, sz, MASONRY))
        v.col(0, 6 * sz, -7, 0, "stone_bricks")
        for x in (-6, 6):
            v.col(x, 5 * sz, -7, 0, pick(x, 4, sz, MASONRY))
    # the centre pylons on the cutwaters: 3 wide, stepped to a brazier 9 m
    # over the river (a metre taller than the deck's lantern pillars)
    for sz in (-1, 1):
        z = 5 * sz
        for x in (-1, 0, 1):
            v.col(x, z, 2, 7, "stone_bricks")
        v.set(-1, 4, z, "terracotta")
        v.set(1, 4, z, "terracotta")
        v.set(0, 4, z, "terracotta")
        v.set(0, 8, z, "stone_bricks")
        v.set(0, 9, z, "brazier")
        # an ash rag tied below the brazier, outside face
        v.set(0, 6, z + sz, "black")
        v.set(0, 5, z + sz, "red")
    # lantern pillars at the ends, where the deck first steps up, and over
    # the river piers: a lantern every 6-7 m along both parapets
    for x in (-L, -13, -6, 6, 13, L):
        for z in (-4, 4):
            r = _rise(x)
            v.col(x, z, r + 1, r + 3, "stone_bricks")
            v.set(x, r + 4, z, "lantern")
    # Ashcloak raid rags tied to the west inner pillars' outer faces, seen on
    # the approach from the toll hut (flat one-wide sheets: red over black)
    for x in (-13,):
        for sz in (-1, 1):
            r = _rise(x)
            v.set(x, r + 3, 5 * sz, "red")
            v.set(x, r + 2, 5 * sz, "black")
    v.points["middle"] = (0, 4, 0)
    v.points["west"] = (-L - 1, 1, 0)
    v.points["east"] = (L + 1, 1, 0)
    return v


# ------------------------------------------------------------- the toll point

@recipe
def emberford_toll_hut():
    """The toll keeper's plank hut, without the old bar that ran along the
    road (the bar is the separate emberford_toll_gate across it)."""
    v = R["toll_hut"]()
    for y in (0, 1, 2):
        for x in range(4, 9):
            v.c.pop((x, y, 3), None)
    # a firewood stack by the west wall
    v.set(-3, 0, 1, "log")
    v.set(-3, 0, 0, "log")
    v.set(-3, 1, 0, "log")
    return v


@recipe
def emberford_toll_gate():
    """The toll gate across the road: a stone post beyond each kerb (14 m apart,
    a 13 m clear opening), a timber beam 5 m up from post to post with a
    red-and-white striped valance under it, a lantern on each post. Built
    along x; place it rot 1 so it spans a road running east-west."""
    v = Vol("prop")
    for sx in (-7, 7):
        v.col(sx, 0, -3, 4, "stone_bricks")
        for dz in (-1, 1):
            v.col(sx, dz, -2, 0, "cobble")
        v.set(sx, 6, 0, "lantern")
    for x in range(-7, 8):
        v.set(x, 5, 0, "timber_beam")
    for x in range(-6, 7):
        v.set(x, 4, 0, "terracotta" if x % 2 == 0 else "plaster")
    return v


@recipe
def emberford_toll_desk():
    """The toll keeper's table: a plank table with the coin chest on it, a
    stool, barrels and a crate stack, and a lantern on a post."""
    v = Vol("prop")
    for x in (0, 1):
        v.set(x, 0, 0, "planks")
    v.set(1, 1, 0, "chest")
    v.set(0, 0, -1, "log")            # the keeper's stool
    v.col(-2, 0, 0, 2, "timber_beam")
    v.set(-2, 3, 0, "lantern")
    v.set(3, 0, 0, "barrel")
    v.set(3, 0, -1, "barrel")
    v.set(4, 0, -1, "barrel")
    v.set(3, 1, -1, "crate")
    v.set(-2, 0, -2, "crate")
    v.set(-1, 0, -2, "crate")
    v.set(-2, 1, -2, "crate")
    return v


# ------------------------------------------------------- raid story and camps

@recipe
def emberford_scorch():
    """Scorched ground: a ragged stepped patch of basalt and dirt in the
    ground's top layer, with a few charred ends. 7 x 7."""
    v = Vol("prop")
    for x in range(-3, 4):
        for z in range(-3, 4):
            d = x * x + z * z
            if d > 10 + rnd(x, 0, z, 81) % 4:
                continue
            v.set(x, -1, z, "basalt" if rnd(x, 1, z, 82) % 3 == 0 else "dirt")
    v.set(1, 0, -1, "basalt")
    v.set(-2, 0, 1, "log")
    return v


@recipe
def emberford_burnt_cart():
    """A burnt-out cart: the charred bed tipped on its axle, a wheel off and
    lying flat, the load gone to ash, a last ember in a brazier of its own
    coals, on scorched ground."""
    v = Vol("ruin")
    for x in range(-3, 4):
        for z in range(-3, 4):
            if x * x + z * z <= 11 + rnd(x, 0, z, 83) % 3:
                v.set(x, -1, z, "basalt" if rnd(x, 2, z, 84) % 2 else "dirt")
    # the bed: half its planks gone, black ends
    for x in range(-2, 2):
        v.set(x, 1, 0, "dark_planks" if x % 2 else "basalt")
        if x != 0:
            v.set(x, 1, 1, "dark_planks")
    # one wheel still on, one fallen beside
    v.set(-2, 0, -1, "dark_planks")
    v.set(1, 0, -1, "dark_planks")
    v.set(-2, 0, 2, "dark_planks")
    for (x, z) in ((3, 2), (3, 3), (2, 3)):
        v.set(x, 0, z, "dark_planks")
    v.set(2, 1, 0, "timber_beam")
    v.set(3, 0, 0, "timber_beam")
    v.set(0, 0, 0, "brazier")
    v.set(-1, 2, 0, "basalt")
    v.set(-3, 0, -2, "barrel")
    return v


@recipe
def emberford_cart_overturned():
    """The scouts' prize: a stolen cart rolled onto its side (the bed a plank
    wall, two wheels up, the shafts in the air), its load spilled."""
    v = Vol("ruin")
    for x in range(-2, 3):
        for y in (0, 1):
            v.set(x, y, 0, "planks")
        v.set(x, 0, 1, "dark_planks")           # the bed's side board
    for wx in (-2, 2):
        v.set(wx, 2, -1, "dark_planks")         # wheels sticking up and out
        v.set(wx, 2, 0, "dark_planks")
        v.set(wx, 3, 0, "dark_planks")
    v.set(3, 1, 0, "timber_beam")
    v.set(4, 1, 0, "timber_beam")
    v.set(4, 0, 0, "timber_beam")
    for (x, y, z) in ((-1, 0, 2), (0, 0, 3), (1, 0, 2), (0, 0, 2), (0, 1, 2), (2, 0, 3), (-3, 0, 1)):
        v.set(x, y, z, pick(x, y, z, ["hay_bale", "crate", "barrel", "hay_bale"]))
    return v


@recipe
def emberford_spill():
    """A line of spilled loot dragged along the grass: crates, a barrel, a
    hay bale and a broken crate's planks, 6 m long."""
    v = Vol("prop")
    for (x, y, z, b) in ((0, 0, 0, "crate"), (1, 0, 1, "barrel"), (0, 0, 3, "hay_bale"),
                         (-1, 0, 4, "planks"), (1, 0, 5, "crate"), (1, 1, 5, "crate"),
                         (0, -1, 2, "dirt"), (0, -1, 1, "dirt"), (1, -1, 3, "dirt")):
        v.set(x, y, z, b)
    return v


@recipe
def emberford_loot():
    """A raider's stash: stacked crates and barrels, a hay bale, a log."""
    v = Vol("prop")
    for (x, y, z, b) in ((0, 0, 0, "crate"), (1, 0, 0, "crate"), (0, 1, 0, "crate"),
                         (0, 0, 1, "barrel"), (-1, 0, 1, "barrel"), (2, 0, 1, "hay_bale"),
                         (1, 0, 2, "crate"), (-1, 0, -1, "log"), (0, 0, -1, "log")):
        v.set(x, y, z, b)
    return v


@recipe
def emberford_lookout():
    """The scouts' lookout: a 4 x 4 m plank platform 5 m up on four log
    posts braced with whole-beam rails, a ladder post, a rail round the top,
    a brazier and an Ashcloak flag 11 m up."""
    v = Vol("prop")
    P = 5
    for (x, z) in ((-2, -2), (1, -2), (-2, 1), (1, 1)):
        v.col(x, z, -1, P, "log")
    # braces: straight beam rings at 2 m (face to face with the posts)
    for x in range(-1, 1):
        v.set(x, 2, -2, "timber_beam")
        v.set(x, 2, 1, "timber_beam")
    for z in range(-1, 1):
        v.set(-2, 2, z, "timber_beam")
        v.set(1, 2, z, "timber_beam")
    # the deck and its rail
    for x in range(-2, 2):
        for z in range(-2, 2):
            if (x, z) not in ((-2, -2), (1, -2), (-2, 1), (1, 1)):
                v.set(x, P, z, "planks")
    for (x, z) in ((-2, -2), (1, -2), (-2, 1), (1, 1), (-1, -2), (0, -2), (-2, -1), (1, -1)):
        v.set(x, P + 1, z, "timber_beam")
    v.set(0, P + 1, 0, "brazier")
    # the ladder up the south side
    v.col(-1, 2, 0, P, "timber_beam")
    # the flag pole on the north-east post, flag hung outward (+x)
    v.col(1, -2, P + 2, P + 6, "timber_beam")
    for y in range(P + 3, P + 6):
        v.set(2, y, -2, "black")
        v.set(3, y, -2, "red")
    v.set(1, P + 7, -2, "lantern")
    # a tarp of red cloth as a windbreak on the west rail
    v.set(-2, P + 1, 0, "red")
    return v


# ----------------------------------------------------------------- boar ground

@recipe
def emberford_wallow():
    """A boar wallow one step down: a 6 x 4 m bed of clay and mud with a
    hand's depth of water in the middle (1 m), dirt churned round it."""
    v = Vol("prop")
    for x in range(-4, 5):
        for z in range(-3, 4):
            d = (x / 4.2) ** 2 + (z / 3.2) ** 2
            if d <= 0.55:
                v.set(x, -1, z, "water")
                v.set(x, -2, z, "clay")
            elif d <= 1.0:
                v.set(x, -1, z, "clay")
            elif d <= 1.5:
                v.set(x, -1, z, "dirt" if rnd(x, 0, z, 85) % 3 else "clay")
    return v


@recipe
def emberford_rooted():
    """Rooted-up earth where boars have dug: a ragged dirt patch with clay
    clods and turned sods. 5 x 5."""
    v = Vol("prop")
    for x in range(-2, 3):
        for z in range(-2, 3):
            if rnd(x, 0, z, 86) % 5:
                v.set(x, -1, z, "dirt" if rnd(x, 1, z, 87) % 3 else "clay")
    v.set(1, 0, 0, "dirt")
    v.set(-1, 0, 1, "clay")
    return v


@recipe
def emberford_bones():
    """A bone pile by the wallow: a skull, ribs and long bones (marble) on
    dirt, and a broken boar spear."""
    v = Vol("prop")
    for x in range(-2, 2):
        for z in range(-1, 2):
            v.set(x, -1, z, "dirt")
    for (x, y, z) in ((0, 0, 0), (-1, 0, 0), (0, 1, 0), (1, 0, 1), (-2, 0, 1)):
        v.set(x, y, z, "marble")
    v.set(1, 0, -1, "timber_beam")
    v.set(2, 0, -1, "timber_beam")
    return v


# ------------------------------------------------------------------ river life

@recipe
def emberford_watermill():
    """A ruined watermill on the Emberrun's west bank: a roofless stone shell
    (walls broken in steps, mossy only at the foot), a millstone on the floor,
    an axle through the east wall to a stepped wheel frame standing in the
    river on its own pier, and a rubble fan spilling west. Placed with the
    place at water level ("level": "water", "fit": "pad"): y = -1 is the bank
    top, y = -2 the water's top."""
    v = Vol("ruin")
    v.foundation = "cobble"
    x0, x1, z0, z1 = -4, 4, -3, 3
    for x in range(x0, x1 + 1):
        for z in range(z0, z1 + 1):
            v.set(x, -1, z, "cobble")
    heights = {}
    for x in range(x0, x1 + 1):
        for z in range(z0, z1 + 1):
            if x not in (x0, x1) and z not in (z0, z1):
                continue
            # the shell breaks in steps from the river corner down to the west
            h = 6 - max(0, (x1 - x) // 2) - (1 if z == z1 else 0)
            if x == x0 and z in (0, 1):
                h = 0                      # the door, fallen in
            if z == z0 and x in (-1, 0):
                h = 2                      # a breach
            heights[(x, z)] = h
            for y in range(h):
                v.set(x, y, z, "mossy_cobble" if y == 0 else pick(x, y, z, ["stone_bricks", "stone_bricks", "cobble"]))
    # windows
    for (x, z) in ((2, z0), (2, z1), (-2, z1)):
        if heights.get((x, z), 0) > 3:
            v.col(x, z, 2, 2, "air")
    # the millstone
    for (x, z) in ((-1, -1), (0, -1), (-1, 0), (0, 0)):
        v.set(x, 0, z, "stone")
    v.set(-1, 1, -1, "cobble")
    # the axle through the east wall to the wheel
    W = 8
    for x in range(x1, W + 2):
        v.set(x, 1, 0, "timber_beam")
    # the wheel: a stepped ring of radius 3 in the y-z plane, a plus of
    # spokes, the bottom paddles in the water
    for y in range(-2, 5):
        for z in range(-3, 4):
            d = math.hypot(z, y - 1)
            if 2.5 <= d < 3.7 or (z == 0 or y == 1) and d < 3:
                v.set(W, y, z, "dark_planks")
    # its pier on the river side, and a stone race wall on the bank side
    v.col(W + 1, 0, -7, 1, "stone_bricks")
    v.col(W + 1, -1, -7, 0, "stone_bricks")
    v.col(W + 1, 1, -7, 0, "stone_bricks")
    for z in range(-4, 5):
        v.col(W - 2, z, -3, -1, "stone_bricks")
    # rubble fanning west from the fallen door
    for (x, y, z) in ((-5, 0, 0), (-5, 0, 1), (-6, 0, 1), (-7, 0, 0), (-6, 0, -1), (-5, 1, 1),
                      (-8, 0, 2), (-9, 0, 1)):
        v.set(x, y, z, pick(x, y, z, ["cobble", "stone_bricks", "mossy_cobble"]))
    v.set(-6, 0, 3, "log")
    v.set(-5, 0, 3, "log")
    return v


@recipe
def emberford_beacon():
    """The old beacon on the south hill: a stepped round base (radius 4 and
    3), a 3 x 3 stone shaft broken at one corner, a cobble cap and a brazier
    10 m up, a fallen course at the foot."""
    v = Vol("ruin")
    v.foundation = "stone_bricks"
    for x in range(-4, 5):
        for z in range(-4, 5):
            d = x * x + z * z
            if d <= 16 + 2:
                v.set(x, 0, z, pick(x, 0, z, ["cobble", "stone_bricks", "mossy_cobble"]))
            if d <= 9 + 1:
                v.set(x, 1, z, pick(x, 1, z, ["stone_bricks", "cobble"]))
    for x in range(-1, 2):
        for z in range(-1, 2):
            top = 8 if (x, z) != (1, 1) else 5
            for y in range(2, top + 1):
                v.set(x, y, z, pick(x, y, z, ["stone_bricks", "stone_bricks", "cobble"]))
    v.set(0, 5, -1, "terracotta")
    v.set(-1, 5, 0, "terracotta")
    v.set(1, 5, 0, "terracotta")
    v.set(0, 5, 1, "terracotta")
    v.set(0, 9, 0, "cobble")
    v.set(0, 10, 0, "brazier")
    # tumbled blocks
    for (x, y, z) in ((5, 0, 2), (5, 1, 2), (5, 0, 3), (4, 0, 5)):
        v.set(x, y, z, pick(x, y, z, ["cobble", "stone_bricks"]))
    return v


# ----------------------------------------------------------------- hunter camp

@recipe
def emberford_hunter_camp():
    """The ford-trail hunters' camp, dressed: the yellow tent and brazier, a
    spit over the fire, a drying rack of hides, a pelt on the crate, and a
    boar's skull and tusks on the post."""
    v = R["hunter_camp"]()
    # the spit: two posts and a bar over the brazier at (1, 0, 1)
    v.col(0, 1, 0, 2, "timber_beam")
    v.col(2, 1, 0, 2, "timber_beam")
    v.set(1, 2, 1, "timber_beam")
    # a pelt on the crate
    v.set(0, 1, 3, "orange")
    # the drying rack: posts and a top beam with hides in one plane (z = 4)
    for x in (-5, -1):
        v.col(x, 4, 0, 3, "timber_beam")
    for x in range(-4, -1):
        v.set(x, 3, 4, "timber_beam")
    for x in (-4, -3, -2):
        v.set(x, 2, 4, "orange" if x != -3 else "white")
        v.set(x, 1, 4, "orange" if x != -3 else "white")
    # the trophy on the post at (4, -3): skull and tusks
    v.set(4, 3, -3, "marble")
    v.set(4, 3, -2, "white")
    v.set(4, 3, -4, "white")
    v.set(5, 0, -2, "barrel")
    return v


@recipe
def emberford_scout_tent():
    """A raider's small ridge tent in Ashcloak red and black (5 m long,
    3 m tall): the roadside ambushers travel light."""
    import build
    return build.ridge_tent(5, 2, ["red", "black"])
