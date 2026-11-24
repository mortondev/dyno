//! An agent is a body doing one task at a time (walk somewhere, fight,
//! loot, talk, gather, rest), with telemetry. Brains and the text interface
//! choose the tasks; the agent carries them out tick by tick.

use crate::body::{Body, Ev, NavState, compass};
use crate::telemetry::{Activity, Death, Fight, Reward, StuckAt, Telemetry};
use dyno_protocol::{ClientMsg, EntityKind, RECALL};
use dyno_world::content::{Ability, BuffKind, Objective, Resource, WeaponKind};
use glam::Vec3;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Where to go.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Pos(Vec3),
    Ent(u32),
    Npc(u16),
}

/// What to do once an NPC is talking to us.
#[derive(Clone, Debug, PartialEq)]
pub enum Deal {
    Accept(u16),
    Complete(u16, u8),
    SellJunk,
    /// buy food up to this many items in the bags
    BuyFood(u32),
    Buy(u16, u8),
    Sell(u8),
}

#[derive(Clone, Debug)]
pub enum Task {
    Goto {
        to: Target,
        range: f32,
        why: String,
    },
    Fight {
        target: u32,
        start: Instant,
        last_dealt: Instant,
        sent_attack: Option<Instant>,
        low: f32,
        taken: f32,
        casts: u32,
        creature: u16,
        level: u8,
    },
    Loot {
        corpse: u32,
        sent: Option<Instant>,
    },
    Talk {
        npc: u16,
        deals: Vec<Deal>,
        sent: Option<Instant>,
    },
    Gather {
        node: [i32; 3],
        /// where to stand (None: next to it)
        stand: Option<[i32; 3]>,
        sent: Option<Instant>,
    },
    Rest {
        until: f32,
        start: Instant,
        ate: bool,
    },
    Wait {
        until: Instant,
        spawn: bool,
    },
    Recall {
        sent: Option<Instant>,
    },
    Respawn {
        at: Instant,
    },
}

impl Task {
    pub fn name(&self) -> &'static str {
        match self {
            Task::Goto { .. } => "goto",
            Task::Fight { .. } => "fight",
            Task::Loot { .. } => "loot",
            Task::Talk { .. } => "talk",
            Task::Gather { .. } => "gather",
            Task::Rest { .. } => "rest",
            Task::Wait { .. } => "wait",
            Task::Recall { .. } => "recall",
            Task::Respawn { .. } => "respawn",
        }
    }

    pub fn fight(target: u32, creature: u16, level: u8) -> Task {
        let now = Instant::now();
        Task::Fight {
            target,
            start: now,
            last_dealt: now,
            sent_attack: None,
            low: f32::MAX,
            taken: 0.0,
            casts: 0,
            creature,
            level,
        }
    }

    pub fn talk(npc: u16, deals: Vec<Deal>) -> Task {
        Task::Talk {
            npc,
            deals,
            sent: None,
        }
    }

    pub fn rest(until: f32) -> Task {
        Task::Rest {
            until,
            start: Instant::now(),
            ate: false,
        }
    }

    pub fn wait(secs: f32, spawn: bool) -> Task {
        Task::Wait {
            until: Instant::now() + Duration::from_secs_f32(secs),
            spawn,
        }
    }
}

/// How a task ended.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Running,
    Done(String),
    Failed(String),
}

pub struct Agent {
    pub body: Body,
    pub task: Option<Task>,
    /// how the last task ended (taken by whoever reads it)
    pub last: Option<(String, Outcome)>,
    pub tel: Telemetry,
    /// readable recent events, newest last
    pub log: VecDeque<(f64, String)>,
    /// dots we put on creatures: (creature, ability) -> until
    dots: HashMap<(u32, u16), Instant>,
    /// the creature that hurt us most recently, if we know
    pub attacker: Option<u32>,
    /// the target of the last fight that ended
    pub last_fight: Option<u32>,
    last_activity: Instant,
    last_track: f64,
    seen_stuck: usize,
    last_level: u8,
    xp_seen: (u8, u32),
    pub doing: String,
}

pub fn ability_name(a: &Ability) -> String {
    a.name.clone()
}

impl Agent {
    pub fn new(body: Body, brain: &str) -> Agent {
        let class = body
            .class
            .and_then(|c| body.content.class(c))
            .map(|c| c.name.clone())
            .unwrap_or_default();
        let tel = Telemetry {
            name: body.name.clone(),
            class,
            brain: brain.to_string(),
            start_level: body.level,
            level: body.level,
            ..Default::default()
        };
        let lvl = body.level;
        Agent {
            xp_seen: (body.level, body.xp),
            body,
            task: None,
            last: None,
            tel,
            log: VecDeque::new(),
            dots: HashMap::new(),
            attacker: None,
            last_fight: None,
            last_activity: Instant::now(),
            last_track: -100.0,
            seen_stuck: 0,
            last_level: lvl,
            doing: String::new(),
        }
    }

    pub fn say(&mut self, line: impl Into<String>) {
        let t = self.body.secs();
        self.log.push_back((t, line.into()));
        while self.log.len() > 60 {
            self.log.pop_front();
        }
    }

    pub fn set(&mut self, t: Task) {
        self.body.stop();
        self.task = Some(t);
    }

    pub fn cancel(&mut self) {
        if let Some(Task::Fight { .. }) = &self.task {
            self.body.send(ClientMsg::AutoAttack { target: None });
        }
        self.task = None;
        self.body.stop();
    }

    /// The baseline once the join burst is in (level and class known).
    pub fn baseline(&mut self) {
        self.tel.start_level = self.body.level;
        self.tel.level = self.body.level;
        self.last_level = self.body.level;
        self.xp_seen = (self.body.level, self.body.xp);
        self.tel.class = self
            .body
            .class
            .and_then(|c| self.body.content.class(c))
            .map(|c| c.name.clone())
            .unwrap_or_default();
    }

    /// One tick: messages, the task, physics. Returns false when disconnected.
    pub fn update(&mut self) -> bool {
        let alive = self.body.pump();
        self.digest();
        if self.tel.first_party.is_none() && self.body.party.len() >= 2 {
            self.tel.first_party = Some(self.body.secs());
        }
        if !alive {
            if let Some(r) = self.body.closed.clone() {
                self.tel.disconnects.push(r);
            }
            return false;
        }
        if let Some(t) = self.task.take() {
            let mut t = t;
            match self.step(&mut t) {
                Outcome::Running => self.task = Some(t),
                o => {
                    self.last = Some((t.name().to_string(), o));
                    self.body.stop();
                }
            }
        }
        self.body.tick();
        self.account();
        true
    }

    fn rel(&self, p: Vec3) -> [i32; 3] {
        [
            p.x.floor() as i32 - self.body.hearth[0],
            p.y.floor() as i32 - self.body.hearth[1],
            p.z.floor() as i32 - self.body.hearth[2],
        ]
    }

    /// Time accounting and the position track.
    fn account(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_activity).as_secs_f64();
        self.last_activity = now;
        let b = &self.body;
        let act = if b.dead.is_some() {
            Activity::Dead
        } else {
            match &self.task {
                Some(Task::Fight { .. }) => Activity::Fight,
                Some(Task::Loot { .. }) => Activity::Loot,
                Some(Task::Gather { .. }) => {
                    if b.nav.state == NavState::Moving {
                        Activity::Travel
                    } else {
                        Activity::Gather
                    }
                }
                Some(Task::Rest { .. }) => Activity::Rest,
                Some(Task::Wait { spawn: true, .. }) => Activity::WaitSpawn,
                Some(Task::Wait { .. }) if self.doing == "taking a break" => Activity::Afk,
                Some(Task::Respawn { .. }) => Activity::Dead,
                Some(Task::Goto { .. }) | Some(Task::Talk { .. }) | Some(Task::Recall { .. })
                    if b.nav.state == NavState::Moving =>
                {
                    if b.nav.stalls >= 3 {
                        Activity::Stuck
                    } else {
                        Activity::Travel
                    }
                }
                _ if b.zone.2 => Activity::Town,
                _ => Activity::Idle,
            }
        };
        self.tel.add_time(act, dt);
        let t = b.secs();
        self.tel.secs = t;
        self.tel.distance = b.distance as f64;
        self.tel.money = b.money;
        if t - self.last_track >= 10.0 {
            self.last_track = t;
            let r = self.rel(b.player.pos);
            self.tel.track.push((t, r[0], r[2], self.doing.clone()));
            let up: Vec<String> = b
                .creatures()
                .filter(|e| crate::body::on_canopy(&b.world, e.pos))
                .map(|e| {
                    let p = self.rel(e.pos);
                    format!(
                        "{} lv{} at {},{},{} (ground {})",
                        self.cname(e.creature().unwrap_or(0)),
                        e.level,
                        p[0],
                        p[1],
                        p[2],
                        crate::body::ground(
                            &b.world,
                            e.pos.x.floor() as i32,
                            e.pos.z.floor() as i32
                        ) - b.hearth[1]
                    )
                })
                .collect();
            for u in up {
                if self.tel.canopy.len() < 100 && !self.tel.canopy.contains(&u) {
                    self.tel.canopy.push(u);
                }
            }
        }
        while self.seen_stuck < self.body.stuck_log.len() {
            let s = self.body.stuck_log[self.seen_stuck].clone();
            self.seen_stuck += 1;
            let at = self.rel(s.at);
            let goal = self.rel(Vec3::from(s.goal));
            self.tel.stuck.push(StuckAt {
                t,
                at,
                goal,
                why: s.why,
                doing: self.doing.clone(),
            });
        }
    }

    /// Turn body events into telemetry and log lines.
    fn digest(&mut self) {
        let events = std::mem::take(&mut self.body.events);
        let t = self.body.secs();
        for (_, e) in events {
            match e {
                Ev::Kill { creature, xp, .. } => {
                    self.tel.kills += 1;
                    self.tel.xp_kills += xp as u64;
                    self.tel.first_kill.get_or_insert(t);
                    self.tel.reward(t, Reward::Kill);
                    let name = self.cname(creature);
                    self.say(format!("Killed {name} (+{xp} xp)"));
                }
                Ev::KillNoCredit { creature, .. } => {
                    self.tel.kills_no_credit += 1;
                    let me = self.body.pos();
                    if !self
                        .body
                        .players
                        .values()
                        .any(|(_, p)| p.distance(me) < 20.0)
                    {
                        // nobody else was there: not a stolen kill
                        self.tel.kills_no_credit_alone += 1;
                    }
                    let name = self.cname(creature);
                    self.say(format!("{name} died, but you got no credit"));
                }
                Ev::Gain { xp, money } => {
                    if xp > 0 {
                        self.tel.xp_total += xp as u64;
                    }
                    if money > 0 {
                        self.tel.money_in += money as u64;
                    }
                }
                Ev::Money(d) if d < 0 => {
                    self.tel.money_out += (-d) as u64;
                }
                Ev::LevelUp(l) => {
                    self.tel.level = l;
                    self.tel.first_level.get_or_insert(t);
                    self.tel.reward(t, Reward::Level);
                    self.say(format!("LEVEL UP: you are now level {l}"));
                }
                Ev::QuestAccepted(q) => {
                    let key = self.qkey(q);
                    self.tel.quests.push(crate::telemetry::QuestRun {
                        key: key.clone(),
                        accepted: t,
                        done: None,
                    });
                    let title = self.qtitle(q);
                    self.say(format!("Quest accepted: {title} [{key}]"));
                }
                Ev::QuestDone(q) => {
                    let key = self.qkey(q);
                    if let Some(r) = self
                        .tel
                        .quests
                        .iter_mut()
                        .rev()
                        .find(|r| r.key == key && r.done.is_none())
                    {
                        r.done = Some(t);
                    } else {
                        self.tel.quests.push(crate::telemetry::QuestRun {
                            key: key.clone(),
                            accepted: t,
                            done: Some(t),
                        });
                    }
                    self.tel.first_quest.get_or_insert(t);
                    self.tel.reward(t, Reward::Quest);
                    let title = self.qtitle(q);
                    self.say(format!("QUEST COMPLETE: {title}"));
                }
                Ev::QuestProgress(q) => {
                    let line = self.quest_line(q);
                    self.say(format!("Progress: {line}"));
                }
                Ev::Item { item, count, first } => {
                    let name = self.body.content.item(item).name.to_string();
                    if first {
                        self.tel.items_new += 1;
                        self.tel.reward(t, Reward::NewItem);
                    }
                    self.say(format!("+{count} {name}"));
                }
                Ev::ItemLost { .. } => {}
                Ev::Died(m) => {
                    let at = self.rel(self.body.player.pos);
                    self.tel.deaths.push(Death {
                        t,
                        cause: m.clone(),
                        level: self.body.level,
                        at,
                        zone: self.body.zone.0.clone(),
                    });
                    self.say(format!("DIED: {m}"));
                }
                Ev::Respawned => self.say("You are alive again"),
                Ev::Hurt(a) => {
                    if let Some(Task::Fight { taken, .. }) = &mut self.task {
                        *taken += a;
                    }
                    if self
                        .attacker
                        .is_none_or(|id| !self.body.ents.contains_key(&id))
                    {
                        self.attacker = self.guess_attacker();
                    }
                }
                Ev::Dealt { .. } => {
                    if let Some(Task::Fight { last_dealt, .. }) = &mut self.task {
                        *last_dealt = Instant::now();
                    }
                }
                Ev::Zone { name, safe } => {
                    if !name.is_empty() && !self.tel.areas.contains(&name) {
                        self.tel.areas.push(name.clone());
                        if self.tel.areas.len() > 1 {
                            self.tel.reward(t, Reward::NewArea);
                        }
                    }
                    let n = if name.is_empty() {
                        "the wilds".into()
                    } else {
                        name
                    };
                    self.say(format!("Entered {n}{}", if safe { " (safe)" } else { "" }));
                }
                Ev::System(s) => {
                    if s.starts_with("You gather") {
                        self.tel.gathers += 1;
                        self.tel.reward(t, Reward::Gather);
                    }
                    if s.starts_with("You loot") || s.starts_with("You receive loot") {
                        self.tel.loots += 1;
                        self.tel.reward(t, Reward::Loot);
                    }
                    // other players' news is noise for us
                    if (!s.contains(" reached level ") || s.contains(&self.body.name))
                        && !s.starts_with("Quest accepted:")
                        && !s.starts_with("Quest complete:")
                        && !s.ends_with(" left")
                        && !s.ends_with(" joined")
                        && !s.contains(" has become a")
                    {
                        self.say(s);
                    }
                }
                Ev::Chat { from, text } => {
                    if from != self.body.name {
                        self.say(format!("<{from}> {text}"))
                    }
                }
                Ev::WorldEvent(title) => self.say(format!("World event: {title}")),
                Ev::Invite(from) => self.say(format!("{from} invites you to a party")),
                Ev::Closed(r) => self.say(format!("Disconnected: {r}")),
                Ev::Teleport(from, to) => {
                    if Instant::now() > self.body.teleport_ok_until && self.body.dead.is_none() {
                        // the movement check put us back
                        let (f, t2) = (self.rel(from), self.rel(to));
                        let v = self.body.player.vel;
                        self.tel.rubberbands.push(crate::telemetry::StuckAt {
                            t,
                            at: f,
                            goal: t2,
                            why: format!(
                                "moved back {:.1} blocks (on ground {}, in water {})",
                                from.distance(to),
                                self.body.player.on_ground,
                                self.body.world.get(
                                    from.x.floor() as i32,
                                    from.y.floor() as i32,
                                    from.z.floor() as i32
                                ) == dyno_world::WATER
                            ),
                            doing: self.doing.clone(),
                        });
                        let _ = v;
                        self.say("(the server moved you back)");
                    }
                }
                Ev::Dialog(_) | Ev::Loot { .. } | Ev::Money(_) => {}
            }
        }
        let lvl = self.body.level;
        let xp = self.body.xp;
        if (lvl, xp) != self.xp_seen {
            self.xp_seen = (lvl, xp);
            self.tel.xp_curve.push((t, lvl, self.tel.xp_total));
            self.tel.xp_quests = self.tel.xp_total.saturating_sub(self.tel.xp_kills);
        }
        if lvl > self.last_level {
            self.last_level = lvl;
            self.tel.level = lvl;
        }
        self.tel.latency_ms = self.body.latency.clone();
    }

    pub fn cname(&self, c: u16) -> String {
        self.body
            .content
            .creature(c)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "?".into())
    }
    pub fn qkey(&self, q: u16) -> String {
        self.body
            .content
            .quest(q)
            .map(|q| q.key.clone())
            .unwrap_or_default()
    }
    pub fn qtitle(&self, q: u16) -> String {
        self.body
            .content
            .quest(q)
            .map(|q| q.title.clone())
            .unwrap_or_default()
    }

    /// "Thin the Herd: Grazer 3/8"
    pub fn quest_line(&self, q: u16) -> String {
        let c = &self.body.content;
        let Some(quest) = c.quest(q) else {
            return String::new();
        };
        let Some(p) = self.body.quests.iter().find(|x| x.quest == q) else {
            return quest.title.clone();
        };
        let parts: Vec<String> = quest
            .objectives
            .iter()
            .zip(&p.counts)
            .map(|(o, n)| match o {
                Objective::Kill { creature, count } => {
                    format!("{} {n}/{count}", self.cname(*creature))
                }
                Objective::Collect { item, count } => {
                    format!(
                        "{} {}/{count}",
                        c.item(*item).name,
                        self.body.count(*item).min(*count as u32)
                    )
                }
                Objective::Talk { npc } => format!(
                    "talk to {} {}",
                    c.npc(*npc).map(|n| n.name.as_str()).unwrap_or("?"),
                    if *n > 0 { "(done)" } else { "" }
                ),
                Objective::Explore { name, .. } => {
                    format!("explore {name} {}", if *n > 0 { "(done)" } else { "" })
                }
            })
            .collect();
        format!("{}: {}", quest.title, parts.join(", "))
    }

    /// The nearest hostile creature close enough to be hitting us.
    pub fn guess_attacker(&self) -> Option<u32> {
        let b = &self.body;
        b.creatures()
            .filter(|e| {
                let c = e.creature().and_then(|c| b.content.creature(c));
                let ranged = c.is_some_and(|c| {
                    matches!(
                        c.ability,
                        Some(dyno_world::content::CreatureAbility::Bolt { .. })
                    )
                });
                let reach = if ranged { 16.0 } else { 5.5 };
                b.dist3(e.pos) < reach
            })
            .min_by(|a, c| {
                b.dist3(a.pos)
                    .partial_cmp(&b.dist3(c.pos))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|e| e.id)
    }

    /// Is quest `q` ready to hand in?
    pub fn quest_ready(&self, q: u16) -> bool {
        let c = &self.body.content;
        let Some(quest) = c.quest(q) else {
            return false;
        };
        let Some(p) = self.body.quests.iter().find(|x| x.quest == q) else {
            return false;
        };
        quest
            .objectives
            .iter()
            .zip(&p.counts)
            .all(|(o, n)| match o {
                Objective::Collect { item, count } => self.body.count(*item) >= *count as u32,
                _ => *n >= o.need(),
            })
    }

    /// Can we take quest `q` now (the same checks as the server)?
    pub fn can_take(&self, q: u16) -> bool {
        let b = &self.body;
        let Some(quest) = b.content.quest(q) else {
            return false;
        };
        !b.done.contains(&q)
            && !b.quests.iter().any(|x| x.quest == q)
            && b.quests.len() < 20
            && b.level + 1 >= quest.level
            && quest.requires.iter().all(|r| b.done.contains(r))
    }

    /// The best reward choice for us.
    pub fn best_choice(&self, q: u16) -> u8 {
        let b = &self.body;
        let Some(quest) = b.content.quest(q) else {
            return 0;
        };
        let now = b.gear_score(&b.equip);
        let mut best = (f32::MIN, 0u8);
        for (i, (item, _)) in quest.choice.iter().enumerate() {
            let score = match b.upgrade_place(*item) {
                Some(place) => {
                    let mut e = b.equip.clone();
                    e[place] = Some(dyno_protocol::Stack::new(*item, 1));
                    b.gear_score(&e) - now
                }
                None => {
                    let view = b.content.item(*item);
                    let usable = b
                        .class
                        .and_then(|c| b.content.class(c))
                        .is_some_and(|c| dyno_sim::character::cannot_equip(c, 60, &view).is_none());
                    if usable {
                        -1.0
                    } else {
                        -100.0 + view.value as f32 * 0.001
                    }
                }
            };
            if score > best.0 {
                best = (score, i as u8);
            }
        }
        best.1
    }

    // ---------------- combat ----------------

    fn class_info(&self) -> Option<(Resource, bool)> {
        let b = &self.body;
        let c = b.class.and_then(|c| b.content.class(c))?;
        let ranged = c.weapons.contains(&WeaponKind::Bow)
            || (c.resource == Resource::Mana
                && !c.weapons.contains(&WeaponKind::Sword)
                && !c.weapons.contains(&WeaponKind::Mace))
            || c.key == "priest"
            || c.key == "mage"
            || c.key == "hunter";
        Some((c.resource, ranged))
    }

    /// The range we fight from.
    pub fn fight_range(&self) -> f32 {
        let b = &self.body;
        match self.class_info() {
            Some((_, true)) => {
                // casters and hunters stand back while they have the power to
                let best = b
                    .class
                    .map(|c| b.content.known_abilities(c, b.level))
                    .unwrap_or_default()
                    .iter()
                    .filter(|a| a.range > 6.0 && a.damage[0] > 0.0 && b.power >= a.cost)
                    .map(|a| a.range)
                    .fold(0.0f32, f32::max);
                if best > 0.0 {
                    (best - 4.0).clamp(3.0, 22.0)
                } else {
                    2.5
                }
            }
            _ => 2.5,
        }
    }

    fn cast(&mut self, a: &Ability, target: Option<u32>) {
        let name = ability_name(a);
        let now = self.body.secs();
        self.tel.first_casts.entry(name.clone()).or_insert(now);
        *self.tel.abilities.entry(name).or_insert(0) += 1;
        if a.charge || a.blink > 0.0 {
            self.body.expect_teleport(1.5);
        }
        self.body.send(ClientMsg::Cast {
            ability: a.id,
            target,
        });
        // the server answers with cooldowns; until then assume the GCD
        self.body.gcd_until = Instant::now() + Duration::from_millis(1000);
        if a.cast > 0.0 {
            self.body.casting = Some((a.id, Instant::now() + Duration::from_secs_f32(a.cast)));
        }
        if a.dot[0] > 0.0
            && let Some(t) = target
        {
            self.dots.insert(
                (t, a.id),
                Instant::now() + Duration::from_secs_f32(a.dot[2].max(6.0)),
            );
        }
        // control: remember how long it holds, so we do not waste another
        let hold = a.stun.max(a.root).max(a.slow[1]);
        if hold > 0.0
            && a.dot[0] <= 0.0
            && let Some(t) = target
        {
            self.dots
                .insert((t, a.id), Instant::now() + Duration::from_secs_f32(hold));
        }
    }

    /// Pick and use one ability, if one makes sense now. Returns true if cast.
    pub fn rotation(&mut self, target: Option<u32>) -> bool {
        self.rotation_moving(target, true)
    }

    /// The rotation; spells with a cast time only when `still`.
    pub fn rotation_moving(&mut self, target: Option<u32>, still: bool) -> bool {
        let b = &self.body;
        let Some(class) = b.class else { return false };
        let now = Instant::now();
        if b.is_casting() || b.gcd_until > now || b.dead.is_some() {
            return false;
        }
        let known: Vec<Ability> = b
            .content
            .known_abilities(class, b.level)
            .into_iter()
            .cloned()
            .collect();
        let hp = b.health / b.max_health.max(1.0);
        let te = target.and_then(|t| b.ents.get(&t)).cloned();
        let dist = te.as_ref().map(|e| b.dist3(e.pos)).unwrap_or(f32::MAX);
        let thp = te
            .as_ref()
            .map(|e| e.health / e.max.max(1.0))
            .unwrap_or(1.0);
        let near_foes = b
            .creatures()
            .filter(|e| {
                b.dist3(e.pos) < 7.0
                    && e.creature()
                        .and_then(|c| b.content.creature(c))
                        .is_some_and(|c| c.hostile)
            })
            .count();
        let usable = |a: &Ability| {
            b.power + 0.01 >= a.cost
                && b.cooldowns.get(&a.id).is_none_or(|t| *t <= now)
                && (still || a.cast <= 0.0)
        };
        let aura = |a: &Ability| b.auras.get(&a.id).is_some_and(|t| *t > now);
        let in_range = |a: &Ability| dist <= a.range + 0.8;
        let mut pick: Option<(Ability, Option<u32>)> = None;
        let pressed = te.is_some() && dist < 5.0;
        // 1. survive: a shield as soon as something is on us
        if hp < 0.9 && pressed {
            pick = known
                .iter()
                .find(|a| {
                    usable(a) && a.buff.is_some_and(|x| x.kind == BuffKind::Absorb) && !aura(a)
                })
                .map(|a| (a.clone(), None));
        }
        if pick.is_none() && (hp < 0.35 || (hp < 0.6 && near_foes >= 2)) {
            pick = known
                .iter()
                .find(|a| usable(a) && a.buff.is_some_and(|x| x.kind == BuffKind::Ward) && !aura(a))
                .map(|a| (a.clone(), None));
        }
        if pick.is_none() && hp < 0.5 {
            pick = known
                .iter()
                .filter(|a| usable(a) && a.heal[0] > 0.0 && a.damage[0] == 0.0)
                .max_by(|x, y| x.heal[0].partial_cmp(&y.heal[0]).unwrap())
                .map(|a| (a.clone(), None));
        }
        if pick.is_none() && hp < 0.7 {
            pick = known
                .iter()
                .find(|a| usable(a) && a.hot[0] > 0.0 && !aura(a))
                .map(|a| (a.clone(), None));
        }
        // 2. buffs that are not running
        if pick.is_none() {
            pick = known
                .iter()
                .find(|a| {
                    usable(a)
                        && a.combo >= 0
                        && a.damage[0] == 0.0
                        && a.buff.is_some_and(|x| {
                            !matches!(
                                x.kind,
                                BuffKind::Ward | BuffKind::Absorb | BuffKind::Stealth
                            )
                        })
                        && !aura(a)
                })
                .map(|a| (a.clone(), None));
        }
        if let Some(t) = target
            && te.is_some()
            && pick.is_none()
        {
            let held = |a: &Ability| self.dots.get(&(t, a.id)).is_some_and(|u| *u > now);
            // 3. sneak up on a foe that has not seen us, and charge in from afar
            if thp >= 0.999 && dist > 6.0 && dist < 30.0 {
                pick = known
                    .iter()
                    .find(|a| {
                        usable(a) && a.buff.is_some_and(|x| x.kind == BuffKind::Stealth) && !aura(a)
                    })
                    .map(|a| (a.clone(), None));
            }
            if pick.is_none() {
                pick = known
                    .iter()
                    .find(|a| a.charge && usable(a) && dist > 8.0 && in_range(a))
                    .map(|a| (a.clone(), Some(t)));
            }
            // an opener on a foe we have not touched
            if pick.is_none() && thp >= 0.9 {
                pick = known
                    .iter()
                    .find(|a| a.opener > 1.0 && usable(a) && in_range(a))
                    .map(|a| (a.clone(), Some(t)));
            }
            // 4. finishers on enough combo points
            let combo = if b.combo.1 == Some(t) { b.combo.0 } else { 0 };
            if pick.is_none() && (combo >= 3 || (combo >= 1 && thp < 0.25)) {
                pick = known
                    .iter()
                    .filter(|a| a.combo < 0 && usable(a) && (a.range == 0.0 || in_range(a)))
                    .max_by(|x, y| x.damage[0].partial_cmp(&y.damage[0]).unwrap())
                    .map(|a| (a.clone(), if a.range > 0.0 { Some(t) } else { None }));
            }
            // control: stun what is on us; pin or slow what is still coming;
            // freeze the ground when surrounded or hurt
            if pick.is_none() && thp > 0.3 {
                pick = known
                    .iter()
                    .find(|a| {
                        let ctl = a.stun > 0.0 || a.root > 0.0 || a.slow[1] > 0.0;
                        if !ctl || a.combo < 0 || a.charge || !usable(a) || held(a) {
                            return false;
                        }
                        if a.range == 0.0 {
                            a.radius > 0.0 && dist <= a.radius && (near_foes >= 2 || hp < 0.6)
                        } else if a.range > 6.0 {
                            in_range(a) && (a.stun > 0.0 || dist > 5.0)
                        } else {
                            in_range(a) && a.stun > 0.0
                        }
                    })
                    .map(|a| (a.clone(), if a.range > 0.0 { Some(t) } else { None }));
            }
            // 5. damage over time not yet on it (not on nearly dead foes)
            if pick.is_none() && thp > 0.35 {
                pick = known
                    .iter()
                    .find(|a| {
                        a.dot[0] > 0.0
                            && a.combo >= 0
                            && usable(a)
                            && in_range(a)
                            && self.dots.get(&(t, a.id)).is_none_or(|u| *u <= now)
                    })
                    .map(|a| (a.clone(), Some(t)));
            }
            // 6. area damage when it would hit
            if pick.is_none() && (near_foes >= 2 || dist < 4.0) {
                pick = known
                    .iter()
                    .find(|a| {
                        a.radius > 0.0
                            && a.range == 0.0
                            && a.damage[0] > 0.0
                            && usable(a)
                            && dist <= a.radius
                            && (near_foes >= 2 || a.cooldown > 0.0)
                    })
                    .map(|a| (a.clone(), None));
            }
            // 7. the strongest direct hit
            if pick.is_none() {
                pick = known
                    .iter()
                    .filter(|a| {
                        (a.damage[0] > 1.0 || a.weapon > 0.0)
                            && a.range > 0.0
                            && a.combo >= 0
                            && !a.charge
                            && a.stun <= 0.0
                            && a.root <= 0.0
                            && (a.opener <= 1.0 || thp >= 0.9)
                            && usable(a)
                            && in_range(a)
                    })
                    // like players: a hit with a cooldown goes first when it is
                    // ready, then the strongest spammable one
                    .max_by(|x, y| {
                        let w = |a: &Ability| {
                            (a.cooldown > 0.0) as u8 as f32 * 1000.0
                                + a.damage[0]
                                + a.damage[1]
                                + a.weapon * 8.0
                        };
                        w(x).partial_cmp(&w(y)).unwrap()
                    })
                    .map(|a| (a.clone(), Some(t)));
            }
        }
        match pick {
            Some((a, tgt)) => {
                self.cast(&a, tgt);
                true
            }
            None => false,
        }
    }

    // ---------------- tasks ----------------

    fn target_pos(&self, t: &Target) -> Option<Vec3> {
        match t {
            Target::Pos(p) => Some(*p),
            Target::Ent(id) => self.body.ents.get(id).map(|e| e.pos),
            Target::Npc(n) => self.body.npc_pos(*n),
        }
    }

    fn step(&mut self, task: &mut Task) -> Outcome {
        if self.body.dead.is_some() && !matches!(task, Task::Respawn { .. }) {
            if let Task::Fight {
                start,
                taken,
                casts,
                creature,
                level,
                target,
                ..
            } = task
            {
                // a fight we lost
                self.tel.fights.push(Fight {
                    start: self.body.secs() - start.elapsed().as_secs_f64(),
                    secs: start.elapsed().as_secs_f64(),
                    creature: self.cname(*creature),
                    level: *level,
                    my_level: self.body.level,
                    won: false,
                    damage_taken: *taken,
                    casts: *casts,
                    why: "died".into(),
                });
                self.last_fight = Some(*target);
            }
            return Outcome::Failed("dead".into());
        }
        match task {
            Task::Goto { to, range, .. } => {
                let Some(p) = self.target_pos(to) else {
                    return Outcome::Failed("target gone".into());
                };
                // places count by the map (x, z); people and creatures need us level with them
                let loose = matches!(to, Target::Pos(_));
                if self.body.dist(p) <= *range && (loose || (self.body.pos().y - p.y).abs() < 3.5) {
                    return Outcome::Done("arrived".into());
                }
                if loose {
                    self.body.go_to_h(p, *range, 40.0);
                } else {
                    self.body.go_to(p, *range);
                }
                match &self.body.nav.state {
                    NavState::Arrived => Outcome::Done("arrived".into()),
                    NavState::Failed(why) => Outcome::Failed(why.clone()),
                    _ => Outcome::Running,
                }
            }
            Task::Fight {
                target,
                start,
                last_dealt,
                sent_attack,
                low,
                taken,
                casts,
                creature,
                level,
            } => {
                let t = *target;
                let end = |a: &mut Agent, won: bool, why: &str| {
                    a.tel.fights.push(Fight {
                        start: a.body.secs() - start.elapsed().as_secs_f64(),
                        secs: start.elapsed().as_secs_f64(),
                        creature: a.cname(*creature),
                        level: *level,
                        my_level: a.body.level,
                        won,
                        damage_taken: *taken,
                        casts: *casts,
                        why: why.to_string(),
                    });
                    a.body.send(ClientMsg::AutoAttack { target: None });
                    a.last_fight = Some(t);
                    if a.attacker == Some(t) {
                        a.attacker = None;
                    }
                    if won {
                        Outcome::Done(why.to_string())
                    } else {
                        Outcome::Failed(why.to_string())
                    }
                };
                let Some(e) = self.body.ents.get(&t).cloned() else {
                    return end(self, true, "it died");
                };
                if e.health <= 0.0 {
                    return end(self, true, "it died");
                }
                // it got away and healed (leash), or we cannot hurt it
                if e.health < *low - 0.5 {
                    *low = e.health;
                }
                if *low < e.max * 0.7 && e.health >= e.max - 0.01 {
                    return end(self, false, "it fled home and healed");
                }
                if last_dealt.elapsed() > Duration::from_secs(25) {
                    return end(self, false, "could not reach or hurt it");
                }
                if start.elapsed() > Duration::from_secs(45) && *low > e.max * 0.85 {
                    return end(self, false, "no progress: it hardly took damage");
                }
                if start.elapsed() > Duration::from_secs(150) {
                    return end(self, false, "took too long");
                }
                let range = self.fight_range();
                let d = self.body.dist3(e.pos);
                // DYNO_BOT_TRACE=1: a line per second of every fight, for diagnosis
                if std::env::var_os("DYNO_BOT_TRACE").is_some()
                    && start.elapsed().as_millis() % 1000 < 50
                {
                    let me = self.body.pos();
                    eprintln!(
                        "[trace] {} t={:.0} {} #{t} hp {:.0}/{:.0} me {:.0}/{:.0} d={:.1} dy={:.1} at {:?} me at {:?} attacking {:?}",
                        self.body.name,
                        start.elapsed().as_secs_f32(),
                        self.cname(*creature),
                        e.health,
                        e.max,
                        self.body.health,
                        self.body.max_health,
                        d,
                        e.pos.y - me.y,
                        self.rel(e.pos),
                        self.rel(me),
                        self.body.attacking,
                    );
                }
                if !self.body.is_casting() {
                    if d > range {
                        self.body.go_to(e.pos, (range - 0.5).max(1.5));
                        if let NavState::Failed(why) = self.body.nav.state.clone() {
                            return end(self, false, &format!("cannot reach it: {why}"));
                        }
                    } else {
                        self.body.stop();
                    }
                }
                self.body.face(e.pos);
                if self.body.attacking != Some(t)
                    && sent_attack.is_none_or(|s| s.elapsed() > Duration::from_secs(2))
                    && d < 40.0
                {
                    *sent_attack = Some(Instant::now());
                    self.body.send(ClientMsg::AutoAttack { target: Some(t) });
                }
                let v = self.body.player.vel;
                let still = v.x.abs() + v.z.abs() < 0.3;
                if d < 32.0 && self.rotation_moving(Some(t), still) {
                    *casts += 1;
                    // standing still for a cast
                    if self.body.is_casting() {
                        self.body.stop();
                    }
                }
                Outcome::Running
            }
            Task::Loot { corpse, sent } => {
                let id = *corpse;
                let Some(e) = self.body.ents.get(&id).cloned() else {
                    return Outcome::Failed("the body is gone".into());
                };
                if !matches!(e.kind, EntityKind::Corpse { loot: true, .. }) {
                    return Outcome::Done("nothing left".into());
                }
                if self.body.dist3(e.pos) > 4.5 {
                    self.body.go_to(e.pos, 2.5);
                    if let NavState::Failed(why) = self.body.nav.state.clone() {
                        return Outcome::Failed(why);
                    }
                    return Outcome::Running;
                }
                self.body.stop();
                match sent {
                    None => {
                        *sent = Some(Instant::now());
                        self.body.send(ClientMsg::Loot { id, all: true });
                        Outcome::Running
                    }
                    Some(s) if s.elapsed() > Duration::from_millis(1500) => {
                        Outcome::Failed("not ours to loot".into())
                    }
                    Some(_) => Outcome::Running,
                }
            }
            Task::Talk { npc, deals, sent } => {
                let n = *npc;
                let Some(e) = self.body.npc_ent(n).cloned() else {
                    // not in sight yet: walk to where they stand
                    let Some(p) = self.body.npc_pos(n) else {
                        return Outcome::Failed("no such NPC".into());
                    };
                    if self.body.dist(p) < 8.0 {
                        return Outcome::Failed("NPC is not here".into());
                    }
                    self.body.go_to(p, 4.0);
                    if let NavState::Failed(why) = self.body.nav.state.clone() {
                        return Outcome::Failed(format!("cannot reach NPC: {why}"));
                    }
                    return Outcome::Running;
                };
                if self.body.dist3(e.pos) > 4.8 {
                    self.body.go_to(e.pos, 3.0);
                    if let NavState::Failed(why) = self.body.nav.state.clone() {
                        return Outcome::Failed(format!("cannot reach NPC: {why}"));
                    }
                    return Outcome::Running;
                }
                self.body.stop();
                self.body.face(e.pos);
                match sent {
                    None => {
                        *sent = Some(Instant::now());
                        self.body.dialog = None;
                        self.body.send(ClientMsg::Talk { entity: e.id });
                        Outcome::Running
                    }
                    Some(s) => {
                        if self.body.dialog.is_some_and(|(_, d)| d == n) {
                            let deals = std::mem::take(deals);
                            self.do_deals(e.id, &deals);
                            Outcome::Done("talked".into())
                        } else if s.elapsed() > Duration::from_secs(3) {
                            Outcome::Failed("no answer".into())
                        } else {
                            Outcome::Running
                        }
                    }
                }
            }
            Task::Gather { node, stand, sent } => {
                let c = Vec3::new(
                    node[0] as f32 + 0.5,
                    node[1] as f32 + 0.5,
                    node[2] as f32 + 0.5,
                );
                let eye = self.body.player.eye();
                if sent.is_none() && eye.distance(c) > 6.3 {
                    match stand {
                        Some(s) => self.body.go_to(
                            Vec3::new(s[0] as f32 + 0.5, s[1] as f32, s[2] as f32 + 0.5),
                            0.45,
                        ),
                        None => self.body.go_to(Vec3::new(c.x, node[1] as f32, c.z), 2.2),
                    }
                    match self.body.nav.state.clone() {
                        NavState::Failed(why) => return Outcome::Failed(why),
                        NavState::Arrived => {
                            if self.body.player.eye().distance(c) > 6.8 {
                                return Outcome::Failed("cannot get close enough".into());
                            }
                        }
                        _ => return Outcome::Running,
                    }
                }
                self.body.stop();
                match sent {
                    None => {
                        // a cast breaks if we drift: come to rest first
                        let v = self.body.player.vel;
                        if v.x.abs() + v.z.abs() > 0.15 || !self.body.player.on_ground {
                            return Outcome::Running;
                        }
                        *sent = Some(Instant::now());
                        self.body.send(ClientMsg::Gather { pos: *node });
                        Outcome::Running
                    }
                    Some(s) => {
                        let since = s.elapsed();
                        // the answer: a gather line or a reason it failed
                        if since > Duration::from_millis(300) && !self.body.is_casting() {
                            let recent: Vec<String> = self
                                .log
                                .iter()
                                .rev()
                                .take(4)
                                .filter(|(t, _)| self.body.secs() - t < since.as_secs_f64() + 0.1)
                                .map(|(_, l)| l.clone())
                                .collect();
                            if let Some(l) = recent.iter().find(|l| l.starts_with("You gather")) {
                                return Outcome::Done(l.clone());
                            }
                            if let Some(l) = recent.iter().find(|l| {
                                l.starts_with("Requires")
                                    || l.contains("gathered recently")
                                    || l.starts_with("Too far")
                                    || l.starts_with("Needs")
                                    || l.contains("Interrupted")
                            }) {
                                return Outcome::Failed(l.clone());
                            }
                        }
                        if since > Duration::from_secs(4) {
                            Outcome::Failed("nothing happened".into())
                        } else {
                            Outcome::Running
                        }
                    }
                }
            }
            Task::Rest { until, start, ate } => {
                let hp = self.body.health / self.body.max_health.max(1.0);
                let mana_ok = self.body.max_power <= 0.0
                    || self.class_info().is_none_or(|(r, _)| r != Resource::Mana)
                    || self.body.power / self.body.max_power >= *until;
                if hp >= *until && mana_ok {
                    return Outcome::Done("rested".into());
                }
                if self.body.in_combat()
                    && self
                        .body
                        .last_hurt
                        .is_some_and(|t| t.elapsed() < Duration::from_secs(2))
                {
                    return Outcome::Failed("attacked".into());
                }
                if !*ate && !self.body.in_combat() && hp < 0.8 {
                    *ate = true;
                    if self.body.food_count() > 0 {
                        self.body.send(ClientMsg::EatBest);
                    }
                }
                // heal yourself if you can
                if hp < 0.8 && !self.body.in_combat() {
                    let b = &self.body;
                    let heal = b.class.and_then(|c| {
                        b.content
                            .known_abilities(c, b.level)
                            .into_iter()
                            .filter(|a| a.heal[0] > 0.0 && a.damage[0] == 0.0 && b.power >= a.cost)
                            .max_by(|x, y| x.heal[0].partial_cmp(&y.heal[0]).unwrap())
                            .cloned()
                    });
                    if let Some(a) = heal
                        && self.body.ready(a.id)
                    {
                        self.cast(&a, None);
                    }
                }
                if start.elapsed() > Duration::from_secs(90) {
                    return Outcome::Done("rested long enough".into());
                }
                Outcome::Running
            }
            Task::Wait { until, .. } => {
                if Instant::now() >= *until {
                    Outcome::Done("waited".into())
                } else {
                    Outcome::Running
                }
            }
            Task::Recall { sent } => match sent {
                None => {
                    *sent = Some(Instant::now());
                    self.body.stop();
                    self.body.expect_teleport(12.0);
                    self.body.send(ClientMsg::Recall);
                    *self.tel.abilities.entry("Recall".into()).or_insert(0) += 1;
                    Outcome::Running
                }
                Some(s) => {
                    if s.elapsed() > Duration::from_millis(600) && !self.body.is_casting() {
                        if s.elapsed() > Duration::from_secs(7) {
                            Outcome::Done("home".into())
                        } else {
                            Outcome::Failed("recall did not start".into())
                        }
                    } else {
                        Outcome::Running
                    }
                }
            },
            Task::Respawn { at } => {
                if self.body.dead.is_none() {
                    return Outcome::Done("alive".into());
                }
                if Instant::now() >= *at {
                    *at = Instant::now() + Duration::from_secs(3);
                    self.body.expect_teleport(5.0);
                    self.body.send(ClientMsg::Respawn);
                }
                Outcome::Running
            }
        }
    }

    /// After an NPC opened a conversation: accept, hand in, sell, buy.
    fn do_deals(&mut self, npc_ent: u32, deals: &[Deal]) {
        for d in deals {
            match d {
                Deal::Accept(q) => self.body.send(ClientMsg::AcceptQuest { quest: *q }),
                Deal::Complete(q, choice) => self.body.send(ClientMsg::CompleteQuest {
                    quest: *q,
                    choice: *choice,
                }),
                Deal::SellJunk => {
                    for s in self.body.junk_slots() {
                        self.body.send(ClientMsg::Sell { from: s as u8 });
                    }
                }
                Deal::Sell(slot) => self.body.send(ClientMsg::Sell { from: *slot }),
                Deal::BuyFood(want) => {
                    let Some(npc) = self.body.ents.get(&npc_ent).and_then(|e| e.npc()) else {
                        continue;
                    };
                    let c = &self.body.content;
                    let Some(n) = c.npc(npc) else { continue };
                    // the most filling food we can afford
                    let pick = n
                        .sells
                        .iter()
                        .filter(|(i, p)| c.item(*i).food > 0 && *p <= self.body.money)
                        .max_by_key(|(i, p)| (c.item(*i).food as u32 * 100) / (*p).max(1))
                        .copied();
                    let have = self.body.food_count();
                    if let Some((item, price)) = pick
                        && have < *want
                    {
                        let n = ((*want - have).min(self.body.money / price.max(1))).min(20) as u8;
                        if n > 0 {
                            self.body.send(ClientMsg::Buy {
                                npc: npc_ent,
                                item,
                                count: n,
                            });
                        }
                    }
                }
                Deal::Buy(item, n) => self.body.send(ClientMsg::Buy {
                    npc: npc_ent,
                    item: *item,
                    count: *n,
                }),
            }
        }
    }

    /// Wear anything in the bags that is better than what we wear.
    pub fn equip_upgrades(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        let slots: Vec<(usize, u16)> = self
            .body
            .pack
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.map(|s| (i, s.item)))
            .collect();
        let mut used = Vec::new();
        for (i, item) in slots {
            if self.body.content.item(item).gear.is_none() {
                continue;
            }
            // one item per place until the server answers
            if let Some(place) = self.body.upgrade_place(item)
                && !used.contains(&place)
            {
                used.push(place);
                self.body.send(ClientMsg::Equip { from: i as u8 });
                let name = self.body.content.item(item).name.to_string();
                let now = self.body.secs();
                self.tel.first_upgrade.get_or_insert(now);
                out.push(name.clone());
                self.say(format!("Equipped {name}"));
            }
        }
        out
    }

    pub fn recall(&mut self) {
        let _ = RECALL;
        self.set(Task::Recall { sent: None });
    }

    /// "12m NE"
    pub fn where_is(&self, p: Vec3) -> String {
        format!(
            "{:.0}m {}",
            self.body.dist(p),
            compass(self.body.player.pos, p)
        )
    }
}
