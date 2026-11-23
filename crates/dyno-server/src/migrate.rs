//! Growing an existing world to a larger map without losing anything: the
//! old world is kept whole in the middle of a freshly generated one, the new
//! land is blended into its edges, and every saved position moves with it.

use crate::game::log;
use dyno_world::layout::Layout;
use dyno_world::{self as w, BlockId, World};
use serde_json::Value;
use std::path::Path;

/// How many columns the new land takes to blend into the old edge.
const BLEND: i32 = 14;
const WATER_LEVEL: i32 = 24;

fn natural(b: BlockId) -> bool {
    matches!(
        b,
        w::AIR
            | w::GRASS
            | w::DIRT
            | w::STONE
            | w::SAND
            | w::SNOW
            | w::WATER
            | w::LEAVES
            | w::LOG
            | 17
            | 33
            | 34
            | 35
            | 36
            | 44
    )
}

/// Back up the save files, then place `old` in the centre of a new world of `size`.
pub fn expand(world_path: &Path, old: World, size: [u32; 3], seed: u32) -> World {
    let (ox, oz) = (
        (size[0] as i32 - old.size[0] as i32) / 2,
        (size[2] as i32 - old.size[2] as i32) / 2,
    );
    // back up everything first
    for name in ["world.dynw", "players.json", "chests.json", "server.json"] {
        let from = world_path.with_file_name(name);
        if from.exists() {
            let to = world_path.with_file_name(format!("{name}.before-{}", size[0]));
            if let Err(e) = std::fs::copy(&from, &to) {
                log(format!("Warning: cannot back up {}: {e}", from.display()));
            }
        }
    }
    log(format!(
        "Growing the world from {:?} to {:?}: the old land moves by ({ox}, {oz}); backups end in .before-{}",
        old.size, size, size[0]
    ));
    let mut new = World::generate(size, seed);
    let (w0, h0, d0) = (
        old.size[0] as i32,
        old.size[1].min(size[1]) as i32,
        old.size[2] as i32,
    );
    // the old land, exactly as it was
    for x in 0..w0 {
        for z in 0..d0 {
            for y in 0..h0 {
                new.set(x + ox, y, z + oz, old.get(x, y, z));
            }
        }
    }
    // blend the new land's height into the old edge
    for x in (ox - BLEND)..(ox + w0 + BLEND) {
        for z in (oz - BLEND)..(oz + d0 + BLEND) {
            let inside = x >= ox && x < ox + w0 && z >= oz && z < oz + d0;
            if inside || !new.in_bounds([x, 1, z]) {
                continue;
            }
            let (cx, cz) = ((x - ox).clamp(0, w0 - 1), (z - oz).clamp(0, d0 - 1));
            let d = ((x - ox - cx).abs()).max((z - oz - cz).abs());
            let t = (d as f32 / BLEND as f32).clamp(0.0, 1.0);
            let t = t * t * (3.0 - 2.0 * t);
            let h_old = old.surface(cx, cz) as f32;
            let h_new = new.surface(x, z) as f32;
            let h = (h_old + (h_new - h_old) * t).round() as i32;
            for y in 1..size[1] as i32 - 1 {
                let here = new.get(x, y, z);
                if !natural(here) {
                    continue;
                }
                let b = if y > h {
                    if y <= WATER_LEVEL && h < WATER_LEVEL {
                        w::WATER
                    } else {
                        w::AIR
                    }
                } else if y == h {
                    if h < WATER_LEVEL + 1 {
                        w::SAND
                    } else {
                        w::GRASS
                    }
                } else if y > h - 4 {
                    w::DIRT
                } else if here == w::AIR || here == w::WATER {
                    w::STONE
                } else {
                    here
                };
                new.set(x, y, z, b);
            }
        }
    }
    // saved positions move with the land
    let shift = |v: &mut Value| {
        if let Some(a) = v.as_array_mut()
            && a.len() == 3
            && let (Some(x), Some(z)) = (a[0].as_f64(), a[2].as_f64())
        {
            let int = a[0].is_i64() || a[0].is_u64();
            if int {
                a[0] = Value::from(x as i64 + ox as i64);
                a[2] = Value::from(z as i64 + oz as i64);
            } else {
                a[0] = Value::from(x + ox as f64);
                a[2] = Value::from(z + oz as f64);
            }
        }
    };
    let edit = |name: &str, f: &dyn Fn(&mut Value)| {
        let path = world_path.with_file_name(name);
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let Ok(mut v) = serde_json::from_slice::<Value>(&bytes) else {
            return;
        };
        f(&mut v);
        if let Ok(out) = serde_json::to_vec_pretty(&v)
            && let Err(e) = std::fs::write(&path, out)
        {
            log(format!("Warning: cannot write {}: {e}", path.display()));
        }
    };
    edit("players.json", &|v| {
        if let Some(m) = v.as_object_mut() {
            for p in m.values_mut() {
                if let Some(pos) = p.get_mut("pos") {
                    shift(pos);
                }
                if let Some(sp) = p.get_mut("spawn") {
                    shift(sp);
                }
            }
        }
    });
    edit("chests.json", &|v| {
        if let Some(m) = v.as_object_mut() {
            let moved: serde_json::Map<String, Value> = std::mem::take(m)
                .into_iter()
                .map(|(k, val)| {
                    let p: Vec<i64> = k.split(',').filter_map(|x| x.parse().ok()).collect();
                    let key = if p.len() == 3 {
                        format!("{},{},{}", p[0] + ox as i64, p[1], p[2] + oz as i64)
                    } else {
                        k
                    };
                    (key, val)
                })
                .collect();
            *m = moved;
        }
    });
    edit("server.json", &|v| {
        if let Some(h) = v.get_mut("spawn_hearth") {
            shift(h);
        }
        if let Some(r) = v.get_mut("regrow").and_then(|r| r.as_array_mut()) {
            for e in r.iter_mut() {
                if let Some(pos) = e.get_mut(0) {
                    shift(pos);
                }
            }
        }
    });
    new
}

/// Blocks a player can be put down on: the ground and paving, never a
/// roof, a tree or a wall top.
fn floor(b: BlockId) -> bool {
    matches!(
        b,
        w::GRASS | w::DIRT | w::SAND | w::STONE | w::SNOW | w::COBBLE | w::FLAGSTONES | 19
    )
}

/// The nearest place to stand to a spot of the new world: open ground under
/// the sky, as close as possible.
pub fn standing(world: &World, x: i32, z: i32) -> Option<[f32; 3]> {
    for r in 0..48i32 {
        for dz in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dz.abs() != r {
                    continue;
                }
                let (px, pz) = (x + dx, z + dz);
                if !world.in_bounds([px, 1, pz]) {
                    continue;
                }
                let y = world.surface(px, pz);
                if y > 0
                    && floor(world.get(px, y, pz))
                    && world.get(px, y + 1, pz) == w::AIR
                    && world.get(px, y + 2, pz) == w::AIR
                {
                    return Some([px as f32 + 0.5, y as f32 + 1.01, pz as f32 + 0.5]);
                }
            }
        }
    }
    None
}

/// Where a position of the old world belongs in the new one, as an offset
/// (x, z) from the new spawn hearth. `p` is an offset from the old hearth;
/// `under` says it was underground. Someone at a place stays at that place,
/// as far from its middle in proportion; out in the country, positions
/// spread with the map.
pub fn map_offset(l: &Layout, old_size: u32, p: [f32; 2], under: bool) -> [f32; 2] {
    // the place whose old reach holds the spot (the tightest one), or for
    // someone underground, the nearest
    let mut best: Option<(f32, &dyno_world::layout::Place)> = None;
    for place in &l.places {
        let Some(was) = &place.was else { continue };
        let d = (p[0] - was.at[0] as f32).hypot(p[1] - was.at[1] as f32);
        let score = if under {
            d
        } else if d <= was.radius {
            was.radius
        } else {
            continue;
        };
        if best.is_none_or(|(s, _)| score < s) {
            best = Some((score, place));
        }
    }
    if let Some((_, place)) = best
        && let Some(was) = &place.was
    {
        let c = l.place_at(place);
        if under {
            return [c[0] as f32, c[1] as f32];
        }
        let reach = place.pad.as_ref().map_or(l.place_radius(place), |pad| {
            pad.radius[0].min(pad.radius[1])
        });
        let k = (reach / was.radius).max(1.0);
        return [
            c[0] as f32 + (p[0] - was.at[0] as f32) * k,
            c[1] as f32 + (p[1] - was.at[1] as f32) * k,
        ];
    }
    let from = l.migrate.as_ref().map_or(old_size, |m| m.from_size).max(1);
    let k = l.size[0] as f32 / from as f32;
    [p[0] * k, p[1] * k]
}

/// What a rebuild did, for the log and for tests.
#[derive(Debug, Default, PartialEq)]
pub struct Rebuilt {
    pub players: usize,
    pub chests_dropped: usize,
    pub regrow_dropped: usize,
}

/// The one-time move to a world built from a layout: back up the save,
/// then put every saved position at its equivalent place in the new world.
/// Characters keep everything they have: only `pos` and `spawn` change.
/// Blocks of the old world are not carried over (the backup keeps them).
pub fn rebuild(
    world_path: &Path,
    old: &World,
    meta: &crate::store::Meta,
    new: &World,
    built: &dyno_world::worldgen::Built,
    l: &Layout,
) -> std::io::Result<Rebuilt> {
    let tag = format!("before-layout-{}", l.version);
    for name in ["world.dynw", "players.json", "chests.json", "server.json"] {
        let from = world_path.with_file_name(name);
        let to = world_path.with_file_name(format!("{name}.{tag}"));
        if from.exists() && !to.exists() {
            std::fs::copy(&from, &to)?;
        }
    }
    let old_hearth = meta.spawn_hearth.unwrap_or([
        old.size[0] as i32 / 2,
        old.surface(old.size[0] as i32 / 2, old.size[2] as i32 / 2) + 1,
        old.size[2] as i32 / 2,
    ]);
    let h = built.hearth;
    let home = [h[0] as f32 + 2.5, h[1] as f32 + 0.01, h[2] as f32 + 0.5];
    let mut done = Rebuilt::default();
    let moved = |v: &Value| -> Option<[f32; 3]> {
        let a = v.as_array()?;
        let (x, y, z) = (
            a.first()?.as_f64()?,
            a.get(1)?.as_f64()?,
            a.get(2)?.as_f64()?,
        );
        let under = (old.surface(x.floor() as i32, z.floor() as i32) as f64) - y > 4.0;
        let off = map_offset(
            l,
            old.size[0],
            [
                (x - old_hearth[0] as f64) as f32,
                (z - old_hearth[2] as f64) as f32,
            ],
            under,
        );
        let (nx, nz) = (h[0] + off[0].round() as i32, h[2] + off[1].round() as i32);
        Some(standing(new, nx, nz).unwrap_or(home))
    };
    let players = world_path.with_file_name("players.json");
    if let Ok(bytes) = std::fs::read(&players) {
        let mut v: Value = serde_json::from_slice(&bytes).map_err(std::io::Error::other)?;
        if let Some(m) = v.as_object_mut() {
            for (name, p) in m.iter_mut() {
                let before = p.get("pos").cloned().unwrap_or(Value::Null);
                let at = moved(&before).unwrap_or(home);
                p["pos"] = serde_json::json!(at);
                if let Some(sp) = p.get("spawn").filter(|s| !s.is_null()).cloned() {
                    p["spawn"] = moved(&sp).map_or(Value::Null, |a| serde_json::json!(a));
                }
                done.players += 1;
                log(format!("  {name}: {before} -> {at:?}"));
            }
        }
        crate::store::write_atomic(
            &players,
            &serde_json::to_vec_pretty(&v).map_err(std::io::Error::other)?,
        )?;
    }
    // chests stood on blocks that are gone; the backup keeps their contents
    let chests = world_path.with_file_name("chests.json");
    if let Ok(bytes) = std::fs::read(&chests)
        && let Ok(Value::Object(m)) = serde_json::from_slice::<Value>(&bytes)
        && !m.is_empty()
    {
        done.chests_dropped = m.len();
        crate::store::write_atomic(&chests, b"{}")?;
    }
    let server = world_path.with_file_name("server.json");
    let mut v: Value = std::fs::read(&server)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    done.regrow_dropped = v
        .get("regrow")
        .and_then(|r| r.as_array())
        .map_or(0, |r| r.len());
    v["regrow"] = serde_json::json!([]);
    v["spawn_hearth"] = serde_json::json!(h);
    v["world_layout"] = serde_json::json!(l.version);
    crate::store::write_atomic(
        &server,
        &serde_json::to_vec_pretty(&v).map_err(std::io::Error::other)?,
    )?;
    log(format!(
        "Rebuilt the world from layout v{}: {:?} became {:?}; {} characters moved to their places; \
         {} chests and {} regrowing nodes belonged to the old land; the old save is kept as *.{tag}",
        l.version, old.size, new.size, done.players, done.chests_dropped, done.regrow_dropped
    ));
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growing_keeps_builds_and_moves_saves() {
        let dir = std::env::temp_dir().join(format!("dyno-migrate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("world.dynw");
        let mut old = World::generate([64, 48, 64], 7);
        let chest = [20, old.surface(20, 30) + 1, 30];
        old.set(chest[0], chest[1], chest[2], w::CHEST);
        old.save(&path).unwrap();
        std::fs::write(
            dir.join("players.json"),
            r#"{"ada":{"pos":[20.5,30.0,30.5],"spawn":null,"health":20.0,"food":20.0}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("chests.json"),
            format!(r#"{{"{},{},{}":[]}}"#, chest[0], chest[1], chest[2]),
        )
        .unwrap();
        let new = expand(&path, old, [96, 48, 96], 7);
        assert_eq!(new.size, [96, 48, 96]);
        assert_eq!(
            new.get(chest[0] + 16, chest[1], chest[2] + 16),
            w::CHEST,
            "builds keep their place, shifted"
        );
        let players: Value =
            serde_json::from_slice(&std::fs::read(dir.join("players.json")).unwrap()).unwrap();
        assert_eq!(players["ada"]["pos"][0].as_f64(), Some(36.5));
        assert_eq!(players["ada"]["pos"][2].as_f64(), Some(46.5));
        let chests: Value =
            serde_json::from_slice(&std::fs::read(dir.join("chests.json")).unwrap()).unwrap();
        assert!(
            chests
                .get(format!("{},{},{}", chest[0] + 16, chest[1], chest[2] + 16))
                .is_some()
        );
        assert!(
            dir.join("world.dynw.before-96").exists(),
            "a backup is kept"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
