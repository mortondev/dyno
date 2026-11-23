"""Townsfolk and their routines: writes the ambient NPCs (a sweeper, two
lumberjacks, a fisher, a miner...) and the routines of the named ones into
content/rules/hearthblock.json, and what each holds into
content/figures.json. Run again after changing the tables below:

    python3 tools/life/townsfolk.py

A routine is a list of stops (crates/dyno-world/src/content.rs,
`RoutineStep`): `at` is in blocks from where the NPC stands, `near` means
"beside the nearest such block", `anim` is held while there, `emotes` are
thrown in now and then, `secs` is how long they stay.
"""
import json, os, re

ROOT = os.path.join(os.path.dirname(__file__), '..', '..', 'content')


def stop(secs, at=(0, 0), **kw):
    return {'at': list(at), 'secs': list(secs), **kw}


# routines for the NPCs the rules already have: they stay where players
# expect to find them
NAMED = {
    'brannoc': [stop((20, 40), emotes=['Point', 'Bow'])],
    'maren': [stop((20, 40), emotes=['Point', 'Wave'])],
    'tilda': [stop((20, 40), emotes=['Wave', 'Cheer'])],
    'hollis': [
        stop((14, 24), near='anvil', within=6, anim='Hammer'),
        stop((6, 10), emotes=['Wave']),
    ],
    'holt': [stop((20, 40), emotes=['Point'])],
    'garrick': [stop((20, 40), emotes=['Point', 'Wave'])],
    'ostrin': [stop((20, 40), emotes=['Wave'])],
    'wenna': [stop((20, 40), emotes=['Wave', 'Point'])],
    'bree': [stop((20, 40), emotes=['Point'])],
    'dunmore': [stop((20, 40), emotes=['Cheer', 'Point'])],
}

# key, name, title, colour, at (from the hearth), greeting, what they hold,
# routine
TOWNSFOLK = [
    ('nell', 'Nell', 'Street Sweeper', [0.5, 0.42, 0.6], [3, 0, -7],
     "Mind the dust. And the boots. Especially the boots.",
     ('broom', [0.6, 0.45, 0.25]),
     [stop((10, 16), anim='Sweep'),
      stop((10, 16), at=(4, -5), anim='Sweep'),
      stop((4, 7), at=(-3, -8), emotes=['Wave']),
      stop((10, 16), at=(-4, -2), anim='Sweep')]),
    ('odo', 'Odo', 'Greengrocer', [0.35, 0.55, 0.3], [-6, 0, -12],
     "Fresh from Ostrin's fields this morning. Well. Yesterday morning.",
     None,
     [stop((16, 30), near='crate', within=8, anim='Talk', emotes=['Wave', 'Point']),
      stop((5, 9), emotes=['Wave'])]),
    ('bram', 'Bram', 'Townsman', [0.6, 0.35, 0.25], [2, 0, 5],
     "Three hundred years that hearth burned. I give it three more, with you about.",
     None,
     [stop((5, 9), at=(5, 3), emotes=['Wave']),
      stop((5, 9), at=(-5, 6)),
      stop((6, 12), at=(-6, -4), emotes=['Point']),
      stop((5, 9), at=(4, -5), emotes=['Wave'])]),
    ('ilsa', 'Ilsa', 'Townswoman', [0.7, 0.45, 0.5], [4, 0, 11],
     "Sit a while. The stones are still warm if you believe hard enough.",
     None,
     [stop((25, 45), anim='Sit'),
      stop((6, 10), at=(-5, -3), emotes=['Wave', 'Cheer'])]),
    ('tam', 'Tam', 'Lumberjack', [0.6, 0.3, 0.2], [-47, -3, -88],
     "Birch splits clean. Oak fights back. I respect oak.",
     ('hatchet', [0.62, 0.64, 0.7]),
     [stop((14, 22), near='log', within=10, anim='Chop'),
      stop((4, 7), at=(3, 2), emotes=['Cheer']),
      stop((12, 20), near='birch_log', within=12, anim='Chop')]),
    ('wyn', 'Wyn', 'Lumberjack', [0.3, 0.45, 0.3], [-40, -3, -90],
     "Garrick says axes down. Garrick isn't the one behind on his cords.",
     ('hatchet', [0.62, 0.64, 0.7]),
     [stop((14, 22), near='birch_log', within=12, anim='Chop'),
      stop((10, 16), at=(2, 3), anim='Sit'),
      stop((12, 20), at=(-3, 4), near='log', within=8, anim='Chop')]),
    ('edric', 'Guard Edric', 'North Watch', [0.25, 0.35, 0.6], [-2, -7, -46],
     "Quiet night. I don't trust quiet nights.",
     None,
     [stop((4, 8), at=(6, 0)),
      stop((4, 8), at=(6, 6), emotes=['Point']),
      stop((4, 8), at=(-4, 6)),
      stop((4, 8), at=(-4, 0))]),
    ('jory', 'Jory', 'Farmhand', [0.7, 0.6, 0.3], [14, -10, 101],
     "Ostrin says the mill grinds, the hands rake. So I rake.",
     ('hoe', [0.55, 0.55, 0.6]),
     [stop((12, 18), near='hay_bale', within=12, anim='Sweep'),
      stop((10, 16), at=(-4, 3), anim='Chop'),
      stop((4, 7), at=(2, -3), emotes=['Wave'])]),
    ('marl', 'Old Marl', 'Fisherman', [0.3, 0.45, 0.6], [-31, -11, 134],
     "Hush. You'll scare the supper.",
     ('rod', [0.55, 0.38, 0.22]),
     [stop((45, 90), near='water', within=12, anim='Fish'),
      stop((5, 8), at=(1, 1), emotes=['Wave'])]),
    ('dag', 'Dag', 'Miner', [0.45, 0.4, 0.35], [124, -4, 7],
     "Coal's shallow here. Iron's deep. Candlekin are deeper. Guess which I dig.",
     ('pickaxe', [0.6, 0.6, 0.66]),
     [stop((14, 22), near='coal_ore', within=14, anim='Chop'),
      stop((8, 14), at=(-2, 3), anim='Sit'),
      stop((12, 18), near='iron_ore', within=14, anim='Chop')]),
    ('sela', 'Sela', 'Wayfarer', [0.5, 0.3, 0.55], [95, -6, 0],
     "East road's no place to walk alone. So I walk it quickly.",
     None,
     [stop((4, 8), at=(-9, 0), emotes=['Wave']),
      stop((6, 10), at=(9, 0), emotes=['Point'])]),
    ('hobb', 'Carter Hobb', 'Carter', [0.55, 0.45, 0.3], [50, -6, -9],
     "A good wagon, that was. Forty years. Then Ashcloaks.",
     None,
     [stop((12, 20), near='crate', within=10, anim='Kneel'),
      stop((5, 9), at=(3, -2), emotes=['Point'])]),
]


def rules():
    path = os.path.join(ROOT, 'rules', 'hearthblock.json')
    data = json.load(open(path))
    keys = [t[0] for t in TOWNSFOLK]
    data['npcs'] = [n for n in data['npcs'] if n['key'] not in keys]
    for n in data['npcs']:
        if n['key'] in NAMED:
            n['routine'] = NAMED[n['key']]
    for key, name, title, color, at, greeting, _, routine in TOWNSFOLK:
        data['npcs'].append({'key': key, 'name': name, 'title': title, 'color': color, 'at': at,
                             'greeting': greeting, 'routine': routine})
    open(path, 'w').write(json.dumps(data, indent=1))


def figures():
    path = os.path.join(ROOT, 'figures.json')
    text = open(path).read()
    # take out what an earlier run wrote
    text = re.sub(r'\n    "\w+": \{"hold": .*', '', text)
    text = re.sub(r'\n      "hold": \["mace".*', '', text)
    head = '\n  "npcs": {'
    assert text.count(head) == 1
    lines = ''.join('\n    "%s": {"hold": %s},' % (t[0], json.dumps(list(t[6])))
                    for t in TOWNSFOLK if t[6])
    text = text.replace(head, head + lines)
    # the smith's hammer
    smith = '\n    "hollis": {'
    assert text.count(smith) == 1
    text = text.replace(smith, smith + '\n      "hold": ["mace", [0.42, 0.42, 0.46]],')
    json.loads(text)
    open(path, 'w').write(text)


if __name__ == '__main__':
    rules()
    figures()
