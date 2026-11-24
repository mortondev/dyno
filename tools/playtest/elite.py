"""Who beat an elite, from `dyno bots` reports of staged runs
(docs/playtest/first-30-minutes-round2.md, Old Tusk).

    python3 tools/playtest/elite.py "Old Tusk" REPORT.json [REPORT.json ...]

One line per bot per fight with the creature: won or lost, how long, the
damage taken, the health at the pull, and whether the bot died to it.
"""
import gzip, json, os, sys


def main():
    name, paths = sys.argv[1], sys.argv[2:]
    print("| run | bot | class | level | brain | fights | won | lost | longest fight | damage taken (most) | deaths to it | quest done |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for p in paths:
        r = json.load(gzip.open(p, "rt") if p.endswith(".gz") else open(p))
        for t in r["telemetry"]:
            fs = [f for f in t["fights"] if f["creature"] == name]
            won = sum(1 for f in fs if f["won"])
            deaths = sum(1 for d in t["deaths"] if name in d["cause"])
            done = any(q["done"] is not None for q in t["quests"] if q["key"] == "old_tusk")
            longest = max((f["secs"] for f in fs), default=0)
            dmg = max((f["damage_taken"] for f in fs), default=0)
            print(f"| {os.path.basename(p)[:-5]} | {t['name'][-5:]} | {t['class']} | {t['level']} | {t['brain']} | {len(fs)} | {won} | {len(fs) - won} | {longest:.0f} s | {dmg:.0f} | {deaths} | {'yes' if done else 'no'} |")
            for f in fs:
                print(f"|  |  |  |  |  | at {f['start']:.0f} s | {'won' if f['won'] else 'lost'} |  | {f['secs']:.0f} s | {f['damage_taken']:.0f} |  | {f.get('why', '')} |")


if __name__ == "__main__":
    main()
