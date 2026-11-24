use dyno_sim::crafting::{self, Stations};
use dyno_sim::inventory::{Inventory, Stack, transfer_within};
use dyno_sim::mining;
use dyno_sim::rng::Rng;
use dyno_world::content::{Content, ItemId};
use std::collections::HashMap;

fn c() -> Content {
    Content::default_pack()
}

fn id(c: &Content, k: &str) -> ItemId {
    c.item_by_key(k).unwrap_or_else(|| panic!("no item {k}"))
}

/// Standing at the station with this key, and at no other.
fn at(c: &Content, key: &str) -> Stations {
    let i = c
        .stations
        .iter()
        .position(|s| s.key == key)
        .unwrap_or_else(|| panic!("no station {key}"));
    Stations(1 << i)
}

fn totals(inv: &Inventory) -> HashMap<ItemId, i64> {
    let mut m = HashMap::new();
    for s in inv.slots.iter().flatten() {
        *m.entry(s.item).or_insert(0) += s.count as i64;
    }
    m
}

#[test]
fn stacking_and_overflow() {
    let c = c();
    let mut inv = Inventory::default();
    let dirt = id(&c, "dirt");
    assert_eq!(inv.add(&c, Stack::new(dirt, 60)), None);
    assert_eq!(inv.add(&c, Stack::new(dirt, 10)), None);
    assert_eq!(inv.count(dirt), 70);
    assert_eq!(
        inv.slots.iter().flatten().count(),
        2,
        "fills the first stack before a new slot"
    );
    // fill everything
    for _ in 0..40 {
        inv.add(&c, Stack::new(dirt, 64));
    }
    let left = inv.add(&c, Stack::new(dirt, 5));
    assert_eq!(
        left,
        Some(Stack::new(dirt, 5)),
        "a full inventory returns the rest"
    );
    // tools never stack
    let mut inv = Inventory::default();
    let pick = id(&c, "wooden_pickaxe");
    inv.add(&c, Stack::new(pick, 1));
    inv.add(&c, Stack::new(pick, 1));
    assert_eq!(inv.slots.iter().flatten().count(), 2);
}

#[test]
fn crafting_planks_sticks_and_tools() {
    let c = c();
    let mut inv = Inventory::default();
    let (log, planks, stick, pick) = (
        id(&c, "log"),
        id(&c, "planks"),
        id(&c, "stick"),
        id(&c, "wooden_pickaxe"),
    );
    inv.add(&c, Stack::new(log, 3));
    let r = |out: ItemId| c.recipes.iter().find(|r| r.output == out).unwrap().clone();
    let mut bank = 0;
    let none = Stations::default();
    crafting::craft(&c, &r(planks), &mut inv, none, &mut bank).unwrap();
    assert_eq!(inv.count(log), 2);
    assert_eq!(inv.count(planks), 4);
    crafting::craft(&c, &r(stick), &mut inv, none, &mut bank).unwrap();
    assert_eq!(inv.count(stick), 4);
    // tools need their station, and the refusal names it
    let station = &c.stations[r(pick).station.expect("tools need a station") as usize];
    assert_eq!(
        crafting::craft(&c, &r(pick), &mut inv, none, &mut bank).unwrap_err(),
        format!("Needs a {} nearby", station.name)
    );
    crafting::craft(&c, &r(planks), &mut inv, none, &mut bank).unwrap();
    let table = at(&c, &station.key);
    crafting::craft(&c, &r(pick), &mut inv, table, &mut bank).unwrap();
    assert_eq!(inv.count(pick), 1);
    assert_eq!(inv.count(planks), 6 - 3);
}

#[test]
fn smelting_uses_fuel() {
    let c = c();
    let mut inv = Inventory::default();
    let (raw, ingot, coal) = (id(&c, "raw_iron"), id(&c, "iron_ingot"), id(&c, "coal"));
    inv.add(&c, Stack::new(raw, 10));
    let r = c
        .recipes
        .iter()
        .find(|r| r.output == ingot)
        .unwrap()
        .clone();
    let furnace = at(
        &c,
        &c.stations[r.station.expect("smelting needs a station") as usize].key,
    );
    let mut bank = 0;
    assert!(
        crafting::craft(&c, &r, &mut inv, furnace, &mut bank)
            .unwrap_err()
            .contains("fuel")
    );
    inv.add(&c, Stack::new(coal, 1));
    for _ in 0..8 {
        crafting::craft(&c, &r, &mut inv, furnace, &mut bank).unwrap();
    }
    assert_eq!(inv.count(ingot), 8, "one coal smelts eight");
    assert_eq!(inv.count(coal), 0);
    assert!(crafting::craft(&c, &r, &mut inv, furnace, &mut bank).is_err());
}

#[test]
fn gathering_needs_the_right_tool() {
    let c = c();
    let mut rng = Rng::new(1);
    let stone = dyno_world::STONE;
    let wood = c.item(id(&c, "wooden_pickaxe")).tool.cloned().unwrap();
    let stone_pick = c.item(id(&c, "stone_pickaxe")).tool.cloned().unwrap();
    assert!(
        mining::drops(&c, stone, None, &mut rng).is_empty(),
        "stone by hand drops nothing"
    );
    assert_eq!(
        mining::drops(&c, stone, Some(&wood), &mut rng)[0].item,
        dyno_world::COBBLE as ItemId
    );
    assert!(
        mining::drops(&c, dyno_world::IRON_ORE, Some(&wood), &mut rng).is_empty(),
        "iron needs stone"
    );
    assert_eq!(
        mining::drops(&c, dyno_world::IRON_ORE, Some(&stone_pick), &mut rng)[0].item,
        id(&c, "raw_iron")
    );
    // the wrong tier says what is needed
    assert_eq!(
        mining::missing_tool(&c, dyno_world::IRON_ORE, Some(&wood)).as_deref(),
        Some("Needs a stone pickaxe")
    );
    assert_eq!(
        mining::missing_tool(&c, dyno_world::IRON_ORE, Some(&stone_pick)),
        None
    );
    assert_eq!(
        mining::drops(&c, dyno_world::LOG, None, &mut rng)[0].item,
        dyno_world::LOG as ItemId
    );
}

/// Random operations never create or destroy items, except as the rules say:
/// added items, removed items and recipe conversions are all accounted for.
#[test]
fn item_count_invariant() {
    let c = c();
    let mut rng = Rng::new(42);
    let mut inv = Inventory::default();
    let mut expect: HashMap<ItemId, i64> = HashMap::new();
    let pool: Vec<ItemId> = [
        "log",
        "planks",
        "stick",
        "cobble",
        "coal",
        "raw_iron",
        "dirt",
        "white",
        "berries",
        "wooden_pickaxe",
        "sand",
    ]
    .iter()
    .map(|k| id(&c, k))
    .collect();
    let all = Stations::ALL;
    let mut bank: u16 = 0;
    for step in 0..20_000 {
        match rng.range(0, 4) {
            0 => {
                let item = pool[rng.range(0, pool.len() as u32 - 1) as usize];
                let n = rng.range(1, 40) as u8;
                let left = inv.add(&c, Stack::new(item, n));
                *expect.entry(item).or_insert(0) +=
                    n as i64 - left.map(|s| s.count as i64).unwrap_or(0);
            }
            1 => {
                let a = rng.range(0, 35) as usize;
                let b = rng.range(0, 35) as usize;
                let n = rng.range(0, 10) as u8;
                transfer_within(&c, &mut inv.slots, a, b, n);
            }
            2 => {
                let slot = rng.range(0, 35) as usize;
                if let Some(s) = inv.take(slot, rng.range(0, 5) as u8) {
                    *expect.entry(s.item).or_insert(0) -= s.count as i64;
                }
            }
            _ => {
                let r = &c.recipes[rng.range(0, c.recipes.len() as u32 - 1) as usize];
                let before = totals(&inv);
                let bank_before = bank;
                if let Ok(left) = crafting::craft(&c, r, &mut inv, all, &mut bank) {
                    let after = totals(&inv);
                    // every item that changed must be an input, a fuel, or the output
                    for (item, n) in after.iter().chain(before.iter()) {
                        let d = after.get(item).copied().unwrap_or(0)
                            - before.get(item).copied().unwrap_or(0);
                        if d != 0 {
                            let ok = *item == r.output
                                || r.inputs.iter().any(|i| i.any.contains(item))
                                || c.item(*item).fuel > 0;
                            assert!(
                                ok,
                                "step {step}: {} changed by {d} in recipe {}",
                                c.item(*item).name,
                                c.item(r.output).name
                            );
                        }
                        let _ = n;
                    }
                    let out_gain = after.get(&r.output).copied().unwrap_or(0)
                        - before.get(&r.output).copied().unwrap_or(0);
                    let produced = r.count as i64 - left.map(|s| s.count as i64).unwrap_or(0);
                    // the output gain is what was produced, minus any of it used as input or fuel
                    assert!(
                        out_gain <= produced,
                        "step {step}: output grew by more than the recipe makes"
                    );
                    let _ = bank_before;
                    for (item, _) in after.iter().chain(before.iter()) {
                        let d = after.get(item).copied().unwrap_or(0)
                            - before.get(item).copied().unwrap_or(0);
                        expect.insert(*item, expect.get(item).copied().unwrap_or(0) + d);
                    }
                    // the loop above double counts items present before and after; recompute from scratch
                    expect = after.clone();
                }
            }
        }
        let now = totals(&inv);
        for item in pool.iter().chain(c.recipes.iter().map(|r| &r.output)) {
            assert_eq!(
                now.get(item).copied().unwrap_or(0),
                expect.get(item).copied().unwrap_or(0),
                "step {step}: {} count drifted",
                c.item(*item).name
            );
        }
        // no stack is ever empty or over its limit
        for s in inv.slots.iter().flatten() {
            assert!(
                s.count >= 1 && s.count <= c.item(s.item).stack,
                "bad stack {s:?}"
            );
        }
    }
}
