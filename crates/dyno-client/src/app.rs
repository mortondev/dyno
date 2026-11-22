//! Window, input, screens (connect, joining, playing) and the HUD.

use crate::audio::{Audio, Sfx};
use crate::game::{self, Input, Player, Remote, Remotes, Target};
use crate::light::Light;
use crate::net::{self, Joined, NetEvent};
use crate::render::{self, Gfx, Vertex};
use crate::settings::Settings;
use crate::survival::{self, PackCmd, PackView};
use dyno_protocol::{
    self as proto, ClientMsg, Content, EntityKind, EnvDamage, Mode, PlayerId, ServerMsg, SlotRef,
    Stack,
};
use dyno_world::{self as w, World};
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Stroke, Vec2};
use egui_winit::winit;
use glam::Vec3;
use std::collections::HashMap;
use std::collections::HashSet;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{
    DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

const ACCENT: Color32 = Color32::from_rgb(240, 138, 60);

pub fn run(prefill: Option<String>, update_note: String) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| format!("cannot open a window: {e}"))?;
    let mut settings = Settings::load();
    if let Some(a) = prefill {
        settings.last_server = a;
    }
    let mut app = App {
        menu: Menu {
            addr: settings.last_server.clone(),
            name: settings.name.clone(),
            error: None,
        },
        settings,
        gfx: None,
        audio: Audio::new(),
        egui_ctx: egui::Context::default(),
        egui_state: None,
        screen: Screen::Menu,
        host: None,
        last_frame: Instant::now(),
        update_note,
        fatal: None,
        autojoined: false,
        last_shot: Instant::now(),
    };
    app.audio.volume = app.settings.volume;
    event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
    if let Some(h) = app.host.take() {
        h.stop();
    }
    match app.fatal {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

struct Menu {
    addr: String,
    name: String,
    error: Option<String>,
}

enum Screen {
    Menu,
    Joining {
        rx: Receiver<Result<Joined, String>>,
        label: String,
        since: Instant,
    },
    Playing(Box<Play>),
}

struct Chat {
    at: Instant,
    from: Option<String>,
    text: String,
}

struct Play {
    world: World,
    conn: net::Connection,
    my_id: PlayerId,
    server_version: String,
    url: String,
    player: Player,
    remotes: Remotes,
    chat: Vec<Chat>,
    chat_open: bool,
    chat_text: String,
    focus_chat: bool,
    menu_open: bool,
    captured: bool,
    keys: HashSet<KeyCode>,
    slot: usize,
    last_pose: Instant,
    target: Option<Target>,
    fps: (Instant, u32, u32),
    light: Light,
    particles: Vec<Particle>,
    /// time of day at `time_at`, and day length in seconds
    time: f32,
    day_seconds: f32,
    time_at: Instant,
    swing: f32,
    bob: f32,
    step_acc: f32,
    fov_kick: f32,
    /// the pack screen (and creative item grid) is open
    picker_open: bool,
    content: Content,
    mode: Mode,
    pack: Vec<Option<Stack>>,
    health: f32,
    food: f32,
    fuel: u16,
    chest: Option<([i32; 3], Vec<Option<Stack>>)>,
    selected: Option<SlotRef>,
    recipe_filter: String,
    entities: HashMap<u32, ClientEntity>,
    /// block being broken: position, seconds so far, seconds needed
    dig: Option<([i32; 3], f32, f32)>,
    mouse_left: bool,
    dead: Option<String>,
    hurt_flash: f32,
    toasts: Vec<(Instant, String)>,
    fall_from: Option<f32>,
    air: f32,
    drown_clock: f32,
    sent_held: usize,
    /// night ambience: firefly positions and phases, and the next cricket
    fireflies: Vec<(Vec3, f32)>,
    next_cricket: f32,
    /// my recent single-block edits: (position, block before, block after)
    undo: Vec<([i32; 3], w::BlockId, w::BlockId)>,
    /// fill selection corners
    sel: (Option<[i32; 3]>, Option<[i32; 3]>),
}

struct ClientEntity {
    kind: EntityKind,
    from: [f32; 3],
    to: [f32; 3],
    at: Instant,
    yaw: f32,
    born: Instant,
}

impl ClientEntity {
    fn pos(&self) -> Vec3 {
        let t = (self.at.elapsed().as_secs_f32() / 0.1).clamp(0.0, 1.0);
        Vec3::from(self.from).lerp(Vec3::from(self.to), t)
    }
}

impl Play {
    fn held(&self) -> Option<Stack> {
        self.pack.get(self.slot).copied().flatten()
    }
    fn held_block(&self) -> Option<w::BlockId> {
        self.held().and_then(|s| self.content.item(s.item).block)
    }
    fn held_tool(&self) -> Option<&dyno_world::content::Tool> {
        self.held().and_then(|s| self.content.item(s.item).tool)
    }
    fn creative(&self) -> bool {
        self.mode == Mode::Creative
    }
    fn toast(&mut self, t: impl Into<String>) {
        self.toasts.push((Instant::now(), t.into()));
    }
}

struct Particle {
    pos: Vec3,
    vel: Vec3,
    color: [f32; 3],
    life: f32,
    size: f32,
}

fn spawn_particles(out: &mut Vec<Particle>, at: [i32; 3], block: w::BlockId, n: usize, seed: u32) {
    let color = w::block(block).color;
    let mut r = seed.wrapping_mul(2_654_435_761) | 1;
    let mut rnd = || {
        r ^= r << 13;
        r ^= r >> 17;
        r ^= r << 5;
        r as f32 / u32::MAX as f32
    };
    for _ in 0..n {
        let pos = Vec3::new(
            at[0] as f32 + 0.15 + rnd() * 0.7,
            at[1] as f32 + 0.15 + rnd() * 0.7,
            at[2] as f32 + 0.15 + rnd() * 0.7,
        );
        let vel = Vec3::new(rnd() - 0.5, rnd() * 0.8 + 0.4, rnd() - 0.5) * 4.0;
        let shade = 0.8 + rnd() * 0.3;
        out.push(Particle {
            pos,
            vel,
            color: [color[0] * shade, color[1] * shade, color[2] * shade],
            life: 0.5 + rnd() * 0.6,
            size: 0.07 + rnd() * 0.06,
        });
    }
}

/// Sky, sun and light for a time of day: 0.0 sunrise, 0.25 noon, 0.5 sunset, 0.75 midnight.
pub(crate) fn sky(t: f32) -> render::Sky {
    let a = t * std::f32::consts::TAU;
    let e = a.sin();
    let sun = Vec3::new(a.cos(), e, 0.3).normalize();
    let k = ((e + 0.1) / 0.42).clamp(0.0, 1.0);
    let k = k * k * (3.0 - 2.0 * k);
    let g = (-(e / 0.17).powi(2)).exp();
    let lerp = |a: [f32; 3], b: [f32; 3], t: f32| {
        [
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
        ]
    };
    let add = |a: [f32; 3], b: [f32; 3], t: f32| {
        [
            (a[0] + b[0] * t).min(1.0),
            (a[1] + b[1] * t).min(1.0),
            (a[2] + b[2] * t).min(1.0),
        ]
    };
    let mut top = lerp([0.02, 0.025, 0.07], [0.30, 0.52, 0.90], k);
    let mut horizon = lerp([0.05, 0.07, 0.14], [0.68, 0.81, 0.96], k);
    horizon = lerp(horizon, [0.98, 0.55, 0.30], g * 0.75);
    top = add(top, [0.22, 0.12, 0.28], g * 0.6);
    let mut light = lerp([0.30, 0.34, 0.50], [1.0, 0.99, 0.96], k);
    light = lerp(light, [1.0, 0.74, 0.52], g * 0.55);
    render::Sky {
        top,
        horizon,
        light,
        sun_dir: sun.into(),
        stars: 1.0 - k,
    }
}

fn material_of(b: w::BlockId) -> w::Material {
    w::block(b).material
}

struct Tunnel {
    child: Child,
    url: Arc<Mutex<Option<String>>>,
    error: Arc<Mutex<Option<String>>>,
}

struct Host {
    handle: dyno_server::Handle,
    port: u16,
    lan: Option<String>,
    tunnel: Option<Tunnel>,
}

impl Host {
    fn stop(mut self) {
        if let Some(t) = self.tunnel.as_mut() {
            let _ = t.child.kill();
        }
        self.handle.stop();
    }
}

struct App {
    audio: Audio,
    settings: Settings,
    menu: Menu,
    gfx: Option<Gfx>,
    egui_ctx: egui::Context,
    egui_state: Option<egui_winit::State>,
    screen: Screen,
    host: Option<Host>,
    last_frame: Instant,
    update_note: String,
    fatal: Option<String>,
    autojoined: bool,
    last_shot: Instant,
}

fn lan_ip() -> Option<String> {
    let s = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:80").ok()?;
    Some(s.local_addr().ok()?.ip().to_string())
}

fn world_path() -> std::path::PathBuf {
    if let Some(p) = std::env::var_os("DYNO_DATA_DIR") {
        return std::path::PathBuf::from(p).join("worlds/default/world.dynw");
    }
    directories::ProjectDirs::from("io", "mortondev", "dyno")
        .map(|d| d.data_dir().join("worlds/default/world.dynw"))
        .unwrap_or_else(|| "world/world.dynw".into())
}

fn start_tunnel(port: u16) -> Result<Tunnel, String> {
    let mut child = Command::new("cloudflared")
        .args(["tunnel", "--no-autoupdate", "--url", &format!("http://localhost:{port}")])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "cloudflared is not installed. Get it from developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/".to_string())?;
    let url = Arc::new(Mutex::new(None));
    let error = Arc::new(Mutex::new(None));
    for pipe in [
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let url = url.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            for line in std::io::BufReader::new(pipe).lines().map_while(Result::ok) {
                if let Some(i) = line.find("https://") {
                    let rest = &line[i..];
                    let end = rest
                        .find(|c: char| c.is_whitespace() || c == '|')
                        .unwrap_or(rest.len());
                    let u = &rest[..end];
                    if u.contains("trycloudflare.com") {
                        *url.lock().unwrap() = Some(u.to_string());
                    }
                }
            }
        });
    }
    Ok(Tunnel { child, url, error })
}

impl App {
    fn window(&self) -> Option<&Arc<Window>> {
        self.gfx.as_ref().and_then(|g| g.window.as_ref())
    }

    fn set_capture(&mut self, on: bool) {
        let Some(win) = self.window().cloned() else {
            return;
        };
        if let Screen::Playing(p) = &mut self.screen {
            if on {
                let ok = win
                    .set_cursor_grab(CursorGrabMode::Locked)
                    .or_else(|_| win.set_cursor_grab(CursorGrabMode::Confined));
                win.set_cursor_visible(false);
                p.captured = ok.is_ok() || true;
            } else {
                let _ = win.set_cursor_grab(CursorGrabMode::None);
                win.set_cursor_visible(true);
                p.captured = false;
                p.keys.clear();
            }
        }
    }

    fn start_join(&mut self, addr: String, remember: bool) {
        let name = proto::clean_name(&self.menu.name);
        self.menu.name = name.clone();
        self.settings.name = name.clone();
        if remember {
            self.settings.remember_server(&addr);
        }
        let _ = self.settings.save();
        let (tx, rx) = channel();
        let a = addr.clone();
        let token = self.settings.token;
        std::thread::spawn(move || {
            let _ = tx.send(net::connect(
                &a,
                &name,
                token,
                false,
                Duration::from_secs(10),
            ));
        });
        self.menu.error = None;
        self.screen = Screen::Joining {
            rx,
            label: addr,
            since: Instant::now(),
        };
    }

    fn start_host(&mut self) {
        if self.host.is_none() {
            let port = proto::DEFAULT_PORT;
            let cfg = dyno_server::Config {
                bind: std::net::SocketAddr::from(([0, 0, 0, 0], port)),
                world_path: world_path(),
                ..Default::default()
            };
            match dyno_server::start(cfg) {
                Ok(handle) => {
                    self.host = Some(Host {
                        handle,
                        port,
                        lan: lan_ip(),
                        tunnel: None,
                    })
                }
                Err(e) => {
                    self.menu.error = Some(format!(
                        "Cannot host on port {port}: {e}. Is another server running?"
                    ));
                    return;
                }
            }
        }
        let port = self
            .host
            .as_ref()
            .map(|h| h.port)
            .unwrap_or(proto::DEFAULT_PORT);
        self.start_join(format!("127.0.0.1:{port}"), false);
    }

    fn poll(&mut self) {
        // joining
        if let Screen::Joining { rx, .. } = &self.screen {
            match rx.try_recv() {
                Ok(Ok(j)) => {
                    let light = Light::new(&j.world);
                    let mut p = Play {
                        player: Player::new(j.spawn),
                        remotes: j
                            .players
                            .iter()
                            .filter(|i| i.id != j.your_id)
                            .map(|i| (i.id, Remote::new(i.name.clone(), i.pose)))
                            .collect(),
                        chat: vec![Chat {
                            at: Instant::now(),
                            from: None,
                            text: j.motd.clone(),
                        }],
                        world: j.world,
                        conn: j.conn,
                        my_id: j.your_id,
                        server_version: j.server_version,
                        url: j.url,
                        chat_open: false,
                        chat_text: String::new(),
                        focus_chat: false,
                        menu_open: false,
                        captured: false,
                        keys: HashSet::new(),
                        slot: 0,
                        last_pose: Instant::now(),
                        target: None,
                        fps: (Instant::now(), 0, 0),
                        light,
                        particles: Vec::new(),
                        time: 0.05,
                        day_seconds: 1200.0,
                        time_at: Instant::now(),
                        swing: 0.0,
                        bob: 0.0,
                        step_acc: 0.0,
                        fov_kick: 0.0,
                        picker_open: false,
                        fireflies: Vec::new(),
                        next_cricket: 3.0,
                        undo: Vec::new(),
                        sel: (None, None),
                        content: j.content,
                        mode: j.mode,
                        pack: vec![None; dyno_sim::SLOTS],
                        health: 20.0,
                        food: 20.0,
                        fuel: 0,
                        chest: None,
                        selected: None,
                        recipe_filter: String::new(),
                        entities: HashMap::new(),
                        dig: None,
                        mouse_left: false,
                        dead: None,
                        hurt_flash: 0.0,
                        toasts: Vec::new(),
                        fall_from: None,
                        air: 10.0,
                        drown_clock: 0.0,
                        sent_held: 99,
                    };
                    p.chat.push(Chat { at: Instant::now(), from: None, text: if p.creative() { "Creative mode. Click to play. WASD move, F fly, left click break, right click place, E all items, G G then V to fill, Ctrl+Z undo, T chat, /help.".into() } else { "Survival mode. Hold left click to break blocks, E opens your pack and recipes, right click places or eats, Q throws, T chat, /help. Punch a tree to start, and make light before night falls.".into() } });
                    if let Some(g) = self.gfx.as_mut() {
                        g.load_world(&p.world, &p.light);
                    }
                    self.screen = Screen::Playing(Box::new(p));
                    self.audio.play(Sfx::Join, 0.5);
                    self.set_capture(true);
                }
                Ok(Err(e)) => {
                    self.menu.error = Some(e);
                    self.screen = Screen::Menu;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(_) => self.screen = Screen::Menu,
            }
        }
        // network messages
        let mut closed = None;
        let mut release = false;
        if let Screen::Playing(p) = &mut self.screen {
            while let Ok(ev) = p.conn.rx.try_recv() {
                match ev {
                    NetEvent::Closed(r) => closed = Some(r),
                    NetEvent::Msg(m) => match m {
                        ServerMsg::Poses(list) => {
                            for (id, pose) in list {
                                if id == p.my_id {
                                    continue;
                                }
                                p.remotes
                                    .entry(id)
                                    .or_insert_with(|| Remote::new("?".into(), pose))
                                    .set(pose);
                            }
                        }
                        ServerMsg::PlayerJoined(info) => {
                            self.audio.play(Sfx::Join, 0.4);
                            if info.id != p.my_id {
                                p.remotes.insert(info.id, Remote::new(info.name, info.pose));
                            }
                        }
                        ServerMsg::PlayerLeft { id } => {
                            p.remotes.remove(&id);
                        }
                        ServerMsg::Block { pos, block } => {
                            let old = p.world.get(pos[0], pos[1], pos[2]);
                            if old != block {
                                p.world.set(pos[0], pos[1], pos[2], block);
                                p.light.changed(&p.world, pos);
                                let r = if w::emission(old) > 0
                                    || w::emission(block) > 0
                                    || p.light.get(pos[0], pos[1], pos[2]) > 0
                                {
                                    15
                                } else {
                                    1
                                };
                                if let Some(g) = self.gfx.as_mut() {
                                    g.block_changed(&p.world, &p.light, pos, r);
                                }
                                // another player changed it: show and play it, quieter with distance
                                let d = Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32)
                                    .distance(p.player.eye());
                                let gain = (1.0 - d / 32.0).clamp(0.0, 1.0);
                                if block == w::AIR {
                                    spawn_particles(
                                        &mut p.particles,
                                        pos,
                                        old,
                                        10,
                                        pos[0] as u32 ^ (pos[2] as u32) << 8,
                                    );
                                    self.audio.play(Sfx::Break(material_of(old)), gain);
                                } else {
                                    self.audio.play(Sfx::Place(material_of(block)), gain);
                                }
                            }
                        }
                        ServerMsg::Chat { from, text } => {
                            self.audio.play(Sfx::Chat, 0.6);
                            p.chat.push(Chat {
                                at: Instant::now(),
                                from: Some(from),
                                text,
                            })
                        }
                        ServerMsg::System { text } => p.chat.push(Chat {
                            at: Instant::now(),
                            from: None,
                            text,
                        }),
                        ServerMsg::Time { time, day_seconds } => {
                            p.time = time;
                            p.day_seconds = day_seconds.max(10.0);
                            p.time_at = Instant::now();
                        }
                        ServerMsg::Fill { a, b, block } => {
                            for y in a[1]..=b[1] {
                                for z in a[2]..=b[2] {
                                    for x in a[0]..=b[0] {
                                        p.world.set(x, y, z, block);
                                    }
                                }
                            }
                            p.light.changed_box(&p.world, a, b);
                            if let Some(g) = self.gfx.as_mut() {
                                g.region_changed(&p.world, &p.light, a, b, 15);
                            }
                            let c = Vec3::new(
                                (a[0] + b[0]) as f32 * 0.5,
                                (a[1] + b[1]) as f32 * 0.5,
                                (a[2] + b[2]) as f32 * 0.5,
                            );
                            let gain = (1.0 - c.distance(p.player.eye()) / 48.0).clamp(0.2, 1.0);
                            let m = if block == w::AIR {
                                Sfx::Break(w::Material::Stone)
                            } else {
                                Sfx::Place(material_of(block))
                            };
                            self.audio.play(m, gain);
                        }
                        ServerMsg::Teleport { pos } => {
                            p.player.pos = Vec3::from(pos);
                            p.player.vel = Vec3::ZERO;
                            p.fall_from = None;
                            p.dead = None;
                        }
                        ServerMsg::Pack(slots) => p.pack = slots,
                        ServerMsg::Status {
                            health,
                            food,
                            mode,
                            fuel,
                        } => {
                            p.health = health;
                            p.food = food;
                            p.fuel = fuel;
                            if mode != p.mode {
                                p.mode = mode;
                                if mode == Mode::Survival {
                                    p.player.flying = false;
                                }
                                p.toast(if mode == Mode::Creative {
                                    "Creative mode"
                                } else {
                                    "Survival mode"
                                });
                            }
                        }
                        ServerMsg::Container(c) => {
                            p.chest = c;
                            if p.chest.is_some() && !p.picker_open {
                                p.picker_open = true;
                                p.selected = None;
                                self.audio.play(Sfx::Place(w::Material::Wood), 0.5);
                                release = true;
                            }
                        }
                        ServerMsg::EntitySpawn { id, kind, pos, yaw } => {
                            p.entities.insert(
                                id,
                                ClientEntity {
                                    kind,
                                    from: pos,
                                    to: pos,
                                    at: Instant::now(),
                                    yaw,
                                    born: Instant::now(),
                                },
                            );
                        }
                        ServerMsg::EntityMoves(list) => {
                            for (id, pos, yaw) in list {
                                if let Some(e) = p.entities.get_mut(&id) {
                                    e.from = e.pos().into();
                                    e.to = pos;
                                    e.yaw = yaw;
                                    e.at = Instant::now();
                                }
                            }
                        }
                        ServerMsg::EntityGone { id, picked_by } => {
                            p.entities.remove(&id);
                            if picked_by == Some(p.my_id) {
                                self.audio.play(Sfx::Click, 0.7);
                            }
                        }
                        ServerMsg::EntityHurt { .. } => {}
                        ServerMsg::Hurt { amount, push } => {
                            p.hurt_flash = (0.4 + amount * 0.08).min(1.0);
                            p.player.vel += Vec3::from(push);
                            self.audio.play(Sfx::Break(w::Material::Cloth), 0.9);
                        }
                        ServerMsg::Died { message } => {
                            p.dead = Some(message);
                            p.dig = None;
                            p.picker_open = false;
                            release = true;
                        }
                        ServerMsg::Milestone { title } => {
                            p.toast(format!("Journal: {title}"));
                            self.audio.play(Sfx::Join, 0.7);
                        }
                        ServerMsg::Content(c) => p.content = *c,
                        ServerMsg::Digging { .. } => {}
                        ServerMsg::Reject { reason } => closed = Some(reason),
                        _ => {}
                    },
                }
            }
            if p.chat.len() > 200 {
                p.chat.drain(..100);
            }
        }
        if release {
            self.set_capture(false);
        }
        if let Some(r) = closed {
            self.set_capture(false);
            self.menu.error = Some(format!("Disconnected: {r}"));
            self.screen = Screen::Menu;
        }
    }

    fn frame(&mut self) {
        // DYNO_AUTOJOIN=1 joins the prefilled address at start (for scripted tests and agents)
        if matches!(self.screen, Screen::Menu)
            && std::env::var_os("DYNO_AUTOJOIN").is_some()
            && self.menu.error.is_none()
            && !self.autojoined
        {
            self.autojoined = true;
            if std::env::var("DYNO_AUTOJOIN").as_deref() == Ok("host") {
                self.start_host();
            } else {
                let a = self.menu.addr.clone();
                self.act(Action::Join(a));
            }
        }
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;
        self.poll();
        let Some(gfx) = self.gfx.as_mut() else { return };
        let (wdt, hgt) = (gfx.config.width as f32, gfx.config.height as f32);
        let fov = self.settings.fov_degrees;
        let far = self.settings.view_distance;

        // DYNO_SCREENSHOT=path saves the view every 3 seconds while playing (scripted tests, agents)
        if let (Screen::Playing(_), Some(path)) =
            (&self.screen, std::env::var_os("DYNO_SCREENSHOT"))
            && self.last_shot.elapsed() > Duration::from_secs(3)
        {
            self.last_shot = Instant::now();
            gfx.screenshot = Some(path.into());
        }
        // time of day (also drawn behind the menu)
        let sky_now = match &self.screen {
            Screen::Playing(p) => sky(p.time + p.time_at.elapsed().as_secs_f32() / p.day_seconds),
            _ => sky(0.12),
        };

        // simulate
        let mut view = None;
        let mut boxes: Vec<Vertex> = Vec::new();
        let mut lines: Vec<Vertex> = Vec::new();
        if let Screen::Playing(p) = &mut self.screen {
            let k = |c: KeyCode| p.keys.contains(&c);
            let active =
                p.captured && !p.chat_open && !p.menu_open && !p.picker_open && p.dead.is_none();
            let input = if active {
                Input {
                    forward: (k(KeyCode::KeyW) as i32 - k(KeyCode::KeyS) as i32) as f32,
                    right: (k(KeyCode::KeyD) as i32 - k(KeyCode::KeyA) as i32) as f32,
                    jump: k(KeyCode::Space),
                    down: k(KeyCode::ShiftLeft) || k(KeyCode::ShiftRight),
                    sprint: k(KeyCode::ControlLeft) || k(KeyCode::KeyR),
                }
            } else {
                Input::default()
            };
            let before = p.player.pos;
            if !p.creative() {
                p.player.flying = false;
            }
            p.player.update(&p.world, input, dt);
            p.hurt_flash = (p.hurt_flash - dt * 1.5).max(0.0);
            p.toasts
                .retain(|(t, _)| t.elapsed() < Duration::from_secs(5));
            if p.slot != p.sent_held {
                p.sent_held = p.slot;
                let _ = p.conn.tx.send(ClientMsg::Held { slot: p.slot as u8 });
            }
            // falls and drowning are measured here and reported
            if !p.creative() && p.dead.is_none() {
                let in_water = p.world.get(
                    p.player.pos.x.floor() as i32,
                    (p.player.pos.y + 0.3).floor() as i32,
                    p.player.pos.z.floor() as i32,
                ) == w::WATER;
                if p.player.on_ground || in_water {
                    if let Some(y0) = p.fall_from.take() {
                        let fell = y0 - p.player.pos.y;
                        if fell > 3.5 && !in_water {
                            let _ = p.conn.tx.send(ClientMsg::Hurt {
                                kind: EnvDamage::Fall,
                                amount: (fell - 3.0).floor() as u8,
                            });
                        }
                    }
                } else {
                    let y = p.player.pos.y;
                    p.fall_from = Some(p.fall_from.map_or(y, |f| f.max(y)));
                }
                let head_wet = p.world.get(
                    p.player.eye().x.floor() as i32,
                    p.player.eye().y.floor() as i32,
                    p.player.eye().z.floor() as i32,
                ) == w::WATER;
                if head_wet {
                    p.air -= dt;
                    if p.air < 0.0 {
                        p.drown_clock += dt;
                        if p.drown_clock >= 1.0 {
                            p.drown_clock = 0.0;
                            let _ = p.conn.tx.send(ClientMsg::Hurt {
                                kind: EnvDamage::Drown,
                                amount: 2,
                            });
                        }
                    }
                } else {
                    p.air = (p.air + dt * 4.0).min(10.0);
                }
            }
            // mining: hold the left button on a block
            let can_mine = active && p.mouse_left && !p.creative();
            let target_block = p.target.as_ref().map(|t| t.block);
            match (can_mine, target_block) {
                (true, Some(pos)) => {
                    let b = p.world.get(pos[0], pos[1], pos[2]);
                    let need = dyno_sim::mining::break_time(&p.content, b, p.held_tool());
                    match (&mut p.dig, need) {
                        (Some((dp, done, total)), Some(_)) if *dp == pos => {
                            *done += dt;
                            p.swing = p.swing.max(0.6);
                            if *done >= *total {
                                let pos = *dp;
                                p.dig = None;
                                let _ = p.conn.tx.send(ClientMsg::Dig { pos });
                                p.world.set(pos[0], pos[1], pos[2], w::AIR);
                                p.light.changed(&p.world, pos);
                                gfx.block_changed(&p.world, &p.light, pos, 15);
                                spawn_particles(
                                    &mut p.particles,
                                    pos,
                                    b,
                                    14,
                                    (pos[0] * 31 + pos[1] * 7 + pos[2]) as u32,
                                );
                                self.audio.play(Sfx::Break(material_of(b)), 0.9);
                            } else if (*done / 0.25).floor() != ((*done - dt) / 0.25).floor() {
                                self.audio.play(Sfx::Step(material_of(b)), 0.45);
                            }
                        }
                        (_, Some(total)) => {
                            p.dig = Some((pos, 0.0, total));
                            let _ = p.conn.tx.send(ClientMsg::DigStart { pos });
                        }
                        (_, None) => p.dig = None,
                    }
                }
                _ => {
                    p.dig = None;
                }
            }
            // footsteps, view bob and sprint field-of-view kick
            let moved =
                Vec3::new(p.player.pos.x - before.x, 0.0, p.player.pos.z - before.z).length();
            if p.player.on_ground && !p.player.flying {
                p.bob += moved * 2.1;
                p.step_acc += moved;
                if p.step_acc > 2.0 {
                    p.step_acc = 0.0;
                    let below = p.world.get(
                        p.player.pos.x.floor() as i32,
                        (p.player.pos.y - 0.2).floor() as i32,
                        p.player.pos.z.floor() as i32,
                    );
                    if below != w::AIR {
                        self.audio.play(Sfx::Step(material_of(below)), 0.5);
                    }
                }
            }
            let speed = moved / dt.max(1e-4);
            let kick_target = if input.sprint && speed > 4.0 {
                8.0
            } else {
                0.0
            };
            p.fov_kick += (kick_target - p.fov_kick) * (dt * 8.0).min(1.0);
            p.swing = (p.swing - dt * 4.0).max(0.0);
            if p.last_pose.elapsed() >= Duration::from_millis(50) {
                p.last_pose = Instant::now();
                let _ = p.conn.tx.send(ClientMsg::Pose(p.player.pose()));
            }
            p.target = game::raycast(&p.world, p.player.eye(), p.player.look_dir(), game::REACH);
            let light_at = |l: &Light, v: Vec3| {
                l.get(v.x.floor() as i32, v.y.floor() as i32, v.z.floor() as i32) as f32 / 15.0
            };
            let anim_t = p.fps.0.elapsed().as_secs_f32() + p.time_at.elapsed().as_secs_f32();
            for (id, r) in &p.remotes {
                let pose = r.current();
                let base = Vec3::from(pose.pos);
                let bl = light_at(&p.light, base + Vec3::Y);
                let walk = (r.speed() / 4.0).min(1.0);
                render::push_avatar(
                    &mut boxes,
                    base,
                    pose.yaw,
                    pose.pitch,
                    game::player_color(*id),
                    bl,
                    anim_t * 9.0,
                    walk,
                );
            }
            // night ambience: fireflies and crickets
            let night = sky_now.stars;
            if night > 0.5 {
                let t = p.fps.0.elapsed().as_secs_f32() + p.time_at.elapsed().as_secs_f32();
                let center = p.player.pos;
                let seed = (t * 1000.0) as u32;
                while p.fireflies.len() < 28 {
                    let k = p.fireflies.len() as u32 + seed;
                    let h =
                        |n: u32| ((n.wrapping_mul(2_654_435_761) >> 8) & 0xffff) as f32 / 65535.0;
                    let x = center.x + (h(k) - 0.5) * 28.0;
                    let z = center.z + (h(k ^ 0x55) - 0.5) * 28.0;
                    let ground = p.world.surface(x.floor() as i32, z.floor() as i32) as f32;
                    p.fireflies.push((
                        Vec3::new(x, ground + 1.2 + h(k ^ 0xaa) * 2.5, z),
                        h(k ^ 0x33) * std::f32::consts::TAU,
                    ));
                }
                p.fireflies.retain(|(pos, _)| pos.distance(center) < 22.0);
                for (pos, ph) in &mut p.fireflies {
                    *ph += dt;
                    *pos += Vec3::new(
                        (*ph * 0.7).sin(),
                        (*ph * 1.3).sin() * 0.5,
                        (*ph * 0.9).cos(),
                    ) * dt
                        * 0.6;
                    let glow = ((*ph * 2.3).sin() * 0.5 + 0.5) * night;
                    if glow > 0.15 {
                        render::push_oriented_box(
                            &mut boxes,
                            *pos,
                            [Vec3::X, Vec3::Y, Vec3::Z],
                            Vec3::splat(0.035),
                            [0.85 * glow + 0.1, 1.0 * glow + 0.1, 0.35 * glow],
                            true,
                            1.0,
                        );
                    }
                }
                p.next_cricket -= dt;
                if p.next_cricket <= 0.0 {
                    p.next_cricket = 2.0 + (t * 7.3).sin().abs() * 5.0;
                    self.audio.play(Sfx::Cricket, 0.25 * night);
                }
            } else {
                p.fireflies.clear();
            }
            // particles
            for pt in &mut p.particles {
                pt.vel.y -= 18.0 * dt;
                let next = pt.pos + pt.vel * dt;
                if w::is_solid(p.world.get(
                    next.x.floor() as i32,
                    next.y.floor() as i32,
                    next.z.floor() as i32,
                )) {
                    pt.vel *= -0.25;
                } else {
                    pt.pos = next;
                }
                pt.life -= dt;
                let s = pt.size * (pt.life * 2.0).clamp(0.0, 1.0);
                render::push_box(
                    &mut boxes,
                    pt.pos - Vec3::splat(s),
                    pt.pos + Vec3::splat(s),
                    pt.color,
                    light_at(&p.light, pt.pos),
                );
            }
            p.particles.retain(|pt| pt.life > 0.0);
            if let Some(t) = &p.target {
                let b = Vec3::new(t.block[0] as f32, t.block[1] as f32, t.block[2] as f32);
                let (lo, hi) = (b - Vec3::splat(0.002), b + Vec3::splat(1.002));
                let cs = [
                    Vec3::new(lo.x, lo.y, lo.z),
                    Vec3::new(hi.x, lo.y, lo.z),
                    Vec3::new(hi.x, lo.y, hi.z),
                    Vec3::new(lo.x, lo.y, hi.z),
                    Vec3::new(lo.x, hi.y, lo.z),
                    Vec3::new(hi.x, hi.y, lo.z),
                    Vec3::new(hi.x, hi.y, hi.z),
                    Vec3::new(lo.x, hi.y, hi.z),
                ];
                let edge = if sky_now.stars > 0.6 {
                    [0.9, 0.9, 0.9, 0.7]
                } else {
                    [0.02, 0.02, 0.02, 0.75]
                };
                for (a, b) in [
                    (0, 1),
                    (1, 2),
                    (2, 3),
                    (3, 0),
                    (4, 5),
                    (5, 6),
                    (6, 7),
                    (7, 4),
                    (0, 4),
                    (1, 5),
                    (2, 6),
                    (3, 7),
                ] {
                    for i in [a, b] {
                        lines.push(Vertex {
                            pos: cs[i].into(),
                            normal: [0.0; 3],
                            color: edge,
                            light: 1.0,
                            sky: 1.0,
                            pattern: 0.0,
                        });
                    }
                }
            }
            // fill selection box
            if let Some(a) = p.sel.0 {
                let b = p.sel.1.or(p.target.as_ref().map(|t| t.block)).unwrap_or(a);
                let lo = Vec3::new(
                    a[0].min(b[0]) as f32,
                    a[1].min(b[1]) as f32,
                    a[2].min(b[2]) as f32,
                ) - Vec3::splat(0.01);
                let hi = Vec3::new(
                    a[0].max(b[0]) as f32,
                    a[1].max(b[1]) as f32,
                    a[2].max(b[2]) as f32,
                ) + Vec3::splat(1.01);
                let cs = [
                    Vec3::new(lo.x, lo.y, lo.z),
                    Vec3::new(hi.x, lo.y, lo.z),
                    Vec3::new(hi.x, lo.y, hi.z),
                    Vec3::new(lo.x, lo.y, hi.z),
                    Vec3::new(lo.x, hi.y, lo.z),
                    Vec3::new(hi.x, hi.y, lo.z),
                    Vec3::new(hi.x, hi.y, hi.z),
                    Vec3::new(lo.x, hi.y, hi.z),
                ];
                for (i, j) in [
                    (0, 1),
                    (1, 2),
                    (2, 3),
                    (3, 0),
                    (4, 5),
                    (5, 6),
                    (6, 7),
                    (7, 4),
                    (0, 4),
                    (1, 5),
                    (2, 6),
                    (3, 7),
                ] {
                    for k in [i, j] {
                        lines.push(Vertex {
                            pos: cs[k].into(),
                            normal: [0.0; 3],
                            color: [1.0, 0.55, 0.15, 0.95],
                            light: 1.0,
                            sky: 1.0,
                            pattern: 0.0,
                        });
                    }
                }
            }
            // camera with view bob
            let dir = p.player.look_dir();
            let right = dir.cross(Vec3::Y).normalize_or_zero();
            let up = right.cross(dir);
            let bob_amt = if p.player.on_ground && !p.player.flying {
                (speed / 6.0).min(1.0)
            } else {
                0.0
            };
            let eye = p.player.eye()
                + Vec3::Y * (p.bob.sin().abs() * 0.06 * bob_amt)
                + right * ((p.bob * 0.5).sin() * 0.03 * bob_amt);
            let proj = glam::Mat4::perspective_rh(
                (fov + p.fov_kick).to_radians(),
                wdt / hgt.max(1.0),
                0.05,
                far,
            );
            let vp = proj * glam::Mat4::look_to_rh(eye, dir, Vec3::Y);
            // block in hand, with a swing when breaking or placing
            let sw = (p.swing * std::f32::consts::PI).sin();
            let held_item = p.held();
            let hand = eye + dir * (0.62 - sw * 0.12) + right * (0.36 - sw * 0.08)
                - up * (0.34 + sw * 0.1 - (p.bob * 0.5).cos().abs() * 0.015 * bob_amt);
            let (s1, c1) = (0.6f32 - sw * 0.8).sin_cos();
            let ax = [right * c1 + dir * s1, up, dir * c1 - right * s1];
            render::push_oriented_box(
                &mut boxes,
                hand,
                ax,
                Vec3::splat(0.11),
                held_item
                    .map(|s| p.content.item(s.item).color)
                    .unwrap_or([0.91, 0.76, 0.62]),
                held_item
                    .and_then(|s| p.content.item(s.item).block)
                    .is_some_and(|b| w::emission(b) > 0)
                    || held_item.is_some_and(|s| p.content.item(s.item).light > 0),
                light_at(&p.light, eye),
            );
            // items lying in the world: small spinning cubes
            for e in p.entities.values() {
                if let EntityKind::Item(s) = e.kind {
                    let it = p.content.item(s.item);
                    let pos = e.pos();
                    let t = e.born.elapsed().as_secs_f32();
                    let bob = (t * 2.5).sin() * 0.06 + 0.18;
                    let (sy, cy) = (e.yaw + t * 1.6).sin_cos();
                    let ax = [Vec3::new(cy, 0.0, -sy), Vec3::Y, Vec3::new(sy, 0.0, cy)];
                    let size = if it.block.is_some() { 0.13 } else { 0.1 };
                    let glow = it.block.is_some_and(|b| w::emission(b) > 0) || it.light > 0;
                    let n = if s.count > 1 { 2 } else { 1 };
                    for k in 0..n {
                        let off = Vec3::new(k as f32 * 0.08, k as f32 * 0.06, k as f32 * 0.05);
                        render::push_oriented_box(
                            &mut boxes,
                            pos + Vec3::Y * bob + off,
                            ax,
                            Vec3::new(
                                size,
                                size,
                                if it.block.is_some() {
                                    size
                                } else {
                                    size * 0.35
                                },
                            ),
                            it.color,
                            glow,
                            light_at(&p.light, pos),
                        );
                    }
                }
            }
            view = Some((vp, eye, far));
            p.fps.1 += 1;
            if p.fps.0.elapsed() >= Duration::from_secs(1) {
                p.fps = (Instant::now(), 0, p.fps.1);
            }
        }

        // UI
        let Some(state) = self.egui_state.as_mut() else {
            return;
        };
        let Some(window) = gfx.window.clone() else {
            return;
        };
        let raw = state.take_egui_input(&window);
        let ctx = self.egui_ctx.clone();
        let mut actions = Vec::new();
        let out = ctx.run(raw, |ctx| {
            ui(
                ctx,
                &mut self.screen,
                &mut self.menu,
                &self.settings,
                &mut self.host,
                &self.update_note,
                view.map(|v| v.0),
                &mut actions,
            );
        });
        state.handle_platform_output(&window, out.platform_output.clone());
        gfx.frame(view, &sky_now, &boxes, &lines, &ctx, out);
        for a in actions {
            self.act(a);
        }
    }

    fn act(&mut self, a: Action) {
        match a {
            Action::Respawn => {
                if let Screen::Playing(p) = &mut self.screen {
                    let _ = p.conn.tx.send(ClientMsg::Respawn);
                }
            }
            Action::Pack(cmds) => {
                let mut close = false;
                if let Screen::Playing(p) = &mut self.screen {
                    for c in cmds {
                        match c {
                            PackCmd::Move(a, b, n) => {
                                let _ = p.conn.tx.send(ClientMsg::Move {
                                    from: a,
                                    to: b,
                                    count: n,
                                });
                            }
                            PackCmd::Throw(a, n) => {
                                let _ = p.conn.tx.send(ClientMsg::Throw { from: a, count: n });
                            }
                            PackCmd::Craft(r, n) => {
                                let _ = p.conn.tx.send(ClientMsg::Craft {
                                    recipe: r,
                                    times: n,
                                });
                                self.audio.play(Sfx::Place(w::Material::Wood), 0.6);
                            }
                            PackCmd::Creative(item) => {
                                let _ = p.conn.tx.send(ClientMsg::CreativeSet {
                                    slot: p.slot as u8,
                                    item,
                                });
                            }
                            PackCmd::Close => {
                                p.picker_open = false;
                                p.selected = None;
                                if p.chest.take().is_some() {
                                    let _ = p.conn.tx.send(ClientMsg::CloseContainer);
                                }
                                close = true;
                            }
                        }
                    }
                }
                if close {
                    self.set_capture(true);
                }
            }
            Action::Join(addr) => match proto::server_url(&addr) {
                Ok(_) => self.start_join(addr, true),
                Err(e) => self.menu.error = Some(e),
            },
            Action::Host => self.start_host(),
            Action::StopHost => {
                if let Some(h) = self.host.take() {
                    h.stop();
                }
            }
            Action::Tunnel => {
                if let Some(h) = self.host.as_mut()
                    && h.tunnel.is_none()
                {
                    match start_tunnel(h.port) {
                        Ok(t) => h.tunnel = Some(t),
                        Err(e) => self.menu.error = Some(e),
                    }
                }
            }
            Action::CancelJoin => self.screen = Screen::Menu,
            Action::Resume => {
                if let Screen::Playing(p) = &mut self.screen {
                    p.menu_open = false;
                }
                self.set_capture(true);
            }
            Action::Disconnect => {
                self.set_capture(false);
                self.screen = Screen::Menu;
            }
            Action::Capture => self.set_capture(true),
            Action::Quit => self.fatal = None,
        }
    }

    fn key(&mut self, code: KeyCode, pressed: bool, repeat: bool) {
        let Screen::Playing(p) = &mut self.screen else {
            return;
        };
        if p.chat_open || p.menu_open || p.picker_open || p.dead.is_some() {
            return;
        }
        if pressed {
            p.keys.insert(code);
        } else {
            p.keys.remove(&code);
        }
        if !pressed || repeat {
            return;
        }
        match code {
            KeyCode::Escape => {
                p.menu_open = true;
                self.set_capture(false);
            }
            KeyCode::KeyT | KeyCode::Enter | KeyCode::Slash => {
                p.chat_open = true;
                p.focus_chat = true;
                if code == KeyCode::Slash {
                    p.chat_text = "/".into();
                }
                self.set_capture(false);
            }
            KeyCode::KeyF if p.creative() => {
                p.player.flying = !p.player.flying;
                p.player.vel.y = 0.0;
            }
            KeyCode::KeyQ => {
                let all = p.keys.contains(&KeyCode::ControlLeft)
                    || p.keys.contains(&KeyCode::ControlRight);
                if p.held().is_some() {
                    let _ = p.conn.tx.send(ClientMsg::Throw {
                        from: SlotRef::Pack(p.slot as u8),
                        count: if all { 0 } else { 1 },
                    });
                    p.swing = 1.0;
                }
            }
            KeyCode::KeyE => {
                p.picker_open = true;
                p.selected = None;
                self.set_capture(false);
            }
            KeyCode::KeyZ
                if p.creative()
                    && (p.keys.contains(&KeyCode::ControlLeft)
                        || p.keys.contains(&KeyCode::ControlRight)) =>
            {
                while let Some((pos, old, new)) = p.undo.pop() {
                    if p.world.get(pos[0], pos[1], pos[2]) == new {
                        let _ = p.conn.tx.send(ClientMsg::SetBlock { pos, block: old });
                        break;
                    }
                }
            }
            KeyCode::KeyG if p.creative() => {
                if let Some(t) = &p.target {
                    p.sel = match p.sel {
                        (Some(a), None) => (Some(a), Some(t.block)),
                        _ => (Some(t.block), None),
                    };
                } else {
                    p.sel = (None, None);
                }
            }
            KeyCode::KeyV | KeyCode::KeyX if p.creative() => {
                if let (Some(a), Some(b)) = p.sel {
                    let block = if code == KeyCode::KeyX {
                        w::AIR
                    } else {
                        p.held_block().unwrap_or(w::STONE)
                    };
                    let _ = p.conn.tx.send(ClientMsg::Fill { a, b, block });
                    p.sel = (None, None);
                }
            }
            _ => {
                let digits = [
                    KeyCode::Digit1,
                    KeyCode::Digit2,
                    KeyCode::Digit3,
                    KeyCode::Digit4,
                    KeyCode::Digit5,
                    KeyCode::Digit6,
                    KeyCode::Digit7,
                    KeyCode::Digit8,
                    KeyCode::Digit9,
                ];
                if let Some(i) = digits.iter().position(|d| *d == code) {
                    p.slot = i;
                }
            }
        }
    }

    fn click(&mut self, button: MouseButton) {
        let Some(gfx) = self.gfx.as_mut() else { return };
        let Screen::Playing(p) = &mut self.screen else {
            return;
        };
        if !p.captured || p.menu_open || p.chat_open || p.picker_open || p.dead.is_some() {
            return;
        }
        p.swing = 1.0;
        let Some(t) = &p.target else { return };
        let (tb, tp) = (t.block, t.place);
        match button {
            MouseButton::Left if p.creative() => {
                let pos = tb;
                let old = p.world.get(pos[0], pos[1], pos[2]);
                p.world.set(pos[0], pos[1], pos[2], w::AIR);
                p.light.changed(&p.world, pos);
                gfx.block_changed(&p.world, &p.light, pos, 15);
                spawn_particles(
                    &mut p.particles,
                    pos,
                    old,
                    14,
                    (pos[0] * 31 + pos[1] * 7 + pos[2]) as u32,
                );
                self.audio.play(Sfx::Break(material_of(old)), 0.9);
                p.undo.push((pos, old, w::AIR));
                let _ = p.conn.tx.send(ClientMsg::Dig { pos });
            }
            MouseButton::Left => {}
            MouseButton::Right => {
                let looked = p.world.get(tb[0], tb[1], tb[2]);
                let sneaking = p.keys.contains(&KeyCode::ShiftLeft);
                if !sneaking && matches!(looked, w::CHEST | w::BED) {
                    let _ = p.conn.tx.send(ClientMsg::Interact { pos: tb });
                    return;
                }
                if !sneaking && matches!(looked, w::CRAFTING_TABLE | w::FURNACE) {
                    p.picker_open = true;
                    p.selected = None;
                    self.set_capture(false);
                    return;
                }
                let held = p.held();
                if let Some(s) = held
                    && p.content.item(s.item).food > 0
                    && !p.creative()
                    && p.food < 20.0
                    && p.held_block().is_none()
                {
                    let _ = p.conn.tx.send(ClientMsg::UseHeld);
                    self.audio.play(Sfx::Step(w::Material::Foliage), 0.8);
                    return;
                }
                let Some(block) = p.held_block() else { return };
                let pos = tp;
                let here = p.world.get(pos[0], pos[1], pos[2]);
                if (here == w::AIR || here == w::WATER)
                    && p.world.in_bounds(pos)
                    && (!w::is_solid(block) || !p.player.overlaps_block(pos))
                {
                    p.world.set(pos[0], pos[1], pos[2], block);
                    p.light.changed(&p.world, pos);
                    gfx.block_changed(&p.world, &p.light, pos, 15);
                    self.audio.play(Sfx::Place(material_of(block)), 0.9);
                    if p.creative() {
                        p.undo.push((pos, here, block));
                        if p.undo.len() > 64 {
                            p.undo.remove(0);
                        }
                    } else if let Some(Some(s)) = p.pack.get_mut(p.slot) {
                        // predict the count; the server sends the real pack
                        s.count = s.count.saturating_sub(1);
                        if s.count == 0 {
                            p.pack[p.slot] = None;
                        }
                    }
                    let _ = p.conn.tx.send(ClientMsg::Place {
                        pos,
                        slot: p.slot as u8,
                    });
                }
            }
            MouseButton::Middle if p.creative() => {
                let b = p.world.get(tb[0], tb[1], tb[2]);
                if b != w::AIR && b != w::WATER {
                    let _ = p.conn.tx.send(ClientMsg::CreativeSet {
                        slot: p.slot as u8,
                        item: b as u16,
                    });
                    self.audio.play(Sfx::Click, 0.6);
                }
            }
            _ => {}
        }
    }
}

enum Action {
    Pack(Vec<PackCmd>),
    Respawn,
    Join(String),
    Host,
    StopHost,
    Tunnel,
    CancelJoin,
    Resume,
    Disconnect,
    Capture,
    #[allow(dead_code)]
    Quit,
}

fn host_panel(ui: &mut egui::Ui, host: &mut Option<Host>, actions: &mut Vec<Action>) {
    let Some(h) = host.as_mut() else { return };
    ui.label(
        RichText::new("You are hosting this world")
            .strong()
            .color(ACCENT),
    );
    ui.label(format!(
        "{} player(s) online. The world saves automatically.",
        h.handle.server.player_count()
    ));
    if let Some(lan) = &h.lan {
        ui.horizontal(|ui| {
            ui.label("Same network:");
            let a = format!("{lan}:{}", h.port);
            ui.monospace(&a);
            if ui.small_button("Copy").clicked() {
                ui.ctx().copy_text(a);
            }
        });
    }
    match &h.tunnel {
        None => {
            if ui
                .button("Share over the internet (Cloudflare tunnel)")
                .clicked()
            {
                actions.push(Action::Tunnel);
            }
        }
        Some(t) => {
            let url = t.url.lock().unwrap().clone();
            let err = t.error.lock().unwrap().clone();
            match (url, err) {
                (Some(u), _) => {
                    ui.horizontal(|ui| {
                        ui.label("Internet:");
                        ui.monospace(&u);
                        if ui.small_button("Copy").clicked() {
                            ui.ctx().copy_text(u.clone());
                        }
                    });
                    ui.label(
                        RichText::new(
                            "Send this address to friends. They paste it into Server address.",
                        )
                        .small(),
                    );
                }
                (None, Some(e)) => {
                    ui.colored_label(Color32::LIGHT_RED, e);
                }
                (None, None) => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Opening tunnel...");
                    });
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn ui(
    ctx: &egui::Context,
    screen: &mut Screen,
    menu: &mut Menu,
    settings: &Settings,
    host: &mut Option<Host>,
    update_note: &str,
    vp: Option<glam::Mat4>,
    actions: &mut Vec<Action>,
) {
    let version = env!("CARGO_PKG_VERSION");
    match screen {
        Screen::Menu => {
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space((ui.available_height() * 0.12).max(10.0));
                    egui::Frame::window(ui.style()).inner_margin(24.0).show(ui, |ui| {
                        ui.set_width(440.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Dyno").size(40.0).strong().color(ACCENT));
                            ui.label(RichText::new(format!("Build together · v{version} · {update_note}")).weak());
                            ui.add_space(14.0);
                            ui.label("Your name");
                            ui.add(egui::TextEdit::singleline(&mut menu.name).hint_text("wren").desired_width(f32::INFINITY).char_limit(proto::MAX_NAME));
                            ui.add_space(8.0);
                            ui.label("Server address");
                            let r = ui.add(
                                egui::TextEdit::singleline(&mut menu.addr)
                                    .hint_text("192.168.1.20 or https://name.trycloudflare.com")
                                    .desired_width(f32::INFINITY),
                            );
                            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            ui.add_space(6.0);
                            if ui.add_sized([ui.available_width(), 34.0], egui::Button::new(RichText::new("Join").strong().size(16.0)).fill(ACCENT.gamma_multiply(0.85))).clicked() || enter {
                                actions.push(Action::Join(menu.addr.clone()));
                            }
                            if let Some(e) = &menu.error {
                                ui.add_space(6.0);
                                ui.colored_label(Color32::from_rgb(240, 120, 110), e);
                            }
                            if !settings.recent.is_empty() {
                                ui.add_space(10.0);
                                ui.label(RichText::new("Recent").weak());
                                for a in settings.recent.iter().take(5) {
                                    if ui.link(a).clicked() {
                                        menu.addr = a.clone();
                                        actions.push(Action::Join(a.clone()));
                                    }
                                }
                            }
                            ui.add_space(12.0);
                            ui.separator();
                            ui.add_space(6.0);
                            if host.is_none() {
                                if ui.button("Host a world on this computer").clicked() {
                                    actions.push(Action::Host);
                                }
                                ui.label(RichText::new("Friends on your network join with your IP address. For the internet, share a Cloudflare tunnel after you start hosting.").small().weak());
                            } else {
                                host_panel(ui, host, actions);
                                ui.horizontal(|ui| {
                                    if ui.button("Play in my world").clicked() {
                                        actions.push(Action::Host);
                                    }
                                    if ui.button("Stop hosting").clicked() {
                                        actions.push(Action::StopHost);
                                    }
                                });
                            }
                        });
                    });
                });
            });
        }
        Screen::Joining { label, since, .. } => {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() * 0.4);
                        ui.spinner();
                        ui.label(RichText::new(format!("Joining {label} ...")).size(18.0));
                        ui.label(RichText::new(format!("{} s", since.elapsed().as_secs())).weak());
                        if ui.button("Cancel").clicked() {
                            actions.push(Action::CancelJoin);
                        }
                    });
                });
        }
        Screen::Playing(p) => hud(ctx, p, host, vp, actions),
    }
}

fn hud(
    ctx: &egui::Context,
    p: &mut Play,
    host: &mut Option<Host>,
    vp: Option<glam::Mat4>,
    actions: &mut Vec<Action>,
) {
    let screen = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Background,
        egui::Id::new("hud"),
    ));
    let c = screen.center();
    // crosshair
    if p.captured {
        let s = Stroke::new(2.0_f32, Color32::from_white_alpha(220));
        painter.line_segment([c - Vec2::new(9.0, 0.0), c + Vec2::new(9.0, 0.0)], s);
        painter.line_segment([c - Vec2::new(0.0, 9.0), c + Vec2::new(0.0, 9.0)], s);
    }
    // name tags
    if let Some(vp) = vp {
        for r in p.remotes.values() {
            let pose = r.current();
            let head = glam::Vec3::from(pose.pos) + glam::Vec3::Y * 2.15;
            let clip = vp * head.extend(1.0);
            if clip.w > 0.1 {
                let ndc = clip.truncate() / clip.w;
                let pos = Pos2::new(
                    screen.left() + (ndc.x * 0.5 + 0.5) * screen.width(),
                    screen.top() + (0.5 - ndc.y * 0.5) * screen.height(),
                );
                let font = FontId::proportional(14.0);
                let galley = painter.layout_no_wrap(r.name.clone(), font, Color32::WHITE);
                let rect = Rect::from_center_size(pos, galley.size() + Vec2::new(10.0, 4.0));
                painter.rect_filled(rect, 4.0, Color32::from_black_alpha(140));
                painter.galley(rect.min + Vec2::new(5.0, 2.0), galley, Color32::WHITE);
            }
        }
    }
    // top-left info
    let pos = p.player.pos;
    let info = format!(
        "Dyno {} · {} · {} online · {} fps\nx {:.0}  y {:.0}  z {:.0}{}",
        env!("CARGO_PKG_VERSION"),
        p.url,
        p.remotes.len() + 1,
        p.fps.2,
        pos.x,
        pos.y,
        pos.z,
        if p.player.flying { "  · flying" } else { "" }
    );
    painter.text(
        screen.left_top() + Vec2::new(12.0, 10.0),
        Align2::LEFT_TOP,
        info,
        FontId::monospace(13.0),
        Color32::from_white_alpha(230),
    );
    if p.server_version != env!("CARGO_PKG_VERSION") {
        painter.text(
            screen.left_top() + Vec2::new(12.0, 48.0),
            Align2::LEFT_TOP,
            format!("server runs dyno {}", p.server_version),
            FontId::monospace(12.0),
            Color32::from_rgb(232, 184, 74),
        );
    }
    // hotbar from the pack
    let slot = 46.0;
    let total = 9.0 * (slot + 4.0);
    let start = Pos2::new(c.x - total / 2.0, screen.bottom() - slot - 14.0);
    painter.rect_filled(
        Rect::from_min_size(
            start - Vec2::splat(6.0),
            Vec2::new(total + 8.0, slot + 12.0),
        ),
        8.0,
        Color32::from_black_alpha(130),
    );
    for i in 0..9 {
        let r = Rect::from_min_size(
            start + Vec2::new(i as f32 * (slot + 4.0), 0.0),
            Vec2::splat(slot),
        );
        if let Some(s) = p.pack.get(i).copied().flatten() {
            survival::stack_icon(&painter, r, &p.content, &s);
        }
        let stroke = if i == p.slot {
            Stroke::new(2.5_f32, ACCENT)
        } else {
            Stroke::new(1.0_f32, Color32::from_white_alpha(60))
        };
        painter.rect_stroke(r, 5.0, stroke, egui::StrokeKind::Inside);
        painter.text(
            r.left_top() + Vec2::new(4.0, 2.0),
            Align2::LEFT_TOP,
            format!("{}", i + 1),
            FontId::monospace(10.0),
            Color32::from_white_alpha(140),
        );
    }
    let held_name = p
        .held()
        .map(|s| p.content.item(s.item).name.to_string())
        .unwrap_or_default();
    let mut label_y = start.y - 12.0;
    if !p.creative() {
        survival::bars(
            &painter,
            Pos2::new(start.x, start.y - 18.0),
            Pos2::new(start.x + total, start.y - 18.0),
            p.health,
            p.food,
        );
        label_y -= 16.0;
        if p.air < 10.0 {
            let k = (p.air / 10.0).clamp(0.0, 1.0);
            let r = Rect::from_min_size(
                Pos2::new(start.x + total - 110.0, start.y - 34.0),
                Vec2::new(110.0 * k, 5.0),
            );
            painter.rect_filled(r, 2.0, Color32::from_rgb(110, 170, 230));
            label_y -= 8.0;
        }
    }
    painter.text(
        Pos2::new(c.x, label_y),
        Align2::CENTER_BOTTOM,
        held_name,
        FontId::proportional(15.0),
        Color32::WHITE,
    );
    // mining progress ring around the crosshair
    if let Some((_, done, total)) = p.dig
        && total > 0.05
    {
        let k = (done / total).clamp(0.0, 1.0);
        let n = 32;
        let pts: Vec<Pos2> = (0..=((n as f32 * k) as usize))
            .map(|i| {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 / n as f32 * std::f32::consts::TAU;
                c + Vec2::new(a.cos(), a.sin()) * 16.0
            })
            .collect();
        if pts.len() > 1 {
            painter.add(egui::Shape::line(pts, Stroke::new(3.0_f32, ACCENT)));
        }
    }
    // being hurt flashes the screen edge red
    if p.hurt_flash > 0.0 {
        let a = (p.hurt_flash * 120.0) as u8;
        for (r, _) in [
            (
                Rect::from_min_size(screen.min, Vec2::new(screen.width(), 40.0)),
                0,
            ),
            (
                Rect::from_min_size(
                    Pos2::new(screen.left(), screen.bottom() - 40.0),
                    Vec2::new(screen.width(), 40.0),
                ),
                0,
            ),
            (
                Rect::from_min_size(screen.min, Vec2::new(40.0, screen.height())),
                0,
            ),
            (
                Rect::from_min_size(
                    Pos2::new(screen.right() - 40.0, screen.top()),
                    Vec2::new(40.0, screen.height()),
                ),
                0,
            ),
        ] {
            painter.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(200, 20, 20, a));
        }
    }
    // milestone and mode toasts
    for (k, (at, text)) in p.toasts.iter().enumerate() {
        let fade = (1.0 - (at.elapsed().as_secs_f32() - 3.5).max(0.0) / 1.5).clamp(0.0, 1.0);
        let pos = Pos2::new(c.x, screen.top() + 70.0 + k as f32 * 34.0);
        let galley = painter.layout_no_wrap(
            text.clone(),
            FontId::proportional(18.0),
            Color32::WHITE.gamma_multiply(fade),
        );
        let r = Rect::from_center_size(pos, galley.size() + Vec2::new(24.0, 12.0));
        painter.rect_filled(r, 8.0, Color32::from_black_alpha((170.0 * fade) as u8));
        painter.rect_stroke(
            r,
            8.0,
            Stroke::new(1.5_f32, ACCENT.gamma_multiply(fade)),
            egui::StrokeKind::Inside,
        );
        painter.galley(r.min + Vec2::new(12.0, 6.0), galley, Color32::WHITE);
    }
    // chat
    let chat_bottom = label_y - 46.0;
    let visible: Vec<&Chat> = p
        .chat
        .iter()
        .rev()
        .filter(|m| p.chat_open || m.at.elapsed() < Duration::from_secs(15))
        .take(10)
        .collect();
    let mut y = chat_bottom;
    for m in visible {
        let (txt, col) = match &m.from {
            Some(f) => (format!("<{f}> {}", m.text), Color32::WHITE),
            None => (m.text.clone(), Color32::from_rgb(232, 200, 110)),
        };
        let galley = painter.layout(txt, FontId::proportional(14.0), col, 520.0);
        y -= galley.size().y + 4.0;
        let r = Rect::from_min_size(
            Pos2::new(screen.left() + 10.0, y - 2.0),
            galley.size() + Vec2::new(10.0, 4.0),
        );
        painter.rect_filled(r, 3.0, Color32::from_black_alpha(120));
        painter.galley(Pos2::new(screen.left() + 15.0, y), galley, col);
    }
    if p.chat_open {
        egui::Area::new(egui::Id::new("chat_in"))
            .fixed_pos(Pos2::new(screen.left() + 10.0, chat_bottom + 6.0))
            .show(ctx, |ui| {
                let r = ui.add(
                    egui::TextEdit::singleline(&mut p.chat_text)
                        .desired_width(520.0)
                        .hint_text("Say something, Enter to send, Esc to cancel"),
                );
                if p.focus_chat {
                    r.request_focus();
                    p.focus_chat = false;
                }
                let send = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let cancel = ui.input(|i| i.key_pressed(egui::Key::Escape));
                if send {
                    let t = p.chat_text.trim().to_string();
                    if !t.is_empty() {
                        let _ = p.conn.tx.send(ClientMsg::Chat { text: t });
                    }
                    p.chat_text.clear();
                    p.chat_open = false;
                    actions.push(Action::Capture);
                } else if cancel {
                    p.chat_text.clear();
                    p.chat_open = false;
                    actions.push(Action::Capture);
                }
            });
    }
    // fill hint (creative)
    if let (true, Some(a)) = (p.creative(), p.sel.0) {
        let b = p.sel.1.or(p.target.as_ref().map(|t| t.block)).unwrap_or(a);
        let n: i64 = (0..3).map(|i| ((a[i] - b[i]).abs() + 1) as i64).product();
        let msg = if p.sel.1.is_some() {
            format!("{n} cells selected · V fill with held block · X clear · G start again")
        } else {
            format!("{n} cells · look at the other corner and press G")
        };
        let col = if n > dyno_protocol::MAX_FILL {
            Color32::from_rgb(240, 110, 100)
        } else {
            ACCENT
        };
        painter.text(
            Pos2::new(c.x, label_y - 22.0),
            Align2::CENTER_BOTTOM,
            msg,
            FontId::proportional(14.0),
            col,
        );
    }
    // pack screen
    if p.picker_open {
        let stations = survival::stations_near(&p.world, p.player.pos.into());
        let view = PackView {
            content: &p.content,
            pack: &p.pack,
            chest: p.chest.as_ref().map(|(_, s)| s.as_slice()),
            stations,
            fuel: p.fuel,
            mode: p.mode,
            selected: &mut p.selected,
            hotbar_slot: p.slot,
        };
        let cmds = survival::pack_window(ctx, view, &mut p.recipe_filter);
        if !cmds.is_empty() {
            actions.push(Action::Pack(cmds));
        }
    }
    // death screen
    if let Some(msg) = &p.dead {
        painter.rect_filled(
            screen,
            0.0,
            Color32::from_rgba_unmultiplied(90, 10, 10, 110),
        );
        egui::Area::new(egui::Id::new("dead"))
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                egui::Frame::window(ui.style())
                    .inner_margin(20.0)
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new("You died")
                                    .size(30.0)
                                    .strong()
                                    .color(Color32::from_rgb(240, 110, 100)),
                            );
                            ui.label(msg);
                            ui.label(
                                RichText::new(
                                    "Your items are where you fell. Friends can pick them up too.",
                                )
                                .weak(),
                            );
                            ui.add_space(8.0);
                            if ui
                                .add_sized(
                                    [200.0, 34.0],
                                    egui::Button::new(RichText::new("Respawn").strong()),
                                )
                                .clicked()
                            {
                                actions.push(Action::Respawn);
                            }
                        });
                    });
            });
    }
    // player list on Tab
    if p.keys.contains(&KeyCode::Tab) {
        egui::Area::new(egui::Id::new("players"))
            .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 60.0))
            .show(ctx, |ui| {
                egui::Frame::window(ui.style()).show(ui, |ui| {
                    ui.label(RichText::new(format!("{} online", p.remotes.len() + 1)).strong());
                    ui.label("you");
                    for r in p.remotes.values() {
                        ui.label(&r.name);
                    }
                });
            });
    }
    // pause menu
    if p.menu_open {
        egui::Window::new("Menu").collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, Vec2::ZERO).show(ctx, |ui| {
            ui.set_width(380.0);
            if ui.add_sized([ui.available_width(), 32.0], egui::Button::new("Back to game")).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                actions.push(Action::Resume);
            }
            ui.add_space(6.0);
            host_panel(ui, host, actions);
            ui.add_space(6.0);
            ui.label(RichText::new(format!("Connected to {}", p.url)).small().weak());
            ui.label(RichText::new("Controls: WASD move · Space jump · Shift down when flying · Ctrl or R sprint · F fly · left click break · right click place · middle click pick · 1-9 or wheel blocks · E all blocks · G corner, V fill, X clear · Ctrl+Z undo · T chat · Tab players").small().weak());
            if ui.button("Leave server").clicked() {
                actions.push(Action::Disconnect);
            }
        });
    } else if !p.captured && !p.chat_open {
        egui::Area::new(egui::Id::new("click"))
            .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, 40.0))
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("Click to play")
                        .size(18.0)
                        .color(Color32::WHITE),
                );
            });
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title(format!("Dyno {}", env!("CARGO_PKG_VERSION")))
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 760.0));
        let window = match el.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                self.fatal = Some(format!("cannot create window: {e}"));
                el.exit();
                return;
            }
        };
        match Gfx::new(window.clone()) {
            Ok(g) => {
                eprintln!("graphics: {}", g.adapter_info);
                self.egui_ctx.set_visuals(egui::Visuals::dark());
                self.egui_ctx.style_mut(|s| {
                    s.visuals.selection.bg_fill = ACCENT.gamma_multiply(0.6);
                    s.visuals.hyperlink_color = Color32::from_rgb(111, 179, 210);
                });
                let state = egui_winit::State::new(
                    self.egui_ctx.clone(),
                    egui::ViewportId::ROOT,
                    &window,
                    Some(window.scale_factor() as f32),
                    None,
                    Some(g.device.limits().max_texture_dimension_2d as usize),
                );
                self.egui_state = Some(state);
                self.gfx = Some(g);
            }
            Err(e) => {
                self.fatal = Some(e);
                el.exit();
            }
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let captured = matches!(&self.screen, Screen::Playing(p) if p.captured);
        if let (Some(state), Some(g)) = (self.egui_state.as_mut(), self.gfx.as_ref())
            && (!captured
                || matches!(
                    event,
                    WindowEvent::Resized(_)
                        | WindowEvent::ScaleFactorChanged { .. }
                        | WindowEvent::Focused(_)
                ))
            && let Some(win) = &g.window
        {
            let _ = state.on_window_event(win, &event);
        }
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) => {
                if let Some(g) = self.gfx.as_mut() {
                    g.resize(s.width, s.height);
                }
            }
            WindowEvent::RedrawRequested => self.frame(),
            WindowEvent::Focused(false) => self.set_capture(false),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    let pressed = event.state == ElementState::Pressed;
                    if code == KeyCode::Tab
                        && let Screen::Playing(p) = &mut self.screen
                    {
                        if pressed {
                            p.keys.insert(code);
                        } else {
                            p.keys.remove(&code);
                        }
                    }
                    if captured {
                        self.key(code, pressed, event.repeat);
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                if let Screen::Playing(p) = &mut self.screen {
                    p.mouse_left = false;
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                if button == MouseButton::Left
                    && captured
                    && let Screen::Playing(p) = &mut self.screen
                {
                    p.mouse_left = true;
                }
                let (playing_free, menu_or_chat) = match &self.screen {
                    Screen::Playing(p) => {
                        (!p.captured, p.menu_open || p.chat_open || p.picker_open)
                    }
                    _ => (false, true),
                };
                if playing_free && !menu_or_chat && button == MouseButton::Left {
                    self.set_capture(true);
                } else if captured {
                    self.click(button);
                }
            }
            WindowEvent::MouseWheel { delta, .. } if captured => {
                if let Screen::Playing(p) = &mut self.screen {
                    let d = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(p) => (p.y / 40.0) as f32,
                    };
                    let n = 9;
                    if d.abs() > 0.1 {
                        p.slot = ((p.slot as i32 - d.signum() as i32).rem_euclid(n)) as usize;
                    }
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _el: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event
            && let Screen::Playing(p) = &mut self.screen
            && p.captured
            && !p.menu_open
            && !p.chat_open
        {
            let s = 0.0025 * self.settings.mouse_sensitivity;
            p.player.yaw -= delta.0 as f32 * s;
            p.player.pitch = (p.player.pitch - delta.1 as f32 * s).clamp(-1.55, 1.55);
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        if let Some(w) = self.window() {
            w.request_redraw();
        }
    }
}
