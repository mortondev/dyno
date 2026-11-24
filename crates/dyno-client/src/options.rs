//! The graphics options a player can change: the quality tier of the costly
//! passes, water, ground cover, leaf detail and ambient life. One small panel draws them
//! on the title's Options page and in the pause menu; the app applies a
//! change at once and saves it. The sound levels are here too: a slider
//! for the master level and one for each bus of the mixer (audio.rs).

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

/// The sound levels a player can change, each 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sound {
    pub master: f32,
    pub music: f32,
    pub effects: f32,
    pub ambience: f32,
    pub ui: f32,
}

impl Sound {
    pub fn of(s: &Settings) -> Sound {
        let level = |v: f32| {
            if v.is_finite() {
                v.clamp(0.0, 1.0)
            } else {
                1.0
            }
        };
        Sound {
            master: level(s.volume),
            music: level(s.volume_music),
            effects: level(s.volume_effects),
            ambience: level(s.volume_ambience),
            ui: level(s.volume_ui),
        }
    }

    pub fn into(self, s: &mut Settings) {
        s.volume = self.master;
        s.volume_music = self.music;
        s.volume_effects = self.effects;
        s.volume_ambience = self.ambience;
        s.volume_ui = self.ui;
    }

    /// What the mixer is told. A slider is heard as loudness, so its
    /// middle is far below half the signal: the level is its square.
    pub fn volumes(self) -> crate::audio::Volumes {
        crate::audio::Volumes {
            master: self.master * self.master,
            music: self.music * self.music,
            effects: self.effects * self.effects,
            ambience: self.ambience * self.ambience,
            ui: self.ui * self.ui,
        }
    }

    fn levels(&mut self) -> [&mut f32; 5] {
        [
            &mut self.master,
            &mut self.music,
            &mut self.effects,
            &mut self.ambience,
            &mut self.ui,
        ]
    }
}

/// The sound levels as the panel shows them: the mixer follows them every
/// frame (app/sound.rs), so a slider is heard while it is dragged.
static SOUND: Mutex<Option<Sound>> = Mutex::new(None);

pub fn set_sound(s: Sound) {
    *SOUND.lock().unwrap() = Some(s);
}

pub fn sound() -> Option<Sound> {
    *SOUND.lock().unwrap()
}

/// A slider row: its name, a gold bar to drag, its value as a percentage.
/// (moved, released): the value changed; the pointer came off it.
fn slider(ui: &mut egui::Ui, name: &str, v: &mut f32) -> (bool, bool) {
    let mut out = (false, false);
    ui.horizontal(|ui| {
        ui.add_sized(
            [104.0, 26.0],
            egui::Label::new(
                egui::RichText::new(name)
                    .font(theme::title_font(13.0))
                    .color(theme::TEXT),
            ),
        );
        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(260.0, 26.0), egui::Sense::click_and_drag());
        let track = egui::Rect::from_center_size(rect.center(), egui::vec2(rect.width(), 10.0));
        if let Some(at) = resp.interact_pointer_pos()
            && (resp.dragged() || resp.clicked())
        {
            let new = ((at.x - track.left() - 6.0) / (track.width() - 12.0)).clamp(0.0, 1.0);
            // (in steps of one in a hundred)
            let new = (new * 100.0).round() / 100.0;
            out.0 = new != *v;
            *v = new;
        }
        out.1 = resp.drag_stopped() || resp.clicked();
        let p = ui.painter();
        p.rect_filled(track, 3.0, theme::WOOD_DARK);
        let knob = track.left() + 6.0 + (track.width() - 12.0) * *v;
        let filled = egui::Rect::from_min_max(track.min, egui::pos2(knob, track.max.y));
        p.rect_filled(filled, 3.0, theme::GOLD_DARK);
        p.rect_stroke(
            track,
            3.0,
            egui::Stroke::new(1.0_f32, theme::GOLD_DARK),
            egui::StrokeKind::Inside,
        );
        let bright = resp.hovered() || resp.dragged();
        p.rect_filled(
            egui::Rect::from_center_size(
                egui::pos2(knob, track.center().y),
                egui::vec2(10.0, 20.0),
            ),
            2.0,
            if bright {
                theme::GOLD_BRIGHT
            } else {
                theme::GOLD
            },
        );
        ui.label(
            egui::RichText::new(format!("{:.0}%", *v * 100.0))
                .font(theme::title_font(12.0))
                .color(theme::TEXT),
        );
    });
    out
}

/// What a player can change about the interface, and what it remembers
/// for them: its size, the tutorial tips, each character's action bar.
#[derive(Clone, Debug, PartialEq)]
pub struct Interface {
    /// beside the natural size for the window (see [`zoom_for`])
    pub scale: f32,
    pub tips: bool,
    /// the camera shakes on a crit or a heavy blow
    pub shake: bool,
    pub tips_seen: Vec<String>,
    pub bars: std::collections::BTreeMap<String, Vec<String>>,
}

impl Default for Interface {
    fn default() -> Self {
        Interface::of(&Settings::default())
    }
}

impl Interface {
    pub fn of(s: &Settings) -> Interface {
        Interface {
            scale: s.ui_scale.clamp(0.7, 1.5),
            tips: s.tips,
            shake: s.camera_shake,
            tips_seen: s.tips_seen.clone(),
            bars: s.bars.clone(),
        }
    }

    pub fn into(self, s: &mut Settings) {
        s.ui_scale = self.scale;
        s.tips = self.tips;
        s.camera_shake = self.shake;
        s.tips_seen = self.tips_seen;
        s.bars = self.bars;
    }
}

static INTERFACE: Mutex<Option<Interface>> = Mutex::new(None);

pub fn set_interface(i: Interface) {
    *INTERFACE.lock().unwrap() = Some(i);
}

/// The interface options (before the app has set them: the defaults).
pub fn interface() -> Interface {
    INTERFACE.lock().unwrap().clone().unwrap_or_default()
}

/// Change the interface options; true when something changed (the caller
/// then asks the app to save: `Action::Graphics`).
pub fn change_interface(f: impl FnOnce(&mut Interface)) -> bool {
    let mut lock = INTERFACE.lock().unwrap();
    let mut i = lock.clone().unwrap_or_default();
    let was = i.clone();
    f(&mut i);
    let changed = i != was;
    *lock = Some(i);
    changed
}

/// The interface is laid out for a window 1000 points tall, so it takes
/// the same share of a 1080p and a 1440p screen; the player's scale goes
/// on top. This is egui's zoom factor for a window `height_px` tall.
pub fn zoom_for(height_px: f32, native_pixels_per_point: f32) -> f32 {
    let natural = (height_px / 1000.0).clamp(0.75, 2.6);
    natural * interface().scale / native_pixels_per_point.max(0.1)
}

/// Set the interface's size for this frame (before `Context::run`).
pub fn apply_zoom(ctx: &egui::Context, height_px: f32) {
    let native = ctx.native_pixels_per_point().unwrap_or(1.0);
    let zoom = zoom_for(height_px, native);
    if (ctx.zoom_factor() - zoom).abs() > 0.001 {
        ctx.set_zoom_factor(zoom);
    }
}

/// The steps the interface size row offers.
const SCALES: [f32; 5] = [0.8, 0.9, 1.0, 1.15, 1.3];

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
    let mut changed = g != was;
    if changed {
        set(g);
    }
    // the interface: its size and the tutorial tips (words from the pack;
    // the rows are left out until it has arrived)
    let t = crate::uitext::t;
    let scales = crate::uitext::lines("options.scales");
    if scales.len() == SCALES.len() {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(t("options.interface"))
                .font(theme::title_font(14.0))
                .color(theme::GOLD),
        );
        let now = interface();
        let names: Vec<&str> = scales.iter().map(String::as_str).collect();
        if let Some(i) = row(
            ui,
            &t("options.size"),
            Some(step(&SCALES, now.scale)),
            &names,
        ) {
            changed |= change_interface(|o| o.scale = SCALES[i]);
        }
        let (off, on, again) = (t("options.off"), t("options.on"), t("options.tips_again"));
        if let Some(i) = row(
            ui,
            &t("options.tips"),
            Some(now.tips as usize),
            &[&off, &on, &again],
        ) {
            changed |= change_interface(|o| match i {
                0 => o.tips = false,
                1 => o.tips = true,
                _ => {
                    o.tips = true;
                    o.tips_seen.clear();
                }
            });
        }
        if let Some(i) = row(
            ui,
            &t("options.shake"),
            Some(now.shake as usize),
            &[&off, &on],
        ) {
            changed |= change_interface(|o| o.shake = i == 1);
        }
    }
    // sound: the master level and each bus (names from the pack)
    let names = crate::uitext::lines("options.volumes");
    if names.len() == 5 {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(t("options.sound"))
                .font(theme::title_font(14.0))
                .color(theme::GOLD),
        );
        let mut s = sound().unwrap_or_else(|| Sound::of(&Settings::peek()));
        let mut moved = false;
        for (name, v) in names.iter().zip(s.levels()) {
            let (m, released) = slider(ui, name, v);
            moved |= m;
            // heard at once (the mixer follows `sound()`); saved when the
            // pointer comes off the slider
            changed |= released;
        }
        if moved {
            set_sound(s);
        }
    }
    changed
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

    #[test]
    fn sound_levels_go_into_settings_and_back() {
        let mut s = Settings::default();
        let mut v = Sound::of(&s);
        assert_eq!(v.master, 0.8);
        v.music = 0.25;
        v.ui = 0.0;
        v.into(&mut s);
        assert_eq!((s.volume_music, s.volume_ui), (0.25, 0.0));
        assert_eq!(Sound::of(&s), v);
        // heard as loudness: a quarter of the way is a sixteenth of the
        // signal, and the ends are silence and full
        let m = v.volumes();
        assert_eq!((m.music, m.ui), (0.0625, 0.0));
        assert!((m.master - 0.64).abs() < 1e-6);
        // a settings file from before the buses: the old volume is the master
        let old: Settings = serde_json::from_str(r#"{ "volume": 0.5 }"#).unwrap();
        let v = Sound::of(&old);
        assert_eq!((v.master, v.effects), (0.5, 1.0));
    }

    #[test]
    fn the_interface_is_the_same_share_of_any_screen() {
        // 1080p and 1440p: the same number of points down the window
        let a = 1080.0 / zoom_for(1080.0, 1.0);
        let b = 1440.0 / zoom_for(1440.0, 1.0);
        assert!((a - b).abs() < 0.5, "{a} {b}");
        // a HiDPI window's own factor is not applied twice
        assert!((zoom_for(2160.0, 2.0) - 1.08).abs() < 0.01);
    }
}
