//! Parties: up to five players who adventure together. Parties live above
//! the shards: members are drawn into the same shard so they always see the
//! same world.

use std::collections::HashMap;
use std::time::{Duration, Instant};

pub const MAX_PARTY: usize = 5;
const INVITE_LIFE: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, PartialEq)]
pub struct Party {
    /// display names; the first is the leader
    pub members: Vec<String>,
}

impl Party {
    pub fn leader(&self) -> &str {
        &self.members[0]
    }
}

#[derive(Default)]
pub struct Parties {
    next: u32,
    parties: HashMap<u32, Party>,
    /// lower-case name to party
    of: HashMap<String, u32>,
    /// lower-case invitee to (inviter display name, when)
    invites: HashMap<String, (String, Instant)>,
}

fn key(n: &str) -> String {
    n.to_lowercase()
}

impl Parties {
    pub fn party_of(&self, name: &str) -> Option<&Party> {
        self.of.get(&key(name)).and_then(|i| self.parties.get(i))
    }

    /// Everyone in `name`'s party, them included (empty: no party).
    pub fn members(&self, name: &str) -> Vec<String> {
        self.party_of(name)
            .map(|p| p.members.clone())
            .unwrap_or_default()
    }

    pub fn all(&self) -> impl Iterator<Item = &Party> {
        self.parties.values()
    }

    /// `from` asks `to` to join their party.
    pub fn invite(&mut self, from: &str, to: &str) -> Result<(), String> {
        if key(from) == key(to) {
            return Err("You cannot invite yourself.".into());
        }
        if self.party_of(to).is_some() {
            return Err(format!("{to} is already in a party."));
        }
        if let Some(p) = self.party_of(from) {
            if key(p.leader()) != key(from) {
                return Err("Only the party leader can invite.".into());
            }
            if p.members.len() >= MAX_PARTY {
                return Err("Your party is full.".into());
            }
        }
        self.invites
            .insert(key(to), (from.to_string(), Instant::now()));
        Ok(())
    }

    /// `name` accepts their invite. Returns the party's members.
    pub fn accept(&mut self, name: &str) -> Result<Vec<String>, String> {
        let Some((from, at)) = self.invites.remove(&key(name)) else {
            return Err("No one has invited you.".into());
        };
        if at.elapsed() > INVITE_LIFE {
            return Err("That invitation has expired.".into());
        }
        if self.party_of(name).is_some() {
            return Err("You are already in a party.".into());
        }
        let id = match self.of.get(&key(&from)) {
            Some(id) => *id,
            None => {
                self.next += 1;
                let id = self.next;
                self.parties.insert(
                    id,
                    Party {
                        members: vec![from.clone()],
                    },
                );
                self.of.insert(key(&from), id);
                id
            }
        };
        let p = self.parties.get_mut(&id).unwrap();
        if p.members.len() >= MAX_PARTY {
            return Err("That party is full.".into());
        }
        p.members.push(name.to_string());
        self.of.insert(key(name), id);
        Ok(p.members.clone())
    }

    /// `name` declines; returns who invited them.
    pub fn decline(&mut self, name: &str) -> Option<String> {
        self.invites.remove(&key(name)).map(|(f, _)| f)
    }

    /// `name` leaves their party. Returns who is left (to tell them); a party
    /// of one disbands, and the next member leads if the leader left.
    pub fn leave(&mut self, name: &str) -> Vec<String> {
        let Some(id) = self.of.remove(&key(name)) else {
            return Vec::new();
        };
        let Some(p) = self.parties.get_mut(&id) else {
            return Vec::new();
        };
        p.members.retain(|m| key(m) != key(name));
        let left = p.members.clone();
        if left.len() <= 1 {
            self.parties.remove(&id);
            for m in &left {
                self.of.remove(&key(m));
            }
        }
        left
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invite_accept_and_leave() {
        let mut ps = Parties::default();
        assert!(ps.accept("bob").is_err(), "no invite yet");
        ps.invite("Ada", "Bob").unwrap();
        assert_eq!(ps.accept("bob").unwrap(), vec!["Ada", "bob"]);
        assert_eq!(ps.party_of("ADA").unwrap().leader(), "Ada");
        assert!(ps.invite("bob", "cy").is_err(), "only the leader invites");
        ps.invite("ada", "Cy").unwrap();
        ps.accept("Cy").unwrap();
        assert_eq!(ps.members("cy").len(), 3);
        // the leader leaves: the next member leads
        assert_eq!(ps.leave("ada"), vec!["bob", "Cy"]);
        assert_eq!(ps.party_of("cy").unwrap().leader(), "bob");
        // down to one: the party is gone
        ps.leave("cy");
        assert!(ps.party_of("bob").is_none());
        assert_eq!(ps.decline("nobody"), None);
    }
}
