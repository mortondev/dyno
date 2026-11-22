//! Block light: light from glowing blocks spreads through air and fades by one
//! level per cell. Recomputed locally around each change.

use dyno_world::{self as w, World};
use std::collections::VecDeque;

pub struct Light {
    size: [i32; 3],
    data: Vec<u8>,
}

const R: i32 = 15;

impl Light {
    pub fn new(world: &World) -> Self {
        let size = [
            world.size[0] as i32,
            world.size[1] as i32,
            world.size[2] as i32,
        ];
        let mut l = Light {
            size,
            data: vec![0; (size[0] * size[1] * size[2]) as usize],
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

    /// Light level 0..=15 at a cell.
    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        self.idx(x, y, z).map(|i| self.data[i]).unwrap_or(0)
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
        self.recompute(
            world,
            [p[0] - R, p[1] - R, p[2] - R],
            [p[0] + R, p[1] + R, p[2] + R],
        );
    }

    /// Rebuild light inside the box [lo, hi]. Light is spread in a scratch
    /// buffer that covers the box plus the reach of light, so emitters outside
    /// the box still count, and then only the box is copied back.
    fn recompute(&mut self, world: &World, lo: [i32; 3], hi: [i32; 3]) {
        let lo = [lo[0].max(0), lo[1].max(0), lo[2].max(0)];
        let hi = [
            hi[0].min(self.size[0] - 1),
            hi[1].min(self.size[1] - 1),
            hi[2].min(self.size[2] - 1),
        ];
        let elo = [(lo[0] - R).max(0), (lo[1] - R).max(0), (lo[2] - R).max(0)];
        let ehi = [
            (hi[0] + R).min(self.size[0] - 1),
            (hi[1] + R).min(self.size[1] - 1),
            (hi[2] + R).min(self.size[2] - 1),
        ];
        let es = [
            ehi[0] - elo[0] + 1,
            ehi[1] - elo[1] + 1,
            ehi[2] - elo[2] + 1,
        ];
        let eidx = |x: i32, y: i32, z: i32| -> Option<usize> {
            let (lx, ly, lz) = (x - elo[0], y - elo[1], z - elo[2]);
            if lx < 0 || ly < 0 || lz < 0 || lx >= es[0] || ly >= es[1] || lz >= es[2] {
                None
            } else {
                Some(((ly * es[2] + lz) * es[0] + lx) as usize)
            }
        };
        let mut tmp = vec![0u8; (es[0] * es[1] * es[2]) as usize];
        let mut q = VecDeque::new();
        for y in elo[1]..=ehi[1] {
            for z in elo[2]..=ehi[2] {
                for x in elo[0]..=ehi[0] {
                    let e = w::emission(world.get(x, y, z));
                    if e > 0 {
                        tmp[eidx(x, y, z).unwrap()] = e;
                        q.push_back((x, y, z));
                    }
                }
            }
        }
        while let Some((x, y, z)) = q.pop_front() {
            let l = tmp[eidx(x, y, z).unwrap()];
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
                if tmp[j] >= l - 1 || w::is_opaque(world.get(nx, ny, nz)) {
                    continue;
                }
                tmp[j] = l - 1;
                q.push_back((nx, ny, nz));
            }
        }
        for y in lo[1]..=hi[1] {
            for z in lo[2]..=hi[2] {
                for x in lo[0]..=hi[0] {
                    let i = self.idx(x, y, z).unwrap();
                    self.data[i] = tmp[eidx(x, y, z).unwrap()];
                }
            }
        }
    }
}
