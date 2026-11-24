#!/usr/bin/env python3
"""The instruments the music is played on: short samples the client
pitches up and down, as a tracker does.

usage: python3 tools/audio/instruments.py [--out DIR]

Writes content/music/inst/<name>.wav (16-bit, mono, 22050 Hz) and prints,
for each, the note it sounds at. Held instruments (strings, flute, horn,
voices) are one seamless loop: every partial makes a whole number of
cycles in the sample, so it goes round with no seam for as long as a
note is held. Plucked and struck ones (lute, bells, drums) ring out once.
compose.py reads the notes from ROOTS below. Deterministic.
"""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from synth import (  # noqa: E402
    SR,
    TAU,
    bp,
    decay,
    fade,
    hp,
    lp,
    midi,
    mix,
    modal,
    noise,
    norm,
    pluck,
    rng_for,
    seamless,
    secs,
    tone,
    trim,
    write_wav,
)


def note_of(f):
    return 69.0 + 12.0 * math.log2(f / 440.0)


def held(r, note, cycles, partials, voices=((0, 1.0),), vibrato=0.0, vib_cycles=0, breath=0.0):
    """A held tone as one loop. `cycles` of the fundamental fit the sample
    exactly. partials: (harmonic, level). voices: (cycles sharp or flat
    over the loop, level): a second player a hair out of tune. Returns the
    samples and the note they really sound at (a whole number of samples
    is never quite the note asked for)."""
    n = int(round(cycles * SR / midi(note)))
    out = [0.0] * n
    for offset, vlevel in voices:
        for h, level in partials:
            c = h * (cycles + offset)
            if c * SR / n > SR * 0.45:
                continue
            ph = r.uniform(0.0, TAU)
            w = TAU * c / n
            if vibrato:
                # (a whole number of wobbles, so the loop still closes)
                depth = vibrato * c / max(1, vib_cycles)
                for i in range(n):
                    out[i] += vlevel * level * math.sin(w * i + ph + depth * math.sin(TAU * vib_cycles * i / n))
            else:
                for i in range(n):
                    out[i] += vlevel * level * math.sin(w * i + ph)
    if breath:
        air = seamless(bp(noise(n / SR + 0.2, r), midi(note) * 4.0, 0.8), 0.2)[:n]
        top = max(abs(v) for v in out)
        out = [a + breath * top * b for a, b in zip(out, air + [0.0] * (n - len(air)))]
    return out, note_of(cycles * SR / n)


def strings(r, note):
    partials = [(h, 1.0 / h**1.15) for h in range(1, 14)]
    return held(r, note, 220, partials, ((0, 1.0), (1, 0.7), (-1, 0.7), (2, 0.35)))


def flute(r, note):
    partials = [(1, 1.0), (2, 0.32), (3, 0.11), (4, 0.05), (5, 0.02)]
    return held(r, note, 400, partials, vibrato=1.2, vib_cycles=4, breath=0.05)


def horn(r, note):
    # a band of strength round 400-900 Hz, as a bell of brass has
    f = midi(note)
    partials = [(h, math.exp(-(((h * f) - 600.0) / 650.0) ** 2) / h**0.4 + 0.25 / h**2) for h in range(1, 13)]
    return held(r, note, 200, partials, ((0, 1.0), (1, 0.25)))


def voices(r, note):
    # "ooh": strength round 350 and 800 Hz, three singers
    f = midi(note)
    partials = []
    for h in range(1, 16):
        x = h * f
        level = math.exp(-((x - 380.0) / 180.0) ** 2) + 0.45 * math.exp(-((x - 820.0) / 200.0) ** 2) + 0.06 * math.exp(-((x - 2700.0) / 400.0) ** 2)
        partials.append((h, level + 0.02))
    return held(r, note, 300, partials, ((0, 1.0), (1, 0.8), (-1, 0.8)), vibrato=1.0, vib_cycles=5)


def lute(r, note):
    x = lp(pluck(midi(note), 2.2, r, 0.55, 0.9972), 5000)
    body = bp(x, 190.0, 1.5)
    return trim(fade(mix((x, 0, 1.0), (body, 0, 0.25)), 0.0005, 0.2)), float(note)


def bell_bar(r, note):
    f = midi(note)
    x = modal([(f, 1.0, 1.6), (f * 2.0, 0.15, 1.0), (f * 2.76, 0.3, 0.7), (f * 5.4, 0.12, 0.3)], 1.7, r)
    return trim(fade(x, 0.0005, 0.1)), float(note)


def drum_low(r):
    skin = decay(tone(92, 52, 0.7, 0.4), 0.7, 0.002)
    ring = modal([(148, 0.5, 0.35), (221, 0.25, 0.22)], 0.4, r)
    slap = decay(lp(noise(0.05, r), 900), 0.035)
    return trim(mix((skin, 0, 1.0), (ring, 0, 0.5), (slap, 0, 0.5)))


def drum_mid(r):
    skin = decay(tone(175, 118, 0.35, 0.4), 0.35, 0.002)
    ring = modal([(282, 0.5, 0.2), (410, 0.25, 0.12)], 0.25, r)
    slap = decay(lp(noise(0.04, r), 1800), 0.025)
    return trim(mix((skin, 0, 1.0), (ring, 0, 0.5), (slap, 0, 0.6)))


def shaker(r):
    return trim(decay(hp(noise(0.09, r), 3500), 0.08, 0.012))


def pick(r):
    """Iron on rock, a long way down a tunnel."""
    x = modal([(2350, 1.0, 0.3), (3890, 0.6, 0.2), (5200, 0.3, 0.1)], 0.32, r)
    tick = decay(hp(noise(0.01, r), 2000), 0.008)
    return trim(mix((x, 0, 0.8), (tick, 0, 1.0)))


# name -> (maker, the note asked for); drums have no note (played as they are)
ROOTS = {
    "strings_c3": (strings, 48),
    "strings_c4": (strings, 60),
    "flute_c5": (flute, 72),
    "flute_g5": (flute, 79),
    "horn_c3": (horn, 48),
    "horn_g3": (horn, 55),
    "voices_a3": (voices, 57),
    "voices_e4": (voices, 64),
    "lute_a2": (lute, 45),
    "lute_a3": (lute, 57),
    "lute_a4": (lute, 69),
    "bell_c5": (bell_bar, 72),
    "bell_c6": (bell_bar, 84),
}
DRUMS = {"drum_low": drum_low, "drum_mid": drum_mid, "shaker": shaker, "pick": pick}


def build(out=None):
    """Make every sample; with `out`, write them. name -> (samples, note)."""
    made = {}
    for name, (maker, note) in ROOTS.items():
        x, real = maker(rng_for("inst/" + name), note)
        made[name] = (norm(x, 0.7), real)
    for name, maker in DRUMS.items():
        made[name] = (norm(maker(rng_for("inst/" + name)), 0.8), 60.0)
    if out:
        total = 0
        for name, (x, real) in made.items():
            total += write_wav(os.path.join(out, name + ".wav"), x)
            print(f"music/inst/{name}.wav  {len(x) / SR:.2f} s  note {real:.4f}")
        print(f"{total} bytes")
    return made


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    out = os.path.join(here, "..", "..", "content", "music", "inst")
    args = sys.argv[1:]
    if args[:1] == ["--out"]:
        out = args[1]
    build(out)


if __name__ == "__main__":
    main()
