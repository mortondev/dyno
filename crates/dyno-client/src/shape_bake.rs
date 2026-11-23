//! One-off bake: runs today's detail-shape code over many neighbourhoods and
//! writes what it draws as `.vox` shapes, rule tables and block tags into
//! `content/` (`content/blocks.json`, `content/shapes/*.vox`).
//!
//! `cargo test -p dyno-client --release bake_shapes -- --ignored --nocapture`

use crate::detail::{BAKE, old_cell, old_classify, old_groups};
use crate::light::Light;
use crate::render::{self, SECTION};
use crate::terrain_mesh::{F_DETAIL, F_OPAQUE, Grid, at, flags_with};
use dyno_world::{self as w, BlockId, World};
use glam::Vec3;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, VecDeque};

const N: i32 = SECTION;
/// Stands for the block being baked in a neighbourhood.
const SELF: BlockId = 255;

const OLD_TERRAIN: [BlockId; 12] = [
    w::GRASS,
    w::DIRT,
    w::STONE,
    w::SAND,
    w::EMBERSTONE,
    w::SNOW,
    15,
    16,
    w::CLAY,
    w::BEDROCK,
    w::COAL_ORE,
    w::IRON_ORE,
];
const OLD_PAVING: [BlockId; 3] = [w::COBBLE, w::FLAGSTONES, 19];
const WOOL: std::ops::RangeInclusive<BlockId> = 23..=32;

fn snake(b: BlockId) -> String {
    let n = w::block(b).name.to_lowercase();
    let mut s: String = n
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    while s.contains("__") {
        s = s.replace("__", "_");
    }
    s.trim_matches('_').to_string()
}

fn name_of(b: BlockId) -> String {
    // block names must be unique keys
    let s = snake(b);
    let dup = (0..w::BLOCK_COUNT).filter(|o| snake(*o) == s).count() > 1;
    if dup { format!("{s}_{b}") } else { s }
}

// ---------------------------------------------------------- conditions

fn at_(d: [i32; 3], is: &str) -> Value {
    json!({"at": d, "is": is})
}
fn all(v: Vec<Value>) -> Value {
    json!({ "all": v })
}
fn any(v: Vec<Value>) -> Value {
    json!({ "any": v })
}
fn not(v: Value) -> Value {
    json!({ "not": v })
}
fn add(a: [i32; 3], b: [i32; 3]) -> [i32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
const H4: [[i32; 3]; 4] = [[1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1]];
const D6: [[i32; 3]; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];

/// A class: its rule (no parts yet) and its features.
struct ClassSpec {
    rule: Value,
    features: Vec<Value>,
}

fn class(rule: Value, features: Vec<Value>) -> ClassSpec {
    ClassSpec { rule, features }
}

fn cube() -> ClassSpec {
    class(json!({"class": "cube", "fills": true}), vec![])
}

/// Roofs and tents: slopes and peaks over the blocks of the group `g`.
fn sloped(g: &str, cloth: bool) -> Vec<ClassSpec> {
    let top = if cloth {
        Some(all(vec![
            not(at_([0, 1, 0], "solid")),
            any(vec![not(at_([0, -1, 0], "solid")), at_([0, -1, 0], "cloth")]),
        ]))
    } else {
        None
    };
    let peak = all(vec![
        at_([0, -1, -1], g),
        at_([0, -1, 1], g),
        not(at_([0, 1, 1], g)),
        not(at_([0, 1, -1], g)),
    ]);
    let wedge = if cloth {
        at_([0, -1, 0], "cloth")
    } else {
        all(vec![at_([0, -1, 0], "wall"), not(at_([0, -1, 0], "roof"))])
    };
    let wedge_at = |x: i32| {
        if cloth {
            at_([x, -1, 0], "cloth")
        } else {
            all(vec![at_([x, -1, 0], "wall"), not(at_([x, -1, 0], "roof"))])
        }
    };
    let slope_f = vec![
        at_([-1, 0, 0], g),
        at_([1, 0, 0], g),
        at_([0, -1, -1], g),
        wedge.clone(),
        at_([0, 0, 1], "opaque"),
        at_([0, 0, 1], g),
        at_([0, 1, 1], g),
        at_([0, 0, 1], "roof"),
        at_([0, 0, 1], "wall"),
        wedge_at(-1),
        wedge_at(1),
    ];
    let peak_f = vec![
        at_([-1, 0, 0], g),
        at_([1, 0, 0], g),
        wedge,
        wedge_at(-1),
        wedge_at(1),
    ];
    let with_top = |c: Value| match &top {
        Some(t) => all(vec![t.clone(), c]),
        None => c,
    };
    let mut v = vec![
        class(
            json!({"class": "peak", "fills": false, "turns": [1, 0], "when": with_top(peak)}),
            peak_f,
        ),
        class(
            {
                let mut r = json!({"class": "slope", "fills": false, "turns": [1, 3, 0, 2],
                    "pick": "best", "min": 1.0,
                    "score": [[2.0, at_([0, 1, 1], g)], [2.0, at_([0, -1, -1], g)],
                              [-1.0, at_([0, 1, -1], g)], [-1.0, at_([0, -1, 1], g)]]});
                if let Some(t) = &top {
                    r["when"] = t.clone();
                }
                r
            },
            slope_f,
        ),
    ];
    if cloth {
        v.push(class(
            json!({"class": "sheet", "fills": false,
                   "when": all(vec![not(at_([0, 1, 0], "solid")), not(at_([0, -1, 0], "solid"))])}),
            H4.iter().map(|d| at_(*d, "cloth")).collect(),
        ));
    }
    v.push(cube());
    v
}

fn timber(log: bool) -> Vec<ClassSpec> {
    let wall_h = any(H4.iter().map(|d| at_(*d, "wall")).collect());
    let wood_x = any(vec![at_([1, 0, 0], "wood"), at_([-1, 0, 0], "wood")]);
    let wood_z = any(vec![at_([0, 0, 1], "wood"), at_([0, 0, -1], "wood")]);
    let wall_v = any(vec![at_([0, -1, 0], "wall"), at_([0, 1, 0], "wall")]);
    let corner = all(vec![
        wood_x.clone(),
        wood_z.clone(),
        at_([0, -1, 0], "wood"),
        at_([0, 1, 0], "solid"),
    ]);
    let beam = any(vec![
        wall_h,
        all(vec![wall_v, any(vec![wood_x.clone(), wood_z.clone()])]),
        corner,
    ]);
    let beam_f: Vec<Value> = D6
        .iter()
        .map(|d| any(vec![at_(*d, "opaque"), at_(*d, "roof")]))
        .collect();
    let down = all(vec![
        at_([0, -1, 0], "solid"),
        not(at_([0, -1, 0], "banner")),
        not(at_([0, -1, 0], "lantern")),
    ]);
    let up = all(vec![at_([0, 1, 0], "solid"), not(at_([0, 1, 0], "banner"))]);
    let arm = |d: [i32; 3]| {
        any(vec![
            at_(d, "wood"),
            all(vec![at_(d, "wall"), not(down.clone()), not(up.clone())]),
        ])
    };
    let post = any(vec![
        up.clone(),
        down.clone(),
        all(H4.iter().map(|d| not(arm(*d))).collect()),
    ]);
    let mut stick_f = vec![down.clone(), up.clone(), post];
    for d in H4 {
        stick_f.push(arm(d));
    }
    for d in H4 {
        let b = add(d, [0, -1, 0]);
        stick_f.push(all(vec![at_(b, "solid"), not(at_(b, "banner"))]));
    }
    stick_f.push(all(vec![
        not(at_([0, -1, 0], "wood")),
        not(at_([0, -1, 0], "stone")),
    ]));
    let mut v = vec![class(
        json!({"class": "beam", "fills": true, "when": beam}),
        beam_f,
    )];
    if log {
        let trunk_f = vec![
            at_([0, 1, 0], "self"),
            at_([0, 2, 0], "self"),
            at_([0, -1, 0], "self"),
            at_([0, -1, 0], "solid"),
            at_([0, 1, 0], "leaves"),
            all(H4.iter().map(|d| at_(*d, "leaves")).collect()),
        ];
        v.push(class(
            json!({"class": "trunk", "fills": false,
                   "when": all(vec![not(wood_x), not(wood_z)])}),
            trunk_f,
        ));
    }
    v.push(class(json!({"class": "stick", "fills": false}), stick_f));
    v
}

fn pillar_free(o: [i32; 3]) -> Value {
    let mut c = vec![at_(o, "stone")];
    for d in H4 {
        c.push(not(at_(add(o, d), "wall")));
    }
    for d in [[1, 0, 1], [1, 0, -1], [-1, 0, 1], [-1, 0, -1]]
        .into_iter()
        .chain(H4)
    {
        c.push(not(at_(add(o, d), "roof")));
    }
    all(c)
}

fn stone() -> Vec<ClassSpec> {
    let pillar = all(vec![
        pillar_free([0, 0, 0]),
        any(vec![
            pillar_free([0, -1, 0]),
            pillar_free([0, 1, 0]),
            at_([0, -1, 0], "terrain"),
            at_([0, -1, 0], "paving"),
        ]),
    ]);
    vec![
        class(
            json!({"class": "pillar", "fills": false, "when": pillar}),
            vec![at_([0, -1, 0], "class:pillar"), at_([0, 1, 0], "class:pillar")],
        ),
        cube(),
    ]
}

fn prop(jitter: f32, features: Vec<Value>) -> Vec<ClassSpec> {
    vec![class(
        json!({"class": "prop", "fills": false, "jitter": jitter}),
        features,
    )]
}

fn lantern() -> Vec<ClassSpec> {
    let f = vec![
        all(vec![at_([0, -1, 0], "solid"), not(at_([0, -1, 0], "banner"))]),
        at_([0, 1, 0], "solid"),
    ];
    vec![
        class(
            json!({"class": "wall_lantern", "fills": false, "turns": [1, 3, 0, 2],
                   "when": at_([0, 0, 1], "wall")}),
            f.clone(),
        ),
        class(json!({"class": "lantern", "fills": false}), f),
    ]
}

fn banner() -> Vec<ClassSpec> {
    let f = vec![at_([0, 1, 0], "banner"), at_([0, -1, 0], "banner")];
    vec![
        class(
            json!({"class": "wall_banner", "fills": false, "turns": [1, 3, 0, 2],
                   "when": at_([0, 0, 1], "wall")}),
            f.clone(),
        ),
        class(
            json!({"class": "hung_banner", "fills": false, "turns": [0],
                   "when": any(vec![at_([1, 1, 0], "wood"), at_([-1, 1, 0], "wood")])}),
            f.clone(),
        ),
        class(
            json!({"class": "hung_banner", "fills": false, "turns": [1]}),
            f,
        ),
    ]
}

fn pane(door: bool) -> Vec<ClassSpec> {
    let solidish = |d: [i32; 3]| any(vec![at_(d, "wall"), at_(d, "wood")]);
    let f = if door {
        vec![at_([0, 1, 0], "self"), at_([0, -1, 0], "self")]
    } else {
        vec![
            at_([-1, 0, 0], "self"),
            at_([1, 0, 0], "self"),
            at_([0, 1, 0], "self"),
            at_([0, -1, 0], "self"),
        ]
    };
    vec![class(
        json!({"class": "pane", "fills": false, "turns": [0, 1], "pick": "best", "min": -100.0,
               "score": [[1.0, solidish([1, 0, 0])], [1.0, solidish([-1, 0, 0])]]}),
        f,
    )]
}

/// A rule set to bake: its classes, the blocks baked for it (each with its
/// own `.vox`), the blocks that use each, and the neighbours to sample.
struct SetSpec {
    name: &'static str,
    classes: Vec<ClassSpec>,
    /// (baked block, vox file stem, blocks drawn with it)
    variants: Vec<(BlockId, &'static str, Vec<BlockId>)>,
    palette: Vec<(BlockId, f32)>,
    samples: usize,
}

fn sets() -> Vec<SetSpec> {
    use w::*;
    let wool: Vec<BlockId> = WOOL.collect();
    let mut v = vec![
        SetSpec {
            name: "roof",
            classes: sloped("roof", false),
            variants: vec![
                (RED_ROOF_TILES, "roof_red", vec![RED_ROOF_TILES]),
                (SLATE_ROOF, "roof_slate", vec![SLATE_ROOF]),
                (THATCH, "roof_thatch", vec![THATCH]),
            ],
            palette: vec![
                (AIR, 5.0),
                (SELF, 3.0),
                (SLATE_ROOF, 1.0),
                (PLASTER, 3.0),
                (STONE_BRICKS, 1.0),
                (TIMBER_BEAM, 1.0),
                (GLASS, 0.4),
                (23, 0.4),
                (LEADED_WINDOW, 0.3),
                (GRASS, 0.5),
            ],
            samples: 24000,
        },
        SetSpec {
            name: "cloth",
            classes: sloped("cloth", true),
            variants: vec![(23, "cloth", wool)],
            palette: vec![
                (AIR, 6.0),
                (SELF, 3.0),
                (27, 1.0),
                (PLASTER, 1.5),
                (TIMBER_BEAM, 1.0),
                (RED_ROOF_TILES, 0.5),
                (GRASS, 1.0),
            ],
            samples: 16000,
        },
        SetSpec {
            name: "timber",
            classes: timber(false),
            variants: vec![(TIMBER_BEAM, "timber", vec![TIMBER_BEAM])],
            palette: vec![
                (AIR, 7.0),
                (SELF, 2.0),
                (LOG, 0.7),
                (PLASTER, 1.5),
                (STONE_BRICKS, 1.0),
                (COBBLE, 0.7),
                (GRASS, 1.0),
                (RED_BANNER, 0.5),
                (LANTERN, 0.5),
                (RED_ROOF_TILES, 0.6),
                (GLASS, 0.3),
            ],
            samples: 24000,
        },
        SetSpec {
            name: "log",
            classes: timber(true),
            variants: vec![(LOG, "log", vec![LOG]), (BIRCH_LOG, "birch_log", vec![BIRCH_LOG])],
            palette: vec![
                (AIR, 7.0),
                (SELF, 3.0),
                (TIMBER_BEAM, 0.6),
                (PLASTER, 1.0),
                (COBBLE, 0.7),
                (GRASS, 1.5),
                (LEAVES, 3.0),
                (RED_BANNER, 0.3),
                (LANTERN, 0.3),
                (RED_ROOF_TILES, 0.4),
            ],
            samples: 24000,
        },
        SetSpec {
            name: "stone",
            classes: stone(),
            variants: vec![(COBBLE, "pillar", vec![COBBLE, STONE_BRICKS, 19, BRICK])],
            palette: vec![
                (AIR, 6.0),
                (SELF, 3.0),
                (STONE_BRICKS, 1.0),
                (PLASTER, 1.0),
                (GRASS, 1.0),
                (FLAGSTONES, 0.5),
                (RED_ROOF_TILES, 0.6),
                (TIMBER_BEAM, 0.5),
            ],
            samples: 16000,
        },
        SetSpec {
            name: "lantern",
            classes: lantern(),
            variants: vec![(LANTERN, "lantern", vec![LANTERN])],
            palette: vec![
                (AIR, 6.0),
                (PLASTER, 2.0),
                (TIMBER_BEAM, 1.0),
                (GRASS, 1.0),
                (RED_BANNER, 0.5),
                (COBBLE, 1.0),
            ],
            samples: 4000,
        },
        SetSpec {
            name: "banner",
            classes: banner(),
            variants: vec![(RED_BANNER, "banner", vec![RED_BANNER, BLUE_BANNER])],
            palette: vec![
                (AIR, 5.0),
                (SELF, 2.0),
                (PLASTER, 2.0),
                (TIMBER_BEAM, 2.0),
                (GRASS, 1.0),
            ],
            samples: 4000,
        },
        SetSpec {
            name: "window",
            classes: pane(false),
            variants: vec![(LEADED_WINDOW, "window", vec![LEADED_WINDOW])],
            palette: vec![
                (AIR, 4.0),
                (SELF, 3.0),
                (PLASTER, 2.0),
                (STONE_BRICKS, 1.0),
                (TIMBER_BEAM, 2.0),
                (GLASS, 0.5),
                (WOODEN_DOOR, 0.5),
            ],
            samples: 6000,
        },
        SetSpec {
            name: "door",
            classes: pane(true),
            variants: vec![(WOODEN_DOOR, "door", vec![WOODEN_DOOR])],
            palette: vec![
                (AIR, 4.0),
                (SELF, 3.0),
                (PLASTER, 2.0),
                (TIMBER_BEAM, 1.0),
            ],
            samples: 3000,
        },
    ];
    let above_self = || vec![at_([0, 1, 0], "self")];
    for (name, b, jitter, f) in [
        ("barrel", BARREL, 3.0, above_self()),
        ("crate", CRATE, 1.0, above_self()),
        ("hay_bale", HAY_BALE, 0.5, above_self()),
        ("anvil", ANVIL, 1.0, vec![]),
        ("brazier", BRAZIER, 0.0, vec![]),
        ("flower_bed", FLOWER_BED, 0.0, vec![]),
    ] {
        v.push(SetSpec {
            name,
            classes: prop(jitter, f),
            variants: vec![(b, name, vec![b])],
            palette: vec![(AIR, 4.0), (SELF, 2.0), (PLASTER, 1.0), (GRASS, 1.0)],
            samples: 600,
        });
    }
    v
}

// ------------------------------------------------------------- voxelising

/// A material as data: whose texture, tint, glow, translucent.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
struct MatKey {
    src: String,
    tint: [i32; 3],
    glow: bool,
    clear: bool,
}

struct Tri {
    p: [Vec3; 3],
    n: Vec3,
    mat: u32,
    nclass: u8,
    face: u8,
}

/// Solid voxels (16 a block) of a set of triangles: inside closed or
/// paired surfaces by ray parity on all three axes, and every voxel a
/// surface passes through. Each takes the material of the surface nearest.
fn voxelize(tris: &[Tri]) -> Vec<([i32; 3], u32)> {
    if tris.is_empty() {
        return Vec::new();
    }
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for t in tris {
        for p in t.p {
            lo = lo.min(p);
            hi = hi.max(p);
        }
    }
    let lo = lo.floor().as_ivec3() - 1;
    let hi = hi.ceil().as_ivec3() + 1;
    let size = hi - lo;
    let idx = |p: [i32; 3]| -> Option<usize> {
        let r = [p[0] - lo.x, p[1] - lo.y, p[2] - lo.z];
        (r[0] >= 0 && r[1] >= 0 && r[2] >= 0 && r[0] < size.x && r[1] < size.y && r[2] < size.z)
            .then(|| ((r[1] * size.z + r[2]) * size.x + r[0]) as usize)
    };
    let n = (size.x * size.y * size.z) as usize;
    let mut solid = vec![false; n];
    let mut votes: Vec<HashMap<u32, f32>> = vec![HashMap::new(); n];
    // surfaces
    for t in tris {
        let e = [(t.p[1] - t.p[0]).length(), (t.p[2] - t.p[1]).length(), (t.p[0] - t.p[2]).length()];
        let steps = (e.iter().fold(0.0f32, |a, b| a.max(*b)) / 0.25).ceil().max(1.0) as i32;
        let un = t.n.normalize_or_zero();
        for i in 0..=steps {
            for j in 0..=steps - i {
                let (a, b) = (i as f32 / steps as f32, j as f32 / steps as f32);
                let p = t.p[0] + (t.p[1] - t.p[0]) * a + (t.p[2] - t.p[0]) * b - un * 0.3;
                let v = p.floor().as_ivec3();
                if let Some(k) = idx([v.x, v.y, v.z]) {
                    solid[k] = true;
                    *votes[k].entry(t.mat).or_default() += 1.0;
                }
            }
        }
    }
    // insides, by parity along each axis
    for axis in 0..3usize {
        let (ua, va) = ((axis + 1) % 3, (axis + 2) % 3);
        let mut hits: HashMap<(i32, i32), Vec<(f32, i8)>> = HashMap::new();
        for t in tris {
            let e1 = t.p[1] - t.p[0];
            let e2 = t.p[2] - t.p[0];
            let mut g = e1.cross(e2);
            if g.dot(t.n) < 0.0 {
                g = -g;
            }
            if g[axis].abs() < 1e-6 {
                continue;
            }
            let sign: i8 = if g[axis] < 0.0 { -1 } else { 1 };
            let (umin, umax) = (
                t.p.iter().map(|p| p[ua]).fold(f32::MAX, f32::min),
                t.p.iter().map(|p| p[ua]).fold(f32::MIN, f32::max),
            );
            let (vmin, vmax) = (
                t.p.iter().map(|p| p[va]).fold(f32::MAX, f32::min),
                t.p.iter().map(|p| p[va]).fold(f32::MIN, f32::max),
            );
            for u in (umin - 0.5).ceil() as i32..=(umax - 0.5).floor() as i32 {
                for v in (vmin - 0.5).ceil() as i32..=(vmax - 0.5).floor() as i32 {
                    let (pu, pv) = (u as f32 + 0.5, v as f32 + 0.5);
                    // 2D barycentric
                    let d = |a: Vec3, b: Vec3| (b[ua] - a[ua]) * (pv - a[va]) - (b[va] - a[va]) * (pu - a[ua]);
                    let w0 = d(t.p[1], t.p[2]);
                    let w1 = d(t.p[2], t.p[0]);
                    let w2 = d(t.p[0], t.p[1]);
                    let inside = (w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0)
                        || (w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0);
                    if !inside {
                        continue;
                    }
                    // the plane's height along the axis at (pu, pv)
                    let num = g[ua] * (pu - t.p[0][ua]) + g[va] * (pv - t.p[0][va]);
                    let h = t.p[0][axis] - num / g[axis];
                    hits.entry((u, v)).or_default().push((h, sign));
                }
            }
        }
        for ((u, v), mut list) in hits {
            list.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut start: Option<f32> = None;
            for (h, s) in list {
                if s < 0 {
                    if start.is_none() {
                        start = Some(h);
                    }
                } else if let Some(a) = start.take() {
                    for k in (a - 0.5).ceil() as i32..=(h - 0.5).floor() as i32 {
                        if (k as f32 + 0.5) <= a || (k as f32 + 0.5) >= h {
                            continue;
                        }
                        let mut p = [0; 3];
                        p[axis] = k;
                        p[ua] = u;
                        p[va] = v;
                        if let Some(i) = idx(p) {
                            solid[i] = true;
                        }
                    }
                }
            }
        }
    }
    // materials: the most voted, spread inward to voxels no surface touched
    let mut mat: Vec<Option<u32>> = votes
        .iter()
        .map(|m| {
            m.iter()
                .max_by(|a, b| a.1.total_cmp(b.1).then(b.0.cmp(a.0)))
                .map(|(k, _)| *k)
        })
        .collect();
    let mut queue: VecDeque<usize> = (0..n).filter(|i| solid[*i] && mat[*i].is_some()).collect();
    let (sx, sy, sz) = (size.x as usize, size.y as usize, size.z as usize);
    while let Some(i) = queue.pop_front() {
        let x = i % sx;
        let z = (i / sx) % sz;
        let y = i / (sx * sz);
        let mut next = Vec::new();
        if x > 0 {
            next.push(i - 1);
        }
        if x + 1 < sx {
            next.push(i + 1);
        }
        if z > 0 {
            next.push(i - sx);
        }
        if z + 1 < sz {
            next.push(i + sx);
        }
        if y > 0 {
            next.push(i - sx * sz);
        }
        if y + 1 < sy {
            next.push(i + sx * sz);
        }
        for j in next {
            if solid[j] && mat[j].is_none() {
                mat[j] = mat[i];
                queue.push_back(j);
            }
        }
    }
    let mut out = Vec::new();
    for i in 0..n {
        if let (true, Some(m)) = (solid[i], mat[i]) {
            let x = (i % sx) as i32 + lo.x;
            let z = ((i / sx) % sz) as i32 + lo.z;
            let y = (i / (sx * sz)) as i32 + lo.y;
            out.push(([x, y, z], m));
        }
    }
    out.sort();
    out
}

// ------------------------------------------------------------------ baking

/// Inverse of a quarter turn, on a point in the cell (about its middle).
fn unturn(p: Vec3, q: u8) -> Vec3 {
    let c = Vec3::new(0.5, 0.0, 0.5);
    let v = p - c;
    let r = match q & 3 {
        0 => v,
        // inverse of (x,z) -> (z,-x) is (x,z) -> (-z, x)
        1 => Vec3::new(-v.z, v.y, v.x),
        2 => Vec3::new(-v.x, v.y, -v.z),
        _ => Vec3::new(v.z, v.y, -v.x),
    };
    r + c
}

fn turn_i(d: [i32; 3], q: u8) -> [i32; 3] {
    match q & 3 {
        0 => d,
        1 => [d[2], d[1], -d[0]],
        2 => [-d[0], d[1], -d[2]],
        _ => [-d[2], d[1], d[0]],
    }
}

/// Everything learned about one rule set.
#[derive(Default)]
struct Learned {
    /// materials, by index
    mats: Vec<MatKey>,
    mat_index: HashMap<MatKey, u32>,
    /// (material, normal class) -> face -> area
    faces: HashMap<(u32, u8), HashMap<u8, f32>>,
    /// geometry store
    geoms: Vec<Vec<([i32; 3], u32)>>,
    geom_index: HashMap<Vec<([i32; 3], u32)>, u32>,
    /// class -> part -> feature bits -> (geometry per variant, glow) -> count
    seen: BTreeMap<String, BTreeMap<String, HashMap<u64, HashMap<(Vec<u32>, Option<[i32; 3]>), u32>>>>,
    /// class -> how many samples
    classes: BTreeMap<String, u32>,
    mismatch: u32,
    cells: u32,
}

impl Learned {
    fn mat(&mut self, k: MatKey) -> u32 {
        if let Some(i) = self.mat_index.get(&k) {
            return *i;
        }
        let i = self.mats.len() as u32;
        self.mats.push(k.clone());
        self.mat_index.insert(k, i);
        i
    }
    fn geom(&mut self, g: Vec<([i32; 3], u32)>) -> u32 {
        if let Some(i) = self.geom_index.get(&g) {
            return *i;
        }
        let i = self.geoms.len() as u32;
        self.geoms.push(g.clone());
        self.geom_index.insert(g, i);
        i
    }
}

/// The texture layer of each block face, to name a material by its block.
fn layer_owner(layer: u16, prefer: &[BlockId]) -> Option<(BlockId, u8)> {
    let t = render::textures();
    for b in prefer.iter().copied().chain(1..w::BLOCK_COUNT) {
        for f in 0..3u8 {
            if t.face(b, f as usize) == layer {
                return Some((b, f));
            }
        }
    }
    None
}

const NONE_GEOM: u32 = u32::MAX;

/// Bake one cell: what today's code draws, by part, in the class's frame.
#[allow(clippy::too_many_arguments)]
fn bake_cell(
    l: &mut Learned,
    g: &Grid,
    light: &Light,
    c: usize,
    wp: [i32; 3],
    class_name: &str,
    q: u8,
    feats: &[bool],
    labels: &[&'static str],
) -> HashMap<&'static str, (u32, Option<[i32; 3]>)> {
    let old = BAKE.with(|b| {
        b.set(true);
        let r = old_cell(g, light, c, wp);
        b.set(false);
        r
    });
    let own = g.ids[c];
    let base = Vec3::new(wp[0] as f32, wp[1] as f32, wp[2] as f32);
    let nbr = |d: [i32; 3]| {
        let t = turn_i(d, q);
        g.ids[(c as isize
            + t[0] as isize * crate::terrain_mesh::DX as isize
            + t[1] as isize * crate::terrain_mesh::DY as isize
            + t[2] as isize * crate::terrain_mesh::DZ as isize) as usize]
    };
    // triangles by part
    let mut by_part: HashMap<&'static str, Vec<Tri>> = HashMap::new();
    let mut marks = old.marks.clone();
    marks.push(("", old.opaque.len(), old.trans.len()));
    let tex = render::textures();
    let _ = tex;
    for win in marks.windows(2) {
        let (label, o0, t0) = win[0];
        let (_, o1, t1) = win[1];
        for (verts, clear) in [(&old.opaque[o0..o1], false), (&old.trans[t0..t1], true)] {
            for tri in verts.chunks(3) {
                if tri.len() < 3 {
                    continue;
                }
                let v = tri[0];
                let layer = v.tex[2] as u16;
                let glow = v.color[3] > 1.9;
                let wedge = label == "wedge" || label == "wedge_back";
                let frame = label.strip_prefix("frame_").map(|s| match s {
                    "l" => [-1, 0, 0],
                    "r" => [1, 0, 0],
                    "t" => [0, 1, 0],
                    _ => [0, -1, 0],
                });
                let (src, face) = (|| {
                    for f in 0..3u8 {
                        if render::textures().face(own, f as usize) == layer {
                            return (String::from("self"), f);
                        }
                    }
                    if wedge {
                        let b = nbr([0, -1, 0]);
                        for f in 0..3u8 {
                            if render::textures().face(b, f as usize) == layer {
                                return ("at:0,-1,0".to_string(), f);
                            }
                        }
                    }
                    if let Some(d) = frame {
                        let b = nbr(d);
                        if render::textures().face(b, 1) == layer
                            || render::textures().face(w::TIMBER_BEAM, 1) == layer
                        {
                            return (format!("frame:{},{},{}", d[0], d[1], d[2]), 1);
                        }
                    }
                    match layer_owner(
                        layer,
                        &[w::TIMBER_BEAM, 20, w::STONE_BRICKS, w::BRAZIER, w::LOG, w::LANTERN, w::HAY_BALE],
                    ) {
                        Some((b, f)) => (format!("block:{}", name_of(b)), f),
                        None => ("self".to_string(), 1),
                    }
                })();
                let tint = [v.color[0], v.color[1], v.color[2]].map(|x| (x * 1000.0).round() as i32);
                let m = l.mat(MatKey {
                    src,
                    tint,
                    glow,
                    clear,
                });
                let p = [0, 1, 2].map(|i| unturn(Vec3::from(tri[i].pos) - base, q) * 16.0);
                let nw = Vec3::from(tri[0].normal) + Vec3::from(tri[1].normal) + Vec3::from(tri[2].normal);
                let n = unturn(nw + Vec3::new(0.5, 0.0, 0.5), q) - Vec3::new(0.5, 0.0, 0.5);
                let n = if n.length() < 1e-4 {
                    (p[1] - p[0]).cross(p[2] - p[0])
                } else {
                    n
                };
                let nn = n.normalize_or_zero();
                let nclass = if nn.y > 0.5 {
                    0
                } else if nn.y < -0.5 {
                    2
                } else {
                    1
                };
                let area = (p[1] - p[0]).cross(p[2] - p[0]).length() * 0.5;
                *l.faces
                    .entry((m, nclass))
                    .or_default()
                    .entry(face)
                    .or_default() += area;
                by_part.entry(label).or_default().push(Tri {
                    p,
                    n,
                    mat: m,
                    nclass,
                    face,
                });
            }
        }
    }
    let _ = feats;
    let _ = class_name;
    // the lamp's glow goes with its lamp (or its body)
    let glow_part = if labels.contains(&"lamp") { "lamp" } else { "body" };
    let glow = old.glow.map(|gl| {
        let p = unturn(Vec3::from(gl.pos) - base, q) * 16.0;
        p.round().as_ivec3().to_array()
    });
    let mut out = HashMap::new();
    for label in labels {
        let geom = match by_part.get(label) {
            Some(tris) => {
                let v = voxelize(tris);
                if v.is_empty() { NONE_GEOM } else { l.geom(v) }
            }
            None => NONE_GEOM,
        };
        let gl = if *label == glow_part { glow } else { None };
        out.insert(*label, (geom, gl));
    }
    for t in by_part.values().flatten() {
        let _ = (t.nclass, t.face);
    }
    // parts the list did not know about
    for k in by_part.keys() {
        if !labels.contains(k) {
            panic!("unlisted part {k}");
        }
    }
    out
}

/// The parts each set's classes may draw.
fn labels_of(set: &str, class: &str) -> Vec<&'static str> {
    match (set, class) {
        ("roof" | "cloth", "slope") => vec!["rows", "under", "eave", "ends", "wedge", "wedge_back", "cap"],
        ("roof" | "cloth", "peak") => vec!["rows", "under", "ends", "wedge", "ridge"],
        ("cloth", "sheet") => vec!["sheet", "flap0", "flap1", "flap2", "flap3"],
        (_, "beam") => vec!["skin+x", "skin-x", "skin+y", "skin-y", "skin+z", "skin-z"],
        (_, "stick") => vec!["post", "foot", "cap", "base", "arm0", "arm1", "arm2", "arm3", "beam"],
        (_, "trunk") => vec!["trunk"],
        (_, "pillar") => vec!["foot", "head", "shaft"],
        ("lantern", _) => vec!["bracket", "hanger", "lamp"],
        ("banner", _) => vec!["body", "rod"],
        ("window", _) => vec![
            "body", "frame_l", "frame_r", "frame_t", "frame_b", "sill", "mullion", "glass",
        ],
        ("door", _) => vec!["body", "straps"],
        _ => vec!["body", "lid", "twine", "coals"],
    }
}

/// A random neighbourhood around (cx, cy, cz).
fn scatter(world: &mut World, c: [i32; 3], pal: &[(BlockId, f32)], me: BlockId, rng: &mut u64) {
    let total: f32 = pal.iter().map(|p| p.1).sum();
    for dy in -3..=3i32 {
        for dz in -2..=2i32 {
            for dx in -2..=2i32 {
                if dx == 0 && dy == 0 && dz == 0 {
                    world.set(c[0], c[1], c[2], me);
                    continue;
                }
                // farther cells are emptier
                let far = dx.abs().max(dz.abs()).max((dy.abs() - 1).max(0));
                *rng ^= *rng << 13;
                *rng ^= *rng >> 7;
                *rng ^= *rng << 17;
                let r = (*rng >> 11) as f32 / (1u64 << 53) as f32;
                if far >= 2 && r < 0.35 {
                    world.set(c[0] + dx, c[1] + dy, c[2] + dz, w::AIR);
                    continue;
                }
                let mut x = ((*rng >> 20) & 0xFFFF) as f32 / 65536.0 * total;
                let mut b = w::AIR;
                for (k, wt) in pal {
                    if x < *wt {
                        b = *k;
                        break;
                    }
                    x -= wt;
                }
                let b = if b == SELF { me } else { b };
                world.set(c[0] + dx, c[1] + dy, c[2] + dz, b);
            }
        }
    }
}

/// Run every set: sample, learn, write.
#[test]
#[ignore = "writes content/blocks.json and content/shapes/"]
fn bake_shapes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let specs = sets();
    // 1. tags and the rules without tables
    let old_detail = |b: BlockId| specs.iter().any(|s| s.variants.iter().any(|v| v.2.contains(&b)));
    let mut blocks = serde_json::Map::new();
    for b in 1..w::BLOCK_COUNT {
        let mut tags: Vec<String> = old_groups(b).into_iter().map(String::from).collect();
        if OLD_TERRAIN.contains(&b) {
            tags.push("terrain".into());
        }
        if OLD_PAVING.contains(&b) {
            tags.push("paving".into());
        }
        if w::is_leaves(b) {
            tags.push("leaves".into());
        }
        if b == w::WATER {
            tags.push("water".into());
        }
        let chamfer = w::is_opaque(b)
            && !OLD_TERRAIN.contains(&b)
            && !w::is_leaves(b)
            && b != w::WATER
            && w::block(b).alpha >= 1.0
            && w::emission(b) == 0;
        if chamfer {
            tags.push("chamfer".into());
        }
        let mut e = json!({"id": b, "tags": tags});
        for s in &specs {
            for (_, stem, users) in &s.variants {
                if users.contains(&b) {
                    e["shape"] = json!({"rules": s.name, "vox": format!("shapes/{stem}.vox")});
                }
            }
        }
        blocks.insert(name_of(b), e);
    }
    let rules_stage = |tables: Option<&BTreeMap<String, Value>>| -> Value {
        let mut out = serde_json::Map::new();
        for s in &specs {
            if let Some(t) = tables {
                if let Some(v) = t.get(s.name) {
                    out.insert(s.name.to_string(), v.clone());
                }
                continue;
            }
            let classes: Vec<Value> = s
                .classes
                .iter()
                .map(|c| {
                    let mut r = c.rule.clone();
                    if !c.features.is_empty() || r["class"] != "cube" {
                        r["parts"] = json!([{"name": "all", "features": c.features, "table": {}}]);
                    }
                    r
                })
                .collect();
            out.insert(s.name.to_string(), json!({"models": [], "classes": classes}));
        }
        Value::Object(out)
    };
    let stage1 = json!({"blocks": blocks, "shape_rules": rules_stage(None)});
    let text = serde_json::to_string(&stage1).unwrap();
    crate::assets::insert("blocks.json", text.clone().into_bytes());
    let pk = crate::shapes::parse_pack(&text).expect("stage one parses");
    let old_lut = flags_with(old_detail, |b| OLD_TERRAIN.contains(&b));

    // 2. sample
    let mut tables: BTreeMap<String, Value> = BTreeMap::new();
    let mut palettes: serde_json::Map<String, Value> = serde_json::Map::new();
    let _ = std::fs::create_dir_all(root.join("shapes"));
    let mut rng = 0x9E37_79B9_7F4A_7C15u64;
    for s in &specs {
        let mut l = Learned::default();
        let n_var = s.variants.len();
        let per_world = 16 * 16;
        let worlds = s.samples.div_ceil(per_world);
        // the town too: what is really built
        let town = {
            let mut world = World::generate([128, 64, 128], 7);
            let hearth = world.ensure_spawn_hearth(None);
            w::village::build_hearthstead(&mut world, hearth);
            world
        };
        let mut jobs: Vec<(World, Vec<[i32; 3]>)> = Vec::new();
        for _ in 0..worlds {
            let mut world = World::empty([128, 32, 128]);
            let mut sites = Vec::new();
            for sz in 0..16 {
                for sx in 0..16 {
                    let c = [sx * 8 + 4, 12, sz * 8 + 4];
                    scatter(&mut world, c, &s.palette, s.variants[0].0, &mut rng);
                    sites.push(c);
                }
            }
            jobs.push((world, sites));
        }
        {
            let users: Vec<BlockId> = s.variants.iter().flat_map(|v| v.2.clone()).collect();
            let mut sites = Vec::new();
            for y in 1..63 {
                for z in 4..124 {
                    for x in 4..124 {
                        if users.contains(&town.get(x, y, z)) {
                            sites.push([x, y, z]);
                        }
                    }
                }
            }
            // town sites keep their own block; bake them as the first variant
            jobs.push((town, sites));
        }
        for (world, sites) in jobs {
            // every variant over the same neighbourhoods
            let mut per_var: Vec<(World, Light)> = Vec::new();
            for (vi, (vb, _, users)) in s.variants.iter().enumerate() {
                let mut wv = world.clone();
                for c in &sites {
                    let here = wv.get(c[0], c[1], c[2]);
                    if users.contains(&here) || vi > 0 || here == s.variants[0].0 {
                        wv.set(c[0], c[1], c[2], *vb);
                    }
                }
                let light = Light::new(&wv);
                per_var.push((wv, light));
            }
            let mut by_section: BTreeMap<[i32; 3], Vec<[i32; 3]>> = BTreeMap::new();
            for c in &sites {
                by_section.entry(c.map(|v| v.div_euclid(N))).or_default().push(*c);
            }
            for (sec, cells) in by_section {
                let o = sec.map(|v| v * N);
                let mut grids = Vec::new();
                for (wv, _) in &per_var {
                    let mut ga = Grid::load_with(wv, o, &old_lut);
                    let mut gb = Grid::load_with(wv, o, &old_lut);
                    old_classify(&mut ga);
                    crate::shapes::classify_with(&pk, &mut gb);
                    ga.cls = gb.cls.clone();
                    // which blocks fill their cells must agree
                    for y in -3..N + 3 {
                        for z in -3..N + 3 {
                            for x in -3..N + 3 {
                                let k = at(x, y, z);
                                if ga.fl[k] & F_DETAIL != 0 {
                                    l.cells += 1;
                                    if (ga.fl[k] ^ gb.fl[k]) & F_OPAQUE != 0 {
                                        l.mismatch += 1;
                                    }
                                }
                            }
                        }
                    }
                    grids.push(ga);
                }
                for c in cells {
                    let local = [c[0] - o[0], c[1] - o[1], c[2] - o[2]];
                    let k = at(local[0], local[1], local[2]);
                    let mut geoms: Vec<HashMap<&'static str, (u32, Option<[i32; 3]>)>> = Vec::new();
                    let mut key: Option<(String, u64)> = None;
                    for (vi, ga) in grids.iter().enumerate() {
                        let Some((cname, q, feats)) = crate::shapes::features_at(&pk, ga, k) else {
                            break;
                        };
                        let bits = feats
                            .iter()
                            .enumerate()
                            .fold(0u64, |a, (i, f)| a | (u64::from(*f) << i));
                        if vi == 0 {
                            key = Some((cname.clone(), bits));
                        }
                        let labels = labels_of(s.name, &cname);
                        let light = &per_var[vi].1;
                        geoms.push(bake_cell(&mut l, ga, light, k, c, &cname, q, &feats, &labels));
                    }
                    let Some((cname, bits)) = key else { continue };
                    if geoms.len() != n_var {
                        continue;
                    }
                    *l.classes.entry(cname.clone()).or_default() += 1;
                    for label in labels_of(s.name, &cname) {
                        let tuple: Vec<u32> = geoms.iter().map(|g| g[label].0).collect();
                        let glow = geoms[0][label].1;
                        *l.seen
                            .entry(cname.clone())
                            .or_default()
                            .entry(label.to_string())
                            .or_default()
                            .entry(bits)
                            .or_default()
                            .entry((tuple, glow))
                            .or_default() += 1;
                    }
                }
            }
        }
        // 3. tables: for each part, the fewest features that decide it
        let mut models: Vec<String> = Vec::new();
        let mut model_of: HashMap<(Vec<u32>, String), String> = HashMap::new();
        let mut classes_json = Vec::new();
        let mut conflicts = 0;
        for c in &s.classes {
            let cname = c.rule["class"].as_str().unwrap().to_string();
            let mut r = c.rule.clone();
            let Some(parts) = l.seen.get(&cname) else {
                classes_json.push(r);
                continue;
            };
            let mut parts_json = Vec::new();
            for (label, by_bits) in parts {
                // the commonest outcome of each pattern
                let mut best: BTreeMap<u64, (Vec<u32>, Option<[i32; 3]>)> = BTreeMap::new();
                for (bits, outs) in by_bits {
                    if outs.len() > 1 {
                        conflicts += 1;
                    }
                    let (o, _) = outs
                        .iter()
                        .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
                        .unwrap();
                    best.insert(*bits, o.clone());
                }
                if best.values().all(|(t, _)| t.iter().all(|g| *g == NONE_GEOM)) {
                    continue;
                }
                let nf = c.features.len();
                let mut keep: Vec<usize> = (0..nf).collect();
                for f in 0..nf {
                    let trial: Vec<usize> = keep.iter().copied().filter(|k| *k != f).collect();
                    let mut m: HashMap<u64, &(Vec<u32>, Option<[i32; 3]>)> = HashMap::new();
                    let ok = best.iter().all(|(bits, o)| {
                        let key = trial
                            .iter()
                            .enumerate()
                            .fold(0u64, |a, (i, k)| a | ((bits >> k & 1) << i));
                        *m.entry(key).or_insert(o) == o
                    });
                    if ok {
                        keep = trial;
                    }
                }
                let mut table = serde_json::Map::new();
                for (bits, (tuple, glow)) in &best {
                    let key = keep
                        .iter()
                        .enumerate()
                        .fold(0u32, |a, (i, k)| a | (((bits >> k & 1) as u32) << i));
                    let key = crate::shapes::bits_key(key, keep.len());
                    if tuple.iter().all(|g| *g == NONE_GEOM) {
                        table.insert(key, Value::Null);
                        continue;
                    }
                    let name = model_of
                        .entry((tuple.clone(), label.clone()))
                        .or_insert_with(|| {
                            let n = format!("{cname}.{label}.{}", models.len());
                            models.push(n.clone());
                            n
                        })
                        .clone();
                    let mut e = json!({"model": name});
                    if let Some(gp) = glow {
                        e["glow"] = json!(gp.map(|v| v as f32 / 16.0));
                    }
                    table.insert(key, e);
                }
                parts_json.push(json!({
                    "name": label,
                    "features": keep.iter().map(|k| c.features[*k].clone()).collect::<Vec<_>>(),
                    "table": table,
                }));
            }
            r["parts"] = Value::Array(parts_json);
            classes_json.push(r);
        }
        // 4. the models, one file per variant, and their palettes
        let mut faces_of: HashMap<u32, [u8; 3]> = HashMap::new();
        for m in 0..l.mats.len() as u32 {
            let pick = |nc: u8| {
                l.faces
                    .get(&(m, nc))
                    .and_then(|f| f.iter().max_by(|a, b| a.1.total_cmp(b.1)).map(|x| *x.0))
            };
            let side = pick(1).or(pick(0)).or(pick(2)).unwrap_or(1);
            faces_of.insert(m, [pick(0).unwrap_or(side), side, pick(2).unwrap_or(side)]);
        }
        let mut used: Vec<u32> = Vec::new();
        for ((tuple, _), _) in &model_of {
            for g in tuple {
                if *g != NONE_GEOM {
                    for (_, m) in &l.geoms[*g as usize] {
                        if !used.contains(m) {
                            used.push(*m);
                        }
                    }
                }
            }
        }
        used.sort();
        assert!(used.len() < 255, "{}: {} materials", s.name, used.len());
        let pal_index: HashMap<u32, u8> = used.iter().enumerate().map(|(i, m)| (*m, i as u8 + 1)).collect();
        for (vi, (_, stem, _)) in s.variants.iter().enumerate() {
            let mut file = crate::vox::File::default();
            let mut pal = serde_json::Map::new();
            for (m, i) in &pal_index {
                let k = &l.mats[*m as usize];
                let tint = k.tint.map(|v| v as f32 / 1000.0);
                let faces = faces_of[m];
                let mut e = if let Some(d) = k.src.strip_prefix("frame:") {
                    let d: Vec<i32> = d.split(',').map(|v| v.parse().unwrap()).collect();
                    json!({"at": d, "faces": [1, 1, 1],
                           "if": all(vec![at_([d[0], d[1], d[2]], "wall"), not(at_([d[0], d[1], d[2]], "pane")), not(at_([d[0], d[1], d[2]], "glass"))]),
                           "else": {"block": name_of(w::TIMBER_BEAM), "faces": [1, 1, 1], "tint": tint}})
                } else if let Some(d) = k.src.strip_prefix("at:") {
                    let d: Vec<i32> = d.split(',').map(|v| v.parse().unwrap()).collect();
                    json!({"at": d, "faces": faces})
                } else if let Some(b) = k.src.strip_prefix("block:") {
                    json!({"block": b, "faces": faces})
                } else {
                    json!({"block": "self", "faces": faces})
                };
                e["tint"] = json!(tint);
                if k.glow {
                    e["glow"] = json!(true);
                }
                if k.clear {
                    e["clear"] = json!(true);
                }
                pal.insert(i.to_string(), e);
                // a colour for the editor: the texture's tint over grey
                file.palette[*i as usize] = [
                    (tint[0] * 180.0) as u8,
                    (tint[1] * 180.0) as u8,
                    (tint[2] * 180.0) as u8,
                    255,
                ];
            }
            let mut names: Vec<(&String, &Vec<u32>)> =
                model_of.iter().map(|((t, _), n)| (n, t)).collect();
            names.sort();
            for (name, tuple) in names {
                let g = tuple[vi];
                let voxels = if g == NONE_GEOM {
                    Vec::new()
                } else {
                    l.geoms[g as usize].iter().map(|(p, m)| (*p, pal_index[m])).collect()
                };
                file.models.push(crate::vox::Model {
                    name: name.clone(),
                    voxels,
                });
            }
            let bytes = crate::vox::write(&file).expect("vox");
            std::fs::write(root.join(format!("shapes/{stem}.vox")), bytes).unwrap();
            palettes.insert(format!("shapes/{stem}.vox"), json!({"palette": pal}));
        }
        println!(
            "{}: {:?}, {} models, {} materials, {} conflicting patterns, {}/{} fill mismatches",
            s.name,
            l.classes,
            models.len(),
            used.len(),
            conflicts,
            l.mismatch,
            l.cells
        );
        tables.insert(s.name.to_string(), json!({"models": models, "classes": classes_json}));
    }
    let out = json!({
        "blocks": blocks,
        "shape_rules": rules_stage(Some(&tables)),
        "shape_files": palettes,
    });
    std::fs::write(root.join("blocks.json"), serde_json::to_string_pretty(&out).unwrap() + "\n").unwrap();
}
