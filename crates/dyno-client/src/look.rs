//! Render settings from the pack (`render.json`): the colours and strengths
//! that make the painted look: leaf shading, ground tone, relief, the colour
//! grade, water, and the sky and fog through the day. The shader gets them
//! as a uniform block. Without the file the picture is neutral: no grade,
//! no tone drift, grey skies.

use crate::assets;
use bytemuck::{Pod, Zeroable};
use serde::Deserialize;
use std::sync::{Arc, Mutex};

/// The pack file.
pub const RENDER_JSON: &str = "render.json";

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct RenderSettings {
    pub leaves: Leaves,
    pub tone: Tone,
    /// how strongly a texture's painted light bends the surface normal
    pub relief: f32,
    pub grade: Grade,
    pub water: Water,
    /// height haze in low valleys: how much it greys the picture
    pub haze: f32,
    pub sky: SkyPalette,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Leaves {
    /// texture repeats per metre
    pub scale: f32,
    /// tint of leaves facing down (shade) and up (sunlit crowns)
    pub shade_under: [f32; 3],
    pub shade_over: [f32; 3],
    /// brightness from the slow world tone: base + range × tone
    pub tone_base: f32,
    pub tone_range: f32,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Tone {
    /// ground tint at the two ends of the slow tone drift
    pub ground_low: [f32; 3],
    pub ground_high: [f32; 3],
    /// brightness of built blocks at the two ends
    pub built_low: f32,
    pub built_high: f32,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Grade {
    /// multiplies every colour: warm or cool light
    pub tint: [f32; 3],
    pub saturation: f32,
    /// contrast around mid grey
    pub contrast: f32,
    /// highlights above this roll off instead of clipping
    pub knee: f32,
    pub rolloff: f32,
}

/// Water: the look where nothing else is said, named kinds of water that
/// change parts of it, and where each kind lies (water.rs draws it).
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Water {
    pub deep: [f32; 3],
    pub shallow: [f32; 3],
    pub foam: [f32; 3],
    /// metres of water over the bed at which the colour is fully `deep`
    pub depth: f32,
    /// metres you can see through the water before `shallow` covers the bed
    pub clarity: f32,
    /// foam forms where the water is shallower than this (metres)
    pub foam_width: f32,
    /// how much of the sky and the land the surface mirrors, 0..1
    pub reflect: f32,
    /// strength of the ripples
    pub ripple: f32,
    /// glints of the sun and the moon
    pub sparkle: f32,
    /// speed along a course, metres a second
    pub flow: f32,
    /// what you see when your eyes are under: the colour everything fades
    /// to, and how fast (per metre)
    pub fog: [f32; 3],
    pub fog_density: f32,
    /// kinds of water by name: each changes what it names of the above
    pub kinds: std::collections::BTreeMap<String, WaterKind>,
    /// still water of a kind: an ellipse around a place
    pub areas: Vec<WaterArea>,
    /// running water of a kind: it flows along the points in order
    pub courses: Vec<WaterCourse>,
}

/// A kind of water: what it changes of the base look.
#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default, deny_unknown_fields)]
pub struct WaterKind {
    pub deep: Option<[f32; 3]>,
    pub shallow: Option<[f32; 3]>,
    pub foam: Option<[f32; 3]>,
    pub depth: Option<f32>,
    pub clarity: Option<f32>,
    pub foam_width: Option<f32>,
    pub reflect: Option<f32>,
    pub ripple: Option<f32>,
    pub sparkle: Option<f32>,
    pub flow: Option<f32>,
    pub fog: Option<[f32; 3]>,
    pub fog_density: Option<f32>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WaterArea {
    pub kind: String,
    /// centre (x, z) from the spawn hearth
    pub at: [f32; 2],
    /// half-axes along x and z
    pub radius: [f32; 2],
    /// the part of the radius over which the kind fades in, 0..1
    #[serde(default = "feather")]
    pub feather: f32,
}

fn feather() -> f32 {
    0.25
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WaterCourse {
    pub kind: String,
    /// x, z from the spawn hearth, and half the width there
    pub points: Vec<[f32; 3]>,
}

/// Sky and fog colours (sRGB) through the day: night, day, and the glow
/// added round sunrise and sunset.
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct SkyPalette {
    pub night_top: [f32; 3],
    pub day_top: [f32; 3],
    /// the fog colour is the horizon colour
    pub night_horizon: [f32; 3],
    pub day_horizon: [f32; 3],
    pub sunset_horizon: [f32; 3],
    /// added to the top of the sky at sunset
    pub sunset_top: [f32; 3],
    pub night_light: [f32; 3],
    pub day_light: [f32; 3],
    pub sunset_light: [f32; 3],
}

impl Default for RenderSettings {
    fn default() -> Self {
        RenderSettings {
            leaves: Leaves::default(),
            tone: Tone::default(),
            relief: 0.0,
            grade: Grade::default(),
            water: Water::default(),
            haze: 0.0,
            sky: SkyPalette::default(),
        }
    }
}

impl Default for Leaves {
    fn default() -> Self {
        Leaves {
            scale: 1.0,
            shade_under: [1.0; 3],
            shade_over: [1.0; 3],
            tone_base: 1.0,
            tone_range: 0.0,
        }
    }
}

impl Default for Tone {
    fn default() -> Self {
        Tone {
            ground_low: [1.0; 3],
            ground_high: [1.0; 3],
            built_low: 1.0,
            built_high: 1.0,
        }
    }
}

impl Default for Grade {
    fn default() -> Self {
        Grade {
            tint: [1.0; 3],
            saturation: 1.0,
            contrast: 1.0,
            knee: 1.0,
            rolloff: 0.0,
        }
    }
}

impl Default for Water {
    fn default() -> Self {
        Water {
            deep: [0.1, 0.2, 0.3],
            shallow: [0.2, 0.3, 0.4],
            foam: [0.9; 3],
            depth: 3.0,
            clarity: 2.5,
            foam_width: 0.25,
            reflect: 0.5,
            ripple: 1.0,
            sparkle: 1.0,
            flow: 0.0,
            fog: [0.1, 0.2, 0.3],
            fog_density: 0.12,
            kinds: Default::default(),
            areas: Vec::new(),
            courses: Vec::new(),
        }
    }
}

impl Default for SkyPalette {
    fn default() -> Self {
        SkyPalette {
            night_top: [0.05; 3],
            day_top: [0.6; 3],
            night_horizon: [0.08; 3],
            day_horizon: [0.7; 3],
            sunset_horizon: [0.7; 3],
            sunset_top: [0.0; 3],
            night_light: [0.3; 3],
            day_light: [1.0; 3],
            sunset_light: [1.0; 3],
        }
    }
}

impl RenderSettings {
    pub fn parse(text: &str) -> Result<RenderSettings, String> {
        serde_json::from_str(text).map_err(|e| format!("{RENDER_JSON}: {e}"))
    }
}

/// The settings in the pack now, parsed again only when the file changes.
pub fn settings() -> RenderSettings {
    type Cache = Option<(Arc<[u8]>, RenderSettings)>;
    static CACHE: Mutex<Cache> = Mutex::new(None);
    let Some(bytes) = assets::get(RENDER_JSON) else {
        return RenderSettings::default();
    };
    let mut c = CACHE.lock().unwrap();
    if let Some((b, s)) = c.as_ref()
        && Arc::ptr_eq(b, &bytes)
    {
        return s.clone();
    }
    let s = std::str::from_utf8(&bytes)
        .map_err(|e| e.to_string())
        .and_then(RenderSettings::parse)
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            RenderSettings::default()
        });
    *c = Some((bytes, s.clone()));
    s
}

/// The settings as the shader's `Look` uniform block.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct LookUniform {
    /// rgb: leaves facing down; w: leaf tone base
    leaf_under: [f32; 4],
    /// rgb: leaves facing up; w: leaf tone range
    leaf_over: [f32; 4],
    /// rgb: ground tint, low tone; w: built blocks, low tone
    tone_low: [f32; 4],
    /// rgb: ground tint, high tone; w: built blocks, high tone
    tone_high: [f32; 4],
    /// rgb: grade tint; w: saturation
    grade_tint: [f32; 4],
    /// contrast, knee, rolloff, relief
    grade: [f32; 4],
    /// unused since water.rs took the water over; kept so the block's
    /// layout stays as the shader has it
    water_deep: [f32; 4],
    water_shallow: [f32; 4],
    foam: [f32; 4],
    /// leaf scale, haze, 0, 0
    misc: [f32; 4],
}

impl RenderSettings {
    pub fn uniform(&self) -> LookUniform {
        let v = |c: [f32; 3], w: f32| [c[0], c[1], c[2], w];
        let l = &self.leaves;
        let t = &self.tone;
        let g = &self.grade;
        LookUniform {
            leaf_under: v(l.shade_under, l.tone_base),
            leaf_over: v(l.shade_over, l.tone_range),
            tone_low: v(t.ground_low, t.built_low),
            tone_high: v(t.ground_high, t.built_high),
            grade_tint: v(g.tint, g.saturation),
            grade: [g.contrast, g.knee, g.rolloff, self.relief],
            water_deep: v(self.water.deep, 0.0),
            water_shallow: v(self.water.shallow, 0.0),
            foam: v(self.water.foam, 0.0),
            misc: [l.scale, self.haze, 0.0, 0.0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_files_keep_the_rest_neutral() {
        let s = RenderSettings::parse(r#"{"relief": 0.5, "grade": {"saturation": 2.0}}"#).unwrap();
        assert_eq!(s.relief, 0.5);
        assert_eq!(s.grade.saturation, 2.0);
        assert_eq!(s.grade.contrast, 1.0);
        assert!(RenderSettings::parse(r#"{"relif": 0.5}"#).is_err());
    }
}
