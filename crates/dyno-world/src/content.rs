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
    /// 0 poor, 1 common, 2 uncommon, 3 rare, 4 epic
    #[serde(default)]
    pub quality: u8,
    /// vendor price in copper
    #[serde(default)]
    pub value: u32,
    /// level needed to equip
    #[serde(default)]
    pub level: u8,
    #[serde(default)]
    pub gear: Option<Gear>,
    /// the item's picture in the pack ("icons/stick.png"); items that place
    /// a block show the block instead
    #[serde(default)]
    pub image: Option<String>,
}

/// Where a piece of gear is worn. The paper doll has one place per slot,
/// except rings and trinkets, which have two.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Slot {
    Head,
    Neck,
    Shoulders,
    Back,
    Chest,
    Wrist,
    Hands,
    Waist,
    Legs,
    Feet,
    Ring,
    Trinket,
    MainHand,
    OffHand,
    TwoHand,
    Ranged,
}

/// Paper doll places, in display order.
pub const EQUIP_PLACES: [(Slot, &str); 17] = [
    (Slot::Head, "Head"),
    (Slot::Neck, "Neck"),
    (Slot::Shoulders, "Shoulders"),
    (Slot::Back, "Back"),
    (Slot::Chest, "Chest"),
    (Slot::Wrist, "Wrist"),
    (Slot::Hands, "Hands"),
    (Slot::Waist, "Waist"),
    (Slot::Legs, "Legs"),
    (Slot::Feet, "Feet"),
    (Slot::Ring, "Finger"),
    (Slot::Ring, "Finger"),
    (Slot::Trinket, "Trinket"),
    (Slot::Trinket, "Trinket"),
    (Slot::MainHand, "Main hand"),
    (Slot::OffHand, "Off hand"),
    (Slot::Ranged, "Ranged"),
];
pub const MAIN_HAND: usize = 14;
pub const OFF_HAND: usize = 15;
pub const RANGED: usize = 16;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ArmorType {
    Cloth,
    Leather,
    Mail,
    Plate,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WeaponKind {
    Sword,
    Axe,
    Mace,
    Dagger,
    Staff,
    Polearm,
    Bow,
    Wand,
    Shield,
    Offhand,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Stats {
    pub str: i16,
    pub agi: i16,
    pub int: i16,
    pub sta: i16,
    pub spi: i16,
}

impl Stats {
    pub fn add(&mut self, o: &Stats) {
        self.str += o.str;
        self.agi += o.agi;
        self.int += o.int;
        self.sta += o.sta;
        self.spi += o.spi;
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Weapon {
    pub kind: WeaponKind,
    pub min: f32,
    pub max: f32,
    /// seconds between swings
    pub speed: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Gear {
    pub slot: Slot,
    #[serde(default)]
    pub armor_type: Option<ArmorType>,
    #[serde(default)]
    pub weapon: Option<Weapon>,
    #[serde(default)]
    pub armor: u16,
    #[serde(default)]
    pub stats: Stats,
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
    /// profession and skill needed to gather it
    #[serde(default)]
    pub skill: Option<(String, u16)>,
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
    /// profession and skill needed to make it
    #[serde(default)]
    pub skill: Option<(String, u16)>,
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
    /// size of the model (1 is normal)
    #[serde(default = "one_f")]
    pub scale: f32,
    /// which look of the model (horns, tusks, armour...)
    #[serde(default)]
    pub variant: u8,
    /// seconds between attacks
    #[serde(default = "attack_speed")]
    pub attack_speed: f32,
    /// a special move
    #[serde(default)]
    pub ability: Option<CreatureAbility>,
    /// sight range for hostile creatures (0: the default)
    #[serde(default)]
    pub aggro: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CreatureAbility {
    /// rushes at a target this far away, at three times speed
    Charge { range: f32 },
    /// attacks from range with a bolt
    Bolt { range: f32 },
    /// runs away at low health
    Flee { below: f32 },
}

fn one_f() -> f32 {
    1.0
}
fn attack_speed() -> f32 {
    1.5
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
    /// what they sell, and the price in copper
    #[serde(default)]
    pub sells: Vec<(ItemId, u32)>,
    /// shown only in (true) or out of (false) a phase
    #[serde(default)]
    pub phase: Option<(u8, bool)>,
}

/// A phase: a way the world looks to you once your story has moved it on.
/// Two players side by side can see different things.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Phase {
    pub key: String,
    /// you are in the phase once you have finished this quest
    pub done: u16,
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
    /// refills quickly (10 a second) and steadily
    Energy,
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
    /// heaviest armour the class can wear
    pub armor: ArmorType,
    pub weapons: Vec<WeaponKind>,
    /// base stats at level 1, and gained per level
    pub base: Stats,
    pub growth: Stats,
    /// item keys a new character of this class starts with (worn when possible)
    pub starter: Vec<(String, u8)>,
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
    /// adds a weapon swing times this (0: none)
    pub weapon: f32,
    /// damage over time: base, per level, seconds (ticks every 3 s)
    pub dot: [f32; 3],
    /// healing over time on yourself: base, per level, seconds
    pub hot: [f32; 3],
    /// a timed effect on yourself
    pub buff: Option<Buff>,
    /// combo points: positive builds them, negative spends all (damage times points)
    pub combo: i8,
    /// rush to the target first
    pub charge: bool,
    /// rage or energy given (Onrush)
    pub gives: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuffKind {
    /// +percent damage dealt
    Damage,
    /// -percent damage taken
    Ward,
    /// +percent attack speed
    Haste,
    Strength,
    Agility,
    Intellect,
    Stamina,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Buff {
    pub kind: BuffKind,
    pub amount: f32,
    pub secs: f32,
    /// extra seconds per combo point spent
    #[serde(default)]
    pub per_combo: f32,
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
    #[serde(default)]
    pub professions: Vec<Profession>,
    #[serde(default)]
    pub areas: Vec<Area>,
    #[serde(default)]
    pub phases: Vec<Phase>,
}

/// A named place on the map: its name shows when you enter, and it holds
/// creature camps with fixed spawn points and respawn timers.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Area {
    pub key: String,
    pub name: String,
    /// centre (x, z) relative to the spawn hearth, and radius
    pub at: [i32; 2],
    pub radius: f32,
    /// level band of creatures here
    pub levels: [u8; 2],
    /// a hearth's peace: no hostile creatures spawn
    #[serde(default)]
    pub safe: bool,
    #[serde(default)]
    pub camps: Vec<Camp>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Camp {
    pub creature: u16,
    /// shown only in (true) or out of (false) a phase
    #[serde(default)]
    pub phase: Option<(u8, bool)>,
    /// spawn points in the camp (around its centre)
    pub count: u8,
    /// centre (x, z) relative to the spawn hearth; spawn points scatter within `spread`
    pub at: [i32; 2],
    pub spread: f32,
    /// seconds before a killed creature returns
    pub respawn: u16,
    /// a rare named elite: chance it appears instead, its name, and its strength
    #[serde(default)]
    pub rare: Option<Rare>,
    /// underground: spawn near this height relative to the hearth, not on the surface
    #[serde(default)]
    pub y: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Rare {
    pub name: String,
    pub chance: f32,
    /// health and damage multiplier
    pub power: f32,
    /// extra levels
    pub levels: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Profession {
    pub key: String,
    pub name: String,
    /// gathering or crafting
    pub gathering: bool,
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
    #[serde(default)]
    professions: Vec<Profession>,
    #[serde(default)]
    areas: Vec<AArea>,
    #[serde(default)]
    phases: Vec<APhase>,
}

#[derive(Deserialize)]
struct APhase {
    key: String,
    done: String,
}

#[derive(Deserialize)]
struct AArea {
    key: String,
    name: String,
    at: [i32; 2],
    radius: f32,
    levels: [u8; 2],
    #[serde(default)]
    safe: bool,
    #[serde(default)]
    camps: Vec<ACamp>,
}

#[derive(Deserialize)]
struct ACamp {
    creature: String,
    count: u8,
    at: [i32; 2],
    #[serde(default = "eight")]
    spread: f32,
    #[serde(default = "five_min")]
    respawn: u16,
    #[serde(default)]
    rare: Option<Rare>,
    #[serde(default)]
    y: Option<i32>,
    #[serde(default)]
    phase: Option<String>,
}

fn five_min() -> u16 {
    300
}

#[derive(Deserialize)]
struct AClass {
    key: String,
    name: String,
    resource: Resource,
    color: [f32; 3],
    #[serde(default)]
    blurb: String,
    #[serde(default = "cloth")]
    armor: ArmorType,
    #[serde(default)]
    weapons: Vec<WeaponKind>,
    #[serde(default)]
    base: Stats,
    #[serde(default)]
    growth: Stats,
    #[serde(default)]
    starter: Vec<(String, u8)>,
}

fn cloth() -> ArmorType {
    ArmorType::Cloth
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
    weapon: f32,
    dot: [f32; 3],
    hot: [f32; 3],
    buff: Option<Buff>,
    combo: i8,
    charge: bool,
    gives: f32,
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
    /// item keys, each optionally with a price: "cooked_meat:25"
    #[serde(default)]
    sells: Vec<String>,
    /// "key" to show only in a phase, "!key" only outside it
    #[serde(default)]
    phase: Option<String>,
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
    #[serde(default)]
    quality: Option<u8>,
    #[serde(default)]
    value: u32,
    #[serde(default)]
    level: u8,
    #[serde(default)]
    gear: Option<Gear>,
    #[serde(default)]
    image: Option<String>,
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
    #[serde(default)]
    skill: Option<(String, u16)>,
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
    #[serde(default)]
    skill: Option<(String, u16)>,
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
    #[serde(default = "one_f")]
    scale: f32,
    #[serde(default)]
    variant: u8,
    #[serde(default = "attack_speed")]
    attack_speed: f32,
    #[serde(default)]
    ability: Option<CreatureAbility>,
    #[serde(default)]
    aggro: f32,
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
                quality: a.quality.unwrap_or(1),
                value: a.value,
                level: a.level,
                gear: a.gear.clone(),
                image: a.image.clone(),
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
                    skill: a.skill.clone(),
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
                    skill: None,
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
                skill: r.skill.clone(),
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
                scale: c.scale,
                variant: c.variant,
                attack_speed: c.attack_speed,
                ability: c.ability,
                aggro: c.aggro,
            });
        }
        let phase_of = |v: &Option<String>| -> Result<Option<(u8, bool)>, String> {
            let Some(v) = v else { return Ok(None) };
            let (key, inside) = match v.strip_prefix('!') {
                Some(k) => (k, false),
                None => (v.as_str(), true),
            };
            let i = f
                .phases
                .iter()
                .position(|p| p.key == key)
                .ok_or_else(|| format!("unknown phase '{key}'"))?;
            if i >= 64 {
                return Err("at most 64 phases".into());
            }
            Ok(Some((i as u8, inside)))
        };
        let mut npcs: Vec<Npc> = Vec::new();
        for (i, n) in f.npcs.iter().enumerate() {
            let mut sells = Vec::new();
            for s in &n.sells {
                let (k, price) = match s.split_once(':') {
                    Some((k, p)) => (k.trim(), p.trim().parse::<u32>().ok()),
                    None => (s.trim(), None),
                };
                let id = item(k)?;
                // a merchant asks four times what they would pay you
                let value = f
                    .items
                    .iter()
                    .find(|it| it.key == k)
                    .map(|it| it.value)
                    .unwrap_or(0);
                let price = price.unwrap_or(value * 4).max(1);
                sells.push((id, price));
            }
            npcs.push(Npc {
                id: i as u16,
                key: n.key.clone(),
                name: n.name.clone(),
                title: n.title.clone(),
                color: n.color,
                at: n.at,
                greeting: n.greeting.clone(),
                sells,
                phase: phase_of(&n.phase)?,
            });
        }
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
                armor: c.armor,
                weapons: c.weapons.clone(),
                base: c.base,
                growth: c.growth,
                starter: c.starter.clone(),
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
                shade_bonus: if a.shade_bonus > 0.0 {
                    a.shade_bonus
                } else {
                    1.0
                },
                blink: a.blink,
                recall: a.recall,
                color: a.color,
                icon: a.icon.clone(),
                weapon: a.weapon,
                dot: a.dot,
                hot: a.hot,
                buff: a.buff,
                combo: a.combo,
                charge: a.charge,
                gives: a.gives,
            });
        }
        let mut areas = Vec::new();
        for a in &f.areas {
            let mut camps = Vec::new();
            for c in &a.camps {
                let id = creatures
                    .iter()
                    .find(|x: &&Creature| x.key == c.creature)
                    .map(|x| x.id)
                    .ok_or_else(|| {
                        format!("area '{}': unknown creature '{}'", a.key, c.creature)
                    })?;
                camps.push(Camp {
                    creature: id,
                    phase: phase_of(&c.phase)?,
                    count: c.count,
                    at: c.at,
                    spread: c.spread,
                    respawn: c.respawn,
                    rare: c.rare.clone(),
                    y: c.y,
                });
            }
            areas.push(Area {
                key: a.key.clone(),
                name: a.name.clone(),
                at: a.at,
                radius: a.radius,
                levels: a.levels,
                safe: a.safe,
                camps,
            });
        }
        let mut phases = Vec::new();
        for p in &f.phases {
            let done = quests
                .iter()
                .find(|q| q.key == p.done)
                .map(|q| q.id)
                .ok_or_else(|| format!("phase '{}': unknown quest '{}'", p.key, p.done))?;
            phases.push(Phase {
                key: p.key.clone(),
                done,
            });
        }
        Ok(Content {
            areas,
            phases,
            professions: f.professions,
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
                quality: 1,
                value: 0,
                level: 0,
                gear: None,
                image: None,
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
                quality: it.quality,
                value: it.value,
                level: it.level,
                gear: it.gear.as_ref(),
                image: it.image.as_deref(),
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
                quality: 0,
                value: 0,
                level: 0,
                gear: None,
                image: None,
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
    pub quality: u8,
    pub value: u32,
    pub level: u8,
    pub gear: Option<&'a Gear>,
    /// the item's picture in the pack, if it has one
    pub image: Option<&'a str>,
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
