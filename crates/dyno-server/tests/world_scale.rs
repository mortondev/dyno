//! The world the built-in pack makes, measured against the tables of
//! docs/design/scale.md: doors, storeys, roads, trees, the plaza, how far
//! apart places are in running time, and how often a road shows something.
//!
//! Everything is measured in the generated world (blocks and the roads as
//! built), not read back from the layout. Sizes of things hold at any world
//! size. Distances between places are written for the layout's design size
//! (2048 m); a smaller world draws them in by `spread`, and the running
//! times are checked against the targets drawn in by the same amount. The
//! ignored test builds the design-size world and checks the real targets:
//!
//! ```sh
//! cargo test --release -p dyno-server --test world_scale -- --ignored --nocapture
//! ```

use dyno_server::pack::Pack;
use dyno_world::layout::Layout;
use dyno_world::stamp::Stamps;
use dyno_world::worldgen::{self, Built, ROAD, SITE, WET, measure};
use dyno_world::{self as w, World};
use std::collections::{BinaryHeap, HashMap};
use std::sync::OnceLock;

/// Running speed, m/s (docs/design/scale.md).
const RUN: f32 = 4.4;

struct Made {
    layout: Layout,
    stamps: Stamps,
    world: World,
    built: Built,
}

fn make(design: bool) -> Made {
    let pack = Pack::builtin();
    let mut layout = (**pack.layout.as_ref().expect("the pack has a layout")).clone();
    if design {
        layout.size = layout.design;
    }
    let stamps = (*pack.stamps).clone();
    let (world, built) = worldgen::build(&layout, &stamps, layout.size).expect("the world builds");
    Made {
        layout,
        stamps,
        world,
        built,
    }
}

fn made() -> &'static Made {
    static M: OnceLock<Made> = OnceLock::new();
    M.get_or_init(|| make(false))
}

#[test]
fn the_pack_has_a_valid_layout_and_stamps() {
    let pack = Pack::builtin();
    let layout = pack.layout.as_ref().expect("world/layout.json");
    layout.check_stamps(&pack.stamps).unwrap();
    assert!(
        pack.stamps.0.len() >= 40,
        "the buildings and props are stamps"
    );
    // the world's data stays on the server
    assert!(
        !pack
            .manifest
            .files
            .iter()
            .any(|f| f.path.starts_with("stamps/") || f.path.starts_with("world/")),
        "clients are sent the world, not what it is built from"
    );
    // every stamp of a building keeps the heroic proportions
    for (name, s) in &pack.stamps.0 {
        if s.kind == "building" {
            for d in &s.doors {
                assert!(
                    (2.8..=3.2).contains(&(d.height as f32)),
                    "{name}: a door {} m high",
                    d.height
                );
            }
            for pair in s.storeys.windows(2) {
                let h = (pair[1] - pair[0]) as f32;
                assert!((4.0..=4.5).contains(&h), "{name}: a storey of {h} m");
            }
        }
        assert!(!s.census().is_empty(), "{name} is empty");
    }
    // every place is used or stands for something
    for p in &layout.places {
        assert!(
            !p.stamps.is_empty()
                || !p.trees.is_empty()
                || matches!(p.kind.as_str(), "zone" | "camp"),
            "place {} has nothing in it",
            p.key
        );
    }
    // the rules stand on the layout: nothing in them is left unresolved
    for a in &pack.content.areas {
        assert!(a.radius > 0.0, "{}", a.key);
    }
    let hub = &pack.content.areas[0];
    assert_eq!(hub.at, [0, 0], "the first area is the hub, at the hearth");
}

#[test]
fn doors_and_storeys_are_heroic() {
    let m = made();
    let (mut doors, mut floors) = (0, 0);
    for (place, name, at, rot) in &m.built.stamps {
        let s = m.stamps.get(name).unwrap();
        if s.kind != "building" {
            continue;
        }
        for d in &s.doors {
            let p = s.to_world(*at, *rot, d.at);
            let h = measure::door_height(&m.world, p);
            assert!(
                (2.8..=3.2).contains(&(h as f32)),
                "{place}/{name}: the door at {p:?} is {h} m high (block {} there)",
                m.world.get(p[0], p[1], p[2])
            );
            doors += 1;
        }
        let Some(probe) = s.points.get("floor") else {
            continue;
        };
        let mut levels = Vec::new();
        for st in &s.storeys {
            let p = s.to_world(*at, *rot, [probe[0], *st, probe[2]]);
            assert!(
                w::is_solid(m.world.get(p[0], p[1] - 1, p[2]))
                    && !w::is_solid(m.world.get(p[0], p[1], p[2])),
                "{place}/{name}: no floor to stand on at {p:?}"
            );
            levels.push(p[1]);
        }
        for pair in levels.windows(2) {
            let h = (pair[1] - pair[0]) as f32;
            assert!(
                (4.0..=4.5).contains(&h),
                "{place}/{name}: a storey of {h} m"
            );
            floors += 1;
        }
    }
    assert!(doors >= 15, "only {doors} doors measured");
    assert!(floors >= 10, "only {floors} storeys measured");
}

#[test]
fn roads_are_wide_and_trails_narrow() {
    let m = made();
    let size = m.world.size;
    let mut seen = HashMap::new();
    for road in &m.built.roads {
        let range = if road.way == "road" { 6..=8 } else { 2..=3 };
        let (mut ok, mut n) = (0, 0);
        for i in (5..road.line.len().saturating_sub(5)).step_by(7) {
            let p = road.line[i];
            // open country only: not where it runs into a square, a yard
            // or the water, or meets another road
            let near = (-6..=6).any(|dz| {
                (-6..=6).any(|dx| {
                    let (x, z) = (p[0] + dx, p[2] + dz);
                    m.built.flags[(z * size[0] as i32 + x) as usize] & (SITE | WET) != 0
                })
            });
            let crossing = m.built.roads.iter().any(|o| {
                o.key != road.key
                    && o.line
                        .iter()
                        .any(|q| (q[0] - p[0]).abs() <= 8 && (q[2] - p[2]).abs() <= 8)
            });
            if near || crossing {
                continue;
            }
            let wide = measure::road_width(&m.built, size, road, i);
            n += 1;
            if range.contains(&wide) {
                ok += 1;
            }
            assert!(
                (range.start() - 1..=range.end() + 1).contains(&wide),
                "{} is {wide} m wide at {p:?}",
                road.key
            );
        }
        if n < 4 {
            // too short a stretch of open country to judge
            continue;
        }
        assert!(
            ok * 100 >= n * 80,
            "{}: {ok} of {n} widths are within {range:?} m",
            road.key
        );
        *seen.entry(road.way.as_str()).or_insert(0) += n;
    }
    assert!(seen["road"] >= 30 && seen["trail"] >= 30, "{seen:?}");
}

/// Species, then height, trunk and crown width as ranges in metres.
type Sizes = (&'static [&'static str], [i32; 2], [i32; 2], [i32; 2]);

/// Is there another tree within `r` metres?
fn alone(m: &Made, at: [i32; 3], r: i32) -> bool {
    !m.built
        .trees
        .iter()
        .any(|(_, o)| *o != at && (o[0] - at[0]).abs() <= r && (o[2] - at[2]).abs() <= r)
}

#[test]
fn trees_are_the_sizes_of_the_scale_table() {
    let m = made();
    // docs/design/scale.md: height, trunk, crown width (metres)
    let table: [Sizes; 3] = [
        (
            &["oak", "birch", "willow", "dark_oak"],
            [11, 16],
            [1, 2],
            [8, 12],
        ),
        (&["hero_oak"], [18, 27], [2, 3], [14, 18]),
        (&["pine"], [15, 21], [1, 2], [0, 12]),
    ];
    // every size rule is data, and the data says what the table says
    for (names, height, girth, crown) in table {
        for name in names {
            let sp = &m.layout.flora.species[*name];
            assert!(
                sp.height[0] >= height[0] && sp.height[1] <= height[1],
                "{name}: heights {:?}",
                sp.height
            );
            assert!(
                sp.girth[0] >= girth[0] && sp.girth[1] <= girth[1],
                "{name}: trunks {:?}",
                sp.girth
            );
            assert!(
                sp.crown.width[0] >= crown[0] && sp.crown.width[1] <= crown[1],
                "{name}: crowns {:?}",
                sp.crown.width
            );
        }
    }
    // and the trees standing in the world are those sizes
    let mut counted: HashMap<&str, usize> = HashMap::new();
    for (species, at) in &m.built.trees {
        let Some((_, height, girth, crown)) =
            table.iter().find(|t| t.0.contains(&species.as_str()))
        else {
            continue;
        };
        if !alone(m, *at, 20) {
            continue;
        }
        let wood = |b| matches!(b, w::LOG | w::BIRCH_LOG);
        let tree = |b| wood(b) || w::is_leaves(b);
        // the top: the highest block of the tree over its foot
        let top = (1..40)
            .rev()
            .find(|dy| {
                (-1..=2).any(|dz| {
                    (-1..=2).any(|dx| tree(m.world.get(at[0] + dx, at[1] + dy, at[2] + dz)))
                })
            })
            .unwrap_or(0);
        assert!(
            (height[0]..=height[1]).contains(&top),
            "{species} at {at:?} is {top} m tall"
        );
        // the trunk at chest height
        let thick = (0..4)
            .take_while(|dx| wood(m.world.get(at[0] + dx, at[1] + 3, at[2])))
            .count() as i32;
        assert!(
            (girth[0]..=girth[1]).contains(&thick),
            "{species} at {at:?} has a trunk {thick} m thick"
        );
        // the crown at its widest, west to east
        // (above the bushes and hedges)
        let wide = (4..=top)
            .map(|dy| {
                // (no further out than a crown of this kind can reach: the
                // next tree's leaves are not this one's)
                let r = crown[1] / 2 + 1;
                let xs: Vec<i32> = (-r..=r + 2)
                    .filter(|dx| {
                        (-r..=r + 2)
                            .any(|dz| w::is_leaves(m.world.get(at[0] + dx, at[1] + dy, at[2] + dz)))
                    })
                    .collect();
                xs.last().zip(xs.first()).map_or(0, |(a, b)| a - b + 1)
            })
            .max()
            .unwrap_or(0);
        if species != "dark_oak" {
            assert!(
                wide >= crown[0].max(6) - 2 && wide <= crown[1],
                "{species} at {at:?} has a crown {wide} m wide"
            );
        }
        *counted.entry(species.as_str()).or_default() += 1;
    }
    assert!(
        counted.get("oak").copied().unwrap_or(0) >= 10,
        "{counted:?}"
    );
    assert!(
        counted.get("hero_oak").copied().unwrap_or(0) >= 3,
        "{counted:?}"
    );
    assert!(
        counted.get("pine").copied().unwrap_or(0) >= 5,
        "{counted:?}"
    );
}

#[test]
fn woods_stand_in_clumps_with_open_meadow_between() {
    let m = made();
    // trees per hectare over the whole land, in cells of 32 m: woods are
    // dense where they are woods, and much of the land has no tree at all
    let cell = 32;
    let (cx, cz) = (m.world.size[0] as i32 / cell, m.world.size[2] as i32 / cell);
    let mut count = vec![0u32; (cx * cz) as usize];
    for (_, at) in &m.built.trees {
        count[((at[2] / cell).min(cz - 1) * cx + (at[0] / cell).min(cx - 1)) as usize] += 1;
    }
    let land: Vec<u32> = (0..cz)
        .flat_map(|z| (0..cx).map(move |x| (x, z)))
        .filter(|(x, z)| {
            // not the mountains of the rim
            let edge = 5;
            *x >= edge && *z >= edge && *x < cx - edge && *z < cz - edge
        })
        .map(|(x, z)| count[(z * cx + x) as usize])
        .collect();
    let open = land.iter().filter(|n| **n == 0).count() as f32 / land.len() as f32;
    let dense = land.iter().filter(|n| **n >= 6).count() as f32 / land.len() as f32;
    assert!(
        open > 0.12,
        "only {:.0} % of the land is open",
        open * 100.0
    );
    assert!(
        dense > 0.08,
        "only {:.0} % of the land is wood",
        dense * 100.0
    );
}

#[test]
fn the_plaza_is_a_heroic_square() {
    let m = made();
    let h = m.built.hearth;
    let sx = m.world.size[0] as i32;
    let paved = |x: i32, z: i32| m.built.flags[(z * sx + x) as usize] & ROAD != 0;
    // east to west through the hearth, between the roads that leave it
    let across = |dz: i32| {
        (1..60).take_while(|d| paved(h[0] + d, h[2] + dz)).count()
            + (1..60).take_while(|d| paved(h[0] - d, h[2] + dz)).count()
            + 1
    };
    let wide = across(6).max(across(-6));
    assert!((25..=45).contains(&wide), "the plaza is {wide} m across");
}

/// The y a runner's feet are at in a column, near `y`.
fn footing(world: &World, x: i32, y: i32, z: i32) -> Option<i32> {
    // the nearest standing height to `y`: under an arch, not on top of it
    (0..=8).flat_map(|d| [y - d, y + d]).find(|yy| {
        let below = world.get(x, yy - 1, z);
        (w::is_solid(below) || below == w::WATER)
            && !w::is_solid(world.get(x, *yy, z))
            && !w::is_solid(world.get(x, yy + 1, z))
    })
}

/// The road network as built: seconds of running between places.
struct Network {
    nodes: Vec<[i32; 3]>,
    edges: Vec<Vec<(usize, f32)>>,
}

impl Network {
    fn new(m: &Made) -> Network {
        let mut nodes = Vec::new();
        let mut edges: Vec<Vec<(usize, f32)>> = Vec::new();
        let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for road in &m.built.roads {
            let mut last: Option<usize> = None;
            for p in &road.line {
                let i = nodes.len();
                nodes.push(*p);
                edges.push(Vec::new());
                if let Some(l) = last {
                    let d = ((nodes[l][0] - p[0]) as f32).hypot((nodes[l][2] - p[2]) as f32);
                    edges[l].push((i, d));
                    edges[i].push((l, d));
                }
                // roads that meet are joined
                for dz in -1..=1 {
                    for dx in -1..=1 {
                        for &j in grid
                            .get(&(p[0] / 4 + dx, p[2] / 4 + dz))
                            .into_iter()
                            .flatten()
                        {
                            let d =
                                ((nodes[j][0] - p[0]) as f32).hypot((nodes[j][2] - p[2]) as f32);
                            if d <= 4.0 && last != Some(j) {
                                edges[j].push((i, d));
                                edges[i].push((j, d));
                            }
                        }
                    }
                }
                grid.entry((p[0] / 4, p[2] / 4)).or_default().push(i);
                last = Some(i);
            }
        }
        Network { nodes, edges }
    }

    /// The node nearest a spot, and how far off the road the spot is.
    fn nearest(&self, at: [i32; 3]) -> (usize, f32) {
        self.nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (i, ((n[0] - at[0]) as f32).hypot((n[2] - at[2]) as f32)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap()
    }

    /// Metres by road from one spot to another (with the walk to and from the road).
    fn metres(&self, a: [i32; 3], b: [i32; 3]) -> f32 {
        let ((from, d0), (to, d1)) = (self.nearest(a), self.nearest(b));
        let mut best = vec![f32::MAX; self.nodes.len()];
        let mut heap = BinaryHeap::new();
        best[from] = 0.0;
        heap.push((std::cmp::Reverse(0u32), from));
        while let Some((std::cmp::Reverse(d), i)) = heap.pop() {
            let d = d as f32 / 100.0;
            if d > best[i] + 0.01 {
                continue;
            }
            if i == to {
                break;
            }
            for &(j, len) in &self.edges[i] {
                if d + len < best[j] {
                    best[j] = d + len;
                    heap.push((std::cmp::Reverse(((d + len) * 100.0) as u32), j));
                }
            }
        }
        assert!(best[to] < f32::MAX, "no road from {a:?} to {b:?}");
        best[to] + d0 + d1
    }
}

fn place(m: &Made, key: &str) -> [i32; 3] {
    *m.built
        .places
        .get(key)
        .unwrap_or_else(|| panic!("place {key} is not in the world"))
}

/// The trips of docs/design/scale.md, as (from, to, seconds at design size).
const NEAR: [(&str, &str); 4] = [
    ("hearthstead", "north_watch"),
    ("hearthstead", "wrecked_caravan"),
    ("fishers_landing", "shrine_shore"),
    ("miners_rest", "hollow_mines"),
];
const HUBS: [(&str, &str); 2] = [
    ("hearthstead", "miners_rest"),
    ("hearthstead", "fishers_landing"),
];

fn running_times(m: &Made) -> Vec<(String, f32, [f32; 2])> {
    let net = Network::new(m);
    let s = m.layout.spread();
    let mut out = Vec::new();
    for (a, b) in NEAR {
        let t = net.metres(place(m, a), place(m, b)) / RUN;
        out.push((format!("{a} to {b}"), t, [30.0 * s, 60.0 * s]));
    }
    for (a, b) in HUBS {
        let t = net.metres(place(m, a), place(m, b)) / RUN;
        out.push((format!("{a} to {b}"), t, [120.0 * s, 180.0 * s]));
    }
    // across the zone: the longest trip by road between two named places
    let named: Vec<&str> = m
        .layout
        .places
        .iter()
        .filter(|p| !p.name.is_empty() && p.kind != "zone" && m.built.places.contains_key(&p.key))
        .map(|p| p.key.as_str())
        .collect();
    let mut far = (String::new(), 0.0f32);
    for a in [
        "north_pass",
        "old_barrow",
        "hollow_mines",
        "shrine_shore",
        "fishers_landing",
    ] {
        for b in &named {
            let t = net.metres(place(m, a), place(m, b)) / RUN;
            if t > far.1 {
                far = (format!("{a} to {b}"), t);
            }
        }
    }
    out.push((
        format!("across the zone ({})", far.0),
        far.1,
        [360.0 * s, 600.0 * s],
    ));
    out
}

#[test]
fn places_are_minutes_of_running_apart() {
    let m = made();
    for (trip, t, [lo, hi]) in running_times(m) {
        assert!(
            (lo..=hi).contains(&t),
            "{trip}: {t:.0} s of running, wanted {lo:.0} to {hi:.0} s (spread {})",
            m.layout.spread()
        );
    }
}

#[test]
fn roads_can_be_run() {
    let m = made();
    for road in &m.built.roads {
        let mut last: Option<i32> = None;
        for p in &road.line {
            let y = footing(&m.world, p[0], last.unwrap_or(p[1] + 1), p[2])
                .unwrap_or_else(|| panic!("{}: nowhere to stand at {p:?}", road.key));
            if let Some(l) = last {
                assert!(
                    (y - l).abs() <= 1,
                    "{}: a step of {} m at {p:?} (column {:?})",
                    road.key,
                    y - l,
                    road.line
                        .iter()
                        .skip_while(|q| *q != p)
                        .take(1)
                        .chain(
                            road.line
                                .iter()
                                .filter(|q| (q[0] - p[0]).abs() + (q[2] - p[2]).abs() < 7)
                        )
                        .map(|q| (
                            q[0],
                            q[1],
                            q[2],
                            m.built.flags[(q[2] * m.world.size[0] as i32 + q[0]) as usize]
                        ))
                        .collect::<Vec<_>>()
                );
            }
            last = Some(y);
        }
    }
}

/// The longest stretch of each road with nothing to see, in metres.
fn longest_gaps(m: &Made) -> Vec<(String, f32)> {
    let s = m.layout.spread();
    let reach = 40.0 * s + 12.0;
    // a place is in sight from the road within that reach, or while the
    // road runs through its own ground
    let sights: Vec<([i32; 3], f32)> = m
        .layout
        .places
        .iter()
        .filter(|p| p.sight)
        .filter_map(|p| {
            let own = p
                .pad
                .as_ref()
                .map_or(0.0, |pad| pad.radius[0].max(pad.radius[1]) + 12.0);
            Some((*m.built.places.get(&p.key)?, reach.max(own)))
        })
        .collect();
    m.built
        .roads
        .iter()
        .map(|road| {
            let (mut gap, mut worst, mut at) = (0.0f32, 0.0f32, [0; 3]);
            for pair in road.line.windows(2) {
                let p = pair[1];
                gap += ((p[0] - pair[0][0]) as f32).hypot((p[2] - pair[0][2]) as f32);
                if sights
                    .iter()
                    .any(|(q, r)| ((q[0] - p[0]) as f32).hypot((q[2] - p[2]) as f32) <= *r)
                {
                    gap = 0.0;
                }
                if gap > worst {
                    (worst, at) = (gap, p);
                }
            }
            (format!("{} (worst near {at:?})", road.key), worst)
        })
        .collect()
}

#[test]
fn there_is_something_every_150_m_of_road() {
    let m = made();
    let s = m.layout.spread();
    for (road, gap) in longest_gaps(m) {
        assert!(
            gap <= 150.0 * s + 1.0,
            "{road}: {gap:.0} m with nothing to see (at most {:.0} m at this size)",
            150.0 * s
        );
    }
}

#[test]
fn the_land_has_hills_and_a_wall_of_mountains() {
    let m = made();
    let ground = m.layout.terrain.ground;
    let v = m.layout.rise();
    let size = m.world.size;
    let top = |x: i32, z: i32| m.world.ground(x, z) as f32;
    // the rim: the highest ground along the edges
    let mut rim = 0.0f32;
    for i in (8..size[0] as i32 - 8).step_by(4) {
        for e in [10, size[2] as i32 - 10] {
            rim = rim.max(top(i, e)).max(top(e, i));
        }
    }
    assert!(
        rim - ground >= 90.0 * v && rim - ground <= 180.0,
        "the rim rises {:.0} m (rise factor {v:.2})",
        rim - ground
    );
    // rolling country everywhere: in every 250 m square of the zone the
    // land rises and falls, by 15 m or more in most of them, so a view has
    // a foreground, a valley and far hills
    let (mut squares, mut rolling) = (0, 0);
    let edge = (m.layout.terrain.rim.map_or(0.0, |r| r.width) * m.layout.spread()) as i32 + 60;
    let step = 250;
    let mut z0 = edge;
    while z0 + step <= size[2] as i32 - edge {
        let mut x0 = edge;
        while x0 + step <= size[0] as i32 - edge {
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            for z in (z0..z0 + step).step_by(5) {
                for x in (x0..x0 + step).step_by(5) {
                    if m.built.flags[(z * size[0] as i32 + x) as usize] & WET == 0 {
                        lo = lo.min(top(x, z));
                        hi = hi.max(top(x, z));
                    }
                }
            }
            squares += 1;
            assert!(
                hi - lo >= 8.0 && hi - lo <= 70.0,
                "the land round [{x0}, {z0}] rises {:.0} m",
                hi - lo
            );
            if hi - lo >= 15.0 {
                rolling += 1;
            }
            x0 += step;
        }
        z0 += step;
    }
    assert!(
        squares == 0 || rolling * 10 >= squares * 7,
        "{rolling} of {squares} squares of country roll by 15 m or more"
    );
}

/// The world at its design size (2048 × 256 × 2048), with the real targets
/// of docs/design/scale.md, and what it costs to make. It holds a gigabyte
/// of blocks in memory until the sparse server world lands.
#[test]
#[ignore]
fn design_size_world_meets_the_running_times() {
    let t = std::time::Instant::now();
    let m = make(true);
    println!(
        "built {:?} in {:.1} s: {} places, {} stamps, {} trees",
        m.world.size,
        t.elapsed().as_secs_f32(),
        m.built.places.len(),
        m.built.stamps.len(),
        m.built.trees.len()
    );
    assert_eq!(m.layout.spread(), 1.0);
    let mut bad = Vec::new();
    for (trip, t, [lo, hi]) in running_times(&m) {
        println!(
            "{trip}: {t:.0} s ({:.1} min), target {lo:.0}-{hi:.0} s",
            t / 60.0
        );
        if !(lo..=hi).contains(&t) {
            bad.push(trip);
        }
    }
    for (road, gap) in longest_gaps(&m) {
        println!("{road}: longest stretch with nothing to see {gap:.0} m");
        if gap > 151.0 {
            bad.push(road);
        }
    }
    let size = m.world.size;
    let mut high = 0;
    for z in (0..size[2] as i32).step_by(8) {
        for x in (0..size[0] as i32).step_by(8) {
            high = high.max(m.world.ground(x, z));
        }
    }
    println!(
        "highest ground {high} of {} (the rim rises {} m over the meadow)",
        size[1],
        high - m.layout.terrain.ground as i32
    );
    assert!(high < size[1] as i32 - 30, "the mountains need their sky");
    assert!(bad.is_empty(), "outside the targets: {bad:?}");
}

/// Can someone on foot get from one cell to (near) another? Steps of a
/// metre up, drops of three, wading and swimming allowed.
fn walkable(world: &World, from: [i32; 3], to: [i32; 3]) -> bool {
    let open = |p: [i32; 3]| !w::is_solid(world.get(p[0], p[1], p[2]));
    let stand = |p: [i32; 3]| {
        let below = world.get(p[0], p[1] - 1, p[2]);
        open(p)
            && open([p[0], p[1] + 1, p[2]])
            && (w::is_solid(below) || below == w::WATER || world.get(p[0], p[1], p[2]) == w::WATER)
    };
    // start on the floor at or just under the point
    let Some(start) = (0..4)
        .flat_map(|d| {
            [
                [from[0], from[1] - d, from[2]],
                [from[0], from[1] + d, from[2]],
            ]
        })
        .find(|p| stand(*p))
    else {
        return false;
    };
    let mut seen = std::collections::HashSet::from([start]);
    let mut queue = std::collections::VecDeque::from([start]);
    while let Some(p) = queue.pop_front() {
        if (p[0] - to[0]).abs() <= 3 && (p[2] - to[2]).abs() <= 3 && (p[1] - to[1]).abs() <= 3 {
            return true;
        }
        if seen.len() > 400_000 {
            return false;
        }
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            for dy in [1, 0, -1, -2, -3] {
                let q = [p[0] + dx, p[1] + dy, p[2] + dz];
                // stay with the place: a route through it, not round the country
                if (q[0] - from[0]).abs() > 160 || (q[2] - from[2]).abs() > 160 {
                    continue;
                }
                if stand(q) && seen.insert(q) {
                    queue.push_back(q);
                    break;
                }
            }
        }
    }
    false
}

/// The rules land on the world: every camp has ground for its creatures,
/// every NPC and objective a place to stand, and the dug-out places can be
/// walked from their doors to their depths.
#[test]
fn camps_people_and_objectives_stand_in_the_world() {
    use dyno_world::content::Objective;
    let m = made();
    let pack = Pack::builtin();
    let c = &pack.content;
    let h = m.built.hearth;
    // the server's own search: feet on ground, never on a tree's crown
    let free = |x: i32, y: i32, z: i32| {
        w::is_solid(m.world.get(x, y - 1, z))
            && !w::is_leaves(m.world.get(x, y - 1, z))
            && !w::is_solid(m.world.get(x, y, z))
            && !w::is_solid(m.world.get(x, y + 1, z))
    };
    // camps: as the server plans them
    let points = dyno_server::camps::plan_spawn_points(&m.world, c, h);
    for (ai, a) in c.areas.iter().enumerate() {
        for (ci, camp) in a.camps.iter().enumerate() {
            let n = points
                .iter()
                .filter(|p| p.area == ai && p.camp == ci)
                .count();
            assert!(
                n * 2 >= camp.count as usize && n >= 1,
                "{}: camp {ci} has ground for {n} of its {} creatures (at {:?}, y {:?})",
                a.key,
                camp.count,
                camp.at,
                camp.y
            );
        }
        // an area is somewhere in the world
        assert!(
            m.world.in_bounds([h[0] + a.at[0], 1, h[2] + a.at[1]]),
            "{} is outside the world",
            a.key
        );
    }
    // people: where the server will put them (the same search it makes)
    for n in &c.npcs {
        let (x, z) = (h[0] + n.at[0], h[2] + n.at[2]);
        let y0 = h[1] + n.at[1];
        let y = (0..12)
            .flat_map(|d| [y0 + d, y0 - d])
            .find(|y| free(x, *y, z));
        assert!(
            m.world.in_bounds([x, 1, z]) && y.is_some(),
            "{} has nowhere to stand at {:?}",
            n.key,
            [x, y0, z]
        );
        // and not shut in: on the ground floor of the world, not on a roof
        let y = y.unwrap();
        assert!(
            y - m.world.ground(x, z) <= 1 || !w::is_solid(m.world.get(x, y + 2, z)),
            "{} is boxed in at {:?}",
            n.key,
            [x, y, z]
        );
    }
    // explore objectives: a spot to reach, on the surface
    for q in &c.quests {
        for o in &q.objectives {
            if let Objective::Explore { at, radius, name } = o {
                let (x, z) = (h[0] + at[0], h[2] + at[2]);
                assert!(m.world.in_bounds([x, 1, z]), "{name} is outside the world");
                let r = *radius as i32;
                let reachable = (-r..=r).any(|dz| {
                    (-r..=r).any(|dx| {
                        let y = m.world.surface(x + dx, z + dz) + 1;
                        free(x + dx, y, z + dz) && (y - (h[1] + at[1])).abs() <= r + 6
                    })
                });
                assert!(
                    reachable,
                    "{name}: nowhere to stand within {radius} m of {:?}",
                    [x, z]
                );
            }
        }
    }
    // the dug-out places: from the door to the depths on foot
    for (site, name, at, rot) in &m.built.stamps {
        let s = m.stamps.get(name).unwrap();
        for (from, to) in [
            ("adit", "upper_galleries"),
            ("adit", "flooded_drift"),
            ("adit", "deep_seam"),
            ("door", "crypt"),
        ] {
            let (Some(a), Some(b)) = (s.point(from, *at, *rot), s.point(to, *at, *rot)) else {
                continue;
            };
            assert!(
                walkable(&m.world, a, b),
                "{site}/{name}: no way on foot from {from} {a:?} to {to} {b:?}"
            );
            // and from the open air to the door
            // (the barrow's own place is the middle of its mound: start
            // on the grass east of it)
            let mut out = place(m, site);
            if from == "door" {
                out[0] += 24;
            }
            assert!(
                walkable(&m.world, out, a),
                "{site}/{name}: no way in from {out:?} to {from} {a:?}"
            );
        }
    }
}

/// The one-time rebuild: a save from the world before layouts is moved
/// onto the built world. Everyone stays at their place, with everything
/// they had.
#[test]
fn a_rebuild_moves_everyone_to_their_place_and_keeps_what_they_have() {
    use dyno_server::{migrate, store};
    use serde_json::{Value, json};
    let m = made();
    let dir = std::env::temp_dir().join(format!("dyno-rebuild-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("world.dynw");
    // the old world: 384 wide, its hearth off-centre as the live one's is
    let old = World::test_terrain([384, 96, 384], 7);
    old.save(&path).unwrap();
    let hearth = [195, old.surface(195, 193) + 1, 193];
    let at = |dx: f64, dz: f64| {
        let (x, z) = (hearth[0] as f64 + dx, hearth[2] as f64 + dz);
        json!([
            x + 0.5,
            old.surface(x as i32, z as i32) as f64 + 1.0,
            z + 0.5
        ])
    };
    let kit = json!({"token": 77, "level": 9, "xp": 1234, "money": 4321, "class": "priest",
        "pack": [["iron_sword", 1, 0], null, ["berries", 5, 0]], "equip": [null, ["worn_tunic", 1, 3]],
        "quests": [["cold_hearth", [0]]], "done": ["meat_for_the_pot"], "bank": [["coal", 12, 0]],
        "something_new": {"kept": true}});
    let mut players = serde_json::Map::new();
    let mut put = |name: &str, pos: Value, spawn: Value| {
        let mut p = kit.clone();
        p["pos"] = pos;
        p["spawn"] = spawn;
        players.insert(name.into(), p);
    };
    put("in_town", at(0.0, 10.0), Value::Null);
    put("at_the_watch", at(2.0, -52.0), at(0.0, 5.0));
    put("at_the_rest", at(131.0, 8.0), Value::Null);
    // deep in the mine: far below the surface, north of the yard
    put(
        "in_the_mine",
        json!([hearth[0] as f64 + 174.5, 6.0, hearth[2] as f64 - 105.5]),
        Value::Null,
    );
    put("in_the_wild", at(-60.0, 100.0), Value::Null);
    std::fs::write(
        dir.join("players.json"),
        serde_json::to_vec(&players).unwrap(),
    )
    .unwrap();
    std::fs::write(dir.join("chests.json"), b"{}").unwrap();
    let meta = store::Meta {
        day: 12,
        clock: 0.4,
        spawn_hearth: Some(hearth),
        regrow: vec![([200, 30, 200], 5, 99)],
        ..Default::default()
    };
    store::save_meta(&store::meta_path(&path), &meta).unwrap();

    let done = migrate::rebuild(&path, &old, &meta, &m.world, &m.built, &m.layout).unwrap();
    assert_eq!(done.players, 5);
    assert_eq!(done.regrow_dropped, 1);

    let after: Value =
        serde_json::from_slice(&std::fs::read(dir.join("players.json")).unwrap()).unwrap();
    let pos = |name: &str, field: &str| -> [f32; 3] {
        let a = after[name][field].as_array().unwrap();
        [0, 1, 2].map(|i| a[i].as_f64().unwrap() as f32)
    };
    let near = |p: [f32; 3], key: &str| {
        let q = place(m, key);
        (p[0] - q[0] as f32).hypot(p[2] - q[2] as f32)
    };
    assert!(near(pos("in_town", "pos"), "hearthstead") < 40.0);
    assert!(near(pos("at_the_watch", "pos"), "north_watch") < 20.0);
    assert!(near(pos("at_the_watch", "spawn"), "hearthstead") < 40.0);
    assert!(near(pos("at_the_rest", "pos"), "miners_rest") < 25.0);
    assert!(near(pos("in_the_mine", "pos"), "hollow_mines") < 25.0);
    // out in the country: the same direction, further by the map's growth
    let wild = pos("in_the_wild", "pos");
    let k = m.world.size[0] as f32 / 384.0;
    let want = [
        m.built.hearth[0] as f32 - 60.0 * k,
        m.built.hearth[2] as f32 + 100.0 * k,
    ];
    assert!(
        (wild[0] - want[0]).hypot(wild[2] - want[1]) < 50.0,
        "{wild:?} {want:?}"
    );
    for (name, before) in &players {
        // on open ground, under the sky
        let p = pos(name, "pos");
        let (x, y, z) = (
            p[0].floor() as i32,
            p[1].floor() as i32,
            p[2].floor() as i32,
        );
        assert!(
            w::is_solid(m.world.get(x, y - 1, z)),
            "{name} stands on something"
        );
        assert!(!w::is_solid(m.world.get(x, y, z)) && !w::is_solid(m.world.get(x, y + 1, z)));
        assert_eq!(m.world.surface(x, z), y - 1, "{name} is under the sky");
        // nothing but the position changed
        let mut a = after[name].clone();
        let mut b = before.clone();
        for v in [&mut a, &mut b] {
            v.as_object_mut().unwrap().remove("pos");
            v.as_object_mut().unwrap().remove("spawn");
        }
        assert_eq!(a, b, "{name} keeps everything");
    }
    let saved = store::load_meta(&store::meta_path(&path));
    assert_eq!(saved.spawn_hearth, Some(m.built.hearth));
    assert_eq!(saved.world_layout, Some(m.layout.version));
    assert_eq!((saved.day, saved.clock), (12, 0.4), "the calendar is kept");
    assert!(saved.regrow.is_empty());
    // the old save is kept, and a second run never overwrites it
    let backup = dir.join(format!("players.json.before-layout-{}", m.layout.version));
    let kept = std::fs::read(&backup).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&kept).unwrap(),
        Value::Object(players.clone())
    );
    assert!(
        dir.join(format!("world.dynw.before-layout-{}", m.layout.version))
            .exists()
    );
    migrate::rebuild(&path, &old, &meta, &m.world, &m.built, &m.layout).unwrap();
    assert_eq!(std::fs::read(&backup).unwrap(), kept);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A world already built from a layout is rebuilt for a new version of it:
/// nobody is moved to a "place", everyone stays where they stand.
#[test]
fn a_rebuild_of_a_layout_world_leaves_everyone_where_they_stand() {
    use dyno_server::{migrate, store};
    use serde_json::{Value, json};
    let m = made();
    let dir = std::env::temp_dir().join(format!("dyno-rebuild2-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("world.dynw");
    let h = m.built.hearth;
    // in town, south of the hearth; and deep in the mine
    let town = [h[0] as f64 + 0.5, h[1] as f64 + 0.01, h[2] as f64 + 22.5];
    let (_, name, at, rot) = m
        .built
        .stamps
        .iter()
        .find(|s| s.1 == "hollow_mines")
        .unwrap();
    let seam = m
        .stamps
        .get(name)
        .unwrap()
        .point("deep_seam", *at, *rot)
        .unwrap();
    let deep = (0..4)
        .map(|d| [seam[0], seam[1] - d, seam[2]])
        .find(|p| {
            w::is_solid(m.world.get(p[0], p[1] - 1, p[2]))
                && !w::is_solid(m.world.get(p[0], p[1], p[2]))
        })
        .expect("the deep seam has a floor");
    let mine = [
        deep[0] as f64 + 0.5,
        deep[1] as f64 + 0.01,
        deep[2] as f64 + 0.5,
    ];
    let players = json!({
        "in_town": {"pos": town, "spawn": null, "level": 4},
        "in_the_mine": {"pos": mine, "spawn": null, "level": 9},
    });
    std::fs::write(
        dir.join("players.json"),
        serde_json::to_vec(&players).unwrap(),
    )
    .unwrap();
    let meta = store::Meta {
        spawn_hearth: Some(h),
        world_layout: Some(m.layout.version.saturating_sub(1)),
        ..Default::default()
    };
    store::save_meta(&store::meta_path(&path), &meta).unwrap();
    m.world.save(&path).unwrap();
    migrate::rebuild(&path, &m.world, &meta, &m.world, &m.built, &m.layout).unwrap();
    let after: Value =
        serde_json::from_slice(&std::fs::read(dir.join("players.json")).unwrap()).unwrap();
    for (name, want) in [("in_town", town), ("in_the_mine", mine)] {
        let a = after[name]["pos"].as_array().unwrap();
        for i in 0..3 {
            assert!(
                (a[i].as_f64().unwrap() - want[i]).abs() < 0.05,
                "{name} moved: {a:?} from {want:?}"
            );
        }
    }
    assert_eq!(after["in_the_mine"]["level"], json!(9));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Home and crafting are reached on foot, not only stood beside: from the
/// plaza a player can walk to the innkeeper (talking makes the inn home)
/// and to a block of every crafting station.
#[test]
fn the_innkeeper_and_the_stations_can_be_walked_to() {
    let m = made();
    let pack = Pack::builtin();
    let c = &pack.content;
    let h = m.built.hearth;
    // (a free block at the middle of the town)
    let plaza = place(m, "hearthstead");
    // the server's own search: feet on ground, never on a tree's crown
    let free = |x: i32, y: i32, z: i32| {
        w::is_solid(m.world.get(x, y - 1, z))
            && !w::is_leaves(m.world.get(x, y - 1, z))
            && !w::is_solid(m.world.get(x, y, z))
            && !w::is_solid(m.world.get(x, y + 1, z))
    };
    let keepers: Vec<_> = c.npcs.iter().filter(|n| n.innkeeper).collect();
    assert!(!keepers.is_empty(), "the pack has an innkeeper");
    for n in keepers {
        // where the server puts them (the search of `spawn_npcs`)
        let (x, z) = (h[0] + n.at[0], h[2] + n.at[2]);
        let y0 = h[1] + n.at[1];
        let y = (0..12)
            .flat_map(|d| [y0 + d, y0 - d])
            .find(|y| free(x, *y, z))
            .unwrap_or_else(|| panic!("{} has nowhere to stand", n.key));
        // `walkable` ends within 3 blocks; talking reaches 6
        assert!(
            walkable(&m.world, plaza, [x, y, z]),
            "no way on foot from the plaza to {} at {:?}",
            n.name,
            [x, y, z]
        );
    }
    for st in &c.stations {
        // within 3 blocks of one of its blocks; a station serves 5 blocks round
        let reached = st
            .blocks
            .iter()
            .flat_map(|b| m.world.positions_of(*b))
            .filter(|p| (p[0] - h[0]).abs() <= 150 && (p[2] - h[2]).abs() <= 150)
            .any(|p| walkable(&m.world, plaza, p));
        assert!(reached, "no {} within a walk of the plaza", st.name);
    }
}

/// Profession crafting works in the live world: every recipe of the pack
/// can be made by a player there. Its station stands in the built world;
/// each ingredient can be had (from a resource node that is in the world
/// and can be gathered with a tool that can be had, from a creature, a
/// merchant, a quest, a starter kit or another recipe); and the skill it
/// asks for can be reached by using what asks for less.
#[test]
fn every_recipe_can_be_made_in_the_live_world() {
    use std::collections::{BTreeMap, HashSet};
    let m = made();
    let pack = Pack::builtin();
    let c = &pack.content;
    assert!(!c.recipes.is_empty(), "the pack has recipes");

    // 1. stations: each one a recipe names has a block standing in the world
    let mut standing = HashSet::new();
    for (i, st) in c.stations.iter().enumerate() {
        if st
            .blocks
            .iter()
            .any(|b| !m.world.positions_of(*b).is_empty())
        {
            standing.insert(i as u8);
        }
    }
    for r in &c.recipes {
        if let Some(s) = r.station {
            assert!(
                standing.contains(&s),
                "{}: no {} ({}) stands in the world",
                c.item(r.output).name,
                c.stations[s as usize].name,
                c.stations[s as usize].key
            );
        }
    }

    // 2. what a player can come by, grown until nothing more is added
    let mut have: HashSet<u16> = HashSet::new();
    let key = |k: &str| c.item_by_key(k).unwrap_or_else(|| panic!("no item {k}"));
    for (k, _) in &c.world.starter {
        have.insert(key(k));
    }
    for class in &c.classes {
        for (k, _) in &class.starter {
            have.insert(key(k));
        }
    }
    for cr in &c.creatures {
        have.extend(cr.drops.iter().map(|d| d.item));
    }
    for n in &c.npcs {
        have.extend(n.sells.iter().map(|(i, _)| *i));
    }
    for q in &c.quests {
        have.extend(q.items.iter().chain(&q.choice).map(|(i, _)| *i));
    }
    // resource nodes that are in the world
    let nodes: Vec<u8> = (0..w::BLOCK_COUNT)
        .filter(|b| c.rule(*b).respawn > 0 && !m.world.positions_of(*b).is_empty())
        .collect();
    loop {
        let before = have.len();
        for b in &nodes {
            let rule = c.rule(*b);
            // the tool the node asks for can be had
            let tool_ok = rule.tier == 0
                || have.iter().any(|i| {
                    c.item(*i)
                        .tool
                        .is_some_and(|t| Some(t.kind) == rule.tool && t.tier >= rule.tier)
                });
            if tool_ok {
                have.extend(rule.drops.iter().map(|d| d.item));
            }
        }
        for r in &c.recipes {
            if r.inputs
                .iter()
                .all(|i| i.any.iter().any(|a| have.contains(a)))
            {
                have.insert(r.output);
            }
        }
        if have.len() == before {
            break;
        }
    }
    for r in &c.recipes {
        for i in &r.inputs {
            assert!(
                i.any.iter().any(|a| have.contains(a)),
                "{}: nothing in the live world gives {}",
                c.item(r.output).name,
                i.any
                    .iter()
                    .map(|a| c.item(*a).name)
                    .collect::<Vec<_>>()
                    .join(" or ")
            );
        }
    }

    // 3. skills: each step of a profession is within reach of the one below
    //    (using something raises a skill until it is 75 over what it asks)
    let mut ladders: BTreeMap<&str, Vec<u16>> = BTreeMap::new();
    for r in &c.recipes {
        if let Some((p, n)) = &r.skill {
            ladders.entry(p).or_default().push(*n);
        }
    }
    for b in &nodes {
        if let Some((p, n)) = &c.rule(*b).skill {
            ladders.entry(p).or_default().push(*n);
        }
    }
    for (prof, mut needs) in ladders {
        assert!(
            c.professions.iter().any(|p| p.key == prof),
            "no profession {prof}"
        );
        needs.sort();
        let mut reach = 1u16; // learned at 1 by using something that asks for 1
        assert!(
            needs[0] <= 1,
            "{prof}: nothing teaches it (lowest need {})",
            needs[0]
        );
        for n in needs {
            assert!(
                n <= reach,
                "{prof}: skill {n} cannot be reached (what asks for less stops at {reach})"
            );
            assert!(
                n <= dyno_sim::progression::skill_cap(dyno_sim::progression::MAX_LEVEL),
                "{prof}: skill {n} is over the cap of the highest level"
            );
            reach = reach.max(n + 74);
        }
    }
}
