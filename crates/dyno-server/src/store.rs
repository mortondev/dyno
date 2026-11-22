//! Saving players and chests as JSON next to the world. Items are saved by
//! key, so content edits that change item IDs do not scramble inventories.

use dyno_protocol::Mode;
use dyno_sim::inventory::{Inventory, Stack};
use dyno_world::content::Content;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A saved stack: item key, count, wear.
pub type SavedStack = (String, u8, u16);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerData {
    #[serde(default)]
    pub token: u64,
    #[serde(default = "survival")]
    pub mode: Mode,
    pub health: f32,
    pub food: f32,
    pub pos: [f32; 3],
    #[serde(default)]
    pub spawn: Option<[f32; 3]>,
    #[serde(default)]
    pub pack: Vec<Option<SavedStack>>,
    #[serde(default)]
    pub fuel: u16,
    #[serde(default)]
    pub milestones: Vec<String>,
    /// unix seconds when last online
    #[serde(default)]
    pub last_seen: u64,
}

/// Server-wide saved state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Meta {
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub day: u32,
    #[serde(default)]
    pub clock: f32,
    #[serde(default)]
    pub events: Vec<(u64, String)>,
    /// the ruined hearth new players start beside
    #[serde(default)]
    pub spawn_hearth: Option<[i32; 3]>,
}

pub fn meta_path(world: &Path) -> PathBuf {
    world.with_file_name("server.json")
}

pub fn load_meta(path: &Path) -> Meta {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save_meta(path: &Path, m: &Meta) -> std::io::Result<()> {
    write_atomic(path, &serde_json::to_vec_pretty(m).unwrap_or_default())
}

fn survival() -> Mode {
    Mode::Survival
}

impl PlayerData {
    pub fn new(mode: Mode, spawn: [f32; 3]) -> Self {
        PlayerData {
            token: 0,
            mode,
            health: 20.0,
            food: 20.0,
            pos: spawn,
            spawn: None,
            pack: Vec::new(),
            fuel: 0,
            milestones: Vec::new(),
            last_seen: 0,
        }
    }
}

pub fn pack_to_keys(c: &Content, inv: &Inventory) -> Vec<Option<SavedStack>> {
    inv.slots
        .iter()
        .map(|s| s.map(|s| (c.item_key(s.item), s.count, s.wear)))
        .collect()
}

pub fn pack_from_keys(c: &Content, saved: &[Option<SavedStack>]) -> Inventory {
    let mut inv = Inventory::default();
    for (i, s) in saved.iter().enumerate().take(inv.slots.len()) {
        if let Some((k, n, wear)) = s
            && let Some(item) = c.item_by_key(k)
        {
            inv.slots[i] = Some(Stack {
                item,
                count: *n,
                wear: *wear,
            });
        }
    }
    inv
}

pub fn players_path(world: &Path) -> PathBuf {
    world.with_file_name("players.json")
}

pub fn chests_path(world: &Path) -> PathBuf {
    world.with_file_name("chests.json")
}

pub fn load_players(path: &Path) -> HashMap<String, PlayerData> {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn load_chests(c: &Content, path: &Path) -> HashMap<[i32; 3], Vec<Option<Stack>>> {
    let raw: HashMap<String, Vec<Option<SavedStack>>> = std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    raw.into_iter()
        .filter_map(|(k, v)| {
            let p: Vec<i32> = k.split(',').filter_map(|x| x.parse().ok()).collect();
            let pos: [i32; 3] = p.try_into().ok()?;
            let mut slots = vec![None; crate::game::CHEST_SLOTS];
            for (i, s) in v.into_iter().enumerate().take(slots.len()) {
                if let Some((key, n, wear)) = s
                    && let Some(item) = c.item_by_key(&key)
                {
                    slots[i] = Some(Stack {
                        item,
                        count: n,
                        wear,
                    });
                }
            }
            Some((pos, slots))
        })
        .collect()
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

pub fn save_players(path: &Path, players: &HashMap<String, PlayerData>) -> std::io::Result<()> {
    write_atomic(
        path,
        &serde_json::to_vec_pretty(players).unwrap_or_default(),
    )
}

pub fn save_chests(
    c: &Content,
    path: &Path,
    chests: &HashMap<[i32; 3], Vec<Option<Stack>>>,
) -> std::io::Result<()> {
    let raw: HashMap<String, Vec<Option<SavedStack>>> = chests
        .iter()
        .map(|(p, slots)| {
            (
                format!("{},{},{}", p[0], p[1], p[2]),
                slots
                    .iter()
                    .map(|s| s.map(|s| (c.item_key(s.item), s.count, s.wear)))
                    .collect(),
            )
        })
        .collect();
    write_atomic(path, &serde_json::to_vec(&raw).unwrap_or_default())
}
