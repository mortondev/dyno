#!/bin/bash
S=${S:-$(pwd)/tools/concepts/work}
gen() {
  name=$1; desc=$2
  [ -f $S/out/$name.png ] && return
  timeout 900 codex exec --skip-git-repo-check --sandbox workspace-write -C $S/out "$(cat $S/style.txt) Location: $desc Generate this improved higher-fidelity version of the attached screenshot with your image generation tool, and save it in the current directory as $name.png." -i $S/ref/$name.jpg < /dev/null > $S/logs/$name.log 2>&1
  echo "$name $( [ -f $S/out/$name.png ] && echo ok || echo FAILED )"
}
export -f gen; export S
mkdir -p $S/logs
cat <<'LIST' | xargs -P 4 -L 1 bash -c 'gen "$0" "$1"'
hearthstead_plaza "Hearthstead, the starting town: a cobbled market plaza around a great stone hearth, half-timbered houses with red, blue and golden roofs, a stone watchtower, market stalls with striped awnings."
hearthstead_street "A main street of Hearthstead: timber-framed houses with plaster walls, flower boxes, lanterns on posts, a cobbled road leading to the watchtower and market."
hearthstead_dusk "Hearthstead seen from the hills at dusk: the town's rooftops, the watchtower and warm window light, with evening mist."
watch_tower "The Watch tower of Hearthstead: a four-storey stone tower with battlements and banners rising over the market, half-timbered houses around it."
north_watch "The North Watch: a timber watch post and beacon on the road north of Hearthstead, among oak woods."
logging_camp "Garrick's Logging Camp in the Birchrise hills: felled trunks, log piles, a lumber hut and tents among birch and oak woods."
hollow_oak "The Hollow Oak: a vast ancient hollow oak tree beside a woodland pond, with tangled roots, spider webs and dappled shade."
old_barrow "The Old Barrow in Barrowwood: an ancient burial mound with standing stones and a crypt entrance, gravestones among dark gnarled trees, eerie twilight."
barrow_crypt "Barrowwood at night: dark twisted woods, scattered graves, the barrow crypt and a misty lake shore under moonlight with ghostly lights."
windmill_farm "Ostrin's windmill farm: a timber windmill, golden wheat fields, a farmhouse, fences and haystacks in rolling meadows."
mirrormere "Mirrormere lake and the Emberrun river: a calm lake with reeds, a sandy shore, docks and oak woods."
fishers_landing "Fisher's Landing: a small fishing village on the lake shore with wooden docks, boats, nets drying and a cosy hut at golden hour."
sunken_shrine "The Sunken Shrine: half-drowned ancient stone ruins in the shallows of the lake, with glowing crystals and lily pads."
wrecked_caravan "A wrecked merchant caravan by the East Road: broken wagons, scattered crates and barrels, a cobbled road through meadows and oaks."
ashcloak_stockade "The Ashcloak Stockade at night: a bandit camp with a palisade, tents, campfires and torches beside a river."
miners_rest "Miner's Rest: a mining settlement on a ridge with a timber headframe, ore carts, miners' cabins and a road cut through grey cliffs."
hollow_mines "The Hollow Mines: a mine entrance in a grey cliff with a tall wooden headframe, tracks and ore carts, wooded hills above."
LIST
echo all done
