//! Light: block light from glowing blocks, and sky light from open sky.
//! Both fade by one level per cell through air and are recomputed locally
//! around each change.
//!
//! Light is stored per section of 16³, both levels packed in one byte (sky
//! in the high nibble). A section that is all open sky with no block light
//! (the air above the land, and everything not yet streamed) costs nothing.

use crate::{self as w, SECTION, SECTION_CELLS, SectionPos, World};
use std::collections::VecDeque;

pub struct Light {
    size: [i32; 3],
    dims: [i32; 3],
    secs: Vec<Option<Box<[u8; SECTION_CELLS]>>>,
}

const R: i32 = 15;
/// Sky 15, block 0: the value of every cell of an unallocated section.
const OPEN: u8 = 0xf0;

impl Light {
    pub fn new(world: &World) -> Self {
        let size = [
            world.size[0] as i32,
            world.size[1] as i32,
            world.size[2] as i32,
        ];
        let dims = world.section_dims();
        let mut l = Light {
            size,
            dims,
            secs: vec![None; (dims[0] * dims[1] * dims[2]) as usize],
        };
        if world.is_sparse() {
            let held = world.held_list();
            l.arrived(world, &held);
        } else {
            l.recompute(world, [0, 0, 0], [size[0] - 1, size[1] - 1, size[2] - 1]);
        }
        l
    }

    /// Light the columns of sections that arrived in a streamed world (and
    /// the edges of their neighbours, which the new cells may shade or light).
    pub fn arrived(&mut self, world: &World, sections: &[SectionPos]) {
        let mut cols: Vec<[i32; 2]> = sections.iter().map(|s| [s[0], s[2]]).collect();
        cols.sort_unstable();
        cols.dedup();
        let Some(first) = cols.first() else { return };
        let (mut lo, mut hi) = (*first, *first);
        for c in &cols {
            lo = [lo[0].min(c[0]), lo[1].min(c[1])];
            hi = [hi[0].max(c[0]), hi[1].max(c[1])];
        }
        let area = ((hi[0] - lo[0] + 1) * (hi[1] - lo[1] + 1)) as usize;
        let boxed = |a: [i32; 2], b: [i32; 2]| {
            (
                [a[0] * SECTION, 0, a[1] * SECTION],
                [
                    b[0] * SECTION + SECTION - 1,
                    0,
                    b[1] * SECTION + SECTION - 1,
                ],
            )
        };
        if area <= cols.len() * 4 {
            // close together: one pass over the box round them all
            let (a, b) = boxed(lo, hi);
            self.changed_box(world, a, b);
        } else {
            for c in cols {
                let (a, b) = boxed(c, c);
                self.changed_box(world, a, b);
            }
        }
    }

    /// Free the light of a section that left a streamed world.
    pub fn forget(&mut self, s: SectionPos) {
        if let Some(i) = self.sect(s[0], s[1], s[2]) {
            self.secs[i] = None;
        }
    }

    /// Bytes of light held (for measurements).
    pub fn bytes(&self) -> usize {
        self.secs.iter().flatten().count() * SECTION_CELLS
    }

    #[inline]
    fn sect(&self, sx: i32, sy: i32, sz: i32) -> Option<usize> {
        let d = self.dims;
        if sx < 0 || sy < 0 || sz < 0 || sx >= d[0] || sy >= d[1] || sz >= d[2] {
            None
        } else {
            Some(((sy * d[2] + sz) * d[0] + sx) as usize)
        }
    }

    #[inline]
    fn packed(&self, x: i32, y: i32, z: i32) -> Option<u8> {
        if x < 0 || y < 0 || z < 0 || x >= self.size[0] || y >= self.size[1] || z >= self.size[2] {
            return None;
        }
        let i = self.sect(x / SECTION, y / SECTION, z / SECTION)?;
        Some(match &self.secs[i] {
            None => OPEN,
            Some(c) => {
                let l = ((y % SECTION * SECTION + z % SECTION) * SECTION + x % SECTION) as usize;
                c[l]
            }
        })
    }

    #[inline]
    fn put(&mut self, x: i32, y: i32, z: i32, v: u8) {
        let Some(i) = self.sect(x / SECTION, y / SECTION, z / SECTION) else {
            return;
        };
        let l = ((y % SECTION * SECTION + z % SECTION) * SECTION + x % SECTION) as usize;
        match &mut self.secs[i] {
            Some(c) => c[l] = v,
            None if v == OPEN => {}
            slot @ None => {
                let mut c = Box::new([OPEN; SECTION_CELLS]);
                c[l] = v;
                *slot = Some(c);
            }
        }
    }

    /// Block light level 0..=15 at a cell.
    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        self.packed(x, y, z).map(|v| v & 15).unwrap_or(0)
    }

    /// Sky light level 0..=15 at a cell. Above the world it is 15.
    #[inline]
    pub fn sky(&self, x: i32, y: i32, z: i32) -> u8 {
        if y >= self.size[1] {
            return 15;
        }
        self.packed(x, y, z).map(|v| v >> 4).unwrap_or(15)
    }

    /// Recompute light after a box of cells changed.
    pub fn changed_box(&mut self, world: &World, lo: [i32; 3], hi: [i32; 3]) {
        self.recompute(
            world,
            [lo[0] - R, lo[1] - R, lo[2] - R],
            [hi[0] + R, hi[1] + R, hi[2] + R],
        );
    }

    /// Recompute light after a change at `p`.
    pub fn changed(&mut self, world: &World, p: [i32; 3]) {
        self.changed_box(world, p, p);
    }

    /// Rebuild light in the columns of the box [lo, hi] (full height). Light
    /// is spread in scratch buffers that also cover the reach of light, so
    /// sources outside the box still count; then only the box is copied back.
    fn recompute(&mut self, world: &World, lo: [i32; 3], hi: [i32; 3]) {
        let sy = self.size[1];
        let lo = [lo[0].max(0), 0, lo[2].max(0)];
        let hi = [
            hi[0].min(self.size[0] - 1),
            sy - 1,
            hi[2].min(self.size[2] - 1),
        ];
        let elo = [(lo[0] - R).max(0), 0, (lo[2] - R).max(0)];
        let ehi = [
            (hi[0] + R).min(self.size[0] - 1),
            sy - 1,
            (hi[2] + R).min(self.size[2] - 1),
        ];
        let es = [ehi[0] - elo[0] + 1, sy, ehi[2] - elo[2] + 1];
        let eidx = |x: i32, y: i32, z: i32| -> Option<usize> {
            let (lx, lz) = (x - elo[0], z - elo[2]);
            if lx < 0 || y < 0 || lz < 0 || lx >= es[0] || y >= es[1] || lz >= es[2] {
                None
            } else {
                Some(((y * es[2] + lz) * es[0] + lx) as usize)
            }
        };
        let n = (es[0] * es[1] * es[2]) as usize;
        let mut bl = vec![0u8; n];
        let mut sk = vec![0u8; n];
        // the box's cells read once (the fill below visits each many times):
        // bit 7 opaque, low bits the light it gives
        let mut lut = [0u8; 256];
        for (b, v) in lut.iter_mut().enumerate().take(w::BLOCK_COUNT as usize) {
            let b = b as w::BlockId;
            *v = w::emission(b).min(15) | if w::is_opaque(b) { 0x80 } else { 0 };
        }
        let mut cell = vec![0u8; n];
        for y in 0..sy {
            for z in elo[2]..=ehi[2] {
                let row = ((y * es[2] + (z - elo[2])) * es[0]) as usize;
                for x in elo[0]..=ehi[0] {
                    cell[row + (x - elo[0]) as usize] = lut[world.get(x, y, z) as usize];
                }
            }
        }
        let opaque = |i: usize| cell[i] & 0x80 != 0;
        // highest opaque cell of each column
        let mut top = vec![-1i32; (es[0] * es[2]) as usize];
        for z in elo[2]..=ehi[2] {
            for x in elo[0]..=ehi[0] {
                let mut t = -1;
                for y in (0..sy).rev() {
                    if opaque(eidx(x, y, z).unwrap()) {
                        t = y;
                        break;
                    }
                }
                top[((z - elo[2]) * es[0] + (x - elo[0])) as usize] = t;
            }
        }
        let top_at = |x: i32, z: i32| -> i32 {
            if x < elo[0] || x > ehi[0] || z < elo[2] || z > ehi[2] {
                sy
            } else {
                top[((z - elo[2]) * es[0] + (x - elo[0])) as usize]
            }
        };
        let mut qb = VecDeque::new();
        let mut qs = VecDeque::new();
        for z in elo[2]..=ehi[2] {
            for x in elo[0]..=ehi[0] {
                let t = top_at(x, z);
                let nmax = [
                    top_at(x + 1, z),
                    top_at(x - 1, z),
                    top_at(x, z + 1),
                    top_at(x, z - 1),
                ]
                .into_iter()
                .max()
                .unwrap_or(t);
                for y in (t + 1).max(0)..sy {
                    let i = eidx(x, y, z).unwrap();
                    sk[i] = 15;
                    if y <= nmax {
                        qs.push_back((x, y, z));
                    }
                }
                for y in 0..sy {
                    let e = cell[eidx(x, y, z).unwrap()] & 0x0f;
                    if e > 0 {
                        bl[eidx(x, y, z).unwrap()] = e;
                        qb.push_back((x, y, z));
                    }
                }
            }
        }
        for (buf, q) in [(&mut bl, &mut qb), (&mut sk, &mut qs)] {
            while let Some((x, y, z)) = q.pop_front() {
                let l = buf[eidx(x, y, z).unwrap()];
                if l <= 1 {
                    continue;
                }
                for (dx, dy, dz) in [
                    (1, 0, 0),
                    (-1, 0, 0),
                    (0, 1, 0),
                    (0, -1, 0),
                    (0, 0, 1),
                    (0, 0, -1),
                ] {
                    let (nx, ny, nz) = (x + dx, y + dy, z + dz);
                    let Some(j) = eidx(nx, ny, nz) else { continue };
                    if buf[j] >= l - 1 || opaque(j) {
                        continue;
                    }
                    buf[j] = l - 1;
                    q.push_back((nx, ny, nz));
                }
            }
        }
        for y in 0..sy {
            for z in lo[2]..=hi[2] {
                for x in lo[0]..=hi[0] {
                    let j = eidx(x, y, z).unwrap();
                    self.put(x, y, z, (sk[j] << 4) | bl[j]);
                }
            }
        }
        // sections left all open sky need no storage
        for sy in 0..self.dims[1] {
            for sz in lo[2] / SECTION..=hi[2] / SECTION {
                for sx in lo[0] / SECTION..=hi[0] / SECTION {
                    if let Some(i) = self.sect(sx, sy, sz)
                        && self.secs[i]
                            .as_ref()
                            .is_some_and(|c| c.iter().all(|&v| v == OPEN))
                    {
                        self.secs[i] = None;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caves_are_dark_and_open_sky_is_bright() {
        let mut world = World::empty([16, 16, 16]);
        for z in 0..16 {
            for x in 0..16 {
                for y in 0..8 {
                    world.set(x, y, z, w::STONE);
                }
            }
        }
        // a sealed room at y 2..4
        for z in 4..10 {
            for x in 4..10 {
                for y in 2..5 {
                    world.set(x, y, z, w::AIR);
                }
            }
        }
        let l = Light::new(&world);
        assert_eq!(l.sky(8, 10, 8), 15);
        assert_eq!(l.sky(6, 3, 6), 0);
        // a lantern lights the room
        world.set(6, 2, 6, w::LANTERN);
        let mut l = l;
        l.changed(&world, [6, 2, 6]);
        assert_eq!(l.get(6, 3, 6), 14);
        assert_eq!(l.get(8, 3, 6), 12);
    }

    /// cargo test --release -p dyno-world light_speed -- --ignored --nocapture
    #[test]
    #[ignore]
    fn light_speed() {
        let world = World::generate([384, 96, 384], 20261122);
        let mut c = World::sparse(world.size);
        for s in world.held_list() {
            c.put_section(s, &world.encode_section(s)).unwrap();
        }
        let mut l = Light::new(&c);
        let t = std::time::Instant::now();
        let n = 20;
        for i in 0..n {
            let col: Vec<SectionPos> = (0..6).map(|y| [2 + i, y, 12]).collect();
            l.arrived(&c, &col);
        }
        eprintln!(
            "one column arriving: {:.1} ms",
            t.elapsed().as_secs_f64() * 1e3 / n as f64
        );
        let t = std::time::Instant::now();
        let _ = Light::new(&world);
        eprintln!("the whole world: {:.0} ms", t.elapsed().as_secs_f64() * 1e3);
    }

    #[test]
    fn streamed_sections_light_like_the_whole_world() {
        let world = World::generate([64, 48, 64], 7);
        let whole = Light::new(&world);
        let mut c = World::sparse(world.size);
        let mut l = Light::new(&c);
        assert_eq!(l.bytes(), 0, "nothing here yet: nothing stored");
        // sections arrive in two batches, far apart
        let all: Vec<_> = (0..3)
            .flat_map(|y| (0..4).flat_map(move |z| (0..4).map(move |x| [x, y, z])))
            .collect();
        let (a, b): (Vec<_>, Vec<_>) = all.iter().partition(|s| s[0] < 2);
        for batch in [a, b] {
            for s in &batch {
                c.put_section(*s, &world.encode_section(*s)).unwrap();
            }
            l.arrived(&c, &batch);
        }
        for y in 0..48 {
            for z in 0..64 {
                for x in 0..64 {
                    assert_eq!(l.sky(x, y, z), whole.sky(x, y, z), "sky at {x},{y},{z}");
                    assert_eq!(l.get(x, y, z), whole.get(x, y, z), "light at {x},{y},{z}");
                }
            }
        }
        assert!(l.bytes() < 48 * SECTION_CELLS, "open sky is not stored");
        l.forget([0, 0, 0]);
        assert_eq!(l.sky(1, 1, 1), 15);
    }
}
