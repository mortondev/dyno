//! Character progression: levels, XP and money, at a classic-MMO pace
//! (docs/design/rpg.md, "Progression").

/// Highest level for now.
pub const MAX_LEVEL: u8 = 20;

/// XP needed to go from `level` to the next level. 0 at the cap.
pub fn xp_to_next(level: u8) -> u32 {
    const TABLE: [u32; 10] = [400, 900, 1400, 2100, 2800, 3600, 4500, 5400, 6500, 7600];
    match level {
        0 => 400,
        l if l >= MAX_LEVEL => 0,
        l if (l as usize) <= TABLE.len() => TABLE[l as usize - 1],
        l => 7600 + 1300 * (l as u32 - 10),
    }
}

/// XP for killing a mob, by the killer's and the mob's levels.
/// Same level: 45 + 5 × level. Higher mobs give up to 20% more;
/// lower mobs give less, and grey mobs (5 or more below) give none.
pub fn kill_xp(player: u8, mob: u8) -> u32 {
    let base = 45.0 + 5.0 * mob as f32;
    let diff = mob as i32 - player as i32;
    let factor = if diff >= 0 {
        1.0 + 0.05 * diff.min(4) as f32
    } else if diff <= -5 {
        0.0
    } else {
        1.0 + 0.18 * diff as f32 // -1: 0.82 ... -4: 0.28
    };
    (base * factor).round() as u32
}

/// Quest XP after the over-level penalty: full up to 5 levels above the
/// quest, then 20% less per level, down to 10%.
pub fn quest_xp(base: u32, player: u8, quest: u8) -> u32 {
    let over = player as i32 - quest as i32 - 5;
    let f = if over <= 0 {
        1.0
    } else {
        (1.0 - 0.2 * over as f32).max(0.1)
    };
    (base as f32 * f).round() as u32
}

/// Colour band of a mob or quest for a player: 0 grey, 1 green, 2 yellow, 3 orange, 4 red.
pub fn difficulty(player: u8, other: u8) -> u8 {
    let d = other as i32 - player as i32;
    match d {
        d if d <= -5 => 0,
        -4..=-3 => 1,
        -2..=2 => 2,
        3..=4 => 3,
        _ => 4,
    }
}

/// Add XP; returns the number of levels gained.
pub fn gain(level: &mut u8, xp: &mut u32, amount: u32) -> u8 {
    let mut ups = 0;
    *xp += amount;
    while *level < MAX_LEVEL && *xp >= xp_to_next(*level) {
        *xp -= xp_to_next(*level);
        *level += 1;
        ups += 1;
    }
    if *level >= MAX_LEVEL {
        *xp = 0;
    }
    ups
}

/// Most health at a level.
pub fn max_health(level: u8) -> f32 {
    50.0 + 12.0 * (level.max(1) as f32 - 1.0)
}

/// Damage multiplier for a player's level.
pub fn power(level: u8) -> f32 {
    1.0 + 0.12 * (level.max(1) as f32 - 1.0)
}

/// Health and damage multipliers for a mob's level.
pub fn mob_scale(level: u8) -> (f32, f32) {
    let l = level.max(1) as f32 - 1.0;
    (1.0 + 0.28 * l, 1.0 + 0.16 * l)
}

/// "1g 20s 5c"
pub fn money_text(copper: u32) -> String {
    let (g, s, c) = (copper / 10_000, copper / 100 % 100, copper % 100);
    let mut parts = Vec::new();
    if g > 0 {
        parts.push(format!("{g}g"));
    }
    if s > 0 {
        parts.push(format!("{s}s"));
    }
    if c > 0 || parts.is_empty() {
        parts.push(format!("{c}c"));
    }
    parts.join(" ")
}

/// Highest profession skill at a character level.
pub fn skill_cap(level: u8) -> u16 {
    (50 + 5 * level as u16).min(150)
}

/// Chance (0..1) that using something needing `need` raises a skill of `have`:
/// orange (within 25) always, yellow (within 50) 75%, green (within 75) 25%,
/// grey never.
pub fn skill_up_chance(have: u16, need: u16) -> f32 {
    match have.saturating_sub(need) {
        0..25 => 1.0,
        25..50 => 0.75,
        50..75 => 0.25,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_follow_the_table() {
        assert_eq!(xp_to_next(1), 400);
        assert_eq!(xp_to_next(10), 7600);
        assert_eq!(xp_to_next(11), 8900);
        assert_eq!(xp_to_next(MAX_LEVEL), 0);
        let (mut l, mut x) = (1, 0);
        assert_eq!(gain(&mut l, &mut x, 1300), 2);
        assert_eq!((l, x), (3, 0));
    }

    #[test]
    fn kill_xp_by_level() {
        assert_eq!(kill_xp(1, 1), 50);
        assert!(kill_xp(3, 5) > kill_xp(3, 3));
        assert_eq!(kill_xp(10, 5), 0);
        assert!(kill_xp(5, 3) < kill_xp(5, 5));
    }

    #[test]
    fn pace_level_five_needs_about_four_thousand_eight_hundred_xp() {
        let total: u32 = (1..5).map(xp_to_next).sum();
        assert_eq!(total, 4800);
    }

    #[test]
    fn money() {
        assert_eq!(money_text(0), "0c");
        assert_eq!(money_text(12_345), "1g 23s 45c");
    }
}
