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
