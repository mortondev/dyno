"""Put the machine's load and the server's tick cost into the report of a
swarm that ran against a separate server (a `--local` swarm records them
itself).

    DYNO_TICK_STATS=1 dyno server ... > server.log      (a line a minute: mean, p95, worst)
    (cat /proc/loadavg; while sleep 60; do cat /proc/loadavg; done) > load.log
    python3 tools/playtest/attach_tick.py REPORT.json server.log load.log

The p95 written is the worst minute's p95, so it is an upper bound on the
run's.
"""
import json, os, re, sys

report, server, load = sys.argv[1:4]
ticks = []
for line in open(server):
    m = re.search(r"tick: mean ([\d.]+) ms, p95 ([\d.]+) ms, worst ([\d.]+) ms over (\d+) ticks, (\d+) players", line)
    if m and int(m.group(5)) > 0:
        ticks.append((float(m.group(1)), float(m.group(2)), float(m.group(3)), int(m.group(4))))
loads = [float(l.split()[0]) for l in open(load) if l.strip()]
r = json.load(open(report))
n = sum(t[3] for t in ticks)
r["machine"] = {
    "load_start": loads[0], "load_mean": sum(loads) / len(loads), "load_max": max(loads),
    "cores": os.cpu_count(),
    "tick": {
        "mean_ms": sum(t[0] * t[3] for t in ticks) / max(1, n),
        "p95_ms": max(t[1] for t in ticks),
        "p99_ms": max(t[1] for t in ticks),
        "worst_ms": max(t[2] for t in ticks),
        "ticks": n,
        "worst_minute_p95_ms": max(t[1] for t in ticks),
        "note": "from the server's log: p95 is the worst minute's (an upper bound); p99 not measured",
    },
}
json.dump(r, open(report, "w"))
print(json.dumps(r["machine"]))
