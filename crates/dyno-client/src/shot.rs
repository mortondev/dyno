//! Headless rendering: join a server, render one frame off-screen, save it.
//! Used by `dyno shot` for scripted tests and AI agents.

use crate::game::{self, Player};
use crate::light::Light;
use crate::models;
use crate::net::{self, NetEvent};
use crate::render::Gfx;
use dyno_protocol::{ClientMsg, Pose, ServerMsg};
use dyno_world::{self as w, World};
use glam::Vec3;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct ShotOptions {
    pub addr: String,
    pub name: String,
    /// Feet position. Default: the world spawn.
    pub pos: Option<[f32; 3]>,
    pub yaw: f32,
    pub pitch: f32,
    /// Time of day. Default: the server time.
    pub time: Option<f32>,
    pub width: u32,
    pub height: u32,
    pub out: PathBuf,
    /// "none" for the world only; "hud", "pack", "dig" or "dead" to draw the UI
    pub ui: String,
    /// join as this player instead of as an invisible observer
    pub player: Option<String>,
    /// camera distance behind the player (0: first person). Default: the game's.
    pub zoom: Option<f32>,
    /// render a front-end page instead: title, realms, select, class, look, name, loading
    pub front: Option<String>,
    /// stand a figure on the ground and frame it with the play camera
    pub stand: Option<Stand>,
}

/// The camera of the gameplay spec (docs/design/scale.md): this far behind
/// the character, looking this many degrees down, with this vertical field
/// of view at 16:9.
pub const STAND_ZOOM: f32 = 9.0;
pub const STAND_PITCH: f32 = 15.0;
pub const STAND_FOV: f32 = 75.0;

/// A stand-in player on the ground, facing a landmark, seen as the player
/// behind it would see the place (`dyno shot --stand`).
#[derive(Clone, Copy, Debug)]
pub struct Stand {
    /// where the figure stands: x, z, and a height to stand near (None:
    /// the open ground of the column)
    pub at: [f32; 2],
    pub near_y: Option<f32>,
    /// the point it faces
    pub face: [f32; 3],
    /// degrees the camera looks down
    pub pitch: f32,
    /// `at`, `near_y` and `face` are measured from the world's hearth, as
    /// content positions are
    pub from_hearth: bool,
}

/// The yaw that looks from `from` toward `to` (0 faces -z, as the player's).
pub fn yaw_toward(from: [f32; 2], to: [f32; 2]) -> f32 {
    (from[0] - to[0]).atan2(from[1] - to[1])
}

/// The height of the feet of someone standing in column (x, z): nearest to
/// `near_y` when given (under a roof, in a cave), else on the open ground.
/// None where there is nowhere to stand.
pub fn standing_y(world: &World, x: i32, z: i32, near_y: Option<f32>) -> Option<f32> {
    let free = |y: i32| {
        let b = world.get(x, y, z);
        !w::is_solid(b) && b != w::WATER
    };
    let stands = |y: i32| {
        let below = world.get(x, y - 1, z);
        w::is_solid(below) && !w::is_leaves(below) && free(y) && free(y + 1)
    };
    match near_y {
        Some(n) => {
            let n = n.round() as i32;
            (0..=8)
                .flat_map(|d| [n - d, n + d])
                .find(|y| stands(*y))
                .map(|y| y as f32)
        }
        None => {
            let y = world.ground(x, z) + 1;
            stands(y).then_some(y as f32)
        }
    }
}

/// The play camera behind a character: where it ends up and how far back
/// it got.
#[derive(Clone, Copy, Debug)]
pub struct Chase {
    pub eye: Vec3,
    pub dir: Vec3,
    /// metres behind the character's eye; less than the zoom asked for when
    /// something solid stopped it
    pub reached: f32,
}

/// The game's third-person camera for a character with its feet at `feet`:
/// `zoom` metres behind the eyes along the view, pulled in short of walls
/// exactly as play does it.
pub fn chase(world: &World, feet: Vec3, yaw: f32, pitch: f32, zoom: f32) -> Chase {
    let mut p = Player::new(feet.into());
    p.yaw = yaw;
    p.pitch = pitch;
    let dir = p.look_dir();
    let eye = crate::app::pull_back(world, p.eye(), dir, zoom);
    // pull_back lifts the camera 8% of the way it went
    let reached = (p.eye() - eye).dot(dir) / (1.0 - 0.08 * dir.y);
    Chase { eye, dir, reached }
}

/// The stand-in's look: the default body in the first class's colour and
/// starter kit, all from the content the server sent.
pub fn stand_in_look(c: &dyno_world::content::Content) -> models::Look {
    let class = c.classes.first();
    let mut look = models::Look::from_appearance(
        &Default::default(),
        class.map(|c| c.color).unwrap_or([0.5, 0.5, 0.6]),
    );
    let Some(class) = class else {
        return look;
    };
    look.wear(class.starter.iter().map(|(k, _)| k.as_str()));
    look.weapon = class
        .starter
        .iter()
        .find_map(|(k, _)| models::weapon_look(c, c.item_by_key(k)));
    look
}

/// Returns a one-line description of what was rendered.
pub fn shot(o: ShotOptions) -> Result<String, String> {
    crate::assets::load_local_pack();
    if let Some(page) = &o.front {
        let mut g = Gfx::new_headless(o.width, o.height)?;
        return crate::app::shot_front(&mut g, page, &o.addr, o.player.as_deref(), &o.out);
    }
    if let Some(st) = o.stand {
        // an observer: the world streams around where the figure stands
        let rel = st.from_hearth;
        let j = net::connect_with(
            &o.addr,
            &o.name,
            net::JoinOptions {
                observer: true,
                view_at: (!rel).then_some([st.at[0], st.near_y.unwrap_or(0.0), st.at[1]]),
                ..Default::default()
            },
            Duration::from_secs(10),
        )?;
        let h = if rel {
            j.hearth.map(|v| v as f32)
        } else {
            [0.0; 3]
        };
        let at = [st.at[0] + h[0], st.at[1] + h[2]];
        let near_y = st.near_y.map(|y| y + h[1]);
        let face = [st.face[0] + h[0], st.face[1] + h[1], st.face[2] + h[2]];
        // the height is settled once the ground has streamed in
        let pos = [at[0], near_y.unwrap_or(-1.0), at[1]];
        if rel {
            // the hearth is known only now: move the view there
            let _ = j.conn.tx.send(ClientMsg::ViewAt {
                pos: [at[0], near_y.unwrap_or(0.0), at[1]],
            });
        }
        let mut g = Gfx::new_headless(o.width, o.height)?;
        return crate::app::shot_with_ui(
            j,
            &mut g,
            Some(pos),
            yaw_toward(at, [face[0], face[2]]),
            -st.pitch.to_radians(),
            o.time,
            "stand",
            o.out,
            Some(o.zoom.unwrap_or(STAND_ZOOM)),
        );
    }
    if o.ui != "none" || o.player.is_some() {
        let name = o.player.clone().unwrap_or_else(|| o.name.clone());
        let observer = o.player.is_none();
        // the world streams round the camera (or where a camera flight starts)
        let view_at = crate::app::shot_run()
            .map(|r| [r.0[0], 40.0, r.0[1]])
            .or(o.pos);
        let joining = Instant::now();
        let j = net::connect_with(
            &o.addr,
            &name,
            net::JoinOptions {
                token: crate::settings::tool_token(&name),
                observer,
                view_at,
                prepare_sections: true,
                ..Default::default()
            },
            Duration::from_secs(10),
        )?;
        if std::env::var_os("DYNO_SHOT_STATS").is_some() {
            eprintln!(
                "join: {} ms, {} KB of the world before play ({} sections held, {} needed), {} KB of files",
                joining.elapsed().as_millis(),
                j.world_bytes / 1024,
                j.world.held_sections().0,
                net::sections_around(
                    &j.world,
                    view_at.unwrap_or(j.spawn),
                    net::LOAD_RADIUS.min(j.view)
                )
                .len(),
                j.downloaded / 1024
            );
        }
        let mut g = Gfx::new_headless(o.width, o.height)?;
        let ui = if o.ui == "none" { "hud" } else { &o.ui };
        return crate::app::shot_with_ui(
            j, &mut g, o.pos, o.yaw, o.pitch, o.time, ui, o.out, o.zoom,
        );
    }
    // the world streams in around the camera
    let mut j = net::connect_with(
        &o.addr,
        &o.name,
        net::JoinOptions {
            observer: true,
            view_at: o.pos,
            ..Default::default()
        },
        Duration::from_secs(10),
    )?;
    let mut server_time = 0.25;
    let mut poses: Vec<Pose> = j.players.iter().map(|p| p.pose).collect();
    // until the rest of the view has streamed in (it goes quiet)
    let started = Instant::now();
    let mut last_section = Instant::now();
    while started.elapsed() < Duration::from_millis(400)
        || (last_section.elapsed() < Duration::from_millis(250)
            && started.elapsed() < Duration::from_secs(4))
    {
        match j.conn.rx.recv_timeout(Duration::from_millis(50)) {
            Ok(NetEvent::Msg(
                m @ (ServerMsg::Sections(_)
                | ServerMsg::SectionsGone(_)
                | ServerMsg::Block { .. }
                | ServerMsg::Fill { .. }),
            )) => {
                net::apply_world(&mut j.world, &m);
                last_section = Instant::now();
            }
            Ok(NetEvent::Msg(ServerMsg::Time { time, .. })) => server_time = time,
            Ok(NetEvent::Msg(ServerMsg::Poses(list))) => {
                poses = list.into_iter().map(|(_, p)| p).collect()
            }
            Ok(NetEvent::Closed(r)) => return Err(r),
            _ => {}
        }
    }
    let light = Light::new(&j.world);
    let mut g = Gfx::new_headless(o.width, o.height)?;
    let stats = std::env::var_os("DYNO_SHOT_STATS").is_some();
    let load_start = Instant::now();
    g.set_focus(Vec3::from(o.pos.unwrap_or(j.spawn)));
    g.load_world(&j.world, &light);
    let load_ms = load_start.elapsed().as_secs_f64() * 1e3;
    let t = o.time.unwrap_or(server_time);
    let mut player = Player::new(o.pos.unwrap_or(j.spawn));
    player.yaw = o.yaw;
    player.pitch = o.pitch;
    // the sky of the place: its mood is the pack's (mood.rs)
    crate::mood::set_world(j.hearth, &j.content.areas);
    let sky = crate::sky::sky_at(t, Some(player.eye()));
    let far = 160.0;
    let vp = player.view_proj(o.width as f32 / o.height as f32, 75.0, far);
    let mut boxes = Vec::new();
    for (i, pose) in poses.iter().enumerate() {
        let base = Vec3::from(pose.pos);
        let bl = light.get(
            base.x.floor() as i32,
            base.y.floor() as i32 + 1,
            base.z.floor() as i32,
        ) as f32
            / 15.0;
        models::push_avatar(
            &mut boxes,
            base,
            pose.yaw,
            pose.pitch,
            game::player_color(i as u32 + 1),
            bl,
            0.8,
            0.6,
        );
    }
    g.water.look_from(&j.world, player.eye().into(), j.hearth);
    g.screenshot = Some(o.out.clone());
    let ctx = crate::theme::new_context();
    let out = ctx.run(egui::RawInput::default(), |_| {});
    g.frame(Some((vp, player.eye(), far)), &sky, &boxes, &[], &ctx, out);
    if stats {
        // frame time on the headless path: the world only, GPU waited on
        g.wait_idle();
        let n = 30;
        let t0 = Instant::now();
        for _ in 0..n {
            let o = ctx.run(egui::RawInput::default(), |_| {});
            g.frame(Some((vp, player.eye(), far)), &sky, &boxes, &[], &ctx, o);
            g.wait_idle();
        }
        let frame_ms = t0.elapsed().as_secs_f64() * 1e3 / n as f64;
        let (vo, vt) = g.world_vertices();
        eprintln!(
            "stats: load_world {load_ms:.0} ms, frame {frame_ms:.2} ms, world vertices {vo} opaque + {vt} translucent"
        );
        let (gv, levels) = g.ground_stats();
        eprintln!("ground: {gv} vertices; sections by voxels a metre {levels:?}");
    }
    Ok(format!(
        "rendered {}x{} at {:?} yaw {} pitch {} time {:.2} with {} other players to {}",
        o.width,
        o.height,
        player.pos,
        o.yaw,
        o.pitch,
        t,
        poses.len(),
        o.out.display()
    ))
}
