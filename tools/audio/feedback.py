#!/usr/bin/env python3
"""The feedback sounds of the pack: footsteps per ground, the interface,
and the blows of a fight. Each is built from struck wood, plucked strings,
small bells, coins, cloth and breath of air rather than from beeps.

usage: python3 tools/audio/feedback.py [--only PREFIX] [--out DIR]

Writes content/sounds/{step,ui,combat}/<name>_<n>.wav and three breaking
blocks under world/ (16-bit, mono, 22050 Hz). Deterministic: the same source writes the same bytes
(tools/audio/check.sh proves it). Which event plays which file, and how
loud, is content/audio.json.
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from synth import (  # noqa: E402
    bell,
    bp,
    decay,
    finish,
    gain,
    grains,
    horn,
    hp,
    lp,
    midi,
    mix,
    modal,
    noise,
    pluck,
    reverb,
    rng_for,
    sweep,
    tone,
    write_wav,
)

# ---------------------------------------------------------------- footsteps


def thump(r, f=95.0, t=0.07, drop=0.6):
    """The weight of a foot coming down."""
    return decay(tone(f * r.uniform(0.92, 1.08), f * drop, t), t, 0.002)


def step_grass(r, v):
    blades = decay(bp(noise(0.16, r), r.uniform(2600, 3600), 0.8), 0.15, 0.012)
    rustle = grains(0.17, r, 9, 0.012, 2500, 6500, 2.5, 0.8)
    return mix((thump(r, 85), 0, 0.5), (blades, 0.004, 0.5), (rustle, 0.01, 0.45))


def step_dirt(r, v):
    body = decay(lp(noise(0.1, r), r.uniform(650, 900)), 0.09, 0.003)
    grit = grains(0.11, r, 7, 0.008, 1800, 3800, 3.0, 0.6)
    return mix((thump(r, 100), 0, 0.8), (body, 0, 0.9), (grit, 0.012, 0.3))


def step_stone(r, v):
    f = r.uniform(0.9, 1.15)
    ring = modal([(410 * f, 1.0, 0.035), (870 * f, 0.6, 0.028), (1630 * f, 0.35, 0.02)], 0.06, r)
    click = decay(hp(noise(0.03, r), 1800), 0.018)
    scuff = decay(bp(noise(0.07, r), 2400, 1.2), 0.06, 0.01)
    return mix((thump(r, 120, 0.05), 0, 0.6), (ring, 0, 0.5), (click, 0, 0.7), (scuff, 0.03, 0.18))


def step_wood(r, v):
    f = r.uniform(0.88, 1.12)
    board = modal(
        [(165 * f, 1.0, 0.11), (392 * f, 0.55, 0.08), (735 * f, 0.3, 0.05), (1180 * f, 0.15, 0.03)],
        0.14,
        r,
    )
    knock = decay(lp(noise(0.02, r), 2500), 0.015)
    creak = decay(tone(540 * f, 610 * f, 0.07), 0.07, 0.02)
    return mix((board, 0, 0.9), (knock, 0, 0.5), (creak, 0.03, 0.04 * (v % 2)))


def step_sand(r, v):
    heel = decay(bp(noise(0.12, r), r.uniform(3200, 4200), 0.7), 0.11, 0.015)
    toe = decay(bp(noise(0.14, r), r.uniform(4200, 5600), 0.7), 0.13, 0.03)
    return mix((thump(r, 75, 0.06), 0, 0.3), (heel, 0, 0.7), (toe, 0.07, 0.5))


def step_water(r, v):
    splash = bell(sweep(noise(0.3, r), r.uniform(500, 700), r.uniform(2200, 3000), 1.2, 0.6), 0.12, 1.5)
    spray = decay(hp(noise(0.2, r), 3500), 0.18, 0.02)
    parts = [(splash, 0, 0.9), (spray, 0.03, 0.25), (thump(r, 70, 0.09), 0, 0.5)]
    for _ in range(3):  # bubbles: small rising chirps
        f = r.uniform(380, 720)
        parts.append((decay(tone(f, f * 1.7, 0.04, 0.7), 0.04, 0.004), r.uniform(0.06, 0.24), 0.22))
    return mix(*parts)


def step_snow(r, v):
    crunch = grains(0.2, r, 46, 0.006, 900, 3600, 2.2, 0.75)
    pack = decay(lp(noise(0.16, r), 1100), 0.15, 0.01)
    squeak = decay(bp(noise(0.12, r), r.uniform(1500, 1900), 9.0), 0.1, 0.03)
    return mix((thump(r, 70, 0.08), 0, 0.45), (crunch, 0, 0.6), (pack, 0, 0.6), (squeak, 0.04, 0.22))


# ---------------------------------------------------------------- interface


def coin(r, f=1.0, t=0.22):
    """One coin against another: a bright, slightly out-of-tune ring."""
    f *= r.uniform(0.94, 1.06)
    ring = modal(
        [
            (3150 * f, 1.0, t),
            (3190 * f, 0.6, t * 0.9),
            (4720 * f, 0.7, t * 0.7),
            (6230 * f, 0.5, t * 0.5),
            (8900 * f, 0.3, t * 0.3),
        ],
        t,
        r,
    )
    tick = decay(hp(noise(0.006, r), 3000), 0.005)
    return mix((ring, 0, 1.0), (tick, 0, 0.8))


def glock(note, t, r, level=1.0):
    """A small bell bar: warm, with one high partial for the sparkle."""
    f = midi(note)
    return gain(
        modal([(f, 1.0, t), (f * 2.76, 0.28, t * 0.45), (f * 5.4, 0.1, t * 0.2), (f * 2.0, 0.12, t * 0.6)], t, r),
        level,
    )


def harp(note, t, r, bright=0.55):
    return lp(pluck(midi(note), t, r, bright, 0.9965), 5200)


def ui_click(r, v):
    f = (1.0, 1.12, 0.92)[v] * r.uniform(0.98, 1.02)
    wood = modal([(880 * f, 1.0, 0.03), (1720 * f, 0.5, 0.02), (2650 * f, 0.25, 0.012)], 0.04, r)
    tick = decay(lp(noise(0.006, r), 4000), 0.004)
    return mix((wood, 0, 1.0), (tick, 0, 0.4))


def ui_open(r, v):
    # parchment drawn out, and the soft knock of a clasp
    page = lp(bell(sweep(noise(0.22, r), 600, r.uniform(2200, 2700), 1.8, 0.7), 0.55, 1.2), 3600)
    clasp = modal([(310, 1.0, 0.05), (640, 0.4, 0.03)], 0.06, r)
    crackle = grains(0.2, r, 10, 0.01, 1800, 3800, 2.5)
    return mix((page, 0, 0.8), (clasp, 0.0, 0.45), (crackle, 0.02, 0.2))


def ui_close(r, v):
    page = lp(bell(sweep(noise(0.18, r), r.uniform(2100, 2600), 500, 1.8, 0.8), 0.3, 1.2), 3400)
    thud = decay(tone(170, 110, 0.07), 0.07, 0.003)
    clasp = modal([(280, 1.0, 0.05), (590, 0.4, 0.03)], 0.06, r)
    return mix((page, 0, 0.7), (thud, 0.11, 0.6), (clasp, 0.11, 0.3))


def ui_item_pick(r, v):
    cloth = lp(bell(bp(noise(0.11, r), r.uniform(1700, 2300), 1.2), 0.3, 1.5), 4000)
    f = r.uniform(0.9, 1.1)
    clink = modal([(2150 * f, 1.0, 0.07), (3420 * f, 0.5, 0.05)], 0.08, r)
    return mix((cloth, 0, 0.8), (clink, 0.045, 0.22))


def ui_item_drop(r, v):
    thud = decay(tone(r.uniform(150, 175), 95, 0.08), 0.08, 0.002)
    leather = decay(lp(noise(0.07, r), 1400), 0.06, 0.003)
    cloth = bell(bp(noise(0.09, r), 2000, 0.8), 0.2, 1.5)
    return mix((thud, 0, 0.9), (leather, 0, 0.5), (cloth, 0.01, 0.25))


def ui_coin(r, v):
    return mix((coin(r, 1.0), 0, 0.9), (coin(r, 1.19), r.uniform(0.055, 0.075), 0.7))


def ui_buy(r, v):
    # coins counted out of the purse, falling in pitch; the purse drawn shut
    parts = [(coin(r, 1.25 - 0.09 * i, 0.16), 0.065 * i + r.uniform(0, 0.012), 0.8 - 0.1 * i) for i in range(4)]
    pouch = bell(bp(noise(0.12, r), 1500, 0.8), 0.4, 1.5)
    return mix(*parts, (pouch, 0.26, 0.35))


def ui_sell(r, v):
    # coins dropped into the purse, rising; the last one settles
    parts = [(coin(r, 0.9 + 0.1 * i, 0.16), 0.06 * i + r.uniform(0, 0.012), 0.6 + 0.1 * i) for i in range(4)]
    purse = decay(lp(noise(0.06, r), 1200), 0.05)
    return mix((purse, 0, 0.5), *parts, (coin(r, 1.35, 0.3), 0.27, 0.7))


def ui_loot(r, v):
    root = (74, 76, 71)[v]
    a = harp(root, 0.5, r)
    b = harp(root + 5, 0.55, r)
    shine = glock(root + 17, 0.4, r, 0.25)
    cloth = bell(bp(noise(0.1, r), 2400, 0.8), 0.3, 1.5)
    return reverb(mix((cloth, 0, 0.3), (a, 0.03, 0.8), (b, 0.11, 0.8), (shine, 0.12, 1.0)), 0.18, tail=0.3)


def ui_error(r, v):
    # two dull knocks on wood: "no", without a buzzer
    f = (1.0, 0.94, 1.06)[v]
    k1 = lp(modal([(196 * f, 1.0, 0.09), (410 * f, 0.4, 0.05)], 0.1, r), 900)
    k2 = lp(modal([(165 * f, 1.0, 0.11), (350 * f, 0.4, 0.06)], 0.12, r), 800)
    tap = decay(lp(noise(0.01, r), 1500), 0.008)
    return mix((k1, 0, 0.9), (tap, 0, 0.3), (k2, 0.11, 1.0), (tap, 0.11, 0.3))


def ui_quest_step(r, v):
    note = (81, 83, 86)[v]
    return reverb(mix((glock(note, 0.45, r), 0, 0.9), (harp(note - 12, 0.4, r), 0, 0.35)), 0.2, tail=0.25)


def ui_quest_accept(r, v):
    # a question asked: two plucked notes rising a fifth, a low string under
    # (each variant is the same phrase played again: other strings, other
    # timing, the same notes)
    low = lp(pluck(midi(50), 0.9, r, 0.4, 0.997), 2500)
    at = 0.16 + r.uniform(-0.015, 0.02)
    return reverb(
        mix((low, 0, 0.6), (harp(62, 0.7, r), 0.0, 0.8), (harp(69, 0.9, r), at, 0.9), (glock(81, 0.6, r, 0.3), at, 1.0)),
        0.24,
        tail=0.5,
    )


def ui_quest_ready(r, v):
    # the work is done: three notes up the chord
    parts = [(harp(n, 0.8, r), 0.11 * i + r.uniform(0, 0.015), 0.8) for i, n in enumerate((62, 66, 69))]
    return reverb(mix(*parts, (glock(81, 0.7, r, 0.35), 0.23, 1.0)), 0.24, tail=0.5)


def ui_quest_complete(r, v):
    # the answer: a rolled chord, a bell on top, a warm horn fifth under it
    chord = (50, 57, 62, 66, 69, 74)
    parts = [(harp(n, 1.5, r), 0.055 * i + r.uniform(0, 0.012), 0.7) for i, n in enumerate(chord)]
    horns = mix((horn(midi(50), 1.5, r, 0.8), 0, 0.5), (horn(midi(57), 1.5, r, 0.8), 0, 0.4), (horn(midi(62), 1.5, r, 0.7), 0, 0.3))
    bells = mix((glock(86, 1.2, r), 0.33, 0.5), (glock(90, 1.0, r), 0.45, 0.35))
    return reverb(mix(*parts, (lp(horns, 1800), 0.05, 0.45), (bells, 0, 1.0)), 0.3, tail=0.9)


def ui_level_up(r, v):
    # a rise through two octaves into a held bright chord, with horns
    run = (50, 57, 62, 66, 69, 74, 78, 81)
    parts = [(harp(n, 1.2, r, 0.65), 0.07 * i, 0.55 + 0.03 * i) for i, n in enumerate(run)]
    at = 0.07 * len(run)
    for n, g in ((62, 0.5), (69, 0.42), (74, 0.36), (78, 0.28)):
        parts.append((lp(horn(midi(n), 1.9, r, 1.1), 2600), at - 0.25, g * 0.6))
    for i, n in enumerate((86, 90, 93, 98)):
        parts.append((glock(n, 1.4, r), at + 0.09 * i, 0.4))
    drum = decay(tone(82, 55, 0.3), 0.3, 0.003)
    parts.append((drum, at, 0.9))
    return reverb(mix(*parts), 0.32, 1.1, tail=1.2)


def ui_ability_new(r, v):
    # something learned: a glassy rise, then one sure plucked note
    parts = []
    for i, n in enumerate((69, 74, 76, 81, 86)):
        parts.append((glock(n + 12, 0.7, r), 0.06 * i, 0.3 + 0.05 * i))
    air = lp(bell(sweep(noise(0.5, r), 1200, 3600, 2.5, 1.0), 0.75, 1.5), 4500)
    parts.append((air, 0, 0.1))
    parts.append((harp(62, 1.0, r), 0.32, 0.9))
    parts.append((harp(74, 1.0, r), 0.32, 0.6))
    return reverb(mix(*parts), 0.3, tail=0.8)


# ---------------------------------------------------------------- combat


def whoosh(r, t, f_lo, f_top, q=1.3, peak=0.55):
    up = sweep(noise(t, r), f_lo, f_top, q, 0.6)
    return bell(up, peak, 1.6)


def combat_swing(r, v):
    t = r.uniform(0.2, 0.26)
    # (the air a blade moves is mostly below 4 kHz: above that it hisses)
    air = lp(whoosh(r, t, r.uniform(350, 500), r.uniform(1500, 2000), 2.0), 3800)
    edge = bell(bp(noise(t, r), r.uniform(3000, 3600), 4.0), 0.6, 3.0)
    return mix((air, 0, 0.9), (edge, 0, 0.12))


def combat_shoot(r, v):
    f = r.uniform(96, 118)
    string = lp(pluck(f, 0.3, r, 0.7, 0.985), 2200)
    limb = modal([(240, 1.0, 0.06), (530, 0.5, 0.04)], 0.07, r)
    arrow = bell(sweep(noise(0.2, r), 1200, r.uniform(3800, 4600), 2.0, 0.5), 0.2, 2.0)
    return mix((string, 0, 0.9), (limb, 0, 0.5), (arrow, 0.015, 0.45))


def body_blow(r, weight=1.0):
    thud = decay(tone(125 * r.uniform(0.9, 1.1), 55, 0.13 * weight, 0.6), 0.13 * weight, 0.001)
    smack = decay(bp(noise(0.12 * weight, r), r.uniform(750, 1050), 0.7), 0.1 * weight)
    crunch = decay(lp(noise(0.07, r), 2600), 0.05)
    snap = decay(lp(noise(0.03, r), 4200), 0.018)
    return mix((thud, 0, 1.0), (smack, 0, 1.0), (crunch, 0.004, 0.6), (snap, 0, 0.5))


def combat_hit(r, v):
    blade = modal([(1450 * r.uniform(0.9, 1.1), 1.0, 0.05), (2380 * r.uniform(0.9, 1.1), 0.6, 0.035)], 0.06, r)
    return mix((body_blow(r), 0, 1.0), (blade, 0.002, 0.14))


def combat_crit(r, v):
    f = r.uniform(0.92, 1.08)
    ring = modal(
        [(1180 * f, 1.0, 0.34), (1195 * f, 0.6, 0.3), (1930 * f, 0.6, 0.24), (2890 * f, 0.4, 0.16), (4350 * f, 0.2, 0.09)],
        0.36,
        r,
    )
    sub = decay(tone(85, 42, 0.22, 0.5), 0.22, 0.002)
    crack = decay(hp(noise(0.03, r), 2500), 0.02)
    lead = whoosh(r, 0.07, 800, 3200, 1.2, 0.8)
    return mix((lead, 0, 0.35), (body_blow(r, 1.3), 0.05, 1.0), (sub, 0.05, 0.9), (crack, 0.05, 0.5), (ring, 0.052, 0.3))


def combat_hurt(r, v):
    # a blow taken: dull, close, with the rattle of what you wear
    blow = lp(body_blow(r, 1.2), 1600)
    rattle = grains(0.12, r, 8, 0.012, 1800, 3600, 5.0, 0.5)
    breath = bell(bp(noise(0.14, r), r.uniform(650, 850), 2.5), 0.25, 1.5)
    return mix((blow, 0, 1.0), (rattle, 0.01, 0.2), (breath, 0.03, 0.3))


def combat_heal(r, v):
    root = (69, 71, 67)[v]
    parts = []
    for i, n in enumerate((root, root + 4, root + 7, root + 12)):
        f = midi(n)
        note = bell(mix((tone(f, f, 0.6), 0, 1.0), (tone(f * 2, f * 2, 0.6), 0, 0.25)), 0.25, 1.0)
        parts.append((note, 0.07 * i, 0.4))
    air = lp(bell(sweep(noise(0.7, r), 1500, 4200, 2.5, 1.0), 0.6, 1.5), 5000)
    parts.append((air, 0, 0.08))
    parts.append((glock(root + 24, 0.5, r), 0.26, 0.2))
    return reverb(mix(*parts), 0.3, tail=0.5)


def combat_kill(r, v):
    # a creature goes down: the fall, and the breath that leaves it
    fall = lp(body_blow(r, 1.6), 1200)
    scuff = decay(lp(noise(0.16, r), 900), 0.15, 0.02)
    sigh = lp(bell(sweep(noise(0.5, r), r.uniform(1400, 1800), 350, 2.2, 0.8), 0.2, 1.3), 3000)
    return mix((fall, 0, 1.0), (scuff, 0.05, 0.4), (sigh, 0.06, 0.35))


def combat_death(r, v):
    # your own fall: a low drum, a falling pair of notes, a long breath out
    drum = decay(tone(70, 38, 0.6, 0.5), 0.6, 0.003)
    skin = decay(lp(noise(0.05, r), 700), 0.04)
    parts = [(drum, 0, 1.0), (skin, 0, 0.5)]
    for i, n in enumerate((62, 58, 53)):
        parts.append((lp(horn(midi(n - 12), 1.2, r, 0.6), 900), 0.1 + 0.42 * i, 0.5))
        parts.append((glock(n + 12, 1.2, r), 0.1 + 0.42 * i, 0.16))
    out = bell(sweep(noise(1.6, r), 900, 200, 1.2, 0.7), 0.15, 1.0)
    parts.append((out, 0.05, 0.14))
    return reverb(mix(*parts), 0.35, 1.2, tail=1.2)


def combat_bolt(r, v):
    f = r.uniform(0.9, 1.1)
    zip_ = bell(mix((tone(1500 * f, 320 * f, 0.26, 0.6), 0, 0.5), (tone(2250 * f, 480 * f, 0.26, 0.6), 0, 0.25)), 0.15, 1.2)
    air = lp(bell(sweep(noise(0.28, r), 2600, 600, 2.2, 0.7), 0.2, 1.3), 4200)
    land = decay(tone(140, 70, 0.08), 0.08, 0.002)
    return mix((zip_, 0, 0.45), (air, 0, 0.7), (land, 0.2, 0.5))


def combat_burst(r, v):
    boom = decay(tone(95 * r.uniform(0.9, 1.1), 38, 0.4, 0.5), 0.4, 0.004)
    body = decay(lp(noise(0.35, r), 600), 0.3, 0.004)
    ring = lp(bell(sweep(noise(0.55, r), 400, r.uniform(2400, 3000), 1.6, 0.5), 0.12, 1.2), 4000)
    f = r.uniform(0.9, 1.1)
    shimmer = modal([(1320 * f, 1.0, 0.4), (1980 * f, 0.6, 0.3), (2640 * f, 0.3, 0.2)], 0.4, r)
    return reverb(mix((boom, 0, 1.0), (body, 0, 0.6), (ring, 0, 0.35), (shimmer, 0.03, 0.08)), 0.2, tail=0.4)


# ---------------------------------------------------------------- blocks
# (the older breaking sounds of these three were clipped; the rest of
# sounds/world/ is from the first bake and waits its turn)


def break_stone(r, v):
    crack = decay(hp(noise(0.03, r), 1200), 0.02)
    f = r.uniform(0.9, 1.1)
    block = modal([(330 * f, 1.0, 0.07), (710 * f, 0.6, 0.05), (1240 * f, 0.3, 0.03)], 0.08, r)
    rubble = grains(0.22, r, 16, 0.014, 400, 2400, 2.0, 0.6)
    return mix((thump(r, 90, 0.1), 0, 0.9), (crack, 0, 0.8), (block, 0, 0.6), (rubble, 0.02, 0.5))


def break_glass(r, v):
    f = r.uniform(0.9, 1.12)
    ring = modal(
        [(2100 * f, 1.0, 0.16), (3350 * f, 0.8, 0.13), (4900 * f, 0.6, 0.1), (6700 * f, 0.4, 0.07)],
        0.18,
        r,
    )
    crack = decay(hp(noise(0.02, r), 2500), 0.012)
    shards = grains(0.28, r, 26, 0.008, 3000, 7000, 6.0, 0.55)
    return mix((crack, 0, 0.9), (ring, 0, 0.5), (shards, 0.01, 0.6))


def break_foliage(r, v):
    snap = modal([(620 * r.uniform(0.9, 1.1), 1.0, 0.03), (1350, 0.5, 0.02)], 0.04, r)
    leaves = lp(grains(0.2, r, 28, 0.012, 1500, 4500, 2.0, 0.6), 5000)
    brush = bell(bp(noise(0.16, r), r.uniform(2200, 2800), 0.8), 0.25, 1.4)
    return mix((snap, 0, 0.6), (leaves, 0, 0.7), (brush, 0, 0.35))


# (directory, name, variants, recipe, peak): the peak is the file's own
# level; how loud an event plays is audio.json's business
SOUNDS = [
    ("step", "grass", 4, step_grass, 0.6),
    ("step", "dirt", 4, step_dirt, 0.6),
    ("step", "stone", 4, step_stone, 0.6),
    ("step", "wood", 4, step_wood, 0.6),
    ("step", "sand", 4, step_sand, 0.6),
    ("step", "water", 4, step_water, 0.6),
    ("step", "snow", 4, step_snow, 0.6),
    ("ui", "click", 3, ui_click, 0.6),
    ("ui", "open", 3, ui_open, 0.6),
    ("ui", "close", 3, ui_close, 0.6),
    ("ui", "item_pick", 3, ui_item_pick, 0.6),
    ("ui", "item_drop", 3, ui_item_drop, 0.6),
    ("ui", "coin", 3, ui_coin, 0.6),
    ("ui", "buy", 3, ui_buy, 0.6),
    ("ui", "sell", 3, ui_sell, 0.6),
    ("ui", "loot", 3, ui_loot, 0.7),
    ("ui", "error", 3, ui_error, 0.6),
    ("ui", "quest_step", 3, ui_quest_step, 0.6),
    ("ui", "quest_accept", 3, ui_quest_accept, 0.7),
    ("ui", "quest_ready", 3, ui_quest_ready, 0.7),
    ("ui", "quest_complete", 3, ui_quest_complete, 0.8),
    ("ui", "level_up", 1, ui_level_up, 0.8),
    ("ui", "ability_new", 1, ui_ability_new, 0.7),
    ("combat", "swing", 4, combat_swing, 0.6),
    ("combat", "shoot", 3, combat_shoot, 0.7),
    ("combat", "hit", 4, combat_hit, 0.8),
    ("combat", "crit", 3, combat_crit, 0.9),
    ("combat", "hurt", 3, combat_hurt, 0.8),
    ("combat", "heal", 3, combat_heal, 0.7),
    ("combat", "kill", 3, combat_kill, 0.8),
    ("combat", "death", 1, combat_death, 0.8),
    ("combat", "bolt", 3, combat_bolt, 0.7),
    ("combat", "burst", 3, combat_burst, 0.8),
    ("world", "break_stone", 4, break_stone, 0.8),
    ("world", "break_glass", 4, break_glass, 0.8),
    ("world", "break_foliage", 4, break_foliage, 0.7),
]


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    out = os.path.join(here, "..", "..", "content", "sounds")
    only = ""
    args = sys.argv[1:]
    while args:
        a = args.pop(0)
        if a == "--only":
            only = args.pop(0)
        elif a == "--out":
            out = args.pop(0)
    total = 0
    for folder, name, variants, recipe, peak in SOUNDS:
        if not f"{folder}/{name}".startswith(only):
            continue
        for v in range(variants):
            r = rng_for(f"{folder}/{name}_{v}")
            x = finish(recipe(r, v), peak)
            total += write_wav(os.path.join(out, folder, f"{name}_{v}.wav"), x)
            print(f"{folder}/{name}_{v}.wav  {len(x) / 22050:.2f} s")
    print(f"{total} bytes")


if __name__ == "__main__":
    main()
