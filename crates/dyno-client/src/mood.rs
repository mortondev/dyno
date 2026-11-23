//! Moods: how a place tints the light and air, as the pack describes it
//! (`render.json`: `mood` for everywhere, `moods` for regions). A region is
//! a circle round one of the world's areas (by its key in the rules) or
//! round a point given relative to the spawn hearth; its mood fades in over
//! its `feather`, and where regions overlap the one with the higher `order`
//! lies on top (of equals, the smaller). The
//! engine only blends: which places have which mood is the pack's business.

use crate::look::RenderSettings;
use glam::Vec3;
use serde::Deserialize;
use std::sync::Mutex;

/// What a mood may change. Anything left out keeps what lies beneath.
#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Mood {
    /// multiply the grade of the finished picture
    pub tint: Option<[f32; 3]>,
    pub exposure: Option<f32>,
    pub saturation: Option<f32>,
    pub contrast: Option<f32>,
    /// multiplies the sky's colours (and with them the fog at the horizon)
    pub sky_tint: Option<[f32; 3]>,
    /// multiplies sunlight and moonlight
    pub light_tint: Option<[f32; 3]>,
    /// multiplies the light from the sky
    pub ambient: Option<f32>,
    /// the haze's own colour here, and how much of it replaces the hour's
    pub haze_color: Option<[f32; 3]>,
    pub haze_color_mix: Option<f32>,
    /// multiplies the haze's density
    pub haze: Option<f32>,
    pub fog_color: Option<[f32; 3]>,
    pub fog_color_mix: Option<f32>,
    /// multiplies the fog's density
    pub fog: Option<f32>,
    /// raises the fog's top (metres)
    pub fog_lift: Option<f32>,
    /// cloud cover, 0 to 1
    pub clouds: Option<f32>,
    /// the hour this place likes best (0 sunrise, 0.25 noon, 0.5 sunset,
    /// 0.75 midnight) and how far it pulls the sky towards it (0 not at
    /// all .. 1 always that hour). Only the look moves: the world's clock
    /// is the server's.
    pub time: Option<f32>,
    pub time_pull: Option<f32>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Region {
    /// the key of an area in the rules: its centre and radius
    pub area: Option<String>,
    /// or a centre (x, z) relative to the spawn hearth, and a radius
    pub at: Option<[f32; 2]>,
    pub radius: Option<f32>,
    /// the mood fades in over this many metres outside the radius
    pub feather: f32,
    /// where regions overlap, the higher order lies on top
    pub order: i32,
    pub mood: Mood,
}

impl Default for Region {
    fn default() -> Self {
        Region {
            area: None,
            at: None,
            radius: None,
            feather: 24.0,
            order: 0,
            mood: Mood::default(),
        }
    }
}

/// The moods of a spot, blended.
#[derive(Clone, Copy, Debug)]
pub struct Blend {
    pub tint: [f32; 3],
    pub exposure: f32,
    pub saturation: f32,
    pub contrast: f32,
    pub sky_tint: [f32; 3],
    pub light_tint: [f32; 3],
    pub ambient: f32,
    pub haze_color: [f32; 3],
    pub haze_color_mix: f32,
    pub haze: f32,
    pub fog_color: [f32; 3],
    pub fog_color_mix: f32,
    pub fog: f32,
    pub fog_lift: f32,
    pub clouds: f32,
    /// added to the time of day
    pub time_shift: f32,
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [mix(a[0], b[0], t), mix(a[1], b[1], t), mix(a[2], b[2], t)]
}

impl Blend {
    /// No mood at all, with the pack's cloud cover.
    pub fn plain(s: &RenderSettings) -> Blend {
        Blend {
            tint: [1.0; 3],
            exposure: 1.0,
            saturation: 1.0,
            contrast: 1.0,
            sky_tint: [1.0; 3],
            light_tint: [1.0; 3],
            ambient: 1.0,
            haze_color: [1.0; 3],
            haze_color_mix: 0.0,
            haze: 1.0,
            fog_color: [1.0; 3],
            fog_color_mix: 0.0,
            fog: 1.0,
            fog_lift: 0.0,
            clouds: s.dome.clouds.cover,
            time_shift: 0.0,
        }
    }

    /// Lay a mood over this one with weight `w`, at time of day `t`.
    fn lay(&mut self, m: &Mood, w: f32, t: f32) {
        let f = |v: &mut f32, o: Option<f32>| {
            if let Some(o) = o {
                *v = mix(*v, o, w);
            }
        };
        let c = |v: &mut [f32; 3], o: Option<[f32; 3]>| {
            if let Some(o) = o {
                *v = mix3(*v, o, w);
            }
        };
        c(&mut self.tint, m.tint);
        f(&mut self.exposure, m.exposure);
        f(&mut self.saturation, m.saturation);
        f(&mut self.contrast, m.contrast);
        c(&mut self.sky_tint, m.sky_tint);
        c(&mut self.light_tint, m.light_tint);
        f(&mut self.ambient, m.ambient);
        // a colour fades in with its own weight, so it never passes through
        // the neutral white on the way
        if let Some(col) = m.haze_color {
            let k = m.haze_color_mix.unwrap_or(1.0) * w;
            let total = self.haze_color_mix + k * (1.0 - self.haze_color_mix);
            if total > 0.0 {
                self.haze_color = mix3(self.haze_color, col, k / total);
            }
            self.haze_color_mix = total;
        }
        f(&mut self.haze, m.haze);
        if let Some(col) = m.fog_color {
            let k = m.fog_color_mix.unwrap_or(1.0) * w;
            let total = self.fog_color_mix + k * (1.0 - self.fog_color_mix);
            if total > 0.0 {
                self.fog_color = mix3(self.fog_color, col, k / total);
            }
            self.fog_color_mix = total;
        }
        f(&mut self.fog, m.fog);
        f(&mut self.fog_lift, m.fog_lift);
        f(&mut self.clouds, m.clouds);
        if let Some(hour) = m.time {
            // the short way round the clock
            let mut d = (hour - t).rem_euclid(1.0);
            if d > 0.5 {
                d -= 1.0;
            }
            let shift = d * m.time_pull.unwrap_or(1.0).clamp(0.0, 1.0);
            self.time_shift = mix(self.time_shift, shift, w);
        } else if m.time_pull == Some(0.0) {
            self.time_shift = mix(self.time_shift, 0.0, w);
        }
    }
}

/// The places of the world being played: the spawn hearth, and each area's
/// key, centre (relative to the hearth) and radius.
struct Places {
    hearth: [f32; 2],
    areas: Vec<(String, [f32; 2], f32)>,
}

static PLACES: Mutex<Option<Places>> = Mutex::new(None);

/// Tell the moods where the world's areas lie (on joining a world).
pub fn set_world(hearth: [i32; 3], areas: &[dyno_world::content::Area]) {
    *PLACES.lock().unwrap() = Some(Places {
        hearth: [hearth[0] as f32, hearth[2] as f32],
        areas: areas
            .iter()
            .map(|a| (a.key.clone(), [a.at[0] as f32, a.at[1] as f32], a.radius))
            .collect(),
    });
}

/// No world: only the pack's mood for everywhere applies.
pub fn clear_world() {
    *PLACES.lock().unwrap() = None;
}

/// How much of a region's mood reaches a point `d` metres from its centre.
fn weight(d: f32, radius: f32, feather: f32) -> f32 {
    if d <= radius {
        return 1.0;
    }
    if feather <= 0.0 || d >= radius + feather {
        return 0.0;
    }
    let t = 1.0 - (d - radius) / feather;
    t * t * (3.0 - 2.0 * t)
}

/// The mood at a spot in the world (`None`: nowhere), at time of day `t`.
pub fn blend(s: &RenderSettings, t: f32, at: Option<Vec3>) -> Blend {
    let mut b = Blend::plain(s);
    b.lay(&s.mood, 1.0, t);
    let places = PLACES.lock().unwrap();
    let (Some(at), Some(places)) = (at, places.as_ref()) else {
        return b;
    };
    let rel = [at.x - places.hearth[0], at.z - places.hearth[1]];
    blend_regions(&mut b, &s.moods, &places.areas, rel, t);
    b
}

fn blend_regions(
    b: &mut Blend,
    regions: &[Region],
    areas: &[(String, [f32; 2], f32)],
    rel: [f32; 2],
    t: f32,
) {
    // the regions that reach this spot, lowest first: by order, then widest
    // first, so the smaller lie on top
    let mut reach: Vec<(i32, f32, f32, &Mood)> = regions
        .iter()
        .filter_map(|r| {
            let area = r
                .area
                .as_ref()
                .and_then(|k| areas.iter().find(|a| &a.0 == k));
            let centre = r.at.or(area.map(|a| a.1))?;
            let radius = r.radius.or(area.map(|a| a.2))?;
            let d = (rel[0] - centre[0]).hypot(rel[1] - centre[1]);
            let w = weight(d, radius, r.feather);
            (w > 0.0).then_some((r.order, radius, w, &r.mood))
        })
        .collect();
    reach.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.total_cmp(&a.1)));
    for (_, _, w, m) in reach {
        b.lay(m, w, t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(area: &str, mood: Mood) -> Region {
        Region {
            area: Some(area.into()),
            mood,
            ..Default::default()
        }
    }

    #[test]
    fn smaller_regions_lie_on_top_and_fade_at_their_edges() {
        let s = RenderSettings::default();
        let areas = vec![
            ("wide".to_string(), [0.0, 0.0], 100.0),
            ("small".to_string(), [0.0, 0.0], 10.0),
        ];
        let regions = vec![
            region(
                "small",
                Mood {
                    exposure: Some(2.0),
                    ..Default::default()
                },
            ),
            region(
                "wide",
                Mood {
                    exposure: Some(0.5),
                    saturation: Some(1.5),
                    ..Default::default()
                },
            ),
            region("nowhere", Mood::default()),
        ];
        let at = |x: f32| {
            let mut b = Blend::plain(&s);
            blend_regions(&mut b, &regions, &areas, [x, 0.0], 0.25);
            b
        };
        assert_eq!(at(0.0).exposure, 2.0);
        assert_eq!(at(0.0).saturation, 1.5);
        assert_eq!(at(50.0).exposure, 0.5);
        let edge = at(22.0).exposure;
        assert!(edge > 0.5 && edge < 2.0, "{edge}");
        assert_eq!(at(200.0).exposure, 1.0);
    }

    #[test]
    fn the_hour_is_pulled_the_short_way_round() {
        let s = RenderSettings::default();
        let mut b = Blend::plain(&s);
        let m = Mood {
            time: Some(0.02),
            time_pull: Some(0.5),
            ..Default::default()
        };
        b.lay(&m, 1.0, 0.9);
        assert!((b.time_shift - 0.06).abs() < 1e-5, "{}", b.time_shift);
        let mut b = Blend::plain(&s);
        b.lay(&m, 1.0, 0.25);
        assert!((b.time_shift + 0.115).abs() < 1e-5, "{}", b.time_shift);
    }

    #[test]
    fn a_colour_fades_in_without_passing_through_white() {
        let s = RenderSettings::default();
        let mut b = Blend::plain(&s);
        let m = Mood {
            haze_color: Some([0.5, 0.2, 0.8]),
            ..Default::default()
        };
        b.lay(&m, 0.1, 0.0);
        for (got, want) in b.haze_color.iter().zip([0.5, 0.2, 0.8]) {
            assert!((got - want).abs() < 1e-5, "{got} for {want}");
        }
        assert!((b.haze_color_mix - 0.1).abs() < 1e-6);
    }
}
