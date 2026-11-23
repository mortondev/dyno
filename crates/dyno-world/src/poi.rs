//! The landmarks of the zone around Hearthstead: each one is a small scene
//! that tells what happened there.
//!
//! `plan` levels the ground of every site that fits in the world, before the
//! terrain is written; `build` then puts the buildings on it. Everything is
//! placed through [`Zone::put`], which never replaces what a player made.

use super::zones::*;
use super::*;

/// Ground level of every site that is built, None for the ones left out.
#[derive(Default)]
pub(crate) struct Sites {
    watch: Option<i32>,
    camp: Option<i32>,
    oak: Option<i32>,
    barrow: Option<i32>,
    mill: Option<i32>,
    landing: Option<i32>,
    shrine: Option<i32>,
    caravan: Option<i32>,
    stockade: Option<i32>,
    toll: Option<i32>,
    rest: Option<i32>,
    /// the mine yard and the top of the ridge above it
    mine: Option<(i32, i32)>,
    farms: [Option<i32>; 3],
}

const WATCH: [i32; 2] = [0, -56];
const CAMP: [i32; 2] = [-45, -88];
const OAK: [i32; 2] = [-100, -55];
const BARROW: [i32; 2] = [-130, 10];
const MILL: [i32; 2] = [15, 95];
const LANDING: [i32; 2] = [-30, 135];
const SHRINE: [i32; 2] = [20, 155];
const CARAVAN: [i32; 2] = [50, -12];
const STOCKADE: [i32; 2] = [62, 60];
const TOLL: [i32; 2] = [81, -8];
const REST: [i32; 2] = [130, 8];
/// Farmsteads of Hearthmeadow.
const FARMS: [[i32; 2]; 3] = [[-54, 24], [54, -34], [-26, 60]];
/// Sheep fields of Hearthmeadow: x range, z range, side of the gate.
const PASTURES: [([i32; 2], [i32; 2], u8); 5] = [
    ([44, 62], [-62, -47], 2),
    ([42, 57], [22, 36], 3),
    ([-63, -46], [42, 57], 0),
    ([-62, -45], [-56, -42], 1),
    ([-70, -56], [-10, 6], 1),
];

/// Level the ground of the sites.
pub(crate) fn plan(z: &mut Zone) -> Sites {
    let mut s = Sites::default();
    let pad = |z: &mut Zone, c: [i32; 2], r: [i32; 2], lift: i32| {
        z.fits(
            [c[0] - r[0] - 8, c[0] + r[0] + 8],
            [c[1] - r[1] - 8, c[1] + r[1] + 8],
        )
        .then(|| z.pad(c, r, lift))
    };
    s.watch = pad(z, WATCH, [5, 5], 0);
    s.camp = pad(z, CAMP, [11, 9], 0);
    s.oak = pad(z, OAK, [8, 8], 0);
    s.barrow = pad(z, BARROW, [14, 13], 0);
    s.mill = pad(z, [MILL[0] + 9, MILL[1] + 1], [17, 11], 0);
    s.caravan = pad(z, CARAVAN, [4, 2], 0);
    s.stockade = pad(z, STOCKADE, [11, 11], 0);
    s.rest = pad(z, [REST[0] - 2, REST[1]], [10, 8], 0);
    for (i, c) in FARMS.into_iter().enumerate() {
        s.farms[i] = pad(z, c, [8, 7], 0);
    }
    if let Some(lake) = z.lake_level {
        z.pad_at(LANDING, [7, 9], lake + 1, 5);
        s.landing = Some(lake + 1);
        s.shrine = Some(lake);
    }
    if z.bridge.is_some() {
        s.toll = pad(z, TOLL, [3, 3], 0);
    }
    if z.has_ridge {
        // the yard is the bowl the ridge left open; the headframe stands on
        // the rock above its east wall
        // (high enough above the bedrock for three levels of mine below it)
        let yard = z.ground(YARD[0], YARD[1]).max(29);
        for dz in -8..=8 {
            for dx in -8..=6 {
                let (x, zz) = (YARD[0] + dx, YARD[1] + dz);
                if in_yard(x, zz) {
                    z.set_ground(x, zz, yard);
                    z.flag(x, zz, SITE);
                }
            }
        }
        let top = z.ground(HEADFRAME[0], HEADFRAME[1]);
        z.pad_at(HEADFRAME, [3, 3], top, 0);
        s.mine = Some((yard, top));
    }
    s
}

/// Build the landmarks on their levelled sites.
pub(crate) fn build(z: &mut Zone, s: &Sites) {
    if let Some(l) = s.watch {
        north_watch(z, l);
    }
    if let Some(l) = s.camp {
        logging_camp(z, l);
    }
    if let Some(l) = s.oak {
        hollow_oak(z, l);
    }
    if let Some(l) = s.barrow {
        old_barrow(z, l);
    }
    if let Some(l) = s.mill {
        windmill(z, l);
    }
    if let Some(l) = s.landing {
        fishers_landing(z, l);
    }
    if let Some(l) = s.shrine {
        sunken_shrine(z, l);
    }
    if let Some(l) = s.caravan {
        wrecked_caravan(z, l);
    }
    if let Some(l) = s.stockade {
        stockade(z, l);
    }
    if let Some(b) = z.bridge {
        bridge(z, b);
    }
    if let Some(l) = s.toll {
        toll_hut(z, l);
    }
    if let Some(l) = s.rest {
        miners_rest(z, l);
    }
    if let Some((yard, top)) = s.mine {
        mine_head(z, yard, top);
    }
    for (i, c) in FARMS.into_iter().enumerate() {
        if let Some(l) = s.farms[i] {
            farmstead(z, c, l, i);
        }
    }
    for (x, zz, gate) in PASTURES {
        if z.fits([x[0] - 2, x[1] + 2], [zz[0] - 2, zz[1] + 2]) {
            pasture(z, x, zz, gate);
        }
    }
}

// ------------------------------------------------------------------ parts

fn pick(x: i32, y: i32, z: i32, salt: u32, of: &[BlockId]) -> BlockId {
    of[(hash3(x, y, z, salt) % of.len() as u32) as usize]
}

/// Old masonry: stone bricks with moss and the odd cobble.
fn masonry(x: i32, y: i32, z: i32) -> BlockId {
    pick(
        x,
        y,
        z,
        0x57,
        &[
            STONE_BRICKS,
            STONE_BRICKS,
            STONE_BRICKS,
            MOSSY,
            STONE_BRICKS,
            STONE_BRICKS,
            STONE_BRICKS,
        ],
    )
}

struct Hut {
    x: [i32; 2],
    z: [i32; 2],
    /// height of the walls above the floor
    h: i32,
    wall: BlockId,
    frame: BlockId,
    roof: BlockId,
    floor: BlockId,
    /// the door: side (0 north, 1 east, 2 south, 3 west) and its coordinate
    door: (u8, i32),
    /// the ridge runs east-west
    ridge_x: bool,
}

/// A small house: floor, framed walls with a window in each, a door and a
/// stepped pitched roof with an overhang of one block.
fn hut(z: &mut Zone, h: &Hut) {
    let (x, zz) = (h.x, h.z);
    z.fill([x[0], 0, zz[0]], [x[1], 0, zz[1]], h.floor);
    z.fill([x[0], 1, zz[0]], [x[1], h.h + 4, zz[1]], AIR);
    for cz in zz[0]..=zz[1] {
        for cx in x[0]..=x[1] {
            let (ex, ez) = (cx == x[0] || cx == x[1], cz == zz[0] || cz == zz[1]);
            if !ex && !ez {
                continue;
            }
            let mid = if ez {
                cx * 2 == x[0] + x[1]
            } else {
                cz * 2 == zz[0] + zz[1]
            };
            for y in 1..=h.h {
                let b = if (ex && ez) || y == h.h {
                    h.frame
                } else if mid && y == 2 {
                    LEADED_WINDOW
                } else {
                    h.wall
                };
                z.put(cx, y, cz, b);
            }
        }
    }
    let (dx, dz) = match h.door.0 {
        0 => (h.door.1, zz[0]),
        1 => (x[1], h.door.1),
        2 => (h.door.1, zz[1]),
        _ => (x[0], h.door.1),
    };
    z.column(dx, dz, 1..=2, AIR);
    z.put(dx, 3.min(h.h), dz, h.frame);
    // the roof
    let (lo, hi, la, lb) = if h.ridge_x {
        (zz[0], zz[1], x[0], x[1])
    } else {
        (x[0], x[1], zz[0], zz[1])
    };
    for k in 0.. {
        let (a, b) = (lo - 1 + k, hi + 1 - k);
        if a > b {
            break;
        }
        let y = h.h + k;
        let set = |z: &mut Zone, l: i32, c: i32, block: BlockId| {
            if h.ridge_x {
                z.put(l, y, c, block);
            } else {
                z.put(c, y, l, block);
            }
        };
        for l in la - 1..=lb + 1 {
            set(z, l, a, h.roof);
            set(z, l, b, h.roof);
        }
        if k > 0 {
            for c in a + 1..b {
                let block = if c - lo == hi - c { h.frame } else { h.wall };
                set(z, la, c, block);
                set(z, lb, c, block);
            }
        }
    }
}

/// An A-frame tent of wool: `len` long from (x, zc), `half` to each side.
/// The far end is closed, the near end open. With `along_x` it runs along
/// x from `x` and `zc` is its middle z; otherwise it runs along z from `x`
/// and `zc` is its middle x. A negative `len` runs it backwards.
fn tent(z: &mut Zone, x: i32, zc: i32, len: i32, half: i32, along_x: bool, cloth: [BlockId; 2]) {
    let (dir, len) = (len.signum(), len.abs());
    for l in 0..len {
        for k in -half..=half {
            let y = 1 + half - k.abs();
            let a = x + dir * l;
            let (px, pz) = if along_x { (a, zc + k) } else { (zc + k, a) };
            let b = cloth[usize::from(l % 3 == 1)];
            z.put(px, y, pz, b);
            for h in 1..y {
                z.put(px, h, pz, if l == len - 1 { cloth[0] } else { AIR });
            }
        }
    }
    // the ridge pole
    let (px, pz) = if along_x {
        (x - dir, zc)
    } else {
        (zc, x - dir)
    };
    z.column(px, pz, 1..=half + 1, TIMBER_BEAM);
}

/// A lean-to: three plank walls and a sloping roof, open to the south.
fn lean_to(z: &mut Zone, x: [i32; 2], zz: [i32; 2], roof: BlockId) {
    for cx in x[0]..=x[1] {
        for cz in zz[0]..=zz[1] {
            let side = cx == x[0] || cx == x[1];
            let back = cz == zz[0];
            if (side && (cz == zz[0] || cz == zz[1])) || (back && side) {
                z.column(cx, cz, 1..=3, TIMBER_BEAM);
            } else if back || (side && cz < zz[1]) {
                z.column(cx, cz, 1..=2, PLANKS);
            }
            let y = if cz - zz[0] < 2 { 3 } else { 4 };
            z.put(cx, y, cz, roof);
        }
        z.put(cx, 4, zz[1] + 1, roof);
    }
}

/// A pile of logs along x: three, then one on top.
fn log_pile(z: &mut Zone, x: [i32; 2], zc: i32, log: BlockId) {
    for cx in x[0]..=x[1] {
        for k in -1..=1 {
            z.put(cx, 1, zc + k, log);
        }
        if cx > x[0] {
            z.put(cx, 2, zc, log);
        }
    }
}

// ------------------------------------------------------------- landmarks

/// North Watch: a stone tower with a burning brazier and nobody home.
fn north_watch(z: &mut Zone, l: i32) {
    z.origin(WATCH[0], l, WATCH[1]);
    z.fill([-3, 0, -3], [3, 0, 3], FLAGSTONES);
    z.fill([-2, 1, -2], [2, 15, 2], AIR);
    for y in 1..=14 {
        for c in -3..=3i32 {
            for (x, zz) in [(c, -3), (c, 3), (-3, c), (3, c)] {
                let slit = (x == 0 || zz == 0) && matches!(y, 6 | 7 | 10 | 11);
                z.put(x, y, zz, if slit { AIR } else { masonry(x, y, zz) });
            }
        }
    }
    // the door looks back at the town
    z.column(0, 3, 1..=2, AIR);
    z.put(0, 0, 4, FLAGSTONES);
    z.put(0, 0, 5, FLAGSTONES);
    // wooden treads wind up the inside of the wall
    let ring = [
        (1, 2),
        (2, 2),
        (2, 1),
        (2, 0),
        (2, -1),
        (2, -2),
        (1, -2),
        (0, -2),
        (-1, -2),
        (-2, -2),
        (-2, -1),
        (-2, 0),
        (-2, 1),
    ];
    for (k, (x, zz)) in ring.into_iter().enumerate() {
        z.put(x, k as i32 + 1, zz, PLANKS);
        if k == 1 {
            z.put(x, 1, zz, DARK_PLANKS);
        }
    }
    // the fighting deck, flared out on corbels
    z.fill([-2, 14, -2], [2, 14, 2], PLANKS);
    z.put(-2, 14, 0, AIR);
    z.put(-2, 14, 1, AIR);
    for c in -4..=4i32 {
        for (x, zz) in [(c, -4), (c, 4), (-4, c), (4, c)] {
            if (x + zz).rem_euclid(2) == 0 {
                z.put(x, 13, zz, STONE_BRICKS);
                z.put(x, 15, zz, masonry(x, 15, zz));
            }
            z.put(x, 14, zz, masonry(x, 14, zz));
        }
    }
    z.put(0, 15, 0, STONE_BRICKS);
    z.put(0, 16, 0, BRAZIER);
    // a lamp under the deck, the watchman's banner over the door
    z.put(0, 13, 0, LANTERN);
    z.column(0, 4, 9..=11, RED_BANNER);
    // the bedroll is cold, the fire by the door long out
    z.put(-1, 1, -1, BED);
    z.put(1, 1, -1, CRATE);
    z.put(0, 1, -1, BARREL);
    z.put(3, 0, 6, COAL_ORE);
    z.put(4, 0, 6, COBBLE);
    z.put(2, 0, 6, COBBLE);
    z.put(3, 0, 5, COBBLE);
    z.put(3, 0, 7, COBBLE);
    z.put(5, 1, 7, LOG);
    z.put(-3, 1, 5, CRATE);
}

/// The Logging Camp: sheds, tents and log piles along the road, and a birch
/// left lying where it fell.
fn logging_camp(z: &mut Zone, l: i32) {
    z.origin(CAMP[0], l, CAMP[1]);
    // sheds north of the road
    lean_to(z, [-10, -5], [-8, -5], DARK_PLANKS);
    z.put(-9, 1, -7, CRAFTING_TABLE);
    z.put(-8, 1, -7, CRATE);
    z.put(-6, 1, -7, BARREL);
    z.put(-6, 2, -7, BARREL);
    lean_to(z, [4, 9], [-8, -5], DARK_PLANKS);
    for x in 5..=8 {
        z.put(x, 1, -7, BIRCH_LOG);
        z.put(x, 1, -6, LOG);
        if x < 8 {
            z.put(x, 2, -7, BIRCH_LOG);
        }
    }
    log_pile(z, [-3, 2], -6, BIRCH_LOG);
    log_pile(z, [-10, -6], -1, LOG);
    // tents south of the road
    tent(z, 6, -8, 4, 2, false, [WOOL_WHITE, WOOL_WHITE]);
    tent(z, 6, 8, 4, 2, false, [WOOL_WHITE, WOOL_YELLOW]);
    z.put(-8, 1, 8, BED);
    z.put(8, 1, 8, BED);
    // the fire, with logs to sit on
    z.put(0, 0, 7, COBBLE);
    z.put(0, 1, 7, BRAZIER);
    for (x, zz) in [(1, 7), (-1, 7), (0, 6), (0, 8)] {
        z.put(x, 0, zz, COBBLE);
    }
    z.put(-2, 1, 8, LOG);
    z.put(2, 1, 8, LOG);
    z.put(2, 1, 6, LOG);
    // a birch dropped across the yard, its stump beside it; the saw horse
    // still holds a half-cut log
    z.column(3, -1, 1..=2, BIRCH_LOG);
    for x in 4..=8 {
        z.put(x, 1, -2, BIRCH_LOG);
    }
    for (x, zz) in [(9, -2), (9, -1), (9, -3), (10, -2)] {
        z.put(x, 1, zz, BIRCH_LEAVES);
    }
    z.put(9, 2, -2, BIRCH_LEAVES);
    z.put(-4, 1, 0, TIMBER_BEAM);
    z.put(-2, 1, 0, TIMBER_BEAM);
    z.put(-4, 2, 0, BIRCH_LOG);
    z.put(-3, 2, 0, BIRCH_LOG);
    z.put(-2, 2, 0, BIRCH_LOG);
    z.put(-1, 1, 0, BIRCH_LOG);
    // stumps all round
    for (x, zz) in [(-11, 5), (11, -1), (-11, -3), (11, 5), (-3, 9), (4, 5)] {
        z.put(x, 1, zz, if (x + zz) % 2 == 0 { BIRCH_LOG } else { LOG });
    }
    z.put(11, 1, -4, CRATE);
    z.put(11, 1, -5, BARREL);
    let (x, zz) = (CAMP[0], CAMP[1]);
    z.lamp(x - 5, zz);
    z.lamp(x + 4, zz + 4);
}

/// The Hollow Oak: a giant dead tree hung with webs, with what the
/// Websnares caught wrapped up round its roots.
fn hollow_oak(z: &mut Zone, l: i32) {
    z.origin(OAK[0], l, OAK[1]);
    let radius = |y: i32| 4.3 - 2.6 * smooth((y - 1) as f32 / 15.0);
    for y in 1..=22 {
        let r = radius(y);
        for dz in -5..=5i32 {
            for dx in -5..=5i32 {
                let d2 = (dx * dx + dz * dz) as f32;
                // the bark is ragged, and the top is broken off
                let rag = (hash3(dx, y / 2, dz, 0x0a4) % 10) as f32 * 0.06;
                let snapped = y > 19 && hash3(dx, y, dz, 0x0a5) % 3 < (y - 19) as u32;
                if d2 <= (r + rag) * (r + rag) && !snapped {
                    let inside = y <= 6 && d2 <= (r - 1.6) * (r - 1.6);
                    z.put(dx, y, dz, if inside { AIR } else { LOG });
                }
            }
        }
    }
    // the mouth of the hollow, on the side of the trail
    z.fill([3, 1, -1], [5, 3, 1], AIR);
    z.put(4, 4, 0, AIR);
    // roots
    for (dx, dz, n) in [
        (1, 1, 3),
        (-1, 0, 3),
        (0, -1, 4),
        (0, 1, 3),
        (-1, 1, 4),
        (-1, -1, 3),
        (1, -1, 4),
    ] {
        let reach = if dx != 0 && dz != 0 { 3 } else { 5 };
        for k in 0..n {
            let (x, zz) = (dx * (reach + k), dz * (reach + k));
            z.put(x, 1, zz, LOG);
            if k < 2 {
                z.put(x, 2, zz, LOG);
            }
            if k == 0 {
                z.put(x, 3, zz, LOG);
            }
        }
    }
    // limbs: (height, direction, length, rise)
    let limbs: [(i32, [f32; 2], i32, i32); 8] = [
        (11, [1.0, 0.2], 9, 4),
        (13, [-1.0, 0.3], 10, 5),
        (12, [0.2, 1.0], 8, 3),
        (15, [0.1, -1.0], 9, 5),
        (17, [0.7, 0.7], 7, 4),
        (18, [-0.7, -0.6], 7, 5),
        (9, [-0.6, 0.8], 7, 2),
        (14, [0.8, -0.6], 8, 4),
    ];
    for (n, (h, d, len, up)) in limbs.into_iter().enumerate() {
        let from = radius(h) - 0.5;
        let mut end = [0, 0, 0];
        for i in 1..=len {
            let p = [
                (d[0] * (i as f32 + from)).round() as i32,
                h + i * up / len,
                (d[1] * (i as f32 + from)).round() as i32,
            ];
            z.put(p[0], p[1], p[2], LOG);
            if i <= 4 {
                z.put(p[0], p[1] - 1, p[2], LOG);
            }
            end = p;
        }
        // twigs at the end
        let (sx, sz) = (d[0].signum() as i32, d[1].signum() as i32);
        z.put(end[0] + sx, end[1] + 1, end[2], LOG);
        z.put(end[0], end[1] + 1, end[2] + sz, LOG);
        z.put(end[0] + sx, end[1] + 2, end[2] + sz, LOG);
        z.put(end[0] - sx, end[1] + 2, end[2], LOG);
        // webs: a sheet in the crook of some limbs, a thread to the ground
        // from others, and a cocoon hanging under every third
        match n % 3 {
            0 => {
                for i in 2..=len - 2 {
                    let x = (d[0] * (i as f32 + from)).round() as i32;
                    let zz = (d[1] * (i as f32 + from)).round() as i32;
                    for k in 2..=(len - i).min(3) {
                        z.put(x, h + i * up / len - k, zz, WOOL_WHITE);
                    }
                }
            }
            1 => {
                for k in 1..end[1] {
                    let (x, zz) = (end[0] + sx * k / 2, end[2] + sz * k / 2);
                    if z.look(x, end[1] - k, zz) != AIR {
                        break;
                    }
                    if k % 3 != 0 {
                        z.put(x, end[1] - k, zz, WOOL_WHITE);
                    }
                }
            }
            _ => {
                let (x, zz) = (end[0] - sx, end[2] - sz);
                let y = end[1] - 2;
                z.put(x, y, zz, WOOL_WHITE);
                z.put(x, y - 1, zz, CRATE);
                z.put(x, y - 2, zz, WOOL_WHITE);
            }
        }
    }
    // cocooned crates round the roots and in the hollow, webs on the ground
    for (x, zz) in [(6, 4), (-6, -3), (3, -7), (-4, 7)] {
        z.put(x, 1, zz, CRATE);
        z.put(x, 2, zz, WOOL_WHITE);
        z.put(x + 1, 1, zz, WOOL_WHITE);
        z.put(x, 1, zz - 1, WOOL_WHITE);
    }
    z.put(0, 1, 1, CRATE);
    z.put(0, 2, 1, WOOL_WHITE);
    z.put(-1, 1, 0, CRATE);
    z.put(-1, 1, -1, WOOL_WHITE);
    z.put(0, 1, -1, WOOL_WHITE);
    for dz in -8..=8i32 {
        for dx in -8..=8i32 {
            let d2 = dx * dx + dz * dz;
            if d2 > 20
                && d2 < 70
                && rnd(dx, dz, 0x3eb).is_multiple_of(9)
                && z.look(dx, 1, dz) == AIR
            {
                z.put(dx, 0, dz, WOOL_WHITE);
            }
        }
    }
}

/// The Old Barrow: a mossy mound over a crypt of three dark rooms.
fn old_barrow(z: &mut Zone, l: i32) {
    z.origin(BARROW[0], l, BARROW[1]);
    let height = |dx: i32, dz: i32| {
        let d = ((dx * dx + dz * dz) as f32).sqrt();
        if d < 12.0 {
            (7.6 * (d / 12.0 * std::f32::consts::FRAC_PI_2).cos()).round() as i32
        } else {
            0
        }
    };
    for dz in -12..=12i32 {
        for dx in -12..=12i32 {
            let h = height(dx, dz);
            for y in 1..=h {
                let b = if y == h {
                    pick(
                        dx,
                        y,
                        dz,
                        0xba,
                        &[GRASS, MOSSY, GRASS, COBBLE, GRASS, MOSSY, GRASS],
                    )
                } else if y >= h - 1 {
                    DIRT
                } else {
                    COBBLE
                };
                z.put(dx, y, dz, b);
            }
        }
    }
    // the crypt: antechamber, great chamber, burial room. Floor four down.
    let rooms: [([i32; 2], [i32; 2]); 3] =
        [([2, 6], [-2, 2]), ([-7, -1], [-3, 3]), ([-6, -2], [-9, -5])];
    for (x, zz) in rooms {
        for cz in zz[0] - 1..=zz[1] + 1 {
            for cx in x[0] - 1..=x[1] + 1 {
                for y in -4..=0 {
                    let b = if y == -4 {
                        pick(cx, y, cz, 0xbb, &[FLAGSTONES, FLAGSTONES, MOSSY, COBBLE])
                    } else {
                        masonry(cx, y, cz)
                    };
                    z.put(cx, y, cz, b);
                }
            }
        }
    }
    for (x, zz) in rooms {
        z.fill([x[0], -3, zz[0]], [x[1], -1, zz[1]], AIR);
    }
    // doors between the rooms
    z.column(1, 0, -3..=-2, AIR);
    z.column(0, 0, -3..=-2, AIR);
    z.column(-4, -4, -3..=-2, AIR);
    // the way in from the east: steps down under a stone porch
    for x in 7..=12 {
        let fl = (x - 11).clamp(-4, 0);
        for k in -2..=2i32 {
            if k.abs() == 2 {
                for y in fl..=3 {
                    z.put(x, y, k, masonry(x, y, k));
                }
            } else {
                z.put(x, fl, k, masonry(x, fl, k));
                z.column(x, k, fl + 1..=(fl + 3).min(2), AIR);
                z.put(x, 3, k, STONE_BRICKS);
                if fl + 3 < 2 {
                    z.column(x, k, fl + 4..=2, STONE_BRICKS);
                }
            }
        }
    }
    for k in [-2, 2] {
        z.column(13, k, 1..=3, MOSSY);
        z.column(14, 2 * k - k.signum(), 1..=2, MOSSY);
    }
    for k in -2..=2 {
        z.put(13, 4, k, STONE_BRICKS);
        z.put(12, 4, k, MOSSY);
    }
    // the great chamber: pillars and a stone coffin, its lid pushed aside
    for (x, zz) in [(-6, -2), (-6, 2), (-2, -2), (-2, 2)] {
        z.column(x, zz, -3..=-1, STONE_BRICKS);
    }
    z.fill([-5, -3, -1], [-5, -3, 1], STONE_BRICKS);
    z.put(-5, -2, -1, MOSSY);
    z.put(-4, -3, -1, MOSSY);
    // urns in the antechamber
    z.put(2, -3, -2, BARREL);
    z.put(2, -3, 2, BARREL);
    z.put(6, -3, 2, CRATE);
    // the burial room: three plain graves and what was left with them
    for x in [-6, -4, -2] {
        z.put(x, -3, -9, STONE_BRICKS);
        z.put(x, -3, -8, MOSSY);
    }
    z.put(-2, -3, -5, CRATE);
    z.put(-6, -3, -5, BARREL);
    // standing stones on the mound, and a dead tree on its crown
    for (x, zz, h) in [
        (-8, -5, 3),
        (-9, 3, 2),
        (-4, 9, 3),
        (4, 9, 2),
        (5, -9, 3),
        (-2, -10, 2),
    ] {
        let g = height(x, zz);
        z.column(x, zz, g + 1..=g + h, MOSSY);
    }
    let g = height(-1, 1);
    z.column(-1, 1, g + 1..=g + 6, LOG);
    for (x, y, zz) in [
        (0, 5, 1),
        (1, 6, 1),
        (-1, 6, 0),
        (-1, 7, -1),
        (-2, 4, 1),
        (-3, 5, 2),
        (-1, 7, 1),
    ] {
        z.put(x, g + y, zz, LOG);
    }
}

/// The windmill farm: the mill with its sails towards the town, a field
/// half cut, and grain sacks slashed open by the door.
fn windmill(z: &mut Zone, l: i32) {
    z.origin(MILL[0], l, MILL[1]);
    let oct =
        |dx: i32, dz: i32, r: i32| dx.abs() <= r && dz.abs() <= r && dx.abs() + dz.abs() < 2 * r;
    for dz in -4..=4i32 {
        for dx in -4..=4i32 {
            if oct(dx, dz, 3) {
                z.put(dx, 0, dz, FLAGSTONES);
                let wall = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .any(|d| !oct(dx + d.0, dz + d.1, 3));
                for y in 1..=11 {
                    let b = if !wall {
                        AIR
                    } else if y <= 2 {
                        masonry(dx, y, dz)
                    } else if dx.abs() + dz.abs() == 5 || matches!(y, 3 | 7 | 11) {
                        TIMBER_BEAM
                    } else {
                        PLASTER
                    };
                    z.put(dx, y, dz, b);
                }
            }
            // the cap
            for (y, r) in [(12, 4), (13, 3), (14, 2), (15, 1)] {
                if oct(dx, dz, r) {
                    z.put(dx, y, dz, THATCH);
                }
            }
        }
    }
    z.put(0, 16, 0, THATCH);
    z.column(-3, 0, 1..=2, AIR);
    z.put(-4, 0, 0, FLAGSTONES);
    for (x, y, zz) in [(3, 5, 0), (0, 5, 3), (-3, 8, 0), (0, 9, 3), (3, 9, 0)] {
        z.put(x, y, zz, LEADED_WINDOW);
    }
    // the stone and its shaft
    z.put(0, 1, 0, STONE_BRICKS);
    z.column(0, 0, 2..=10, TIMBER_BEAM);
    z.put(1, 1, 1, HAY_BALE);
    z.put(1, 1, -1, BARREL);
    z.put(0, 4, -2, LANTERN);
    // the sails, on the north face
    z.put(0, 10, -4, TIMBER_BEAM);
    z.put(0, 10, -5, TIMBER_BEAM);
    for k in 1..=7 {
        for (i, (ax, ay)) in [(0, 1), (1, 0), (0, -1), (-1, 0)].into_iter().enumerate() {
            z.put(ax * k, 10 + ay * k, -5, TIMBER_BEAM);
            if k >= 2 {
                // the cloth trails each arm
                let (cx, cy) = [(1, 0), (0, -1), (-1, 0), (0, 1)][i];
                z.put(ax * k + cx, 10 + ay * k + cy, -5, WOOL_WHITE);
                if k >= 4 {
                    z.put(ax * k + 2 * cx, 10 + ay * k + 2 * cy, -5, WOOL_WHITE);
                }
            }
        }
    }
    // the field east of the mill: standing wheat, a cut corner, a scarecrow
    let (fx, fz): ([i32; 2], [i32; 2]) = ([7, 24], [-8, 9]);
    for cz in fz[0]..=fz[1] {
        for cx in fx[0]..=fx[1] {
            let ring = cx == fx[0] || cx == fx[1] || cz == fz[0] || cz == fz[1];
            if ring {
                let gate = cx == fx[0] && (0..=1).contains(&cz);
                let post = (cx + cz).rem_euclid(4i32) == 0;
                if !gate {
                    z.put(cx, 1, cz, if post { LOG } else { PLANKS });
                }
                continue;
            }
            z.put(cx, 0, cz, DIRT);
            let path = (0..=1).contains(&cz) || cx == 16;
            let cut = cx > 16 && cz > 1;
            if path {
                z.put(cx, 0, cz, GRASS);
            } else if cut {
                if rnd(cx, cz, 0x3a).is_multiple_of(9) {
                    z.put(cx, 1, cz, HAY_BALE);
                }
            } else if cx % 2 == 0 && !rnd(cx, cz, 0x3b).is_multiple_of(7) {
                z.put(cx, 1, cz, HAY_BALE);
            }
        }
    }
    z.column(16, -4, 1..=2, TIMBER_BEAM);
    z.put(15, 2, -4, TIMBER_BEAM);
    z.put(17, 2, -4, TIMBER_BEAM);
    z.put(16, 3, -4, HAY_BALE);
    for (x, y, zz) in [(21, 1, 6), (22, 1, 6), (21, 2, 6), (22, 1, 7), (19, 1, 4)] {
        z.put(x, y, zz, HAY_BALE);
    }
    // sacks by the door, slashed, the grain trodden into the mud
    for (x, y, zz, b) in [
        (-5, 1, 2, WOOL_WHITE),
        (-5, 1, 3, WOOL_WHITE),
        (-5, 2, 2, WOOL_WHITE),
        (-4, 1, 3, WOOL_WHITE),
        (-6, 1, 3, WOOL_YELLOW),
        (-6, 0, 4, WOOL_YELLOW),
        (-5, 0, 4, WOOL_YELLOW),
        (-7, 0, 3, WOOL_YELLOW),
        (-4, 0, 5, WOOL_YELLOW),
        (-3, 1, -3, CRATE),
        (-4, 1, -2, BARREL),
    ] {
        z.put(x, y, zz, b);
    }
    // a store shed south of the mill
    lean_to(z, [-4, 1], [6, 9], THATCH);
    z.put(-3, 1, 7, BARREL);
    z.put(-2, 1, 7, HAY_BALE);
    z.put(-2, 2, 7, HAY_BALE);
    z.put(0, 1, 7, CRATE);
    // spilled grain marks the way the thieves went: east, to the ford
    for (x, zz) in [(16, 83), (27, 84), (38, 85), (50, 87), (61, 86)] {
        if z.flags(x, zz) & WET == 0 && !z.locked(x, zz) {
            let g = z.ground(x, zz);
            if z.block(x, g + 1, zz) == AIR {
                z.set(x, g, zz, WOOL_YELLOW);
            }
        }
    }
    z.lamp(MILL[0] - 5, MILL[1] - 3);
}

/// Fisher's Landing: a hearth, two thatched huts, a jetty and empty nets.
fn fishers_landing(z: &mut Zone, l: i32) {
    z.origin(LANDING[0], l, LANDING[1]);
    for dz in -2..=4i32 {
        for dx in -3..=3i32 {
            if dx * dx + (dz - 1) * (dz - 1) <= 7 {
                z.put(
                    dx,
                    0,
                    dz,
                    if (dx + dz) % 4 == 0 {
                        COBBLE
                    } else {
                        FLAGSTONES
                    },
                );
            }
        }
    }
    z.put(0, 1, 1, HEARTH);
    for (zz, door) in [([-8, -4], -6), ([5, 9], 7)] {
        hut(
            z,
            &Hut {
                x: [-7, -3],
                z: zz,
                h: 3,
                wall: PLANKS,
                frame: TIMBER_BEAM,
                roof: THATCH,
                floor: PLANKS,
                door: (1, door),
                ridge_x: true,
            },
        );
    }
    z.put(-6, 1, -7, BED);
    z.put(-6, 1, -5, BARREL);
    z.put(-6, 1, 8, BED);
    z.put(-6, 1, 6, CRATE);
    z.put(-2, 1, -8, BARREL);
    z.put(-2, 1, -7, BARREL);
    z.put(-2, 2, -8, CRATE);
    // the jetty runs out east into the lake
    let bed = |z: &Zone, x: i32, zz: i32| z.rise(x, zz);
    for x in 6..=21 {
        for k in 0..=1 {
            z.put(x, 0, k, if x % 5 == 0 { DARK_PLANKS } else { PLANKS });
        }
    }
    z.fill([19, 0, -2], [21, 0, 3], PLANKS);
    for (x, zz) in [
        (9, -1),
        (9, 2),
        (14, -1),
        (14, 2),
        (18, -3),
        (18, 4),
        (22, -3),
        (22, 4),
    ] {
        for y in bed(z, x, zz) + 1..=1 {
            z.put(x, y, zz, LOG);
        }
    }
    z.put(22, 2, -3, LANTERN);
    z.put(20, 1, 3, BARREL);
    z.put(21, 1, 3, BARREL);
    z.put(21, 1, -2, CRATE);
    // a punt tied up beside it
    z.fill([11, -1, 3], [14, -1, 4], DARK_PLANKS);
    z.put(11, 0, 3, PLANKS);
    z.put(11, 0, 4, PLANKS);
    z.put(14, 0, 3, PLANKS);
    z.put(14, 0, 4, PLANKS);
    // the drying rack: the nets are empty
    z.column(4, -8, 1..=3, TIMBER_BEAM);
    z.column(4, -3, 1..=3, TIMBER_BEAM);
    for zz in -7..=-4 {
        z.put(4, 3, zz, TIMBER_BEAM);
        for y in 1..=2 {
            if (zz + y) % 2 == 0 {
                z.put(4, y, zz, WOOL_WHITE);
            }
        }
    }
    z.put(5, 1, 5, BARREL);
    z.put(5, 1, 6, CRATE);
    z.put(4, 1, 8, HAY_BALE);
    z.lamp(LANDING[0] + 3, LANDING[1] - 2);
    z.lamp(LANDING[0] - 1, LANDING[1] + 5);
}

/// The Sunken Shrine: a ring of broken pillars in the shallows round an
/// older hearth, drowned and still glowing.
fn sunken_shrine(z: &mut Zone, lake: i32) {
    z.origin(SHRINE[0], lake, SHRINE[1]);
    for dz in -7..=7i32 {
        for dx in -7..=7i32 {
            if dx * dx + dz * dz > 42 || z.look(dx, 0, dz) != WATER {
                continue;
            }
            for y in -6..=-1 {
                if matches!(z.look(dx, y, dz), WATER | SAND | CLAY) {
                    let b = if y == -1 {
                        masonry(dx, y, dz)
                    } else {
                        STONE_BRICKS
                    };
                    z.put(dx, y, dz, b);
                }
            }
        }
    }
    // the hearth lies a block under the water, its kerb just breaking the surface
    z.put(0, -1, 0, EMBERSTONE);
    for (x, zz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        z.put(x, 0, zz, MOSSY);
    }
    let pillars = [
        (5, 0, 5),
        (4, 4, 2),
        (0, 5, 4),
        (-4, 4, 1),
        (-5, 0, 5),
        (-4, -4, 3),
        (0, -5, 0),
        (4, -4, 4),
    ];
    for (x, zz, h) in pillars {
        for y in -1..=h {
            z.put(x, y, zz, masonry(x, y, zz));
        }
        if h <= 3 {
            z.put(x, h + 1, zz, GLOWSHROOM);
        }
    }
    // fallen drums and a broken lintel
    for (x, y, zz) in [
        (1, 0, -5),
        (2, 0, -6),
        (-2, 0, 2),
        (2, 0, 3),
        (3, 0, 3),
        (-3, 0, -2),
    ] {
        z.put(x, y, zz, masonry(x, y, zz));
    }
    z.put(2, 1, 3, GLOWSHROOM);
    z.put(-3, 1, -2, GLOWSHROOM);
    z.put(5, 5, 1, STONE_BRICKS);
    z.put(5, 5, -1, STONE_BRICKS);
    // stepping stones in from the west shore
    for (x, zz) in [(-11, 4), (-10, 4), (-9, 3), (-8, 3), (-7, 2), (-6, 2)] {
        if z.look(x, 0, zz) == WATER {
            z.put(
                x,
                0,
                zz,
                if (x + zz) % 2 == 0 {
                    MOSSY
                } else {
                    STONE_BRICKS
                },
            );
        }
    }
}

/// The Wrecked Caravan: a cart on its side, the load gone, the grass burnt.
fn wrecked_caravan(z: &mut Zone, l: i32) {
    z.origin(CARAVAN[0], l, CARAVAN[1]);
    for dz in -2..=2i32 {
        for dx in -4..=4i32 {
            let r = rnd(dx, dz, 0xca) % 5;
            if r < 3 && dx.abs() + dz.abs() < 6 {
                z.put(dx, 0, dz, [COAL_ORE, WOOL_BLACK, DIRT][r as usize]);
            }
        }
    }
    for x in -2..=2 {
        z.put(x, 1, 0, PLANKS);
        z.put(x, 2, 0, if x == 0 { DARK_PLANKS } else { PLANKS });
        z.put(x, 1, 1, DARK_PLANKS);
    }
    z.put(-2, 3, 0, PLANKS);
    z.put(-1, 2, -1, LOG);
    z.put(2, 2, -1, LOG);
    z.put(4, 1, 2, LOG);
    z.put(3, 1, 0, TIMBER_BEAM);
    z.put(4, 1, 0, TIMBER_BEAM);
    for (x, y, zz, b) in [
        (-3, 1, 1, CRATE),
        (0, 1, 2, CRATE),
        (1, 1, 2, BARREL),
        (-1, 1, -2, CRATE),
        (2, 1, 1, CRATE),
        (2, 2, 1, CRATE),
        (-4, 1, -1, TIMBER_BEAM),
        (-4, 2, -1, WOOL_BLACK),
        (3, 0, -2, RED_BANNER),
        (4, 0, -2, RED_BANNER),
    ] {
        z.put(x, y, zz, b);
    }
    // what they dropped on the way back to the stockade
    for (i, (x, zz)) in [(53, -1), (55, 11), (57, 23), (59, 35), (61, 44)]
        .into_iter()
        .enumerate()
    {
        if z.flags(x, zz) == 0 && !z.locked(x, zz) {
            let g = z.ground(x, zz);
            if z.block(x, g + 1, zz) == AIR && is_solid(z.block(x, g, zz)) {
                z.set(x, g + 1, zz, if i % 2 == 0 { CRATE } else { BARREL });
            }
        }
    }
}

/// The Ashcloak Stockade: a palisade round tents and stolen stores, with
/// the town's banners trodden in as rugs.
fn stockade(z: &mut Zone, l: i32) {
    z.origin(STOCKADE[0], l, STOCKADE[1]);
    for c in -9..=9i32 {
        for (x, zz) in [(c, -9), (c, 9), (-9, c), (9, c)] {
            if zz == 9 && x.abs() <= 1 {
                continue;
            }
            let r = rnd(x, zz, 0x57c);
            let corner = x.abs() == 9 && zz.abs() == 9;
            let h = if corner { 5 } else { 3 + (r % 2) as i32 };
            z.column(
                x,
                zz,
                1..=h,
                if r.is_multiple_of(4) {
                    TIMBER_BEAM
                } else {
                    LOG
                },
            );
        }
    }
    for x in [-2, 2] {
        z.column(x, 9, 1..=5, TIMBER_BEAM);
        z.put(x, 6, 9, WOOL_BLACK);
    }
    for x in -2..=2 {
        z.put(x, 5, 9, TIMBER_BEAM);
    }
    z.put(2, 4, 8, LANTERN);
    // a look-out in the north-east corner, with steps up
    for (x, zz) in [(6, -8), (8, -8), (6, -6), (8, -6)] {
        z.column(x, zz, 1..=3, TIMBER_BEAM);
    }
    z.fill([6, 4, -8], [8, 4, -6], PLANKS);
    for k in 1..=3 {
        z.column(5, -5 - k, 1..=k, DARK_PLANKS);
    }
    // tents: the leader's is the big one
    tent(z, -1, -5, -6, 3, true, [WOOL_BLACK, WOOL_RED]);
    tent(z, 1, 5, 4, 2, false, [WOOL_BLACK, WOOL_BLACK]);
    tent(z, 1, -6, 4, 2, false, [WOOL_BLACK, WOOL_BLACK]);
    z.put(-5, 1, -4, BED);
    z.put(-5, 1, -6, CRATE);
    z.put(-5, 2, -6, LANTERN);
    z.put(5, 1, 3, BED);
    z.put(-6, 1, 3, BED);
    // town banners for rugs
    for x in 0..=2 {
        z.put(x, 0, -6, RED_BANNER);
        z.put(x, 0, -5, BLUE_BANNER);
        z.put(x, 0, -4, RED_BANNER);
    }
    z.put(5, 0, 0, BLUE_BANNER);
    z.put(5, 0, -1, BLUE_BANNER);
    // the fire
    z.put(0, 0, 2, COBBLE);
    z.put(0, 1, 2, BRAZIER);
    z.put(-2, 1, 2, LOG);
    z.put(0, 1, 4, LOG);
    z.put(2, 1, 1, LOG);
    // the stores: barrels, crates and the miller's sacks
    for (x, y, zz, b) in [
        (8, 1, 8, BARREL),
        (7, 1, 8, BARREL),
        (8, 2, 8, BARREL),
        (8, 1, 7, CRATE),
        (6, 1, 8, CRATE),
        (7, 2, 8, CRATE),
        (8, 1, 6, BARREL),
        (5, 1, 8, WOOL_WHITE),
        (4, 1, 8, WOOL_WHITE),
        (5, 2, 8, WOOL_WHITE),
        (3, 1, 8, HAY_BALE),
        (8, 1, 5, WOOL_YELLOW),
    ] {
        z.put(x, y, zz, b);
    }
    // a cage in the south-west corner
    for cx in -8..=-5 {
        for cz in 5..=8 {
            if cx == -8 || cx == -5 || cz == 5 || cz == 8 {
                z.column(cx, cz, 1..=2, TIMBER_BEAM);
            }
            z.put(cx, 3, cz, DARK_PLANKS);
        }
    }
    z.put(-7, 1, 7, HAY_BALE);
    // sharpened stakes outside the gate
    for (x, zz) in [(-4, 11), (-3, 11), (3, 11), (4, 11)] {
        z.column(x, zz, 1..=2, TIMBER_BEAM);
    }
}

/// Emberford Bridge: three stone arches, one of them patched with planks.
fn bridge(z: &mut Zone, b: Bridge) {
    z.origin(0, b.deck, 0);
    let (w, e) = (b.x[0], b.x[1]);
    let n = e - w - 1;
    let piers = [w + 1 + n / 3, e - 1 - n / 3];
    for x in w - 2..=e + 2 {
        let wet = x > w && x < e;
        let pier = piers.contains(&x);
        let haunch = piers.iter().any(|p| (x - p).abs() == 1) || x == w + 1 || x == e - 1;
        for k in -2..=2i32 {
            let top = if k.abs() <= 1 && !rnd(x, k, 0xb1).is_multiple_of(5) {
                FLAGSTONES
            } else {
                masonry(x, 0, k)
            };
            z.put(x, 0, k, top);
            let low = z.rise(x, k) + 1;
            if !wet || pier {
                for y in low..0 {
                    z.put(x, y, k, masonry(x, y, k));
                }
            } else if haunch {
                z.put(x, -1, k, masonry(x, -1, k));
            }
            if k.abs() == 2 && !rnd(x, k, 0xb2).is_multiple_of(9) {
                z.put(x, 1, k, masonry(x, 1, k));
            }
        }
        if pier {
            // cutwaters
            for k in [-3, 3] {
                for y in z.rise(x, k) + 1..=-2 {
                    z.put(x, y, k, STONE_BRICKS);
                }
            }
        }
    }
    // the plank patch over the hole in the middle arch
    let mid = (piers[0] + piers[1]) / 2;
    for x in mid - 1..=mid + 1 {
        z.put(x, 0, 1, PLANKS);
        z.put(x, 0, 2, DARK_PLANKS);
        z.put(x, 1, 2, AIR);
        z.put(x, 0, 0, if x == mid { PLANKS } else { FLAGSTONES });
    }
    z.put(mid - 2, 2, 2, TIMBER_BEAM);
    z.put(mid + 2, 2, 2, TIMBER_BEAM);
    // lamps at the town end; the far end is dark
    for k in [-2, 2] {
        z.put(w - 2, 2, k, STONE_BRICKS);
        z.put(w - 2, 3, k, LANTERN);
        z.put(e + 2, 2, k, STONE_BRICKS);
    }
}

/// The toll hut at the west end of the bridge, and the bar across the road.
fn toll_hut(z: &mut Zone, l: i32) {
    z.origin(TOLL[0], l, TOLL[1]);
    hut(
        z,
        &Hut {
            x: [-2, 2],
            z: [-2, 2],
            h: 3,
            wall: PLANKS,
            frame: TIMBER_BEAM,
            roof: SLATE_ROOF,
            floor: PLANKS,
            door: (2, -1),
            ridge_x: true,
        },
    );
    // the hatch where the toll was paid, and the ledger inside
    z.put(1, 2, 2, AIR);
    z.put(1, 1, 2, BARREL);
    z.put(-1, 1, -1, BOOKSHELF);
    z.put(1, 1, -1, BED);
    z.put(0, 3, 1, LANTERN);
    z.put(3, 1, 3, CRATE);
    z.put(-3, 1, 2, BARREL);
    z.column(3, 3, 2..=3, BLUE_BANNER);
    z.put(3, 4, 3, TIMBER_BEAM);
    z.put(3, 4, 2, TIMBER_BEAM);
    // the bar is up: nobody is taking tolls
    let x = TOLL[0] + 2;
    for (zz, top) in [(-3, 4), (3, 2)] {
        let g = z.ground(x, zz);
        if z.flags(x, zz) & (ROAD | WET) == 0 {
            for y in 1..=top {
                z.set(x, g + y, zz, TIMBER_BEAM);
            }
            if top == 4 {
                z.set(x, g + 5, zz + 1, TIMBER_BEAM);
                z.set(x, g + 6, zz + 2, TIMBER_BEAM);
            }
        }
    }
}

/// Miner's Rest: a hearth, a tent inn and a smith's lean-to, where the
/// miners wait and will not go back down.
fn miners_rest(z: &mut Zone, l: i32) {
    z.origin(REST[0], l, REST[1]);
    for dz in 0..=6i32 {
        for dx in -5..=1i32 {
            if (dx + 2) * (dx + 2) + (dz - 3) * (dz - 3) <= 7 {
                z.put(
                    dx,
                    0,
                    dz,
                    if (dx - dz) % 4 == 0 {
                        COBBLE
                    } else {
                        FLAGSTONES
                    },
                );
            }
        }
    }
    z.put(-2, 1, 3, HEARTH);
    // the tent inn, open towards the hearth
    tent(z, -5, 5, -7, 3, true, [WOOL_WHITE, WOOL_RED]);
    z.put(-11, 1, 4, BED);
    z.put(-11, 1, 6, BED);
    z.put(-9, 1, 3, BED);
    z.put(-9, 1, 7, BARREL);
    z.put(-7, 1, 7, CRATE);
    z.column(-5, 9, 1..=3, DARK_PLANKS);
    z.put(-5, 4, 9, PLANKS);
    z.put(-4, 4, 9, PLANKS);
    z.put(-4, 3, 9, RED_BANNER);
    // the smith's lean-to
    lean_to(z, [3, 7], [3, 6], DARK_PLANKS);
    z.put(5, 1, 5, ANVIL);
    z.put(4, 0, 4, COBBLE);
    z.put(4, 1, 4, BRAZIER);
    z.put(6, 1, 4, BARREL);
    z.put(8, 1, 4, COAL_ORE);
    z.put(8, 1, 5, COAL_ORE);
    z.put(8, 2, 4, COAL_ORE);
    z.put(8, 1, 6, IRON_ORE);
    // north of the road: pit props, an ore cart on its rails, a notice board
    for x in -11..=-8 {
        z.put(x, 1, -6, TIMBER_BEAM);
        z.put(x, 1, -5, TIMBER_BEAM);
        if x > -11 && x < -8 {
            z.put(x, 2, -6, TIMBER_BEAM);
        }
    }
    for x in 2..=7 {
        z.put(x, 0, -5, if x % 2 == 0 { DARK_PLANKS } else { PLANKS });
    }
    z.put(4, 1, -5, CRATE);
    z.put(5, 1, -5, IRON_ORE);
    z.put(0, 1, -6, CRATE);
    z.put(1, 1, -6, CRATE);
    z.put(0, 2, -6, BARREL);
    z.put(3, 1, -7, DARK_PLANKS);
    z.put(4, 1, -7, DARK_PLANKS);
    z.fill([3, 2, -7], [4, 3, -7], PLANKS);
    z.put(3, 3, -6, BLUE_BANNER);
    tent(z, -4, -4, -3, 2, false, [WOOL_WHITE, WOOL_WHITE]);
    z.lamp(REST[0] - 1, REST[1] - 4);
    z.lamp(REST[0] + 2, REST[1] + 1);
    z.lamp(REST[0] + 7, REST[1] - 4);
}

/// The yard in front of the mine entrance: a rounded bowl in the ridge.
pub(crate) fn in_yard(x: i32, z: i32) -> bool {
    let (dx, dz) = (x - YARD[0], z - YARD[1]);
    (-7..=5).contains(&dx) && dz.abs() <= 6 && dx * dx + dz * dz < 57
}

/// The mine head: the timbered entrance in the yard wall, and the
/// headframe on the ridge above it with a brazier on top.
fn mine_head(z: &mut Zone, yard: i32, top: i32) {
    z.origin(0, yard, 0);
    for zz in [-22, -18] {
        z.column(160, zz, 1..=4, TIMBER_BEAM);
        z.put(159, 4, zz, LANTERN);
    }
    for zz in -22..=-18 {
        z.put(160, 4, zz, TIMBER_BEAM);
        z.put(160, 5, zz, if zz % 2 == 0 { PLANKS } else { DARK_PLANKS });
    }
    for x in 152..=160 {
        z.put(x, 0, -20, if x % 2 == 0 { DARK_PLANKS } else { PLANKS });
    }
    z.put(155, 1, -20, CRATE);
    z.put(154, 1, -20, IRON_ORE);
    // spoil, props and barrels round the yard
    for (x, y, zz, b) in [
        (151, 1, -13, COBBLE),
        (152, 1, -13, STONE),
        (151, 1, -12, STONE),
        (152, 2, -13, COBBLE),
        (150, 1, -14, COBBLE),
        (158, 1, -12, TIMBER_BEAM),
        (159, 1, -12, TIMBER_BEAM),
        (158, 1, -13, TIMBER_BEAM),
        (159, 2, -12, TIMBER_BEAM),
        (159, 1, -15, BARREL),
        (152, 1, -22, BARREL),
        (153, 1, -22, CRATE),
    ] {
        z.put(x, y, zz, b);
    }
    // the headframe
    z.origin(HEADFRAME[0], top, HEADFRAME[1]);
    z.fill([-1, 0, -1], [1, 0, 1], PLANKS);
    for (x, zz) in [(-2, -2), (2, -2), (-2, 2), (2, 2)] {
        z.column(x, zz, 1..=12, TIMBER_BEAM);
    }
    for c in -2..=2i32 {
        for (x, zz) in [(c, -2), (c, 2), (-2, c), (2, c)] {
            for y in [4, 8] {
                z.put(x, y, zz, TIMBER_BEAM);
            }
            z.put(x, 12, zz, PLANKS);
        }
    }
    z.fill([-1, 12, -1], [1, 12, 1], PLANKS);
    // the winding wheel and the fire above it
    for (y, zz) in [(13, 0), (15, 0), (14, -1), (14, 1)] {
        z.put(0, y, zz, DARK_PLANKS);
    }
    z.put(0, 14, 0, LOG);
    z.put(0, 13, -1, TIMBER_BEAM);
    z.put(0, 13, 1, TIMBER_BEAM);
    z.put(0, 16, 0, BRAZIER);
    // back stays lean away from the shaft
    for k in 0..=5 {
        for zz in [-2, 2] {
            z.put(3 + k, 11 - 2 * k, zz, TIMBER_BEAM);
            z.put(3 + k, 10 - 2 * k, zz, TIMBER_BEAM);
        }
    }
    z.put(-3, 1, 3, CRATE);
    z.put(3, 1, -3, BARREL);
}

/// A farmstead of Hearthmeadow: a thatched cottage, a plot and a hay stack.
fn farmstead(z: &mut Zone, c: [i32; 2], l: i32, n: usize) {
    z.origin(c[0], l, c[1]);
    hut(
        z,
        &Hut {
            x: [-7, -1],
            z: [-5, -1],
            h: 3,
            wall: PLASTER,
            frame: TIMBER_BEAM,
            roof: THATCH,
            floor: PLANKS,
            door: (2, -4),
            ridge_x: true,
        },
    );
    z.put(-6, 1, -4, BED);
    z.put(-2, 1, -4, CRAFTING_TABLE);
    z.put(-2, 1, -2, BARREL);
    z.put(-5, 1, 0, FLOWER_BED);
    z.put(-3, 1, 0, FLOWER_BED);
    // the chimney
    z.column(0, -3, 1..=7, STONE_BRICKS);
    // a kitchen plot behind a low fence
    for cz in -6..=0i32 {
        for cx in 2..=8 {
            let ring = cx == 2 || cx == 8 || cz == -6 || cz == 0;
            if ring {
                if !(cx == 2 && cz == -3) && (cx + cz).rem_euclid(2i32) == 0 {
                    z.put(cx, 1, cz, TIMBER_BEAM);
                }
            } else {
                z.put(cx, 0, cz, DIRT);
                if cx % 2 == 1 && !rnd(cx + n as i32, cz, 0xfa).is_multiple_of(4) {
                    z.put(cx, 1, cz, if cx == 5 { HAY_BALE } else { FLOWER_BED });
                }
            }
        }
    }
    for (x, y, zz) in [(4, 1, 4), (5, 1, 4), (4, 2, 4), (4, 1, 5)] {
        z.put(x, y, zz, HAY_BALE);
    }
    z.put(-8, 1, 2, BARREL);
    z.put(1, 1, 3, CRATE);
    z.lamp(c[0] - 4, c[1] + 2);
}

/// A sheep field of Hearthmeadow: a log and plank fence that follows the
/// ground, a gate, a shelter and a trough.
fn pasture(z: &mut Zone, x: [i32; 2], zz: [i32; 2], gate: u8) {
    let (mx, mz) = ((x[0] + x[1]) / 2, (zz[0] + zz[1]) / 2);
    for cz in zz[0]..=zz[1] {
        for cx in x[0]..=x[1] {
            if z.locked(cx, cz) || z.flags(cx, cz) != 0 || z.water(cx, cz) > 0 {
                continue;
            }
            let g = z.ground(cx, cz);
            let (ex, ez) = (cx == x[0] || cx == x[1], cz == zz[0] || cz == zz[1]);
            if !ex && !ez {
                // grazed short: no trees inside
                for y in g + 1..g + 12 {
                    if plant(z.block(cx, y, cz)) {
                        z.set(cx, y, cz, AIR);
                    }
                }
                continue;
            }
            let open = match gate {
                0 => cz == zz[0] && (cx - mx).abs() <= 1,
                1 => cx == x[1] && (cz - mz).abs() <= 1,
                2 => cz == zz[1] && (cx - mx).abs() <= 1,
                _ => cx == x[0] && (cz - mz).abs() <= 1,
            };
            if open || z.block(cx, g + 1, cz) != AIR && !plant(z.block(cx, g + 1, cz)) {
                continue;
            }
            for y in g + 1..g + 12 {
                if plant(z.block(cx, y, cz)) {
                    z.set(cx, y, cz, AIR);
                }
            }
            let post = (ex && ez) || (cx - x[0] + cz - zz[0]) % 4 == 0;
            z.set(cx, g + 1, cz, if post { LOG } else { PLANKS });
            if ex && ez {
                z.set(cx, g + 2, cz, LOG);
            }
        }
    }
    // a shelter in one corner, hay and a trough by it
    let (sx, sz) = (x[0] + 2, zz[0] + 2);
    let g = z.ground(sx + 1, sz + 1);
    if (sx..=sx + 3).all(|cx| (sz..=sz + 2).all(|cz| z.flags(cx, cz) == 0 && !z.locked(cx, cz))) {
        for (cx, cz) in [(sx, sz), (sx + 3, sz), (sx, sz + 2), (sx + 3, sz + 2)] {
            for y in z.ground(cx, cz) + 1..=g + 2 {
                z.set(cx, y, cz, TIMBER_BEAM);
            }
        }
        for cz in sz - 1..=sz + 3 {
            for cx in sx - 1..=sx + 4 {
                z.set(cx, g + 3, cz, THATCH);
            }
        }
        z.set(sx + 1, g + 1, sz + 1, HAY_BALE);
        z.set(sx + 2, g + 1, sz + 1, HAY_BALE);
    }
    let (tx, tz) = (x[1] - 3, zz[1] - 3);
    for cx in tx..=tx + 1 {
        let g = z.ground(cx, tz);
        if z.flags(cx, tz) == 0 && !z.locked(cx, tz) && z.block(cx, g + 1, tz) == AIR {
            z.set(cx, g + 1, tz, BARREL);
        }
    }
}
