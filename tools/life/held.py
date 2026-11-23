"""Tools townsfolk hold at work that no player item needs yet: a fishing
rod and a broom (content/models/held/rod.vox, broom.vox), and their entries
in content/figures.json (`figures` and `held.kinds`).

    python3 tools/life/held.py

A held model is named `held.<kind>/base/item/set`; the hand grips it at the
origin and its shaft runs up y.
"""
import json, os, re, sys

sys.path.insert(0, os.path.dirname(__file__))
import critters

ROOT = os.path.join(os.path.dirname(__file__), '..', '..', 'content')
critters.OUT = os.path.join(ROOT, 'models', 'held')

WOOD, DARK, LINE, RED, STRAW, STRAW2 = 1, 2, 3, 4, 5, 6
PALETTE = [(140, 98, 56), (84, 54, 32), (226, 226, 220), (208, 52, 44), (222, 186, 96), (188, 150, 70)]


def rod():
    m = critters.Model()
    m.box((0, 1), (-10, 20), (0, 1), DARK)       # the butt
    m.box((0, 1), (21, 70), (0, 1), WOOD)        # the rod
    m.box((2, 3), (2, 6), (0, 1), DARK)          # the reel
    for i in range(20):                          # the line, out from the tip
        m.box(2 + i, 70 - i // 2, 0, LINE)
    m.box(21, (-24, 60), 0, LINE)                # and down to the float
    m.box((20, 22), (-28, -25), (-1, 1), RED)
    return m


def broom():
    m = critters.Model()
    m.box((0, 1), (-32, 38), (0, 1), WOOD)
    m.box((-2, 3), (-34, -32), (-1, 2), DARK)    # the binding
    for x in range(-4, 6):
        for z in range(-1, 3):
            m.box(x, (-40, -35), z, STRAW if (x + z) % 2 else STRAW2)
    return m


def figure(kind):
    return {
        'file': 'models/held/%s.vox' % kind,
        'grids': [{'groups': [{'layers': [{'layer': 'held.%s/base' % kind}], 'name': 'held.%s' % kind}],
                   'max': [24, 72, 8], 'min': [-24, -40, -8], 'name': 'main', 'parts': ['', 'item']}],
        'lods': 1,
        'palette': [None] + [{'c': [round(c / 255, 3) for c in rgb]} for rgb in PALETTE],
    }


KINDS = {
    'rod': {'carry_turn': 0, 'figure': 'held.rod', 'fp': [0.36, 0.4, 0.15, 0], 'grip': 'one_hand', 'unit': 0.021},
    'broom': {'carry_turn': 0, 'figure': 'held.broom', 'fp': [0.36, 0.4, 0.15, 0], 'grip': 'one_hand', 'unit': 0.021},
}

if __name__ == '__main__':
    critters.write('rod', PALETTE, {'held.rod/base/item/set': rod()})
    critters.write('broom', PALETTE, {'held.broom/base/item/set': broom()})
    path = os.path.join(ROOT, 'figures.json')
    text = open(path).read()
    # take out what an earlier run wrote
    text = re.sub(r'\n    "held\.(rod|broom)": \{"file".*', '', text)
    text = re.sub(r'\n      "(rod|broom)": \{"carry_turn".*', '', text)
    for head, indent, entries in [
        ('\n  "figures": {', '    ', {'held.' + k: figure(k) for k in KINDS}),
        ('\n    "kinds": {', '      ', KINDS),
    ]:
        assert text.count(head) == 1, head
        lines = ''.join('\n%s"%s": %s,' % (indent, k, json.dumps(v)) for k, v in entries.items())
        text = text.replace(head, head + lines)
    json.loads(text)
    open(path, 'w').write(text)
