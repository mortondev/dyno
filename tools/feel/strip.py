"""Contact strips from the clips `dyno drive` records (its `clip` command).

    python3 tools/feel/strip.py CLIPS_DIR CLASS OUT.jpg

A clip is a folder of frames (f0000.ppm ...) and frames.txt, one line a
frame: its number, the milliseconds since the clip began, and what the
client learned that frame (hit, crit, kill). CLIPS_DIR holds CLASS-hit,
CLASS-crit and CLASS-kill. The strip has a row for each: the frames
nearest to fixed moments around the blow (the first frame that carries
the row's event), cut out round the target, close together just after it, where a hit-stop, a shake
or a burst of particles lasts only a few frames. The same moments are
taken from every clip, so two strips can be set side by side.

Needs ImageMagick (`magick`).
"""
import os, shutil, subprocess, sys, tempfile

# milliseconds from the blow
MOMENTS = [-150, -50, 0, 35, 70, 110, 160, 240, 400, 700, 1100]
ROWS = ["hit", "crit", "kill"]
# the part of the picture round the target that a tile shows: share of width
# and height
CROP = (0.34, 0.5)
TILE = 208


def frames(clip):
    out = []
    with open(os.path.join(clip, "frames.txt")) as f:
        for line in f:
            w = line.split()
            if len(w) >= 2:
                at = [x for x in w[2:] if x.startswith("@")]
                at = tuple(float(v) for v in at[0][1:].split(",")) if at else None
                out.append((int(w[0]), float(w[1]), [x for x in w[2:] if not x.startswith("@")], at))
    return out


def pick(clip, event):
    fs = frames(clip)
    hit = next((f for f in fs if event in f[2]), None) or next((f for f in fs if f[2]), None)
    if hit is None:
        sys.exit(f"{clip}: no blow landed in the clip")
    chosen = []
    for m in MOMENTS:
        want = hit[1] + m
        best = min(fs, key=lambda f: abs(f[1] - want))
        chosen.append(os.path.join(clip, f"f{best[0]:04d}.ppm"))
    # where the target stood when the blow landed (or when last seen)
    seen = [f[3] for f in fs if f[3] and f[0] <= hit[0]] or [f[3] for f in fs if f[3]]
    return chosen, (seen[-1] if seen else (0.5, 0.55))


def main():
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    clips, cls, out = sys.argv[1:]
    tmp = tempfile.mkdtemp()
    tiles = []
    for row in ROWS:
        files, at = pick(os.path.join(clips, f"{cls}-{row}"), row)
        w, h = (
            int(x)
            for x in subprocess.check_output(["magick", "identify", "-format", "%w %h", files[0]]).split()
        )
        # the part of the picture round the target, where the blow lands
        cw, ch = int(w * CROP[0]), int(h * CROP[1])
        x = min(w - cw, max(0, int(at[0] * w) - cw // 2))
        y = min(h - ch, max(0, int(at[1] * h) - ch // 2))
        th = int(TILE * ch / cw)
        for f in files:
            t = os.path.join(tmp, f"{len(tiles):03d}.png")
            subprocess.check_call(
                ["magick", f, "-crop", f"{cw}x{ch}+{x}+{y}", "+repage", "-resize", f"{TILE}x{th}!", t]
            )
            tiles.append(t)
    cmd = ["magick", "montage"] + tiles + [
        "-tile", f"{len(MOMENTS)}x{len(ROWS)}", "-geometry", "+2+2", "-background", "#101010",
        "-quality", "78", out,
    ]
    os.makedirs(os.path.dirname(os.path.abspath(out)), exist_ok=True)
    subprocess.check_call(cmd)
    shutil.rmtree(tmp)
    print(out)


if __name__ == "__main__":
    main()
