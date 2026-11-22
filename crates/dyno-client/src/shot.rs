//! Headless rendering: join a server, render one frame off-screen, save it.
//! Used by `dyno shot` for scripted tests and AI agents.

use crate::game::{self, Player};
use crate::light::Light;
use crate::net::{self, NetEvent};
use crate::render::{self, Gfx};
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
}

/// Returns a one-line description of what was rendered.
pub fn shot(o: ShotOptions) -> Result<String, String> {
    let j = net::connect(&o.addr, &o.name, Duration::from_secs(10))?;
    let mut server_time = 0.25;
    let mut poses: Vec<Pose> = j.players.iter().map(|p| p.pose).collect();
    let until = Instant::now() + Duration::from_millis(400);
    while Instant::now() < until {
        match j.conn.rx.recv_timeout(Duration::from_millis(50)) {
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
    g.load_world(&j.world, &light);
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
        render::push_avatar(
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
