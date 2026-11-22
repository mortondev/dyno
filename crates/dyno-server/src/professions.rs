//! Professions: gathering and crafting skills that rise with use.

use crate::game::State;
use dyno_protocol::{PlayerId, ServerMsg};
use dyno_sim::progression as prog;

impl State {
    pub fn send_skills(&self, id: PlayerId) {
        if let Some(p) = self.players.get(&id) {
            let cap = prog::skill_cap(p.level);
            let mut list: Vec<(String, u16, u16)> =
                p.skills.iter().map(|(k, v)| (k.clone(), *v, cap)).collect();
            list.sort();
            p.send(&ServerMsg::Skills(list));
        }
    }

    /// Can this player use something that needs `need`? Learns a profession
    /// at skill 1 the first time, if nothing more is needed.
    pub fn check_skill(
        &mut self,
        id: PlayerId,
        need: &Option<(String, u16)>,
    ) -> Result<(), String> {
        let Some((prof, n)) = need else { return Ok(()) };
        if !self.rpg() {
            return Ok(());
        }
        let name = self.profession_name(prof);
        let Some(p) = self.players.get_mut(&id) else {
            return Err(String::new());
        };
        let have = p.skills.get(prof).copied().unwrap_or(0);
        if have == 0 && *n <= 1 {
            p.skills.insert(prof.clone(), 1);
            p.send(&ServerMsg::System {
                text: format!("You have learned {name}."),
            });
            self.send_skills(id);
            return Ok(());
        }
        if have < *n {
            return Err(format!("Requires {name} ({n})"));
        }
        Ok(())
    }

    /// After a success: maybe raise the skill.
    pub fn skill_up(&mut self, id: PlayerId, need: &Option<(String, u16)>) {
        let Some((prof, n)) = need else { return };
        if !self.rpg() {
            return;
        }
        let roll = self.rng.f32();
        let name = self.profession_name(prof);
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        let have = p.skills.get(prof).copied().unwrap_or(0);
        let cap = prog::skill_cap(p.level);
        if have == 0 || have >= cap || roll >= prog::skill_up_chance(have, *n) {
            return;
        }
        p.skills.insert(prof.clone(), have + 1);
        p.send(&ServerMsg::System {
            text: format!("Your skill in {name} has increased to {}.", have + 1),
        });
        self.send_skills(id);
    }

    fn profession_name(&self, key: &str) -> String {
        self.content
            .professions
            .iter()
            .find(|p| p.key == key)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| key.to_string())
    }
}

impl State {
    /// Start gathering from a node: a short cast; moving cancels it.
    pub fn gather(&mut self, id: PlayerId, pos: [i32; 3]) {
        let now = std::time::Instant::now();
        let b = self.world.get(pos[0], pos[1], pos[2]);
        let rule = self.content.rule(b).clone();
        let Some(p) = self.players.get(&id) else {
            return;
        };
        if p.dead || p.casting.is_some() || !self.rpg() || rule.respawn == 0 {
            return;
        }
        let eye = [p.pose.pos[0], p.pose.pos[1] + 1.6, p.pose.pos[2]];
        let c = [
            pos[0] as f32 + 0.5,
            pos[1] as f32 + 0.5,
            pos[2] as f32 + 0.5,
        ];
        if crate::game::dist(eye, c) > 7.0 {
            return self.tell(id, "Too far away.");
        }
        if self
            .depleted
            .get(&pos)
            .is_some_and(|t| *t > crate::game::now_secs())
        {
            return self.tell(id, "This has been gathered recently.");
        }
        let tool = rule
            .tool
            .and_then(|k| dyno_sim::inventory::best_tool(&p.pack, &self.content, k))
            .map(|(_, t)| t.clone());
        if let Some(why) = dyno_sim::mining::missing_tool(&self.content, b, tool.as_ref()) {
            return self.tell(id, why);
        }
        if let Err(e) = self.check_skill(id, &rule.skill) {
            return self.tell(id, e);
        }
        let secs = match rule.tool {
            Some(dyno_world::content::ToolKind::Pickaxe) => 2.2,
            _ => 1.6,
        };
        let p = self.players.get_mut(&id).unwrap();
        let from = p.pose.pos;
        p.casting = Some(crate::abilities::Casting {
            ability: dyno_protocol::GATHER,
            target: None,
            end: now + std::time::Duration::from_secs_f32(secs),
            from,
            node: Some(pos),
        });
        self.broadcast(
            &ServerMsg::Casting {
                player: id,
                ability: Some(dyno_protocol::GATHER),
                secs,
            },
            None,
        );
    }

    /// The cast finished: loot into the bags, a skill-up, and the node rests.
    pub fn finish_gather(&mut self, id: PlayerId, pos: [i32; 3]) {
        let b = self.world.get(pos[0], pos[1], pos[2]);
        let rule = self.content.rule(b).clone();
        if rule.respawn == 0 {
            return;
        }
        let Some(p) = self.players.get(&id) else {
            return;
        };
        let tool = rule
            .tool
            .and_then(|k| dyno_sim::inventory::best_tool(&p.pack, &self.content, k))
            .map(|(_, t)| t.clone());
        let drops = dyno_sim::mining::drops(&self.content, b, tool.as_ref(), &mut self.rng);
        let mut lines = Vec::new();
        let p = self.players.get_mut(&id).unwrap();
        let pos_f = p.pose.pos;
        let mut spill = Vec::new();
        for s in drops {
            lines.push(if s.count > 1 {
                format!("{} ×{}", self.content.item(s.item).name, s.count)
            } else {
                self.content.item(s.item).name.to_string()
            });
            if let Some(rest) = p.pack.add(&self.content, s) {
                spill.push(rest);
            }
        }
        p.send(&ServerMsg::Pack(p.pack.slots.clone()));
        if !lines.is_empty() {
            p.send(&ServerMsg::System {
                text: format!("You gather: {}", lines.join(", ")),
            });
        }
        for s in spill {
            self.spawn_item([pos_f[0], pos_f[1] + 1.0, pos_f[2]], s, 0.1);
        }
        self.skill_up(id, &rule.skill);
        let until = crate::game::now_secs() + rule.respawn as u64;
        // ore is worked out (plain stone until it regrows); trees and bushes rest
        if matches!(
            b,
            dyno_world::COAL_ORE | dyno_world::IRON_ORE | dyno_world::EMBERSTONE
        ) {
            self.set_block(pos, dyno_world::STONE);
            self.regrow.push((pos, b, until));
        } else {
            self.depleted.insert(pos, until);
        }
    }
}
