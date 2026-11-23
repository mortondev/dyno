"""The canonical gameplay shots for the concept references and the visual gauntlet.

Renders the cameras of tools/concepts/cameras.json with `dyno shot --stand`:
a stand-in player on the ground facing each landmark, seen through the game's
own third-person camera (docs/design/scale.md), with no interface. The
cameras are data, chosen by `dyno scout` from tools/concepts/landmarks.json;
run that again when the world changes.

usage: python3 tools/concepts/shots.py OUT_DIR [ADDRESS] [name ...]

Writes OUT_DIR/<name>.jpg for each camera (or the named ones) and
OUT_DIR/contact.jpg, a sheet of every JPG of the cameras in OUT_DIR.
Environment: DYNO (the dyno binary, default ./target/debug/dyno),
SHOT_SIZE (default 1280x720).
"""
import json, os, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))


def cameras():
    with open(os.path.join(HERE, 'cameras.json')) as f:
        return json.load(f)['cameras']


def csv(v):
    return ','.join(f'{x:g}' for x in v)


def shoot(exe, addr, cam, out_dir, size):
    ppm = os.path.join(out_dir, cam['name'] + '.ppm')
    jpg = ppm[:-4] + '.jpg'
    r = subprocess.run(
        [exe, 'shot', addr, '--from-hearth',
         '--stand', csv(cam['stand']), '--face', csv(cam['face']),
         '--zoom', f"{cam['zoom']:g}", '--pitch', f"{cam['pitch']:g}",
         '--time', f"{cam['time']:g}", '--size', size, '--out', ppm],
        capture_output=True, text=True, timeout=600)
    if r.returncode != 0 or not os.path.exists(ppm):
        return False, (r.stderr or r.stdout).strip().splitlines()[-1:]
    subprocess.run(['magick', ppm, '-quality', '90', jpg], check=True)
    os.remove(ppm)
    return True, jpg


def contact(out_dir, names):
    have = [n for n in names if os.path.exists(os.path.join(out_dir, n + '.jpg'))]
    if not have:
        return None
    sheet = os.path.join(out_dir, 'contact.jpg')
    args = ['magick', 'montage']
    for n in have:
        args += ['-label', n, os.path.join(out_dir, n + '.jpg')]
    args += ['-tile', '3x', '-geometry', '640x360+6+6', '-background', '#181818',
             '-fill', '#e8e8e8', '-pointsize', '18', '-quality', '88', sheet]
    subprocess.run(args, check=True)
    return sheet


if __name__ == '__main__':
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    out_dir = sys.argv[1]
    addr = sys.argv[2] if len(sys.argv) > 2 else '127.0.0.1:7499'
    cams = cameras()
    known = [c['name'] for c in cams]
    names = sys.argv[3:] or known
    unknown = [n for n in names if n not in known]
    if unknown:
        sys.exit(f"no camera called {', '.join(unknown)}; cameras.json has: {', '.join(known)}")
    os.makedirs(out_dir, exist_ok=True)
    exe = os.environ.get('DYNO', './target/debug/dyno')
    size = os.environ.get('SHOT_SIZE', '1280x720')
    failed = []
    for cam in cams:
        if cam['name'] not in names:
            continue
        ok, what = shoot(exe, addr, cam, out_dir, size)
        print(cam['name'], what if ok else f'FAILED: {what}', flush=True)
        if not ok:
            failed.append(cam['name'])
    sheet = contact(out_dir, known)
    if sheet:
        print('contact sheet', sheet)
    if failed:
        sys.exit(f"failed: {', '.join(failed)}")
