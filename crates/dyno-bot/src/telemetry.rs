//! What a bot measures while it plays: a timeline of rewards, time spent per
//! activity, fights, deaths, quests, travel and money. A finished timeline
//! becomes a `BotReport` (plain data) that the fun report aggregates.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What the bot is doing right now, for the time split.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Activity {
    Travel,
    Fight,
    Loot,
    Gather,
    /// talking, vending and standing about in a safe town
    Town,
    Rest,
    /// at a camp with nothing alive to fight
    WaitSpawn,
    Stuck,
    Dead,
    Idle,
    /// a chosen break (casual players)
    Afk,
}

impl Activity {
    pub fn all() -> [Activity; 11] {
        use Activity::*;
        [
            Travel, Fight, Loot, Gather, Town, Rest, WaitSpawn, Stuck, Dead, Idle, Afk,
        ]
    }
    pub fn key(self) -> &'static str {
        match self {
            Activity::Travel => "travel",
            Activity::Fight => "fight",
            Activity::Loot => "loot",
            Activity::Gather => "gather",
            Activity::Town => "town",
            Activity::Rest => "rest",
            Activity::WaitSpawn => "wait_spawn",
            Activity::Stuck => "stuck",
            Activity::Dead => "dead",
            Activity::Idle => "idle",
            Activity::Afk => "afk",
        }
    }
}

/// Kinds of rewarding moments, for the reward cadence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reward {
    Kill,
    Loot,
    Quest,
    Level,
    NewItem,
    NewArea,
    Gather,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Fight {
    pub start: f64,
    pub secs: f64,
    pub creature: String,
    pub level: u8,
    pub my_level: u8,
    pub won: bool,
    pub damage_taken: f32,
    pub casts: u32,
    /// how it ended
    #[serde(default)]
    pub why: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Death {
    pub t: f64,
    pub cause: String,
    pub level: u8,
    /// hearth-relative position
    pub at: [i32; 3],
    pub zone: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StuckAt {
    pub t: f64,
    /// hearth-relative position
    pub at: [i32; 3],
    /// hearth-relative goal
    pub goal: [i32; 3],
    pub why: String,
    pub doing: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct QuestRun {
    pub key: String,
    pub accepted: f64,
    pub done: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Blocker {
    pub t: f64,
    pub quest: String,
    pub what: String,
    pub why: String,
}

/// A bot's measurements while it plays.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Telemetry {
    pub name: String,
    pub class: String,
    pub brain: String,
    pub start_level: u8,
    /// seconds played
    pub secs: f64,
    pub time: BTreeMap<String, f64>,
    pub rewards: Vec<(f64, Reward)>,
    /// (t, level, total XP earned since joining)
    pub xp_curve: Vec<(f64, u8, u64)>,
    pub xp_total: u64,
    pub xp_quests: u64,
    pub xp_kills: u64,
    pub level: u8,
    pub first_kill: Option<f64>,
    pub first_quest: Option<f64>,
    pub first_level: Option<f64>,
    pub quests: Vec<QuestRun>,
    pub fights: Vec<Fight>,
    pub deaths: Vec<Death>,
    pub stuck: Vec<StuckAt>,
    pub blockers: Vec<Blocker>,
    pub unreachable: Vec<String>,
    pub abilities: BTreeMap<String, u32>,
    pub distance: f64,
    pub money_in: u64,
    pub money_out: u64,
    pub money: u32,
    pub kills: u32,
    pub kills_no_credit: u32,
    /// of those, with no other player within 20 blocks
    #[serde(default)]
    pub kills_no_credit_alone: u32,
    pub loots: u32,
    pub items_new: u32,
    pub areas: Vec<String>,
    pub gathers: u32,
    pub latency_ms: Vec<u32>,
    pub disconnects: Vec<String>,
    pub notes: Vec<String>,
    /// creatures seen standing on tree canopies: "Grazer lv1 at x,y,z" (hearth-relative)
    pub canopy: Vec<String>,
    /// (t, x, z, what the bot was doing) hearth-relative, every 10 s
    pub track: Vec<(f64, i32, i32, String)>,
}

impl Telemetry {
    pub fn add_time(&mut self, a: Activity, secs: f64) {
        *self.time.entry(a.key().to_string()).or_insert(0.0) += secs;
    }

    pub fn reward(&mut self, t: f64, r: Reward) {
        self.rewards.push((t, r));
    }

    pub fn note(&mut self, s: impl Into<String>) {
        let s = s.into();
        if !self.notes.contains(&s) && self.notes.len() < 50 {
            self.notes.push(s);
        }
    }

    pub fn blocker(&mut self, t: f64, quest: &str, what: &str, why: &str) {
        if self
            .blockers
            .iter()
            .any(|b| b.quest == quest && b.what == what && b.why == why)
        {
            return;
        }
        self.blockers.push(Blocker {
            t,
            quest: quest.into(),
            what: what.into(),
            why: why.into(),
        });
    }
}
