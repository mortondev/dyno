"""Round 2 of the first 30 minutes: deaths attributed, travel runs, reward gaps
and tags, from `dyno bots` reports (docs/playtest/first-30-minutes-round2.md).

    python3 tools/playtest/round2.py REPORT.json [REPORT.json ...]     every table, the runs pooled
    python3 tools/playtest/round2.py --xp                              quest XP against the levels, from the rules
    python3 tools/playtest/round2.py --tags REPORT.json [...]          a crowd at one camp: one row per report
    python3 tools/playtest/round2.py --rules RULES.json --layout LAYOUT.json REPORT.json ...
                                             classify with the rules and the layout the runs were made on
                                             (git show COMMIT:content/rules/hearthblock.json, .../world/layout.json)

The bots record facts (who was near, health at the pull, the time of day);
this script decides what they mean, from the rules and the layout, so a
definition can change without another swarm.

Definitions:
- on a road: within 10 m of a road or trail of the layout (straight between its stops).
- in a quest area: within 45 m of a camp of a creature that a quest of level 5
  or under asks for (to kill, or for what it drops), within an explore
  objective's circle + 10 m, within 30 m of a quest ore place, or in a safe hub.
- engaged: the pull's target, plus every creature within 6 m at the death that
  is hostile by its rules or that the bot had hit in the last 10 s.
- a death's cause, the first that fits: over-level (killer 2+ levels above),
  night (a Shade), adds (2+ engaged), jumped (under 60% health when a creature
  attacked it: the last "pull" was a defence), low pull (under 60% health at a
  pull the bot chose: the brain's), else one-on-one (the class lane's).
- a night death: any death while it is night; "to the night" is a death to a Shade.
"""
import gzip, json, math, os, statistics, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import beats  # noqa: E402

ROOT = os.path.join(HERE, "..", "..")
# the rules the reports were made on: quest areas and camps are read from them
_RULES_PATH = os.path.join(ROOT, "content", "rules", "hearthblock.json")
if "--rules" in sys.argv:
    _i = sys.argv.index("--rules")
    _RULES_PATH = sys.argv[_i + 1]
    del sys.argv[_i:_i + 2]
RULES = json.load(open(_RULES_PATH))
_LAYOUT_PATH = os.path.join(ROOT, "content", "world", "layout.json")
if "--layout" in sys.argv:
    _i = sys.argv.index("--layout")
    _LAYOUT_PATH = sys.argv[_i + 1]
    del sys.argv[_i:_i + 2]
LAYOUT = json.load(open(_LAYOUT_PATH))
PLACES = {p["key"]: p for p in LAYOUT["places"]}
BRAINS = ["completionist", "casual", "explorer", "social", "party", "grinder"]


def read(path):
    """A report, plain or gzipped (the 30-minute runs are kept as .json.gz)."""
    with (gzip.open(path, "rt") if path.endswith(".gz") else open(path)) as f:
        return json.load(f)


def place_at(key, at=None):
    p = PLACES[key]["at"]
    if not at:
        return (p[0], p[1])
    return (p[0] + at[0], p[1] + at[-1])


def roads():
    out = []
    for r in LAYOUT["roads"]:
        pts = [place_at(s) if isinstance(s, str) else tuple(s) for s in r["path"]]
        out += list(zip(pts, pts[1:]))
    return out


ROADS = roads()


def seg_dist(p, a, b):
    ax, az = a
    bx, bz = b
    dx, dz = bx - ax, bz - az
    l2 = dx * dx + dz * dz
    t = 0 if l2 == 0 else max(0, min(1, ((p[0] - ax) * dx + (p[1] - az) * dz) / l2))
    return math.hypot(p[0] - ax - t * dx, p[1] - az - t * dz)


def on_road(p):
    return min(seg_dist(p, a, b) for a, b in ROADS) <= 10


def quest_circles():
    """(x, z, radius, what) of everything a quest of level 5 or under sends you to."""
    out = []
    quests = [q for q in RULES["quests"] if q.get("level", 1) <= 5]
    want = set()
    drops = {}
    for c in RULES["creatures"]:
        for d in c.get("drops", []):
            drops.setdefault(d["item"], set()).add(c["key"])
    for q in quests:
        for o in q["objectives"]:
            if "kill" in o:
                want.add(o["kill"])
            if "collect" in o:
                want |= drops.get(o["collect"], set())
            if "explore" in o:
                x, z = place_at(o["place"], o.get("at"))
                out.append((x, z, o.get("radius", 12) + 10, q["key"]))
    for a in RULES["areas"]:
        if a.get("safe"):
            x, z = place_at(a["place"], a.get("at"))
            r = a["radius"] if a["radius"] != "place" else PLACES[a["place"]].get("radius", 20)
            out.append((x, z, r, a["name"]))
        for c in a.get("camps", []):
            if c["creature"] in want:
                x, z = place_at(c["place"], c.get("at"))
                out.append((x, z, 45, c["creature"] + " camp"))
    for k in ("old_quarry", "gate_rocks", "beacon_rocks"):
        if k in PLACES:
            x, z = place_at(k)
            out.append((x, z, 30, PLACES[k].get("name") or k))
    return out


CIRCLES = quest_circles()


def quest_area(p):
    for x, z, r, what in CIRCLES:
        if math.hypot(p[0] - x, p[1] - z) <= r:
            return what
    return None


def nearest_place(p):
    best = min(
        (pl for pl in LAYOUT["places"] if pl.get("name") or pl["kind"] in ("hub", "site", "wayside", "camp")),
        key=lambda pl: math.hypot(p[0] - pl["at"][0], p[1] - pl["at"][1]))
    d = math.hypot(p[0] - best["at"][0], p[1] - best["at"][1])
    name = best.get("name") or best["key"]
    return name if d < 30 else f"{d:.0f} m from {name}"


def mmss(x):
    return "-" if x is None else f"{int(x // 60)}:{int(x % 60):02d}"


def med(xs):
    xs = [x for x in xs if x is not None]
    return statistics.median(xs) if xs else None


def load(paths):
    reps = []
    for i, p in enumerate(paths):
        r = read(p)
        for t in r["telemetry"]:
            t["run"] = chr(ord("A") + i)
        reps.append(r)
    return reps


def machine(reps, paths):
    print("| run | report | load: start / mean / max (cores) | server tick: mean / p95 / p99 / worst | worst minute's p95 | counts (p95 < 50 ms) |")
    print("|---|---|---|---|---|---|")
    for i, (r, p) in enumerate(zip(reps, paths)):
        m = r.get("machine") or {}
        t = m.get("tick")
        if t and t.get("note"):
            # a swarm against a separate server: the worst minute's p95 bounds the run's
            tick = f"{t['mean_ms']:.2f} / under {t['p95_ms']:.2f} / not measured / {t['worst_ms']:.0f} ms"
        else:
            tick = f"{t['mean_ms']:.2f} / {t['p95_ms']:.2f} / {t['p99_ms']:.2f} / {t['worst_ms']:.0f} ms" if t else "not measured"
        wm = f"{t['worst_minute_p95_ms']:.2f} ms" if t else "-"
        ok = ("yes" if t["p95_ms"] < 50 else "NO") if t else "?"
        print(f"| {chr(ord('A') + i)} | {os.path.basename(p)} | {m.get('load_start', 0):.1f} / {m.get('load_mean', 0):.1f} / {m.get('load_max', 0):.1f} ({m.get('cores', '?')}) | {tick} | {wm} | {ok} |")
    print()


def gaps_of(t):
    """Stretches without a rewarding moment: (start, length, end is session end)."""
    ts = [0.0]
    for x, _ in t["rewards"]:
        if x - ts[-1] > 2.0 or len(ts) == 1:
            ts.append(x)
    out = [(a, b - a, False) for a, b in zip(ts, ts[1:])]
    out.append((ts[-1], t["secs"] - ts[-1], True))
    return out


def active_secs(t):
    return t["secs"] - t["time"].get("afk", 0.0)


def group_table(tels, key, order=None):
    groups = {}
    for t in tels:
        groups.setdefault(t[key], []).append(t)
    names = [n for n in (order or sorted(groups)) if n in groups] + ["all"]
    groups["all"] = tels
    cols = ["first kill", "level 2", "level 3", "level 4", "level 5", "level at end", "quests done", "gear reward",
            "first upgrade", "median gap", "longest gap", "worst gap", "travel %", "fight %", "deaths", "bot-hours",
            "deaths/h", "lost kills %", "money"]
    print(f"| {key} (bots) | " + " | ".join(cols) + " |")
    print("|---|" + "---|" * len(cols))
    for n in names:
        g = groups[n]
        r = beats.row(g)
        hours = sum(active_secs(t) for t in g) / 3600
        deaths = sum(len(t["deaths"]) for t in g)
        secs = sum(active_secs(t) for t in g)
        kills = sum(t["kills"] for t in g)
        lost = sum(t["kills_no_credit"] - t.get("kills_no_credit_alone", 0) for t in g)
        longest = [max(l for _, l, _ in gaps_of(t)) for t in g]
        vals = {
            "first kill": mmss(r["first kill"]), "gear reward": mmss(r["gear reward"]),
            "first upgrade": mmss(r["first upgrade"]),
            "level at end": f"{r['level at end']:g}", "quests done": f"{r['quests']:g}",
            "median gap": f"{r['median gap']:.0f} s", "longest gap": mmss(med(longest)), "worst gap": mmss(max(longest)),
            "travel %": f"{100 * sum(t['time'].get('travel', 0) for t in g) / secs:.0f}%",
            "fight %": f"{100 * sum(t['time'].get('fight', 0) for t in g) / secs:.0f}%",
            "deaths": str(deaths), "bot-hours": f"{hours:.2f}", "deaths/h": f"{deaths / hours:.1f}",
            "lost kills %": f"{100 * lost / max(1, kills + lost):.0f}%",
            "money": f"{med([t['money'] for t in g]):g}",
        }
        for l in (2, 3, 4, 5):
            vals[f"level {l}"] = mmss(r[f"level {l}"])
        print(f"| {n} ({len(g)}) | " + " | ".join(vals[c] for c in cols) + " |")
    print()


def death_rows(tels):
    rows = []
    for t in tels:
        for d in t["deaths"]:
            pull = t["pulls"][d["pull"]] if d.get("pull") is not None and d["pull"] < len(t.get("pulls", [])) else None
            near = d.get("near", [])
            killer = d.get("killer") or d["cause"]
            klevel = d.get("killer_level", 0)
            if not klevel and pull and pull["creature"] == killer:
                klevel = pull["level"]
            gap = klevel - d["level"] if klevel else None
            engaged = [n for n in near if n["dist"] <= 6 and (n["hostile"] or n["mine"])]
            if pull and not any(n["name"] == pull["creature"] and n["dist"] <= 6 for n in near):
                # the target is not among them (it died, or ran): it still counted
                pass
            names = {}
            for n in engaged:
                k = f"{n['name']} {n['level']}"
                names[k] = names.get(k, 0) + 1
            p = (d["at"][0], d["at"][2])
            qa = quest_area(p)
            road = on_road(p)
            hp = pull["hp"] if pull else None
            if gap is not None and gap >= 2:
                why = "over-level"
            elif killer == "Shade":
                why = "night"
            elif len(engaged) >= 2:
                why = "adds"
            elif hp is not None and hp < 0.6 and pull["doing"].startswith("defending"):
                why = "jumped"
            elif hp is not None and hp < 0.6:
                why = "low pull"
            else:
                why = "one-on-one"
            rows.append({
                "run": t["run"], "bot": t["name"][-5:], "brain": t["brain"], "class": t["class"], "t": d["t"],
                "level": d["level"], "killer": killer, "klevel": klevel, "gap": gap,
                "engaged": ", ".join(f"{n}×{c}" if c > 1 else n for n, c in names.items()) or "none in reach",
                "n": len(engaged), "night": d.get("night", False), "hp": hp,
                "pull": (f"{pull['creature']} {pull['level']}" + (" (attacked)" if pull["doing"].startswith("defending") else ""))
                if pull else "no pull in the last minute",
                "padds": pull["adds"] if pull else None,
                "where": nearest_place(p), "zone": d["zone"], "qa": qa, "road": road, "why": why,
                "doing": d.get("doing", ""), "players": d.get("players_near", 0), "at": p,
            })
    return rows


def deaths(tels):
    rows = death_rows(tels)
    print("| run | bot | brain | class | at | lv | killer (gap) | engaged at death | day/night | health at pull | pulled | where | quest area | road | cause |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for r in sorted(rows, key=lambda r: (BRAINS.index(r["brain"]) if r["brain"] in BRAINS else 9, r["run"], r["t"])):
        gap = "?" if r["gap"] is None else f"{r['gap']:+d}"
        hp = "-" if r["hp"] is None else f"{100 * r['hp']:.0f}%"
        print(f"| {r['run']} | {r['bot']} | {r['brain']} | {r['class']} | {mmss(r['t'])} | {r['level']} | {r['killer']} {r['klevel'] or '?'} ({gap}) | {r['engaged']} | {'night' if r['night'] else 'day'} | {hp} | {r['pull']} | {r['where']} | {r['qa'] or 'outside'} | {'yes' if r['road'] else 'no'} | {r['why']} |")
    print()
    # by brain and cause
    causes = ["over-level", "night", "adds", "jumped", "low pull", "one-on-one"]
    print("| brain | bot-hours | deaths | per hour | " + " | ".join(causes) + " | deaths at night at level 1-4 on a road or in a quest area | of those, to a Shade |")
    print("|---|---|---|---|" + "---|" * len(causes) + "---|---|")
    for b in BRAINS + ["all"]:
        g = [t for t in tels if b == "all" or t["brain"] == b]
        if not g:
            continue
        hours = sum(active_secs(t) for t in g) / 3600
        rs = [r for r in rows if b == "all" or r["brain"] == b]
        bad = [r for r in rs if r["night"] and r["level"] <= 4 and (r["road"] or r["qa"])]
        print(f"| {b} | {hours:.2f} | {len(rs)} | {len(rs) / hours:.1f} | " + " | ".join(str(sum(r['why'] == c for r in rs)) for c in causes) + f" | {len(bad)} | {sum(r['killer'] == 'Shade' for r in bad)} |")
    print()
    one = [r for r in rows if r["why"] == "one-on-one"]
    if one:
        print("One-on-one losses at the same level or below (for the class lane):\n")
        for r in one:
            hp = "-" if r["hp"] is None else f"{100 * r['hp']:.0f}%"
            print(f"- {r['class']} {r['level']} ({r['brain']}) to {r['killer']} {r['klevel'] or '?'}, pulled at {hp} health, {'night' if r['night'] else 'day'}, {r['where']}")
        print()
    return rows


def pulls(tels):
    """How the bots pull: health at the pull and adds, per brain."""
    print("| brain | fights | pulled under 60% health | under 40% | fights with an add | with two or more | at night |")
    print("|---|---|---|---|---|---|---|")
    for b in BRAINS + ["all"]:
        ps = [p for t in tels if b == "all" or t["brain"] == b for p in t.get("pulls", [])]
        if not ps:
            continue
        n = len(ps)
        f = lambda c: f"{100 * sum(1 for p in ps if c(p)) / n:.0f}%"
        print(f"| {b} | {n} | {f(lambda p: p['hp'] < 0.6)} | {f(lambda p: p['hp'] < 0.4)} | {f(lambda p: p['adds'] >= 1)} | {f(lambda p: p['adds'] >= 2)} | {f(lambda p: p['night'])} |")
    print()


def travel(tels):
    print("| brain | travel | fight | gather | town | rest | loot | waiting for spawns | dead | idle + stuck |")
    print("|---|---|---|---|---|---|---|---|---|---|")
    for b in BRAINS + ["all"]:
        g = [t for t in tels if b == "all" or t["brain"] == b]
        if not g:
            continue
        secs = sum(active_secs(t) for t in g)
        s = lambda *ks: f"{100 * sum(t['time'].get(k, 0) for t in g for k in ks) / secs:.0f}%"
        print(f"| {b} ({len(g)}) | {s('travel')} | {s('fight')} | {s('gather')} | {s('town')} | {s('rest')} | {s('loot')} | {s('wait_spawn')} | {s('dead')} | {s('idle', 'stuck')} |")
    print()
    # what the travelling was for: seconds of travel by purpose (the trail's 5 s samples)
    why = {}
    total = 0
    for t in tels:
        if t["brain"] == "grinder":
            continue
        for s, x, z, act, doing in t.get("trail", []):
            if act != "travel":
                continue
            k = purpose(doing)
            why[k] = why.get(k, 0) + 5
            total += 5
    print("Travel by purpose (all but grinders), share of travel time:\n")
    print("| what the bot was on its way to | minutes | share |")
    print("|---|---|---|")
    for k, v in sorted(why.items(), key=lambda kv: -kv[1])[:22]:
        print(f"| {k} | {v / 60:.1f} | {100 * v / max(1, total):.0f}% |")
    print()
    # the longest runs, one line per route
    routes = {}
    for t in tels:
        for r in t.get("runs", []):
            k = (nearest_place(r["from"]), nearest_place(r["to"]), purpose(r["end"] or r["start"]))
            routes.setdefault(k, []).append((r["secs"], t))
    print("The longest runs (unbroken travel), by route:\n")
    print("| from | to | for | longest | median | times | bots |")
    print("|---|---|---|---|---|---|---|")
    top = sorted(routes.items(), key=lambda kv: -max(s for s, _ in kv[1]))[:12]
    for (a, b, w), rs in top:
        secs = [s for s, _ in rs]
        print(f"| {a} | {b} | {w} | {mmss(max(secs))} | {mmss(statistics.median(secs))} | {len(rs)} | {len({t['name'] for _, t in rs})} |")
    print()


def purpose(doing):
    d = doing.strip()
    if not d:
        return "(nothing)"
    for lead in ("visiting ", "quest business with "):
        if d.startswith(lead):
            return d
    if ":" in d:
        return d
    w = d.split()
    if w[0] in ("following", "helping", "waiting", "with"):
        return w[0] + " a party member"
    if w[0] == "taking" and len(w) > 1 and w[1].startswith("Bot"):
        return "taking a party member to an NPC"
    return d


def gaps(tels, limit=120):
    rows = []
    for t in tels:
        for start, length, last in gaps_of(t):
            if length <= limit:
                continue
            tr = [x for x in t.get("trail", []) if start <= x[0] <= start + length]
            acts, doing = {}, {}
            for _, _, _, a, d in tr:
                acts[a] = acts.get(a, 0) + 1
                doing[purpose(d)] = doing.get(purpose(d), 0) + 1
            top_act = sorted(acts.items(), key=lambda kv: -kv[1])
            top_do = sorted(doing.items(), key=lambda kv: -kv[1])[:2]
            a = (tr[0][1], tr[0][2]) if tr else (0, 0)
            b = (tr[-1][1], tr[-1][2]) if tr else (0, 0)
            rows.append((length, t, start, last, a, b, top_act, top_do, len(tr)))
    rows.sort(key=lambda r: -r[0])
    print(f"| run | bot | brain | from | gap | where: from, to | mostly | doing |")
    print("|---|---|---|---|---|---|---|---|")
    for length, t, start, last, a, b, acts, dos, n in rows:
        mostly = ", ".join(f"{k} {100 * v / max(1, n):.0f}%" for k, v in acts[:2])
        doing = "; ".join(k for k, _ in dos)
        end = " (to the end)" if last else ""
        print(f"| {t['run']} | {t['name'][-5:]} | {t['brain']} | {mmss(start)} | {mmss(length)}{end} | {nearest_place(a)} → {nearest_place(b)} | {mostly} | {doing} |")
    print()
    longest = [max(l for _, l, _ in gaps_of(t)) for t in tels]
    print(f"Longest gap per bot: median {mmss(med(longest))}, worst {mmss(max(longest))}; {len(rows)} gaps over {limit // 60}:00 among {len(tels)} bots.\n")


def quests(tels):
    stat = {}
    for t in tels:
        for q in t["quests"]:
            s = stat.setdefault(q["key"], [0, 0, []])
            s[0] += 1
            if q["done"] is not None:
                s[1] += 1
                s[2].append(q["done"])
    level = {q["key"]: q.get("level", 1) for q in RULES["quests"]}
    print("| quest | level | accepted | done | done at (median) |")
    print("|---|---|---|---|---|")
    for k, (a, d, ts) in sorted(stat.items(), key=lambda kv: (level.get(kv[0], 0), kv[0])):
        print(f"| {k} | {level.get(k, '?')} | {a} | {d} | {mmss(med(ts))} |")
    print()


XP_NEXT = [400, 900, 1400, 2100, 2800]  # dyno_sim::progression::xp_to_next, levels 1 to 5


def quest_xp(q):
    # content.rs default_quest_xp: about five same-level kills
    return q.get("xp", ((45 + 5 * q.get("level", 1)) * 5 + 9) // 10 * 10)


def xp_table():
    """Quest XP against the level curve: can a player reach level 5 on quests alone?"""
    by = {q["key"]: q for q in RULES["quests"]}
    rows = [q for q in RULES["quests"] if q.get("level", 1) <= 5]
    kills = {}
    scale = RULES["world"].get("kill_xp", 1.0)
    print("| quest | level | giver | objectives | XP | needs the night | group |")
    print("|---|---|---|---|---|---|---|")
    total = day = solo_day = 0
    kill_xp = 0
    for q in sorted(rows, key=lambda q: (q.get("level", 1), q["key"])):
        xp = quest_xp(q)
        chain, todo = set(), [q["key"]]
        while todo:
            k = todo.pop()
            if k in chain:
                continue
            chain.add(k)
            todo += by[k].get("requires", [])
        night = any(o.get("kill") == "shade" or o.get("collect") == "shade_dust" for k in chain for o in by[k]["objectives"])
        group = "(Group)" in q["title"]
        obj = ", ".join(
            f"kill {o['count']} {o['kill']}" if "kill" in o else f"collect {o['count']} {o['collect']}" if "collect" in o
            else f"talk to {o['talk']}" if "talk" in o else f"find {o['explore']}" for o in q["objectives"])
        print(f"| {q['title']} | {q.get('level', 1)} | {q['giver']} | {obj} | {xp} | {'yes' if night else ''} | {'yes' if group else ''} |")
        total += xp
        if not night:
            day += xp
            if not group:
                solo_day += xp
                for o in q["objectives"]:
                    if "kill" in o:
                        # the kills a quest asks for, at the quest's level (45 + 5 × level, scaled)
                        kill_xp += o["count"] * round((45 + 5 * q.get("level", 1)) * scale)
    need = sum(XP_NEXT[:4])
    places = len(RULES["areas"]) - 1
    disc = RULES["world"].get("discovery_xp", [0, 0])
    print(f"\nLevel 5 needs {need} XP ({' + '.join(map(str, XP_NEXT[:4]))}).")
    print(f"Quest XP at level 5 and under: {total}; by day {day}; by day and alone {solo_day}.")
    print(f"The kills those kill quests ask for pay about {kill_xp} more (collect quests' kills not counted).")
    print(f"Finding a place pays {disc[0]} + {disc[1]} a level; there are {places} to find.")


def tags(paths):
    """A crowd in one area: who got the kills they fought for."""
    cols = ["bots", "tick p95", "kills with credit", "kills lost to another's tag", "lost", "bots with no first kill by 2:00",
            "first kill", "Meat for the Pot done", "Thinning the Herd done", "Wool for Winter done", "level 2", "level 3",
            "XP an hour", "waiting for spawns", "travel", "deaths"]
    print("| run | " + " | ".join(cols) + " |")
    print("|---|" + "---|" * len(cols))
    for p in paths:
        r = read(p)
        tels = r["telemetry"]
        m = (r.get("machine") or {}).get("tick")
        kills = sum(t["kills"] for t in tels)
        lost = sum(t["kills_no_credit"] - t.get("kills_no_credit_alone", 0) for t in tels)
        secs = sum(t["secs"] for t in tels)

        def done(key):
            ts = [q["done"] for t in tels for q in t["quests"] if q["key"] == key and q["done"] is not None]
            return f"{mmss(med(ts))} ({len(ts)} of {len(tels)})"

        def level(l):
            ts = [next((x for x, lv, _ in t["xp_curve"] if lv >= l), None) for t in tels]
            got = [x for x in ts if x is not None]
            return f"{mmss(med(got))} ({len(got)} of {len(tels)})"
        share = lambda k: f"{100 * sum(t['time'].get(k, 0) for t in tels) / secs:.0f}%"
        vals = [str(len(tels)), f"{m['p95_ms']:.2f} ms" if m else "?", str(kills), str(lost),
                f"{100 * lost / max(1, kills + lost):.0f}%",
                str(sum(1 for t in tels if t["first_kill"] is None or t["first_kill"] > 120)),
                mmss(med([t["first_kill"] for t in tels])), done("meat_for_the_pot"), done("thin_the_herd"),
                done("wool_for_winter"), level(2), level(3),
                f"{med([t['xp_total'] / t['secs'] * 3600 for t in tels]):.0f}", share("wait_spawn"), share("travel"),
                str(sum(len(t["deaths"]) for t in tels))]
        print(f"| {os.path.basename(p).split('.json')[0]} | " + " | ".join(vals) + " |")


def main():
    args = sys.argv[1:]
    if args and args[0] == "--tags":
        tags(args[1:])
        return
    if args and args[0] == "--xp":
        xp_table()
        return
    reps = load(args)
    tels = [t for r in reps for t in r["telemetry"]]
    print("## Machine and tick\n")
    machine(reps, args)
    print("## Per player type (medians; deaths per hour pooled over the bot-hours)\n")
    group_table(tels, "brain", BRAINS)
    print("## Per class\n")
    group_table(tels, "class")
    print("## Deaths, each one\n")
    deaths(tels)
    print("## How the bots pull\n")
    pulls(tels)
    print("## Time and travel\n")
    travel(tels)
    print("## Reward gaps over 2:00\n")
    gaps(tels)
    print("## Quests\n")
    quests(tels)


if __name__ == "__main__":
    main()
