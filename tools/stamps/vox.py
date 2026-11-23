"""MagicaVoxel `.vox` files as the game reads them (crates/dyno-client/src/vox.rs):
named models placed by their translation, a palette of 255 colours.

Game axes are y-up with +z forward; a game voxel (x, y, z) is stored at
(x, -z - 1, y).

    python3 tools/stamps/vox.py content/shapes/crate.vox     # list its models
"""
import struct, sys


def _chunk(cid, body):
    return cid + struct.pack("<ii", len(body), 0) + body


def _string(s):
    b = s.encode()
    return struct.pack("<i", len(b)) + b


def _dict(d):
    out = struct.pack("<i", len(d))
    for k, v in d:
        out += _string(k) + _string(v)
    return out


def write(path, models, palette):
    """models: [(name, {(x, y, z): colour index})]; palette: {index: (r, g, b)}."""
    body = b""
    places = []
    for name, vox in models:
        ps = [((x, -z - 1, y), c) for (x, y, z), c in sorted(vox.items())]
        lo = [min(p[0][a] for p in ps) for a in range(3)]
        hi = [max(p[0][a] for p in ps) for a in range(3)]
        size = [hi[a] - lo[a] + 1 for a in range(3)]
        body += _chunk(b"SIZE", struct.pack("<iii", *size))
        xyzi = struct.pack("<i", len(ps))
        for p, c in ps:
            xyzi += bytes([p[0] - lo[0], p[1] - lo[1], p[2] - lo[2], c])
        body += _chunk(b"XYZI", xyzi)
        places.append([lo[a] + size[a] // 2 for a in range(3)])
    body += _chunk(b"nTRN", struct.pack("<i", 0) + _dict([]) + struct.pack("<iiii", 1, -1, -1, 1) + _dict([]))
    grp = struct.pack("<i", 1) + _dict([]) + struct.pack("<i", len(models))
    for i in range(len(models)):
        grp += struct.pack("<i", 2 + 2 * i)
    body += _chunk(b"nGRP", grp)
    for i, ((name, _), at) in enumerate(zip(models, places)):
        nid = 2 + 2 * i
        body += _chunk(b"nTRN", struct.pack("<i", nid) + _dict([("_name", name)])
                       + struct.pack("<iiii", nid + 1, -1, 0, 1) + _dict([("_t", "%d %d %d" % tuple(at))]))
        body += _chunk(b"nSHP", struct.pack("<i", nid + 1) + _dict([]) + struct.pack("<ii", 1, i) + _dict([]))
    pal = b""
    for i in range(1, 256):
        r, g, b = palette.get(i, (0, 0, 0))
        pal += bytes([r, g, b, 255])
    pal += bytes(4)
    body += _chunk(b"RGBA", pal)
    with open(path, "wb") as f:
        f.write(b"VOX " + struct.pack("<i", 200) + b"MAIN" + struct.pack("<ii", 0, len(body)) + body)


def read(path):
    """[(name, {(x, y, z): colour})] in game coordinates."""
    d = open(path, "rb").read()
    at = 20
    sizes, voxels, names, trans, shapes = [], [], {}, {}, {}
    def rs(p):
        n = struct.unpack_from("<i", d, p)[0]
        return d[p + 4:p + 4 + n].decode(), p + 4 + n
    def rd(p):
        n = struct.unpack_from("<i", d, p)[0]
        p += 4
        out = {}
        for _ in range(n):
            k, p = rs(p)
            v, p = rs(p)
            out[k] = v
        return out, p
    while at < len(d):
        cid = d[at:at + 4]
        n, _ = struct.unpack_from("<ii", d, at + 4)
        b = at + 12
        if cid == b"SIZE":
            sizes.append(struct.unpack_from("<iii", d, b))
        elif cid == b"XYZI":
            k = struct.unpack_from("<i", d, b)[0]
            voxels.append([tuple(d[b + 4 + 4 * i:b + 8 + 4 * i]) for i in range(k)])
        elif cid == b"nTRN":
            nid = struct.unpack_from("<i", d, b)[0]
            attrs, p = rd(b + 4)
            child = struct.unpack_from("<i", d, p)[0]
            frame, _ = rd(p + 16)
            names[child] = attrs.get("_name", "")
            trans[child] = tuple(int(v) for v in frame.get("_t", "0 0 0").split())
        elif cid == b"nSHP":
            nid = struct.unpack_from("<i", d, b)[0]
            _, p = rd(b + 4)
            shapes[nid] = struct.unpack_from("<i", d, p + 4)[0]
        at = b + n
    out = []
    for nid, model in sorted(shapes.items(), key=lambda kv: kv[1]):
        size, t = sizes[model], trans.get(nid, (0, 0, 0))
        lo = [t[a] - size[a] // 2 for a in range(3)]
        vox = {}
        for (x, y, z, c) in voxels[model]:
            vx, vy, vz = x + lo[0], y + lo[1], z + lo[2]
            vox[(vx, vz, -vy - 1)] = c
        out.append((names.get(nid, ""), vox))
    return out


if __name__ == "__main__":
    for name, vox in read(sys.argv[1]):
        lo = [min(p[a] for p in vox) for a in range(3)]
        hi = [max(p[a] for p in vox) for a in range(3)]
        print(name, len(vox), "voxels", lo, hi, "colours", sorted(set(vox.values())))
