"""A small synthesis toolkit for the pack's sounds: standard library only.

Everything is a list of floats at SR samples a second, mono. Every random
choice comes from a generator seeded by the sound's name, so the same
source always writes the same bytes (pack files are named by their hash:
a drifting sound would make every client download it again).
"""

import math
import os
import random
import struct
import zlib

SR = 22050
TAU = 2.0 * math.pi


def rng_for(name):
    """The random generator of one sound, seeded by its name."""
    return random.Random(zlib.crc32(name.encode()))


def secs(t):
    return max(1, int(round(t * SR)))


def silence(t):
    return [0.0] * secs(t)


def noise(t, r):
    u = r.uniform
    return [u(-1.0, 1.0) for _ in range(secs(t))]


# ---- filters ----


def biquad(x, b0, b1, b2, a1, a2):
    y = [0.0] * len(x)
    x1 = x2 = y1 = y2 = 0.0
    for i, v in enumerate(x):
        o = b0 * v + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2
        x2, x1, y2, y1 = x1, v, y1, o
        y[i] = o
    return y


def _rbj(f, q):
    w = TAU * min(f, SR * 0.45) / SR
    return math.cos(w), math.sin(w) / (2.0 * q)


def lp(x, f, q=0.707):
    c, a = _rbj(f, q)
    a0 = 1.0 + a
    return biquad(x, (1 - c) / 2 / a0, (1 - c) / a0, (1 - c) / 2 / a0, -2 * c / a0, (1 - a) / a0)


def hp(x, f, q=0.707):
    c, a = _rbj(f, q)
    a0 = 1.0 + a
    return biquad(x, (1 + c) / 2 / a0, -(1 + c) / a0, (1 + c) / 2 / a0, -2 * c / a0, (1 - a) / a0)


def bp(x, f, q=1.0):
    c, a = _rbj(f, q)
    a0 = 1.0 + a
    return biquad(x, a / a0, 0.0, -a / a0, -2 * c / a0, (1 - a) / a0)


def sweep(x, f0, f1, q=1.5, curve=1.0, mode="band"):
    """A filter whose centre glides from f0 to f1 (a state-variable filter,
    run twice a sample so it holds up to about 7 kHz)."""
    n = len(x)
    y = [0.0] * n
    low = band = 0.0
    damp = 1.0 / q
    for i, v in enumerate(x):
        t = (i / n) ** curve
        fc = f0 * (f1 / f0) ** t
        f = 2.0 * math.sin(math.pi * min(fc, 7000.0) / (2.0 * SR))
        for _ in range(2):
            low += f * band
            high = v - low - damp * band
            band += f * high
        y[i] = band if mode == "band" else low
    return y


# ---- envelopes ----


def decay(x, t, attack=0.0):
    """Multiply by an exponential decay that falls to 1/1000 in `t` seconds,
    after a raised-cosine attack."""
    k = math.log(1000.0) / (t * SR)
    a = attack * SR
    out = [0.0] * len(x)
    for i, v in enumerate(x):
        e = math.exp(-k * i)
        if i < a:
            e *= 0.5 - 0.5 * math.cos(math.pi * i / a)
        out[i] = v * e
    return out


def bell(x, peak=0.5, sharp=2.0):
    """Multiply by a hump that rises to its top at `peak` (0..1) of the way."""
    n = len(x)
    out = [0.0] * n
    for i, v in enumerate(x):
        t = i / n
        u = t / peak if t < peak else (1.0 - t) / (1.0 - peak)
        out[i] = v * (0.5 - 0.5 * math.cos(math.pi * max(0.0, min(1.0, u)))) ** sharp
    return out


def fade(x, t_in=0.002, t_out=0.01):
    n = len(x)
    a, b = min(n, secs(t_in)), min(n, secs(t_out))
    out = list(x)
    for i in range(a):
        out[i] *= i / a
    for i in range(b):
        out[n - 1 - i] *= i / b
    return out


def gain(x, g):
    return [v * g for v in x]


# ---- sources ----


def modal(partials, t, r=None):
    """Struck things (wood, stone, metal, glass): a sum of decaying sines,
    each (frequency, level, seconds to fall to 1/1000)."""
    n = secs(t)
    out = [0.0] * n
    for f, amp, d in partials:
        if f >= SR * 0.47:
            continue
        k = math.log(1000.0) / (d * SR)
        w = TAU * f / SR
        ph = r.uniform(0.0, 0.4) if r else 0.0
        m = min(n, int(d * SR * 1.2))
        for i in range(m):
            out[i] += amp * math.exp(-k * i) * math.sin(w * i + ph)
    return out


def tone(f0, f1, t, curve=1.0):
    """A sine gliding from f0 to f1."""
    n = secs(t)
    out = [0.0] * n
    ph = 0.0
    for i in range(n):
        u = (i / n) ** curve
        ph += TAU * (f0 * (f1 / f0) ** u) / SR
        out[i] = math.sin(ph)
    return out


def pluck(f, t, r, bright=0.5, sustain=0.996):
    """A plucked string (Karplus-Strong): a lute or small harp."""
    n = secs(t)
    period = SR / f
    size = int(period)
    frac = period - size
    # the pluck: noise, softened by how bright the string is
    buf = [r.uniform(-1.0, 1.0) for _ in range(size + 1)]
    for _ in range(int((1.0 - bright) * 6)):
        buf = [(buf[i - 1] + buf[i]) * 0.5 for i in range(len(buf))]
    # (a string at rest sits at zero: without this the note carries an offset)
    mean = sum(buf) / len(buf)
    buf = [v - mean for v in buf]
    out = [0.0] * n
    last = 0.0
    j = 0
    m = len(buf)
    for i in range(n):
        a = buf[j]
        b = buf[(j + 1) % m]
        v = a + (b - a) * frac
        out[i] = v
        # the string loses its top as it rings
        new = sustain * (0.5 * v + 0.5 * last)
        last = v
        buf[j] = new
        j = (j + 1) % m
    return out


def horn(f, t, r, bright=1.0, vibrato=0.004):
    """A soft horn or reed: harmonics that open as the note swells."""
    n = secs(t)
    out = [0.0] * n
    ph = [r.uniform(0.0, TAU) for _ in range(9)]
    vib = r.uniform(4.6, 5.4)
    w = 0.0
    for i in range(n):
        u = i / n
        swell = min(1.0, u / 0.25) * min(1.0, (1.0 - u) / 0.35)
        swell = 0.5 - 0.5 * math.cos(math.pi * swell)
        # (the vibrato comes in as the note is held)
        w += TAU * f * (1.0 + vibrato * math.sin(TAU * vib * i / SR) * u) / SR
        v = 0.0
        for k in range(1, 9):
            if f * k > SR * 0.45:
                break
            v += math.sin(k * w + ph[k]) * (swell ** (1.0 + 0.45 * (k - 1) / bright)) / k**1.15
        out[i] = v
    return out


def grains(t, r, count, length, f_lo, f_hi, q=2.0, lean=1.0):
    """Many tiny filtered clicks scattered over `t`: crunch, rustle, grit."""
    out = silence(t)
    n = len(out)
    for _ in range(count):
        at = int((r.random() ** lean) * (n - 1))
        g = decay(bp(noise(length, r), r.uniform(f_lo, f_hi), q), length)
        a = r.uniform(0.4, 1.0)
        for i, v in enumerate(g):
            if at + i < n:
                out[at + i] += v * a
    return out


# ---- putting sounds together ----


def mix(*parts):
    """Sum (samples, start in seconds, level) triples."""
    n = max(secs(at) + len(x) for x, at, _ in parts)
    out = [0.0] * n
    for x, at, g in parts:
        o = secs(at) if at > 0 else 0
        for i, v in enumerate(x):
            out[o + i] += v * g
    return out


def reverb(x, wet=0.25, size=1.0, damp=0.35, tail=0.6):
    """A small room (four combs and two all-passes), for the longer cues."""
    x = list(x) + silence(tail)
    n = len(x)
    acc = [0.0] * n
    for d, fb in ((1116, 0.80), (1188, 0.79), (1277, 0.78), (1356, 0.77)):
        d = int(d * size * SR / 44100)
        buf = [0.0] * d
        j = 0
        store = 0.0
        for i in range(n):
            o = buf[j]
            store = o * (1.0 - damp) + store * damp
            buf[j] = x[i] + store * fb
            j = (j + 1) % d
            acc[i] += o
    for d in (225, 556):
        d = int(d * size * SR / 44100)
        buf = [0.0] * d
        j = 0
        for i in range(n):
            b = buf[j]
            v = acc[i]
            buf[j] = v + b * 0.5
            acc[i] = b - v * 0.5
            j = (j + 1) % d
    return [x[i] * (1.0 - wet) + acc[i] * wet * 0.25 for i in range(n)]


def norm(x, peak=0.7):
    top = max(abs(v) for v in x) or 1.0
    return [v * peak / top for v in x]


def trim(x, floor=0.0006):
    """Drop the silence at the end (and keep a few samples of it)."""
    n = len(x)
    while n > 64 and abs(x[n - 1]) < floor:
        n -= 1
    return fade(x[: min(len(x), n + 32)], 0.0, 0.004)


def finish(x, peak=0.7, t_in=0.001, t_out=0.012):
    # (nothing below 30 Hz: no offset, no thump a speaker cannot play)
    return norm(fade(trim(hp(x, 30.0)), t_in, t_out), peak)


def write_wav(path, x):
    """16-bit PCM, mono. No dither and no metadata: the same samples give
    the same file."""
    os.makedirs(os.path.dirname(path), exist_ok=True)
    data = b"".join(struct.pack("<h", int(round(max(-1.0, min(1.0, v)) * 32767.0))) for v in x)
    head = b"RIFF" + struct.pack("<I", 36 + len(data)) + b"WAVEfmt "
    head += struct.pack("<IHHIIHH", 16, 1, 1, SR, SR * 2, 2, 16)
    head += b"data" + struct.pack("<I", len(data))
    with open(path, "wb") as f:
        f.write(head + data)
    return len(head) + len(data)


def midi(n):
    return 440.0 * 2.0 ** ((n - 69) / 12.0)
