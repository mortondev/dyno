//! MagicaVoxel `.vox` files: the format figures, held items and detail
//! shapes are stored in, so artists can open and edit them.
//!
//! A file holds a palette and any number of models, each named through the
//! scene graph (`nTRN` → `nSHP`) and placed by its translation. Models keep
//! their place, so parts of one figure line up as they do in the game.
//!
//! Axes: MagicaVoxel is z-up; the game is y-up with +z forward. A game voxel
//! `(x, y, z)` is stored at `(x, -z - 1, y)`.

use std::collections::HashMap;

/// One named model: its voxels in game coordinates, with palette indices
/// (1..=255).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Model {
    pub name: String,
    pub voxels: Vec<([i32; 3], u8)>,
}

/// A whole file: models in order and a 256-entry palette (index 0 unused).
#[derive(Clone, Debug, PartialEq)]
pub struct File {
    pub models: Vec<Model>,
    /// sRGB colours by palette index; `palette[0]` is empty space
    pub palette: [[u8; 4]; 256],
}

impl Default for File {
    fn default() -> Self {
        File {
            models: Vec::new(),
            palette: [[0, 0, 0, 255]; 256],
        }
    }
}

impl File {
    /// The model with this name.
    pub fn model(&self, name: &str) -> Option<&Model> {
        self.models.iter().find(|m| m.name == name)
    }
}

fn to_vox(p: [i32; 3]) -> [i32; 3] {
    [p[0], -p[2] - 1, p[1]]
}

fn from_vox(p: [i32; 3]) -> [i32; 3] {
    [p[0], p[2], -p[1] - 1]
}

// ---------------------------------------------------------------- reading

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let end = self.at.checked_add(n).filter(|e| *e <= self.data.len());
        let end = end.ok_or("vox: file ends early")?;
        let s = &self.data[self.at..end];
        self.at = end;
        Ok(s)
    }

    fn i32(&mut self) -> Result<i32, String> {
        let b = self.take(4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn string(&mut self) -> Result<String, String> {
        let n = self.i32()?.max(0) as usize;
        Ok(String::from_utf8_lossy(self.take(n)?).into_owned())
    }

    fn dict(&mut self) -> Result<HashMap<String, String>, String> {
        let n = self.i32()?.max(0);
        let mut d = HashMap::new();
        for _ in 0..n {
            let k = self.string()?;
            let v = self.string()?;
            d.insert(k, v);
        }
        Ok(d)
    }
}

enum Node {
    Transform {
        name: Option<String>,
        child: i32,
        at: [i32; 3],
    },
    Group,
    Shape {
        model: i32,
    },
}

/// Read a `.vox` file.
pub fn read(data: &[u8]) -> Result<File, String> {
    if data.len() < 8 || &data[0..4] != b"VOX " {
        return Err("vox: not a MagicaVoxel file".into());
    }
    let mut r = Reader { data, at: 8 };
    // MAIN: no content, every other chunk is its child
    let id = r.take(4)?.to_vec();
    if id != b"MAIN" {
        return Err("vox: no MAIN chunk".into());
    }
    let content = r.i32()?.max(0) as usize;
    let _children = r.i32()?;
    r.take(content)?;
    let mut sizes: Vec<[i32; 3]> = Vec::new();
    let mut raw: Vec<Vec<[u8; 4]>> = Vec::new();
    let mut nodes: HashMap<i32, Node> = HashMap::new();
    let mut file = File::default();
    let mut default_palette = true;
    while r.at + 12 <= data.len() {
        let id: [u8; 4] = r.take(4)?.try_into().unwrap_or_default();
        let content = r.i32()?.max(0) as usize;
        let children = r.i32()?.max(0) as usize;
        let body = r.take(content)?;
        let mut c = Reader { data: body, at: 0 };
        match &id {
            b"SIZE" => sizes.push([c.i32()?, c.i32()?, c.i32()?]),
            b"XYZI" => {
                let n = c.i32()?.max(0) as usize;
                let mut v = Vec::with_capacity(n);
                for _ in 0..n {
                    let b = c.take(4)?;
                    v.push([b[0], b[1], b[2], b[3]]);
                }
                raw.push(v);
            }
            b"RGBA" => {
                default_palette = false;
                // the file's colour i is palette index i + 1
                for i in 0..255 {
                    let b = c.take(4)?;
                    file.palette[i + 1] = [b[0], b[1], b[2], b[3]];
                }
            }
            b"nTRN" => {
                let node = c.i32()?;
                let attrs = c.dict()?;
                let child = c.i32()?;
                let _reserved = c.i32()?;
                let _layer = c.i32()?;
                let frames = c.i32()?.max(0);
                let mut at = [0; 3];
                for f in 0..frames {
                    let d = c.dict()?;
                    if f == 0
                        && let Some(t) = d.get("_t")
                    {
                        let v: Vec<i32> = t
                            .split_whitespace()
                            .filter_map(|s| s.parse().ok())
                            .collect();
                        if v.len() == 3 {
                            at = [v[0], v[1], v[2]];
                        }
                    }
                }
                nodes.insert(
                    node,
                    Node::Transform {
                        name: attrs.get("_name").cloned(),
                        child,
                        at,
                    },
                );
            }
            b"nGRP" => {
                let node = c.i32()?;
                nodes.insert(node, Node::Group);
            }
            b"nSHP" => {
                let node = c.i32()?;
                let _attrs = c.dict()?;
                let n = c.i32()?;
                let model = if n > 0 { c.i32()? } else { -1 };
                nodes.insert(node, Node::Shape { model });
            }
            _ => {}
        }
        // children of other chunks are not used here
        r.take(children.min(data.len() - r.at))?;
    }
    if default_palette {
        return Err("vox: no palette (RGBA chunk)".into());
    }
    // each shape's name and place from the transform above it
    let mut placed: HashMap<i32, (String, [i32; 3])> = HashMap::new();
    for node in nodes.values() {
        if let Node::Transform { name, child, at } = node
            && let Some(Node::Shape { model }) = nodes.get(child)
        {
            let name = name.clone().unwrap_or_else(|| model.to_string());
            placed.insert(*model, (name, *at));
        }
    }
    for (i, (size, vox)) in sizes.iter().zip(&raw).enumerate() {
        let (name, at) = placed
            .get(&(i as i32))
            .cloned()
            .unwrap_or_else(|| (i.to_string(), [size[0] / 2, size[1] / 2, size[2] / 2]));
        // MagicaVoxel centres a model on its translation
        let lo = [
            at[0] - size[0] / 2,
            at[1] - size[1] / 2,
            at[2] - size[2] / 2,
        ];
        let voxels = vox
            .iter()
            .map(|v| {
                let p = [
                    lo[0] + v[0] as i32,
                    lo[1] + v[1] as i32,
                    lo[2] + v[2] as i32,
                ];
                (from_vox(p), v[3])
            })
            .collect();
        file.models.push(Model { name, voxels });
    }
    Ok(file)
}

// ---------------------------------------------------------------- writing

struct Out(Vec<u8>);

impl Out {
    fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn string(&mut self, s: &str) {
        self.i32(s.len() as i32);
        self.0.extend_from_slice(s.as_bytes());
    }
    fn dict(&mut self, d: &[(&str, String)]) {
        self.i32(d.len() as i32);
        for (k, v) in d {
            self.string(k);
            self.string(v);
        }
    }
    fn chunk(&mut self, id: &[u8; 4], body: &[u8]) {
        self.0.extend_from_slice(id);
        self.i32(body.len() as i32);
        self.i32(0);
        self.0.extend_from_slice(body);
    }
}

/// Write a `.vox` file. Models larger than 256 voxels on an axis are an
/// error (MagicaVoxel's limit).
pub fn write(file: &File) -> Result<Vec<u8>, String> {
    let mut body = Out(Vec::new());
    let mut places = Vec::new();
    for m in &file.models {
        let ps: Vec<[i32; 3]> = m.voxels.iter().map(|(p, _)| to_vox(*p)).collect();
        let (mut lo, mut hi) = ([0; 3], [0; 3]);
        if let Some(first) = ps.first() {
            lo = *first;
            hi = *first;
        }
        for p in &ps {
            for a in 0..3 {
                lo[a] = lo[a].min(p[a]);
                hi[a] = hi[a].max(p[a]);
            }
        }
        let size = [hi[0] - lo[0] + 1, hi[1] - lo[1] + 1, hi[2] - lo[2] + 1];
        if size.iter().any(|s| *s > 256) {
            return Err(format!("vox: model {} is {size:?}, over 256", m.name));
        }
        let mut s = Out(Vec::new());
        for v in size {
            s.i32(v);
        }
        body.chunk(b"SIZE", &s.0);
        let mut x = Out(Vec::new());
        x.i32(ps.len() as i32);
        for (p, (_, c)) in ps.iter().zip(&m.voxels) {
            x.0.extend_from_slice(&[
                (p[0] - lo[0]) as u8,
                (p[1] - lo[1]) as u8,
                (p[2] - lo[2]) as u8,
                *c,
            ]);
        }
        body.chunk(b"XYZI", &x.0);
        places.push([
            lo[0] + size[0] / 2,
            lo[1] + size[1] / 2,
            lo[2] + size[2] / 2,
        ]);
    }
    // scene graph: root transform, a group, then a transform and shape per
    // model
    let mut n = Out(Vec::new());
    n.i32(0);
    n.dict(&[]);
    n.i32(1);
    n.i32(-1);
    n.i32(-1);
    n.i32(1);
    n.dict(&[]);
    body.chunk(b"nTRN", &n.0);
    let mut g = Out(Vec::new());
    g.i32(1);
    g.dict(&[]);
    g.i32(file.models.len() as i32);
    for i in 0..file.models.len() {
        g.i32(2 + 2 * i as i32);
    }
    body.chunk(b"nGRP", &g.0);
    for (i, (m, at)) in file.models.iter().zip(&places).enumerate() {
        let id = 2 + 2 * i as i32;
        let mut t = Out(Vec::new());
        t.i32(id);
        t.dict(&[("_name", m.name.clone())]);
        t.i32(id + 1);
        t.i32(-1);
        t.i32(0);
        t.i32(1);
        t.dict(&[("_t", format!("{} {} {}", at[0], at[1], at[2]))]);
        body.chunk(b"nTRN", &t.0);
        let mut s = Out(Vec::new());
        s.i32(id + 1);
        s.dict(&[]);
        s.i32(1);
        s.i32(i as i32);
        s.dict(&[]);
        body.chunk(b"nSHP", &s.0);
    }
    let mut pal = Out(Vec::new());
    for c in &file.palette[1..] {
        pal.0.extend_from_slice(c);
    }
    pal.0.extend_from_slice(&[0, 0, 0, 0]);
    body.chunk(b"RGBA", &pal.0);
    let mut out = Out(b"VOX ".to_vec());
    out.i32(200);
    out.0.extend_from_slice(b"MAIN");
    out.i32(0);
    out.i32(body.0.len() as i32);
    out.0.extend_from_slice(&body.0);
    Ok(out.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let mut f = File::default();
        f.palette[1] = [255, 0, 0, 255];
        f.palette[7] = [1, 2, 3, 255];
        f.models.push(Model {
            name: "head".into(),
            voxels: vec![([0, 48, 0], 1), ([-3, 50, 7], 7), ([2, 63, -6], 1)],
        });
        f.models.push(Model {
            name: "arm.r".into(),
            voxels: vec![([11, 33, -3], 7)],
        });
        let bytes = write(&f).unwrap();
        let back = read(&bytes).unwrap();
        assert_eq!(back.palette[1], [255, 0, 0, 255]);
        assert_eq!(back.palette[7], [1, 2, 3, 255]);
        assert_eq!(back.models.len(), 2);
        for (a, b) in f.models.iter().zip(&back.models) {
            assert_eq!(a.name, b.name);
            let mut x = a.voxels.clone();
            let mut y = b.voxels.clone();
            x.sort();
            y.sort();
            assert_eq!(x, y);
        }
    }
}
