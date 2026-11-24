//! The RPG layer: levels, XP, money, NPCs, quests and resource nodes
//! (docs/design/rpg.md).

use crate::game::{Entity, State, dist};
use dyno_protocol::{EntityKind, Mode, PlayerId, QuestProgress, ServerMsg};
use dyno_sim::inventory::Stack;
use dyno_sim::progression as prog;
use dyno_world::content::Objective;
use dyno_world::{self as w};
use std::time::{Duration, Instant};

/// How close you must stand to talk to an NPC or hand in a quest.
const TALK_RANGE: f32 = 6.0;
/// At most this many quests in the log.
const LOG_SIZE: usize = 20;

impl State {
    /// Is this world run with RPG rules (protected world, no hunger, no item loss)?
    pub fn rpg(&self) -> bool {
        self.content.world.protected
    }

    pub fn max_health(&self, id: PlayerId) -> f32 {
        if !self.rpg() {
            return 20.0;
        }
        if let Some(s) = self.sheet(id) {
            return s.max_health;
        }
        self.players
            .get(&id)
            .map(|p| prog::max_health(p.level))
            .unwrap_or(20.0)
    }

    pub fn send_character(&self, id: PlayerId) {
        let max = self.max_health(id);
        if let Some(p) = self.players.get(&id) {
            p.send(&ServerMsg::Character {
                class: p.class,
                look: p.look,
                level: p.level,
                xp: p.xp,
                xp_next: prog::xp_to_next(p.level),
                money: p.money,
                max_health: max,
            });
        }
    }

    pub fn send_quests(&self, id: PlayerId) {
        if let Some(p) = self.players.get(&id) {
            let mut done: Vec<u16> = p.done.iter().copied().collect();
            done.sort();
            p.send(&ServerMsg::Quests {
                log: p.quests.clone(),
                done,
            });
        }
    }

    /// Give XP and money, with floating text, level-ups and the news.
    pub fn reward(&mut self, id: PlayerId, xp: u32, money: u32) {
        let max_before = self.max_health(id);
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        let ups = prog::gain(&mut p.level, &mut p.xp, xp);
        p.money += money;
        p.send(&ServerMsg::Gain { xp, money });
        let (name, level) = (p.name.clone(), p.level);
        if ups > 0 {
            // the character sheet's maximum (class, stats and gear), not the bare curve
            let max = self.max_health(id);
            let p = self.players.get_mut(&id).unwrap();
            p.health = max; // a level-up heals you fully
            let _ = max_before;
            self.broadcast(&ServerMsg::LevelUp { player: id, level }, None);
            self.event(format!("{name} reached level {level}!"));
            // what the new level teaches, by name, with what it does
            let class = self.players[&id].class;
            let learned: Vec<String> = self
                .content
                .abilities
                .iter()
                .filter(|a| Some(a.class) == class && a.level > level - ups && a.level <= level)
                .map(|a| format!("You have learned {}: {}", a.name, a.text))
                .collect();
            for line in learned {
                self.tell(id, line);
            }
            // a level-up fills your mana too (rage and energy keep their own rules)
            let max_power = self.max_power(id);
            let mana = class
                .and_then(|c| self.content.class(c))
                .is_some_and(|c| c.resource == dyno_world::content::Resource::Mana);
            if mana && let Some(p) = self.players.get_mut(&id) {
                p.power = max_power;
            }
            self.send_power(id);
            let p = &self.players[&id];
            p.send(&p.status());
        }
        self.send_character(id);
    }

    // ---------------- NPCs ----------------

    /// Place every NPC near the spawn hearth (after start or /reload).
    pub fn spawn_npcs(&mut self) {
        let old: Vec<u32> = self
            .entities
            .iter()
            .filter(|(_, e)| matches!(e.kind, EntityKind::Npc(_)))
            .map(|(i, _)| *i)
            .collect();
        for id in old {
            self.entities.remove(&id);
            self.broadcast(
                &ServerMsg::EntityGone {
                    id,
                    picked_by: None,
                },
                None,
            );
        }
        let h = self.spawn_hearth;
        type Placed = (u16, [i32; 3], Option<(u8, bool)>);
        let npcs: Vec<Placed> = self
            .content
            .npcs
            .iter()
            .map(|n| (n.id, n.at, n.phase))
            .collect();
        for (nid, at, phase) in npcs {
            let (x, z) = (h[0] + at[0], h[2] + at[2]);
            let y0 = h[1] + at[1];
            let free = |y: i32| {
                w::is_solid(self.world.get(x, y - 1, z))
                    && !w::is_solid(self.world.get(x, y, z))
                    && self.world.get(x, y, z) != w::WATER
                    && !w::is_solid(self.world.get(x, y + 1, z))
            };
            let y = (0..12)
                .flat_map(|d| [y0 + d, y0 - d])
                .find(|y| free(*y))
                .unwrap_or(y0);
            let pos = [x as f32 + 0.5, y as f32, z as f32 + 0.5];
            // face the hearth
            let yaw = (-(h[0] as f32 + 0.5 - pos[0])).atan2(-(h[2] as f32 + 0.5 - pos[2]));
            let id = self.next_entity;
            self.next_entity += 1;
            let now = Instant::now();
            self.entities.insert(
                id,
                Entity {
                    kind: EntityKind::Npc(nid),
                    pos,
                    vel: [0.0; 3],
                    yaw,
                    born: now,
                    pickup_after: now,
                    moved: false,
                    mob: None,
                    phase,
                },
            );
            self.broadcast(
                &ServerMsg::EntitySpawn {
                    id,
                    kind: EntityKind::Npc(nid),
                    pos,
                    yaw,
                },
                None,
            );
        }
        self.start_routines();
    }

    fn npc_entity(&self, npc: u16) -> Option<(u32, [f32; 3])> {
        self.entities
            .iter()
            .find(|(_, e)| e.kind == EntityKind::Npc(npc))
            .map(|(i, e)| (*i, e.pos))
    }

    fn near_npc(&self, id: PlayerId, npc: u16) -> bool {
        let Some(p) = self.players.get(&id) else {
            return false;
        };
        self.npc_entity(npc)
            .is_some_and(|(_, at)| dist(at, p.pose.pos) <= TALK_RANGE)
    }

    pub fn talk(&mut self, id: PlayerId, entity: u32) {
        let Some(EntityKind::Npc(npc)) = self.entities.get(&entity).map(|e| e.kind) else {
            return;
        };
        if !self.near_npc(id, npc) {
            return;
        }
        // talking may finish a "talk to" objective
        let mut changed = false;
        if let Some(p) = self.players.get_mut(&id) {
            for q in p.quests.iter_mut() {
                let Some(quest) = self.content.quest(q.quest) else {
                    continue;
                };
                for (i, o) in quest.objectives.iter().enumerate() {
                    if let Objective::Talk { npc: n } = o
                        && *n == npc
                        && q.counts[i] == 0
                    {
                        q.counts[i] = 1;
                        changed = true;
                    }
                }
            }
            p.send(&ServerMsg::Dialog { entity, npc });
        }
        if changed {
            self.send_quests(id);
        }
    }

    // ---------------- quests ----------------

    pub fn can_take(&self, id: PlayerId, quest: u16) -> Result<(), String> {
        let p = self.players.get(&id).ok_or("no player")?;
        let q = self.content.quest(quest).ok_or("no such quest")?;
        if p.done.contains(&quest) {
            return Err("You have already done that.".into());
        }
        if p.quests.iter().any(|x| x.quest == quest) {
            return Err("That quest is already in your log.".into());
        }
        if p.quests.len() >= LOG_SIZE {
            return Err("Your quest log is full.".into());
        }
        if p.level + 1 < q.level {
            return Err(format!("You need to be level {} for that.", q.level - 1));
        }
        if !q.requires.iter().all(|r| p.done.contains(r)) {
            return Err("You are not ready for that yet.".into());
        }
        Ok(())
    }

    pub fn accept_quest(&mut self, id: PlayerId, quest: u16) {
        let Some(giver) = self.content.quest(quest).map(|q| q.giver) else {
            return;
        };
        if !self.near_npc(id, giver) {
            return self.tell(id, "Stand closer to talk.");
        }
        if let Err(e) = self.can_take(id, quest) {
            return self.tell(id, e);
        }
        let q = self.content.quest(quest).unwrap();
        let counts = vec![0; q.objectives.len()];
        let title = q.title.clone();
        let p = self.players.get_mut(&id).unwrap();
        p.quests.push(QuestProgress { quest, counts });
        self.tell(id, format!("Quest accepted: {title}"));
        self.update_quests(id);
        self.send_quests(id);
    }

    pub fn abandon_quest(&mut self, id: PlayerId, quest: u16) {
        if let Some(p) = self.players.get_mut(&id) {
            p.quests.retain(|q| q.quest != quest);
        }
        self.send_quests(id);
    }

    /// Recount "collect" and "explore" objectives. Returns true if anything changed.
    pub fn update_quests(&mut self, id: PlayerId) -> bool {
        let h = self.spawn_hearth;
        let Some(p) = self.players.get_mut(&id) else {
            return false;
        };
        let mut changed = false;
        let pos = p.pose.pos;
        for q in p.quests.iter_mut() {
            let Some(quest) = self.content.quest(q.quest) else {
                continue;
            };
            for (i, o) in quest.objectives.iter().enumerate() {
                let now = match o {
                    Objective::Collect { item, count } => {
                        p.pack.count(*item).min(*count as u32) as u8
                    }
                    Objective::Explore { at, radius, .. } => {
                        let c = [
                            (h[0] + at[0]) as f32 + 0.5,
                            (h[1] + at[1]) as f32,
                            (h[2] + at[2]) as f32 + 0.5,
                        ];
                        let reached =
                            ((c[0] - pos[0]).powi(2) + (c[2] - pos[2]).powi(2)).sqrt() <= *radius;
                        if reached || q.counts[i] > 0 { 1 } else { 0 }
                    }
                    _ => continue,
                };
                if q.counts[i] != now {
                    q.counts[i] = now;
                    changed = true;
                }
            }
        }
        changed
    }

    /// A creature died to this player: XP and kill objectives.
    /// Who shares a kill made by `killer` at `at`: them, and their party
    /// members nearby in this shard.
    pub fn credited(&self, killer: PlayerId, at: [f32; 3]) -> Vec<PlayerId> {
        let Some(k) = self.players.get(&killer) else {
            return Vec::new();
        };
        let mut out = vec![killer];
        for (id, p) in &self.players {
            if *id != killer
                && k.party.iter().any(|m| m.eq_ignore_ascii_case(&p.name))
                && crate::game::dist(p.pose.pos, at) < 80.0
            {
                out.push(*id);
            }
        }
        out
    }

    /// Items this player still needs for a collect objective.
    pub fn quest_items_needed(&self, id: PlayerId) -> Vec<u16> {
        let Some(p) = self.players.get(&id) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for q in &p.quests {
            let Some(quest) = self.content.quest(q.quest) else {
                continue;
            };
            for o in &quest.objectives {
                if let Objective::Collect { item, count } = o
                    && p.pack.count(*item) < *count as u32
                {
                    out.push(*item);
                }
            }
        }
        out
    }

    /// Put a quest drop straight into a player's bags (or at their feet if full).
    pub fn give_loot(&mut self, id: PlayerId, st: Stack) {
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        let rest = p.pack.add(&self.content, st);
        let pos = p.pose.pos;
        p.send(&ServerMsg::Pack(p.pack.slots.clone()));
        let name = self.content.item(st.item).name.to_string();
        let text = if st.count > 1 {
            format!("You receive loot: {name} ×{}", st.count)
        } else {
            format!("You receive loot: {name}")
        };
        p.send(&ServerMsg::System { text });
        if let Some(r) = rest {
            self.spawn_item([pos[0], pos[1] + 1.0, pos[2]], r, 0.1);
        }
        if self.update_quests(id) {
            self.send_quests(id);
        }
    }

    pub fn credit_kill(&mut self, id: PlayerId, creature: u16, level: u8, share: f32) {
        if !self.rpg() {
            return;
        }
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        let scale = self.content.world.kill_xp.unwrap_or(1.0).max(0.0);
        let base = (prog::kill_xp(p.level, level) as f32 * share * scale).round() as u32;
        // rested: the same again, out of the bank
        let bonus = base.min(p.rested);
        p.rested -= bonus;
        let xp = base + bonus;
        if bonus > 0 {
            p.send(&ServerMsg::Rested { pool: p.rested });
            if p.rested == 0 {
                p.send(&ServerMsg::System {
                    text: "You feel normal again.".into(),
                });
            }
        }
        let mut changed = false;
        for q in p.quests.iter_mut() {
            let Some(quest) = self.content.quest(q.quest) else {
                continue;
            };
            for (i, o) in quest.objectives.iter().enumerate() {
                if let Objective::Kill { creature: c, count } = o
                    && *c == creature
                    && q.counts[i] < *count
                {
                    q.counts[i] += 1;
                    changed = true;
                }
            }
        }
        if changed {
            self.send_quests(id);
        }
        if xp > 0 {
            self.reward(id, xp, 0);
        }
    }

    fn quest_done(&self, id: PlayerId, quest: u16) -> bool {
        let (Some(p), Some(q)) = (self.players.get(&id), self.content.quest(quest)) else {
            return false;
        };
        p.quests.iter().find(|x| x.quest == quest).is_some_and(|x| {
            q.objectives
                .iter()
                .zip(&x.counts)
                .all(|(o, n)| *n >= o.need())
        })
    }

    pub fn complete_quest(&mut self, id: PlayerId, quest: u16, choice: u8) {
        self.update_quests(id);
        let Some(q) = self.content.quest(quest).cloned() else {
            return;
        };
        if !self.near_npc(id, q.ender) {
            return self.tell(id, "Stand closer to talk.");
        }
        if !self.quest_done(id, quest) {
            return self.tell(id, "That is not finished yet.");
        }
        let c = &self.content;
        let p = self.players.get_mut(&id).unwrap();
        // take the collected items
        for o in &q.objectives {
            if let Objective::Collect { item, count } = o {
                p.pack.remove(*item, *count as u32);
            }
        }
        let mut gifts: Vec<Stack> = q.items.iter().map(|(i, n)| Stack::new(*i, *n)).collect();
        if !q.choice.is_empty() {
            let (i, n) = q.choice[(choice as usize).min(q.choice.len() - 1)];
            gifts.push(Stack::new(i, n));
        }
        let mut spill = Vec::new();
        for g in gifts {
            if let Some(rest) = p.pack.add(c, g) {
                spill.push(rest);
            }
        }
        p.quests.retain(|x| x.quest != quest);
        p.done.insert(quest);
        let level = p.level;
        let pos = p.pose.pos;
        let name = p.name.clone();
        p.send(&ServerMsg::Pack(p.pack.slots.clone()));
        for s in spill {
            self.spawn_item([pos[0], pos[1] + 1.0, pos[2]], s, 0.1);
        }
        self.tell(id, format!("Quest complete: {}", q.title));
        if let Some(w) = &self.webhook {
            let _ = w.send(format!("{name} completed \"{}\"", q.title));
        }
        let xp = prog::quest_xp(q.xp, level, q.level);
        self.reward(id, xp, q.money);
        self.send_quests(id);
        self.refresh_phases(id);
    }

    // ---------------- resource nodes ----------------

    /// Grow back harvested nodes whose time has come (if nobody stands there).
    pub fn tick_regrow(&mut self) {
        let now = crate::game::now_secs();
        self.depleted.retain(|_, t| *t > now);
        let due: Vec<([i32; 3], u8)> = self
            .regrow
            .iter()
            .filter(|(_, _, t)| *t <= now)
            .map(|(p, b, _)| (*p, *b))
            .collect();
        for (pos, b) in due {
            let busy = self.players.values().any(|p| {
                let f = p.pose.pos;
                (f[0].floor() as i32, f[2].floor() as i32) == (pos[0], pos[2])
                    && (pos[1] - f[1].floor() as i32).abs() <= 1
            });
            if busy {
                continue;
            }
            self.regrow.retain(|(p, _, _)| *p != pos);
            let here = self.world.get(pos[0], pos[1], pos[2]);
            if here == w::AIR || here == w::STONE {
                self.set_block_as(pos, b, false);
            }
        }
    }

    /// May this player break this block? (Protected worlds allow only nodes.)
    pub fn may_harvest(&mut self, id: PlayerId, b: u8) -> bool {
        if !self.rpg() {
            return true;
        }
        let Some(p) = self.players.get_mut(&id) else {
            return false;
        };
        if p.mode == Mode::Creative || self.content.rule(b).respawn > 0 {
            return true;
        }
        if p.hint_at.elapsed() > Duration::from_secs(4) {
            p.hint_at = Instant::now();
            p.send(&ServerMsg::System {
                text: "The land here is protected. You can gather from trees, ore and bushes."
                    .into(),
            });
        }
        false
    }
}

/// Build a beacon (a stone pillar with a lantern) at every explore objective
/// that does not have one, so players can see what they are looking for.
/// Returns true if the world changed.
pub fn build_landmarks(
    world: &mut dyno_world::World,
    c: &dyno_world::content::Content,
    hearth: [i32; 3],
) -> bool {
    let mut changed = false;
    for q in &c.quests {
        for o in &q.objectives {
            let Objective::Explore { at, .. } = o else {
                continue;
            };
            let (x, z) = (hearth[0] + at[0], hearth[2] + at[2]);
            if !world.in_bounds([x, 1, z]) || world.find_near(w::LANTERN, x, z, 1).is_some() {
                continue;
            }
            let y = world.surface(x, z);
            for (dx, dz) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
                world.set(x + dx, y, z + dz, 19);
            }
            for k in 1..=4 {
                world.set(x, y + k, z, if k == 4 { 19 } else { w::COBBLE });
            }
            world.set(x, y + 5, z, w::LANTERN);
            changed = true;
        }
    }
    changed
}
