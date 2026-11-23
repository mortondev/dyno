//! Character, NPC, creature and held-item models: fine voxel figures, built
//! in code, meshed once per look and posed part by part each frame.

use crate::anim::{self, Anim, Animator, BipedRig, Grip, Pose, Skeleton};
use crate::render::{Vertex, push_block_cube};
pub use crate::voxmodel::set_view;
use crate::voxmodel::{
    Cache, Ink, Mesh, Paint, Vox, emit, hash3, in_rbox, joint, key_of, mix, scale, view_distance,
};
use dyno_world as w;
use dyno_world::content::{ArmorType, WeaponKind};
use glam::{Affine3A, Mat3, Vec3};
use std::rc::Rc;

/// Voxels per block for figures.
const FINE: f32 = 32.0;

/// How a person looks: colours and pieces of an outfit.
#[derive(Clone, Copy, Debug)]
pub struct Look {
    pub skin: [f32; 3],
    pub hair: [f32; 3],
    /// 0 bald, 1 short, 2 long, 3 hood, 4 tied up in a bun
    pub hair_style: u8,
    pub beard: bool,
    /// main garment (tabard or robe)
    pub tunic: [f32; 3],
    /// trim, belt buckle and embroidery
    pub trim: [f32; 3],
    pub pants: [f32; 3],
    pub boots: [f32; 3],
    /// metal shoulder guards
    pub pauldrons: bool,
    /// a long robe instead of trousers
    pub robe: bool,
    pub apron: Option<[f32; 3]>,
    pub cape: Option<[f32; 3]>,
    /// what the outfit is made of
    pub armor: ArmorType,
    /// an open-faced helm over the hair
    pub helmet: bool,
    /// the weapon in the right hand, and its colour
    pub weapon: Option<(WeaponKind, [f32; 3])>,
    pub eyes: [f32; 3],
    /// 0 slight, 1 average, 2 burly
    pub build: u8,
    /// old: a long beard and bushy brows
    pub elder: bool,
    pub gloves: bool,
    /// chest emblem: 0 none, 1 diamond, 2 cross, 3 chevrons, 4 flame
    pub emblem: u8,
    /// 0 hidden, 1 round, 2 pointed, 3 goblin-kin (great ears, long nose)
    pub ears: u8,
    /// the set of the face: 0 cheerful, 1 stern, 2 calm
    pub mood: u8,
    /// a scarf drawn up over the nose and mouth
    pub scarf: Option<[f32; 3]>,
    /// a sash at the waist, knotted at the hip
    pub sash: Option<[f32; 3]>,
    /// a lit candle stuck to the top of the head
    pub candle: bool,
}

fn hash(n: u32) -> u32 {
    let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^ (x >> 12)
}

impl Look {
    /// The palettes the character creator offers.
    pub const SKINS: [[f32; 3]; 8] = [
        [0.98, 0.86, 0.75],
        [0.96, 0.80, 0.66],
        [0.87, 0.68, 0.52],
        [0.78, 0.58, 0.42],
        [0.72, 0.52, 0.38],
        [0.60, 0.42, 0.30],
        [0.48, 0.33, 0.23],
        [0.38, 0.26, 0.18],
    ];
    pub const HAIRS: [[f32; 3]; 10] = [
        [0.12, 0.12, 0.14],
        [0.18, 0.12, 0.08],
        [0.30, 0.19, 0.10],
        [0.45, 0.28, 0.12],
        [0.62, 0.22, 0.10],
        [0.78, 0.40, 0.15],
        [0.85, 0.68, 0.35],
        [0.92, 0.84, 0.62],
        [0.85, 0.85, 0.82],
        [0.45, 0.48, 0.55],
    ];
    pub const EYES: [[f32; 3]; 6] = [
        [0.25, 0.42, 0.70],
        [0.30, 0.52, 0.30],
        [0.42, 0.28, 0.16],
        [0.45, 0.48, 0.55],
        [0.55, 0.40, 0.70],
        [0.70, 0.55, 0.20],
    ];

    /// A player's look from the appearance they chose, dressed in their class colour.
    pub fn from_appearance(a: &dyno_protocol::Appearance, color: [f32; 3]) -> Look {
        let pick = |v: &[[f32; 3]], i: u8| v[i as usize % v.len()];
        let mut l = Look::player(0, color);
        l.skin = pick(&Self::SKINS, a.skin);
        l.hair = pick(&Self::HAIRS, a.hair_color);
        l.eyes = pick(&Self::EYES, a.eyes);
        l.hair_style = a.hair_style.min(4);
        l.beard = a.beard;
        l.build = a.build.min(2);
        l.mood = a.mood.min(2);
        l.ears = a.ears.min(2);
        l
    }

    /// A traveller's look from a seed and a signature colour.
    pub fn player(seed: u32, color: [f32; 3]) -> Look {
        let h = hash(seed);
        let skins = [
            [0.96, 0.80, 0.66],
            [0.87, 0.68, 0.52],
            [0.72, 0.52, 0.38],
            [0.55, 0.38, 0.27],
            [0.98, 0.86, 0.75],
        ];
        let hairs = [
            [0.18, 0.12, 0.08],
            [0.45, 0.28, 0.12],
            [0.85, 0.68, 0.35],
            [0.62, 0.22, 0.10],
            [0.12, 0.12, 0.14],
            [0.85, 0.85, 0.82],
        ];
        let eyes = [
            [0.25, 0.42, 0.70],
            [0.30, 0.52, 0.30],
            [0.42, 0.28, 0.16],
            [0.45, 0.48, 0.55],
        ];
        let pauldrons = (h >> 10).is_multiple_of(2);
        let armor = match (h >> 12) % 4 {
            0 => ArmorType::Cloth,
            1 => ArmorType::Leather,
            2 => ArmorType::Mail,
            _ => ArmorType::Plate,
        };
        Look {
            skin: skins[(h % 5) as usize],
            hair: hairs[((h >> 3) % 6) as usize],
            hair_style: 1 + ((h >> 6) % 2) as u8,
            beard: (h >> 8).is_multiple_of(3),
            tunic: color,
            trim: [0.85, 0.68, 0.25],
            pants: [0.30, 0.24, 0.20],
            boots: [0.22, 0.15, 0.10],
            pauldrons,
            robe: false,
            apron: None,
            cape: Some([color[0] * 0.6, color[1] * 0.6, color[2] * 0.6]),
            armor,
            helmet: false,
            weapon: None,
            eyes: eyes[((h >> 14) % 4) as usize],
            build: 1,
            elder: false,
            gloves: armor != ArmorType::Cloth,
            emblem: 1 + ((h >> 16) % 4) as u8,
            ears: if (h >> 18).is_multiple_of(5) { 2 } else { 1 },
            mood: ((h >> 21) % 3) as u8,
            scarf: None,
            sash: None,
            candle: false,
        }
    }

    /// The look of a named townsperson; others get one from their colour.
    pub fn npc(key: &str, color: [f32; 3]) -> Look {
        let mut l = Look::player(key.bytes().map(|b| b as u32).sum(), color);
        l.cape = None;
        l.ears = 1;
        // townsfolk dress plainly
        if matches!(l.armor, ArmorType::Mail | ArmorType::Plate) {
            l.armor = ArmorType::Leather;
        }
        match key {
            "brannoc" => {
                l.skin = [0.92, 0.76, 0.62];
                l.hair = [0.90, 0.90, 0.88];
                l.hair_style = 2;
                l.beard = true;
                l.elder = true;
                l.robe = true;
                l.armor = ArmorType::Cloth;
                l.gloves = false;
                l.tunic = [0.45, 0.25, 0.18];
                l.trim = [0.90, 0.70, 0.30];
                l.pauldrons = false;
                l.eyes = [0.35, 0.50, 0.70];
                l.emblem = 0;
                l.build = 0;
                l.mood = 2;
            }
            "maren" => {
                l.skin = [0.96, 0.80, 0.66];
                l.hair = [0.68, 0.22, 0.10];
                l.hair_style = 2;
                l.beard = false;
                l.tunic = [0.20, 0.30, 0.62];
                l.trim = [0.85, 0.75, 0.40];
                l.pauldrons = true;
                l.armor = ArmorType::Mail;
                l.gloves = true;
                l.cape = Some([0.12, 0.18, 0.42]);
                l.eyes = [0.28, 0.55, 0.36];
                l.emblem = 4;
                l.weapon = Some((WeaponKind::Sword, [0.72, 0.72, 0.78]));
                l.mood = 1;
            }
            "tilda" => {
                l.skin = [0.97, 0.82, 0.70];
                l.hair = [0.45, 0.28, 0.12];
                l.hair_style = 4;
                l.beard = false;
                l.tunic = [0.62, 0.38, 0.22];
                l.trim = [0.80, 0.30, 0.25];
                l.pauldrons = false;
                l.armor = ArmorType::Cloth;
                l.gloves = false;
                l.robe = true;
                l.apron = Some([0.92, 0.90, 0.82]);
                l.eyes = [0.42, 0.28, 0.16];
                l.emblem = 0;
                l.build = 0;
                l.mood = 0;
            }
            "hollis" => {
                l.skin = [0.72, 0.52, 0.38];
                l.hair_style = 0;
                l.beard = true;
                l.hair = [0.20, 0.14, 0.10];
                l.tunic = [0.40, 0.36, 0.34];
                l.pauldrons = false;
                l.armor = ArmorType::Leather;
                l.gloves = true;
                l.apron = Some([0.35, 0.24, 0.14]);
                l.eyes = [0.30, 0.22, 0.14];
                l.emblem = 0;
                l.build = 2;
                l.mood = 1;
            }
            _ => {}
        }
        l
    }

    fn key(&self) -> u64 {
        let f = |c: [f32; 3]| [c[0].to_bits(), c[1].to_bits(), c[2].to_bits()];
        let o = |c: Option<[f32; 3]>| c.map(f).unwrap_or([1, 2, 3]);
        let colors = [
            f(self.skin),
            f(self.hair),
            f(self.tunic),
            f(self.trim),
            f(self.pants),
            f(self.boots),
            f(self.eyes),
            o(self.apron),
            o(self.cape),
            o(self.scarf),
            o(self.sash),
        ];
        let mut k = [0u32; 46];
        for (slot, c) in k.iter_mut().zip(colors.iter().flatten()) {
            *slot = *c;
        }
        k[33..].copy_from_slice(&[
            self.hair_style as u32,
            self.beard as u32,
            self.pauldrons as u32,
            self.robe as u32,
            self.armor as u32,
            self.helmet as u32,
            self.build as u32,
            self.elder as u32,
            self.gloves as u32,
            self.emblem as u32,
            self.ears as u32,
            self.mood as u32,
            self.candle as u32,
        ]);
        key_of(&k)
    }
}

// ---------------------------------------------------------------- people

const STEEL: [f32; 3] = [0.66, 0.68, 0.74];
const LEATHER: [f32; 3] = [0.36, 0.24, 0.14];

const P_HEAD: u8 = 1;
const P_TORSO: u8 = 2;
const P_FLAP_F: u8 = 3;
const P_FLAP_B: u8 = 4;
const P_SKIRT: u8 = 5;
const P_ARM_U: [u8; 2] = [6, 7];
const P_ARM_L: [u8; 2] = [8, 9];
const P_THIGH: [u8; 2] = [10, 11];
const P_SHIN: [u8; 2] = [12, 13];
const P_CAPE: [u8; 3] = [14, 15, 16];

/// Where the parts hinge, in voxels from the feet.
/// Heads are modelled at 16 voxels for a readable face, then drawn at 55%:
/// about 0.27 m, life size on a 1.78 m adult (scale review, 1 block = 1 m).
const HEAD_SCALE: f32 = 0.55;
const HIP_Y: f32 = 26.0;
const KNEE_Y: f32 = 13.0;
const LEG_X: f32 = 5.0;
const WAIST_Y: f32 = 26.0;
const NECK_Y: f32 = 48.0;
const SHOULDER_Y: f32 = 45.0;
const ELBOW_Y: f32 = 34.0;
const FLAP_Y: f32 = 28.0;
const CAPE_Y: [f32; 3] = [47.0, 34.0, 21.0];
const CAPE_Z: f32 = -6.5;

struct Person {
    head: Mesh,
    torso: Mesh,
    flap_f: Mesh,
    flap_b: Mesh,
    skirt: Mesh,
    arm_u: [Mesh; 2],
    arm_l: [Mesh; 2],
    thigh: [Mesh; 2],
    shin: [Mesh; 2],
    cape: [Mesh; 3],
    /// arm pivot distance from the centre (wider for the burly)
    arm_x: f32,
}

#[cfg(test)]
impl Person {
    fn quads(&self) -> usize {
        self.head.quads()
            + self.torso.quads()
            + self.flap_f.quads()
            + self.flap_b.quads()
            + self.skirt.quads()
            + self
                .arm_u
                .iter()
                .chain(&self.arm_l)
                .chain(&self.thigh)
                .chain(&self.shin)
                .chain(&self.cape)
                .map(Mesh::quads)
                .sum::<usize>()
    }
}

/// `a..b` on the right side, or its mirror image on the left.
fn mx(s: i32, a: i32, b: i32) -> [i32; 2] {
    if s > 0 { [a, b] } else { [-b, -a] }
}

/// A voxel column on the right side, or its mirror on the left.
fn mi(s: i32, x: i32) -> i32 {
    if s > 0 { x } else { -x - 1 }
}

const EMBLEMS: [[&str; 7]; 4] = [
    [
        "..##..", ".####.", "######", "######", ".####.", "..##..", "......",
    ],
    [
        "..##..", "..##..", "######", "######", "..##..", "..##..", "..##..",
    ],
    [
        "##..##", ".####.", "..##..", "......", "##..##", ".####.", "..##..",
    ],
    [
        "..#...", "..##..", ".###..", ".####.", "######", "##.###", ".####.",
    ],
];

/// The two tones of chain mail, by row.
fn mail(v: &mut Vox, y: i32) -> Ink {
    if y.rem_euclid(2) == 0 {
        v.ink([0.56, 0.58, 0.63])
    } else {
        v.ink([0.40, 0.42, 0.47])
    }
}

fn build_head(v: &mut Vox, l: &Look) {
    v.part = P_HEAD;
    let skin = v.ink(l.skin);
    let skin_d = v.tone(l.skin, 0.86);
    let lo = [-8, 48, -7];
    let hi = [8, 64, 8];
    v.rbox([-8, 8], [48, 64], [-7, 8], 2, [true; 3], skin);
    // a little jaw shading under the cheeks
    v.repaint([-8, 8], [48, 50], [-7, 8], |_, _, _, _| skin_d);
    // eyes: a dark lash line over white, with a pupil above a coloured iris
    let white = v.ink([0.97, 0.97, 0.95]);
    let pupil = v.ink([0.07, 0.06, 0.09]);
    let iris = v.ink(l.eyes);
    let lash = v.ink(scale(mix(l.skin, [0.1, 0.05, 0.05], 0.75), 1.0));
    let brow_rgb = if l.hair_style == 0 && !l.beard {
        scale(l.skin, 0.6)
    } else if l.elder {
        l.hair
    } else {
        scale(l.hair, 0.72)
    };
    let brow = v.ink(brow_rgb);
    for s in [1, -1] {
        for x in 2..5 {
            v.dye(mi(s, x), 57, 7, lash);
        }
        v.dye(mi(s, 2), 56, 7, white);
        v.dye(mi(s, 3), 56, 7, pupil);
        v.dye(mi(s, 4), 56, 7, white);
        v.dye(mi(s, 2), 55, 7, white);
        v.dye(mi(s, 3), 55, 7, iris);
        v.dye(mi(s, 4), 55, 7, white);
        // brows; bushy on the old
        match l.mood {
            1 => {
                // stern: drawn down towards the nose
                v.dye(mi(s, 1), 58, 7, brow);
                for x in 2..6 {
                    v.dye(mi(s, x), 59, 7, brow);
                }
            }
            2 => {
                // calm: level
                for x in 2..6 {
                    v.dye(mi(s, x), 59, 7, brow);
                }
            }
            _ => {
                // cheerful: arched, with a lash at the corner of the eye
                for x in 1..5 {
                    v.dye(mi(s, x), 59, 7, brow);
                }
                v.dye(mi(s, 5), 58, 7, brow);
                v.dye(mi(s, 5), 57, 7, lash);
            }
        }
        if l.elder {
            for x in 2..6 {
                v.set(mi(s, x), 59, 8, brow);
            }
        }
        // cheeks
        let blush = v.ink(mix(l.skin, [0.95, 0.45, 0.40], 0.28));
        v.dye(mi(s, 4), 53, 7, blush);
        v.dye(mi(s, 5), 53, 7, blush);
        // ears
        if (l.ears == 2 || !matches!(l.hair_style, 2 | 4)) && l.ears > 0 && l.hair_style != 3 {
            v.fill(mx(s, 8, 9), [53, 57], [-1, 2], skin_d);
            if l.ears == 2 {
                v.fill(mx(s, 8, 9), [57, 59], [-2, 1], skin_d);
                v.fill(mx(s, 9, 10), [58, 61], [-3, -1], skin_d);
            }
        }
    }
    // nose and mouth
    let nose = v.tone(l.skin, 0.93);
    v.fill([-1, 1], [53, 55], [8, 9], nose);
    let mouth = v.ink(mix(scale(l.skin, 0.55), [0.55, 0.15, 0.15], 0.5));
    match l.mood {
        1 => {
            // a firm line
            for x in -2..2 {
                v.dye(x, 51, 7, mouth);
            }
        }
        2 => {
            // a small smile
            for x in -1..1 {
                v.dye(x, 51, 7, mouth);
            }
            v.dye(-2, 52, 7, mouth);
            v.dye(1, 52, 7, mouth);
        }
        _ => {
            // a broad smile
            for x in -2..2 {
                v.dye(x, 51, 7, mouth);
            }
            v.dye(-3, 52, 7, mouth);
            v.dye(2, 52, 7, mouth);
        }
    }
    if l.ears == 3 {
        // goblin-kin: great ears, a long nose, wide eyes and a snaggle tooth
        let inner = v.ink(mix(l.skin, [0.85, 0.50, 0.45], 0.35));
        for s in [1, -1] {
            v.fill(mx(s, 9, 12), [54, 58], [-2, 1], skin_d);
            v.fill(mx(s, 11, 14), [56, 60], [-2, 0], skin_d);
            v.fill(mx(s, 13, 16), [58, 62], [-2, -1], skin_d);
            v.fill(mx(s, 9, 13), [55, 58], [0, 1], inner);
            v.dye(mi(s, 2), 54, 7, white);
            v.dye(mi(s, 3), 54, 7, iris);
            v.dye(mi(s, 4), 54, 7, white);
            v.dye(mi(s, 3), 55, 7, pupil);
        }
        v.fill([-1, 1], [51, 55], [8, 11], nose);
        v.fill([-1, 1], [51, 53], [11, 12], skin_d);
        v.dye(-2, 50, 7, white);
    }
    if let Some(scarf) = l.scarf {
        let c = v.ink(scarf);
        let c_d = v.tone(scarf, 0.78);
        v.repaint([-9, 9], [48, 55], [-1, 10], |_, y, _, _| {
            if y % 3 == 0 { c_d } else { c }
        });
        // pulled up over the nose
        v.fill([-2, 2], [53, 56], [8, 9], c);
    }

    // hair, as shells around the skull
    let hair = v.ink(l.hair);
    let hair_lo = v.tone(l.hair, 0.78);
    let hair_hi = v.ink(mix(scale(l.hair, 1.25), [1.0, 1.0, 1.0], 0.12));
    let strand = |x: i32, y: i32, z: i32| -> Ink {
        // strands run front to back on top and down the sides and back
        let side = !(-7..7).contains(&x);
        let k = if side { z } else { x };
        let shine = (y == 61 && (side || z < -5) || (y >= 63 && (2..4).contains(&z)))
            && k.rem_euclid(4) != 0;
        if shine {
            hair_hi
        } else if k.rem_euclid(3) == 0 || (y >= 63 && (-1..1).contains(&x)) {
            hair_lo
        } else {
            hair
        }
    };
    let shell = |p: [i32; 3], n: i32| -> bool {
        let lo = [lo[0] - n, lo[1] - n, lo[2] - n];
        let hi = [hi[0] + n, hi[1] + n, hi[2] + n];
        in_rbox(p, lo, hi, 2 + n, [true; 3])
    };
    let fringe = |x: i32| 60 + [1, 0, 0, 1, 0, 2, 1, 0][x.rem_euclid(8) as usize];
    // where the scalp is covered
    let scalp = |x: i32, y: i32, z: i32, style: u8| -> bool {
        let long = style == 2 || style == 4;
        if z >= 6 {
            y >= fringe(x) + i32::from(style == 4)
        } else if z >= 2 {
            y >= if long { 50 } else { 58 } && (y >= 58 || !(-8..8).contains(&x))
        } else if z >= -3 {
            y >= if long { 49 } else { 57 }
        } else {
            y >= if long { 47 } else { 51 }
        }
    };
    if matches!(l.hair_style, 1 | 2 | 4) {
        for y in 46..66 {
            for z in -9..10 {
                for x in -10..10 {
                    if shell([x, y, z], 1) && !shell([x, y, z], 0) && scalp(x, y, z, l.hair_style) {
                        v.add(x, y, z, strand(x, y, z));
                    }
                }
            }
        }
        if !l.helmet {
            // tufts that break the outline
            for (x, z, h) in [(-3, 1, 1), (2, -3, 1), (4, 3, 1), (-5, -4, 1), (0, -6, 1)] {
                for y in 65..65 + h {
                    v.add(x, y, z, hair);
                    v.add(x + 1, y, z, hair_lo);
                }
            }
        }
    }
    match l.hair_style {
        2 => {
            // long: a fall of hair down the back, ragged at the ends
            for x in -8i32..8 {
                let end = 38 + [0, 1, 0, 2, 1, 0, 1, 3][x.rem_euclid(8) as usize] + (x.abs() / 6);
                for y in end..50 {
                    for z in -10..-8 {
                        if (x == -8 || x == 7) && z == -10 {
                            continue;
                        }
                        let c = if y == 44 && x.rem_euclid(4) != 0 {
                            hair_hi
                        } else if x.rem_euclid(3) == 0 {
                            hair_lo
                        } else {
                            hair
                        };
                        v.add(x, y, z, c);
                    }
                }
            }
            // locks in front of the shoulders
            for s in [1, -1] {
                for y in 44..50 {
                    v.add(mi(s, 8), y, -1, hair);
                    v.add(mi(s, 8), y, 0, hair_lo);
                    if y > 45 {
                        v.add(mi(s, 8), y, 1, hair);
                    }
                }
            }
        }
        4 => {
            // tied up: a big bun high on the back of the head, with a ribbon
            let ribbon = v.ink(l.trim);
            let ribbon_d = v.tone(l.trim, 0.75);
            v.blob([0.0, 64.5, -6.0], [4.6, 4.2, 4.4], hair);
            v.repaint([-6, 6], [59, 70], [-12, 0], |x, y, z, c| {
                if y >= 65 || z < -7 {
                    strand(x + 1, 0, z + y)
                } else {
                    c
                }
            });
            v.fill([-5, 5], [61, 62], [-10, -2], ribbon);
            v.fill([-5, 5], [61, 62], [-3, -2], ribbon_d);
            // the bow's tails
            v.fill([-3, -1], [57, 61], [-9, -8], ribbon);
            v.fill([1, 3], [58, 61], [-9, -8], ribbon);
            // a few loose strands by the cheeks
            for s in [1, -1] {
                v.add(mi(s, 8), 48, 1, hair_lo);
                v.add(mi(s, 8), 47, 1, hair);
            }
        }
        3 => {
            // a deep hood in the garment's colour, trimmed at the opening
            let cloth = v.ink(l.tunic);
            let cloth_d = v.tone(l.tunic, 0.8);
            let trim = v.ink(l.trim);
            for y in 46..68 {
                for z in -10..11 {
                    for x in -11..11 {
                        let p = [x, y, z];
                        let face = z >= 6 && (49..61).contains(&y) && (-6..6).contains(&x);
                        if shell(p, 2) && !shell(p, 0) && !face && (y >= 49 || z < 3) {
                            let c = if z >= 8 {
                                trim
                            } else if x.rem_euclid(5) == 0 && z < 6 {
                                cloth_d
                            } else {
                                cloth
                            };
                            v.add(x, y, z, c);
                        }
                    }
                }
            }
            // the point of the hood, drooping back
            v.fill([-2, 2], [64, 67], [-11, -8], cloth);
            v.fill([-1, 1], [62, 65], [-13, -11], cloth_d);
        }
        _ => {}
    }
    if l.beard {
        let b = v.ink(l.hair);
        let b_lo = v.tone(l.hair, 0.8);
        let tone = |x: i32| if x.rem_euclid(3) == 0 { b_lo } else { b };
        for y in 46..56 {
            for z in 0..10 {
                for x in -10..10 {
                    let p = [x, y, z];
                    if !shell(p, 1) || shell(p, 0) {
                        continue;
                    }
                    let side = !(-8..8).contains(&x);
                    let on = if side {
                        y <= 54 && z >= 2
                    } else if z >= 8 {
                        // leave the mouth showing under a moustache
                        y <= 50
                            || (y == 52 && (-4..4).contains(&x))
                            || (y == 51 && !(-2..2).contains(&x))
                    } else {
                        y <= 52
                    };
                    if on {
                        v.add(x, y, z, tone(x));
                    }
                }
            }
        }
        // a thicker chin
        v.fill([-4, 4], [46, 50], [9, 10], b);
        v.fill([-3, 3], [45, 47], [5, 9], b_lo);
        if l.elder {
            // a long beard, tapering down the chest
            for y in 35..48 {
                let half = 1 + (y - 35) * 4 / 12;
                for x in -half..half {
                    let front = if y > 44 { 10 } else { 9 };
                    for z in 6..front {
                        v.add(x, y, z, tone(x + y / 5));
                    }
                }
            }
            // drooping moustache ends
            for s in [1, -1] {
                v.fill(mx(s, 3, 5), [48, 52], [9, 10], b);
            }
        }
    }
    if l.candle {
        // a stub of candle in a pool of its own wax, burning
        let wax = v.ink([0.96, 0.93, 0.80]);
        let wax_d = v.ink([0.84, 0.79, 0.62]);
        let wick = v.ink([0.10, 0.08, 0.07]);
        let flame = v.glow([1.0, 0.62, 0.12]);
        let flame_hi = v.glow([1.0, 0.95, 0.62]);
        let top = if l.hair_style == 0 { 64 } else { 66 };
        v.fill([-3, 3], [top, top + 1], [-3, 3], wax_d);
        v.fill([-2, 2], [top, top + 5], [-2, 2], wax);
        v.fill([-2, 0], [top + 5, top + 6], [-2, 1], wax);
        for (x, z, len) in [(3, 1, 3), (-4, -2, 2), (0, 3, 4), (-2, -4, 2), (4, -1, 2)] {
            for y in top - len..top {
                v.set(x, y, z, wax);
            }
        }
        v.fill([-1, 0], [top + 5, top + 7], [-1, 0], wick);
        v.fill([-2, 1], [top + 7, top + 10], [-2, 1], flame);
        v.fill([-1, 0], [top + 7, top + 12], [-1, 0], flame_hi);
        v.fill([-1, 0], [top + 12, top + 13], [-1, 0], flame);
    }
    if l.helmet {
        // an open helm: a steel cap with a brow band, nasal bar and crest
        let steel = v.ink(STEEL);
        let steel_hi = v.tone(STEEL, 1.2);
        let steel_d = v.tone(STEEL, 0.75);
        let trim = v.ink(l.trim);
        for y in 52..67 {
            for z in -9..10 {
                for x in -10..10 {
                    let p = [x, y, z];
                    if !shell(p, 1) || shell(p, 0) {
                        continue;
                    }
                    let cheek = !(-8..8).contains(&x) && (-4..5).contains(&z) && y >= 52;
                    let back = z < -3 && y >= 53;
                    if y >= 58 || cheek || back {
                        let c = if y == 58 {
                            trim
                        } else if y >= 64 {
                            steel_hi
                        } else if (y + z).rem_euclid(7) == 0 && y < 58 {
                            steel_d
                        } else {
                            steel
                        };
                        v.set(x, y, z, c);
                    }
                }
            }
        }
        v.fill([-1, 1], [54, 59], [8, 9], steel);
        v.fill([-1, 1], [65, 67], [-7, 7], trim);
        v.fill([-1, 1], [60, 66], [8, 9], trim);
        for s in [1, -1] {
            v.set(mi(s, 5), 60, 8, steel_hi);
            v.set(mi(s, 8), 60, 3, steel_hi);
        }
    }
}

fn build_legs(v: &mut Vox, l: &Look) {
    let pants = v.ink(l.pants);
    let pants_d = v.tone(l.pants, 0.82);
    let boot = v.ink(l.boots);
    let boot_hi = v.tone(l.boots, 1.45);
    let boot_d = v.tone(l.boots, 0.65);
    let trim = v.ink(l.trim);
    let steel = v.ink(STEEL);
    let steel_hi = v.tone(STEEL, 1.2);
    let steel_d = v.tone(STEEL, 0.72);
    for (i, s) in [1, -1].into_iter().enumerate() {
        if !l.robe {
            v.part = P_THIGH[i];
            v.rbox(
                mx(s, 1, 9),
                [12, 27],
                [-4, 5],
                1,
                [false, false, false],
                pants,
            );
            match l.armor {
                ArmorType::Plate => {
                    // cuisses over the thighs
                    v.fill(mx(s, 1, 10), [15, 24], [1, 6], steel);
                    v.fill(mx(s, 2, 9), [23, 24], [5, 6], steel_hi);
                    v.fill(mx(s, 1, 10), [15, 16], [1, 6], steel_d);
                }
                ArmorType::Leather => {
                    // a patch and a strap
                    let hide = v.ink(LEATHER);
                    v.fill(mx(s, 1, 10), [17, 19], [-4, 5], hide);
                    v.set(mi(s, 9), 18, 2, trim);
                }
                ArmorType::Cloth | ArmorType::Mail => {}
            }
            // a seam down the outside
            v.repaint(mx(s, 8, 9), [12, 27], [0, 1], |_, _, _, _| pants_d);
        }
        v.part = P_SHIN[i];
        if !l.robe {
            v.fill(mx(s, 1, 9), [9, 14], [-4, 5], pants);
            v.fill(mx(s, 2, 8), [12, 14], [5, 6], pants_d);
        }
        // boot: shaft, foot with a toe cap, sole and a turned-down cuff
        v.rbox(mx(s, 0, 10), [1, 11], [-5, 6], 1, [true, false, true], boot);
        v.rbox(mx(s, 0, 10), [1, 5], [-5, 9], 1, [true, false, true], boot);
        v.fill(mx(s, 1, 9), [5, 6], [6, 8], boot);
        v.rbox(
            mx(s, 0, 10),
            [0, 1],
            [-5, 9],
            1,
            [true, false, true],
            boot_d,
        );
        v.repaint(mx(s, 0, 10), [1, 3], [7, 9], |_, _, _, _| boot_hi);
        if l.armor == ArmorType::Plate {
            // greaves with a knee guard
            v.fill(mx(s, 0, 10), [5, 12], [2, 7], steel);
            v.fill(mx(s, 1, 9), [11, 14], [5, 7], steel_hi);
            v.fill(mx(s, 0, 10), [5, 6], [2, 7], steel_d);
            v.set(mi(s, 4), 12, 7, trim);
            v.set(mi(s, 5), 12, 7, trim);
        } else {
            // cuff, laces and a buckle
            v.rbox(
                mx(s, 0, 10),
                [9, 11],
                [-5, 6],
                1,
                [true, false, true],
                boot_hi,
            );
            for y in [5, 7] {
                v.fill(mx(s, 3, 7), [y, y + 1], [5, 6], boot_d);
            }
            v.fill(mx(s, 9, 10), [6, 8], [-1, 2], trim);
        }
    }
}

fn build_torso(v: &mut Vox, l: &Look) -> i32 {
    v.part = P_TORSO;
    let cloth = v.ink(l.tunic);
    let cloth_d = v.tone(l.tunic, 0.8);
    let cloth_hi = v.tone(l.tunic, 1.18);
    let trim = v.ink(l.trim);
    let trim_d = v.tone(l.trim, 0.7);
    let skin = v.ink(l.skin);
    let pants = v.ink(l.pants);
    let belt = v.ink([0.25, 0.16, 0.09]);
    let belt_hi = v.ink([0.36, 0.24, 0.14]);
    let steel = v.ink(STEEL);
    let steel_hi = v.tone(STEEL, 1.22);
    let steel_d = v.tone(STEEL, 0.72);
    // half width and depth by build
    let (hw, zb, zf) = match l.build {
        0 => (9, -4, 5),
        2 => (11, -6, 7),
        _ => (10, -5, 6),
    };
    v.fill([-9, 9], [23, 28], [-4, 5], pants);
    v.rbox([-hw, hw], [27, 47], [zb, zf], 1, [true, false, true], cloth);
    // shoulders reach out to the arms
    v.rbox(
        [-hw - 2, hw + 2],
        [42, 47],
        [zb + 1, zf - 1],
        1,
        [true, true, true],
        cloth,
    );
    v.fill([-3, 3], [47, 49], [-3, 3], skin);
    let front = zf - 1;
    // folds: soft dark creases under the chest, lit ridges beside them
    if matches!(l.armor, ArmorType::Cloth | ArmorType::Leather) {
        for x in [-6, 5] {
            v.repaint([x, x + 1], [30, 39], [zb, zf], |_, _, _, _| cloth_d);
            v.repaint([x + 1, x + 2], [30, 37], [front, zf], |_, _, _, _| cloth_hi);
        }
        v.repaint([-1, 1], [30, 36], [zb, zb + 1], |_, _, _, _| cloth_d);
    }
    // collar: a V of skin edged with trim
    for y in 41..47 {
        let half = (y - 40) / 2;
        for x in -half..half {
            v.dye(x, y, front, skin);
        }
        if l.apron.is_none() {
            v.dye(-half - 1, y, front, trim);
            v.dye(half, y, front, trim);
        }
    }
    let ring = if l.apron.is_some() { cloth_d } else { trim };
    v.repaint([-5, 5], [46, 47], [zb, zf], |x, _, z, c| {
        if (-4..4).contains(&x) && z < front && z > zb {
            c
        } else {
            ring
        }
    });
    match l.armor {
        ArmorType::Cloth => {
            // an embroidered band down the front
            v.repaint([-1, 1], [30, 41], [front, zf], |_, y, _, _| {
                if y % 3 == 0 { trim_d } else { trim }
            });
        }
        ArmorType::Leather => {
            // a darker yoke with stitching, and a strap across the chest
            let hide = v.ink(LEATHER);
            let hide_hi = v.tone(LEATHER, 1.35);
            v.repaint([-hw - 2, hw + 2], [42, 46], [zb, zf], |x, y, _, c| {
                if c == skin || c == trim {
                    c
                } else if y == 42 && x.rem_euclid(2) == 0 {
                    hide_hi
                } else {
                    hide
                }
            });
            for y in 30..46 {
                let x = (y - 38) * 9 / 8;
                for dx in 0..3 {
                    for z in [zb - 1, zf] {
                        if (-hw..hw).contains(&(x + dx)) {
                            v.set(x + dx, y, z, if dx == 1 { hide_hi } else { hide });
                        }
                    }
                }
            }
            v.fill([1, 4], [39, 42], [zf, zf + 1], trim);
            v.set(2, 40, zf, trim_d);
        }
        ArmorType::Mail => {
            // mail showing at the sides of a tabard
            v.repaint([-hw, hw], [27, 46], [zb, zf], |x, y, z, c| {
                let tabard = (-6..6).contains(&x) && (z == front || z == zb);
                if c == skin || c == trim || tabard {
                    c
                } else if y.rem_euclid(2) == 0 {
                    steel
                } else {
                    steel_d
                }
            });
            for z in [front, zb] {
                v.repaint([-6, -5], [27, 46], [z, z + 1], |_, _, _, c| {
                    if c == skin { c } else { trim }
                });
                v.repaint([5, 6], [27, 46], [z, z + 1], |_, _, _, c| {
                    if c == skin { c } else { trim }
                });
            }
        }
        ArmorType::Plate => {
            // a breastplate with a lit ridge, and a gorget
            v.rbox(
                [-hw, hw],
                [35, 46],
                [zb - 1, zf + 1],
                1,
                [true, true, true],
                steel,
            );
            v.fill([-hw + 1, hw - 1], [33, 35], [zb, zf], steel_d);
            v.fill([-1, 1], [36, 45], [zf + 1, zf + 2], steel_hi);
            v.fill([-hw + 2, hw - 2], [44, 45], [zf, zf + 1], steel_hi);
            v.fill([-5, 5], [46, 48], [-4, 5], steel);
            v.fill([-3, 3], [47, 49], [-3, 3], skin);
            for s in [1, -1] {
                v.set(mi(s, hw - 3), 37, zf, trim);
                v.set(mi(s, hw - 3), 43, zf, trim);
            }
        }
    }
    // emblem on the chest
    if l.emblem > 0 && l.apron.is_none() && !l.elder {
        let art = EMBLEMS[(l.emblem as usize - 1) % EMBLEMS.len()];
        let z = if l.armor == ArmorType::Plate {
            zf
        } else {
            front
        };
        for (row, line) in art.iter().enumerate() {
            for (col, ch) in line.bytes().enumerate() {
                if ch == b'#' {
                    let (x, y) = (col as i32 - 3, 40 - row as i32);
                    if l.armor == ArmorType::Plate || v.get(x, y, z) != skin {
                        v.dye(x, y, z, trim);
                    }
                }
            }
        }
    }
    // belt with a buckle and a pouch
    v.rbox(
        [-hw - 1, hw + 1],
        [27, 30],
        [zb - 1, zf + 1],
        1,
        [true, false, true],
        belt,
    );
    v.repaint(
        [-hw - 1, hw + 1],
        [28, 29],
        [zb - 1, zf + 1],
        |x, _, _, _| {
            if x.rem_euclid(4) == 1 { belt_hi } else { belt }
        },
    );
    if let Some(sash) = l.sash {
        // a broad sash over the belt, knotted at the hip, its ends hanging
        let c = v.ink(sash);
        let c_d = v.tone(sash, 0.75);
        v.rbox(
            [-hw - 1, hw + 1],
            [27, 32],
            [zb - 1, zf + 1],
            1,
            [true, false, true],
            c,
        );
        v.repaint(
            [-hw - 1, hw + 1],
            [29, 30],
            [zb - 1, zf + 1],
            |_, _, _, _| c_d,
        );
        v.fill([-hw - 2, -hw + 2], [27, 31], [zf - 1, zf + 2], c_d);
        v.fill([-hw - 1, -hw + 1], [17, 27], [zf, zf + 1], c);
        v.fill([-hw + 1, -hw + 3], [20, 27], [zf, zf + 1], c_d);
    }
    v.fill([-2, 2], [26, 31], [zf, zf + 2], trim);
    v.fill([-1, 1], [28, 29], [zf + 1, zf + 2], belt);
    if l.apron.is_none() {
        v.fill([hw - 5, hw - 1], [23, 28], [zf, zf + 2], belt_hi);
        v.fill([hw - 5, hw - 1], [27, 28], [zf, zf + 2], belt);
        v.set(hw - 3, 26, zf + 2, trim);
    }
    if let Some(apron) = l.apron {
        // bib, neck strap and ties
        let a = v.ink(apron);
        let a_d = v.tone(apron, 0.82);
        v.fill([-6, 6], [30, 41], [zf, zf + 1], a);
        v.fill([-7, 7], [27, 33], [zf, zf + 2], a);
        v.fill([-6, 6], [40, 41], [zf, zf + 1], a_d);
        for s in [1, -1] {
            v.fill(mx(s, 5, 7), [41, 47], [zf, zf + 1], a_d);
            v.fill(mx(s, 5, 7), [47, 48], [zb + 2, zf + 1], a_d);
            v.fill(mx(s, 7, hw + 2), [30, 31], [zb - 1, zf + 1], a_d);
        }
        v.fill([-2, 2], [29, 32], [zb - 2, zb - 1], a);
        v.fill([-4, -2], [27, 30], [zb - 2, zb - 1], a_d);
        v.fill([2, 4], [27, 30], [zb - 2, zb - 1], a_d);
    }
    if l.hair_style == 3 {
        // the hood's cowl lies on the shoulders
        v.rbox(
            [-8, 8],
            [45, 49],
            [zb - 1, zf + 1],
            2,
            [true, false, true],
            cloth_d,
        );
        v.fill([-3, 3], [47, 49], [-3, 3], skin);
    }
    if l.pauldrons {
        // layered shoulder plates, rimmed and riveted
        for s in [1, -1] {
            v.rbox(
                mx(s, hw - 1, hw + 8),
                [42, 49],
                [-6, 7],
                2,
                [true; 3],
                steel,
            );
            v.rbox(mx(s, hw, hw + 6), [48, 50], [-4, 5], 1, [true; 3], steel_hi);
            v.repaint(mx(s, hw - 1, hw + 8), [42, 43], [-6, 7], |_, _, _, _| trim);
            v.repaint(mx(s, hw - 1, hw + 8), [45, 46], [-6, 7], |_, _, _, _| {
                steel_d
            });
            for z in [-6, 6] {
                v.dye(mi(s, hw + 2), 44, z, steel_hi);
                v.dye(mi(s, hw + 5), 44, z, steel_hi);
            }
            v.dye(mi(s, hw + 7), 44, -2, steel_hi);
            v.dye(mi(s, hw + 7), 44, 2, steel_hi);
        }
    }
    if let Some(cape) = l.cape {
        // the cape's collar and two clasps
        let c = v.ink(cape);
        v.fill([-hw - 1, hw + 1], [45, 48], [zb - 2, zb + 1], c);
        for s in [1, -1] {
            v.fill(mx(s, 5, hw), [46, 48], [zb, zf], c);
            v.fill(mx(s, 5, 8), [44, 47], [zf, zf + 1], trim);
            v.set(mi(s, 6), 45, zf, trim_d);
        }
    }

    // the garment below the belt
    let hem_trim = |y: i32, base: Ink, low: i32| if y <= low + 1 { trim } else { base };
    if l.robe {
        v.part = P_SKIRT;
        for y in 2..28 {
            let grow = (28 - y) * 3 / 26;
            let (x0, x1) = (-hw - grow, hw + grow);
            let (z0, z1) = (zb - grow, zf + grow);
            for z in z0..z1 {
                for x in x0..x1 {
                    if !in_rbox(
                        [x, y, z],
                        [x0, y, z0],
                        [x1, y + 1, z1],
                        2,
                        [true, false, true],
                    ) {
                        continue;
                    }
                    // a wavy hem: every other fold hangs lower
                    let fold = (x + 40).rem_euclid(4);
                    if y == 2 && fold >= 2 {
                        continue;
                    }
                    let c = if (4..6).contains(&y) {
                        trim
                    } else if y == 7 {
                        trim_d
                    } else if fold == 0 {
                        cloth_d
                    } else if fold == 2 && y < 20 {
                        cloth_hi
                    } else {
                        cloth
                    };
                    v.set(x, y, z, c);
                }
            }
        }
        if let Some(apron) = l.apron {
            let a = v.ink(apron);
            let a_d = v.tone(apron, 0.82);
            for y in 9..28 {
                let grow = (28 - y) * 3 / 26;
                let z = zf + grow;
                for x in -8i32..8 {
                    let c = if x.rem_euclid(5) == 2 && y < 22 {
                        a_d
                    } else {
                        a
                    };
                    v.set(x, y, z, c);
                }
            }
            // a pocket
            v.fill([2, 7], [16, 21], [zf + 2, zf + 3], a_d);
            v.fill([3, 6], [16, 20], [zf + 2, zf + 3], a);
        }
    } else {
        let mail_skirt = l.armor == ArmorType::Mail;
        let low = if l.armor == ArmorType::Cloth { 17 } else { 19 };
        for (part, z0, z1) in [(P_FLAP_F, 0, zf), (P_FLAP_B, zb, 0)] {
            v.part = part;
            for y in low..28 {
                for z in z0..z1 {
                    for x in -hw..hw {
                        let rim = x == -hw || x == hw - 1 || z == zb || z == zf - 1;
                        if !rim || ((x == -hw || x == hw - 1) && (z == zb || z == zf - 1)) {
                            continue;
                        }
                        // split at the front and back so the legs can stride
                        if (-1..1).contains(&x) && y < 24 {
                            continue;
                        }
                        let c = match l.armor {
                            ArmorType::Leather => {
                                // hanging straps
                                if y < 23 && (x + 40).rem_euclid(4) == 0 {
                                    continue;
                                }
                                if y == low { trim } else { v.ink(LEATHER) }
                            }
                            ArmorType::Plate => {
                                // tassets
                                if y == low || y == 23 { steel_d } else { steel }
                            }
                            _ if mail_skirt => {
                                if (-6..6).contains(&x) {
                                    continue;
                                }
                                mail(v, y)
                            }
                            _ => {
                                if (x + 40).rem_euclid(5) == 0 {
                                    hem_trim(y, cloth_d, low)
                                } else {
                                    hem_trim(y, cloth, low)
                                }
                            }
                        };
                        v.set(x, y, z, c);
                    }
                }
            }
            if mail_skirt {
                // the tabard hangs longer than the mail, to a point
                let z = if part == P_FLAP_F { zf - 1 } else { zb };
                for y in 13..28 {
                    for x in -6..6 {
                        let inset = (16 - y).max(0);
                        if x < -6 + inset || x >= 6 - inset {
                            continue;
                        }
                        let edge = x == -6 + inset || x == 5 - inset || y == 13;
                        v.set(x, y, z, if edge { trim } else { cloth });
                    }
                }
            }
        }
        if let Some(apron) = l.apron {
            v.part = P_FLAP_F;
            let a = v.ink(apron);
            let a_d = v.tone(apron, 0.8);
            let a_hi = v.tone(apron, 1.25);
            v.fill([-8, 8], [12, 28], [zf, zf + 1], a);
            v.fill([-8, 8], [12, 13], [zf, zf + 1], a_d);
            // a tool pocket with stitching
            v.fill([-6, 6], [18, 23], [zf + 1, zf + 2], a_d);
            v.fill([-5, 5], [18, 22], [zf + 1, zf + 2], a);
            for x in [-5, -2, 1, 4] {
                v.set(x, 22, zf + 1, a_hi);
            }
        }
    }
    hw
}

fn build_arms(v: &mut Vox, l: &Look, hw: i32) {
    let cloth = v.ink(l.tunic);
    let cloth_d = v.tone(l.tunic, 0.8);
    let trim = v.ink(l.trim);
    let skin = v.ink(l.skin);
    let skin_d = v.tone(l.skin, 0.84);
    let glove = v.tone(l.boots, 1.25);
    let glove_hi = v.tone(l.boots, 1.7);
    let steel = v.ink(STEEL);
    let steel_hi = v.tone(STEEL, 1.22);
    let steel_d = v.tone(STEEL, 0.72);
    let hide = v.ink(LEATHER);
    let hide_hi = v.tone(LEATHER, 1.4);
    let shirt = v.ink(mix(l.tunic, [0.9, 0.86, 0.76], 0.7));
    let a = hw + 1;
    for (i, s) in [1, -1].into_iter().enumerate() {
        v.part = P_ARM_U[i];
        v.rbox(
            mx(s, a, a + 6),
            [33, 47],
            [-3, 3],
            1,
            [true, true, true],
            cloth,
        );
        match l.armor {
            ArmorType::Mail => {
                for y in 33..47 {
                    let c = mail(v, y);
                    v.repaint(mx(s, a, a + 6), [y, y + 1], [-3, 3], |_, _, _, _| c);
                }
            }
            ArmorType::Plate => {
                v.repaint(mx(s, a, a + 6), [33, 47], [-3, 3], |_, y, _, _| {
                    if y % 4 == 1 { steel_d } else { steel }
                });
            }
            _ => {
                v.repaint(mx(s, a, a + 6), [33, 35], [-3, 3], |_, _, _, _| cloth_d);
                v.repaint(mx(s, a + 5, a + 6), [35, 44], [0, 1], |_, _, _, _| cloth_d);
            }
        }
        v.part = P_ARM_L[i];
        let sleeve = match l.armor {
            ArmorType::Cloth if l.robe => cloth,
            ArmorType::Cloth => shirt,
            ArmorType::Leather => shirt,
            ArmorType::Mail => steel_d,
            ArmorType::Plate => steel,
        };
        v.rbox(
            mx(s, a, a + 6),
            [23, 35],
            [-3, 3],
            1,
            [true, false, true],
            sleeve,
        );
        match l.armor {
            ArmorType::Cloth if l.robe => {
                // a bell sleeve with a trimmed mouth
                v.rbox(
                    mx(s, a - 1, a + 7),
                    [23, 29],
                    [-4, 4],
                    1,
                    [true, false, true],
                    cloth,
                );
                v.repaint(mx(s, a - 1, a + 7), [23, 25], [-4, 4], |_, _, _, _| trim);
                v.repaint(mx(s, a, a + 6), [33, 35], [-3, 3], |_, _, _, _| cloth_d);
            }
            ArmorType::Cloth => {
                v.repaint(mx(s, a, a + 6), [24, 26], [-3, 3], |_, _, _, _| trim);
                v.repaint(mx(s, a, a + 6), [32, 35], [-3, 3], |_, _, _, _| cloth);
            }
            ArmorType::Leather => {
                // a laced bracer
                v.rbox(
                    mx(s, a - 1, a + 7),
                    [25, 32],
                    [-4, 4],
                    1,
                    [true, false, true],
                    hide,
                );
                for y in [26, 28, 30] {
                    v.repaint(mx(s, a - 1, a + 7), [y, y + 1], [3, 4], |x, _, _, c| {
                        if x.rem_euclid(2) == 0 { hide_hi } else { c }
                    });
                }
                v.repaint(mx(s, a, a + 6), [32, 35], [-3, 3], |_, _, _, _| cloth);
            }
            ArmorType::Mail => {
                for y in 29..35 {
                    let c = mail(v, y);
                    v.repaint(mx(s, a, a + 6), [y, y + 1], [-3, 3], |_, _, _, _| c);
                }
                v.rbox(
                    mx(s, a - 1, a + 7),
                    [24, 29],
                    [-4, 4],
                    1,
                    [true, false, true],
                    glove,
                );
                v.repaint(mx(s, a - 1, a + 7), [28, 29], [-4, 4], |_, _, _, _| {
                    glove_hi
                });
            }
            ArmorType::Plate => {
                // vambrace and elbow guard
                v.rbox(
                    mx(s, a - 1, a + 7),
                    [24, 31],
                    [-4, 4],
                    1,
                    [true, false, true],
                    steel,
                );
                v.repaint(mx(s, a - 1, a + 7), [30, 31], [-4, 4], |_, _, _, _| {
                    steel_hi
                });
                v.repaint(mx(s, a - 1, a + 7), [24, 25], [-4, 4], |_, _, _, _| steel_d);
                v.fill(mx(s, a + 1, a + 5), [32, 36], [-4, -3], steel_hi);
                v.set(mi(s, a + 6), 27, 0, trim);
            }
        }
        // the hand: big, with a thumb and knuckles
        let (hand, hand_d) = if l.gloves {
            (glove, glove_hi)
        } else {
            (skin, skin_d)
        };
        v.rbox(
            mx(s, a - 1, a + 6),
            [17, 24],
            [-4, 4],
            1,
            [true, true, true],
            hand,
        );
        v.fill(mx(s, a - 2, a - 1), [20, 23], [0, 3], hand);
        v.repaint(mx(s, a - 1, a + 6), [19, 20], [3, 4], |x, _, _, c| {
            if x.rem_euclid(2) == 0 { hand_d } else { c }
        });
        if l.gloves {
            v.repaint(mx(s, a - 1, a + 6), [23, 24], [-4, 4], |_, _, _, _| hand_d);
        }
    }
}

fn build_cape(v: &mut Vox, l: &Look, hw: i32) {
    let Some(cape) = l.cape else {
        return;
    };
    let c = v.ink(cape);
    let c_d = v.tone(cape, 0.78);
    let c_hi = v.tone(cape, 1.25);
    let trim = v.ink(l.trim);
    for y in 7..48 {
        let seg = if y >= 34 {
            0
        } else if y >= 21 {
            1
        } else {
            2
        };
        v.part = P_CAPE[seg];
        let half = hw + 1 + (47 - y) / 9;
        for x in -half..half {
            // a ragged, scalloped hem
            let hem = 7 + [0, 0, 1, 2, 1, 0][(x + 60).rem_euclid(6) as usize];
            if y < hem {
                continue;
            }
            let fold = (x + 60).rem_euclid(5);
            let ink = if y <= hem + 1 || x == -half || x == half - 1 {
                trim
            } else if fold == 0 {
                c_d
            } else if fold == 2 && y < 40 {
                c_hi
            } else {
                c
            };
            v.set(x, y, -7, ink);
            // folds stand proud of the cloth
            if fold == 2 && y < 44 && y > hem + 1 {
                v.set(x, y, -8, c);
            }
        }
    }
}

fn build_person(l: &Look) -> [Person; 3] {
    let mut v = Vox::new([-28, 0, -16], [28, 82, 16]);
    build_legs(&mut v, l);
    let hw = build_torso(&mut v, l);
    build_arms(&mut v, l, hw);
    build_cape(&mut v, l, hw);
    build_head(&mut v, l);
    let arm_x = hw as f32 + 4.0;
    let sides = [1.0f32, -1.0];
    lods(&v, |v| Person {
        head: v.mesh(P_HEAD, [0.0, NECK_Y, 0.0]),
        torso: v.mesh(P_TORSO, [0.0, WAIST_Y, 0.0]),
        flap_f: v.mesh(P_FLAP_F, [0.0, FLAP_Y, 0.0]),
        flap_b: v.mesh(P_FLAP_B, [0.0, FLAP_Y, 0.0]),
        skirt: v.mesh(P_SKIRT, [0.0, HIP_Y, 0.0]),
        arm_u: [0, 1].map(|i| v.mesh(P_ARM_U[i], [sides[i] * arm_x, SHOULDER_Y, 0.0])),
        arm_l: [0, 1].map(|i| v.mesh(P_ARM_L[i], [sides[i] * arm_x, ELBOW_Y, 0.0])),
        thigh: [0, 1].map(|i| v.mesh(P_THIGH[i], [sides[i] * LEG_X, HIP_Y, 0.0])),
        shin: [0, 1].map(|i| v.mesh(P_SHIN[i], [sides[i] * LEG_X, KNEE_Y, 0.0])),
        cape: [0, 1, 2].map(|i| v.mesh(P_CAPE[i], [0.0, CAPE_Y[i], CAPE_Z])),
        arm_x,
    })
}

/// A model at full, half and quarter detail.
fn lods<T>(v: &Vox, mesh: impl Fn(&Vox) -> T) -> [T; 3] {
    let half = v.half();
    let quarter = half.half();
    [mesh(v), mesh(&half), mesh(&quarter)]
}

/// Which level of detail to draw at `p`: full nearby, coarser far away, where
/// a fine voxel is smaller than a pixel. Full when the camera is not known.
fn lod_at(p: Vec3) -> usize {
    match view_distance(p) {
        Some(d) if d > 40.0 => 2,
        Some(d) if d > 14.0 => 1,
        _ => 0,
    }
}

thread_local! {
    static PEOPLE: Cache<[Person; 3]> = Cache::default();
}

/// Draw a player: legs and arms swing with `walk` (0 standing, 1 walking).
#[allow(clippy::too_many_arguments)]
pub fn push_avatar(
    out: &mut Vec<Vertex>,
    feet: Vec3,
    yaw: f32,
    pitch: f32,
    color: [f32; 3],
    light: f32,
    phase: f32,
    walk: f32,
) {
    let seed = (color[0] * 997.0 + color[1] * 131.0 + color[2] * 17.0) as u32;
    push_person(
        out,
        feet,
        yaw,
        pitch,
        &Look::player(seed, color),
        light,
        phase,
        walk,
    );
}

/// The frame of a figure standing at `feet`: x right, y up, z forward, in
/// voxels of `1 / per_block` blocks.
fn root(feet: Vec3, yaw: f32, per_block: f32) -> Affine3A {
    let (sy, cy) = yaw.sin_cos();
    let k = 1.0 / per_block;
    Affine3A::from_cols(
        (Vec3::new(cy, 0.0, -sy) * k).into(),
        (Vec3::Y * k).into(),
        (Vec3::new(-sy, 0.0, -cy) * k).into(),
        feet.into(),
    )
}

fn rx(a: f32) -> Mat3 {
    Mat3::from_rotation_x(a)
}

/// A bone's frame: `parent`, moved to `pivot` plus the pose's offset for the
/// bone, and turned by its rotation.
fn bone(parent: &Affine3A, pivot: Vec3, pose: &Pose, i: usize) -> Affine3A {
    *parent * Affine3A::from_rotation_translation(pose.rot[i], pivot + pose.off[i])
}

/// A pose for callers with no animation state: idle, or walking at `phase`
/// by `walk` (0..1), `phase / 9` seconds in.
fn simple_pose(walk: f32, phase: f32, pose: impl Fn(Anim, &anim::At) -> Pose) -> Pose {
    let at = anim::At {
        t: phase / 9.0,
        stride: phase,
        seed: 0.3,
    };
    let idle = pose(Anim::Idle, &at);
    let mut p = if walk > 0.0 {
        idle.blend(&pose(Anim::Walk, &at), walk.min(1.0), u32::MAX)
    } else {
        idle
    };
    p.k[anim::key::CLOCK] = at.t;
    p
}

/// What a person holds, as a grip.
fn grip_of(held: Option<(Kind, [f32; 3])>) -> Grip {
    match held.map(|(k, _)| k) {
        None => Grip::Empty,
        Some(Kind::BattleAxe | Kind::Staff | Kind::Polearm) => Grip::TwoHand,
        Some(Kind::Bow) => Grip::Bow,
        Some(_) => Grip::OneHand,
    }
}

/// The rig of a person in `look`, holding its weapon.
pub fn person_rig(look: &Look) -> BipedRig {
    BipedRig {
        grip: grip_of(look.weapon.map(|(k, c)| (weapon_kind(k), c))),
        robe: look.robe,
    }
}

/// Draw a person in an outfit. Proportions are heroic: big hands, head and
/// shoulders. `phase` drives the stride, `walk` is 0 standing to 1 walking.
/// (Figures with animation state use [`push_person_anim`].)
#[allow(clippy::too_many_arguments)]
pub fn push_person(
    out: &mut Vec<Vertex>,
    feet: Vec3,
    yaw: f32,
    pitch: f32,
    look: &Look,
    light: f32,
    phase: f32,
    walk: f32,
) {
    let held = look.weapon.map(|(kind, color)| (weapon_kind(kind), color));
    let rig = person_rig(look);
    let pose = simple_pose(walk, phase, |a, at| anim::biped(a, at, &rig));
    draw_person(
        out,
        feet,
        yaw,
        pitch,
        look,
        Paint::lit(light),
        &pose,
        held,
        1.0,
    );
}

/// Draw a person posed by its animator at `now` (seconds).
#[allow(clippy::too_many_arguments)]
pub fn push_person_anim(
    out: &mut Vec<Vertex>,
    feet: Vec3,
    yaw: f32,
    pitch: f32,
    look: &Look,
    light: f32,
    animator: &mut Animator,
    now: f32,
) {
    let held = look.weapon.map(|(kind, color)| (weapon_kind(kind), color));
    let rig = person_rig(look);
    let pose = animator.pose(now, Skeleton::Biped.upper(), |a, at| {
        anim::biped(a, at, &rig)
    });
    draw_person(
        out,
        feet,
        yaw,
        pitch,
        look,
        Paint::lit(light),
        &pose,
        held,
        1.0,
    );
}

/// A person, lit and tinted by `paint`, posed, with `held` in the right
/// hand, `scale` times life size.
#[allow(clippy::too_many_arguments)]
fn draw_person(
    out: &mut Vec<Vertex>,
    feet: Vec3,
    yaw: f32,
    pitch: f32,
    look: &Look,
    paint: Paint,
    pose: &Pose,
    held: Option<(Kind, [f32; 3])>,
    scale: f32,
) {
    use anim::biped::*;
    let m = PEOPLE.with(|c| c.get(look.key(), || build_person(look)));
    let m = &m[lod_at(feet)];
    let base = root(feet, yaw, FINE / scale);
    // the pelvis carries the legs, the skirts and the body
    let pelvis = bone(&base, Vec3::new(0.0, HIP_Y, 0.0), pose, PELVIS);
    for (i, s) in [1.0f32, -1.0].into_iter().enumerate() {
        let hip = bone(&pelvis, Vec3::new(s * LEG_X, 0.0, 0.0), pose, THIGH[i]);
        emit(out, &m.thigh[i], &hip, paint);
        let shin = bone(&hip, Vec3::new(0.0, KNEE_Y - HIP_Y, 0.0), pose, SHIN[i]);
        emit(out, &m.shin[i], &shin, paint);
    }
    emit(out, &m.skirt, &pelvis, paint);
    // the tunic's skirts lift with the stride
    let lift = pose.k[anim::key::SHAPE];
    let flap = joint(&pelvis, Vec3::new(0.0, FLAP_Y - HIP_Y, 0.0), rx(-lift));
    emit(out, &m.flap_f, &flap, paint);
    let flap = joint(&pelvis, Vec3::new(0.0, FLAP_Y - HIP_Y, 0.0), rx(lift));
    emit(out, &m.flap_b, &flap, paint);
    let body = bone(&pelvis, Vec3::new(0.0, WAIST_Y - HIP_Y, 0.0), pose, SPINE);
    emit(out, &m.torso, &body, paint);
    // cape: three hinged lengths, streaming back at speed
    let flow = pose.k[anim::key::FLOW];
    let clock = pose.k[anim::key::CLOCK];
    let mut frame = bone(
        &body,
        Vec3::new(0.0, CAPE_Y[0] - WAIST_Y, CAPE_Z),
        pose,
        CAPE,
    );
    for (i, seg) in m.cape.iter().enumerate() {
        if seg.is_empty() {
            break;
        }
        let k = i as f32;
        let a = [0.07, 0.05, 0.06][i]
            + flow * [0.26, 0.16, 0.12][i]
            + (clock * 4.5 - k * 0.9).sin() * (0.025 + 0.02 * k) * (1.0 + flow);
        let at = if i == 0 {
            Vec3::ZERO
        } else {
            Vec3::new(0.0, -13.0, 0.0)
        };
        frame = joint(&frame, at, rx(a));
        emit(out, seg, &frame, paint);
    }
    for (i, s) in [1.0f32, -1.0].into_iter().enumerate() {
        let shoulder = bone(
            &body,
            Vec3::new(s * m.arm_x, SHOULDER_Y - WAIST_Y, 0.0),
            pose,
            ARM_U[i],
        );
        emit(out, &m.arm_u[i], &shoulder, paint);
        let fore = bone(
            &shoulder,
            Vec3::new(0.0, ELBOW_Y - SHOULDER_Y, 0.0),
            pose,
            ARM_L[i],
        );
        emit(out, &m.arm_l[i], &fore, paint);
        if i == 0
            && let Some((kind, color)) = held
        {
            // gripped in the fist, pointing up and ahead
            let lean = Mat3::from_rotation_z(-0.22);
            let grip = joint(&fore, Vec3::new(0.0, 20.5 - ELBOW_Y, 0.5), rx(1.25) * lean)
                * Affine3A::from_quat(pose.rot[GRIP]);
            let m = held_model(kind, color, 1, false);
            let fit = Mat3::from_rotation_y(m.carry_turn)
                * Mat3::from_diagonal(Vec3::splat(m.unit * FINE));
            emit(out, &m.mesh, &(grip * Affine3A::from_mat3(fit)), paint);
        }
    }
    // head, tilted by pitch
    let head = bone(&body, Vec3::new(0.0, NECK_Y - WAIST_Y, 0.0), pose, HEAD)
        * Affine3A::from_mat3(rx(-pitch * 0.55))
        * Affine3A::from_scale(Vec3::splat(HEAD_SCALE));
    emit(out, &m.head, &head, paint);
}

// ------------------------------------------------------------- creatures

/// A four-legged beast: body, a head on a hinge, and four legs.
struct Beast {
    body: Mesh,
    head: Mesh,
    /// front right, front left, back right, back left
    legs: [Mesh; 4],
    neck: [f32; 3],
    hip_y: f32,
    /// leg centres: (x, z)
    legs_at: [(f32, f32); 4],
    /// dips its head to graze when standing
    grazes: bool,
}

const G_BODY: u8 = 1;
const G_HEAD: u8 = 2;
const G_LEG: [u8; 4] = [3, 4, 5, 6];
const G_NECK: [f32; 3] = [0.0, 30.0, 13.0];
const G_HIP_Y: f32 = 17.0;
/// leg centres: (x, z)
const G_LEGS: [(f32, f32); 4] = [(6.5, 9.5), (-6.5, 9.5), (6.5, -9.5), (-6.5, -9.5)];

/// A woolly grazer: a round fleece of clumps on four dark legs, with a long
/// dark face, floppy ears and a pair of curled horns.
fn build_grazer(color: [f32; 3]) -> [Beast; 3] {
    let mut v = Vox::new([-20, 0, -22], [20, 48, 38]);
    let wool = v.ink(color);
    let wool_hi = v.ink(mix(scale(color, 1.12), [1.0, 1.0, 1.0], 0.25));
    let wool_lo = v.tone(color, 0.84);
    let wool_d = v.tone(color, 0.7);
    let face_rgb = [0.34, 0.26, 0.21];
    let face = v.ink(face_rgb);
    let face_d = v.tone(face_rgb, 0.75);
    let muzzle = v.ink([0.62, 0.50, 0.43]);
    let nose = v.ink([0.20, 0.13, 0.12]);
    let hoof = v.ink([0.16, 0.13, 0.12]);
    let horn = v.ink([0.62, 0.52, 0.38]);
    let horn_d = v.ink([0.40, 0.31, 0.22]);
    let pink = v.ink([0.85, 0.58, 0.55]);
    let eye = v.ink([0.95, 0.74, 0.25]);
    let pupil = v.ink([0.05, 0.04, 0.04]);

    // legs: slim and dark, with a cloven hoof and a tuft of fleece on top
    for (i, (lx, lz)) in G_LEGS.iter().enumerate() {
        v.part = G_LEG[i];
        let (x, z) = (*lx as i32, *lz as i32);
        let (x0, z0) = (if x > 0 { x - 2 } else { x - 3 }, z - 2);
        v.rbox(
            [x0, x0 + 5],
            [0, 19],
            [z0, z0 + 5],
            1,
            [true, false, true],
            face,
        );
        v.fill([x0, x0 + 5], [0, 3], [z0, z0 + 6], hoof);
        v.cut([x0 + 2, x0 + 3], [0, 2], [z0 + 4, z0 + 6]);
        v.repaint([x0, x0 + 5], [3, 4], [z0, z0 + 6], |_, _, _, _| face_d);
        v.repaint([x0, x0 + 5], [9, 11], [z0, z0 + 5], |_, _, _, _| face_d);
        v.blob(
            [x0 as f32 + 2.5, 16.5, z0 as f32 + 2.5],
            [3.6, 3.2, 3.6],
            wool_lo,
        );
    }

    // body: a core with clumps of fleece all over it
    v.part = G_BODY;
    v.blob([0.0, 26.0, 0.0], [9.5, 8.0, 14.5], wool);
    v.blob([0.0, 21.0, 0.0], [8.0, 4.0, 12.5], wool_d);
    for i in 0..64 {
        let h = hash3(i, 7, 13);
        let (a, b) = (
            (h & 0xFFFF) as f32 / 65535.0 * std::f32::consts::TAU,
            ((h >> 16) & 0xFF) as f32 / 255.0,
        );
        // height on the body: mostly the top and sides
        let up = -0.35 + b * 1.3;
        let ring = (1.0 - up.min(1.0).powi(2)).max(0.0).sqrt();
        let at = [
            a.cos() * ring * 9.5,
            26.0 + up.min(1.0) * 8.0,
            a.sin() * ring * 14.5,
        ];
        let r = 2.6 + ((h >> 24) & 7) as f32 * 0.22;
        let ink = match (h >> 27) % 5 {
            0 | 1 => wool,
            2 => wool_lo,
            _ if up > 0.3 => wool_hi,
            _ => wool,
        };
        v.blob(at, [r, r * 0.9, r], ink);
    }
    // a stub of a tail, and a ruff at the neck
    v.blob([0.0, 28.0, -16.0], [2.6, 3.0, 2.4], wool_hi);
    v.blob([0.0, 30.0, 12.5], [6.5, 6.0, 3.5], wool_hi);

    // head: long and dark, on its own hinge so it can dip to graze
    v.part = G_HEAD;
    v.rbox([-3, 3], [26, 33], [11, 18], 1, [true; 3], face);
    v.rbox([-6, 6], [27, 39], [16, 28], 2, [true; 3], face);
    v.rbox([-4, 4], [26, 33], [26, 34], 1, [true; 3], muzzle);
    v.fill([-2, 2], [31, 32], [34, 35], nose);
    v.fill([-1, 1], [30, 31], [34, 35], nose);
    v.repaint([-4, 4], [28, 29], [29, 34], |_, _, _, _| nose);
    // a pale blaze down the nose
    v.repaint([-1, 1], [33, 39], [22, 28], |_, _, _, _| muzzle);
    // fleece cap
    v.blob([0.0, 39.5, 19.5], [6.0, 3.0, 4.6], wool_hi);
    v.blob([2.5, 40.5, 18.0], [2.6, 2.2, 2.6], wool);
    v.blob([-3.0, 40.0, 21.0], [2.4, 2.2, 2.4], wool);
    for s in [1, -1] {
        // eyes on the sides: amber with a level bar of a pupil, under a lid
        v.fill(mx(s, 5, 6), [34, 37], [23, 27], eye);
        v.fill(mx(s, 5, 6), [35, 36], [23, 27], pupil);
        v.fill(mx(s, 6, 7), [37, 38], [22, 28], face_d);
        // and seen from the front, a glint of them
        v.set(mi(s, 4), 35, 27, eye);
        v.set(mi(s, 4), 34, 27, pupil);
        // ears: out and down
        v.fill(mx(s, 6, 10), [33, 35], [17, 20], face);
        v.fill(mx(s, 9, 12), [31, 34], [17, 20], face);
        v.fill(mx(s, 7, 11), [33, 34], [18, 19], pink);
        // horns: a ridged spiral, curling back, down and out
        let sf = s as f32;
        for k in 0..90 {
            let t = k as f32 / 89.0;
            let ang = 1.9 + t * 7.6;
            let rad = 6.2 - t * 4.0;
            let at = [
                sf * (6.5 + t * 6.0),
                36.0 + ang.sin() * rad,
                15.5 + ang.cos() * rad,
            ];
            let w = 2.3 - t * 1.4;
            v.blob(at, [w, w, w], if (k / 5) % 2 == 0 { horn } else { horn_d });
        }
    }

    lods(&v, |v| Beast {
        body: v.mesh(G_BODY, [0.0; 3]),
        head: v.mesh(G_HEAD, G_NECK),
        legs: [0, 1, 2, 3].map(|i| v.mesh(G_LEG[i], [G_LEGS[i].0, G_HIP_Y, G_LEGS[i].1])),
        neck: G_NECK,
        hip_y: G_HIP_Y,
        legs_at: G_LEGS,
        grazes: true,
    })
}

struct Shade {
    body: Mesh,
    hood: Mesh,
    eyes: Mesh,
    arm_u: [Mesh; 2],
    arm_l: [Mesh; 2],
    tatters: [Mesh; 5],
}

const S_BODY: u8 = 1;
const S_HOOD: u8 = 2;
const S_EYES: u8 = 3;
const S_ARM_U: [u8; 2] = [4, 5];
const S_ARM_L: [u8; 2] = [6, 7];
const S_TATTER: [u8; 5] = [8, 9, 10, 11, 12];
const S_WAIST: [f32; 3] = [0.0, 20.0, 0.0];
const S_NECK: [f32; 3] = [0.0, 46.0, 0.0];
const S_EYE: [f32; 3] = [0.0, 54.0, 4.0];
const S_SHOULDER: (f32, f32) = (11.5, 43.0);
const S_ELBOW_Y: f32 = 30.0;
const S_HEM_Y: f32 = 20.0;
/// tatter centres along the hem
const S_TATTER_X: [f32; 5] = [-7.0, -3.5, 0.0, 3.5, 7.0];

/// A Shade: a hooded wraith with no legs, its robe ending in rags, long
/// arms with claws, and two pale eyes burning in an empty hood.
fn build_shade(color: [f32; 3]) -> [Shade; 3] {
    let mut v = Vox::new([-20, 0, -14], [20, 70, 14]);
    let dark = v.ink(color);
    let deep = v.tone(color, 0.62);
    let edge = v.ink([
        (color[0] * 1.6).min(1.0),
        (color[1] * 1.4).min(1.0),
        (color[2] * 1.9).min(1.0),
    ]);
    let rim = v.ink(mix(color, [0.55, 0.45, 0.80], 0.45));
    let void = v.ink([0.015, 0.01, 0.03]);
    let bone = v.ink([0.72, 0.68, 0.82]);
    let bone_d = v.ink([0.45, 0.42, 0.58]);
    let fire = v.glow([0.86, 0.72, 1.0]);
    let fire_hi = v.glow([1.0, 0.96, 1.0]);

    // robe: narrow at the waist, broad at the shoulders, hanging in folds
    v.part = S_BODY;
    for y in 20..46 {
        let half = 6 + (y - 20) * 4 / 18;
        let half = half.min(10);
        let (z0, z1) = (-4 - (y - 20) / 13, 5 + (y - 20) / 13);
        for z in z0..z1 {
            for x in -half..half {
                if !in_rbox(
                    [x, y, z],
                    [-half, y, z0],
                    [half, y + 1, z1],
                    2,
                    [true, false, true],
                ) {
                    continue;
                }
                let fold = (x + 40).rem_euclid(4);
                let c = if fold == 0 {
                    deep
                } else if fold == 2 && y < 38 {
                    rim
                } else {
                    dark
                };
                v.set(x, y, z, c);
            }
        }
    }
    // a ragged mantle over the shoulders
    for x in -12i32..12 {
        let low = 40 + [0, 1, 2, 0, 1, 3, 1, 0][(x + 40).rem_euclid(8) as usize];
        for y in low..48 {
            for z in -7..7 {
                if in_rbox([x, y, z], [-12, 38, -7], [12, 48, 7], 3, [true; 3]) {
                    let c = if y == low {
                        edge
                    } else if (x + 40).rem_euclid(5) == 0 {
                        deep
                    } else {
                        dark
                    };
                    v.set(x, y, z, c);
                }
            }
        }
    }
    // a pale clasp of bone at the throat
    v.fill([-2, 2], [43, 46], [7, 8], bone);
    v.set(-1, 44, 7, bone_d);
    v.set(0, 44, 7, bone_d);

    // the hem: five rags of different lengths, each free to sway
    let lengths = [13, 17, 11, 18, 14];
    for (i, cx) in S_TATTER_X.iter().enumerate() {
        v.part = S_TATTER[i];
        let cx = *cx as i32;
        let len = lengths[i];
        for dy in 0..len {
            let y = 21 - dy;
            // each rag narrows to a point, with a notch torn out of it
            let half = (2 - dy * 2 / len).max(0);
            let lean = (dy / 5) * if i % 2 == 0 { 1 } else { -1 };
            for x in cx - half - 1..cx + half + 1 {
                if dy == len / 2 && x == cx {
                    continue;
                }
                let c = if dy >= len - 2 {
                    edge
                } else if (x + dy).rem_euclid(3) == 0 {
                    deep
                } else {
                    dark
                };
                for z in -2..2 - (dy * 2 / len) {
                    v.set(x + lean, y, z + (i as i32 % 2), c);
                }
            }
        }
    }

    // hood: a shell around darkness, open at the front, with a drooping peak
    v.part = S_HOOD;
    for y in 45..64 {
        for z in -9..9 {
            for x in -9..9 {
                let p = [x, y, z];
                if !in_rbox(p, [-9, 45, -9], [9, 64, 9], 3, [true; 3]) {
                    continue;
                }
                let inside = in_rbox(p, [-6, 47, -5], [6, 60, 10], 2, [true, true, false]);
                if inside {
                    if z <= 0 {
                        v.set(x, y, z, void);
                    }
                    continue;
                }
                let c = if z >= 7 {
                    edge
                } else if (x + z + 40).rem_euclid(5) == 0 {
                    deep
                } else {
                    dark
                };
                v.set(x, y, z, c);
            }
        }
    }
    v.fill([-3, 3], [63, 66], [-9, -3], dark);
    v.fill([-2, 2], [64, 67], [-12, -8], dark);
    v.fill([-1, 1], [62, 66], [-14, -11], deep);
    v.fill([-1, 1], [60, 63], [-15, -13], edge);
    // torn edge under the chin
    for x in -6i32..6 {
        if (x + 40).rem_euclid(3) != 1 {
            v.set(x, 44, 7, edge);
        }
    }

    // eyes: slanted and burning
    v.part = S_EYES;
    for s in [1, -1] {
        v.fill(mx(s, 1, 6), [52, 55], [3, 5], fire);
        v.fill(mx(s, 3, 7), [55, 56], [3, 5], fire);
        v.fill(mx(s, 5, 7), [56, 57], [3, 5], fire);
        v.fill(mx(s, 2, 5), [53, 54], [3, 5], fire_hi);
        v.cut(mx(s, 1, 2), [54, 55], [3, 5]);
        v.cut(mx(s, 4, 6), [52, 53], [3, 5]);
    }

    // arms: long and thin in ragged sleeves, ending in three claws
    for (i, s) in [1, -1].into_iter().enumerate() {
        v.part = S_ARM_U[i];
        v.rbox(mx(s, 9, 14), [29, 46], [-3, 3], 1, [true; 3], dark);
        v.repaint(mx(s, 9, 14), [29, 46], [-3, 3], |_, y, _, c| {
            if y % 5 == 0 { deep } else { c }
        });
        v.part = S_ARM_L[i];
        // the sleeve ends in tatters above a bony forearm
        for x in 9..14 {
            let low = 23 + [0, 2, 1, 3, 0][(x - 9) as usize];
            for y in low..31 {
                for z in -3..3 {
                    if in_rbox([x, y, z], [9, low, -3], [14, 31, 3], 1, [true, false, true]) {
                        v.set(mi(s, x), y, z, if y == low { edge } else { dark });
                    }
                }
            }
        }
        v.fill(mx(s, 10, 13), [15, 26], [-1, 2], bone_d);
        v.fill(mx(s, 10, 13), [12, 16], [-2, 3], bone);
        for (cx, cz, len) in [(10, -2, 6), (11, 0, 8), (12, 2, 6)] {
            for dy in 0..len {
                let c = if dy >= len - 2 { edge } else { bone };
                v.set(mi(s, cx), 12 - dy, cz + dy / 3, c);
            }
        }
        // a thumb
        v.set(mi(s, 9), 13, 2, bone);
        v.set(mi(s, 9), 12, 3, bone);
        v.set(mi(s, 9), 11, 3, edge);
    }

    lods(&v, |v| Shade {
        body: v.mesh(S_BODY, S_WAIST),
        hood: v.mesh(S_HOOD, S_NECK),
        eyes: v.mesh(S_EYES, S_EYE),
        arm_u: [0, 1].map(|i| {
            let s = if i == 0 { 1.0 } else { -1.0 };
            v.mesh(S_ARM_U[i], [s * S_SHOULDER.0, S_SHOULDER.1, 0.0])
        }),
        arm_l: [0, 1].map(|i| {
            let s = if i == 0 { 1.0 } else { -1.0 };
            v.mesh(S_ARM_L[i], [s * S_SHOULDER.0, S_ELBOW_Y, 0.0])
        }),
        tatters: [0, 1, 2, 3, 4].map(|i| v.mesh(S_TATTER[i], [S_TATTER_X[i], S_HEM_Y, 0.0])),
    })
}

/// A thornboar: a bristly, high-shouldered boar with curved tusks, small
/// angry eyes and a ridge of thorns down its back.
fn build_thornboar(color: [f32; 3]) -> [Beast; 3] {
    const NECK: [f32; 3] = [0.0, 19.0, 11.0];
    const HIP_Y: f32 = 11.0;
    const LEGS: [(f32, f32); 4] = [(5.5, 8.5), (-5.5, 8.5), (5.5, -9.5), (-5.5, -9.5)];
    let mut v = Vox::new([-16, 0, -24], [16, 40, 36]);
    let hide = v.ink(color);
    let hide_d = v.tone(color, 0.72);
    let hide_hi = v.tone(color, 1.3);
    let belly = v.tone(color, 0.58);
    let hoof = v.ink([0.10, 0.08, 0.07]);
    let snout = v.ink(mix(color, [0.75, 0.50, 0.45], 0.6));
    let nostril = v.ink([0.12, 0.07, 0.07]);
    let ivory = v.ink([0.95, 0.91, 0.76]);
    let ivory_d = v.ink([0.74, 0.68, 0.50]);
    let thorn = v.ink([0.20, 0.24, 0.12]);
    let thorn_hi = v.ink([0.62, 0.66, 0.34]);
    let eye = v.glow([1.0, 0.42, 0.10]);
    let brow = v.ink([0.06, 0.04, 0.04]);
    // coarse bristles: flecks of dark and light through the coat
    let coat = |x: i32, y: i32, z: i32| match hash3(x, y / 2, z) % 9 {
        0 | 1 => hide_d,
        2 => hide_hi,
        _ => hide,
    };

    for (i, (lx, lz)) in LEGS.iter().enumerate() {
        v.part = G_LEG[i];
        let (x0, z0) = (lx.floor() as i32 - 3, lz.floor() as i32 - 3);
        v.rbox(
            [x0, x0 + 6],
            [0, 14],
            [z0, z0 + 6],
            1,
            [true, false, true],
            hide_d,
        );
        v.fill([x0, x0 + 6], [0, 3], [z0, z0 + 7], hoof);
        v.cut([x0 + 2, x0 + 4], [0, 2], [z0 + 5, z0 + 7]);
    }

    v.part = G_BODY;
    v.blob([0.0, 18.5, -2.0], [8.5, 7.5, 14.0], hide);
    // high shoulders, lower rump
    v.blob([0.0, 22.0, 6.0], [8.0, 8.5, 8.0], hide);
    v.blob([0.0, 14.0, -1.0], [7.0, 4.0, 12.0], belly);
    v.repaint([-16, 16], [0, 40], [-24, 36], |x, y, z, c| {
        if c == hide { coat(x, y, z) } else { c }
    });
    // thorns: a ragged crest along the spine, and a shorter row each side
    for z in -14i32..14 {
        for (x, every, tall) in [(-1, 3, 6), (0, 3, 6), (-5, 4, 3), (4, 4, 3)] {
            if (z + 40).rem_euclid(every) != 0 {
                continue;
            }
            let Some(top) = (10..40).rev().find(|y| v.solid(x, *y, z)) else {
                continue;
            };
            let h = tall - (hash3(x, 0, z) % 3) as i32;
            for y in top + 1..top + 1 + h {
                v.set(x, y, z, if y >= top + h - 1 { thorn_hi } else { thorn });
            }
        }
    }
    // a thin tail with a tuft
    v.fill([-1, 1], [17, 21], [-17, -15], hide_d);
    v.fill([-1, 1], [13, 17], [-18, -17], hide_d);
    v.fill([-1, 1], [11, 13], [-19, -17], thorn);

    v.part = G_HEAD;
    v.rbox([-6, 6], [11, 26], [10, 23], 2, [true; 3], hide);
    v.rbox([-4, 4], [10, 19], [21, 31], 1, [true; 3], hide_d);
    v.repaint([-6, 6], [11, 26], [10, 23], |x, y, z, c| {
        if c == hide { coat(x, y, z) } else { c }
    });
    // the snout's flat disc, and a dark mouth line
    v.fill([-3, 3], [12, 18], [31, 32], snout);
    v.fill([-2, -1], [14, 16], [32, 33], nostril);
    v.fill([1, 2], [14, 16], [32, 33], nostril);
    v.repaint([-4, 4], [12, 13], [24, 31], |_, _, _, _| nostril);
    // bristles between the ears
    for z in [12, 15, 18] {
        v.fill([-1, 1], [26, 29], [z, z + 1], thorn);
        v.fill([-1, 1], [29, 30], [z, z + 1], thorn_hi);
    }
    for s in [1, -1] {
        // small, deep-set, angry: an ember under a slanted brow
        v.set(mi(s, 3), 21, 23, eye);
        v.set(mi(s, 4), 21, 23, eye);
        v.set(mi(s, 2), 21, 23, brow);
        v.fill(mx(s, 3, 6), [22, 23], [23, 24], brow);
        v.set(mi(s, 5), 23, 23, brow);
        v.set(mi(s, 2), 22, 23, hide_d);
        // ears
        v.fill(mx(s, 4, 7), [25, 28], [12, 15], hide_d);
        v.fill(mx(s, 5, 7), [28, 30], [12, 14], hide_d);
        // tusks: out of the jaw, sweeping up and back
        for (dx, y, z, c) in [
            (4, 12, 27, ivory_d),
            (5, 12, 27, ivory_d),
            (5, 13, 27, ivory),
            (5, 14, 27, ivory),
            (5, 15, 26, ivory),
            (6, 15, 27, ivory),
            (6, 16, 26, ivory),
            (6, 17, 26, ivory),
            (6, 18, 25, ivory),
            (6, 19, 25, ivory_d),
        ] {
            v.set(mi(s, dx), y, z, c);
            v.set(mi(s, dx), y, z + 1, c);
        }
    }

    lods(&v, |v| Beast {
        body: v.mesh(G_BODY, [0.0; 3]),
        head: v.mesh(G_HEAD, NECK),
        legs: [0, 1, 2, 3].map(|i| v.mesh(G_LEG[i], [LEGS[i].0, HIP_Y, LEGS[i].1])),
        neck: NECK,
        hip_y: HIP_Y,
        legs_at: LEGS,
        grazes: false,
    })
}

/// A leg or claw of a crawler: its mesh, where it hinges, which side it is
/// on (+1 or -1) and where in the stride it steps.
struct Limb {
    mesh: Mesh,
    pivot: [f32; 3],
    side: f32,
    phase: f32,
}

/// A many-legged thing: a body, legs that step in turn, and claws.
struct Crawler {
    body: Mesh,
    legs: Vec<Limb>,
    claws: Vec<Limb>,
    /// legs reach fore and aft and the body travels sideways
    sideways: bool,
}

/// Lay a jointed limb through the given points, about two voxels thick.
fn strut(v: &mut Vox, path: &[[f32; 3]], thick: f32, ink: impl Fn(usize) -> Ink) {
    let mut n = 0;
    for pair in path.windows(2) {
        let (a, b) = (Vec3::from(pair[0]), Vec3::from(pair[1]));
        let steps = (a.distance(b) * 1.5).ceil() as usize;
        for k in 0..=steps {
            let p = a.lerp(b, k as f32 / steps.max(1) as f32);
            v.blob(p.into(), [thick; 3], ink(n));
            n += 1;
        }
    }
}

/// A mirepincher: a flat swamp crab with a mottled shell, eyes on stalks,
/// two heavy claws and six legs. Its face is to its right (+x): it scuttles
/// sideways.
fn build_mirepincher(color: [f32; 3]) -> [Crawler; 3] {
    let mut v = Vox::new([-14, 0, -26], [28, 26, 26]);
    let shell = v.ink(color);
    let shell_d = v.tone(color, 0.66);
    let shell_hi = v.ink(mix(scale(color, 1.3), [0.8, 0.8, 0.5], 0.25));
    let moss = v.ink(mix(color, [0.20, 0.38, 0.16], 0.6));
    let belly = v.ink(mix(color, [0.85, 0.80, 0.62], 0.65));
    let claw = v.ink(mix(color, [0.70, 0.42, 0.22], 0.55));
    let claw_d = v.tone(mix(color, [0.70, 0.42, 0.22], 0.55), 0.7);
    let leg = v.ink(mix(color, [0.70, 0.42, 0.22], 0.25));
    let tip = v.ink([0.93, 0.88, 0.72]);
    let black = v.ink([0.04, 0.04, 0.05]);
    let white = v.ink([0.95, 0.95, 0.92]);

    v.part = 1;
    // a low dome over a pale plate
    v.blob([0.0, 8.5, 0.0], [10.5, 6.0, 12.5], shell);
    v.cut([-12, 12], [0, 7], [-14, 14]);
    v.rbox([-9, 9], [5, 7], [-11, 11], 1, [true, false, true], belly);
    // mottling in broad blotches, a pale rim, and a few warts
    v.repaint([-12, 12], [7, 16], [-14, 14], |x, y, z, _| {
        let blotch = hash3(x.div_euclid(3), 1, z.div_euclid(3)) % 7;
        if y == 7 {
            shell_hi
        } else if blotch < 2 {
            shell_d
        } else if blotch == 2 {
            moss
        } else {
            shell
        }
    });
    for (x, z) in [(-3, -5), (2, 4), (-4, 6), (4, -6), (0, -1)] {
        let top = (7..20).rev().find(|y| v.solid(x, *y, z)).unwrap_or(12);
        v.set(x, top + 1, z, shell_hi);
    }
    for s in [1, -1] {
        // eyes on stalks at the front
        v.fill([7, 8], [10, 15], mx(s, 2, 3), shell_d);
        v.fill([7, 9], [15, 17], mx(s, 1, 3), black);
        v.set(8, 16, mi(s, 2), white);
        // feelers
        v.fill([9, 11], [8, 9], mx(s, 0, 1), claw_d);
    }

    let mut claws = Vec::new();
    for (i, s) in [1, -1].into_iter().enumerate() {
        v.part = 2 + i as u8;
        // an arm, then a great pincer with a pale biting edge
        v.fill([6, 13], [6, 9], mx(s, 6, 9), claw_d);
        v.rbox([12, 23], [4, 12], mx(s, 4, 11), 2, [true; 3], claw);
        v.cut([18, 23], [7, 8], mx(s, 4, 11));
        v.repaint([17, 23], [6, 7], mx(s, 4, 11), |_, _, _, _| tip);
        v.repaint([17, 23], [8, 9], mx(s, 4, 11), |_, _, _, _| tip);
        v.repaint([12, 23], [11, 12], mx(s, 4, 11), |_, _, _, _| claw_d);
        v.repaint([12, 16], [4, 12], mx(s, 4, 11), |x, y, z, c| {
            if hash3(x, y, z).is_multiple_of(5) {
                claw_d
            } else {
                c
            }
        });
        claws.push((
            2 + i as u8,
            [7.0, 7.5, s as f32 * 7.5],
            s as f32,
            i as f32 * 2.0,
        ));
    }

    let mut legs = Vec::new();
    for (i, s) in [1.0f32, -1.0].into_iter().enumerate() {
        for (k, x) in [-5.0f32, -0.5, 4.0].into_iter().enumerate() {
            let part = 4 + (i * 3 + k) as u8;
            v.part = part;
            let splay = (k as f32 - 1.0) * 2.5;
            strut(
                &mut v,
                &[
                    [x, 7.5, s * 9.0],
                    [x + splay * 0.5, 12.5, s * 15.0],
                    [x + splay, 0.8, s * 20.0],
                ],
                1.2,
                |n| if n % 5 == 4 { shell_d } else { leg },
            );
            v.blob([x + splay, 1.0, s * 20.0], [1.0, 1.4, 1.0], tip);
            let phase = if (i + k) % 2 == 0 {
                0.0
            } else {
                std::f32::consts::PI
            };
            legs.push((part, [x, 7.5, s * 9.0], s, phase));
        }
    }

    lods(&v, |v| Crawler {
        body: v.mesh(1, [0.0; 3]),
        legs: limbs(v, &legs),
        claws: limbs(v, &claws),
        sideways: true,
    })
}

fn limbs(v: &Vox, list: &[(u8, [f32; 3], f32, f32)]) -> Vec<Limb> {
    list.iter()
        .map(|(part, pivot, side, phase)| Limb {
            mesh: v.mesh(*part, *pivot),
            pivot: *pivot,
            side: *side,
            phase: *phase,
        })
        .collect()
}

/// A websnare: a wide, low weaver on eight arched legs, pale with dark
/// bands, a cluster of glowing eyes, and sacs of silk under its tail.
fn build_websnare(color: [f32; 3]) -> [Crawler; 3] {
    let mut v = Vox::new([-30, 0, -26], [30, 30, 28]);
    let pale = v.ink(color);
    let pale_hi = v.ink(mix(color, [1.0, 1.0, 1.0], 0.4));
    let band = v.ink(mix(scale(color, 0.3), [0.12, 0.08, 0.14], 0.5));
    let dusk = v.tone(color, 0.62);
    let silk = v.ink([0.95, 0.95, 0.90]);
    let silk_d = v.ink([0.76, 0.78, 0.76]);
    let fang = v.ink([0.10, 0.07, 0.09]);
    let fang_tip = v.ink([0.90, 0.85, 0.70]);
    let eye = v.glow([1.0, 0.30, 0.38]);
    let eye_hi = v.glow([1.0, 0.78, 0.70]);

    v.part = 1;
    // a swollen abdomen, banded across, with a pale mark like an hourglass
    v.blob([0.0, 12.5, -10.0], [8.5, 7.5, 10.5], pale);
    v.repaint([-10, 10], [4, 22], [-22, 2], |x, y, z, _| {
        let stripe = (z + 40).rem_euclid(5) == 0 && y > 9;
        let mark =
            (-2..2).contains(&x) && y > 17 && ((-13..-11).contains(&z) || (-8..-6).contains(&z));
        if mark {
            pale_hi
        } else if stripe {
            band
        } else if y < 9 {
            dusk
        } else {
            pale
        }
    });
    // silk sacs slung beneath, and a spinneret
    for (x, y, z, r) in [
        (-4.5, 6.5, -17.0, 3.2),
        (4.5, 6.5, -17.0, 3.2),
        (0.0, 5.5, -20.5, 3.4),
        (0.0, 8.0, -13.0, 2.6),
    ] {
        v.blob([x, y, z], [r, r, r], silk);
    }
    v.repaint([-9, 9], [2, 10], [-25, -9], |x, y, z, c| {
        if c == silk && (x + y + z).rem_euclid(3) == 0 {
            silk_d
        } else {
            c
        }
    });
    v.fill([-1, 1], [11, 13], [-22, -20], band);
    // the head end: darker, low and broad
    v.blob([0.0, 10.0, 5.0], [6.5, 4.5, 7.0], dusk);
    v.repaint([-7, 7], [12, 15], [-2, 12], |x, _, z, c| {
        if (x + z).rem_euclid(4) == 0 { band } else { c }
    });
    // eyes: two great ones and six lesser, all aglow
    for s in [1, -1] {
        v.fill(mx(s, 1, 3), [10, 12], [11, 13], eye);
        v.set(mi(s, 1), 11, 12, eye_hi);
        v.set(mi(s, 4), 11, 11, eye);
        v.set(mi(s, 4), 12, 10, eye);
        v.set(mi(s, 2), 13, 10, eye);
        v.set(mi(s, 0), 13, 11, eye_hi);
        // fangs
        v.fill(mx(s, 1, 3), [5, 9], [11, 13], fang);
        v.fill(mx(s, 1, 2), [3, 5], [12, 13], fang_tip);
    }

    let mut legs = Vec::new();
    for (i, s) in [1.0f32, -1.0].into_iter().enumerate() {
        for (k, (z, reach)) in [(9.0f32, 8.0f32), (6.0, 3.0), (3.0, -3.0), (0.0, -9.0)]
            .into_iter()
            .enumerate()
        {
            let part = 2 + (i * 4 + k) as u8;
            v.part = part;
            // up to a high knee, then down to a foot far out
            strut(
                &mut v,
                &[
                    [s * 4.5, 10.0, z],
                    [s * 13.0, 21.0, z + reach * 0.5],
                    [s * 22.0, 12.0, z + reach * 0.85],
                    [s * 26.0, 0.8, z + reach],
                ],
                1.15,
                |n| if n % 7 >= 5 { band } else { pale },
            );
            let phase = if (i + k) % 2 == 0 {
                0.0
            } else {
                std::f32::consts::PI
            };
            legs.push((part, [s * 4.5, 10.0, z], s, phase));
        }
    }

    lods(&v, |v| Crawler {
        body: v.mesh(1, [0.0; 3]),
        legs: limbs(v, &legs),
        claws: Vec::new(),
        sideways: false,
    })
}

/// A gloomwisp: a will-o'-the-wisp. A bright core in a violet veil, motes
/// circling it, and a tail that trails behind.
struct Wisp {
    core: Mesh,
    veil: Mesh,
    mote: Mesh,
    tail: [Mesh; 4],
}

fn build_wisp(color: [f32; 3]) -> [Wisp; 3] {
    let mut v = Vox::new([-12, -12, -12], [12, 12, 12]);
    let violet = mix(color, [0.62, 0.38, 1.0], 0.6);
    let hot = v.glow(mix(color, [1.0, 1.0, 1.0], 0.7));
    let warm = v.glow(color);
    let cool = v.glow(mix(color, violet, 0.5));
    let dim = v.glow(violet);
    let deep = v.glow(scale(violet, 0.7));
    v.part = 1;
    for y in -5i32..5 {
        for z in -5i32..5 {
            for x in -5i32..5 {
                // an octahedron softened towards a ball, brightest within
                let (a, b, c) = (
                    (x as f32 + 0.5).abs(),
                    (y as f32 + 0.5).abs(),
                    (z as f32 + 0.5).abs(),
                );
                let (taxi, round) = (a + b + c, a * a + b * b + c * c);
                if taxi < 6.6 && round < 23.0 {
                    let ink = if taxi < 3.0 || (x + y + z).rem_euclid(3) == 0 {
                        hot
                    } else if round > 15.0 {
                        cool
                    } else {
                        warm
                    };
                    v.set(x, y, z, ink);
                }
            }
        }
    }
    // the veil: tatters of violet light standing off the core
    v.part = 2;
    for y in -8i32..8 {
        for z in -8i32..8 {
            for x in -8i32..8 {
                let r = ((x as f32 + 0.5).powi(2)
                    + (y as f32 + 0.5).powi(2) * 0.8
                    + (z as f32 + 0.5).powi(2))
                .sqrt();
                let h = hash3(x.div_euclid(2), y.div_euclid(3), z.div_euclid(2)) % 9;
                if (6.2..7.6).contains(&r) && h < 2 && v.get(x, y, z) == 0 {
                    v.set(x, y, z, if y > 2 { dim } else { deep });
                }
            }
        }
    }
    v.part = 3;
    v.rbox([-1, 1], [-1, 2], [-1, 1], 0, [true; 3], warm);
    v.set(-1, 1, 0, hot);
    // the tail: four fading puffs, each its own small model
    let puff = |i: usize| -> Vox {
        let mut p = Vox::new([-5, -5, -5], [5, 5, 5]);
        let tones = [color, mix(color, violet, 0.5), violet, scale(violet, 0.7)];
        let ink = p.glow(tones[i]);
        let core = p.glow(mix(color, [1.0, 1.0, 1.0], 0.7));
        p.part = 1;
        let r = 3.4 - i as f32 * 0.7;
        p.blob([0.0, 0.0, 0.0], [r, r * 1.15, r], ink);
        if i < 2 {
            p.blob([0.0, 0.0, 0.0], [r * 0.5; 3], core);
        }
        p
    };
    let puffs = [0, 1, 2, 3].map(puff);
    let tails = [
        puffs[0].half(),
        puffs[1].half(),
        puffs[2].half(),
        puffs[3].half(),
    ];
    let far = [
        tails[0].half(),
        tails[1].half(),
        tails[2].half(),
        tails[3].half(),
    ];
    let half = v.half();
    let quarter = half.half();
    let build = |v: &Vox, tail: &[Vox; 4]| Wisp {
        core: v.mesh(1, [0.0; 3]),
        veil: v.mesh(2, [0.0; 3]),
        mote: v.mesh(3, [0.0; 3]),
        tail: [0, 1, 2, 3].map(|i| tail[i].mesh(1, [0.0; 3])),
    };
    [
        build(&v, &puffs),
        build(&half, &tails),
        build(&quarter, &far),
    ]
}

/// The look of an Ashcloak: a ragged hooded cloak in the creature's colour
/// over leather, a scarf across the face. The leader wears mail and a red
/// sash.
fn ashcloak_look(color: [f32; 3], leader: bool) -> Look {
    let mut l = Look::player(if leader { 11 } else { 5 }, color);
    l.skin = if leader {
        [0.72, 0.52, 0.38]
    } else {
        [0.84, 0.66, 0.52]
    };
    l.hair = [0.16, 0.12, 0.10];
    l.hair_style = 3;
    l.beard = false;
    l.tunic = color;
    l.trim = [0.50, 0.42, 0.30];
    l.pants = [0.22, 0.20, 0.19];
    l.boots = [0.20, 0.14, 0.10];
    l.cape = Some(scale(color, 0.78));
    l.armor = if leader {
        ArmorType::Mail
    } else {
        ArmorType::Leather
    };
    l.pauldrons = leader;
    l.gloves = true;
    l.robe = false;
    l.apron = None;
    l.helmet = false;
    l.weapon = None;
    l.eyes = [0.30, 0.24, 0.18];
    l.build = if leader { 2 } else { 1 };
    l.elder = false;
    l.emblem = 0;
    l.ears = 0;
    l.mood = 1;
    l.scarf = Some(if leader {
        [0.52, 0.12, 0.10]
    } else {
        scale(color, 0.5)
    });
    l.sash = leader.then_some([0.72, 0.13, 0.10]);
    l
}

/// The look of a Candlekin: a wax-pale little mine dweller with great ears
/// and a lit candle stuck to its head.
fn candlekin_look(color: [f32; 3]) -> Look {
    let mut l = Look::player(2, color);
    l.skin = color;
    l.hair_style = 0;
    l.beard = false;
    l.tunic = [0.36, 0.27, 0.19];
    l.trim = [0.52, 0.40, 0.24];
    l.pants = [0.24, 0.19, 0.15];
    l.boots = scale(color, 0.7);
    l.cape = None;
    l.armor = ArmorType::Cloth;
    l.pauldrons = false;
    l.gloves = false;
    l.robe = false;
    l.apron = None;
    l.helmet = false;
    l.weapon = None;
    l.eyes = [0.98, 0.78, 0.20];
    l.build = 0;
    l.elder = false;
    l.emblem = 0;
    l.ears = 3;
    l.mood = 0;
    l.candle = true;
    l
}

thread_local! {
    static BEASTS: Cache<([Beast; 3], anim::QuadRig)> = Cache::default();
    static CRAWLERS: Cache<([Crawler; 3], anim::MultiRig)> = Cache::default();
    static SHADES: Cache<[Shade; 3]> = Cache::default();
    static WISPS: Cache<[Wisp; 3]> = Cache::default();
}

fn color_key(variant: u8, c: [f32; 3]) -> u64 {
    key_of(&[
        variant as u32,
        c[0].to_bits(),
        c[1].to_bits(),
        c[2].to_bits(),
    ])
}

/// The rig of a beast, measured from its meshes.
fn quad_rig(m: &Beast) -> anim::QuadRig {
    // the widest part of it, legs and all: how high it lies on its side
    let widest = |mesh: &Mesh, x: f32| {
        mesh.verts
            .iter()
            .map(|v| (v.pos[0] + x).abs())
            .fold(0.0f32, f32::max)
    };
    let half_width = m
        .legs
        .iter()
        .zip(&m.legs_at)
        .map(|(l, (x, _))| widest(l, *x))
        .fold(widest(&m.body, 0.0).max(widest(&m.head, 0.0)), f32::max)
        .max(4.0);
    let half_length = m
        .legs_at
        .iter()
        .map(|(_, z)| z.abs())
        .fold(0.0f32, f32::max);
    anim::QuadRig {
        grazes: m.grazes,
        hip_y: m.hip_y,
        half_width,
        half_length,
    }
}

/// The rig of a crawler, measured from its meshes.
fn multi_rig(m: &Crawler) -> anim::MultiRig {
    let height = m.body.verts.iter().map(|v| v.pos[1]).fold(0.0f32, f32::max);
    // the front pair: the foremost leg on each side
    let front = |l: &Limb| {
        !m.legs
            .iter()
            .any(|o| o.side == l.side && o.pivot[2] > l.pivot[2] + 0.1)
    };
    anim::MultiRig {
        legs: m
            .legs
            .iter()
            .map(|l| anim::LegRig {
                side: l.side,
                phase: l.phase,
                front: front(l),
            })
            .collect(),
        claws: m.claws.len(),
        sideways: m.sideways,
        height,
    }
}

/// What a humanoid creature looks like and holds: Ashcloak bandit, Ashcloak
/// leader, Candlekin.
fn humanoid_look(variant: u8, color: [f32; 3]) -> (Look, (Kind, [f32; 3])) {
    match variant {
        1 => (
            ashcloak_look(color, false),
            (Kind::Dagger, [0.62, 0.63, 0.68]),
        ),
        2 => (
            ashcloak_look(color, true),
            (Kind::BattleAxe, [0.55, 0.56, 0.62]),
        ),
        _ => (candlekin_look(color), (Kind::Pickaxe, [0.50, 0.50, 0.55])),
    }
}

/// The attack a creature makes: its weapon for those that hold one, else
/// its natural attack (bite, gore, claw snap, rake, bolt).
pub fn creature_attack(model: dyno_world::content::Model, variant: u8) -> Anim {
    match Skeleton::of(model, variant) {
        Skeleton::Biped => anim::attack_for(grip_of(Some(humanoid_look(variant, [0.5; 3]).1))),
        Skeleton::Wraith | Skeleton::Quadruped | Skeleton::Multileg | Skeleton::Wisp => {
            Anim::AttackUnarmed
        }
    }
}

/// Draw a creature. `variant` picks the look within its model kind:
/// quadrupeds are 0 Grazer, 1 Thornboar, 2 Mirepincher, 3 Websnare;
/// humanoids 0 Shade, 1 Ashcloak bandit, 2 Ashcloak leader, 3 Candlekin; the
/// orb is the Gloomwisp. `flash` (0 to 1) whitens it when hit; `scale` is its
/// size. It is posed by `animator` at `now` (seconds).
#[allow(clippy::too_many_arguments)]
pub fn push_creature(
    out: &mut Vec<Vertex>,
    model: dyno_world::content::Model,
    variant: u8,
    feet: Vec3,
    yaw: f32,
    color: [f32; 3],
    light: f32,
    flash: f32,
    scale: f32,
    animator: &mut Animator,
    now: f32,
) {
    let paint = Paint {
        flash,
        ..Paint::lit(if flash > 0.0 { 1.0 } else { light })
    };
    let lod = lod_at(feet);
    let key = color_key(variant, color);
    let skeleton = Skeleton::of(model, variant);
    let upper = skeleton.upper();
    let base = root(feet, yaw, FINE / scale);
    match skeleton {
        Skeleton::Multileg => {
            let m = CRAWLERS.with(|c| {
                c.get(key, || {
                    let m = if variant == 2 {
                        build_mirepincher(color)
                    } else {
                        build_websnare(color)
                    };
                    let rig = multi_rig(&m[0]);
                    (m, rig)
                })
            });
            let (m, rig) = (&m.0[lod], &m.1);
            let pose = animator.pose(now, upper, |a, at| anim::multileg(a, at, rig));
            use anim::multi::*;
            let body = bone(&base, Vec3::ZERO, &pose, BODY);
            emit(out, &m.body, &body, paint);
            for (i, leg) in m.legs.iter().take(MAX_LEGS).enumerate() {
                let at = Vec3::from(leg.pivot);
                emit(out, &leg.mesh, &bone(&body, at, &pose, LEG0 + i), paint);
            }
            for (i, claw) in m.claws.iter().take(2).enumerate() {
                let at = Vec3::from(claw.pivot);
                emit(out, &claw.mesh, &bone(&body, at, &pose, CLAW[i]), paint);
            }
        }
        Skeleton::Quadruped => {
            let m = BEASTS.with(|c| {
                c.get(key, || {
                    let m = if variant == 1 {
                        build_thornboar(color)
                    } else {
                        build_grazer(color)
                    };
                    let rig = quad_rig(&m[0]);
                    (m, rig)
                })
            });
            let (m, rig) = (&m.0[lod], &m.1);
            let pose = animator.pose(now, upper, |a, at| anim::quadruped(a, at, rig));
            use anim::quad::*;
            // the body turns about the middle of its back at hip height
            let mid = Vec3::new(0.0, m.hip_y, 0.0);
            let body = bone(&base, mid, &pose, BODY) * Affine3A::from_translation(-mid);
            for (i, (lx, lz)) in m.legs_at.iter().enumerate() {
                let hip = bone(&body, Vec3::new(*lx, m.hip_y, *lz), &pose, LEGS[i]);
                emit(out, &m.legs[i], &hip, paint);
            }
            emit(out, &m.body, &body, paint);
            let head = bone(&body, Vec3::from(m.neck), &pose, HEAD);
            emit(out, &m.head, &head, paint);
        }
        Skeleton::Biped => {
            let (look, held) = humanoid_look(variant, color);
            let rig = BipedRig {
                grip: grip_of(Some(held)),
                robe: look.robe,
            };
            let pose = animator.pose(now, upper, |a, at| anim::biped(a, at, &rig));
            draw_person(out, feet, yaw, 0.0, &look, paint, &pose, Some(held), scale);
        }
        Skeleton::Wraith => {
            let m = SHADES.with(|c| c.get(key, || build_shade(color)));
            let m = &m[lod];
            let pose = animator.pose(now, upper, anim::wraith);
            use anim::wraith::*;
            let clock = pose.k[anim::key::CLOCK];
            let flow = pose.k[anim::key::FLOW];
            let hover = Vec3::Y * 4.0;
            let body = bone(&base, Vec3::from(S_WAIST) + hover, &pose, BODY);
            emit(out, &m.body, &body, paint);
            // the rags trail behind and sway on their own
            for (i, rag) in m.tatters.iter().enumerate() {
                let k = i as f32;
                let sway = (clock * 3.0 + k * 2.1).sin();
                let trail = flow + (clock * 2.3 + k * 1.3).sin() * 0.1;
                let rot = rx(trail) * Mat3::from_rotation_z(sway * 0.14);
                let at = Vec3::new(S_TATTER_X[i], S_HEM_Y - S_WAIST[1], 0.0);
                emit(out, rag, &joint(&body, at, rot), paint);
            }
            let hood = bone(
                &body,
                Vec3::new(0.0, S_NECK[1] - S_WAIST[1], 0.0),
                &pose,
                HOOD,
            );
            emit(out, &m.hood, &hood, paint);
            // the eyes glow whatever the light, and shut when it blinks or dies
            let open = pose.k[anim::key::SHAPE].clamp(0.1, 1.5);
            let eyes = joint(
                &hood,
                Vec3::new(0.0, S_EYE[1] - S_NECK[1], S_EYE[2]),
                Mat3::from_diagonal(Vec3::new(1.0, open, 1.0)),
            );
            emit(
                out,
                &m.eyes,
                &eyes,
                Paint {
                    light: 1.0,
                    glow: true,
                    ..paint
                },
            );
            // long reaching arms
            for (i, s) in [1.0f32, -1.0].into_iter().enumerate() {
                let shoulder = bone(
                    &body,
                    Vec3::new(s * S_SHOULDER.0, S_SHOULDER.1 - S_WAIST[1], 0.0),
                    &pose,
                    ARM_U[i],
                );
                emit(out, &m.arm_u[i], &shoulder, paint);
                let fore = bone(
                    &shoulder,
                    Vec3::new(0.0, S_ELBOW_Y - S_SHOULDER.1, 0.0),
                    &pose,
                    ARM_L[i],
                );
                emit(out, &m.arm_l[i], &fore, paint);
            }
        }
        Skeleton::Wisp => {
            let m = WISPS.with(|c| c.get(key, || build_wisp(color)));
            let m = &m[lod];
            let pose = animator.pose(now, upper, anim::wisp);
            use anim::wisp::*;
            let t = pose.k[anim::key::CLOCK];
            let glow = Paint {
                glow: true,
                ..paint
            };
            let base = bone(&base, Vec3::new(0.0, 0.85 * FINE, 0.0), &pose, ROOT);
            let size = pose.k[anim::key::SHAPE].max(0.2);
            let spin = Mat3::from_rotation_y(t * 0.9) * Mat3::from_rotation_x(0.3);
            let core = base * Affine3A::from_mat3(spin * Mat3::from_diagonal(Vec3::splat(size)));
            emit(out, &m.core, &core, glow);
            let veil = Mat3::from_rotation_y(-t * 0.6)
                * Mat3::from_diagonal(Vec3::splat(pose.off[VEIL].x.max(0.2)));
            emit(out, &m.veil, &(base * Affine3A::from_mat3(veil)), glow);
            // motes circle on tilted rings
            let spread = pose.k[anim::key::SPREAD].max(0.2);
            for k in 0..5 {
                let kf = k as f32;
                let a = t * (1.1 + kf * 0.23) + kf * 1.9;
                let r = (9.5 + kf * 1.2) * spread;
                let p = Mat3::from_rotation_z(0.5 * kf - 1.0)
                    * Vec3::new(a.cos() * r, (a * 1.7).sin() * 2.0, a.sin() * r);
                let spin = Mat3::from_rotation_y(a * 2.0) * Mat3::from_rotation_x(a);
                emit(out, &m.mote, &joint(&base, p, spin), glow);
            }
            // the tail hangs below at rest and streams behind when it moves
            let flow = pose.k[anim::key::FLOW];
            let back = flow.max(0.0);
            let dir =
                Vec3::new(0.0, -(1.0 - (flow - 0.25).max(0.0) * 0.66), -back).normalize_or_zero();
            let fade = size.min(1.0);
            for (i, puff) in m.tail.iter().enumerate() {
                let kf = i as f32;
                let sway = (t * 3.1 - kf * 0.9).sin() * (1.0 + kf * 0.8);
                let p = dir * (6.0 + kf * 4.4) * fade
                    + Vec3::new(sway, 0.0, (t * 2.3 - kf).cos() * 0.6);
                let s = (1.0 + (t * 5.0 - kf * 1.3).sin() * 0.1) * fade;
                let rot = Mat3::from_rotation_y(t + kf) * Mat3::from_diagonal(Vec3::splat(s));
                emit(out, puff, &joint(&base, p, rot), glow);
            }
        }
    }
}

// ------------------------------------------------------------ held items

/// A held thing: its mesh, and how big a voxel of it is in blocks.
struct Held {
    mesh: Mesh,
    /// blocks per voxel at true size
    unit: f32,
    /// turn about the grip (radians) when carried by a figure
    carry_turn: f32,
}

/// What to model for an item.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Sword,
    BattleAxe,
    Mace,
    Dagger,
    Staff,
    Polearm,
    Bow,
    Wand,
    Shield,
    Tome,
    Pickaxe,
    Hatchet,
    Shovel,
    Hoe,
    Torch,
    Stick,
    Lump,
    Ingot,
    Gem,
    Dust,
    Berries,
    Meat,
    Helm,
    Garment,
    Ring,
    Hand,
}

fn weapon_kind(k: WeaponKind) -> Kind {
    match k {
        WeaponKind::Sword => Kind::Sword,
        WeaponKind::Axe => Kind::BattleAxe,
        WeaponKind::Mace => Kind::Mace,
        WeaponKind::Dagger => Kind::Dagger,
        WeaponKind::Staff => Kind::Staff,
        WeaponKind::Polearm => Kind::Polearm,
        WeaponKind::Bow => Kind::Bow,
        WeaponKind::Wand => Kind::Wand,
        WeaponKind::Shield => Kind::Shield,
        WeaponKind::Offhand => Kind::Tome,
    }
}

/// The accent colour of a quality: poor, common, uncommon, rare, epic.
fn quality_color(q: u8) -> [f32; 3] {
    match q {
        0 => [0.55, 0.55, 0.55],
        1 => [0.85, 0.68, 0.25],
        2 => [0.30, 0.85, 0.35],
        3 => [0.30, 0.55, 1.00],
        _ => [0.72, 0.35, 0.95],
    }
}

const WOOD: [f32; 3] = [0.50, 0.35, 0.20];
const GOLD: [f32; 3] = [0.85, 0.66, 0.25];

/// Paint a shape from rows of text, top row first, centred on x: `#` main,
/// `+` light, `-` dark, `*` accent. Each row is one voxel high.
#[allow(clippy::too_many_arguments)]
fn stamp(v: &mut Vox, art: &[&str], top: i32, z: [i32; 2], main: Ink, hi: Ink, lo: Ink, acc: Ink) {
    for (row, line) in art.iter().enumerate() {
        let w = line.len() as i32;
        for (col, ch) in line.bytes().enumerate() {
            let c = match ch {
                b'#' => main,
                b'+' => hi,
                b'-' => lo,
                b'*' => acc,
                _ => continue,
            };
            let x = col as i32 - w / 2;
            for zz in z[0]..z[1] {
                v.set(x, top - row as i32, zz, c);
            }
        }
    }
}

/// A wooden haft from `y0` to `y1`, 2 x 2, with grain and a wrapped grip.
fn haft(v: &mut Vox, y0: i32, y1: i32, grip: [i32; 2]) {
    let wood = v.ink(WOOD);
    let wood_d = v.tone(WOOD, 0.78);
    let wood_hi = v.tone(WOOD, 1.2);
    let wrap = v.ink([0.30, 0.18, 0.10]);
    let wrap_hi = v.ink([0.42, 0.27, 0.16]);
    for y in y0..y1 {
        let c = if (grip[0]..grip[1]).contains(&y) {
            if y.rem_euclid(3) == 0 { wrap_hi } else { wrap }
        } else if y.rem_euclid(7) == 3 {
            wood_d
        } else {
            wood
        };
        v.fill([-1, 1], [y, y + 1], [-1, 1], c);
        if !(grip[0]..grip[1]).contains(&y) && y.rem_euclid(7) != 3 {
            v.set(0, y, 0, wood_hi);
        }
    }
}

fn build_held(kind: Kind, color: [f32; 3], quality: u8, glows: bool, frame: u8) -> Held {
    let mut v = Vox::new([-24, -40, -8], [24, 72, 8]);
    v.part = 1;
    let main = v.ink(color);
    let hi = v.ink(mix(scale(color, 1.3), [1.0, 1.0, 1.0], 0.2));
    let lo = v.tone(color, 0.68);
    let steel = v.ink(STEEL);
    let steel_d = v.tone(STEEL, 0.7);
    let gold = v.ink(GOLD);
    let gold_d = v.tone(GOLD, 0.7);
    let qc = quality_color(quality);
    // good things carry a gem that shines; ember things burn from within
    let gem = if quality >= 2 { v.glow(qc) } else { v.ink(qc) };
    let ember = if glows {
        v.glow(mix(color, [1.0, 0.9, 0.5], 0.4))
    } else {
        lo
    };
    let leather = v.ink([0.30, 0.18, 0.10]);
    let leather_hi = v.ink([0.44, 0.28, 0.16]);
    // (length in blocks of the whole thing, voxels it spans, carry turn)
    let (real, span, carry_turn) = match kind {
        Kind::Sword => {
            // pommel, wrapped grip, flared guard, fullered blade
            v.rbox([-2, 2], [-10, -6], [-2, 2], 1, [true; 3], gold);
            v.set(-1, -8, 1, gem);
            v.set(0, -8, 1, gem);
            v.set(-1, -8, -2, gem);
            v.set(0, -8, -2, gem);
            for y in -6i32..4 {
                let c = if y.rem_euclid(2) == 0 {
                    leather
                } else {
                    leather_hi
                };
                v.fill([-1, 1], [y, y + 1], [-1, 1], c);
            }
            stamp(
                &mut v,
                &[
                    "++..............++",
                    "+##............##+",
                    ".################.",
                    "..--############--..",
                ],
                7,
                [-2, 2],
                gold,
                gold,
                gold_d,
                gem,
            );
            v.fill([-2, 2], [4, 8], [-2, 2], gold);
            v.fill([-1, 1], [5, 7], [2, 3], gem);
            v.fill([-1, 1], [5, 7], [-3, -2], gem);
            for y in 8..52 {
                // the blade narrows to a point over its last rows
                let half = if y >= 46 { (52 - y + 1) / 2 + 1 } else { 4 };
                for x in -half..half {
                    let edge = x == -half || x == half - 1;
                    let fuller = (-1..1).contains(&x) && y < 40;
                    let c = if edge {
                        hi
                    } else if fuller {
                        ember
                    } else {
                        main
                    };
                    v.fill([x, x + 1], [y, y + 1], [-1, 1], c);
                }
            }
            (1.15, 62.0, 0.0)
        }
        Kind::Dagger => {
            v.rbox([-1, 1], [-7, -5], [-1, 1], 0, [true; 3], gold);
            for y in -5i32..2 {
                let c = if y.rem_euclid(2) == 0 {
                    leather
                } else {
                    leather_hi
                };
                v.fill([-1, 1], [y, y + 1], [-1, 1], c);
            }
            stamp(
                &mut v,
                &["+........+", "##########", ".--####--."],
                4,
                [-1, 1],
                gold,
                gold,
                gold_d,
                gem,
            );
            v.fill([-1, 1], [2, 4], [1, 2], gem);
            for y in 5..26 {
                let half = if y >= 22 { (26 - y + 1) / 2 } else { 2 };
                for x in -half..half {
                    let c = if x == half - 1 || x == -half {
                        hi
                    } else if y < 18 {
                        ember
                    } else {
                        main
                    };
                    v.fill([x, x + 1], [y, y + 1], [-1, 1], c);
                }
            }
            (0.62, 33.0, 0.0)
        }
        Kind::BattleAxe | Kind::Hatchet => {
            let big = kind == Kind::BattleAxe;
            let top = if big { 40 } else { 30 };
            haft(&mut v, -12, top, [-9, 2]);
            v.fill([-2, 2], [-14, -12], [-2, 2], steel);
            // a bearded blade with a bright bevelled edge
            let art: &[&str] = if big {
                &[
                    "...........##+.",
                    ".........####++",
                    ".--.....#####++",
                    "-###...######++",
                    "-############++",
                    "-############++",
                    "-###**#######++",
                    "-###**#######++",
                    "-############++",
                    "-###...######++",
                    ".--.....#####++",
                    ".........####++",
                    "..........###++",
                    "...........##++",
                    "............#+.",
                ]
            } else {
                &[
                    "........##+",
                    "......####+",
                    "-########++",
                    "-###**###++",
                    "-###**###++",
                    "-########++",
                    "......####+",
                    ".......###+",
                    "........##+",
                ]
            };
            // the art is drawn with the haft under its `**`
            let w = art[0].len() as i32;
            let shift = w / 2 - 5;
            for (row, line) in art.iter().enumerate() {
                for (col, ch) in line.bytes().enumerate() {
                    let c = match ch {
                        b'#' => main,
                        b'+' => hi,
                        b'-' => lo,
                        b'*' => steel_d,
                        _ => continue,
                    };
                    let x = col as i32 - w / 2 + shift;
                    v.fill(
                        [x, x + 1],
                        [top - 2 - row as i32, top - 1 - row as i32],
                        [-1, 1],
                        c,
                    );
                }
            }
            if glows {
                v.fill([3, 5], [top - 10, top - 6], [-1, 1], ember);
            }
            v.fill([-2, 2], [top - 1, top + 1], [-2, 2], steel);
            if big {
                (1.25, 56.0, 0.0)
            } else {
                (0.85, 46.0, 0.0)
            }
        }
        Kind::Mace => {
            haft(&mut v, -10, 22, [-8, 2]);
            v.fill([-2, 2], [-12, -10], [-2, 2], steel);
            v.fill([-2, 2], [20, 23], [-2, 2], steel_d);
            // a flanged head with a spike
            v.rbox([-4, 4], [23, 33], [-4, 4], 2, [true; 3], main);
            for y in 24i32..32 {
                let reach = 6 - (y - 28).abs().max(1);
                v.fill([-reach, reach], [y, y + 1], [-1, 1], hi);
                v.fill([-1, 1], [y, y + 1], [-reach, reach], hi);
            }
            v.rbox([-4, 4], [27, 29], [-4, 4], 1, [true, false, true], lo);
            v.fill([-1, 1], [33, 37], [-1, 1], hi);
            v.set(-1, 28, 4, gem);
            v.set(0, 28, 4, gem);
            v.set(-1, 28, -5, gem);
            v.set(0, 28, -5, gem);
            if glows {
                v.fill([-1, 1], [36, 38], [-1, 1], ember);
            }
            (0.95, 50.0, 0.0)
        }
        Kind::Staff | Kind::Wand => {
            let staff = kind == Kind::Staff;
            let top = if staff { 50 } else { 16 };
            let bottom = if staff { -26 } else { -8 };
            haft(&mut v, bottom, top, if staff { [-6, 6] } else { [-8, -2] });
            if staff {
                v.fill([-2, 2], [bottom - 2, bottom + 1], [-2, 2], steel);
                for y in [14, 30] {
                    v.fill([-2, 2], [y, y + 2], [-2, 2], gold);
                }
                // three prongs curl around a crystal
                let stone = v.glow(if glows {
                    mix(color, [1.0, 0.9, 0.6], 0.3)
                } else {
                    qc
                });
                let stone_hi = v.glow([1.0, 1.0, 0.95]);
                let wood = v.ink(WOOD);
                let wood_d = v.tone(WOOD, 0.75);
                for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    for k in 0..12 {
                        let out = [1, 2, 3, 4, 4, 5, 5, 5, 4, 4, 3, 2][k as usize];
                        let span = |d: i32| match d {
                            1 => [out, out + 1],
                            -1 => [-out - 1, -out],
                            _ => [-1, 1],
                        };
                        let c = if k % 3 == 0 { wood_d } else { wood };
                        v.fill(span(dx), [top + k, top + k + 1], span(dz), c);
                    }
                }
                for y in 0i32..9 {
                    let r: i32 = 3 - (y - 4).abs() * 3 / 4;
                    for z in -r..r {
                        for x in -r..r {
                            if x.abs().max(x + 1) + z.abs().max(z + 1) <= r + 1 {
                                let c = if (x + y + z).rem_euclid(3) == 0 {
                                    stone_hi
                                } else {
                                    stone
                                };
                                v.set(x, top + 2 + y, z, c);
                            }
                        }
                    }
                }
                (1.9, 92.0, 0.0)
            } else {
                // a slim wand: banded handle, a bright star at the tip
                let tip = v.glow(if glows {
                    mix(color, [1.0, 0.9, 0.6], 0.3)
                } else {
                    qc
                });
                v.fill([-2, 2], [-9, -7], [-2, 2], gold);
                v.fill([-2, 2], [-2, 0], [-2, 2], gold);
                v.repaint([-1, 1], [0, top], [-1, 1], |_, y, _, c| {
                    if y % 4 == 0 { main } else { c }
                });
                v.fill([-1, 1], [top, top + 4], [-1, 1], tip);
                v.fill([-2, 2], [top + 1, top + 3], [-1, 1], tip);
                v.fill([-1, 1], [top + 1, top + 3], [-2, 2], tip);
                (0.6, 30.0, 0.0)
            }
        }
        Kind::Polearm => {
            haft(&mut v, -34, 40, [-6, 8]);
            v.fill([-2, 2], [-36, -33], [-2, 2], steel);
            v.fill([-2, 2], [38, 42], [-2, 2], steel_d);
            stamp(
                &mut v,
                &[
                    "....+.....",
                    "...+#+....",
                    "...+#+....",
                    "..+###+...",
                    "..+###+...",
                    "..+###+...",
                    "..+###+...",
                    ".+#####+..",
                    ".+#####+..",
                    ".+#####+..",
                    ".+#####+-.",
                    "..+###+.--",
                    "..+###+..-",
                    "...###..--",
                    "...###.--.",
                    "...######.",
                ],
                57,
                [-1, 1],
                main,
                hi,
                lo,
                gem,
            );
            (2.1, 96.0, 0.0)
        }
        Kind::Bow => {
            // limbs bend back from a wrapped grip; a pale string joins the tips
            let wood = v.ink(mix(color, WOOD, 0.5));
            let wood_d = v.tone(mix(color, WOOD, 0.5), 0.75);
            let string = v.ink([0.92, 0.90, 0.82]);
            for y in -32i32..33 {
                let t = y as f32 / 32.0;
                let bend = (9.0 * (1.0 - t * t)).round() as i32 - 6;
                let recurve = if y.abs() > 28 { (y.abs() - 28) / 2 } else { 0 };
                let x = bend - recurve;
                let thick = if y.abs() < 6 {
                    3
                } else if y.abs() < 24 {
                    2
                } else {
                    1
                };
                let c = if y.abs() < 5 {
                    if y.rem_euclid(2) == 0 {
                        leather
                    } else {
                        leather_hi
                    }
                } else if y.rem_euclid(6) == 0 {
                    wood_d
                } else {
                    wood
                };
                v.fill([x, x + thick + 1], [y, y + 1], [-1, 1], c);
                if y.abs() == 31 || y.abs() == 32 {
                    v.fill([x - 1, x + 2], [y, y + 1], [-1, 1], gold);
                }
                if y.abs() < 31 {
                    v.set(-8, y, 0, string);
                }
            }
            v.fill([-8, -6], [-32, -30], [0, 1], string);
            v.fill([-8, -6], [31, 33], [0, 1], string);
            v.fill([4, 6], [-1, 1], [-2, 2], gem);
            (1.35, 66.0, std::f32::consts::FRAC_PI_2)
        }
        Kind::Shield => {
            // a heater shield: rimmed, bossed, with a chevron
            let art = [
                "+++++++++++++++++++++++++",
                "+#######################+",
                "+#######################+",
                "+#######################+",
                "+###########-###########+",
                "+##########---##########+",
                "+#########--#--#########+",
                "+########--###--########+",
                "+#######--#####--#######+",
                "+######--#######--######+",
                "+#####--####*####--#####+",
                "+####--####***####--####+",
                "+#######################+",
                "+######################+.",
                ".+#####################+.",
                ".+####################+..",
                "..+###################+..",
                "..+##################+...",
                "...+################+....",
                "....+##############+.....",
                ".....+############+......",
                "......+##########+.......",
                ".......++######++........",
                ".........++##++..........",
                "...........++............",
            ];
            let rim = if quality >= 2 { gold } else { steel };
            for (row, line) in art.iter().enumerate() {
                for (col, ch) in line.bytes().enumerate() {
                    let (x, y) = (col as i32 - 12, 13 - row as i32);
                    let c = match ch {
                        b'#' => main,
                        b'+' => rim,
                        b'-' => hi,
                        b'*' => gem,
                        _ => continue,
                    };
                    v.fill([x, x + 1], [y, y + 1], [-1, 1], c);
                    if ch == b'+' {
                        v.set(x, y, -2, rim);
                    }
                }
            }
            // boss and rivets on the face, a strap behind
            v.rbox([-3, 3], [-7, -1], [-3, -1], 1, [true; 3], steel);
            v.fill([-1, 1], [-5, -3], [-4, -3], hi);
            for (x, y) in [(-9, 10), (8, 10), (-8, -4), (7, -4)] {
                v.set(x, y, -2, steel);
            }
            v.fill([-5, 5], [0, 2], [1, 2], leather);
            (0.85, 26.0, 0.0)
        }
        Kind::Tome => {
            // a clasped book
            let page = v.ink([0.93, 0.90, 0.78]);
            v.fill([-7, 7], [-9, 9], [-3, 3], main);
            v.fill([-6, 7], [-8, 8], [-2, 2], page);
            v.fill([-7, -6], [-9, 9], [-3, 3], lo);
            for y in [-9, 8] {
                v.fill([-7, 7], [y, y + 1], [-3, 3], main);
            }
            for (x, y) in [(-5, 7), (5, 7), (-5, -8), (5, -8)] {
                v.set(x, y, 3, gold);
                v.set(x, y, -4, gold);
            }
            v.fill([-2, 2], [-2, 2], [3, 4], gold);
            v.fill([-1, 1], [-1, 1], [3, 4], gem);
            v.fill([5, 8], [-1, 1], [-3, 4], gold);
            (0.5, 18.0, 0.0)
        }
        Kind::Pickaxe => {
            haft(&mut v, -14, 28, [-11, 0]);
            // a double pick, drooping to two bright points
            stamp(
                &mut v,
                &[
                    "........##########........",
                    ".....####***##***####.....",
                    "...#####***####***#####...",
                    ".+####....######....####+.",
                    "++###......####......###++",
                    "++#....................#++",
                    "+........................+",
                ],
                32,
                [-1, 1],
                main,
                hi,
                lo,
                steel_d,
            );
            v.fill([-2, 2], [27, 32], [-2, 2], lo);
            v.fill([-2, 2], [29, 30], [-2, 2], hi);
            (0.9, 48.0, 0.0)
        }
        Kind::Shovel => {
            haft(&mut v, -16, 22, [-13, -4]);
            let knob = v.ink(WOOD);
            v.fill([-3, 3], [-18, -16], [-1, 1], knob);
            stamp(
                &mut v,
                &[
                    "....-##+....",
                    "...-####+...",
                    "..-######+..",
                    ".-########+.",
                    "-##########+",
                    "-##########+",
                    "-##########+",
                    "-##########+",
                    "-##########+",
                    "-##########+",
                    "-##########+",
                    "-####--####+",
                    "++++....++++",
                ],
                35,
                [-1, 0],
                main,
                hi,
                lo,
                steel_d,
            );
            v.fill([-1, 1], [22, 30], [0, 1], lo);
            (0.9, 52.0, 0.0)
        }
        Kind::Hoe => {
            haft(&mut v, -14, 28, [-11, 0]);
            v.fill([-2, 2], [26, 29], [-2, 2], lo);
            stamp(
                &mut v,
                &[
                    "########..",
                    "#######+..",
                    "..####++..",
                    "...###+...",
                    "...##+....",
                ],
                28,
                [-1, 1],
                main,
                hi,
                lo,
                steel_d,
            );
            (0.9, 46.0, 0.0)
        }
        Kind::Torch => {
            // a stick with a pitch-soaked wrap and a flame that flickers
            haft(&mut v, -10, 8, [0, 0]);
            let pitch = v.ink([0.16, 0.12, 0.10]);
            let pitch_hi = v.ink([0.30, 0.22, 0.16]);
            v.rbox([-2, 2], [6, 11], [-2, 2], 0, [true; 3], pitch);
            v.repaint([-2, 2], [7, 8], [-2, 2], |x, _, _, c| {
                if x.rem_euclid(2) == 0 { pitch_hi } else { c }
            });
            v.repaint([-2, 2], [9, 10], [-2, 2], |x, _, _, c| {
                if x.rem_euclid(2) == 1 { pitch_hi } else { c }
            });
            let outer = v.glow([1.0, 0.45, 0.08]);
            let mid = v.glow([1.0, 0.72, 0.15]);
            let core = v.glow([1.0, 0.95, 0.60]);
            let f = frame as i32;
            for y in 0..10 {
                let r: i32 = [2, 3, 3, 3, 2, 2, 2, 1, 1, 1][y as usize];
                // the flame leans a different way each frame
                let lean = [(0, 0), (1, 0), (0, 1)][f as usize % 3];
                let (ox, oz) = (lean.0 * y / 4, lean.1 * y / 4);
                for z in -r..r {
                    for x in -r..r {
                        if r > 1 && (x == -r || x == r - 1) && (z == -r || z == r - 1) {
                            continue;
                        }
                        let inner = x.abs().max(x + 1).max(z.abs().max(z + 1));
                        let c = if inner <= r - 2 && y < 6 {
                            core
                        } else if inner < r || y > 6 {
                            mid
                        } else {
                            outer
                        };
                        v.set(x + ox, 11 + y, z + oz, c);
                    }
                }
            }
            // sparks
            v.set(1 - f, 22 + f, 0, mid);
            v.set(-2 + f, 19 + f * 2, 1, outer);
            (0.62, 34.0, 0.0)
        }
        Kind::Stick => {
            haft(&mut v, -12, 12, [0, 0]);
            let wood = v.ink(WOOD);
            v.fill([1, 3], [2, 4], [-1, 0], wood);
            v.fill([2, 4], [4, 6], [-1, 0], wood);
            (0.45, 24.0, 0.0)
        }
        Kind::Lump => {
            // a knobbly chunk with flecks
            v.rbox([-5, 5], [-4, 4], [-4, 4], 2, [true; 3], main);
            v.rbox([-3, 6], [-5, 1], [-2, 5], 2, [true; 3], lo);
            v.rbox([-6, 1], [0, 5], [-5, 2], 2, [true; 3], main);
            v.repaint([-7, 7], [-6, 6], [-6, 6], |x, y, z, c| {
                if hash3(x, y, z).is_multiple_of(9) {
                    hi
                } else {
                    c
                }
            });
            (0.32, 12.0, 0.0)
        }
        Kind::Ingot => {
            for y in -3..3 {
                let inset = (y + 3) / 2;
                v.fill(
                    [-8 + inset, 8 - inset],
                    [y, y + 1],
                    [-4 + inset, 4 - inset],
                    main,
                );
            }
            v.repaint([-8, 8], [2, 3], [-4, 4], |_, _, _, _| hi);
            v.repaint([-8, 8], [-3, -2], [-4, 4], |_, _, _, _| lo);
            v.repaint([-3, 3], [2, 3], [-1, 1], |_, _, _, _| main);
            (0.36, 16.0, 0.0)
        }
        Kind::Gem => {
            let body = if glows { v.glow(color) } else { main };
            let bright = if glows {
                v.glow(mix(color, [1.0, 1.0, 1.0], 0.5))
            } else {
                hi
            };
            for y in -7i32..7 {
                let r = if y >= 2 {
                    6 - (y - 2)
                } else {
                    6 - (2 - y) * 6 / 9
                };
                for z in -r..r {
                    for x in -r..r {
                        if x.abs().max(x + 1) + z.abs().max(z + 1) <= r + r / 2 {
                            let c = if (x + z + y).rem_euclid(4) == 0 || y == 2 {
                                bright
                            } else {
                                body
                            };
                            v.set(x, y, z, c);
                        }
                    }
                }
            }
            (0.3, 14.0, 0.0)
        }
        Kind::Dust => {
            // a little heap with glints
            for y in -3..3 {
                let r = 6 - (y + 3);
                for z in -r..r {
                    for x in -r..r {
                        if x * x + z * z <= r * r + 1 {
                            let h = hash3(x, y, z) % 7;
                            v.set(
                                x,
                                y,
                                z,
                                if h == 0 {
                                    hi
                                } else if h < 3 {
                                    lo
                                } else {
                                    main
                                },
                            );
                        }
                    }
                }
            }
            (0.3, 12.0, 0.0)
        }
        Kind::Berries => {
            let leaf = v.ink([0.25, 0.55, 0.22]);
            let stem = v.ink([0.35, 0.28, 0.15]);
            for (x, y, z) in [(-3.0, -2.0, 0.0), (3.0, -1.0, 1.0), (0.0, 2.5, -1.0)] {
                v.blob([x, y, z], [3.1, 3.1, 3.1], main);
                v.set(x as i32 - 2, y as i32 + 1, z as i32 + 2, hi);
                v.set(x as i32, y as i32 - 3, z as i32, lo);
            }
            v.fill([0, 1], [5, 9], [-1, 0], stem);
            v.fill([1, 5], [8, 9], [-2, 1], leaf);
            v.fill([-4, 0], [7, 8], [-1, 2], leaf);
            (0.3, 14.0, 0.0)
        }
        Kind::Meat => {
            // a drumstick
            let bone = v.ink([0.93, 0.90, 0.80]);
            v.blob([0.0, 3.0, 0.0], [5.0, 6.0, 4.5], main);
            v.repaint([-6, 6], [-4, 10], [-6, 6], |x, y, z, c| {
                if y > 6 || hash3(x, y, z).is_multiple_of(11) {
                    hi
                } else if y < 0 {
                    lo
                } else {
                    c
                }
            });
            v.fill([-1, 1], [-10, -2], [-1, 1], bone);
            v.fill([-2, 0], [-12, -10], [-1, 1], bone);
            v.fill([0, 2], [-12, -10], [-1, 1], bone);
            (0.4, 22.0, 0.0)
        }
        Kind::Helm => {
            v.rbox([-6, 6], [-5, 6], [-6, 6], 2, [true; 3], main);
            v.cut([-5, 5], [-5, 4], [-5, 5]);
            v.cut([-4, 4], [-5, 1], [3, 6]);
            v.repaint([-6, 6], [1, 2], [-6, 6], |_, _, _, _| gold);
            v.fill([-1, 1], [-3, 2], [5, 6], main);
            v.fill([-1, 1], [5, 8], [-5, 5], gem);
            v.repaint(
                [-6, 6],
                [4, 6],
                [-6, 6],
                |_, _, _, c| if c == main { hi } else { c },
            );
            (0.4, 14.0, 0.0)
        }
        Kind::Garment => {
            // folded cloth tied with a cord
            v.rbox([-7, 7], [-4, 4], [-5, 5], 1, [true; 3], main);
            v.repaint([-7, 7], [-1, 0], [-5, 5], |_, _, _, _| lo);
            v.repaint([-7, 7], [2, 3], [-5, 5], |_, _, _, _| lo);
            v.repaint([-7, 7], [3, 4], [-5, 5], |_, _, _, _| hi);
            v.fill([-1, 1], [-5, 5], [-6, 6], gold);
            v.fill([-2, 2], [3, 5], [-1, 1], gem);
            (0.42, 14.0, 0.0)
        }
        Kind::Ring => {
            for y in -4i32..4 {
                for x in -4i32..4 {
                    let r = ((x as f32 + 0.5).powi(2) + (y as f32 + 0.5).powi(2)).sqrt();
                    if (2.4..4.2).contains(&r) {
                        v.fill(
                            [x, x + 1],
                            [y, y + 1],
                            [-1, 1],
                            if x + y > 0 { hi } else { main },
                        );
                    }
                }
            }
            v.fill([-1, 1], [4, 6], [-1, 1], gem);
            (0.2, 10.0, 0.0)
        }
        Kind::Hand => {
            // a bare forearm and fist
            let skin = v.ink(color);
            let skin_d = v.tone(color, 0.84);
            let cuff = v.ink([0.36, 0.30, 0.26]);
            let cuff_hi = v.ink([0.50, 0.42, 0.34]);
            v.rbox([-3, 3], [-16, 2], [-3, 3], 1, [true, false, true], skin);
            v.rbox([-4, 4], [-16, -6], [-4, 4], 1, [true, false, true], cuff);
            v.repaint([-4, 4], [-7, -6], [-4, 4], |_, _, _, _| cuff_hi);
            v.rbox([-4, 4], [2, 10], [-4, 4], 1, [true; 3], skin);
            v.fill([-5, -4], [3, 7], [-1, 3], skin);
            for x in [-2, 0, 2] {
                v.repaint([x, x + 1], [6, 10], [3, 4], |_, _, _, _| skin_d);
            }
            v.repaint([-4, 4], [5, 6], [3, 4], |_, _, _, _| skin_d);
            (0.36, 26.0, 0.0)
        }
    };
    Held {
        mesh: v.mesh(1, [0.0; 3]),
        unit: real / span,
        carry_turn,
    }
}

thread_local! {
    static HELD: Cache<Held> = Cache::default();
}

fn held_model(kind: Kind, color: [f32; 3], quality: u8, glows: bool) -> Rc<Held> {
    held_frame(kind, color, quality, glows, 0)
}

/// A held model with an animation frame (the torch's flame).
fn held_frame(kind: Kind, color: [f32; 3], quality: u8, glows: bool, frame: u8) -> Rc<Held> {
    let key = key_of(&[
        kind as u32,
        frame as u32,
        color[0].to_bits(),
        color[1].to_bits(),
        color[2].to_bits(),
        quality as u32,
        glows as u32,
    ]);
    HELD.with(|c| c.get(key, || build_held(kind, color, quality, glows, frame)))
}

/// Which torch flame to show now.
fn flicker() -> u8 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static START: OnceLock<Instant> = OnceLock::new();
    let t = START.get_or_init(Instant::now).elapsed().as_secs_f32();
    ((t * 7.0) as u32 % 3) as u8
}

/// Draw the item held in the first-person hand. `ax` are the hand's axes
/// (right, up, forward) and `hand` its centre.
pub fn push_held(
    out: &mut Vec<Vertex>,
    c: &dyno_world::content::Content,
    item: Option<dyno_world::content::ItemId>,
    hand: Vec3,
    ax: [Vec3; 3],
    light: f32,
) {
    use dyno_world::content::{Icon, Slot, ToolKind};
    let paint = Paint::lit(light);
    // `tilt` leans the top away from the eye, `roll` leans it in towards the
    // middle of the view, `lift` raises it along the hand's up
    let draw = |out: &mut Vec<Vertex>, m: &Held, size: f32, tilt: f32, roll: f32, lift: f32| {
        let (ts, tc) = tilt.sin_cos();
        let up = ax[1] * tc + ax[2] * ts;
        let fwd = ax[2] * tc - ax[1] * ts;
        let (rs, rc) = roll.sin_cos();
        let (right, up) = (ax[0] * rc + up * rs, up * rc - ax[0] * rs);
        let u = m.unit * size;
        let xf = Affine3A::from_cols(
            (right * u).into(),
            (up * u).into(),
            (fwd * u).into(),
            (hand + ax[1] * lift).into(),
        );
        emit(out, &m.mesh, &xf, paint);
    };
    let Some(item) = item else {
        // a fist, reaching in from the corner
        let m = held_model(Kind::Hand, [0.91, 0.76, 0.62], 1, false);
        draw(out, &m, 0.8, 0.75, 0.35, -0.02);
        return;
    };
    let it = c.item(item);
    if let Some(b) = it.block {
        if w::shape(b) == w::Shape::Post {
            if w::emission(b) > 0 {
                let m = held_frame(Kind::Torch, it.color, 1, true, flicker());
                draw(out, &m, 0.62, 0.25, 0.12, 0.03);
            } else {
                crate::render::push_post_oriented(out, hand + ax[1] * 0.02, ax, 0.45, b, light);
            }
        } else {
            push_block_cube(out, hand, ax, 0.11, b, light);
        }
        return;
    }
    let glows = it.light > 0 || it.name.contains("Ember");
    let weapon = it.gear.and_then(|g| g.weapon.as_ref()).map(|w| w.kind);
    let kind = if let Some(k) = weapon {
        weapon_kind(k)
    } else if let Some(t) = it.tool {
        match t.kind {
            ToolKind::Pickaxe => Kind::Pickaxe,
            ToolKind::Axe => Kind::Hatchet,
            ToolKind::Shovel => Kind::Shovel,
            ToolKind::Hoe => Kind::Hoe,
            ToolKind::Sword => Kind::Sword,
        }
    } else if let Some(g) = it.gear {
        match g.slot {
            Slot::Head => Kind::Helm,
            Slot::Ring | Slot::Neck | Slot::Trinket => Kind::Ring,
            _ => Kind::Garment,
        }
    } else {
        match it.icon {
            Icon::Stick => Kind::Stick,
            Icon::Ingot => Kind::Ingot,
            Icon::Gem => Kind::Gem,
            Icon::Dust => Kind::Dust,
            Icon::Berries => Kind::Berries,
            Icon::Meat => Kind::Meat,
            Icon::Lump | Icon::Block | Icon::Tool => Kind::Lump,
        }
    };
    let m = held_model(kind, it.color, it.quality, glows);
    // long things are drawn smaller so they stay in view; weapons point up
    // and ahead, a shield is held up to show its face
    let (size, tilt, roll, lift) = match kind {
        Kind::Staff | Kind::Polearm => (0.36, 0.4, 0.15, 0.0),
        Kind::Sword | Kind::BattleAxe => (0.5, 0.35, 0.18, 0.0),
        Kind::Bow => (0.42, 0.2, 0.12, 0.12),
        Kind::Shield => (0.5, -0.1, 0.1, 0.1),
        Kind::Mace | Kind::Dagger | Kind::Wand => (0.6, 0.35, 0.18, 0.0),
        Kind::Pickaxe | Kind::Hatchet | Kind::Shovel | Kind::Hoe => (0.58, 0.35, 0.18, 0.0),
        Kind::Tome => (0.7, -0.2, 0.1, 0.05),
        _ => (0.75, -0.35, 0.15, 0.03),
    };
    draw(out, &m, size, tilt, roll, lift);
}

/// The weapon an item is, with its colour, for `Look::weapon`. `None` for
/// anything that is not a main-hand weapon.
pub fn weapon_look(
    c: &dyno_world::content::Content,
    item: Option<dyno_world::content::ItemId>,
) -> Option<(WeaponKind, [f32; 3])> {
    let it = c.item(item?);
    let kind = match it.gear.and_then(|g| g.weapon.as_ref()) {
        Some(w) => w.kind,
        None if it
            .tool
            .is_some_and(|t| t.kind == dyno_world::content::ToolKind::Sword) =>
        {
            WeaponKind::Sword
        }
        None => return None,
    };
    (!matches!(kind, WeaponKind::Shield | WeaponKind::Offhand)).then_some((kind, it.color))
}

/// Draw a weapon at true size, gripped at `hand`. `axes` are unit vectors:
/// across the blade, along it (towards the tip), and through its flat. (A
/// bow is turned a quarter about its length, so its limbs lie fore and aft.)
pub fn push_weapon_in_hand(
    out: &mut Vec<Vertex>,
    kind: WeaponKind,
    color: [f32; 3],
    hand: Vec3,
    axes: [Vec3; 3],
    light: f32,
) {
    let m = held_model(weapon_kind(kind), color, 1, false);
    let fit = Mat3::from_rotation_y(m.carry_turn) * Mat3::from_diagonal(Vec3::splat(m.unit));
    let frame = Affine3A::from_cols(axes[0].into(), axes[1].into(), axes[2].into(), hand.into());
    emit(
        out,
        &m.mesh,
        &(frame * Affine3A::from_mat3(fit)),
        Paint::lit(light),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prints quad counts and the cost of drawing a crowd.
    /// `cargo test -p dyno-client --release crowd -- --nocapture`
    #[test]
    fn crowd_is_cheap() {
        let looks: Vec<Look> = ["brannoc", "maren", "tilda", "hollis"]
            .iter()
            .map(|k| Look::npc(k, [0.4, 0.5, 0.6]))
            .chain((0..16).map(|i| Look::player(i, crate::game::player_color(i))))
            .collect();
        for (i, l) in looks.iter().enumerate().take(8) {
            let m = build_person(l);
            let q: Vec<usize> = m.iter().map(Person::quads).collect();
            let m = &m[0];
            println!(
                "look {i}: {q:?} quads (head {}, torso {}, arms {}, legs {}, skirts {}, cape {})",
                m.head.quads(),
                m.torso.quads(),
                m.arm_u
                    .iter()
                    .chain(&m.arm_l)
                    .map(Mesh::quads)
                    .sum::<usize>(),
                m.thigh
                    .iter()
                    .chain(&m.shin)
                    .map(Mesh::quads)
                    .sum::<usize>(),
                m.skirt.quads() + m.flap_f.quads() + m.flap_b.quads(),
                m.cape.iter().map(Mesh::quads).sum::<usize>(),
            );
        }
        let mut out = Vec::new();
        // spacing 1: all close. spacing 4: a street, most of them far off
        for (name, view, spacing) in [
            ("no view, all full detail", false, 1.0f32),
            ("camera known, all within 14 blocks", true, 0.6),
            ("camera known, spread over 60 blocks", true, 3.0),
        ] {
            set_view(view.then_some((Vec3::new(-2.0, 1.6, 3.0), Vec3::new(0.4, 0.8, 0.3))));
            let draw = |out: &mut Vec<Vertex>, t: f32| {
                out.clear();
                for (i, l) in looks.iter().enumerate() {
                    let at = Vec3::new(i as f32 * spacing, 0.0, (i % 3) as f32);
                    push_person(out, at, i as f32, 0.1, l, 0.8, t, 1.0);
                }
            };
            draw(&mut out, 0.0);
            let n = 300;
            let start = std::time::Instant::now();
            for i in 0..n {
                draw(&mut out, i as f32 * 0.1);
            }
            let per = start.elapsed().as_secs_f64() * 1000.0 / n as f64;
            println!(
                "{} people, {name}: {} vertices, {per:.3} ms per frame",
                looks.len(),
                out.len()
            );
        }
        set_view(None);
        assert!(!out.is_empty());
    }

    /// Renders contact sheets of every model, for looking at while modelling:
    /// `STUDIO=/some/dir cargo test -p dyno-client studio -- --ignored`
    #[test]
    #[ignore = "needs a GPU; writes images"]
    fn studio() {
        let Ok(dir) = std::env::var("STUDIO") else {
            return;
        };
        let only = std::env::var("SCENE").ok();
        let dir = std::path::PathBuf::from(dir);
        let (w, h) = (1600u32, 900u32);
        let mut g = crate::render::Gfx::new_headless(w, h).expect("a headless renderer");
        let time: f32 = std::env::var("TIME")
            .ok()
            .and_then(|t| t.parse().ok())
            .unwrap_or(0.3);
        let sky = crate::app::sky(time);
        let content = dyno_world::content::Content::default_pack();
        let npcs = ["brannoc", "maren", "tilda", "hollis"];
        let npc = |k: &str| Look::npc(k, [0.4, 0.5, 0.6]);
        let mut shoot =
            |name: &str, eye: Vec3, target: Vec3, fov: f32, draw: &dyn Fn(&mut Vec<Vertex>)| {
                if only.as_deref().is_some_and(|o| !name.contains(o)) {
                    return;
                }
                let mut boxes = Vec::new();
                crate::render::push_box(
                    &mut boxes,
                    Vec3::new(-60.0, -0.5, -60.0),
                    Vec3::new(60.0, 0.0, 60.0),
                    [0.42, 0.50, 0.36],
                    0.0,
                );
                set_view(Some((eye, Vec3::from(sky.sun_dir))));
                draw(&mut boxes);
                set_view(None);
                let far = 160.0;
                let vp =
                    glam::Mat4::perspective_rh(fov.to_radians(), w as f32 / h as f32, 0.05, far)
                        * glam::Mat4::look_at_rh(eye, target, Vec3::Y);
                g.screenshot = Some(dir.join(format!("{name}.ppm")));
                let ctx = egui::Context::default();
                let out = ctx.run(egui::RawInput::default(), |_| {});
                g.frame(Some((vp, eye, far)), &sky, &boxes, &[], &ctx, out);
                println!("{name}: {} vertices", boxes.len());
            };
        let beast = |b: &mut Vec<Vertex>,
                     m,
                     variant: u8,
                     at: Vec3,
                     yaw: f32,
                     walk: f32,
                     flash: f32,
                     t: f32| {
            let color = match (m, variant) {
                (dyno_world::content::Model::Quadruped, 1) => [0.33, 0.22, 0.15],
                (dyno_world::content::Model::Quadruped, 2) => [0.38, 0.42, 0.24],
                (dyno_world::content::Model::Quadruped, 3) => [0.80, 0.78, 0.72],
                (dyno_world::content::Model::Quadruped, _) => [0.86, 0.82, 0.7],
                (dyno_world::content::Model::Humanoid, 1 | 2) => [0.46, 0.45, 0.44],
                (dyno_world::content::Model::Humanoid, 3) => [0.90, 0.86, 0.70],
                (dyno_world::content::Model::Humanoid, _) => [0.16, 0.12, 0.22],
                (dyno_world::content::Model::Orb, _) => [0.35, 0.85, 1.0],
            };
            let base = if walk > 0.1 { Anim::Walk } else { Anim::Idle };
            let mut a = Animator::still(base, t * 9.0);
            push_creature(b, m, variant, at, yaw, color, 0.0, flash, 1.0, &mut a, t);
        };
        let creature = |b: &mut Vec<Vertex>, m, at: Vec3, yaw: f32, walk: f32, flash: f32, t| {
            beast(b, m, 0, at, yaw, walk, flash, t)
        };
        use dyno_world::content::Model;
        // figures face -z by default (yaw 0); the camera looks from -z
        let pi = std::f32::consts::PI;
        shoot(
            "people_front",
            Vec3::new(0.0, 1.5, -5.2),
            Vec3::new(0.0, 1.05, 0.0),
            40.0,
            &|b| {
                for (i, k) in npcs.iter().enumerate() {
                    let x = (i as f32 - 1.5) * 1.7;
                    push_person(b, Vec3::new(x, 0.0, 0.0), 0.0, 0.0, &npc(k), 0.0, 0.4, 0.0);
                }
            },
        );
        // the zone creatures: each from the front quarter, the side (walking),
        // the back quarter, and hit
        for (name, m, variant, eye, look_at) in [
            ("thornboar", Model::Quadruped, 1u8, [0.0, 1.2, -4.4], 0.6f32),
            ("mirepincher", Model::Quadruped, 2, [0.0, 1.5, -4.2], 0.3),
            ("websnare", Model::Quadruped, 3, [0.0, 1.9, -5.0], 0.4),
            ("ashcloak", Model::Humanoid, 1, [0.0, 1.5, -5.2], 1.05),
            (
                "ashcloak_leader",
                Model::Humanoid,
                2,
                [0.0, 1.5, -5.2],
                1.05,
            ),
            ("candlekin", Model::Humanoid, 3, [0.0, 1.5, -5.2], 1.05),
            ("gloomwisp", Model::Orb, 0, [0.0, 1.2, -4.0], 0.8),
        ] {
            shoot(
                &format!("zoo_{name}"),
                Vec3::from(eye),
                Vec3::new(0.0, look_at, 0.0),
                40.0,
                &|b| {
                    beast(b, m, variant, Vec3::new(-2.6, 0.0, 0.0), 0.5, 0.0, 0.0, 0.5);
                    beast(
                        b,
                        m,
                        variant,
                        Vec3::new(-0.8, 0.0, 0.0),
                        pi / 2.0,
                        1.0,
                        0.0,
                        1.4,
                    );
                    beast(
                        b,
                        m,
                        variant,
                        Vec3::new(1.0, 0.0, 0.0),
                        pi + 0.6,
                        0.0,
                        0.0,
                        2.3,
                    );
                    beast(b, m, variant, Vec3::new(2.7, 0.0, 0.0), -0.7, 1.0, 0.6, 3.1);
                },
            );
            // and close, face on
            shoot(
                &format!("zoo_{name}_near"),
                Vec3::new(1.0, look_at * 1.5 + 0.5, -2.2),
                Vec3::new(0.0, look_at * 1.5, 0.0),
                40.0,
                &|b| beast(b, m, variant, Vec3::ZERO, 0.25, 0.0, 0.0, 0.5),
            );
        }
        // the coarser models, magnified from where they are used
        for (name, dist, fov) in [("lod1", 20.0f32, 10.0f32), ("lod2", 50.0, 4.2)] {
            shoot(
                name,
                Vec3::new(0.0, 1.5, -dist),
                Vec3::new(0.0, 1.05, 0.0),
                fov,
                &|b| {
                    for (i, k) in npcs.iter().enumerate() {
                        let x = (i as f32 - 1.5) * 1.3;
                        push_person(b, Vec3::new(x, 0.0, 0.0), 0.0, 0.0, &npc(k), 0.0, 0.4, 0.0);
                    }
                    creature(
                        b,
                        Model::Quadruped,
                        Vec3::new(-3.4, 0.0, 0.0),
                        1.0,
                        0.0,
                        0.0,
                        0.0,
                    );
                    creature(
                        b,
                        Model::Humanoid,
                        Vec3::new(3.3, 0.0, 0.0),
                        0.3,
                        0.0,
                        0.0,
                        0.0,
                    );
                },
            );
        }
        shoot(
            "people_back",
            Vec3::new(2.0, 1.9, 5.0),
            Vec3::new(0.0, 1.05, 0.0),
            40.0,
            &|b| {
                for (i, k) in npcs.iter().enumerate() {
                    let x = (i as f32 - 1.5) * 1.7;
                    push_person(b, Vec3::new(x, 0.0, 0.0), 0.0, 0.0, &npc(k), 0.0, 0.4, 0.0);
                }
            },
        );
        shoot(
            "faces",
            Vec3::new(0.0, 1.8, -2.9),
            Vec3::new(0.0, 1.62, 0.0),
            36.0,
            &|b| {
                for (i, k) in npcs.iter().enumerate() {
                    let x = (i as f32 - 1.5) * 0.95;
                    let yaw = [0.35, 0.0, -0.25, -0.5][i];
                    push_person(b, Vec3::new(x, 0.0, 0.0), yaw, 0.0, &npc(k), 0.0, 0.4, 0.0);
                }
            },
        );
        shoot(
            "players",
            Vec3::new(-1.5, 1.7, -6.2),
            Vec3::new(0.0, 1.0, 0.0),
            42.0,
            &|b| {
                for i in 0..6u32 {
                    let x = (i as f32 - 2.5) * 1.6;
                    let mut l = Look::player(i + 1, crate::game::player_color(i));
                    l.armor = [
                        ArmorType::Cloth,
                        ArmorType::Leather,
                        ArmorType::Mail,
                        ArmorType::Plate,
                        ArmorType::Plate,
                        ArmorType::Cloth,
                    ][i as usize];
                    l.gloves = l.armor != ArmorType::Cloth;
                    l.helmet = i == 4;
                    l.hair_style = if i == 5 { 3 } else { l.hair_style };
                    l.weapon = [
                        Some((WeaponKind::Staff, [0.5, 0.4, 0.9])),
                        Some((WeaponKind::Dagger, [0.7, 0.7, 0.75])),
                        Some((WeaponKind::Bow, [0.5, 0.35, 0.2])),
                        Some((WeaponKind::Sword, [0.72, 0.72, 0.78])),
                        Some((WeaponKind::Axe, [0.6, 0.62, 0.68])),
                        Some((WeaponKind::Mace, [0.6, 0.6, 0.65])),
                    ][i as usize];
                    let walk = if i % 2 == 0 { 0.0 } else { 1.0 };
                    push_person(
                        b,
                        Vec3::new(x, 0.0, 0.0),
                        0.3 - i as f32 * 0.25,
                        0.0,
                        &l,
                        0.0,
                        1.0 + i as f32,
                        walk,
                    );
                }
            },
        );
        shoot(
            "walk",
            Vec3::new(-7.0, 1.4, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            40.0,
            &|b| {
                for i in 0..5 {
                    let z = (i as f32 - 2.0) * 1.7;
                    let l = if i % 2 == 0 {
                        npc("maren")
                    } else {
                        npc("brannoc")
                    };
                    push_person(
                        b,
                        Vec3::new(0.0, 0.0, z),
                        0.0,
                        0.2,
                        &l,
                        0.0,
                        i as f32 * 1.3,
                        1.0,
                    );
                }
            },
        );
        shoot(
            "grazer",
            Vec3::new(0.0, 1.3, -4.6),
            Vec3::new(0.0, 0.7, 0.0),
            40.0,
            &|b| {
                creature(
                    b,
                    Model::Quadruped,
                    Vec3::new(-2.6, 0.0, 0.0),
                    0.5,
                    0.0,
                    0.0,
                    0.0,
                );
                creature(
                    b,
                    Model::Quadruped,
                    Vec3::new(-0.8, 0.0, 0.0),
                    pi / 2.0,
                    1.0,
                    0.0,
                    0.0,
                );
                creature(
                    b,
                    Model::Quadruped,
                    Vec3::new(1.0, 0.0, 0.0),
                    pi + 0.6,
                    0.0,
                    0.0,
                    0.0,
                );
                // grazing: t where the dip is deepest
                creature(
                    b,
                    Model::Quadruped,
                    Vec3::new(2.7, 0.0, 0.0),
                    -0.9,
                    0.0,
                    0.0,
                    2.1,
                );
            },
        );
        shoot(
            "shade",
            Vec3::new(0.0, 1.5, -5.0),
            Vec3::new(0.0, 1.1, 0.0),
            40.0,
            &|b| {
                creature(
                    b,
                    Model::Humanoid,
                    Vec3::new(-2.4, 0.0, 0.0),
                    0.3,
                    0.0,
                    0.0,
                    0.0,
                );
                creature(
                    b,
                    Model::Humanoid,
                    Vec3::new(-0.6, 0.0, 0.0),
                    pi / 2.0,
                    1.0,
                    0.0,
                    1.0,
                );
                creature(
                    b,
                    Model::Humanoid,
                    Vec3::new(1.0, 0.0, 0.0),
                    pi - 0.5,
                    0.0,
                    0.0,
                    2.0,
                );
                creature(
                    b,
                    Model::Humanoid,
                    Vec3::new(2.4, 0.0, 0.0),
                    -0.3,
                    1.0,
                    0.7,
                    3.0,
                );
                creature(b, Model::Orb, Vec3::new(3.4, 0.0, -1.0), 0.0, 0.0, 0.0, 3.0);
            },
        );
        shoot(
            "shade_face",
            Vec3::new(0.3, 1.9, -1.9),
            Vec3::new(0.0, 1.6, 0.0),
            40.0,
            &|b| {
                creature(
                    b,
                    Model::Humanoid,
                    Vec3::new(-0.5, 0.0, 0.0),
                    0.2,
                    0.0,
                    0.0,
                    0.5,
                );
                creature(b, Model::Orb, Vec3::new(0.8, 0.6, 0.0), 0.0, 0.0, 0.0, 3.0);
            },
        );
        // held items, as the first-person hand shows them, in a grid
        let keys: Vec<Option<&str>> = vec![
            None,
            Some("watch_longsword"),
            Some("worn_shortsword"),
            Some("watch_greataxe"),
            Some("rough_maul"),
            Some("shadowbane_mace"),
            Some("keen_dagger"),
            Some("apprentice_staff"),
            Some("ember_staff"),
            Some("ashwood_longbow"),
            Some("battered_buckler"),
            Some("acolyte_rod"),
            Some("torch"),
            Some("wooden_pickaxe"),
            Some("stone_pickaxe"),
            Some("iron_pickaxe"),
            Some("iron_axe"),
            Some("iron_shovel"),
            Some("ember_sword"),
            Some("ember_pickaxe"),
            Some("stick"),
            Some("ember_shard"),
            Some("shade_dust"),
            Some("berries"),
            Some("raw_meat"),
            Some("iron_ingot"),
            Some("coal"),
            Some("stone"),
            Some("lantern"),
        ];
        for k in keys.iter().flatten() {
            if content.item_by_key(k).is_none() {
                println!("no item {k}");
            }
        }
        for (sheet, part) in keys.chunks(15).enumerate() {
            shoot(
                &format!("items_{sheet}"),
                Vec3::new(0.0, 1.5, -3.3),
                Vec3::new(0.0, 1.5, 0.0),
                40.0,
                &|b| {
                    for (i, k) in part.iter().enumerate() {
                        let (col, row) = ((i % 5) as f32, (i / 5) as f32);
                        let hand = Vec3::new((col - 2.0) * 0.72, 2.2 - row * 0.78, 0.0);
                        // as seen in the hand: right is the viewer's right
                        let ax = [Vec3::NEG_X, Vec3::Y, Vec3::Z];
                        let item = k.and_then(|k| content.item_by_key(k));
                        if k.is_none() || item.is_some() {
                            push_held(b, &content, item, hand, ax, 0.0);
                        }
                    }
                },
            );
        }
        // every kind of held model, straight from the builder, and under it
        // every weapon as a figure carries it
        let kinds = [
            Kind::Sword,
            Kind::BattleAxe,
            Kind::Mace,
            Kind::Dagger,
            Kind::Staff,
            Kind::Polearm,
            Kind::Bow,
            Kind::Wand,
            Kind::Shield,
            Kind::Tome,
            Kind::Pickaxe,
            Kind::Hatchet,
            Kind::Shovel,
            Kind::Hoe,
            Kind::Torch,
            Kind::Stick,
            Kind::Lump,
            Kind::Ingot,
            Kind::Gem,
            Kind::Dust,
            Kind::Berries,
            Kind::Meat,
            Kind::Helm,
            Kind::Garment,
            Kind::Ring,
            Kind::Hand,
        ];
        for (sheet, part) in kinds.chunks(9).enumerate() {
            shoot(
                &format!("kinds_{sheet}"),
                Vec3::new(0.0, 1.3, -3.4),
                Vec3::new(0.0, 1.3, 0.0),
                40.0,
                &|b| {
                    for (i, kind) in part.iter().enumerate() {
                        let x = (4.0 - i as f32) * 0.5;
                        let colors = [[0.72, 0.72, 0.78], [0.85, 0.3, 0.25], [0.3, 0.45, 0.8]];
                        let m = held_model(*kind, colors[i % 3], 1 + (i % 4) as u8, i % 4 == 3);
                        // true size, except the small things, which are doubled
                        let k = m.unit * if m.unit * 30.0 < 0.5 { 2.0 } else { 1.0 };
                        let xf = Affine3A::from_cols(
                            (Vec3::NEG_X * k).into(),
                            (Vec3::Y * k).into(),
                            (Vec3::Z * k).into(),
                            Vec3::new(x, 1.5, 0.0).into(),
                        );
                        emit(b, &m.mesh, &xf, Paint::lit(0.0));
                    }
                },
            );
        }
        shoot(
            "weapons_in_hand",
            Vec3::new(0.0, 1.3, -3.6),
            Vec3::new(0.0, 1.3, 0.0),
            40.0,
            &|b| {
                let all = [
                    WeaponKind::Sword,
                    WeaponKind::Axe,
                    WeaponKind::Mace,
                    WeaponKind::Dagger,
                    WeaponKind::Staff,
                    WeaponKind::Polearm,
                    WeaponKind::Bow,
                    WeaponKind::Wand,
                    WeaponKind::Shield,
                    WeaponKind::Offhand,
                ];
                for (i, kind) in all.into_iter().enumerate() {
                    let hand = Vec3::new((4.5 - i as f32) * 0.5, 1.0, 0.0);
                    let axes = [Vec3::NEG_X, Vec3::Y, Vec3::Z];
                    push_weapon_in_hand(b, kind, [0.6, 0.62, 0.7], hand, axes, 0.0);
                }
            },
        );
        // the first-person view, as the game frames it
        for k in [
            None,
            Some("watch_longsword"),
            Some("ember_staff"),
            Some("torch"),
            Some("iron_pickaxe"),
            Some("battered_buckler"),
            Some("ashwood_longbow"),
            Some("watch_greataxe"),
            Some("berries"),
        ] {
            let eye = Vec3::new(0.0, 1.6, 0.0);
            let dir = Vec3::new(0.0, -0.1, 1.0).normalize();
            let right = dir.cross(Vec3::Y).normalize();
            let up = right.cross(dir);
            let hand = eye + dir * 0.62 + right * 0.36 - up * 0.34;
            let (s1, c1) = 0.6f32.sin_cos();
            let ax = [right * c1 + dir * s1, up, dir * c1 - right * s1];
            let item = k.and_then(|k| content.item_by_key(k));
            shoot(
                &format!("fp_{}", k.unwrap_or("hand")),
                eye,
                eye + dir,
                75.0,
                &|b| {
                    push_held(b, &content, item, hand, ax, 0.0);
                    push_person(
                        b,
                        Vec3::new(-0.6, 0.0, 3.0),
                        0.2,
                        0.0,
                        &npc("maren"),
                        0.0,
                        0.0,
                        0.0,
                    );
                },
            );
        }
    }
}

/// The animation contract, checked against every model.
#[cfg(test)]
mod anim_tests {
    use super::*;
    use dyno_world::content::{Content, Model};

    /// Prints every figure's standing size in metres (1 block = 1 m), for
    /// the scale review: `cargo test -p dyno-client measure_figures -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn measure_figures() {
        let c = Content::default_pack();
        let size = |v: &[Vertex]| {
            let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for x in v {
                let p = Vec3::from(x.pos);
                lo = lo.min(p);
                hi = hi.max(p);
            }
            hi - lo
        };
        let mut v = Vec::new();
        let look = Look::from_appearance(&Default::default(), [0.5, 0.5, 0.6]);
        push_person(&mut v, Vec3::ZERO, 0.0, 0.0, &look, 1.0, 0.0, 0.0);
        let d = size(&v);
        println!("person: {:.2} tall, {:.2} wide, {:.2} deep", d.y, d.x, d.z);
        for cr in &c.creatures {
            let mut v = Vec::new();
            let mut a = Animator::new(1, 0.0);
            push_creature(
                &mut v,
                cr.model,
                cr.variant,
                Vec3::ZERO,
                0.0,
                cr.color,
                1.0,
                0.0,
                cr.scale,
                &mut a,
                0.0,
            );
            let d = size(&v);
            println!(
                "{}: {:.2} tall, {:.2} wide, {:.2} long (scale {})",
                cr.key, d.y, d.x, d.z, cr.scale
            );
        }
    }

    /// When to sample an animation: through a one-shot and well past its
    /// end, or across several seconds of a loop.
    fn times(a: Anim) -> Vec<f32> {
        match a.duration() {
            Some(d) => vec![0.0, d * 0.3, d * 0.5, d * 0.8, d, d * 3.0],
            None => vec![0.0, 0.7, 2.3, 4.6, 7.9, 30.1],
        }
    }

    /// Geometry was made, every vertex is a number, and the figure stays in
    /// its box: nothing under the ground, nothing flung far off. `loose`
    /// allows half a block under: swimmers sit low in the water, and a
    /// figure getting up from the ground passes through it for a moment.
    /// Problems are collected in `bad`.
    fn check(
        out: &[Vertex],
        feet: Vec3,
        scale: f32,
        loose: bool,
        what: &str,
        bad: &mut Vec<String>,
    ) {
        if out.is_empty() {
            bad.push(format!("{what}: no geometry"));
            return;
        }
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for v in out {
            let p = Vec3::from(v.pos);
            if !p.is_finite() {
                bad.push(format!("{what}: a vertex is not a number"));
                return;
            }
            lo = lo.min(p);
            hi = hi.max(p);
        }
        let (lo, hi) = ((lo - feet) / scale, (hi - feet) / scale);
        let floor = if loose { -0.5 } else { -0.13 };
        if lo.y < floor {
            bad.push(format!("{what}: sinks {:.2} blocks underground", -lo.y));
        }
        if hi.y > 3.0 {
            bad.push(format!("{what}: reaches {:.2} blocks up", hi.y));
        }
        for d in [lo.x, lo.z, hi.x, hi.z] {
            if d.abs() > 2.6 {
                bad.push(format!("{what}: reaches {d:.2} blocks out"));
            }
        }
    }

    fn report(bad: &[String]) {
        for b in bad {
            println!("{b}");
        }
        assert!(
            bad.is_empty(),
            "{} problems, the first: {}",
            bad.len(),
            bad[0]
        );
    }

    fn people() -> Vec<(&'static str, Look)> {
        let mut list = Vec::new();
        for (name, weapon) in [
            ("unarmed", None),
            ("sword", Some(WeaponKind::Sword)),
            ("axe", Some(WeaponKind::Axe)),
            ("staff", Some(WeaponKind::Staff)),
            ("bow", Some(WeaponKind::Bow)),
        ] {
            let mut l = Look::player(3, [0.6, 0.2, 0.2]);
            l.weapon = weapon.map(|k| (k, [0.6, 0.6, 0.65]));
            l.cape = Some([0.3, 0.2, 0.5]);
            list.push((name, l));
        }
        let mut robed = Look::player(9, [0.2, 0.3, 0.7]);
        robed.robe = true;
        list.push(("robed", robed));
        list.push(("npc", Look::npc("maren", [0.4, 0.5, 0.6])));
        list
    }

    /// Every creature in the game's content and every kind of player
    /// performs every animation.
    #[test]
    fn every_figure_performs_every_animation() {
        set_view(None);
        let content = Content::default_pack();
        let feet = Vec3::new(100.0, 50.0, 100.0);
        let mut out = Vec::new();
        let mut bad = Vec::new();
        for c in &content.creatures {
            for &a in Anim::ALL {
                for t in times(a) {
                    let mut an = Animator::still(a, 1.3 + t * 5.0);
                    out.clear();
                    push_creature(
                        &mut out, c.model, c.variant, feet, 0.4, c.color, 1.0, 0.0, c.scale,
                        &mut an, t,
                    );
                    let what = format!("{} {a:?} at {t:.2}s", c.key);
                    check(&out, feet, c.scale, a == Anim::Swim, &what, &mut bad);
                }
            }
        }
        for (name, look) in people() {
            for &a in Anim::ALL {
                for t in times(a) {
                    let mut an = Animator::still(a, 1.3 + t * 5.0);
                    out.clear();
                    push_person_anim(&mut out, feet, 0.4, 0.1, &look, 1.0, &mut an, t);
                    let what = format!("{name} {a:?} at {t:.2}s");
                    check(&out, feet, 1.0, a == Anim::Swim, &what, &mut bad);
                }
            }
        }
        report(&bad);
    }

    /// Changing state halfway through anything stays well formed: every
    /// pair of animations, crossfading, with an action layered on top.
    #[test]
    fn crossfades_stay_well_formed() {
        set_view(None);
        let content = Content::default_pack();
        let feet = Vec3::new(10.0, 5.0, 10.0);
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut bad = Vec::new();
        for c in &content.creatures {
            if !seen.insert(Skeleton::of(c.model, c.variant)) {
                continue;
            }
            for &from in Anim::ALL {
                for &to in Anim::ALL {
                    let mut an = Animator::new(5, 0.0);
                    an.drive(0.0, from, 0.0, 1.0);
                    out.clear();
                    push_creature(
                        &mut out, c.model, c.variant, feet, 0.0, c.color, 1.0, 0.0, 1.0, &mut an,
                        0.6,
                    );
                    an.drive(0.7, to, 0.4, 1.0);
                    an.play(0.7, Anim::HitReact);
                    out.clear();
                    push_creature(
                        &mut out,
                        c.model,
                        c.variant,
                        feet,
                        0.0,
                        c.color,
                        1.0,
                        0.0,
                        1.0,
                        &mut an,
                        0.7 + anim::FADE * 0.5,
                    );
                    let what = format!("{} {from:?} to {to:?}", c.key);
                    check(&out, feet, 1.0, true, &what, &mut bad);
                }
            }
        }
        report(&bad);
    }

    /// No animation is a silent copy of standing idle: each skeleton is
    /// rigged to do something for every one.
    #[test]
    fn every_skeleton_animates_every_animation() {
        let person = BipedRig {
            grip: Grip::OneHand,
            robe: false,
        };
        let beast = quad_rig(&build_grazer([0.8, 0.8, 0.7])[0]);
        let crab = multi_rig(&build_mirepincher([0.4, 0.4, 0.2])[0]);
        let spider = multi_rig(&build_websnare([0.8, 0.8, 0.7])[0]);
        type PoseFn<'a> = Box<dyn Fn(Anim, &anim::At) -> Pose + 'a>;
        let rigs: Vec<(&str, PoseFn)> = vec![
            ("biped", Box::new(|a, at| anim::biped(a, at, &person))),
            ("wraith", Box::new(anim::wraith)),
            (
                "quadruped",
                Box::new(|a, at| anim::quadruped(a, at, &beast)),
            ),
            ("crab", Box::new(|a, at| anim::multileg(a, at, &crab))),
            ("spider", Box::new(|a, at| anim::multileg(a, at, &spider))),
            ("wisp", Box::new(anim::wisp)),
        ];
        for (name, pose) in &rigs {
            for &a in Anim::ALL.iter().filter(|a| **a != Anim::Idle) {
                let differs = times(a).iter().any(|&t| {
                    let at = anim::At {
                        t,
                        stride: 1.0 + t * 3.0,
                        seed: 0.3,
                    };
                    let p = pose(a, &at);
                    assert!(p.is_finite(), "{name} {a:?}: not a number");
                    p.distance(&pose(Anim::Idle, &at)) > 0.05
                });
                assert!(differs, "{name} does nothing for {a:?}");
            }
        }
    }

    #[test]
    fn creatures_have_a_skeleton_and_an_attack() {
        let content = Content::default_pack();
        for c in &content.creatures {
            let s = Skeleton::of(c.model, c.variant);
            assert!(!s.bones().is_empty());
            let a = creature_attack(c.model, c.variant);
            assert!(
                a.duration().is_some(),
                "{}: attack must be a one-shot",
                c.key
            );
        }
        assert_eq!(Skeleton::of(Model::Orb, 0), Skeleton::Wisp);
    }

    /// The best of several runs of 60 frames (steady under a loaded
    /// machine): 20 walking players and 48 creatures, all animated.
    /// `cargo test -p dyno-client --release animated_crowd -- --nocapture`
    #[test]
    fn animated_crowd_cost() {
        fn best(mut f: impl FnMut(f32)) -> f64 {
            let mut best = f64::MAX;
            for r in 0..7 {
                let start = std::time::Instant::now();
                for i in 0..60 {
                    f((r * 60 + i) as f32 * 0.016);
                }
                best = best.min(start.elapsed().as_secs_f64() * 1000.0 / 60.0);
            }
            best
        }
        let content = Content::default_pack();
        let looks: Vec<Look> = (0..20)
            .map(|i| Look::player(i, crate::game::player_color(i)))
            .collect();
        let mut out = Vec::new();
        set_view(Some((Vec3::new(-2.0, 1.6, 3.0), Vec3::new(0.4, 0.8, 0.3))));
        let mut people: Vec<Animator> = (0..20).map(|i| Animator::new(i, 0.0)).collect();
        let p = best(|t| {
            out.clear();
            for (i, l) in looks.iter().enumerate() {
                let at = Vec3::new(i as f32 * 0.6, 0.0, (i % 3) as f32);
                people[i].drive(t, Anim::Walk, 0.07, 1.0);
                push_person_anim(&mut out, at, i as f32, 0.1, l, 0.8, &mut people[i], t);
            }
        });
        let pv = out.len();
        let n = content.creatures.len();
        let mut beasts: Vec<Animator> = (0..n * 4).map(|i| Animator::new(i as u32, 0.0)).collect();
        let c = best(|t| {
            out.clear();
            for k in 0..4 {
                for (i, c) in content.creatures.iter().enumerate() {
                    let at = Vec3::new(i as f32 * 1.5, 0.0, k as f32 * 2.0);
                    let a = &mut beasts[k * n + i];
                    a.drive(t, Anim::Walk, 0.05, 1.0);
                    push_creature(
                        &mut out, c.model, c.variant, at, i as f32, c.color, 0.8, 0.0, 1.0, a, t,
                    );
                }
            }
        });
        println!(
            "AB people: {p:.3} ms ({pv} verts); creatures: {c:.3} ms ({} verts)",
            out.len()
        );
        set_view(None);
    }

    /// Prints the cost of drawing a crowd of every creature, animated.
    /// `cargo test -p dyno-client --release creature_crowd -- --nocapture`
    #[test]
    fn creature_crowd_is_cheap() {
        let content = Content::default_pack();
        let mut out = Vec::new();
        set_view(Some((Vec3::new(-2.0, 1.6, 3.0), Vec3::new(0.4, 0.8, 0.3))));
        let n = content.creatures.len() * 4;
        let mut anims: Vec<Animator> = (0..n).map(|i| Animator::new(i as u32, 0.0)).collect();
        let mut draw = |out: &mut Vec<Vertex>, t: f32| {
            out.clear();
            for k in 0..4 {
                for (i, c) in content.creatures.iter().enumerate() {
                    let at = Vec3::new(i as f32 * 1.5, 0.0, k as f32 * 2.0);
                    let a = &mut anims[k * content.creatures.len() + i];
                    let base = [Anim::Walk, Anim::Run, Anim::Ready, Anim::Idle][k];
                    a.drive(t, base, 0.05, 1.0);
                    push_creature(
                        out, c.model, c.variant, at, i as f32, c.color, 0.8, 0.0, 1.0, a, t,
                    );
                }
            }
        };
        draw(&mut out, 0.0);
        let frames = 300;
        let start = std::time::Instant::now();
        for i in 0..frames {
            draw(&mut out, i as f32 * 0.1);
        }
        let per = start.elapsed().as_secs_f64() * 1000.0 / frames as f64;
        println!(
            "{n} creatures: {} vertices, {per:.3} ms per frame",
            out.len()
        );
        set_view(None);
        assert!(!out.is_empty());
    }
}
