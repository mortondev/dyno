# South road and the windmill farm: the section's own stamps.
# Build with:
#   python3 tools/stamps/build.py south_road_clear_10 south_road_clear_14 \
#     south_road_clear_20 south_road_ash_flag south_road_listening_stones \
#     south_road_mere_shrine south_road_windmill south_road_broken_tower \
#     south_road_footing south_road_broken_cart south_road_trough \
#     south_road_loot south_road_mill_goods south_road_broken_pen
from build import recipe, R
from kit import *

# ---------------------------------------------------------------------------
# Shared pieces (director/tools/shared_pieces.py), copied verbatim.

KEY = "south_road"   # <- your section key


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
# Landmarks


@recipe
def south_road_listening_stones():
    """The Listening Stones, to the G4 standing_stones spec: a ring of radius
    8 of 2 x 1 m slabs (long side on the tangent) 4-7 m tall, a trilithon
    (two uprights and a 1 m lintel) on the +z side, one fallen 2 x 1 x 5 slab
    on the -z side, and a low 3 x 3 altar with the brazier. 19 x 19, 7 m.
    Place with rot 3 to turn the trilithon east (+x) and the fallen slab west."""
    v = Vol("ruin")

    def slab(cells, h, salt):
        for i, (x, z) in enumerate(cells):
            for y in range(h):
                # weathered: a mossy foot, grey stone above
                v.set(x, y, z, "mossy_cobble" if y == 0 and (i + salt) % 2 == 0 else "stone")
            # a worn ring of bare stone in the turf at each foot
            v.set(x, -1, z, "stone")

    # the trilithon on +z: uprights 2 x 1, 6 m, a lintel 7 x 1 on top
    slab([(-3, 8), (-2, 8)], 6, 0)
    slab([(2, 8), (3, 8)], 6, 1)
    for x in range(-3, 4):
        v.set(x, 6, 8, "stone")
    # the ring (tangent slabs at the quarters, 2 x 1 at the eighths)
    slab([(8, -1), (8, 0)], 5, 2)            # east
    slab([(-8, 0), (-8, 1)], 6, 3)           # west
    slab([(5, -6), (6, -6)], 4, 4)           # north-east
    slab([(6, 5), (6, 6)], 7, 5)             # south-east, the tallest, by the trilithon
    slab([(-6, 5), (-6, 6)], 5, 6)           # south-west
    # the fallen one on -z: 5 long, 2 wide, lying flat, cracked in two
    for x in range(-2, 3):
        for z in (-9, -8):
            if x != 0:
                v.set(x, 0, z, "stone" if (x + z) % 3 else "mossy_cobble")
    v.set(0, 0, -8, "mossy_cobble")
    # the altar: a low 3 x 3 table of dressed stone with the brazier
    for x in range(-1, 2):
        for z in range(-1, 2):
            v.set(x, 0, z, "stone_bricks")
            v.set(x, -1, z, "stone")
    v.set(0, 1, 0, "brazier")
    # flagstones worn into the turf on the way in from the open side
    for (x, z) in ((-3, 0), (-4, 0), (-5, -1), (-6, -1), (0, -3), (0, -4), (3, 1), (4, 1)):
        v.set(x, -1, z, "flagstones")
    return v


@recipe
def south_road_mere_shrine():
    """The Wayshrine of the Mere (G4 wayshrine, built tall): a 6 x 6
    stone_bricks plinth in two 1 m steps, a 2 x 2 shaft 3 m tall with a
    lantern niche on its -z face, a 4 x 4 capstone and a stepped cap with a
    lantern: 7.5 m. Blue banners stand on the step under the capstone on the
    -x and -z faces (the road side), candles (lanterns) on the plinth corners,
    and flower beds on the lake (+x, +z) sides."""
    v = Vol("prop")
    for x in range(-3, 3):
        for z in range(-3, 3):
            v.set(x, -1, z, "cobble")
            v.set(x, 0, z, "stone_bricks")
    for x in range(-2, 2):
        for z in range(-2, 2):
            v.set(x, 1, z, "stone_bricks")
    for x in (-1, 0):
        for z in (-1, 0):
            v.col(x, z, 2, 4, "stone_bricks")
    v.set(-1, 3, -1, "lantern")          # the niche, facing the road
    for x in range(-2, 2):
        for z in range(-2, 2):
            v.set(x, 5, z, "stone_bricks")
    for x in (-1, 0):
        for z in (-1, 0):
            v.set(x, 6, z, "cobble")
    v.set(-1, 7, -1, "lantern")
    # banners on the road faces, standing on the step under the capstone
    for (x, z) in ((-2, -2), (1, -2), (-2, 1)):
        v.col(x, z, 2, 4, "blue_banner")
    # candles on the plinth corners
    for (x, z) in ((-3, -3), (2, -3), (-3, 2), (2, 2)):
        v.set(x, 1, z, "lantern")
    # flower beds round the lake sides
    for z in range(-3, 3):
        v.set(3, 0, z, "flower_bed")
    for x in range(-3, 3):
        v.set(x, 0, 3, "flower_bed")
    v.set(3, 0, 3, "flower_bed")
    return v


@recipe
def south_road_windmill():
    """The windmill with its sails as a + (up, down, left, right) in the
    plane south of the wall: each arm a timber spar 9 m long with a solid
    3 m wide cloth panel on its trailing side, striped in whole rows or
    columns (white with a yellow tip band). No cube diagonals."""
    v = R["windmill"]()
    hy = v.points["sails"][1]
    # strip the old diagonal sails (everything in the sail plane z = 6, and
    # anything hanging off it in the air) but keep the hub
    for (x, y, z) in list(v.c):
        if z == 6 and y >= 3:
            del v.c[(x, y, z)]
    v.set(0, hy, 5, "timber_beam")
    v.set(0, hy, 6, "timber_beam")
    # a 2 x 2 hub boss
    for (x, y) in ((0, hy), (1, hy), (0, hy + 1), (1, hy + 1)):
        v.set(x, y, 6, "dark_planks")
    # arms: (spar direction, panel direction)
    arms = (((0, 1), (1, 0)), ((1, 0), (0, -1)), ((0, -1), (-1, 0)), ((-1, 0), (0, 1)))
    base = {(0, 1): (0, hy + 1), (1, 0): (1, hy), (0, -1): (1, hy), (-1, 0): (0, hy + 1)}
    for (dx, dy), (px, py) in arms:
        ox, oy = base[(dx, dy)]
        for i in range(1, 10):
            sx, sy = ox + dx * i, oy + dy * i
            v.set(sx, sy, 6, "timber_beam")
            if i >= 3:
                for k in range(1, 4):
                    cloth = "yellow" if i >= 8 else "white"
                    v.set(sx + px * k, sy + py * k, 6, cloth)
    return v


@recipe
def south_road_broken_tower():
    """The Broken Tower re-cut: a 9 x 9 shell broken on a stepped slant from
    13 m at the north-west corner to 2-3 m at the south-east, arrow slits
    (1 x 2 m) at each 4 m storey, a doorway on the +x (road) side, mossy
    stone only at the base courses, a hearth and log seats inside, and a
    rubble fan spilling 6 m toward the road (+x)."""
    v = Vol("ruin")
    a = 4

    def top(x, z):
        # high at (-a, -a), low at (a, a), stepped in whole metres
        t = 13 - (x + a) * 0.62 - (z + a) * 0.62
        return int(t) + (rnd(x, 0, z, 3) % 2)

    for x in range(-a, a + 1):
        for z in range(-a, a + 1):
            if max(abs(x), abs(z)) != a:
                v.set(x, -1, z, pick(x, -1, z, ["cobble", "cobble", "mossy_cobble", "dirt"]))
                continue
            for y in range(0, max(top(x, z), 1)):
                if y <= 1:
                    b = pick(x, y, z, ["mossy_cobble", "cobble", "stone_bricks"])
                else:
                    b = pick(x, y, z, ["stone_bricks", "stone_bricks", "stone_bricks", "cobble"])
                v.set(x, y, z, b)
    # arrow slits, 1 x 2, at each storey where the wall still stands
    for k in (1, 2):
        y0 = k * STOREY + 1
        for (x, z) in ((0, -a), (-a, 0), (a, -2), (-2, a)):
            if top(x, z) > y0 + 2:
                v.col(x, z, y0, y0 + 1, "air")
    # the doorway on the road side: 2 wide, 3 high
    for z in (0, 1):
        v.col(a, z, 0, 2, "air")
    v.doors.append({"at": [a, 0, 0], "height": 3})
    # inside: a cold hearth and log seats, a chest in the high corner
    v.set(0, -1, 0, "cobble")
    v.set(0, 0, 0, "brazier")
    v.set(-2, 0, 0, "log")
    v.set(-2, 0, 1, "log")
    v.set(1, 0, -2, "log")
    v.set(-3, 0, -3, "chest")
    # the rubble fan: stepped, highest against the broken wall
    for x in range(a + 1, a + 7):
        for z in range(-1, 6):
            d = (x - a) + abs(z - 2) * 0.8
            r = rnd(x, 0, z, 21) % 4
            if d <= 2.5:
                h = 2 if r else 1
            elif d <= 4.5:
                h = 1 if r else 0
            elif d <= 6.5:
                h = 1 if r == 0 else 0
            else:
                h = 0
            for y in range(h):
                v.set(x, y, z, pick(x, y, z, ["cobble", "mossy_cobble", "stone_bricks", "cobble"]))
    # spill inside the low corner too
    for (x, y, z) in ((2, 0, 3), (3, 0, 2), (3, 1, 3), (3, 0, 3), (1, 0, 3)):
        v.set(x, y, z, "cobble")
    return v


@recipe
def south_road_footing():
    """The footing of the old curtain wall that ran from the Broken Tower:
    a 12 m run, 2 m thick, of 1 m blocks, with a few 2 m stubs and gaps
    where it was robbed. Rotate for a north-south run."""
    v = Vol("ruin")
    for x in range(-6, 6):
        r = rnd(x, 0, 0, 31) % 7
        if r == 0:
            continue          # robbed out
        for z in (0, 1):
            if r == 1 and z == 1:
                continue
            v.set(x, 0, z, pick(x, 0, z, ["mossy_cobble", "cobble", "cobble", "stone_bricks"]))
            if r == 6 and z == 0:
                v.set(x, 1, z, pick(x, 1, z, ["stone_bricks", "cobble"]))
    # a tumbled block beside it
    v.set(2, 0, 3, "cobble")
    v.set(-3, 0, -2, "mossy_cobble")
    return v


# ---------------------------------------------------------------------------
# Props


@recipe
def south_road_broken_cart():
    """A cart that lost a wheel on the road: the bed down at one end, the
    wheel lying flat beside it, its hay spilled on the verge. About 6 x 4."""
    v = Vol("prop")
    for z in (0, 1):
        v.set(-2, 0, z, "planks")      # the dropped end on the ground
        for x in (-1, 0, 1):
            v.set(x, 1, z, "planks")
        v.set(-2, 1, z, "planks")
    # wheels: the good pair at the far end, the near pair gone
    v.set(1, 0, -1, "dark_planks")
    v.set(1, 0, 2, "dark_planks")
    # the lost wheel, lying flat
    for (x, z) in ((-3, 3), (-2, 3)):
        v.set(x, 0, z, "dark_planks")
    # the shafts, down on the ground in front
    v.set(2, 1, 0, "timber_beam")
    v.set(3, 0, 0, "timber_beam")
    v.set(4, 0, 0, "timber_beam")
    # the load: hay still on the bed and spilled off the low end
    v.set(0, 2, 0, "hay_bale")
    v.set(1, 2, 1, "hay_bale")
    v.set(-1, 2, 1, "hay_bale")
    v.set(-3, 0, 0, "hay_bale")
    v.set(-3, 0, 1, "hay_bale")
    v.set(-4, 0, 1, "hay_bale")
    v.set(-3, 1, 1, "hay_bale")
    v.set(0, 0, 3, "barrel")
    return v


@recipe
def south_road_trough():
    """A plank watering trough, 4 x 2 m, with water in it."""
    v = Vol("prop")
    for x in range(-2, 2):
        for z in (0, 1):
            v.set(x, -1, z, "planks")
    for x in range(-2, 2):
        for z in (-1, 2):
            v.set(x, 0, z, "planks")
    for z in (0, 1):
        v.set(-3, 0, z, "planks")
        v.set(2, 0, z, "planks")
    for x in range(-2, 2):
        for z in (0, 1):
            v.set(x, 0, z, "water")
    v.set(3, 0, 0, "hay_bale")
    return v


@recipe
def south_road_broken_pen():
    """The flock's pen after the bandits came through: an L of post and
    rail with a gate lying open, a run knocked flat, scattered hay. Open
    on two sides (no closed ring)."""
    v = Vol("prop")
    # north run, with a gap where the gate was
    for x in range(-8, 9):
        if x in (1, 2, 3):
            continue
        v.set(x, 0, -6, "timber_beam")
        if x % 3 == 0:
            v.set(x, 1, -6, "timber_beam")
    # the gate, lying open on the grass
    for x in (1, 2, 3):
        v.set(x, -1, -8, "dark_planks")
    # west run, broken short
    for z in range(-5, 5):
        if z in (0, 1):
            continue
        v.set(-8, 0, z, "timber_beam")
        if z % 3 == 0:
            v.set(-8, 1, z, "timber_beam")
    # a knocked-down rail lying in the grass
    for z in (0, 1, 2):
        v.set(-10, -1, z, "dark_planks")
    # scattered hay
    for (x, z) in ((-5, -4), (-6, -4), (4, 3), (-3, 4)):
        v.set(x, 0, z, "hay_bale")
    return v


@recipe
def south_road_loot():
    """Stolen goods piled by the thieves' cart: crates, barrels, sacks
    (hay bales), one crate broken open on the ground."""
    v = Vol("prop")
    for (x, y, z, b) in ((0, 0, 0, "crate"), (1, 0, 0, "crate"), (0, 1, 0, "crate"),
                         (0, 0, 1, "barrel"), (2, 0, 1, "barrel"), (-1, 0, 2, "hay_bale"),
                         (1, 0, 2, "hay_bale"), (1, 1, 2, "hay_bale"), (3, 0, -1, "crate"),
                         (-2, 0, 0, "barrel")):
        v.set(x, y, z, b)
    v.set(3, -1, 1, "planks")    # the broken crate's boards
    v.set(4, -1, 1, "planks")
    return v


@recipe
def south_road_mill_goods():
    """Flour sacks (hay bales), barrels and crates waiting by the mill door
    for Pip's cart, with a lamp post: the carter's stop."""
    v = Vol("prop")
    for (x, y, z, b) in ((0, 0, 0, "hay_bale"), (1, 0, 0, "hay_bale"), (0, 1, 0, "hay_bale"),
                         (0, 0, 1, "hay_bale"), (2, 0, 1, "barrel"), (3, 0, 1, "barrel"),
                         (3, 1, 1, "crate"), (3, 0, 0, "crate")):
        v.set(x, y, z, b)
    lamp_post(v, -2, 0, 0, 4)
    return v


@recipe
def south_road_scorch():
    """A patch of scorched ground (the 'scorched' mix, dirt 3 : basalt 1)
    for a hostile camp on a slope, where a pad would cut the hill: a stepped
    disc of radius 4 laid into the turf, two cells deep. Place with fit
    float, several overlapping, so it follows the ground."""
    v = Vol("prop")
    for x in range(-4, 5):
        for z in range(-4, 5):
            if x * x + z * z <= 18:
                b = "basalt" if rnd(x, 0, z, 51) % 4 == 0 else "dirt"
                v.set(x, -1, z, b)
                v.set(x, -2, z, "dirt")
    return v
