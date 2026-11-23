"""Print a stamp: its header and one layer (the ground layer by default).

    python3 tools/stamps/show.py content/stamps/inn.dstamp [layer above the ground]
"""
import json, sys


def load(path):
    d = open(path, "rb").read()
    _, head, body = d.split(b"\n", 2)
    h = json.loads(head)
    cells, p = [], 0
    while p < len(body):
        v = body[p]
        p += 1
        n = sh = 0
        while True:
            b = body[p]
            p += 1
            n |= (b & 0x7F) << sh
            if not b & 0x80:
                break
            sh += 7
        cells += [v] * n
    return h, cells


if __name__ == "__main__":
    h, cells = load(sys.argv[1])
    up = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    print({k: v for k, v in h.items() if k != "palette"})
    sx, sy, sz = h["size"]
    y = h["anchor"][1] + up
    for z in range(sz):
        row = ""
        for x in range(sx):
            key = h["palette"][cells[(y * sz + z) * sx + x]]
            row += "." if key == "skip" else "_" if key == "air" else key[0]
        print(row, z)
