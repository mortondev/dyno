//! Lighting and meshing the streamed world off the main thread.
//!
//! The game keeps its own copy of the world (movement, picking, the
//! minimap) and of its light (figures are lit by it). A worker thread keeps
//! another copy of both: the game sends it every section that arrives or
//! leaves and every block edit, in order, and where the camera is. The
//! worker lights whole columns once they are here, meshes the sections that
//! are ready (a section and its 26 neighbours have arrived: the mesh and the
//! light read past its edge), nearest the camera and in front of it first,
//! and sends back the meshes and copies of the light it changed. The game
//! uploads the meshes within a time budget each frame (`app.rs`), so the
//! frame never waits on the world.
//!
//! Edits come first: a dug block is relit and remeshed before anything that
//! streamed in. Meshes for a section the game has dropped since are
//! ignored when they arrive.

use crate::render;
use crate::terrain_mesh::SectionGeo;
use dyno_world::light::{Light, SectionLight};
use dyno_world::{BlockId, SECTION, SectionData, SectionPos, World};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError, channel};
use std::time::{Duration, Instant};

enum Job {
    Put(Vec<(SectionPos, SectionData)>),
    Drop(Vec<SectionPos>),
    Block([i32; 3], BlockId),
    Fill([i32; 3], [i32; 3], BlockId),
    Focus([f32; 3], [f32; 3]),
}

/// What the worker made since it last sent.
struct Made {
    light: Vec<(SectionPos, SectionLight)>,
    meshes: Vec<(SectionPos, SectionGeo)>,
}

/// Columns lit together (a column's light takes a few milliseconds).
const LIGHT_COLUMNS: usize = 6;
/// Sections meshed together: small, so an edit waits little behind them.
const MESH_BATCH: usize = 16;
/// A column waits this long to be whole before it is lit anyway.
const WHOLE_WAIT: Duration = Duration::from_millis(500);

/// The game's side of the worker.
pub struct WorldWork {
    tx: Sender<Job>,
    rx: Receiver<Made>,
    /// meshes made and not uploaded yet, in the order they were made
    pub queue: VecDeque<(SectionPos, SectionGeo)>,
    /// jobs sent and not yet taken in by the worker
    pending: Arc<AtomicUsize>,
    /// the worker has light or meshing to do
    busy: Arc<AtomicBool>,
    /// results sent by the worker, and taken in here
    sent: Arc<AtomicUsize>,
    taken: usize,
    focus: Option<([f32; 3], [f32; 3])>,
    /// `DYNO_WORLD_INLINE=1`: the same work done on the calling thread, a
    /// step per `poll` (as before the worker: to compare frame times)
    inline: Option<Box<Worker>>,
}

impl WorldWork {
    /// Start a worker on a copy of the world as it is now (the sections
    /// that arrived with the join are lit and meshed first).
    pub fn start(world: World) -> WorldWork {
        let (tx, jobs) = channel();
        let (made, rx) = channel();
        let pending = Arc::new(AtomicUsize::new(0));
        let busy = Arc::new(AtomicBool::new(true));
        let sent = Arc::new(AtomicUsize::new(0));
        let (p, b, n) = (pending.clone(), busy.clone(), sent.clone());
        let inline = std::env::var_os("DYNO_WORLD_INLINE").is_some();
        let inline = if inline {
            drop((jobs, made));
            Some(Box::new(Worker::new(world)))
        } else {
            std::thread::Builder::new()
                .name("world".into())
                .spawn(move || Worker::new(world).run(jobs, made, p, b, n))
                .expect("the world thread starts");
            None
        };
        WorldWork {
            tx,
            rx,
            queue: VecDeque::new(),
            pending,
            busy,
            sent,
            taken: 0,
            focus: None,
            inline,
        }
    }

    fn send(&mut self, job: Job) {
        if let Some(w) = self.inline.as_mut() {
            w.take(job);
            return;
        }
        self.pending.fetch_add(1, Ordering::SeqCst);
        if self.tx.send(job).is_err() {
            self.pending.fetch_sub(1, Ordering::SeqCst);
        }
    }

    /// Sections that arrived (the game has stored them in its own copy;
    /// the two copies share them).
    pub fn put(&mut self, list: Vec<(SectionPos, SectionData)>) {
        if !list.is_empty() {
            self.send(Job::Put(list));
        }
    }

    /// Sections the server let go of.
    pub fn drop_sections(&mut self, list: Vec<SectionPos>) {
        if !list.is_empty() {
            self.send(Job::Drop(list));
        }
    }

    /// A block changed (relit and remeshed before anything else).
    pub fn block(&mut self, pos: [i32; 3], block: BlockId) {
        self.send(Job::Block(pos, block));
    }

    /// A box of cells was filled with one block.
    pub fn fill(&mut self, a: [i32; 3], b: [i32; 3], block: BlockId) {
        self.send(Job::Fill(a, b, block));
    }

    /// Where the camera is and looks: what is near it and in front of it is
    /// meshed first. Sent only when it moved or turned noticeably.
    pub fn focus(&mut self, cam: [f32; 3], dir: [f32; 3]) {
        let moved = self.focus.is_none_or(|(c, d)| {
            let dc = (0..3).map(|i| (c[i] - cam[i]).powi(2)).sum::<f32>();
            let dd = (0..3).map(|i| d[i] * dir[i]).sum::<f32>();
            dc > 16.0 || dd < 0.9
        });
        if moved {
            self.focus = Some((cam, dir));
            self.send(Job::Focus(cam, dir));
        }
    }

    /// Take in what the worker made: light goes into the game's copy at
    /// once (a copy per section), meshes join the upload queue.
    pub fn poll(&mut self, light: &mut Light) {
        if let Some(w) = self.inline.as_mut() {
            if w.has_work()
                && let (_, Some(m)) = w.step()
            {
                for (s, l) in m.light {
                    light.set_section(s, l);
                }
                self.queue.extend(m.meshes);
            }
            return;
        }
        while let Ok(m) = self.rx.try_recv() {
            self.taken += 1;
            for (s, l) in m.light {
                light.set_section(s, l);
            }
            self.queue.extend(m.meshes);
        }
    }

    /// Nothing sent is still waiting, being worked on or to be uploaded.
    pub fn idle(&self) -> bool {
        if let Some(w) = &self.inline {
            return !w.has_work() && self.queue.is_empty();
        }
        self.pending.load(Ordering::SeqCst) == 0
            && !self.busy.load(Ordering::SeqCst)
            && self.sent.load(Ordering::SeqCst) == self.taken
            && self.queue.is_empty()
    }
}

/// The worker's side: its own world and light, and what is left to do.
struct Worker {
    world: World,
    light: Light,
    /// arrived and not lit yet, and when they arrived
    unlit: Vec<(SectionPos, Instant)>,
    /// meshed since they last arrived
    built: HashSet<SectionPos>,
    /// to mesh (if ready) once their light is done
    todo: HashSet<SectionPos>,
    /// changed by an edit: meshed before anything else
    urgent: HashSet<SectionPos>,
    cam: [f32; 3],
    dir: [f32; 3],
    /// levels of detail of the ground, from the camera
    lods: crate::fine::Lods,
    /// the level each built section was meshed at
    built_lod: std::collections::HashMap<SectionPos, crate::fine::Lod>,
    /// the camera has been told (until then leaves are meshed in full)
    focused: bool,
    /// sections with leaves drawn as cubes: whether in clumps (near the
    /// camera) or a cube a block; remeshed when the camera makes it the other
    leafy: std::collections::HashMap<SectionPos, bool>,
    /// where the camera was when `leafy` was last looked through
    leafy_at: [f32; 3],
}

impl Worker {
    fn new(world: World) -> Worker {
        let dims = world.section_dims();
        let held = world.held_list();
        let at = Instant::now() - WHOLE_WAIT;
        Worker {
            light: Light::blank(&world),
            world,
            unlit: held.into_iter().map(|s| (s, at)).collect(),
            built: HashSet::new(),
            todo: HashSet::new(),
            urgent: HashSet::new(),
            cam: [0.0; 3],
            dir: [0.0, 0.0, -1.0],
            lods: crate::fine::Lods::new(dims),
            built_lod: std::collections::HashMap::new(),
            focused: false,
            leafy: std::collections::HashMap::new(),
            leafy_at: [f32::MAX; 3],
        }
    }

    fn take(&mut self, job: Job) {
        match job {
            Job::Put(list) => {
                let now = Instant::now();
                for (s, data) in list {
                    if self.world.put_prepared(s, data).is_ok() {
                        self.built.remove(&s);
                        self.unlit.retain(|(u, _)| *u != s);
                        self.unlit.push((s, now));
                    }
                }
            }
            Job::Drop(list) => {
                for s in list {
                    self.world.drop_section(s);
                    self.light.forget(s);
                    self.built.remove(&s);
                    self.built_lod.remove(&s);
                    self.leafy.remove(&s);
                    self.todo.remove(&s);
                    self.urgent.remove(&s);
                    self.unlit.retain(|(u, _)| *u != s);
                }
            }
            Job::Block(pos, block) => {
                if self.world.get(pos[0], pos[1], pos[2]) != block
                    && self.world.set(pos[0], pos[1], pos[2], block)
                {
                    self.edited(pos, pos);
                }
            }
            Job::Fill(a, b, block) => {
                for y in a[1]..=b[1] {
                    for z in a[2]..=b[2] {
                        for x in a[0]..=b[0] {
                            self.world.set(x, y, z, block);
                        }
                    }
                }
                self.edited(a, b);
            }
            Job::Focus(cam, dir) => {
                self.cam = cam;
                self.dir = dir;
                self.focused = true;
                self.leaves_again();
                // ground whose level of detail changed is meshed again
                if self.lods.update(cam) {
                    for (s, l) in &self.built_lod {
                        if *l != self.lods.lod(*s) {
                            self.todo.insert(*s);
                        }
                    }
                }
            }
        }
    }

    /// Relight round an edit and remesh every section its light reaches.
    fn edited(&mut self, lo: [i32; 3], hi: [i32; 3]) {
        self.light.changed_box(&self.world, lo, hi);
        let r = 15;
        let d = self.world.section_dims();
        let s0 = |v: i32, i: usize| (v - r).div_euclid(SECTION).clamp(0, d[i] - 1);
        let s1 = |v: i32, i: usize| (v + r).div_euclid(SECTION).clamp(0, d[i] - 1);
        for sy in s0(lo[1], 1)..=s1(hi[1], 1) {
            for sz in s0(lo[2], 2)..=s1(hi[2], 2) {
                for sx in s0(lo[0], 0)..=s1(hi[0], 0) {
                    self.urgent.insert([sx, sy, sz]);
                }
            }
        }
    }

    /// Remesh the sections whose leaves should now be drawn finer or
    /// coarser, the camera having moved on.
    fn leaves_again(&mut self) {
        let moved: f32 = (0..3)
            .map(|i| (self.cam[i] - self.leafy_at[i]).powi(2))
            .sum();
        if moved < 16.0 {
            return;
        }
        self.leafy_at = self.cam;
        let Some(f) = crate::foliage::current() else {
            return;
        };
        for (s, fine) in &self.leafy {
            if f.fine(Some(self.cam), *s, Some(*fine)) != *fine && self.built.contains(s) {
                self.todo.insert(*s);
            }
        }
    }

    /// How soon a section should be meshed: near the camera and in front of it first.
    fn priority(&self, s: SectionPos) -> f32 {
        let c = [
            (s[0] * SECTION + SECTION / 2) as f32 - self.cam[0],
            (s[1] * SECTION + SECTION / 2) as f32 - self.cam[1],
            (s[2] * SECTION + SECTION / 2) as f32 - self.cam[2],
        ];
        let dist = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
        let ahead = c[0] * self.dir[0] + c[1] * self.dir[1] + c[2] * self.dir[2];
        if ahead > -SECTION as f32 {
            dist * 0.5
        } else {
            dist
        }
    }

    /// Light up to a few columns that are whole (or waited long enough),
    /// nearest the camera first. Returns whether it lit any.
    fn light_some(&mut self) -> bool {
        let layers = self.world.section_dims()[1];
        let mut cols: Vec<([i32; 2], f32)> = Vec::new();
        for (s, at) in &self.unlit {
            let c = [s[0], s[2]];
            if cols.iter().any(|(k, _)| *k == c) {
                continue;
            }
            let whole = (0..layers).all(|y| self.world.has_section([c[0], y, c[1]]));
            if whole || at.elapsed() >= WHOLE_WAIT {
                let dx = (c[0] * SECTION + SECTION / 2) as f32 - self.cam[0];
                let dz = (c[1] * SECTION + SECTION / 2) as f32 - self.cam[2];
                cols.push((c, dx.hypot(dz)));
            }
        }
        if cols.is_empty() {
            return false;
        }
        cols.sort_by(|a, b| a.1.total_cmp(&b.1));
        cols.truncate(LIGHT_COLUMNS);
        let mut batch = Vec::new();
        self.unlit.retain(|(s, _)| {
            let take = cols.iter().any(|(c, _)| *c == [s[0], s[2]]);
            if take {
                batch.push(*s);
            }
            !take
        });
        self.light.arrived(&self.world, &batch);
        // what each arrival makes ready: itself and its neighbours not yet
        // meshed (no built mesh read it, and light reaches less than a
        // section), and what lies below it in its column (the sky's light
        // falls straight down)
        let d = self.world.section_dims();
        for s in &batch {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    for dx in -1..=1 {
                        let n = [s[0] + dx, s[1] + dy, s[2] + dz];
                        if (0..3).all(|i| n[i] >= 0 && n[i] < d[i])
                            && (n == *s || !self.built.contains(&n))
                        {
                            self.todo.insert(n);
                        }
                    }
                }
            }
            for y in 0..s[1] - 1 {
                if self.built.contains(&[s[0], y, s[2]]) {
                    self.todo.insert([s[0], y, s[2]]);
                }
            }
        }
        true
    }

    /// A section and its 26 neighbours all one block, open air or the same
    /// opaque block: nothing in it is drawn.
    fn nothing_to_draw(&self, s: SectionPos) -> bool {
        let Some(b) = self.world.section_uniform(s) else {
            return false;
        };
        if b != dyno_world::AIR && !dyno_world::is_opaque(b) {
            return false;
        }
        let d = self.world.section_dims();
        (-1..=1).all(|dy| {
            (-1..=1).all(|dz| {
                (-1..=1).all(|dx| {
                    let n = [s[0] + dx, s[1] + dy, s[2] + dz];
                    if (0..3).all(|i| n[i] >= 0 && n[i] < d[i]) {
                        self.world.section_uniform(n) == Some(b)
                    } else {
                        // outside the world: air, or stone below it (as `get`)
                        b == if n[1] < 0 {
                            dyno_world::STONE
                        } else {
                            dyno_world::AIR
                        }
                    }
                })
            })
        })
    }

    /// Mesh a batch: edits first, then the most wanted of the rest. Returns
    /// the meshes made.
    fn mesh_some(&mut self) -> Vec<(SectionPos, SectionGeo)> {
        let ready = |w: &World, s: &SectionPos| render::section_ready(w, *s);
        // not ready yet: their neighbours will bring them back when they
        // come; not lit yet: lighting them brings them back
        let world = &self.world;
        let unlit: HashSet<SectionPos> = self.unlit.iter().map(|(s, _)| *s).collect();
        self.urgent
            .retain(|s| ready(world, s) && !unlit.contains(s));
        self.todo.retain(|s| ready(world, s) && !unlit.contains(s));
        // edits first, then the most wanted; sections with nothing to draw
        // (the sky) cost nothing and do not count toward the batch
        let mut urgent: Vec<SectionPos> = self.urgent.iter().copied().collect();
        urgent.sort_by(|a, b| self.priority(*a).total_cmp(&self.priority(*b)));
        let mut rest: Vec<(f32, SectionPos)> = self
            .todo
            .iter()
            .filter(|s| !self.urgent.contains(*s))
            .map(|s| (self.priority(*s), *s))
            .collect();
        rest.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut list = Vec::new();
        let mut real = 0;
        for s in urgent.into_iter().chain(rest.into_iter().map(|(_, s)| s)) {
            if self.nothing_to_draw(s) {
                if list.len() < 4096 {
                    list.push(s);
                }
            } else if real < MESH_BATCH {
                real += 1;
                list.push(s);
            }
            if real >= MESH_BATCH && list.len() >= 4096 {
                break;
            }
        }
        if list.is_empty() {
            return Vec::new();
        }
        for s in &list {
            self.urgent.remove(s);
            self.todo.remove(s);
            self.built.insert(*s);
        }
        // the open sky, and rock all round, draw nothing: no need to mesh
        let (empty, solid): (Vec<SectionPos>, Vec<SectionPos>) =
            list.into_iter().partition(|s| self.nothing_to_draw(*s));
        // each with how its leaves are drawn now, and its ground at the
        // level of detail the camera calls for
        let with: Vec<_> = solid
            .iter()
            .map(|s| (*s, self.leafy.get(s).copied(), self.lods.lod(*s)))
            .collect();
        for (s, _, l) in &with {
            self.built_lod.insert(*s, *l);
        }
        let focus = self.focused.then_some(self.cam);
        let geos = render::mesh_many_packed(&self.world, &self.light, &with, focus);
        for s in &empty {
            self.leafy.remove(s);
        }
        for (s, g) in solid.iter().zip(&geos) {
            match g.leaves_fine {
                Some(fine) => self.leafy.insert(*s, fine),
                None => self.leafy.remove(s),
            };
        }
        empty
            .into_iter()
            .map(|s| (s, SectionGeo::default()))
            .chain(solid.into_iter().zip(geos))
            .collect()
    }

    fn has_work(&self) -> bool {
        !self.unlit.is_empty() || !self.todo.is_empty() || !self.urgent.is_empty()
    }

    /// One round: light some columns (edits were lit as they came), mesh a
    /// batch. Returns whether it lit any, and what it made.
    fn step(&mut self) -> (bool, Option<Made>) {
        let lit = if self.urgent.is_empty() {
            self.light_some()
        } else {
            false
        };
        let meshes = self.mesh_some();
        let light: Vec<(SectionPos, SectionLight)> = self
            .light
            .take_changed()
            .into_iter()
            .map(|s| (s, self.light.section(s)))
            .collect();
        let made = (!light.is_empty() || !meshes.is_empty()).then_some(Made { light, meshes });
        (lit, made)
    }

    fn run(
        mut self,
        jobs: Receiver<Job>,
        made: Sender<Made>,
        pending: Arc<AtomicUsize>,
        busy: Arc<AtomicBool>,
        sent: Arc<AtomicUsize>,
    ) {
        loop {
            if !self.has_work() {
                busy.store(false, Ordering::SeqCst);
                match jobs.recv() {
                    Ok(j) => {
                        busy.store(true, Ordering::SeqCst);
                        pending.fetch_sub(1, Ordering::SeqCst);
                        self.take(j);
                    }
                    Err(_) => return,
                }
            }
            loop {
                match jobs.try_recv() {
                    Ok(j) => {
                        busy.store(true, Ordering::SeqCst);
                        pending.fetch_sub(1, Ordering::SeqCst);
                        self.take(j);
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return,
                }
            }
            let (lit, out) = self.step();
            if let Some(m) = out {
                // counted before it is sent: never idle with a result in the pipe
                sent.fetch_add(1, Ordering::SeqCst);
                if made.send(m).is_err() {
                    return;
                }
            } else if !lit {
                // columns waiting to be whole: wait for more to arrive
                match jobs.recv_timeout(Duration::from_millis(20)) {
                    Ok(j) => {
                        pending.fetch_sub(1, Ordering::SeqCst);
                        self.take(j);
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The worker's meshes and light are those of the whole world built at
    /// once, however the sections arrive.
    #[test]
    fn the_worker_meshes_and_lights_like_the_whole_world() {
        let world = World::generate([64, 48, 64], 7);
        let mut game = World::sparse(world.size);
        let first: Vec<SectionPos> = world.held_list().into_iter().filter(|s| s[0] < 2).collect();
        for s in &first {
            game.put_section(*s, &world.encode_section(*s)).unwrap();
        }
        let mut work = WorldWork::start(game.clone());
        let mut light = Light::blank(&game);
        let rest: Vec<(SectionPos, SectionData)> = world
            .held_list()
            .into_iter()
            .filter(|s| s[0] >= 2)
            .map(|s| (s, World::prepare_section(&world.encode_section(s)).unwrap()))
            .collect();
        for (s, d) in &rest {
            game.put_prepared(*s, d.clone()).unwrap();
        }
        work.focus([32.0, 30.0, 32.0], [0.0, 0.0, -1.0]);
        work.put(rest);
        // an edit after: it is in the meshes and the light
        game.set(20, 40, 20, dyno_world::LANTERN);
        work.block([20, 40, 20], dyno_world::LANTERN);
        let t0 = Instant::now();
        let mut meshes = std::collections::HashMap::new();
        while !work.idle() || meshes.is_empty() {
            assert!(
                t0.elapsed() < Duration::from_secs(20),
                "the worker finishes"
            );
            work.poll(&mut light);
            for (s, g) in work.queue.drain(..) {
                meshes.insert(s, g);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let whole = Light::new(&game);
        for (x, y, z) in [(20, 41, 20), (10, 10, 10), (50, 30, 40), (63, 47, 63)] {
            assert_eq!(
                light.get(x, y, z),
                whole.get(x, y, z),
                "light at {x},{y},{z}"
            );
            assert_eq!(light.sky(x, y, z), whole.sky(x, y, z), "sky at {x},{y},{z}");
        }
        let all = game.held_list();
        assert_eq!(meshes.len(), all.len(), "every section meshed");
        // each at the level of detail the worker chose for it
        let at: Vec<_> = all.iter().map(|s| (*s, meshes[s].lod)).collect();
        let once = render::mesh_many(&game, &whole, &at);
        // the ground's faces merge by light, and a section may be meshed
        // before the light next door has settled: compare the area it
        // covers, which is its shape
        let area = |ground: &[render::Vertex]| -> f64 {
            ground
                .as_chunks::<4>()
                .0
                .iter()
                .map(|q| {
                    let a = glam::Vec3::from(q[1].pos) - glam::Vec3::from(q[0].pos);
                    let b = glam::Vec3::from(q[3].pos) - glam::Vec3::from(q[0].pos);
                    a.cross(b).length() as f64
                })
                .sum()
        };
        for (s, g) in all.iter().zip(once) {
            // the worker's are packed for the card (pack.rs)
            let m = &meshes[s];
            let packed = m.packed.clone().unwrap_or_default();
            assert_eq!(
                (packed.meshed[1] as usize, m.trans.len()),
                (g.opaque.len(), g.trans.len()),
                "section {s:?}"
            );
            let ground = packed.ground(*s);
            assert!(
                (area(&ground) - area(&g.ground)).abs() < 1e-3,
                "section {s:?}: ground"
            );
        }
    }
}
