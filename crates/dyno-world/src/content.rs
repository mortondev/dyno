//! Game content as data: items, recipes, block rules, creatures and milestones.
//!
//! Content is written as JSON with readable keys (see `content/hearthblock.json`)
//! and resolved into compact numeric IDs. The server loads it and sends the
//! resolved form to each player when they join, so new items and recipes need
//! no client release.
//!
//! Item IDs below 256 are block items (the item ID equals the block ID).
//! Other items start at 256.

use crate::{BLOCK_COUNT, BlockId, block, block_by_key, block_key};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type ItemId = u16;
pub const FIRST_ITEM: ItemId = 256;

/// The content that ships with Dyno: the Hearthblock pack.
pub const DEFAULT_JSON: &str = include_str!("../../../content/hearthblock.json");

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Pickaxe,
    Axe,
    Shovel,
    Sword,
    Hoe,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Icon {
    Block,
    Stick,
    Lump,
    Ingot,
    Gem,
    Dust,
    Berries,
    Meat,
    Tool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Tool {
    pub kind: ToolKind,
    pub tier: u8,
    pub speed: f32,
    pub damage: f32,
    pub durability: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Item {
    pub id: ItemId,
    pub key: String,
    pub name: String,
    pub color: [f32; 3],
    pub icon: Icon,
    pub stack: u8,
    pub tool: Option<Tool>,
    /// hunger points restored when eaten
    pub food: u8,
    /// furnace fuel units
    pub fuel: u8,
    /// block placed by this item
    pub block: Option<BlockId>,
    /// light level when held (ember tools glow)
    pub light: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Drop {
    pub item: ItemId,
    pub min: u8,
    pub max: u8,
    pub chance: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BlockRule {
    /// seconds-scale toughness; negative means unbreakable
    pub hardness: f32,
    /// the tool that mines it fastest
    pub tool: Option<ToolKind>,
    /// minimum tool tier for drops; 0 means no tool is needed
    pub tier: u8,
    pub drops: Vec<Drop>,
    /// a resource node: in a protected world it can be harvested, and grows
    /// back after this many seconds (0: not a node)
    #[serde(default)]
    pub respawn: u16,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Station {
    None,
    Table,
    Furnace,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Ingredient {
    /// any of these items count
    pub any: Vec<ItemId>,
    pub count: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Recipe {
    pub id: u16,
    pub output: ItemId,
    pub count: u8,
    pub inputs: Vec<Ingredient>,
    pub station: Station,
    /// furnace fuel units used
    pub fuel: u8,
    pub group: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Model {
    Quadruped,
    Humanoid,
    Orb,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Spawn {
    /// grass in daylight
    Meadow,
    /// where block light is low, at night or underground
    Dark,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Creature {
    pub id: u16,
    pub key: String,
    pub name: String,
    pub health: f32,
    pub speed: f32,
    pub hostile: bool,
    pub damage: f32,
    pub drops: Vec<Drop>,
    pub color: [f32; 3],
    pub model: Model,
    pub spawn: Spawn,
    /// most of this creature near one player
    pub cap: u8,
    pub burns_in_daylight: bool,
    /// lowest and highest level; the level rises with distance from the spawn
    pub levels: [u8; 2],
}

/// A friendly character who stands in the world, talks and gives quests.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Npc {
    pub id: u16,
    pub key: String,
    pub name: String,
    /// shown under the name, like "Hearthkeeper"
    pub title: String,
    pub color: [f32; 3],
    /// where they stand, relative to the spawn hearth
    pub at: [i32; 3],
    pub greeting: String,
}

/// One thing a quest asks for.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Objective {
    Kill {
        creature: u16,
        count: u8,
    },
    Collect {
        item: ItemId,
        count: u8,
    },
    Talk {
        npc: u16,
    },
    /// reach a place (relative to the spawn hearth)
    Explore {
        name: String,
        at: [i32; 3],
        radius: f32,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Quest {
    pub id: u16,
    pub key: String,
    pub title: String,
    pub level: u8,
    /// NPC who offers it, and NPC who takes it back
    pub giver: u16,
    pub ender: u16,
    pub text: String,
    /// what the ender says while it is not done
    pub progress: String,
    /// what the ender says on completion
    pub done: String,
    /// quests that must be finished first
    pub requires: Vec<u16>,
    pub objectives: Vec<Objective>,
    pub xp: u32,
    /// in copper: 100 copper is 1 silver, 100 silver is 1 gold
    pub money: u32,
    /// items always given
    pub items: Vec<(ItemId, u8)>,
    /// pick one of these
    pub choice: Vec<(ItemId, u8)>,
}

impl Objective {
    pub fn need(&self) -> u8 {
        match self {
            Objective::Kill { count, .. } | Objective::Collect { count, .. } => *count,
            _ => 1,
        }
    }
}

/// What a class spends to use abilities.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Resource {
    /// fills by fighting, drains out of combat
    Rage,
    /// a pool that refills over time
    Mana,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Class {
    pub id: u16,
    pub key: String,
    pub name: String,
    pub resource: Resource,
    pub color: [f32; 3],
    /// one line for the class picker
    pub blurb: String,
}

/// A class ability. Its effects are the non-zero fields.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Ability {
    pub id: u16,
    pub key: String,
    pub name: String,
    pub class: u16,
    /// learned at this level
    pub level: u8,
    pub text: String,
    pub cost: f32,
    /// seconds
    pub cooldown: f32,
    /// cast time in seconds; 0 is instant
    pub cast: f32,
    /// how far the target may be; 0 means self only
    pub range: f32,
    /// damage to the target: base + per level
    pub damage: [f32; 2],
    /// healing to yourself: base + per level
    pub heal: [f32; 2],
    /// hits every hostile creature within this radius (of you, or of the target)
    pub radius: f32,
    /// holds the target in place for this many seconds
    pub root: f32,
    /// stuns the target for this many seconds
    pub stun: f32,
    /// damage multiplier against creatures that burn in daylight (Shades)
    pub shade_bonus: f32,
    /// jump forward this many blocks
    pub blink: f32,
    /// return to your hearth or bed
    pub recall: bool,
    /// colour of the effect and icon
    pub color: [f32; 3],
    /// item key whose sprite is the icon
    pub icon: String,
}

/// World rules.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct WorldRules {
    /// RPG rules: players in survival may only harvest blocks marked as
    /// resource nodes, cannot place blocks, keep their items on death, and
    /// do not get hungry
    #[serde(default)]
    pub protected: bool,
    /// item keys and counts a new character starts with
    #[serde(default)]
    pub starter: Vec<(String, u8)>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Milestone {
    pub key: String,
    pub title: String,
    pub hint: String,
    /// "collect:<item>", "craft:<item>", "place:<block>", "kill:<creature>", "night"
    pub trigger: String,
}

/// Resolved content, ready to use and to send.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Content {
    pub name: String,
    pub version: u32,
    /// non-block items, `items[i].id == FIRST_ITEM + i`
    pub items: Vec<Item>,
    /// one rule per block ID
    pub blocks: Vec<BlockRule>,
    pub recipes: Vec<Recipe>,
    pub creatures: Vec<Creature>,
    pub milestones: Vec<Milestone>,
    #[serde(default)]
    pub npcs: Vec<Npc>,
    #[serde(default)]
    pub quests: Vec<Quest>,
    #[serde(default)]
    pub world: WorldRules,
    #[serde(default)]
    pub classes: Vec<Class>,
    #[serde(default)]
    pub abilities: Vec<Ability>,
}

// ---------- authored form ----------

#[derive(Deserialize)]
struct AFile {
    name: String,
    version: u32,
    items: Vec<AItem>,
    #[serde(default)]
    blocks: HashMap<String, ABlock>,
    recipes: Vec<ARecipe>,
    #[serde(default)]
    creatures: Vec<ACreature>,
    #[serde(default)]
    milestones: Vec<Milestone>,
    #[serde(default)]
    npcs: Vec<ANpc>,
    #[serde(default)]
    quests: Vec<AQuest>,
    #[serde(default)]
    world: WorldRules,
    #[serde(default)]
    classes: Vec<AClass>,
    #[serde(default)]
    abilities: Vec<AAbility>,
}

#[derive(Deserialize)]
struct AClass {
    key: String,
    name: String,
    resource: Resource,
    color: [f32; 3],
    #[serde(default)]
    blurb: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct AAbility {
    key: String,
    name: String,
    class: String,
    level: u8,
    text: String,
    cost: f32,
    cooldown: f32,
    cast: f32,
    range: f32,
    damage: [f32; 2],
    heal: [f32; 2],
    radius: f32,
    root: f32,
    stun: f32,
    shade_bonus: f32,
    blink: f32,
    recall: bool,
    color: [f32; 3],
    icon: String,
}

#[derive(Deserialize)]
struct ANpc {
    key: String,
    name: String,
    #[serde(default)]
    title: String,
    color: [f32; 3],
    at: [i32; 3],
    #[serde(default)]
    greeting: String,
}

#[derive(Deserialize)]
struct AObjective {
    #[serde(default)]
    kill: Option<String>,
    #[serde(default)]
    collect: Option<String>,
    #[serde(default)]
    talk: Option<String>,
    #[serde(default)]
    explore: Option<String>,
    #[serde(default)]
    at: [i32; 3],
    #[serde(default = "eight")]
    radius: f32,
    #[serde(default = "one")]
    count: u8,
}

fn eight() -> f32 {
    8.0
}

#[derive(Deserialize)]
struct AQuest {
    key: String,
    title: String,
    level: u8,
    giver: String,
    #[serde(default)]
    ender: Option<String>,
    text: String,
    #[serde(default)]
    progress: String,
    #[serde(default)]
    done: String,
    #[serde(default)]
    requires: Vec<String>,
    objectives: Vec<AObjective>,
    /// None: worked out from the level
    #[serde(default)]
    xp: Option<u32>,
    #[serde(default)]
    money: u32,
    #[serde(default)]
    items: Vec<(String, u8)>,
    #[serde(default)]
    choice: Vec<(String, u8)>,
}

#[derive(Deserialize)]
struct AItem {
    key: String,
    name: String,
    color: [f32; 3],
    icon: Icon,
    #[serde(default)]
    stack: Option<u8>,
    #[serde(default)]
    tool: Option<Tool>,
    #[serde(default)]
    food: u8,
    #[serde(default)]
    fuel: u8,
    #[serde(default)]
    block: Option<String>,
    #[serde(default)]
    light: u8,
}

#[derive(Deserialize)]
struct ADrop {
    item: String,
    #[serde(default = "one")]
    min: u8,
    #[serde(default = "one")]
    max: u8,
    #[serde(default = "always")]
    chance: f32,
}

fn one() -> u8 {
    1
}
fn always() -> f32 {
    1.0
}

#[derive(Deserialize)]
struct ABlock {
    hardness: f32,
    #[serde(default)]
    tool: Option<ToolKind>,
    #[serde(default)]
    tier: u8,
    /// None: the block drops itself
    #[serde(default)]
    drops: Option<Vec<ADrop>>,
    #[serde(default)]
    respawn: u16,
}

#[derive(Deserialize)]
struct ARecipe {
    out: String,
    #[serde(default = "one")]
    count: u8,
    /// [["log|birch_log", 1], ...]
    #[serde(rename = "in")]
    inputs: Vec<(String, u8)>,
    #[serde(default = "no_station")]
    station: Station,
    #[serde(default)]
    fuel: u8,
    #[serde(default)]
    group: String,
}

fn no_station() -> Station {
    Station::None
}

#[derive(Deserialize)]
struct ACreature {
    key: String,
    name: String,
    health: f32,
    speed: f32,
    #[serde(default)]
    hostile: bool,
    #[serde(default)]
    damage: f32,
    #[serde(default)]
    drops: Vec<ADrop>,
    color: [f32; 3],
    model: Model,
    spawn: Spawn,
    cap: u8,
    #[serde(default)]
    burns_in_daylight: bool,
    #[serde(default = "level_one")]
    levels: [u8; 2],
}

fn level_one() -> [u8; 2] {
    [1, 1]
}

impl Content {
    /// The built-in Hearthblock content.
    pub fn default_pack() -> Content {
        Content::from_json(DEFAULT_JSON).expect("built-in content is valid")
    }

    /// Parse and resolve authored JSON. Errors name the bad key.
    pub fn from_json(text: &str) -> Result<Content, String> {
        let f: AFile = serde_json::from_str(text).map_err(|e| format!("content JSON: {e}"))?;
        let mut keys: HashMap<String, ItemId> = HashMap::new();
        for b in 0..BLOCK_COUNT {
            keys.insert(block_key(b), b as ItemId);
        }
        for (i, it) in f.items.iter().enumerate() {
            if keys
                .insert(it.key.clone(), FIRST_ITEM + i as ItemId)
                .is_some()
            {
                return Err(format!(
                    "item key '{}' is used twice or clashes with a block",
                    it.key
                ));
            }
        }
        let item = |k: &str| {
            keys.get(k)
                .copied()
                .ok_or_else(|| format!("unknown item '{k}'"))
        };
        let drops = |v: &Vec<ADrop>| -> Result<Vec<Drop>, String> {
            v.iter()
                .map(|d| {
                    Ok(Drop {
                        item: item(&d.item)?,
                        min: d.min,
                        max: d.max.max(d.min),
                        chance: d.chance,
                    })
                })
                .collect()
        };
        let mut items = Vec::new();
        for (i, a) in f.items.iter().enumerate() {
            let block = match &a.block {
                Some(k) => Some(
                    block_by_key(k)
                        .ok_or_else(|| format!("unknown block '{k}' on item '{}'", a.key))?,
                ),
                None => None,
            };
            let stack = a
                .stack
                .unwrap_or(if a.tool.is_some() { 1 } else { 64 })
                .max(1);
            items.push(Item {
                id: FIRST_ITEM + i as ItemId,
                key: a.key.clone(),
                name: a.name.clone(),
                color: a.color,
                icon: a.icon,
                stack,
                tool: a.tool.clone(),
                food: a.food,
                fuel: a.fuel,
                block,
                light: a.light,
            });
        }
        let mut blocks = Vec::new();
        for b in 0..BLOCK_COUNT {
            let key = block_key(b);
            let rule = match f.blocks.get(&key) {
                Some(a) => BlockRule {
                    hardness: a.hardness,
                    tool: a.tool,
                    tier: a.tier,
                    respawn: a.respawn,
                    drops: match &a.drops {
                        Some(d) => drops(d)?,
                        None => vec![Drop {
                            item: b as ItemId,
                            min: 1,
                            max: 1,
                            chance: 1.0,
                        }],
                    },
                },
                None => BlockRule {
                    hardness: if b == crate::AIR || b == crate::WATER {
                        -1.0
                    } else {
                        1.0
                    },
                    tool: None,
                    tier: 0,
                    respawn: 0,
                    drops: vec![Drop {
                        item: b as ItemId,
                        min: 1,
                        max: 1,
                        chance: 1.0,
                    }],
                },
            };
            blocks.push(rule);
        }
        for k in f.blocks.keys() {
            if block_by_key(k).is_none() {
                return Err(format!("unknown block '{k}' in blocks"));
            }
        }
        let mut recipes = Vec::new();
        for (i, r) in f.recipes.iter().enumerate() {
            let mut inputs = Vec::new();
            for (alts, n) in &r.inputs {
                let any = alts
                    .split('|')
                    .map(|k| item(k.trim()))
                    .collect::<Result<Vec<_>, _>>()?;
                inputs.push(Ingredient { any, count: *n });
            }
            recipes.push(Recipe {
                id: i as u16,
                output: item(&r.out)?,
                count: r.count.max(1),
                inputs,
                station: r.station,
                fuel: r.fuel,
                group: if r.group.is_empty() {
                    "other".into()
                } else {
                    r.group.clone()
                },
            });
        }
        let mut creatures = Vec::new();
        for (i, c) in f.creatures.iter().enumerate() {
            creatures.push(Creature {
                id: i as u16,
                key: c.key.clone(),
                name: c.name.clone(),
                health: c.health,
                speed: c.speed,
                hostile: c.hostile,
                damage: c.damage,
                drops: drops(&c.drops)?,
                color: c.color,
                model: c.model,
                spawn: c.spawn,
                cap: c.cap,
                burns_in_daylight: c.burns_in_daylight,
                levels: [c.levels[0].max(1), c.levels[1].max(c.levels[0]).max(1)],
            });
        }
        let npcs: Vec<Npc> = f
            .npcs
            .iter()
            .enumerate()
            .map(|(i, n)| Npc {
                id: i as u16,
                key: n.key.clone(),
                name: n.name.clone(),
                title: n.title.clone(),
                color: n.color,
                at: n.at,
                greeting: n.greeting.clone(),
            })
            .collect();
        let npc = |k: &str| -> Result<u16, String> {
            npcs.iter()
                .find(|n| n.key == k)
                .map(|n| n.id)
                .ok_or_else(|| format!("unknown NPC '{k}'"))
        };
        let creature = |k: &str| -> Result<u16, String> {
            creatures
                .iter()
                .find(|c: &&Creature| c.key == k)
                .map(|c| c.id)
                .ok_or_else(|| format!("unknown creature '{k}'"))
        };
        let mut quests: Vec<Quest> = Vec::new();
        for (i, q) in f.quests.iter().enumerate() {
            let ctx = |e: String| format!("quest '{}': {e}", q.key);
            let mut objectives = Vec::new();
            for o in &q.objectives {
                let ob = if let Some(k) = &o.kill {
                    Objective::Kill {
                        creature: creature(k).map_err(ctx)?,
                        count: o.count,
                    }
                } else if let Some(k) = &o.collect {
                    Objective::Collect {
                        item: item(k).map_err(ctx)?,
                        count: o.count,
                    }
                } else if let Some(k) = &o.talk {
                    Objective::Talk {
                        npc: npc(k).map_err(ctx)?,
                    }
                } else if let Some(name) = &o.explore {
                    Objective::Explore {
                        name: name.clone(),
                        at: o.at,
                        radius: o.radius,
                    }
                } else {
                    return Err(ctx(
                        "an objective needs kill, collect, talk or explore".into()
                    ));
                };
                objectives.push(ob);
            }
            let mut requires = Vec::new();
            for r in &q.requires {
                let id = f
                    .quests
                    .iter()
                    .position(|x| &x.key == r)
                    .ok_or_else(|| ctx(format!("requires unknown quest '{r}'")))?;
                requires.push(id as u16);
            }
            let list = |v: &[(String, u8)]| -> Result<Vec<(ItemId, u8)>, String> {
                v.iter().map(|(k, n)| item(k).map(|i| (i, *n))).collect()
            };
            let giver = npc(&q.giver).map_err(ctx)?;
            quests.push(Quest {
                id: i as u16,
                key: q.key.clone(),
                title: q.title.clone(),
                level: q.level.max(1),
                giver,
                ender: match &q.ender {
                    Some(e) => npc(e).map_err(ctx)?,
                    None => giver,
                },
                text: q.text.clone(),
                progress: if q.progress.is_empty() {
                    "How goes it?".into()
                } else {
                    q.progress.clone()
                },
                done: if q.done.is_empty() {
                    "Well done.".into()
                } else {
                    q.done.clone()
                },
                requires,
                objectives,
                xp: q.xp.unwrap_or_else(|| default_quest_xp(q.level.max(1))),
                money: q.money,
                items: list(&q.items).map_err(ctx)?,
                choice: list(&q.choice).map_err(ctx)?,
            });
        }
        let classes: Vec<Class> = f
            .classes
            .iter()
            .enumerate()
            .map(|(i, c)| Class {
                id: i as u16,
                key: c.key.clone(),
                name: c.name.clone(),
                resource: c.resource,
                color: c.color,
                blurb: c.blurb.clone(),
            })
            .collect();
        let mut abilities = Vec::new();
        for (i, a) in f.abilities.iter().enumerate() {
            let class = classes
                .iter()
                .find(|c| c.key == a.class)
                .map(|c| c.id)
                .ok_or_else(|| format!("ability '{}': unknown class '{}'", a.key, a.class))?;
            if !a.icon.is_empty() {
                item(&a.icon).map_err(|e| format!("ability '{}': {e}", a.key))?;
            }
            abilities.push(Ability {
                id: i as u16,
                key: a.key.clone(),
                name: a.name.clone(),
                class,
                level: a.level.max(1),
                text: a.text.clone(),
                cost: a.cost,
                cooldown: a.cooldown,
                cast: a.cast,
                range: a.range,
                damage: a.damage,
                heal: a.heal,
                radius: a.radius,
                root: a.root,
                stun: a.stun,
                shade_bonus: if a.shade_bonus > 0.0 { a.shade_bonus } else { 1.0 },
                blink: a.blink,
                recall: a.recall,
                color: a.color,
                icon: a.icon.clone(),
            });
        }
        Ok(Content {
            classes,
            abilities,
            name: f.name,
            version: f.version,
            items,
            blocks,
            recipes,
            creatures,
            milestones: f.milestones,
            npcs,
            quests,
            world: f.world,
        })
    }

    /// Item data for any item ID, including block items.
    pub fn item(&self, id: ItemId) -> ItemView<'_> {
        if id < FIRST_ITEM {
            let b = id.min(255) as BlockId;
            let d = block(b);
            ItemView {
                id,
                name: d.name,
                color: d.color,
                icon: Icon::Block,
                stack: 64,
                tool: None,
                food: 0,
                fuel: match b {
                    crate::LOG | crate::BIRCH_LOG => 4,
                    crate::PLANKS | 20 | 21 => 2,
                    _ => 0,
                },
                block: Some(b),
                light: 0,
            }
        } else if let Some(it) = self.items.get((id - FIRST_ITEM) as usize) {
            ItemView {
                id,
                name: &it.name,
                color: it.color,
                icon: it.icon,
                stack: it.stack,
                tool: it.tool.as_ref(),
                food: it.food,
                fuel: it.fuel,
                block: it.block,
                light: it.light,
            }
        } else {
            ItemView {
                id,
                name: "Unknown",
                color: [1.0, 0.0, 1.0],
                icon: Icon::Lump,
                stack: 64,
                tool: None,
                food: 0,
                fuel: 0,
                block: None,
                light: 0,
            }
        }
    }

    /// Item ID for a key ("stick", "stone").
    pub fn item_by_key(&self, key: &str) -> Option<ItemId> {
        if let Some(b) = block_by_key(key) {
            return Some(b as ItemId);
        }
        self.items.iter().find(|i| i.key == key).map(|i| i.id)
    }

    /// Stable key of an item, for saving.
    pub fn item_key(&self, id: ItemId) -> String {
        if id < FIRST_ITEM {
            block_key(id.min(255) as BlockId)
        } else {
            self.items
                .get((id - FIRST_ITEM) as usize)
                .map(|i| i.key.clone())
                .unwrap_or_default()
        }
    }

    pub fn rule(&self, b: BlockId) -> &BlockRule {
        &self.blocks[(b as usize).min(self.blocks.len() - 1)]
    }

    pub fn class(&self, id: u16) -> Option<&Class> {
        self.classes.get(id as usize)
    }
    pub fn ability(&self, id: u16) -> Option<&Ability> {
        self.abilities.get(id as usize)
    }
    /// A class's abilities known at a level, in the order they are learned.
    pub fn known_abilities(&self, class: u16, level: u8) -> Vec<&Ability> {
        let mut v: Vec<&Ability> = self
            .abilities
            .iter()
            .filter(|a| a.class == class && a.level <= level)
            .collect();
        v.sort_by_key(|a| (a.level, a.id));
        v
    }
    pub fn npc(&self, id: u16) -> Option<&Npc> {
        self.npcs.get(id as usize)
    }
    pub fn quest(&self, id: u16) -> Option<&Quest> {
        self.quests.get(id as usize)
    }
    pub fn creature(&self, id: u16) -> Option<&Creature> {
        self.creatures.get(id as usize)
    }
}

/// Uniform view of an item, whether it is a block item or not.
#[derive(Clone, Copy, Debug)]
pub struct ItemView<'a> {
    pub id: ItemId,
    pub name: &'a str,
    pub color: [f32; 3],
    pub icon: Icon,
    pub stack: u8,
    pub tool: Option<&'a Tool>,
    pub food: u8,
    pub fuel: u8,
    pub block: Option<BlockId>,
    pub light: u8,
}

/// Quest XP when the content does not say: about five same-level kills,
/// so that quests give a little over half of all XP.
pub fn default_quest_xp(level: u8) -> u32 {
    let kill = 45 + 5 * level as u32;
    (kill * 5).div_ceil(10) * 10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pack_loads() {
        let c = Content::default_pack();
        assert!(c.items.len() > 20);
        assert!(c.recipes.len() > 20);
        let pick = c.item_by_key("wooden_pickaxe").unwrap();
        assert_eq!(c.item(pick).tool.unwrap().kind, ToolKind::Pickaxe);
        assert_eq!(c.item(crate::STONE as ItemId).name, "Stone");
        // stone drops cobble and needs a pickaxe
        let r = c.rule(crate::STONE);
        assert_eq!(r.tier, 1);
        assert_eq!(r.drops[0].item, crate::COBBLE as ItemId);
        assert!(c.rule(crate::BEDROCK).hardness < 0.0);
    }

    #[test]
    fn bad_keys_are_reported() {
        let e = Content::from_json(
            r#"{"name":"x","version":1,"items":[],"recipes":[{"out":"nope","in":[]}]}"#,
        )
        .unwrap_err();
        assert!(e.contains("nope"), "{e}");
    }
}
