"""Fourth batch: the first 30 minutes against docs/design/quality-bar.md
(sections 3 to 5 and 12). Run once after fun_review_rules_3.py.

    python3 tools/playtest/fun_review_rules_4.py
"""
import json, os, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
PATH = os.path.join(ROOT, "content", "rules", "hearthblock.json")
r = json.load(open(PATH))
if any(c["key"] == "old_tusk" for c in r["creatures"]):
    sys.exit("already applied")

quests = {q["key"]: q for q in r["quests"]}
areas = {a["key"]: a for a in r["areas"]}
creatures = {c["key"]: c for c in r["creatures"]}

# 12. a reason to group in the first 30 minutes: Old Tusk is always in his
# hollow west of the North Watch, an elite for two, and Holt asks for him
boar = creatures["thornboar"]
tusk = dict(boar)
tusk.update({"key": "old_tusk", "name": "Old Tusk", "health": 150, "damage": 9, "levels": [4, 4],
             "scale": 1.5, "cap": 0, "drops": [{"item": "old_tusk_mantle", "chance": 1.0},
                                     {"item": "boar_tusk", "min": 2, "max": 3, "chance": 1.0},
                                     {"item": "thornhide_boots", "chance": 0.5}]})
r["creatures"].append(tusk)
for c in areas["birchrise"]["camps"]:
    c.pop("rare", None)
areas["birchrise"]["camps"].append(
    {"creature": "old_tusk", "count": 1, "place": "north_watch", "at": [-135, -70], "spread": 4,
     "respawn": 150, "levels": [4, 4]})
r["quests"].append({
    "key": "old_tusk", "title": "Old Tusk (Group)", "level": 4, "giver": "holt", "requires": ["boar_trouble"],
    "text": "The boars follow one old brute with tusks like scythes. He keeps to a hollow west of here. "
            "He has put two of my men in their beds: take a friend, and bring him down.",
    "progress": "Old Tusk. West of the tower, past the first boar pack. Not alone.",
    "done": "That is the herd broken. The Watch owes you, both of you.",
    "objectives": [{"kill": "old_tusk", "count": 1}],
    "xp": 500, "money": 200,
    "choice": [["watchman_vest", 1], ["ranger_jerkin", 1], ["hearthkeeper_robe", 1]]})

# 5. a chest piece by day, by about minute 20: Boar Trouble pays in armour
# (Eyes in the Dark, the only chest reward, can only be done at night)
quests["boar_trouble"]["choice"] = [["watchman_vest", 1], ["ranger_jerkin", 1], ["hearthkeeper_robe", 1]]
quests["eyes_in_the_dark"]["choice"] = [["shadowweave_gloves", 1], ["smith_gloves", 1], ["thornhide_boots", 1]]

# the starter kit: bread enough for the first ten fights (new players do not
# know to stop and eat berries one at a time)
r["world"]["starter"] = [["wooden_pickaxe", 1], ["wooden_axe", 1], ["hearth_bread", 6], ["berries", 5]]

# 4. short quest text with the objective in one line: the long ones
quests["know_the_vale"]["text"] = (
    "The Great Hearth once lit the whole vale. Before we relight it, see what it will shine on. "
    "Stand at the North Pass, the Barrow Overlook, the Shrine Shore and the Ridge Cut. "
    "It is a long road: take a friend past the bridge.")
quests["lay_of_the_land"]["text"] = (
    "If you're to be any use to the Watch, learn the meadow. Go and stand in three places: the Old "
    "Quarry north-west, the Trapper's Camp on the west trail, and the wreck of Hobb's wagon on the east road.")
quests["iron_for_hollis"]["text"] = (
    "The Old Quarry has iron beside the coal. You need a stone pickaxe (Brannoc gives one for kindling) "
    "and Mining 5: dig coal until you have it. Bring me 6 raw iron.")

json.dump(r, open(PATH, "w"), indent=1)
open(PATH, "a").write("\n")
print("applied")
