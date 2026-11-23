//! Light: block light from glowing blocks, and sky light from open sky.
//! Both fade by one level per cell through air and are recomputed locally
//! around each change.
//!
//! Light is stored per section of 16³, both levels packed in one byte (sky
//! in the high nibble). A section that is all open sky with no block light
//! (the air above the land, and everything not yet streamed) costs nothing.

use crate::{self as w, Packed, SECTION, SECTION_CELLS, SectionPos, World};
use std::sync::Arc;

#[derive(Clone)]
pub struct Light {
    size: [i32; 3],
    dims: [i32; 3],
    secs: Vec<SectionLight>,
    /// sections whose light may have changed since `take_changed`
    changed: Vec<bool>,
    changed_list: Vec<usize>,
}

/// One section's light (both levels packed in a byte a cell), as copied
/// between threads (`Light::section`, `Light::set_section`). A section all
/// at one level (open sky; the dark under the ground) costs nothing.
///
/// Sections are shared between copies of the light (shards) until one of
/// them changes (copy on write), and packed with a palette when they hold
/// few levels.
#[derive(Clone, Debug, PartialEq)]
pub enum SectionLight {
    Uniform(u8),
    Packed(Arc<Packed>),
    Cells(Arc<[u8; SECTION_CELLS]>),
}

impl SectionLight {
    /// The smallest form of these levels.
    fn compacted(c: &[u8; SECTION_CELLS]) -> SectionLight {
        if c.iter().all(|&v| v == c[0]) {
            return SectionLight::Uniform(c[0]);
        }
        match Packed::pack(c) {
            Some(p) => SectionLight::Packed(Arc::new(p)),
            None => SectionLight::Cells(Arc::new(*c)),
        }
    }

    fn cells(&self) -> [u8; SECTION_CELLS] {
        match self {
            SectionLight::Uniform(v) => [*v; SECTION_CELLS],
            SectionLight::Packed(p) => p.unpack(),
            SectionLight::Cells(c) => **c,
        }
    }

    #[inline]
    fn get(&self, l: usize) -> u8 {
        match self {
            SectionLight::Uniform(v) => *v,
            SectionLight::Packed(p) => p.get(l),
            SectionLight::Cells(c) => c[l],
        }
    }

    fn bytes(&self) -> usize {
        match self {
            SectionLight::Uniform(_) => 0,
            SectionLight::Packed(p) => p.bytes(),
            SectionLight::Cells(_) => SECTION_CELLS,
        }
    }
}

const R: i32 = 15;

/// Light worked out for the columns of a box, ready to store.
struct Patch {
    lo: [i32; 3],
    hi: [i32; 3],
    /// the cells worked out are those below this height (a whole number
    /// of sections); above it, all open sky
    top: i32,
    /// packed levels, y then z then x
    light: Vec<u8>,
}
/// Sky 15, block 0: the value of every cell of an unallocated section.
const OPEN: u8 = 0xf0;

impl Light {
    /// Light for a world of this size with nothing worked out yet (open sky
    /// everywhere): for a copy filled from elsewhere (`set_section`).
    pub fn blank(world: &World) -> Self {
        let size = [
            world.size[0] as i32,
            world.size[1] as i32,
            world.size[2] as i32,
        ];
        let dims = world.section_dims();
        let n = (dims[0] * dims[1] * dims[2]) as usize;
        Light {
            size,
            dims,
            secs: vec![SectionLight::Uniform(OPEN); n],
            changed: vec![false; n],
            changed_list: Vec::new(),
        }
    }

    pub fn new(world: &World) -> Self {
        let mut l = Light::blank(world);
        if world.is_sparse() {
            let held = world.held_list();
            l.arrived(world, &held);
        } else {
            l.whole(world);
        }
        l
    }

    /// Light a whole world, in tiles of columns on every core (a tile's
    /// scratch is a few megabytes; the whole of a big world at once would
    /// be gigabytes).
    fn whole(&mut self, world: &World) {
        const TILE: i32 = 128;
        let [sx, _, sz] = self.size;
        let mut tiles = Vec::new();
        for z in (0..sz).step_by(TILE as usize) {
            for x in (0..sx).step_by(TILE as usize) {
                tiles.push((
                    [x, 0, z],
                    [(x + TILE).min(sx) - 1, 0, (z + TILE).min(sz) - 1],
                ));
            }
        }
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .clamp(1, 8);
        for group in tiles.chunks(threads) {
            let this = &*self;
            let patches: Vec<Patch> = std::thread::scope(|sc| {
                let jobs: Vec<_> = group
                    .iter()
                    .map(|(a, b)| sc.spawn(move || this.compute(world, *a, *b)))
                    .collect();
                jobs.into_iter()
                    .filter_map(|j| j.join().expect("a light thread panicked"))
                    .collect()
            });
            for p in patches {
                self.apply(p);
            }
        }
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
        // a box of columns (whole height), widened by the reach of light
        let boxed = |a: [i32; 2], b: [i32; 2]| {
            (
                [a[0] * SECTION - R, 0, a[1] * SECTION - R],
                [
                    b[0] * SECTION + SECTION - 1 + R,
                    0,
                    b[1] * SECTION + SECTION - 1 + R,
                ],
            )
        };
        if area <= cols.len() * 4 {
            // close together: one pass over the box round them all
            let (a, b) = boxed(lo, hi);
            self.recompute(world, a, b);
            return;
        }
        // scattered (a ring round the player): each column on its own, on
        // every core; overlapping patches agree (the same world made them)
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .clamp(1, 8)
            .min(cols.len());
        let per = cols.len().div_ceil(threads);
        let this = &*self;
        let patches: Vec<Patch> = std::thread::scope(|sc| {
            let jobs: Vec<_> = cols
                .chunks(per)
                .map(|part| {
                    sc.spawn(move || {
                        part.iter()
                            .filter_map(|c| {
                                let (a, b) = boxed(*c, *c);
                                this.compute(world, a, b)
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            jobs.into_iter()
                .flat_map(|j| j.join().expect("a light thread panicked"))
                .collect()
        });
        for p in patches {
            self.apply(p);
        }
    }

    /// Free the light of a section that left a streamed world.
    pub fn forget(&mut self, s: SectionPos) {
        if let Some(i) = self.sect(s[0], s[1], s[2]) {
            self.secs[i] = SectionLight::Uniform(OPEN);
            self.mark(i);
        }
    }

    fn mark(&mut self, i: usize) {
        if !self.changed[i] {
            self.changed[i] = true;
            self.changed_list.push(i);
        }
    }

    /// The sections whose light may have changed since the last call.
    pub fn take_changed(&mut self) -> Vec<SectionPos> {
        let d = self.dims;
        let list = std::mem::take(&mut self.changed_list);
        list.into_iter()
            .map(|i| {
                self.changed[i] = false;
                let i = i as i32;
                [i % d[0], i / (d[0] * d[2]), (i / d[0]) % d[2]]
            })
            .collect()
    }

    /// A copy of one section's light.
    pub fn section(&self, s: SectionPos) -> SectionLight {
        self.sect(s[0], s[1], s[2])
            .map_or(SectionLight::Uniform(OPEN), |i| self.secs[i].clone())
    }

    /// Store one section's light, worked out elsewhere.
    pub fn set_section(&mut self, s: SectionPos, light: SectionLight) {
        if let Some(i) = self.sect(s[0], s[1], s[2]) {
            self.secs[i] = light;
        }
    }

    /// Bytes of light held (for measurements).
    pub fn bytes(&self) -> usize {
        self.secs.iter().map(SectionLight::bytes).sum()
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
        let l = ((y % SECTION * SECTION + z % SECTION) * SECTION + x % SECTION) as usize;
        Some(self.secs[i].get(l))
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
        if let Some(p) = self.compute(world, lo, hi) {
            self.apply(p);
        }
    }

    /// The light of the columns of the box [lo, hi], worked out but not stored.
    fn compute(&self, world: &World, lo: [i32; 3], hi: [i32; 3]) -> Option<Patch> {
        self.compute_with(world, lo, hi, true)
    }

    /// `compute`, working out the open sky cell by cell too if not `clamp`.
    fn compute_with(
        &self,
        world: &World,
        lo: [i32; 3],
        hi: [i32; 3],
        clamp: bool,
    ) -> Option<Patch> {
        let sy = self.size[1];
        let lo = [lo[0].max(0), 0, lo[2].max(0)];
        let hi = [
            hi[0].min(self.size[0] - 1),
            sy - 1,
            hi[2].min(self.size[2] - 1),
        ];
        if lo[0] > hi[0] || lo[2] > hi[2] {
            return None;
        }
        let elo = [(lo[0] - R).max(0), 0, (lo[2] - R).max(0)];
        let ehi = [
            (hi[0] + R).min(self.size[0] - 1),
            sy - 1,
            (hi[2] + R).min(self.size[2] - 1),
        ];
        // the open sky over the land (layers of sections all air, or all of
        // some clear block that gives no light, over every column the light
        // could come from) is lit by the sky alone: work out the light up
        // to a section above the highest layer with anything in it (light
        // from a block reaches less than a section), and leave the rest open
        let clear = |s: SectionPos| {
            let b = if world.has_section(s) {
                world.section_uniform(s)
            } else {
                Some(w::AIR) // not here yet: air, as `World::get` reads it
            };
            b.is_some_and(|b| !w::is_opaque(b) && w::emission(b) == 0)
        };
        let mut highest = self.dims[1] - 1;
        while clamp
            && highest >= 0
            && (elo[2] / SECTION..=ehi[2] / SECTION)
                .all(|z| (elo[0] / SECTION..=ehi[0] / SECTION).all(|x| clear([x, highest, z])))
        {
            highest -= 1;
        }
        let sy = ((highest + 2) * SECTION).min(sy);
        let (ex, ez) = (ehi[0] - elo[0] + 1, ehi[2] - elo[2] + 1);
        // the scratch box with a border of one opaque cell all round, so the
        // fill needs no bounds checks (light never leaves the box)
        let (px, pz) = ((ex + 2) as usize, (ez + 2) as usize);
        let layer = px * pz;
        let n = layer * (sy + 2) as usize;
        let at = |x: i32, y: i32, z: i32| -> usize {
            ((y + 1) as usize * pz + (z - elo[2] + 1) as usize) * px + (x - elo[0] + 1) as usize
        };
        // the box's cells read once (the fill visits each many times):
        // bit 7 opaque, low bits the light it gives
        let mut lut = [0u8; 256];
        for (b, v) in lut.iter_mut().enumerate().take(w::BLOCK_COUNT as usize) {
            let b = b as w::BlockId;
            *v = w::emission(b).min(15) | if w::is_opaque(b) { 0x80 } else { 0 };
        }
        let mut cell = vec![0x80u8; n];
        for y in 0..sy {
            for z in elo[2]..=ehi[2] {
                let row = at(elo[0], y, z);
                for (k, x) in (elo[0]..=ehi[0]).enumerate() {
                    cell[row + k] = lut[world.get(x, y, z) as usize];
                }
            }
        }
        let mut bl = vec![0u8; n];
        let mut sk = vec![0u8; n];
        // highest opaque cell of each column
        let mut top = vec![-1i32; (ex * ez) as usize];
        for z in elo[2]..=ehi[2] {
            for x in elo[0]..=ehi[0] {
                let mut t = -1;
                for y in (0..sy).rev() {
                    if cell[at(x, y, z)] & 0x80 != 0 {
                        t = y;
                        break;
                    }
                }
                top[((z - elo[2]) * ex + (x - elo[0])) as usize] = t;
            }
        }
        let top_at = |x: i32, z: i32| -> i32 {
            if x < elo[0] || x > ehi[0] || z < elo[2] || z > ehi[2] {
                sy
            } else {
                top[((z - elo[2]) * ex + (x - elo[0])) as usize]
            }
        };
        let mut qb: Vec<u32> = Vec::new();
        let mut qs: Vec<u32> = Vec::new();
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
                    let i = at(x, y, z);
                    sk[i] = 15;
                    if y <= nmax {
                        qs.push(i as u32);
                    }
                }
                for y in 0..sy {
                    let i = at(x, y, z);
                    let e = cell[i] & 0x0f;
                    if e > 0 {
                        bl[i] = e;
                        qb.push(i as u32);
                    }
                }
            }
        }
        let steps = [
            1isize,
            -1,
            px as isize,
            -(px as isize),
            layer as isize,
            -(layer as isize),
        ];
        for (buf, q) in [(&mut bl, &mut qb), (&mut sk, &mut qs)] {
            let mut head = 0;
            while head < q.len() {
                let i = q[head] as usize;
                head += 1;
                let l = buf[i];
                if l <= 1 {
                    continue;
                }
                for d in steps {
                    let j = (i as isize + d) as usize;
                    if buf[j] >= l - 1 || cell[j] & 0x80 != 0 {
                        continue;
                    }
                    buf[j] = l - 1;
                    q.push(j as u32);
                }
            }
        }
        let mut out = Vec::with_capacity(((hi[0] - lo[0] + 1) * (hi[2] - lo[2] + 1) * sy) as usize);
        for y in 0..sy {
            for z in lo[2]..=hi[2] {
                for x in lo[0]..=hi[0] {
                    let j = at(x, y, z);
                    out.push((sk[j] << 4) | bl[j]);
                }
            }
        }
        Some(Patch {
            lo,
            hi,
            top: sy,
            light: out,
        })
    }

    /// Store worked-out light, a section at a time, each in its smallest form.
    fn apply(&mut self, p: Patch) {
        let Patch { lo, hi, top, light } = p;
        let (w, d) = ((hi[0] - lo[0] + 1) as usize, (hi[2] - lo[2] + 1) as usize);
        let sy_all = self.size[1];
        for sy in 0..self.dims[1] {
            for sz in lo[2] / SECTION..=hi[2] / SECTION {
                for sx in lo[0] / SECTION..=hi[0] / SECTION {
                    let Some(i) = self.sect(sx, sy, sz) else {
                        continue;
                    };
                    self.mark(i);
                    let (x0, x1) = (
                        (sx * SECTION).max(lo[0]),
                        (sx * SECTION + SECTION - 1).min(hi[0]),
                    );
                    let (z0, z1) = (
                        (sz * SECTION).max(lo[2]),
                        (sz * SECTION + SECTION - 1).min(hi[2]),
                    );
                    let (y0, y1) = (sy * SECTION, (sy * SECTION + SECTION - 1).min(sy_all - 1));
                    if y0 >= top {
                        // open sky
                        if self.secs[i] == SectionLight::Uniform(OPEN) {
                            continue;
                        }
                        self.mark(i);
                        let whole = x0 == sx * SECTION
                            && x1 == sx * SECTION + SECTION - 1
                            && z0 == sz * SECTION
                            && z1 == sz * SECTION + SECTION - 1;
                        if whole {
                            self.secs[i] = SectionLight::Uniform(OPEN);
                            continue;
                        }
                        let mut c = self.secs[i].cells();
                        for y in y0..=y1 {
                            for z in z0..=z1 {
                                let l = ((y % SECTION * SECTION + z % SECTION) * SECTION) as usize;
                                for x in x0..=x1 {
                                    c[l + (x % SECTION) as usize] = OPEN;
                                }
                            }
                        }
                        self.secs[i] = SectionLight::compacted(&c);
                        continue;
                    }
                    let mut c = self.secs[i].cells();
                    for y in y0..=y1 {
                        for z in z0..=z1 {
                            let row = (y as usize * d + (z - lo[2]) as usize) * w;
                            let l = ((y % SECTION * SECTION + z % SECTION) * SECTION) as usize;
                            for x in x0..=x1 {
                                c[l + (x % SECTION) as usize] = light[row + (x - lo[0]) as usize];
                            }
                        }
                    }
                    self.secs[i] = SectionLight::compacted(&c);
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

    /// Lighting the columns a client gets round a point of a saved world,
    /// as the client's worker does (six columns at a time, nearest first):
    ///
    /// ```sh
    /// DYNO_BENCH_WORLD=world.dynw DYNO_BENCH_AT=1024,1024 \
    ///   cargo test --release -p dyno-world light_round_a_point -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn light_round_a_point() {
        let Ok(path) = std::env::var("DYNO_BENCH_WORLD") else {
            eprintln!("set DYNO_BENCH_WORLD (and DYNO_BENCH_AT=x,z)");
            return;
        };
        let world = World::decode(&std::fs::read(path).unwrap()).unwrap();
        let d = world.section_dims();
        let at: Vec<f32> = std::env::var("DYNO_BENCH_AT")
            .map(|v| v.split(',').filter_map(|x| x.trim().parse().ok()).collect())
            .unwrap_or_else(|_| vec![world.size[0] as f32 / 2.0, world.size[2] as f32 / 2.0]);
        let col = [(at[0] as i32) / SECTION, (at[1] as i32) / SECTION];
        // what each layer is made of, over the 9 × 9 columns round the point
        let mut layers = Vec::new();
        for y in 0..d[1] {
            let (mut open, mut solid, mut mixed) = (0, 0, 0);
            for z in col[1] - 4..=col[1] + 4 {
                for x in col[0] - 4..=col[0] + 4 {
                    match world.section_uniform([x, y, z]) {
                        Some(b) if w::is_opaque(b) => solid += 1,
                        Some(_) => open += 1,
                        None => mixed += 1,
                    }
                }
            }
            layers.push(format!("{y}:{open}/{solid}/{mixed}"));
        }
        eprintln!("layers (open/solid/mixed of 81): {}", layers.join(" "));
        // the columns within 48 m, nearest first
        let mut cols: Vec<(f32, [i32; 2])> = Vec::new();
        for z in col[1] - 4..=col[1] + 4 {
            for x in col[0] - 4..=col[0] + 4 {
                let dx = (x * SECTION + SECTION / 2) as f32 - at[0];
                let dz = (z * SECTION + SECTION / 2) as f32 - at[1];
                let h = dx.hypot(dz);
                if h <= 48.0 && x >= 0 && z >= 0 && x < d[0] && z < d[2] {
                    cols.push((h, [x, z]));
                }
            }
        }
        cols.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut c = World::sparse(world.size);
        for (_, [x, z]) in &cols {
            for y in 0..d[1] {
                c.put_section([*x, y, *z], &world.encode_section([*x, y, *z]))
                    .unwrap();
            }
        }
        let mut best = f64::MAX;
        for _ in 0..5 {
            let mut l = Light::blank(&c);
            let t = std::time::Instant::now();
            for part in cols.chunks(6) {
                let batch: Vec<SectionPos> = part
                    .iter()
                    .flat_map(|(_, [x, z])| (0..d[1]).map(move |y| [*x, y, *z]))
                    .collect();
                l.arrived(&c, &batch);
            }
            best = best.min(t.elapsed().as_secs_f64() * 1e3);
        }
        eprintln!(
            "{} columns {} sections tall lit in {best:.0} ms ({:.1} ms a column, the best of 5)",
            cols.len(),
            d[1],
            best / cols.len() as f64
        );
        // the sections a client's view holds (160 m), and the share all one block
        for r in [40.0f32, 160.0] {
            let (mut n, mut one) = (0, 0);
            for z in 0..d[2] {
                for x in 0..d[0] {
                    let dx = (x * SECTION + SECTION / 2) as f32 - at[0];
                    let dz = (z * SECTION + SECTION / 2) as f32 - at[1];
                    if dx.hypot(dz) <= r {
                        for y in 0..d[1] {
                            n += 1;
                            one += world.section_uniform([x, y, z]).is_some() as usize;
                        }
                    }
                }
            }
            eprintln!("within {r} m: {n} sections, {one} all one block");
        }
        if std::env::var_os("DYNO_BENCH_WHOLE").is_some() {
            let t = std::time::Instant::now();
            let _ = Light::new(&world);
            eprintln!("the whole world lit in {:.1} s", t.elapsed().as_secs_f64());
        }
    }

    /// The open sky over the land is not worked out cell by cell, and the
    /// light is the same as if it were: under an overhang high up, round a
    /// glowing block just under the open sky and one alone in it.
    #[test]
    fn the_open_sky_is_lit_as_if_worked_out() {
        let mut world = World::generate([64, 128, 64], 11);
        let top = (0..64)
            .flat_map(|z| (0..64).map(move |x| (x, z)))
            .filter_map(|(x, z)| world.column_top(x, z))
            .map(|(y, _)| y)
            .max()
            .unwrap();
        // a roof two sections over the land, a lamp under it at the edge of
        // its section, and a lamp alone in the sky
        let roof = (top / SECTION + 2) * SECTION;
        assert!(roof + 2 * SECTION < 128, "room over the land ({top})");
        for z in 10..30 {
            for x in 10..30 {
                world.set(x, roof, z, w::STONE);
            }
        }
        let lamp = w::BLOCK_COUNT as usize;
        let glow = (0..lamp as u16)
            .map(|b| b as w::BlockId)
            .find(|b| w::emission(*b) >= 14)
            .expect("a glowing block");
        world.set(40, roof - 1, 40, glow);
        world.set(20, roof + SECTION + 15, 50, glow);
        world.set(50, roof + 1, 12, glow);
        world.compact();
        let lit = Light::new(&world);
        let mut full = Light::blank(&world);
        let p = full
            .compute_with(&world, [0, 0, 0], [63, 127, 63], false)
            .unwrap();
        full.apply(p);
        for y in 0..128 {
            for z in 0..64 {
                for x in 0..64 {
                    assert_eq!(lit.sky(x, y, z), full.sky(x, y, z), "sky at {x},{y},{z}");
                    assert_eq!(lit.get(x, y, z), full.get(x, y, z), "light at {x},{y},{z}");
                }
            }
        }
        // and streamed in, a column at a time
        let mut c = World::sparse(world.size);
        let mut l = Light::new(&c);
        let d = world.section_dims();
        for z in 0..d[2] {
            for x in 0..d[0] {
                let col: Vec<SectionPos> = (0..d[1]).map(|y| [x, y, z]).collect();
                for s in &col {
                    c.put_section(*s, &world.encode_section(*s)).unwrap();
                }
                l.arrived(&c, &col);
            }
        }
        for y in 0..128 {
            for z in 0..64 {
                for x in 0..64 {
                    assert_eq!(
                        l.sky(x, y, z),
                        full.sky(x, y, z),
                        "streamed sky at {x},{y},{z}"
                    );
                    assert_eq!(
                        l.get(x, y, z),
                        full.get(x, y, z),
                        "streamed light at {x},{y},{z}"
                    );
                }
            }
        }
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
        // a copy kept elsewhere, from the sections that changed
        let mut copy = Light::blank(&c);
        for s in l.take_changed() {
            copy.set_section(s, l.section(s));
        }
        assert!(l.take_changed().is_empty());
        for (x, y, z) in [(5, 5, 5), (40, 20, 33), (63, 47, 0)] {
            assert_eq!(copy.sky(x, y, z), l.sky(x, y, z));
            assert_eq!(copy.get(x, y, z), l.get(x, y, z));
        }
        l.forget([0, 0, 0]);
        assert_eq!(l.sky(1, 1, 1), 15);
    }
}
