//! Path finding over the block grid for bots that walk like players.
//!
//! A node is a cell where a player's feet can be: two cells of room (feet
//! and head) with something under them to stand on, or the top of water to
//! swim in. From a node you can walk to the eight neighbours on the same
//! level, step up one block (players auto-step in RPG worlds), or drop down
//! at most `MAX_DROP` blocks (deeper falls hurt). Diagonals need both
//! orthogonal neighbours free so the 0.6-wide body does not clip a corner.

use dyno_world::{self as w, World};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

/// Deepest drop a path may take (falls over 3.5 blocks hurt).
pub const MAX_DROP: i32 = 3;

pub type Cell = [i32; 3];

fn solid(world: &World, x: i32, y: i32, z: i32) -> bool {
    w::is_solid(world.get(x, y, z))
}

fn water(world: &World, x: i32, y: i32, z: i32) -> bool {
    world.get(x, y, z) == w::WATER
}

/// Room for a body (feet and head) at this cell.
pub fn room(world: &World, c: Cell) -> bool {
    world.in_bounds(c) && !solid(world, c[0], c[1], c[2]) && !solid(world, c[0], c[1] + 1, c[2])
}

/// Can a player stand (or swim) with their feet in this cell?
pub fn standable(world: &World, c: Cell) -> bool {
    if !room(world, c) {
        return false;
    }
    let [x, y, z] = c;
    if solid(world, x, y - 1, z) {
        return true;
    }
    // swimming at the surface: feet in water, head out of it
    water(world, x, y, z) && !water(world, x, y + 1, z)
}

/// Is this node in water (slow, and costs more)?
pub fn wet(world: &World, c: Cell) -> bool {
    water(world, c[0], c[1], c[2]) || water(world, c[0], c[1] - 1, c[2])
}

/// The node a player at `pos` (feet) stands on, searching a little around.
pub fn node_at(world: &World, pos: [f32; 3]) -> Option<Cell> {
    let base = [
        pos[0].floor() as i32,
        (pos[1] + 0.05).floor() as i32,
        pos[2].floor() as i32,
    ];
    // straight down first (we may be in the air, jumping or falling)
    for dy in [0, -1, 1, -2, -3, -4, -5, -6, 2] {
        let c = [base[0], base[1] + dy, base[2]];
        if standable(world, c) {
            return Some(c);
        }
    }
    for r in 1..=2 {
        for dz in -r..=r {
            for dx in -r..=r {
                for dy in [0, 1, -1, -2, 2] {
                    let c = [base[0] + dx, base[1] + dy, base[2] + dz];
                    if standable(world, c) {
                        return Some(c);
                    }
                }
            }
        }
    }
    None
}

/// Where you end up stepping from `from` towards (dx, dz): the same level,
/// one up, or a drop. None if the way is blocked.
fn step(world: &World, from: Cell, dx: i32, dz: i32) -> Option<Cell> {
    let [x, y, z] = from;
    let (nx, nz) = (x + dx, z + dz);
    if room(world, [nx, y, nz]) {
        // walk on, or fall to the first support below
        for d in 0..=MAX_DROP {
            let c = [nx, y - d, nz];
            if d > 0 && !room(world, c) {
                return None;
            }
            if standable(world, c) {
                return Some(c);
            }
        }
        return None;
    }
    // one block up: needs room above our head to rise into
    let up = [nx, y + 1, nz];
    if standable(world, up) && !solid(world, x, y + 2, z) {
        return Some(up);
    }
    None
}

#[derive(Clone, Copy, PartialEq)]
struct Open {
    f: f32,
    g: f32,
    cell: Cell,
}
impl Eq for Open {}
impl Ord for Open {
    fn cmp(&self, o: &Self) -> Ordering {
        o.f.partial_cmp(&self.f)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.g.partial_cmp(&o.g).unwrap_or(Ordering::Equal))
    }
}
impl PartialOrd for Open {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// What a search is looking for: any node within `range` (horizontally) of
/// `to`, and within `height` blocks of it vertically.
#[derive(Clone, Copy, Debug)]
pub struct Goal {
    pub to: [f32; 3],
    pub range: f32,
    pub height: f32,
}

impl Goal {
    pub fn new(to: [f32; 3], range: f32) -> Self {
        Goal {
            to,
            range,
            height: 3.0,
        }
    }

    fn hdist(&self, c: Cell) -> f32 {
        let dx = c[0] as f32 + 0.5 - self.to[0];
        let dz = c[2] as f32 + 0.5 - self.to[2];
        (dx * dx + dz * dz).sqrt()
    }

    pub fn reached(&self, c: Cell) -> bool {
        self.hdist(c) <= self.range && (c[1] as f32 - self.to[1]).abs() <= self.height
    }

    fn h(&self, c: Cell) -> f32 {
        (self.hdist(c) - self.range).max(0.0)
    }
}

/// A found path: the nodes to walk through, the first being where we stand.
#[derive(Clone, Debug, Default)]
pub struct Path {
    pub cells: Vec<Cell>,
    /// true when the search ran out before reaching the goal (the path leads
    /// to the closest node it found)
    pub partial: bool,
    pub expanded: usize,
}

/// A* from `start` to any node that meets `goal`. Gives up after `budget`
/// expanded nodes, returning the path to the closest node found if that is
/// nearer the goal than the start (a partial path), or None.
pub fn find_path(world: &World, start: Cell, goal: Goal, budget: usize) -> Option<Path> {
    if !standable(world, start) {
        return None;
    }
    let mut open = BinaryHeap::new();
    let mut came: HashMap<Cell, (Cell, f32)> = HashMap::new();
    came.insert(start, (start, 0.0));
    open.push(Open {
        f: goal.h(start),
        g: 0.0,
        cell: start,
    });
    let mut best = (goal.h(start), start);
    let mut expanded = 0;
    const DIRS: [(i32, i32); 8] = [
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (1, -1),
        (-1, 1),
        (-1, -1),
    ];
    while let Some(Open { g, cell, .. }) = open.pop() {
        if came.get(&cell).is_some_and(|(_, bg)| *bg < g - 1e-4) {
            continue;
        }
        if goal.reached(cell) {
            return Some(Path {
                cells: rebuild(&came, start, cell),
                partial: false,
                expanded,
            });
        }
        expanded += 1;
        if expanded > budget {
            break;
        }
        let h = goal.h(cell);
        if h < best.0 {
            best = (h, cell);
        }
        let here_wet = wet(world, cell);
        for (dx, dz) in DIRS {
            let diag = dx != 0 && dz != 0;
            let Some(next) = step(world, cell, dx, dz) else {
                continue;
            };
            if diag {
                // both sides must be open at our level and the target's
                let top = cell[1].max(next[1]);
                let ok = [[cell[0] + dx, cell[2]], [cell[0], cell[2] + dz]]
                    .iter()
                    .all(|[x, z]| (cell[1]..=top + 1).all(|y| !solid(world, *x, y, *z)));
                if !ok || next[1] != cell[1] {
                    continue;
                }
            }
            let mut cost = if diag { 1.414 } else { 1.0 };
            let dy = next[1] - cell[1];
            if dy > 0 {
                cost += 0.6;
            } else if dy < 0 {
                cost += 0.25 * (-dy) as f32;
            }
            if here_wet || wet(world, next) {
                cost *= 3.0;
            }
            let ng = g + cost;
            if came.get(&next).is_none_or(|(_, bg)| ng < *bg) {
                came.insert(next, (cell, ng));
                open.push(Open {
                    // a little greedy: much faster on open ground
                    f: ng + goal.h(next) * 1.3,
                    g: ng,
                    cell: next,
                });
            }
        }
    }
    if best.1 != start && best.0 + 2.0 < goal.h(start) {
        return Some(Path {
            cells: rebuild(&came, start, best.1),
            partial: true,
            expanded,
        });
    }
    None
}

fn rebuild(came: &HashMap<Cell, (Cell, f32)>, start: Cell, end: Cell) -> Vec<Cell> {
    let mut out = vec![end];
    let mut c = end;
    while c != start {
        c = came[&c].0;
        out.push(c);
    }
    out.reverse();
    out
}

/// Path length in blocks (horizontal).
pub fn length(cells: &[Cell]) -> f32 {
    cells
        .windows(2)
        .map(|p| {
            let dx = (p[1][0] - p[0][0]) as f32;
            let dz = (p[1][2] - p[0][2]) as f32;
            (dx * dx + dz * dz).sqrt()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat stone floor at y = 4 (feet at y = 5).
    fn floor(n: u32) -> World {
        let mut wd = World::empty([n, 32, n]);
        for x in 0..n as i32 {
            for z in 0..n as i32 {
                wd.set(x, 4, z, w::STONE);
            }
        }
        wd
    }

    #[test]
    fn straight_line_on_flat_ground() {
        let wd = floor(32);
        let p = find_path(&wd, [2, 5, 2], Goal::new([20.5, 5.0, 2.5], 0.5), 10_000).unwrap();
        assert!(!p.partial);
        assert_eq!(*p.cells.last().unwrap(), [20, 5, 2]);
        assert!((length(&p.cells) - 18.0).abs() < 0.01);
    }

    #[test]
    fn goes_around_a_wall() {
        let mut wd = floor(32);
        // a wall across x = 10 from z = 0 to z = 20, three high
        for z in 0..=20 {
            for y in 5..8 {
                wd.set(10, y, z, w::STONE);
            }
        }
        let p = find_path(&wd, [5, 5, 5], Goal::new([15.5, 5.0, 5.5], 0.5), 50_000).unwrap();
        assert!(!p.partial);
        assert!(p.cells.iter().any(|c| c[2] > 20), "went round the end");
        assert!(p.cells.iter().all(|c| c[0] != 10 || c[2] > 20));
    }

    #[test]
    fn steps_up_one_block_but_not_two() {
        let mut wd = floor(32);
        // a one-high step at x >= 10
        for x in 10..32 {
            for z in 0..32 {
                wd.set(x, 5, z, w::STONE);
            }
        }
        let p = find_path(&wd, [5, 5, 5], Goal::new([15.5, 6.0, 5.5], 0.5), 10_000).unwrap();
        assert_eq!(*p.cells.last().unwrap(), [15, 6, 5]);
        // a two-high cliff all round: no way up
        let mut wd = floor(32);
        for x in 10..32 {
            for z in 0..32 {
                wd.set(x, 5, z, w::STONE);
                wd.set(x, 6, z, w::STONE);
            }
        }
        let p = find_path(&wd, [5, 5, 5], Goal::new([15.5, 7.0, 5.5], 0.5), 10_000);
        // only a partial path to the foot of the cliff
        assert!(p.is_none_or(|p| p.partial));
    }

    #[test]
    fn drops_down_but_not_too_far() {
        let mut wd = World::empty([32, 32, 32]);
        // high platform at y 10, low ground at y 4 (a 6 block drop) and a
        // staircase down at z >= 20
        for x in 0..32 {
            for z in 0..32 {
                wd.set(x, 4, z, w::STONE);
                if x < 10 {
                    wd.set(x, 10, z, w::STONE);
                }
            }
        }
        for (i, x) in (10..14).enumerate() {
            for z in 20..24 {
                for y in 4..=(9 - 2 * i as i32) {
                    wd.set(x, y, z, w::STONE);
                }
            }
        }
        let p = find_path(&wd, [5, 11, 5], Goal::new([20.5, 5.0, 5.5], 0.5), 50_000).unwrap();
        assert!(!p.partial);
        assert!(p.cells.iter().any(|c| c[2] >= 20), "took the stairs");
        for pair in p.cells.windows(2) {
            assert!(pair[0][1] - pair[1][1] <= MAX_DROP);
        }
    }

    #[test]
    fn range_goal_stops_short() {
        let wd = floor(32);
        let p = find_path(&wd, [2, 5, 2], Goal::new([20.5, 5.0, 2.5], 4.0), 10_000).unwrap();
        let end = *p.cells.last().unwrap();
        assert!(end[0] >= 16 && end[0] <= 17, "{end:?}");
    }

    #[test]
    fn diagonal_does_not_cut_corners() {
        let mut wd = floor(16);
        wd.set(6, 5, 5, w::STONE);
        wd.set(6, 6, 5, w::STONE);
        assert!(step(&wd, [5, 5, 5], 1, 1).is_some());
        let p = find_path(&wd, [5, 5, 4], Goal::new([6.5, 5.0, 6.5], 0.1), 1000).unwrap();
        for pair in p.cells.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if a[0] != b[0] && a[2] != b[2] {
                assert!(!solid(&wd, b[0], 5, a[2]) && !solid(&wd, a[0], 5, b[2]));
            }
        }
    }

    #[test]
    fn swims_across_water() {
        let mut wd = floor(32);
        // a pool from x 8 to 20, water from y 2 to 5 (surface at feet level 5)
        for x in 8..20 {
            for z in 0..32 {
                for y in 2..=4 {
                    wd.set(x, y, z, w::WATER);
                }
                wd.set(x, 1, z, w::STONE);
            }
        }
        let p = find_path(&wd, [4, 5, 4], Goal::new([24.5, 5.0, 4.5], 0.5), 20_000).unwrap();
        assert!(!p.partial);
        assert!(p.cells.iter().any(|c| wet(&wd, *c)));
    }

    #[test]
    fn partial_path_when_out_of_budget() {
        let wd = floor(128);
        let p = find_path(&wd, [2, 5, 2], Goal::new([120.5, 5.0, 120.5], 0.5), 50).unwrap();
        assert!(p.partial);
        let end = *p.cells.last().unwrap();
        assert!(end[0] > 2 || end[2] > 2);
    }
}
