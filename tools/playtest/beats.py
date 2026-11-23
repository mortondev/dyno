"""The first-30-minutes beats from `dyno bots` reports (docs/design/quality-bar.md,
sections 3 to 5 and 12): one row per report, medians over its bots.

    python3 tools/playtest/beats.py REPORT.json [REPORT.json ...]

Times are minutes:seconds from the join. "-" means no bot got there.
"""
import json, math, statistics, sys


def med(xs):
    xs = [x for x in xs if x is not None]
    return statistics.median(xs) if xs else None


def mmss(x):
    return "-" if x is None else f"{int(x // 60)}:{int(x % 60):02d}"


def first(rewards, kind, after=0.0):
    return next((t for t, k in rewards if k == kind and t >= after), None)


def level_at(curve, level):
    return next((t for t, l, _ in curve if l >= level), None)


def row(path):
    d = json.load(open(path))
    tels = d["telemetry"]
    out = {}
    out["bots"] = len(tels)
    out["first kill"] = med([t["first_kill"] for t in tels])
    out["first loot"] = med([first(t["rewards"], "loot") for t in tels])
    out["first quest"] = med([t["first_quest"] for t in tels])
    out["out of town"] = med([
        next((s for s, x, z, _ in t["track"] if math.hypot(x - t["track"][0][1], z - t["track"][0][2]) > 80), None)
        if t["track"] else None for t in tels])
    out["first discovery"] = med([first(t["rewards"], "new_area") for t in tels])
    out["first upgrade"] = med([t.get("first_upgrade") for t in tels])

    def second_ability(t):
        casts = sorted(v for k, v in t.get("first_casts", {}).items() if k != "Recall")
        return casts[1] if len(casts) > 1 else None
    out["second ability"] = med([second_ability(t) for t in tels])
    for l in (2, 3, 4, 5):
        got = [level_at(t["xp_curve"], l) for t in tels]
        out[f"level {l}"] = med(got) if sum(g is not None for g in got) * 2 > len(got) else None
    out["level at end"] = med([t["level"] for t in tels])
    gaps, longest = [], []
    for t in tels:
        ts = [0.0] + [x for x, _ in t["rewards"]] + [t["secs"]]
        g = [b - a for a, b in zip(ts, ts[1:])]
        gaps.append(statistics.median(g))
        longest.append(max(g))
    out["reward gap"] = med(gaps)
    out["longest gap"] = med(longest)
    out["worst gap"] = max(longest)
    secs = sum(t["secs"] for t in tels)
    out["deaths/h"] = sum(len(t["deaths"]) for t in tels) / secs * 3600
    tm = {}
    for t in tels:
        for k, v in t["time"].items():
            tm[k] = tm.get(k, 0) + v
    out["travel %"] = 100 * tm.get("travel", 0) / max(1, sum(tm.values()))
    out["quests"] = med([sum(1 for q in t["quests"] if q["done"] is not None) for t in tels])
    out["money"] = med([t["money"] for t in tels])
    out["party at"] = med([t.get("first_party") for t in tels])
    return out


COLS = ["first kill", "first loot", "out of town", "first discovery", "first upgrade", "second ability",
        "level 2", "level 3", "level 4", "level 5", "level at end", "quests", "reward gap", "longest gap",
        "worst gap", "deaths/h", "travel %", "money", "party at"]
TIMES = {"first kill", "first loot", "out of town", "first discovery", "first upgrade", "second ability",
         "level 2", "level 3", "level 4", "level 5", "longest gap", "worst gap", "party at"}

print("| run | " + " | ".join(COLS) + " |")
print("|---|" + "---|" * len(COLS))
for p in sys.argv[1:]:
    try:
        r = row(p)
    except Exception as e:  # a run that did not finish
        print(f"| {p.split('/')[-1][:-5]} | (no report: {e}) |")
        continue
    cells = []
    for c in COLS:
        v = r[c]
        if c in TIMES:
            cells.append(mmss(v))
        elif v is None:
            cells.append("-")
        elif c in ("reward gap",):
            cells.append(f"{v:.0f} s")
        elif c in ("deaths/h",):
            cells.append(f"{v:.1f}")
        elif c == "travel %":
            cells.append(f"{v:.0f}%")
        else:
            cells.append(f"{v:g}")
    print(f"| {p.split('/')[-1][:-5]} | " + " | ".join(cells) + " |")
