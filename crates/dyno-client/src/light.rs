//! Light: block light from glowing blocks, and sky light from open sky.
//! Both fade by one level per cell through air and are recomputed locally
//! around each change.

use dyno_world::{self as w, World};
use std::collections::VecDeque;

pub struct Light {
    size: [i32; 3],
    block: Vec<u8>,
    sky: Vec<u8>,
}

const R: i32 = 15;

impl Light {
    pub fn new(world: &World) -> Self {
        let size = [
            world.size[0] as i32,
            world.size[1] as i32,
            world.size[2] as i32,
        ];
        let n = (size[0] * size[1] * size[2]) as usize;
        let mut l = Light {
            size,
            block: vec![0; n],
            sky: vec![0; n],
        };
        l.recompute(world, [0, 0, 0], [size[0] - 1, size[1] - 1, size[2] - 1]);
        l
    }

    #[inline]
    fn idx(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        if x < 0 || y < 0 || z < 0 || x >= self.size[0] || y >= self.size[1] || z >= self.size[2] {
            None
        } else {
            Some(((y * self.size[2] + z) * self.size[0] + x) as usize)
        }
    }

    /// Block light level 0..=15 at a cell.
    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        self.idx(x, y, z).map(|i| self.block[i]).unwrap_or(0)
    }

    /// Sky light level 0..=15 at a cell. Above the world it is 15.
    #[inline]
    pub fn sky(&self, x: i32, y: i32, z: i32) -> u8 {
        if y >= self.size[1] {
            return 15;
        }
        self.idx(x, y, z).map(|i| self.sky[i]).unwrap_or(15)
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
        // highest opaque cell of each column
        let mut top = vec![-1i32; (es[0] * es[2]) as usize];
        for z in elo[2]..=ehi[2] {
            for x in elo[0]..=ehi[0] {
                let mut t = -1;
                for y in (0..sy).rev() {
                    if w::is_opaque(world.get(x, y, z)) {
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
                    let e = w::emission(world.get(x, y, z));
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
                    if buf[j] >= l - 1 || w::is_opaque(world.get(nx, ny, nz)) {
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
                    let i = self.idx(x, y, z).unwrap();
                    let j = eidx(x, y, z).unwrap();
                    self.block[i] = bl[j];
                    self.sky[i] = sk[j];
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
}
