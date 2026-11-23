//! Anatomy: the bare body and head of a person, as distance fields over
//! rig units (feet at y = 0, facing +z, right is +x), for each body type.

use super::sdf::*;
use glam::{Vec3, vec3};

/// The body's parts in the main grid (one side; the other is mirrored).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bp {
    Torso,
    ArmU,
    ArmL,
    Thigh,
    Shin,
}

pub const BPS: [Bp; 5] = [Bp::Torso, Bp::ArmU, Bp::ArmL, Bp::Thigh, Bp::Shin];

/// One body type: its joints (right side) and how full each mass is.
#[derive(Clone, Debug)]
pub struct Body {
    pub female: bool,
    pub hip: Vec3,
    pub knee: Vec3,
    pub ankle: Vec3,
    pub shoulder: Vec3,
    pub elbow: Vec3,
    pub wrist: Vec3,
    /// the head's pivot (top of the neck)
    pub head: Vec3,
    pub pelvis_y: f32,
    pub cape: Vec3,
}

impl Body {
    pub fn male() -> Body {
        Body {
            female: false,
            hip: vec3(4.3, 30.0, 0.0),
            knee: vec3(4.15, 16.6, 0.4),
            ankle: vec3(4.0, 3.9, -0.6),
            shoulder: vec3(9.8, 48.3, -0.3),
            elbow: vec3(11.0, 37.4, -0.7),
            wrist: vec3(11.4, 27.7, 0.3),
            head: vec3(0.0, 51.8, 0.0),
            pelvis_y: 30.0,
            cape: vec3(0.0, 49.4, -5.0),
        }
    }

    pub fn female() -> Body {
        Body {
            female: true,
            hip: vec3(4.4, 30.0, 0.0),
            knee: vec3(3.75, 16.6, 0.4),
            ankle: vec3(3.55, 3.9, -0.5),
            shoulder: vec3(8.5, 47.6, -0.3),
            elbow: vec3(9.5, 37.0, -0.6),
            wrist: vec3(9.85, 27.9, 0.3),
            head: vec3(0.0, 51.3, 0.0),
            pelvis_y: 30.0,
            cape: vec3(0.0, 48.8, -4.4),
        }
    }

    pub fn torso(&self, p: Vec3) -> f32 {
        let m = mirror(p);
        if self.female {
            let rib = ellipsoid(p, vec3(0.0, 42.6, 0.0), vec3(6.4, 6.2, 4.1));
            let bust = ellipsoid(m, vec3(2.55, 42.9, 3.0), vec3(2.45, 2.35, 2.2));
            let abs = ellipsoid(p, vec3(0.0, 37.0, 0.3), vec3(4.7, 4.6, 3.4));
            let pel = ellipsoid(p, vec3(0.0, 31.3, -0.2), vec3(7.3, 4.4, 4.3));
            let glute = ellipsoid(m, vec3(3.3, 30.3, -2.4), vec3(3.5, 3.7, 2.9));
            let lat = ellipsoid(m, vec3(4.2, 42.0, -1.2), vec3(2.0, 4.6, 2.6));
            let trap = cone(m, vec3(1.0, 48.4, -0.8), vec3(6.6, 47.3, -0.8), 1.6, 1.3);
            let neck = cone(p, vec3(0.0, 46.5, -0.4), vec3(0.0, 52.6, 0.0), 2.05, 1.8);
            let clav = cone(m, vec3(0.8, 47.5, 1.4), vec3(6.8, 47.7, 0.3), 0.8, 0.8);
            let mut d = smin(rib, bust, 0.9);
            d = smin(d, abs, 2.6);
            d = smin(d, pel, 2.4);
            d = smin(d, glute, 1.6);
            d = smin(d, lat, 1.8);
            d = smin(d, trap, 1.4);
            d = smin(d, neck, 1.4);
            smin(d, clav, 0.9)
        } else {
            let rib = ellipsoid(p, vec3(0.0, 43.2, 0.2), vec3(7.5, 6.7, 4.6));
            let pec = ellipsoid(m, vec3(3.5, 45.0, 3.0), vec3(3.8, 2.8, 1.9));
            let abs = ellipsoid(p, vec3(0.0, 37.4, 0.6), vec3(5.5, 5.0, 3.8));
            let pel = ellipsoid(p, vec3(0.0, 31.7, -0.1), vec3(6.8, 4.2, 4.3));
            let glute = ellipsoid(m, vec3(3.0, 30.6, -2.3), vec3(3.2, 3.4, 2.6));
            let lat = ellipsoid(m, vec3(5.0, 42.2, -1.4), vec3(2.4, 5.4, 3.0));
            let trap = cone(m, vec3(1.2, 49.4, -0.9), vec3(7.8, 48.0, -0.9), 2.1, 1.7);
            let neck = cone(p, vec3(0.0, 47.0, -0.4), vec3(0.0, 53.0, 0.1), 2.55, 2.25);
            let clav = cone(m, vec3(1.0, 48.3, 1.7), vec3(8.0, 48.5, 0.4), 1.05, 1.05);
            let mut d = smin(rib, pec, 1.2);
            d = smin(d, abs, 2.6);
            d = smin(d, pel, 2.6);
            d = smin(d, glute, 1.5);
            d = smin(d, lat, 2.0);
            d = smin(d, trap, 1.6);
            d = smin(d, neck, 1.5);
            smin(d, clav, 1.0)
        }
    }

    /// Upper arm and shoulder cap (right side; `m` mirrored).
    pub fn arm_u(&self, m: Vec3) -> f32 {
        let f = self.female;
        let delt = ellipsoid(
            m,
            self.shoulder + vec3(0.35, -0.6, 0.0),
            if f { vec3(2.45, 3.0, 2.5) } else { vec3(3.05, 3.7, 3.1) },
        );
        let prof: &[(f32, f32, f32)] = if f {
            &[(0.0, 1.95, 2.1), (0.45, 1.8, 2.05), (1.0, 1.55, 1.65)]
        } else {
            &[(0.0, 2.45, 2.7), (0.45, 2.3, 2.8), (1.0, 1.95, 2.15)]
        };
        let arm = limb(m, self.shoulder, self.elbow, Vec3::Z, prof);
        smin(delt, arm, 1.4)
    }

    /// Forearm and hand (right side).
    pub fn arm_l(&self, m: Vec3) -> f32 {
        let prof: &[(f32, f32, f32)] = if self.female {
            &[(0.0, 1.6, 1.65), (0.25, 1.75, 1.75), (1.0, 1.2, 1.0)]
        } else {
            &[(0.0, 2.0, 2.05), (0.25, 2.3, 2.25), (1.0, 1.45, 1.2)]
        };
        let fore = limb(m, self.elbow, self.wrist, Vec3::Z, prof);
        smin(fore, self.hand(m), 0.7)
    }

    /// The hand's frame: down the hand, across (palm faces -x), front.
    pub fn hand_frame(&self) -> (Vec3, Vec3, Vec3) {
        let dir = (self.wrist - self.elbow).normalize();
        let s = (Vec3::X - dir * dir.x).normalize();
        let f = dir.cross(s);
        (dir, s, f)
    }

    /// A point in hand coordinates: (across, down, front) from the wrist.
    pub fn hand_local(&self, m: Vec3) -> Vec3 {
        let (dir, s, f) = self.hand_frame();
        let d = m - self.wrist;
        vec3(d.dot(s), d.dot(dir), d.dot(f))
    }

    pub fn hand(&self, m: Vec3) -> f32 {
        let q = self.hand_local(m);
        let k = if self.female { 0.82 } else { 1.0 };
        let q = q / k;
        let palm = rbox(q, vec3(0.0, 1.8, 0.15), vec3(0.8, 1.75, 1.55), 0.65);
        // the fingers curl in towards the palm
        let curl = (q.y - 3.3).max(0.0) * 0.32;
        let qf = vec3(q.x + curl, q.y, q.z);
        let fingers = rbox(qf, vec3(-0.15, 4.35, 0.0), vec3(0.62, 1.15, 1.45), 0.55);
        let thumb = cone(q, vec3(-0.45, 0.9, 1.25), vec3(-1.05, 3.1, 1.85), 0.62, 0.45);
        let d = smin(smin(palm, fingers, 0.5), thumb, 0.5);
        d * k
    }

    pub fn thigh(&self, m: Vec3) -> f32 {
        let prof: &[(f32, f32, f32)] = if self.female {
            &[(0.0, 4.0, 4.1), (0.3, 3.5, 3.7), (0.8, 2.55, 2.7), (1.0, 2.3, 2.45)]
        } else {
            &[(0.0, 3.9, 4.1), (0.3, 3.65, 3.95), (0.75, 2.85, 3.0), (1.0, 2.5, 2.65)]
        };
        let t = limb(m, self.hip, self.knee, Vec3::Z, prof);
        let quad = ellipsoid(
            m,
            self.hip + vec3(0.3, -6.0, 1.3),
            if self.female { vec3(2.4, 4.6, 2.2) } else { vec3(2.8, 5.2, 2.5) },
        );
        smin(t, quad, 1.2)
    }

    pub fn shin(&self, m: Vec3) -> f32 {
        let prof: &[(f32, f32, f32)] = if self.female {
            &[(0.0, 2.3, 2.4), (0.3, 2.2, 2.5), (0.8, 1.5, 1.55), (1.0, 1.35, 1.4)]
        } else {
            &[(0.0, 2.6, 2.7), (0.28, 2.6, 2.9), (0.78, 1.85, 1.9), (1.0, 1.65, 1.7)]
        };
        let s = limb(m, self.knee, self.ankle, Vec3::Z, prof);
        let calf = ellipsoid(
            m,
            self.knee + vec3(0.1, -4.3, -1.0),
            if self.female { vec3(1.9, 3.2, 1.8) } else { vec3(2.25, 3.6, 2.05) },
        );
        let d = smin(s, calf, 1.2);
        smin(d, self.foot(m), 0.8)
    }

    pub fn foot(&self, m: Vec3) -> f32 {
        let k = if self.female { 0.88 } else { 1.0 };
        let a = self.ankle;
        let bottom = 0.45;
        let h = (a.y - bottom) * 0.5;
        let foot = rbox(
            m,
            vec3(a.x, bottom + h * 0.55, a.z + 1.5 * k),
            vec3(1.3 * k, h * 0.55, 3.3 * k),
            0.9 * k,
        );
        let heel = ellipsoid(m, vec3(a.x, bottom + 1.2, a.z - 1.2), vec3(1.25, 1.3, 1.3) * k);
        smin(smin(foot, heel, 0.8), cone(m, a, vec3(a.x, bottom + 1.0, a.z + 0.6), 1.5 * k, 1.3 * k), 0.8)
    }

    /// Each part's distance at `p` (the side `p` is on).
    pub fn parts(&self, p: Vec3) -> [f32; 5] {
        let m = mirror(p);
        [
            self.torso(p),
            self.arm_u(m),
            self.arm_l(m),
            self.thigh(m),
            self.shin(m),
        ]
    }

    /// The bare body: the parts blended where they join.
    pub fn body(&self, p: Vec3) -> f32 {
        let [t, au, al, th, sh] = self.parts(p);
        let arm = smin(au, al, 0.6);
        let leg = smin(th, sh, 0.9);
        smin(smin(t, arm, 0.9), leg, 1.8)
    }

    /// Which part a point belongs to (nearest part, with a little bias
    /// so joints cut where the limb starts).
    pub fn part_of(&self, p: Vec3) -> Bp {
        let d = self.parts(p);
        let bias = [0.0, -0.5, -0.15, 0.25, -0.2];
        let mut best = (Bp::Torso, f32::MAX);
        for (i, bp) in BPS.iter().enumerate() {
            let v = d[i] + bias[i];
            if v < best.1 {
                best = (*bp, v);
            }
        }
        // the hip: above the hip joint's line it is the pelvis
        if best.0 == Bp::Thigh && p.y > self.hip.y + 0.3 - (p.x.abs() - self.hip.x).max(0.0) * 0.6 {
            return Bp::Torso;
        }
        best.0
    }

    pub fn part_name(&self, p: Vec3) -> &'static str {
        let r = p.x >= 0.0;
        match (self.part_of(p), r) {
            (Bp::Torso, _) => "torso",
            (Bp::ArmU, true) => "arm_u.r",
            (Bp::ArmU, false) => "arm_u.l",
            (Bp::ArmL, true) => "arm_l.r",
            (Bp::ArmL, false) => "arm_l.l",
            (Bp::Thigh, true) => "thigh.r",
            (Bp::Thigh, false) => "thigh.l",
            (Bp::Shin, true) => "shin.r",
            (Bp::Shin, false) => "shin.l",
        }
    }

    // ------------------------------------------------------------ head

    /// Head landmarks relative to the pivot height.
    pub fn hy(&self, y: f32) -> f32 {
        self.head.y - 51.8 + y
    }

    pub fn cranium(&self, p: Vec3) -> f32 {
        let s = if self.female { 0.95 } else { 1.0 };
        ellipsoid(
            p,
            vec3(0.0, self.hy(57.2), -0.6 * s),
            vec3(3.95, 4.3, 4.6) * s,
        )
    }

    /// The face and skull, without eyes or ears.
    pub fn skull(&self, p: Vec3) -> f32 {
        let m = mirror(p);
        let f = self.female;
        let y = |v: f32| self.hy(v);
        let s = if f { 0.95 } else { 1.0 };
        let mut d = self.cranium(p);
        let jaw = if f {
            ellipsoid(p, vec3(0.0, y(54.5), 0.9), vec3(2.75, 2.3, 3.1))
        } else {
            ellipsoid(p, vec3(0.0, y(54.3), 1.0), vec3(3.25, 2.5, 3.3))
        };
        let chin = if f {
            ellipsoid(p, vec3(0.0, y(53.1), 2.85), vec3(1.0, 0.85, 0.95))
        } else {
            ellipsoid(p, vec3(0.0, y(52.85), 3.0), vec3(1.5, 1.0, 1.15))
        };
        let cheek = ellipsoid(
            m,
            vec3(2.3 * s, y(if f { 56.2 } else { 55.9 }), 2.55 * s),
            vec3(1.35, 1.0, 1.3) * s,
        );
        let brow = if f {
            cone(m, vec3(0.3, y(57.75), 3.55), vec3(2.3, y(57.95), 3.1), 0.45, 0.35)
        } else {
            cone(m, vec3(0.2, y(57.55), 3.55), vec3(2.45, y(57.7), 3.25), 0.8, 0.6)
        };
        let nose = smin(
            cone(
                p,
                vec3(0.0, y(57.2), 4.05 * s),
                vec3(0.0, y(55.35), if f { 5.05 } else { 5.3 }),
                if f { 0.32 } else { 0.42 },
                if f { 0.48 } else { 0.62 },
            ),
            ellipsoid(
                p,
                vec3(0.0, y(55.3), if f { 4.6 } else { 4.8 }),
                if f { vec3(0.7, 0.4, 0.5) } else { vec3(0.88, 0.46, 0.55) },
            ),
            0.35,
        );
        let lips = ellipsoid(
            p,
            vec3(0.0, y(54.1), 3.95 * s),
            if f { vec3(1.15, 0.6, 0.6) } else { vec3(1.25, 0.5, 0.55) },
        );
        d = smin(d, jaw, 1.6);
        d = smin(d, chin, 0.9);
        d = smin(d, cheek, 1.0);
        d = smin(d, brow, 0.7);
        d = smin(d, nose, 0.5);
        d = smin(d, lips, 0.5);
        // the eye sockets
        let socket = ellipsoid(m, vec3(1.55 * s, y(56.85), 4.2 * s), vec3(0.95, 0.62, 0.7) * s);
        d = smax(d, -socket, 0.35);
        let neck = cone(
            p,
            vec3(0.0, self.head.y - 2.6, -0.3),
            vec3(0.0, self.head.y + 1.6, 0.0),
            if f { 1.95 } else { 2.4 },
            if f { 1.85 } else { 2.25 },
        );
        smin(d, neck, 1.2)
    }

    /// The eyeballs (right one at +x).
    pub fn eye_centre(&self, right: bool) -> Vec3 {
        let s = if self.female { 0.95 } else { 1.0 };
        vec3(if right { 1.55 } else { -1.55 } * s, self.hy(56.85), 3.5 * s)
    }

    pub fn eye_r(&self) -> f32 {
        if self.female { 0.73 } else { 0.75 }
    }

    pub fn eyes(&self, p: Vec3) -> f32 {
        let m = mirror(p);
        sphere(m, self.eye_centre(true), self.eye_r())
    }

    pub fn head(&self, p: Vec3) -> f32 {
        self.skull(p).min(self.eyes(p))
    }
}
