//! Moving what was saved in a world to the world built from the pack's
//! layout: every saved position goes to its place in the new one.

use crate::game::log;
use dyno_world::layout::Layout;
use dyno_world::{self as w, BlockId, World};
use serde_json::Value;
use std::path::Path;

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
    // what is moved is always the save as it was before the rebuild: if a
    // rebuild was cut short (positions written, the new world not yet), the
    // next start moves the same old positions again, not the moved ones
    let before = |name: &str| {
        let kept = world_path.with_file_name(format!("{name}.{tag}"));
        std::fs::read(if kept.exists() {
            kept
        } else {
            world_path.with_file_name(name)
        })
    };
    let meta_before: Option<crate::store::Meta> = before("server.json")
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok());
    let meta = meta_before.as_ref().unwrap_or(meta);
    let old_hearth = meta.spawn_hearth.unwrap_or([
        old.size[0] as i32 / 2,
        old.surface(old.size[0] as i32 / 2, old.size[2] as i32 / 2) + 1,
        old.size[2] as i32 / 2,
    ]);
    let h = built.hearth;
    let home = [h[0] as f32 + 2.5, h[1] as f32 + 0.01, h[2] as f32 + 0.5];
    let mut done = Rebuilt::default();
    // a world already built from a layout keeps its geography: everyone
    // stays where they stand (by offset from the hearth, in proportion if
    // the map's size changed). Only a world from before layouts is mapped
    // place by place.
    let from_layout = meta.world_layout.is_some();
    let grow = new.size[0] as f32 / old.size[0].max(1) as f32;
    let moved = |v: &Value| -> Option<[f32; 3]> {
        let a = v.as_array()?;
        let (x, y, z) = (
            a.first()?.as_f64()?,
            a.get(1)?.as_f64()?,
            a.get(2)?.as_f64()?,
        );
        let under = (old.surface(x.floor() as i32, z.floor() as i32) as f64) - y > 4.0;
        let p = [
            (x - old_hearth[0] as f64) as f32,
            (z - old_hearth[2] as f64) as f32,
        ];
        if from_layout {
            let (nx, nz) = (h[0] as f32 + p[0] * grow, h[2] as f32 + p[1] * grow);
            let ny = h[1] as f32 + (y as f32 - old_hearth[1] as f32);
            let (cx, cy, cz) = (nx.floor() as i32, ny.floor() as i32, nz.floor() as i32);
            // the very spot, where it is still somewhere to stand (indoors,
            // underground); else open ground beside it
            if w::is_solid(new.get(cx, cy - 1, cz))
                && !w::is_solid(new.get(cx, cy, cz))
                && !w::is_solid(new.get(cx, cy + 1, cz))
            {
                return Some([nx, ny, nz]);
            }
            return Some(standing(new, cx, cz).unwrap_or(home));
        }
        let off = map_offset(l, old.size[0], p, under);
        let (nx, nz) = (h[0] + off[0].round() as i32, h[2] + off[1].round() as i32);
        Some(standing(new, nx, nz).unwrap_or(home))
    };
    let players = world_path.with_file_name("players.json");
    if let Ok(bytes) = before("players.json") {
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
    if let Ok(bytes) = before("chests.json")
        && let Ok(Value::Object(m)) = serde_json::from_slice::<Value>(&bytes)
        && !m.is_empty()
    {
        done.chests_dropped = m.len();
        crate::store::write_atomic(&chests, b"{}")?;
    }
    let server = world_path.with_file_name("server.json");
    let mut v: Value = before("server.json")
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
