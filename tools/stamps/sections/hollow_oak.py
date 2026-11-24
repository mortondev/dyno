from build import recipe, R, blob
from kit import *
import math

# The Hollow Oak section (brief.md section 7, card "hollow_oak").
# Build with:
#   python3 tools/stamps/build.py hollow_oak_tree hollow_oak_web_sheet hollow_oak_cocoons \
#       hollow_oak_clear_6 hollow_oak_clear_10 hollow_oak_clear_14 hollow_oak_clear_20 \
#       hollow_oak_hunters hollow_oak_handcart hollow_oak_capring hollow_oak_limb hollow_oak_woodcut

# ---------------------------------------------------------------------------
# Shared pieces (director/tools/shared_pieces.py), copied verbatim; KEY set.

KEY = "hollow_oak"   # <- your section key


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
# Section helpers


def curtain(v, cells, top, drops):
    """Silk hung from a solid run above (a rail or a limb): for each (x, z)
    cell, a column of cloth `drops[i]` long under `top`. Every cloth cell has
    a solid block straight above it, so the cloth rule draws flat cubes and
    never a slanted ribbon; the ragged lengths and gaps read as torn web."""
    for (x, z), n in zip(cells, drops):
        for k in range(1, n + 1):
            if v.get(x, top - k, z) is None:
                v.set(x, top - k, z, "white")
            else:
                break


def _root_web():
    """A web strung across the gap between two roots: two dead posts, a rail,
    and torn silk hanging from it to the ground. 7 x 4 x 1."""
    v = Vol("prop")
    for x in (-3, 3):
        v.col(x, 0, 0, 3, "log")
    for x in range(-2, 3):
        v.set(x, 3, 0, "log")
    curtain(v, [(x, 0) for x in range(-2, 3)], 3, [2, 0, 3, 1, 2])
    return v


def walk4(pts):
    """Cells of a 4-connected path through the (x, z) waypoints: every step
    moves along x or z only, so no two cells meet only at a corner."""
    out = [tuple(pts[0])]
    for (x1, z1) in pts[1:]:
        x, z = out[-1]
        while (x, z) != (x1, z1):
            dx, dz = x1 - x, z1 - z
            # step along the axis with the larger remaining error
            if abs(dx) >= abs(dz) and dx:
                x += 1 if dx > 0 else -1
            elif dz:
                z += 1 if dz > 0 else -1
            out.append((x, z))
    return out


def glowcaps(v, x, z, dx, dz):
    """A cluster of glowing caps in a root crotch: three cells flush with the
    turf, so only the block's top (a spread of small caps) shows."""
    v.set(x, -1, z, "glowshroom")
    v.set(x + dx, -1, z + dz, "glowshroom")
    v.set(x + dx - dz, -1, z + dz + dx, "glowshroom")


# ---------------------------------------------------------------------------
# The Hollow Oak, re-cut


@recipe
def hollow_oak_tree():
    """The Hollow Oak (re-cut from R["hollow_oak"]()): 26 m, a trunk that
    leans 1-2 m east toward the trail and swells into buttresses at the foot,
    roots that step down from two blocks wide to one and run under the turf,
    web lattices strung between root pairs and under two low limbs, glowing
    cap clusters in the root crotches, and the lit hollow behind the
    south door. Limbs and roots are 4-connected (no corner-only joins)."""
    v = Vol("landmark")
    H = 26
    TOP = H - 5   # trunk top

    def girth(y):
        if y < 2:
            return 6.4 - y * 0.5
        if y < 6:
            return 5.4 - (y - 2) * 0.28
        if y < 12:
            return 4.3
        return max(2.2, 4.3 - (y - 12) * 0.3)

    def lean(y):
        # the trunk drifts 2 m east and 1 m north by the crown, in steps
        f = (max(0, y - 4) / (TOP - 4)) ** 1.3
        return round(2.0 * f), round(-1.0 * f)

    # buttresses: bearings (radians, atan2(z, x)) and how far they stand out;
    # none on the door side (south, +z)
    butt = [(0.15, 2.4), (-0.95, 2.0), (-2.2, 2.2), (2.75, 2.2), (0.9, 1.4), (2.25, 1.4)]

    for y in range(0, TOP + 1):
        r = girth(y)
        ox, oz = lean(y)
        ri = int(r) + 5
        for x in range(-ri, ri + 1):
            for z in range(-ri, ri + 1):
                a = math.atan2(z, x)
                bump = 0.45 * math.sin(a * 5 + y * 0.2)
                if y < 9:
                    for (b, s) in butt:
                        d = math.atan2(math.sin(a - b), math.cos(a - b))
                        bump += s * (1 - y / 9) ** 1.4 * math.exp(-(d * d) / 0.06)
                d = math.hypot(x, z)
                if d <= r + bump:
                    hollow = y <= 5 and d <= min(r - 2.4, 3.2)
                    v.set(x + ox, y, z + oz, "air" if hollow else "log")

    # moss on every exposed step of the flared foot: a log with open sky
    # above draws as a sawn post, so the steps carry a mossy cap
    for (x, y, z), b in list(v.c.items()):
        if b == "log" and y < 10 and (x, y + 1, z) not in v.c:
            v.set(x, y + 1, z, "leaves")

    # great roots: straight axis-aligned runs (one turns a corner) so every
    # log lies as a beam with bark on top. A log never stands on a log (that
    # draws a sawn post); near the trunk the root carries a cap of moss, two
    # wide, then one wide, then it dives flush under the turf.
    roots = [
        ([(5, 4), (5, 14)], (1, 0)),               # SE, flanks the door
        ([(-5, 4), (-5, 14)], (-1, 0)),            # SW, flanks the door
        ([(5, 3), (11, 3)], (0, 1)),               # E, south (clear of the trail)
        ([(-5, 3), (-11, 3), (-11, 12)], (0, 1)),  # W, turning south
        ([(5, -4), (16, -4)], (0, -1)),            # E, north
        ([(-5, -4), (-15, -4)], (0, -1)),          # W, north
        ([(4, -5), (4, -16)], (1, 0)),             # N, east
        ([(-4, -5), (-4, -14)], (-1, 0)),          # N, west
    ]
    for path, (wx, wz) in roots:
        cells = walk4(path)
        for k, (x, z) in enumerate(cells):
            d = math.hypot(x, z)
            last = len(cells) - k <= 2
            for (cx, cz) in [(x, z)] + ([(x + wx, z + wz)] if d < 10 and k < len(cells) - 4 else []):
                if v.get(cx, 0, cz) is not None:
                    continue   # inside the trunk, the hollow or a buttress
                if last or d >= 12.5:
                    v.set(cx, -1, cz, "log")       # flush: the root dives
                else:
                    v.set(cx, 0, cz, "log")
                    if d < 8.6:
                        v.set(cx, 1, cz, "leaves")  # moss on the root's back

    # the rotted doorway, south: three wide, five high, into the hollow; the
    # door channel runs out through the flare so the hollow stays lit
    for x in (-1, 0, 1):
        for z in range(2, 11):
            v.col(x, z, 0, 4 if x == 0 else 3, "air")
    for x in (-2, 2):
        for z in range(4, 11):
            v.col(x, z, 0, 1, "air")
    v.points["hollow"] = (0, 0, 0)
    v.points["door"] = (0, 0, 9)
    v.fill((-3, -1, -3), (3, -1, 3), "dirt")
    v.set(-2, 0, -2, "lantern")
    v.set(2, 0, -2, "lantern")
    v.set(0, 3, -3, "lantern")
    for (x, z) in ((1, -2), (2, -2), (-2, 1), (-1, 2)):
        v.set(x, -1, z, "glowshroom")       # caps on the floor light the hollow

    # web nests: the shared web sheet (an orthogonal lattice in one plane)
    # strung across each gap between a root pair, its posts on the roots
    for (x, z, rot) in ((-9, 8, 0), (10, -1, 1), (-10, -1, 1), (0, -10, 0)):
        v.merge(_root_web(), at=(x, 0, z), rot=rot)
    # cocoons and bones at the nests
    for (x, z) in ((-8, 10), (-12, 0), (11, 1), (12, -2), (0, -12), (2, -13)):
        if v.get(x, 0, z) is None:
            v.set(x, -1, z, "dirt")
            v.col(x, z, 0, 1 if (x + z) % 2 else 0, "white")

    # glowing cap clusters in the root crotches (flush with the turf)
    for (x, z, dx, dz) in ((7, 1, 1, 0), (-7, 0, -1, 0), (2, -8, 0, -1), (-5, -7, 0, -1),
                           (7, -6, 1, 0), (-8, 7, -1, 0)):
        glowcaps(v, x, z, dx, dz)

    # two low limbs that break the column: axis-aligned, stepping up as
    # right-angle stairs like the snags' limbs
    def limb(dirx, dirz, y0, reach):
        cells = []
        y = y0
        ox, oz = lean(y0)
        for s in range(2, reach + 1):
            x, z = ox + dirx * s, oz + dirz * s
            if s > 2 and s % 4 == 0:
                v.set(x, y, z, "log")      # riser: the step joins face to face
                y += 1
            v.set(x, y, z, "log")
            cells.append((x, y, z))
        return cells

    east = limb(1, 0, 10, 14)
    west = limb(-1, 0, 9, 12)
    for cells, salt in ((east, 101), (west, 102)):
        ex, ey, ez = cells[-1]
        blob(v, ex, ey + 1, ez, 3.6, 2.0, "leaves", salt)
        mx, my, mz = cells[len(cells) // 2 + 1]
        blob(v, mx, my + 2, mz, 2.4, 1.4, "leaves", salt + 10)
    # silk hung under the low limbs: torn curtains from the limb itself
    for cells, drops in ((east, [0, 0, 3, 1, 4, 2, 0, 3]), (west, [0, 0, 2, 4, 1, 3, 2])):
        for (x, y, z), n in zip(cells, drops):
            curtain(v, [(x, z)], y, [n])

    # upper limbs (4-connected, one log thick), each with leaf clumps
    limbs = [(1, 0.25, 13), (-1, 0.45, 14), (0.35, -1, 15), (-0.45, 1, 14), (0.85, 0.9, 16), (-0.9, -0.8, 16),
             (0.9, -0.7, 17), (-0.6, 0.95, 18)]
    cx, cz = lean(TOP)
    for i, (dx, dz, y0) in enumerate(limbs):
        n = math.hypot(dx, dz)
        dx, dz = dx / n, dz / n
        reach = 8 + (i % 3) * 2
        ox, oz = lean(y0)
        pts = walk4([(ox + round(dx * 3), oz + round(dz * 3)), (ox + round(dx * reach), oz + round(dz * reach))])
        y = y0
        for s, (x, z) in enumerate(pts):
            if s and s % 3 == 0:
                v.set(x, y, z, "log")
                y += 1
            v.set(x, y, z, "log")
        ex, ez = pts[-1]
        blob(v, ex, y + 2, ez, 4.0, 2.4, "leaves", i)
        if i % 2 == 0:
            blob(v, ex * 0.6 + dz * 3, y + 4, ez * 0.6 - dx * 3, 3.0, 2.0, "leaves", 20 + i)
        # a hanging strand of leaves under each clump: the crown has no flat floor
        for k in range(1, 3 + i % 3):
            if v.get(ex, y + 1 - k, ez) is None:
                v.set(ex, y + 1 - k, ez, "leaves")
    for i in range(5):
        ang = i * math.tau / 5
        blob(v, cx + math.cos(ang) * 5, H - 4 - (i % 2) * 2, cz + math.sin(ang) * 5, 3.6, 2.4, "leaves", 60 + i)
    blob(v, cx, H - 2, cz, 4.6, 2.4, "leaves", 70)
    return v


# ---------------------------------------------------------------------------
# Props


@recipe
def hollow_oak_hunters():
    """Rysa's hunters' camp east of the pond: a lean-to of stepped logs over a
    bedroll, a campfire with a log seat, a drying rack hung with silk
    bundles, a crate stack, and Nim's mushroom basket. Anchor = the camp's
    middle; the two NPC spots (2,-4) and (-2,4) keep 3 m clear. 11 x 15."""
    v = Vol("prop")
    # worn floor round the fire (dirt 4 : grass 1)
    for x in range(-2, 6):
        for z in range(-2, 4):
            if (x - 1) ** 2 + z * z <= 9 and rnd(x, 0, z, 5) % 5:
                v.set(x, -1, z, "dirt")
    # campfire: brazier in a cobble ring, a log seat to the south
    for (x, z) in ((1, 0), (0, 0), (2, 0), (1, -1), (1, 1)):
        v.set(x, -1, z, "cobble")
    v.set(1, 0, 0, "brazier")
    v.set(1, 0, 2, "log")
    v.set(2, 0, 2, "log")
    # lean-to: open to the west (the fire), roof of logs stepping down east
    for z in range(-3, 4):
        v.set(5, 3, z, "log")
        v.set(6, 3, z, "log")
        v.set(7, 2, z, "log")
        v.set(8, 1, z, "log")
        v.set(8, 0, z, "log")
    for z in (-3, 3):
        v.col(5, z, 0, 2, "timber_beam")
        v.set(7, 1, z, "timber_beam")
        v.set(7, 0, z, "timber_beam")
    v.set(6, 0, -1, "bed")
    v.set(6, 0, 0, "bed")
    v.set(7, 0, 2, "chest")
    v.set(5, 2, 0, "lantern")
    # drying rack hung with silk bundles (a flat cloth row under the bar)
    for z in (-6, -10):
        v.col(6, z, 0, 2, "timber_beam")
    for z in range(-10, -5):
        v.set(6, 3, z, "timber_beam")
    for z in (-9, -8, -7):
        v.set(6, 2, z, "white")
    v.set(5, 0, -8, "white")
    v.set(5, 0, -9, "white")
    # crates, stacked
    v.set(4, 0, 6, "crate")
    v.set(5, 0, 6, "crate")
    v.set(4, 0, 7, "crate")
    v.set(4, 1, 6, "crate")
    v.set(6, 0, 6, "barrel")
    # Nim's mushroom basket, with sorted caps beside it
    v.set(0, 0, 8, "barrel")
    v.set(1, -1, 8, "glowshroom")
    v.set(1, -1, 9, "glowshroom")
    return v


@recipe
def hollow_oak_handcart():
    """The lost forager's handcart, broken on one wheel and tipped: a plank
    bed stepping down to the broken side, a spilled basket and caps, a silk
    bundle where the websnares took her. 7 x 5."""
    v = Vol("prop")
    for z in (0, 1):
        v.set(-2, 1, z, "planks")
        v.set(-1, 1, z, "planks")
        v.set(0, 0, z, "planks")
        v.set(1, 0, z, "planks")
    v.set(-2, 0, -1, "dark_planks")     # the whole wheel
    v.set(-2, 0, 2, "dark_planks")
    v.set(2, -1, 2, "dark_planks")      # the broken one, flat in the grass
    v.set(-3, 1, 0, "timber_beam")      # handles
    v.set(-3, 1, 1, "timber_beam")
    v.set(-3, 0, 0, "timber_beam")
    v.set(-3, 0, 1, "timber_beam")
    v.set(0, 1, 0, "barrel")
    v.set(2, 0, 0, "crate")
    for (x, z) in ((3, 1), (3, 2), (4, 1)):
        v.set(x, -1, z, "glowshroom")
    v.set(1, -1, -2, "dirt")
    v.col(1, -2, 0, 1, "white")
    return v


@recipe
def hollow_oak_capring():
    """A ring of pale glowing caps flush in the turf (Nim's "pale caps for
    sleep"), in clusters of two and three, round a rotten stump that brings
    the toadstool cover in. 9 x 9."""
    v = Vol("prop")
    for i in range(8):
        a = i * math.tau / 8 + 0.3
        x, z = round(math.cos(a) * 4), round(math.sin(a) * 4)
        v.set(x, -1, z, "glowshroom")
        if i % 2:
            v.set(x + (1 if abs(x) < abs(z) else 0), -1, z + (1 if abs(x) >= abs(z) else 0), "glowshroom")
        if i % 3 == 0:
            v.set(x, -1, z + (1 if z <= 0 else -1), "glowshroom")
    v.set(0, 0, 0, "log")
    v.set(0, -1, 0, "dirt")
    v.set(1, -1, 0, "dirt")
    return v


@recipe
def hollow_oak_limb():
    """A fallen limb from the old oak: a log lying along x with a side branch
    (4-connected), a leaf tuft, and caps in its shade. 8 x 4."""
    v = Vol("prop")
    for x in range(-3, 4):
        v.set(x, 0, 0, "log")
    for z in (1, 2):
        v.set(1, 0, z, "log")
    v.set(-4, 0, 0, "dirt")
    v.set(1, 1, 2, "leaves")
    v.set(-1, -1, -1, "glowshroom")
    v.set(-2, -1, -1, "glowshroom")
    return v


@recipe
def hollow_oak_woodcut():
    """A woodcutters' stop: a chopping stump with the axe left in it, a
    sawhorse with a half-sawn log, the sawn rounds, and a spread of chips.
    7 x 5."""
    v = Vol("prop")
    for x in range(-3, 4):
        for z in range(-2, 3):
            if x * x + z * z <= 7 and rnd(x, 0, z, 9) % 3:
                v.set(x, -1, z, "birch_planks" if rnd(x, 1, z, 9) % 2 else "dirt")
    v.set(-2, 0, 0, "log")              # chopping stump
    v.set(-2, 1, 0, "timber_beam")      # the axe handle standing in it
    for x in (0, 3):                    # sawhorse legs
        v.set(x, 0, 1, "timber_beam")
    for x in range(0, 4):               # the log across it, half cut
        v.set(x, 1, 1, "log")
    v.set(2, 0, -1, "log")              # sawn rounds
    v.set(3, 0, -1, "log")
    v.set(3, 1, -1, "log")
    return v
