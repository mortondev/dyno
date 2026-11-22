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
            });
        }
        Ok(Content {
            name: f.name,
            version: f.version,
            items,
            blocks,
            recipes,
            creatures,
            milestones: f.milestones,
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
