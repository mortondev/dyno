//! The graphics options a player can change: the quality tier of the costly
//! passes, water, ground cover and leaf detail. One small panel draws them
//! on the title's Options page and in the pause menu; the app applies a
//! change at once and saves it.

use crate::post::Quality;
use crate::settings::Settings;
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
}

impl Graphics {
    pub fn of(s: &Settings) -> Graphics {
        Graphics {
            // (not chosen yet: shown as high until the first run picks)
            tier: if s.graphics > 3 { 2 } else { s.graphics },
            water: s.water_quality.min(2),
            cover: s.ground_cover,
            leaves: s.leaf_detail,
        }
    }

    pub fn into(self, s: &mut Settings) {
        s.graphics = self.tier;
        s.water_quality = self.water;
        s.ground_cover = self.cover;
        s.leaf_detail = self.leaves;
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
            },
            1 => Graphics {
                tier: 1,
                water: 1,
                cover: 1.0,
                leaves: 1.0,
            },
            2 => Graphics {
                tier: 2,
                water: 2,
                cover: 1.0,
                leaves: 1.0,
            },
            _ => Graphics {
                tier: 3,
                water: 2,
                cover: 1.5,
                leaves: 2.0,
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

/// Draw the options. True when the player changed one.
pub fn panel(ui: &mut egui::Ui) -> bool {
    // (before the app has set them: what is saved)
    let was = get().unwrap_or_else(|| Graphics::of(&Settings::peek()));
    let mut g = was;
    ui.horizontal(|ui| {
        ui.label("Graphics");
        for (i, name) in Quality::NAMES.iter().enumerate() {
            let mut text = name.to_string();
            text[..1].make_ascii_uppercase();
            // a tier is a preset: it sets the rows below too
            if ui.selectable_label(g.tier == i as u8, text).clicked() {
                g = Graphics::preset(i as u8);
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Water");
        for (i, name) in ["Plain", "Clear", "Mirrored"].iter().enumerate() {
            if ui.selectable_label(g.water == i as u8, *name).clicked() {
                g.water = i as u8;
            }
        }
    });
    ui.add(egui::Slider::new(&mut g.cover, 0.0..=2.0).text("Ground cover"));
    ui.add(egui::Slider::new(&mut g.leaves, 0.0..=4.0).text("Leaf detail"));
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
    }
}
