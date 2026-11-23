"""Second batch of the fun review's rules changes, after the first swarm on
the changed world (1.63x the design pace, first kill at 92 s, 27% of kills
lost to another tag, a named elite one-shotting level 1s outside the gate).
Run once after fun_review_rules.py; refuses to run twice.

    python3 tools/playtest/fun_review_rules_2.py
"""
import json, os, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
PATH = os.path.join(ROOT, "content", "rules", "hearthblock.json")
r = json.load(open(PATH))
if r["world"]["discovery_xp"] != [30, 20]:
    sys.exit("already applied")

quests = {q["key"]: q for q in r["quests"]}
areas = {a["key"]: a for a in r["areas"]}
creatures = {c["key"]: c for c in r["creatures"]}

# pace: the first quests paid for a 260 m run that is now 100 m
r["world"]["discovery_xp"] = [20, 15]
for k, xp in (("cold_hearth", 40), ("meat_for_the_pot", 150), ("thin_the_herd", 170),
              ("kindling", 140), ("wool_for_winter", 110), ("broken_tower", 110),
              ("lay_of_the_land", 300), ("north_watch", 140), ("lookouts_eye", 120),
              ("coal_for_the_watch", 190), ("watchfire", 110), ("boar_trouble", 220),
              ("tusks_and_tempers", 220)):
    quests[k]["xp"] = xp
creatures["grazer"]["health"] = 36

# the first flocks graze at the town's edge (100 m from the hearth), six to a flock
meadow = areas["hearthmeadow"]["camps"]
ring = {(108, -72): [72, -72], (98, 98): [72, 72], (-102, 88): [-72, 72], (-98, -92): [-72, -72]}
for c in meadow:
    at = tuple(c.get("at", ()))
    if c.get("place") == "hearthstead" and at in ring:
        c["at"], c["count"], c["spread"], c["respawn"] = ring[at], 6, 17, 50
    if c.get("rare"):
        # a named ram a level 2 can take with care, not a wall for level 1s
        c["rare"].update({"power": 1.5, "levels": 0, "chance": 0.08})
meadow += [
    {"creature": "grazer", "count": 5, "place": "hearthstead", "at": [135, 20], "spread": 18, "respawn": 60},
    {"creature": "grazer", "count": 5, "place": "hearthstead", "at": [-130, -25], "spread": 18, "respawn": 60},
    {"creature": "grazer", "count": 5, "place": "hearthstead", "at": [-30, 135], "spread": 18, "respawn": 60},
    {"creature": "grazer", "count": 5, "place": "hearthstead", "at": [30, -130], "spread": 18, "respawn": 60},
]

# the night: Shades hit a little softer (8 of 13 deaths, all in the last 2.5 minutes)
creatures["shade"]["damage"] = 3.5

json.dump(r, open(PATH, "w"), indent=1)
open(PATH, "a").write("\n")
print("applied")
