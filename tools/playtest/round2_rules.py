"""Round 2 of the first 30 minutes (docs/playtest/first-30-minutes-round2.md):
what the baseline's deaths, runs and gaps asked for. Run once on main's rules.

    python3 tools/playtest/round2_rules.py
"""
import json, os, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
PATH = os.path.join(ROOT, "content", "rules", "hearthblock.json")
r = json.load(open(PATH))
if "shared_tags" in r["world"]:
    sys.exit("already applied")

quests = {q["key"]: q for q in r["quests"]}
areas = {a["key"]: a for a in r["areas"]}
creatures = {c["key"]: c for c in r["creatures"]}
npcs = {n["key"]: n for n in r["npcs"]}
w = r["world"]

# --- 3. arrival: a new character wakes 7 m south of the hearth (world round
# 3) and a client starts facing north, at the hearth. Brannoc stood beside
# and a little behind; he now stands between you and the hearth, to the left,
# with his "!" in the picture
npcs["brannoc"]["at"] = [-3, 0, 5]

# --- 12. tags: anyone outside the tagger's party who took a tenth of a
# creature's health off it shares the kill: quest credit, their own quest
# drops, and 70% of the kill's XP (a party of two gets 85% each, so grouping
# still pays more; at 100% a crowd of twelve reached level 2 in under two
# minutes). The body's other loot stays the tagger's.
w["shared_tags"] = {"damage": 0.1, "xp": 0.7}
# where players crowd a camp it comes back sooner (to 40% of its time with
# five or more there), and a creature may come back 5 m from a player, not
# 12: one player waiting in Old Tusk's hollow kept him from coming back at all
w["crowd_respawn"] = {"radius": 45, "per_player": 0.8, "least": 0.4, "near": 5}

# --- the night: Shades rise in the dark's haunts, not round every player on
# every road, and leave characters under level 5 alone until struck
w["dark_in_areas"] = True
creatures["shade"]["spares_below"] = 5
r["areas"].append({"key": "dimwood", "name": "The Dimwood", "place": "north_watch", "at": [100, -65],
                   "radius": 45, "levels": [2, 3], "dark": True})
areas["forgotten_graves"]["dark"] = True
areas["old_barrow"]["dark"] = True
q = quests["eyes_in_the_dark"]
q["text"] = ("East of this tower the wood goes dim, and after dusk the Shades stand among the trees. "
             "They pass over the green and the weak until struck: strike first, one at a time. Destroy 5 of them.")
q["progress"] = "The Dimwood, east of here, after dusk. One at a time."
quests["dust_of_shadows"]["text"] = (
    "When a Shade comes apart it leaves dust behind. In the old days we fed that dust to the Great Hearth and it "
    "burned brighter for it. The Dimwood by the North Watch is thick with them after dusk. Bring me 8.")

# --- Holt's boars stood on the logging trail, and Old Tusk beside it: both
# killed travellers to Garrick's camp. The pack moves south-west of the
# tower, a second stands east of it, and Old Tusk's hollow is 80 m off the trail.
moved = 0
for c in areas["birchrise"]["camps"]:
    if c["creature"] == "thornboar" and c.get("place") == "north_watch":
        c.update({"at": [-60, 30], "count": 5, "respawn": 60})
        moved += 1
    if c["creature"] == "old_tusk":
        c.update({"at": [-150, -15], "respawn": 100})
        moved += 1
assert moved == 2, "Holt's boar pack and Old Tusk's camp are where round 1 put them"
areas["birchrise"]["camps"].append(
    {"creature": "thornboar", "count": 4, "place": "north_watch", "at": [70, 45], "spread": 16, "respawn": 60})
quests["boar_trouble"]["text"] = ("Thornboars charge anything on the north road. There is a pack either side of "
                                  "this tower: five fewer would make it walkable again.")
quests["old_tusk"]["text"] = ("The boars follow one old brute with tusks like scythes. He keeps to a hollow in the "
                              "trees west of here, past the pack. He has put two of my men in their beds: do not "
                              "go alone. Take two with you, and bring him down.")
quests["old_tusk"]["progress"] = "Old Tusk. West of the tower, past the boar pack. Three of you, not one."
quests["old_tusk"]["done"] = "That is the herd broken. The Watch owes you, all of you."

# --- the Barrow Warden (level 6, under the barrow) saw through its roof: he
# killed three level 4 visitors to Anwen, who stands 9 m from him above ground
creatures["barrow_warden"]["aggro"] = 3

# --- Old Tusk fell to one geared level 4 warrior in 15 s and to a priest who
# healed through him (276 health, 9 damage). With more of both he outlasts any
# one level 4 and still falls to three in seconds.
creatures["old_tusk"].update({"health": 240, "damage": 12})

# --- travel: Anwen stands at the Old Barrow, 650 m out, and offered her
# quest to anyone of level 4. Garrick now sends you there first.
quests["forgotten_graves"]["requires"] = ["into_barrowwood"]
# The Lay of the Land was a kilometre round a town with no west gate: the
# wreck on the east road (the far side of the wall) is left to the east road
q = quests["lay_of_the_land"]
q["objectives"] = [o for o in q["objectives"] if o.get("place") != "wrecked_caravan"]
q["text"] = ("If you're to be any use to the Watch, learn the meadow. Go and stand in two places: the Old Quarry "
             "north-west of this gate, and the Trapper's Camp on the west trail beyond it.")
q["progress"] = "Quarry, then the camp."
q["xp"] = 200
# Thieves in the Fields: the only Ashcloaks were 160 m from Ostrin, five in
# a knot (two came at once). A camp stands in his east field, spread out.
spread = 0
for c in areas["mirrormere"]["camps"]:
    if c["creature"] == "ashcloak":
        c["spread"] = 24
        spread += 1
assert spread == 1, "one Ashcloak camp by the mere"
areas["mirrormere"]["camps"].append(
    {"creature": "ashcloak", "count": 5, "place": "windmill_farm", "at": [80, -55], "spread": 24, "respawn": 75})
quests["thieves_in_fields"]["text"] = ("Ashcloak bandits raid my east field at all hours. Drive five of them off for good.")
quests["stolen_grain"]["text"] = ("They carry off my grain in sacks, out of the east field. Bring back four and Tilda gets her flour.")

# --- Thieves in the Fields counted any Ashcloak anywhere: bots (and the
# tracker) went 450 m to the level 5 scouts on the East Road and died there.
# Ostrin's thieves are their own creature, in his east field and nowhere else.
thief = dict(creatures["ashcloak"])
thief.update({"key": "ashcloak_thief", "name": "Ashcloak Thief", "levels": [3, 4]})
r["creatures"].append(thief)
field = [c for c in areas["mirrormere"]["camps"] if c.get("place") == "windmill_farm" and c["creature"] == "ashcloak"]
assert len(field) == 1, "the east field camp"
field[0]["creature"] = "ashcloak_thief"
quests["thieves_in_fields"]["objectives"] = [{"kill": "ashcloak_thief", "count": 5}]
quests["thieves_in_fields"]["text"] = "Ashcloak thieves raid my east field at all hours. Drive five of them off for good."

json.dump(r, open(PATH, "w"), indent=1)
open(PATH, "a").write("\n")
print("applied")
