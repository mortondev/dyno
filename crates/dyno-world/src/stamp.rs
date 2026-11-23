//! Stamps: voxel structures as data files in the pack (`stamps/*.dstamp`).
//! Buildings, props, ruins and dug-out places are stamps; the layout says
//! where each one goes.
//!
//! A stamp file is three parts:
//!
//! ```text
//! DSTP1\n
//! {"size":[x,y,z],"anchor":[x,y,z],"palette":["skip","air","planks",…],"points":{…},…}\n
//! run-length cells: (palette index, varint count) pairs, y then z then x
//! ```
//!
//! Palette entry `skip` leaves the world as it is; `air` carves (cellars,
//! mines, doorways in a hillside). Every other entry is a block key. The
//! anchor is the cell that lands on the placement point: its y is the first
//! layer above the ground. `points` are named cells (doors, floors, where
//! someone stands) that tests and the layout refer to.

use crate::{AIR, BlockId, World, block_by_key, is_solid};
use serde::Deserialize;
use std::collections::BTreeMap;

const MAGIC: &[u8] = b"DSTP1\n";
/// Palette index of cells that leave the world alone.
const SKIP: u8 = 0;

#[derive(Deserialize)]
struct Header {
    size: [i32; 3],
    anchor: [i32; 3],
    palette: Vec<String>,
    #[serde(default)]
    points: BTreeMap<String, [i32; 3]>,
    #[serde(default)]
    foundation: Option<String>,
    #[serde(default)]
    storeys: Vec<i32>,
    #[serde(default)]
    doors: Vec<Door>,
    #[serde(default)]
    kind: String,
}

/// A doorway of a stamp: the lowest cell of the opening and its clear height.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub struct Door {
    pub at: [i32; 3],
    pub height: i32,
}

/// What a stamp cell does to the world.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cell {
    Skip,
    Set(BlockId),
}

#[derive(Clone, Debug)]
pub struct Stamp {
    pub size: [i32; 3],
    pub anchor: [i32; 3],
    /// per cell (y, then z, then x): 0 skips, n sets `palette[n]`
    cells: Vec<u8>,
    palette: Vec<Cell>,
    /// named cells, in stamp coordinates
    pub points: BTreeMap<String, [i32; 3]>,
    /// filled down to the ground under the lowest solid cells of the
    /// stamp's ground layer, where the land falls away
    pub foundation: Option<BlockId>,
    /// floor levels (the walking surface's y in stamp cells), lowest first
    pub storeys: Vec<i32>,
    pub doors: Vec<Door>,
    /// what it is ("building", "prop", "ruin", "dig"…): for tests and tools
    pub kind: String,
}

/// A quarter turn count (0 to 3) applied to a stamp offset (x, z), turning
/// clockwise seen from above (east to south).
pub fn turn(p: [i32; 2], rot: u8) -> [i32; 2] {
    match rot % 4 {
        0 => p,
        1 => [-p[1], p[0]],
        2 => [-p[0], -p[1]],
        _ => [p[1], -p[0]],
    }
}

impl Stamp {
    /// Parse a stamp file. Errors say what is wrong.
    pub fn parse(data: &[u8]) -> Result<Stamp, String> {
        let rest = data
            .strip_prefix(MAGIC)
            .ok_or("not a stamp (no DSTP1 header)")?;
        let end = rest
            .iter()
            .position(|&b| b == b'\n')
            .ok_or("the stamp header has no end")?;
        let h: Header =
            serde_json::from_slice(&rest[..end]).map_err(|e| format!("stamp header: {e}"))?;
        if h.size.iter().any(|&s| !(1..=512).contains(&s)) {
            return Err(format!("stamp size {:?} is not supported", h.size));
        }
        if h.palette.len() > 255 || h.palette.first().map(String::as_str) != Some("skip") {
            return Err("the palette must start with \"skip\" and have at most 255 entries".into());
        }
        let mut palette = Vec::with_capacity(h.palette.len());
        for (i, key) in h.palette.iter().enumerate() {
            palette.push(match (i, key.as_str()) {
                (0, _) => Cell::Skip,
                (_, "air") => Cell::Set(AIR),
                (_, k) => Cell::Set(block_by_key(k).ok_or(format!("unknown block \"{k}\""))?),
            });
        }
        let total = (h.size[0] * h.size[1] * h.size[2]) as usize;
        let mut cells = Vec::with_capacity(total);
        let body = &rest[end + 1..];
        let mut p = 0;
        while p < body.len() {
            let v = body[p];
            p += 1;
            if v as usize >= palette.len() {
                return Err(format!("cell value {v} is outside the palette"));
            }
            let (mut n, mut shift) = (0usize, 0);
            loop {
                let byte = *body.get(p).ok_or("truncated stamp cells")?;
                p += 1;
                n |= ((byte & 0x7f) as usize) << shift;
                if byte & 0x80 == 0 {
                    break;
                }
                shift += 7;
                if shift > 28 {
                    return Err("bad run length".into());
                }
            }
            if cells.len() + n > total {
                return Err("more cells than the stamp's size".into());
            }
            cells.resize(cells.len() + n, v);
        }
        if cells.len() != total {
            return Err(format!("{} cells for a stamp of {total}", cells.len()));
        }
        let inside = |p: &[i32; 3]| (0..3).all(|i| p[i] >= 0 && p[i] < h.size[i]);
        if let Some((k, _)) = h.points.iter().find(|(_, p)| !inside(p)) {
            return Err(format!("point \"{k}\" is outside the stamp"));
        }
        if h.doors.iter().any(|d| !inside(&d.at)) {
            return Err("a door is outside the stamp".into());
        }
        let foundation = match &h.foundation {
            Some(k) => Some(block_by_key(k).ok_or(format!("unknown block \"{k}\""))?),
            None => None,
        };
        Ok(Stamp {
            size: h.size,
            anchor: h.anchor,
            cells,
            palette,
            points: h.points,
            foundation,
            storeys: h.storeys,
            doors: h.doors,
            kind: h.kind,
        })
    }

    /// The cell at stamp coordinates.
    pub fn cell(&self, x: i32, y: i32, z: i32) -> Cell {
        let [sx, sy, sz] = self.size;
        if x < 0 || y < 0 || z < 0 || x >= sx || y >= sy || z >= sz {
            return Cell::Skip;
        }
        match self.cells[((y * sz + z) * sx + x) as usize] {
            SKIP => Cell::Skip,
            v => self.palette[v as usize],
        }
    }

    /// Where a stamp cell lands when the anchor is put at `at`, turned by
    /// `rot` quarter turns.
    pub fn to_world(&self, at: [i32; 3], rot: u8, cell: [i32; 3]) -> [i32; 3] {
        let d = turn([cell[0] - self.anchor[0], cell[2] - self.anchor[2]], rot);
        [at[0] + d[0], at[1] + cell[1] - self.anchor[1], at[2] + d[1]]
    }

    /// World position of a named point.
    pub fn point(&self, name: &str, at: [i32; 3], rot: u8) -> Option<[i32; 3]> {
        self.points.get(name).map(|p| self.to_world(at, rot, *p))
    }

    /// The columns the stamp covers on the ground, as offsets (x, z) from
    /// the anchor after turning: the corners of the box round them.
    pub fn footprint(&self, rot: u8) -> ([i32; 2], [i32; 2]) {
        let a = turn([-self.anchor[0], -self.anchor[2]], rot);
        let b = turn(
            [
                self.size[0] - 1 - self.anchor[0],
                self.size[2] - 1 - self.anchor[2],
            ],
            rot,
        );
        (
            [a[0].min(b[0]), a[1].min(b[1])],
            [a[0].max(b[0]), a[1].max(b[1])],
        )
    }

    /// Does the stamp hold anything at or above its ground layer in this
    /// column (stamp coordinates)?
    pub fn covers(&self, x: i32, z: i32) -> bool {
        (0..self.size[1]).any(|y| self.cell(x, y, z) != Cell::Skip)
    }

    /// Write the stamp into the world with its anchor at `at`. Cells that
    /// fall outside the world are dropped.
    pub fn place(&self, world: &mut World, at: [i32; 3], rot: u8) {
        let [sx, sy, sz] = self.size;
        for y in 0..sy {
            for z in 0..sz {
                for x in 0..sx {
                    if let Cell::Set(b) = self.cell(x, y, z) {
                        let p = self.to_world(at, rot, [x, y, z]);
                        world.set(p[0], p[1], p[2], b);
                    }
                }
            }
        }
        let Some(found) = self.foundation else { return };
        // under the walls: carry the lowest course down to the ground
        let y0 = self.anchor[1] - 1;
        for z in 0..sz {
            for x in 0..sx {
                let Cell::Set(b) = self.cell(x, y0.max(0), z) else {
                    continue;
                };
                if !is_solid(b) {
                    continue;
                }
                let p = self.to_world(at, rot, [x, y0.max(0), z]);
                for d in 1..=24 {
                    if is_solid(world.get(p[0], p[1] - d, p[2])) {
                        break;
                    }
                    world.set(p[0], p[1] - d, p[2], found);
                }
            }
        }
    }

    /// Blocks used, with counts (for reports and tests).
    pub fn census(&self) -> BTreeMap<BlockId, usize> {
        let mut out = BTreeMap::new();
        for &v in &self.cells {
            if let Cell::Set(b) = self.palette[v as usize] {
                *out.entry(b).or_default() += 1;
            }
        }
        out
    }
}

/// The stamps of a pack, by name (`stamps/inn.dstamp` is "inn").
#[derive(Clone, Debug, Default)]
pub struct Stamps(pub BTreeMap<String, Stamp>);

impl Stamps {
    /// Parse every `stamps/*.dstamp` among a pack's files.
    pub fn from_files<'a>(
        files: impl Iterator<Item = (&'a str, &'a [u8])>,
    ) -> Result<Stamps, String> {
        let mut out = BTreeMap::new();
        for (path, bytes) in files {
            let Some(name) = path
                .strip_prefix("stamps/")
                .and_then(|p| p.strip_suffix(".dstamp"))
            else {
                continue;
            };
            let s = Stamp::parse(bytes).map_err(|e| format!("{path}: {e}"))?;
            out.insert(name.to_string(), s);
        }
        Ok(Stamps(out))
    }

    pub fn get(&self, name: &str) -> Option<&Stamp> {
        self.0.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(header: &str, cells: &[(u8, u8)]) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        out.extend_from_slice(header.as_bytes());
        out.push(b'\n');
        for (v, n) in cells {
            out.extend_from_slice(&[*v, *n]);
        }
        out
    }

    #[test]
    fn a_stamp_places_turns_and_carves() {
        // 3 × 2 × 1: planks, skip, cobble on the ground layer; air above the planks
        let s = Stamp::parse(&file(
            r#"{"size":[3,2,1],"anchor":[0,0,0],"palette":["skip","air","planks","cobble"],"points":{"end":[2,0,0]}}"#,
            &[(2, 1), (0, 1), (3, 1), (1, 1), (0, 2)],
        ))
        .unwrap();
        let mut w = World::empty([16, 8, 16]);
        w.set(8, 3, 8, crate::STONE);
        s.place(&mut w, [8, 2, 8], 0);
        assert_eq!(w.get(8, 2, 8), crate::PLANKS);
        assert_eq!(w.get(10, 2, 8), crate::COBBLE);
        assert_eq!(w.get(8, 3, 8), AIR, "air carves");
        assert_eq!(s.point("end", [8, 2, 8], 0), Some([10, 2, 8]));
        // a quarter turn: east becomes south
        assert_eq!(s.point("end", [8, 2, 8], 1), Some([8, 2, 10]));
        assert_eq!(s.footprint(1), ([0, 0], [0, 2]));
        assert_eq!(s.footprint(2), ([-2, 0], [0, 0]));
    }

    #[test]
    fn bad_stamps_are_refused() {
        assert!(Stamp::parse(b"nope").is_err());
        let h = r#"{"size":[1,1,1],"anchor":[0,0,0],"palette":["skip","no_such_block"]}"#;
        assert!(Stamp::parse(&file(h, &[(0, 1)])).is_err());
        let h = r#"{"size":[2,1,1],"anchor":[0,0,0],"palette":["skip","planks"]}"#;
        assert!(Stamp::parse(&file(h, &[(1, 1)])).is_err(), "too few cells");
        assert!(
            Stamp::parse(&file(h, &[(2, 2)])).is_err(),
            "outside the palette"
        );
    }
}
