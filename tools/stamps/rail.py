"""The rail block's shape and texture: content/shapes/rail.vox and
content/textures/rail.png. Run from the workspace:

    python3 tools/stamps/rail.py

The shape is a length of mine track in one cell, 16 voxels a block: two
iron rails running along x on two timber sleepers. The block's rules
(blocks.json, shape_rules.rail) turn it to follow its neighbours.
"""
import os, struct, sys, zlib
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import vox

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "content")


def png(path, rows):
    raw = b"".join(b"\x00" + bytes(v for px in row for v in px) for row in rows)
    def chunk(kind, data):
        c = kind + data
        return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", len(rows[0]), len(rows), 8, 6, 0, 0, 0))
                + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def rnd(x, y, salt=0):
    h = (x * 0x27D4EB2D ^ y * 0x165667B1 ^ salt * 0x9E3779B9) & 0xFFFFFFFF
    h ^= h >> 15
    h = (h * 0x2C1B3C6D) & 0xFFFFFFFF
    return (h ^ (h >> 12)) & 0xFF


def main():
    # iron: dark grey-brown in flat cells four pixels across, a worn bright line along the top
    rows = []
    for y in range(32):
        row = []
        for x in range(32):
            n = rnd(x // 4, y // 4, 3) % 22
            base = (78 + n, 72 + n, 70 + n)
            if y // 4 in (0, 7):
                base = (118 + n, 116 + n, 120 + n)
            if rnd(x // 4, y // 4, 9) % 11 == 0:
                base = (104, 70, 48)      # a fleck of rust
            row.append(base + (255,))
        rows.append(row)
    png(os.path.join(ROOT, "textures", "rail.png"), rows)
    # colour 1: the rails (the block's own texture); colour 2: the sleepers (dark planks)
    body = {}
    for sx in ((2, 3, 4), (10, 11, 12)):
        for x in sx:
            for z in range(1, 15):
                for y in (0, 1):
                    body[(x, y, z)] = 2
    for rz in ((3, 4), (11, 12)):
        for z in rz:
            for x in range(16):
                for y in (2, 3):
                    body[(x, y, z)] = 1
    vox.write(os.path.join(ROOT, "shapes", "rail.vox"), [("track.body.0", body)],
              {1: (96, 90, 88), 2: (74, 52, 34)})
    print("rail.vox:", len(body), "voxels; rail.png: 32x32")


if __name__ == "__main__":
    main()
