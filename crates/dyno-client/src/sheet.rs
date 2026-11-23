//! Animation contact sheets: every kind of figure in its key poses, drawn
//! off-screen from a third-person camera, for `dyno shot --anim-sheet`.

use crate::anim::{Anim, Animator};
use crate::models::{self, Look};
use crate::render::{self, Gfx, Vertex};
use dyno_world::content::{Content, Model, WeaponKind};
use glam::Vec3;
use std::f32::consts::FRAC_PI_2;
use std::path::{Path, PathBuf};

/// A figure on the sheet.
enum Figure {
    Person(Look),
    Creature(Model, u8, [f32; 3]),
}

impl Figure {
    fn skeleton(&self) -> String {
        match self {
            Figure::Person(_) => "biped".into(),
            Figure::Creature(m, v, _) => models::creature_skeleton(*m, *v).unwrap_or_default(),
        }
    }

    fn attack(&self) -> Anim {
        match self {
            Figure::Person(l) => crate::anim::attack_for(models::grip(l)),
            Figure::Creature(m, v, _) => models::creature_attack(*m, *v),
        }
    }

    fn draw(&self, out: &mut Vec<Vertex>, a: &mut Animator, now: f32) {
        let yaw = 0.0;
        match self {
            Figure::Person(l) => {
                models::push_person_anim(out, Vec3::ZERO, yaw, 0.0, l, 0.0, a, now)
            }
            Figure::Creature(m, v, c) => {
                models::push_creature(out, *m, *v, Vec3::ZERO, yaw, *c, 0.0, 0.0, 1.0, a, now)
            }
        }
    }
}

/// The figures: players with each kind of weapon, then every creature
/// model in the content (one per look).
fn figures(content: &Content) -> Vec<(String, Figure)> {
    let mut list = Vec::new();
    for (name, seed, color, weapon, robe) in [
        (
            "Warrior (sword)",
            3,
            [0.62, 0.18, 0.16],
            Some(WeaponKind::Sword),
            false,
        ),
        (
            "Mage (staff, robe)",
            9,
            [0.22, 0.34, 0.78],
            Some(WeaponKind::Staff),
            true,
        ),
        (
            "Hunter (bow)",
            5,
            [0.25, 0.55, 0.25],
            Some(WeaponKind::Bow),
            false,
        ),
        ("Brawler (unarmed)", 12, [0.70, 0.55, 0.20], None, false),
    ] {
        let mut l = Look::player(seed, color);
        l.weapon = weapon.map(|k| (k, [0.62, 0.63, 0.68]));
        if robe {
            l.set("robe", 1);
            l.colors.remove("cape");
        }
        list.push((name.to_string(), Figure::Person(l)));
    }
    let mut seen = Vec::new();
    for c in &content.creatures {
        if seen.contains(&(c.model, c.variant)) {
            continue;
        }
        seen.push((c.model, c.variant));
        list.push((
            c.name.clone(),
            Figure::Creature(c.model, c.variant, c.color),
        ));
    }
    list
}

/// When to show an animation: a one-shot at its most telling moment, a
/// loop a little way in.
fn moment(a: Anim) -> f32 {
    let at = |u: f32| models::duration(a).unwrap_or(1.0) * u;
    match a {
        Anim::Attack1H => at(0.4),
        Anim::AttackUnarmed => at(0.45),
        Anim::Attack2H => at(0.5),
        Anim::Shoot => at(0.6),
        Anim::Special | Anim::Wave | Anim::Bow | Anim::Cheer | Anim::Point => at(0.45),
        Anim::JumpStart => at(0.5),
        Anim::JumpLand => at(0.2),
        Anim::CastRelease => at(0.22),
        Anim::HitReact => at(0.15),
        Anim::Death => at(0.72),
        _ => 1.05,
    }
}

/// One cell: a figure frozen in `anim`.
fn cell(fig: &Figure, anim: Anim) -> Vec<Vertex> {
    cell_at(fig, anim, moment(anim))
}

/// A figure frozen `t` seconds into `anim`.
fn cell_at(fig: &Figure, anim: Anim, t: f32) -> Vec<Vertex> {
    // a stride with the legs well apart
    let mut a = Animator::still(anim, FRAC_PI_2 + 0.15 + t * 6.0);
    let mut out = Vec::new();
    fig.draw(&mut out, &mut a, t);
    out
}

fn bounds(v: &[Vertex]) -> (Vec3, Vec3) {
    v.iter().fold(
        (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
        |(lo, hi), v| (lo.min(v.pos.into()), hi.max(v.pos.into())),
    )
}

/// A row of the sheet: its figure's name, and labelled cells.
type Row = (String, Vec<(String, Vec<Vertex>)>);

/// Render a grid of cells: one row per figure, one column per animation.
/// Each row shares one camera so poses compare at the same size.
fn grid(
    g: &mut Gfx,
    rows: &[Row],
    (cw, ch): (u32, u32),
    scratch: &Path,
) -> Result<(u32, u32, Vec<u8>), String> {
    let cols = rows.iter().map(|r| r.1.len()).max().unwrap_or(0) as u32;
    let (w, h) = (cw * cols, ch * rows.len() as u32);
    let mut img = vec![0u8; (w * h * 3) as usize];
    let sky = crate::app::sky(0.32);
    // a third-person camera: above, and a quarter round from the front
    let az = 35f32.to_radians();
    let el = 22f32.to_radians();
    let back = Vec3::new(az.sin() * el.cos(), el.sin(), -az.cos() * el.cos());
    let fov = 30f32.to_radians();
    let tmp = scratch.join("anim-sheet-cell.ppm");
    for (r, (name, cells)) in rows.iter().enumerate() {
        let (lo, hi) = cells.iter().fold(
            (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
            |(lo, hi), (_, v)| {
                let (a, b) = bounds(v);
                (lo.min(a), hi.max(b))
            },
        );
        let lo = lo.max(Vec3::splat(-3.0));
        let hi = hi.min(Vec3::splat(4.0));
        let target = (lo + hi) * 0.5;
        let radius = ((hi - lo).length() * 0.5).max(0.6);
        let eye = target + back * (radius / (fov * 0.5).tan() * 0.85);
        let vp = glam::Mat4::perspective_rh(fov, cw as f32 / ch as f32, 0.05, 200.0)
            * glam::Mat4::look_at_rh(eye, target, Vec3::Y);
        for (c, (label, verts)) in cells.iter().enumerate() {
            let mut boxes = Vec::with_capacity(verts.len() + 64);
            render::push_box(
                &mut boxes,
                Vec3::new(-60.0, -0.5, -60.0),
                Vec3::new(60.0, 0.0, 60.0),
                [0.42, 0.50, 0.36],
                0.0,
            );
            boxes.extend_from_slice(verts);
            let ctx = egui::Context::default();
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(cw as f32, ch as f32),
                )),
                ..Default::default()
            };
            let text = if c == 0 {
                format!("{name}\n{label}")
            } else {
                label.clone()
            };
            let out = ctx.run(raw, |ctx| {
                let p = ctx.layer_painter(egui::LayerId::background());
                let font = egui::FontId::proportional(13.0);
                let galley = p.layout_no_wrap(text.clone(), font, egui::Color32::WHITE);
                let r = egui::Rect::from_min_size(egui::pos2(4.0, 4.0), galley.size()).expand(3.0);
                p.rect_filled(r, 3.0, egui::Color32::from_black_alpha(150));
                p.galley(egui::pos2(4.0, 4.0), galley, egui::Color32::WHITE);
            });
            g.screenshot = Some(tmp.clone());
            g.frame(Some((vp, eye, 200.0)), &sky, &boxes, &[], &ctx, out);
            let px = read_ppm(&tmp)?;
            for y in 0..ch {
                let src = &px[(y * cw * 3) as usize..((y + 1) * cw * 3) as usize];
                let at = (((r as u32 * ch + y) * w + c as u32 * cw) * 3) as usize;
                img[at..at + src.len()].copy_from_slice(src);
            }
        }
    }
    let _ = std::fs::remove_file(&tmp);
    Ok((w, h, img))
}

/// The pixels of a binary PPM written by the renderer.
fn read_ppm(path: &Path) -> Result<Vec<u8>, String> {
    let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    // header: P6, width, height, 255, each followed by whitespace
    let mut fields = 0;
    let mut i = 0;
    while fields < 4 && i < data.len() {
        while i < data.len() && data[i].is_ascii_whitespace() {
            i += 1;
        }
        while i < data.len() && !data[i].is_ascii_whitespace() {
            i += 1;
        }
        fields += 1;
    }
    Ok(data.get(i + 1..).unwrap_or_default().to_vec())
}

/// Write an image: PPM, or PNG through ImageMagick when the name asks for it.
fn save(path: &Path, (w, h, px): (u32, u32, Vec<u8>)) -> Result<PathBuf, String> {
    let png = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("png"));
    let ppm = if png {
        path.with_extension("ppm")
    } else {
        path.to_path_buf()
    };
    let mut data = format!("P6\n{w} {h}\n255\n").into_bytes();
    data.extend_from_slice(&px);
    std::fs::write(&ppm, data).map_err(|e| format!("{}: {e}", ppm.display()))?;
    if png {
        let ok = std::process::Command::new("magick")
            .arg(&ppm)
            .arg(path)
            .status()
            .is_ok_and(|s| s.success());
        if ok {
            let _ = std::fs::remove_file(&ppm);
            return Ok(path.to_path_buf());
        }
        eprintln!("magick not found: left a PPM at {}", ppm.display());
    }
    Ok(ppm)
}

/// Render the contact sheets: `out` holds every figure in the key poses
/// (idle, walk, run, attack, cast or special, hit, death mid-fall, dead);
/// beside it, `<name>-all` holds every animation for one figure of each
/// skeleton.
pub fn anim_sheet(out: &Path) -> Result<String, String> {
    crate::assets::load_local_pack();
    let content = Content::default_pack();
    let figs = figures(&content);
    let (cw, ch) = (240u32, 260u32);
    let mut g = Gfx::new_headless(cw, ch)?;
    models::set_view(None);
    let scratch = out
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let key = |f: &Figure| -> Vec<(String, Anim)> {
        let cast = match f {
            Figure::Person(_) => Anim::CastDirected,
            Figure::Creature(..) => Anim::Special,
        };
        vec![
            ("Idle".into(), Anim::Idle),
            ("Walk".into(), Anim::Walk),
            ("Run".into(), Anim::Run),
            (format!("{:?}", f.attack()), f.attack()),
            (format!("{cast:?}"), cast),
            ("HitReact".into(), Anim::HitReact),
            ("Death".into(), Anim::Death),
            ("Dead".into(), Anim::Dead),
        ]
    };
    let rows: Vec<_> = figs
        .iter()
        .map(|(name, f)| {
            let cells = key(f)
                .into_iter()
                .map(|(label, a)| (label, cell(f, a)))
                .collect();
            (name.clone(), cells)
        })
        .collect();
    let main = save(out, grid(&mut g, &rows, (cw, ch), &scratch)?)?;
    let mut written = vec![main.display().to_string()];
    // every animation, for one figure of each skeleton, 8 to a row
    let mut done = Vec::new();
    for (name, f) in &figs {
        let s = f.skeleton();
        let crab = matches!(f, Figure::Creature(Model::Quadruped, 2, _));
        if done.contains(&(s.clone(), crab)) {
            continue;
        }
        done.push((s, crab));
        let rows: Vec<_> = Anim::ALL
            .chunks(8)
            .map(|chunk| {
                let cells = chunk
                    .iter()
                    .map(|a| (a.name().to_string(), cell(f, *a)))
                    .collect();
                (name.clone(), cells)
            })
            .collect();
        let stem = out.file_stem().and_then(|s| s.to_str()).unwrap_or("sheet");
        let ext = out.extension().and_then(|s| s.to_str()).unwrap_or("ppm");
        let slug: String = name
            .chars()
            .take_while(|c| *c != '(')
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_lowercase();
        let path = out.with_file_name(format!("{stem}-all-{slug}.{ext}"));
        written.push(
            save(&path, grid(&mut g, &rows, (cw, ch), &scratch)?)?
                .display()
                .to_string(),
        );
    }
    // one-shots played out frame by frame, for checking the motion
    let mut rows = Vec::new();
    for (fig, anims) in [
        (
            0,
            &[
                Anim::Attack1H,
                Anim::Ready,
                Anim::CastRelease,
                Anim::Death,
                Anim::JumpStart,
            ][..],
        ),
        (1, &[Anim::Attack2H, Anim::CastOmni][..]),
        (2, &[Anim::Shoot][..]),
        (3, &[Anim::AttackUnarmed, Anim::HitReact][..]),
    ] {
        for a in anims {
            rows.push((fig, *a));
        }
    }
    for (i, (_, f)) in figs.iter().enumerate().skip(4) {
        rows.push((i, f.attack()));
        rows.push((i, Anim::Death));
    }
    let rows: Vec<_> = rows
        .into_iter()
        .map(|(i, a)| {
            let (name, f) = &figs[i];
            let d = models::duration(a).unwrap_or(2.0);
            let cells = (0..8)
                .map(|k| {
                    let t = d * k as f32 / 7.0;
                    (format!("{a:?} {t:.2}s"), cell_at(f, a, t))
                })
                .collect();
            (name.clone(), cells)
        })
        .collect();
    let stem = out.file_stem().and_then(|s| s.to_str()).unwrap_or("sheet");
    let ext = out.extension().and_then(|s| s.to_str()).unwrap_or("ppm");
    let path = out.with_file_name(format!("{stem}-motion.{ext}"));
    written.push(
        save(&path, grid(&mut g, &rows, (cw, ch), &scratch)?)?
            .display()
            .to_string(),
    );
    Ok(format!(
        "{} figures x 8 key poses, and every animation per skeleton: {}",
        figs.len(),
        written.join(", ")
    ))
}

/// A camera for one cell: where it is, what it looks at, its field of view.
#[derive(Clone, Copy)]
struct Cam {
    eye: Vec3,
    target: Vec3,
    fov: f32,
}

impl Cam {
    /// Looking at `target` from `dist` away, `az` degrees round from the
    /// figure's front and `el` degrees up.
    fn orbit(target: Vec3, dist: f32, az: f32, el: f32, fov: f32) -> Cam {
        let (az, el) = (az.to_radians(), el.to_radians());
        let back = Vec3::new(az.sin() * el.cos(), el.sin(), -az.cos() * el.cos());
        Cam {
            eye: target + back * dist,
            target,
            fov: fov.to_radians(),
        }
    }
}

/// Render one cell: `verts` on a ground plane from `cam`, labelled.
fn shoot(
    g: &mut Gfx,
    verts: &[Vertex],
    cam: Cam,
    (cw, ch): (u32, u32),
    label: &str,
    tmp: &Path,
) -> Result<Vec<u8>, String> {
    let sky = crate::app::sky(0.32);
    let vp = glam::Mat4::perspective_rh(cam.fov, cw as f32 / ch as f32, 0.02, 200.0)
        * glam::Mat4::look_at_rh(cam.eye, cam.target, Vec3::Y);
    let mut boxes = Vec::with_capacity(verts.len() + 64);
    render::push_box(
        &mut boxes,
        Vec3::new(-60.0, -0.5, -60.0),
        Vec3::new(60.0, 0.0, 60.0),
        [0.42, 0.50, 0.36],
        0.0,
    );
    boxes.extend_from_slice(verts);
    let ctx = egui::Context::default();
    let raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(cw as f32, ch as f32),
        )),
        ..Default::default()
    };
    let text = label.to_string();
    let out = ctx.run(raw, |ctx| {
        if text.is_empty() {
            return;
        }
        let p = ctx.layer_painter(egui::LayerId::background());
        let font = egui::FontId::proportional(13.0);
        let galley = p.layout_no_wrap(text.clone(), font, egui::Color32::WHITE);
        let r = egui::Rect::from_min_size(egui::pos2(4.0, 4.0), galley.size()).expand(3.0);
        p.rect_filled(r, 3.0, egui::Color32::from_black_alpha(150));
        p.galley(egui::pos2(4.0, 4.0), galley, egui::Color32::WHITE);
    });
    g.screenshot = Some(tmp.to_path_buf());
    g.frame(Some((vp, cam.eye, 200.0)), &sky, &boxes, &[], &ctx, out);
    read_ppm(tmp)
}

/// Paste cells into one image, `cols` to a row.
fn paste(cells: &[Vec<u8>], cols: usize, (cw, ch): (u32, u32)) -> (u32, u32, Vec<u8>) {
    let rows = cells.len().div_ceil(cols.max(1));
    let (w, h) = (cw * cols as u32, ch * rows as u32);
    let mut img = vec![0u8; (w * h * 3) as usize];
    for (i, px) in cells.iter().enumerate() {
        let (r, c) = ((i / cols) as u32, (i % cols) as u32);
        for y in 0..ch {
            let src = &px[(y * cw * 3) as usize..((y + 1) * cw * 3) as usize];
            let at = (((r * ch + y) * w + c * cw) * 3) as usize;
            img[at..at + src.len()].copy_from_slice(src);
        }
    }
    (w, h, img)
}

/// The outfit sheets: each item set in `figures.json`'s showcase on each
/// of its people, from the front, three-quarter and back, idle, ready and
/// mid-attack, and close up on the face. `out` gets an overview (every
/// set from the front and three-quarter); `<out>-<set>` one sheet per set.
pub fn outfit_sheet(out: &Path) -> Result<String, String> {
    crate::assets::load_local_pack();
    let sets = crate::figure::showcase();
    if sets.is_empty() {
        return Err("figures.json has no showcase".into());
    }
    let size = (300u32, 420u32);
    let mut g = Gfx::new_headless(size.0, size.1)?;
    models::set_view(None);
    let dir = out
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let tmp = dir.join("outfit-sheet-cell.ppm");
    let stem = out.file_stem().and_then(|s| s.to_str()).unwrap_or("outfits");
    let ext = out.extension().and_then(|s| s.to_str()).unwrap_or("ppm");
    let body = |az: f32| Cam::orbit(Vec3::new(0.0, 0.93, 0.0), 4.3, az, 6.0, 28.0);
    let face = |az: f32| Cam::orbit(Vec3::new(0.0, 1.6, 0.0), 1.25, az, 3.0, 28.0);
    let pose = |l: &Look, a: Anim, t: f32| -> Vec<Vertex> {
        let fig = Figure::Person(l.clone());
        cell_at(&fig, a, t)
    };
    let mut written = Vec::new();
    let mut overview = Vec::new();
    for (set, people) in &sets {
        let mut cells = Vec::new();
        for (who, l) in people {
            let attack = crate::anim::attack_for(models::grip(l));
            let idle = pose(l, Anim::Idle, 0.4);
            let shots = [
                (format!("{set}\n{who}\nfront"), idle.clone(), body(0.0)),
                ("three-quarter".to_string(), idle.clone(), body(38.0)),
                ("back".to_string(), idle.clone(), body(160.0)),
                ("ready".to_string(), pose(l, Anim::Ready, 1.0), body(30.0)),
                (format!("{attack:?}"), pose(l, attack, moment(attack)), body(55.0)),
                ("portrait".to_string(), idle.clone(), face(0.0)),
                ("portrait three-quarter".to_string(), idle.clone(), face(32.0)),
            ];
            for (i, (label, verts, cam)) in shots.into_iter().enumerate() {
                let px = shoot(&mut g, &verts, cam, size, &label, &tmp)?;
                if i < 2 {
                    overview.push(px.clone());
                }
                cells.push(px);
            }
        }
        let slug: String = set
            .chars()
            .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        let path = out.with_file_name(format!("{stem}-{slug}.{ext}"));
        written.push(save(&path, paste(&cells, 7, size))?.display().to_string());
    }
    // levels of detail: the first person of each of the first sets, as
    // drawn from 2, 7, 18 and 45 blocks away (seen from up close)
    let mut lods = Vec::new();
    for (set, people) in sets.iter().take(4) {
        let Some((_, l)) = people.first() else {
            continue;
        };
        for d in [2.0f32, 7.0, 18.0, 45.0] {
            models::set_view(Some((Vec3::new(0.0, 1.6, -d), Vec3::new(0.4, 0.8, -0.3))));
            let verts = pose(l, Anim::Idle, 0.4);
            models::set_view(None);
            let label = format!("{set}\nas from {d} blocks: {} vertices", verts.len());
            lods.push(shoot(&mut g, &verts, body(20.0), size, &label, &tmp)?);
        }
    }
    let path = out.with_file_name(format!("{stem}-lods.{ext}"));
    written.push(save(&path, paste(&lods, 4, size))?.display().to_string());
    let main = save(out, paste(&overview, 8, size))?;
    let _ = std::fs::remove_file(&tmp);
    written.insert(0, main.display().to_string());
    Ok(format!("{} sets: {}", sets.len(), written.join(", ")))
}
