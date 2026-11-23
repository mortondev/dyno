//! A text interface for AI playtesters: one command per line in, a compact
//! description of the situation out. The body keeps living between
//! commands (and defends itself), so a slow reader is not killed for it.
//!
//! Positions are shown as x,z blocks from the spawn hearth (north is -z).

use crate::agent::{Agent, Deal, Outcome, Target, Task};
use crate::body::{Body, TICK};
use crate::brain::{Nodes, Personality, Scripted, stand_for};
use dyno_protocol::{ClientMsg, EntityKind};
use dyno_world::content::{Objective, Resource};
use glam::Vec3;
use std::io::{BufRead, Write};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::{Duration, Instant};

pub struct Options {
    pub addr: String,
    pub name: String,
    pub class: Option<String>,
    pub script: Option<std::path::PathBuf>,
}

pub const HELP: &str = "\
Commands (names match by prefix or part; #id picks an entity; x,z is blocks from the hearth):
  look | quests | bags | help         what is around / your quest log / your bags
  goto <npc|creature|area|quest|x,z|home>   walk there (stops if attacked)
  talk <npc>                          talk: shows their quests and wares
  accept <quest|all>                  accept (walks to the giver)
  turnin <quest> [choice#|item]       hand in (walks to the ender)
  attack [creature|#id]               fight until it ends (auto rotation)
  cast <ability> [target]             use one ability
  loot                                loot the nearest corpse you may loot
  gather <item>                       gather from the nearest node that gives it
  buy <item> [n] | sell junk | sell <item>   trade with the nearest merchant
  equip <item|best> | eat | rest | recall | respawn
  say <text>                          chat (or a /command)
  wait <seconds>                      let time pass
  auto <minutes>                      let a scripted player play for you
  quit";

/// What a command is doing while it runs.
enum Running {
    Task {
        what: String,
        interrupt_on_hurt: bool,
        started: Instant,
        xp0: u32,
        lvl0: u8,
    },
    Auto {
        brain: Box<Scripted>,
        until: Instant,
    },
}

pub struct Session {
    pub a: Agent,
    nodes: Option<Nodes>,
    /// nodes we gathered (they rest for a few minutes)
    gathered: std::collections::HashMap<[i32; 3], Instant>,
    printed: f64,
    running: Option<Running>,
    defend: bool,
    /// a finished command's report, held until the server's answers are in
    pending: Option<(String, Instant, Option<u16>)>,
}

fn lc(s: &str) -> String {
    s.to_lowercase()
}

/// Does `name` match what the user typed (prefix of a word, or a substring)?
fn matches(name: &str, q: &str) -> bool {
    let (n, q) = (lc(name), lc(q.trim()));
    !q.is_empty()
        && (n == q
            || n.contains(&q)
            || n.replace(['_', ' ', '\''], "")
                .contains(&q.replace(['_', ' ', '\''], "")))
}

impl Session {
    pub fn new(body: Body) -> Session {
        let mut a = Agent::new(body, "play");
        let settle = Instant::now();
        while settle.elapsed() < Duration::from_millis(1500) {
            a.update();
            std::thread::sleep(Duration::from_secs_f32(TICK));
        }
        a.baseline();
        let nodes = Some(Nodes::scan(&a.body.world, &a.body.content));
        Session {
            a,
            nodes,
            gathered: Default::default(),
            printed: 0.0,
            running: None,
            defend: true,
            pending: None,
        }
    }

    fn rel(&self, p: Vec3) -> (i32, i32) {
        (
            p.x.floor() as i32 - self.a.body.hearth[0],
            p.z.floor() as i32 - self.a.body.hearth[2],
        )
    }

    fn whereis(&self, p: Vec3) -> String {
        let (x, z) = self.rel(p);
        format!("{} ({x},{z})", self.a.where_is(p))
    }

    // ---------------- finding things ----------------

    fn find_npc(&self, q: &str) -> Option<u16> {
        let c = &self.a.body.content;
        c.npcs
            .iter()
            .filter(|n| matches(&n.name, q) || matches(&n.key, q) || matches(&n.title, q))
            .min_by(|x, y| {
                let d = |n: u16| {
                    self.a
                        .body
                        .npc_pos(n)
                        .map(|p| self.a.body.dist(p))
                        .unwrap_or(1e9)
                };
                d(x.id).partial_cmp(&d(y.id)).unwrap()
            })
            .map(|n| n.id)
    }

    fn find_creature_kind(&self, q: &str) -> Option<u16> {
        self.a
            .body
            .content
            .creatures
            .iter()
            .find(|c| matches(&c.name, q) || matches(&c.key, q))
            .map(|c| c.id)
    }

    /// A live creature: by #id, by name (nearest), or the nearest hostile.
    fn find_creature(&self, q: &str) -> Option<u32> {
        let b = &self.a.body;
        if let Some(id) = q
            .trim()
            .strip_prefix('#')
            .and_then(|s| s.parse::<u32>().ok())
        {
            return b.ents.get(&id).map(|e| e.id);
        }
        let kind = if q.trim().is_empty() {
            None
        } else {
            Some(self.find_creature_kind(q)?)
        };
        b.creatures()
            .filter(|e| match kind {
                Some(k) => e.creature() == Some(k),
                None => e
                    .creature()
                    .and_then(|c| b.content.creature(c))
                    .is_some_and(|c| c.hostile),
            })
            .min_by(|x, y| b.dist3(x.pos).partial_cmp(&b.dist3(y.pos)).unwrap())
            .map(|e| e.id)
    }

    fn find_quest(&self, q: &str) -> Option<u16> {
        let c = &self.a.body.content;
        // prefer quests in the log, then ones we can take
        let ok = |id: u16| self.a.body.quests.iter().any(|x| x.quest == id);
        c.quests
            .iter()
            .filter(|x| matches(&x.key, q) || matches(&x.title, q))
            .max_by_key(|x| (ok(x.id), self.a.can_take(x.id)))
            .map(|x| x.id)
    }

    fn find_item(&self, q: &str, in_bags: bool) -> Option<u16> {
        let b = &self.a.body;
        if in_bags {
            return b
                .pack
                .iter()
                .flatten()
                .map(|s| s.item)
                .find(|i| matches(b.content.item(*i).name, q));
        }
        let c = &b.content;
        c.items
            .iter()
            .find(|i| matches(&i.name, q) || matches(&i.key, q))
            .map(|i| i.id)
            .or_else(|| dyno_world::block_by_key(&lc(q).replace(' ', "_")).map(|b| b as u16))
    }

    fn find_area(&self, q: &str) -> Option<Vec3> {
        let b = &self.a.body;
        b.content
            .areas
            .iter()
            .find(|a| matches(&a.name, q) || matches(&a.key, q))
            .map(|a| b.rel2(a.at))
    }

    /// Where the next objective of a quest is, and what it is.
    fn objective_target(&self, q: u16) -> Option<(Vec3, String)> {
        let b = &self.a.body;
        let quest = b.content.quest(q)?;
        if self.a.quest_ready(q) {
            return b
                .npc_pos(quest.ender)
                .map(|p| (p, format!("turn in to {}", npc_name(b, quest.ender))));
        }
        let p = b.quests.iter().find(|x| x.quest == q)?;
        for (o, n) in quest.objectives.iter().zip(&p.counts) {
            match o {
                Objective::Kill { creature, count } if n < count => {
                    return self
                        .creature_where(*creature)
                        .map(|p| (p, format!("kill {}", cname(b, *creature))));
                }
                Objective::Collect { item, count } if b.count(*item) < *count as u32 => {
                    // a creature that drops it, else a node
                    let drop = b
                        .content
                        .creatures
                        .iter()
                        .filter(|c| c.drops.iter().any(|d| d.item == *item && d.chance >= 0.1))
                        .filter_map(|c| self.creature_where(c.id).map(|p| (p, c.name.clone())))
                        .min_by(|x, y| b.dist(x.0).partial_cmp(&b.dist(y.0)).unwrap());
                    // a node to gather it from
                    let me = b.pos();
                    let node = self.nodes.as_ref().and_then(|ns| {
                        ns.by_block
                            .iter()
                            .filter(|(blk, _)| {
                                let r = b.content.rule(**blk);
                                r.drops.iter().any(|d| d.item == *item)
                                    || (r.drops.is_empty() && **blk as u16 == *item)
                            })
                            .flat_map(|(blk, v)| {
                                v.iter()
                                    .filter(move |c| b.world.get(c[0], c[1], c[2]) == *blk)
                            })
                            .map(|c| Vec3::new(c[0] as f32 + 0.5, c[1] as f32, c[2] as f32 + 0.5))
                            .filter(|p| {
                                p.y >= crate::body::ground(&b.world, p.x as i32, p.z as i32) as f32
                                    - 4.0
                            })
                            .min_by(|x, y| x.distance(me).partial_cmp(&y.distance(me)).unwrap())
                    });
                    let name = b.content.item(*item).name;
                    let by_drop = drop.map(|(p, n)| (p, format!("{name} (dropped by {n})")));
                    let by_node = node.map(|p| {
                        (
                            p,
                            format!("gather {name} (use: gather {})", name.to_lowercase()),
                        )
                    });
                    return match (by_drop, by_node) {
                        (Some(d), Some(n)) => Some(if b.dist(n.0) < b.dist(d.0) { n } else { d }),
                        (d, n) => d.or(n),
                    };
                }
                Objective::Talk { npc } if *n == 0 => {
                    return b
                        .npc_pos(*npc)
                        .map(|p| (p, format!("talk to {}", npc_name(b, *npc))));
                }
                Objective::Explore { name, at, .. } if *n == 0 => {
                    return Some((b.rel(*at), format!("explore {name}")));
                }
                _ => {}
            }
        }
        None
    }

    /// The nearest live creature of a kind, else its nearest camp.
    fn creature_where(&self, creature: u16) -> Option<Vec3> {
        let b = &self.a.body;
        let me = b.pos();
        b.creatures()
            .filter(|e| e.creature() == Some(creature))
            .map(|e| e.pos)
            .chain(
                b.content
                    .areas
                    .iter()
                    .flat_map(|a| a.camps.iter())
                    .filter(|c| c.creature == creature)
                    .map(|c| b.rel2(c.at)),
            )
            .min_by(|x, y| x.distance(me).partial_cmp(&y.distance(me)).unwrap())
    }

    // ---------------- describing ----------------

    pub fn status_line(&self) -> String {
        let b = &self.a.body;
        let class = b.class.and_then(|c| b.content.class(c));
        let res = match class.map(|c| c.resource) {
            Some(Resource::Rage) => "Rage",
            Some(Resource::Energy) => "Energy",
            Some(Resource::Mana) => "Mana",
            None => "Power",
        };
        format!(
            "{} lv{} {} | HP {:.0}/{:.0} | {res} {:.0}/{:.0} | XP {}/{} | {} | rested {} | bags {}/36",
            b.name,
            b.level,
            class.map(|c| c.name.as_str()).unwrap_or("(no class)"),
            b.health,
            b.max_health,
            b.power,
            b.max_power,
            b.xp,
            b.xp_next,
            money(b.money),
            b.rested,
            36 - b.free_slots()
        )
    }

    pub fn describe(&self) -> String {
        let b = &self.a.body;
        let mut s = String::new();
        let (x, z) = self.rel(b.pos());
        let hour = ((6.0 + b.clock() * 24.0) % 24.0) as u32;
        let zone = if b.zone.0.is_empty() {
            "the wilds"
        } else {
            &b.zone.0
        };
        s += &format!(
            "== {zone}{} lv{}-{} | {:02}:00 {} | at {x},{z} (from hearth; north is -z)\n",
            if b.zone.2 { " (safe)" } else { "" },
            b.zone.1[0],
            b.zone.1[1],
            hour,
            if b.is_night() { "NIGHT" } else { "day" },
        );
        s += &self.status_line();
        s += "\n";
        if let Some(d) = &b.dead {
            s += &format!("YOU ARE DEAD ({d}). Type 'respawn'.\n");
        }
        if let Some((t, text)) = &b.world_event {
            s += &format!("World event: {t}: {text}\n");
        }
        if let Some(t) = b.attacking.and_then(|t| b.ents.get(&t)) {
            s += &format!(
                "Fighting: {} lv{} {:.0}/{:.0} {}\n",
                ent_name(b, t),
                t.level,
                t.health,
                t.max,
                self.a.where_is(t.pos)
            );
        }
        // quests
        s += &format!("Quests ({}/20):\n", b.quests.len());
        if b.quests.is_empty() {
            s += "  (none: look for NPCs marked !)\n";
        }
        for q in &b.quests {
            let key = self.a.qkey(q.quest);
            let line = self.a.quest_line(q.quest);
            let goal = match self.objective_target(q.quest) {
                Some((p, what)) => format!(" -> {what} {}", self.whereis(p)),
                None => String::new(),
            };
            let ready = if self.a.quest_ready(q.quest) {
                " READY"
            } else {
                ""
            };
            s += &format!("  [{key}]{ready} {line}{goal}\n");
        }
        // NPCs
        let mut npcs: Vec<(f32, String)> = Vec::new();
        for n in &b.content.npcs {
            let Some(e) = b.npc_ent(n.id) else { continue };
            let d = b.dist(e.pos);
            let offers: Vec<String> = b
                .content
                .quests
                .iter()
                .filter(|q| q.giver == n.id && self.a.can_take(q.id))
                .map(|q| q.key.clone())
                .collect();
            let turnins: Vec<String> = b
                .quests
                .iter()
                .filter(|q| {
                    b.content.quest(q.quest).is_some_and(|x| x.ender == n.id)
                        && self.a.quest_ready(q.quest)
                })
                .map(|q| self.a.qkey(q.quest))
                .collect();
            if d > 70.0 && offers.is_empty() && turnins.is_empty() {
                continue;
            }
            let mark = if !turnins.is_empty() {
                "?"
            } else if !offers.is_empty() {
                "!"
            } else {
                " "
            };
            let mut line = format!("  {mark} {} <{}> {}", n.name, n.title, self.whereis(e.pos));
            if !turnins.is_empty() {
                line += &format!(" turn in: {}", turnins.join(", "));
            }
            if !offers.is_empty() {
                line += &format!(" offers: {}", offers.join(", "));
            }
            if !n.sells.is_empty() {
                line += " [vendor]";
            }
            npcs.push((d, line));
        }
        npcs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        if !npcs.is_empty() {
            s += "NPCs:\n";
            for (_, l) in npcs.iter().take(8) {
                s += l;
                s += "\n";
            }
        }
        // creatures
        let wanted: Vec<u16> = b
            .quests
            .iter()
            .filter_map(|q| b.content.quest(q.quest))
            .flat_map(|q| q.objectives.iter())
            .filter_map(|o| match o {
                Objective::Kill { creature, .. } => Some(*creature),
                _ => None,
            })
            .collect();
        let mut cs: Vec<&crate::body::Ent> =
            b.creatures().filter(|e| b.dist(e.pos) < 60.0).collect();
        cs.sort_by(|x, y| b.dist(x.pos).partial_cmp(&b.dist(y.pos)).unwrap());
        if !cs.is_empty() {
            s += &format!("Creatures within 60m ({}):\n", cs.len());
            for e in cs.iter().take(7) {
                let c = e.creature().and_then(|c| b.content.creature(c));
                let hostile = c.is_some_and(|c| c.hostile);
                let quest = e.creature().is_some_and(|c| wanted.contains(&c));
                s += &format!(
                    "  #{} {} lv{} {:.0}/{:.0} {}{}{}\n",
                    e.id,
                    ent_name(b, e),
                    e.level,
                    e.health,
                    e.max,
                    self.a.where_is(e.pos),
                    if hostile { " hostile" } else { "" },
                    if quest { " [quest]" } else { "" }
                );
            }
        }
        let corpses: Vec<String> = b
            .ents
            .values()
            .filter(|e| {
                matches!(e.kind, EntityKind::Corpse { loot: true, .. }) && b.dist(e.pos) < 25.0
            })
            .map(|e| {
                let mine = b.my_corpses.iter().any(|(id, _)| *id == e.id);
                format!(
                    "{} #{} {}{}",
                    ent_name(b, e),
                    e.id,
                    self.a.where_is(e.pos),
                    if mine { " (yours)" } else { "" }
                )
            })
            .collect();
        if !corpses.is_empty() {
            s += &format!("Lootable corpses: {}\n", corpses.join("; "));
        }
        let players: Vec<String> = b
            .players
            .values()
            .filter(|(_, p)| b.dist(*p) < 40.0)
            .map(|(n, p)| format!("{n} {}", self.a.where_is(*p)))
            .collect();
        if !players.is_empty() {
            s += &format!("Players near: {}\n", players.join(", "));
        }
        s += &format!("Bags: {}\n", self.bags_short());
        s
    }

    fn bags_short(&self) -> String {
        let b = &self.a.body;
        let mut items: Vec<(u16, u32)> = Vec::new();
        for st in b.pack.iter().flatten() {
            match items.iter_mut().find(|(i, _)| *i == st.item) {
                Some(x) => x.1 += st.count as u32,
                None => items.push((st.item, st.count as u32)),
            }
        }
        let junk: u32 = b
            .junk_slots()
            .iter()
            .filter_map(|i| b.pack[*i])
            .map(|s| b.content.item(s.item).value * s.count as u32)
            .sum();
        let list: Vec<String> = items
            .iter()
            .map(|(i, n)| {
                let it = b.content.item(*i);
                let up = if it.gear.is_some() && b.is_upgrade(*i) {
                    " (UPGRADE)"
                } else {
                    ""
                };
                if *n > 1 {
                    format!("{} x{n}{up}", it.name)
                } else {
                    format!("{}{up}", it.name)
                }
            })
            .collect();
        format!("{} | junk worth {}", list.join(", "), money(junk))
    }

    fn quests_full(&self) -> String {
        let b = &self.a.body;
        let mut s = String::new();
        for q in &b.quests {
            let Some(quest) = b.content.quest(q.quest) else {
                continue;
            };
            s += &format!(
                "[{}] {} (lv{})\n  {}\n  {}\n",
                quest.key,
                quest.title,
                quest.level,
                short(&quest.text, 200),
                self.a.quest_line(q.quest)
            );
            if let Some((p, what)) = self.objective_target(q.quest) {
                s += &format!("  next: {what} {}\n", self.whereis(p));
            }
            s += &format!(
                "  ender: {} | reward: {}\n",
                npc_name(b, quest.ender),
                reward_text(b, quest.id)
            );
        }
        let avail: Vec<String> = b
            .content
            .quests
            .iter()
            .filter(|q| self.a.can_take(q.id))
            .map(|q| format!("{} (lv{}, from {})", q.key, q.level, npc_name(b, q.giver)))
            .collect();
        s += &format!(
            "Available to accept: {}\n",
            if avail.is_empty() {
                "none".into()
            } else {
                avail.join(", ")
            }
        );
        s += &format!("Done: {}\n", b.done.len());
        s
    }

    fn bags_full(&self) -> String {
        let b = &self.a.body;
        let mut s = String::new();
        for (i, st) in b.pack.iter().enumerate() {
            if let Some(st) = st {
                let it = b.content.item(st.item);
                let mut extra = Vec::new();
                if it.food > 0 {
                    extra.push(format!("food {}", it.food));
                }
                if it.value > 0 {
                    extra.push(format!("sells {}", money(it.value)));
                }
                if let Some(g) = it.gear {
                    extra.push(format!("{:?}", g.slot).to_lowercase());
                    if b.is_upgrade(st.item) {
                        extra.push("UPGRADE".into());
                    }
                }
                s += &format!(
                    "  {i:2}: {} x{} {}\n",
                    it.name,
                    st.count,
                    if extra.is_empty() {
                        String::new()
                    } else {
                        format!("({})", extra.join(", "))
                    }
                );
            }
        }
        let worn: Vec<String> = b
            .equip
            .iter()
            .flatten()
            .map(|st| b.content.item(st.item).name.to_string())
            .collect();
        s += &format!(
            "Wearing: {}\n",
            if worn.is_empty() {
                "nothing".into()
            } else {
                worn.join(", ")
            }
        );
        let abil: Vec<String> = b
            .class
            .map(|c| b.content.known_abilities(c, b.level))
            .unwrap_or_default()
            .iter()
            .map(|a| format!("{} ({:.0})", a.name, a.cost))
            .collect();
        s += &format!("Abilities: {}\n", abil.join(", "));
        s
    }

    fn npc_text(&self, npc: u16) -> String {
        let b = &self.a.body;
        let Some(n) = b.content.npc(npc) else {
            return String::new();
        };
        let mut s = format!(
            "{} <{}>: \"{}\"\n",
            n.name,
            n.title,
            short(&n.greeting, 220)
        );
        for q in b
            .content
            .quests
            .iter()
            .filter(|q| q.giver == npc && self.a.can_take(q.id))
        {
            s += &format!(
                "  offers [{}] {} (lv{}): {}\n    objectives: {} | reward: {}\n",
                q.key,
                q.title,
                q.level,
                short(&q.text, 240),
                objectives_text(b, q.id),
                reward_text(b, q.id)
            );
        }
        for q in b
            .quests
            .iter()
            .filter(|q| b.content.quest(q.quest).is_some_and(|x| x.ender == npc))
        {
            let ready = self.a.quest_ready(q.quest);
            s += &format!(
                "  {} [{}]{}\n",
                if ready {
                    "ready to turn in:"
                } else {
                    "in progress:"
                },
                self.a.qkey(q.quest),
                if ready {
                    format!(" choices: {}", choices_text(b, q.quest))
                } else {
                    String::new()
                }
            );
        }
        if !n.sells.is_empty() {
            let wares: Vec<String> = n
                .sells
                .iter()
                .map(|(i, p)| format!("{} {}", b.content.item(*i).name, money(*p)))
                .collect();
            s += &format!("  sells: {}\n", wares.join(", "));
        }
        s
    }

    /// New log lines since the last print.
    fn recent(&mut self) -> String {
        let mut s = String::new();
        let lines: Vec<String> = self
            .a
            .log
            .iter()
            .filter(|(t, _)| *t > self.printed)
            .map(|(_, l)| l.clone())
            .collect();
        if let Some((t, _)) = self.a.log.back() {
            self.printed = *t;
        }
        let n = lines.len();
        if n > 0 {
            s += "Events:\n";
            for l in lines.iter().skip(n.saturating_sub(14)) {
                s += &format!("  {l}\n");
            }
        }
        s
    }

    // ---------------- commands ----------------

    fn start(&mut self, t: Task, what: &str, interrupt: bool) {
        self.a.set(t);
        self.running = Some(Running::Task {
            what: what.into(),
            interrupt_on_hurt: interrupt,
            started: Instant::now(),
            xp0: self.a.tel.xp_total as u32,
            lvl0: self.a.body.level,
        });
    }

    /// Start a command. Returns text to print now (when it finishes at once).
    pub fn command(&mut self, line: &str) -> Option<String> {
        let line = line.trim();
        let (cmd, rest) = line.split_once(' ').unwrap_or((line, ""));
        let rest = rest.trim();
        let b = &self.a.body;
        let lcmd = lc(cmd);
        if b.dead.is_some()
            && !matches!(
                lcmd.as_str(),
                "respawn"
                    | "release"
                    | "look"
                    | "l"
                    | "quests"
                    | "q"
                    | "log"
                    | "bags"
                    | "inv"
                    | "i"
                    | "help"
                    | "say"
                    | "wait"
                    | "probe"
            )
        {
            return Some(
                "You are dead. Type 'respawn' to return to the hearth (you keep everything)."
                    .into(),
            );
        }
        match lcmd.as_str() {
            "" => Some(String::new()),
            "help" | "?" => Some(HELP.into()),
            "look" | "l" => Some(String::new()),
            "quests" | "q" | "log" => Some(self.quests_full()),
            "bags" | "inv" | "i" => Some(self.bags_full()),
            "goto" | "go" | "walk" => {
                let (to, label, range) = match self.resolve_place(rest) {
                    Ok(x) => x,
                    Err(e) => return Some(e),
                };
                self.start(
                    Task::Goto {
                        to,
                        range,
                        why: label.clone(),
                    },
                    &format!("goto {label}"),
                    true,
                );
                None
            }
            "talk" | "t" => {
                let Some(npc) = self.find_npc(rest) else {
                    return Some(format!("No NPC matches '{rest}'."));
                };
                self.start(
                    Task::talk(npc, Vec::new()),
                    &format!("talk {}", npc_name(b, npc)),
                    true,
                );
                None
            }
            "accept" => {
                let ids: Vec<u16> = if lc(rest) == "all" {
                    // everything offered by the nearest NPC who has something
                    let me = b.pos();
                    let giver = b
                        .content
                        .quests
                        .iter()
                        .filter(|q| self.a.can_take(q.id))
                        .filter_map(|q| b.npc_pos(q.giver).map(|p| (q.giver, p.distance(me))))
                        .min_by(|x, y| x.1.partial_cmp(&y.1).unwrap())
                        .map(|x| x.0);
                    b.content
                        .quests
                        .iter()
                        .filter(|q| Some(q.giver) == giver && self.a.can_take(q.id))
                        .map(|q| q.id)
                        .collect()
                } else {
                    self.find_quest(rest).into_iter().collect()
                };
                let Some(first) = ids.first().and_then(|q| b.content.quest(*q)) else {
                    return Some(format!("No quest to accept matches '{rest}'."));
                };
                if !self.a.can_take(first.id) {
                    return Some(format!(
                        "You cannot take [{}] now (done, in your log, level or prerequisites).",
                        first.key
                    ));
                }
                let giver = first.giver;
                let deals = ids.iter().map(|q| Deal::Accept(*q)).collect();
                self.start(
                    Task::talk(giver, deals),
                    &format!("accept at {}", npc_name(b, giver)),
                    true,
                );
                None
            }
            "turnin" | "complete" | "handin" => {
                let mut parts: Vec<&str> = rest.split_whitespace().collect();
                let choice_arg = if parts.len() > 1 { parts.pop() } else { None };
                let qname = parts.join(" ");
                let Some(q) = self.find_quest(&qname) else {
                    return Some(format!("No quest matches '{qname}'."));
                };
                if !self.a.quest_ready(q) {
                    return Some(format!(
                        "[{}] is not finished: {}",
                        self.a.qkey(q),
                        self.a.quest_line(q)
                    ));
                }
                let quest = b.content.quest(q)?;
                let choice = match choice_arg {
                    Some(c) => c
                        .parse::<u8>()
                        .ok()
                        .map(|n| n.saturating_sub(1))
                        .or_else(|| {
                            quest
                                .choice
                                .iter()
                                .position(|(i, _)| matches(b.content.item(*i).name, c))
                                .map(|p| p as u8)
                        }),
                    None => None,
                }
                .unwrap_or_else(|| self.a.best_choice(q));
                let ender = quest.ender;
                self.start(
                    Task::talk(ender, vec![Deal::Complete(q, choice)]),
                    &format!("turn in at {}", npc_name(b, ender)),
                    true,
                );
                None
            }
            "attack" | "fight" | "kill" | "a" => {
                let target = if rest.is_empty() {
                    b.attacking.or_else(|| self.find_creature(""))
                } else {
                    self.find_creature(rest)
                };
                let Some(id) = target else {
                    return Some(format!("Nothing to attack matches '{rest}'."));
                };
                let e = b.ents.get(&id)?;
                let (c, l) = (e.creature().unwrap_or(0), e.level);
                let label = format!("fight {} #{id}", ent_name(b, e));
                self.start(Task::fight(id, c, l), &label, false);
                None
            }
            "cast" | "c" => {
                let mut words: Vec<&str> = rest.split_whitespace().collect();
                let known: Vec<dyno_world::content::Ability> = b
                    .class
                    .map(|c| b.content.known_abilities(c, b.level))
                    .unwrap_or_default()
                    .into_iter()
                    .cloned()
                    .collect();
                // the longest prefix of words that names an ability
                let mut found = None;
                for n in (1..=words.len()).rev() {
                    let q = words[..n].join(" ");
                    if let Some(a) = known
                        .iter()
                        .find(|a| matches(&a.name, &q) || matches(&a.key, &q))
                    {
                        found = Some(a.clone());
                        words.drain(..n);
                        break;
                    }
                }
                let Some(ab) = found else {
                    let names: Vec<String> = known.iter().map(|a| a.name.clone()).collect();
                    return Some(format!("You know: {}", names.join(", ")));
                };
                let target = if words.is_empty() {
                    b.attacking.or_else(|| self.find_creature(""))
                } else {
                    self.find_creature(&words.join(" "))
                };
                let needs =
                    ab.range > 0.0 && (ab.damage[0] > 0.0 || ab.root > 0.0 || ab.stun > 0.0);
                self.a.body.send(ClientMsg::Cast {
                    ability: ab.id,
                    target: if needs { target } else { None },
                });
                *self.a.tel.abilities.entry(ab.name.clone()).or_insert(0) += 1;
                self.start(
                    Task::wait(0.6 + ab.cast, false),
                    &format!("cast {}", ab.name),
                    false,
                );
                None
            }
            "loot" => {
                let me = b.pos();
                let mine = b
                    .my_corpses
                    .iter()
                    .rev()
                    .filter_map(|(id, _)| b.ents.get(id))
                    .find(|e| {
                        matches!(e.kind, EntityKind::Corpse { loot: true, .. })
                            && e.pos.distance(me) < 30.0
                    })
                    .map(|e| e.id);
                let corpse = if let Some(id) =
                    rest.strip_prefix('#').and_then(|s| s.parse::<u32>().ok())
                {
                    Some(id)
                } else if mine.is_some() {
                    mine
                } else {
                    b.ents
                        .values()
                        .filter(|e| {
                            matches!(e.kind, EntityKind::Corpse { loot: true, .. })
                                && e.pos.distance(me) < 30.0
                        })
                        .min_by(|x, y| x.pos.distance(me).partial_cmp(&y.pos.distance(me)).unwrap())
                        .map(|e| e.id)
                };
                let Some(id) = corpse else {
                    return Some("No lootable corpse within 30m.".into());
                };
                self.start(
                    Task::Loot {
                        corpse: id,
                        sent: None,
                    },
                    "loot",
                    true,
                );
                None
            }
            "gather" | "g" => {
                let Some(item) = self.find_item(rest, false) else {
                    return Some(format!("No item matches '{rest}'."));
                };
                let name = b.content.item(item).name.to_string();
                match self.gather_node(item) {
                    Some((node, stand)) => {
                        self.start(
                            Task::Gather {
                                node,
                                stand: Some(stand),
                                sent: None,
                            },
                            &format!("gather {name}"),
                            true,
                        );
                        None
                    }
                    None => Some(format!("No node that gives {name} in reach.")),
                }
            }
            "buy" => {
                let mut parts: Vec<&str> = rest.split_whitespace().collect();
                let n = parts.last().and_then(|s| s.parse::<u8>().ok());
                if n.is_some() {
                    parts.pop();
                }
                let q = parts.join(" ");
                let me = b.pos();
                let offer = b
                    .content
                    .npcs
                    .iter()
                    .flat_map(|n| n.sells.iter().map(move |(i, p)| (n.id, *i, *p)))
                    .filter(|(_, i, _)| matches(b.content.item(*i).name, &q))
                    .min_by(|x, y| {
                        let d = |n: u16| b.npc_pos(n).map(|p| p.distance(me)).unwrap_or(1e9);
                        d(x.0).partial_cmp(&d(y.0)).unwrap()
                    });
                let Some((npc, item, price)) = offer else {
                    return Some(format!("Nobody sells '{q}'."));
                };
                let count = n.unwrap_or(1);
                let label = format!(
                    "buy {} x{count} ({} each) from {}",
                    b.content.item(item).name,
                    money(price),
                    npc_name(b, npc)
                );
                self.start(Task::talk(npc, vec![Deal::Buy(item, count)]), &label, true);
                None
            }
            "sell" => {
                let deal = if rest.is_empty() || lc(rest) == "junk" {
                    Deal::SellJunk
                } else {
                    let Some(slot) = b.pack.iter().position(|s| {
                        s.is_some_and(|s| matches(b.content.item(s.item).name, rest))
                    }) else {
                        return Some(format!("You have no '{rest}'."));
                    };
                    Deal::Sell(slot as u8)
                };
                let me = b.pos();
                let npc = b
                    .content
                    .npcs
                    .iter()
                    .filter(|n| !n.sells.is_empty())
                    .filter_map(|n| b.npc_pos(n.id).map(|p| (n.id, p.distance(me))))
                    .min_by(|x, y| x.1.partial_cmp(&y.1).unwrap())
                    .map(|x| x.0)?;
                self.start(
                    Task::talk(npc, vec![deal]),
                    &format!("sell to {}", npc_name(b, npc)),
                    true,
                );
                None
            }
            "equip" | "wear" => {
                if rest.is_empty() || lc(rest) == "best" {
                    let got = self.a.equip_upgrades();
                    return Some(if got.is_empty() {
                        "Nothing in your bags is an upgrade.".into()
                    } else {
                        format!("Equipping {}", got.join(", "))
                    });
                }
                let Some(slot) = b
                    .pack
                    .iter()
                    .position(|s| s.is_some_and(|s| matches(b.content.item(s.item).name, rest)))
                else {
                    return Some(format!("You have no '{rest}'."));
                };
                self.a.body.send(ClientMsg::Equip { from: slot as u8 });
                self.start(Task::wait(0.5, false), "equip", false);
                None
            }
            "eat" => {
                self.a.body.send(ClientMsg::EatBest);
                self.start(Task::wait(0.5, false), "eat", false);
                None
            }
            "rest" => {
                self.start(Task::rest(0.95), "rest", false);
                None
            }
            "recall" | "hearth" => {
                self.start(Task::Recall { sent: None }, "recall", false);
                None
            }
            "respawn" | "release" => {
                if b.dead.is_none() {
                    return Some("You are not dead.".into());
                }
                self.start(Task::Respawn { at: Instant::now() }, "respawn", false);
                None
            }
            "say" | "/" => {
                let text = if cmd == "/" {
                    format!("/{rest}")
                } else {
                    rest.to_string()
                };
                self.a.body.say(&text);
                self.start(Task::wait(0.8, false), "say", false);
                None
            }
            "wait" | "w" => {
                let secs: f32 = rest.parse().unwrap_or(5.0);
                self.start(
                    Task::wait(secs.clamp(0.1, 600.0), false),
                    &format!("wait {secs} s"),
                    true,
                );
                None
            }
            "auto" => {
                let mins: f32 = rest.parse().unwrap_or(2.0);
                self.a.cancel();
                self.running = Some(Running::Auto {
                    brain: Box::new(Scripted::new(Personality::Completionist, 7)),
                    until: Instant::now() + Duration::from_secs_f32(mins.clamp(0.1, 60.0) * 60.0),
                });
                None
            }
            "probe" => {
                // debugging: the ground around a place and whether we can walk there
                let nums: Vec<i32> = rest
                    .split(',')
                    .filter_map(|v| v.trim().parse().ok())
                    .collect();
                if nums.len() < 2 {
                    return Some("probe x,z".into());
                }
                let (x, z) = (b.hearth[0] + nums[0], b.hearth[2] + nums[1]);
                let mut t = String::new();
                for dz in -3..=3 {
                    for dx in -3..=3 {
                        let g = crate::body::ground(&b.world, x + dx, z + dz) - b.hearth[1];
                        let top = b.world.surface(x + dx, z + dz) - b.hearth[1];
                        t += &format!("{g:>4}{}", if top != g { "^" } else { " " });
                    }
                    t += "\n";
                }
                let to = b.rel([nums[0], 0, nums[1]]);
                let start = crate::nav::node_at(&b.world, b.pos().into());
                let p = start.and_then(|st| {
                    crate::nav::find_path(
                        &b.world,
                        st,
                        crate::nav::Goal::new(to.into(), 1.5),
                        120_000,
                    )
                });
                t += &format!(
                    "ground heights from the hearth (^: canopy above); path from {:?}: {}",
                    start,
                    match p {
                        Some(p) => format!(
                            "{} steps, partial {}, expanded {}",
                            p.cells.len(),
                            p.partial,
                            p.expanded
                        ),
                        None => "none".into(),
                    }
                );
                Some(t)
            }
            "defend" => {
                self.defend = lc(rest) != "off";
                Some(format!(
                    "Auto-defend is {}.",
                    if self.defend { "on" } else { "off" }
                ))
            }
            "stop" => {
                self.a.cancel();
                self.running = None;
                Some("Stopped.".into())
            }
            other if other.starts_with('/') => {
                self.a.body.say(line);
                self.start(Task::wait(0.8, false), "command", false);
                None
            }
            other => Some(format!("Unknown command '{other}'. Type help.")),
        }
    }

    fn resolve_place(&self, q: &str) -> Result<(Target, String, f32), String> {
        let b = &self.a.body;
        let q = q.trim();
        if q.is_empty() {
            return Err("goto where? (an NPC, creature, area, quest key, x,z or home)".into());
        }
        let nums: Vec<i32> = q.split(',').filter_map(|v| v.trim().parse().ok()).collect();
        if nums.len() >= 2 && q.contains(',') {
            let (x, z) = (nums[0], *nums.last().unwrap());
            return Ok((Target::Pos(b.rel([x, 0, z])), format!("{x},{z}"), 1.5));
        }
        // a quest key said exactly: where its next objective is
        if let Some(quest) = b.content.quests.iter().find(|x| x.key == lc(q))
            && let Some((p, what)) = self.objective_target(quest.id)
        {
            return Ok((Target::Pos(p), what, 4.0));
        }
        if matches!(lc(q).as_str(), "home" | "hearth" | "town") {
            return Ok((Target::Pos(b.rel([0, 0, 3])), "the hearth".into(), 3.0));
        }
        if let Some(n) = self.find_npc(q) {
            return Ok((Target::Npc(n), npc_name(b, n), 3.5));
        }
        if let Some(id) = self.find_creature(q) {
            let name = b.ents.get(&id).map(|e| ent_name(b, e)).unwrap_or_default();
            return Ok((Target::Ent(id), name, 4.0));
        }
        if let Some(p) = self.find_area(q) {
            return Ok((Target::Pos(p), q.to_string(), 6.0));
        }
        if let Some(k) = self.find_creature_kind(q)
            && let Some(p) = self.creature_where(k)
        {
            return Ok((Target::Pos(p), format!("{} camp", cname(b, k)), 6.0));
        }
        if let Some(qid) = self.find_quest(q)
            && let Some((p, what)) = self.objective_target(qid)
        {
            return Ok((Target::Pos(p), what, 4.0));
        }
        Err(format!("Nowhere matches '{q}'."))
    }

    fn gather_node(&mut self, item: u16) -> Option<([i32; 3], [i32; 3])> {
        if self.nodes.is_none() {
            self.nodes = Some(Nodes::scan(&self.a.body.world, &self.a.body.content));
        }
        let b = &self.a.body;
        let me = b.pos();
        let mut cands: Vec<(f32, [i32; 3])> = Vec::new();
        for (blk, list) in &self.nodes.as_ref()?.by_block {
            let r = b.content.rule(*blk);
            if !(r.drops.iter().any(|d| d.item == item)
                || (r.drops.is_empty() && *blk as u16 == item))
            {
                continue;
            }
            for c in list {
                let resting = self
                    .gathered
                    .get(c)
                    .is_some_and(|t| t.elapsed() < Duration::from_secs(300));
                if b.world.get(c[0], c[1], c[2]) == *blk && !resting {
                    cands.push((
                        Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32).distance(me),
                        *c,
                    ));
                }
            }
        }
        cands.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());
        let pick = cands
            .into_iter()
            .take(30)
            .find_map(|(_, c)| stand_for(&b.world, c).map(|s| (c, s)));
        if let Some((c, _)) = pick {
            self.gathered.insert(c, Instant::now());
        }
        pick
    }

    /// Advance the running command. Returns its report when it ends.
    pub fn tick(&mut self) -> Option<String> {
        if let Some((_, at, _)) = &self.pending
            && Instant::now() >= *at
        {
            let alive = self.a.update();
            let _ = alive;
            let (mut t, _, npc) = self.pending.take()?;
            if let Some(npc) = npc {
                t += "\n";
                t += &self.npc_text(npc);
            }
            return Some(t);
        }
        if let Some(Running::Auto { brain, until }) = &mut self.running {
            brain.think(&mut self.a);
            if Instant::now() >= *until || brain.quit {
                self.a.cancel();
                self.running = None;
                let alive = self.a.update();
                let _ = alive;
                return Some("Auto play finished.".into());
            }
        }
        let alive = self.a.update();
        if !alive {
            self.running = None;
            return Some(format!(
                "Disconnected: {}",
                self.a.body.closed.clone().unwrap_or_default()
            ));
        }
        // idle: defend yourself
        if self.running.is_none()
            && self.defend
            && self.a.task.is_none()
            && self.a.body.dead.is_none()
            && self
                .a
                .body
                .last_hurt
                .is_some_and(|t| t.elapsed() < Duration::from_millis(800))
            && let Some(id) = self.a.guess_attacker()
            && let Some(e) = self.a.body.ents.get(&id)
        {
            let (c, l) = (e.creature().unwrap_or(0), e.level);
            let name = ent_name(&self.a.body, e);
            self.a.say(format!("(auto-defend) fighting {name}"));
            self.a.set(Task::fight(id, c, l));
        }
        let Some(Running::Task {
            what,
            interrupt_on_hurt,
            started,
            xp0,
            lvl0,
        }) = &self.running
        else {
            return None;
        };
        let (what, interrupt, started, xp0, lvl0) =
            (what.clone(), *interrupt_on_hurt, *started, *xp0, *lvl0);
        if interrupt
            && self
                .a
                .body
                .last_hurt
                .is_some_and(|t| t.elapsed() < Duration::from_millis(500) && t > started)
            && self.a.task.is_some()
        {
            self.a.cancel();
            self.running = None;
            let who = self
                .a
                .guess_attacker()
                .and_then(|id| self.a.body.ents.get(&id))
                .map(|e| format!("{} #{}", ent_name(&self.a.body, e), e.id))
                .unwrap_or_else(|| "something".into());
            return Some(format!(
                "{what}: INTERRUPTED, attacked by {who}. Use 'attack' to fight back."
            ));
        }
        if self.a.task.is_some() {
            return None;
        }
        self.running = None;
        let (_, out) = self
            .a
            .last
            .take()
            .unwrap_or(("".into(), Outcome::Done("done".into())));
        let secs = started.elapsed().as_secs_f32();
        let xp = (self.a.tel.xp_total as u32).saturating_sub(xp0);
        let mut s = match out {
            Outcome::Done(why) => format!("{what}: done ({why}) in {secs:.0} s"),
            Outcome::Failed(why) => format!("{what}: FAILED ({why}) after {secs:.0} s"),
            Outcome::Running => format!("{what}: stopped"),
        };
        if xp > 0 {
            s += &format!(", +{xp} xp");
        }
        if self.a.body.level > lvl0 {
            s += &format!(", LEVEL {}", self.a.body.level);
        }
        let npc = if what.starts_with("talk")
            || what.starts_with("accept")
            || what.starts_with("turn in")
        {
            self.a.body.dialog.map(|(_, n)| n)
        } else {
            None
        };
        if (what.starts_with("fight") || what == "auto-defend")
            && let Some(f) = self.a.tel.fights.last()
        {
            s += &format!(" (took {:.0} damage, {} casts)", f.damage_taken, f.casts);
        }
        // let the server's answers (quest log, bags, money) arrive first
        self.pending = Some((s, Instant::now() + Duration::from_millis(500), npc));
        None
    }

    /// Is a command still running (or its report not yet printed)?
    pub fn busy(&self) -> bool {
        self.running.is_some() || self.pending.is_some()
    }

    /// Print the result of a command and the situation.
    pub fn report(&mut self, result: &str) -> String {
        let mut s = String::new();
        if !result.is_empty() {
            s += result.trim_end();
            s += "\n";
        }
        s += &self.recent();
        s += &self.describe();
        s += "[ready]\n";
        s
    }
}

pub fn money(c: u32) -> String {
    let (g, s, cp) = (c / 10000, (c / 100) % 100, c % 100);
    match (g, s) {
        (0, 0) => format!("{cp}c"),
        (0, _) => format!("{s}s {cp}c"),
        _ => format!("{g}g {s}s {cp}c"),
    }
}

fn short(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(n).collect::<String>())
    }
}

fn npc_name(b: &Body, n: u16) -> String {
    b.content
        .npc(n)
        .map(|n| n.name.clone())
        .unwrap_or_else(|| "?".into())
}

fn cname(b: &Body, c: u16) -> String {
    b.content
        .creature(c)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "?".into())
}

fn ent_name(b: &Body, e: &crate::body::Ent) -> String {
    if let Some(n) = &e.name {
        return format!("{n}{}", if e.elite { " (elite)" } else { "" });
    }
    match e.kind {
        EntityKind::Creature(c) => cname(b, c),
        EntityKind::Corpse { creature, .. } => format!("{} corpse", cname(b, creature)),
        EntityKind::Npc(n) => npc_name(b, n),
        EntityKind::Item(s) => b.content.item(s.item).name.to_string(),
    }
}

fn objectives_text(b: &Body, q: u16) -> String {
    let Some(quest) = b.content.quest(q) else {
        return String::new();
    };
    quest
        .objectives
        .iter()
        .map(|o| match o {
            Objective::Kill { creature, count } => format!("kill {count} {}", cname(b, *creature)),
            Objective::Collect { item, count } => {
                format!("collect {count} {}", b.content.item(*item).name)
            }
            Objective::Talk { npc } => format!("talk to {}", npc_name(b, *npc)),
            Objective::Explore { name, .. } => format!("explore {name}"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn choices_text(b: &Body, q: u16) -> String {
    let Some(quest) = b.content.quest(q) else {
        return String::new();
    };
    if quest.choice.is_empty() {
        return "none".into();
    }
    quest
        .choice
        .iter()
        .enumerate()
        .map(|(i, (it, _))| {
            let up = if b.is_upgrade(*it) { " UPGRADE" } else { "" };
            format!("{}={}{up}", i + 1, b.content.item(*it).name)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn reward_text(b: &Body, q: u16) -> String {
    let Some(quest) = b.content.quest(q) else {
        return String::new();
    };
    let mut s = format!("{} xp, {}", quest.xp, money(quest.money));
    if !quest.items.is_empty() {
        let v: Vec<&str> = quest
            .items
            .iter()
            .map(|(i, _)| b.content.item(*i).name)
            .collect();
        s += &format!(", items: {}", v.join(", "));
    }
    if !quest.choice.is_empty() {
        s += &format!(", choose one: {}", choices_text(b, q));
    }
    s
}

/// Lines of input: stdin or a script file, read on a helper thread.
fn input(script: Option<std::path::PathBuf>) -> Result<Receiver<Option<String>>, String> {
    let (tx, rx) = channel();
    match script {
        Some(p) => {
            let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            for l in text.lines() {
                let l = l.trim();
                if !l.is_empty() && !l.starts_with('#') {
                    let _ = tx.send(Some(l.to_string()));
                }
            }
            let _ = tx.send(None);
        }
        None => {
            std::thread::spawn(move || {
                let stdin = std::io::stdin();
                for l in stdin.lock().lines() {
                    match l {
                        Ok(l) => {
                            if tx.send(Some(l)).is_err() {
                                return;
                            }
                        }
                        Err(_) => break,
                    }
                }
                let _ = tx.send(None);
            });
        }
    }
    Ok(rx)
}

/// Run the text interface until `quit` or the end of the input.
pub fn run(o: Options) -> Result<(), String> {
    let class = o.class.clone().unwrap_or_else(|| "warrior".into());
    let body = Body::join(
        &o.addr,
        &o.name,
        Some(&class),
        crate::swarm::look(o.name.len()),
    )?;
    let rx = input(o.script.clone())?;
    let mut s = Session::new(body);
    let mut out = std::io::stdout();
    let _ = writeln!(
        out,
        "Joined {} as {}. Type 'help' for commands.",
        o.addr, o.name
    );
    let r = s.report("");
    let _ = write!(out, "{r}");
    let _ = out.flush();
    let mut ended = false;
    loop {
        let t0 = Instant::now();
        if !s.busy() {
            if let Some(Task::Fight { .. }) = &s.a.task {
                // an auto-defend fight: report when it is over
                s.running = Some(Running::Task {
                    what: "auto-defend".into(),
                    interrupt_on_hurt: false,
                    started: Instant::now(),
                    xp0: s.a.tel.xp_total as u32,
                    lvl0: s.a.body.level,
                });
            } else if ended {
                break;
            } else {
                match rx.try_recv() {
                    Ok(Some(line)) => {
                        let _ = writeln!(out, "> {line}");
                        let l = lc(line.trim());
                        if l == "quit" || l == "exit" {
                            break;
                        }
                        if let Some(text) = s.command(&line) {
                            let r = s.report(&text);
                            let _ = write!(out, "{r}");
                            let _ = out.flush();
                        }
                    }
                    Ok(None) | Err(TryRecvError::Disconnected) => ended = true,
                    Err(TryRecvError::Empty) => {}
                }
            }
        }
        if let Some(done) = s.tick() {
            let r = s.report(&done);
            let _ = write!(out, "{r}");
            let _ = out.flush();
            if s.a.body.closed.is_some() {
                break;
            }
        }
        let spent = t0.elapsed().as_secs_f32();
        if spent < TICK {
            std::thread::sleep(Duration::from_secs_f32(TICK - spent));
        }
    }
    Ok(())
}
