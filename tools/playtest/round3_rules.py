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


def content():
    sys.exit("content: not written yet")


{"group": group, "content": content}.get(stage, lambda: sys.exit(__doc__))()
json.dump(r, open(PATH, "w"), indent=1)
open(PATH, "a").write("\n")
print("applied", stage, "to", PATH)
