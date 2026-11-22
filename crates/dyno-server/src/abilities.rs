//! Classes and abilities: casting, cooldowns, rage and mana, and spell effects
//! (docs/design/rpg.md, "Classes").

use crate::game::{State, dist};
use dyno_protocol::{PlayerId, RECALL, ServerMsg};
use dyno_world::content::{Ability, Resource};
use std::time::{Duration, Instant};

/// The global cooldown after any ability.
const GCD: f32 = 1.2;
/// Recall: cast time and cooldown.
const RECALL_CAST: f32 = 8.0;
const RECALL_COOLDOWN: f32 = 300.0;

/// A cast in progress.
#[derive(Clone, Debug)]
pub struct Casting {
    pub ability: u16,
    pub target: Option<u32>,
    pub end: Instant,
    /// where the caster stood; moving away cancels the cast
    pub from: [f32; 3],
}

impl State {
    pub fn max_power(&self, id: PlayerId) -> f32 {
        let Some(p) = self.players.get(&id) else { return 0.0 };
        match p.class.and_then(|c| self.content.class(c)).map(|c| c.resource) {
            Some(Resource::Rage) => 100.0,
            Some(Resource::Mana) => 100.0 + 15.0 * (p.level as f32 - 1.0),
            None => 0.0,
        }
    }

    pub fn send_power(&self, id: PlayerId) {
        let max = self.max_power(id);
        if let Some(p) = self.players.get(&id) {
            p.send(&ServerMsg::Power { value: p.power.min(max), max });
        }
    }

    pub fn send_cooldowns(&self, id: PlayerId) {
        let Some(p) = self.players.get(&id) else { return };
        let now = Instant::now();
        let list = p
            .cooldowns
            .iter()
            .filter(|(_, t)| **t > now)
            .map(|(a, t)| (*a, (*t - now).as_secs_f32()))
            .collect();
        let global = p.gcd_until.saturating_duration_since(now).as_secs_f32();
        p.send(&ServerMsg::Cooldowns { list, global });
    }

    pub fn choose_class(&mut self, id: PlayerId, class: u16) {
        if self.content.class(class).is_none() {
            return;
        }
        let Some(p) = self.players.get_mut(&id) else { return };
        if p.class.is_some() {
            return;
        }
        p.class = Some(class);
        let name = p.name.clone();
        let cname = self.content.class(class).map(|c| c.name.clone()).unwrap_or_default();
        let max = self.max_power(id);
        let rage = self.content.class(class).is_some_and(|c| c.resource == Resource::Rage);
        let p = self.players.get_mut(&id).unwrap();
        p.power = if rage { 0.0 } else { max };
        self.tell(id, format!("You are now a {cname}. Your abilities are on keys 1 to 7."));
        self.broadcast(&ServerMsg::System { text: format!("{name} has become a {cname}.") }, Some(id));
        self.send_character(id);
        self.send_power(id);
    }

    fn fail(&self, id: PlayerId, why: &str) {
        self.tell(id, why);
    }

    /// Start using an ability (or the recall).
    pub fn cast(&mut self, id: PlayerId, ability: u16, target: Option<u32>) {
        let now = Instant::now();
        let Some(p) = self.players.get(&id) else { return };
        if p.dead || p.casting.is_some() || p.gcd_until > now {
            return;
        }
        if ability == RECALL {
            if p.cooldowns.get(&RECALL).is_some_and(|t| *t > now) {
                return self.fail(id, "Recall is not ready yet.");
            }
            if p.combat_until > now {
                return self.fail(id, "You cannot recall while in combat.");
            }
            let from = p.pose.pos;
            let p = self.players.get_mut(&id).unwrap();
            p.casting = Some(Casting { ability, target: None, end: now + Duration::from_secs_f32(RECALL_CAST), from });
            p.gcd_until = now + Duration::from_secs_f32(GCD);
            self.broadcast(&ServerMsg::Casting { player: id, ability: Some(ability), secs: RECALL_CAST }, None);
            return;
        }
        let Some(a) = self.content.ability(ability).cloned() else { return };
        if Some(a.class) != p.class || a.level > p.level {
            return;
        }
        if p.cooldowns.get(&ability).is_some_and(|t| *t > now) {
            return self.fail(id, "That is not ready yet.");
        }
        if p.power + 0.01 < a.cost {
            let res = p.class.and_then(|c| self.content.class(c)).map(|c| c.resource);
            return self.fail(id, if res == Some(Resource::Rage) { "Not enough rage." } else { "Not enough mana." });
        }
        // abilities that hit a target need one in range
        let needs_target = a.range > 0.0 && (a.damage[0] > 0.0 || a.root > 0.0 || a.stun > 0.0);
        let target = if needs_target {
            let Some(t) = target.filter(|t| self.entities.get(t).is_some_and(|e| e.mob.is_some())) else {
                return self.fail(id, "You have no target.");
            };
            let e = &self.entities[&t];
            if dist(e.pos, p.pose.pos) > a.range + 1.0 {
                return self.fail(id, "Out of range.");
            }
            Some(t)
        } else {
            None
        };
        let from = p.pose.pos;
        let p = self.players.get_mut(&id).unwrap();
        p.gcd_until = now + Duration::from_secs_f32(GCD);
        if a.cast > 0.0 {
            p.casting = Some(Casting { ability, target, end: now + Duration::from_secs_f32(a.cast), from });
            self.broadcast(&ServerMsg::Casting { player: id, ability: Some(ability), secs: a.cast }, None);
            self.send_cooldowns(id);
        } else {
            self.resolve(id, &a, target);
        }
    }

    /// Advance casts: cancel them if the caster moved, finish them when due.
    pub fn tick_casts(&mut self) {
        let now = Instant::now();
        let mut done = Vec::new();
        let mut cancelled = Vec::new();
        for (id, p) in self.players.iter_mut() {
            let Some(c) = &p.casting else { continue };
            let moved = ((p.pose.pos[0] - c.from[0]).powi(2) + (p.pose.pos[2] - c.from[2]).powi(2)).sqrt() > 0.5;
            if p.dead || moved {
                p.casting = None;
                cancelled.push(*id);
            } else if c.end <= now {
                done.push((*id, p.casting.take().unwrap()));
            }
        }
        for id in cancelled {
            self.broadcast(&ServerMsg::Casting { player: id, ability: None, secs: 0.0 }, None);
            self.tell(id, "Interrupted.");
        }
        for (id, c) in done {
            self.broadcast(&ServerMsg::Casting { player: id, ability: None, secs: 0.0 }, None);
            if c.ability == RECALL {
                let to = self.players[&id].spawn.unwrap_or_else(|| self.spawn_point());
                let p = self.players.get_mut(&id).unwrap();
                p.pose.pos = to;
                p.cooldowns.insert(RECALL, now + Duration::from_secs_f32(RECALL_COOLDOWN));
                p.send(&ServerMsg::Teleport { pos: to });
                self.send_cooldowns(id);
                continue;
            }
            let Some(a) = self.content.ability(c.ability).cloned() else { continue };
            // the target must still be there and in range
            let target = c.target.filter(|t| {
                self.entities.get(t).is_some_and(|e| e.mob.is_some() && dist(e.pos, self.players[&id].pose.pos) <= a.range + 3.0)
            });
            if c.target.is_some() && target.is_none() {
                self.tell(id, "Your target is gone.");
                continue;
            }
            self.resolve(id, &a, target);
        }
    }

    /// Apply an ability's effects and pay for it.
    fn resolve(&mut self, id: PlayerId, a: &Ability, target: Option<u32>) {
        let now = Instant::now();
        let Some(p) = self.players.get_mut(&id) else { return };
        p.power = (p.power - a.cost).max(0.0);
        if a.cooldown > 0.0 {
            p.cooldowns.insert(a.id, now + Duration::from_secs_f32(a.cooldown));
        }
        let level = p.level as f32;
        let eye = [p.pose.pos[0], p.pose.pos[1] + 1.4, p.pose.pos[2]];
        let feet = p.pose.pos;
        let yaw = p.pose.yaw;
        let roll = 0.9 + self.rng.f32() * 0.2;
        let dmg = (a.damage[0] + a.damage[1] * (level - 1.0)) * roll;
        let heal = a.heal[0] + a.heal[1] * (level - 1.0);
        // damage: one target, or everything around the target or you
        let mut hits: Vec<u32> = Vec::new();
        if a.radius > 0.0 {
            let center = target.and_then(|t| self.entities.get(&t)).map(|e| e.pos).unwrap_or(feet);
            hits = self
                .entities
                .iter()
                .filter(|(_, e)| e.mob.is_some() && dist(e.pos, center) <= a.radius)
                .map(|(i, _)| *i)
                .collect();
            self.broadcast(&ServerMsg::Effect { color: a.color, from: center, to: center, radius: a.radius }, None);
        } else if let Some(t) = target {
            hits.push(t);
            let to = self.entities[&t].pos;
            let to = [to[0], to[1] + 1.0, to[2]];
            if a.range > 6.0 {
                self.broadcast(&ServerMsg::Effect { color: a.color, from: eye, to, radius: 0.0 }, None);
            } else {
                self.broadcast(&ServerMsg::Effect { color: a.color, from: to, to, radius: 1.2 }, None);
            }
        }
        if dmg > 0.0 {
            for t in hits {
                let bonus = self
                    .entities
                    .get(&t)
                    .and_then(|e| e.mob.as_ref())
                    .and_then(|m| self.content.creature(m.creature))
                    .is_some_and(|c| c.burns_in_daylight);
                let amount = dmg * if bonus { a.shade_bonus } else { 1.0 };
                self.hurt_mob(id, t, amount, a.stun, a.root);
            }
        } else if let Some(t) = target {
            self.hurt_mob(id, t, 0.0, a.stun, a.root);
        }
        if heal > 0.0 {
            let max = self.max_health(id);
            if let Some(p) = self.players.get_mut(&id) {
                p.health = (p.health + heal).min(max);
                p.send(&p.status());
            }
            self.broadcast(&ServerMsg::Effect { color: a.color, from: feet, to: feet, radius: 1.5 }, None);
        }
        if a.blink > 0.0 {
            let (s, c) = yaw.sin_cos();
            let mut to = feet;
            for k in 1..=(a.blink as i32) {
                let t = [feet[0] - s * k as f32, feet[1], feet[2] - c * k as f32];
                let x = t[0].floor() as i32;
                let y = t[1].floor() as i32;
                let z = t[2].floor() as i32;
                let free = !dyno_world::is_solid(self.world.get(x, y, z)) && !dyno_world::is_solid(self.world.get(x, y + 1, z));
                if !free {
                    break;
                }
                to = t;
            }
            self.broadcast(&ServerMsg::Effect { color: a.color, from: feet, to, radius: 0.0 }, None);
            if let Some(p) = self.players.get_mut(&id) {
                p.pose.pos = to;
                p.casting = None;
                p.send(&ServerMsg::Teleport { pos: to });
            }
        }
        self.send_power(id);
        self.send_cooldowns(id);
    }

    /// Damage a creature on behalf of a player, with an optional stun and root.
    pub fn hurt_mob(&mut self, pid: PlayerId, id: u32, amount: f32, stun: f32, root: f32) {
        let now = Instant::now();
        let Some(e) = self.entities.get_mut(&id) else { return };
        let Some(mob) = e.mob.as_mut() else { return };
        mob.health -= amount;
        mob.last_hit = Some((pid, now));
        mob.target = Some(pid);
        mob.flee = 5.0;
        if let Some(p) = self.players.get(&pid) {
            mob.flee_from = p.pose.pos;
        }
        if stun > 0.0 {
            mob.stunned_until = now + Duration::from_secs_f32(stun);
        }
        if root > 0.0 {
            mob.rooted_until = now + Duration::from_secs_f32(root);
        }
        let (level, health, max) = (mob.level, mob.health.max(0.0), mob.max_health);
        let dead = mob.health <= 0.0;
        if amount > 0.0 {
            self.broadcast(&ServerMsg::EntityHurt { id }, None);
        }
        self.broadcast(&ServerMsg::EntityInfo { id, level, health, max }, None);
        if let Some(p) = self.players.get_mut(&pid) {
            p.combat_until = now + Duration::from_secs(6);
        }
        if dead {
            self.kill_mob_by(id);
        }
    }

    /// Rage fades out of combat; mana refills (slowly in combat).
    pub fn tick_power(&mut self, dt: f32) {
        let now = Instant::now();
        let ids: Vec<PlayerId> = self.players.keys().copied().collect();
        for id in ids {
            let max = self.max_power(id);
            let res = self.players[&id].class.and_then(|c| self.content.class(c)).map(|c| c.resource);
            let p = self.players.get_mut(&id).unwrap();
            let fighting = p.combat_until > now;
            let before = p.power;
            match res {
                Some(Resource::Rage) if !fighting => p.power = (p.power - 3.0 * dt).max(0.0),
                Some(Resource::Mana) => {
                    let rate = if fighting { 0.006 } else { 0.03 };
                    p.power = (p.power + max * rate * dt).min(max);
                }
                _ => {}
            }
            p.power_clock += dt;
            if p.power != before && p.power_clock > 0.5 {
                p.power_clock = 0.0;
                self.send_power(id);
            }
        }
    }

    /// Rage from dealing or taking a blow.
    pub fn gain_rage(&mut self, id: PlayerId, amount: f32) {
        let is_rage = self
            .players
            .get(&id)
            .and_then(|p| p.class)
            .and_then(|c| self.content.class(c))
            .is_some_and(|c| c.resource == Resource::Rage);
        if !is_rage {
            return;
        }
        if let Some(p) = self.players.get_mut(&id) {
            p.power = (p.power + amount).min(100.0);
        }
        self.send_power(id);
    }

    /// Eat the most filling food in your bags.
    pub fn eat_best(&mut self, id: PlayerId) {
        let Some(p) = self.players.get(&id) else { return };
        let Some(slot) = dyno_sim::inventory::best_food(&p.pack, &self.content) else {
            return self.fail(id, "You have nothing to eat.");
        };
        if p.combat_until > Instant::now() {
            return self.fail(id, "You cannot eat in combat.");
        }
        self.players.get_mut(&id).unwrap().held = slot as u8;
        self.use_food(id);
    }
}
