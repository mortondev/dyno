"""The first-30-minutes beats from `dyno bots` reports (docs/design/quality-bar.md,
sections 3 to 5 and 12): one row per report, medians over its bots.

    python3 tools/playtest/beats.py REPORT.json [REPORT.json ...]
    python3 tools/playtest/beats.py --by class|brain REPORT.json    (one row per class or brain of one mixed swarm)

Times are minutes:seconds from the join. "-" means no bot got there.
"""
import json, math, os, statistics, sys

# quests that pay in gear (a choice of items), from the pack's rules
_RULES = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "content", "rules", "hearthblock.json")
GEAR_QUESTS = {q["key"] for q in json.load(open(_RULES))["quests"] if q.get("choice")}


def med(xs):
    xs = [x for x in xs if x is not None]
    return statistics.median(xs) if xs else None


def mmss(x):
    return "-" if x is None else f"{int(x // 60)}:{int(x % 60):02d}"


def first(rewards, kind, after=0.0):
    return next((t for t, k in rewards if k == kind and t >= after), None)


def level_at(curve, level):
    return next((t for t, l, _ in curve if l >= level), None)


def row(tels):
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
    out["gear reward"] = med([
        min((q["done"] for q in t["quests"] if q["done"] is not None and q["key"] in GEAR_QUESTS), default=None)
        for t in tels])

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
        # moments within 2 s of each other are one (a kill, its loot, a level)
        ts = [0.0]
        for x, _ in t["rewards"]:
            if x - ts[-1] > 2.0 or len(ts) == 1:
                ts.append(x)
        ts.append(t["secs"])
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


COLS = ["first kill", "first loot", "out of town", "first discovery", "gear reward", "first upgrade", "second ability",
        "level 2", "level 3", "level 4", "level 5", "level at end", "quests", "reward gap", "longest gap",
        "worst gap", "deaths/h", "travel %", "money", "party at"]
TIMES = {"gear reward", "first kill", "first loot", "out of town", "first discovery", "first upgrade", "second ability",
         "level 2", "level 3", "level 4", "level 5", "longest gap", "worst gap", "party at"}

print("| run | " + " | ".join(COLS) + " |")
print("|---|" + "---|" * len(COLS))
args = sys.argv[1:]
by = None
if args and args[0] == "--by":
    by, args = args[1], args[2:]
groups = []
for p in args:
    name = p.split("/")[-1][:-5]
    try:
        tels = json.load(open(p))["telemetry"]
    except Exception as e:  # a run that did not finish
        print(f"| {name} | (no report: {e}) |")
        continue
    if by:
        for k in sorted({t[by] for t in tels}):
            groups.append((k, [t for t in tels if t[by] == k]))
        groups.append(("all", tels))
    else:
        groups.append((name, tels))
for name, tels in groups:
    r = row(tels)
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
    print(f"| {name} ({r['bots']}) | " + " | ".join(cells) + " |")
