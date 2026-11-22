//! A player inventory: 36 slots. Slots 0 to 8 are the hotbar.

use dyno_world::content::{Content, ItemId};
use serde::{Deserialize, Serialize};

pub const SLOTS: usize = 36;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stack {
    pub item: ItemId,
    pub count: u8,
    /// tool wear; 0 for new tools and all other items
    pub wear: u16,
}

impl Stack {
    pub fn new(item: ItemId, count: u8) -> Self {
        Stack {
            item,
            count,
            wear: 0,
        }
    }
}

/// Can `b` be merged into `a`?
fn stacks_with(c: &Content, a: &Stack, b: &Stack) -> bool {
    a.item == b.item && a.wear == 0 && b.wear == 0 && c.item(a.item).stack > 1
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<Option<Stack>>,
}

impl Default for Inventory {
    fn default() -> Self {
        Inventory {
            slots: vec![None; SLOTS],
        }
    }
}

impl Inventory {
    /// Add a stack. Fills existing stacks first, then empty slots (hotbar first).
    /// Returns what did not fit.
    pub fn add(&mut self, c: &Content, mut s: Stack) -> Option<Stack> {
        let max = c.item(s.item).stack;
        for slot in self.slots.iter_mut().flatten() {
            if s.count == 0 {
                break;
            }
            if stacks_with(c, slot, &s) && slot.count < max {
                let n = (max - slot.count).min(s.count);
                slot.count += n;
                s.count -= n;
            }
        }
        for slot in self.slots.iter_mut() {
            if s.count == 0 {
                break;
            }
            if slot.is_none() {
                let n = s.count.min(max);
                *slot = Some(Stack { count: n, ..s });
                s.count -= n;
            }
        }
        if s.count > 0 { Some(s) } else { None }
    }

    /// Total count of an item.
    pub fn count(&self, item: ItemId) -> u32 {
        self.slots
            .iter()
            .flatten()
            .filter(|s| s.item == item)
            .map(|s| s.count as u32)
            .sum()
    }

    /// Remove `n` of an item from the back slots first. All or nothing.
    pub fn remove(&mut self, item: ItemId, n: u32) -> bool {
        if self.count(item) < n {
            return false;
        }
        let mut left = n;
        for slot in self.slots.iter_mut().rev() {
            if left == 0 {
                break;
            }
            if let Some(s) = slot
                && s.item == item
            {
                let take = (s.count as u32).min(left);
                s.count -= take as u8;
                left -= take;
                if s.count == 0 {
                    *slot = None;
                }
            }
        }
        true
    }

    /// Take up to `n` items out of a slot (0 means all).
    pub fn take(&mut self, slot: usize, n: u8) -> Option<Stack> {
        let s = self.slots.get_mut(slot)?.as_mut()?;
        let n = if n == 0 { s.count } else { n.min(s.count) };
        let out = Stack { count: n, ..*s };
        s.count -= n;
        if s.count == 0 {
            self.slots[slot] = None;
        }
        Some(out)
    }

    pub fn is_empty(&self) -> bool {
        self.slots.iter().all(|s| s.is_none())
    }

    /// Every stack, for dropping on death.
    pub fn drain(&mut self) -> Vec<Stack> {
        self.slots.iter_mut().filter_map(|s| s.take()).collect()
    }
}

/// Move items from one slot to another. `count` 0 moves everything.
/// Merges same items up to the stack size; swaps different items when moving everything.
pub fn transfer(c: &Content, src: &mut Option<Stack>, dst: &mut Option<Stack>, count: u8) {
    let Some(s) = src.as_mut() else { return };
    let want = if count == 0 {
        s.count
    } else {
        count.min(s.count)
    };
    match dst {
        None => {
            let max = c.item(s.item).stack;
            let n = want.min(max);
            *dst = Some(Stack { count: n, ..*s });
            s.count -= n;
        }
        Some(d) if stacks_with(c, d, s) => {
            let max = c.item(d.item).stack;
            let n = want.min(max.saturating_sub(d.count));
            d.count += n;
            s.count -= n;
        }
        Some(_) => {
            if want == s.count {
                std::mem::swap(src, dst);
                return;
            }
        }
    }
    if src.map(|s| s.count) == Some(0) {
        *src = None;
    }
}

/// Move between two slots of the same slot list.
pub fn transfer_within(
    c: &Content,
    slots: &mut [Option<Stack>],
    from: usize,
    to: usize,
    count: u8,
) {
    if from == to || from >= slots.len() || to >= slots.len() {
        return;
    }
    let (a, b) = if from < to {
        let (l, r) = slots.split_at_mut(to);
        (&mut l[from], &mut r[0])
    } else {
        let (l, r) = slots.split_at_mut(from);
        (&mut r[0], &mut l[to])
    };
    transfer(c, a, b, count);
}

/// The best tool of a kind in a pack (highest tier, then speed), and its slot.
pub fn best_tool<'a>(
    inv: &Inventory,
    c: &'a dyno_world::content::Content,
    kind: dyno_world::content::ToolKind,
) -> Option<(usize, &'a dyno_world::content::Tool)> {
    inv.slots
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.and_then(|s| c.item(s.item).tool.map(|t| (i, t))))
        .filter(|(_, t)| t.kind == kind)
        .max_by(|a, b| (a.1.tier, a.1.speed).partial_cmp(&(b.1.tier, b.1.speed)).unwrap_or(std::cmp::Ordering::Equal))
}

/// The hardest-hitting item in a pack (your weapon), and its slot.
pub fn best_weapon<'a>(
    inv: &Inventory,
    c: &'a dyno_world::content::Content,
) -> Option<(usize, &'a dyno_world::content::Tool)> {
    inv.slots
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.and_then(|s| c.item(s.item).tool.map(|t| (i, t))))
        .max_by(|a, b| a.1.damage.partial_cmp(&b.1.damage).unwrap_or(std::cmp::Ordering::Equal))
}

/// The food in a pack that restores the most, and its slot.
pub fn best_food(inv: &Inventory, c: &dyno_world::content::Content) -> Option<usize> {
    inv.slots
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.map(|s| (i, c.item(s.item).food)))
        .filter(|(_, f)| *f > 0)
        .max_by_key(|(_, f)| *f)
        .map(|(i, _)| i)
}
