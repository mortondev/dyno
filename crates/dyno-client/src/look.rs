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
    /// (was the height haze; `atmosphere` describes haze and fog now)
    pub haze: f32,
    pub sky: SkyPalette,
    /// sunlight, moonlight, sky light and lamps (atmos.rs)
    pub light: crate::atmos::Lighting,
    /// aerial haze and fog in low ground (atmos.rs)
    pub atmosphere: crate::atmos::Atmosphere,
    /// sun, moon, stars, clouds and the far ridges (sky.rs)
    pub dome: crate::sky::DomeLook,
    /// exposure, bloom and ambient occlusion (post.rs)
    pub post: crate::post::PostLook,
    /// the mood of the whole world, and of regions of it (mood.rs)
    pub mood: crate::mood::Mood,
    pub moods: Vec<crate::mood::Region>,
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

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Water {
    pub deep: [f32; 3],
    pub shallow: [f32; 3],
    pub foam: [f32; 3],
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
    /// the sky just after sunset and before sunrise, and how much of it
    /// shows (0: none)
    pub dusk_top: [f32; 3],
    pub dusk_horizon: [f32; 3],
    pub dusk: f32,
    /// how high the sun stands (as the sine of its height) while the
    /// sunset colours still show
    pub sunset_width: f32,
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
            light: Default::default(),
            atmosphere: Default::default(),
            dome: Default::default(),
            post: Default::default(),
            mood: Default::default(),
            moods: Vec::new(),
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
            dusk_top: [0.2; 3],
            dusk_horizon: [0.4; 3],
            dusk: 0.0,
            sunset_width: 0.17,
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
