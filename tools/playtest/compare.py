"""Before and after, side by side, per player type or per class, from two
sets of `dyno bots` reports (docs/playtest/first-30-minutes-round2.md).

    python3 tools/playtest/compare.py brain|class BEFORE.json[,BEFORE.json...] AFTER.json[,AFTER.json...]

Medians over the bots of a set; deaths an hour are pooled over the time
played; time shares are of the time played.
"""
import os, statistics, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import beats  # noqa: E402
import round2  # noqa: E402


def load(paths):
    return [t for p in paths.split(",") for t in round2.read(p)["telemetry"]]


def stats(g):
    r = beats.row(g)
    secs = sum(round2.active_secs(t) for t in g)
    deaths = sum(len(t["deaths"]) for t in g)
    kills = sum(t["kills"] for t in g)
    lost = sum(t["kills_no_credit"] - t.get("kills_no_credit_alone", 0) for t in g)
    longest = [max(l for _, l, _ in round2.gaps_of(t)) for t in g]
    share = lambda *ks: f"{100 * sum(t['time'].get(k, 0) for t in g for k in ks) / secs:.0f}%"
    five = sum(1 for t in g if any(l >= 5 for _, l, _ in t["xp_curve"]))
    return [
        round2.mmss(r["level 2"]), round2.mmss(r["level 3"]), round2.mmss(r["level 4"]),
        f"{r['level at end']:g}", f"{five} of {len(g)}", f"{r['quests']:g}",
        f"{statistics.median([t['xp_total'] for t in g]):.0f}",
        round2.mmss(round2.med(longest)), round2.mmss(max(longest)),
        share("travel"), share("fight"), share("wait_spawn"),
        f"{deaths} in {secs / 3600:.2f} h", f"{deaths / (secs / 3600):.1f}",
        f"{100 * lost / max(1, kills + lost):.0f}%",
    ]


COLS = ["level 2", "level 3", "level 4", "level at end", "reached 5", "quests done", "XP earned", "longest gap (median)",
        "longest gap (worst)", "travel", "fighting", "waiting for spawns", "deaths", "deaths an hour", "kills lost to a tag"]


def main():
    key, before, after = sys.argv[1], load(sys.argv[2]), load(sys.argv[3])
    names = [n for n in round2.BRAINS if any(t["brain"] == n for t in before)] if key == "brain" else sorted({t["class"] for t in before})
    print(f"| {key} | | bots | " + " | ".join(COLS) + " |")
    print("|---|---|---|" + "---|" * len(COLS))
    for n in names + ["all"]:
        for label, tels in (("before", before), ("after", after)):
            g = [t for t in tels if n == "all" or t[key] == n]
            if g:
                print(f"| {n if label == 'before' else ''} | {label} | {len(g)} | " + " | ".join(stats(g)) + " |")


if __name__ == "__main__":
    main()
