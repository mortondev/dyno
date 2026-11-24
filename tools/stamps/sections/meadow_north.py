# North meadow (meadow_north): the Old Quarry, the North Watch, Beacon Rocks,
# the flocks and waysides between Hearthstead and Birchrise.
# Shared pieces (clearings) are copied verbatim from the world-detail brief.
from build import recipe, R
from kit import *

KEY = "meadow_north"   # <- your section key


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


# ------------------------------------------------------------ the Old Quarry

def _quarry_depth(x, z):
    """How deep the quarry is cut at (x, z): 0 at the rim, 3 on the bench
    that rings the west, north and south, 6 on the floor; a stair ramp of
    1 m steps every 2 m climbs out to the east (the trail end)."""
    d = 0
    # the upper bench: a stepped rectangle, corners cut in 2 m steps
    if abs(z) <= 10 and -13 <= x <= 10:
        if not (abs(z) >= 9 and (x <= -12 or x >= 9)) and not (abs(z) == 10 and (x <= -10 or x >= 7)):
            d = 3
    # the floor
    if abs(z) <= 6 and -9 <= x <= 4:
        if not (abs(z) == 6 and (x <= -8 or x >= 3)):
            d = 6
    # the ramp out to the east
    if abs(z) <= 3 and x >= 5:
        d = max(d, max(0, 6 - (x - 3) // 2))
    return d


def _quarry_rock(x, y, z):
    """Cut-face rock in level beds: stone with cobble courses, a pale
    stone_bricks bed, never mossy (sunlit, freshly cut)."""
    if y in (-2, -5):
        return pick(x, y, z, ["cobble", "stone", "cobble"], 3)
    if y == -7:
        return pick(x, y, z, ["stone_bricks", "stone"], 4)
    return pick(x, y, z, ["stone", "stone", "stone", "cobble"], 5)


@recipe
def meadow_north_quarry_pit():
    """The Old Quarry: a pit 27 x 21 m cut 6 m into the meadow in two 3 m
    terraces of stepped stone faces (west, north, south), a stair ramp out
    to the east, coal and iron seams showing on the faces at walking height,
    a rubble apron round the lip. Place with fit ground; 31 x 25 footprint."""
    v = Vol("prop")
    v.foundation = "stone"
    X0, X1, Z0, Z1 = -15, 15, -12, 12
    for x in range(X0, X1 + 1):
        for z in range(Z0, Z1 + 1):
            d = _quarry_depth(x, z)
            # carve: air from the floor up, and a little over the rim so no
            # graded trail or wild cover fills the cut
            if d > 0:
                for y in range(-d, 3):
                    v.set(x, y, z, "air")
            # the floor
            if d >= 6:
                fl = pick(x, 0, z, ["dirt", "stone", "cobble", "dirt", "stone"], 6)
            elif d > 0:
                fl = pick(x, 0, z, ["stone", "cobble", "dirt"], 7)
            else:
                fl = None
            if fl:
                v.set(x, -d - 1, z, fl)
            # rock behind every face: the full column down to the floor of
            # the deepest neighbour
            deep = max(_quarry_depth(x + dx, z + dz) for dx, dz in ((1, 0), (-1, 0), (0, 1), (0, -1)))
            if deep > d:
                for y in range(-deep - 1, -d - 1):
                    v.set(x, y, z, _quarry_rock(x, y, z))
                if d == 0:
                    # the lip: worked stone and rubble, not lawn
                    v.set(x, -1, z, pick(x, 0, z, ["cobble", "stone", "dirt"], 8))
    # the highwall: on the west and north the cut goes on up into a rock
    # spur 3 m above the meadow, stepping down outward in 1 m courses, so
    # the faces read from the trail and the road, not only from the lip
    for x in range(-18, 8):
        for z in range(-15, 13):
            if _quarry_depth(x, z) > 0:
                continue
            if not (x <= -10 or z <= -7):
                continue
            n = min(
                (max(abs(x - a), abs(z - b)) for a in range(-15, 16) for b in range(-12, 13)
                 if _quarry_depth(a, b) > 0),
                default=9,
            )
            h = 4 - n
            # the spur fades out toward the open south and east ends
            if z >= 9 or x >= 4:
                h -= 1
            if z >= 11 or x >= 6:
                h -= 1
            for y in range(-1, h):
                v.set(x, y, z, _quarry_rock(x, y - 3, z))
    for (x, y, z, b) in ((-14, 0, -3, "coal_ore"), (-14, 1, 2, "coal_ore"), (-14, 0, 5, "iron_ore"),
                         (-4, 0, -11, "coal_ore"), (-8, 1, -11, "coal_ore")):
        v.set(x, y, z, b)
    # seams on the faces at walking height: coal on the lower west and
    # north faces, iron on the upper west face (gathering spots)
    for (x, y, z, b) in (
        (-10, -6, -2, "coal_ore"), (-10, -5, -1, "coal_ore"), (-10, -6, 1, "coal_ore"),
        (-10, -6, 4, "iron_ore"), (-10, -5, 3, "coal_ore"),
        (-5, -6, -7, "coal_ore"), (-4, -5, -7, "coal_ore"), (-2, -6, -7, "coal_ore"),
        (1, -6, 7, "coal_ore"), (-3, -5, 7, "iron_ore"), (-6, -6, 7, "coal_ore"),
        (-14, -3, 0, "iron_ore"), (-14, -2, -2, "coal_ore"), (-14, -3, 3, "coal_ore"),
        (-6, -3, -11, "coal_ore"), (-3, -2, -11, "iron_ore"), (2, -3, 11, "coal_ore"),
    ):
        v.set(x, y, z, b)
    # a timber shoring frame on the north face, a lantern hung on it
    for x in (-7, -3):
        v.col(x, -6, -6, -4, "timber_beam")
    for x in range(-7, -2):
        v.set(x, -3, -6, "timber_beam")
    v.set(-5, -4, -6, "lantern")
    # cut blocks waiting on the floor and the bench
    for (x, y, z, b) in (
        (-6, -6, 2, "stone_bricks"), (-5, -6, 2, "stone_bricks"), (-6, -5, 2, "stone_bricks"),
        (-6, -6, 3, "stone_bricks"), (0, -6, -4, "stone"), (1, -6, -4, "cobble"),
        (-11, -3, -8, "stone_bricks"), (-10, -3, -8, "stone_bricks"),
        (-2, -6, 4, "crate"), (-1, -6, 5, "barrel"),
        (2, -6, 1, "anvil"),
    ):
        v.set(x, y, z, b)
    # a hoist on the north bench, standing 6 m over the rim: two posts, a
    # head beam, a jib reaching out over the floor with a hook and a load
    for x in (-1, 3):
        v.col(x, -8, -3, 5, "timber_beam")
    for x in range(-1, 4):
        v.set(x, 6, -8, "timber_beam")
    for z in range(-7, -2):
        v.set(1, 6, z, "timber_beam")
    for y in range(1, 6):
        v.set(1, y, -3, "timber_beam")
    v.set(1, 0, -3, "stone_bricks")
    v.set(1, 5, -7, "lantern")
    v.set(-1, 6, -8, "lantern")
    v.points["floor"] = (-3, -6, 0)
    return v


# ------------------------------------------------------------ the beacon

@recipe
def meadow_north_beacon():
    """Beacon Rocks' signal fire: a stepped stone stack 6 m tall (a 7 m
    stepped round base, a 5 m drum, a 3 x 3 stone_bricks plinth) with a
    brazier on top and a lantern niche. 7 x 7 x 6. Its steps climb."""
    v = Vol("prop")
    v.foundation = "stone"
    for x in range(-3, 4):
        for z in range(-3, 4):
            r = x * x + z * z
            if r <= 11:
                v.set(x, 0, z, pick(x, 0, z, ["stone", "cobble", "stone"], 21))
            if r <= 8:
                v.set(x, 1, z, pick(x, 1, z, ["stone", "stone", "cobble"], 22))
            if r <= 5:
                v.set(x, 2, z, pick(x, 2, z, ["stone", "cobble"], 23))
                v.set(x, 3, z, pick(x, 3, z, ["stone", "stone_bricks"], 24))
            if abs(x) <= 1 and abs(z) <= 1:
                v.set(x, 4, z, "stone_bricks")
    # the fire basket and its rim
    v.set(0, 5, 0, "brazier")
    for (x, z) in ((-1, -1), (1, -1), (-1, 1), (1, 1)):
        v.set(x, 5, z, "stone_bricks")
    # a stair notch up the south side (1 m treads)
    v.set(0, 1, 3, "stone")
    v.set(0, 2, 2, "stone")
    v.set(0, 3, 2, "air")
    v.set(0, 3, 1, "stone_bricks")
    # lantern niche on the road (west) side
    v.set(-2, 2, 0, "lantern")
    return v


@recipe
def meadow_north_log_store():
    """The beacon's fuel: split logs stacked 3-2-1 under a plank lean-to
    on timber posts. 5 x 3 x 3."""
    v = Vol("prop")
    for x in range(-2, 3):
        v.set(x, 0, 0, "log")
        v.set(x, 0, 1, "log")
        if abs(x) <= 1:
            v.set(x, 1, 0, "log")
        if x == 0:
            v.set(x, 2, 0, "log")
    for x in (-3, 3):
        v.col(x, -1, 0, 2, "timber_beam")
        v.col(x, 1, 0, 1, "timber_beam")
    for x in range(-3, 4):
        v.set(x, 3, -1, "dark_planks")
        v.set(x, 3, 0, "dark_planks")
        v.set(x, 2, 1, "dark_planks")
    return v


# ------------------------------------------------------------ waysides

@recipe
def meadow_north_well():
    """The Woodcutters' Well on a cobble apron 7 x 7 (stepped corners),
    a bucket bench and a crate beside it."""
    v = Vol("prop")
    for x in range(-3, 4):
        for z in range(-3, 4):
            if x * x + z * z <= 11:
                v.set(x, -1, z, pick(x, 0, z, ["cobble", "cobble", "flagstones", "stone"], 31))
    v.merge(R["well"]())
    v.set(2, 0, 2, "barrel")
    v.set(-2, 0, 2, "crate")
    return v


@recipe
def meadow_north_shepherd_rack():
    """A shepherd's fleece rack: a timber frame with white fleeces hung in
    one flat sheet, a crook post and a hay bale. 5 x 4 x 1."""
    v = Vol("prop")
    for x in (-2, 2):
        v.col(x, 0, 0, 3, "timber_beam")
    for x in range(-2, 3):
        v.set(x, 3, 0, "timber_beam")
    for x in (-1, 0, 1):
        v.set(x, 2, 0, "white")
        v.set(x, 1, 0, "white")
    v.set(3, 0, 1, "hay_bale")
    v.set(-3, 0, 1, "barrel")
    return v
