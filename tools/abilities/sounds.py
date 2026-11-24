#!/usr/bin/env python3
"""Synthesise the ability sounds of the pack: content/sounds/abilities/<name>_<n>.wav
(16-bit mono PCM, 16 kHz). An ability names its sound in the rules
(`"sound": "frost"`); the client plays a random variant. Standard library
only, and the same output every run.

    python3 tools/abilities/sounds.py
"""
import math, os, random, struct, wave

ROOT = os.path.join(os.path.dirname(__file__), '..', '..')
OUT = os.path.join(ROOT, 'content', 'sounds', 'abilities')
SR = 16000


def env(t, attack, decay, length):
    """Rise over `attack`, then fall off with time constant `decay`, to 0 at `length`."""
    if t < 0 or t >= length:
        return 0.0
    a = min(1.0, t / attack) if attack > 0 else 1.0
    tail = min(1.0, (length - t) / 0.02)
    return a * math.exp(-max(0.0, t - attack) / decay) * tail


def noise(rng, n):
    return [rng.uniform(-1, 1) for _ in range(n)]


def lowpass(x, cut):
    """One-pole low-pass; `cut` is a frequency or a function of time."""
    out, y = [], 0.0
    for i, v in enumerate(x):
        c = cut(i / SR) if callable(cut) else cut
        k = 1 - math.exp(-2 * math.pi * c / SR)
        y += k * (v - y)
        out.append(y)
    return out


def highpass(x, cut):
    lp = lowpass(x, cut)
    return [a - b for a, b in zip(x, lp)]


def tone(n, freq, shape=math.sin):
    """A tone; `freq` is Hz or a function of time."""
    out, ph = [], 0.0
    for i in range(n):
        f = freq(i / SR) if callable(freq) else freq
        ph += 2 * math.pi * f / SR
        out.append(shape(ph))
    return out


def mixdown(length, *layers):
    """Layers are (start seconds, gain, samples)."""
    n = int(length * SR)
    out = [0.0] * n
    for start, gain, xs in layers:
        o = int(start * SR)
        for i, v in enumerate(xs):
            if 0 <= o + i < n:
                out[o + i] += v * gain
    return out


def shaped(xs, attack, decay, length=None):
    length = length or len(xs) / SR
    return [v * env(i / SR, attack, decay, length) for i, v in enumerate(xs)]


def whoosh(rng, secs, f0, f1, attack=0.03, decay=0.12):
    n = int(secs * SR)
    x = lowpass(noise(rng, n), lambda t: f0 + (f1 - f0) * (t / secs))
    return shaped(highpass(x, 250), attack, decay)


def bell(secs, freqs, decay=0.18):
    n = int(secs * SR)
    out = [0.0] * n
    for k, f in enumerate(freqs):
        part = shaped(tone(n, f), 0.004, decay / (1 + 0.35 * k))
        out = [a + b / (1 + k * 0.5) for a, b in zip(out, part)]
    return out


def thump(secs, f0, f1, decay=0.09):
    n = int(secs * SR)
    return shaped(tone(n, lambda t: f1 + (f0 - f1) * math.exp(-t / 0.05)), 0.003, decay)


def saw(ph):
    return 2 * ((ph / (2 * math.pi)) % 1.0) - 1


# ---------------- the sounds ----------------
def slash(rng, v):
    return mixdown(0.26, (0, 0.9, whoosh(rng, 0.22, 5200 - 600 * v, 900)), (0.05, 0.25, bell(0.15, [2300 + 250 * v, 3700], 0.05)))


def stab(rng, v):
    return mixdown(0.18, (0, 0.8, whoosh(rng, 0.12, 6000, 1800 + 300 * v, 0.01, 0.05)), (0.03, 0.5, thump(0.1, 300, 140, 0.03)))


def stab_heavy(rng, v):
    return mixdown(0.3, (0, 0.8, whoosh(rng, 0.16, 4200, 900, 0.02, 0.07)), (0.07, 0.9, thump(0.2, 220, 70, 0.07)), (0.07, 0.2, bell(0.12, [1900], 0.04)))


def thud(rng, v):
    return mixdown(0.36, (0, 1.0, thump(0.34, 170 + 20 * v, 48, 0.11)), (0, 0.5, shaped(lowpass(noise(rng, int(0.08 * SR)), 1400), 0.002, 0.025)))


def charge(rng, v):
    n = int(0.34 * SR)
    rise = lowpass(noise(rng, n), lambda t: 500 + 5200 * (t / 0.34) ** 2)
    rise = [x * min(1.0, (i / n) ** 1.5 * 1.4) * (1.0 if i < n - 200 else (n - i) / 200) for i, x in enumerate(rise)]
    return mixdown(0.6, (0, 0.8, rise), (0.31, 1.0, thump(0.28, 190, 50, 0.1)))


def shout(rng, v):
    n = int(0.42 * SR)
    f = lambda t: (170 + 25 * v) * (1 + 0.25 * math.exp(-t / 0.05)) * (1 - 0.18 * t / 0.42) + 6 * math.sin(2 * math.pi * 7 * t)
    voice = tone(n, f, saw)
    # two rough formants of an open "hah"
    a = lowpass(highpass(voice, 500), 1100)
    b = lowpass(highpass(voice, 1500), 2600)
    breath = lowpass(highpass(noise(rng, n), 900), 3000)
    x = [a[i] * 1.2 + b[i] * 0.6 + breath[i] * 0.25 for i in range(n)]
    return mixdown(0.44, (0, 0.9, shaped(x, 0.02, 0.2)))


def shield(rng, v):
    return mixdown(0.6, (0, 0.6, bell(0.58, [523 + 20 * v, 830, 1318, 2100], 0.3)), (0, 0.3, whoosh(rng, 0.2, 3000, 6000, 0.05, 0.08)))


def holy_strike(rng, v):
    return mixdown(0.5, (0, 0.9, thump(0.25, 180, 60, 0.08)), (0, 0.45, whoosh(rng, 0.15, 4500, 1000, 0.01, 0.06)), (0.02, 0.5, bell(0.46, [880, 1320 + 30 * v, 1760], 0.22)))


def heal(rng, v):
    notes = [523.25, 659.25, 783.99, 1046.5]
    layers = [(0.09 * i, 0.5, bell(0.4, [f, f * 2], 0.2)) for i, f in enumerate(notes)]
    return mixdown(0.72, *layers, (0, 0.12, whoosh(rng, 0.5, 2500, 6000, 0.2, 0.2)))


def seal(rng, v):
    n = int(0.55 * SR)
    chord = [sum(math.sin(2 * math.pi * f * i / SR) for f in (392, 587.3, 784)) / 3 for i in range(n)]
    swell = [x * math.sin(math.pi * min(1.0, i / n)) ** 0.7 for i, x in enumerate(chord)]
    return mixdown(0.6, (0, 0.6, swell), (0.3, 0.4, bell(0.28, [1568, 2349], 0.12)))


def holy_burst(rng, v):
    return mixdown(0.75, (0, 0.5, whoosh(rng, 0.5, 800, 5000, 0.15, 0.2)), (0.08, 0.55, bell(0.65, [659, 988, 1319, 1976], 0.32)), (0.08, 0.6, thump(0.3, 140, 50, 0.1)))


def holy_bolt(rng, v):
    n = int(0.2 * SR)
    chirp = shaped(tone(n, lambda t: 620 + 4200 * t), 0.01, 0.09)
    return mixdown(0.42, (0, 0.6, chirp), (0.12, 0.5, bell(0.28, [1175 + 40 * v, 1760], 0.13)), (0, 0.25, whoosh(rng, 0.18, 2500, 5000, 0.02, 0.07)))


def shot(rng, v):
    n = int(0.22 * SR)
    twang = shaped(tone(n, lambda t: (210 + 20 * v) * (1 + 0.5 * math.exp(-t / 0.02)), saw), 0.002, 0.06)
    return mixdown(0.3, (0, 0.6, lowpass(twang, 1800)), (0.02, 0.7, whoosh(rng, 0.2, 6500, 2200, 0.01, 0.06)))


def shot_heavy(rng, v):
    n = int(0.3 * SR)
    twang = shaped(tone(n, lambda t: 150 * (1 + 0.6 * math.exp(-t / 0.03)), saw), 0.002, 0.09)
    return mixdown(0.45, (0, 0.7, lowpass(twang, 1400)), (0.02, 0.8, whoosh(rng, 0.34, 5500, 1200, 0.015, 0.1)), (0.0, 0.5, thump(0.15, 160, 70, 0.05)))


def snare(rng, v):
    click = lambda: shaped(highpass(noise(rng, int(0.03 * SR)), 1800), 0.001, 0.008)
    return mixdown(0.34, (0, 0.9, click()), (0.07, 1.0, click()), (0.07, 0.5, bell(0.25, [310, 742, 1190], 0.09)), (0.07, 0.6, thump(0.12, 200, 90, 0.04)))


def sneak(rng, v):
    n = int(0.5 * SR)
    x = lowpass(noise(rng, n), lambda t: 2200 * math.exp(-t / 0.2) + 200)
    return mixdown(0.52, (0, 0.9, shaped(x, 0.08, 0.2)))


def shadow(rng, v):
    n = int(0.6 * SR)
    f = lambda t: 150 * (1 - 0.35 * t / 0.6)
    a = tone(n, f)
    b = tone(n, lambda t: f(t) * 1.06)
    x = [(a[i] + b[i]) * 0.5 * (0.7 + 0.3 * math.sin(2 * math.pi * 11 * i / SR)) for i in range(n)]
    hiss = lowpass(highpass(noise(rng, n), 1500), 3500)
    return mixdown(0.62, (0, 0.8, shaped(x, 0.04, 0.3)), (0, 0.2, shaped(hiss, 0.1, 0.2)))


def fire(rng, v):
    n = int(0.5 * SR)
    x = lowpass(noise(rng, n), lambda t: 700 + 2600 * math.exp(-((t - 0.12) / 0.12) ** 2))
    crackle = [(rng.uniform(-1, 1) if rng.random() < 0.012 else 0.0) for _ in range(n)]
    return mixdown(0.52, (0, 1.0, shaped(x, 0.06, 0.2)), (0, 0.5, shaped(lowpass(crackle, 5000), 0.02, 0.3)), (0, 0.5, thump(0.25, 110 + 10 * v, 45, 0.1)))


def fire_blast(rng, v):
    n = int(0.36 * SR)
    x = lowpass(noise(rng, n), lambda t: 6000 * math.exp(-t / 0.07) + 300)
    return mixdown(0.38, (0, 1.0, shaped(x, 0.003, 0.09)), (0, 0.9, thump(0.3, 150, 40, 0.1)))


def frost(rng, v):
    return mixdown(0.4, (0, 0.4, bell(0.36, [2093 + 90 * v, 3136, 4186, 5274], 0.11)), (0, 0.5, shaped(highpass(noise(rng, int(0.22 * SR)), 4000), 0.01, 0.06)),
                   (0.0, 0.3, whoosh(rng, 0.2, 5000, 2500, 0.01, 0.07)))


def frost_burst(rng, v):
    layers = [(0.04 * i, 0.3, bell(0.3, [f, f * 1.5], 0.1)) for i, f in enumerate([2637, 2093, 3136, 2349, 3520])]
    return mixdown(0.6, *layers, (0, 0.5, shaped(highpass(noise(rng, int(0.4 * SR)), 3500), 0.02, 0.12)), (0, 0.5, thump(0.2, 130, 60, 0.07)))


def blink(rng, v):
    n = int(0.28 * SR)
    chirp = shaped(tone(n, lambda t: 500 + 2800 * math.sin(math.pi * t / 0.28)), 0.01, 0.15)
    return mixdown(0.4, (0, 0.55, chirp), (0.16, 0.3, bell(0.22, [3136, 4699], 0.07)), (0, 0.3, whoosh(rng, 0.25, 1500, 6000, 0.05, 0.1)))


# name: (maker, variants)
SOUNDS = {
    'slash': (slash, 2), 'stab': (stab, 2), 'stab_heavy': (stab_heavy, 1), 'thud': (thud, 2), 'charge': (charge, 1),
    'shout': (shout, 2), 'shield': (shield, 1), 'holy_strike': (holy_strike, 2), 'heal': (heal, 1), 'seal': (seal, 1),
    'holy_burst': (holy_burst, 1), 'holy_bolt': (holy_bolt, 2), 'shot': (shot, 2), 'shot_heavy': (shot_heavy, 1),
    'snare': (snare, 1), 'sneak': (sneak, 1), 'shadow': (shadow, 1), 'fire': (fire, 2), 'fire_blast': (fire_blast, 1),
    'frost': (frost, 2), 'frost_burst': (frost_burst, 1), 'blink': (blink, 1),
}


def main():
    os.makedirs(OUT, exist_ok=True)
    total = 0
    for name, (make, variants) in sorted(SOUNDS.items()):
        for v in range(variants):
            rng = random.Random(f'{name}/{v}')
            xs = make(rng, v)
            peak = max(1e-6, max(abs(x) for x in xs))
            data = b''.join(struct.pack('<h', int(x / peak * 0.8 * 32767)) for x in xs)
            path = os.path.join(OUT, f'{name}_{v}.wav')
            with wave.open(path, 'wb') as w:
                w.setnchannels(1)
                w.setsampwidth(2)
                w.setframerate(SR)
                w.writeframes(data)
            total += len(data)
    print(f'{sum(v for _, v in SOUNDS.values())} files, {total // 1024} KB in {os.path.relpath(OUT, ROOT)}')


if __name__ == '__main__':
    main()
