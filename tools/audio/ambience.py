#!/usr/bin/env python3
"""The sounds of places: beds that go round (the air of a meadow, the
night, leaves, the barrow's moan, a town's murmur, a mine, wind, rain),
things that sound where they stand (fire, running water, a lapping
shore, an anvil) and voices heard now and then (birds, an owl, crickets,
frogs, crows, gulls, drips, gusts).

usage: python3 tools/audio/ambience.py [--only PREFIX] [--out DIR]

Writes content/sounds/amb/<name>_<n>.wav (16-bit, mono, 22050 Hz). The
beds and loops go round without a seam. Deterministic, like feedback.py.
Where and when each is heard is content/sounds/amb/ambience.json.
"""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import synth  # noqa: E402
from synth import (  # noqa: E402
    SR,
    TAU,
    bell,
    bp,
    decay,
    fade,
    gain,
    grains,
    hp,
    lfo,
    lp,
    modal,
    noise,
    norm,
    reverb,
    rng_for,
    seamless,
    secs,
    silence,
    sweep,
    times,
    tone,
    trim,
    wander,
    write_wav,
)

FOLD = 1.0  # seconds of each loop folded under its start


def mix(*parts):
    """synth.mix leaves one sample of silence after parts that start at
    once; in a loop that sample is a click, so it is dropped here."""
    return synth.mix(*parts)[:-1]


# ------------------------------------------------------------------ beds


def bed_air(r, v):
    """A meadow by day: the least breeze, and grass moving in it."""
    t = 6.0 + FOLD
    low = lp(noise(t, r), 700)
    high = bp(noise(t, r), 3800, 0.7)
    n = len(low)
    breath = wander(n, r, 0.5, 0.5, 1.0)
    stir = wander(n, r, 0.8, 0.1, 1.0)
    return seamless(mix((times(low, breath), 0, 1.0), (times(high, stir), 0, 0.22)), FOLD)


def cricket_voice(r, n, loop):
    """One cricket for a whole loop: chirps of a few quick pulses, a whole
    number of them, so the loop closes."""
    f = r.uniform(4100, 5300)
    chirps = r.randint(8, 14)
    pulses = r.randint(3, 4)
    rate = r.uniform(28, 36)
    phase = r.random()
    out = [0.0] * n
    for c in range(chirps):
        t0 = (c + phase) / chirps * loop
        for p in range(pulses):
            at = int((t0 + p / rate) * SR) % n
            m = int(SR / rate * 0.6)
            for i in range(m):
                e = math.sin(math.pi * i / m) ** 2
                out[(at + i) % n] += e * math.sin(TAU * f * i / SR)
    return out


def bed_night(r, v):
    """Night: a field of crickets, near and far, over still air."""
    loop = 6.0
    n = secs(loop)
    parts = []
    for k in range(11):
        level = r.uniform(0.15, 1.0) ** 2
        parts.append((cricket_voice(r, n, loop), 0, level))
    chorus = lp(mix(*parts), 7000)
    air = seamless(lp(noise(loop + FOLD, r), 400), FOLD)
    return mix((chorus, 0, 0.5), (air, 0, 0.5))


def bed_leaves(r, v):
    """A birch wood: leaves that hiss as the air moves through them."""
    t = 6.0 + FOLD
    n = secs(t)
    hiss = bp(noise(t, r), 3200, 0.6)
    gusts = wander(n, r, 0.45, 0.15, 1.0)
    dry = grains(t, r, 90, 0.01, 2500, 6000, 3.0)
    low = lp(noise(t, r), 500)
    return seamless(mix((times(hiss, gusts), 0, 0.8), (times(dry, gusts), 0, 0.25), (low, 0, 0.5)), FOLD)


def moan(r, n, f_lo, f_hi, q, cycles):
    """Wind in a hollow: noise through a narrow band that slides, a whole
    number of slides to the loop."""
    x = [r.uniform(-1.0, 1.0) for _ in range(n + secs(FOLD))]
    where = lfo(len(x), cycles * len(x) / n, r, f_lo, f_hi)
    out = [0.0] * len(x)
    low = band = 0.0
    damp = 1.0 / q
    for i, s in enumerate(x):
        f = 2.0 * math.sin(math.pi * where[i] / SR)
        low += f * band
        high = s - low - damp * band
        band += f * high
        out[i] = band
    return seamless(out, FOLD)


def bed_eerie(r, v):
    """The barrow wood: a low moan that never settles, and cold air."""
    n = secs(8.0)
    a = moan(r, n, 170, 300, 9.0, 2)
    b = moan(r, n, 390, 560, 12.0, 3)
    under = seamless(lp(noise(8.0 + FOLD, r), 110), FOLD)
    swell = lfo(n, 1, r, 0.5, 1.0)
    return mix((times(a, swell), 0, 0.8), (b, 0, 0.3), (under, 0, 0.9))


def talker(r, t):
    """One voice in a crowd, too far off to make out: a buzz through two
    wandering vowel bands, in syllables."""
    n = secs(t)
    pitch = r.uniform(95, 210)
    drift = wander(n, r, 2.0, 0.92, 1.1)
    ph = 0.0
    buzz = [0.0] * n
    for i in range(n):
        ph += pitch * drift[i] / SR
        ph -= math.floor(ph)
        buzz[i] = 2.0 * ph - 1.0
    f1 = wander(n, r, 3.0, 300, 800)
    f2 = wander(n, r, 3.0, 900, 2000)
    out = [0.0] * n
    for where, level in ((f1, 1.0), (f2, 0.5)):
        low = band = 0.0
        for i, s in enumerate(buzz):
            f = 2.0 * math.sin(math.pi * where[i] / SR)
            low += f * band
            high = s - low - 0.25 * band
            band += f * high
            out[i] += band * level
    # syllables, and pauses between phrases
    syll = wander(n, r, r.uniform(3.5, 5.5), -0.3, 1.0)
    phrase = wander(n, r, 0.45, -0.6, 1.0)
    return [o * max(0.0, s) * max(0.0, p) for o, s, p in zip(out, syll, phrase)]


def bed_town(r, v):
    """A town at work: people talking somewhere near, never a word."""
    t = 8.0 + FOLD
    crowd = mix(*[(talker(r, t), 0, r.uniform(0.4, 1.0)) for _ in range(7)])
    crowd = lp(crowd, 1500)
    far = reverb(crowd, 0.5, 1.3, 0.5, tail=0.0)[: secs(t)]
    air = lp(noise(t, r), 500)
    return seamless(mix((far, 0, 1.0), (air, 0, 0.12)), FOLD)


def bed_mine(r, v):
    """Underground: the weight of the hill, and air moving in a shaft."""
    n = secs(8.0)
    rumble = seamless(lp(lp(noise(8.0 + FOLD, r), 90), 90), FOLD)
    shaft = moan(r, n, 210, 260, 14.0, 1)
    swell = lfo(n, 2, r, 0.4, 1.0)
    return mix((rumble, 0, 1.0), (times(shaft, swell), 0, 0.12))


def bed_wind(r, v):
    """High ground: wind with nothing in its way."""
    n = secs(7.0)
    body = moan(r, n, 380, 950, 1.6, 2)
    whistle = moan(r, n, 1500, 2300, 11.0, 3)
    gusts = lfo(n, 2, r, 0.35, 1.0)
    return mix((times(body, gusts), 0, 1.0), (times(whistle, gusts), 0, 0.06))


def bed_rain(r, v):
    """Steady rain on leaves and ground."""
    t = 5.0 + FOLD
    sheet = lp(hp(noise(t, r), 1400), 7500)
    drops = grains(t, r, 500, 0.006, 1500, 6500, 2.5)
    low = lp(noise(t, r), 350)
    return seamless(mix((sheet, 0, 0.6), (drops, 0, 0.5), (low, 0, 0.5)), FOLD)


# ------------------------------------------------- things that stand still


def loop_fire(r, v):
    """A fire: its low breath, and wood cracking."""
    t = 5.0 + FOLD
    n = secs(t)
    roar = times(lp(noise(t, r), 320), wander(n, r, 6.0, 0.4, 1.0))
    hiss = times(bp(noise(t, r), 2400, 0.8), wander(n, r, 3.0, 0.0, 0.5))
    crack = silence(t)
    for _ in range(46):
        at = r.randrange(n - 800)
        g = decay(hp(noise(0.012, r), r.uniform(900, 2500)), r.uniform(0.004, 0.012))
        a = r.uniform(0.2, 1.0) ** 2
        for i, s in enumerate(g):
            crack[at + i] += s * a
    return seamless(mix((roar, 0, 1.0), (hiss, 0, 0.12), (crack, 0, 0.8)), FOLD)


def babble(r, t, bands, speed):
    """Moving water: bands of noise, each coming and going on its own."""
    n = secs(t)
    parts = []
    for f, level in bands:
        parts.append((times(bp(noise(t, r), f * r.uniform(0.9, 1.1), 2.5), wander(n, r, speed, 0.1, 1.0)), 0, level))
    return mix(*parts)


def loop_river(r, v):
    """A stream over stones."""
    t = 5.0 + FOLD
    water = babble(r, t, ((550, 1.0), (1000, 0.9), (1700, 0.7), (2800, 0.4), (4200, 0.2)), 8.0)
    parts = [(water, 0, 1.0)]
    for _ in range(26):  # bubbles
        f = r.uniform(500, 1400)
        parts.append((decay(tone(f, f * 1.6, 0.035, 0.7), 0.035, 0.004), r.uniform(0.0, t - 0.1), 0.2))
    return seamless(mix(*parts), FOLD)


def loop_lap(r, v):
    """A lake's edge: small waves coming in, one after another."""
    loop = 6.0
    n = secs(loop)
    out = silence(loop)
    for k in range(3):
        wave = bell(sweep(noise(1.9, r), 350, r.uniform(1300, 1900), 1.0, 0.6), 0.6, 1.3)
        wash = bell(bp(noise(1.2, r), 2600, 0.7), 0.3, 1.5)
        w = mix((wave, 0, 1.0), (wash, 1.0, 0.3))
        at = int((k + r.uniform(0.0, 0.25)) / 3.0 * n)
        for i, s in enumerate(w):
            out[(at + i) % n] += s
    still = seamless(lp(noise(loop + FOLD, r), 600), FOLD)
    return mix((out, 0, 1.0), (still, 0, 0.15))


# ------------------------------------------------------------- now and then


def chirp(f0, f1, t, r, trill=0.0, rate=0.0):
    """One note of a bird: a whistle that slides, perhaps with a trill."""
    n = secs(t)
    out = [0.0] * n
    ph = 0.0
    for i in range(n):
        u = i / n
        f = f0 * (f1 / f0) ** u
        if trill:
            f *= 1.0 + trill * math.sin(TAU * rate * i / SR)
        ph += TAU * f / SR
        out[i] = math.sin(ph) + 0.18 * math.sin(2.0 * ph)
    return bell(out, 0.35, 0.8)


def phrase(notes, r):
    """Notes one after another: (start, f0, f1, seconds, trill, rate, level)."""
    return mix(*[(chirp(f0, f1, t, r, tr, rt), at, lv) for at, f0, f1, t, tr, rt, lv in notes])


def bird(r, v):
    j = lambda f: f * r.uniform(0.96, 1.04)  # noqa: E731
    songs = [
        # three falling notes
        [(0.0, j(4200), j(3300), 0.11, 0, 0, 1.0), (0.16, j(3900), j(3000), 0.11, 0, 0, 0.9), (0.32, j(3600), j(2700), 0.15, 0, 0, 0.8)],
        # a long trill
        [(0.0, j(3600), j(3900), 0.5, 0.06, 28, 0.9)],
        # two notes: high, low
        [(0.0, j(4600), j(4500), 0.16, 0, 0, 1.0), (0.24, j(3500), j(3400), 0.28, 0, 0, 0.9)],
        # a warble up and down
        [(0.0, j(2800), j(3600), 0.12, 0, 0, 0.8), (0.13, j(3600), j(2900), 0.1, 0, 0, 0.9), (0.26, j(3000), j(4000), 0.12, 0, 0, 1.0), (0.4, j(3900), j(3200), 0.16, 0.03, 22, 0.8)],
        # rising whistles
        [(0.0, j(2600), j(3300), 0.18, 0, 0, 0.8), (0.26, j(2700), j(3500), 0.18, 0, 0, 0.9), (0.52, j(2800), j(3800), 0.22, 0, 0, 1.0)],
        # a quick twitter
        [(0.07 * k, j(4000 + 250 * (k % 3)), j(4400 + 200 * (k % 2)), 0.045, 0, 0, 0.6 + 0.07 * k) for k in range(7)],
    ]
    return reverb(lp(phrase(songs[v], r), 6500), 0.3, 1.2, 0.5, tail=0.5)


def owl(r, v):
    f = r.uniform(330, 370)
    hoots = [(0.0, 0.38), (0.62, 0.2), (0.86, 0.42)] if v != 1 else [(0.0, 0.45), (0.75, 0.5)]
    parts = []
    for at, t in hoots:
        note = mix((tone(f * 1.04, f * 0.97, t), 0, 1.0), (tone(f * 2.08, f * 1.94, t), 0, 0.12))
        breath = bp(noise(t, r), f * 2.0, 2.0)
        parts.append((bell(mix((note, 0, 1.0), (breath, 0, 0.08)), 0.3, 0.9), at, 1.0))
    return reverb(mix(*parts), 0.4, 1.4, 0.5, tail=0.8)


def cricket(r, v):
    """One cricket close by: two or three chirps."""
    f = r.uniform(4200, 5000)
    rate = r.uniform(28, 34)
    parts = []
    for c in range(2 + v % 2):
        for p in range(3):
            m = 0.6 / rate
            pulse = [math.sin(math.pi * i / secs(m)) ** 2 * math.sin(TAU * f * i / SR) for i in range(secs(m))]
            parts.append((pulse, 0.21 * c + p / rate, 1.0))
    return lp(mix(*parts), 7000)


def frog(r, v):
    f = r.uniform(62, 88)
    parts = []
    for k in range(2 if v else 1):
        t = r.uniform(0.22, 0.34)
        n = secs(t)
        pulses = [0.0] * n
        step = int(SR / f)
        for i in range(0, n, step):
            pulses[i] = 1.0
        body = mix((bp(pulses, r.uniform(520, 680), 5.0), 0, 1.0), (bp(pulses, r.uniform(1300, 1600), 6.0), 0, 0.5))
        parts.append((bell(body, 0.3, 0.7), 0.42 * k, 1.0))
    return reverb(mix(*parts), 0.2, tail=0.3)


def crow(r, v):
    parts = []
    for k in range(2 + (v % 2)):
        t = r.uniform(0.26, 0.36)
        n = secs(t)
        f0 = r.uniform(430, 500)
        ph = 0.0
        saw = [0.0] * n
        rough = wander(n, r, 70.0, 0.75, 1.25)
        for i in range(n):
            ph += f0 * (1.0 - 0.15 * i / n) * rough[i] / SR
            ph -= math.floor(ph)
            saw[i] = 2.0 * ph - 1.0
        caw = mix((bp(saw, 1400, 2.0), 0, 1.0), (bp(saw, 2600, 3.0), 0, 0.5), (bp(noise(t, r), 1800, 1.5), 0, 0.25))
        parts.append((bell(caw, 0.25, 0.6), 0.45 * k, 1.0 - 0.1 * k))
    return reverb(mix(*parts), 0.35, 1.3, 0.5, tail=0.6)


def gull(r, v):
    parts = []
    for k in range(2 + v % 2):
        f = r.uniform(1300, 1600)
        t = r.uniform(0.3, 0.5)
        n = secs(t)
        out = [0.0] * n
        ph = 0.0
        for i in range(n):
            u = i / n
            ph += TAU * f * (1.0 + 0.35 * math.sin(math.pi * u) - 0.15 * u) / SR
            out[i] = math.sin(ph) + 0.4 * math.sin(2 * ph) + 0.2 * math.sin(3 * ph)
        parts.append((bell(out, 0.3, 0.8), 0.55 * k, 1.0 - 0.15 * k))
    return reverb(lp(mix(*parts), 5500), 0.35, 1.3, 0.5, tail=0.6)


def drip(r, v):
    f = r.uniform(750, 1300)
    plink = decay(tone(f, f * 1.7, 0.05, 0.6), 0.05, 0.002)
    ring = decay(tone(f * 1.7, f * 1.65, 0.12), 0.12, 0.002)
    return reverb(mix((plink, 0, 1.0), (ring, 0.03, 0.35)), 0.5, 1.6, 0.3, tail=1.4)


def gust(r, v):
    t = r.uniform(2.4, 3.2)
    body = bell(sweep(noise(t, r), 300, r.uniform(900, 1300), 1.5, 1.0), r.uniform(0.4, 0.6), 1.2)
    leaves = bell(bp(noise(t, r), 3500, 0.7), 0.55, 2.0)
    return mix((body, 0, 1.0), (leaves, 0, 0.2))


def anvil(r, v):
    """A smith's hammer on the anvil: the blow, the ring, the small bounce."""
    f = r.uniform(0.96, 1.05)
    def strike(level):
        ring = modal(
            [(1870 * f, 1.0, 0.5), (1905 * f, 0.5, 0.45), (3120 * f, 0.7, 0.35), (4630 * f, 0.5, 0.22), (5950 * f, 0.3, 0.15)],
            0.55,
            r,
        )
        thunk = decay(tone(210, 120, 0.05), 0.05, 0.001)
        tick = decay(hp(noise(0.008, r), 2500), 0.006)
        return gain(mix((ring, 0, 0.7), (thunk, 0, 0.9), (tick, 0, 0.8)), level)
    return reverb(mix((strike(1.0), 0, 1.0), (strike(0.25), r.uniform(0.11, 0.15), 1.0)), 0.25, tail=0.4)


# (name, variants, recipe, peak, loop): for a loop, the last is the
# frequency below which it is cut (a bed must not rumble); 0 is not a loop
SOUNDS = [
    ("air", 1, bed_air, 0.5, 90),
    ("night", 1, bed_night, 0.5, 120),
    ("leaves", 1, bed_leaves, 0.5, 120),
    ("eerie", 1, bed_eerie, 0.6, 40),
    ("town", 1, bed_town, 0.5, 110),
    ("mine", 1, bed_mine, 0.6, 30),
    ("wind", 1, bed_wind, 0.6, 90),
    ("rain", 1, bed_rain, 0.5, 120),
    ("fire", 1, loop_fire, 0.6, 60),
    ("river", 1, loop_river, 0.5, 110),
    ("lap", 1, loop_lap, 0.5, 90),
    ("bird", 6, bird, 0.5, 0),
    ("owl", 3, owl, 0.5, 0),
    ("cricket", 3, cricket, 0.4, 0),
    ("frog", 3, frog, 0.5, 0),
    ("crow", 3, crow, 0.5, 0),
    ("gull", 3, gull, 0.5, 0),
    ("drip", 4, drip, 0.5, 0),
    ("gust", 3, gust, 0.5, 0),
    ("anvil", 4, anvil, 0.6, 0),
]


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    out = os.path.join(here, "..", "..", "content", "sounds", "amb")
    only = ""
    args = sys.argv[1:]
    while args:
        a = args.pop(0)
        if a == "--only":
            only = args.pop(0)
        elif a == "--out":
            out = args.pop(0)
    total = 0
    for name, variants, recipe, peak, loop in SOUNDS:
        if not name.startswith(only):
            continue
        for v in range(variants):
            r = rng_for(f"amb/{name}_{v}")
            x = recipe(r, v)
            if loop:
                # (filtered twice round, keeping the second: its ends still meet)
                x = norm(hp(hp(x + x, loop), loop)[len(x) :], peak)
            else:
                x = norm(fade(trim(hp(x, 30.0)), 0.002, 0.02), peak)
            total += write_wav(os.path.join(out, f"{name}_{v}.wav"), x)
            print(f"amb/{name}_{v}.wav  {len(x) / SR:.2f} s")
    print(f"{total} bytes")


if __name__ == "__main__":
    main()
