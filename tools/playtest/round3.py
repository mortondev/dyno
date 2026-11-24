"""Round 3 of the first 30 minutes (docs/playtest/first-30-minutes-round3.md):
the tables round 2's script does not print, from `dyno bots` reports.

    python3 tools/playtest/round3.py [--rules RULES.json] REPORT.json [REPORT.json ...]

- reward gaps among the questing bots (completionist, casual, explorer,
  social): the median of each bot's longest gap, the worst, and every gap
  over 2:00 with where and what the bot was doing;
- the five longest unbroken runs of travel by the questing bots;
- travel shares of the three types the goal names;
- looking for group: who grouped with whom through the server's word,
  and who killed Old Tusk, alone or with others.
"""
import os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import round2  # noqa: E402  (reads --rules and --layout from argv)

QUESTING = ["completionist", "casual", "explorer", "social"]


def main():
    paths = sys.argv[1:]
    reps = [(chr(ord("A") + i), round2.read(p)) for i, p in enumerate(paths)]
    tels = [(run, t) for run, r in reps for t in r["telemetry"]]
    q = [(run, t) for run, t in tels if t["brain"] in QUESTING]

    print("## Reward gaps, questing bots\n")
    longest = [max(l for _, l, _ in round2.gaps_of(t)) for _, t in q]
    over = []
    for run, t in q:
        for start, length, last in round2.gaps_of(t):
            if length > 120:
                over.append((length, start, last, run, t))
    print(f"{len(q)} bots: median longest gap {round2.mmss(round2.med(longest))}, "
          f"worst {round2.mmss(max(longest))}, {len(over)} gaps over 2:00\n")
    print("| run | bot | brain | from | gap | where: from, to | doing |")
    print("|---|---|---|---|---|---|---|")
    for length, start, last, run, t in sorted(over, key=lambda x: -x[0]):
        trail = [p for p in t.get("trail", []) if start <= p[0] <= start + length]
        a = round2.nearest_place((trail[0][1], trail[0][2])) if trail else "?"
        b = round2.nearest_place((trail[-1][1], trail[-1][2])) if trail else "?"
        doing = []
        for p in trail:
            d = round2.purpose(p[4])
            if d not in doing:
                doing.append(d)
        end = " (to the end)" if last else ""
        print(f"| {run} | {t['name'][-5:]} | {t['brain']} | {round2.mmss(start)} | {round2.mmss(length)}{end} "
              f"| {a} → {b} | {'; '.join(doing[:3])} |")
    print()

    print("## The five longest runs of travel, questing bots\n")
    runs = []
    for run, t in q:
        for r in t.get("runs", []):
            runs.append((r["secs"], run, t, r))
    print("| | run | bot | at | length | from | to | for |")
    print("|---|---|---|---|---|---|---|---|")
    for i, (secs, run, t, r) in enumerate(sorted(runs, key=lambda x: -x[0])[:5]):
        print(f"| {i + 1} | {run} | {t['name'][-5:]} | {round2.mmss(r['t'])} | {round2.mmss(secs)} "
              f"| {round2.nearest_place(r['from'])} | {round2.nearest_place(r['to'])} "
              f"| {round2.purpose(r['end'] or r['start'])} |")
    print()

    print("## Travel share\n")
    print("| brain | bots | travel |")
    print("|---|---|---|")
    for b in QUESTING:
        g = [t for _, t in tels if t["brain"] == b]
        if g:
            secs = sum(round2.active_secs(t) for t in g)
            share = sum(t["time"].get("travel", 0) for t in g) / secs
            print(f"| {b} | {len(g)} | {100 * share:.0f}% |")
    print()

    print("## Looking for group\n")
    for run, t in tels:
        for n in t.get("notes", []):
            text = str(n)
            if "looking for group" in text:
                print(f"- run {run}, {t['name'][-5:]} ({t['brain']}): {text}")
    for run, t in tels:
        for qq in t["quests"]:
            if qq["key"] == "old_tusk" and qq.get("done") is not None:
                print(f"- run {run}, {t['name'][-5:]} ({t['brain']}, {t['class']}) handed in Old Tusk at {round2.mmss(qq['done'])}")
    deaths = [(run, t, d) for run, t in tels for d in t["deaths"] if d.get("killer") == "Old Tusk"]
    for run, t, d in deaths:
        print(f"- run {run}, {t['name'][-5:]} ({t['brain']}, level {d['level']}) died to Old Tusk at "
              f"{round2.mmss(d['t'])}, doing: {d.get('doing', '')}")
    print()


if __name__ == "__main__":
    main()
