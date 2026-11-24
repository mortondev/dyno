"""Birchrise and the logging camp: recipes for the section's own stamps.

Build them with
    python3 tools/stamps/build.py birchrise_clear_6 birchrise_clear_10 ...
(the names are listed in BUILD at the bottom of this file).
"""
from build import recipe, R, ridge_tent, blob
from kit import *
import math

KEY = "birchrise"


# ------------------------------------------------ shared pieces (verbatim)

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


# ------------------------------------------------ the logging camp

@recipe
def birchrise_sawmill():
    """The camp's sawmill with a dark oak wheel rim, so the wheel reads
    against the birch-plank yard and the pale wood round it."""
    v = R["sawmill"]()
    for k, b in list(v.c.items()):
        if b == "birch_planks":
            v.c[k] = "dark_planks"
    return v


@recipe
def birchrise_tent():
    """The loggers' tent in cream and straw stripes (plain white canvas
    goes blue in the wood's shade)."""
    return ridge_tent(6, 3, ["yellow", "white"])


@recipe
def birchrise_yard():
    """A trodden dirt yard, about 22 x 16 m, flush with the ground: dirt
    with a little gravel and a ragged edge into the grass."""
    v = Vol("prop")
    for x in range(-12, 13):
        for z in range(-9, 10):
            d = (x / 11.0) ** 2 + (z / 8.0) ** 2
            j = (rnd(x, 0, z, 41) % 100) / 100
            if d <= 0.75 + 0.35 * j:
                v.set(x, -1, z, pick(x, 0, z, ["dirt", "dirt", "dirt", "dirt", "dirt", "dirt", "cobble"], 42))
    return v


@recipe
def birchrise_skid():
    """A skid road: birch cross-skids every 3 m with a felled trunk being
    dragged along them toward the mill. 3 x 2 x 13, runs along z."""
    v = Vol("prop")
    for z in range(-6, 7, 3):
        for x in (-1, 0, 1):
            v.set(x, 0, z, "birch_log")
    for z in range(-5, 3):
        v.set(0, 1, z, "log")
    for z in range(-6, 7):
        if z % 3:
            v.set(0, -1, z, "dirt")
    return v


@recipe
def birchrise_chopping():
    """A chopping block with the axe left in it, split wood stacked by it,
    and a sawhorse holding a half-cut log."""
    v = Vol("prop")
    v.set(0, 0, 0, "log")              # the block
    v.set(0, 1, 0, "timber_beam")      # the axe handle standing in it
    for (x, y, z) in ((2, 0, -1), (2, 0, 0), (2, 0, 1), (2, 1, -1), (2, 1, 0), (3, 0, 0)):
        v.set(x, y, z, "birch_planks")  # split billets
    for z in (-1, 1):
        v.set(-3, 0, z, "timber_beam")  # sawhorse legs
    for z in (-1, 0, 1):
        v.set(-3, 1, z, "birch_log")    # the log across them
    v.set(-1, -1, 1, "dirt")
    v.set(1, -1, -1, "dirt")
    return v


@recipe
def birchrise_cut():
    """The fresh cut: a dirt-and-sawdust floor with stumps and the tops
    and brash of felled birches."""
    v = Vol("prop")
    for x in range(-8, 9):
        for z in range(-7, 8):
            d = (x / 8.0) ** 2 + (z / 7.0) ** 2
            if d <= 0.6 + 0.4 * (rnd(x, 0, z, 51) % 100) / 100 and rnd(x, 0, z, 52) % 3:
                v.set(x, -1, z, pick(x, 0, z, ["dirt", "dirt", "dirt", "basalt"], 53))
    for (x, z) in ((-5, -4), (-1, -6), (4, -3), (6, 2), (-6, 3), (1, 1), (-2, 5), (3, 6)):
        v.set(x, 0, z, "birch_log")     # stumps
    v.set(1, 1, 1, "timber_beam")       # a wedge left standing in one
    for x in range(-3, 3):
        v.set(x, 0, -2, "birch_log")    # a felled trunk, lopped
    v.set(3, 0, -2, "birch_log")
    return v


# ------------------------------------------------ beast camps

@recipe
def birchrise_wallow():
    """A boar wallow: a muddy pool a metre deep in churned dark mud and dirt,
    about 13 x 11 m, flush with the ground (walkable, never a pit)."""
    v = Vol("prop")
    for x in range(-7, 8):
        for z in range(-6, 7):
            d = (x / 6.5) ** 2 + (z / 5.5) ** 2
            j = (rnd(x, 0, z, 61) % 100) / 100
            if d <= 0.32 + 0.1 * j:
                v.set(x, -1, z, "water")
                v.set(x, -2, z, "dirt")
            elif d <= 0.85 + 0.3 * j:
                v.set(x, -1, z, pick(x, 0, z, ["dirt", "dirt", "basalt", "dirt", "dirt"], 62))
            elif d <= 1.35 and rnd(x, 0, z, 63) % 3 == 0:
                v.set(x, -1, z, "dirt")
    return v


@recipe
def birchrise_bones():
    """Gnawed bones on rooted-up ground: a pale spine and ribs, a skull."""
    v = Vol("prop")
    for x in range(-3, 4):
        for z in range(-2, 3):
            if (x * x) / 9 + (z * z) / 4 <= 1 and rnd(x, 0, z, 71) % 3:
                v.set(x, -1, z, "dirt")
    v.set(-1, 0, 0, "marble")
    v.set(0, 0, 0, "marble")
    v.set(0, 0, 1, "marble")
    v.set(2, 0, -1, "marble")
    v.set(-2, 0, 1, "dirt")
    return v


@recipe
def birchrise_den():
    """A boar den: a horseshoe of tumbled stone and cobble, open toward
    +z (the camp), round a churned dirt floor with bones. 11 x 3 x 9."""
    v = Vol("prop")
    v.foundation = "stone"
    for i, a in enumerate(range(200, 341, 28)):
        r = math.radians(a)
        x, z = math.cos(r) * 4.0, math.sin(r) * 3.5
        blob(v, x, 0.6, z, 1.5 + (i % 2) * 0.4, 1.3 + (i % 3) * 0.3, "stone" if i % 2 else "cobble", 80 + i)
    v.c = {k: b for k, b in v.c.items() if k[1] >= 0}
    for x in range(-3, 4):
        for z in range(-2, 4):
            if (x, 0, z) not in v.c:
                v.set(x, -1, z, pick(x, 0, z, ["dirt", "dirt", "dirt", "basalt"], 81))
    v.set(-1, 0, 1, "marble")
    v.set(0, 0, 1, "marble")
    v.set(1, 0, 2, "log")
    return v


@recipe
def birchrise_hay_rack():
    """A shepherd's hay rack: posts and a top rail over a hay load, and a
    pale salt lick on a stump beside it."""
    v = Vol("prop")
    for x in (-2, 2):
        for z in (-1, 1):
            v.col(x, z, 0, 2, "timber_beam")
    for x in range(-2, 3):
        for z in (-1, 1):
            v.set(x, 3, z, "timber_beam")
    for x in (-1, 0, 1):
        v.set(x, 0, 0, "hay_bale")
        v.set(x, 1, 0, "hay_bale")
    v.set(0, 2, 0, "hay_bale")
    v.set(4, 0, 1, "log")
    v.set(4, 1, 1, "marble")
    for (x, z) in ((-3, 2), (-1, 2), (1, 2), (3, -2), (4, 0)):
        v.set(x, -1, z, "dirt")
    return v


@recipe
def birchrise_broken_fence():
    """A run of post-and-rail fence, broken: a post down, a rail lying in
    the grass. 13 m along x."""
    v = Vol("prop")
    for x in range(-6, 7):
        if 1 <= x <= 3:
            continue
        v.set(x, 0, 0, "timber_beam")
        if x % 3 == 0:
            v.set(x, 1, 0, "timber_beam")
    for x in (1, 2, 3):
        v.set(x, 0, 1, "timber_beam")   # the fallen rail
    return v


@recipe
def birchrise_field_wall():
    """A dry-stone field wall of cobble and stone (no moss: it goes navy in
    sun), tumbled in places. 17 m along x."""
    v = Vol("prop")
    for x in range(-8, 9):
        h = 1 + (rnd(x, 0, 0, 91) % 3 > 0)
        if rnd(x, 0, 0, 92) % 8 == 0:
            h = 0
            v.set(x, 0, 1, "cobble")
        for y in range(h):
            v.set(x, y, 0, pick(x, y, 0, ["cobble", "stone", "cobble", "stone_bricks"], 93))
    return v


# ------------------------------------------------ the North Pass

@recipe
def birchrise_crag():
    """One wall of the pass: a ridge of stepped rock 26 m long (along z),
    9 m wide and 3-9 m high, terraced, grass and stone on its ledges. It carries its
    base down to the ground (place it "fit": "float")."""
    v = Vol("prop")
    v.foundation = "stone"
    # a solid ridge: each 1 m slice along z is a stack of terraces whose
    # half-width narrows by a metre every other course
    for z in range(-12, 13):
        top = 4 + (rnd(0, 0, z // 3, 101) % 5) - (2 if abs(z) >= 11 else 0)
        sx = (rnd(0, 1, z // 4, 102) % 3) - 1
        for y in range(top + 1):
            hw = 4 - y // 2
            for x in range(-hw, hw + 1):
                v.set(x + sx, y, z, pick(x, y, z, ["stone", "stone", "cobble", "stone_bricks"], 103))
    v.c = {k: b for k, b in v.c.items() if k[1] >= 0}
    for (x, y, z) in list(v.c):
        if (x, y + 1, z) not in v.c:
            r = rnd(x, y, z, 112) % 5
            v.c[(x, y, z)] = "grass" if r < 3 else "stone"
    return v


@recipe
def birchrise_pass_tower():
    """The ruined watchtower of the pass on its own stepped rise of stone,
    three 1 m terraces (13, 11, 9 m square) so it looks down on the road."""
    v = Vol("ruin")
    v.foundation = "stone"
    for y, r in ((0, 6), (1, 5), (2, 4)):
        for x in range(-r, r + 1):
            for z in range(-r, r + 1):
                edge = max(abs(x), abs(z)) == r
                if edge and rnd(x, y, z, 121) % 4 == 0:
                    continue   # a ragged edge
                v.set(x, y, z, pick(x, y, z, ["stone", "cobble", "stone", "grass"] if not edge else ["stone", "cobble"], 122))
    t = R["ruin_tower"]()
    for (x, y, z), b in t.c.items():
        if max(abs(x), abs(z)) > 4:
            continue
        if b == "mossy_cobble" and y > 0:
            b = "cobble"
        v.set(x, y + 3, z, b)
    for d in t.doors:
        v.doors.append({"at": [d["at"][0], d["at"][1] + 3, d["at"][2]], "height": d["height"]})
    # rubble spilling down the road side (-x) in a 5 m fan
    for (x, y, z) in ((-7, 0, 0), (-7, 0, 1), (-8, 0, -1), (-6, 1, 0), (-7, 0, -2), (-5, 2, 1)):
        v.set(x, y, z, "cobble" if (x + z) % 2 else "stone_bricks")
    return v


@recipe
def birchrise_rubble():
    """Fallen masonry: a 6 m fan of blocks, highest at the back."""
    v = Vol("ruin")
    for x in range(-3, 4):
        for z in range(0, 6):
            if rnd(x, 0, z, 131) % 3 == 0 or (abs(x) + z) > 6:
                continue
            v.set(x, 0, z, pick(x, 0, z, ["cobble", "stone_bricks", "stone"], 132))
            if z <= 1 and abs(x) <= 1:
                v.set(x, 1, z, pick(x, 1, z, ["stone_bricks", "cobble"], 133))
    return v


# ------------------------------------------------ waysides and the woods

@recipe
def birchrise_cairn():
    """A trail cairn 3 m tall: a 3 x 3 course, a cross, a capstone, with a
    ring of set stones round it."""
    v = Vol("prop")
    for x in (-1, 0, 1):
        for z in (-1, 0, 1):
            v.set(x, 0, z, pick(x, 0, z, ["cobble", "stone"], 141))
    for (x, z) in ((0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)):
        v.set(x, 1, z, pick(x, 1, z, ["stone", "cobble"], 142))
    v.set(0, 2, 0, "stone_bricks")
    for (x, z) in ((3, 0), (-3, 0), (0, 3), (0, -3), (2, 2), (-2, -2)):
        v.set(x, -1, z, "cobble")
    return v


@recipe
def birchrise_log_seat():
    """A felled trunk rolled to the verge as a seat, a stump at each end."""
    v = Vol("prop")
    for x in range(-2, 3):
        v.set(x, 0, 0, "birch_log")
    v.set(-3, 0, 1, "log")
    v.set(3, 0, -1, "log")
    return v


@recipe
def birchrise_hide_rack():
    """A hunter's rack: two posts and a pole with a hide stretched flat in
    one plane, and a pelt pile at its foot."""
    v = Vol("prop")
    for x in (-2, 2):
        v.col(x, 0, 0, 2, "timber_beam")
    for x in range(-2, 3):
        v.set(x, 3, 0, "timber_beam")
    for x in (-1, 0, 1):
        for y in (1, 2):
            v.set(x, y, 0, "orange")
    v.set(0, 0, 1, "crate")
    return v


@recipe
def birchrise_blind():
    """A hunter's high seat: a 3 x 3 platform on four posts 4 m up, a rail
    round it, a thatch hood and a ladder on the open side."""
    v = Vol("prop")
    for x in (-1, 1):
        for z in (-1, 1):
            v.col(x, z, 0, 7, "timber_beam")
    for x in (-1, 0, 1):
        for z in (-1, 0, 1):
            v.set(x, 4, z, "dark_planks")
    for x in (-1, 0, 1):
        v.set(x, 5, -1, "planks")
    for z in (0,):
        v.set(-1, 5, z, "planks")
        v.set(1, 5, z, "planks")
    for x in (-2, -1, 0, 1, 2):
        for z in (-2, -1, 0, 1, 2):
            if max(abs(x), abs(z)) == 2:
                v.set(x, 7, z, "thatch")
            else:
                v.set(x, 8, z, "thatch")
    v.col(0, 2, 0, 4, "timber_beam")   # the ladder
    v.set(0, 5, 0, "lantern")
    return v


@recipe
def birchrise_charcoal_clamp():
    """A charcoal burner's clamp: a stepped earth dome smouldering at the
    top, with billets stacked ready."""
    v = Vol("prop")
    for y, r in ((0, 3.6), (1, 2.6), (2, 1.5)):
        for x in range(-4, 5):
            for z in range(-4, 5):
                if x * x + z * z <= r * r:
                    v.set(x, y, z, pick(x, y, z, ["dirt", "dirt", "basalt"], 151))
    v.set(0, 3, 0, "brazier")
    for x in range(-5, 6):
        for z in range(-5, 6):
            if 16 < x * x + z * z <= 26 and rnd(x, 0, z, 152) % 2:
                v.set(x, -1, z, "basalt" if rnd(x, 0, z, 153) % 3 == 0 else "dirt")
    for z in (-1, 0, 1):
        v.set(6, 0, z, "birch_log")
        v.set(6, 1, z, "birch_log") if z != 1 else None
    return v


BUILD = [
    "birchrise_clear_6", "birchrise_clear_10", "birchrise_clear_14", "birchrise_clear_20",
    "birchrise_sawmill", "birchrise_tent", "birchrise_yard", "birchrise_skid", "birchrise_chopping",
    "birchrise_cut", "birchrise_wallow", "birchrise_bones", "birchrise_den", "birchrise_hay_rack",
    "birchrise_broken_fence", "birchrise_field_wall", "birchrise_crag", "birchrise_pass_tower",
    "birchrise_rubble", "birchrise_cairn", "birchrise_log_seat", "birchrise_hide_rack",
    "birchrise_blind", "birchrise_charcoal_clamp",
]
