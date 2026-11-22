//! World events: things that happen to everyone nearby, with phases and a
//! shared goal (docs/design/rpg.md, "Dynamic world"). First: Nightfall
//! Incursion, where Shades assault a gate of Hearthstead after dusk.

use crate::game::{State, dist};
use dyno_protocol::{PlayerId, ServerMsg};
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// Gates of Hearthstead, as (name, direction x, direction z).
const GATES: [(&str, f32, f32); 4] = [
    ("north", 0.0, -1.0),
    ("east", 1.0, 0.0),
    ("south", 0.0, 1.0),
    ("west", -1.0, 0.0),
];
const WAVES: u8 = 3;
const WAVE_GAP: Duration = Duration::from_secs(45);

#[derive(Clone, Debug)]
pub struct Incursion {
    pub gate: usize,
    pub wave: u8,
    pub next_wave: Instant,
    pub alive: HashSet<u32>,
    pub killed: u32,
    pub total: u32,
    pub fighters: HashSet<PlayerId>,
    /// started by an operator (runs even by day)
    pub forced: bool,
}

impl State {
    fn town(&self) -> [f32; 3] {
        let h = self.spawn_hearth;
        [h[0] as f32 + 0.5, h[1] as f32, h[2] as f32 + 0.5]
    }

    fn players_near_town(&self) -> usize {
        let t = self.town();
        self.players
            .values()
            .filter(|p| !p.dead && !p.away() && dist(p.pose.pos, t) < 140.0)
            .count()
    }

    /// Called when dusk falls.
    pub fn maybe_start_incursion(&mut self) {
        if self.incursion.is_some() || !self.rpg() || self.players_near_town() == 0 {
            return;
        }
        let Some(_) = self.content.creatures.iter().find(|c| c.key == "shade") else {
            return;
        };
        let gate = self.rng.range(0, 3) as usize;
        let name = GATES[gate].0;
        self.incursion = Some(Incursion {
            gate,
            wave: 0,
            next_wave: Instant::now() + Duration::from_secs(20),
            alive: HashSet::new(),
            killed: 0,
            total: 0,
            fighters: HashSet::new(),
            forced: !self.is_night(),
        });
        self.event(format!(
            "Nightfall: Shades gather at the {name} gate of Hearthstead!"
        ));
        self.send_event_state();
    }

    /// Waves, progress, and the end at dawn.
    pub fn tick_events(&mut self) {
        let Some(mut inc) = self.incursion.take() else {
            return;
        };
        inc.alive.retain(|id| self.entities.contains_key(id));
        let now = Instant::now();
        let night = self.is_night() || self.clock > 0.45 || inc.forced;
        if inc.wave < WAVES && inc.next_wave <= now && night {
            inc.wave += 1;
            inc.next_wave = now + WAVE_GAP;
            let (_, gx, gz) = GATES[inc.gate];
            let t = self.town();
            let at = [t[0] + gx * 44.0, t[1], t[2] + gz * 44.0];
            let n = 2 + inc.wave as usize + self.players_near_town().min(4);
            let shade = self
                .content
                .creatures
                .iter()
                .find(|c| c.key == "shade")
                .map(|c| c.id)
                .unwrap_or(0);
            for k in 0..n {
                let a = k as f32 * 2.4;
                let x = (at[0] + a.cos() * 3.0).floor() as i32;
                let z = (at[2] + a.sin() * 3.0).floor() as i32;
                if !self.world.in_bounds([x, 1, z]) {
                    continue;
                }
                let y = self.world.surface(x, z) + 1;
                let id = self.spawn_creature(shade, [x as f32 + 0.5, y as f32, z as f32 + 0.5]);
                if let Some(m) = self.entities.get_mut(&id).and_then(|e| e.mob.as_mut()) {
                    m.level = 2 + inc.wave;
                    m.event = true;
                    // they march on the town
                    m.home = t;
                }
                inc.alive.insert(id);
                inc.total += 1;
            }
            let wave = inc.wave;
            self.incursion = Some(inc);
            self.event(format!("Wave {wave} of {WAVES} pours out of the dark!"));
            self.send_event_state();
            return;
        }
        // all waves out and all destroyed: victory
        if inc.wave >= WAVES && inc.alive.is_empty() {
            let fighters: Vec<PlayerId> = inc.fighters.iter().copied().collect();
            self.event("The Watch holds! Hearthstead stands through the night.".into());
            for id in fighters {
                let level = self.players.get(&id).map(|p| p.level).unwrap_or(1);
                self.reward(id, 150 + 60 * level as u32, 120);
            }
            self.incursion = None;
            self.send_event_state();
            return;
        }
        // dawn: the Shades fade
        if !night && inc.wave > 0 {
            self.event("Dawn breaks; the Shades withdraw. They will return.".into());
            self.incursion = None;
            self.send_event_state();
            return;
        }
        self.incursion = Some(inc);
    }

    /// An event creature died: count it and remember who fought.
    pub fn event_kill(&mut self, id: u32, by: Option<PlayerId>) {
        if let Some(inc) = self.incursion.as_mut()
            && inc.alive.remove(&id)
        {
            inc.killed += 1;
            if let Some(p) = by {
                inc.fighters.insert(p);
            }
            self.send_event_state();
        }
    }

    /// Tell everyone near town what the event needs now (or that it ended).
    pub fn send_event_state(&self) {
        let msg = match &self.incursion {
            Some(inc) => ServerMsg::WorldEvent {
                title: "Nightfall Incursion".into(),
                text: if inc.wave == 0 {
                    format!("Shades gather at the {} gate. Hold it!", GATES[inc.gate].0)
                } else {
                    format!(
                        "Wave {} of {WAVES}: {} Shades destroyed",
                        inc.wave, inc.killed
                    )
                },
                progress: if inc.total == 0 {
                    0.0
                } else {
                    inc.killed as f32 / (inc.total.max(1) as f32) * inc.wave as f32 / WAVES as f32
                },
            },
            None => ServerMsg::WorldEvent {
                title: String::new(),
                text: String::new(),
                progress: 0.0,
            },
        };
        self.broadcast(&msg, None);
    }
}
