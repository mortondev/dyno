//! Trading between two players standing near each other: each puts items
//! and coins on the table, both accept, and the swap happens at once or not
//! at all. Anything changing clears both acceptances, so nobody is tricked.

use crate::game::{State, dist};
use dyno_protocol::{PlayerId, ServerMsg, Stack, TradeOp, TradeView};
use std::time::{Duration, Instant};

const REACH: f32 = 10.0;
const LINES: usize = 6;

#[derive(Clone, Debug, Default)]
pub struct Trade {
    pub with: PlayerId,
    /// bag slot and count
    pub offer: Vec<(u8, u8)>,
    pub money: u32,
    pub accept: bool,
}

impl State {
    fn near_each_other(&self, a: PlayerId, b: PlayerId) -> bool {
        match (self.players.get(&a), self.players.get(&b)) {
            (Some(x), Some(y)) => !x.dead && !y.dead && dist(x.pose.pos, y.pose.pos) <= REACH,
            _ => false,
        }
    }

    pub fn trade(&mut self, id: PlayerId, op: TradeOp) {
        match op {
            TradeOp::Ask(target) => {
                if target == id
                    || self.trades.contains_key(&id)
                    || self.trades.contains_key(&target)
                {
                    return self.tell(id, "They are busy.");
                }
                if !self.near_each_other(id, target) {
                    return self.tell(id, "You are too far away to trade.");
                }
                let name = self.players[&id].name.clone();
                self.trade_asks.insert(target, (id, Instant::now()));
                if let Some(t) = self.players.get(&target) {
                    t.send(&ServerMsg::TradeAsk { from: name });
                }
                let them = self.players[&target].name.clone();
                self.tell(id, format!("You ask {them} to trade."));
            }
            TradeOp::Agree => {
                let Some((asker, at)) = self.trade_asks.remove(&id) else {
                    return self.tell(id, "No one has asked to trade with you.");
                };
                if at.elapsed() > Duration::from_secs(60)
                    || !self.near_each_other(id, asker)
                    || self.trades.contains_key(&asker)
                    || self.trades.contains_key(&id)
                {
                    return self.tell(id, "That trade is no longer possible.");
                }
                self.trades.insert(
                    id,
                    Trade {
                        with: asker,
                        ..Default::default()
                    },
                );
                self.trades.insert(
                    asker,
                    Trade {
                        with: id,
                        ..Default::default()
                    },
                );
                self.send_trade(id);
            }
            TradeOp::Offer { slot, count } => {
                let Some(stack) = self
                    .players
                    .get(&id)
                    .and_then(|p| p.pack.slots.get(slot as usize).copied().flatten())
                else {
                    return;
                };
                let Some(t) = self.trades.get_mut(&id) else {
                    return;
                };
                if t.offer.len() >= LINES || t.offer.iter().any(|(s, _)| *s == slot) {
                    return;
                }
                let n = if count == 0 {
                    stack.count
                } else {
                    count.min(stack.count)
                };
                t.offer.push((slot, n));
                self.changed(id);
            }
            TradeOp::Withdraw { line } => {
                let Some(t) = self.trades.get_mut(&id) else {
                    return;
                };
                if (line as usize) < t.offer.len() {
                    t.offer.remove(line as usize);
                }
                self.changed(id);
            }
            TradeOp::Money(n) => {
                let have = self.players.get(&id).map(|p| p.money).unwrap_or(0);
                let Some(t) = self.trades.get_mut(&id) else {
                    return;
                };
                t.money = n.min(have);
                self.changed(id);
            }
            TradeOp::Accept => {
                let Some(t) = self.trades.get_mut(&id) else {
                    return;
                };
                t.accept = true;
                let other = t.with;
                if self.trades.get(&other).is_some_and(|o| o.accept) {
                    self.complete_trade(id, other);
                } else {
                    self.send_trade(id);
                }
            }
            TradeOp::Cancel => self.cancel_trade(id, "Trade cancelled."),
        }
    }

    /// One side changed the table: both must accept again.
    fn changed(&mut self, id: PlayerId) {
        let Some(other) = self.trades.get(&id).map(|t| t.with) else {
            return;
        };
        for who in [id, other] {
            if let Some(t) = self.trades.get_mut(&who) {
                t.accept = false;
            }
        }
        self.send_trade(id);
    }

    fn lines(&self, id: PlayerId) -> (Vec<Stack>, u32) {
        let (Some(t), Some(p)) = (self.trades.get(&id), self.players.get(&id)) else {
            return (Vec::new(), 0);
        };
        let items = t
            .offer
            .iter()
            .filter_map(|(s, n)| {
                p.pack
                    .slots
                    .get(*s as usize)
                    .copied()
                    .flatten()
                    .map(|st| Stack {
                        count: (*n).min(st.count),
                        ..st
                    })
            })
            .collect();
        (items, t.money)
    }

    /// Show the window to both sides.
    fn send_trade(&self, id: PlayerId) {
        let Some(other) = self.trades.get(&id).map(|t| t.with) else {
            return;
        };
        for (me, them) in [(id, other), (other, id)] {
            let (mine, my_money) = self.lines(me);
            let (theirs, their_money) = self.lines(them);
            let view = TradeView {
                with: self
                    .players
                    .get(&them)
                    .map(|p| p.name.clone())
                    .unwrap_or_default(),
                mine,
                theirs,
                my_money,
                their_money,
                i_accept: self.trades.get(&me).is_some_and(|t| t.accept),
                they_accept: self.trades.get(&them).is_some_and(|t| t.accept),
            };
            if let Some(p) = self.players.get(&me) {
                p.send(&ServerMsg::Trade(Some(view)));
            }
        }
    }

    pub fn cancel_trade(&mut self, id: PlayerId, why: &str) {
        let Some(t) = self.trades.remove(&id) else {
            return;
        };
        self.trades.remove(&t.with);
        for who in [id, t.with] {
            if let Some(p) = self.players.get(&who) {
                p.send(&ServerMsg::Trade(None));
                p.send(&ServerMsg::System {
                    text: why.to_string(),
                });
            }
        }
    }

    /// Both accepted: swap everything at once, or nothing.
    fn complete_trade(&mut self, a: PlayerId, b: PlayerId) {
        if !self.near_each_other(a, b) {
            return self.cancel_trade(a, "Trade cancelled: you are too far apart.");
        }
        let (ta, tb) = (self.trades[&a].clone(), self.trades[&b].clone());
        let (Some(pa), Some(pb)) = (self.players.get(&a), self.players.get(&b)) else {
            return self.cancel_trade(a, "Trade cancelled.");
        };
        if pa.money < ta.money || pb.money < tb.money {
            return self.cancel_trade(a, "Trade cancelled: not enough coins.");
        }
        // take each side's offer out of a copy of their bags
        let take = |pack: &dyno_sim::inventory::Inventory,
                    offer: &[(u8, u8)]|
         -> Option<(dyno_sim::inventory::Inventory, Vec<Stack>)> {
            let mut pack = pack.clone();
            let mut out = Vec::new();
            for (s, n) in offer {
                let st = pack.slots.get(*s as usize).copied().flatten()?;
                if st.count < *n {
                    return None;
                }
                out.push(Stack { count: *n, ..st });
                pack.slots[*s as usize] = if st.count == *n {
                    None
                } else {
                    Some(Stack {
                        count: st.count - n,
                        ..st
                    })
                };
            }
            Some((pack, out))
        };
        let (Some((mut bag_a, give_a)), Some((mut bag_b, give_b))) =
            (take(&pa.pack, &ta.offer), take(&pb.pack, &tb.offer))
        else {
            return self.cancel_trade(a, "Trade cancelled: something on the table moved.");
        };
        for st in &give_b {
            if bag_a.add(&self.content, *st).is_some() {
                return self.cancel_trade(a, "Trade cancelled: not enough room in the bags.");
            }
        }
        for st in &give_a {
            if bag_b.add(&self.content, *st).is_some() {
                return self.cancel_trade(a, "Trade cancelled: not enough room in the bags.");
            }
        }
        self.trades.remove(&a);
        self.trades.remove(&b);
        for (me, bag, paid, got) in [
            (a, bag_a, ta.money, tb.money),
            (b, bag_b, tb.money, ta.money),
        ] {
            if let Some(p) = self.players.get_mut(&me) {
                p.pack = bag;
                p.money = p.money - paid + got;
                p.send(&ServerMsg::Pack(p.pack.slots.clone()));
                p.send(&ServerMsg::Trade(None));
                p.send(&ServerMsg::System {
                    text: "Trade complete.".into(),
                });
            }
            self.send_character(me);
        }
        for id in [a, b] {
            if self.update_quests(id) {
                self.send_quests(id);
            }
        }
        crate::game::log(format!(
            "Trade between players {a} and {b}: {} item lines, {} and {} copper",
            give_a.len() + give_b.len(),
            ta.money,
            tb.money
        ));
    }

    /// Trades end if anyone walks away, dies or leaves.
    pub fn tick_trades(&mut self) {
        let ids: Vec<PlayerId> = self.trades.keys().copied().collect();
        for id in ids {
            let Some(other) = self.trades.get(&id).map(|t| t.with) else {
                continue;
            };
            if !self.near_each_other(id, other) {
                self.cancel_trade(id, "Trade cancelled: you are too far apart.");
            }
        }
        self.trade_asks
            .retain(|_, (_, at)| at.elapsed() < Duration::from_secs(60));
    }
}
