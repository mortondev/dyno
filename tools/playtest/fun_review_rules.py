"""The fun review's changes to content/rules/hearthblock.json, as one script
(docs/playtest/fun-review-2026-11-23.md says why). It was run once; it is
kept as the record of what was changed and refuses to run twice.

    python3 tools/playtest/fun_review_rules.py
"""
import json, os, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
PATH = os.path.join(ROOT, "content", "rules", "hearthblock.json")
r = json.load(open(PATH))
if "discovery_xp" in r["world"]:
    sys.exit("already applied")

quests = {q["key"]: q for q in r["quests"]}
areas = {a["key"]: a for a in r["areas"]}
npcs = {n["key"]: n for n in r["npcs"]}
items = {i["key"]: i for i in r["items"]}
creatures = {c["key"]: c for c in r["creatures"]}

# ---------------------------------------------------------------- world
# first steps into a named place pay: 30 XP + 20 a level of the place
r["world"]["discovery_xp"] = [30, 20]

# ---------------------------------------------------------------- camps
# The meadow: flocks from 125 m of the hearth (they began at 260 m, a minute's
# run before the first fight), five to a flock, back in a minute.
meadow = areas["hearthmeadow"]
for c in meadow["camps"]:
    c["count"], c["spread"], c["respawn"] = 5, 20, 75
for at, rare in (
    ([108, -72], None),
    ([98, 98], {"name": "Greatfleece", "chance": 0.1, "power": 2.0, "levels": 1,
                "drops": [{"item": "grazer_hide_leggings"}, {"item": "woolen_cloak"}]}),
    ([-102, 88], None),
    ([-98, -92], None),
    ([185, 60], None),
    ([-190, -70], None),
    ([60, -165], None),
    ([-70, 190], None),
):
    c = {"creature": "grazer", "count": 5, "place": "hearthstead", "at": at, "spread": 20, "respawn": 60}
    if rare:
        c["rare"] = rare
    meadow["camps"].append(c)
# young boars by the west trail and the south road: a second creature at level 2
meadow["camps"] += [
    {"creature": "thornboar", "count": 3, "place": "hearthstead", "at": [-150, 40], "spread": 16, "respawn": 90},
    {"creature": "thornboar", "count": 3, "place": "hearthstead", "at": [55, 255], "spread": 16, "respawn": 90},
    {"creature": "thornboar", "count": 3, "place": "hearthstead", "at": [150, -120], "spread": 16, "respawn": 90},
]

# The East Road's circle swallowed the south-east meadow (the first flock said "lv4-5").
areas["east_road"]["radius"] = 190

# camps keep their own levels where the area's band is wide, and rares drop their prize
def camp(area, creature, n=0):
    return [c for c in areas[area]["camps"] if c["creature"] == creature][n]

camp("hollow_mines", "hollow_foreman")["levels"] = [10, 10]
camp("hollow_mines", "hollow_foreman")["respawn"] = 600
camp("hollow_mines", "candlekin", 0)["levels"] = [5, 6]
camp("hollow_mines", "candlekin", 1)["levels"] = [6, 7]
camp("hollow_mines", "gloomwisp")["levels"] = [7, 8]
camp("barrowwood", "barrow_warden")["levels"] = [6, 6]
for c in areas["east_road"]["camps"]:
    if c.get("place") == "ashcloak_stockade":
        c["levels"] = [6, 6] if c["creature"] == "rook_varn" else [5, 6]
for c in areas["birchrise"]["camps"]:
    c["count"] = 4
    if c.get("rare"):
        c["rare"]["drops"] = [{"item": "old_tusk_mantle"}]
for c in areas["hollow_oak"]["camps"]:
    if c.get("rare"):
        c["rare"]["drops"] = [{"item": "silkmother_gloves"}]
camp("sunken_shrine", "gloomwisp")["rare"]["drops"] = [{"item": "drowned_lamp_staff"}]
camp("mirrormere", "mirepincher", 1)["rare"] = {
    "name": "Old Snapper", "chance": 0.12, "power": 2.2, "levels": 1,
    "drops": [{"item": "mireshell_buckler"}, {"item": "pincher_shell", "min": 2, "max": 3}]}
camp("east_road", "ashcloak", 0)["rare"] = {
    "name": "Quick Fenna", "chance": 0.12, "power": 2.2, "levels": 1,
    "drops": [{"item": "ashcloak_hatchet"}, {"item": "coin_purse", "min": 2, "max": 3}]}

# things on the way between the hubs: small packs within sight of the roads
areas["birchrise"]["camps"].append(
    {"creature": "thornboar", "count": 3, "place": "north_watch", "at": [-70, -40], "spread": 16, "respawn": 120})
areas["mirrormere"]["camps"] += [
    {"creature": "mirepincher", "count": 3, "place": "windmill_farm", "at": [-70, 60], "spread": 14, "respawn": 120},
    {"creature": "grazer", "count": 4, "place": "windmill_farm", "at": [-60, -70], "spread": 18, "respawn": 90},
]

# ---------------------------------------------------------------- areas to find
def area(key, name, place, radius, levels, **more):
    a = {"key": key, "name": name, "place": place, "radius": radius, "levels": levels}
    a.update(more)
    r["areas"].append(a)

area("old_quarry", "The Old Quarry", "old_quarry", 30, [1, 2])
area("beacon_rocks", "Beacon Rocks", "beacon_rocks", 22, [2, 3])
area("broken_tower", "The Broken Tower", "south_road_tower_7", 26, [1, 2])
area("trappers_camp", "Trapper's Camp", "west_trail_camp_11", 26, [1, 2])
area("westfold_farm", "Westfold Farm", "meadow_farm_1", 32, [1, 2])
area("eastfold_farm", "Eastfold Farm", "meadow_farm_2", 32, [1, 2])
area("southfold_farm", "Southfold Farm", "meadow_farm_3", 32, [1, 2])
area("wrecked_caravan", "The Wrecked Caravan", "wrecked_caravan", 28, [2, 3])
area("watch_hill", "Watch Hill Lookout", "north_road_view_2", 26, [2, 3])
area("woodcutters_well", "Woodcutters' Well", "logging_trail_well_14", 24, [2, 3])
area("hunters_rest", "Hunter's Rest", "oak_trail_camp_16", 26, [2, 3])
area("north_pass", "The North Pass", "north_pass", 40, [4, 5])
area("barrow_overlook", "Barrow Overlook", "oak_trail_view_19", 26, [4, 5])
area("forgotten_graves", "The Forgotten Graves", "barrowwood_graves", 30, [4, 5])
area("old_barrow", "The Old Barrow", "old_barrow", 42, [5, 6])
area("bluebell_glade", "Bluebell Glade", "west_trail_glade_13", 26, [4, 5])
area("listening_stones", "The Listening Stones", "south_road_stones_9", 26, [3, 4])
area("mere_wayshrine", "Wayshrine of the Mere", "south_road_wayshrine_10", 26, [3, 4])
area("emberford_bridge", "Emberford Bridge", "emberford_bridge", 30, [4, 5])
area("emberrun_ford", "Emberrun Ford", "emberrun_ford", 28, [4, 5])
area("ashcloak_stockade", "The Ashcloak Stockade", "ashcloak_stockade", 46, [5, 6])
area("ridge_cut", "The Ridge Cut", "ridge_cut", 24, [5, 6])

# ---------------------------------------------------------------- rides
RIDES = {  # hub area: (name, place, where you get off, the carter)
    "hearthstead": ("Hearthstead", "hearthstead", [40, 4]),
    "logging_camp": ("Garrick's Logging Camp", "logging_camp", [8, 13]),
    "windmill": ("Ostrin's Farm", "windmill_farm", [-10, 14]),
    "fishers_landing": ("Fisher's Landing", "fishers_landing", [2, -4]),
    "miners_rest": ("Miner's Rest", "miners_rest", [8, -4]),
}
for akey, (name, place, at) in RIDES.items():
    r["items"].append({
        "key": f"ride_{akey}", "name": f"Cart ride to {name}", "color": [0.62, 0.45, 0.25],
        "icon": "lump", "image": "icons/grain_sack.png", "stack": 1, "quality": 1,
        "ride": {"place": place, "at": at, "area": akey}})
PRICE = {"hearthstead": 15, "logging_camp": 30, "windmill": 30, "fishers_landing": 40, "miners_rest": 40}

def carter(key, name, place, at, greeting, home):
    r["npcs"].append({
        "key": key, "name": name, "title": "Carter", "color": [0.58, 0.44, 0.26],
        "place": place, "at": at, "greeting": greeting,
        "sells": [f"ride_{k}:{p}" for k, p in PRICE.items() if k != home],
        "routine": [{"at": [0, 0], "secs": [20, 40], "emotes": ["Wave", "Point"]}]})

carter("jeb", "Carter Jeb", "hearthstead", [40, 0, 7],
       "Legs tired? I drive to anywhere you've already walked to. I don't go looking for new places.", "hearthstead")
carter("nan", "Carter Nan", "logging_camp", [8, 0, 15],
       "Timber goes out, people come back. Been somewhere on foot? Then I can take you.", "logging_camp")
carter("pip", "Carter Pip", "windmill_farm", [-12, 0, 16],
       "Flour to town twice a day. There's room on the sacks, if you know where you're going.", "windmill")
carter("corwin", "Ferryman Corwin", "fishers_landing", [3, 0, -6],
       "Boat's laid up, but the cart still runs. Anywhere you've been, for a few coppers.", "fishers_landing")
carter("ada", "Carter Ada", "miners_rest", [8, 0, -6],
       "Ore went west, once. Now it's just people who've had enough. Where to?", "miners_rest")

# ---------------------------------------------------------------- quests
def explore(name, place, at=(0, 0, 0), radius=12):
    return {"explore": name, "place": place, "at": list(at), "radius": radius}

# Hearthstead: fewer kills, the coal is somewhere a level 3 can go
quests["thin_the_herd"]["objectives"][0]["count"] = 8
quests["thin_the_herd"]["text"] = (
    "The Grazers have grown bold without the Watch to drive them off, and they trample our "
    "gardens. Thin the herd: 8 of them should do. The nearest flocks graze just outside the gates.")
quests["kindling"]["objectives"][0]["count"] = 8
quests["kindling"]["text"] = quests["kindling"]["text"].replace("10 logs", "8 logs")
quests["coal_for_the_watch"]["text"] = (
    "Light keeps the Shades back, and torches need coal. The Old Quarry lies north-west of the "
    "town, under a minute on foot: look for black-flecked stone in the rock faces there and bring "
    "me 6 coal. Your wooden pick will do for coal.")
quests["iron_for_hollis"]["text"] = (
    "The Old Quarry has iron too: rust-red flecks beside the coal. You'll want a stone pickaxe "
    "(Old Brannoc gives one for kindling, or I sell them) and a little practice: dig coal until "
    "your Mining reaches 5. Bring me 6 raw iron and I'll show you what a forge can do.")
quests["iron_for_hollis"]["xp"] = 320

# The North Watch: found, and handed in where you found it (it was three one-minute runs)
nw = quests["north_watch"]
nw["ender"] = "holt"
nw["text"] = ("Before the dark came, we kept a beacon north of here. Follow the north road to it, "
              "and if Sergeant Holt still holds the watch there, give him my orders and do what he asks.")
nw["progress"] = "The Captain sent you? Find the beacon first: it is right behind me."
nw["done"] = "Still burning, and so are we. Maren's orders, is it? Then there's work for you here."
nw["xp"] = 180
r["quests"] = [q for q in r["quests"] if q["key"] != "report_to_holt"]
for q in r["quests"]:
    q["requires"] = ["north_watch" if k == "report_to_holt" else k for k in q.get("requires", [])]
eyes = quests["eyes_in_the_dark"]
eyes["giver"] = "holt"
eyes["text"] = ("Shades rise when the sun goes down, and in deep caves at any hour. They cannot cross "
                "into a hearth's light: fight from the fire here if you can. Destroy 5 of them.")
quests["watchfire"]["text"] = (
    "The watchfire eats coal. There is a seam in the Beacon Rocks, a stone's throw east of the "
    "tower: bring me 3 coal and it burns through the night.")

# breadcrumbs that are offered when their hub has work for you
quests["windmill_breadcrumb"]["level"] = 4
quests["word_from_east"]["level"] = 5
quests["word_from_east"]["xp"] = 120
quests["windmill_breadcrumb"]["xp"] = 120
quests["garricks_camp"]["xp"] = 100
quests["down_to_water"]["xp"] = 100
for k in ("windmill_breadcrumb", "word_from_east", "garricks_camp", "down_to_water"):
    quests[k]["money"] = 50

new = [
    {"key": "broken_tower", "title": "The Broken Tower", "level": 2, "giver": "tolan",
     "requires": ["cold_hearth"],
     "text": "There's an old watchtower on the south road, three minutes out. Something has been "
             "moving in it after dark. Walk down, look it over and tell me what you see.",
     "progress": "South gate, then follow the road. You can't miss it.",
     "done": "Empty? Good. Empty is the best news I've had this week.",
     "objectives": [explore("the Broken Tower", "south_road_tower_7", (7, 0, 3), 14)],
     "xp": 160, "money": 40},
    {"key": "lay_of_the_land", "title": "The Lay of the Land", "level": 2, "giver": "tolan",
     "requires": ["broken_tower"],
     "text": "If you're to be any use to the Watch, learn the meadow. Three places: the Old Quarry "
             "to the north-west, the Trapper's Camp on the west trail, and the wreck of Hobb's wagon "
             "on the east road. Go and stand in each.",
     "progress": "Quarry, camp, caravan.",
     "done": "Now you know the ground better than half my men. Take these: you've walked enough to need them.",
     "objectives": [explore("the Old Quarry", "old_quarry", (-8, 0, -10), 16),
                    explore("the Trapper's Camp", "west_trail_camp_11", (3, 0, 8), 14),
                    explore("the Wrecked Caravan", "wrecked_caravan", (0, 0, 9), 14)],
     "xp": 420, "money": 90, "choice": [["thornhide_boots", 1], ["mudcaked_boots", 1], ["woolen_cloak", 1]]},
    {"key": "lookouts_eye", "title": "A Lookout's Eye", "level": 3, "giver": "holt",
     "requires": ["north_watch"],
     "text": "From the hill up the road you can see the whole of Birchrise. Climb to the lookout "
             "and count the boar packs, so I know what we're up against.",
     "progress": "Up the north road. The viewpoint is on the left.",
     "done": "That many? Then the boars are your next job.",
     "objectives": [explore("Watch Hill Lookout", "north_road_view_2", (8, 0, 2), 14)],
     "xp": 180, "money": 50},
    {"key": "word_for_rysa", "title": "Word for the Hunter", "level": 4, "giver": "garrick",
     "ender": "rysa", "requires": ["garricks_camp"],
     "text": "Rysa hunts out by the Hollow Oak, west along the trail. Tell her the camp will pay for "
             "every Websnare she clears. And mind the webs.",
     "done": "Garrick pays now? Then stay: there's more here than I can shoot.",
     "objectives": [{"talk": "rysa"}], "xp": 120, "money": 50},
    {"key": "webs_between_roots", "title": "Webs Between the Roots", "level": 4, "giver": "rysa",
     "requires": ["word_for_rysa"],
     "text": "They nest under the Hollow Oak and come out along the roots. Kill 8 Websnares. If a "
             "big pale one comes down the trunk, that's the Silkmother: don't fight her alone.",
     "progress": "Eight. I'm counting.",
     "done": "The wood's quieter already. Take your pick.",
     "objectives": [{"kill": "websnare", "count": 8}],
     "xp": 320, "money": 120, "choice": [["webspun_cloak", 1], ["shadowweave_gloves", 1], ["smith_gloves", 1]]},
    {"key": "forgotten_graves", "title": "The Forgotten Graves", "level": 5, "giver": "anwen",
     "ender": "calder", "requires": [],
     "text": "The graves north of here are older than the Barrow. Calder lights a lamp there every "
             "night. Find the graves, then find him, and ask what he has seen.",
     "progress": "Have you stood among the graves?",
     "done": "Seen? The Warden walks, that's what I've seen. Somebody ought to put him back.",
     "objectives": [explore("the Forgotten Graves", "barrowwood_graves", (0, 0, 8), 14), {"talk": "calder"}],
     "xp": 300, "money": 100},
    {"key": "know_the_vale", "title": "Know the Vale", "level": 5, "giver": "brannoc",
     "requires": ["coal_for_the_watch"],
     "text": "The Great Hearth once lit the whole vale, pass to shore. Before we relight it, someone "
             "must see what it will shine on. Stand at the North Pass, at the Barrow Overlook in the "
             "west, on the shore before the Sunken Shrine in the south, and at the Ridge Cut in the "
             "east. It is a long road: take it as it comes, and take a friend past the bridge.",
     "progress": "North, west, south, east. The vale is bigger than it looks from here.",
     "done": "Pass to shore, and back alive. Then wear something worthy of it.",
     "objectives": [explore("the North Pass", "north_pass", (8, 0, 6), 16),
                    explore("the Barrow Overlook", "oak_trail_view_19", (7, 0, -4), 14),
                    explore("the Shrine Shore", "shrine_shore", (0, 0, 0), 14),
                    explore("the Ridge Cut", "ridge_cut", (-6, 0, 6), 14)],
     "xp": 1500, "money": 400,
     "choice": [["ironbound_legplates", 1], ["wildstalker_leggings", 1],
                ["nightstalker_breeches", 1], ["apprentice_trousers", 1]]},
]
r["quests"] += new

# rewards: the level 8 sets come from the east road, the shore and the mines
def choice(q, *keys):
    quests[q]["choice"] = [[k, 1] for k in keys]

choice("barrow_warden", "ironbound_pauldrons", "wildstalker_mantle", "nightstalker_spaulders", "apprentice_mantle")
choice("candles_in_dark", "ironbound_gauntlets", "wildstalker_gauntlets", "nightstalker_gloves", "apprentice_wraps")
choice("wax_and_ore", "ironbound_sabatons", "wildstalker_boots", "nightstalker_boots", "apprentice_slippers")
choice("wisps_on_water", "ironbound_helm", "wildstalker_hood", "nightstalker_cowl", "apprentice_circlet")
choice("hollow_foreman", "ironbound_cuirass", "wildstalker_hauberk", "nightstalker_jerkin", "apprentice_robe")
quests["rook_varn"]["choice"] += [["wildstalker_quiver", 1]]
quests["hollow_foreman"]["title"] = "The Hollow Foreman (Group)"
quests["rook_varn"]["text"] += " He keeps two Cutthroats at his side: bring a friend, or pull them one at a time."
quests["barrow_warden"]["title"] = "The Barrow Warden (Group)"

# the belts drop in the mines and at the shrine; Bree sells belts and the plain pieces dear
for c, ch in (("candlekin", 0.012), ("gloomwisp", 0.02), ("ashcloak_leader", 0.02)):
    for k in ("ironbound_girdle", "wildstalker_belt", "nightstalker_belt", "apprentice_sash"):
        creatures[c]["drops"].append({"item": k, "chance": ch})
npcs["bree"]["sells"] += ["ironbound_girdle", "wildstalker_belt", "nightstalker_belt", "apprentice_sash"]
creatures["rook_varn"]["drops"].append({"item": "ashcloak_hatchet", "chance": 0.5})

json.dump(r, open(PATH, "w"), indent=1)
open(PATH, "a").write("\n")
print("applied:", len(r["quests"]), "quests,", len(r["areas"]), "areas,", len(r["npcs"]), "NPCs")
