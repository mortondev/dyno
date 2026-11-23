//! The fun report: per-bot summaries, a swarm aggregate, and checks of the
//! fun thresholds in docs/playtest/fun-metrics.md, as JSON and Markdown.

use crate::telemetry::{Activity, Reward, Telemetry};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// XP from level 1 to 5 (400 + 900 + 1400 + 2100) in the design's 45 minutes.
pub const DESIGN_XP_PER_HOUR: f64 = 4800.0 / 0.75;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BotSummary {
    pub name: String,
    pub class: String,
    pub brain: String,
    pub minutes: f64,
    pub start_level: u8,
    pub level: u8,
    pub xp: u64,
    pub xp_per_hour: f64,
    pub xp_quest_share: Option<f64>,
    pub first_kill_s: Option<f64>,
    pub first_quest_s: Option<f64>,
    pub first_level_s: Option<f64>,
    pub quests_accepted: usize,
    pub quests_done: usize,
    pub quest_minutes_median: Option<f64>,
    pub kills: u32,
    pub kills_no_credit: u32,
    pub deaths: usize,
    pub fights: usize,
    pub win_rate: Option<f64>,
    pub fight_secs_median: Option<f64>,
    pub reward_gap_median_s: Option<f64>,
    pub reward_gap_longest_s: f64,
    pub rewards: usize,
    /// share of time per activity
    pub time_split: BTreeMap<String, f64>,
    pub distance: f64,
    pub distance_per_reward: Option<f64>,
    pub abilities_distinct: usize,
    pub top_ability_share: Option<f64>,
    pub money_in: u64,
    pub money_out: u64,
    pub stuck_events: usize,
    pub disconnects: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Check {
    pub key: String,
    pub what: String,
    pub value: Option<f64>,
    pub unit: String,
    pub target: String,
    /// pass, warn, fail, or no data
    pub status: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct QuestStat {
    pub key: String,
    pub accepted_by: usize,
    pub done_by: usize,
    pub median_minutes: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Count {
    pub what: String,
    pub count: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StuckCell {
    /// hearth-relative x, z of a 16-block cell's corner
    pub cell: [i32; 2],
    pub count: usize,
    pub examples: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CurvePoint {
    pub minute: u32,
    pub bots: usize,
    pub median_level: f64,
    pub mean_xp: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Aggregate {
    pub bots: usize,
    pub bot_minutes: f64,
    pub median_first_kill_s: Option<f64>,
    pub median_first_quest_s: Option<f64>,
    pub median_first_level_s: Option<f64>,
    pub share_reached_level_2: f64,
    pub median_xp_per_hour: Option<f64>,
    pub xp_quest_share: Option<f64>,
    pub quests_done: usize,
    pub median_quest_minutes: Option<f64>,
    pub median_reward_gap_s: Option<f64>,
    pub median_longest_drought_s: Option<f64>,
    pub share_drought_over_5min: f64,
    pub deaths: usize,
    pub deaths_per_hour: f64,
    pub deaths_per_fight: Option<f64>,
    pub fights: usize,
    pub win_rate: Option<f64>,
    pub median_fight_secs: Option<f64>,
    pub kills: u32,
    pub kills_no_credit: u32,
    pub time_split: BTreeMap<String, f64>,
    pub distance_per_reward: Option<f64>,
    pub mean_abilities_distinct: f64,
    pub mean_top_ability_share: Option<f64>,
    pub money_in: u64,
    pub money_out: u64,
    pub stuck_per_hour: f64,
    pub disconnects: usize,
    pub latency_p50_ms: Option<f64>,
    pub latency_p95_ms: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Report {
    pub title: String,
    pub server: String,
    pub minutes: f64,
    pub aggregate: Aggregate,
    pub checks: Vec<Check>,
    pub bots: Vec<BotSummary>,
    pub by_brain: BTreeMap<String, Aggregate>,
    pub by_class: BTreeMap<String, Aggregate>,
    pub quests: Vec<QuestStat>,
    pub blockers: Vec<Count>,
    pub deaths_by_cause: Vec<Count>,
    pub unreachable: Vec<Count>,
    pub stuck_heatmap: Vec<StuckCell>,
    pub level_curve: Vec<CurvePoint>,
    pub notes: Vec<Count>,
    /// creatures seen on tree canopies (out of melee and loot reach)
    pub canopy: Vec<Count>,
    pub telemetry: Vec<Telemetry>,
}

pub fn median(v: &[f64]) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = s.len();
    Some(if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) * 0.5
    })
}

pub fn percentile(v: &[f64], p: f64) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let i = ((s.len() - 1) as f64 * p).round() as usize;
    Some(s[i.min(s.len() - 1)])
}

fn mean(v: &[f64]) -> Option<f64> {
    (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
}

/// Gaps between rewarding moments, counting from the start and to the end.
pub fn reward_gaps(t: &Telemetry) -> Vec<f64> {
    let mut times: Vec<f64> = t.rewards.iter().map(|(t, _)| *t).collect();
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // rewards within 2 s are one moment (a kill, its loot and a level)
    let mut moments: Vec<f64> = Vec::new();
    for x in times {
        if moments.last().is_none_or(|l| x - l > 2.0) {
            moments.push(x);
        }
    }
    let mut gaps = Vec::new();
    let mut prev = 0.0;
    for x in &moments {
        gaps.push(x - prev);
        prev = *x;
    }
    if t.secs > prev {
        gaps.push(t.secs - prev);
    }
    gaps
}

pub fn summarize(t: &Telemetry) -> BotSummary {
    let hours = (t.secs / 3600.0).max(1e-6);
    let gaps = reward_gaps(t);
    // the open gap at the end counts toward the longest drought, but not the median
    let closed: Vec<f64> = if t.rewards.is_empty() {
        Vec::new()
    } else {
        gaps[..gaps.len().saturating_sub(1)].to_vec()
    };
    let total: f64 = t.time.values().sum::<f64>().max(1e-6);
    let time_split = Activity::all()
        .iter()
        .map(|a| {
            (
                a.key().to_string(),
                t.time.get(a.key()).copied().unwrap_or(0.0) / total,
            )
        })
        .collect();
    let done: Vec<f64> = t
        .quests
        .iter()
        .filter_map(|q| q.done.map(|d| (d - q.accepted) / 60.0))
        .collect();
    let won = t.fights.iter().filter(|f| f.won).count();
    let casts: u32 = t
        .abilities
        .iter()
        .filter(|(k, _)| *k != "Recall")
        .map(|(_, v)| *v)
        .sum();
    let top = t
        .abilities
        .iter()
        .filter(|(k, _)| *k != "Recall")
        .map(|(_, v)| *v)
        .max()
        .unwrap_or(0);
    let rewards = t.rewards.len();
    BotSummary {
        name: t.name.clone(),
        class: t.class.clone(),
        brain: t.brain.clone(),
        minutes: t.secs / 60.0,
        start_level: t.start_level,
        level: t.level,
        xp: t.xp_total,
        xp_per_hour: t.xp_total as f64 / hours,
        xp_quest_share: (t.xp_total > 0).then(|| t.xp_quests as f64 / t.xp_total as f64),
        first_kill_s: t.first_kill,
        first_quest_s: t.first_quest,
        first_level_s: t.first_level,
        quests_accepted: t.quests.len(),
        quests_done: t.quests.iter().filter(|q| q.done.is_some()).count(),
        quest_minutes_median: median(&done),
        kills: t.kills,
        kills_no_credit: t.kills_no_credit,
        deaths: t.deaths.len(),
        fights: t.fights.len(),
        win_rate: (!t.fights.is_empty()).then(|| won as f64 / t.fights.len() as f64),
        fight_secs_median: median(&t.fights.iter().map(|f| f.secs).collect::<Vec<_>>()),
        reward_gap_median_s: median(&closed),
        reward_gap_longest_s: gaps.iter().copied().fold(0.0, f64::max),
        rewards,
        time_split,
        distance: t.distance,
        distance_per_reward: (rewards > 0).then(|| t.distance / rewards as f64),
        abilities_distinct: t.abilities.keys().filter(|k| *k != "Recall").count(),
        top_ability_share: (casts > 0).then(|| top as f64 / casts as f64),
        money_in: t.money_in,
        money_out: t.money_out,
        stuck_events: t.stuck.len(),
        disconnects: t.disconnects.len(),
    }
}

pub fn aggregate(ts: &[Telemetry]) -> Aggregate {
    let sums: Vec<BotSummary> = ts.iter().map(summarize).collect();
    let opt = |f: &dyn Fn(&BotSummary) -> Option<f64>| -> Vec<f64> {
        sums.iter().filter_map(f).collect()
    };
    let hours: f64 = ts.iter().map(|t| t.secs / 3600.0).sum::<f64>().max(1e-6);
    let mut split: BTreeMap<String, f64> = BTreeMap::new();
    let total: f64 = ts
        .iter()
        .flat_map(|t| t.time.values())
        .sum::<f64>()
        .max(1e-6);
    for a in Activity::all() {
        let v: f64 = ts.iter().filter_map(|t| t.time.get(a.key())).sum();
        split.insert(a.key().into(), v / total);
    }
    let fights: usize = ts.iter().map(|t| t.fights.len()).sum();
    let won: usize = ts
        .iter()
        .map(|t| t.fights.iter().filter(|f| f.won).count())
        .sum();
    let deaths: usize = ts.iter().map(|t| t.deaths.len()).sum();
    let xp: u64 = ts.iter().map(|t| t.xp_total).sum();
    let xpq: u64 = ts.iter().map(|t| t.xp_quests).sum();
    let rewards: usize = ts.iter().map(|t| t.rewards.len()).sum();
    let dist: f64 = ts.iter().map(|t| t.distance).sum();
    let lat: Vec<f64> = ts
        .iter()
        .flat_map(|t| t.latency_ms.iter().map(|v| *v as f64))
        .collect();
    let started_low: Vec<&Telemetry> = ts.iter().filter(|t| t.start_level <= 1).collect();
    Aggregate {
        bots: ts.len(),
        bot_minutes: hours * 60.0,
        median_first_kill_s: median(&opt(&|s| s.first_kill_s)),
        median_first_quest_s: median(&opt(&|s| s.first_quest_s)),
        median_first_level_s: median(&opt(&|s| {
            (s.start_level <= 1).then_some(s.first_level_s).flatten()
        })),
        share_reached_level_2: if started_low.is_empty() {
            0.0
        } else {
            started_low.iter().filter(|t| t.level >= 2).count() as f64 / started_low.len() as f64
        },
        median_xp_per_hour: median(&opt(&|s| (s.minutes >= 2.0).then_some(s.xp_per_hour))),
        xp_quest_share: (xp > 0).then(|| xpq as f64 / xp as f64),
        quests_done: sums.iter().map(|s| s.quests_done).sum(),
        median_quest_minutes: median(
            &ts.iter()
                .flat_map(|t| {
                    t.quests
                        .iter()
                        .filter_map(|q| q.done.map(|d| (d - q.accepted) / 60.0))
                })
                .collect::<Vec<_>>(),
        ),
        median_reward_gap_s: median(&opt(&|s| s.reward_gap_median_s)),
        median_longest_drought_s: median(
            &sums
                .iter()
                .map(|s| s.reward_gap_longest_s)
                .collect::<Vec<_>>(),
        ),
        share_drought_over_5min: if sums.is_empty() {
            0.0
        } else {
            sums.iter()
                .filter(|s| s.reward_gap_longest_s > 300.0)
                .count() as f64
                / sums.len() as f64
        },
        deaths,
        deaths_per_hour: deaths as f64 / hours,
        deaths_per_fight: (fights > 0).then(|| deaths as f64 / fights as f64),
        fights,
        win_rate: (fights > 0).then(|| won as f64 / fights as f64),
        median_fight_secs: median(
            &ts.iter()
                .flat_map(|t| t.fights.iter().filter(|f| f.won).map(|f| f.secs))
                .collect::<Vec<_>>(),
        ),
        kills: ts.iter().map(|t| t.kills).sum(),
        kills_no_credit: ts.iter().map(|t| t.kills_no_credit).sum(),
        time_split: split,
        distance_per_reward: (rewards > 0).then(|| dist / rewards as f64),
        mean_abilities_distinct: mean(
            &sums
                .iter()
                .filter(|s| s.fights > 0)
                .map(|s| s.abilities_distinct as f64)
                .collect::<Vec<_>>(),
        )
        .unwrap_or(0.0),
        mean_top_ability_share: mean(&opt(&|s| s.top_ability_share)),
        money_in: ts.iter().map(|t| t.money_in).sum(),
        money_out: ts.iter().map(|t| t.money_out).sum(),
        stuck_per_hour: ts.iter().map(|t| t.stuck.len()).sum::<usize>() as f64 / hours,
        disconnects: ts.iter().map(|t| t.disconnects.len()).sum(),
        latency_p50_ms: percentile(&lat, 0.5),
        latency_p95_ms: percentile(&lat, 0.95),
    }
}

#[allow(clippy::too_many_arguments)]
/// Grade a value: `good` and `bad` limits, lower is better when `low` is true.
fn grade(
    key: &str,
    what: &str,
    value: Option<f64>,
    unit: &str,
    target: &str,
    good: f64,
    bad: f64,
    low: bool,
) -> Check {
    let status = match value {
        None => "no data",
        Some(v) => {
            let (g, b) = if low {
                (v <= good, v > bad)
            } else {
                (v >= good, v < bad)
            };
            if g {
                "pass"
            } else if b {
                "fail"
            } else {
                "warn"
            }
        }
    };
    Check {
        key: key.into(),
        what: what.into(),
        value,
        unit: unit.into(),
        target: target.into(),
        status: status.into(),
    }
}

/// The fun thresholds (docs/playtest/fun-metrics.md).
pub fn checks(a: &Aggregate) -> Vec<Check> {
    let mut v = vec![
        grade(
            "first_kill",
            "Time to first kill (median)",
            a.median_first_kill_s,
            "s",
            "<= 60 s pass, > 120 s fail",
            60.0,
            120.0,
            true,
        ),
        grade(
            "first_quest",
            "Time to first quest turned in (median)",
            a.median_first_quest_s,
            "s",
            "<= 180 s pass, > 300 s fail",
            180.0,
            300.0,
            true,
        ),
        grade(
            "level_2",
            "Time to level 2 from level 1 (median)",
            a.median_first_level_s,
            "s",
            "<= 300 s (design) pass, > 600 s fail",
            300.0,
            600.0,
            true,
        ),
        grade(
            "xp_rate",
            "XP per hour vs. the design pace (level 5 in 45 min)",
            a.median_xp_per_hour.map(|x| x / DESIGN_XP_PER_HOUR),
            "x design",
            ">= 0.8 pass, < 0.5 fail",
            0.8,
            0.5,
            false,
        ),
        grade(
            "quest_share",
            "Share of XP from quests",
            a.xp_quest_share,
            "",
            ">= 0.45 pass (design 0.6), < 0.3 fail",
            0.45,
            0.3,
            false,
        ),
        grade(
            "quest_time",
            "Minutes per quest (median, accept to turn-in)",
            a.median_quest_minutes,
            "min",
            "<= 10 pass, > 15 fail",
            10.0,
            15.0,
            true,
        ),
        grade(
            "reward_gap",
            "Seconds between rewarding moments (median of bots' medians)",
            a.median_reward_gap_s,
            "s",
            "<= 60 pass, > 90 fail",
            60.0,
            90.0,
            true,
        ),
        grade(
            "drought",
            "Share of bots with a reward drought over 5 minutes",
            (a.bots > 0).then_some(a.share_drought_over_5min),
            "",
            "<= 0.1 pass, > 0.25 fail",
            0.1,
            0.25,
            true,
        ),
        grade(
            "deaths",
            "Deaths per hour of play",
            (a.bot_minutes > 1.0).then_some(a.deaths_per_hour),
            "/h",
            "<= 1.5 pass, > 4 fail",
            1.5,
            4.0,
            true,
        ),
        grade(
            "win_rate",
            "Fights won",
            a.win_rate,
            "",
            ">= 0.9 pass, < 0.75 fail",
            0.9,
            0.75,
            false,
        ),
        grade(
            "fight_length",
            "Seconds per won fight (median)",
            a.median_fight_secs,
            "s",
            "<= 25 pass, > 40 fail (and >= 5)",
            25.0,
            40.0,
            true,
        ),
        grade(
            "dead_time",
            "Share of time idle, stuck, waiting for spawns or dead",
            (a.bots > 0).then_some(
                ["idle", "stuck", "wait_spawn", "dead"]
                    .iter()
                    .map(|k| a.time_split.get(*k).copied().unwrap_or(0.0))
                    .sum::<f64>(),
            ),
            "",
            "<= 0.15 pass, > 0.3 fail",
            0.15,
            0.3,
            true,
        ),
        grade(
            "travel",
            "Share of time travelling",
            a.time_split.get("travel").copied().filter(|_| a.bots > 0),
            "",
            "<= 0.4 pass, > 0.55 fail",
            0.4,
            0.55,
            true,
        ),
        grade(
            "distance_per_reward",
            "Blocks walked per reward",
            a.distance_per_reward,
            "blocks",
            "<= 60 pass, > 120 fail",
            60.0,
            120.0,
            true,
        ),
        grade(
            "variety",
            "Distinct abilities used per fighting bot",
            (a.fights > 0).then_some(a.mean_abilities_distinct),
            "",
            ">= 2 pass, < 1.5 fail",
            2.0,
            1.5,
            false,
        ),
        grade(
            "one_button",
            "Share of casts that are the most used ability",
            a.mean_top_ability_share,
            "",
            "<= 0.7 pass, > 0.85 fail",
            0.7,
            0.85,
            true,
        ),
        grade(
            "kill_credit",
            "Kills lost to someone else's last hit",
            (a.kills + a.kills_no_credit > 0)
                .then(|| a.kills_no_credit as f64 / (a.kills + a.kills_no_credit) as f64),
            "",
            "<= 0.05 pass, > 0.15 fail",
            0.05,
            0.15,
            true,
        ),
        grade(
            "stuck",
            "Stuck events per bot-hour",
            (a.bot_minutes > 1.0).then_some(a.stuck_per_hour),
            "/h",
            "<= 2 pass, > 6 fail",
            2.0,
            6.0,
            true,
        ),
        grade(
            "latency",
            "Action round trip p95",
            a.latency_p95_ms,
            "ms",
            "<= 150 pass, > 400 fail",
            150.0,
            400.0,
            true,
        ),
        grade(
            "disconnects",
            "Unexpected disconnects",
            (a.bots > 0).then_some(a.disconnects as f64),
            "",
            "0 pass, > 1 fail",
            0.0,
            1.0,
            true,
        ),
    ];
    // money: earning should outpace spending in the first hour
    v.push(grade(
        "economy",
        "Copper earned per copper spent",
        (a.money_in > 0).then(|| a.money_in as f64 / (a.money_out.max(1)) as f64),
        "x",
        ">= 1.5 pass, < 1 fail",
        1.5,
        1.0,
        false,
    ));
    v
}

fn counts(v: impl IntoIterator<Item = String>) -> Vec<Count> {
    let mut m: BTreeMap<String, usize> = BTreeMap::new();
    for s in v {
        *m.entry(s).or_insert(0) += 1;
    }
    let mut out: Vec<Count> = m
        .into_iter()
        .map(|(what, count)| Count { what, count })
        .collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then(a.what.cmp(&b.what)));
    out
}

pub fn build(title: &str, server: &str, ts: Vec<Telemetry>) -> Report {
    let aggregate = aggregate(&ts);
    let checks = checks(&aggregate);
    let bots: Vec<BotSummary> = ts.iter().map(summarize).collect();
    let mut by_brain: BTreeMap<String, Vec<Telemetry>> = BTreeMap::new();
    let mut by_class: BTreeMap<String, Vec<Telemetry>> = BTreeMap::new();
    for t in &ts {
        by_brain.entry(t.brain.clone()).or_default().push(t.clone());
        by_class.entry(t.class.clone()).or_default().push(t.clone());
    }
    // quests
    let mut qs: BTreeMap<String, (usize, usize, Vec<f64>)> = BTreeMap::new();
    for t in &ts {
        for q in &t.quests {
            let e = qs.entry(q.key.clone()).or_default();
            e.0 += 1;
            if let Some(d) = q.done {
                e.1 += 1;
                e.2.push((d - q.accepted) / 60.0);
            }
        }
    }
    let quests = qs
        .into_iter()
        .map(|(key, (a, d, m))| QuestStat {
            key,
            accepted_by: a,
            done_by: d,
            median_minutes: median(&m),
        })
        .collect();
    let blockers = counts(ts.iter().flat_map(|t| {
        t.blockers
            .iter()
            .map(|b| format!("{}: {}: {}", b.quest, b.what, b.why))
    }));
    let deaths_by_cause = counts(
        ts.iter()
            .flat_map(|t| t.deaths.iter().map(|d| d.cause.clone())),
    );
    let unreachable = counts(ts.iter().flat_map(|t| t.unreachable.iter().cloned()));
    let notes = counts(ts.iter().flat_map(|t| t.notes.iter().cloned()));
    let canopy = counts(ts.iter().flat_map(|t| t.canopy.iter().cloned()));
    // stuck heatmap in 16-block cells
    let mut heat: BTreeMap<[i32; 2], (usize, Vec<String>)> = BTreeMap::new();
    for t in &ts {
        for s in &t.stuck {
            let cell = [s.at[0].div_euclid(16) * 16, s.at[2].div_euclid(16) * 16];
            let e = heat.entry(cell).or_default();
            e.0 += 1;
            if e.1.len() < 4 {
                e.1.push(format!(
                    "{} at {:?} going to {:?} ({}; {})",
                    t.name, s.at, s.goal, s.why, s.doing
                ));
            }
        }
    }
    let mut stuck_heatmap: Vec<StuckCell> = heat
        .into_iter()
        .map(|(cell, (count, examples))| StuckCell {
            cell,
            count,
            examples,
        })
        .collect();
    stuck_heatmap.sort_by_key(|a| std::cmp::Reverse(a.count));
    // level curve by minute
    let max_min = ts.iter().map(|t| t.secs / 60.0).fold(0.0, f64::max).ceil() as u32;
    let mut level_curve = Vec::new();
    for m in 0..=max_min {
        let s = m as f64 * 60.0;
        let mut lv = Vec::new();
        let mut xp = Vec::new();
        for t in ts.iter().filter(|t| t.secs >= s) {
            let at = t.xp_curve.iter().rev().find(|(ts, ..)| *ts <= s);
            lv.push(at.map(|x| x.1).unwrap_or(t.start_level) as f64);
            xp.push(at.map(|x| x.2).unwrap_or(0) as f64);
        }
        if lv.is_empty() {
            break;
        }
        level_curve.push(CurvePoint {
            minute: m,
            bots: lv.len(),
            median_level: median(&lv).unwrap_or(0.0),
            mean_xp: mean(&xp).unwrap_or(0.0),
        });
    }
    Report {
        title: title.into(),
        server: server.into(),
        minutes: ts.iter().map(|t| t.secs / 60.0).fold(0.0, f64::max),
        aggregate,
        checks,
        bots,
        by_brain: by_brain
            .into_iter()
            .map(|(k, v)| (k, self::aggregate(&v)))
            .collect(),
        by_class: by_class
            .into_iter()
            .map(|(k, v)| (k, self::aggregate(&v)))
            .collect(),
        quests,
        blockers,
        deaths_by_cause,
        unreachable,
        stuck_heatmap,
        level_curve,
        notes,
        canopy,
        telemetry: ts,
    }
}

fn f(v: Option<f64>, digits: usize) -> String {
    match v {
        Some(x) => format!("{x:.digits$}"),
        None => "-".into(),
    }
}

fn pct(v: Option<f64>) -> String {
    match v {
        Some(x) => format!("{:.0}%", x * 100.0),
        None => "-".into(),
    }
}

/// A readable summary.
pub fn markdown(r: &Report) -> String {
    let a = &r.aggregate;
    let mut s = String::new();
    let _ = writeln!(s, "# {}\n", r.title);
    let _ = writeln!(
        s,
        "{} bots on `{}`, {:.1} bot-minutes (longest session {:.1} min). Thresholds: docs/playtest/fun-metrics.md.\n",
        a.bots, r.server, a.bot_minutes, r.minutes
    );
    let fails = r.checks.iter().filter(|c| c.status == "fail").count();
    let warns = r.checks.iter().filter(|c| c.status == "warn").count();
    let _ = writeln!(s, "## Fun checks: {fails} fail, {warns} warn\n");
    let _ = writeln!(s, "| | Check | Value | Target |\n|---|---|---|---|");
    for c in &r.checks {
        let mark = match c.status.as_str() {
            "pass" => "PASS",
            "warn" => "WARN",
            "fail" => "FAIL",
            _ => "n/a",
        };
        let _ = writeln!(
            s,
            "| {mark} | {} | {} {} | {} |",
            c.what,
            f(c.value, 2),
            c.unit,
            c.target
        );
    }
    let _ = writeln!(s, "\n## Pace\n");
    let _ = writeln!(
        s,
        "- First kill: median {} s; first quest turned in: median {} s; level 2: median {} s ({} of level-1 starters reached it)",
        f(a.median_first_kill_s, 0),
        f(a.median_first_quest_s, 0),
        f(a.median_first_level_s, 0),
        pct(Some(a.share_reached_level_2))
    );
    let _ = writeln!(
        s,
        "- XP per hour: median {} (design pace {:.0}); {} of XP from quests; {} quests turned in, median {} min each",
        f(a.median_xp_per_hour, 0),
        DESIGN_XP_PER_HOUR,
        pct(a.xp_quest_share),
        a.quests_done,
        f(a.median_quest_minutes, 1)
    );
    let _ = writeln!(
        s,
        "- Rewards: median gap {} s; median longest drought {} s; {} of bots had a drought over 5 min",
        f(a.median_reward_gap_s, 0),
        f(a.median_longest_drought_s, 0),
        pct(Some(a.share_drought_over_5min))
    );
    let _ = writeln!(
        s,
        "- Combat: {} fights, {} won, median {} s; {} kills, {} lost to another player's last hit; {} deaths ({:.1}/h)",
        a.fights,
        pct(a.win_rate),
        f(a.median_fight_secs, 1),
        a.kills,
        a.kills_no_credit,
        a.deaths,
        a.deaths_per_hour
    );
    let _ = writeln!(
        s,
        "- Travel: {} blocks per reward; money earned {} c, spent {} c",
        f(a.distance_per_reward, 0),
        a.money_in,
        a.money_out
    );
    let _ = writeln!(
        s,
        "- Server: action round trip p50 {} ms, p95 {} ms; {} disconnects; {:.1} stuck events per bot-hour",
        f(a.latency_p50_ms, 0),
        f(a.latency_p95_ms, 0),
        a.disconnects,
        a.stuck_per_hour
    );
    let _ = writeln!(s, "\n## Where the time goes\n");
    let _ = writeln!(s, "| Activity | Share |\n|---|---|");
    for (k, v) in &a.time_split {
        if *v > 0.0005 {
            let _ = writeln!(s, "| {k} | {:.1}% |", v * 100.0);
        }
    }
    let _ = writeln!(s, "\n## Level curve\n");
    let _ = writeln!(
        s,
        "| Minute | Bots | Median level | Mean XP earned |\n|---|---|---|---|"
    );
    let step = (r.level_curve.len() / 12).max(1);
    for p in r.level_curve.iter().step_by(step) {
        let _ = writeln!(
            s,
            "| {} | {} | {:.1} | {:.0} |",
            p.minute, p.bots, p.median_level, p.mean_xp
        );
    }
    let _ = writeln!(s, "\n## Quests\n");
    let _ = writeln!(
        s,
        "| Quest | Accepted | Done | Median min |\n|---|---|---|---|"
    );
    for q in &r.quests {
        let _ = writeln!(
            s,
            "| {} | {} | {} | {} |",
            q.key,
            q.accepted_by,
            q.done_by,
            f(q.median_minutes, 1)
        );
    }
    let list = |s: &mut String, title: &str, v: &[Count]| {
        if v.is_empty() {
            return;
        }
        let _ = writeln!(s, "\n## {title}\n");
        for c in v.iter().take(25) {
            let _ = writeln!(s, "- {} x{}", c.what, c.count);
        }
    };
    list(
        &mut s,
        "Blockers (bots that could not progress an objective)",
        &r.blockers,
    );
    list(&mut s, "Deaths by cause", &r.deaths_by_cause);
    list(&mut s, "Unreachable NPCs", &r.unreachable);
    if !r.stuck_heatmap.is_empty() {
        let _ = writeln!(
            s,
            "\n## Stuck spots (16-block cells, hearth-relative x,z)\n"
        );
        for c in r.stuck_heatmap.iter().take(12) {
            let _ = writeln!(s, "- cell {:?}: {} events", c.cell, c.count);
            for e in &c.examples {
                let _ = writeln!(s, "  - {e}");
            }
        }
    }
    list(&mut s, "Notes", &r.notes);
    list(
        &mut s,
        "Creatures seen on tree canopies (hearth-relative; out of melee and loot reach)",
        &r.canopy,
    );
    let group = |s: &mut String, title: &str, m: &BTreeMap<String, Aggregate>| {
        let _ = writeln!(s, "\n## By {title}\n");
        let _ = writeln!(
            s,
            "| {title} | Bots | XP/h | Quests | Deaths/h | Win | Reward gap s | Dead time |\n|---|---|---|---|---|---|---|---|"
        );
        for (k, a) in m {
            let dead: f64 = ["idle", "stuck", "wait_spawn", "dead"]
                .iter()
                .map(|x| a.time_split.get(*x).copied().unwrap_or(0.0))
                .sum();
            let _ = writeln!(
                s,
                "| {k} | {} | {} | {} | {:.1} | {} | {} | {:.0}% |",
                a.bots,
                f(a.median_xp_per_hour, 0),
                a.quests_done,
                a.deaths_per_hour,
                pct(a.win_rate),
                f(a.median_reward_gap_s, 0),
                dead * 100.0
            );
        }
    };
    group(&mut s, "brain", &r.by_brain);
    group(&mut s, "class", &r.by_class);
    let _ = writeln!(s, "\n## Bots\n");
    let _ = writeln!(
        s,
        "| Bot | Class | Brain | Min | Level | XP/h | Quests | Kills | Deaths | Gap s | Drought s |\n|---|---|---|---|---|---|---|---|---|---|---|"
    );
    for b in &r.bots {
        let _ = writeln!(
            s,
            "| {} | {} | {} | {:.1} | {}->{} | {:.0} | {}/{} | {} | {} | {} | {:.0} |",
            b.name,
            b.class,
            b.brain,
            b.minutes,
            b.start_level,
            b.level,
            b.xp_per_hour,
            b.quests_done,
            b.quests_accepted,
            b.kills,
            b.deaths,
            f(b.reward_gap_median_s, 0),
            b.reward_gap_longest_s
        );
    }
    s
}

pub fn has_reward(t: &Telemetry, r: Reward) -> bool {
    t.rewards.iter().any(|(_, x)| *x == r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{Death, Fight, QuestRun};

    fn bot(name: &str) -> Telemetry {
        let mut t = Telemetry {
            name: name.into(),
            class: "Warrior".into(),
            brain: "completionist".into(),
            start_level: 1,
            level: 2,
            secs: 600.0,
            ..Default::default()
        };
        t.add_time(Activity::Fight, 200.0);
        t.add_time(Activity::Travel, 300.0);
        t.add_time(Activity::Idle, 100.0);
        for (i, at) in [30.0, 31.0, 90.0, 150.0, 400.0].iter().enumerate() {
            t.reward(*at, if i == 0 { Reward::Kill } else { Reward::Loot });
        }
        t.first_kill = Some(30.0);
        t.first_level = Some(250.0);
        t.first_quest = Some(150.0);
        t.xp_total = 1000;
        t.xp_quests = 600;
        t.xp_kills = 400;
        t.quests.push(QuestRun {
            key: "a".into(),
            accepted: 20.0,
            done: Some(140.0),
        });
        t.quests.push(QuestRun {
            key: "b".into(),
            accepted: 160.0,
            done: None,
        });
        t.fights.push(Fight {
            secs: 10.0,
            won: true,
            ..Default::default()
        });
        t.fights.push(Fight {
            secs: 20.0,
            won: false,
            ..Default::default()
        });
        t.deaths.push(Death {
            cause: "You were slain by a Thornboar.".into(),
            ..Default::default()
        });
        t.abilities.insert("Heroic Blow".into(), 8);
        t.abilities.insert("Battle Cry".into(), 2);
        t.distance = 1000.0;
        t
    }

    #[test]
    fn gaps_merge_bursts_and_count_the_end() {
        let t = bot("a");
        // moments at 30 (30, 31 merged), 90, 150, 400; then 200 s to the end
        assert_eq!(reward_gaps(&t), vec![30.0, 60.0, 60.0, 250.0, 200.0]);
        let s = summarize(&t);
        assert_eq!(s.reward_gap_longest_s, 250.0);
        assert_eq!(s.reward_gap_median_s, Some(60.0));
        assert_eq!(s.quests_done, 1);
        assert_eq!(s.win_rate, Some(0.5));
        assert_eq!(s.abilities_distinct, 2);
        assert!((s.top_ability_share.unwrap() - 0.8).abs() < 1e-9);
        assert!((s.time_split["travel"] - 0.5).abs() < 1e-9);
        assert!((s.xp_per_hour - 6000.0).abs() < 1e-6);
    }

    #[test]
    fn report_aggregates_and_grades() {
        let r = build("test", "local", vec![bot("a"), bot("b")]);
        let a = &r.aggregate;
        assert_eq!(a.bots, 2);
        assert_eq!(a.deaths, 2);
        assert_eq!(a.median_first_kill_s, Some(30.0));
        assert_eq!(a.share_reached_level_2, 1.0);
        let c = |k: &str| r.checks.iter().find(|c| c.key == k).unwrap().status.clone();
        assert_eq!(c("first_kill"), "pass");
        assert_eq!(c("level_2"), "pass");
        // 2 deaths in 20 bot-minutes is 6 an hour
        assert_eq!(c("deaths"), "fail");
        assert_eq!(c("win_rate"), "fail");
        // nobody measured latency: no data, never a pass
        assert_eq!(c("latency"), "no data");
        assert_eq!(r.quests.len(), 2);
        assert_eq!(r.deaths_by_cause[0].count, 2);
        assert!(r.level_curve.len() >= 10);
        let md = markdown(&r);
        assert!(md.contains("## Fun checks"));
        assert!(md.contains("Thornboar"));
        // and it round-trips as JSON
        let j = serde_json::to_string(&r).unwrap();
        let back: Report = serde_json::from_str(&j).unwrap();
        assert_eq!(back.bots.len(), 2);
    }

    #[test]
    fn empty_swarm_is_no_data() {
        let r = build("empty", "x", Vec::new());
        assert!(r.checks.iter().all(|c| c.status == "no data"));
    }
}
