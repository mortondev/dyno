//! Headless rendering: join a server, render one frame off-screen, save it.
//! Used by `dyno shot` for scripted tests and AI agents.

use crate::game::{self, Player};
use crate::light::Light;
use crate::models;
use crate::net::{self, NetEvent};
use crate::render::Gfx;
use dyno_protocol::{Pose, ServerMsg};
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
}

/// Returns a one-line description of what was rendered.
pub fn shot(o: ShotOptions) -> Result<String, String> {
    crate::assets::load_local_pack();
    if let Some(page) = &o.front {
        let mut g = Gfx::new_headless(o.width, o.height)?;
        return crate::app::shot_front(&mut g, page, &o.addr, o.player.as_deref(), &o.out);
    }
    if o.ui != "none" || o.player.is_some() {
        let name = o.player.clone().unwrap_or_else(|| o.name.clone());
        let observer = o.player.is_none();
        // an observer streams the world around the camera; a player sees
        // what streams around their character
        let j = net::connect_with(
            &o.addr,
            &name,
            net::JoinOptions {
                token: crate::settings::tool_token(&name),
                observer,
                view_at: o.pos,
                ..Default::default()
            },
            Duration::from_secs(10),
        )?;
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
    let sky = crate::app::sky(t);
    let mut player = Player::new(o.pos.unwrap_or(j.spawn));
    player.yaw = o.yaw;
    player.pitch = o.pitch;
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
    g.screenshot = Some(o.out.clone());
    let ctx = egui::Context::default();
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
