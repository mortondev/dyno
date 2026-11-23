"""Class starter kits, the hero looks of the stand shots and the townsfolk's
working clothes, written into the pack as data.

usage: python3 tools/figures/dress.py [PACK_DIR]   (default content/)
then:  dyno bake-figure PACK_DIR                    (lays figures.json out again)

Idempotent: run it again after changing the tables below.
- rules/hearthblock.json: one item per starter garment, and each class's
  starter list.
- figures.json: what each of those items does to the look (`equip`), the
  named townsfolk (`npcs`), a hero per class (`showcase`, a set named by the
  class key: `dyno shot --stand --hero KEY` draws its first person) and the
  size of carried weapons (`held`).
"""
import json, os, sys

PACK = sys.argv[1] if len(sys.argv) > 1 else 'content'

GOLD = [0.95, 0.76, 0.24]
IRON = [0.5, 0.51, 0.54]
STEEL = [0.74, 0.76, 0.82]
BRONZE = [0.66, 0.45, 0.22]
RED = [0.74, 0.12, 0.1]
BLUE = [0.14, 0.27, 0.68]
GREEN = [0.22, 0.52, 0.2]
WHITE = [0.94, 0.92, 0.86]
LEATHER = [0.42, 0.28, 0.16]


def slot(name, main, trim, accent=None, gem=None):
    return {f'{name}.main': main, f'{name}.trim': trim,
            f'{name}.accent': accent or [c * 0.7 for c in main],
            f'{name}.gem': gem or trim}


def tabard(color, trim=GOLD):
    return {'tabard.main': color, 'tabard.trim': trim}


# ------------------------------------------------------------ starter kits
# key: (name, slot, colour, icon of an existing item, armour, the look)
STARTERS = {
    # warrior: mail under a red tabard, iron pauldrons
    'recruits_tabard': ("Recruit's Tabard", 'chest', RED, 'worn_tunic', 3, {
        'features': {'chest': 3, 'tabard': 1, 'emblem': 1},
        'colors': {**slot('chest', IRON, BRONZE, RED), **tabard(RED)}}),
    'recruits_pauldrons': ("Recruit's Pauldrons", 'shoulders', IRON, 'ironbound_pauldrons', 1, {
        'features': {'shoulders': 4},
        'colors': slot('shoulders', IRON, BRONZE, RED, [0.85, 0.25, 0.15])}),
    # paladin: bright mail under a blue tabard with a golden sun
    'initiates_tabard': ("Initiate's Tabard", 'chest', BLUE, 'worn_tunic', 3, {
        'features': {'chest': 3, 'tabard': 1, 'emblem': 2},
        'colors': {**slot('chest', STEEL, GOLD, BLUE), **tabard(BLUE)}}),
    'initiates_pauldrons': ("Initiate's Pauldrons", 'shoulders', STEEL, 'stormguard_pauldrons', 1, {
        'features': {'shoulders': 4},
        'colors': slot('shoulders', STEEL, GOLD, BLUE, [0.4, 0.7, 1.0])}),
    # hunter: a green jerkin and hood, a quiver
    'trackers_jerkin': ("Tracker's Jerkin", 'chest', GREEN, 'worn_tunic', 3, {
        'features': {'chest': 2, 'quiver': 1, 'tabard': 1, 'emblem': 3},
        'colors': {**slot('chest', [0.5, 0.36, 0.2], LEATHER, [0.36, 0.26, 0.16]),
                   **slot('back', [0.56, 0.36, 0.18], GOLD), **tabard(GREEN)}}),
    'trackers_hood': ("Tracker's Hood", 'head', GREEN, 'wildstalker_hood', 1, {
        'features': {'helm': 3},
        'colors': slot('head', GREEN, LEATHER, [0.38, 0.3, 0.2])}),
    # rogue: dark leather, a red cowl and scarf
    'footpads_jerkin': ("Footpad's Jerkin", 'chest', [0.3, 0.22, 0.2], 'worn_tunic', 3, {
        'features': {'chest': 2},
        'colors': {**slot('chest', [0.46, 0.3, 0.2], [0.7, 0.16, 0.12], [0.3, 0.24, 0.24]),
                   'scarf': [0.7, 0.14, 0.12]}}),
    'footpads_cowl': ("Footpad's Cowl", 'head', [0.6, 0.15, 0.12], 'nightstalker_cowl', 1, {
        'features': {'helm': 2},
        'colors': slot('head', [0.7, 0.15, 0.12], LEATHER, [0.3, 0.24, 0.24])}),
    # priest: a white robe and hood trimmed in gold
    'acolytes_robe': ("Acolyte's Robe", 'chest', WHITE, 'worn_tunic', 3, {
        'features': {'chest': 1, 'robe': 1, 'emblem': 1},
        'colors': slot('chest', WHITE, GOLD, [0.86, 0.74, 0.4])}),
    'acolytes_hood': ("Acolyte's Hood", 'head', WHITE, 'wildstalker_hood', 1, {
        'features': {'helm': 3},
        'colors': slot('head', WHITE, GOLD, [0.86, 0.74, 0.4])}),
    # mage: a blue robe and mantle trimmed in gold
    'novices_robe': ("Novice's Robe", 'chest', BLUE, 'apprentice_robe', 3, {
        'features': {'chest': 1, 'robe': 1, 'emblem': 2},
        'colors': slot('chest', BLUE, GOLD, [0.82, 0.78, 0.64], [0.4, 0.8, 1.0])}),
    'novices_mantle': ("Novice's Mantle", 'shoulders', BLUE, 'apprentice_mantle', 1, {
        'features': {'shoulders': 1},
        'colors': slot('shoulders', [0.12, 0.22, 0.56], GOLD, [0.82, 0.78, 0.64], [0.4, 0.8, 1.0])}),
}
# class: what replaces the worn tunic, and what is added
KITS = {
    'warrior': ['recruits_tabard', 'recruits_pauldrons'],
    'paladin': ['initiates_tabard', 'initiates_pauldrons'],
    'hunter': ['trackers_jerkin', 'trackers_hood'],
    'rogue': ['footpads_jerkin', 'footpads_cowl'],
    'priest': ['acolytes_robe', 'acolytes_hood'],
    'mage': ['novices_robe', 'novices_mantle'],
}

# ------------------------------------------------------------------ heroes
# The stand-in of the gauntlet shots, per class: a person and what they
# wear over the class's starter kit.
SKIN = [[0.96, 0.8, 0.66], [0.87, 0.68, 0.52], [0.72, 0.52, 0.38], [0.98, 0.86, 0.75]]


def person(name, skin, hair, style, body=0, build=1, beard=0, **more):
    return {'name': name, 'color': [0.5, 0.5, 0.5],
            'features': {'body': body, 'build': build, 'hair_style': style, 'beard': beard,
                         'ears': 1, 'mood': 1},
            'colors': {'skin': skin, 'hair': hair, 'boots': [0.3, 0.2, 0.12],
                       'pants': [0.3, 0.24, 0.2], **more}}


HEROES = {
    'warrior': (person('Warrior', SKIN[1], [0.45, 0.28, 0.12], 1, build=2), {
        'wear': ['ironbound_cuirass', 'ironbound_gauntlets', 'ironbound_girdle',
                 'ironbound_legplates', 'ironbound_sabatons', 'recruits_tabard',
                 'ironbound_pauldrons'],
        'features': {'chest': 4},
        'colors': {**slot('shoulders', IRON, GOLD, RED, [0.9, 0.2, 0.15]), 'chest.trim': GOLD},
        'weapon': ['Sword', [0.8, 0.82, 0.88]],
        'carry': {'shield': ['shield', RED]}}),
    'paladin': (person('Paladin', SKIN[0], [0.9, 0.72, 0.3], 1, build=2), {
        'wear': ['stormguard_cuirass', 'stormguard_gauntlets', 'stormguard_girdle',
                 'stormguard_legplates', 'stormguard_sabatons', 'initiates_tabard',
                 'stormguard_pauldrons'],
        'features': {'chest': 4},
        'colors': {**slot('shoulders', STEEL, GOLD, BLUE, [0.4, 0.7, 1.0]), 'chest.trim': GOLD,
                   'legs.trim': GOLD, 'feet.trim': GOLD, 'hands.trim': GOLD},
        'weapon': ['Mace', GOLD]}),
    'hunter': (person('Hunter', SKIN[2], [0.3, 0.19, 0.1], 2, body=1), {
        'wear': ['wildstalker_mantle', 'wildstalker_gauntlets', 'wildstalker_belt',
                 'wildstalker_leggings', 'wildstalker_boots', 'trackers_jerkin',
                 'trackers_hood'],
        'colors': {**slot('shoulders', GREEN, GOLD, [0.3, 0.22, 0.14]),
                   **slot('legs', [0.42, 0.3, 0.18], LEATHER),
                   **slot('feet', [0.3, 0.2, 0.12], LEATHER),
                   **slot('hands', [0.36, 0.26, 0.16], LEATHER)},
        'weapon': ['Bow', [0.62, 0.42, 0.2]]}),
    'rogue': (person('Rogue', SKIN[1], [0.16, 0.12, 0.1], 1), {
        'wear': ['nightstalker_spaulders', 'nightstalker_gloves', 'nightstalker_belt',
                 'nightstalker_breeches', 'nightstalker_boots', 'footpads_jerkin',
                 'footpads_cowl'],
        'colors': {**slot('shoulders', [0.5, 0.33, 0.2], [0.7, 0.16, 0.12]),
                   **slot('legs', [0.36, 0.33, 0.34], LEATHER),
                   **slot('feet', [0.3, 0.2, 0.14], LEATHER),
                   **slot('hands', [0.3, 0.2, 0.14], LEATHER),
                   **slot('waist', [0.4, 0.26, 0.16], GOLD)},
        'weapon': ['Dagger', [0.82, 0.84, 0.9]],
        'carry': {'offhand': ['dagger', [0.82, 0.84, 0.9]]}}),
    'priest': (person('Priest', SKIN[3], [0.85, 0.68, 0.35], 2, body=1), {
        'wear': ['acolytes_robe', 'acolytes_hood'],
        'features': {'shoulders': 1, 'cape': 1, 'weapon_glow': 1, 'light': 14},
        'colors': {**slot('shoulders', WHITE, GOLD, [0.86, 0.74, 0.4], [1.0, 0.85, 0.4]),
                   'cape': WHITE, 'trim': GOLD},
        'weapon': ['Staff', [0.95, 0.8, 0.35]]}),
    'mage': (person('Mage', SKIN[0], [0.45, 0.28, 0.12], 4, body=1), {
        'wear': ['novices_robe', 'novices_mantle'],
        'features': {'cape': 1, 'weapon_glow': 1, 'light': 12},
        'colors': {'cape': BLUE, 'trim': GOLD},
        'weapon': ['Staff', [0.4, 0.8, 1.0]]}),
}

# --------------------------------------------------------------- townsfolk
PLAIN = {'armor': 0, 'pauldrons': 0, 'gloves': 0, 'emblem': 0, 'helmet': 0, 'robe': 0}


def guard(skin, hair, beard, style=1, build=1, body=0):
    """The Watch: mail under a blue tabard with the golden sun, an open helm, a spear."""
    return {
        'features': {**PLAIN, 'chest': 3, 'tabard': 1, 'emblem': 2, 'helm': 4, 'shoulders': 3,
                     'hands': 2, 'beard': beard, 'hair_style': style, 'build': build,
                     'body': body, 'mood': 1},
        'colors': {'skin': skin, 'hair': hair, 'cape': None,
                   **slot('chest', [0.62, 0.64, 0.7], GOLD, BLUE), **tabard(BLUE),
                   **slot('head', [0.66, 0.68, 0.74], GOLD, BLUE, [0.3, 0.5, 1.0]),
                   **slot('shoulders', [0.62, 0.64, 0.7], GOLD),
                   **slot('hands', LEATHER, [0.3, 0.2, 0.12]),
                   'boots': [0.26, 0.18, 0.12], 'pants': [0.24, 0.24, 0.3]},
        'weapon': ['Polearm', [0.72, 0.72, 0.78]],
    }


def hat(kind, main, band):
    return slot('head', main, band)


STRAW = [0.86, 0.72, 0.38]
NPCS = {
    # the hearthkeeper: an elder in a russet robe with a walking staff
    'brannoc': {
        'features': {**PLAIN, 'robe': 1, 'beard': 1, 'build': 0, 'elder': 1, 'hair_style': 2,
                     'mood': 2},
        'colors': {'eyes': [0.35, 0.5, 0.7], 'hair': [0.9, 0.9, 0.88], 'skin': [0.92, 0.76, 0.62],
                   'trim': [0.9, 0.7, 0.3], 'tunic': [0.55, 0.26, 0.16], 'sash': [0.9, 0.7, 0.3]},
        'weapon': ['Staff', [0.5, 0.36, 0.22]]},
    # the captain: plate, a blue tabard and cape, a sword
    'maren': {
        'features': {**PLAIN, 'chest': 4, 'legs': 4, 'shoulders': 4, 'hands': 4, 'feet': 4,
                     'tabard': 1, 'emblem': 2, 'cape': 1, 'beard': 0, 'hair_style': 2, 'mood': 1,
                     'body': 1},
        'colors': {'cape': BLUE, 'eyes': [0.28, 0.55, 0.36], 'hair': [0.68, 0.22, 0.1],
                   'skin': [0.96, 0.8, 0.66], 'trim': GOLD,
                   **slot('chest', STEEL, GOLD, BLUE), **slot('legs', STEEL, GOLD, [0.62, 0.64, 0.7]),
                   **slot('shoulders', STEEL, GOLD, BLUE, [0.3, 0.5, 1.0]),
                   **slot('hands', STEEL, GOLD), **slot('feet', STEEL, GOLD), **tabard(BLUE)},
        'weapon': ['Sword', [0.8, 0.82, 0.88]],
        'carry': {'shield': ['shield', BLUE]}},
    # the cook: a white apron over a warm dress
    'tilda': {
        'features': {**PLAIN, 'robe': 1, 'beard': 0, 'build': 0, 'hair_style': 4, 'mood': 0,
                     'body': 1},
        'colors': {'apron': [0.95, 0.93, 0.86], 'eyes': [0.42, 0.28, 0.16],
                   'hair': [0.45, 0.28, 0.12], 'skin': [0.97, 0.82, 0.7],
                   'trim': [0.85, 0.3, 0.25], 'tunic': [0.72, 0.4, 0.2]}},
    # the smith: a leather apron, heavy gloves, a hammer
    'hollis': {
        'features': {**PLAIN, 'hands': 2, 'beard': 1, 'build': 2, 'hair_style': 0, 'mood': 1},
        'colors': {'apron': [0.36, 0.24, 0.14], 'eyes': [0.3, 0.22, 0.14],
                   'hair': [0.2, 0.14, 0.1], 'skin': [0.72, 0.52, 0.38],
                   'tunic': [0.5, 0.2, 0.14], 'trim': [0.3, 0.3, 0.32],
                   **slot('hands', [0.3, 0.2, 0.12], [0.2, 0.14, 0.1])},
        'hold': ['mace', [0.42, 0.42, 0.46]]},
    'holt': guard([0.87, 0.68, 0.52], [0.3, 0.19, 0.1], 1, build=2),
    'ansel': guard([0.96, 0.8, 0.66], [0.85, 0.68, 0.35], 0),
    # the logging foreman: a checked-red jerkin, a felling axe
    'garrick': {
        'features': {**PLAIN, 'chest': 2, 'hands': 2, 'beard': 1, 'build': 2, 'hair_style': 1,
                     'mood': 1},
        'colors': {'skin': [0.87, 0.68, 0.52], 'hair': [0.62, 0.22, 0.1],
                   **slot('chest', [0.66, 0.2, 0.14], LEATHER, [0.28, 0.2, 0.14]),
                   **slot('hands', LEATHER, [0.3, 0.2, 0.12]),
                   'pants': [0.3, 0.26, 0.2], 'boots': [0.26, 0.18, 0.12]},
        'hold': ['hatchet', [0.62, 0.64, 0.7]]},
    # the farmer: a straw hat, a green tunic, an apron, a hoe
    'ostrin': {
        'features': {**PLAIN, 'helm': 5, 'beard': 1, 'build': 1, 'hair_style': 1, 'mood': 0},
        'colors': {'skin': [0.87, 0.68, 0.52], 'hair': [0.45, 0.28, 0.12],
                   'tunic': [0.36, 0.52, 0.24], 'trim': [0.86, 0.72, 0.38],
                   'apron': [0.5, 0.36, 0.22], 'pants': [0.4, 0.3, 0.2],
                   **hat(5, STRAW, [0.7, 0.2, 0.15])},
        'hold': ['hoe', [0.55, 0.55, 0.6]]},
    # the old fisher: a wide straw hat, sea-blue clothes, a scarf
    'wenna': {
        'features': {**PLAIN, 'helm': 5, 'beard': 0, 'build': 0, 'hair_style': 2, 'elder': 1,
                     'mood': 0, 'body': 1},
        'colors': {'skin': [0.9, 0.72, 0.58], 'hair': [0.85, 0.85, 0.82],
                   'tunic': [0.2, 0.46, 0.62], 'trim': [0.9, 0.88, 0.8],
                   'apron': [0.82, 0.76, 0.6], 'pants': [0.3, 0.3, 0.36],
                   **hat(5, STRAW, [0.2, 0.46, 0.62])},
        'hold': ['rod', [0.55, 0.38, 0.22]]},
    # the quartermaster: a ledger, a sash of office
    'bree': {
        'features': {**PLAIN, 'chest': 2, 'beard': 0, 'build': 1, 'hair_style': 4, 'mood': 1,
                     'body': 1},
        'colors': {'skin': [0.72, 0.52, 0.38], 'hair': [0.18, 0.12, 0.08],
                   **slot('chest', [0.5, 0.32, 0.2], GOLD, [0.82, 0.78, 0.64]),
                   'sash': [0.7, 0.16, 0.12], 'pants': [0.28, 0.24, 0.22]},
        'carry': {'offhand': ['tome', [0.5, 0.16, 0.12]]}},
    # the prospector: a hard hat with a lamp, a pickaxe
    'dunmore': {
        'features': {**PLAIN, 'helm': 6, 'chest': 2, 'hands': 2, 'beard': 1, 'build': 2,
                     'hair_style': 1, 'mood': 0, 'light': 9},
        'colors': {'skin': [0.87, 0.68, 0.52], 'hair': [0.45, 0.48, 0.55],
                   **slot('chest', [0.44, 0.36, 0.26], LEATHER, [0.3, 0.26, 0.22]),
                   **slot('hands', LEATHER, [0.3, 0.2, 0.12]),
                   **hat(6, [0.86, 0.66, 0.16], [0.3, 0.3, 0.32]),
                   'pants': [0.3, 0.27, 0.24]},
        'hold': ['pickaxe', [0.6, 0.6, 0.66]]},
}



def worker(base, **more):
    """A townsperson in the clothes of `base`'s trade, with their own face
    (drawn from their name) and whatever the pack already has them hold."""
    keep = ('skin', 'hair', 'eyes')
    own = ('beard', 'build', 'hair_style', 'body', 'mood', 'elder')
    look = {'features': {k: v for k, v in NPCS[base]['features'].items() if k not in own},
            'colors': {k: v for k, v in NPCS[base]['colors'].items() if k not in keep}}
    for k, v in more.items():
        look[k].update(v)
    return look


# the townsfolk at work (the life lane's): dressed as the named one of
# their trade
NPCS.update({
    'nell': worker('tilda', colors={'tunic': [0.4, 0.5, 0.66], 'trim': [0.9, 0.88, 0.8]}),
    'tam': worker('garrick'),
    'wyn': worker('garrick', colors={'chest.main': [0.3, 0.46, 0.24]}),
    'jory': worker('ostrin', colors={'tunic': [0.62, 0.5, 0.26]}),
    'marl': worker('wenna', colors={'tunic': [0.26, 0.5, 0.42]}),
    'tess': worker('wenna'),
    'dag': worker('dunmore'),
    'pell': worker('dunmore', colors={'chest.main': [0.4, 0.3, 0.26]}),
    'calder': worker('holt'),
    'mira': worker('holt', features={'body': 1}),
})
for k in ('calder', 'mira'):
    NPCS[k]['features']['light'] = 12

# carried weapons are this much bigger than life (the first-person size
# is kept)
BIG = {'sword': 1.4, 'staff': 1.2, 'mace': 1.45, 'battle_axe': 1.35, 'bow': 1.3,
       'polearm': 1.2, 'dagger': 1.35, 'shield': 1.2}
# their size at life size, as the pack had them
LIFE = {'sword': 0.01854838617146015, 'staff': 0.020652173087000847, 'mace': 0.01899999938905239,
        'battle_axe': 0.0223214291036129, 'bow': 0.02045454643666744,
        'polearm': 0.02187499962747097, 'dagger': 0.01878787949681282,
        'shield': 0.032692309468984604}
LIFE_FP = {'sword': 0.5, 'staff': 0.36, 'mace': 0.6, 'battle_axe': 0.5, 'bow': 0.42,
           'polearm': 0.36, 'dagger': 0.6, 'shield': 0.5}


def rules():
    path = os.path.join(PACK, 'rules/hearthblock.json')
    r = json.load(open(path))
    items = {it['key']: it for it in r['items']}
    for key, (name, where, color, icon, armor, _) in STARTERS.items():
        it = {'key': key, 'name': name, 'color': color, 'icon': 'lump',
              'image': f'icons/{icon}.png', 'stack': 1, 'quality': 1, 'level': 1,
              'value': 27 if where == 'chest' else 18,
              'gear': {'slot': where, 'armor_type': 'cloth', 'armor': armor, 'stats': {}}}
        if key in items:
            items[key].clear()
            items[key].update(it)
        else:
            r['items'].append(it)
    for c in r['classes']:
        kit = KITS.get(c['key'])
        if kit is None:
            continue
        own = {k for ks in KITS.values() for k in ks}
        rest = [s for s in c['starter'] if s[0] != 'worn_tunic' and s[0] not in own]
        at = next((i for i, s in enumerate(rest) if s[0] == 'worn_trousers'), len(rest))
        c['starter'] = rest[:at] + [[k, 1] for k in kit] + rest[at:]
    open(path, 'w').write(json.dumps(r, indent=1))
    return r


def figures(r):
    path = os.path.join(PACK, 'figures.json')
    f = json.load(open(path))
    for key, spec in STARTERS.items():
        f['equip']['items'][key] = spec[5]
    # townsfolk others have added stay; what the pack has someone hold stays
    for key, look in NPCS.items():
        hold = f['npcs'].get(key, {}).get('hold')
        f['npcs'][key] = {**look, 'hold': hold} if hold else look
    show = f['showcase']
    # the people and sets of this script are rebuilt; the rest are kept
    # (sets name people by index, so people are only ever appended)
    old = show['people']
    keep = [i for i, p in enumerate(old) if not p.get('hero') and 'npc' not in p]
    people = [old[i] for i in keep]
    sets = []
    for s in show['sets']:
        if s.get('hero') or s['name'] == 'Named townsfolk':
            continue
        if 'on' in s:
            s['on'] = [keep.index(i) for i in s['on'] if i in keep]
        sets.append(s)
    named = [k for k in f['npcs'] if k != '*']
    base = len(people)
    for key in named:
        people.append({'name': key.capitalize(), 'npc': key, 'color': [0.4, 0.5, 0.6]})
    sets.append({'name': 'Named townsfolk', 'on': list(range(base, base + len(named)))})
    # a new character of each class, in its starter kit
    items = {it['key']: it for it in r['items']}
    for i, c in enumerate(r['classes']):
        wear, weapon = [], None
        for key, _ in c['starter']:
            w = items.get(key, {}).get('gear', {}).get('weapon')
            if w and weapon is None and w['kind'] not in ('shield', 'offhand'):
                weapon = [w['kind'].capitalize(), items[key]['color']]
            wear.append(key)
        s = {'name': f"Starter: {c['name']}", 'hero': True, 'on': [i % 4], 'wear': wear}
        if weapon:
            s['weapon'] = weapon
        sets.append(s)
    for key, (who, look) in HEROES.items():
        people.append({**who, 'hero': True})
        sets.append({'name': key, 'hero': True, 'on': [len(people) - 1], **look})
    show['people'], show['sets'] = people, sets
    for kind, k in BIG.items():
        h = f['held']['kinds'][kind]
        h['unit'] = LIFE[kind] * k
        h['fp'][0] = round(LIFE_FP[kind] / k, 4)
    open(path, 'w').write(json.dumps(f, indent=1))


figures(rules())
print('dressed', PACK)
