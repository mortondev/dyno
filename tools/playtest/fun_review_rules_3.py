"""Third batch of the fun review's rules changes, after the second swarm
(first kill 55 s and travel 55%, but 1.6x to 2.2x the design pace, 4.6
deaths an hour, a hunter out of mana after four fights). Run once after
fun_review_rules_2.py; refuses to run twice.

    python3 tools/playtest/fun_review_rules_3.py
"""
import json, os, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
PATH = os.path.join(ROOT, "content", "rules", "hearthblock.json")
r = json.load(open(PATH))
if "kill_xp" in r["world"]:
    sys.exit("already applied")

quests = {q["key"]: q for q in r["quests"]}
items = {i["key"]: i for i in r["items"]}
creatures = {c["key"]: c for c in r["creatures"]}
abilities = {a["key"]: a for a in r["abilities"]}

# pace: with creatures at the gate a kill costs 10 s, not a minute's walk.
# Kills pay 70% of the design formula; the opening quests pay for their length.
r["world"]["kill_xp"] = 0.7
r["world"]["discovery_xp"] = [15, 10]
for k, xp in (("cold_hearth", 30), ("meat_for_the_pot", 110), ("thin_the_herd", 130),
              ("kindling", 100), ("wool_for_winter", 80), ("broken_tower", 90),
              ("lay_of_the_land", 260), ("north_watch", 120), ("lookouts_eye", 100),
              ("coal_for_the_watch", 170), ("watchfire", 100)):
    quests[k]["xp"] = xp

# danger: grazers as they were (longer fights killed 7 bots that never rest),
# Shades a little weaker again (8 of 18 deaths in the last 2.5 minutes)
creatures["grazer"]["health"] = 32
creatures["shade"]["damage"] = 3
creatures["shade"]["health"] = 40

# a Grazer took a third of a caster's health a fight (11 to 19 of the 16 to 25
# deaths in the third swarm were to Grazers, at the flocks at the gate)
creatures["grazer"]["damage"] = 2.5
# boar packs as the last round tuned them: three, and two by the roads
for a in r["areas"]:
    for c in a.get("camps", []):
        if c["creature"] == "thornboar" and a["key"] == "birchrise" and c.get("place") != "north_watch":
            c["count"] = 3
        if c["creature"] == "thornboar" and a["key"] == "hearthmeadow":
            c["count"] = 2

# the hunter's 52 mana lasted four fights
abilities["quick_shot"]["cost"] = 7
abilities["barbed_arrow"]["cost"] = 9

# "gather log" matched the ride to the Logging Camp
items["ride_logging_camp"]["name"] = "Cart ride to Garrick's Camp"

# coal at the west gate for the first coal quest; the quarry has the iron
r["areas"].append({"key": "gate_rocks", "name": "West Gate Rocks", "place": "gate_rocks",
                   "radius": 20, "levels": [1, 2]})
quests["coal_for_the_watch"]["text"] = (
    "Light keeps the Shades back, and torches need coal. There is a seam in the rocks outside "
    "the west gate, and more at the Old Quarry north-west of the town: look for black-flecked "
    "stone and bring me 6 coal. Your wooden pick will do for coal.")

# rares roll at every spawn point on every respawn, so a farmed flock shows
# its rare every couple of minutes: rarer, and the prize is not certain
for a in r["areas"]:
    for c in a.get("camps", []):
        if c.get("rare"):
            for d in c["rare"].get("drops", []):
                d["chance"] = 0.5 if d is c["rare"]["drops"][0] else 0.35
            if c["rare"]["name"] == "Greatfleece":
                c["rare"]["chance"] = 0.03

# a second player at a seam found it mined out for four minutes
r["blocks"]["coal_ore"]["respawn"] = 120

# camps as big as the quests that send you to them: Ostrin's two quests ask
# for 5 bandits and 6 sacks (55% a bandit) from a camp of three
for a in r["areas"]:
    for c in a.get("camps", []):
        if c.get("place") == "mere_ashcloaks":
            c["count"], c["respawn"] = 5, 90
        if c.get("place") in ("road_scouts_1", "road_scouts_2"):
            c["count"] = 3
quests["stolen_grain"]["objectives"][0]["count"] = 4
quests["stolen_grain"]["text"] = quests["stolen_grain"]["text"].replace("6 ", "4 ").replace("six", "four").replace("Six", "Four")

json.dump(r, open(PATH, "w"), indent=1)
open(PATH, "a").write("\n")
print("applied")
