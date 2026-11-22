//! Window, input, screens (connect, joining, playing) and the HUD.

use crate::game::{self, Input, Player, Remote, Remotes, Target};
use crate::net::{self, Joined, NetEvent};
use crate::render::{self, Gfx, Vertex};
use crate::settings::Settings;
use dyno_protocol::{self as proto, ClientMsg, PlayerId, ServerMsg};
use dyno_world::{self as w, World};
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Stroke, Vec2};
use egui_winit::winit;
use glam::Vec3;
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
        egui_ctx: egui::Context::default(),
        egui_state: None,
        screen: Screen::Menu,
        host: None,
        last_frame: Instant::now(),
        update_note,
        fatal: None,
        autojoined: false,
    };
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
        self.gfx.as_ref().map(|g| &g.window)
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
        std::thread::spawn(move || {
            let _ = tx.send(net::connect(&a, &name, Duration::from_secs(10)));
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
                    };
                    p.chat.push(Chat { at: Instant::now(), from: None, text: "Click to play. WASD move, Space jump, F fly, left click break, right click place, 1-9 pick block, T chat, Esc menu.".into() });
                    if let Some(g) = self.gfx.as_mut() {
                        g.load_world(&p.world);
                    }
                    self.screen = Screen::Playing(Box::new(p));
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
                            if info.id != p.my_id {
                                p.remotes.insert(info.id, Remote::new(info.name, info.pose));
                            }
                        }
                        ServerMsg::PlayerLeft { id } => {
                            p.remotes.remove(&id);
                        }
                        ServerMsg::Block { pos, block } => {
                            if p.world.get(pos[0], pos[1], pos[2]) != block {
                                p.world.set(pos[0], pos[1], pos[2], block);
                                if let Some(g) = self.gfx.as_mut() {
                                    g.block_changed(&p.world, pos);
                                }
                            }
                        }
                        ServerMsg::Chat { from, text } => p.chat.push(Chat {
                            at: Instant::now(),
                            from: Some(from),
                            text,
                        }),
                        ServerMsg::System { text } => p.chat.push(Chat {
                            at: Instant::now(),
                            from: None,
                            text,
                        }),
                        ServerMsg::Reject { reason } => closed = Some(reason),
                        _ => {}
                    },
                }
            }
            if p.chat.len() > 200 {
                p.chat.drain(..100);
            }
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

        // simulate
        let mut view = None;
        let mut boxes: Vec<Vertex> = Vec::new();
        let mut lines: Vec<Vertex> = Vec::new();
        if let Screen::Playing(p) = &mut self.screen {
            let k = |c: KeyCode| p.keys.contains(&c);
            let input = if p.captured && !p.chat_open && !p.menu_open {
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
            p.player.update(&p.world, input, dt);
            if p.last_pose.elapsed() >= Duration::from_millis(50) {
                p.last_pose = Instant::now();
                let _ = p.conn.tx.send(ClientMsg::Pose(p.player.pose()));
            }
            p.target = game::raycast(&p.world, p.player.eye(), p.player.look_dir(), game::REACH);
            for (id, r) in &p.remotes {
                let pose = r.current();
                let base = Vec3::from(pose.pos);
                let c = game::player_color(*id);
                render::push_box(
                    &mut boxes,
                    base + Vec3::new(-0.3, 0.0, -0.3),
                    base + Vec3::new(0.3, 1.35, 0.3),
                    c,
                );
                render::push_box(
                    &mut boxes,
                    base + Vec3::new(-0.25, 1.35, -0.25),
                    base + Vec3::new(0.25, 1.85, 0.25),
                    [0.91, 0.76, 0.62],
                );
            }
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
                            color: [0.02, 0.02, 0.02, 0.75],
                        });
                    }
                }
            }
            let vp = p.player.view_proj(wdt / hgt, fov, far);
            view = Some((vp, p.player.eye(), far));
            p.fps.1 += 1;
            if p.fps.0.elapsed() >= Duration::from_secs(1) {
                p.fps = (Instant::now(), 0, p.fps.1);
            }
        }

        // UI
        let Some(state) = self.egui_state.as_mut() else {
            return;
        };
        let window = gfx.window.clone();
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
        gfx.frame(view, &boxes, &lines, &ctx, out);
        for a in actions {
            self.act(a);
        }
    }

    fn act(&mut self, a: Action) {
        match a {
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
        if p.chat_open || p.menu_open {
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
            KeyCode::KeyF => {
                p.player.flying = !p.player.flying;
                p.player.vel.y = 0.0;
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
                    KeyCode::Digit0,
                    KeyCode::Minus,
                    KeyCode::Equal,
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
        if !p.captured || p.menu_open || p.chat_open {
            return;
        }
        let Some(t) = &p.target else { return };
        match button {
            MouseButton::Left => {
                let pos = t.block;
                p.world.set(pos[0], pos[1], pos[2], w::AIR);
                gfx.block_changed(&p.world, pos);
                let _ = p.conn.tx.send(ClientMsg::SetBlock { pos, block: w::AIR });
            }
            MouseButton::Right => {
                let pos = t.place;
                let block = w::PLACEABLE[p.slot];
                let here = p.world.get(pos[0], pos[1], pos[2]);
                if (here == w::AIR || here == w::WATER)
                    && p.world.in_bounds(pos)
                    && !p.player.overlaps_block(pos)
                {
                    p.world.set(pos[0], pos[1], pos[2], block);
                    gfx.block_changed(&p.world, pos);
                    let _ = p.conn.tx.send(ClientMsg::SetBlock { pos, block });
                }
            }
            MouseButton::Middle => {
                let b = p.world.get(t.block[0], t.block[1], t.block[2]);
                if let Some(i) = w::PLACEABLE.iter().position(|x| *x == b) {
                    p.slot = i;
                }
            }
            _ => {}
        }
    }
}

enum Action {
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
    // hotbar
    let n = w::PLACEABLE.len();
    let slot = 44.0;
    let total = n as f32 * (slot + 4.0);
    let start = Pos2::new(c.x - total / 2.0, screen.bottom() - slot - 14.0);
    painter.rect_filled(
        Rect::from_min_size(
            start - Vec2::splat(6.0),
            Vec2::new(total + 8.0, slot + 12.0),
        ),
        8.0,
        Color32::from_black_alpha(130),
    );
    for (i, b) in w::PLACEABLE.iter().enumerate() {
        let r = Rect::from_min_size(
            start + Vec2::new(i as f32 * (slot + 4.0), 0.0),
            Vec2::splat(slot),
        );
        let (rgb, _) = render::block_color(*b);
        let col = Color32::from_rgb(
            (rgb[0] * 255.0) as u8,
            (rgb[1] * 255.0) as u8,
            (rgb[2] * 255.0) as u8,
        );
        painter.rect_filled(r.shrink(8.0), 3.0, col);
        let stroke = if i == p.slot {
            Stroke::new(2.5_f32, ACCENT)
        } else {
            Stroke::new(1.0_f32, Color32::from_white_alpha(60))
        };
        painter.rect_stroke(r, 5.0, stroke, egui::StrokeKind::Inside);
        let key = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "="][i];
        painter.text(
            r.left_top() + Vec2::new(4.0, 2.0),
            Align2::LEFT_TOP,
            key,
            FontId::monospace(10.0),
            Color32::from_white_alpha(170),
        );
    }
    painter.text(
        Pos2::new(c.x, start.y - 12.0),
        Align2::CENTER_BOTTOM,
        w::BLOCK_NAMES[w::PLACEABLE[p.slot] as usize],
        FontId::proportional(15.0),
        Color32::WHITE,
    );
    // chat
    let chat_bottom = start.y - 40.0;
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
            ui.label(RichText::new("Controls: WASD move · Space jump · Shift down when flying · Ctrl or R sprint · F fly · left click break · right click place · middle click pick · 1-9, 0, -, = blocks · T chat · Tab players").small().weak());
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
        {
            let _ = state.on_window_event(&g.window, &event);
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
                state: ElementState::Pressed,
                button,
                ..
            } => {
                let (playing_free, menu_or_chat) = match &self.screen {
                    Screen::Playing(p) => (!p.captured, p.menu_open || p.chat_open),
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
                    let n = w::PLACEABLE.len() as i32;
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
