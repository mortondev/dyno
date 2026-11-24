//! Equipment, the character sheet, auto-attack and vendors.

use crate::game::{State, dist};
use dyno_protocol::{PlayerId, ServerMsg};
use dyno_sim::character::{self, Sheet};
use dyno_sim::inventory::Stack;
use dyno_world::content::{EQUIP_PLACES, RANGED};
use std::time::{Duration, Instant};

/// Melee reach for auto-attack, and the range of a bow.
const MELEE: f32 = 4.5;
const SHOOT: f32 = 30.0;

impl State {
    /// The character sheet of a player with a class.
    pub fn sheet(&self, id: PlayerId) -> Option<Sheet> {
        let p = self.players.get(&id)?;
        let class = self.content.class(p.class?)?;
        Some(character::sheet(
            &self.content,
            class,
            p.level,
            &p.equip,
            &self.buff_stats(id),
        ))
    }

    pub fn send_equipment(&self, id: PlayerId) {
        if let Some(p) = self.players.get(&id) {
            p.send(&ServerMsg::Equipment(p.equip.clone()));
        }
        self.send_wielding(id);
    }

    /// After gear or level changes: keep health and power within the new limits.
    pub fn refresh_sheet(&mut self, id: PlayerId) {
        let max = self.max_health(id);
        if let Some(p) = self.players.get_mut(&id) {
            p.health = p.health.min(max);
            p.send(&p.status());
        }
        self.send_character(id);
        self.send_power(id);
    }

    pub fn equip(&mut self, id: PlayerId, from: u8) {
        let c = &self.content;
        let Some(p) = self.players.get(&id) else {
            return;
        };
        let Some(stack) = p.pack.slots.get(from as usize).copied().flatten() else {
            return;
        };
        let item = c.item(stack.item);
        let Some(g) = item.gear else {
            return self.tell(id, "You cannot wear that.");
        };
        let Some(class) = p.class.and_then(|k| c.class(k)) else {
            return self.tell(id, "Choose a class first.");
        };
        if let Some(why) = character::cannot_equip(class, p.level, &item) {
            return self.tell(id, why);
        }
        let Some((place, off)) = character::equip_place(c, &p.equip, g.slot) else {
            return;
        };
        let p = self.players.get_mut(&id).unwrap();
        p.pack.slots[from as usize] = None;
        let mut back: Vec<Stack> = p.equip[place].take().into_iter().collect();
        // a two-handed weapon frees the off hand; an off-hand item needs a one-handed weapon
        for o in off {
            if let Some(s) = p.equip[o].take() {
                back.push(s);
            }
        }
        p.equip[place] = Some(Stack { count: 1, ..stack });
        if stack.count > 1 {
            p.pack.slots[from as usize] = Some(Stack {
                count: stack.count - 1,
                ..stack
            });
        }
        for s in back {
            if let Some(rest) = p.pack.add(&self.content, s) {
                // no room: put it back on, swap cancelled for this piece
                let pos = p.pose.pos;
                let _ = rest;
                let _ = pos;
            }
        }
        p.send(&ServerMsg::Pack(p.pack.slots.clone()));
        self.send_equipment(id);
        self.refresh_sheet(id);
    }

    pub fn unequip(&mut self, id: PlayerId, place: u8) {
        let c = &self.content;
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        let Some(s) = p.equip.get(place as usize).copied().flatten() else {
            return;
        };
        if p.pack.clone().add(c, s).is_some() {
            return self.tell(id, "Your bags are full.");
        }
        p.equip[place as usize] = None;
        let _ = p.pack.add(c, s);
        p.send(&ServerMsg::Pack(p.pack.slots.clone()));
        self.send_equipment(id);
        self.refresh_sheet(id);
    }

    /// Give a class's starter kit, wearing what can be worn.
    pub fn give_starter(&mut self, id: PlayerId) {
        let Some(class) = self
            .players
            .get(&id)
            .and_then(|p| p.class)
            .and_then(|c| self.content.class(c))
            .cloned()
        else {
            return;
        };
        for (k, n) in &class.starter {
            let Some(item) = self.content.item_by_key(k) else {
                continue;
            };
            let p = self.players.get_mut(&id).unwrap();
            let _ = p.pack.add(&self.content, Stack::new(item, *n));
            if self.content.item(item).gear.is_some()
                && let Some(slot) = p
                    .pack
                    .slots
                    .iter()
                    .position(|s| s.is_some_and(|s| s.item == item))
            {
                self.equip(id, slot as u8);
            }
        }
        if let Some(p) = self.players.get(&id) {
            p.send(&ServerMsg::Pack(p.pack.slots.clone()));
        }
    }

    pub fn set_auto_attack(&mut self, id: PlayerId, target: Option<u32>) {
        let target = target.filter(|t| self.entities.get(t).is_some_and(|e| e.mob.is_some()));
        if let Some(p) = self.players.get_mut(&id)
            && p.auto_target != target
        {
            p.auto_target = target;
            p.send(&ServerMsg::Attacking(target));
        }
    }

    /// Swing (or shoot) at auto-attack targets when the weapon is ready.
    pub fn tick_auto_attacks(&mut self) {
        let now = Instant::now();
        let ids: Vec<PlayerId> = self.players.keys().copied().collect();
        for id in ids {
            let Some(p) = self.players.get(&id) else {
                continue;
            };
            let Some(t) = p.auto_target else { continue };
            if p.dead || p.casting.is_some() || p.swing_at > now {
                continue;
            }
            let Some(e) = self.entities.get(&t).filter(|e| e.mob.is_some()) else {
                self.set_auto_attack(id, None);
                continue;
            };
            let d = dist(e.pos, p.pose.pos);
            let ranged = character::main_weapon(&self.content, &p.equip).is_none()
                && p.equip[RANGED].is_some();
            let bow = p.equip[RANGED]
                .and_then(|s| self.content.item(s.item).gear)
                .and_then(|g| g.weapon);
            let (reach, weapon) = if d > MELEE {
                match bow {
                    Some(b) => (SHOOT, Some(b)),
                    None => (MELEE, None),
                }
            } else {
                (MELEE, None)
            };
            let _ = ranged;
            if d > reach {
                if d > 40.0 {
                    self.set_auto_attack(id, None);
                }
                continue;
            }
            let Some(sheet) = self.sheet(id) else {
                continue;
            };
            let roll = self.rng.f32();
            let (mut amount, speed) = match weapon {
                Some(b) => (
                    b.min + (b.max - b.min) * roll + sheet.attack_power / 14.0 * b.speed,
                    b.speed,
                ),
                None => (character::swing(&sheet, roll), sheet.weapon.2),
            };
            let haste = 1.0 + self.buff_amount(id, dyno_world::content::BuffKind::Haste) / 100.0;
            let speed = speed / haste;
            amount *= 1.0 + self.buff_amount(id, dyno_world::content::BuffKind::Damage) / 100.0;
            let crit = self.rng.f32() * 100.0 < sheet.crit + self.crit_taken(Some(t));
            if crit {
                amount *= 2.0;
            }
            // a seal on the weapon adds to every blow
            amount += self.buff_amount(id, dyno_world::content::BuffKind::Strike);
            self.broadcast(
                &ServerMsg::Act {
                    actor: dyno_protocol::Actor::Player(id),
                    act: if weapon.is_some() {
                        dyno_protocol::Act::Shoot
                    } else {
                        dyno_protocol::Act::Swing
                    },
                },
                Some(id),
            );
            let p = self.players.get_mut(&id).unwrap();
            p.swing_at = now + Duration::from_secs_f32(speed.max(0.8));
            if weapon.is_some() {
                let from = [p.pose.pos[0], p.pose.pos[1] + 1.4, p.pose.pos[2]];
                let to = [e.pos[0], e.pos[1] + 1.0, e.pos[2]];
                self.broadcast(
                    &ServerMsg::Effect {
                        ability: None,
                        color: [0.9, 0.8, 0.6],
                        from,
                        to,
                        radius: 0.0,
                    },
                    None,
                );
            }
            self.players[&id].send(&ServerMsg::Damage {
                entity: Some(t),
                amount,
                crit,
            });
            self.hurt_mob(id, t, amount, 0.0, 0.0);
            self.gain_rage(id, (amount * 0.6).clamp(3.0, 15.0));
        }
    }

    /// Sell a bag slot's stack to a nearby NPC.
    pub fn sell(&mut self, id: PlayerId, from: u8) {
        let near = self.players.get(&id).is_some_and(|p| {
            self.entities.values().any(|e| {
                matches!(e.kind, dyno_protocol::EntityKind::Npc(_)) && dist(e.pos, p.pose.pos) < 6.0
            })
        });
        if !near {
            return self.tell(id, "Find a merchant to sell to.");
        }
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        let Some(s) = p.pack.slots.get(from as usize).copied().flatten() else {
            return;
        };
        let value = self.content.item(s.item).value;
        if value == 0 {
            return self.tell(id, "No merchant wants that.");
        }
        p.pack.slots[from as usize] = None;
        p.money += value * s.count as u32;
        p.send(&ServerMsg::Pack(p.pack.slots.clone()));
        let total = value * s.count as u32;
        p.send(&ServerMsg::Gain {
            xp: 0,
            money: total,
        });
        self.send_character(id);
    }
}

/// Paper-doll place names, for messages.
pub fn place_name(i: usize) -> &'static str {
    EQUIP_PLACES.get(i).map(|p| p.1).unwrap_or("?")
}
