//! The cube ladder of the voxel rule (docs/art/voxel-rule.md): everything
//! drawn is cubes of one of six sizes, each half the one before, from a
//! metre down to 3.125 cm. Meshers and loaders ask here whether a size is
//! on the ladder and snap sizes that vary (a particle growing) to it.

/// Cubes a metre at each step: 1 m, 50 cm, 25 cm, 12.5 cm, 6.25 cm and
/// 3.125 cm.
pub const PER_METRE: [u32; 6] = [1, 2, 4, 8, 16, 32];

/// The finest cube, in metres.
pub const FINEST: f32 = 1.0 / 32.0;

/// The ladder step (1 to 6) of a cube with this edge in metres, when it is
/// one (to within a hundredth of a millimetre).
pub fn step_of(edge: f32) -> Option<u8> {
    PER_METRE
        .iter()
        .position(|n| (edge * *n as f32 - 1.0).abs() < 1e-5 * *n as f32)
        .map(|i| i as u8 + 1)
}

/// The ladder step (1 to 6) of a grid with this many cubes a metre.
pub fn step_of_per_metre(n: f32) -> Option<u8> {
    PER_METRE
        .iter()
        .position(|k| (n - *k as f32).abs() < 1e-4)
        .map(|i| i as u8 + 1)
}

/// The ladder edge nearest to `edge` (by ratio), in metres; `None` for
/// something under two thirds of the finest cube, which is not drawn.
pub fn snap(edge: f32) -> Option<f32> {
    if edge.is_nan() || edge < FINEST * (2.0 / 3.0) {
        return None;
    }
    let step = (-edge.log2()).round().clamp(0.0, 5.0);
    Some(0.5f32.powi(step as i32))
}

/// True when `v` (metres) lies on the grid of cubes `per_metre` a metre.
pub fn on_grid(v: f32, per_metre: f32) -> bool {
    let k = v * per_metre;
    // world coordinates run to a few thousand metres in f32
    (k - k.round()).abs() <= 1e-3 + k.abs() * 1e-6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ladder_has_six_steps() {
        assert_eq!(step_of(1.0), Some(1));
        assert_eq!(step_of(0.5), Some(2));
        assert_eq!(step_of(0.25), Some(3));
        assert_eq!(step_of(0.125), Some(4));
        assert_eq!(step_of(0.0625), Some(5));
        assert_eq!(step_of(0.03125), Some(6));
        for off in [2.0, 0.06, 0.07, 0.18, 0.035, 0.015625, 1.0 / 34.63, 0.2] {
            assert_eq!(step_of(off), None, "{off}");
        }
        assert_eq!(step_of_per_metre(16.0), Some(5));
        assert_eq!(step_of_per_metre(34.63), None);
    }

    #[test]
    fn sizes_snap_to_the_ladder() {
        assert_eq!(snap(1.15), Some(1.0));
        assert_eq!(snap(0.34), Some(0.25));
        assert_eq!(snap(0.36), Some(0.5));
        assert_eq!(snap(0.07), Some(0.0625));
        assert_eq!(snap(0.04), Some(0.03125));
        assert_eq!(snap(0.025), Some(0.03125));
        assert_eq!(snap(0.02), None);
        assert_eq!(snap(0.0), None);
        for e in [0.03, 0.05, 0.1, 0.2, 0.4, 0.8, 3.0] {
            assert!(step_of(snap(e).unwrap()).is_some());
        }
    }

    #[test]
    fn grids() {
        assert!(on_grid(1037.4375, 16.0));
        assert!(!on_grid(1037.45, 16.0));
        assert!(on_grid(-0.5, 2.0));
        assert!(!on_grid(10.8, 16.0));
    }
}
