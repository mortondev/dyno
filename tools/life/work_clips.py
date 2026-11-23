"""The work animations townsfolk play at their spots: Chop, Hammer, Fish,
Sweep (docs/design/animation.md; the routines are in the rules, per NPC).

They are ordinary clips in content/anims/<skeleton>.json, written there by
this script as one line each at the top of the `default` set:

    python3 tools/life/work_clips.py

People get keyframed loops (below: times in seconds, angles in degrees,
eased between keys). Creatures cannot do these literally and get a still
of their `Kneel`, as the contract asks for every animation on every
skeleton.
"""
import json, math, os, re

ANIMS = os.path.join(os.path.dirname(__file__), '..', '..', 'content', 'anims')
NAMES = ['Chop', 'Hammer', 'Fish', 'Sweep']


def eased(keys, loop):
    """keys: [(t, v)...] from 0 to `loop`; sampled 30 times a second with a
    smooth step between keys"""
    out = []
    n = int(round(loop * 30))
    for i in range(n + 1):
        t = loop * i / n
        for (t0, v0), (t1, v1) in zip(keys, keys[1:]):
            if t0 <= t <= t1:
                u = (t - t0) / (t1 - t0) if t1 > t0 else 0.0
                u = u * u * (3 - 2 * u)
                out.append([round(t, 4), round(v0 + (v1 - v0) * u, 3)])
                break
    # drop samples a straight line already gives
    thin = [out[0]]
    for a, b in zip(out[1:], out[2:]):
        p = thin[-1]
        u = (a[0] - p[0]) / (b[0] - p[0])
        if abs(p[1] + (b[1] - p[1]) * u - a[1]) > 0.05:
            thin.append(a)
    thin.append(out[-1])
    return thin


def clip(loop, still, moving):
    time = dict(still)
    for name, keys in moving.items():
        assert keys[0][0] == 0 and keys[-1][0] == loop and keys[0][1] == keys[-1][1], name
        time[name] = eased(keys, loop)
    return {'loop': loop, 'time': dict(sorted(time.items()))}


# feet apart, knees a little bent: someone at work
STANCE = {
    'flow': 0.05, 'pelvis.ry': -10.3, 'spine.ry': 6.9,
    'thigh.l.rx': -28.0, 'thigh.l.rz': -5.7, 'shin.l.rx': 33.0,
    'thigh.r.rx': -5.0, 'thigh.r.rz': 5.7, 'shin.r.rx': 27.5,
}


def chop(default):
    """The two-handed swing, slowed a little, with a breath between blows."""
    src = default['Attack2H']['time']
    pace, loop = 1.2, 1.9
    time = {}
    for name, ch in src.items():
        if isinstance(ch, list):
            keys = [[round(t * pace, 4), v] for t, v in ch]
            time[name] = keys + [[loop, keys[0][1]]]
        else:
            time[name] = ch
    return {'loop': loop, 'time': time}


def hammer():
    return clip(1.0, {
        **STANCE,
        'upper_arm.l.rx': -38.0, 'upper_arm.l.rz': -10.0, 'forearm.l.rx': -80.0,
        'head.rx': 14.0,
    }, {
        'upper_arm.r.rx': [(0, -55), (0.36, -102), (0.5, -38), (0.64, -48), (1.0, -55)],
        'forearm.r.rx': [(0, -75), (0.36, -106), (0.5, -30), (0.64, -52), (1.0, -75)],
        'grip.rx': [(0, -10), (0.36, 16), (0.5, -30), (1.0, -10)],
        'spine.rx': [(0, 14), (0.36, 9), (0.5, 20), (1.0, 14)],
        'pelvis.oy': [(0, 0), (0.36, 0.2), (0.5, -0.5), (1.0, 0)],
    })


def fish():
    return clip(8.0, {
        'flow': 0.05, 'spine.ry': -12.0,
        'thigh.l.rx': -10.0, 'shin.l.rx': 9.0, 'thigh.r.rx': 6.0,
        'upper_arm.r.rx': -42.0, 'upper_arm.r.rz': 4.0, 'forearm.r.rx': -62.0,
        'upper_arm.l.rx': -32.0, 'upper_arm.l.rz': -14.0, 'forearm.l.rx': -88.0,
    }, {
        # a long wait, then a tug on the line
        'grip.rx': [(0, -35), (3.5, -39), (5.0, -35), (5.3, -62), (5.7, -30), (6.4, -37), (8.0, -35)],
        'spine.rx': [(0, 2), (5.0, 2), (5.3, -7), (6.0, 3), (8.0, 2)],
        'head.rx': [(0, 6), (4.0, 9), (8.0, 6)],
        'head.ry': [(0, 0), (2.0, 10), (4.0, -4), (8.0, 0)],
        'pelvis.ox': [(0, 0.5), (4.0, -0.5), (8.0, 0.5)],
    })


def sweep():
    return clip(2.0, {
        'flow': 0.05, 'spine.rx': 18.0, 'head.rx': 12.0,
        'thigh.l.rx': -20.0, 'thigh.l.rz': -5.0, 'shin.l.rx': 24.0,
        'thigh.r.rx': -6.0, 'thigh.r.rz': 5.0, 'shin.r.rx': 20.0,
        'upper_arm.l.rz': -18.0,
    }, {
        'spine.ry': [(0, -20), (1.0, 20), (2.0, -20)],
        'pelvis.ry': [(0, -8), (1.0, 8), (2.0, -8)],
        'pelvis.ox': [(0, -1), (1.0, 1), (2.0, -1)],
        'upper_arm.r.rx': [(0, -20), (1.0, -58), (2.0, -20)],
        'forearm.r.rx': [(0, -70), (1.0, -40), (2.0, -70)],
        'upper_arm.l.rx': [(0, -58), (1.0, -24), (2.0, -58)],
        'forearm.l.rx': [(0, -95), (1.0, -75), (2.0, -95)],
        'grip.rx': [(0, -15), (1.0, 25), (2.0, -15)],
    })


def still_of(c):
    """a clip frozen at its first moment"""
    time = {k: (v[0][1] if isinstance(v, list) else v) for k, v in c['time'].items()}
    return {'loop': 4.0, 'time': time}


def compact(v):
    return json.dumps(v, separators=(',', ':'))


for name in sorted(os.listdir(ANIMS)):
    if not name.endswith('.json'):
        continue
    path = os.path.join(ANIMS, name)
    text = open(path).read()
    # take out what an earlier run wrote
    text = re.sub(r'\n      "(%s)": \{"loop".*' % '|'.join(NAMES), '', text)
    data = json.loads(text)
    default = data['sets']['default']
    if data.get('skeleton') == 'biped':
        clips = {'Chop': chop(default), 'Hammer': hammer(), 'Fish': fish(), 'Sweep': sweep()}
    else:
        clips = {n: still_of(default['Kneel']) for n in NAMES}
    lines = ''.join('\n      "%s": %s,' % (n, compact(c)) for n, c in clips.items())
    head = '\n    "default": {'
    assert text.count(head) == 1, path
    text = text.replace(head, head + lines)
    json.loads(text)
    open(path, 'w').write(text)
    print(name, {n: len(compact(c)) for n, c in clips.items()})
