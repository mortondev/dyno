//! Crafting from a recipe list. A recipe's station must be near; some use fuel.

use crate::inventory::{Inventory, Stack};
use dyno_world::World;
use dyno_world::content::{Content, Recipe};

/// The stations within reach: one bit for each of the content's stations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stations(pub u32);

impl Stations {
    /// Every station (tests).
    pub const ALL: Stations = Stations(u32::MAX);

    pub fn has(&self, s: Option<u8>) -> bool {
        match s {
            None => true,
            Some(i) => self.0 & (1 << i.min(31)) != 0,
        }
    }
}

/// The crafting stations within reach of a position: the server's rule,
/// and what the client shows.
pub fn stations_near(c: &Content, world: &World, pos: [f32; 3]) -> Stations {
    let mut st = Stations::default();
    if c.stations.is_empty() {
        return st;
    }
    let (bx, by, bz) = (
        pos[0].floor() as i32,
        pos[1].floor() as i32,
        pos[2].floor() as i32,
    );
    for y in by - 3..=by + 4 {
        for z in bz - 5..=bz + 5 {
            for x in bx - 5..=bx + 5 {
                let b = world.get(x, y, z);
                if b == dyno_world::AIR {
                    continue;
                }
                for (i, s) in c.stations.iter().enumerate() {
                    if s.blocks.contains(&b) {
                        st.0 |= 1 << i.min(31);
                    }
                }
            }
        }
    }
    st
}

/// Why a recipe cannot be made now, or Ok.
pub fn check(
    c: &Content,
    r: &Recipe,
    inv: &Inventory,
    st: Stations,
    fuel_bank: u16,
) -> Result<(), String> {
    if !st.has(r.station) {
        let name = r
            .station
            .and_then(|i| c.stations.get(i as usize))
            .map_or("station", |s| s.name.as_str());
        return Err(format!("Needs a {name} nearby"));
    }
    let mut scratch = inv.clone();
    for ing in &r.inputs {
        if !take_any(&mut scratch, &ing.any, ing.count as u32) {
            let names: Vec<&str> = ing.any.iter().map(|i| c.item(*i).name).collect();
            return Err(format!("Needs {} {}", ing.count, names.join(" or ")));
        }
    }
    if r.fuel > 0 {
        let mut bank = fuel_bank;
        if !burn_fuel(c, &mut scratch, &mut bank, r.fuel as u16) {
            return Err("Needs fuel: coal, logs, planks or sticks".into());
        }
    }
    Ok(())
}

/// Make a recipe once. Consumes inputs and fuel. Returns output that did not fit.
pub fn craft(
    c: &Content,
    r: &Recipe,
    inv: &mut Inventory,
    st: Stations,
    fuel_bank: &mut u16,
) -> Result<Option<Stack>, String> {
    check(c, r, inv, st, *fuel_bank)?;
    for ing in &r.inputs {
        take_any(inv, &ing.any, ing.count as u32);
    }
    if r.fuel > 0 {
        burn_fuel(c, inv, fuel_bank, r.fuel as u16);
    }
    Ok(inv.add(c, Stack::new(r.output, r.count)))
}

fn take_any(inv: &mut Inventory, any: &[u16], mut n: u32) -> bool {
    let have: u32 = any.iter().map(|i| inv.count(*i)).sum();
    if have < n {
        return false;
    }
    for i in any {
        let k = inv.count(*i).min(n);
        if k > 0 {
            inv.remove(*i, k);
            n -= k;
        }
    }
    true
}

/// Take `need` fuel units from the bank, burning items into it as needed.
fn burn_fuel(c: &Content, inv: &mut Inventory, bank: &mut u16, need: u16) -> bool {
    while *bank < need {
        // burn the least valuable fuel first
        let mut fuels: Vec<(u8, u16)> = inv
            .slots
            .iter()
            .flatten()
            .map(|s| (c.item(s.item).fuel, s.item))
            .filter(|(f, _)| *f > 0)
            .collect();
        fuels.sort();
        let Some((units, item)) = fuels.first().copied() else {
            return false;
        };
        inv.remove(item, 1);
        *bank += units as u16;
    }
    *bank -= need;
    true
}

/// Recipes in display order, with whether each can be made now.
pub fn list<'a>(
    c: &'a Content,
    inv: &Inventory,
    st: Stations,
    fuel_bank: u16,
) -> Vec<(&'a Recipe, Result<(), String>)> {
    c.recipes
        .iter()
        .map(|r| (r, check(c, r, inv, st, fuel_bank)))
        .collect()
}
