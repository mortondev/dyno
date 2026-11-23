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

# key, name, title, colour, where (a place of the layout and the offset from
# it, docs/design/world-layout.md), greeting, what they hold, routine
TOWNSFOLK = [
    ('nell', 'Nell', 'Street Sweeper', [0.5, 0.42, 0.6], ('hearthstead', [3, 0, -34]),
     "Mind the dust. And the boots. Especially the boots.",
     ('broom', [0.6, 0.45, 0.25]),
     [stop((10, 16), anim='Sweep'),
      stop((10, 16), at=(4, -5), anim='Sweep'),
      stop((4, 7), at=(-3, -8), emotes=['Wave']),
      stop((10, 16), at=(-4, -2), anim='Sweep')]),
    ('odo', 'Odo', 'Greengrocer', [0.35, 0.55, 0.3], ('hearthstead', [-4, 0, -38]),
     "Fresh from Ostrin's fields this morning. Well. Yesterday morning.",
     None,
     [stop((16, 30), near='crate', within=8, anim='Talk', emotes=['Wave', 'Point']),
      stop((5, 9), emotes=['Wave'])]),
    ('bram', 'Bram', 'Townsman', [0.6, 0.35, 0.25], ('hearthstead', [6, 0, 1]),
     "Three hundred years that hearth burned. I give it three more, with you about.",
     None,
     [stop((5, 9), at=(5, 3), emotes=['Wave']),
      stop((5, 9), at=(-5, 6)),
      stop((6, 12), at=(-6, -4), emotes=['Point']),
      stop((5, 9), at=(4, -5), emotes=['Wave'])]),
    ('ilsa', 'Ilsa', 'Townswoman', [0.7, 0.45, 0.5], ('hearthstead', [-4, 0, 4]),
     "Sit a while. The stones are still warm if you believe hard enough.",
     None,
     [stop((25, 45), anim='Sit'),
      stop((6, 10), at=(-5, -3), emotes=['Wave', 'Cheer'])]),
    ('tam', 'Tam', 'Lumberjack', [0.6, 0.3, 0.2], ('logging_camp', [7, 0, 9]),
     "Birch splits clean. Oak fights back. I respect oak.",
     ('hatchet', [0.62, 0.64, 0.7]),
     [stop((24, 36), anim='Chop'),
      stop((4, 7), at=(2, -2), emotes=['Cheer'])]),
    ('wyn', 'Wyn', 'Lumberjack', [0.3, 0.45, 0.3], ('logging_camp', [10, 0, 2]),
     "Garrick says axes down. Garrick isn't the one behind on his cords.",
     ('hatchet', [0.62, 0.64, 0.7]),
     [stop((24, 36), anim='Chop'),
      stop((8, 12), at=(-2, 2), anim='Sit')]),
    ('edric', 'Guard Edric', 'North Watch', [0.25, 0.35, 0.6], ('north_watch', [-12, 0, -11]),
     "Quiet night. I don't trust quiet nights.",
     None,
     [stop((4, 8), at=(6, 0)),
      stop((4, 8), at=(6, 6), emotes=['Point']),
      stop((4, 8), at=(-4, 6)),
      stop((4, 8), at=(-4, 0))]),
    ('jory', 'Jory', 'Farmhand', [0.7, 0.6, 0.3], ('windmill_farm', [-30, 0, -25]),
     "Ostrin says the mill grinds, the hands rake. So I rake.",
     ('hoe', [0.55, 0.55, 0.6]),
     [stop((12, 18), near='hay_bale', within=12, anim='Sweep'),
      stop((10, 16), at=(-4, 3), anim='Chop'),
      stop((4, 7), at=(2, -3), emotes=['Wave'])]),
    ('marl', 'Old Marl', 'Fisherman', [0.3, 0.45, 0.6], ('fishers_landing', [-4, 0, -3]),
     "Hush. You'll scare the supper.",
     ('rod', [0.55, 0.38, 0.22]),
     [stop((45, 90), near='water', within=12, anim='Fish'),
      stop((5, 8), at=(1, 1), emotes=['Wave'])]),
    ('dag', 'Dag', 'Miner', [0.45, 0.4, 0.35], ('miners_rest', [0, 0, -12]),
     "Coal's shallow here. Iron's deep. Candlekin are deeper. Guess which I dig.",
     ('pickaxe', [0.6, 0.6, 0.66]),
     [stop((24, 36), anim='Chop'),
      stop((6, 10), at=(2, 2), emotes=['Wave'])]),
    ('sela', 'Sela', 'Wayfarer', [0.5, 0.3, 0.55], ('emberford_bridge', [4, 0, -1]),
     "East road's no place to walk alone. So I walk it quickly.",
     None,
     [stop((4, 8), at=(-9, 0), emotes=['Wave']),
      stop((6, 10), at=(9, 0), emotes=['Point'])]),
    ('hobb', 'Carter Hobb', 'Carter', [0.55, 0.45, 0.3], ('wrecked_caravan', [5, 0, -9]),
     "A good wagon, that was. Forty years. Then Ashcloaks.",
     None,
     [stop((12, 20), near='crate', within=10, anim='Kneel'),
      stop((5, 9), at=(3, -2), emotes=['Point'])]),
    ('tolan', 'Guard Tolan', 'Hearthstead Watch', [0.25, 0.35, 0.6], ('hearthstead', [4, 0, -44]),
     "Tower's sound. It's the roads I'd watch.",
     None,
     [stop((5, 9), emotes=['Point']),
      stop((5, 9), at=(-5, 3)),
      stop((5, 9), at=(-2, -5), emotes=['Wave'])]),
    ('rysa', 'Rysa', 'Hunter', [0.3, 0.5, 0.3], ('hollow_oak', [-8, 0, 36]),
     "Webs between the roots, thick as rope. Don't touch them.",
     None,
     [stop((10, 18), anim='Kneel'),
      stop((6, 10), at=(3, -3), emotes=['Point']),
      stop((6, 10), at=(-2, 3))]),
    ('anwen', 'Anwen', 'Lorekeeper', [0.5, 0.4, 0.65], ('old_barrow', [6, 0, -2]),
     "The stones are older than the hearth. Whatever sleeps under them is older still.",
     None,
     [stop((12, 20), near='mossy_cobble', within=8, anim='Kneel'),
      stop((6, 10), at=(2, 2), emotes=['Point', 'Bow'])]),
    ('calder', 'Brother Calder', 'Lamplighter', [0.6, 0.5, 0.3], ('old_barrow', [11, 0, 7]),
     "A light at the door, every night. It's all that keeps them in.",
     ('torch', [1.0, 0.75, 0.35]),
     [stop((10, 18), emotes=['Point']),
      stop((8, 12), at=(-3, 2), anim='Kneel'),
      stop((6, 10), at=(2, 3))]),
    ('tess', 'Tess', 'Angler', [0.35, 0.5, 0.55], ('fishers_landing', [27, 0, -18]),
     "Mirrormere gives if you're patient. Takes, if you wade.",
     ('rod', [0.55, 0.38, 0.22]),
     [stop((45, 90), near='water', within=12, anim='Fish'),
      stop((5, 8), at=(2, -2), emotes=['Wave'])]),
    ('oswin', 'Oswin', 'Pilgrim', [0.75, 0.72, 0.6], ('sunken_shrine', [3, 0, 21]),
     "They built it to stand in the water. I like to think they knew why.",
     None,
     [stop((14, 24), anim='Kneel', emotes=['Bow']),
      stop((6, 10), at=(-3, 3), emotes=['Bow', 'Point'])]),
    ('mira', 'Mira', 'Night Watch', [0.25, 0.3, 0.45], ('ashcloak_stockade', [9, 0, 9]),
     "Six by the fire, two on the gate. Stay behind my light.",
     ('torch', [1.0, 0.75, 0.35]),
     [stop((24, 36), emotes=['Point']),
      stop((6, 10), at=(2, -2), anim='Kneel')]),
    ('pell', 'Pell', 'Surveyor', [0.5, 0.45, 0.3], ('hollow_mines', [1, 0, 3]),
     "Shaft's sound enough. It's what moved in that isn't.",
     ('pickaxe', [0.6, 0.6, 0.66]),
     [stop((12, 18), near='coal_ore', within=10, anim='Chop'),
      stop((8, 14), emotes=['Point']),
      stop((12, 18), near='iron_ore', within=10, anim='Chop')]),    ('ruk', 'Ruk', 'Miner', [0.4, 0.35, 0.3], ('hollow_mines', [-3, 0, -4]),
     "Sit. Breathe. The dust settles if you let it.",
     ('pickaxe', [0.6, 0.6, 0.66]),
     [stop((20, 36), anim='Sit', emotes=['Wave']),
      stop((8, 12), at=(-2, -2), emotes=['Point'])]),
    ('nim', 'Nim', 'Forager', [0.45, 0.55, 0.3], ('hollow_oak', [-15, 0, 29]),
     "Red caps for stew, pale caps for sleep. Don't mix them up. I did once.",
     None,
     [stop((16, 28), anim='Kneel'),
      stop((6, 10), at=(2, 3), emotes=['Wave']),
      stop((14, 22), at=(4, -1), anim='Kneel')]),
    ('poll', 'Poll', 'Gardener', [0.4, 0.5, 0.3], ('hearthstead', [5, 0, -14]),
     "Beans by the tower wall, where the stone holds the warmth.",
     ('hoe', [0.55, 0.55, 0.6]),
     [stop((12, 20), anim='Chop'),
      stop((10, 16), at=(-3, 2), anim='Kneel'),
      stop((10, 16), at=(-1, 4), anim='Sweep')]),
    ('wat', 'Wat', 'Shepherd', [0.6, 0.55, 0.4], ('north_watch', [-13, 0, -21]),
     "Count them going out, count them coming in. Some nights the sums differ.",
     ('staff', [0.5, 0.36, 0.2]),
     [stop((16, 28), emotes=['Wave', 'Point']),
      stop((14, 22), at=(3, 2), anim='Sit')]),
]


def rules():
    path = os.path.join(ROOT, 'rules', 'hearthblock.json')
    data = json.load(open(path))
    keys = [t[0] for t in TOWNSFOLK]
    data['npcs'] = [n for n in data['npcs'] if n['key'] not in keys]
    for n in data['npcs']:
        if n['key'] in NAMED:
            n['routine'] = NAMED[n['key']]
    for key, name, title, color, (place, at), greeting, _, routine in TOWNSFOLK:
        npc = {'key': key, 'name': name, 'title': title, 'color': color, 'place': place, 'at': at,
               'greeting': greeting, 'routine': routine}
        data['npcs'].append(npc)
    open(path, 'w').write(json.dumps(data, indent=1))


def figures():
    """What each holds: `hold` in the NPC's entry of figures.json. An entry
    that is already there (the character work dresses most townsfolk) is
    left as it is and only checked; a missing one is added as one line."""
    path = os.path.join(ROOT, 'figures.json')
    text = open(path).read()
    have = json.loads(text)['npcs']
    want = {t[0]: list(t[6]) for t in TOWNSFOLK if t[6]}
    want['hollis'] = ['mace', [0.42, 0.42, 0.46]]
    new = ''
    for key, hold in want.items():
        if key not in have:
            new += '\n    "%s": {"hold": %s},' % (key, json.dumps(hold))
        elif have[key].get('hold', [None])[0] != hold[0]:
            print('figures.json: %s holds %s, the table says %s: edit the entry by hand'
                  % (key, have[key].get('hold'), hold[0]))
    if new:
        head = '\n  "npcs": {'
        assert text.count(head) == 1
        text = text.replace(head, head + new)
        json.loads(text)
        open(path, 'w').write(text)


if __name__ == '__main__':
    rules()
    figures()
