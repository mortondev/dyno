"""MagicaVoxel .vox read/write, mirroring crates/dyno-client/src/vox.rs.

Game voxel (x, y, z) is stored at (x, -z - 1, y).
read(path) -> (models: list[(name, {(x,y,z): colour})], palette: list[256 of (r,g,b,a)])
"""
import struct


def _i32(b, at):
    return struct.unpack_from('<i', b, at)[0], at + 4


def _string(b, at):
    n, at = _i32(b, at)
    n = max(n, 0)
    return b[at:at + n].decode('utf8', 'replace'), at + n


def _dict(b, at):
    n, at = _i32(b, at)
    d = {}
    for _ in range(max(n, 0)):
        k, at = _string(b, at)
        v, at = _string(b, at)
        d[k] = v
    return d, at


def read(path):
    data = open(path, 'rb').read()
    assert data[:4] == b'VOX ' and data[8:12] == b'MAIN'
    at = 20
    sizes, raw, nodes = [], [], {}
    palette = [(0, 0, 0, 255)] * 256
    while at + 12 <= len(data):
        cid = data[at:at + 4]
        content, children = struct.unpack_from('<ii', data, at + 4)
        body = data[at + 12:at + 12 + content]
        at += 12 + content
        if cid == b'SIZE':
            sizes.append(struct.unpack_from('<iii', body, 0))
        elif cid == b'XYZI':
            n = struct.unpack_from('<i', body, 0)[0]
            raw.append([tuple(body[4 + 4 * i:8 + 4 * i]) for i in range(n)])
        elif cid == b'RGBA':
            palette = [(0, 0, 0, 255)] + [tuple(body[4 * i:4 * i + 4]) for i in range(255)]
        elif cid == b'nTRN':
            node, c = _i32(body, 0)
            attrs, c = _dict(body, c)
            child, c = _i32(body, c)
            c += 8
            frames, c = _i32(body, c)
            t = (0, 0, 0)
            for f in range(max(frames, 0)):
                d, c = _dict(body, c)
                if f == 0 and '_t' in d:
                    v = [int(s) for s in d['_t'].split()]
                    if len(v) == 3:
                        t = tuple(v)
            nodes[node] = ('T', attrs.get('_name'), child, t)
        elif cid == b'nGRP':
            node, c = _i32(body, 0)
            nodes[node] = ('G',)
        elif cid == b'nSHP':
            node, c = _i32(body, 0)
            _, c = _dict(body, c)
            n, c = _i32(body, c)
            model = _i32(body, c)[0] if n > 0 else -1
            nodes[node] = ('S', model)
    placed = {}
    for n in nodes.values():
        if n[0] == 'T' and nodes.get(n[2], ('',))[0] == 'S':
            model = nodes[n[2]][1]
            placed[model] = (n[1] if n[1] is not None else str(model), n[3])
    models = []
    for i, (size, vox) in enumerate(zip(sizes, raw)):
        name, t = placed.get(i, (str(i), (size[0] // 2, size[1] // 2, size[2] // 2)))
        lo = (t[0] - size[0] // 2, t[1] - size[1] // 2, t[2] - size[2] // 2)
        cells = {}
        for v in vox:
            p = (lo[0] + v[0], lo[1] + v[1], lo[2] + v[2])
            cells[(p[0], p[2], -p[1] - 1)] = v[3]
        models.append((name, cells))
    return models, palette


def _chunk(cid, body):
    return cid + struct.pack('<ii', len(body), 0) + body


def _s(s):
    b = s.encode()
    return struct.pack('<i', len(b)) + b


def _d(items):
    out = struct.pack('<i', len(items))
    for k, v in items:
        out += _s(k) + _s(v)
    return out


def write(path, models, palette):
    body = b''
    places = []
    for name, cells in models:
        ps = [((x, -z - 1, y), c) for (x, y, z), c in cells.items()]
        if ps:
            lo = [min(p[0][a] for p in ps) for a in range(3)]
            hi = [max(p[0][a] for p in ps) for a in range(3)]
        else:
            lo, hi = [0, 0, 0], [0, 0, 0]
        size = [hi[a] - lo[a] + 1 for a in range(3)]
        assert max(size) <= 256, (name, size)
        body += _chunk(b'SIZE', struct.pack('<iii', *size))
        x = struct.pack('<i', len(ps))
        for p, c in ps:
            x += bytes([p[0] - lo[0], p[1] - lo[1], p[2] - lo[2], c])
        body += _chunk(b'XYZI', x)
        places.append([lo[a] + size[a] // 2 for a in range(3)])
    body += _chunk(b'nTRN', struct.pack('<i', 0) + _d([]) + struct.pack('<iiii', 1, -1, -1, 1) + _d([]))
    g = struct.pack('<i', 1) + _d([]) + struct.pack('<i', len(models))
    for i in range(len(models)):
        g += struct.pack('<i', 2 + 2 * i)
    body += _chunk(b'nGRP', g)
    for i, ((name, _), at) in enumerate(zip(models, places)):
        nid = 2 + 2 * i
        body += _chunk(b'nTRN', struct.pack('<i', nid) + _d([('_name', name)])
                       + struct.pack('<iiii', nid + 1, -1, 0, 1) + _d([('_t', f'{at[0]} {at[1]} {at[2]}')]))
        body += _chunk(b'nSHP', struct.pack('<i', nid + 1) + _d([]) + struct.pack('<ii', 1, i) + _d([]))
    pal = b''.join(bytes(c) for c in palette[1:]) + bytes(4)
    body += _chunk(b'RGBA', pal)
    out = b'VOX ' + struct.pack('<i', 200) + b'MAIN' + struct.pack('<ii', 0, len(body)) + body
    open(path, 'wb').write(out)


def show(cells, axis=1, at=None):
    """Print a slice (axis: 0 x, 1 y, 2 z) at coordinate `at` (default: middle)."""
    if not cells:
        print('(empty)')
        return
    lo = [min(p[a] for p in cells) for a in range(3)]
    hi = [max(p[a] for p in cells) for a in range(3)]
    if at is None:
        at = (lo[axis] + hi[axis]) // 2
    ua, va = [a for a in range(3) if a != axis]
    for v in range(hi[va], lo[va] - 1, -1):
        row = ''
        for u in range(lo[ua], hi[ua] + 1):
            p = [0, 0, 0]
            p[axis], p[ua], p[va] = at, u, v
            c = cells.get(tuple(p), 0)
            row += '.' if c == 0 else '0123456789abcdefghijklmnopqrstuvwxyz'[c % 36]
        print(row)
