"""Make the bark textures of the pack: content/textures/log_side.png and
birch_log_side.png, as voxel bark (flat cells an eighth of a metre across,
in upright ridges and grooves, each cell its own shade) rather than painted
strokes. Plain PNGs: repaint them in any editor, or change this and run it.

usage: python3 tools/cover/bark.py [OUT_DIR]   (default content/textures)
"""
import random, struct, sys, zlib

SIZE, CELL = 32, 4


def png(path, rows):
    raw = b''.join(b'\0' + bytes(c for px in row for c in px) for row in rows)

    def chunk(kind, body):
        return struct.pack('>I', len(body)) + kind + body + struct.pack('>I', zlib.crc32(kind + body))

    head = struct.pack('>IIBBBBB', len(rows[0]), len(rows), 8, 2, 0, 0, 0)
    with open(path, 'wb') as f:
        f.write(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', head) + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b''))


def shade(c, k):
    return tuple(max(0, min(255, round(v * k))) for v in c)


def oak(seed):
    rnd = random.Random(seed)
    # warm lit ridges, deep grooves between them, a few pale worn cells and
    # a mossy one: bark that reads from across a meadow
    ridge, groove, worn, moss = (150, 98, 54), (74, 46, 28), (178, 126, 76), (96, 118, 52)
    n = SIZE // CELL
    kinds = [rnd.choice([0, 0, 1]) for _ in range(n)]
    kinds[rnd.randrange(n)] = 1
    kinds[rnd.randrange(n)] = 0
    cells = [[None] * n for _ in range(n)]
    for x in range(n):
        for y in range(n):
            r = rnd.random()
            deep = kinds[x] == 1 or r < 0.12
            base = groove if deep else (worn if r > 0.84 else ridge)
            if not deep and rnd.random() < 0.04:
                base = moss
            cells[y][x] = shade(base, rnd.uniform(0.84, 1.16))
    rows = [[cells[y // CELL][x // CELL] for x in range(SIZE)] for y in range(SIZE)]
    # a darker pixel down the left of each ridge cell, a lighter one down its
    # right: every cell a small lit block
    for y in range(SIZE):
        for x in range(SIZE):
            if kinds[x // CELL] == 0:
                if x % CELL == 0:
                    rows[y][x] = shade(rows[y][x], 0.82)
                elif x % CELL == CELL - 1:
                    rows[y][x] = shade(rows[y][x], 1.1)
    return rows


def birch(seed):
    rnd = random.Random(seed)
    pale, grey, mark = (232, 228, 214), (198, 196, 186), (52, 48, 46)
    n = SIZE // CELL
    cells = [[shade(pale if rnd.random() < 0.78 else grey, rnd.uniform(0.94, 1.04)) for _ in range(n)] for _ in range(n)]
    # dark dashes across the trunk
    for _ in range(4):
        y, x = rnd.randrange(n), rnd.randrange(n)
        for i in range(rnd.choice([1, 2, 2])):
            cells[y][(x + i) % n] = shade(mark, rnd.uniform(0.9, 1.2))
    return [[cells[y // CELL][x // CELL] for x in range(SIZE)] for y in range(SIZE)]


if __name__ == '__main__':
    out = sys.argv[1] if len(sys.argv) > 1 else 'content/textures'
    png(f'{out}/log_side.png', oak(7))
    png(f'{out}/birch_log_side.png', birch(11))
