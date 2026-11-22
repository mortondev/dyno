//! Mining: how long a block takes to break and what it drops.

use crate::inventory::Stack;
use crate::rng::Rng;
use dyno_world::BlockId;
use dyno_world::content::{Content, Tool};

/// Seconds to break a block with the held tool, or None if it cannot be broken.
pub fn break_time(c: &Content, b: BlockId, tool: Option<&Tool>) -> Option<f32> {
    let r = c.rule(b);
    if r.hardness < 0.0 {
        return None;
    }
    // the right kind of tool, and a high enough tier when the block needs one
    let right_tool =
        matches!((r.tool, tool), (Some(k), Some(t)) if k == t.kind && t.tier >= r.tier);
    let t = if right_tool {
        r.hardness * 1.5 / tool.map(|t| t.speed).unwrap_or(1.0)
    } else if r.tier > 0 {
        r.hardness * 5.0
    } else {
        r.hardness * 1.5
    };
    Some(t.max(0.0))
}

/// Does mining this block with this tool give its drops?
pub fn harvests(c: &Content, b: BlockId, tool: Option<&Tool>) -> bool {
    let r = c.rule(b);
    if r.tier == 0 {
        return true;
    }
    matches!((r.tool, tool), (Some(k), Some(t)) if k == t.kind && t.tier >= r.tier)
}

/// The items a broken block drops.
pub fn drops(c: &Content, b: BlockId, tool: Option<&Tool>, rng: &mut Rng) -> Vec<Stack> {
    if !harvests(c, b, tool) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for d in &c.rule(b).drops {
        if rng.f32() < d.chance {
            let n = rng.range(d.min as u32, d.max as u32) as u8;
            if n > 0 {
                out.push(Stack::new(d.item, n));
            }
        }
    }
    out
}

/// Why a block gives nothing with this tool, for the HUD ("Needs a stone pickaxe").
pub fn missing_tool(c: &Content, b: BlockId, tool: Option<&Tool>) -> Option<String> {
    if harvests(c, b, tool) {
        return None;
    }
    let r = c.rule(b);
    let tier = ["", "wooden", "stone", "iron", "ember"]
        .get(r.tier as usize)
        .copied()
        .unwrap_or("better");
    let kind = match r.tool {
        Some(dyno_world::content::ToolKind::Pickaxe) => "pickaxe",
        Some(dyno_world::content::ToolKind::Axe) => "axe",
        Some(dyno_world::content::ToolKind::Shovel) => "shovel",
        _ => "tool",
    };
    Some(format!("Needs a {tier} {kind}"))
}
