"""Round 3 of the first 30 minutes (docs/playtest/first-30-minutes-round3.md).

    python3 tools/playtest/round3_rules.py group [RULES.json]     the group tag only (the baseline's rules)
    python3 tools/playtest/round3_rules.py content [RULES.json]   what the baseline asked for

Each stage applies once; the default file is content/rules/hearthblock.json.
"""
import json, os, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
stage = sys.argv[1] if len(sys.argv) > 1 else ""
PATH = sys.argv[2] if len(sys.argv) > 2 else os.path.join(ROOT, "content", "rules", "hearthblock.json")
r = json.load(open(PATH))
quests = {q["key"]: q for q in r["quests"]}
areas = {a["key"]: a for a in r["areas"]}
creatures = {c["key"]: c for c in r["creatures"]}
npcs = {n["key"]: n for n in r["npcs"]}


def group():
    """A group quest says so in data: `group` is how many it is made for.
    The quest log shows "Looking for group", the server tells those who
    carry it about each other, and the bots leave its creature alone unless
    grouped. Old Tusk falls to a pair (round 2: 17 s, nobody died), so he
    is made for two; his text asked for three."""
    if quests["old_tusk"].get("group"):
        sys.exit("group: already applied")
    quests["old_tusk"]["group"] = 2
    quests["old_tusk"]["text"] = (
        "The boars follow one old brute with tusks like scythes. He keeps to a hollow "
        "in the trees west of here, past the pack. He has put two of my men in their "
        "beds: do not go alone. Find someone to go with you, and bring him down."
    )
    quests["old_tusk"]["progress"] = "Old Tusk. West of the tower, past the boar pack. Not alone."
    quests["barrow_warden"]["group"] = 3


def explore(name, place, radius=16):
    return {"explore": name, "place": place, "at": [0, 0, 0], "radius": radius}


def content():
    """A talk quest's walk carries something to do on the way (round 2: the
    travel that is left is the walks to Garrick, 285 m, and to Ostrin,
    490 m, each for a quest that only says hello)."""
    if quests["garricks_camp"]["objectives"][0].get("explore"):
        sys.exit("content: already applied")
    # Holt to Garrick, along the logging trail: the well on the way is a
    # step of the quest, and Garrick's tusks come from Holt, so the boars by
    # the trail (and Holt's own) are on the way, not a trip back from the camp
    g = quests["garricks_camp"]
    g["objectives"] = [explore("Woodcutters' Well", "logging_trail_well_14"), {"talk": "garrick"}]
    g["text"] = ("West of here Foreman Garrick runs the logging camp. He sent for help two days "
                 "ago. Take the logging trail, past the Woodcutters' Well. Go.")
    t = quests["tusks_and_tempers"]
    t["giver"], t["ender"], t["requires"] = "holt", "garrick", ["north_watch"]
    t["text"] = ("Garrick's lads want thornboar tusks: proof the beasts can bleed. Take him five "
                 "when you go. The boars by the logging trail will give them up, and so will mine.")
    t["progress"] = "Five tusks. The lads are waiting on them."
    # Tilda to Ostrin, down the south road: the Listening Stones on the way
    b = quests["windmill_breadcrumb"]
    b["objectives"] = [explore("the Listening Stones", "south_road_stones_9"), {"talk": "ostrin"}]
    b["text"] = ("Farmer Ostrin owes me flour. Walk south to his farm and see what's keeping him. "
                 "Stop at the Listening Stones on the way; folk say they hum.")
    # Ostrin to Wenna, along the shore: the wayshrine on the way, and her
    # claw meat from the pinchers on that shore, not a trip back out
    d = quests["down_to_water"]
    d["objectives"] = [explore("the Wayshrine of the Mere", "south_road_wayshrine_10"), {"talk": "wenna"}]
    d["text"] = ("Old Wenna at Fisher's Landing could use a strong arm more than I can. Follow the "
                 "shore west, past the old wayshrine.")
    s_ = quests["pincher_supper"]
    s_["giver"], s_["ender"], s_["requires"] = "ostrin", "wenna", ["windmill_breadcrumb"]
    s_["text"] = ("Wenna's had nothing but pincher for supper since the fish went. Take her six "
                  "portions of claw meat: the mirepinchers on the shore path will give it up.")
    # Garrick to Rysa, along the oak trail: Hunter's Rest on the way
    w = quests["word_for_rysa"]
    w["objectives"] = [explore("Hunter's Rest", "oak_trail_camp_16"), {"talk": "rysa"}]


{"group": group, "content": content}.get(stage, lambda: sys.exit(__doc__))()
json.dump(r, open(PATH, "w"), indent=1)
open(PATH, "a").write("\n")
print("applied", stage, "to", PATH)
