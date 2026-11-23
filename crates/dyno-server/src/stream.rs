//! World streaming and area of interest (docs/design/data-and-streaming.md §4).
//!
//! **Sections.** A client holds only the sections of 16³ cells near it. Each
//! player (and observer) has a [`View`]: the sections sent so far and a
//! token bucket. At the end of every tick the shard sends each view the
//! nearest sections it lacks within the view radius (whole columns), as one
//! `Sections` message, while the bucket allows; sections past the radius
//! plus a margin are dropped with `SectionsGone`. A section counts as held
//! from the moment it is queued, and the connection is one ordered queue, so
//! an edit sent after it can never be overtaken by it. Block edits go only
//! to clients that hold the section (`State::broadcast`).
//!
//! The bucket fills at the join rate (2 MB/s) until the first view is
//! complete, then at the play rate (256 KB/s). A tick's batch is at most a
//! tick's budget, so a game message waits behind at most that much; the
//! `/who` check in `tests/streaming.rs` measures it.
//!
//! **Area of interest.** A player hears about entities within
//! `aoi` metres (with `aoi_keep` more before they are forgotten): the
//! interest pass introduces those that come near (`entity_intro`) and sends
//! `EntityGone` for those that leave. Messages about an entity go only to
//! players who know it, so phasing (which decides what a player may know)
//! still applies on top. Poses go only to players within the same radius,
//! and `PlayersFar` tells a client to stop drawing those who left it. The
//! roster (`PlayerJoined`/`PlayerLeft`, parties, chat) stays world-wide.

use crate::game::State;
use dyno_protocol::{PlayerId, Pose, ServerMsg};
use dyno_world::{SECTION, SectionPos, World};
use std::collections::HashSet;

/// How far the world streams and how far players hear about things.
#[derive(Debug, Clone, Copy)]
pub struct StreamConfig {
    /// view radius, metres horizontally (every section of a column within it)
    pub view: f32,
    /// a section is dropped once its column is this much further than `view`
    pub keep: f32,
    /// area of interest: entities, poses and actions within this many metres
    pub aoi: f32,
    /// forgotten only past `aoi + aoi_keep`
    pub aoi_keep: f32,
    /// bytes per second of sections per player during play
    pub rate: usize,
    /// bytes per second until the first view is complete (the loading screen)
    pub join_rate: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        StreamConfig {
            view: 160.0,
            keep: 24.0,
            aoi: 100.0,
            aoi_keep: 12.0,
            rate: 256 * 1024,
            join_rate: 2 * 1024 * 1024,
        }
    }
}

/// What one client holds of the world, and what it still needs.
#[derive(Debug, Clone, Default)]
pub struct View {
    /// sections sent (and not dropped)
    pub held: HashSet<SectionPos>,
    /// the column the wanted list was made for
    at: Option<[i32; 2]>,
    /// sections to send, farthest first (popped from the end)
    want: Vec<SectionPos>,
    /// bytes this view may send now
    tokens: f64,
    /// the first view is complete (the play rate applies from then on)
    pub filled: bool,
    /// where an observer's view is centred (players use their pose)
    pub center: Option<[f32; 3]>,
}

impl View {
    /// A view for a client that already holds these sections (a seamless move).
    pub fn holding(held: impl IntoIterator<Item = SectionPos>) -> View {
        View {
            held: held.into_iter().collect(),
            filled: true,
            ..Default::default()
        }
    }

    pub fn holds_cell(&self, p: [i32; 3]) -> bool {
        self.held.contains(&dyno_world::section_of(p))
    }

    /// Any held section in the box of cells [a, b].
    pub fn holds_box(&self, a: [i32; 3], b: [i32; 3]) -> bool {
        let (sa, sb) = (dyno_world::section_of(a), dyno_world::section_of(b));
        (sa[1]..=sb[1]).any(|y| {
            (sa[2]..=sb[2]).any(|z| (sa[0]..=sb[0]).any(|x| self.held.contains(&[x, y, z])))
        })
    }

    /// Re-plan if the view moved to another column: what to send, nearest
    /// first, and what to drop.
    fn replan(&mut self, world: &World, at: [f32; 3], cfg: &StreamConfig) -> Vec<SectionPos> {
        let col = [
            (at[0].floor() as i32).div_euclid(SECTION),
            (at[2].floor() as i32).div_euclid(SECTION),
        ];
        if self.at == Some(col) {
            return Vec::new();
        }
        self.at = Some(col);
        let d = world.section_dims();
        // somewhere new (a teleport, a recall, a respawn far away): fill
        // the land round it at the join rate, as at a join
        if (0..d[1]).any(|y| !self.held.contains(&[col[0], y, col[1]])) {
            self.filled = false;
        }
        let dist = |s: [i32; 2]| -> f32 {
            let cx = (s[0] * SECTION + SECTION / 2) as f32;
            let cz = (s[1] * SECTION + SECTION / 2) as f32;
            (cx - at[0]).hypot(cz - at[2])
        };
        let gone: Vec<SectionPos> = self
            .held
            .iter()
            .filter(|s| dist([s[0], s[2]]) > cfg.view + cfg.keep)
            .copied()
            .collect();
        for s in &gone {
            self.held.remove(s);
        }
        let r = (cfg.view / SECTION as f32).ceil() as i32 + 1;
        let fy = at[1] / SECTION as f32;
        let mut want: Vec<(f32, SectionPos)> = Vec::new();
        for sz in (col[1] - r).max(0)..=(col[1] + r).min(d[2] - 1) {
            for sx in (col[0] - r).max(0)..=(col[0] + r).min(d[0] - 1) {
                let h = dist([sx, sz]);
                if h > cfg.view {
                    continue;
                }
                for sy in 0..d[1] {
                    let s = [sx, sy, sz];
                    if !self.held.contains(&s) {
                        // nearest first; the layer you stand in before those above and below
                        let v = ((sy as f32 + 0.5) - fy).abs() * SECTION as f32;
                        want.push((h + v * 0.5, s));
                    }
                }
            }
        }
        want.sort_by(|a, b| b.0.total_cmp(&a.0));
        self.want = want.into_iter().map(|(_, s)| s).collect();
        gone
    }

    /// One tick: the sections to send now (within the budget), and those to drop.
    pub fn pump(
        &mut self,
        world: &World,
        at: [f32; 3],
        dt: f32,
        cfg: &StreamConfig,
    ) -> (Option<ServerMsg>, Option<ServerMsg>) {
        self.pump_with(world, at, dt, cfg, &mut |s| world.encode_section(s))
    }

    /// `pump`, encoding sections with `encode` (a shard's cache).
    pub fn pump_with(
        &mut self,
        world: &World,
        at: [f32; 3],
        dt: f32,
        cfg: &StreamConfig,
        encode: &mut dyn FnMut(SectionPos) -> Vec<u8>,
    ) -> (Option<ServerMsg>, Option<ServerMsg>) {
        let gone = self.replan(world, at, cfg);
        let rate = if self.filled { cfg.rate } else { cfg.join_rate } as f64;
        // a tick's worth, and at most a tenth of a second banked
        self.tokens = (self.tokens + rate * dt as f64).min(rate * 0.1);
        let mut batch = Vec::new();
        while self.tokens > 0.0 {
            let Some(s) = self.want.pop() else { break };
            if !self.held.insert(s) {
                continue;
            }
            let data = encode(s);
            self.tokens -= (data.len() + 16) as f64;
            batch.push((s, data));
        }
        if self.want.is_empty() {
            self.filled = true;
        }
        (
            (!batch.is_empty()).then_some(ServerMsg::Sections(batch)),
            (!gone.is_empty()).then_some(ServerMsg::SectionsGone(gone)),
        )
    }
}

fn hdist(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).hypot(a[2] - b[2])
}

impl State {
    /// Stream sections to every player and observer (end of each tick).
    pub fn stream_sections(&mut self, dt: f32) {
        let cfg = self.stream;
        let spawn = self.spawn_point();
        let world = &self.world;
        // each section is encoded once and kept until it changes: many
        // players want the same sections (a restart has everyone join at once)
        let cache = &mut self.encoded;
        let mut encode = |s: SectionPos| {
            let rev = world.section_rev(s);
            match cache.get(&s) {
                Some((r, d)) if *r == rev => d.clone(),
                _ => {
                    let d = world.encode_section(s);
                    cache.insert(s, (rev, d.clone()));
                    d
                }
            }
        };
        for p in self.players.values_mut() {
            let (sent, gone) = p.view.pump_with(world, p.centre(), dt, &cfg, &mut encode);
            for m in [gone, sent].into_iter().flatten() {
                p.send(&m);
            }
        }
        for o in self.observers.values_mut() {
            let at = o.view.center.unwrap_or(spawn);
            let (sent, gone) = o.view.pump_with(world, at, dt, &cfg, &mut encode);
            for m in [gone, sent].into_iter().flatten() {
                let _ = o.tx.send(std::sync::Arc::new(dyno_protocol::encode(&m)));
            }
        }
    }

    /// Introduce the entities that came near each player, and send
    /// `EntityGone` for those that went out of range (or out of the world).
    pub fn update_interest(&mut self) {
        let cfg = self.stream;
        let ids: Vec<PlayerId> = self.players.keys().copied().collect();
        for id in ids {
            let (pos, phases) = {
                let p = &self.players[&id];
                (p.centre(), p.phases)
            };
            let mut enter = Vec::new();
            let mut leave = Vec::new();
            let mut stale = Vec::new();
            {
                let k = self.players[&id].known.borrow();
                for (eid, e) in &self.entities {
                    let d = hdist(e.pos, pos);
                    let known = k.contains(eid);
                    if !known && d <= cfg.aoi && self.sees(phases, *eid) {
                        enter.push(*eid);
                    } else if known && d > cfg.aoi + cfg.aoi_keep {
                        leave.push(*eid);
                    }
                }
                // gone from the world: EntityGone was broadcast already
                stale.extend(
                    k.iter()
                        .filter(|eid| !self.entities.contains_key(eid))
                        .copied(),
                );
            }
            if enter.is_empty() && leave.is_empty() && stale.is_empty() {
                continue;
            }
            let mut msgs = Vec::new();
            for eid in &enter {
                msgs.extend(self.entity_intro(*eid, &self.entities[eid]));
            }
            let p = self.players.get_mut(&id).unwrap();
            let mut known = p.known.borrow_mut();
            for eid in stale {
                known.remove(&eid);
            }
            for eid in leave {
                if known.remove(&eid) {
                    msgs.push(ServerMsg::EntityGone {
                        id: eid,
                        picked_by: None,
                    });
                }
            }
            known.extend(enter);
            drop(known);
            for m in &msgs {
                p.send(m);
            }
        }
    }

    /// Poses of the players near each player (and `PlayersFar` for those
    /// who left the area); every pose to observers.
    pub fn send_poses(&mut self) {
        if self.players.is_empty() {
            return;
        }
        let cfg = self.stream;
        // the dead send no poses, but still come and go: a body left behind
        // by someone who respawned far away is told to go (PlayersFar)
        let all: Vec<(PlayerId, Pose, bool)> = self
            .players
            .iter()
            .map(|(i, p)| (*i, p.pose, p.dead))
            .collect();
        for (id, p) in self.players.iter_mut() {
            let mut list = Vec::new();
            let mut far = Vec::new();
            for (o, pose, dead) in &all {
                if o == id {
                    continue;
                }
                let d = hdist(pose.pos, p.centre());
                let near = p.near.contains(o);
                if d <= cfg.aoi || (near && d <= cfg.aoi + cfg.aoi_keep) {
                    p.near.insert(*o);
                    if !dead {
                        list.push((*o, *pose));
                    }
                } else if near {
                    p.near.remove(o);
                    far.push(*o);
                }
            }
            // gone from the shard: the roster says so (PlayerLeft)
            p.near.retain(|o| all.iter().any(|(i, ..)| i == o));
            if !far.is_empty() {
                p.send(&ServerMsg::PlayersFar(far));
            }
            if !list.is_empty() {
                p.send(&ServerMsg::Poses(list));
            }
        }
        if !self.observers.is_empty() {
            let all = all
                .into_iter()
                .filter(|(.., dead)| !dead)
                .map(|(i, pose, _)| (i, pose))
                .collect();
            let bytes = std::sync::Arc::new(dyno_protocol::encode(&ServerMsg::Poses(all)));
            for o in self.observers.values() {
                let _ = o.tx.send(bytes.clone());
            }
        }
    }

    /// Whether a player is within the area of interest of a point.
    pub fn hears(&self, id: PlayerId, at: [f32; 3]) -> bool {
        self.players
            .get(&id)
            .is_some_and(|p| hdist(p.centre(), at) <= self.stream.aoi + self.stream.aoi_keep)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Forty players join at once (a restart) on the full world: how long
    /// one tick of section streaming takes while every view fills.
    /// cargo test --release -p dyno-server mass_join -- --ignored --nocapture
    #[test]
    #[ignore]
    fn mass_join_tick_time() {
        let world = World::generate([384, 96, 384], 20261122);
        let cfg = StreamConfig::default();
        let mut views: Vec<(View, [f32; 3])> = (0..40)
            .map(|i| {
                let a = i as f32 * 0.7;
                let at = [192.0 + a.cos() * 120.0, 40.0, 192.0 + a.sin() * 120.0];
                (View::default(), at)
            })
            .collect();
        let (mut worst, mut total, mut ticks, mut bytes) = (0.0f64, 0.0f64, 0, 0usize);
        let mut cache: std::collections::HashMap<SectionPos, Vec<u8>> = Default::default();
        let mut encode = |s: SectionPos| {
            cache
                .entry(s)
                .or_insert_with(|| world.encode_section(s))
                .clone()
        };
        while views.iter().any(|(v, _)| !v.filled) {
            let t = std::time::Instant::now();
            for (v, at) in views.iter_mut() {
                if let (Some(ServerMsg::Sections(l)), _) =
                    v.pump_with(&world, *at, 0.05, &cfg, &mut encode)
                {
                    bytes += l.iter().map(|(_, d)| d.len()).sum::<usize>();
                }
            }
            let ms = t.elapsed().as_secs_f64() * 1e3;
            worst = worst.max(ms);
            total += ms;
            ticks += 1;
        }
        eprintln!(
            "40 views filled in {ticks} ticks ({:.1} s of game time): {} KB sent; \
             a tick of streaming took {:.1} ms on average, {:.1} ms at worst (the tick is 50 ms)",
            ticks as f32 * 0.05,
            bytes / 1024,
            total / ticks as f64,
            worst
        );
    }

    #[test]
    fn a_view_streams_nearest_first_within_budget_and_drops_far_sections() {
        let world = World::generate([256, 48, 256], 3);
        let cfg = StreamConfig {
            view: 48.0,
            keep: 16.0,
            rate: 20_000,
            join_rate: 200_000,
            ..Default::default()
        };
        let mut v = View::default();
        let at = [128.0, 30.0, 128.0];
        let (first, gone) = v.pump(&world, at, 0.05, &cfg);
        assert!(gone.is_none());
        let Some(ServerMsg::Sections(list)) = first else {
            panic!("sections at once")
        };
        // the first is the one you stand in
        assert_eq!(list[0].0, [8, 1, 8]);
        let bytes: usize = list.iter().map(|(_, d)| d.len() + 16).sum();
        assert!(bytes as f64 <= 200_000.0 * 0.05 + 20_000.0, "{bytes}");
        // drain: every column within the radius, whole
        for _ in 0..200 {
            v.pump(&world, at, 0.05, &cfg);
        }
        assert!(v.filled);
        let cols: HashSet<[i32; 2]> = v.held.iter().map(|s| [s[0], s[2]]).collect();
        assert!(cols.contains(&[8, 8]) && cols.contains(&[5, 8]) && !cols.contains(&[3, 8]));
        assert_eq!(v.held.len(), cols.len() * 3);
        // walk far east: the west is dropped, the east streams in
        let (_, gone) = v.pump(&world, [228.0, 30.0, 128.0], 0.05, &cfg);
        let Some(ServerMsg::SectionsGone(gone)) = gone else {
            panic!("far sections are dropped")
        };
        assert!(gone.iter().all(|s| s[0] <= 10));
        assert!(!v.held.iter().any(|s| s[0] < 9));
        // a walk keeps the play rate; a teleport to land not held (a
        // recall, a respawn) fills at the join rate, as a join does
        for _ in 0..200 {
            v.pump(&world, [228.0, 30.0, 128.0], 0.05, &cfg);
        }
        assert!(v.filled);
        let (sent, _) = v.pump(&world, [236.0, 30.0, 128.0], 0.05, &cfg);
        assert!(v.filled, "a step within held land");
        let walked: usize = match sent {
            Some(ServerMsg::Sections(l)) => l.iter().map(|(_, d)| d.len() + 16).sum(),
            _ => 0,
        };
        let (sent, _) = v.pump(&world, [40.0, 30.0, 40.0], 0.05, &cfg);
        assert!(!v.filled, "far away: the burst rate");
        let Some(ServerMsg::Sections(l)) = sent else {
            panic!("the land round the arrival streams at once")
        };
        let burst: usize = l.iter().map(|(_, d)| d.len() + 16).sum();
        assert!(
            burst > walked.max(2000),
            "{burst} after a teleport, {walked} walking"
        );
        assert_eq!(l[0].0, [2, 1, 2], "where you land first");
    }
}
