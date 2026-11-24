//! Scripted brains that play like people do: follow the quest chain, fight
//! with a rotation, loot, rest, sell and buy, wear upgrades, gather, and
//! recover when stuck. Personalities change what they care about.

use crate::agent::{Agent, Deal, Outcome, Target, Task};
use crate::nav::{self, Cell};
use dyno_protocol::{EntityKind, RECALL};
use dyno_world::content::{Objective, Spawn};
use dyno_world::{self as w, World};
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Personality {
    /// every quest, in order
    Completionist,
    /// kills: kill quests and grinding
    Grinder,
    /// sees every area, quests on the way
    Explorer,
    /// stays with other players, helps them, talks
    Social,
    /// short sessions, wanders off, takes breaks
    Casual,
    /// quests in a party of two: a leader and a follower who share kills
    Party,
}

impl Personality {
    pub const ALL: [Personality; 6] = [
        Personality::Completionist,
        Personality::Grinder,
        Personality::Explorer,
        Personality::Social,
        Personality::Casual,
        Personality::Party,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Personality::Completionist => "completionist",
            Personality::Grinder => "grinder",
            Personality::Explorer => "explorer",
            Personality::Social => "social",
            Personality::Casual => "casual",
            Personality::Party => "party",
        }
    }

    pub fn parse(s: &str) -> Option<Personality> {
        Personality::ALL
            .into_iter()
            .find(|p| p.key().starts_with(&s.to_ascii_lowercase()))
    }

    /// The i-th bot of a mixed swarm: completionists (the solo baseline),
    /// grinders, explorers, social and casual players, and a party of two.
    pub fn mix(i: usize) -> Personality {
        const ORDER: [Personality; 10] = [
            Personality::Completionist,
            Personality::Grinder,
            Personality::Explorer,
            Personality::Completionist,
            Personality::Social,
            Personality::Casual,
            Personality::Completionist,
            Personality::Grinder,
            Personality::Party,
            Personality::Party,
        ];
        ORDER[i % ORDER.len()]
    }
}

/// Small deterministic random numbers (xorshift).
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.max(1) ^ 0x9e37_79b9_7f4a_7c15)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }
    pub fn pick<T: Copy>(&mut self, v: &[T]) -> Option<T> {
        if v.is_empty() {
            None
        } else {
            Some(v[(self.next_u64() % v.len() as u64) as usize])
        }
    }
}

/// Resource nodes in the world round where we stood when we looked (blocks
/// that regrow after gathering). The world is streamed, so the bot only
/// holds what is near it: the list is made again as it moves on.
#[derive(Clone, Debug, Default)]
pub struct Nodes {
    pub by_block: HashMap<u8, Vec<Cell>>,
    /// where we stood (x, z)
    pub at: [i32; 2],
}

impl Nodes {
    /// how far round the bot the nodes are listed, and how far it may walk
    /// before the list is made again
    const REACH: i32 = 112;
    const STALE: i32 = 48;

    pub fn scan(world: &World, content: &dyno_world::content::Content, me: Vec3) -> Nodes {
        let mut by_block: HashMap<u8, Vec<Cell>> = HashMap::new();
        let gatherable: Vec<bool> = (0..=255u8)
            .map(|b| (b as usize) < content.blocks.len() && content.rule(b).respawn > 0)
            .collect();
        let [sx, sy, sz] = world.size;
        let at = [me.x.floor() as i32, me.z.floor() as i32];
        for z in (at[1] - Self::REACH).max(0)..(at[1] + Self::REACH).min(sz as i32) {
            for x in (at[0] - Self::REACH).max(0)..(at[0] + Self::REACH).min(sx as i32) {
                for y in 0..sy as i32 {
                    let b = world.get(x, y, z);
                    if b != w::AIR && gatherable[b as usize] {
                        by_block.entry(b).or_default().push([x, y, z]);
                    }
                }
            }
        }
        Nodes { by_block, at }
    }

    /// Have we walked far enough that the list should be made again?
    pub fn stale(&self, me: Vec3) -> bool {
        (me.x.floor() as i32 - self.at[0]).abs() > Self::STALE
            || (me.z.floor() as i32 - self.at[1]).abs() > Self::STALE
    }

    /// The list for where the bot stands now.
    pub fn fresh(
        nodes: &mut Option<Nodes>,
        world: &World,
        content: &dyno_world::content::Content,
        me: Vec3,
    ) {
        if nodes.as_ref().is_none_or(|n| n.stale(me)) {
            *nodes = Some(Nodes::scan(world, content, me));
        }
    }
}

/// Can we walk from `from` to stand at `to`?
pub fn reachable(world: &World, from: Vec3, to: Cell) -> bool {
    let Some(start) = nav::node_at(world, from.into()) else {
        return false;
    };
    let goal = nav::Goal::new([to[0] as f32 + 0.5, to[1] as f32, to[2] as f32 + 0.5], 0.6);
    nav::find_path(world, start, goal, 30_000).is_some_and(|p| !p.partial)
}

/// A place to stand to gather `node` (eye within reach), if there is one.
pub fn stand_for(world: &World, node: Cell) -> Option<Cell> {
    let c = Vec3::new(
        node[0] as f32 + 0.5,
        node[1] as f32 + 0.5,
        node[2] as f32 + 0.5,
    );
    let mut best: Option<(f32, Cell)> = None;
    for dz in -2..=2 {
        for dx in -2..=2 {
            for dy in -3..=4 {
                let s = [node[0] + dx, node[1] + dy, node[2] + dz];
                if !nav::standable(world, s) {
                    continue;
                }
                let eye = Vec3::new(s[0] as f32 + 0.5, s[1] as f32 + 1.62, s[2] as f32 + 0.5);
                let d = eye.distance(c);
                // stand on open ground, not in a cave under it
                let open = s[1] > crate::body::ground(world, s[0], s[2]);
                if open && d <= 6.0 && best.is_none_or(|(b, _)| d < b) {
                    best = Some((d, s));
                }
            }
        }
    }
    best.map(|(_, s)| s)
}

pub struct Scripted {
    pub kind: Personality,
    pub rng: Rng,
    nodes: Option<Nodes>,
    avoid_ent: HashMap<u32, Instant>,
    avoid_node: HashMap<Cell, Instant>,
    bad_npc: HashMap<u16, u32>,
    /// (quest, objective) -> why, and when to try again
    blocked: HashMap<(u16, usize), (String, Instant)>,
    /// items we cannot gather (and why)
    no_gather: HashMap<u16, String>,
    session_end: Option<Instant>,
    next_chat: Instant,
    nav_fails: u32,
    probed_party: bool,
    pub quit: bool,
    /// what the current task is for
    purpose: Purpose,
    last_node: Option<Cell>,
    /// in a party of two (Party brains)
    pub party: Option<PartyRole>,
    next_invite: Instant,
    /// camps we reached and found empty (we only see creatures near us), and when
    empty_camps: Vec<([i32; 2], Instant)>,
    /// when we may next try a cart ride
    next_ride: Instant,
    /// when we may next invite someone who looks for a group
    next_lfg: Instant,
}

/// What a party leader is doing, for its follower to copy.
#[derive(Clone, Debug, Default)]
pub struct LeaderView {
    pub pos: Vec3,
    pub fight: Option<u32>,
    pub talk: Option<u16>,
    pub resting: bool,
    /// an NPC the follower has quest business with (the leader goes there too)
    pub follower_npc: Option<u16>,
}

pub type Link = std::sync::Arc<std::sync::Mutex<LeaderView>>;

/// A bot's place in a party of two.
#[derive(Clone, Debug)]
pub struct PartyRole {
    pub leader: bool,
    /// the other member's name (for a follower, the leader)
    pub mate: String,
    /// a leader's further members
    pub others: Vec<String>,
    pub link: Link,
}

#[derive(Clone, Debug, Default, PartialEq)]
enum Purpose {
    #[default]
    None,
    Objective(u16, usize),
    Npc(u16),
    Explore(String),
}

impl Scripted {
    pub fn new(kind: Personality, seed: u64) -> Scripted {
        let mut rng = Rng::new(seed);
        let session_end = (kind == Personality::Casual)
            .then(|| Instant::now() + Duration::from_secs_f32(rng.range(300.0, 900.0)));
        Scripted {
            kind,
            rng,
            nodes: None,
            avoid_ent: HashMap::new(),
            avoid_node: HashMap::new(),
            bad_npc: HashMap::new(),
            blocked: HashMap::new(),
            no_gather: HashMap::new(),
            session_end,
            next_chat: Instant::now() + Duration::from_secs(20),
            nav_fails: 0,
            probed_party: false,
            quit: false,
            purpose: Purpose::None,
            last_node: None,
            party: None,
            next_invite: Instant::now() + Duration::from_secs(3),
            empty_camps: Vec::new(),
            next_ride: Instant::now(),
            next_lfg: Instant::now(),
        }
    }

    /// A party member: the leader invites, the follower accepts and copies.
    pub fn with_party(mut self, role: PartyRole) -> Scripted {
        self.party = Some(role);
        self
    }

    fn in_party(&self, a: &Agent) -> bool {
        self.party
            .as_ref()
            .is_some_and(|r| a.body.party.iter().any(|m| m.eq_ignore_ascii_case(&r.mate)))
    }

    fn mate_pos(&self, a: &Agent) -> Option<Vec3> {
        let r = self.party.as_ref()?;
        a.body
            .players
            .values()
            .find(|(n, _)| n.eq_ignore_ascii_case(&r.mate))
            .map(|(_, p)| *p)
    }

    /// Deals to do at one NPC: hand in what is ready there, take what is offered.
    fn deals_at(&self, a: &Agent, npc: u16) -> Vec<Deal> {
        let b = &a.body;
        let mut out: Vec<Deal> = b
            .quests
            .iter()
            .filter(|q| {
                a.quest_ready(q.quest) && b.content.quest(q.quest).is_some_and(|x| x.ender == npc)
            })
            .map(|q| Deal::Complete(q.quest, a.best_choice(q.quest)))
            .collect();
        out.extend(
            b.content
                .quests
                .iter()
                .filter(|q| q.giver == npc && a.can_take(q.id))
                .map(|q| Deal::Accept(q.id)),
        );
        out
    }

    /// Party upkeep every tick. Returns true when it set the task.
    fn party_tick(&mut self, a: &mut Agent) -> bool {
        let Some(role) = self.party.clone() else {
            return false;
        };
        let now = Instant::now();
        let grouped = self.in_party(a);
        if grouped {
            a.tel.note("grouped in a party of two");
        }
        if role.leader {
            if !grouped && now >= self.next_invite && self.mate_pos(a).is_some() {
                self.next_invite = now + Duration::from_secs(15);
                a.body.say(&format!("/invite {}", role.mate));
            } else if grouped && now >= self.next_invite {
                // a party of three or more: one invitation at a time
                let missing = role
                    .others
                    .iter()
                    .find(|m| !a.body.party.iter().any(|p| p.eq_ignore_ascii_case(m)));
                if let Some(m) = missing {
                    self.next_invite = now + Duration::from_secs(8);
                    a.body.say(&format!("/invite {m}"));
                }
            }
            // publish what we do
            let mut v = role.link.lock().unwrap();
            v.pos = a.body.pos();
            v.fight = match &a.task {
                Some(Task::Fight { target, .. }) => Some(*target),
                _ => None,
            };
            v.talk = match &a.task {
                Some(Task::Talk { npc, .. }) => Some(*npc),
                _ => None,
            };
            v.resting = matches!(a.task, Some(Task::Rest { .. }));
            drop(v);
            // do not run off: wait for the follower to catch up
            if grouped
                && matches!(a.task, Some(Task::Goto { .. }) | Some(Task::Gather { .. }))
                && self.mate_pos(a).is_some_and(|p| a.body.dist(p) > 28.0)
            {
                a.doing = format!("waiting for {}", role.mate);
                a.set(Task::wait(1.5, false));
                return true;
            }
            return false;
        }
        // the follower
        if a.body.invite.take().is_some() {
            a.body.say("/accept");
        }
        let want = self.wanted_npc(a);
        role.link.lock().unwrap().follower_npc = want;
        let v = role.link.lock().unwrap().clone();
        if let Some(t) = v.fight
            && !matches!(&a.task, Some(Task::Fight { target, .. }) if *target == t)
            && !matches!(a.task, Some(Task::Loot { .. }))
            && let Some(e) = a.body.ents.get(&t)
            && let Some(c) = e.creature()
            && a.body.dist(e.pos) < 40.0
        {
            let lvl = e.level;
            a.doing = format!("helping {} fight {}", role.mate, a.cname(c));
            a.set(Task::fight(t, c, lvl));
            return true;
        }
        if let Some(npc) = v.talk
            && !matches!(a.task, Some(Task::Talk { .. }) | Some(Task::Fight { .. }))
        {
            let deals = self.deals_at(a, npc);
            if !deals.is_empty() {
                a.doing = format!("with {} at an NPC", role.mate);
                self.purpose = Purpose::Npc(npc);
                a.set(Task::talk(npc, deals));
                return true;
            }
        }
        false
    }

    /// Looking for group, as a player would use it: someone who carries the
    /// same group quest stands near (the server says who, `Seekers`), so
    /// invite them, or accept their invitation. Grouped for it, the one
    /// invited follows the leader and helps until the quest is done; then
    /// the party breaks up. Nobody waits for a partner: it only happens
    /// when two meet. Returns true when it set the task.
    fn lfg_tick(&mut self, a: &mut Agent) -> bool {
        if self.party.is_some() {
            return false; // a party brain has its partner already
        }
        let now = Instant::now();
        let b = &a.body;
        // the group quests we carry unfinished
        let open: Vec<u16> = b
            .quests
            .iter()
            .filter(|q| {
                b.content.quest(q.quest).is_some_and(|x| x.group > 1) && !a.quest_ready(q.quest)
            })
            .map(|q| q.quest)
            .collect();
        let me = b.pos();
        let near_seekers: Vec<String> = open
            .iter()
            .filter_map(|q| b.seekers.get(q))
            .filter(|(t, _)| t.elapsed() < Duration::from_secs(3))
            .flat_map(|(_, list)| list.iter())
            .filter(|s| Vec3::from(s.pos).distance(me) < 60.0)
            .map(|s| s.name.clone())
            .collect();
        if b.party.is_empty() {
            if let Some(from) = a.body.invite.clone()
                && near_seekers.iter().any(|n| n.eq_ignore_ascii_case(&from))
            {
                a.body.invite = None;
                a.body.say("/accept");
                a.tel.note(format!("looking for group: joined {from}"));
                return false;
            }
            // the one first by name asks, so two do not ask each other
            if now >= self.next_lfg
                && let Some(who) = near_seekers
                    .iter()
                    .filter(|n| n.to_lowercase() > a.body.name.to_lowercase())
                    .min()
            {
                self.next_lfg = now + Duration::from_secs(15);
                a.body.say(&format!("/invite {who}"));
                a.tel.note(format!("looking for group: invited {who}"));
            }
            return false;
        }
        // grouped: done with the group quest, or it was never ours? part
        if open.is_empty() {
            if now >= self.next_lfg {
                self.next_lfg = now + Duration::from_secs(15);
                a.body.say("/leave");
            }
            return false;
        }
        // the leader goes about it; the other keeps beside the leader
        let leader = a.body.party.first().cloned().unwrap_or_default();
        if leader.eq_ignore_ascii_case(&a.body.name) {
            return false;
        }
        let Some(at) = a
            .body
            .players
            .values()
            .find(|(n, _)| n.eq_ignore_ascii_case(&leader))
            .map(|(_, p)| *p)
        else {
            return false;
        };
        if matches!(a.task, Some(Task::Fight { .. }) | Some(Task::Loot { .. }) | Some(Task::Rest { .. })) {
            return false;
        }
        // the leader's fight: a creature beside them that was just hit
        let help = a
            .body
            .creatures()
            .filter(|e| {
                e.pos.distance(at) < 8.0
                    && e.hurt_at
                        .is_some_and(|t| t.elapsed() < Duration::from_secs(3))
            })
            .min_by(|x, y| x.pos.distance(at).total_cmp(&y.pos.distance(at)))
            .and_then(|e| e.creature().map(|c| Task::fight(e.id, c, e.level)));
        if let Some(t) = help {
            a.doing = format!("helping {leader}");
            a.set(t);
            return true;
        }
        let following = matches!(&a.task, Some(Task::Goto { why, .. }) if why == "following");
        if a.body.dist(at) > 9.0 && !following {
            a.doing = format!("following {leader} for a group quest");
            a.set(Task::Goto {
                to: Target::Pos(at),
                range: 4.0,
                why: "following".into(),
            });
            return true;
        }
        false
    }

    /// The nearest NPC we have quest business with.
    fn wanted_npc(&self, a: &Agent) -> Option<u16> {
        let me = a.body.pos();
        a.body
            .content
            .npcs
            .iter()
            .filter(|n| self.bad_npc.get(&n.id).copied().unwrap_or(0) < 3)
            .filter(|n| !self.deals_at(a, n.id).is_empty() || talk_pending(a, n.id))
            .filter_map(|n| a.body.npc_pos(n.id).map(|p| (n.id, p.distance(me))))
            .min_by(|x, y| x.1.partial_cmp(&y.1).unwrap())
            .map(|x| x.0)
    }

    /// The leader takes the follower to an NPC it needs (if it needs one).
    fn lead_choice(&mut self, a: &mut Agent) -> Option<Task> {
        let role = self.party.clone()?;
        if !role.leader || !self.in_party(a) {
            return None;
        }
        let npc = role.link.lock().unwrap().follower_npc?;
        // already there: the follower can do its business
        if a.body.npc_pos(npc).is_none_or(|p| a.body.dist(p) < 8.0) {
            return None;
        }
        let name = a.body.content.npc(npc)?.name.clone();
        a.doing = format!("taking {} to {name}", role.mate);
        self.purpose = Purpose::Npc(npc);
        Some(Task::talk(npc, self.deals_at(a, npc)))
    }

    /// The follower's own choices: stay close, rest with the leader.
    fn follow_choice(&mut self, a: &mut Agent) -> Option<Option<Task>> {
        let role = self.party.clone()?;
        if role.leader {
            return None;
        }
        let v = role.link.lock().unwrap().clone();
        let lead = self.mate_pos(a).unwrap_or(v.pos);
        // our own quest business first (the leader comes along)
        if let Some(npc) = self.wanted_npc(a) {
            let deals = self.deals_at(a, npc);
            a.doing = format!(
                "quest business with {}",
                a.body
                    .content
                    .npc(npc)
                    .map(|n| n.name.as_str())
                    .unwrap_or("?")
            );
            self.purpose = Purpose::Npc(npc);
            return Some(Some(Task::talk(npc, deals)));
        }
        if a.body.dist(lead) > 9.0 {
            a.doing = format!("following {}", role.mate);
            return Some(Some(Task::Goto {
                to: Target::Pos(lead),
                range: 4.0,
                why: "following".into(),
            }));
        }
        if v.resting {
            a.doing = "resting with the party".into();
            return Some(Some(Task::rest(0.95)));
        }
        // gather for our own collect quests only near the leader
        if let Some(t) = self.objective(a)
            && matches!(t, Task::Gather { .. })
        {
            return Some(Some(t));
        }
        a.doing = format!("with {}", role.mate);
        Some(Some(Task::wait(1.0, false)))
    }

    fn avoided(&self, id: u32) -> bool {
        self.avoid_ent.get(&id).is_some_and(|t| *t > Instant::now())
    }

    /// Decide what to do next. Call every tick.
    pub fn think(&mut self, a: &mut Agent) {
        let now = Instant::now();
        if let Some(end) = self.session_end
            && now >= end
            && !matches!(a.task, Some(Task::Fight { .. }))
        {
            self.quit = true;
            a.tel.note("casual session ended");
            return;
        }
        self.learn(a);
        if a.body.dead.is_some() {
            if !matches!(a.task, Some(Task::Respawn { .. })) {
                a.set(Task::Respawn {
                    at: now + Duration::from_secs(3),
                });
            }
            return;
        }
        self.chatter(a);
        if self.party_tick(a) {
            return;
        }
        if self.lfg_tick(a) {
            return;
        }
        // being hit: fight back (unless already fighting)
        let attacked = a
            .body
            .last_hurt
            .is_some_and(|t| t.elapsed() < Duration::from_millis(1500));
        let fighting = matches!(a.task, Some(Task::Fight { .. }));
        if attacked && !fighting {
            let who = a
                .attacker
                .filter(|id| a.body.ents.contains_key(id))
                .or_else(|| a.guess_attacker());
            if let Some(id) = who
                && let Some(e) = a.body.ents.get(&id)
                && let Some(c) = e.creature()
            {
                let lvl = e.level;
                a.doing = format!("defending against {}", a.cname(c));
                a.set(Task::fight(id, c, lvl));
                return;
            }
        }
        if a.task.is_some() {
            // social: drop what you do to help a friend in a fight
            if self.kind == Personality::Social
                && matches!(a.task, Some(Task::Goto { .. }) | Some(Task::Wait { .. }))
                && ready_to_pull(a)
                && let Some(t) = self.assist_target(a)
            {
                a.set(t);
            }
            return;
        }
        if let Some(t) = self.choose(a) {
            a.set(t);
        }
    }

    /// Learn from how the last task ended.
    fn learn(&mut self, a: &mut Agent) {
        let Some((name, out)) = a.last.take() else {
            return;
        };
        let now = Instant::now();
        let t = a.body.secs();
        let Outcome::Failed(why) = out else {
            self.nav_fails = 0;
            if self.kind == Personality::Casual && self.rng.f32() < 0.06 {
                // a break: make tea, read the chat
                a.set(Task::wait(self.rng.range(10.0, 45.0), false));
                a.doing = "taking a break".into();
            }
            return;
        };
        if why.contains("stuck")
            || why.contains("no path")
            || why.contains("repaths")
            || why.contains("reach")
        {
            self.nav_fails += 1;
        }
        match (name.as_str(), &self.purpose) {
            ("fight", _) => {
                if let Some(id) = a.last_fight.take() {
                    self.avoid_ent.insert(id, now + Duration::from_secs(60));
                }
            }
            ("talk", Purpose::Npc(_)) | ("talk", Purpose::Objective(_, _))
                if why.contains("reach") || why.contains("not here") =>
            {
                let n = match self.purpose {
                    Purpose::Npc(n) => n,
                    _ => u16::MAX,
                };
                let c = self.bad_npc.entry(n).or_insert(0);
                *c += 1;
                if let Some(npc) = a.body.content.npc(n) {
                    let s = format!("{} ({why})", npc.name);
                    if *c >= 2 && !a.tel.unreachable.contains(&s) {
                        a.tel.unreachable.push(s);
                    }
                }
            }
            _ => {}
        }
        if let Purpose::Objective(q, i) = self.purpose {
            let key = a.qkey(q);
            let retry = if why.contains("Requires") || why.starts_with("Needs") {
                Duration::from_secs(600)
            } else {
                Duration::from_secs(20)
            };
            if name == "gather" {
                if let Some(n) = self.last_node {
                    // a node we could not get to or use: leave it for good
                    self.avoid_node.insert(n, now + Duration::from_secs(3600));
                }
                if why.starts_with("Requires") || why.starts_with("Needs") {
                    if let Some(item) = self.objective_item(a, q, i) {
                        self.no_gather.insert(item, why.clone());
                    }
                    a.tel.blocker(t, &key, "gather", &why);
                }
            } else if name == "goto" || name == "talk" {
                a.tel.blocker(t, &key, &format!("{name} failed"), &why);
            }
            self.blocked.insert((q, i), (why, now + retry));
        }
        // stuck again and again: recall home, like a player would
        if self.nav_fails >= 5 {
            self.nav_fails = 0;
            a.tel.note("recalled home to get unstuck");
            a.tel.stuck.push(crate::telemetry::StuckAt {
                t,
                at: rel(a, a.body.pos()),
                goal: [0, 0, 0],
                why: "gave up: recall".into(),
                doing: a.doing.clone(),
            });
            a.set(Task::Recall { sent: None });
            a.doing = "recalling home".into();
        }
    }

    fn objective_item(&self, a: &Agent, q: u16, i: usize) -> Option<u16> {
        match a.body.content.quest(q)?.objectives.get(i)? {
            Objective::Collect { item, .. } => Some(*item),
            _ => None,
        }
    }

    fn chatter(&mut self, a: &mut Agent) {
        if self.kind != Personality::Social || Instant::now() < self.next_chat {
            return;
        }
        self.next_chat = Instant::now() + Duration::from_secs_f32(self.rng.range(60.0, 150.0));
        let buddy = nearest_player(a, 60.0).map(|(n, _)| n);
        if !self.probed_party
            && let Some(b) = &buddy
        {
            // do parties exist? (the answer goes to the notes)
            self.probed_party = true;
            a.body.say(&format!("/invite {b}"));
            a.tel.note(format!("tried /invite {b}"));
            return;
        }
        let lines = [
            "anyone doing the grazer quest?",
            "where do I find coal?",
            "lol that boar charged me",
            "need help with anything?",
            "this meadow is crowded",
        ];
        let i = (self.rng.next_u64() % lines.len() as u64) as usize;
        a.body.say(lines[i]);
    }

    fn assist_target(&self, a: &Agent) -> Option<Task> {
        let (_, at) = nearest_player(a, 40.0)?;
        let b = &a.body;
        b.creatures()
            .filter(|e| {
                e.pos.distance(at) < 7.0
                    && e.hurt_at
                        .is_some_and(|t| t.elapsed() < Duration::from_secs(3))
                    && e.level <= b.level + 2
                    && !self.avoided(e.id)
            })
            .min_by(|x, y| x.pos.distance(at).partial_cmp(&y.pos.distance(at)).unwrap())
            .and_then(|e| e.creature().map(|c| Task::fight(e.id, c, e.level)))
    }

    /// Pick the next task.
    fn choose(&mut self, a: &mut Agent) -> Option<Task> {
        self.purpose = Purpose::None;
        let now = Instant::now();
        let b = &a.body;
        // 1. loot what we killed
        let corpse = b
            .my_corpses
            .iter()
            .rev()
            .filter(|(id, t)| t.elapsed() < Duration::from_secs(100) && !self.avoided(*id))
            .filter_map(|(id, _)| b.ents.get(id))
            .find(|e| {
                matches!(e.kind, EntityKind::Corpse { loot: true, .. }) && b.dist3(e.pos) < 30.0
            })
            .map(|e| e.id);
        if let Some(id) = corpse {
            // one try each: if it is not ours after all, leave it
            self.avoid_ent.insert(id, now + Duration::from_secs(200));
            a.doing = "looting".into();
            return Some(Task::Loot {
                corpse: id,
                sent: None,
            });
        }
        // 2. rest when hurt or out of mana, before anything else is pulled:
        // a person eats before the next pull. The combat flag outlasts a
        // kill by a few seconds, so rest as soon as nothing is hitting us
        // (round 2: 28 of the grinders' 35 deaths were pulls the bot chose
        // under 60% health while that flag was up)
        if !ready_to_pull(a) {
            a.doing = "resting".into();
            return Some(Task::rest(0.95));
        }
        // 3. wear upgrades
        if !a.equip_upgrades().is_empty() {
            return Some(Task::wait(0.3, false));
        }
        // 4. a follower stays with the leader; a leader brings the follower along
        if let Some(t) = self.follow_choice(a) {
            return t;
        }
        if let Some(t) = self.lead_choice(a) {
            return Some(t);
        }
        // help a friend
        if self.kind == Personality::Social
            && let Some(t) = self.assist_target(a)
        {
            a.doing = "helping another player".into();
            return Some(t);
        }
        // 5. hand in finished quests, take new ones
        if let Some(t) = self.npc_visit(a) {
            return Some(t);
        }
        // 6. sell junk and buy food when the bags are full or the food is gone
        if let Some(t) = self.vendor(a) {
            return Some(t);
        }
        // 7. explorers look around first
        if self.kind == Personality::Explorer
            && self.rng.f32() < 0.35
            && let Some(t) = self.explore(a)
        {
            return Some(t);
        }
        // 8. quest objectives
        if (self.kind != Personality::Grinder || self.rng.f32() < 0.7)
            && let Some(t) = self.objective(a)
        {
            if let Some(r) = self.ride_for(a, &t) {
                return Some(r);
            }
            return Some(t);
        }
        // 9. nothing to do for quests: explore or grind
        if matches!(self.kind, Personality::Explorer | Personality::Casual)
            && let Some(t) = self.explore(a)
        {
            return Some(t);
        }
        if let Some(t) = self.grind(a) {
            return Some(t);
        }
        if let Some(t) = self.explore(a) {
            return Some(t);
        }
        a.doing = "nothing to do".into();
        Some(Task::wait(5.0, false))
    }

    /// Visit an NPC to hand in quests and take new ones.
    fn npc_visit(&mut self, a: &mut Agent) -> Option<Task> {
        let b = &a.body;
        let mut visits: HashMap<u16, Vec<Deal>> = HashMap::new();
        for q in &b.quests {
            if a.quest_ready(q.quest)
                && let Some(quest) = b.content.quest(q.quest)
            {
                visits
                    .entry(quest.ender)
                    .or_default()
                    .push(Deal::Complete(q.quest, a.best_choice(q.quest)));
            }
        }
        let take_all = self.kind != Personality::Grinder;
        for quest in &b.content.quests {
            if !a.can_take(quest.id) {
                continue;
            }
            let kill_only = quest
                .objectives
                .iter()
                .all(|o| matches!(o, Objective::Kill { .. } | Objective::Talk { .. }));
            if take_all || kill_only {
                visits
                    .entry(quest.giver)
                    .or_default()
                    .push(Deal::Accept(quest.id));
            }
        }
        // handing in first, then accepting what that unlocks (a second visit)
        let me = b.pos();
        let best = visits
            .into_iter()
            .filter(|(n, _)| self.bad_npc.get(n).copied().unwrap_or(0) < 3)
            .filter_map(|(n, deals)| b.npc_pos(n).map(|p| (n, deals, p.distance(me))))
            .min_by(|x, y| {
                // hand-ins beat pick-ups at similar distance
                let wx = x.2
                    - 30.0
                        * x.1
                            .iter()
                            .filter(|d| matches!(d, Deal::Complete(..)))
                            .count() as f32;
                let wy = y.2
                    - 30.0
                        * y.1
                            .iter()
                            .filter(|d| matches!(d, Deal::Complete(..)))
                            .count() as f32;
                wx.partial_cmp(&wy).unwrap()
            })?;
        let (npc, mut deals, d) = best;
        // a far trip to town: recall if we can
        if d > 110.0 && self.can_recall(a) {
            let p = a.body.npc_pos(npc)?;
            let home = a.body.rel([0, 0, 0]);
            if p.distance(home) < 40.0 {
                a.doing = "recalling to town".into();
                return Some(Task::Recall { sent: None });
            }
        }
        // a long way to a hub we have walked to before: take the cart
        if d > 220.0
            && let Some(p) = a.body.npc_pos(npc)
            && let Some(t) = self.ride(a, p)
        {
            return Some(t);
        }
        let b = &a.body;
        let n = b.content.npc(npc)?;
        if !n.sells.is_empty() || b.free_slots() < 8 {
            deals.push(Deal::SellJunk);
        }
        if !n.sells.is_empty() {
            deals.push(Deal::BuyFood(6));
        }
        let name = n.name.clone();
        let what: Vec<String> = deals
            .iter()
            .filter_map(|d| match d {
                Deal::Accept(q) => Some(format!("accept {}", a.qkey(*q))),
                Deal::Complete(q, _) => Some(format!("hand in {}", a.qkey(*q))),
                _ => None,
            })
            .collect();
        a.doing = format!("visiting {name}: {}", what.join(", "));
        self.purpose = Purpose::Npc(npc);
        Some(Task::talk(npc, deals))
    }

    /// A cart ride towards where a task is taking us, if that is far.
    fn ride_for(&mut self, a: &mut Agent, t: &Task) -> Option<Task> {
        let to = match t {
            Task::Goto {
                to: Target::Pos(p), ..
            } => *p,
            Task::Goto {
                to: Target::Npc(n), ..
            }
            | Task::Talk { npc: n, .. } => a.body.npc_pos(*n)?,
            _ => return None,
        };
        if a.body.pos().distance(to) <= 220.0 {
            return None;
        }
        self.ride(a, to)
    }

    /// A carter within reach who sells a ride to a place we have found, at a
    /// price we can pay, that saves 200 m or more of the way to `to`: go
    /// and buy it.
    fn ride(&mut self, a: &mut Agent, to: Vec3) -> Option<Task> {
        let b = &a.body;
        let now = Instant::now();
        if now < self.next_ride || b.in_combat() {
            return None;
        }
        let me = b.pos();
        let afoot = me.distance(to);
        let mut best: Option<(f32, u16, u16, String)> = None;
        for n in &b.content.npcs {
            let Some(at) = b.npc_pos(n.id) else { continue };
            let walk = at.distance(me);
            if walk > 90.0 {
                continue;
            }
            for (item, price) in &n.sells {
                let it = b.content.item(*item);
                let Some(r) = a.ride_of(*item) else { continue };
                // found on foot: we were told its name when we walked in
                let found = r.area.as_ref().is_none_or(|key| {
                    b.content
                        .areas
                        .iter()
                        .find(|ar| &ar.key == key)
                        .is_some_and(|ar| a.tel.areas.contains(&ar.name))
                });
                let off = b.rel2(r.at).distance(to);
                if found
                    && walk + off + 200.0 < afoot
                    && *price + 20 <= b.money
                    && best.as_ref().is_none_or(|x| walk + off < x.0)
                {
                    best = Some((walk + off, n.id, *item, it.name.to_string()));
                }
            }
        }
        let (_, carter, item, name) = best?;
        self.next_ride = now + Duration::from_secs(90);
        a.doing = format!("taking a cart: {name}");
        self.purpose = Purpose::Npc(carter);
        Some(Task::talk(carter, vec![Deal::Buy(item, 1)]))
    }

    fn can_recall(&self, a: &Agent) -> bool {
        a.body
            .cooldowns
            .get(&RECALL)
            .is_none_or(|t| *t <= Instant::now())
            && !a.body.in_combat()
    }

    fn vendor(&mut self, a: &mut Agent) -> Option<Task> {
        let b = &a.body;
        let junk = b.junk_slots();
        let full = b.free_slots() < 5 && !junk.is_empty();
        let hungry = b.food_count() == 0 && b.money >= 40 && b.level >= 2;
        if !full && !hungry {
            return None;
        }
        let me = b.pos();
        let (npc, p) = b
            .content
            .npcs
            .iter()
            .filter(|n| !n.sells.is_empty() && self.bad_npc.get(&n.id).copied().unwrap_or(0) < 3)
            .filter_map(|n| b.npc_ent(n.id).map(|e| (n.id, e.pos)))
            .min_by(|x, y| x.1.distance(me).partial_cmp(&y.1.distance(me)).unwrap())?;
        if p.distance(me) > 160.0 {
            return None;
        }
        a.doing = format!(
            "{} at {}",
            if full { "selling junk" } else { "buying food" },
            b.content.npc(npc).map(|n| n.name.as_str()).unwrap_or("?")
        );
        self.purpose = Purpose::Npc(npc);
        Some(Task::talk(npc, vec![Deal::SellJunk, Deal::BuyFood(8)]))
    }

    /// Work on a quest objective.
    fn objective(&mut self, a: &mut Agent) -> Option<Task> {
        let now = Instant::now();
        let mut options: Vec<(f32, Task, String, (u16, usize))> = Vec::new();
        let log = a.body.quests.clone();
        for qp in &log {
            let Some(quest) = a.body.content.quest(qp.quest).cloned() else {
                continue;
            };
            if a.quest_ready(qp.quest) {
                continue;
            }
            for (i, (o, n)) in quest.objectives.iter().zip(&qp.counts).enumerate() {
                let met = match o {
                    Objective::Collect { item, count } => a.body.count(*item) >= *count as u32,
                    _ => *n >= o.need(),
                };
                if met {
                    continue;
                }
                if let Some((why, until)) = self.blocked.get(&(qp.quest, i))
                    && *until > now
                {
                    let _ = why;
                    continue;
                }
                if self.kind == Personality::Grinder
                    && !matches!(o, Objective::Kill { .. })
                    && self.rng.f32() < 0.5
                {
                    continue;
                }
                let found = match o {
                    Objective::Kill { creature, .. } => self.hunt(a, *creature, &quest.key, i),
                    Objective::Collect { item, .. } => {
                        self.collect(a, *item, &quest.key, qp.quest, i)
                    }
                    Objective::Talk { npc } => a.body.npc_pos(*npc).map(|p| {
                        (
                            a.body.dist(p),
                            Task::talk(*npc, Vec::new()),
                            format!(
                                "talk to {}",
                                a.body
                                    .content
                                    .npc(*npc)
                                    .map(|n| n.name.as_str())
                                    .unwrap_or("?")
                            ),
                        )
                    }),
                    Objective::Explore { name, at, radius } => {
                        let p = a.body.rel(*at);
                        Some((
                            a.body.dist(p),
                            Task::Goto {
                                to: Target::Pos(p),
                                range: (radius * 0.6).max(2.0),
                                why: format!("explore {name}"),
                            },
                            format!("explore {name}"),
                        ))
                    }
                };
                if let Some((d, t, what)) = found {
                    // grouped for a group quest: that first, while the party is here
                    let d = if quest.group > 1 { d * 0.25 } else { d };
                    options.push((d, t, format!("{}: {what}", quest.key), (qp.quest, i)));
                }
            }
        }
        let (_, task, what, (q, i)) = options
            .into_iter()
            .min_by(|x, y| x.0.partial_cmp(&y.0).unwrap())?;
        a.doing = what;
        self.purpose = Purpose::Objective(q, i);
        Some(task)
    }

    /// Find something of `creature` to fight, or go where they live.
    fn hunt(
        &mut self,
        a: &mut Agent,
        creature: u16,
        quest: &str,
        _i: usize,
    ) -> Option<(f32, Task, String)> {
        let b = &a.body;
        let c = b.content.creature(creature)?.clone();
        let t = b.secs();
        if needs_group(b, creature) {
            a.tel.blocker(
                t,
                quest,
                &format!("kill {}", c.name),
                "a group quest: waiting for a party",
            );
            return None;
        }
        if c.levels[0] > b.level + 3 {
            a.tel.blocker(
                t,
                quest,
                &format!("kill {}", c.name),
                "creature far above our level",
            );
            return None;
        }
        if let Some((d, task)) = self.live_target(a, |e| e.creature() == Some(creature)) {
            return Some((d, task, format!("kill {}", c.name)));
        }
        let b = &a.body;
        // none alive in sight (we hear only of what is near): go to their
        // camp; one found empty, try the next before waiting at the nearest
        let me = b.pos();
        // like a player reading the zone levels: a camp in a zone for our
        // level beats a nearer one among stronger creatures
        let camps: Vec<([i32; 2], Vec3, f32)> = b
            .content
            .areas
            .iter()
            .flat_map(|ar| ar.camps.iter().map(move |cp| (ar, cp)))
            .filter(|(ar, cp)| cp.creature == creature && camp_levels(ar, cp)[0] <= b.level + 2)
            .map(|(ar, cp)| {
                let p = b.rel2(cp.at);
                let high = camp_levels(ar, cp)[1] > b.level + 1;
                let cost = p.distance(me) + if high || dangerous(b, p) { 400.0 } else { 0.0 };
                (cp.at, p, cost)
            })
            .collect();
        self.empty_camps
            .retain(|(_, t)| t.elapsed() < Duration::from_secs(45));
        if let Some((at, ..)) = camps.iter().find(|(_, p, _)| b.dist(*p) <= 10.0)
            && !self.empty_camps.iter().any(|(c, _)| c == at)
        {
            self.empty_camps.push((*at, Instant::now()));
        }
        let fresh: Vec<&([i32; 2], Vec3, f32)> = camps
            .iter()
            .filter(|(at, ..)| !self.empty_camps.iter().any(|(c, _)| c == at))
            .collect();
        let pool: Vec<&([i32; 2], Vec3, f32)> = if fresh.is_empty() {
            camps.iter().collect()
        } else {
            fresh
        };
        if let Some((_, p, _)) = pool
            .into_iter()
            .min_by(|x, y| x.2.partial_cmp(&y.2).unwrap())
        {
            let d = b.dist(*p);
            if d > 10.0 {
                return Some((
                    d,
                    Task::Goto {
                        to: Target::Pos(*p),
                        range: 6.0,
                        why: format!("{} camp", c.name),
                    },
                    format!("go to the {} camp", c.name),
                ));
            }
            return Some((
                d,
                Task::wait(6.0, true),
                format!("wait for a {} to spawn", c.name),
            ));
        }
        if c.spawn == Spawn::Dark {
            if !b.is_night() {
                a.tel.blocker(
                    t,
                    quest,
                    &format!("kill {}", c.name),
                    "only at night: waiting for dark",
                );
                return None;
            }
            // where the rules name the dark's haunts, the nearest one for our level
            let haunt = b
                .content
                .areas
                .iter()
                .filter(|ar| ar.dark && ar.levels[0] <= b.level + 1)
                .map(|ar| (b.rel2(ar.at), ar.name.clone(), ar.radius))
                .min_by(|x, y| b.dist(x.0).total_cmp(&b.dist(y.0)));
            if let Some((p, name, radius)) = haunt {
                let d = b.dist(p);
                if d > radius * 0.5 {
                    return Some((
                        d,
                        Task::Goto {
                            to: Target::Pos(p),
                            range: radius * 0.4,
                            why: name.clone(),
                        },
                        format!("go to {name} for {}", c.name),
                    ));
                }
                return Some((
                    d,
                    Task::wait(4.0, true),
                    format!("wait for a {} to rise", c.name),
                ));
            }
            // out into the dark meadow, away from the hearth's light
            let ang = self.rng.range(0.0, std::f32::consts::TAU);
            let p = b.rel([(ang.cos() * 50.0) as i32, 0, (ang.sin() * 50.0) as i32]);
            return Some((
                40.0,
                Task::Goto {
                    to: Target::Pos(p),
                    range: 4.0,
                    why: "looking for shades".into(),
                },
                format!("look for {} in the dark", c.name),
            ));
        }
        a.tel.blocker(
            t,
            quest,
            &format!("kill {}", c.name),
            "no camp and none in sight",
        );
        None
    }

    /// The nearest live creature matching `f` that we may fight.
    fn live_target(&self, a: &Agent, f: impl Fn(&crate::body::Ent) -> bool) -> Option<(f32, Task)> {
        let b = &a.body;
        let me = b.pos();
        b.creatures()
            .filter(|e| f(e) && !self.avoided(e.id))
            .filter(|e| {
                // someone else's fight: leave it (unless it is ours)
                let theirs = e
                    .hurt_at
                    .is_some_and(|t| t.elapsed() < Duration::from_secs(8))
                    && e.my_hit
                        .is_none_or(|t| t.elapsed() > Duration::from_secs(8))
                    && e.health < e.max;
                // creatures that stand on tree tops cannot be reached (or looted)
                !theirs
                    && e.level <= b.level + 2
                    && !e.elite
                    && e.creature().is_none_or(|c| !needs_group(b, c))
                    && !dangerous(b, e.pos)
                    && b.dist(e.pos) < 140.0
                    && !crate::body::on_canopy(&b.world, e.pos)
            })
            .map(|e| (e.pos.distance(me), e))
            .min_by(|x, y| x.0.partial_cmp(&y.0).unwrap())
            .and_then(|(d, e)| e.creature().map(|c| (d, Task::fight(e.id, c, e.level))))
    }

    /// Get an item: from a node we can gather, or from creatures that drop it.
    fn collect(
        &mut self,
        a: &mut Agent,
        item: u16,
        quest: &str,
        q: u16,
        i: usize,
    ) -> Option<(f32, Task, String)> {
        let name = a.body.content.item(item).name.to_string();
        let mut best: Option<(f32, Task, String)> = None;
        if !self.no_gather.contains_key(&item) {
            match self.gather_task(a, item, quest) {
                Ok(Some((d, t))) => best = Some((d, t, format!("gather {name}"))),
                Ok(None) => {}
                Err(t) => return Some(t),
            }
        }
        // creatures that drop it
        let droppers: Vec<(u16, f32)> = a
            .body
            .content
            .creatures
            .iter()
            .filter_map(|c| {
                c.drops
                    .iter()
                    .find(|d| d.item == item)
                    .map(|d| (c.id, d.chance))
            })
            .filter(|(_, ch)| *ch >= 0.1)
            .collect();
        let lvl = a.body.level;
        // only creatures we can take on (a quest giver would not send us further)
        let droppers: Vec<(u16, f32)> = droppers
            .into_iter()
            .filter(|(c, _)| {
                a.body
                    .content
                    .creature(*c)
                    .is_some_and(|c| c.levels[0] <= lvl + 1)
            })
            .collect();
        for (c, chance) in droppers {
            if let Some((d, t, what)) = self.hunt(a, c, quest, i) {
                // a sure drop is worth a longer walk
                let d = d / chance.max(0.2);
                if best.as_ref().is_none_or(|(bd, ..)| d < *bd) {
                    best = Some((d, t, format!("{what} for {name}")));
                }
            }
        }
        if best.is_none() {
            let why = self
                .no_gather
                .get(&item)
                .cloned()
                .unwrap_or_else(|| "no source in reach".into());
            a.tel
                .blocker(a.body.secs(), quest, &format!("collect {name}"), &why);
            self.blocked
                .insert((q, i), (why, Instant::now() + Duration::from_secs(30)));
        }
        best
    }

    /// A node to gather `item` from. Err: first buy the tool for it.
    #[allow(clippy::type_complexity)]
    fn gather_task(
        &mut self,
        a: &mut Agent,
        item: u16,
        quest: &str,
    ) -> Result<Option<(f32, Task)>, (f32, Task, String)> {
        let blocks: Vec<u8> = {
            let c = &a.body.content;
            (0..c.blocks.len().min(256) as u16)
                .map(|b| b as u8)
                .filter(|b| {
                    let r = c.rule(*b);
                    r.respawn > 0
                        && (r.drops.iter().any(|d| d.item == item)
                            || (r.drops.is_empty() && *b as u16 == item))
                })
                .collect()
        };
        if blocks.is_empty() {
            return Ok(None);
        }
        Nodes::fresh(
            &mut self.nodes,
            &a.body.world,
            &a.body.content,
            a.body.pos(),
        );
        let now = Instant::now();
        let me = a.body.pos();
        let mut why_not = None;
        let mut cands: Vec<(f32, Cell)> = Vec::new();
        for blk in blocks {
            let rule = a.body.content.rule(blk).clone();
            // the skill
            if let Some((prof, n)) = &rule.skill {
                let have = a.body.skills.get(prof).copied().unwrap_or(0);
                if *n > 1 && have < *n {
                    why_not = Some(format!("Requires {prof} {n} (have {have})"));
                    // raise the skill on easier nodes of the same profession
                    let easier: Option<u16> = {
                        let c = &a.body.content;
                        (0..c.blocks.len().min(256) as u16)
                            .map(|b| b as u8)
                            .filter(|b| {
                                let r = c.rule(*b);
                                r.respawn > 0
                                    && r.skill
                                        .as_ref()
                                        .is_some_and(|(p, m)| p == prof && *m <= have.max(1))
                            })
                            .find_map(|b| {
                                let r = c.rule(b);
                                r.drops
                                    .first()
                                    .map(|d| d.item)
                                    .or(Some(b as u16))
                                    .filter(|i| *i != item)
                            })
                    };
                    if let Some(other) = easier {
                        let t = a.body.secs();
                        a.tel.blocker(
                            t,
                            quest,
                            &format!("gather {}", a.body.content.item(item).name),
                            &format!("needs {prof} {n}: raising it on easier nodes"),
                        );
                        match self.gather_task(a, other, quest) {
                            Ok(Some(x)) => return Ok(Some(x)),
                            Err(e) => return Err(e),
                            Ok(None) => {}
                        }
                    }
                    continue;
                }
            }
            // the tool: buy one if we must
            if let Some(kind) = rule.tool {
                let pack = dyno_sim::inventory::Inventory {
                    slots: a.body.pack.clone(),
                };
                let tier = dyno_sim::inventory::best_tool(&pack, &a.body.content, kind)
                    .map(|(_, t)| t.tier)
                    .unwrap_or(0);
                if tier < rule.tier {
                    let c = &a.body.content;
                    let shop = c
                        .npcs
                        .iter()
                        .flat_map(|n| n.sells.iter().map(move |(i, p)| (n.id, *i, *p)))
                        .filter(|(_, i, p)| {
                            c.item(*i)
                                .tool
                                .is_some_and(|t| t.kind == kind && t.tier >= rule.tier)
                                && *p <= a.body.money
                        })
                        .min_by_key(|(_, _, p)| *p);
                    match shop {
                        Some((npc, tool, _)) => {
                            let d = a.body.npc_pos(npc).map(|p| a.body.dist(p)).unwrap_or(99.0);
                            self.purpose = Purpose::Npc(npc);
                            return Err((
                                d,
                                Task::talk(npc, vec![Deal::Buy(tool, 1), Deal::SellJunk]),
                                format!("buy a {}", c.item(tool).name),
                            ));
                        }
                        None => {
                            let t = a.body.secs();
                            a.tel.blocker(
                                t,
                                quest,
                                &format!("gather {}", a.body.content.item(item).name),
                                &format!("needs a tier {} tool we cannot afford", rule.tier),
                            );
                            why_not = Some("needs a better tool".into());
                            continue;
                        }
                    }
                }
            }
            if let Some(list) = self.nodes.as_ref().and_then(|n| n.by_block.get(&blk)) {
                for c in list {
                    if a.body.world.get(c[0], c[1], c[2]) != blk {
                        continue; // worked out
                    }
                    if self.avoid_node.get(c).is_some_and(|t| *t > now) {
                        continue;
                    }
                    let p = Vec3::new(c[0] as f32 + 0.5, c[1] as f32, c[2] as f32 + 0.5);
                    cands.push((p.distance(me), *c));
                }
            }
        }
        cands.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());
        let mut tried = 0;
        for (d, c) in cands.into_iter().take(40) {
            let stand = stand_for(&a.body.world, c).filter(|st| {
                // a few path searches at most: stands inside sealed houses
                // and caves are common
                tried += 1;
                tried <= 8 && reachable(&a.body.world, a.body.pos(), *st)
            });
            if let Some(stand) = stand {
                // do not come back to this one soon (it rests after gathering)
                self.avoid_node.insert(c, now + Duration::from_secs(200));
                self.last_node = Some(c);
                return Ok(Some((
                    d,
                    Task::Gather {
                        node: c,
                        stand: Some(stand),
                        sent: None,
                    },
                )));
            }
            self.avoid_node.insert(c, now + Duration::from_secs(3600));
        }
        if let Some(w) = why_not {
            self.no_gather.insert(item, w);
        }
        Ok(None)
    }

    /// Fight whatever is around at our level.
    fn grind(&mut self, a: &mut Agent) -> Option<Task> {
        let lvl = a.body.level;
        let c = &a.body.content;
        let ok = |e: &crate::body::Ent| {
            e.creature()
                .and_then(|k| c.creature(k))
                .is_some_and(|_| e.level + 2 >= lvl && e.level <= lvl + 1)
        };
        if let Some((_, t)) = self.live_target(a, ok) {
            a.doing = "grinding".into();
            return Some(t);
        }
        // go to the nearest camp of something our level
        let b = &a.body;
        let me = b.pos();
        let camp = b
            .content
            .areas
            .iter()
            .flat_map(|ar| ar.camps.iter().map(move |cp| (ar, cp)))
            .filter(|(ar, cp)| {
                // by what stands there: the camp's band, not the kind's
                let band = camp_levels(ar, cp);
                band[0] <= lvl + 1
                    && band[1] + 1 >= lvl
                    && !needs_group(b, cp.creature)
            })
            .map(|(_, cp)| b.rel2(cp.at))
            .filter(|p| !dangerous(b, *p))
            .filter(|p| p.distance(me) > 12.0)
            .min_by(|x, y| x.distance(me).partial_cmp(&y.distance(me)).unwrap())?;
        a.doing = "going to grind".into();
        Some(Task::Goto {
            to: Target::Pos(camp),
            range: 6.0,
            why: "grinding camp".into(),
        })
    }

    /// Visit an area we have not seen.
    fn explore(&mut self, a: &mut Agent) -> Option<Task> {
        let b = &a.body;
        let me = b.pos();
        let lvl = b.level;
        let seen = &a.tel.areas;
        let (name, p, r) = b
            .content
            .areas
            .iter()
            .filter(|ar| !seen.contains(&ar.name) && ar.levels[0] <= lvl + 1)
            .filter(|ar| self.purpose != Purpose::Explore(ar.key.clone()))
            .map(|ar| (ar.name.clone(), b.rel2(ar.at), ar.radius))
            .filter(|(n, ..)| !self.blocked.contains_key(&(u16::MAX, hash_str(n))))
            .min_by(|x, y| x.1.distance(me).partial_cmp(&y.1.distance(me)).unwrap())?;
        a.doing = format!("exploring {name}");
        self.blocked.insert(
            (u16::MAX, hash_str(&name)),
            ("visited".into(), Instant::now() + Duration::from_secs(3600)),
        );
        Some(Task::Goto {
            to: Target::Pos(p),
            range: (r * 0.5).clamp(3.0, 12.0),
            why: format!("explore {name}"),
        })
    }
}

/// Is this place near a camp of creatures well above our level (a zone
/// for later)? Players see the levels and look for yellow ones: the top of
/// the zone's band counts (or a named creature's own level).
pub fn dangerous(b: &crate::body::Body, p: Vec3) -> bool {
    b.content.areas.iter().any(|ar| {
        ar.camps.iter().any(|cp| {
            // what stands there: the camp's band, and a group's creature
            // counts as too strong for one
            let lvl = camp_levels(ar, cp)[1];
            let hostile = b.content.creature(cp.creature).is_some_and(|c| c.hostile);
            hostile
                && (lvl > b.level + 1 || needs_group(b, cp.creature))
                && b.rel2(cp.at).distance(p) < cp.spread + 22.0
        })
    })
}

/// Health and mana at 80% or more: fit to start a fight. (Rage starts
/// empty and energy refills in seconds, so only mana is waited for.)
pub fn ready_to_pull(a: &Agent) -> bool {
    let b = &a.body;
    let mana = b
        .class
        .and_then(|c| b.content.class(c))
        .is_some_and(|c| c.resource == dyno_world::content::Resource::Mana);
    b.health / b.max_health.max(1.0) >= 0.8
        && (!mana || b.max_power <= 0.0 || b.power / b.max_power >= 0.8)
}

/// The levels a camp's creatures come at: the camp's own band, else its
/// area's (as the server spawns them; the creature kind's range is not used).
pub fn camp_levels(ar: &dyno_world::content::Area, cp: &dyno_world::content::Camp) -> [u8; 2] {
    cp.levels.unwrap_or(ar.levels)
}

/// The other members of our party standing within `r`.
fn party_near(b: &crate::body::Body, r: f32) -> usize {
    let me = b.pos();
    b.party
        .iter()
        .filter(|m| !m.eq_ignore_ascii_case(&b.name))
        .filter(|m| {
            b.players
                .values()
                .any(|(n, p)| n.eq_ignore_ascii_case(m) && p.distance(me) < r)
        })
        .count()
}

/// The group quest (`Quest::group` over 1) that asks for this creature, if any.
fn group_quest_for(b: &crate::body::Body, creature: u16) -> Option<&dyno_world::content::Quest> {
    b.content.quests.iter().find(|q| {
        q.group > 1
            && q.objectives
                .iter()
                .any(|o| matches!(o, Objective::Kill { creature: c, .. } if *c == creature))
    })
}

/// A creature made for a group (a group quest asks for it), and we are not
/// in a party of that size with the others beside us: leave it alone.
pub fn needs_group(b: &crate::body::Body, creature: u16) -> bool {
    group_quest_for(b, creature).is_some_and(|q| party_near(b, 40.0) + 1 < q.group as usize)
}

/// Does an open quest ask us to talk to this NPC?
fn talk_pending(a: &Agent, npc: u16) -> bool {
    a.body.quests.iter().any(|q| {
        a.body.content.quest(q.quest).is_some_and(|quest| {
            quest
                .objectives
                .iter()
                .zip(&q.counts)
                .any(|(o, n)| matches!(o, Objective::Talk { npc: x } if *x == npc) && *n == 0)
        })
    })
}

fn hash_str(s: &str) -> usize {
    s.bytes()
        .fold(7usize, |h, b| h.wrapping_mul(31).wrapping_add(b as usize))
}

fn rel(a: &Agent, p: Vec3) -> [i32; 3] {
    [
        p.x.floor() as i32 - a.body.hearth[0],
        p.y.floor() as i32 - a.body.hearth[1],
        p.z.floor() as i32 - a.body.hearth[2],
    ]
}

/// The nearest other player within `r`: name and position.
pub fn nearest_player(a: &Agent, r: f32) -> Option<(String, Vec3)> {
    let me = a.body.pos();
    a.body
        .players
        .values()
        .filter(|(n, p)| *n != a.body.name && p.distance(me) < r)
        .min_by(|x, y| x.1.distance(me).partial_cmp(&y.1.distance(me)).unwrap())
        .map(|(n, p)| (n.clone(), *p))
}

/// Things a brain is told it may assume about the world (for tests).
pub fn known_quests(a: &Agent) -> HashSet<u16> {
    a.body.quests.iter().map(|q| q.quest).collect()
}
