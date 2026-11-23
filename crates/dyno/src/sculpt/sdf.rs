//! Signed distance shapes for sculpting figures: negative inside, roughly
//! the distance to the surface outside (good enough to voxelise and to
//! offset shells by), in rig units.

use glam::{Vec2, Vec3, vec3};

pub fn sphere(p: Vec3, c: Vec3, r: f32) -> f32 {
    (p - c).length() - r
}

/// An ellipsoid (a good approximation of the distance near the surface).
pub fn ellipsoid(p: Vec3, c: Vec3, r: Vec3) -> f32 {
    let q = p - c;
    let k0 = (q / r).length();
    let k1 = (q / (r * r)).length();
    if k1 < 1e-6 {
        return -r.min_element();
    }
    k0 * (k0 - 1.0) / k1
}

/// A box with rounded edges: centre, half size, rounding.
pub fn rbox(p: Vec3, c: Vec3, h: Vec3, r: f32) -> f32 {
    let q = (p - c).abs() - h + Vec3::splat(r);
    q.max(Vec3::ZERO).length() + q.max_element().min(0.0) - r
}

/// A cone with rounded ends between `a` (radius `ra`) and `b` (`rb`).
pub fn cone(p: Vec3, a: Vec3, b: Vec3, ra: f32, rb: f32) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
    (p - (a + ab * t)).length() - (ra + (rb - ra) * t)
}

/// A limb from `a` to `b` whose cross-section is an ellipse that changes
/// along it: `profile` is (t, half width side to side, half depth front to
/// back), t from 0 at `a` to 1 at `b`. `front` says which way is front.
pub fn limb(p: Vec3, a: Vec3, b: Vec3, front: Vec3, profile: &[(f32, f32, f32)]) -> f32 {
    let ab = b - a;
    let len = ab.length().max(1e-6);
    let axis = ab / len;
    let f = (front - axis * front.dot(axis)).normalize_or_zero();
    let side = axis.cross(f);
    let d = p - a;
    let along = d.dot(axis);
    let t = along / len;
    let (rx, rz) = profile_at(profile, t.clamp(0.0, 1.0));
    let q = Vec2::new(d.dot(side), d.dot(f));
    let k = (q / Vec2::new(rx, rz)).length();
    let r = rx.min(rz);
    let radial = (k - 1.0) * r;
    // round the ends
    let cap = if t < 0.0 {
        -along
    } else if t > 1.0 {
        along - len
    } else {
        f32::MIN
    };
    if cap > 0.0 {
        // a rounded end the size of the end's section
        let (rx0, rz0) = profile_at(profile, if t < 0.0 { 0.0 } else { 1.0 });
        let end = rx0.min(rz0);
        return Vec2::new((q / Vec2::new(rx0, rz0)).length() * end, cap).length() - end;
    }
    radial
}

pub fn profile_at(profile: &[(f32, f32, f32)], t: f32) -> (f32, f32) {
    let mut prev = profile[0];
    if t <= prev.0 {
        return (prev.1, prev.2);
    }
    for &cur in &profile[1..] {
        if t <= cur.0 {
            let u = (t - prev.0) / (cur.0 - prev.0).max(1e-6);
            let u = u * u * (3.0 - 2.0 * u);
            return (prev.1 + (cur.1 - prev.1) * u, prev.2 + (cur.2 - prev.2) * u);
        }
        prev = cur;
    }
    (prev.1, prev.2)
}

/// Smooth minimum (union) with blend radius `k`.
pub fn smin(a: f32, b: f32, k: f32) -> f32 {
    if k <= 0.0 {
        return a.min(b);
    }
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.min(b) - h * h * k * 0.25
}

/// Smooth maximum (intersection, or subtraction with `-b`).
pub fn smax(a: f32, b: f32, k: f32) -> f32 {
    -smin(-a, -b, k)
}

/// Mirror to the right side (x >= 0).
pub fn mirror(p: Vec3) -> Vec3 {
    vec3(p.x.abs(), p.y, p.z)
}

/// Smooth step from `a` to `b`.
pub fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Value noise in 0..1, smooth, period about 1 at `freq` 1.
pub fn noise(p: Vec3) -> f32 {
    let i = p.floor();
    let f = p - i;
    let u = f * f * (Vec3::splat(3.0) - 2.0 * f);
    let h = |x: f32, y: f32, z: f32| -> f32 {
        let n = (x as i32).wrapping_mul(374_761_393)
            ^ (y as i32).wrapping_mul(668_265_263)
            ^ (z as i32).wrapping_mul(1_274_126_177);
        let n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
        ((n ^ (n >> 16)) & 0xffff) as f32 / 65535.0
    };
    let mut v = 0.0;
    for c in 0..8 {
        let (dx, dy, dz) = ((c & 1) as f32, ((c >> 1) & 1) as f32, ((c >> 2) & 1) as f32);
        let w = (if dx > 0.0 { u.x } else { 1.0 - u.x })
            * (if dy > 0.0 { u.y } else { 1.0 - u.y })
            * (if dz > 0.0 { u.z } else { 1.0 - u.z });
        v += w * h(i.x + dx, i.y + dy, i.z + dz);
    }
    v
}
