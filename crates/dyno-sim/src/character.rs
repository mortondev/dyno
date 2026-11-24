//! The character sheet: stats from class, level and gear, and the combat
//! numbers they give (docs/design/rpg.md, "Stats and gear").

use crate::inventory::Stack;
use dyno_world::content::{
    ArmorType, Class, Content, EQUIP_PLACES, Gear, MAIN_HAND, OFF_HAND, RANGED, Resource, Slot,
    Stats, Weapon, WeaponKind,
};

/// Everything the character screen shows, and the server fights with.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sheet {
    pub stats: Stats,
    pub armor: f32,
    pub max_health: f32,
    pub max_power: f32,
    pub attack_power: f32,
    pub spell_power: f32,
    /// percent
    pub crit: f32,
    pub spell_crit: f32,
    /// main hand (or fists): min, max, seconds per swing
    pub weapon: (f32, f32, f32),
    /// melee damage per second with auto-attack
    pub dps: f32,
}

/// Unarmed: 1 to 2 damage, a swing every 2 seconds.
const FISTS: (f32, f32, f32) = (1.0, 2.0, 2.0);

pub fn gear<'a>(c: &'a Content, s: &Option<Stack>) -> Option<&'a Gear> {
    s.as_ref().and_then(|s| c.item(s.item).gear)
}

/// The sheet for a class at a level wearing `equip` (17 places, see EQUIP_PLACES).
/// `extra` holds stat buffs. Class growth is per level in tenths.
pub fn sheet(
    c: &Content,
    class: &Class,
    level: u8,
    equip: &[Option<Stack>],
    extra: &Stats,
) -> Sheet {
    let l = level.max(1) as i16 - 1;
    let mut st = class.base;
    st.str += class.growth.str * l / 10;
    st.agi += class.growth.agi * l / 10;
    st.int += class.growth.int * l / 10;
    st.sta += class.growth.sta * l / 10;
    st.spi += class.growth.spi * l / 10;
    st.add(extra);
    let mut armor = 0.0;
    for s in equip {
        if let Some(g) = gear(c, s) {
            st.add(&g.stats);
            armor += g.armor as f32;
        }
    }
    armor += st.agi as f32 * 2.0;
    let lv = level.max(1) as f32;
    let _ = lv;
    let max_health = 20.0 + st.sta as f32 * 5.0;
    let max_power = match class.resource {
        Resource::Rage | Resource::Energy => 100.0,
        Resource::Mana => 20.0 + st.int as f32 * 8.0,
    };
    let agile =
        class.weapons.contains(&WeaponKind::Dagger) || class.weapons.contains(&WeaponKind::Bow);
    let attack_power = if agile {
        (st.str + st.agi) as f32
    } else {
        st.str as f32 * 2.0
    };
    let spell_power = st.int as f32 * 0.5;
    let weapon = equip
        .get(MAIN_HAND)
        .and_then(|s| gear(c, s))
        .and_then(|g| g.weapon)
        .map(|w| (w.min, w.max, w.speed))
        .unwrap_or(FISTS);
    let bonus = attack_power / 14.0 * weapon.2;
    let dps = ((weapon.0 + weapon.1) * 0.5 + bonus) / weapon.2;
    Sheet {
        stats: st,
        armor,
        max_health,
        max_power,
        attack_power,
        spell_power,
        crit: 5.0 + st.agi as f32 / 10.0,
        spell_crit: 5.0 + st.int as f32 / 15.0,
        weapon,
        dps,
    }
}

/// Damage of one auto-attack swing, from a 0..1 roll.
pub fn swing(s: &Sheet, roll: f32) -> f32 {
    let (lo, hi, speed) = s.weapon;
    lo + (hi - lo) * roll + s.attack_power / 14.0 * speed
}

/// Fraction of physical damage armour stops against an attacker of `level`.
pub fn armor_reduction(armor: f32, level: u8) -> f32 {
    (armor / (armor + 100.0 + 50.0 * level as f32)).clamp(0.0, 0.75)
}

/// Why this character cannot wear an item, if they cannot.
pub fn cannot_equip(
    class: &Class,
    level: u8,
    item: &dyno_world::content::ItemView,
) -> Option<String> {
    let g = item.gear?;
    if item.level > level {
        return Some(format!("Requires level {}", item.level));
    }
    if let Some(t) = g.armor_type
        && t > class.armor
    {
        let name = match t {
            ArmorType::Cloth => "cloth",
            ArmorType::Leather => "leather",
            ArmorType::Mail => "mail",
            ArmorType::Plate => "plate",
        };
        return Some(format!("{}s cannot wear {name}", class.name));
    }
    if let Some(w) = g.weapon
        && !class.weapons.contains(&w.kind)
    {
        return Some(format!("{}s cannot use this weapon", class.name));
    }
    None
}

/// The paper-doll places an item can go in (first is preferred).
pub fn places_for(slot: Slot) -> Vec<usize> {
    match slot {
        Slot::TwoHand => vec![MAIN_HAND],
        Slot::OffHand => vec![OFF_HAND],
        Slot::Ranged => vec![RANGED],
        s => EQUIP_PLACES
            .iter()
            .enumerate()
            .filter(|(_, (p, _))| *p == s)
            .map(|(i, _)| i)
            .collect(),
    }
}

/// Where an item of `slot` goes when worn, and which places come off for
/// it: the first empty matching place, else the first; a two-handed weapon
/// also frees the off hand, and an off-hand item takes off a two-handed
/// weapon. The server equips by this and a bot judges upgrades by it, so
/// the two agree (a bot that scored a buckler beside a two-handed axe
/// swapped them for ever).
pub fn equip_place(
    c: &Content,
    equip: &[Option<Stack>],
    slot: Slot,
) -> Option<(usize, Vec<usize>)> {
    let places = places_for(slot);
    let place = places
        .iter()
        .copied()
        .find(|i| equip.get(*i).is_some_and(|s| s.is_none()))
        .or(places.first().copied())?;
    let mut off = Vec::new();
    if slot == Slot::TwoHand {
        off.push(OFF_HAND);
    }
    if place == OFF_HAND
        && gear(c, equip.get(MAIN_HAND).unwrap_or(&None)).is_some_and(|m| m.slot == Slot::TwoHand)
    {
        off.push(MAIN_HAND);
    }
    Some((place, off))
}

/// The weapon in the main hand, if any.
pub fn main_weapon(c: &Content, equip: &[Option<Stack>]) -> Option<Weapon> {
    equip
        .get(MAIN_HAND)
        .and_then(|s| gear(c, s))
        .and_then(|g| g.weapon)
}
