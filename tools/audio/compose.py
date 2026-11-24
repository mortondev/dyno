#!/usr/bin/env python3
"""The music, written down: each theme is a small file of patterns (which
instrument plays which note on which row) that the client plays on the
sampled instruments of instruments.py, as a tracker does.

usage: python3 tools/audio/compose.py [--out DIR]

Writes content/music/<name>.json and content/music/music.json (which
place has which theme). Rows are sixteenths: four to the beat.

The valley has one tune, in D: it is the title's theme in full dress and
Hearthstead's on lute and flute. The other places have their own moods:
Birchrise a slow waltz in E minor, the Barrow wood a drone that will not
resolve, Mirrormere slow harp in F with a raised fourth, the mines a low
C minor march to the sound of picks.
"""

import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from instruments import ROOTS  # noqa: E402

NAMES = {"C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11}


def n(name):
    """'F#4' -> 66 (MIDI numbers; C4 is 60)."""
    semis = NAMES[name[0]]
    rest = name[1:]
    if rest[0] == "#":
        semis, rest = semis + 1, rest[1:]
    elif rest[0] == "b":
        semis, rest = semis - 1, rest[1:]
    return 12 * (int(rest) + 1) + semis


def inst(*samples, looped=False, attack=0.004, release=0.15, gain=1.0, pan=0.0, room=1.0):
    zones = [{"sample": f"music/inst/{s}.wav", "root": ROOTS[s][1] if s in ROOTS else 60} for s in samples]
    return {"zones": zones, "looped": looped, "attack": attack, "release": release, "gain": gain, "pan": pan, "room": room}


# the band: every theme picks from these (and may change a level)
def band(**levels):
    b = {
        "lute": inst("lute_a2", "lute_a3", "lute_a4", release=0.4, gain=0.5, pan=-0.25),
        "harp": inst("lute_a3", "lute_a4", release=0.8, gain=0.42, pan=0.3),
        "flute": inst("flute_c5", "flute_g5", looped=True, attack=0.06, release=0.25, gain=0.3, pan=0.2),
        "strings": inst("strings_c3", "strings_c4", looped=True, attack=0.45, release=0.9, gain=0.16, pan=-0.1),
        "bass": inst("strings_c3", looped=True, attack=0.2, release=0.6, gain=0.22),
        "horn": inst("horn_c3", "horn_g3", looped=True, attack=0.09, release=0.3, gain=0.26, pan=0.1),
        "voices": inst("voices_a3", "voices_e4", looped=True, attack=0.7, release=1.4, gain=0.2),
        "bell": inst("bell_c5", "bell_c6", release=1.2, gain=0.22, pan=0.35),
        "drum": inst("drum_low", release=0.5, gain=0.6, room=0.6),
        "tom": inst("drum_mid", release=0.3, gain=0.45, pan=-0.2, room=0.6),
        "shaker": inst("shaker", release=0.05, gain=0.2, pan=0.4, room=0.3),
        "pick": inst("pick", release=0.3, gain=0.25, pan=0.5, room=1.5),
    }
    for k, v in levels.items():
        b[k] = dict(b[k], gain=b[k]["gain"] * v)
    return b


class Pattern:
    def __init__(self, bars, beats=4):
        self.bar = beats * 4
        self.rows = bars * self.bar
        self.notes = []

    def note(self, who, row, pitch, rows, level=0.8):
        p = n(pitch) if isinstance(pitch, str) else pitch
        self.notes.append([row, who, p, rows, round(level, 3)])
        return self

    def line(self, who, row, text, level=0.8):
        """A melody: 'F#4:8 A4:4 -:4' is a note for 8 rows, one for 4, a
        rest of 4. Bars may be marked with | for the eye."""
        for word in text.replace("|", " ").split():
            pitch, rows = word.split(":")
            rows = float(rows)
            if pitch != "-":
                self.note(who, row, pitch, rows * 0.95, level)
            row += rows
        return self

    def chord(self, who, row, pitches, rows, level=0.6):
        for p in pitches.split():
            self.note(who, row, p, rows, level)
        return self

    def arp(self, who, bar, pitches, step=2, level=0.6, rows=6):
        """Chord notes one after another through a bar, round and round."""
        ps = pitches.split()
        for k in range(self.bar // step):
            # (the first of the bar a little firmer)
            self.note(who, bar * self.bar + k * step, ps[k % len(ps)], rows, level * (1.0 if k else 1.15))
        return self

    def hits(self, who, bar, rows, level=0.8, soft=None):
        for r in rows:
            lv = level * (soft.get(r, 1.0) if soft else 1.0)
            self.note(who, bar * self.bar + r, 60, 2, lv)
        return self

    def data(self):
        return {"rows": self.rows, "notes": sorted(self.notes, key=lambda x: (x[0], x[1], x[2]))}


def song(bpm, patterns, order, used, gain=1.0, reverb=0.3, looped=True, **levels):
    b = band(**levels)
    return {
        "bpm": bpm,
        "rows_per_beat": 4,
        "gain": gain,
        "reverb": reverb,
        "looped": looped,
        "instruments": {k: b[k] for k in used},
        "patterns": {k: p.data() for k, p in patterns.items()},
        "order": order,
    }


# ------------------------------------------------------------ the valley's tune

# four bars each; the tune in D, as the flute plays it in Hearthstead
TUNE_A = "F#4:8 A4:4 D5:4 | B4:8 G4:8 | A4:4 F#4:4 D4:8 | E4:12 -:4"
TUNE_A2 = "F#4:8 A4:4 D5:4 | E5:4 D5:4 B4:8 | A4:4 C#5:4 E5:4 C#5:4 | D5:14 -:2"
TUNE_B = "D5:4 C#5:4 B4:8 | B4:4 A4:4 G4:8 | F#4:4 A4:4 D5:8 | C#5:8 E5:8"
CHORDS_A = [("D3 A3 D4 F#4", "D2"), ("G3 B3 D4 G4", "G2"), ("D3 A3 D4 F#4", "D2"), ("A3 C#4 E4 A4", "A2")]
CHORDS_A2 = [("D3 A3 D4 F#4", "D2"), ("G3 B3 D4 G4", "G2"), ("A3 C#4 E4 A4", "A2"), ("D3 A3 D4 F#4", "D2")]
CHORDS_B = [("B2 F#3 B3 D4", "B2"), ("G3 B3 D4 G4", "G2"), ("D3 A3 D4 F#4", "F#2"), ("A3 C#4 E4 A4", "A2")]


def up(text, semis=12):
    """A line an octave (or so) higher."""
    out = []
    for word in text.split():
        if word == "|" or word.startswith("-"):
            out.append(word)
        else:
            pitch, rows = word.split(":")
            m = n(pitch) + semis
            out.append(f"{'C C# D D# E F F# G G# A A# B'.split()[m % 12]}{m // 12 - 1}:{rows}")
    return " ".join(out)


def hearthstead():
    def part(chords, tune, flute=True, drum=True):
        p = Pattern(4)
        for bar, (chord, bass) in enumerate(chords):
            p.arp("lute", bar, chord + " " + chord.split()[2] + " " + chord.split()[1], level=0.55)
            p.chord("strings", bar * 16, " ".join(chord.split()[:3]), 16, 0.5)
            p.note("bass", bar * 16, bass, 15, 0.6)
            if drum:
                p.hits("drum", bar, [0, 10], 0.45, {10: 0.6})
        if flute:
            p.line("flute", 0, tune, 0.75)
        return p

    patterns = {
        "intro": part(CHORDS_A, TUNE_A, flute=False, drum=False),
        "a": part(CHORDS_A, TUNE_A),
        "a2": part(CHORDS_A2, TUNE_A2),
        "b": part(CHORDS_B, TUNE_B),
    }
    return song(84, patterns, ["intro", "a", "a2", "b", "a2"], ["lute", "flute", "strings", "bass", "drum"], reverb=0.3)


def title():
    def part(chords, tune, lead, high=False, drums=True):
        p = Pattern(4)
        for bar, (chord, bass) in enumerate(chords):
            p.arp("lute", bar, chord + " " + chord.split()[2] + " " + chord.split()[1], level=0.5)
            p.chord("strings", bar * 16, chord, 16, 0.6)
            p.note("bass", bar * 16, bass, 15, 0.8)
            if drums:
                p.hits("drum", bar, [0, 8], 0.7, {8: 0.6})
                p.hits("tom", bar, [6, 14], 0.4)
        p.line(lead, 0, up(tune, -12) if lead == "horn" else tune, 0.85)
        if high:
            p.line("flute", 0, up(tune), 0.6)
            for bar in range(4):
                p.note("bell", bar * 16, chords[bar][0].split()[3], 12, 0.5)
        return p

    swell = Pattern(2)
    swell.chord("strings", 0, "D3 A3 D4", 32, 0.6).note("bass", 0, "D2", 32, 0.7)
    swell.hits("tom", 1, [8, 10, 12, 13, 14, 15], 0.35, {12: 1.2, 13: 1.4, 14: 1.6, 15: 1.8})
    close = Pattern(2)
    close.chord("strings", 0, "D3 A3 D4 F#4", 30, 0.6).note("bass", 0, "D2", 30, 0.8)
    close.chord("horn", 0, "D3 A3", 24, 0.6).hits("drum", 0, [0], 0.8).note("bell", 0, "D5", 16, 0.6)
    patterns = {
        "swell": swell,
        "a": part(CHORDS_A, TUNE_A, "horn"),
        "a2": part(CHORDS_A2, TUNE_A2, "horn", high=True),
        "b": part(CHORDS_B, TUNE_B, "flute"),
        "a3": part(CHORDS_A2, TUNE_A2, "horn", high=True),
        "close": close,
    }
    used = ["lute", "flute", "strings", "bass", "horn", "bell", "drum", "tom"]
    return song(80, patterns, ["swell", "a", "a2", "b", "a3", "close"], used, reverb=0.38)


# ------------------------------------------------------------------ Birchrise


def birchrise():
    # three beats to the bar: a slow waltz, E minor with a bright sixth
    def part(chords, tune, flute=True):
        p = Pattern(4, beats=3)
        for bar, (low, pair) in enumerate(chords):
            r = bar * 12
            p.note("lute", r, low, 10, 0.65)
            p.chord("lute", r + 4, pair, 6, 0.4)
            p.chord("lute", r + 8, pair, 6, 0.35)
            p.chord("strings", r, low.replace("2", "3") + " " + pair, 12, 0.45)
        if flute:
            p.line("flute", 0, tune, 0.7)
        p.note("bell", 0, "B5", 12, 0.35)
        return p

    a = [("E2", "G3 B3"), ("D2", "F#3 A3"), ("C2", "E3 G3"), ("B2", "D3 F#3")]
    a2 = [("E2", "G3 B3"), ("G2", "B3 D4"), ("D2", "F#3 A3"), ("E2", "G3 B3")]
    b = [("A2", "C4 E4"), ("E2", "G3 B3"), ("D2", "F#3 A3"), ("B2", "D#3 F#3")]
    patterns = {
        "intro": part(a, "", flute=False),
        "a": part(a, "E4:8 G4:4 | F#4:8 D4:4 | E4:4 G4:4 C5:4 | B4:12"),
        "a2": part(a2, "B4:8 G4:4 | B4:4 A4:4 G4:4 | F#4:8 A4:4 | E4:12"),
        "b": part(b, "C5:8 A4:4 | B4:8 G4:4 | A4:4 F#4:4 D4:4 | F#4:8 D#4:4"),
    }
    return song(72, patterns, ["intro", "a", "a2", "b", "a2"], ["lute", "flute", "strings", "bell"], reverb=0.34)


# ----------------------------------------------------------------- Barrowwood


def barrowwood():
    # no pulse and no home: a drone on D, a voice that leans on the notes
    # beside it, a bell that answers out of key
    one = Pattern(8)
    one.chord("voices", 0, "D3 A3", 128, 0.6)
    one.line("voices", 0, "F4:32 E4:16 Eb4:16 | D4:32 G#3:32", 0.45)
    one.note("bass", 32, "D2", 30, 0.5).note("bass", 96, "Eb2", 30, 0.5)
    one.note("bell", 22, "A5", 16, 0.4).note("bell", 44, "Bb5", 16, 0.35).note("bell", 70, "D6", 16, 0.4).note("bell", 92, "C#6", 16, 0.3)
    one.line("flute", 52, "D4:6 Eb4:4 D4:10", 0.4)
    one.hits("drum", 0, [0], 0.5).hits("drum", 4, [0], 0.4)
    two = Pattern(8)
    two.chord("voices", 0, "D3 A3", 64, 0.6).chord("voices", 64, "D3 Ab3", 64, 0.6)
    two.line("voices", 0, "A4:32 Ab4:32 | F4:32 E4:16 D4:16", 0.4)
    two.note("bass", 16, "D2", 40, 0.5).note("bass", 80, "Bb1", 40, 0.5)
    two.note("bell", 10, "F5", 16, 0.35).note("bell", 38, "E5", 16, 0.3).note("bell", 76, "Ab5", 16, 0.4).note("bell", 108, "D5", 20, 0.35)
    two.line("flute", 112, "A4:6 Ab4:4 F4:6", 0.4)
    two.hits("drum", 2, [0], 0.4).hits("drum", 6, [0, 6], 0.4, {6: 0.5})
    return song(54, {"one": one, "two": two}, ["one", "two"], ["voices", "bass", "bell", "flute", "drum"], reverb=0.5)


# ----------------------------------------------------------------- Mirrormere


def mirrormere():
    # still water: slow harp rising and falling, a flute in long notes
    def part(chords, tune, drops):
        p = Pattern(4)
        for bar, (arp, pad) in enumerate(chords):
            p.arp("harp", bar, arp, level=0.5, rows=10)
            p.chord("strings", bar * 16, pad, 16, 0.5)
        p.line("flute", 0, tune, 0.6)
        for row, pitch in drops:
            p.note("bell", row, pitch, 12, 0.35)
        return p

    a = [
        ("F3 C4 E4 A4 C5 A4 E4 C4", "F3 A3 C4"),
        ("F3 B3 D4 G4 B4 G4 D4 B3", "F3 G3 B3"),
        ("F3 C4 E4 A4 C5 A4 E4 C4", "F3 A3 C4"),
        ("A2 E3 A3 C4 E4 C4 A3 E3", "A3 C4 E4"),
    ]
    b = [
        ("D3 A3 C4 F4 A4 F4 C4 A3", "D3 F3 A3"),
        ("G2 D3 G3 B3 D4 B3 G3 D3", "G3 B3 D4"),
        ("C3 G3 C4 E4 G4 E4 C4 G3", "C3 E3 G3"),
        ("C3 G3 B3 E4 G4 E4 B3 G3", "C3 E3 B3"),
    ]
    patterns = {
        "a": part(a, "C5:16 | B4:8 D5:8 | E5:12 C5:4 | A4:16", [(6, "E6"), (38, "B5")]),
        "b": part(b, "F4:8 A4:8 | G4:8 B4:8 | E5:16 | G4:12 -:4", [(22, "A5"), (54, "G5")]),
    }
    return song(63, patterns, ["a", "b", "a", "b"], ["harp", "flute", "strings", "bell"], reverb=0.45)


# ------------------------------------------------------------------ the mines


def mines():
    # picks somewhere below, each with its echo; a low horn; C minor
    def part(pads, horn, drips):
        p = Pattern(4)
        for bar, pad in enumerate(pads):
            r = bar * 16
            p.chord("voices", r, pad, 16, 0.45)
            p.note("bass", r, "C2" if bar != 2 else "Ab1", 15, 0.7)
            p.hits("drum", bar, [0], 0.5)
            for beat in (4, 12):
                p.note("pick", r + beat, 60, 2, 0.7)
                p.note("pick", r + beat + 3, 60, 2, 0.3)
                if beat + 6 < 16:
                    p.note("pick", r + beat + 6, 60, 2, 0.14)
        p.line("horn", 0, horn, 0.7)
        for row, pitch in drips:
            p.note("bell", row, pitch, 10, 0.3)
        return p

    patterns = {
        "a": part(["C3 G3 Eb4", "C3 G3 Eb4", "Ab2 Eb3 C4", "G2 D3 B3"], "C3:8 Eb3:4 D3:4 | C3:16 | Ab2:8 Bb2:8 | G2:16", [(26, "G5"), (58, "Eb6")]),
        "b": part(["F2 C3 Ab3", "C3 G3 Eb4", "Ab2 Eb3 C4", "G2 D3 G3"], "F3:8 Eb3:8 | G3:12 Eb3:4 | C3:8 Eb3:4 D3:4 | G2:16", [(10, "C6"), (42, "G5")]),
    }
    return song(66, patterns, ["a", "b", "a", "b"], ["voices", "bass", "horn", "bell", "drum", "pick"], reverb=0.5)


# --------------------------------------------------------- fights and moments


def combat():
    # drums only: it lies over whatever theme is playing, in any key
    def bar(p, i, fill=False):
        p.hits("drum", i, [0, 6, 8] if i % 2 == 0 else [0, 6, 10], 0.75, {6: 0.6})
        p.hits("tom", i, [4, 12] if not fill else [4, 10, 12, 13, 14, 15], 0.7, {13: 0.7, 15: 0.85})
        p.hits("shaker", i, list(range(0, 16, 2)), 0.6, {2: 0.55, 6: 0.55, 10: 0.55, 14: 0.55})

    a = Pattern(2)
    bar(a, 0)
    bar(a, 1)
    b = Pattern(2)
    bar(b, 0)
    bar(b, 1, fill=True)
    return song(120, {"a": a, "b": b}, ["a", "a", "a", "b"], ["drum", "tom", "shaker"], reverb=0.2)


def combat_sting():
    p = Pattern(2)
    p.hits("drum", 0, [0, 3], 1.0, {3: 0.6}).hits("tom", 0, [2], 0.8)
    p.chord("horn", 0, "D3 A3", 7, 0.85).note("bass", 0, "D2", 10, 0.8)
    return song(120, {"s": p}, ["s"], ["drum", "tom", "horn", "bass"], reverb=0.3, looped=False)


def discovery():
    # a new place: the first notes of the valley's tune, opening upward
    p = Pattern(3)
    p.chord("strings", 0, "D3 A3 F#4", 30, 0.6).note("bass", 0, "D2", 28, 0.6)
    for k, pitch in enumerate("D4 F#4 A4 D5 F#5".split()):
        p.note("harp", k * 1.5, pitch, 14, 0.6 + 0.05 * k)
    p.line("flute", 6, "F#5:6 A5:6 D6:14", 0.6)
    p.note("bell", 18, "D6", 16, 0.5).note("bell", 20, "A5", 16, 0.35)
    return song(76, {"s": p}, ["s"], ["strings", "bass", "harp", "flute", "bell"], reverb=0.45, looped=False)


SONGS = {
    "hearthstead": hearthstead,
    "title": title,
    "birchrise": birchrise,
    "barrowwood": barrowwood,
    "mirrormere": mirrormere,
    "mines": mines,
    "combat": combat,
    "combat_sting": combat_sting,
    "discovery": discovery,
}

# which place hears which theme (areas by their key in the rules)
MUSIC = {
    "tracks": {k: {"song": f"music/{k}.json", "gain": 0.7} for k in SONGS},
    "areas": {
        "hearthstead": "hearthstead",
        "hearthmeadow": "hearthstead",
        "windmill": "hearthstead",
        "east_road": "hearthstead",
        "north_watch": "birchrise",
        "birchrise": "birchrise",
        "logging_camp": "birchrise",
        "hollow_oak": "barrowwood",
        "barrowwood": "barrowwood",
        "mirrormere": "mirrormere",
        "fishers_landing": "mirrormere",
        "sunken_shrine": "mirrormere",
        "miners_rest": "mines",
        "hollow_mines": "mines",
    },
    # the mine head stands just outside the rules' area for the mine
    "spots": [{"at": [730, -90], "radius": 80, "theme": "mines"}],
    "wilds": "hearthstead",
    "title": "title",
    "combat": "combat",
    "stingers": {"combat": "combat_sting", "discovery": "discovery"},
    "crossfade": 4.0,
    "dwell": 1.5,
    "combat_dip": 0.45,
    "combat_linger": 5.0,
}


def write(path, data):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    text = json.dumps(data, separators=(",", ":"))
    with open(path, "w") as f:
        f.write(text + "\n")
    return len(text) + 1


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    out = os.path.join(here, "..", "..", "content", "music")
    args = sys.argv[1:]
    if args[:1] == ["--out"]:
        out = args[1]
    total = 0
    for name, make in SONGS.items():
        s = make()
        size = write(os.path.join(out, name + ".json"), s)
        rows = sum(s["patterns"][p]["rows"] for p in s["order"])
        notes = sum(len(s["patterns"][p]["notes"]) for p in s["order"])
        print(f"music/{name}.json  {rows * 15 / s['bpm']:.1f} s  {notes} notes  {size} bytes")
        total += size
    total += write(os.path.join(out, "music.json"), MUSIC)
    print(f"{total} bytes")


if __name__ == "__main__":
    main()
