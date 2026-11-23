//! The graphics options a player can change: the quality tier of the costly
//! passes, water, ground cover, leaf detail and ambient life. One small panel draws them
//! on the title's Options page and in the pause menu; the app applies a
//! change at once and saves it.

use crate::settings::Settings;
use crate::theme;
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Graphics {
    /// 0 low .. 3 ultra (post.rs)
    pub tier: u8,
    /// 0..2 (water.rs)
    pub water: u8,
    /// 0..2 (cover.rs)
    pub cover: f32,
    /// 0..4 (foliage.rs)
    pub leaves: f32,
    /// 0..2 (life.rs)
    pub life: u8,
}

impl Graphics {
    pub fn of(s: &Settings) -> Graphics {
        Graphics {
            // (not chosen yet: shown as high until the first run picks)
            tier: if s.graphics > 3 { 2 } else { s.graphics },
            water: s.water_quality.min(2),
            cover: s.ground_cover,
            leaves: s.leaf_detail,
            life: s.life.min(2),
        }
    }

    pub fn into(self, s: &mut Settings) {
        s.graphics = self.tier;
        s.water_quality = self.water;
        s.ground_cover = self.cover;
        s.leaf_detail = self.leaves;
        s.life = self.life;
    }

    /// What a quality tier sets everything to: the first choice for a
    /// machine (from its graphics adapter) and the panel's presets.
    pub fn preset(tier: u8) -> Graphics {
        match tier {
            0 => Graphics {
                tier: 0,
                water: 0,
                cover: 0.5,
                leaves: 0.5,
                life: 1,
            },
            1 => Graphics {
                tier: 1,
                water: 1,
                cover: 1.0,
                leaves: 1.0,
                life: 2,
            },
            2 => Graphics {
                tier: 2,
                water: 2,
                cover: 1.0,
                leaves: 1.0,
                life: 2,
            },
            _ => Graphics {
                tier: 3,
                water: 2,
                cover: 1.5,
                leaves: 2.0,
                life: 2,
            },
        }
    }
}

/// The options as the panel shows them. The app sets them from the saved
/// settings and reads them back when the panel says they changed.
static CURRENT: Mutex<Option<Graphics>> = Mutex::new(None);

pub fn set(g: Graphics) {
    *CURRENT.lock().unwrap() = Some(g);
}

pub fn get() -> Option<Graphics> {
    *CURRENT.lock().unwrap()
}

/// One choice in a row: a dark button, gold when it is the one chosen.
fn choice(ui: &mut egui::Ui, chosen: bool, text: &str) -> bool {
    let (ink, fill, edge) = if chosen {
        (theme::INK, theme::GOLD, theme::GOLD_BRIGHT)
    } else {
        (theme::GOLD, theme::WOOD_DARK, theme::GOLD_DARK)
    };
    ui.add(
        egui::Button::new(
            egui::RichText::new(text)
                .font(theme::title_font(12.0))
                .color(ink),
        )
        .fill(fill)
        .stroke(egui::Stroke::new(1.0_f32, edge))
        .min_size(egui::vec2(84.0, 26.0)),
    )
    .clicked()
}

/// A row: its name, then its choices. The index of the one clicked.
fn row(ui: &mut egui::Ui, name: &str, chosen: Option<usize>, choices: &[&str]) -> Option<usize> {
    let mut clicked = None;
    ui.horizontal(|ui| {
        ui.add_sized(
            [104.0, 26.0],
            egui::Label::new(
                egui::RichText::new(name)
                    .font(theme::title_font(13.0))
                    .color(theme::TEXT),
            ),
        );
        for (i, text) in choices.iter().enumerate() {
            if choice(ui, chosen == Some(i), text) {
                clicked = Some(i);
            }
        }
    });
    clicked
}

/// The steps the rows offer for ground cover and leaf detail.
const COVER: [f32; 4] = [0.0, 0.5, 1.0, 1.5];
const LEAVES: [f32; 4] = [0.5, 1.0, 2.0, 4.0];

/// Which step a saved value is nearest.
fn step(steps: &[f32], v: f32) -> usize {
    (0..steps.len())
        .min_by(|a, b| (steps[*a] - v).abs().total_cmp(&(steps[*b] - v).abs()))
        .unwrap_or(0)
}

/// Draw the options. True when the player changed one.
pub fn panel(ui: &mut egui::Ui) -> bool {
    // (before the app has set them: what is saved)
    let was = get().unwrap_or_else(|| Graphics::of(&Settings::peek()));
    let mut g = was;
    ui.label(
        egui::RichText::new("Graphics")
            .font(theme::title_font(14.0))
            .color(theme::GOLD),
    );
    // a tier is a preset: it sets the rows below too
    let tiers = ["Low", "Medium", "High", "Ultra"];
    if let Some(i) = row(ui, "Quality", Some(g.tier as usize), &tiers) {
        g = Graphics::preset(i as u8);
    }
    let water = ["Plain", "Clear", "Mirrored"];
    if let Some(i) = row(ui, "Water", Some(g.water as usize), &water) {
        g.water = i as u8;
    }
    let cover = ["None", "Sparse", "Full", "Lush"];
    if let Some(i) = row(ui, "Ground cover", Some(step(&COVER, g.cover)), &cover) {
        g.cover = COVER[i];
    }
    let leaves = ["Near", "Normal", "Far", "Farthest"];
    if let Some(i) = row(ui, "Leaf detail", Some(step(&LEAVES, g.leaves)), &leaves) {
        g.leaves = LEAVES[i];
    }
    let life = ["None", "A little", "All"];
    if let Some(i) = row(ui, "Wildlife", Some(g.life as usize), &life) {
        g.life = i as u8;
    }
    if g != was {
        set(g);
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_go_into_settings_and_back() {
        let mut s = Settings::default();
        let g = Graphics::preset(0);
        g.into(&mut s);
        assert_eq!(Graphics::of(&s), g);
        assert_eq!(s.water_quality, 0);
        assert_eq!(step(&COVER, 1.1), 2);
        assert_eq!(step(&LEAVES, 4.0), 3);
    }
}
