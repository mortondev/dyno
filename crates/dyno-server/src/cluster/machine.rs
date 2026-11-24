//! The cluster's replicated state: what every instance agrees on through the
//! Raft log. It is deliberately small: who is in the cluster, which
//! instance hosts which character (a lock with an epoch), and the characters
//! themselves. Applying a command must be deterministic: it depends only on
//! the command and the state, never on clocks or randomness.

use crate::store::PlayerData;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub type NodeId = u64;

/// An instance in the cluster.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Member {
    pub id: NodeId,
    /// where other instances reach its cluster link
    pub addr: String,
    pub protocol: u32,
    /// "content name vN" and the server version
    pub content: String,
    pub version: String,
    /// players it may host
    pub capacity: u32,
    /// out of contact: its characters were released
    pub down: bool,
}

/// Which instance hosts a character now, under which epoch.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Lock {
    pub node: NodeId,
    pub epoch: u64,
}

/// A character as the cluster stores it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Record {
    /// every claim and move adds one; writes must carry the current epoch
    pub epoch: u64,
    pub data: PlayerData,
}

/// A change to the cluster state, proposed through Raft.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Cmd {
    /// add or update an instance (the leader assigns new ids)
    Member(Member),
    /// remove an instance for good
    Forget { id: NodeId },
    /// an instance is out of contact: release everything it held
    Down { id: NodeId },
    /// an instance is back
    Up { id: NodeId },
    /// the base world every instance must run (sha-256, hex); set once
    WorldHash { hash: String },
    /// host a character on `node`. `fresh` is used when the name is new.
    Claim {
        name: String,
        token: u64,
        node: NodeId,
        fresh: Box<PlayerData>,
    },
    /// write a character (only the lock holder, at its epoch)
    Save {
        name: String,
        node: NodeId,
        epoch: u64,
        data: Box<PlayerData>,
    },
    /// write and let go (the player left)
    Release {
        name: String,
        node: NodeId,
        epoch: u64,
        data: Box<PlayerData>,
    },
    /// hand a character to another instance (a seamless move)
    Move {
        name: String,
        from: NodeId,
        epoch: u64,
        to: NodeId,
        data: Box<PlayerData>,
    },
    /// delete a character that is not online
    Delete { name: String, token: u64 },
    /// import characters from a single server's saves (bootstrap only)
    Import { players: Vec<(String, PlayerData)> },
}

/// What applying a command produced.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Outcome {
    Ok,
    /// the character's epoch after a claim or move, its data, and whether
    /// it was just created
    Claimed {
        epoch: u64,
        data: Box<PlayerData>,
        fresh: bool,
    },
    /// the member id assigned
    Joined {
        id: NodeId,
    },
    Refused(String),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Machine {
    pub members: BTreeMap<NodeId, Member>,
    pub next_member: NodeId,
    pub world_hash: Option<String>,
    pub locks: BTreeMap<String, Lock>,
    pub records: BTreeMap<String, Record>,
}

fn key(name: &str) -> String {
    name.to_lowercase()
}

impl Machine {
    pub fn apply(&mut self, cmd: &Cmd) -> Outcome {
        match cmd {
            Cmd::Member(m) => {
                let mut m = m.clone();
                if m.id == 0 {
                    // a new instance: the next free id (1 is always the founder)
                    let mut id = self.next_member.max(2);
                    while self.members.contains_key(&id) {
                        id += 1;
                    }
                    m.id = id;
                    self.next_member = id + 1;
                } else {
                    self.next_member = self.next_member.max(m.id + 1);
                }
                let id = m.id;
                self.members.insert(id, m);
                Outcome::Joined { id }
            }
            Cmd::Forget { id } => {
                self.members.remove(id);
                self.locks.retain(|_, l| l.node != *id);
                Outcome::Ok
            }
            Cmd::Down { id } => {
                if let Some(m) = self.members.get_mut(id) {
                    m.down = true;
                }
                // a fenced instance has already dropped its players
                self.locks.retain(|_, l| l.node != *id);
                Outcome::Ok
            }
            Cmd::Up { id } => {
                if let Some(m) = self.members.get_mut(id) {
                    m.down = false;
                }
                Outcome::Ok
            }
            Cmd::WorldHash { hash } => {
                if self.world_hash.is_none() {
                    self.world_hash = Some(hash.clone());
                }
                Outcome::Ok
            }
            Cmd::Claim {
                name,
                token,
                node,
                fresh,
            } => {
                let k = key(name);
                if self.members.get(node).is_none_or(|m| m.down) {
                    return Outcome::Refused("That server is not in the world.".into());
                }
                if let Some(l) = self.locks.get(&k)
                    && l.node != *node
                {
                    return Outcome::Refused(format!(
                        "{name} is already playing. Choose another name."
                    ));
                }
                if let Some(r) = self.records.get(&k)
                    && r.data.token != *token
                    && r.data.token != 0
                {
                    return Outcome::Refused(format!(
                        "The name {name} belongs to another player. Choose another name."
                    ));
                }
                let new = !self.records.contains_key(&k);
                let r = self.records.entry(k.clone()).or_insert_with(|| Record {
                    epoch: 0,
                    data: (**fresh).clone(),
                });
                r.epoch += 1;
                let epoch = r.epoch;
                let data = Box::new(r.data.clone());
                self.locks.insert(k, Lock { node: *node, epoch });
                Outcome::Claimed {
                    epoch,
                    data,
                    fresh: new,
                }
            }
            Cmd::Save {
                name,
                node,
                epoch,
                data,
            }
            | Cmd::Release {
                name,
                node,
                epoch,
                data,
            } => {
                let k = key(name);
                let held = self.locks.get(&k)
                    == Some(&Lock {
                        node: *node,
                        epoch: *epoch,
                    });
                if !held {
                    return Outcome::Refused(format!("stale write for {name}"));
                }
                if let Some(r) = self.records.get_mut(&k) {
                    r.data = (**data).clone();
                }
                if matches!(cmd, Cmd::Release { .. }) {
                    self.locks.remove(&k);
                }
                Outcome::Ok
            }
            Cmd::Move {
                name,
                from,
                epoch,
                to,
                data,
            } => {
                let k = key(name);
                let held = self.locks.get(&k)
                    == Some(&Lock {
                        node: *from,
                        epoch: *epoch,
                    });
                if !held {
                    return Outcome::Refused(format!("stale move for {name}"));
                }
                if self.members.get(to).is_none_or(|m| m.down) {
                    return Outcome::Refused("The target server is not in the world.".into());
                }
                let r = self
                    .records
                    .get_mut(&k)
                    .expect("locked characters have records");
                r.data = (**data).clone();
                r.epoch += 1;
                let epoch = r.epoch;
                self.locks.insert(k, Lock { node: *to, epoch });
                Outcome::Claimed {
                    epoch,
                    data: Box::new(r.data.clone()),
                    fresh: false,
                }
            }
            Cmd::Delete { name, token } => {
                let k = key(name);
                if self.locks.contains_key(&k) {
                    return Outcome::Refused("That character is playing.".into());
                }
                if self
                    .records
                    .get(&k)
                    .is_some_and(|r| r.data.token == *token && *token != 0)
                {
                    self.records.remove(&k);
                }
                Outcome::Ok
            }
            Cmd::Import { players } => {
                for (name, d) in players {
                    self.records.entry(key(name)).or_insert_with(|| Record {
                        epoch: 0,
                        data: d.clone(),
                    });
                }
                Outcome::Ok
            }
        }
    }

    /// The characters belonging to an install.
    pub fn characters_of(&self, token: u64) -> Vec<(String, PlayerData)> {
        if token == 0 {
            return Vec::new();
        }
        self.records
            .iter()
            .filter(|(_, r)| r.data.token == token)
            .map(|(k, r)| (k.clone(), r.data.clone()))
            .collect()
    }

    /// Live instances.
    pub fn alive(&self) -> BTreeSet<NodeId> {
        self.members
            .values()
            .filter(|m| !m.down)
            .map(|m| m.id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(id: NodeId) -> Cmd {
        Cmd::Member(Member {
            id,
            addr: format!("127.0.0.1:{}", 7550 + id),
            protocol: 14,
            content: "Hearthblock v8".into(),
            version: "0.6.0".into(),
            capacity: 40,
            down: false,
        })
    }

    fn data(token: u64, money: u32) -> Box<PlayerData> {
        let mut d = PlayerData::new([0.0; 3]);
        d.token = token;
        d.money = money;
        Box::new(d)
    }

    #[test]
    fn one_place_at_a_time_and_stale_writers_are_refused() {
        let mut m = Machine::default();
        assert_eq!(m.apply(&member(1)), Outcome::Joined { id: 1 });
        assert_eq!(m.apply(&member(0)), Outcome::Joined { id: 2 });
        let claim = |node| Cmd::Claim {
            name: "Ada".into(),
            token: 7,
            node,
            fresh: data(7, 0),
        };
        let Outcome::Claimed { epoch: e1, .. } = m.apply(&claim(1)) else {
            panic!()
        };
        // online on 1: node 2 cannot take her
        assert!(matches!(m.apply(&claim(2)), Outcome::Refused(_)));
        // another install cannot use the name
        assert!(matches!(
            m.apply(&Cmd::Claim {
                name: "ada".into(),
                token: 8,
                node: 1,
                fresh: data(8, 0)
            }),
            Outcome::Refused(_)
        ));
        // node 1 saves at its epoch
        let save = |node, epoch, money| Cmd::Save {
            name: "ada".into(),
            node,
            epoch,
            data: data(7, money),
        };
        assert_eq!(m.apply(&save(1, e1, 50)), Outcome::Ok);
        // she moves to node 2; node 1's later writes are stale
        let Outcome::Claimed {
            epoch: e2, data: d, ..
        } = m.apply(&Cmd::Move {
            name: "ada".into(),
            from: 1,
            epoch: e1,
            to: 2,
            data: data(7, 60),
        })
        else {
            panic!()
        };
        assert_eq!(d.money, 60);
        assert!(matches!(m.apply(&save(1, e1, 999)), Outcome::Refused(_)));
        assert_eq!(m.records["ada"].data.money, 60);
        // node 2 goes down: its locks are released; she can come back on 1
        m.apply(&Cmd::Down { id: 2 });
        assert!(matches!(m.apply(&save(2, e2, 1)), Outcome::Refused(_)));
        let Outcome::Claimed {
            epoch: e3, data, ..
        } = m.apply(&claim(1))
        else {
            panic!()
        };
        assert!(e3 > e2);
        assert_eq!(
            data.money, 60,
            "the last committed state, never a stale one"
        );
        // a down node cannot claim
        assert!(matches!(m.apply(&claim(2)), Outcome::Refused(_)));
    }

    #[test]
    fn world_hash_is_set_once_and_deletes_need_the_token() {
        let mut m = Machine::default();
        m.apply(&Cmd::WorldHash { hash: "a".into() });
        m.apply(&Cmd::WorldHash { hash: "b".into() });
        assert_eq!(m.world_hash.as_deref(), Some("a"));
        m.apply(&Cmd::Import {
            players: vec![("bo".into(), *data(3, 5))],
        });
        m.apply(&Cmd::Delete {
            name: "bo".into(),
            token: 4,
        });
        assert!(m.records.contains_key("bo"));
        m.apply(&Cmd::Delete {
            name: "Bo".into(),
            token: 3,
        });
        assert!(!m.records.contains_key("bo"));
    }
}
