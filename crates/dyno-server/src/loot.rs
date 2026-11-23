//! Corpses you loot by hand, merchants who sell, and rested XP: the classic
//! loop of kill, loot, sell, buy, and coming back rested.

use crate::game::{Entity, State, dist};
use dyno_protocol::{EntityKind, PlayerId, ServerMsg, Stack};
use std::time::{Duration, Instant};

/// How long a corpse with loot stays; an empty one fades sooner.
const CORPSE_LIFE: Duration = Duration::from_secs(180);
const EMPTY_LIFE: Duration = Duration::from_secs(25);
/// After this, anyone may loot a corpse.
const OWNER_LOCK: Duration = Duration::from_secs(60);
const REACH: f32 = 6.0;

#[derive(Clone, Debug)]
pub struct Corpse {
    pub creature: u16,
    /// who may loot it (the killer and their party); anyone after a minute
    pub owners: Vec<PlayerId>,
    pub money: u32,
    pub items: Vec<Stack>,
    pub born: Instant,
    pub until: Instant,
}

impl Corpse {
    fn empty(&self) -> bool {
        self.money == 0 && self.items.is_empty()
    }
}

/// Rested XP: a share of a level per hour away, banked up to one and a half levels.
pub fn rested_gain(xp_next: u32, hours: f32, safe: bool) -> u32 {
    let per_hour = if safe { 0.05 } else { 0.0125 };
    (xp_next as f32 * per_hour * hours.max(0.0)) as u32
}

pub fn rested_cap(xp_next: u32) -> u32 {
    xp_next * 3 / 2
}

impl State {
    /// A creature fell: leave its body, holding what it dropped.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_corpse(
        &mut self,
        pos: [f32; 3],
        yaw: f32,
        creature: u16,
        owners: Vec<PlayerId>,
        money: u32,
        items: Vec<Stack>,
        phase: Option<(u8, bool)>,
    ) {
        let id = self.next_entity;
        self.next_entity += 1;
        let now = Instant::now();
        let c = Corpse {
            creature,
            owners,
            money,
            items,
            born: now,
            until: now,
        };
        let loot = !c.empty();
        let c = Corpse {
            until: now + if loot { CORPSE_LIFE } else { EMPTY_LIFE },
            ..c
        };
        let kind = EntityKind::Corpse { creature, loot };
        self.entities.insert(
            id,
            Entity {
                kind,
                pos,
                vel: [0.0; 3],
                yaw,
                born: now,
                pickup_after: now + Duration::from_secs(3600),
                moved: false,
                mob: None,
                phase,
            },
        );
        self.corpses.insert(id, c);
        self.broadcast(&ServerMsg::EntitySpawn { id, kind, pos, yaw }, None);
    }

    /// Coins a creature carries: most hostile things have a few.
    pub fn roll_money(&mut self, creature: u16, level: u8) -> u32 {
        let hostile = self.content.creature(creature).is_some_and(|c| c.hostile);
        if !hostile || self.rng.f32() > 0.65 {
            return 0;
        }
        level.max(1) as u32 * self.rng.range(2, 7)
    }

    /// Fade old corpses; bank rested XP for players resting in town.
    pub fn tick_loot(&mut self, dt: f32) {
        let now = Instant::now();
        let gone: Vec<u32> = self
            .corpses
            .iter()
            .filter(|(_, c)| c.until <= now)
            .map(|(id, _)| *id)
            .collect();
        for id in gone {
            self.corpses.remove(&id);
            if self.entities.remove(&id).is_some() {
                self.broadcast(
                    &ServerMsg::EntityGone {
                        id,
                        picked_by: None,
                    },
                    None,
                );
            }
        }
        // resting in a town or camp banks rested XP while you are online, too
        self.rested_clock += dt;
        if self.rested_clock >= 60.0 && self.rpg() {
            self.rested_clock = 0.0;
            let ids: Vec<PlayerId> = self.players.keys().copied().collect();
            for id in ids {
                let Some(pos) = self.players.get(&id).map(|p| p.pose.pos) else {
                    continue;
                };
                if !self.in_safe_area(pos) {
                    continue;
                }
                let Some(p) = self.players.get_mut(&id) else {
                    continue;
                };
                let next = dyno_sim::progression::xp_to_next(p.level);
                // a minute's share, carried in fractions so slow levels still bank at 5% an hour
                p.rested_carry += next as f32 * 0.05 / 60.0;
                let add = p.rested_carry.floor() as u32;
                p.rested_carry -= add as f32;
                let pool = (p.rested + add).min(rested_cap(next));
                if pool != p.rested {
                    p.rested = pool;
                    p.send(&ServerMsg::Rested { pool });
                }
            }
        }
    }

    fn may_loot(&self, pid: PlayerId, id: u32) -> Result<(), &'static str> {
        let (Some(p), Some(e), Some(c)) = (
            self.players.get(&pid),
            self.entities.get(&id),
            self.corpses.get(&id),
        ) else {
            return Err("There is nothing to loot.");
        };
        if p.dead {
            return Err("You are dead.");
        }
        if dist(p.pose.pos, e.pos) > REACH || !self.sees(p.phases, id) {
            return Err("You are too far away.");
        }
        if !c.owners.is_empty() && !c.owners.contains(&pid) && c.born.elapsed() < OWNER_LOCK {
            return Err("That is not yours to loot.");
        }
        if c.empty() {
            return Err("There is nothing left.");
        }
        Ok(())
    }

    /// Open a corpse (or, with `all`, take everything straight away).
    pub fn open_loot(&mut self, pid: PlayerId, id: u32, all: bool) {
        if let Err(why) = self.may_loot(pid, id) {
            return self.tell(pid, why);
        }
        if all {
            return self.take_loot(pid, id, 255);
        }
        self.send_loot(pid, id);
    }

    fn send_loot(&self, pid: PlayerId, id: u32) {
        let (money, items) = self
            .corpses
            .get(&id)
            .map(|c| (c.money, c.items.clone()))
            .unwrap_or_default();
        if let Some(p) = self.players.get(&pid) {
            p.send(&ServerMsg::Loot { id, money, items });
        }
    }

    /// Take an item (by slot), the money (254) or everything (255).
    pub fn take_loot(&mut self, pid: PlayerId, id: u32, slot: u8) {
        if let Err(why) = self.may_loot(pid, id) {
            self.tell(pid, why);
            return self.send_loot(pid, id);
        }
        let Some(mut c) = self.corpses.remove(&id) else {
            return;
        };
        let mut got_money = 0;
        if (slot == 254 || slot == 255) && c.money > 0 {
            got_money = c.money;
            c.money = 0;
        }
        let picks: Vec<usize> = if slot == 255 {
            (0..c.items.len()).rev().collect()
        } else if (slot as usize) < c.items.len() {
            vec![slot as usize]
        } else {
            Vec::new()
        };
        let mut got = Vec::new();
        let mut full = false;
        if let Some(p) = self.players.get_mut(&pid) {
            p.money += got_money;
            for i in picks {
                let st = c.items[i];
                match p.pack.add(&self.content, st) {
                    None => {
                        got.push(st);
                        c.items.remove(i);
                    }
                    Some(rest) => {
                        if rest.count < st.count {
                            got.push(Stack::new(st.item, st.count - rest.count));
                        }
                        c.items[i] = rest;
                        full = true;
                    }
                }
            }
            if !got.is_empty() {
                p.send(&ServerMsg::Pack(p.pack.slots.clone()));
            }
            if got_money > 0 {
                p.send(&ServerMsg::Gain {
                    xp: 0,
                    money: got_money,
                });
                let text = format!("You loot {}", dyno_sim::progression::money_text(got_money));
                p.send(&ServerMsg::System { text });
            }
            for st in &got {
                let name = self.content.item(st.item).name.to_string();
                let text = if st.count > 1 {
                    format!("You receive loot: {name} ×{}", st.count)
                } else {
                    format!("You receive loot: {name}")
                };
                p.send(&ServerMsg::System { text });
            }
        }
        if full {
            self.tell(pid, "Your bags are full.");
        }
        if got_money > 0 {
            self.send_character(pid);
        }
        if !got.is_empty() && self.update_quests(pid) {
            self.send_quests(pid);
        }
        let emptied = c.empty();
        if emptied {
            // the body stays a little while, no longer glinting
            c.until = Instant::now() + EMPTY_LIFE;
            if let Some(e) = self.entities.get_mut(&id) {
                e.kind = EntityKind::Corpse {
                    creature: c.creature,
                    loot: false,
                };
                let (kind, pos, yaw) = (e.kind, e.pos, e.yaw);
                self.broadcast(&ServerMsg::EntitySpawn { id, kind, pos, yaw }, None);
            }
        }
        self.corpses.insert(id, c);
        self.send_loot(pid, id);
    }

    /// Buy from a merchant standing near you.
    pub fn buy(&mut self, pid: PlayerId, npc: u32, item: u16, count: u8) {
        let Some(EntityKind::Npc(n)) = self.entities.get(&npc).map(|e| e.kind) else {
            return;
        };
        let near = self
            .entities
            .get(&npc)
            .zip(self.players.get(&pid))
            .is_some_and(|(e, p)| dist(e.pos, p.pose.pos) <= REACH && !p.dead);
        if !near || !self.player_sees(pid, npc) {
            return self.tell(pid, "You are too far away.");
        }
        let Some(price) = self
            .content
            .npcs
            .get(n as usize)
            .and_then(|m| m.sells.iter().find(|(i, _)| *i == item))
            .map(|(_, p)| *p)
        else {
            return;
        };
        // a ride: it takes you there, and nothing goes into your bags
        if let Some(ride) = self
            .content
            .items
            .get(item.wrapping_sub(dyno_world::content::FIRST_ITEM) as usize)
            .and_then(|i| i.ride.clone())
        {
            return self.take_ride(pid, item, price, &ride);
        }
        let max = self.content.item(item).stack.max(1);
        let count = count.clamp(1, max);
        let Some(p) = self.players.get_mut(&pid) else {
            return;
        };
        let affordable = (p.money / price.max(1)).min(count as u32) as u8;
        if affordable == 0 {
            return self.tell(pid, "You cannot afford that.");
        }
        let rest = p.pack.add(&self.content, Stack::new(item, affordable));
        let bought = affordable - rest.map(|r| r.count).unwrap_or(0);
        if bought == 0 {
            return self.tell(pid, "Your bags are full.");
        }
        let cost = price * bought as u32;
        p.money -= cost;
        p.send(&ServerMsg::Pack(p.pack.slots.clone()));
        let name = self.content.item(item).name.to_string();
        let what = if bought > 1 {
            format!("{name} ×{bought}")
        } else {
            name
        };
        let text = format!(
            "You buy {what} for {}.",
            dyno_sim::progression::money_text(cost)
        );
        p.send(&ServerMsg::System { text });
        self.send_character(pid);
        if self.update_quests(pid) {
            self.send_quests(pid);
        }
    }
}

impl State {
    /// A ride bought from a merchant: to a place you have already found.
    fn take_ride(
        &mut self,
        pid: PlayerId,
        item: u16,
        price: u32,
        ride: &dyno_world::content::Ride,
    ) {
        let h = self.spawn_hearth;
        let (x, z) = (h[0] + ride.at[0], h[2] + ride.at[1]);
        let Some(p) = self.players.get(&pid) else {
            return;
        };
        if p.combat_until > Instant::now() {
            return self.tell(pid, "You cannot leave in the middle of a fight.");
        }
        if ride.area.as_ref().is_some_and(|a| !p.discovered.contains(a)) {
            return self.tell(
                pid,
                "You have not found that place yet. Walk there once, and the ride is yours.",
            );
        }
        if p.money < price {
            return self.tell(pid, "You cannot afford that.");
        }
        if !self.world.in_bounds([x, 1, z]) {
            return;
        }
        let to = [
            x as f32 + 0.5,
            (self.world.ground(x, z) + 1) as f32,
            z as f32 + 0.5,
        ];
        let name = self.content.item(item).name.to_string();
        let p = self.players.get_mut(&pid).unwrap();
        p.money -= price;
        p.pose.pos = to;
        p.send(&ServerMsg::Teleport { pos: to });
        p.send(&ServerMsg::System {
            text: format!(
                "{name}: you pay {} and climb aboard.",
                dyno_sim::progression::money_text(price)
            ),
        });
        self.send_character(pid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rested_xp_banks_and_caps() {
        assert_eq!(rested_gain(400, 1.0, true), 20);
        assert_eq!(rested_gain(400, 1.0, false), 5);
        assert_eq!(rested_cap(400), 600);
        assert_eq!(rested_gain(400, -3.0, true), 0);
    }
}
