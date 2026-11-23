//! Classes and abilities: casting, cooldowns, rage and mana, and spell effects
//! (docs/design/rpg.md, "Classes").

use crate::game::{State, dist};
use dyno_protocol::{PlayerId, RECALL, ServerMsg};
use dyno_world::content::{Ability, BuffKind, Resource};
use std::time::{Duration, Instant};

/// The global cooldown after any ability (Rogues: 1 second).
const GCD: f32 = 1.5;
const GCD_FAST: f32 = 1.0;
/// Ticks of damage and healing over time come every this many seconds.
pub const TICK: f32 = 3.0;

/// A timed effect on a player.
#[derive(Clone, Debug)]
pub struct Aura {
    pub ability: u16,
    pub buff: Option<dyno_world::content::Buff>,
    /// the buff's strength as cast (with level and spell power); an absorb
    /// shield counts down from it
    pub amount: f32,
    /// healing per tick
    pub heal: f32,
    pub ticks: u8,
    pub next: Instant,
    pub until: Instant,
}

/// Damage over time on a creature.
#[derive(Clone, Debug)]
pub struct Dot {
    pub from: PlayerId,
    pub ability: u16,
    pub per_tick: f32,
    pub ticks: u8,
    pub next: Instant,
}
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
    /// the resource node being gathered
    pub node: Option<[i32; 3]>,
}

impl State {
    pub fn max_power(&self, id: PlayerId) -> f32 {
        self.sheet(id).map(|s| s.max_power).unwrap_or(0.0)
    }

    pub fn send_power(&self, id: PlayerId) {
        let max = self.max_power(id);
        if let Some(p) = self.players.get(&id) {
            p.send(&ServerMsg::Power {
                value: p.power.min(max),
                max,
            });
        }
    }

    pub fn send_cooldowns(&self, id: PlayerId) {
        let Some(p) = self.players.get(&id) else {
            return;
        };
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
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        if p.class.is_some() {
            return;
        }
        p.class = Some(class);
        p.combo_class = self
            .content
            .class(class)
            .is_some_and(|c| c.resource == Resource::Energy);
        let name = p.name.clone();
        let cname = self
            .content
            .class(class)
            .map(|c| c.name.clone())
            .unwrap_or_default();
        let max = self.max_power(id);
        let rage = self
            .content
            .class(class)
            .is_some_and(|c| c.resource == Resource::Rage);
        let p = self.players.get_mut(&id).unwrap();
        p.power = if rage { 0.0 } else { max.max(100.0) };
        self.give_starter(id);
        self.tell(
            id,
            format!("You are now a {cname}. Your abilities are on keys 1 to 7."),
        );
        self.broadcast(
            &ServerMsg::System {
                text: format!("{name} has become a {cname}."),
            },
            Some(id),
        );
        self.send_character(id);
        self.send_power(id);
    }

    fn fail(&self, id: PlayerId, why: &str) {
        self.tell(id, why);
    }

    /// Start using an ability (or the recall).
    pub fn cast(&mut self, id: PlayerId, ability: u16, target: Option<u32>) {
        let now = Instant::now();
        let Some(p) = self.players.get(&id) else {
            return;
        };
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
            p.casting = Some(Casting {
                ability,
                target: None,
                end: now + Duration::from_secs_f32(RECALL_CAST),
                from,
                node: None,
            });
            p.gcd_until = now + Duration::from_secs_f32(if p.combo_class { GCD_FAST } else { GCD });
            self.broadcast(
                &ServerMsg::Casting {
                    player: id,
                    ability: Some(ability),
                    secs: RECALL_CAST,
                },
                None,
            );
            return;
        }
        let Some(a) = self.content.ability(ability).cloned() else {
            return;
        };
        if Some(a.class) != p.class || a.level > p.level {
            return;
        }
        if p.cooldowns.get(&ability).is_some_and(|t| *t > now) {
            return self.fail(id, "That is not ready yet.");
        }
        if p.power + 0.01 < a.cost {
            let res = p
                .class
                .and_then(|c| self.content.class(c))
                .map(|c| c.resource);
            return self.fail(
                id,
                match res {
                    Some(Resource::Rage) => "Not enough rage.",
                    Some(Resource::Energy) => "Not enough energy.",
                    _ => "Not enough mana.",
                },
            );
        }
        if a.buff.is_some_and(|b| b.kind == BuffKind::Stealth) && p.combat_until > now {
            return self.fail(id, "You cannot hide while in combat.");
        }
        // abilities that hit a target need one in range
        let needs_target = a.range > 0.0
            && (a.damage[0] > 0.0 || a.root > 0.0 || a.stun > 0.0 || a.slow[1] > 0.0 || a.charge);
        let target = if needs_target {
            let Some(t) = target.filter(|t| self.entities.get(t).is_some_and(|e| e.mob.is_some()))
            else {
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
        p.gcd_until = now + Duration::from_secs_f32(if p.combo_class { GCD_FAST } else { GCD });
        if a.cast > 0.0 {
            p.casting = Some(Casting {
                ability,
                target,
                end: now + Duration::from_secs_f32(a.cast),
                from,
                node: None,
            });
            self.broadcast(
                &ServerMsg::Casting {
                    player: id,
                    ability: Some(ability),
                    secs: a.cast,
                },
                None,
            );
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
            let moved = ((p.pose.pos[0] - c.from[0]).powi(2) + (p.pose.pos[2] - c.from[2]).powi(2))
                .sqrt()
                > 0.5;
            if p.dead || moved {
                p.casting = None;
                cancelled.push(*id);
            } else if c.end <= now {
                done.push((*id, p.casting.take().unwrap()));
            }
        }
        for id in cancelled {
            self.broadcast(
                &ServerMsg::Casting {
                    player: id,
                    ability: None,
                    secs: 0.0,
                },
                None,
            );
            self.tell(id, "Interrupted.");
        }
        for (id, c) in done {
            self.broadcast(
                &ServerMsg::Casting {
                    player: id,
                    ability: None,
                    secs: 0.0,
                },
                None,
            );
            if c.ability == dyno_protocol::GATHER {
                if let Some(node) = c.node {
                    self.finish_gather(id, node);
                }
                continue;
            }
            if c.ability == RECALL {
                let to = self.players[&id]
                    .spawn
                    .unwrap_or_else(|| self.spawn_point());
                let p = self.players.get_mut(&id).unwrap();
                p.pose.pos = to;
                p.cooldowns
                    .insert(RECALL, now + Duration::from_secs_f32(RECALL_COOLDOWN));
                p.send(&ServerMsg::Teleport { pos: to });
                self.send_cooldowns(id);
                continue;
            }
            let Some(a) = self.content.ability(c.ability).cloned() else {
                continue;
            };
            // the target must still be there and in range
            let target = c.target.filter(|t| {
                self.entities.get(t).is_some_and(|e| {
                    e.mob.is_some() && dist(e.pos, self.players[&id].pose.pos) <= a.range + 3.0
                })
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
        let my_phases = self.players.get(&id).map(|p| p.phases).unwrap_or(0);
        let now = Instant::now();
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        // finishers spend combo points; with none there is nothing to finish
        let combo = if a.combo < 0 {
            if p.combo == 0 || (target.is_some() && p.combo_on != target) {
                p.send(&ServerMsg::System {
                    text: "No combo points on that target.".into(),
                });
                return;
            }
            std::mem::take(&mut p.combo) as f32
        } else {
            1.0
        };
        p.power = (p.power - a.cost).max(0.0);
        if a.gives > 0.0 {
            p.power = (p.power + a.gives).min(100.0);
        }
        // charge: rush to the target first
        if a.charge
            && let Some(t) = target
            && let Some(e) = self.entities.get(&t)
        {
            let (tp, me) = (e.pos, p.pose.pos);
            let d = dist(tp, me).max(0.1);
            let k = ((d - 1.6) / d).max(0.0);
            let to = [
                me[0] + (tp[0] - me[0]) * k,
                tp[1].max(me[1]),
                me[2] + (tp[2] - me[2]) * k,
            ];
            p.pose.pos = to;
            p.send(&ServerMsg::Teleport { pos: to });
        }
        if a.cooldown > 0.0 {
            p.cooldowns
                .insert(a.id, now + Duration::from_secs_f32(a.cooldown));
        }
        let level = p.level as f32;
        let eye = [p.pose.pos[0], p.pose.pos[1] + 1.4, p.pose.pos[2]];
        let feet = p.pose.pos;
        let yaw = p.pose.yaw;
        let roll = 0.9 + self.rng.f32() * 0.2;
        let sheet = self.sheet(id).unwrap_or_default();
        let spell = a.cast > 0.0 || a.range > 6.0 || a.radius > 0.0;
        let power = if spell {
            sheet.spell_power
        } else {
            sheet.attack_power / 7.0
        };
        let crit_chance = if spell { sheet.spell_crit } else { sheet.crit };
        let crit = self.rng.f32() * 100.0 < crit_chance;
        let weapon = if a.weapon > 0.0 {
            dyno_sim::character::swing(&sheet, self.rng.f32()) * a.weapon
        } else {
            0.0
        };
        let boost = 1.0 + self.buff_amount(id, BuffKind::Damage) / 100.0;
        let dmg = if a.damage[0] > 0.0 || weapon > 0.0 {
            (a.damage[0] + a.damage[1] * (level - 1.0) + power + weapon)
                * roll
                * if crit {
                    if spell { 1.5 } else { 2.0 }
                } else {
                    1.0
                }
                * combo
                * boost
        } else {
            0.0
        };
        let heal = if a.heal[0] > 0.0 {
            a.heal[0] + a.heal[1] * (level - 1.0) + sheet.spell_power
        } else {
            0.0
        };
        // damage: one target, or everything around the target or you
        let mut hits: Vec<u32> = Vec::new();
        if a.radius > 0.0 {
            let center = target
                .and_then(|t| self.entities.get(&t))
                .map(|e| e.pos)
                .unwrap_or(feet);
            hits = self
                .entities
                .iter()
                .filter(|(i, e)| {
                    e.mob.is_some() && dist(e.pos, center) <= a.radius && self.sees(my_phases, **i)
                })
                .map(|(i, _)| *i)
                .collect();
            self.broadcast(
                &ServerMsg::Effect {
                    ability: Some(a.id),
                    color: a.color,
                    from: center,
                    to: center,
                    radius: a.radius,
                },
                None,
            );
        } else if let Some(t) = target {
            hits.push(t);
            let to = self.entities[&t].pos;
            let to = [to[0], to[1] + 1.0, to[2]];
            if a.range > 6.0 {
                self.broadcast(
                    &ServerMsg::Effect {
                        ability: Some(a.id),
                        color: a.color,
                        from: eye,
                        to,
                        radius: 0.0,
                    },
                    None,
                );
            } else {
                self.broadcast(
                    &ServerMsg::Effect {
                        ability: Some(a.id),
                        color: a.color,
                        from: to,
                        to,
                        radius: 1.2,
                    },
                    None,
                );
            }
        }
        let slowed = hits.clone();
        if dmg > 0.0 {
            for t in hits {
                let bonus = self
                    .entities
                    .get(&t)
                    .and_then(|e| e.mob.as_ref())
                    .and_then(|m| self.content.creature(m.creature))
                    .is_some_and(|c| c.burns_in_daylight);
                // an opener: more against a creature that is not fighting you yet
                let unaware = a.opener > 1.0
                    && self
                        .entities
                        .get(&t)
                        .and_then(|e| e.mob.as_ref())
                        .is_some_and(|m| !m.threat.contains_key(&id) && m.target != Some(id));
                let amount = dmg
                    * if bonus { a.shade_bonus } else { 1.0 }
                    * if unaware { a.opener } else { 1.0 };
                if let Some(p) = self.players.get(&id) {
                    p.send(&ServerMsg::Damage {
                        entity: Some(t),
                        amount,
                        crit,
                    });
                }
                self.hurt_mob(id, t, amount, a.stun, a.root);
            }
        } else if let Some(t) = target {
            self.hurt_mob(id, t, 0.0, a.stun, a.root);
        }
        // a slow on everything it hit
        if a.slow[1] > 0.0 {
            let until = now + Duration::from_secs_f32(a.slow[1]);
            for t in &slowed {
                if let Some(m) = self.entities.get_mut(t).and_then(|e| e.mob.as_mut()) {
                    m.slow = (a.slow[0] / 100.0).clamp(0.0, 0.9);
                    m.slowed_until = until;
                }
            }
        }
        // combo points: builders add one to the target
        if a.combo > 0
            && let Some(t) = target
            && let Some(p) = self.players.get_mut(&id)
        {
            if p.combo_on != Some(t) {
                p.combo = 0;
                p.combo_on = Some(t);
            }
            p.combo = (p.combo + a.combo as u8).min(5);
        }
        if a.combo != 0
            && let Some(p) = self.players.get(&id)
        {
            p.send(&ServerMsg::Combo {
                points: p.combo,
                target: p.combo_on,
            });
        }
        // damage over time
        if a.dot[2] > 0.0
            && let Some(t) = target
        {
            let ticks = (a.dot[2] / TICK).round().max(1.0) as u8;
            let total = (a.dot[0] + a.dot[1] * (level - 1.0) + power * 0.6) * combo * boost;
            if let Some(m) = self.entities.get_mut(&t).and_then(|e| e.mob.as_mut()) {
                m.dots.retain(|d| !(d.from == id && d.ability == a.id));
                m.dots.push(Dot {
                    from: id,
                    ability: a.id,
                    per_tick: total / ticks as f32,
                    ticks,
                    next: now + Duration::from_secs_f32(TICK),
                });
            }
            self.hurt_mob(id, t, 0.0, 0.0, 0.0);
        }
        // heal over time, and buffs
        if a.hot[2] > 0.0 || a.buff.is_some() {
            let ticks = (a.hot[2] / TICK).round() as u8;
            let total = a.hot[0] + a.hot[1] * (level - 1.0) + sheet.spell_power;
            let secs = a
                .buff
                .map(|b| b.secs + b.per_combo * combo)
                .unwrap_or(a.hot[2]);
            // shields and seals grow with level and spell power
            let amount = a.buff.map_or(0.0, |b| {
                b.amount
                    + b.per_level * (level - 1.0)
                    + match b.kind {
                        BuffKind::Absorb => sheet.spell_power,
                        BuffKind::Strike => sheet.spell_power * 0.3,
                        _ => 0.0,
                    }
            });
            if let Some(p) = self.players.get_mut(&id) {
                p.auras.retain(|x| x.ability != a.id);
                p.auras.push(Aura {
                    ability: a.id,
                    buff: a.buff,
                    amount,
                    heal: if ticks > 0 { total / ticks as f32 } else { 0.0 },
                    ticks,
                    next: now + Duration::from_secs_f32(TICK),
                    until: now + Duration::from_secs_f32(secs),
                });
            }
            self.send_auras(id);
            self.refresh_sheet(id);
            // a buff with nothing else to show for it still flashes
            if heal <= 0.0 && target.is_none() && a.radius <= 0.0 && a.blink <= 0.0 {
                self.broadcast(
                    &ServerMsg::Effect {
                        ability: Some(a.id),
                        color: a.color,
                        from: feet,
                        to: feet,
                        radius: 1.5,
                    },
                    None,
                );
            }
        }
        if heal > 0.0 {
            let max = self.max_health(id);
            if let Some(p) = self.players.get_mut(&id) {
                let before = p.health;
                p.health = (p.health + heal).min(max);
                p.send(&p.status());
                p.send(&ServerMsg::Healed {
                    amount: p.health - before,
                });
            }
            self.broadcast(
                &ServerMsg::Effect {
                    ability: Some(a.id),
                    color: a.color,
                    from: feet,
                    to: feet,
                    radius: 1.5,
                },
                None,
            );
        }
        if a.blink > 0.0 {
            let (s, c) = yaw.sin_cos();
            let mut to = feet;
            for k in 1..=(a.blink as i32) {
                let t = [feet[0] - s * k as f32, feet[1], feet[2] - c * k as f32];
                let x = t[0].floor() as i32;
                let y = t[1].floor() as i32;
                let z = t[2].floor() as i32;
                let free = !dyno_world::is_solid(self.world.get(x, y, z))
                    && !dyno_world::is_solid(self.world.get(x, y + 1, z));
                if !free {
                    break;
                }
                to = t;
            }
            self.broadcast(
                &ServerMsg::Effect {
                    ability: Some(a.id),
                    color: a.color,
                    from: feet,
                    to,
                    radius: 0.0,
                },
                None,
            );
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
        let Some(e) = self.entities.get_mut(&id) else {
            return;
        };
        let Some(mob) = e.mob.as_mut() else { return };
        // an evading creature takes no damage (it heals on the way home)
        if !mob.take_hit(pid, amount) {
            return;
        }
        *mob.threat.entry(pid).or_insert(0.0) += amount.max(1.0);
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
        self.broadcast(
            &ServerMsg::EntityInfo {
                id,
                level,
                health,
                max,
            },
            None,
        );
        let mut unhidden = false;
        if let Some(p) = self.players.get_mut(&pid) {
            p.combat_until = now + Duration::from_secs(6);
            unhidden = drop_stealth(&mut p.auras);
        }
        if unhidden {
            self.send_auras(pid);
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
            let res = self.players[&id]
                .class
                .and_then(|c| self.content.class(c))
                .map(|c| c.resource);
            let p = self.players.get_mut(&id).unwrap();
            let fighting = p.combat_until > now;
            let before = p.power;
            match res {
                Some(Resource::Rage) if !fighting => p.power = (p.power - 3.0 * dt).max(0.0),
                Some(Resource::Mana) => {
                    let rate = if fighting { 0.005 } else { 0.025 };
                    p.power = (p.power + max * rate * dt).min(max);
                }
                Some(Resource::Energy) => p.power = (p.power + 10.0 * dt).min(max),
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
        let Some(p) = self.players.get(&id) else {
            return;
        };
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

impl State {
    /// Sum of a buff kind on a player (percent or stat points).
    pub fn buff_amount(&self, id: PlayerId, kind: dyno_world::content::BuffKind) -> f32 {
        let now = Instant::now();
        self.players.get(&id).map_or(0.0, |p| {
            p.auras
                .iter()
                .filter(|a| a.until > now)
                .filter(|a| a.buff.is_some_and(|b| b.kind == kind))
                .map(|a| a.amount)
                .sum()
        })
    }

    /// Stat points from buffs.
    pub fn buff_stats(&self, id: PlayerId) -> dyno_world::content::Stats {
        use dyno_world::content::BuffKind as K;
        let f = |k| self.buff_amount(id, k) as i16;
        dyno_world::content::Stats {
            str: f(K::Strength),
            agi: f(K::Agility),
            int: f(K::Intellect),
            sta: f(K::Stamina),
            spi: 0,
        }
    }

    pub fn send_auras(&self, id: PlayerId) {
        let now = Instant::now();
        if let Some(p) = self.players.get(&id) {
            let list = p
                .auras
                .iter()
                .filter(|a| a.until > now)
                .map(|a| (a.ability, (a.until - now).as_secs_f32()))
                .collect();
            p.send(&ServerMsg::Auras(list));
        }
    }

    /// Damage and healing over time; buffs running out.
    pub fn tick_auras(&mut self) {
        let now = Instant::now();
        // creature dots
        let mut hits: Vec<(PlayerId, u32, f32)> = Vec::new();
        for (eid, e) in self.entities.iter_mut() {
            let Some(m) = e.mob.as_mut() else { continue };
            for d in m.dots.iter_mut() {
                if d.ticks > 0 && d.next <= now {
                    d.ticks -= 1;
                    d.next = now + Duration::from_secs_f32(TICK);
                    hits.push((d.from, *eid, d.per_tick));
                }
            }
            m.dots.retain(|d| d.ticks > 0);
        }
        for (pid, eid, amount) in hits {
            if let Some(p) = self.players.get(&pid) {
                p.send(&ServerMsg::Damage {
                    entity: Some(eid),
                    amount,
                    crit: false,
                });
            }
            self.hurt_mob(pid, eid, amount, 0.0, 0.0);
        }
        // player hots and expiring buffs
        let ids: Vec<PlayerId> = self.players.keys().copied().collect();
        for id in ids {
            let max = self.max_health(id);
            let p = self.players.get_mut(&id).unwrap();
            let mut healed = 0.0;
            for a in p.auras.iter_mut() {
                if a.ticks > 0 && a.next <= now {
                    a.ticks -= 1;
                    a.next = now + Duration::from_secs_f32(TICK);
                    healed += a.heal;
                }
            }
            let before = p.auras.len();
            p.auras.retain(|a| a.until > now);
            let expired = p.auras.len() != before;
            if healed > 0.0 && !p.dead {
                let was = p.health;
                p.health = (p.health + healed).min(max);
                p.send(&p.status());
                if p.health > was {
                    p.send(&ServerMsg::Healed {
                        amount: p.health - was,
                    });
                }
            }
            if expired {
                self.send_auras(id);
                self.refresh_sheet(id);
            }
        }
    }
}

/// Dealing or taking damage ends stealth. Returns true if an aura went.
pub fn drop_stealth(auras: &mut Vec<Aura>) -> bool {
    let before = auras.len();
    auras.retain(|a| a.buff.is_none_or(|b| b.kind != BuffKind::Stealth));
    auras.len() != before
}
