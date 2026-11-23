//! Window, input, screens (connect, joining, playing) and the HUD.

use crate::anim::{self, Anim, Book as Anims};
use crate::audio::{Audio, Sfx};
use crate::front::{self, Front, FrontCmd};
use crate::game::{self, Input, Player, PlayerPose, Remote, Remotes, Target};
use crate::light::Light;
use crate::models;
use crate::net::{self, Joined, NetEvent};
use crate::render::{self, Gfx, Vertex};
use crate::rpg_ui;
use crate::screens;
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
    // the pack of the last world joined, from the cache (title stage, strings,
    // textures, fonts), unless a dev build already loaded a local one
    if crate::assets::get("pack.json").is_none() {
        crate::cache::Cache::default().load_last();
    }
    #[cfg(feature = "host")]
    if crate::assets::get("pack.json").is_none() {
        crate::load_builtin_pack();
    }
    // the 3D stage behind the title and character screens, from the pack;
    // without one (the first launch ever), a plain backdrop
    let (stage_tx, stage_rx) = channel();
    std::thread::spawn(move || {
        if let Some(stage) = pack_stage() {
            let _ = stage_tx.send(stage);
        }
    });
    let front = Front::new(&settings.recent, &settings.name);
    let mut app = App {
        front,
        stage: None,
        stage_rx: Some(stage_rx),
        stage_loaded: false,
        joining_new: false,
        menu: Menu {
            addr: settings.last_server.clone(),
            name: settings.name.clone(),
            error: None,
        },
        settings,
        gfx: None,
        audio: Audio::new(),
        egui_ctx: crate::theme::new_context(),
        egui_state: None,
        screen: Screen::Menu,
        host: None,
        last_frame: Instant::now(),
        update_note,
        fatal: None,
        autojoined: false,
        cursor: (0.0, 0.0),
        last_shot: Instant::now(),
    };
    for i in 0..app.front.realms.len() {
        let token = app.settings.token;
        app.front.query(i, token, None);
    }

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
        /// the address being joined (for logs; players never see it)
        #[allow(dead_code)]
        label: String,
        since: Instant,
        /// the pack download, for the bar
        progress: Arc<net::Progress>,
    },
    Playing(Box<Play>),
}

struct Chat {
    at: Instant,
    from: Option<String>,
    text: String,
}

struct Play {
    /// the drawn (smooth) height of the ground, for feet
    ground: crate::terrain_mesh::Ground,
    /// the sections the server has streamed (a sparse world)
    world: World,
    /// lights and meshes the world on its own thread (worldwork.rs)
    work: crate::worldwork::WorldWork,
    /// the far view's tiles (farview.rs), those whose meshes are out of
    /// date, and where the camera was when the cut round it was made
    far: crate::farview::FarTiles,
    far_dirty: std::collections::HashSet<[i32; 2]>,
    far_centre: Option<[f32; 2]>,
    /// the land round you has not all streamed in and been meshed yet (a
    /// join, a long teleport): a loading veil is shown
    land_wait: bool,
    /// how far the world streams around you (metres)
    view: f32,
    conn: net::Connection,
    my_id: PlayerId,
    server_version: String,
    url: String,
    player: Player,
    remotes: Remotes,
    /// everyone's animation state
    anims: Anims,
    chat: Vec<Chat>,
    chat_open: bool,
    chat_text: String,
    /// the chat panel shows the combat log instead
    chat_tab_combat: bool,
    /// what happened in fights: newest last
    combat_log: Vec<(Instant, String, Color32)>,
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
    /// ambient effects and critters (life.rs)
    life: crate::life::Life,
    /// what NPCs are busy with: the animation each holds (their routines)
    entity_hold: HashMap<u32, Anim>,
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
    /// big centred title, like "Day 3"
    title: Option<(Instant, String)>,
    /// the creature under the crosshair, nearer than any block
    target_entity: Option<u32>,
    heart_clock: f32,
    /// where we lay down in a bed at night
    sleeping_at: Option<Vec3>,
    my_name: String,
    my_look: dyno_protocol::Appearance,
    hearth: Option<[i32; 3]>,
    class: Option<u16>,
    power: f32,
    power_max: f32,
    /// when each ability is ready again, and the global cooldown
    cooldowns: HashMap<u16, (Instant, f32)>,
    gcd: (Instant, f32),
    /// my cast in progress: ability, start, seconds
    my_cast: Option<(u16, Instant, f32)>,
    class_pick: bool,
    /// camera distance behind you (0: first person)
    zoom: f32,
    equip: Vec<Option<Stack>>,
    skills: Vec<(String, u16, u16)>,
    /// buffs: ability and when they end
    auras: Vec<(u16, Instant)>,
    combo: (u8, Option<u32>),
    /// the named place you are in
    zone: String,
    /// weather from the server (kind, strength), and how far we have blended into it
    weather: (u8, f32),
    weather_mix: f32,
    ambience_clock: f32,
    attacking: Option<u32>,
    /// open tab of the character / bags / spellbook / skills window
    screen_tab: Option<screens::Tab>,
    /// action bar: ability IDs (RECALL, BAR_EAT) on 12 keys
    bar: Vec<Option<u16>>,
    /// floating combat text: when, creature (None: me), text, colour
    combat_text: Vec<(Instant, Option<u32>, String, Color32)>,
    /// WoW-style mouse: buttons held for mouselook, how far the mouse moved while held
    lmb: bool,
    rmb: bool,
    drag: f32,
    /// camera yaw around the character (left-button orbit), radians
    cam_orbit: f32,
    autorun: bool,
    /// last frame's inverse view-projection and screen size, for cursor picking
    inv_vp: Option<glam::Mat4>,
    screen_px: (f32, f32),
    /// the creature or person under the cursor
    hover: Option<u32>,
    /// a world event near you: title, task, progress
    world_event: Option<(String, String, f32)>,
    /// the first-login intro, when it started
    intro: Option<Instant>,
    /// F3: version, address, position, frame rate
    show_info: bool,
    /// my party (empty: none)
    party: Vec<dyno_protocol::PartyMember>,
    /// how tall each creature stands as drawn (for name plates)
    tops: std::cell::RefCell<HashMap<u32, f32>>,
    /// what other players hold in their weapon hand
    wielding: HashMap<u32, u16>,
    /// what other players have equipped (item ids), worn on their figures
    wearing: HashMap<u32, Vec<u16>>,
    /// other players lying dead, and what they are doing (sitting, emoting)
    remote_dead: std::collections::HashSet<u32>,
    remote_hold: HashMap<u32, Anim>,
    /// someone asks to trade
    trade_ask: Option<String>,
    /// the open trade window
    trade: Option<dyno_protocol::TradeView>,
    /// coins typed into the trade window
    trade_money: String,
    /// someone has invited me to their party
    invite: Option<String>,
    /// which copy of the world I am in
    shard: u32,
    level: u8,
    xp: u32,
    xp_next: u32,
    /// banked rested XP
    rested: u32,
    /// an open corpse: id, money, items
    loot: Option<(u32, u32, Vec<dyno_protocol::Stack>)>,
    money: u32,
    max_health: f32,
    quest_log: Vec<dyno_protocol::QuestProgress>,
    quests_done: HashSet<u16>,
    panel: Option<rpg_ui::Panel>,
    /// floating text near the crosshair: when, text, colour
    floaters: Vec<(Instant, String, Color32)>,
    /// the creature or NPC we last hit or picked with Tab
    focus: Option<u32>,
    /// the block we last explained "Needs a stone pickaxe" for
    tier_hint: Option<[i32; 3]>,
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
    minimap: crate::minimap::Minimap,
}

struct ClientEntity {
    kind: EntityKind,
    from: [f32; 3],
    to: [f32; 3],
    at: Instant,
    yaw: f32,
    born: Instant,
    hurt_at: Option<Instant>,
    /// level, health and most health (creatures)
    info: Option<(u8, f32, f32)>,
    /// a rare elite's own name
    name: Option<String>,
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
    /// WoW-style controls: free cursor, hold a button to steer (RPG worlds, not creative).
    fn wow(&self) -> bool {
        self.content.world.protected && !self.creative()
    }
    /// Where the camera looks: the character's facing plus any orbit.
    fn cam_dir(&self) -> Vec3 {
        let (sy, cy) = (self.player.yaw + self.cam_orbit).sin_cos();
        let (sp, cp) = self.player.pitch.sin_cos();
        Vec3::new(-sy * cp, sp, -cy * cp)
    }
    /// The world ray under the mouse cursor.
    fn cursor_ray(&self, cursor: (f64, f64)) -> Option<(Vec3, Vec3)> {
        let inv = self.inv_vp?;
        let (w, h) = self.screen_px;
        let x = (cursor.0 as f32 / w) * 2.0 - 1.0;
        let y = 1.0 - (cursor.1 as f32 / h) * 2.0;
        let near = inv * glam::Vec4::new(x, y, 0.0, 1.0);
        let far = inv * glam::Vec4::new(x, y, 1.0, 1.0);
        let (a, b) = (near.truncate() / near.w, far.truncate() / far.w);
        Some((a, (b - a).normalize_or_zero()))
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
    gravity: f32,
}

/// Motes that drift upward, for a Shade coming apart.
fn spawn_motes(out: &mut Vec<Particle>, at: Vec3, color: [f32; 3], n: usize, seed: u32) {
    let mut r = seed.wrapping_mul(2_654_435_761) | 1;
    let mut rnd = || {
        r ^= r << 13;
        r ^= r >> 17;
        r ^= r << 5;
        r as f32 / u32::MAX as f32
    };
    for _ in 0..n {
        let pos = at + Vec3::new(rnd() - 0.5, rnd() * 1.8, rnd() - 0.5) * Vec3::new(0.7, 1.0, 0.7);
        let vel = Vec3::new(rnd() - 0.5, rnd() * 0.8 + 0.6, rnd() - 0.5) * 1.6;
        let shade = 0.7 + rnd() * 0.6;
        out.push(Particle {
            pos,
            vel,
            color: [color[0] * shade, color[1] * shade, color[2] * shade],
            life: 0.8 + rnd() * 0.9,
            size: 0.04 + rnd() * 0.05,
            gravity: -1.2,
        });
    }
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
            gravity: 18.0,
        });
    }
}

/// What sits on action-bar slot `i` (0-based): an ability, the recall, or food.
enum ActionSlot<'a> {
    Ability(&'a dyno_world::content::Ability),
    Recall,
    Eat,
    Empty,
}

/// Special bar entries besides abilities.
const BAR_EAT: u16 = u16::MAX - 1;
/// Keys of the 12 action-bar slots.
const BAR_KEYS: [&str; 12] = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "="];

/// The default bar: abilities in the order learned, then Recall and Eat.
fn default_bar(p: &Play) -> Vec<Option<u16>> {
    let mut bar = vec![None; 12];
    if let Some(c) = p.class {
        for (i, a) in p
            .content
            .known_abilities(c, p.level)
            .iter()
            .take(10)
            .enumerate()
        {
            bar[i] = Some(a.id);
        }
    }
    bar[10] = Some(dyno_protocol::RECALL);
    bar[11] = Some(BAR_EAT);
    bar
}

fn action_at(p: &Play, i: usize) -> ActionSlot<'_> {
    match p.bar.get(i).copied().flatten() {
        Some(dyno_protocol::RECALL) => ActionSlot::Recall,
        Some(BAR_EAT) => ActionSlot::Eat,
        Some(a) => p
            .content
            .ability(a)
            .filter(|a| Some(a.class) == p.class && a.level <= p.level)
            .map(ActionSlot::Ability)
            .unwrap_or(ActionSlot::Empty),
        None => ActionSlot::Empty,
    }
}

/// Press an action-bar key.
fn use_action(p: &mut Play, i: usize) {
    let target = p.focus.or(p.target_entity).filter(|id| {
        p.entities
            .get(id)
            .is_some_and(|e| matches!(e.kind, EntityKind::Creature(_)))
    });
    let msg = match action_at(p, i) {
        ActionSlot::Ability(a) => ClientMsg::Cast {
            ability: a.id,
            target,
        },
        ActionSlot::Recall => ClientMsg::Recall,
        ActionSlot::Eat => ClientMsg::EatBest,
        ActionSlot::Empty => return,
    };
    if let Some(t) = target {
        p.focus = Some(t);
    }
    // instant abilities: a weapon strike or a flick of the hands, at once
    if let ClientMsg::Cast { ability, .. } = &msg
        && let Some(a) = p.content.ability(*ability)
        && a.cast <= 0.0
    {
        let now = p.anims.now();
        let act = if a.weapon > 0.0 {
            anim::attack_for(p.anims.grip)
        } else {
            Anim::CastRelease
        };
        p.anims.me(now).play_fresh(now, act);
    }
    let _ = p.conn.tx.send(msg);
}

/// One action-bar slot: icon, cooldown sweep, hotkey.
fn draw_action(painter: &egui::Painter, r: Rect, p: &Play, i: usize) {
    let now = Instant::now();
    painter.rect_filled(r, 5.0, Color32::from_black_alpha(150));
    let (icon, cd, usable): (Option<dyno_world::content::ItemId>, f32, bool) = match action_at(p, i)
    {
        ActionSlot::Ability(a) => {
            let item = p.content.item_by_key(&a.icon);
            let own = p
                .cooldowns
                .get(&a.id)
                .map(|(t, total)| t.saturating_duration_since(now).as_secs_f32() / total.max(0.01))
                .unwrap_or(0.0);
            let g = p.gcd.0.saturating_duration_since(now).as_secs_f32() / p.gcd.1.max(0.01);
            let tint = Color32::from_rgb(
                (a.color[0] * 120.0) as u8,
                (a.color[1] * 120.0) as u8,
                (a.color[2] * 120.0) as u8,
            );
            painter.rect_filled(r.shrink(2.0), 4.0, tint.gamma_multiply(0.6));
            (item, own.max(g), p.power + 0.01 >= a.cost)
        }
        ActionSlot::Recall => {
            let own = p
                .cooldowns
                .get(&dyno_protocol::RECALL)
                .map(|(t, total)| t.saturating_duration_since(now).as_secs_f32() / total.max(0.01))
                .unwrap_or(0.0);
            painter.rect_filled(
                r.shrink(2.0),
                4.0,
                Color32::from_rgb(90, 50, 20).gamma_multiply(0.7),
            );
            (
                Some(dyno_world::HEARTH as dyno_world::content::ItemId),
                own,
                true,
            )
        }
        ActionSlot::Eat => {
            let pack = dyno_sim::inventory::Inventory {
                slots: p.pack.clone(),
            };
            let food = dyno_sim::inventory::best_food(&pack, &p.content)
                .and_then(|s| p.pack[s])
                .map(|s| s.item);
            (food, 0.0, food.is_some())
        }
        ActionSlot::Empty => (None, 0.0, false),
    };
    if let Some(item) = icon {
        survival::item_icon(painter, r.shrink(5.0), &p.content, item);
    }
    if !usable && icon.is_some() {
        painter.rect_filled(r, 5.0, Color32::from_rgba_unmultiplied(120, 20, 20, 90));
    }
    if cd > 0.0 {
        let h = r.height() * cd.clamp(0.0, 1.0);
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(r.left(), r.bottom() - h), r.max),
            5.0,
            Color32::from_black_alpha(170),
        );
    }
    let active = p
        .my_cast
        .as_ref()
        .is_some_and(|(a, _, _)| match action_at(p, i) {
            ActionSlot::Ability(x) => x.id == *a,
            ActionSlot::Recall => *a == dyno_protocol::RECALL,
            _ => false,
        });
    let stroke = if active {
        Stroke::new(2.5_f32, Color32::from_rgb(255, 210, 80))
    } else {
        Stroke::new(1.0_f32, Color32::from_rgb(150, 120, 60))
    };
    painter.rect_stroke(r, 5.0, stroke, egui::StrokeKind::Inside);
    painter.text(
        r.left_top() + Vec2::new(4.0, 2.0),
        Align2::LEFT_TOP,
        BAR_KEYS[i],
        FontId::monospace(11.0),
        Color32::from_white_alpha(200),
    );
}

/// A spell bolt: a short trail of sparks from `a` to `b`.
fn spawn_bolt(out: &mut Vec<Particle>, a: Vec3, b: Vec3, color: [f32; 3]) {
    let n = ((b - a).length() * 3.0) as usize + 4;
    for k in 0..n {
        let t = k as f32 / n as f32;
        out.push(Particle {
            pos: a.lerp(b, t),
            vel: (b - a).normalize_or_zero() * 2.0 + Vec3::Y * 0.3,
            color,
            life: 0.15 + 0.35 * t,
            size: 0.09,
            gravity: 0.0,
        });
    }
}

/// A burst of light in a ring.
fn spawn_burst(out: &mut Vec<Particle>, c: Vec3, color: [f32; 3], radius: f32) {
    let n = (radius * 10.0) as usize + 10;
    for k in 0..n {
        let ang = k as f32 / n as f32 * std::f32::consts::TAU;
        let dir = Vec3::new(ang.cos(), 0.0, ang.sin());
        out.push(Particle {
            pos: c + Vec3::Y * 0.6 + dir * 0.4,
            vel: dir * radius * 1.6 + Vec3::Y * 1.0,
            color,
            life: 0.6,
            size: 0.1,
            gravity: -0.5,
        });
    }
}

/// Move a camera back from `eye` along -`dir` by up to `dist`, stopping before walls.
pub(crate) fn pull_back(world: &dyno_world::World, eye: Vec3, dir: Vec3, dist: f32) -> Vec3 {
    let mut d = 0.0;
    let step = 0.1;
    while d < dist {
        let p = eye - dir * (d + step + 0.3);
        if w::is_solid(world.get(p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32)) {
            break;
        }
        d += step;
    }
    eye - dir * d + Vec3::Y * (d * 0.08)
}

/// Stat points from my active buffs.
fn buff_stats(p: &Play) -> dyno_world::content::Stats {
    use dyno_world::content::BuffKind as K;
    let mut s = dyno_world::content::Stats::default();
    for (a, _) in &p.auras {
        if let Some(b) = p.content.ability(*a).and_then(|a| a.buff) {
            let n = b.amount as i16;
            match b.kind {
                K::Strength => s.str += n,
                K::Agility => s.agi += n,
                K::Intellect => s.int += n,
                K::Stamina => s.sta += n,
                _ => {}
            }
        }
    }
    s
}

fn quest_finished(c: &dyno_world::content::Content, q: &dyno_protocol::QuestProgress) -> bool {
    c.quest(q.quest).is_some_and(|x| {
        x.objectives
            .iter()
            .zip(&q.counts)
            .all(|(o, n)| *n >= o.need())
    })
}

/// Draw every creature and NPC, lit by the block light where it stands and
/// animated from what it is doing (see `docs/design/animation.md`).
fn push_creatures(boxes: &mut Vec<Vertex>, p: &Play, book: &mut Anims, clock: f32) {
    let now = book.now();
    let talking = match &p.panel {
        Some(rpg_ui::Panel::Dialog { entity, .. } | rpg_ui::Panel::Vendor { entity, .. }) => {
            Some(*entity)
        }
        _ => None,
    };
    let me = rpg_ui::Me {
        content: &p.content,
        money: p.money,
        level: p.level,
        log: &p.quest_log,
        done: &p.quests_done,
        pos: p.player.pos.into(),
        hearth: p.hearth,
    };
    for (id, e) in &p.entities {
        let pos = p.ground.feet(&p.world, e.pos());
        let step = Vec3::from(e.to) - Vec3::from(e.from);
        let vel = if e.at.elapsed().as_secs_f32() < 0.25 {
            step / 0.1
        } else {
            Vec3::ZERO
        };
        let at = pos + Vec3::Y;
        let light = p.light.get(
            at.x.floor() as i32,
            at.y.floor() as i32,
            at.z.floor() as i32,
        ) as f32
            / 15.0;
        if let EntityKind::Npc(n) = e.kind
            && let Some(npc) = p.content.npc(n)
        {
            let base = if talking == Some(*id) {
                Anim::Talk
            } else {
                anim::locomotion(vel, e.yaw, 4.0)
                    .or(p.entity_hold.get(id).copied())
                    .unwrap_or(Anim::Idle)
            };
            let speed = Vec3::new(vel.x, 0.0, vel.z).length();
            let (a, _) = book.entity(*id, now);
            a.drive_at(now, base, pos, models::person_step(speed));
            let look = models::Look::npc(&npc.key, npc.color);
            models::push_person_anim(boxes, pos, e.yaw, 0.0, &look, light.max(0.35), a, now);
            if let Some(m) = me.marker(n) {
                let turn = p.player.yaw + p.cam_orbit;
                push_marker(
                    boxes,
                    pos + Vec3::Y * (2.45 + 0.06 * (now * 2.2).sin()),
                    turn,
                    m,
                );
            }
            continue;
        }
        let EntityKind::Creature(cid) = e.kind else {
            continue;
        };
        let Some(c) = p.content.creature(cid) else {
            continue;
        };
        let flash = e
            .hurt_at
            .map(|t| (1.0 - t.elapsed().as_secs_f32() / 0.12).max(0.0))
            .unwrap_or(0.0);
        // size: the creature's own scale, and rare elites are a quarter larger
        let scale = c.scale * if e.name.is_some() { 1.25 } else { 1.0 };
        // in a fight: hit lately, or a hostile that has you targeted
        let hurt = e.info.is_some_and(|(_, h, max)| h < max);
        let fighting = (hurt
            && e.hurt_at
                .is_some_and(|t| t.elapsed() < Duration::from_secs(8)))
            || (c.hostile && p.attacking == Some(*id));
        let base = anim::locomotion(vel, e.yaw, c.speed.max(0.5)).unwrap_or(if fighting {
            Anim::Ready
        } else {
            Anim::Idle
        });
        let speed = Vec3::new(vel.x, 0.0, vel.z).length();
        let (a, _) = book.entity(*id, now);
        a.drive_at(
            now,
            base,
            pos,
            models::creature_step(c.model, c.variant, speed, scale),
        );
        let start = boxes.len();
        models::push_creature(
            boxes, c.model, c.variant, pos, e.yaw, c.color, light, flash, scale, a, now,
        );
        // how tall it stands, so its name floats just over it
        let top = boxes[start..]
            .iter()
            .map(|v| v.pos[1])
            .fold(pos.y, f32::max);
        p.tops.borrow_mut().insert(*id, top - pos.y);
    }
    push_corpses(boxes, p, book, clock);
}

/// A quest marker over an NPC's head, built of small shining cubes and
/// turned to the camera: a gold "!" (a quest to take), a gold "?" (one to
/// hand in), a grey "?" (one of yours, not finished).
fn push_marker(out: &mut Vec<Vertex>, at: Vec3, turn: f32, m: rpg_ui::Marker) {
    const BANG: [&str; 7] = [".##.", ".##.", ".##.", ".##.", ".##.", "....", ".##."];
    const ASK: [&str; 7] = [".##.", "#..#", "...#", "..#.", ".#..", "....", ".#.."];
    let (rows, rgb) = match m {
        rpg_ui::Marker::Available => (BANG, [1.0, 0.82, 0.16]),
        rpg_ui::Marker::Ready => (ASK, [1.0, 0.82, 0.16]),
        rpg_ui::Marker::Waiting => (ASK, [0.66, 0.66, 0.66]),
    };
    let (s, c) = turn.sin_cos();
    let right = Vec3::new(c, 0.0, -s);
    let axes = [right, Vec3::Y, Vec3::new(s, 0.0, c)];
    let u = 0.085;
    for (row, line) in rows.iter().enumerate() {
        for (col, ch) in line.bytes().enumerate() {
            if ch != b'#' {
                continue;
            }
            let centre = at + right * ((col as f32 - 1.5) * u) + Vec3::Y * ((6 - row) as f32 * u);
            render::push_oriented_box(out, centre, axes, Vec3::splat(u * 0.5), rgb, true, 1.0);
        }
    }
}

/// An NPC's routine says what it does now: a looping animation is held
/// until the next one (the first of the list lets it go), a one-shot is
/// returned to be played.
fn entity_emote(p: &mut Play, entity: u32, n: u8) -> Option<Anim> {
    match Anim::ALL.get(n as usize).copied() {
        Some(Anim::Idle) | None => {
            p.entity_hold.remove(&entity);
            None
        }
        Some(a) if a.one_shot() => Some(a),
        Some(a) => {
            p.entity_hold.insert(entity, a);
            None
        }
    }
}

/// Ambient life round the camera: smoke, fireflies, birds, deer (life.rs).
/// Critters go into `boxes`; the effects into the renderer's own lists.
#[allow(clippy::too_many_arguments)]
fn push_life(
    p: &mut Play,
    gfx: &mut Gfx,
    boxes: &mut Vec<Vertex>,
    eye: Vec3,
    dir: Vec3,
    time: f32,
    dt: f32,
    quality: u8,
    settle: bool,
) {
    let mut people: Vec<Vec3> = vec![p.player.pos];
    people.extend(
        p.remotes
            .iter()
            .filter(|(_, r)| !r.far)
            .map(|(_, r)| Vec3::from(r.current().pos)),
    );
    let scene = crate::life::Scene {
        world: &p.world,
        light: &p.light,
        content: &p.content,
        hearth: p.hearth,
        time,
        eye,
        dir,
        people: &people,
    };
    let clock = p.anims.now() as f64 + 600.0;
    if settle {
        p.life.settle(&scene, clock, quality);
    }
    let mut out = crate::life::Out {
        boxes,
        fx: &mut gfx.fx,
        glow: &mut gfx.fx_glow,
    };
    p.life.frame(&scene, clock, dt, quality, &mut out);
}

/// The fallen: a creature that died a moment ago plays its death, one found
/// lying there is simply dead; a glint of gold while it still holds loot.
fn push_corpses(boxes: &mut Vec<Vertex>, p: &Play, book: &mut Anims, clock: f32) {
    let now = book.now();
    for (id, e) in &p.entities {
        let EntityKind::Corpse { creature, loot } = e.kind else {
            continue;
        };
        let Some(c) = p.content.creature(creature) else {
            continue;
        };
        let pos = p.ground.feet(&p.world, e.pos());
        let at = pos + Vec3::Y * 0.5;
        let light = p.light.get(
            at.x.floor() as i32,
            at.y.floor() as i32,
            at.z.floor() as i32,
        ) as f32
            / 15.0;
        let fresh = book.fresh_corpse(pos, now);
        let (a, new) = book.entity(*id, now);
        let base = if new && fresh {
            Anim::Death
        } else if new {
            Anim::Dead
        } else {
            a.base()
        };
        a.drive_at(now, base, pos, 1.0);
        let dying = base == Anim::Death && a.base_time(now) < 1.5;
        models::push_creature(
            boxes,
            c.model,
            c.variant,
            pos,
            e.yaw,
            [c.color[0] * 0.75, c.color[1] * 0.75, c.color[2] * 0.75],
            light,
            0.0,
            c.scale,
            a,
            now,
        );
        if loot && !dying {
            // a few motes of gold drifting up
            for k in 0..5 {
                let ph = clock * 0.6 + k as f32 * 0.2 + *id as f32 * 0.13;
                let f = ph.fract();
                let a = k as f32 * 1.3 + clock * 0.4;
                let q = pos
                    + Vec3::new(
                        a.cos() * 0.45 * c.scale,
                        0.15 + f * 1.1,
                        a.sin() * 0.45 * c.scale,
                    );
                let s = 0.05 * (1.0 - f) + 0.015;
                render::push_box(
                    boxes,
                    q - Vec3::splat(s),
                    q + Vec3::splat(s),
                    [1.0, 0.85, 0.35],
                    1.0,
                );
            }
        }
    }
}

/// Your own figure's state, from your movement, casting, fighting and the
/// emote you hold.
fn my_anim(p: &Play, book: &mut Anims, look: &models::Look, now: f32) -> Anim {
    book.grip = models::grip(look);
    let pl = &p.player;
    let wet = |y: f32| {
        p.world.get(
            pl.pos.x.floor() as i32,
            (pl.pos.y + y).floor() as i32,
            pl.pos.z.floor() as i32,
        ) == w::WATER
    };
    let swimming = wet(0.9) && wet(0.3);
    book.casts
        .retain(|_, (_, at, secs)| now - *at < *secs + 0.5);
    let airborne = !pl.on_ground && !pl.flying && !swimming;
    // the moment of leaving the ground, and of landing
    if airborne && book.grounded {
        book.air_since = now;
        if pl.vel.y > 0.5 {
            book.me(now).play(now, Anim::JumpStart);
        }
    } else if !airborne && !book.grounded && now - book.air_since > 0.3 {
        book.me(now).play(now, Anim::JumpLand);
    }
    book.grounded = !airborne;
    let moving = anim::locomotion(pl.vel, pl.yaw, pl.run_speed);
    if moving.is_some() || airborne || p.dead.is_some() {
        book.hold = None;
    }
    if p.dead.is_some() {
        Anim::Death
    } else if swimming {
        Anim::Swim
    } else if airborne {
        if pl.vel.y < -7.0 || now - book.air_since > 0.9 {
            Anim::Fall
        } else {
            Anim::JumpAir
        }
    } else if let Some(m) = moving {
        m
    } else if let Some((a, at, secs)) = p.my_cast
        && at.elapsed().as_secs_f32() < secs + 0.5
    {
        cast_anim(&p.content, a)
    } else if p.loot.is_some() {
        Anim::Kneel
    } else if let Some(h) = book.hold {
        h
    } else if p.attacking.is_some() {
        Anim::Ready
    } else {
        Anim::Idle
    }
}

/// How a figure looks while casting an ability: gathering kneels, heals and
/// recall raise the arms, the rest gather power in front.
fn cast_anim(content: &dyno_world::content::Content, ability: u16) -> Anim {
    if ability == dyno_protocol::GATHER {
        return Anim::Kneel;
    }
    match content.ability(ability) {
        Some(a) if a.damage[1] > 0.0 => Anim::CastDirected,
        Some(a) if a.heal[1] > 0.0 || a.recall || a.hot[0] > 0.0 => Anim::CastOmni,
        Some(_) => Anim::Channel,
        None => Anim::CastOmni,
    }
}

/// Another player's state, from their motion and what they are casting or
/// breaking.
fn remote_anim(p: &Play, book: &mut Anims, id: u32, r: &Remote, now: f32) -> Anim {
    let pose = r.current();
    let vel = r.velocity();
    // in the air: nothing under the feet, and rising or falling
    let under = p.world.get(
        pose.pos[0].floor() as i32,
        (pose.pos[1] - 0.3).floor() as i32,
        pose.pos[2].floor() as i32,
    );
    let high = under == w::AIR && vel.y.abs() > 0.5;
    if p.remote_dead.contains(&id) {
        return Anim::Dead;
    }
    if book.digging.contains_key(&id) {
        // swing after swing while they break a block
        let a = book.player(id, now);
        if a.action(now).is_none() {
            a.play(now, Anim::Attack1H);
        }
    }
    if high {
        if vel.y < -6.0 {
            Anim::Fall
        } else {
            Anim::JumpAir
        }
    } else if let Some(m) = anim::locomotion(vel, pose.yaw, 4.4) {
        m
    } else if let Some(a) = book.casting(id, now) {
        cast_anim(&p.content, a)
    } else if let Some(h) = p.remote_hold.get(&id) {
        *h
    } else {
        Anim::Idle
    }
}

/// The colour of a ranged auto-attack's shot (`dyno-server` gear.rs).
const AUTO_SHOT: [f32; 3] = [0.9, 0.8, 0.6];

/// A spell or shot left `from`: whoever stands there plays its release, or
/// its shot (a creature, its attack).
fn animate_caster(p: &mut Play, from: Vec3, color: [f32; 3]) {
    let now = p.anims.now();
    let shot = color == AUTO_SHOT;
    if from.distance(p.player.eye()) < 1.6 {
        let a = if shot {
            anim::attack_for(p.anims.grip)
        } else {
            Anim::CastRelease
        };
        p.anims.me(now).play_fresh(now, a);
        p.anims.last_swing = now;
        return;
    }
    for (id, r) in p.remotes.iter().filter(|(_, r)| !r.far) {
        let at = Vec3::from(r.current().pos);
        if from.distance(at + Vec3::Y * 1.4) < 1.6 {
            let a = if shot { Anim::Shoot } else { Anim::CastRelease };
            p.anims.player(*id, now).play_fresh(now, a);
            return;
        }
    }
    let caster = p
        .entities
        .iter()
        .filter_map(|(id, e)| match e.kind {
            EntityKind::Creature(c) => Some((*id, e.pos().distance(from), c)),
            _ => None,
        })
        .filter(|(_, d, _)| *d < 2.5)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((id, _, cid)) = caster
        && let Some(c) = p.content.creature(cid)
    {
        let swing = models::creature_attack(c.model, c.variant);
        p.anims.play_entity(id, now, swing);
    }
}

/// You were hit: you flinch, and the nearest hostile in reach strikes.
fn animate_blow(p: &mut Play) {
    let now = p.anims.now();
    p.anims.me(now).play_fresh(now, Anim::HitReact);
    let me = p.player.pos;
    let striker = p
        .entities
        .iter()
        .filter_map(|(id, e)| match e.kind {
            EntityKind::Creature(c) => p
                .content
                .creature(c)
                .filter(|c| c.hostile)
                .map(|c| (*id, e.pos().distance(me) - c.scale, c)),
            _ => None,
        })
        .filter(|(_, d, _)| *d < 3.0)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((id, _, c)) = striker {
        let swing = models::creature_attack(c.model, c.variant);
        p.anims.play_entity(id, now, swing);
    }
}

/// The creature or person along a ray (the mouse cursor), nearest first,
/// unless a block is in the way.
fn pick_entity_ray(p: &Play, o: Vec3, dir: Vec3, max: f32) -> Option<u32> {
    let wall = game::raycast(&p.world, o, dir, max).map(|t| {
        let c = Vec3::new(
            t.block[0] as f32 + 0.5,
            t.block[1] as f32 + 0.5,
            t.block[2] as f32 + 0.5,
        );
        (c - o).length() - 0.5
    });
    let mut best: Option<(u32, f32)> = None;
    for (id, e) in &p.entities {
        let h = match e.kind {
            EntityKind::Npc(_) => 2.0,
            EntityKind::Corpse { .. } => 0.7,
            EntityKind::Creature(c) => match p.content.creature(c).map(|c| (c.model, c.scale)) {
                Some((dyno_world::content::Model::Humanoid, s)) => 2.0 * s,
                Some((_, s)) => 1.3 * s,
                None => 1.3,
            },
            _ => continue,
        };
        let pos = e.pos();
        let (lo, hi) = (
            pos - Vec3::new(0.55, 0.0, 0.55),
            pos + Vec3::new(0.55, h, 0.55),
        );
        let mut t0 = 0.0f32;
        let mut t1 = max;
        let mut hit = true;
        for a in 0..3 {
            let (oa, d) = (o[a], dir[a]);
            if d.abs() < 1e-6 {
                if oa < lo[a] || oa > hi[a] {
                    hit = false;
                    break;
                }
                continue;
            }
            let (mut ta, mut tb) = ((lo[a] - oa) / d, (hi[a] - oa) / d);
            if ta > tb {
                std::mem::swap(&mut ta, &mut tb);
            }
            t0 = t0.max(ta);
            t1 = t1.min(tb);
            if t0 > t1 {
                hit = false;
                break;
            }
        }
        if hit && wall.is_none_or(|w| t0 < w) && best.is_none_or(|(_, bt)| t0 < bt) {
            best = Some((*id, t0));
        }
    }
    best.map(|(id, _)| id)
}

/// The creature under the crosshair, if it is nearer than the targeted block.
fn pick_entity(p: &Play) -> Option<u32> {
    let eye = p.player.eye();
    let dir = p.player.look_dir();
    let block_t = p
        .target
        .as_ref()
        .map(|t| {
            let c = Vec3::new(
                t.block[0] as f32 + 0.5,
                t.block[1] as f32 + 0.5,
                t.block[2] as f32 + 0.5,
            );
            (c - eye).length() - 0.5
        })
        .unwrap_or(f32::MAX);
    let mut best: Option<(u32, f32)> = None;
    for (id, e) in &p.entities {
        let h = match e.kind {
            EntityKind::Npc(_) => 2.0,
            EntityKind::Creature(cid) => match p.content.creature(cid).map(|c| c.model) {
                Some(dyno_world::content::Model::Humanoid) => 2.0,
                _ => 1.25,
            },
            _ => continue,
        };
        let pos = e.pos();
        let lo = pos - Vec3::new(0.45, 0.0, 0.45);
        let hi = pos + Vec3::new(0.45, h, 0.45);
        // slab test
        let mut t0 = 0.0f32;
        let mut t1 = 4.5f32;
        let mut hit = true;
        for a in 0..3 {
            let (o, d) = (eye[a], dir[a]);
            if d.abs() < 1e-6 {
                if o < lo[a] || o > hi[a] {
                    hit = false;
                    break;
                }
                continue;
            }
            let (mut ta, mut tb) = ((lo[a] - o) / d, (hi[a] - o) / d);
            if ta > tb {
                std::mem::swap(&mut ta, &mut tb);
            }
            t0 = t0.max(ta);
            t1 = t1.min(tb);
            if t0 > t1 {
                hit = false;
                break;
            }
        }
        if hit && t0 < block_t && best.is_none_or(|(_, bt)| t0 < bt) {
            best = Some((*id, t0));
        }
    }
    best.map(|(id, _)| id)
}

/// Sky, sun and light for a time of day (0.0 sunrise, 0.25 noon, 0.5 sunset,
/// 0.75 midnight) in no particular place: the title screen and model sheets.
/// In the world, `sky::sky_at` adds the mood of where you stand.
pub(crate) fn sky(t: f32) -> render::Sky {
    crate::sky::sky_at(t, None)
}

/// Weather on a sky: clouds close in, colours grey out, light dims.
fn weathered(mut s: render::Sky, kind: u8, strength: f32) -> render::Sky {
    let (cover, grey, dim) = match kind {
        1 => (0.45, 0.25, 0.12),
        2 => (0.95, 0.55, 0.35),
        3 => (0.6, 0.6, 0.18),
        _ => (0.0, 0.0, 0.0),
    };
    let k = strength.clamp(0.0, 1.0);
    let grey_of = |c: [f32; 3], g: f32| {
        let l = c[0] * 0.3 + c[1] * 0.59 + c[2] * 0.11;
        [
            c[0] + (l - c[0]) * g,
            c[1] + (l - c[1]) * g,
            c[2] + (l - c[2]) * g,
        ]
    };
    s.cover = cover * k;
    s.top = grey_of(s.top, grey * k);
    s.horizon = grey_of(s.horizon, grey * k * 1.2);
    s.light = grey_of(s.light, grey * k * 0.6).map(|v| v * (1.0 - dim * k));
    s
}

fn material_of(b: w::BlockId) -> w::Material {
    w::block(b).material
}

struct Tunnel {
    child: Child,
    url: Arc<Mutex<Option<String>>>,
    error: Arc<Mutex<Option<String>>>,
}

/// A world this game hosts (dev builds with the `host` feature; the player
/// client has no server in it and never makes one).
#[cfg_attr(not(feature = "host"), allow(dead_code))]
struct Host {
    #[cfg(feature = "host")]
    handle: dyno_server::Handle,
    port: u16,
    lan: Option<String>,
    tunnel: Option<Tunnel>,
}

impl Host {
    fn players(&self) -> usize {
        #[cfg(feature = "host")]
        return self.handle.server.player_count();
        #[cfg(not(feature = "host"))]
        0
    }

    fn stop(mut self) {
        if let Some(t) = self.tunnel.as_mut() {
            let _ = t.child.kill();
        }
        #[cfg(feature = "host")]
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
    /// cursor position in physical pixels
    cursor: (f64, f64),
    /// title, realms, character select and creation
    front: Front,
    stage: Option<Stage>,
    stage_rx: Option<Receiver<Stage>>,
    stage_loaded: bool,
    /// the character being joined is brand new (play the intro)
    joining_new: bool,
}

/// The small world behind the front-end screens.
struct Stage {
    world: dyno_world::World,
    light: Light,
    hearth: [i32; 3],
}

/// The pack's entry points (`pack.json`).
fn pack_entry(name: &str) -> Option<String> {
    let meta: serde_json::Value = serde_json::from_str(&crate::assets::text("pack.json")?).ok()?;
    Some(meta.get("entries")?.get(name)?.as_str()?.to_string())
}

/// The title stage: a small saved world in the pack, around a hearth.
fn pack_stage() -> Option<Stage> {
    let bytes = crate::assets::get(&pack_entry("stage")?)?;
    let mut world = World::decode(&bytes).ok()?;
    let hearth = world.ensure_spawn_hearth(None);
    let light = Light::new(&world);
    Some(Stage {
        world,
        light,
        hearth,
    })
}

/// Words the game shows that belong to its content (`ui/strings.json` in
/// the pack): `section.key` as text, or empty while the pack is absent.
pub(crate) fn pack_text(section: &str, key: &str) -> String {
    pack_value(section, key)
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// A list of lines from the pack's strings.
pub(crate) fn pack_lines(section: &str, key: &str) -> Vec<String> {
    pack_value(section, key)
        .and_then(|v| {
            v.as_array().map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str().map(str::to_string))
                    .collect()
            })
        })
        .unwrap_or_default()
}

fn pack_value(section: &str, key: &str) -> Option<serde_json::Value> {
    let path = pack_entry("strings").unwrap_or_else(|| "ui/strings.json".into());
    let all: serde_json::Value = serde_json::from_str(&crate::assets::text(&path)?).ok()?;
    all.get(section)?.get(key).cloned()
}

#[cfg(feature = "host")]
fn lan_ip() -> Option<String> {
    let s = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:80").ok()?;
    Some(s.local_addr().ok()?.ip().to_string())
}

#[cfg(feature = "host")]
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

/// Sections that arrived: kept now for the game, and sent to the world
/// thread to be lit and meshed.
fn sections_in(p: &mut Play, list: Vec<([i32; 3], Vec<u8>)>) {
    let ready = list
        .into_iter()
        .filter_map(|(s, d)| World::prepare_section(&d).ok().map(|p| (s, p)))
        .collect();
    sections_ready(p, ready);
}

/// Sections decoded and packed already (on the network thread): stored in
/// the game's world and the world thread's, which share them.
fn sections_ready(p: &mut Play, list: Vec<([i32; 3], dyno_world::SectionData)>) {
    let mut sent = Vec::with_capacity(list.len());
    for (s, data) in list {
        if p.world.put_prepared(s, data.clone()).is_ok() {
            let lo = [s[0] * 16, s[1] * 16, s[2] * 16];
            p.minimap
                .region_changed(lo, [lo[0] + 15, lo[1] + 15, lo[2] + 15]);
            sent.push((s, data));
        }
    }
    if !sent.is_empty() {
        p.ground.clear();
    }
    p.work.put(sent);
}

/// Sections the server let go of: forget them and stop drawing them.
fn sections_out(p: &mut Play, mut gfx: Option<&mut Gfx>, list: Vec<[i32; 3]>) {
    for s in &list {
        p.world.drop_section(*s);
        p.light.forget(*s);
        if let Some(g) = gfx.as_deref_mut() {
            g.section_gone(*s);
        }
    }
    p.work.drop_sections(list);
}

/// Take in what the world thread made: its light at once, then meshes
/// uploaded until `budget` is spent (at least one). Returns the time spent.
fn world_uploads(p: &mut Play, gfx: Option<&mut Gfx>, budget: Duration) -> Duration {
    let t0 = Instant::now();
    p.work.poll(&mut p.light);
    let Some(g) = gfx else {
        p.work.queue.clear();
        return t0.elapsed();
    };
    while let Some((s, geo)) = p.work.queue.pop_front() {
        // dropped since it was made: nothing to draw
        if p.world.has_section(s) {
            g.put_mesh(s, geo);
        }
        if t0.elapsed() >= budget {
            break;
        }
    }
    t0.elapsed()
}

/// `DYNO_SHOT_RUN="x0,z0,x1,z1[,speed]"`: a camera flight for `dyno shot`.
pub(crate) fn shot_run() -> Option<([f32; 2], [f32; 2], f32)> {
    let v: Vec<f32> = std::env::var("DYNO_SHOT_RUN")
        .ok()?
        .split(',')
        .filter_map(|x| x.trim().parse().ok())
        .collect();
    (v.len() >= 4).then(|| ([v[0], v[1]], [v[2], v[3]], v.get(4).copied().unwrap_or(7.0)))
}

/// Far tiles that arrived: kept, and their meshes (and their neighbours',
/// whose sides face them) remade.
fn far_in(p: &mut Play, list: Vec<([i32; 2], Vec<u8>)>) {
    for (t, data) in list {
        if let Ok(tile) = dyno_world::far::FarTile::decode(&data) {
            p.far.insert(t, tile);
            for d in [[0, 0], [1, 0], [-1, 0], [0, 1], [0, -1]] {
                p.far_dirty.insert([t[0] + d[0], t[1] + d[1]]);
            }
        }
    }
}

fn far_out(p: &mut Play, mut gfx: Option<&mut Gfx>, list: Vec<[i32; 2]>) {
    for t in list {
        p.far.remove(&t);
        p.far_dirty.remove(&t);
        if let Some(g) = gfx.as_deref_mut() {
            g.drop_far(t);
        }
    }
}

/// How far the far view is drawn (its fog ends here).
const FAR_SEE: f32 = 900.0;

/// Mesh far tiles that changed, or whose cut round the camera moved, nearest
/// first, until `budget` is spent. Returns the time spent.
fn far_uploads(p: &mut Play, gfx: &mut Gfx, cam: Vec3, budget: Duration) -> Duration {
    let t0 = Instant::now();
    let tile = dyno_world::far::TILE as f32;
    // the near view is drawn within this; the far view outside it
    let cut = (p.view - 16.0).max(32.0);
    let here = [cam.x, cam.z];
    if p.far_centre
        .is_none_or(|c| (c[0] - here[0]).hypot(c[1] - here[1]) > 16.0)
    {
        p.far_centre = Some(here);
        // the tiles the edge of the cut can touch
        for t in p.far.keys() {
            let c = [
                t[0] as f32 * tile + tile * 0.5,
                t[1] as f32 * tile + tile * 0.5,
            ];
            if (c[0] - here[0]).hypot(c[1] - here[1]) < cut + tile * 1.5 {
                p.far_dirty.insert(*t);
            }
        }
    }
    if p.far_dirty.is_empty() {
        return t0.elapsed();
    }
    let mut todo: Vec<[i32; 2]> = p.far_dirty.iter().copied().collect();
    todo.sort_by(|a, b| {
        let d = |t: &[i32; 2]| (t[0] as f32 * tile - here[0]).hypot(t[1] as f32 * tile - here[1]);
        d(a).total_cmp(&d(b))
    });
    let centre = p.far_centre.unwrap_or(here);
    for t in todo {
        p.far_dirty.remove(&t);
        if p.far.contains_key(&t) {
            let verts = crate::farview::mesh_tile(&p.far, t, centre, cut);
            gfx.put_far(t, &verts);
        }
        if t0.elapsed() >= budget {
            break;
        }
    }
    t0.elapsed()
}

/// The loading veil waits for the land within this many metres of you.
const LAND_RADIUS: f32 = 24.0;

/// Whether the land round `at` is here and drawn: every section of the
/// columns within `r` metres (the loading veil waits for it).
fn land_ready(p: &Play, gfx: &Gfx, at: Vec3, r: f32) -> bool {
    net::sections_around(&p.world, at.into(), r)
        .into_iter()
        .all(|s| p.world.has_section(s) && gfx.is_built(s))
}

/// Play state for a freshly joined server.
fn new_play(j: Joined) -> Play {
    // the moods of its places (mood.rs)
    crate::mood::set_world(j.hearth, &j.content.areas);
    let rpg_world = j.content.world.protected;
    // worked out on the world thread and copied back as it changes
    let light = Light::blank(&j.world);
    // the server names it: the world around it may not have streamed in
    let hearth = Some(j.hearth);
    Play {
        player: Player::new(j.spawn),
        anims: Anims::default(),
        remotes: j
            .players
            .iter()
            .filter(|i| i.id != j.your_id)
            .map(|i| {
                let mut r = Remote::new(i.name.clone(), i.pose);
                r.look = i.look;
                r.color = i.color;
                (i.id, r)
            })
            .collect(),
        chat: vec![Chat {
            at: Instant::now(),
            from: None,
            text: j.motd.clone(),
        }],
        view: j.view,
        work: crate::worldwork::WorldWork::start(j.world.clone()),
        far: Default::default(),
        far_dirty: Default::default(),
        far_centre: None,
        world: j.world,
        land_wait: true,
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
        life: Default::default(),
        entity_hold: HashMap::new(),
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
        minimap: crate::minimap::Minimap::new(),
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
        title: None,
        target_entity: None,
        heart_clock: 0.0,
        sleeping_at: None,
        my_name: j.name.clone(),
        my_look: Default::default(),
        class: None,
        power: 0.0,
        power_max: 0.0,
        cooldowns: HashMap::new(),
        gcd: (Instant::now(), 0.0),
        my_cast: None,
        class_pick: false,
        zoom: if rpg_world { 5.0 } else { 0.0 },
        equip: vec![None; 17],
        skills: Vec::new(),
        auras: Vec::new(),
        combo: (0, None),
        zone: String::new(),
        weather: (0, 0.0),
        weather_mix: 0.0,
        ambience_clock: 0.0,
        attacking: None,
        screen_tab: None,
        bar: Vec::new(),
        combat_text: Vec::new(),
        lmb: false,
        rmb: false,
        drag: 0.0,
        cam_orbit: 0.0,
        autorun: false,
        inv_vp: None,
        screen_px: (1.0, 1.0),
        hover: None,
        world_event: None,
        intro: None,
        chat_tab_combat: false,
        combat_log: Vec::new(),
        show_info: false,
        party: Vec::new(),
        invite: None,
        tops: Default::default(),
        wielding: HashMap::new(),
        wearing: HashMap::new(),
        remote_dead: Default::default(),
        remote_hold: HashMap::new(),
        trade_ask: None,
        trade: None,
        trade_money: String::new(),
        shard: 0,
        ground: Default::default(),
        hearth,
        level: 1,
        xp: 0,
        xp_next: 400,
        rested: 0,
        loot: None,
        money: 0,
        max_health: 20.0,
        quest_log: Vec::new(),
        quests_done: HashSet::new(),
        panel: None,
        floaters: Vec::new(),
        focus: None,
        tier_hint: None,
        toasts: Vec::new(),
        fall_from: None,
        air: 10.0,
        drown_clock: 0.0,
        sent_held: 99,
    }
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
            // WoW mode: the cursor stays free; mouselook is held with a button
            if p.wow() && on && !(p.lmb || p.rmb) {
                p.captured = false;
                return;
            }
            if p.wow() && !on {
                let _ = win.set_cursor_grab(CursorGrabMode::None);
                win.set_cursor_visible(true);
                p.captured = false;
                return;
            }
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

    /// Hold a mouse button to steer: hide and lock the cursor while it is held.
    fn set_mouselook(&mut self, on: bool) {
        let Some(win) = self.window().cloned() else {
            return;
        };
        if let Screen::Playing(p) = &mut self.screen {
            if on {
                let _ = win
                    .set_cursor_grab(CursorGrabMode::Locked)
                    .or_else(|_| win.set_cursor_grab(CursorGrabMode::Confined));
                win.set_cursor_visible(false);
                p.captured = true;
            } else {
                let _ = win.set_cursor_grab(CursorGrabMode::None);
                win.set_cursor_visible(true);
                p.captured = false;
            }
        }
    }

    /// A WoW-style click on the world: left selects, right acts.
    fn world_click(&mut self, button: MouseButton) {
        let Screen::Playing(p) = &mut self.screen else {
            return;
        };
        let hover = p
            .hover
            .and_then(|id| p.entities.get(&id).map(|e| (id, e.kind)));
        match (button, hover) {
            (MouseButton::Left, Some((_, EntityKind::Corpse { .. }))) => {}
            (MouseButton::Left, Some((id, _))) => p.focus = Some(id),
            (MouseButton::Right, Some((id, EntityKind::Npc(_)))) => {
                p.focus = Some(id);
                let _ = p.conn.tx.send(ClientMsg::Talk { entity: id });
            }
            (MouseButton::Right, Some((id, EntityKind::Corpse { .. }))) => {
                let all =
                    p.keys.contains(&KeyCode::ShiftLeft) || p.keys.contains(&KeyCode::ShiftRight);
                let _ = p.conn.tx.send(ClientMsg::Loot { id, all });
            }
            (MouseButton::Right, Some((id, EntityKind::Creature(_)))) => {
                p.focus = Some(id);
                let _ = p.conn.tx.send(ClientMsg::AutoAttack { target: Some(id) });
                p.swing = 1.0;
            }
            (MouseButton::Right, _) => {
                let Some(t) = p.target.as_ref() else { return };
                let pos = t.block;
                let b = p.world.get(pos[0], pos[1], pos[2]);
                if p.content.rule(b).respawn > 0 {
                    let _ = p.conn.tx.send(ClientMsg::Gather { pos });
                } else if matches!(b, w::CHEST | w::BED) {
                    let _ = p.conn.tx.send(ClientMsg::Interact { pos });
                } else if matches!(b, w::CRAFTING_TABLE | w::FURNACE | w::ANVIL) {
                    p.picker_open = true;
                    p.selected = None;
                }
            }
            _ => {}
        }
    }

    /// Enter a realm as a character (new if `create` is set).
    fn start_join_as(&mut self, addr: String, name: String, create: Option<proto::NewCharacter>) {
        let name = proto::clean_name(&name);
        self.settings.name = name.clone();
        self.settings.remember_server(&addr);
        let _ = self.settings.save();
        self.joining_new = create.is_some();
        self.front.error = None;
        let (tx, rx) = channel();
        let a = addr.clone();
        let token = self.settings.token;
        let progress = Arc::new(net::Progress::default());
        let opts = net::JoinOptions {
            prepare_sections: true,
            token,
            create,
            progress: Some(progress.clone()),
            ..Default::default()
        };
        std::thread::spawn(move || {
            let _ = tx.send(net::connect_with(&a, &name, opts, Duration::from_secs(15)));
        });
        self.screen = Screen::Joining {
            rx,
            label: addr,
            since: Instant::now(),
            progress,
        };
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
        let progress = Arc::new(net::Progress::default());
        let opts = net::JoinOptions {
            prepare_sections: true,
            token,
            progress: Some(progress.clone()),
            ..Default::default()
        };
        std::thread::spawn(move || {
            let _ = tx.send(net::connect_with(&a, &name, opts, Duration::from_secs(10)));
        });
        self.menu.error = None;
        self.screen = Screen::Joining {
            rx,
            label: addr,
            since: Instant::now(),
            progress,
        };
    }

    #[cfg(not(feature = "host"))]
    fn start_host(&mut self) {
        self.menu.error = Some("This build of the game cannot host a world.".into());
        self.front.error = self.menu.error.clone();
    }

    #[cfg(feature = "host")]
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
        if let Some(rx) = &self.stage_rx
            && let Ok(st) = rx.try_recv()
        {
            self.stage = Some(st);
            self.stage_rx = None;
            self.stage_loaded = false;
        }
        // joining
        if let Screen::Joining { rx, .. } = &self.screen {
            match rx.try_recv() {
                Ok(Ok(j)) => {
                    let mut p = new_play(j);
                    if self.joining_new {
                        p.intro = Some(Instant::now());
                    }
                    self.stage_loaded = false;
                    if !p.content.world.protected {
                        p.chat.push(Chat { at: Instant::now(), from: None, text: if p.creative() { "Creative mode. Click to play. WASD move, F fly, left click break, right click place, E all items, G G then V to fill, Ctrl+Z undo, T chat, /help.".into() } else { "Survival mode. Hold left click to break blocks, E opens your pack and recipes, right click places or eats, Q throws, T chat, /help. Punch a tree to start, and make light before night falls.".into() } });
                    }
                    // the join returned only once blocks.json and every file
                    // needed before play arrived: take this world's texture
                    // layers in before its meshes are built
                    crate::textures::reload();
                    if let Some(g) = self.gfx.as_mut() {
                        // drawn as the world thread meshes it
                        g.begin_world(&p.world);
                    }
                    self.screen = Screen::Playing(Box::new(p));
                    self.audio.play(Sfx::Join, 0.5);
                    self.set_capture(true);
                }
                Ok(Err(e)) => {
                    self.menu.error = Some(e.clone());
                    self.front.error = Some(e);
                    if self.front.page == front::Page::Title {
                        self.front.page = front::Page::Select;
                    }
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
                    NetEvent::Sections(list) => sections_ready(p, list),
                    // new rules from the pack (after /reload)
                    NetEvent::Content(c) => {
                        if let Some(h) = p.hearth {
                            crate::cover::set_areas_from(&c, h);
                        }
                        p.content = *c
                    }
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
                                let mut r = Remote::new(info.name, info.pose);
                                r.look = info.look;
                                r.color = info.color;
                                p.remotes.insert(info.id, r);
                            }
                        }
                        ServerMsg::PlayerLeft { id } => {
                            p.remotes.remove(&id);
                            p.wielding.remove(&id);
                            p.wearing.remove(&id);
                            p.remote_dead.remove(&id);
                            p.remote_hold.remove(&id);
                        }
                        // moved to another copy of the world: same place, new company
                        ServerMsg::Shard {
                            shard,
                            players,
                            blocks,
                        } => {
                            p.shard = shard;
                            p.entities.clear();
                            p.wielding.clear();
                            p.wearing.clear();
                            p.remote_dead.clear();
                            p.remote_hold.clear();
                            p.remotes.clear();
                            p.focus = None;
                            p.hover = None;
                            p.loot = None;
                            if matches!(
                                p.panel,
                                Some(rpg_ui::Panel::Dialog { .. } | rpg_ui::Panel::Vendor { .. })
                            ) {
                                p.panel = None;
                            }
                            for info in players {
                                if info.id != p.my_id {
                                    let mut r = Remote::new(info.name, info.pose);
                                    r.look = info.look;
                                    r.color = info.color;
                                    p.remotes.insert(info.id, r);
                                }
                            }
                            p.ground.clear();
                            for (pos, block) in blocks {
                                if p.world.get(pos[0], pos[1], pos[2]) != block {
                                    p.world.set(pos[0], pos[1], pos[2], block);
                                    p.minimap.block_changed(pos);
                                    p.work.block(pos, block);
                                }
                            }
                        }
                        ServerMsg::Party { members } => p.party = members,
                        ServerMsg::Wielding { player, item } => match item {
                            Some(i) => {
                                p.wielding.insert(player, i);
                            }
                            None => {
                                p.wielding.remove(&player);
                            }
                        },
                        ServerMsg::Wearing { player, items } => {
                            p.wearing.insert(player, items);
                        }
                        // someone (or something) does something: everyone sees it
                        ServerMsg::Act { actor, act } => {
                            let now = p.anims.now();
                            match actor {
                                dyno_protocol::Actor::Player(pid) if pid != p.my_id => {
                                    let one = match act {
                                        dyno_protocol::Act::Swing => Some(Anim::Attack1H),
                                        dyno_protocol::Act::Shoot => Some(Anim::Shoot),
                                        dyno_protocol::Act::Hurt => Some(Anim::HitReact),
                                        dyno_protocol::Act::Died => {
                                            p.remote_hold.remove(&pid);
                                            p.remote_dead.insert(pid);
                                            Some(Anim::Death)
                                        }
                                        dyno_protocol::Act::Revived => {
                                            p.remote_dead.remove(&pid);
                                            None
                                        }
                                        dyno_protocol::Act::Emote(n) => {
                                            match Anim::ALL.get(n as usize).copied() {
                                                Some(e) if !e.one_shot() => {
                                                    p.remote_hold.insert(pid, e);
                                                    None
                                                }
                                                e => e,
                                            }
                                        }
                                        _ => None,
                                    };
                                    if let Some(a) = one {
                                        p.anims.player(pid, now).play_fresh(now, a);
                                    }
                                }
                                dyno_protocol::Actor::Entity(eid) => {
                                    let a = match act {
                                        dyno_protocol::Act::Swing => Some(Anim::AttackUnarmed),
                                        dyno_protocol::Act::Special => Some(Anim::Special),
                                        dyno_protocol::Act::Shoot => Some(Anim::Shoot),
                                        dyno_protocol::Act::Emote(n) => entity_emote(p, eid, n),
                                        _ => None,
                                    };
                                    if let Some(a) = a {
                                        p.anims.play_entity(eid, now, a);
                                    }
                                }
                                _ => {}
                            }
                        }
                        ServerMsg::TradeAsk { from } => {
                            p.trade_ask = Some(from);
                            self.audio.play(Sfx::Click, 0.8);
                            release = true;
                        }
                        ServerMsg::Trade(view) => {
                            if view.is_some() && p.trade.is_none() {
                                p.screen_tab = Some(screens::Tab::Bags);
                                release = true;
                            }
                            p.trade = view;
                        }
                        ServerMsg::Invite { from } => {
                            p.invite = Some(from);
                            self.audio.play(Sfx::Click, 0.8);
                            release = true;
                        }
                        ServerMsg::Block { pos, block } => {
                            p.ground.clear();
                            let old = p.world.get(pos[0], pos[1], pos[2]);
                            if old != block {
                                p.world.set(pos[0], pos[1], pos[2], block);
                                p.minimap.block_changed(pos);
                                // relit and remeshed on the world thread
                                p.work.block(pos, block);
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
                        // the world streams in around you (and is let go behind you)
                        ServerMsg::Sections(list) => sections_in(p, list),
                        ServerMsg::Far(list) => far_in(p, list),
                        ServerMsg::FarGone(list) => far_out(p, self.gfx.as_mut(), list),
                        ServerMsg::SectionsGone(list) => sections_out(p, self.gfx.as_mut(), list),
                        ServerMsg::PlayersFar(ids) => {
                            for id in ids {
                                if let Some(r) = p.remotes.get_mut(&id) {
                                    r.far = true;
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
                            p.minimap.region_changed(a, b);
                            p.work.fill(a, b, block);
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
                            let born = p
                                .entities
                                .get(&id)
                                .map(|e| e.born)
                                .unwrap_or_else(Instant::now);
                            p.entities.insert(
                                id,
                                ClientEntity {
                                    kind,
                                    from: pos,
                                    to: pos,
                                    at: Instant::now(),
                                    yaw,
                                    born,
                                    hurt_at: None,
                                    info: None,
                                    name: None,
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
                            if let Some(e) = p.entities.get(&id)
                                && let EntityKind::Creature(_) = e.kind
                            {
                                let at = p.ground.feet(&p.world, e.pos());
                                let now = p.anims.now();
                                p.anims.creature_gone(at, now);
                            }
                            if let Some(e) = p.entities.remove(&id)
                                && let EntityKind::Creature(cid) = e.kind
                                && let Some(c) = p.content.creature(cid)
                            {
                                let at = e.pos();
                                let gain = (1.0 - at.distance(p.player.pos) / 32.0).clamp(0.0, 1.0);
                                if c.hostile {
                                    spawn_motes(&mut p.particles, at, [0.62, 0.42, 0.95], 26, id);
                                    self.audio.play(Sfx::Dissolve, 0.8 * gain);
                                } else if gain > 0.0
                                    && e.hurt_at
                                        .is_some_and(|t| t.elapsed() < Duration::from_secs(2))
                                {
                                    spawn_motes(&mut p.particles, at, c.color, 10, id);
                                }
                            }
                            if picked_by == Some(p.my_id) {
                                self.audio.play(Sfx::Pop, 0.7);
                            }
                        }
                        ServerMsg::EntityHurt { id } => {
                            if let Some(e) = p.entities.get_mut(&id) {
                                e.hurt_at = Some(Instant::now());
                                let at = e.pos();
                                let gain = (1.0 - at.distance(p.player.pos) / 32.0).clamp(0.0, 1.0);
                                self.audio.play(Sfx::Hit, 0.8 * gain);
                            }
                        }
                        ServerMsg::Power { value, max } => {
                            p.power = value;
                            p.power_max = max;
                        }
                        ServerMsg::Casting {
                            player,
                            ability,
                            secs,
                        } => {
                            if player == p.my_id {
                                p.my_cast = ability.map(|a| (a, Instant::now(), secs));
                            }
                            let now = p.anims.now();
                            match ability {
                                Some(a) => {
                                    p.anims.casts.insert(player, (a, now, secs));
                                }
                                None => {
                                    p.anims.casts.remove(&player);
                                }
                            }
                        }
                        ServerMsg::Cooldowns { list, global } => {
                            let now = Instant::now();
                            p.cooldowns = list
                                .into_iter()
                                .map(|(a, s)| (a, (now + Duration::from_secs_f32(s), s)))
                                .collect();
                            p.gcd = (now + Duration::from_secs_f32(global), global);
                        }
                        ServerMsg::Effect {
                            color,
                            from,
                            to,
                            radius,
                        } => {
                            let (a, b) = (Vec3::from(from), Vec3::from(to));
                            animate_caster(p, a, color);
                            if radius > 0.0 {
                                spawn_burst(&mut p.particles, a, color, radius);
                                self.audio.play(Sfx::Dissolve, 0.5);
                            } else {
                                spawn_bolt(&mut p.particles, a, b, color);
                                self.audio.play(Sfx::Hit, 0.4);
                            }
                        }
                        ServerMsg::Equipment(e) => p.equip = e,
                        ServerMsg::EntityName { id, name, .. } => {
                            if let Some(e) = p.entities.get_mut(&id) {
                                e.name = Some(name);
                            }
                        }
                        ServerMsg::WorldEvent {
                            title,
                            text,
                            progress,
                        } => {
                            if title.is_empty() {
                                p.world_event = None;
                            } else {
                                if p.world_event.is_none() {
                                    self.audio.play(Sfx::Horn, 0.9);
                                }
                                p.world_event = Some((title, text, progress));
                            }
                        }
                        ServerMsg::Weather { kind, strength } => {
                            if kind != p.weather.0 {
                                p.weather_mix = 0.0;
                            }
                            p.weather = (kind, strength);
                        }
                        ServerMsg::Zone { name, levels, safe } => {
                            if !name.is_empty() {
                                p.title = Some((Instant::now(), name.clone()));
                                let sub = if safe {
                                    "A place of safety".to_string()
                                } else if levels[0] == levels[1] {
                                    format!("Level {}", levels[0])
                                } else {
                                    format!("Levels {} to {}", levels[0], levels[1])
                                };
                                p.toast(sub);
                            }
                            p.zone = name;
                        }
                        ServerMsg::Skills(s) => p.skills = s,
                        ServerMsg::Attacking(t) => p.attacking = t,
                        ServerMsg::Combo { points, target } => p.combo = (points, target),
                        ServerMsg::Auras(list) => {
                            let now = Instant::now();
                            p.auras = list
                                .into_iter()
                                .map(|(a, s)| (a, now + Duration::from_secs_f32(s)))
                                .collect();
                        }
                        ServerMsg::Damage {
                            entity,
                            amount,
                            crit,
                        } => {
                            let who = entity.and_then(|e| p.entities.get(&e)).and_then(|e| match e
                                .kind
                            {
                                EntityKind::Creature(c) => e
                                    .name
                                    .clone()
                                    .or_else(|| p.content.creature(c).map(|c| c.name.clone())),
                                _ => None,
                            });
                            let line = match (entity.is_some(), who) {
                                (true, Some(w)) => (
                                    format!(
                                        "You hit {w} for {}{}.",
                                        amount.round(),
                                        if crit { " (critical)" } else { "" }
                                    ),
                                    Color32::from_rgb(235, 225, 200),
                                ),
                                (true, None) => (
                                    format!("You hit for {}.", amount.round()),
                                    Color32::from_rgb(235, 225, 200),
                                ),
                                (false, _) => (
                                    format!("You take {} damage.", amount.round()),
                                    Color32::from_rgb(235, 120, 100),
                                ),
                            };
                            p.combat_log.push((Instant::now(), line.0, line.1));
                            // a blow on your auto-attack target is your swing
                            // (not a damage-over-time tick: those come slower)
                            let now = p.anims.now();
                            if entity.is_some()
                                && entity == p.attacking
                                && now - p.anims.last_swing > 0.75
                            {
                                let swing = anim::attack_for(p.anims.grip);
                                p.anims.me(now).play_fresh(now, swing);
                                p.anims.last_swing = now;
                            }
                            let (text, col) = match entity {
                                Some(_) if crit => {
                                    (format!("{:.0}!", amount), Color32::from_rgb(255, 230, 80))
                                }
                                Some(_) => (format!("{:.0}", amount), Color32::WHITE),
                                None => (format!("-{:.0}", amount), Color32::from_rgb(255, 70, 60)),
                            };
                            p.combat_text.push((Instant::now(), entity, text, col));
                        }
                        ServerMsg::ChooseClass => {
                            p.class_pick = true;
                            release = true;
                        }
                        ServerMsg::Character {
                            look,
                            class,
                            level,
                            xp,
                            xp_next,
                            money,
                            max_health,
                        } => {
                            p.class = class;
                            p.my_look = look;
                            if class.is_some() {
                                p.class_pick = false;
                            }
                            p.level = level;
                            p.xp = xp;
                            p.xp_next = xp_next;
                            p.money = money;
                            p.max_health = max_health;
                        }
                        ServerMsg::Gain { xp, money } => {
                            if xp > 0 {
                                p.combat_log.push((
                                    Instant::now(),
                                    format!("You gain {xp} experience."),
                                    Color32::from_rgb(190, 140, 250),
                                ));
                            }
                            if money > 0 {
                                p.combat_log.push((
                                    Instant::now(),
                                    format!(
                                        "You receive {}.",
                                        dyno_sim::progression::money_text(money)
                                    ),
                                    Color32::from_rgb(240, 200, 90),
                                ));
                            }
                            if xp > 0 {
                                p.floaters.push((
                                    Instant::now(),
                                    format!("+{xp} XP"),
                                    Color32::from_rgb(190, 140, 255),
                                ));
                            }
                            if money > 0 {
                                p.floaters.push((
                                    Instant::now(),
                                    format!("+{}", dyno_sim::progression::money_text(money)),
                                    Color32::from_rgb(255, 210, 80),
                                ));
                            }
                        }
                        ServerMsg::LevelUp { player, level } => {
                            if player == p.my_id {
                                self.audio.play(Sfx::Chime, 1.0);
                                p.title = Some((Instant::now(), format!("Level {level}!")));
                            }
                        }
                        ServerMsg::EntityInfo {
                            id,
                            level,
                            health,
                            max,
                        } => {
                            if let Some(e) = p.entities.get_mut(&id) {
                                // a real blow (EntityHurt also comes on regen)
                                if e.info.is_some_and(|(_, h, _)| health < h - 0.01) && health > 0.0
                                {
                                    let now = p.anims.now();
                                    p.anims.play_entity(id, now, Anim::HitReact);
                                }
                                e.info = Some((level, health, max));
                            }
                        }
                        ServerMsg::Quests { log, done } => {
                            let before: Vec<bool> =
                                log.iter().map(|q| quest_finished(&p.content, q)).collect();
                            let was: HashMap<u16, bool> = p
                                .quest_log
                                .iter()
                                .map(|q| (q.quest, quest_finished(&p.content, q)))
                                .collect();
                            for (q, now) in log.iter().zip(before) {
                                if now && was.get(&q.quest) == Some(&false) {
                                    let t = p
                                        .content
                                        .quest(q.quest)
                                        .map(|q| q.title.clone())
                                        .unwrap_or_default();
                                    p.toast(format!("{t}: complete! Return to hand it in."));
                                    self.audio.play(Sfx::Join, 0.7);
                                }
                            }
                            p.quest_log = log;
                            p.quests_done = done.into_iter().collect();
                        }
                        ServerMsg::Rested { pool } => p.rested = pool,
                        ServerMsg::Loot { id, money, items } => {
                            if money == 0 && items.is_empty() {
                                if p.loot.as_ref().is_some_and(|l| l.0 == id) {
                                    p.loot = None;
                                }
                            } else {
                                if p.loot.is_none() {
                                    self.audio.play(Sfx::Click, 0.6);
                                }
                                p.loot = Some((id, money, items));
                                release = true;
                            }
                        }
                        ServerMsg::Dialog { entity, npc } => {
                            p.panel = Some(rpg_ui::Panel::Dialog {
                                entity,
                                npc,
                                quest: None,
                                choice: 0,
                            });
                            release = true;
                        }
                        ServerMsg::DayEvent { dusk, day } => {
                            if dusk {
                                self.audio.play(Sfx::Horn, 0.9);
                                p.title = Some((Instant::now(), "Dusk is coming".into()));
                            } else {
                                self.audio.play(Sfx::Chime, 0.8);
                                p.title = Some((Instant::now(), format!("Day {day}")));
                            }
                        }
                        ServerMsg::Hurt { amount, push } => {
                            animate_blow(p);
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
                        ServerMsg::Digging { player, pos } => {
                            let now = p.anims.now();
                            if pos.is_some() {
                                p.anims.digging.entry(player).or_insert(now);
                            } else {
                                p.anims.digging.remove(&player);
                            }
                        }
                        ServerMsg::Reject { reason } => closed = Some(reason),
                        _ => {}
                    },
                }
            }
            // the world thread's light and meshes: uploads within ~3 ms a frame;
            // it works nearest the camera and what it faces first
            world_uploads(p, self.gfx.as_mut(), Duration::from_millis(3));
            if let Some(g) = self.gfx.as_mut() {
                far_uploads(p, g, p.player.eye(), Duration::from_millis(2));
            }
            p.work
                .focus(p.player.eye().into(), p.player.look_dir().into());
            if let Some(g) = self.gfx.as_ref() {
                p.land_wait = !land_ready(p, g, p.player.pos, LAND_RADIUS);
            }
            if p.combat_log.len() > 300 {
                p.combat_log.drain(..150);
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
        let fov = match &self.screen {
            Screen::Playing(p) if p.wow() => self.settings.fov_degrees.min(62.0),
            _ => self.settings.fov_degrees,
        };
        let far = match &self.screen {
            // fog pulls the world in
            Screen::Playing(p) if p.weather.0 == 3 => {
                self.settings.view_distance * (1.0 - 0.6 * p.weather_mix)
            }
            Screen::Playing(p) if p.weather.0 == 2 => {
                self.settings.view_distance * (1.0 - 0.3 * p.weather_mix)
            }
            _ => self.settings.view_distance,
        };
        // no further than the world streams (less a section, whose
        // neighbours must arrive before it is drawn)
        let far = match &self.screen {
            // the far view's land and landmarks (its fog thins weather alike)
            Screen::Playing(p) if !p.far.is_empty() => {
                FAR_SEE * (far / self.settings.view_distance.max(1.0)).min(1.0)
            }
            Screen::Playing(p) => far.min((p.view - 16.0).max(48.0)),
            _ => far,
        };

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
            Screen::Playing(p) => weathered(
                crate::sky::sky_at(
                    p.time + p.time_at.elapsed().as_secs_f32() / p.day_seconds,
                    Some(p.player.eye()),
                ),
                p.weather.0,
                p.weather_mix,
            ),
            _ => sky(0.47),
        };

        // simulate
        let mut view = None;
        let mut boxes: Vec<Vertex> = Vec::new();
        let mut lines: Vec<Vertex> = Vec::new();
        // the front end: Hearthstead at dusk, the character on the dais
        if !matches!(self.screen, Screen::Playing(_))
            && let Some(st) = &self.stage
        {
            if !self.stage_loaded {
                gfx.set_focus(Vec3::new(
                    st.hearth[0] as f32,
                    st.hearth[1] as f32,
                    st.hearth[2] as f32,
                ));
                gfx.load_world(&st.world, &st.light);
                self.stage_loaded = true;
            }
            let h = Vec3::new(
                st.hearth[0] as f32 + 0.5,
                st.hearth[1] as f32,
                st.hearth[2] as f32 + 0.5,
            );
            let t = self.front.turn;
            let clock = now.duration_since(self.last_shot).as_secs_f32();
            let preview = if matches!(self.screen, Screen::Menu) {
                self.front.preview()
            } else {
                None
            };
            let (eye, look_at) = match &preview {
                Some(look) => {
                    // the character stands in the plaza; the camera looks at them
                    let feet = h + Vec3::new(0.0, 1.0, 3.0);
                    let feet = Vec3::new(
                        feet.x,
                        st.world.surface(feet.x as i32, feet.z as i32) as f32 + 1.0,
                        feet.z,
                    );
                    models::push_person(
                        &mut boxes,
                        feet,
                        std::f32::consts::PI + t,
                        0.0,
                        look,
                        1.0,
                        0.0,
                        0.0,
                    );
                    (
                        feet + Vec3::new(0.0, 1.5, 5.2),
                        feet + Vec3::new(0.0, 1.05, 0.0),
                    )
                }
                None => {
                    // a slow flight around the town
                    let a = clock * 0.035;
                    (
                        h + Vec3::new(a.cos() * 44.0, 24.0, a.sin() * 44.0),
                        h + Vec3::new(0.0, 6.0, 0.0),
                    )
                }
            };
            let dir = (look_at - eye).normalize_or_zero();
            // the character is drawn off centre, in the room the panels leave
            let shift = if preview.is_some() {
                self.front.preview_offset()
            } else {
                0.0
            };
            let proj = glam::Mat4::from_translation(Vec3::new(shift, 0.0, 0.0))
                * glam::Mat4::perspective_rh(55f32.to_radians(), wdt / hgt.max(1.0), 0.05, far);
            view = Some((proj * glam::Mat4::look_to_rh(eye, dir, Vec3::Y), eye, far));
            gfx.update_lod(&st.world, &st.light, eye);
            gfx.water.look_from(&st.world, eye.into(), st.hearth);
        }
        if let Screen::Playing(p) = &mut self.screen {
            let k = |c: KeyCode| p.keys.contains(&c);
            let wow = p.wow();
            let active = if wow {
                !p.chat_open && !p.menu_open && !p.class_pick && p.dead.is_none()
            } else {
                p.captured
                    && !p.chat_open
                    && !p.menu_open
                    && !p.picker_open
                    && p.panel.is_none()
                    && !p.class_pick
                    && p.screen_tab.is_none()
                    && p.dead.is_none()
            };
            if wow {
                p.player.auto_step = true;
                p.player.run_speed = 6.2;
                // A and D turn unless steering with the right button; Q and E always strafe
                if active && !p.rmb {
                    let turn = (k(KeyCode::KeyA) as i32 - k(KeyCode::KeyD) as i32) as f32;
                    p.player.yaw += turn * 2.6 * dt;
                }
            }
            let input = if active {
                Input {
                    forward: if wow && ((p.lmb && p.rmb) || p.autorun) && !k(KeyCode::KeyS) {
                        1.0
                    } else {
                        (k(KeyCode::KeyW) as i32 - k(KeyCode::KeyS) as i32) as f32
                    },
                    right: if wow && !p.rmb {
                        (k(KeyCode::KeyE) as i32 - k(KeyCode::KeyQ) as i32) as f32
                    } else if wow {
                        ((k(KeyCode::KeyD) || k(KeyCode::KeyE)) as i32
                            - (k(KeyCode::KeyA) || k(KeyCode::KeyQ)) as i32)
                            as f32
                    } else {
                        (k(KeyCode::KeyD) as i32 - k(KeyCode::KeyA) as i32) as f32
                    },
                    jump: k(KeyCode::Space),
                    down: k(KeyCode::ShiftLeft) || k(KeyCode::ShiftRight),
                    sprint: (k(KeyCode::ControlLeft) || k(KeyCode::KeyR))
                        && (p.food > 0.0 || p.creative()),
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
            if let Some(at) = p.sleeping_at {
                let t = (p.time + p.time_at.elapsed().as_secs_f32() / p.day_seconds).fract();
                if at.distance(p.player.pos) > 1.5 || !(0.52..0.98).contains(&t) {
                    p.sleeping_at = None;
                }
            }
            if p.title
                .as_ref()
                .is_some_and(|(t, _)| t.elapsed() > Duration::from_secs(5))
            {
                p.title = None;
            }
            if !p.creative() && p.dead.is_none() && p.health <= 6.0 {
                p.heart_clock -= dt;
                if p.heart_clock <= 0.0 {
                    p.heart_clock = 0.6 + p.health * 0.1;
                    self.audio.play(Sfx::Heartbeat, 0.9);
                }
            }
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
            let can_mine = active && p.mouse_left && !p.creative() && !p.wow();
            let target_block = p
                .target
                .as_ref()
                .map(|t| t.block)
                .filter(|_| p.target_entity.is_none());
            match (can_mine, target_block) {
                (true, Some(pos)) => {
                    let b = p.world.get(pos[0], pos[1], pos[2]);
                    let pack = dyno_sim::inventory::Inventory {
                        slots: p.pack.clone(),
                    };
                    let best = p
                        .content
                        .rule(b)
                        .tool
                        .and_then(|k| dyno_sim::inventory::best_tool(&pack, &p.content, k))
                        .map(|(_, t)| t);
                    let tool = if p.content.world.protected {
                        best
                    } else {
                        p.held_tool()
                    };
                    let need = dyno_sim::mining::break_time(&p.content, b, tool)
                        .filter(|_| !p.content.world.protected || p.content.rule(b).respawn > 0);
                    match (&mut p.dig, need) {
                        (Some((dp, done, total)), Some(_)) if *dp == pos => {
                            *done += dt;
                            p.swing = p.swing.max(0.6);
                            if *done >= *total {
                                let pos = *dp;
                                p.dig = None;
                                let _ = p.conn.tx.send(ClientMsg::Dig { pos });
                                p.world.set(pos[0], pos[1], pos[2], w::AIR);
                                p.minimap.block_changed(pos);
                                p.work.block(pos, w::AIR);
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
                            if p.tier_hint != Some(pos)
                                && let Some(why) =
                                    dyno_sim::mining::missing_tool(&p.content, b, p.held_tool())
                            {
                                p.tier_hint = Some(pos);
                                p.toast(why);
                            }
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
            if p.wow() {
                // what is under the mouse cursor (blocks only within reach of you)
                if let Some((o, d)) = p.cursor_ray(self.cursor) {
                    let reach = p.zoom + 8.0;
                    p.target = game::raycast(&p.world, o, d, reach).filter(|t| {
                        let c = Vec3::new(
                            t.block[0] as f32 + 0.5,
                            t.block[1] as f32 + 0.5,
                            t.block[2] as f32 + 0.5,
                        );
                        c.distance(p.player.eye()) <= 6.5
                    });
                    p.hover = pick_entity_ray(p, o, d, reach + 30.0);
                } else {
                    p.target = None;
                    p.hover = None;
                }
                p.target_entity = p.hover;
                // walking swings the camera back behind you
                let moving = input.forward != 0.0 || input.right != 0.0;
                if moving && !p.lmb {
                    p.cam_orbit *= 1.0 - (dt * 3.0).min(1.0);
                }
            } else {
                p.target =
                    game::raycast(&p.world, p.player.eye(), p.player.look_dir(), game::REACH);
                p.target_entity = pick_entity(p);
            }
            let light_at = |l: &Light, v: Vec3| {
                l.get(v.x.floor() as i32, v.y.floor() as i32, v.z.floor() as i32) as f32 / 15.0
            };
            let mut book = std::mem::take(&mut p.anims);
            let now = book.now();
            book.prune(now);
            for (id, r) in p.remotes.iter().filter(|(_, r)| !r.far) {
                let pose = r.current();
                let base = p.ground.feet(&p.world, Vec3::from(pose.pos));
                let bl = light_at(&p.light, base + Vec3::Y);
                let state = remote_anim(p, &mut book, *id, r, now);
                let step = models::person_step(r.speed());
                let a = book.player(*id, now);
                a.drive_at(now, state, base, step);
                let mut look = models::Look::from_appearance(&r.look, r.color);
                if let Some(items) = p.wearing.get(id) {
                    let keys: Vec<String> = items.iter().map(|i| p.content.item_key(*i)).collect();
                    look.wear(keys.iter().map(String::as_str));
                }
                look.weapon = models::weapon_look(&p.content, p.wielding.get(id).copied());
                models::push_person_anim(&mut boxes, base, pose.yaw, pose.pitch, &look, bl, a, now);
            }
            p.anims = book;
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
            // weather: blend in, rain falls around you, and the land has its sounds
            p.weather_mix = (p.weather_mix + dt / 20.0).min(p.weather.1.max(0.01));
            let raining = p.weather.0 == 2 && p.weather_mix > 0.1;
            let outside = p.light.sky(
                p.player.pos.x.floor() as i32,
                (p.player.pos.y + 1.6).floor() as i32,
                p.player.pos.z.floor() as i32,
            ) > 10;
            if raining {
                let n = (p.weather_mix * 90.0 * dt * 30.0) as usize;
                let seed = (p.fps.0.elapsed().as_nanos() as u32) | 1;
                let mut r = seed;
                let mut rnd = || {
                    r ^= r << 13;
                    r ^= r >> 17;
                    r ^= r << 5;
                    r as f32 / u32::MAX as f32
                };
                for _ in 0..n.min(60) {
                    let at = p.player.pos
                        + Vec3::new(
                            (rnd() - 0.5) * 30.0,
                            8.0 + rnd() * 6.0,
                            (rnd() - 0.5) * 30.0,
                        );
                    let col = p.world.surface(at.x.floor() as i32, at.z.floor() as i32);
                    if (col as f32) < at.y {
                        p.particles.push(Particle {
                            pos: at,
                            vel: Vec3::new(0.6, -16.0, 0.3),
                            color: [0.65, 0.72, 0.85],
                            life: 1.0,
                            size: 0.025,
                            gravity: 2.0,
                        });
                    }
                }
            }
            p.ambience_clock -= dt;
            if p.ambience_clock <= 0.0 {
                p.ambience_clock = 0.9;
                let near_town = p.hearth.is_some_and(|h| {
                    Vec3::new(h[0] as f32, h[1] as f32, h[2] as f32).distance(p.player.pos) < 30.0
                });
                let tod = (p.time + p.time_at.elapsed().as_secs_f32() / p.day_seconds).fract();
                let day = tod < 0.5;
                let roll = (p.fps.0.elapsed().as_millis() % 1000) as f32 / 1000.0;
                if raining && outside {
                    self.audio.play(Sfx::Rain, 0.5 * p.weather_mix);
                } else if outside && day && p.weather.0 != 2 && roll < 0.35 {
                    self.audio.play(Sfx::Bird, 0.35);
                }
                if outside && roll > 0.8 {
                    self.audio.play(
                        Sfx::Wind,
                        0.25 + 0.3 * if p.weather.0 == 2 { p.weather_mix } else { 0.0 },
                    );
                }
                if near_town && day && roll > 0.55 && roll < 0.7 {
                    self.audio.play(Sfx::Hammer, 0.18);
                }
            }
            // particles
            for pt in &mut p.particles {
                pt.vel.y -= pt.gravity * dt;
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
            let outline = p.target.as_ref().filter(|t| {
                !p.wow()
                    || p.content
                        .rule(p.world.get(t.block[0], t.block[1], t.block[2]))
                        .respawn
                        > 0
            });
            if let Some(t) = outline {
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
                            tex: render::NO_TEX,
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
                            tex: render::NO_TEX,
                        });
                    }
                }
            }
            // camera with view bob
            let dir = p.cam_dir();
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
            // third person: the camera pulls back behind you, short of walls
            let first_person = p.zoom < 0.5;
            let eye = if first_person {
                eye
            } else {
                pull_back(&p.world, p.player.eye(), dir, p.zoom)
            };
            let proj = glam::Mat4::perspective_rh(
                (fov + p.fov_kick).to_radians(),
                wdt / hgt.max(1.0),
                0.05,
                far,
            );
            let vp = proj * glam::Mat4::look_to_rh(eye, dir, Vec3::Y);
            // models skip faces the camera and sun cannot see, and coarsen far away
            models::set_view(Some((eye, Vec3::from(sky_now.sun_dir))));
            let held_item = if p.content.world.protected && !p.creative() {
                // RPG: your best weapon is always in hand
                let pack = dyno_sim::inventory::Inventory {
                    slots: p.pack.clone(),
                };
                dyno_sim::inventory::best_weapon(&pack, &p.content).and_then(|(i, _)| p.pack[i])
            } else {
                p.held()
            };
            if !first_person {
                let mut look = models::Look::from_appearance(
                    &p.my_look,
                    p.class
                        .and_then(|c| p.content.class(c))
                        .map(|c| c.color)
                        .unwrap_or([0.5, 0.5, 0.6]),
                );
                let keys: Vec<String> = p
                    .equip
                    .iter()
                    .flatten()
                    .map(|s| p.content.item_key(s.item))
                    .collect();
                look.wear(keys.iter().map(String::as_str));
                look.weapon = models::weapon_look(&p.content, held_item.map(|s| s.item));
                let mut book = std::mem::take(&mut p.anims);
                let now = book.now();
                let feet = p.ground.feet(&p.world, p.player.pos);
                let state = my_anim(p, &mut book, &look, now);
                let step = models::person_step(speed);
                let me = book.me(now);
                if p.dig.is_some() && me.action(now).is_none() {
                    me.play(now, Anim::Attack1H);
                }
                me.drive_at(now, state, feet, step);
                models::push_person_anim(
                    &mut boxes,
                    feet,
                    p.player.yaw,
                    p.player.pitch,
                    &look,
                    light_at(&p.light, p.player.pos + Vec3::Y),
                    me,
                    now,
                );
                p.anims = book;
            }
            // block in hand, with a swing when breaking or placing
            let sw = (p.swing * std::f32::consts::PI).sin();
            let held_item = if first_person { held_item } else { None };
            let hand = eye + dir * (0.62 - sw * 0.12) + right * (0.36 - sw * 0.08)
                - up * (0.34 + sw * 0.1 - (p.bob * 0.5).cos().abs() * 0.015 * bob_amt);
            let (s1, c1) = (0.6f32 - sw * 0.8).sin_cos();
            let ax = [right * c1 + dir * s1, up, dir * c1 - right * s1];
            if first_person {
                models::push_held(
                    &mut boxes,
                    &p.content,
                    held_item.map(|s| s.item),
                    hand,
                    ax,
                    light_at(&p.light, eye),
                );
            }
            // a ring on the ground under your target: red hostile, yellow neutral, green friendly
            if let Some(e) = p.focus.and_then(|id| p.entities.get(&id)) {
                let (col, r) = match e.kind {
                    EntityKind::Creature(c) => match p.content.creature(c) {
                        Some(cr) if cr.hostile => ([0.95, 0.2, 0.15], 0.75 * cr.scale),
                        Some(cr) => ([0.95, 0.85, 0.2], 0.75 * cr.scale),
                        None => ([0.95, 0.85, 0.2], 0.75),
                    },
                    _ => ([0.3, 0.9, 0.35], 0.7),
                };
                render::push_ground_ring_on(&mut boxes, &p.world, &p.ground, e.pos(), r, col);
            }
            // creatures
            let clock = p.fps.0.elapsed().as_secs_f32() + p.time_at.elapsed().as_secs_f32();
            let mut book = std::mem::take(&mut p.anims);
            push_creatures(&mut boxes, p, &mut book, clock);
            p.anims = book;
            // smoke, fireflies, birds, deer
            let tod = (p.time + p.time_at.elapsed().as_secs_f32() / p.day_seconds).fract();
            let quality = crate::life::quality(self.settings.life);
            push_life(p, gfx, &mut boxes, eye, dir, tod, dt, quality, false);
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
                        if let Some(b) = it.block
                            && w::shape(b) == w::Shape::Post
                        {
                            render::push_post_oriented(
                                &mut boxes,
                                pos + Vec3::Y * (bob + 0.05) + off,
                                ax,
                                0.5,
                                b,
                                light_at(&p.light, pos),
                            );
                            continue;
                        }
                        if let Some(b) = it.block {
                            render::push_block_cube(
                                &mut boxes,
                                pos + Vec3::Y * bob + off,
                                ax,
                                size,
                                b,
                                light_at(&p.light, pos),
                            );
                            continue;
                        }
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
            gfx.water
                .look_from(&p.world, eye.into(), p.hearth.unwrap_or_default());
            p.inv_vp = Some(vp.inverse());
            p.screen_px = (wdt, hgt);
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
        // the pack's fonts, once they arrive
        crate::theme::refresh_fonts(&ctx);
        let mut actions = Vec::new();
        let out = ctx.run(raw, |ctx| {
            ui(
                ctx,
                &mut self.screen,
                &mut self.front,
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
            Action::Screens(cmds) => {
                let mut close = false;
                if let Screen::Playing(p) = &mut self.screen {
                    for c in cmds {
                        match c {
                            screens::ScreenCmd::Equip(i) => {
                                let _ = p.conn.tx.send(ClientMsg::Equip { from: i });
                                self.audio.play(Sfx::Place(w::Material::Cloth), 0.6);
                            }
                            screens::ScreenCmd::Unequip(i) => {
                                let _ = p.conn.tx.send(ClientMsg::Unequip { place: i });
                            }
                            screens::ScreenCmd::Eat(i) => {
                                let _ = p.conn.tx.send(ClientMsg::Held { slot: i });
                                let _ = p.conn.tx.send(ClientMsg::UseHeld);
                                p.sent_held = i as usize;
                            }
                            screens::ScreenCmd::Offer(i) => {
                                let _ = p.conn.tx.send(ClientMsg::Trade(
                                    dyno_protocol::TradeOp::Offer { slot: i, count: 0 },
                                ));
                            }
                            screens::ScreenCmd::Sell(i) => {
                                let _ = p.conn.tx.send(ClientMsg::Sell { from: i });
                            }
                            screens::ScreenCmd::Bind(a, k) => {
                                if p.bar.is_empty() {
                                    p.bar = default_bar(p);
                                }
                                for b in p.bar.iter_mut() {
                                    if *b == Some(a) {
                                        *b = None;
                                    }
                                }
                                p.bar[k as usize] = Some(a);
                            }
                            screens::ScreenCmd::Switch(t) => p.screen_tab = Some(t),
                            screens::ScreenCmd::Close => {
                                p.screen_tab = None;
                                close = true;
                            }
                        }
                    }
                }
                if close {
                    self.set_capture(true);
                }
            }
            Action::ChooseClass(c) => {
                if let Screen::Playing(p) = &mut self.screen {
                    let _ = p.conn.tx.send(ClientMsg::ChooseClass { class: c });
                    p.class_pick = false;
                }
                self.set_capture(true);
            }
            Action::Rpg(cmds) => {
                let mut close = false;
                if let Screen::Playing(p) = &mut self.screen {
                    for c in cmds {
                        match c {
                            rpg_ui::RpgCmd::Accept(q) => {
                                let _ = p.conn.tx.send(ClientMsg::AcceptQuest { quest: q });
                                self.audio.play(Sfx::Click, 0.7);
                            }
                            rpg_ui::RpgCmd::Complete(q, ch) => {
                                let _ = p.conn.tx.send(ClientMsg::CompleteQuest {
                                    quest: q,
                                    choice: ch,
                                });
                            }
                            rpg_ui::RpgCmd::Abandon(q) => {
                                let _ = p.conn.tx.send(ClientMsg::AbandonQuest { quest: q });
                            }
                            rpg_ui::RpgCmd::Show(panel) => {
                                // a merchant opens your bags beside their goods
                                if matches!(panel, rpg_ui::Panel::Vendor { .. }) {
                                    p.screen_tab = Some(screens::Tab::Bags);
                                }
                                p.panel = Some(panel);
                            }
                            rpg_ui::RpgCmd::Buy(npc, item, count) => {
                                let _ = p.conn.tx.send(ClientMsg::Buy { npc, item, count });
                                self.audio.play(Sfx::Click, 0.6);
                            }
                            rpg_ui::RpgCmd::Loot(id, slot) => {
                                let _ = p.conn.tx.send(ClientMsg::LootTake { id, slot });
                            }
                            rpg_ui::RpgCmd::CloseLoot => p.loot = None,
                            rpg_ui::RpgCmd::Close => {
                                p.panel = None;
                                close = true;
                            }
                        }
                    }
                }
                if close {
                    self.set_capture(true);
                }
            }
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
            Action::Graphics => {
                // the options panel changed something: apply and save
                if let Some(o) = crate::options::get() {
                    o.into(&mut self.settings);
                    let _ = self.settings.save();
                    if let Some(g) = &mut self.gfx {
                        g.set_graphics(o.tier);
                        g.water.quality = o.water;
                        g.cover.set_quality(o.cover);
                    }
                    crate::foliage::set_detail(o.leaves);
                }
            }
            Action::Resume => {
                if let Screen::Playing(p) = &mut self.screen {
                    p.menu_open = false;
                }
                self.set_capture(true);
            }
            Action::Disconnect => {
                self.set_capture(false);
                self.screen = Screen::Menu;
                // back to character select, with a fresh list
                self.front.page = front::Page::Select;
                let token = self.settings.token;
                let i = self.front.realm;
                self.front.query(i, token, None);
            }
            Action::Capture => self.set_capture(true),
            Action::Quit => std::process::exit(0),
            Action::Front(cmd) => {
                let token = self.settings.token;
                match cmd {
                    FrontCmd::Query(i) => self.front.query(i, token, None),
                    FrontCmd::Delete(i, name) => self.front.query(i, token, Some(name)),
                    FrontCmd::Enter { addr, name, create } => {
                        self.start_join_as(addr, name, create)
                    }
                    FrontCmd::Host => self.act(Action::Host),
                    FrontCmd::StopHost => self.act(Action::StopHost),
                    FrontCmd::SetWorld(addr) => {
                        self.settings.remember_server(&addr);
                        let _ = self.settings.save();
                    }
                    FrontCmd::Graphics => self.act(Action::Graphics),
                    FrontCmd::Quit => std::process::exit(0),
                }
            }
        }
    }

    fn key(&mut self, code: KeyCode, pressed: bool, repeat: bool) {
        let Screen::Playing(p) = &mut self.screen else {
            return;
        };
        let blocked = if p.wow() {
            p.chat_open || p.menu_open || p.class_pick || p.dead.is_some()
        } else {
            p.chat_open
                || p.menu_open
                || p.picker_open
                || p.panel.is_some()
                || p.class_pick
                || p.screen_tab.is_some()
                || p.dead.is_some()
        };
        if blocked {
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
        if p.wow() && matches!(code, KeyCode::KeyW | KeyCode::KeyS) {
            p.autorun = false;
        }
        if code == KeyCode::F3 {
            p.show_info = !p.show_info;
            return;
        }
        if code == KeyCode::Escape && p.intro.is_some() {
            p.intro = Some(Instant::now() - Duration::from_secs(600));
            return;
        }
        match code {
            KeyCode::Escape
                if p.wow() && (p.panel.is_some() || p.screen_tab.is_some() || p.picker_open) =>
            {
                p.panel = None;
                p.screen_tab = None;
                p.picker_open = false;
            }
            KeyCode::Escape if p.wow() && p.focus.is_some() => {
                p.focus = None;
                let _ = p.conn.tx.send(ClientMsg::AutoAttack { target: None });
            }
            KeyCode::Escape => {
                p.menu_open = true;
                self.set_capture(false);
            }
            KeyCode::KeyR if p.wow() => p.autorun = !p.autorun,
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
            KeyCode::KeyC | KeyCode::KeyB | KeyCode::KeyP | KeyCode::KeyK
                if p.content.world.protected =>
            {
                let tab = match code {
                    KeyCode::KeyC => screens::Tab::Character,
                    KeyCode::KeyB => screens::Tab::Bags,
                    KeyCode::KeyP => screens::Tab::Spellbook,
                    _ => screens::Tab::Skills,
                };
                // the same key closes it again
                p.screen_tab = if p.screen_tab == Some(tab) {
                    None
                } else {
                    Some(tab)
                };
                self.set_capture(false);
            }
            KeyCode::KeyE if p.content.world.protected && !p.creative() => {
                p.screen_tab = Some(screens::Tab::Bags);
                self.set_capture(false);
            }
            KeyCode::KeyL => {
                p.panel = if matches!(p.panel, Some(rpg_ui::Panel::Log { .. })) {
                    None
                } else {
                    Some(rpg_ui::Panel::Log { quest: None })
                };
                self.set_capture(false);
            }
            KeyCode::Tab => {
                // the nearest creature in front of us
                let eye = p.player.eye();
                let dir = p.cam_dir();
                p.focus = p
                    .entities
                    .iter()
                    .filter(|(id, e)| {
                        matches!(e.kind, EntityKind::Creature(_)) && Some(**id) != p.focus
                    })
                    .filter_map(|(id, e)| {
                        let v = e.pos() + Vec3::Y - eye;
                        let d = v.length();
                        (d < 35.0 && v.normalize().dot(dir) > 0.6).then_some((*id, d))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(id, _)| id)
                    .or(p.focus);
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
                    KeyCode::Digit0,
                    KeyCode::Minus,
                    KeyCode::Equal,
                ];
                if let Some(i) = digits.iter().position(|d| *d == code) {
                    if p.content.world.protected && !p.creative() {
                        if p.bar.is_empty() {
                            p.bar = default_bar(p);
                        }
                        use_action(p, i);
                        p.swing = 1.0;
                    } else if i < 9 {
                        p.slot = i;
                    }
                }
            }
        }
    }

    fn click(&mut self, button: MouseButton) {
        if self.gfx.is_none() {
            return;
        }
        let Screen::Playing(p) = &mut self.screen else {
            return;
        };
        if !p.captured
            || p.menu_open
            || p.chat_open
            || p.picker_open
            || p.panel.is_some()
            || p.dead.is_some()
        {
            return;
        }
        p.swing = 1.0;
        if let Some(id) = p.target_entity
            && let Some(EntityKind::Npc(_)) = p.entities.get(&id).map(|e| e.kind)
        {
            if button == MouseButton::Right {
                let _ = p.conn.tx.send(ClientMsg::Talk { entity: id });
            }
            p.focus = Some(id);
            return;
        }
        if let Some(id) = p.target_entity {
            if p.content.world.protected {
                // classic: click to target, and start attacking it
                let _ = p.conn.tx.send(ClientMsg::AutoAttack { target: Some(id) });
            } else if button == MouseButton::Left {
                let _ = p.conn.tx.send(ClientMsg::Attack { entity: id });
            }
            p.focus = Some(id);
            return;
        }
        let Some(t) = &p.target else { return };
        let (tb, tp) = (t.block, t.place);
        match button {
            MouseButton::Left if p.creative() => {
                let pos = tb;
                let old = p.world.get(pos[0], pos[1], pos[2]);
                p.world.set(pos[0], pos[1], pos[2], w::AIR);
                p.minimap.block_changed(pos);
                p.work.block(pos, w::AIR);
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
                    let t = (p.time + p.time_at.elapsed().as_secs_f32() / p.day_seconds).fract();
                    if looked == w::BED && t > 0.52 && t < 0.98 {
                        p.sleeping_at = Some(p.player.pos);
                    }
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
                if p.content.world.protected && !p.creative() {
                    return; // the land is protected; building needs a plot (later)
                }
                let pos = tp;
                let here = p.world.get(pos[0], pos[1], pos[2]);
                if (here == w::AIR || here == w::WATER)
                    && p.world.in_bounds(pos)
                    && (!w::is_solid(block) || !p.player.overlaps_block(pos))
                {
                    p.world.set(pos[0], pos[1], pos[2], block);
                    p.minimap.block_changed(pos);
                    p.work.block(pos, block);
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
    Rpg(Vec<rpg_ui::RpgCmd>),
    ChooseClass(u16),
    Screens(Vec<screens::ScreenCmd>),
    Respawn,
    Join(String),
    Host,
    StopHost,
    Tunnel,
    CancelJoin,
    Resume,
    /// the graphics options changed (options.rs)
    Graphics,
    Disconnect,
    Capture,
    Front(FrontCmd),
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
        h.players()
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
    front_state: &mut Front,
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
            let _ = (menu, settings);
            let mut cmds = Vec::new();
            front::ui(
                ctx,
                front_state,
                version,
                update_note,
                host.is_some(),
                &mut cmds,
            );
            actions.extend(cmds.into_iter().map(Action::Front));
        }
        Screen::Joining {
            since, progress, ..
        } => {
            // the loading screen: the zone's name over the town, a tip, a burning fuse
            // (words from the pack: none on the very first join, before it arrives)
            let screen = ctx.content_rect();
            let p = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Middle,
                egui::Id::new("loading"),
            ));
            p.rect_filled(screen, 0.0, Color32::from_black_alpha(90));
            let title = pack_text("loading", "title");
            crate::theme::title_text(
                &p,
                Pos2::new(screen.center().x, screen.top() + screen.height() * 0.3),
                &title,
                54.0,
                crate::theme::GOLD_BRIGHT,
            );
            crate::theme::subtitle_text(
                &p,
                Pos2::new(
                    screen.center().x,
                    screen.top() + screen.height() * 0.3 + 46.0,
                ),
                &pack_text("loading", "subtitle"),
                18.0,
                crate::theme::TEXT,
            );
            let tips = pack_lines("loading", "tips");
            let tip = tips
                .get((since.elapsed().as_secs() / 5) as usize % tips.len().max(1))
                .cloned()
                .unwrap_or_default();
            let bar = Rect::from_center_size(
                Pos2::new(screen.center().x, screen.bottom() - 70.0),
                Vec2::new(520.0, 10.0),
            );
            p.rect_filled(bar, 4.0, Color32::from_black_alpha(200));
            // the pack download when there is one, then the land around
            // you, else a fuse that slows as it nears the end
            let downloading = progress.fraction().filter(|f| *f < 1.0);
            let (have, need) = (
                progress.sections.load(std::sync::atomic::Ordering::Relaxed),
                progress
                    .sections_needed
                    .load(std::sync::atomic::Ordering::Relaxed),
            );
            let land = (downloading.is_none() && need > 0 && have < need)
                .then(|| have as f32 / need as f32);
            let f = downloading
                .or(land)
                .unwrap_or_else(|| 1.0 - (-since.elapsed().as_secs_f32() / 3.0).exp());
            p.rect_filled(
                Rect::from_min_size(bar.min, Vec2::new(bar.width() * f, bar.height())),
                4.0,
                Color32::from_rgb(240, 150, 50),
            );
            p.circle_filled(
                Pos2::new(bar.left() + bar.width() * f, bar.center().y),
                6.0,
                Color32::from_rgb(255, 230, 150),
            );
            crate::theme::subtitle_text(
                &p,
                Pos2::new(screen.center().x, screen.bottom() - 100.0),
                &tip,
                17.0,
                crate::theme::TEXT,
            );
            let status = match downloading {
                Some(_) => {
                    let done = progress.done.load(std::sync::atomic::Ordering::Relaxed);
                    let total = progress.total.load(std::sync::atomic::Ordering::Relaxed);
                    if total >= 1024 * 1024 {
                        let mb = |b: u64| b as f32 / (1024.0 * 1024.0);
                        format!(
                            "Downloading the world's files: {:.1} of {:.1} MB",
                            mb(done),
                            mb(total)
                        )
                    } else {
                        format!(
                            "Downloading the world's files: {} of {} KB",
                            done / 1024,
                            total / 1024
                        )
                    }
                }
                None if land.is_some() => format!("Loading the land around you: {have} of {need}"),
                None if title.is_empty() => "Entering the world".to_string(),
                None => format!("Entering {title}"),
            };
            crate::theme::shadow_text(
                &p,
                Pos2::new(screen.center().x, screen.bottom() - 40.0),
                Align2::CENTER_CENTER,
                &status,
                crate::theme::body_font(13.0),
                crate::theme::TEXT_DIM,
            );
            egui::Area::new(egui::Id::new("cancel-join"))
                .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-20.0, -20.0))
                .show(ctx, |ui| {
                    if ui.button("Cancel").clicked() {
                        actions.push(Action::CancelJoin);
                    }
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
    // after a join or a long teleport, until the land round you is here
    if p.land_wait {
        let veil = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("land-veil"),
        ));
        veil.rect_filled(screen, 0.0, Color32::from_black_alpha(215));
        crate::theme::shadow_text(
            &veil,
            Pos2::new(c.x, screen.bottom() - 60.0),
            Align2::CENTER_CENTER,
            "Loading the land around you",
            crate::theme::body_font(15.0),
            crate::theme::TEXT,
        );
    }
    // the cursor shows what a click will do
    if p.wow() && !p.captured {
        let icon = match p.hover.and_then(|id| p.entities.get(&id)).map(|e| e.kind) {
            Some(EntityKind::Creature(_)) => egui::CursorIcon::Crosshair,
            Some(EntityKind::Npc(_)) => egui::CursorIcon::Help,
            _ if p.target.is_some() => egui::CursorIcon::Grab,
            _ => egui::CursorIcon::Default,
        };
        if !ctx.is_pointer_over_area() {
            ctx.set_cursor_icon(icon);
        }
    }
    // crosshair
    if p.captured && !p.wow() {
        let s = Stroke::new(2.0_f32, Color32::from_white_alpha(220));
        painter.line_segment([c - Vec2::new(9.0, 0.0), c + Vec2::new(9.0, 0.0)], s);
        painter.line_segment([c - Vec2::new(0.0, 9.0), c + Vec2::new(0.0, 9.0)], s);
    }
    // name tags
    if let Some(vp) = vp {
        for r in p.remotes.values().filter(|r| !r.far) {
            let pose = r.current();
            let head = glam::Vec3::from(pose.pos) + glam::Vec3::Y * 2.0;
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
    // nameplates for creatures and NPCs, with quest markers
    let me = rpg_ui::Me {
        content: &p.content,
        money: p.money,
        level: p.level,
        log: &p.quest_log,
        done: &p.quests_done,
        pos: p.player.pos.into(),
        hearth: p.hearth,
    };
    if let Some(vp) = vp {
        let eye = p.player.eye();
        for (id, e) in &p.entities {
            let (name, sub, height, colour, marker) = match e.kind {
                EntityKind::Npc(n) => {
                    let Some(npc) = p.content.npc(n) else {
                        continue;
                    };
                    let title = (!npc.title.is_empty()).then(|| format!("<{}>", npc.title));
                    (
                        npc.name.clone(),
                        title,
                        2.05,
                        Color32::from_rgb(120, 230, 120),
                        me.marker(n),
                    )
                }
                EntityKind::Creature(c) => {
                    let Some(cr) = p.content.creature(c) else {
                        continue;
                    };
                    let lvl = e.info.map(|i| i.0).unwrap_or(1);
                    let col = if cr.hostile {
                        rpg_ui::level_color(p.level, lvl)
                    } else {
                        Color32::from_rgb(230, 230, 230)
                    };
                    match &e.name {
                        Some(n) => (
                            format!("{n}  {lvl}  Elite"),
                            Some(format!("<{}>", cr.name)),
                            p.tops.borrow().get(id).copied().unwrap_or(2.4) + 0.4,
                            Color32::from_rgb(255, 200, 60),
                            None,
                        ),
                        None => (
                            format!("{}  {}", cr.name, lvl),
                            None,
                            p.tops.borrow().get(id).copied().unwrap_or(1.6) + 0.35,
                            col,
                            None,
                        ),
                    }
                }
                _ => continue,
            };
            let head = e.pos() + glam::Vec3::Y * height;
            let dist = head.distance(eye);
            let picked = p.focus == Some(*id) || p.target_entity == Some(*id);
            if dist > 28.0 && !picked {
                continue;
            }
            let clip = vp * head.extend(1.0);
            if clip.w <= 0.1 {
                continue;
            }
            let ndc = clip.truncate() / clip.w;
            let pos = Pos2::new(
                screen.left() + (ndc.x * 0.5 + 0.5) * screen.width(),
                screen.top() + (0.5 - ndc.y * 0.5) * screen.height(),
            );
            let fade = (1.0 - (dist - 20.0).max(0.0) / 8.0).clamp(0.3, 1.0);
            painter.text(
                pos + Vec2::new(1.0, 1.0),
                Align2::CENTER_BOTTOM,
                &name,
                FontId::proportional(14.0),
                Color32::from_black_alpha((200.0 * fade) as u8),
            );
            painter.text(
                pos,
                Align2::CENTER_BOTTOM,
                &name,
                FontId::proportional(14.0),
                colour.gamma_multiply(fade),
            );
            if let Some(s) = sub {
                painter.text(
                    pos + Vec2::new(0.0, 2.0),
                    Align2::CENTER_TOP,
                    s,
                    FontId::proportional(11.0),
                    Color32::from_gray(200).gamma_multiply(fade),
                );
            }
            if let Some((_, h, m)) = e.info
                && (h < m || picked)
            {
                let r = Rect::from_center_size(pos + Vec2::new(0.0, 6.0), Vec2::new(60.0, 5.0));
                painter.rect_filled(r, 1.0, Color32::from_black_alpha(180));
                painter.rect_filled(
                    Rect::from_min_size(
                        r.min,
                        Vec2::new(r.width() * (h / m).clamp(0.0, 1.0), r.height()),
                    ),
                    1.0,
                    Color32::from_rgb(200, 50, 40),
                );
            }
            // the quest marker itself floats in the world (push_marker)
            let _ = marker;
        }
    }
    let map = p.minimap.show(
        ctx,
        &painter,
        &crate::minimap::View {
            world: &p.world,
            content: &p.content,
            me: &me,
            pos: p.player.pos,
            yaw: p.player.yaw,
            zone: &p.zone,
            time: (p.time + p.time_at.elapsed().as_secs_f32() / p.day_seconds).fract(),
            remotes: &p.remotes,
            entities: p
                .entities
                .values()
                .map(|e| (e.kind, e.pos(), e.name.as_deref()))
                .collect(),
            captured: p.captured,
        },
    );
    // the world event box and the quest tracker stack under the minimap
    let event_top = map.bottom() + 10.0;
    let tracker_top = event_top + if p.world_event.is_some() { 68.0 } else { 0.0 };
    if p.content.world.protected {
        // player frame, target frame, quest tracker, XP bar
        rpg_ui::unit_frame(
            &painter,
            screen.left_top() + Vec2::new(12.0, 52.0),
            &p.my_name,
            p.level,
            Color32::from_rgb(255, 210, 80),
            p.health,
            p.max_health,
            Some(&dyno_sim::progression::money_text(p.money)),
            None,
        );
        if let Some(t) = p
            .focus
            .or(p.target_entity)
            .and_then(|id| p.entities.get(&id))
        {
            let (name, lvl, h, m) = match t.kind {
                EntityKind::Creature(c) => {
                    let (l, h, m) = t.info.unwrap_or((1, 1.0, 1.0));
                    (
                        p.content
                            .creature(c)
                            .map(|c| c.name.clone())
                            .unwrap_or_default(),
                        l,
                        h,
                        m,
                    )
                }
                EntityKind::Npc(n) => (
                    p.content.npc(n).map(|n| n.name.clone()).unwrap_or_default(),
                    p.level,
                    1.0,
                    1.0,
                ),
                _ => (String::new(), 0, 0.0, 0.0),
            };
            if !name.is_empty() {
                rpg_ui::unit_frame(
                    &painter,
                    Pos2::new(c.x - 115.0, screen.top() + 12.0),
                    &name,
                    lvl,
                    rpg_ui::level_color(p.level, lvl),
                    h,
                    m,
                    None,
                    None,
                );
            }
        }
        rpg_ui::tracker(&painter, screen, &me, tracker_top);
    }
    // a world event in progress: an orange tracker box at the right
    if let Some((title, text, prog)) = &p.world_event {
        let r = Rect::from_min_size(
            Pos2::new(screen.right() - 300.0, event_top),
            Vec2::new(284.0, 58.0),
        );
        painter.rect_filled(r, 6.0, Color32::from_black_alpha(170));
        painter.rect_stroke(
            r,
            6.0,
            Stroke::new(1.5_f32, Color32::from_rgb(230, 140, 40)),
            egui::StrokeKind::Inside,
        );
        painter.text(
            r.min + Vec2::new(10.0, 6.0),
            Align2::LEFT_TOP,
            title,
            FontId::proportional(15.0),
            Color32::from_rgb(255, 180, 70),
        );
        painter.text(
            r.min + Vec2::new(10.0, 25.0),
            Align2::LEFT_TOP,
            text,
            FontId::proportional(12.0),
            Color32::from_gray(225),
        );
        let bar = Rect::from_min_size(r.min + Vec2::new(10.0, 44.0), Vec2::new(264.0, 6.0));
        painter.rect_filled(bar, 2.0, Color32::from_black_alpha(200));
        painter.rect_filled(
            Rect::from_min_size(
                bar.min,
                Vec2::new(bar.width() * prog.clamp(0.0, 1.0), bar.height()),
            ),
            2.0,
            Color32::from_rgb(230, 140, 40),
        );
    }
    // floating XP and money
    p.floaters
        .retain(|(t, _, _)| t.elapsed() < Duration::from_millis(1800));
    for (k, (t, text, colour)) in p.floaters.iter().enumerate() {
        let a = t.elapsed().as_secs_f32();
        let at = Pos2::new(c.x + 40.0, c.y - 30.0 - a * 40.0 - k as f32 * 18.0);
        let fade = (1.0 - a / 1.8).clamp(0.0, 1.0);
        painter.text(
            at,
            Align2::LEFT_CENTER,
            text,
            FontId::proportional(18.0),
            colour.gamma_multiply(fade),
        );
    }
    // top-left info: always in the builder modes, on F3 in the RPG
    let pos = p.player.pos;
    let show_info = !p.content.world.protected || p.show_info;
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
    let info = if show_info { info } else { String::new() };
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
    // hotbar from the pack, or the 12-slot action bar
    let rpg_bar = p.content.world.protected && !p.creative();
    if rpg_bar && p.bar.is_empty() && p.class.is_some() {
        p.bar = default_bar(p);
    }
    let slots = if rpg_bar { 12 } else { 9 };
    let slot = if rpg_bar { 44.0 } else { 46.0 };
    let total = slots as f32 * (slot + 4.0);
    let start = Pos2::new(c.x - total / 2.0, screen.bottom() - slot - 14.0);
    painter.rect_filled(
        Rect::from_min_size(
            start - Vec2::splat(6.0),
            Vec2::new(total + 8.0, slot + 12.0),
        ),
        8.0,
        Color32::from_black_alpha(130),
    );
    for i in 0..slots {
        let r = Rect::from_min_size(
            start + Vec2::new(i as f32 * (slot + 4.0), 0.0),
            Vec2::splat(slot),
        );
        if rpg_bar {
            draw_action(&painter, r, p, i);
            continue;
        }
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
    let held_name = if rpg_bar {
        String::new()
    } else {
        p.held()
            .map(|s| p.content.item(s.item).name.to_string())
            .unwrap_or_default()
    };
    let mut label_y = start.y - 12.0;
    if p.content.world.protected {
        rpg_ui::xp_bar(
            &painter,
            Pos2::new(c.x, start.y - 10.0),
            total,
            p.xp,
            p.xp_next,
            p.level,
            p.rested,
        );
        label_y -= 14.0;
    }
    if !p.creative() && !p.content.world.protected {
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
    // asleep in bed: the world dims
    if p.sleeping_at.is_some() {
        painter.rect_filled(screen, 0.0, Color32::from_black_alpha(150));
        painter.text(
            Pos2::new(c.x, c.y + 40.0),
            egui::Align2::CENTER_CENTER,
            "Sleeping… the night passes quickly when half of the players sleep. Move to get up.",
            FontId::proportional(18.0),
            Color32::from_gray(220),
        );
    }
    // the first-login intro: letterbox and a few lines of story
    if let Some(t0) = p.intro {
        let t = t0.elapsed().as_secs_f32();
        // the story is the pack's (ui/strings.json)
        let lines = pack_lines("intro", "lines");
        let total = lines.len() as f32 * 3.4 + 1.5;
        if t > total || lines.is_empty() {
            p.intro = None;
            let toast = pack_text("intro", "toast");
            if !toast.is_empty() {
                p.toast(toast);
            }
        } else {
            let bar = 90.0 * (t / 0.8).min(1.0).min(((total - t) / 0.8).max(0.0));
            painter.rect_filled(
                Rect::from_min_size(screen.min, Vec2::new(screen.width(), bar)),
                0.0,
                Color32::BLACK,
            );
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(screen.left(), screen.bottom() - bar),
                    Vec2::new(screen.width(), bar),
                ),
                0.0,
                Color32::BLACK,
            );
            let k = ((t - 0.6) / 3.4).floor().max(0.0) as usize;
            if let Some(line) = lines.get(k) {
                let local = (t - 0.6) - k as f32 * 3.4;
                let fade = (local / 0.6)
                    .min(1.0)
                    .min(((3.4 - local) / 0.6).max(0.0))
                    .clamp(0.0, 1.0);
                crate::theme::subtitle_text(
                    &painter,
                    Pos2::new(c.x, screen.bottom() - 45.0),
                    line,
                    20.0,
                    Color32::from_rgb(240, 225, 195).gamma_multiply(fade),
                );
            }
            painter.text(
                Pos2::new(screen.right() - 20.0, screen.top() + 20.0),
                Align2::RIGHT_TOP,
                "Esc to skip",
                FontId::proportional(12.0),
                Color32::from_gray(150),
            );
        }
    }
    // big title: "Day 3", "Dusk is coming"
    if let Some((at, text)) = &p.title {
        let t = at.elapsed().as_secs_f32();
        let fade = (t / 0.6).min(1.0) * (1.0 - (t - 3.2).max(0.0) / 1.3).clamp(0.0, 1.0);
        let pos = Pos2::new(c.x, screen.top() + screen.height() * 0.28);
        let galley = painter.layout_no_wrap(
            text.clone(),
            FontId::proportional(46.0),
            Color32::from_rgb(255, 236, 200).gamma_multiply(fade),
        );
        let r = Rect::from_center_size(pos, galley.size());
        painter.galley(
            r.min + Vec2::new(2.0, 3.0),
            painter.layout_no_wrap(
                text.clone(),
                FontId::proportional(46.0),
                Color32::from_black_alpha((160.0 * fade) as u8),
            ),
            Color32::BLACK,
        );
        painter.galley(r.min, galley, Color32::WHITE);
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
    // the combat log takes the panel's place while its tab is chosen
    let combat: Vec<(String, Color32)> = if p.chat_tab_combat {
        p.combat_log
            .iter()
            .rev()
            .take(12)
            .map(|(_, t, c)| (t.clone(), *c))
            .collect()
    } else {
        Vec::new()
    };
    let lines: Vec<(String, Color32)> = if p.chat_tab_combat {
        combat
    } else {
        visible
            .iter()
            .map(|m| match &m.from {
                // channel colours, as players expect them
                Some(f) if f.starts_with("[Party]") => {
                    (format!("{f}: {}", m.text), Color32::from_rgb(140, 190, 255))
                }
                Some(f) if f.starts_with("[From]") || f.starts_with("[To") => {
                    (format!("{f}: {}", m.text), Color32::from_rgb(240, 150, 230))
                }
                Some(f) if f.starts_with("[General]") => {
                    (format!("{f}: {}", m.text), Color32::from_rgb(250, 210, 170))
                }
                Some(f) if f.starts_with("[Trade]") => {
                    (format!("{f}: {}", m.text), Color32::from_rgb(250, 190, 140))
                }
                Some(f) => (format!("{f} says: {}", m.text), Color32::WHITE),
                None => (m.text.clone(), Color32::from_rgb(232, 200, 110)),
            })
            .collect()
    };
    for (txt, col) in lines {
        let galley = painter.layout(txt, FontId::proportional(14.0), col, 520.0);
        y -= galley.size().y + 4.0;
        let r = Rect::from_min_size(
            Pos2::new(screen.left() + 10.0, y - 2.0),
            galley.size() + Vec2::new(10.0, 4.0),
        );
        painter.rect_filled(r, 3.0, Color32::from_black_alpha(120));
        painter.galley(Pos2::new(screen.left() + 15.0, y), galley, col);
    }
    // tabs: Chat and Combat
    egui::Area::new(egui::Id::new("chat_tabs"))
        .fixed_pos(Pos2::new(
            screen.left() + 10.0,
            y.min(chat_bottom - 20.0) - 28.0,
        ))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (label, combat) in [("Chat", false), ("Combat", true)] {
                    let on = p.chat_tab_combat == combat;
                    let t = RichText::new(label).size(12.0).color(if on {
                        crate::theme::GOLD_BRIGHT
                    } else {
                        crate::theme::TEXT_DIM
                    });
                    if ui
                        .add(egui::Button::new(t).fill(Color32::from_black_alpha(if on {
                            170
                        } else {
                            90
                        })))
                        .clicked()
                    {
                        p.chat_tab_combat = combat;
                    }
                }
            });
        });
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
                    // emotes play on your own figure (others cannot see them yet)
                    let emote = t.starts_with('/').then(|| Anim::emote(&t)).flatten();
                    if let Some(e) = emote {
                        // everyone nearby sees it
                        if let Some(n) = Anim::ALL.iter().position(|a| *a == e) {
                            let _ = p.conn.tx.send(ClientMsg::Emote(n as u8));
                        }
                        let now = p.anims.now();
                        if e.one_shot() {
                            p.anims.me(now).play(now, e);
                        } else {
                            p.anims.hold = Some(e);
                        }
                    } else if let Some(who) = t.strip_prefix("/trade ") {
                        // trade with a player near you, by name
                        let who = who.trim();
                        match p
                            .remotes
                            .iter()
                            .find(|(_, r)| r.name.eq_ignore_ascii_case(who))
                        {
                            Some((id, _)) => {
                                let _ = p
                                    .conn
                                    .tx
                                    .send(ClientMsg::Trade(dyno_protocol::TradeOp::Ask(*id)));
                            }
                            None => p.chat.push(Chat {
                                at: Instant::now(),
                                from: None,
                                text: format!("No one called '{who}' is near you."),
                            }),
                        }
                    } else if !t.is_empty() {
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
    // character / bags / spellbook / skills
    if let Some(tab) = p.screen_tab {
        let names: Vec<(String, u16, u16)> = p
            .skills
            .iter()
            .map(|(k, v, m)| {
                let n = p
                    .content
                    .professions
                    .iter()
                    .find(|x| &x.key == k)
                    .map(|x| x.name.clone())
                    .unwrap_or_else(|| k.clone());
                (n, *v, *m)
            })
            .collect();
        let lines: Vec<screens::SkillLine> = names
            .iter()
            .map(|(n, v, m)| screens::SkillLine {
                name: n,
                value: *v,
                max: *m,
            })
            .collect();
        let extra = buff_stats(p);
        let v = screens::View {
            content: &p.content,
            name: &p.my_name,
            class: p.class.and_then(|c| p.content.class(c)),
            level: p.level,
            money: p.money,
            pack: &p.pack,
            equip: &p.equip,
            health: p.health,
            power: p.power,
            trading: p.trade.is_some(),
            at_vendor: matches!(
                p.panel,
                Some(rpg_ui::Panel::Dialog { .. } | rpg_ui::Panel::Vendor { .. })
            ),
            bar: &p.bar,
            skills: &lines,
            extra,
        };
        let cmds = screens::window(ctx, &v, tab);
        if !cmds.is_empty() {
            actions.push(Action::Screens(cmds));
        }
    }
    // buffs, top right
    {
        let now = Instant::now();
        p.auras.retain(|(_, t)| *t > now);
        for (k, (a, t)) in p.auras.iter().enumerate() {
            let Some(ab) = p.content.ability(*a) else {
                continue;
            };
            let r = Rect::from_min_size(
                Pos2::new(map.left() - 44.0 - k as f32 * 40.0, screen.top() + 12.0),
                Vec2::splat(34.0),
            );
            painter.rect_filled(r, 4.0, Color32::from_black_alpha(160));
            if let Some(item) = p.content.item_by_key(&ab.icon) {
                survival::item_icon(&painter, r.shrink(3.0), &p.content, item);
            }
            painter.rect_stroke(
                r,
                4.0,
                Stroke::new(1.0_f32, Color32::from_rgb(200, 170, 90)),
                egui::StrokeKind::Inside,
            );
            let left = t.saturating_duration_since(now).as_secs_f32();
            let txt = if left >= 60.0 {
                format!("{:.0}m", left / 60.0)
            } else {
                format!("{left:.0}s")
            };
            painter.text(
                r.center_bottom() + Vec2::new(0.0, 2.0),
                Align2::CENTER_TOP,
                txt,
                FontId::proportional(11.0),
                Color32::WHITE,
            );
        }
    }
    // combo points under the target frame
    if p.combo.0 > 0 && p.combo.1.is_some() && p.combo.1 == p.focus.or(p.target_entity) {
        for k in 0..5u8 {
            let at = Pos2::new(c.x - 40.0 + k as f32 * 20.0, screen.top() + 80.0);
            let on = k < p.combo.0;
            painter.circle_filled(
                at,
                7.0,
                if on {
                    Color32::from_rgb(230, 60, 40)
                } else {
                    Color32::from_black_alpha(150)
                },
            );
            painter.circle_stroke(
                at,
                7.0,
                Stroke::new(1.0_f32, Color32::from_rgb(200, 170, 90)),
            );
        }
    }
    // floating combat text, rising from the creature (or from you)
    p.combat_text
        .retain(|(t, _, _, _)| t.elapsed() < Duration::from_millis(1400));
    if let Some(vp) = vp {
        for (k, (t, ent, text, col)) in p.combat_text.iter().enumerate() {
            let a = t.elapsed().as_secs_f32();
            let world = match ent {
                Some(id) => p.entities.get(id).map(|e| e.pos() + glam::Vec3::Y * 2.2),
                None => Some(p.player.pos + glam::Vec3::Y * 2.0),
            };
            let Some(wp) = world else { continue };
            let clip = vp * (wp + glam::Vec3::Y * a * 0.8).extend(1.0);
            if clip.w <= 0.1 {
                continue;
            }
            let ndc = clip.truncate() / clip.w;
            let at = Pos2::new(
                screen.left() + (ndc.x * 0.5 + 0.5) * screen.width() + (k % 3) as f32 * 10.0,
                screen.top() + (0.5 - ndc.y * 0.5) * screen.height(),
            );
            let fade = (1.0 - a / 1.4).clamp(0.0, 1.0);
            let size = if text.ends_with('!') { 26.0 } else { 19.0 };
            painter.text(
                at + Vec2::new(1.5, 1.5),
                Align2::CENTER_CENTER,
                text,
                FontId::proportional(size),
                Color32::from_black_alpha((220.0 * fade) as u8),
            );
            painter.text(
                at,
                Align2::CENTER_CENTER,
                text,
                FontId::proportional(size),
                col.gamma_multiply(fade),
            );
        }
    }
    // class picker
    if p.class_pick {
        let mut chosen = None;
        egui::Window::new(
            RichText::new("Choose your path")
                .size(22.0)
                .color(Color32::from_rgb(255, 210, 80)),
        )
        .id(egui::Id::new("class-pick"))
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_width(640.0);
            ui.label(pack_text("classes", "prompt"));
            ui.add_space(8.0);
            ui.columns(p.content.classes.len().max(1), |cols| {
                for (k, c) in p.content.classes.iter().enumerate() {
                    let ui = &mut cols[k];
                    let col = Color32::from_rgb(
                        (c.color[0] * 255.0) as u8,
                        (c.color[1] * 255.0) as u8,
                        (c.color[2] * 255.0) as u8,
                    );
                    ui.label(RichText::new(&c.name).size(20.0).strong().color(col));
                    ui.label(RichText::new(&c.blurb).small());
                    ui.add_space(4.0);
                    for a in p.content.abilities.iter().filter(|a| a.class == c.id) {
                        ui.label(
                            RichText::new(format!("{} (level {})", a.name, a.level))
                                .small()
                                .color(Color32::from_gray(210)),
                        )
                        .on_hover_text(&a.text);
                    }
                    ui.add_space(6.0);
                    if ui
                        .button(RichText::new(format!("Become a {}", c.name)).strong())
                        .clicked()
                    {
                        chosen = Some(c.id);
                    }
                }
            });
        });
        if let Some(c) = chosen {
            actions.push(Action::ChooseClass(c));
        }
    }
    // my cast bar
    if let Some((a, start, secs)) = p.my_cast {
        let name = if a == dyno_protocol::RECALL {
            "Recall".to_string()
        } else if a == dyno_protocol::GATHER {
            "Gathering".to_string()
        } else {
            p.content
                .ability(a)
                .map(|a| a.name.clone())
                .unwrap_or_default()
        };
        let f = (start.elapsed().as_secs_f32() / secs.max(0.01)).min(1.0);
        let r = Rect::from_center_size(
            Pos2::new(c.x, screen.bottom() - 150.0),
            Vec2::new(260.0, 18.0),
        );
        painter.rect_filled(r.expand(2.0), 4.0, Color32::from_black_alpha(200));
        painter.rect_filled(
            Rect::from_min_size(r.min, Vec2::new(r.width() * f, r.height())),
            3.0,
            Color32::from_rgb(230, 180, 60),
        );
        painter.rect_stroke(
            r.expand(2.0),
            4.0,
            Stroke::new(1.5_f32, Color32::from_rgb(200, 160, 80)),
            egui::StrokeKind::Outside,
        );
        painter.text(
            r.center(),
            Align2::CENTER_CENTER,
            name,
            FontId::proportional(13.0),
            Color32::WHITE,
        );
    }
    // rage or mana under the player frame
    if p.content.world.protected && p.power_max > 0.0 {
        let rage = p
            .class
            .and_then(|c| p.content.class(c))
            .is_some_and(|c| c.resource == dyno_world::content::Resource::Rage);
        let r = Rect::from_min_size(
            screen.left_top() + Vec2::new(22.0, 108.0),
            Vec2::new(210.0, 10.0),
        );
        painter.rect_filled(r, 3.0, Color32::from_rgb(20, 20, 40));
        let col = if rage {
            Color32::from_rgb(200, 40, 40)
        } else {
            Color32::from_rgb(50, 110, 230)
        };
        painter.rect_filled(
            Rect::from_min_size(
                r.min,
                Vec2::new(
                    r.width() * (p.power / p.power_max).clamp(0.0, 1.0),
                    r.height(),
                ),
            ),
            3.0,
            col,
        );
        painter.text(
            r.center(),
            Align2::CENTER_CENTER,
            format!("{} / {}", p.power.floor() as i32, p.power_max as i32),
            FontId::proportional(10.0),
            Color32::WHITE,
        );
    }
    // RPG windows
    if let Some(mut panel) = p.panel.clone() {
        let me = rpg_ui::Me {
            content: &p.content,
            money: p.money,
            level: p.level,
            log: &p.quest_log,
            done: &p.quests_done,
            pos: p.player.pos.into(),
            hearth: p.hearth,
        };
        let cmds = match panel {
            rpg_ui::Panel::Dialog { .. } => rpg_ui::dialog_window(ctx, &me, &mut panel),
            rpg_ui::Panel::Log { .. } => rpg_ui::log_window(ctx, &me, &mut panel),
            rpg_ui::Panel::Vendor { .. } => rpg_ui::vendor_window(ctx, &me, &mut panel),
        };
        p.panel = Some(panel);
        if !cmds.is_empty() {
            actions.push(Action::Rpg(cmds));
        }
    }
    // party frames, under my own
    let others: Vec<&dyno_protocol::PartyMember> =
        p.party.iter().filter(|m| m.name != p.my_name).collect();
    for (i, m) in others.iter().enumerate() {
        let r = Rect::from_min_size(
            Pos2::new(screen.left() + 14.0, screen.top() + 96.0 + i as f32 * 40.0),
            Vec2::new(170.0, 34.0),
        );
        painter.rect_filled(r, 4.0, Color32::from_rgba_unmultiplied(20, 14, 10, 200));
        painter.rect_stroke(
            r,
            4.0,
            Stroke::new(1.0_f32, crate::theme::GOLD_DARK),
            egui::StrokeKind::Inside,
        );
        let col = m
            .class
            .and_then(|c| p.content.class(c))
            .map(|c| {
                Color32::from_rgb(
                    (c.color[0] * 255.0) as u8,
                    (c.color[1] * 255.0) as u8,
                    (c.color[2] * 255.0) as u8,
                )
            })
            .unwrap_or(crate::theme::TEXT);
        let label = if m.online {
            format!(
                "{}{}  {}",
                if m.leader { "* " } else { "" },
                m.name,
                m.level
            )
        } else {
            format!("{} (offline)", m.name)
        };
        painter.text(
            r.left_top() + Vec2::new(8.0, 3.0),
            Align2::LEFT_TOP,
            label,
            crate::theme::body_font(13.0),
            if m.online {
                col
            } else {
                Color32::from_gray(130)
            },
        );
        let bar = Rect::from_min_size(r.left_top() + Vec2::new(8.0, 21.0), Vec2::new(154.0, 7.0));
        painter.rect_filled(bar, 2.0, Color32::from_black_alpha(200));
        let f = (m.health / m.max.max(1.0)).clamp(0.0, 1.0);
        painter.rect_filled(
            Rect::from_min_size(bar.min, Vec2::new(bar.width() * f, bar.height())),
            2.0,
            Color32::from_rgb(60, 180, 70),
        );
    }
    // a party invitation
    if let Some(from) = p.invite.clone() {
        let mut answer = None;
        egui::Window::new("Party invitation")
            .id(egui::Id::new("party-invite"))
            .title_bar(false)
            .resizable(false)
            .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 120.0))
            .frame(crate::theme::window_frame())
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(format!("{from} invites you to join their party."))
                        .font(crate::theme::body_font(16.0))
                        .color(crate::theme::TEXT),
                );
                ui.horizontal(|ui| {
                    if ui.button("Accept").clicked() {
                        answer = Some("/accept");
                    }
                    if ui.button("Decline").clicked() {
                        answer = Some("/decline");
                    }
                });
            });
        if let Some(a) = answer {
            let _ = p.conn.tx.send(ClientMsg::Chat { text: a.into() });
            p.invite = None;
        }
    }
    // someone asks to trade
    if let Some(from) = p.trade_ask.clone() {
        let mut answer = None;
        egui::Window::new("Trade request")
            .id(egui::Id::new("trade-ask"))
            .title_bar(false)
            .resizable(false)
            .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 190.0))
            .frame(crate::theme::window_frame())
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(format!("{from} wants to trade with you."))
                        .font(crate::theme::body_font(16.0))
                        .color(crate::theme::TEXT),
                );
                ui.horizontal(|ui| {
                    if ui.button("Trade").clicked() {
                        answer = Some(dyno_protocol::TradeOp::Agree);
                    }
                    if ui.button("Decline").clicked() {
                        answer = Some(dyno_protocol::TradeOp::Cancel);
                    }
                });
            });
        if let Some(a) = answer {
            let _ = p.conn.tx.send(ClientMsg::Trade(a));
            p.trade_ask = None;
        }
    }
    // the trade window
    if let Some(view) = p.trade.clone() {
        let mut ops = Vec::new();
        let mut open = true;
        crate::theme::window(
            ctx,
            "trade",
            "Trade",
            &format!("with {}", view.with),
            430.0,
            (Align2::LEFT_CENTER, Vec2::new(40.0, 0.0)),
            &mut open,
            |ui| {
                ui.label(
                    RichText::new("Right-click items in your bags to offer them. Click a line to take it back.")
                        .small()
                        .color(crate::theme::TEXT_DIM),
                );
                ui.columns(2, |cols| {
                    for (k, (title, items, money, accept)) in [
                        ("You offer", &view.mine, view.my_money, view.i_accept),
                        (
                            view.with.as_str(),
                            &view.theirs,
                            view.their_money,
                            view.they_accept,
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let ui = &mut cols[k];
                        ui.label(
                            RichText::new(title)
                                .font(crate::theme::title_font(14.0))
                                .color(crate::theme::GOLD),
                        );
                        for (i, st) in items.iter().enumerate() {
                            let it = p.content.item(st.item);
                            let label = if st.count > 1 {
                                format!("{} ×{}", it.name, st.count)
                            } else {
                                it.name.to_string()
                            };
                            let r = ui.add(egui::Button::new(
                                RichText::new(label).color(crate::theme::quality_color(it.quality)),
                            ));
                            if k == 0 && r.clicked() {
                                ops.push(dyno_protocol::TradeOp::Withdraw { line: i as u8 });
                            }
                        }
                        ui.label(
                            RichText::new(dyno_sim::progression::money_text(money))
                                .color(crate::theme::GOLD),
                        );
                        ui.label(
                            RichText::new(if accept {
                                "Accepted"
                            } else {
                                "Not yet accepted"
                            })
                            .small()
                            .color(if accept {
                                Color32::from_rgb(120, 210, 120)
                            } else {
                                crate::theme::TEXT_DIM
                            }),
                        );
                    }
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label("Coins (copper):");
                    let r =
                        ui.add(egui::TextEdit::singleline(&mut p.trade_money).desired_width(80.0));
                    if r.lost_focus()
                        && let Ok(n) = p.trade_money.trim().parse::<u32>()
                    {
                        ops.push(dyno_protocol::TradeOp::Money(n));
                    }
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new("Accept").color(crate::theme::GOLD_BRIGHT))
                        .clicked()
                    {
                        ops.push(dyno_protocol::TradeOp::Accept);
                    }
                    if ui.button("Cancel").clicked() {
                        ops.push(dyno_protocol::TradeOp::Cancel);
                    }
                });
            },
        );
        if !open {
            ops.push(dyno_protocol::TradeOp::Cancel);
        }
        for op in ops {
            let _ = p.conn.tx.send(ClientMsg::Trade(op));
        }
    }
    // an open corpse
    if let Some((id, money, items)) = &p.loot {
        let gone = p
            .entities
            .get(id)
            .is_none_or(|e| (e.pos() - p.player.pos).length() > 7.0);
        let cmds = if gone {
            vec![rpg_ui::RpgCmd::CloseLoot]
        } else {
            rpg_ui::loot_window(ctx, &p.content, *id, *money, items)
        };
        if !cmds.is_empty() {
            actions.push(Action::Rpg(cmds));
        }
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
        egui::Window::new("Menu").frame(crate::theme::window_frame()).collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, Vec2::ZERO).show(ctx, |ui| {
            ui.set_width(380.0);
            if ui.add_sized([ui.available_width(), 32.0], egui::Button::new("Back to game")).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                actions.push(Action::Resume);
            }
            ui.add_space(6.0);
            if crate::options::panel(ui) {
                actions.push(Action::Graphics);
            }
            ui.add_space(6.0);
            host_panel(ui, host, actions);
            ui.add_space(6.0);
            ui.label(RichText::new(format!("Connected to {}", p.url)).small().weak());
            let controls = if p.wow() {
                "Controls: hold right mouse to steer · hold left mouse to look around · both to run · W S move · A D turn · Q E strafe · R autorun · wheel zoom · left click select · right click attack, talk or gather · Tab target · 1-0 - = abilities · C B P K L windows · T chat"
            } else {
                "Controls: WASD move · Space jump · Shift down when flying · Ctrl or R sprint · F fly · left click break · right click place · middle click pick · 1-9 or wheel blocks · E all blocks · G corner, V fill, X clear · Ctrl+Z undo · T chat · Tab players"
            };
            ui.label(RichText::new(controls).small().weak());
            if ui.button("Leave server").clicked() {
                actions.push(Action::Disconnect);
            }
        });
    } else if !p.wow()
        && !p.captured
        && !p.chat_open
        && !p.picker_open
        && p.panel.is_none()
        && p.dead.is_none()
    {
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
            Ok(mut g) => {
                eprintln!("graphics: {}", g.adapter_info);
                // nothing chosen yet: the tier picked for this adapter, and
                // what goes with it
                if self.settings.graphics > 3 {
                    crate::options::Graphics::preset(g.graphics_tier).into(&mut self.settings);
                    let _ = self.settings.save();
                }
                let opts = crate::options::Graphics::of(&self.settings);
                crate::options::set(opts);
                g.water.quality = opts.water;
                g.cover.set_quality(opts.cover);
                crate::foliage::set_detail(opts.leaves);
                crate::theme::apply(&self.egui_ctx);
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
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x, position.y);
            }
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
                    let wow_free = matches!(&self.screen, Screen::Playing(p) if p.wow() && !p.chat_open && !p.menu_open)
                        && !self.egui_ctx.wants_keyboard_input();
                    if captured || wow_free {
                        self.key(code, pressed, event.repeat);
                    }
                }
            }
            WindowEvent::MouseInput {
                state,
                button: button @ (MouseButton::Left | MouseButton::Right),
                ..
            } if matches!(&self.screen, Screen::Playing(p) if p.wow() && !p.menu_open && !p.chat_open && !p.class_pick && p.dead.is_none()) =>
            {
                let over_ui =
                    self.egui_ctx.is_pointer_over_area() || self.egui_ctx.wants_pointer_input();
                let Screen::Playing(p) = &mut self.screen else {
                    return;
                };
                let held = p.lmb || p.rmb;
                if state == ElementState::Pressed {
                    if over_ui && !held {
                        return; // the interface has it
                    }
                    if !held {
                        p.drag = 0.0;
                    }
                    if button == MouseButton::Right {
                        p.rmb = true;
                        // the character turns to where the camera looks
                        p.player.yaw += p.cam_orbit;
                        p.cam_orbit = 0.0;
                    } else {
                        p.lmb = true;
                    }
                    if !held {
                        self.set_mouselook(true);
                    }
                } else {
                    if !held {
                        return;
                    }
                    if button == MouseButton::Right {
                        p.rmb = false;
                    } else {
                        p.lmb = false;
                    }
                    let click = p.drag < 6.0;
                    if !(p.lmb || p.rmb) {
                        self.set_mouselook(false);
                        if click {
                            self.world_click(button);
                        }
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
                    Screen::Playing(p) => (
                        !p.captured,
                        p.menu_open
                            || p.chat_open
                            || p.picker_open
                            || p.panel.is_some()
                            || p.class_pick
                            || p.screen_tab.is_some(),
                    ),
                    _ => (false, true),
                };
                if playing_free && !menu_or_chat && button == MouseButton::Left {
                    self.set_capture(true);
                } else if captured {
                    self.click(button);
                }
            }
            WindowEvent::MouseWheel { delta, .. }
                if captured
                    || (matches!(&self.screen, Screen::Playing(p) if p.wow())
                        && !self.egui_ctx.is_pointer_over_area()) =>
            {
                if let Screen::Playing(p) = &mut self.screen {
                    let d = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(p) => (p.y / 40.0) as f32,
                    };
                    let n = 9;
                    if p.content.world.protected && !p.creative() {
                        // RPG: the wheel zooms the camera
                        p.zoom = (p.zoom - d * 1.2).clamp(0.0, 12.0);
                        if p.zoom < 1.2 && d > 0.0 {
                            p.zoom = 0.0;
                        }
                    } else if d.abs() > 0.1 {
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
            if p.wow() {
                p.drag += (delta.0.abs() + delta.1.abs()) as f32;
                if p.rmb {
                    p.player.yaw -= delta.0 as f32 * s;
                } else {
                    p.cam_orbit -= delta.0 as f32 * s;
                }
                p.player.pitch = (p.player.pitch - delta.1 as f32 * s).clamp(-1.4, 1.4);
                return;
            }
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

/// Render a front-end page (title, realms, select, class, look, name,
/// loading) for `dyno shot --front` (agents and design review). `addr` is a
/// realm to list; `as_name` picks the install whose characters are shown.
pub fn shot_front(
    gfx: &mut Gfx,
    page: &str,
    addr: &str,
    as_name: Option<&str>,
    out: &std::path::Path,
) -> Result<String, String> {
    // the title stage and words from the pack, as the game shows them
    #[cfg(feature = "host")]
    if crate::assets::get("pack.json").is_none() {
        crate::load_builtin_pack();
    }
    let Stage {
        world,
        light,
        hearth,
    } = pack_stage().ok_or("the pack has no title stage (content/stage)")?;
    gfx.set_focus(Vec3::new(
        hearth[0] as f32,
        hearth[1] as f32,
        hearth[2] as f32,
    ));
    gfx.load_world(&world, &light);
    let recent = if addr.is_empty() {
        Vec::new()
    } else {
        vec![addr.to_string()]
    };
    let mut f = Front::new(&recent, "");
    let token = as_name.map(crate::settings::tool_token).unwrap_or(0);
    if !addr.is_empty() {
        match net::realm(addr, token, None, Duration::from_secs(6)) {
            Ok((info, chars, ping)) => {
                let r = &mut f.realms[0];
                r.info = Some(info);
                r.chars = chars;
                r.ping = Some(ping);
            }
            Err(e) => f.realms[0].error = Some(e),
        }
    }
    f.page = match page {
        "realms" => front::Page::Realms,
        "select" => front::Page::Select,
        "class" => front::Page::Class,
        "look" => front::Page::Appearance,
        "name" => front::Page::Name,
        _ => front::Page::Title,
    };
    f.class = 1;
    f.randomise(42);
    if page == "name" {
        f.name = front::random_name(7);
    }
    let (wdt, hgt) = (gfx.config.width as f32, gfx.config.height as f32);
    let far = 160.0;
    let h = Vec3::new(
        hearth[0] as f32 + 0.5,
        hearth[1] as f32,
        hearth[2] as f32 + 0.5,
    );
    let mut boxes = Vec::new();
    let (eye, at) = match f.preview() {
        Some(look) => {
            let feet = h + Vec3::new(0.0, 1.0, 3.0);
            let feet = Vec3::new(
                feet.x,
                world.surface(feet.x as i32, feet.z as i32) as f32 + 1.0,
                feet.z,
            );
            models::push_person(
                &mut boxes,
                feet,
                std::f32::consts::PI + 0.35,
                0.0,
                &look,
                1.0,
                0.0,
                0.0,
            );
            (
                feet + Vec3::new(0.0, 1.5, 5.2),
                feet + Vec3::new(0.0, 1.05, 0.0),
            )
        }
        None => (
            h + Vec3::new(30.0, 22.0, 32.0),
            h + Vec3::new(0.0, 6.0, 0.0),
        ),
    };
    let dir = (at - eye).normalize_or_zero();
    let vp = glam::Mat4::from_translation(Vec3::new(f.preview_offset(), 0.0, 0.0))
        * glam::Mat4::perspective_rh(55f32.to_radians(), wdt / hgt, 0.05, far)
        * glam::Mat4::look_to_rh(eye, dir, Vec3::Y);
    let ctx = crate::theme::new_context();
    crate::theme::apply(&ctx);
    ctx.style_mut(|s| s.animation_time = 0.0);
    let raw = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(wdt, hgt),
        )),
        ..Default::default()
    };
    let sky_now = sky(0.47);
    let mut cmds = Vec::new();
    let loading = page == "loading";
    let mut draw = |ctx: &egui::Context, cmds: &mut Vec<FrontCmd>| {
        if loading {
            let (_tx, rx) = channel();
            let progress = Arc::new(net::Progress::default());
            progress
                .total
                .store(214_597, std::sync::atomic::Ordering::Relaxed);
            progress
                .done
                .store(96_000, std::sync::atomic::Ordering::Relaxed);
            let mut screen = Screen::Joining {
                rx,
                label: addr.to_string(),
                since: Instant::now() - Duration::from_secs(2),
                progress,
            };
            let mut menu = Menu {
                addr: String::new(),
                name: String::new(),
                error: None,
            };
            let settings = Settings::default();
            let mut host = None;
            let mut acts = Vec::new();
            ui(
                ctx,
                &mut screen,
                &mut f,
                &mut menu,
                &settings,
                &mut host,
                "Up to date",
                None,
                &mut acts,
            );
        } else {
            front::ui(
                ctx,
                &mut f,
                env!("CARGO_PKG_VERSION"),
                "Up to date",
                false,
                cmds,
            );
        }
    };
    let first = ctx.run(raw(), |ctx| draw(ctx, &mut cmds));
    gfx.frame(Some((vp, eye, far)), &sky_now, &boxes, &[], &ctx, first);
    let second = ctx.run(raw(), |ctx| draw(ctx, &mut cmds));
    gfx.screenshot = Some(out.to_path_buf());
    gfx.frame(Some((vp, eye, far)), &sky_now, &boxes, &[], &ctx, second);
    Ok(format!("rendered front page {page} to {}", out.display()))
}

/// Render one frame with the real HUD, for `dyno shot --ui` (tests and agents).
/// `ui` is "hud", "pack", "dig" or "dead"; or "stand" for the world alone
/// with a stand-in figure on the ground at `pos` (see `shot::Stand`; a
/// negative height there means the open ground of the column).
#[allow(clippy::too_many_arguments)]
pub fn shot_with_ui(
    j: Joined,
    gfx: &mut Gfx,
    pos: Option<[f32; 3]>,
    yaw: f32,
    pitch: f32,
    time: Option<f32>,
    ui: &str,
    out: std::path::PathBuf,
    zoom: Option<f32>,
) -> Result<String, String> {
    let mut p = new_play(j);
    if let Some(z) = zoom {
        p.zoom = z;
    }
    let stand = ui == "stand";
    // a stand shot waits for its whole view, not just for the stream to pause
    let view_missing = |p: &Play| {
        stand
            && pos.is_some_and(|at| {
                net::sections_around(&p.world, at, p.view - 2.0 * w::SECTION as f32)
                    .iter()
                    .any(|s| !p.world.has_section(*s))
            })
    };
    let stats = std::env::var_os("DYNO_SHOT_STATS").is_some();
    // DYNO_SHOT_STATS=gpu: each frame waits for the GPU, so its time counts
    let stats_gpu = std::env::var("DYNO_SHOT_STATS").is_ok_and(|v| v == "gpu");
    // the camera: where asked (the server streams the world round it), else
    // the character; DYNO_SHOT_RUN="x0,z0,x1,z1[,speed]" flies it across
    // the map to measure frames while the land streams in
    let run = shot_run();
    let start_cam = run
        .map(|r| [r.0[0], p.player.pos.y + 40.0, r.0[1]])
        .or(pos)
        .unwrap_or(p.player.pos.into());
    if run.is_some() || pos.is_some() {
        let _ = p.conn.tx.send(ClientMsg::ViewAt { pos: start_cam });
    }
    p.player.pos = start_cam.into();
    p.player.yaw = yaw;
    p.player.pitch = pitch;
    // the world is drawn as play draws it: lit and meshed on the world
    // thread, uploaded within a budget each frame, frames rendered at ~60 Hz
    gfx.begin_world(&p.world);
    let fctx = crate::theme::new_context();
    let mut frames: Vec<(f64, f64)> = Vec::new();
    let mut land_at: Option<Duration> = None;
    let started = Instant::now();
    let mut last_section = Instant::now();
    let mut last_view = Instant::now();
    loop {
        let frame_t0 = Instant::now();
        let mut world_t = Duration::ZERO;
        loop {
            let ev = p.conn.rx.try_recv();
            if ev.is_err() {
                break;
            }
            match ev {
                Ok(NetEvent::Msg(m)) => match m {
                    ServerMsg::Sections(list) => {
                        let t = Instant::now();
                        sections_in(&mut p, list);
                        world_t += t.elapsed();
                        last_section = Instant::now();
                    }
                    ServerMsg::SectionsGone(list) => {
                        let t = Instant::now();
                        sections_out(&mut p, Some(&mut *gfx), list);
                        world_t += t.elapsed();
                    }
                    ServerMsg::Far(list) => {
                        let t = Instant::now();
                        far_in(&mut p, list);
                        world_t += t.elapsed();
                        last_section = Instant::now();
                    }
                    ServerMsg::FarGone(list) => far_out(&mut p, Some(&mut *gfx), list),
                    m @ (ServerMsg::Block { .. } | ServerMsg::Fill { .. }) => {
                        net::apply_world(&mut p.world, &m);
                        match m {
                            ServerMsg::Block { pos, block } => p.work.block(pos, block),
                            ServerMsg::Fill { a, b, block } => p.work.fill(a, b, block),
                            _ => {}
                        }
                    }
                    ServerMsg::Pack(s) => p.pack = s,
                    ServerMsg::Status {
                        health,
                        food,
                        mode,
                        fuel,
                    } => {
                        p.health = health;
                        p.food = food;
                        p.mode = mode;
                        p.fuel = fuel;
                    }
                    ServerMsg::Time { time, day_seconds } => {
                        p.time = time;
                        p.day_seconds = day_seconds;
                        p.time_at = Instant::now();
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
                                hurt_at: None,
                                info: None,
                                name: None,
                            },
                        );
                    }
                    ServerMsg::Character {
                        look,
                        class,
                        level,
                        xp,
                        xp_next,
                        money,
                        max_health,
                    } => {
                        p.class = class;
                        p.my_look = look;
                        if class.is_some() {
                            p.class_pick = false;
                        }
                        p.level = level;
                        p.xp = xp;
                        p.xp_next = xp_next;
                        p.money = money;
                        p.max_health = max_health;
                    }
                    ServerMsg::EntityMoves(list) => {
                        for (id, pos, yaw) in list {
                            if let Some(e) = p.entities.get_mut(&id) {
                                e.from = pos;
                                e.to = pos;
                                e.yaw = yaw;
                            }
                        }
                    }
                    ServerMsg::EntityGone { id, .. } => {
                        p.entities.remove(&id);
                    }
                    ServerMsg::Act {
                        actor: dyno_protocol::Actor::Entity(id),
                        act: dyno_protocol::Act::Emote(n),
                    } => {
                        entity_emote(&mut p, id, n);
                    }
                    ServerMsg::Weather { kind, strength } => {
                        p.weather = (kind, strength);
                        p.weather_mix = strength;
                    }
                    ServerMsg::Equipment(e) => p.equip = e,
                    ServerMsg::Zone { name, .. } => p.zone = name,
                    ServerMsg::Skills(sk) => p.skills = sk,
                    ServerMsg::Power { value, max } => {
                        p.power = value;
                        p.power_max = max;
                    }
                    ServerMsg::Quests { log, done } => {
                        p.quest_log = log;
                        p.quests_done = done.into_iter().collect();
                    }
                    ServerMsg::EntityInfo {
                        id,
                        level,
                        health,
                        max,
                    } => {
                        if let Some(e) = p.entities.get_mut(&id) {
                            e.info = Some((level, health, max));
                        }
                    }
                    ServerMsg::Container(c) => p.chest = c,
                    ServerMsg::Milestone { title } => p.toast(format!("Journal: {title}")),
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
                    _ => {}
                },
                Ok(NetEvent::Sections(list)) => {
                    let t = Instant::now();
                    sections_ready(&mut p, list);
                    world_t += t.elapsed();
                    last_section = Instant::now();
                }
                Ok(NetEvent::Content(c)) => p.content = *c,
                Ok(NetEvent::Closed(r)) => return Err(r),
                Err(_) => {}
            }
        }
        let a = world_uploads(&mut p, Some(&mut *gfx), Duration::from_millis(3));
        let eye = p.player.eye();
        let b = far_uploads(&mut p, gfx, eye, Duration::from_millis(2));
        world_t += a + b;
        // the camera, flying or still
        let t = started.elapsed().as_secs_f32();
        let flying = if let Some((from, to, speed)) = run {
            let len = (to[0] - from[0]).hypot(to[1] - from[1]).max(1.0);
            let k = (t * speed / len).min(1.0);
            let (x, z) = (
                from[0] + (to[0] - from[0]) * k,
                from[1] + (to[1] - from[1]) * k,
            );
            let ground = p.world.surface(x as i32, z as i32);
            let y = if ground > 0 {
                ground as f32 + 3.0
            } else {
                p.player.pos.y
            };
            p.player.pos = Vec3::new(x, y, z);
            p.player.yaw = (-(to[0] - from[0])).atan2(-(to[1] - from[1]));
            p.player.pitch = -0.1;
            if last_view.elapsed() > Duration::from_millis(100) {
                last_view = Instant::now();
                let _ = p.conn.tx.send(ClientMsg::ViewAt {
                    pos: p.player.pos.into(),
                });
            }
            k < 1.0
        } else {
            false
        };
        p.work
            .focus(p.player.eye().into(), p.player.look_dir().into());
        if land_at.is_none() && land_ready(&p, gfx, p.player.pos, LAND_RADIUS) {
            land_at = Some(started.elapsed());
        }
        if stats || run.is_some() {
            // a frame of the world, as the game draws it
            let (fw, fh) = (gfx.config.width as f32, gfx.config.height as f32);
            let see = if p.far.is_empty() { 160.0 } else { FAR_SEE };
            let vp = p.player.view_proj(fw / fh, 70.0, see);
            gfx.water.look_from(
                &p.world,
                p.player.eye().into(),
                p.hearth.unwrap_or_default(),
            );
            let sky_now = crate::sky::sky_at(p.time, Some(p.player.eye()));
            let out = fctx.run(egui::RawInput::default(), |_| {});
            gfx.frame(
                Some((vp, p.player.eye(), see)),
                &sky_now,
                &[],
                &[],
                &fctx,
                out,
            );
            if stats_gpu {
                gfx.wait_idle();
            }
            frames.push((
                world_t.as_secs_f64() * 1e3,
                frame_t0.elapsed().as_secs_f64() * 1e3,
            ));
        }
        let quiet = started.elapsed() > Duration::from_millis(700)
            && last_section.elapsed() > Duration::from_millis(250)
            && p.work.idle();
        let limit = if run.is_some() {
            120
        } else if stand {
            40
        } else {
            8
        };
        if (!flying && quiet && !view_missing(&p)) || started.elapsed() > Duration::from_secs(limit)
        {
            break;
        }
        // ~60 frames a second
        if let Some(rest) = Duration::from_millis(16).checked_sub(frame_t0.elapsed()) {
            std::thread::sleep(rest);
        }
    }
    // everything made, uploaded (the picture is complete)
    while !p.work.idle() {
        world_uploads(&mut p, Some(&mut *gfx), Duration::from_secs(1));
        std::thread::sleep(Duration::from_millis(5));
    }
    world_uploads(&mut p, Some(&mut *gfx), Duration::from_secs(1));
    let eye = p.player.eye();
    far_uploads(&mut p, gfx, eye, Duration::from_secs(5));
    p.land_wait = !land_ready(&p, gfx, p.player.pos, LAND_RADIUS);
    if stats || run.is_some() {
        let pct = |v: &mut Vec<f64>, q: f64| {
            v.sort_by(|a, b| a.total_cmp(b));
            v[((v.len() - 1) as f64 * q) as usize]
        };
        let (mut wt, mut ft): (Vec<f64>, Vec<f64>) = frames.iter().copied().unzip();
        if !wt.is_empty() {
            eprintln!(
                "frames: {} in {:.1} s; land round the camera drawn after {} ms; world work on the main thread per frame: median {:.2} ms, 95% {:.2} ms, max {:.2} ms; whole frame: median {:.1} ms, 95% {:.1} ms, max {:.1} ms; frames over 20 ms: {}",
                frames.len(),
                started.elapsed().as_secs_f32(),
                land_at.map_or("never".into(), |d| d.as_millis().to_string()),
                pct(&mut wt, 0.5),
                pct(&mut wt, 0.95),
                pct(&mut wt, 1.0),
                pct(&mut ft, 0.5),
                pct(&mut ft, 0.95),
                pct(&mut ft, 1.0),
                ft.iter().filter(|f| **f > 20.0).count()
            );
        }
    }
    if std::env::var_os("DYNO_SHOT_VERIFY").is_some() {
        // the meshes built bit by bit, against the whole view at once
        let built = gfx.world_vertices();
        let whole = Light::new(&p.world);
        gfx.set_focus(p.player.eye());
        gfx.load_world(&p.world, &whole);
        let again = gfx.world_vertices();
        eprintln!(
            "verify: streamed meshes {built:?} vertices, the same view at once {again:?}{}",
            if built == again {
                " (the same)"
            } else {
                " (DIFFERENT)"
            }
        );
        p.light = whole;
    }
    if run.is_none() {
        p.player.pos = start_cam.into();
        p.player.yaw = yaw;
        p.player.pitch = pitch;
    }
    if stand {
        let at = p.player.pos;
        let (x, z) = (at.x.floor() as i32, at.z.floor() as i32);
        let near = (at.y >= 0.0).then_some(at.y);
        p.player.pos.y = crate::shot::standing_y(&p.world, x, z, near)
            .ok_or_else(|| format!("nowhere to stand at {x}, {z}"))?;
        // the same sky for every round of shots
        p.weather = (0, 0.0);
        p.weather_mix = 0.0;
    }
    p.captured = ui != "pack";
    p.picker_open = ui == "pack";
    p.screen_tab = match ui {
        "char" => Some(screens::Tab::Character),
        "bags" => Some(screens::Tab::Bags),
        "spells" => Some(screens::Tab::Spellbook),
        "skills" => Some(screens::Tab::Skills),
        _ => None,
    };
    if ui == "log" {
        p.panel = Some(rpg_ui::Panel::Log {
            quest: p.quest_log.first().map(|q| q.quest),
        });
    }
    if ui == "talk" {
        // the nearest NPC, reading their first quest
        let me = p.player.pos;
        if let Some((id, n)) = p
            .entities
            .iter()
            .filter_map(|(id, e)| match e.kind {
                EntityKind::Npc(n) => Some((*id, n, e.pos().distance(me))),
                _ => None,
            })
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(id, n, _)| (id, n))
        {
            let q = p.content.quests.iter().find(|q| q.giver == n).map(|q| q.id);
            p.panel = Some(rpg_ui::Panel::Dialog {
                entity: id,
                npc: n,
                quest: q,
                choice: 0,
            });
        }
    }
    if ui == "vendor" {
        // the nearest merchant's goods, with the bags open
        let me = p.player.pos;
        if let Some((id, n)) = p
            .entities
            .iter()
            .filter_map(|(id, e)| match e.kind {
                EntityKind::Npc(n) if p.content.npc(n).is_some_and(|m| !m.sells.is_empty()) => {
                    Some((*id, n, e.pos().distance(me)))
                }
                _ => None,
            })
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(id, n, _)| (id, n))
        {
            p.panel = Some(rpg_ui::Panel::Vendor { entity: id, npc: n });
            p.screen_tab = Some(screens::Tab::Bags);
            p.money = p.money.max(57);
        }
    }
    if ui == "loot" {
        // a fallen creature a few steps ahead, its loot open
        let d = p.player.look_dir();
        let d = Vec3::new(d.x, 0.0, d.z).normalize_or_zero();
        let at = p.player.pos + d * 4.0 + Vec3::new(-d.z, 0.0, d.x) * 0.8;
        let at = Vec3::new(
            at.x,
            p.world.surface(at.x as i32, at.z as i32) as f32 + 1.0,
            at.z,
        );
        let creature = p
            .content
            .creatures
            .iter()
            .find(|c| c.key == "thornboar")
            .map(|c| c.id)
            .unwrap_or(0);
        let id = 9_000_000;
        p.entities.insert(
            id,
            ClientEntity {
                kind: EntityKind::Corpse {
                    creature,
                    loot: true,
                },
                from: at.into(),
                to: at.into(),
                at: Instant::now(),
                yaw: p.player.yaw + 1.2,
                born: if std::env::var_os("DYNO_CORPSE_UPRIGHT").is_some() {
                    Instant::now()
                } else {
                    Instant::now() - Duration::from_secs(5)
                },
                hurt_at: None,
                info: None,
                name: None,
            },
        );
        let item = |k: &str| p.content.item_by_key(k);
        let items = [("boar_tusk", 2), ("torn_hide", 1), ("thornhide_boots", 1)]
            .iter()
            .filter_map(|(k, n)| item(k).map(|i| dyno_protocol::Stack::new(i, *n)))
            .collect();
        p.loot = Some((id, 23, items));
    }
    if !stand {
        p.target = game::raycast(&p.world, p.player.eye(), p.player.look_dir(), game::REACH);
    }
    if ui == "dig"
        && let Some(t) = &p.target
    {
        p.dig = Some((t.block, 0.65, 1.0));
    }
    if ui == "dead" {
        p.dead = Some("You fell from a high place.".into());
    }
    let t = time.unwrap_or(p.time);
    p.time = t;
    let sky_now = weathered(
        crate::sky::sky_at(t, Some(p.player.eye())),
        p.weather.0,
        p.weather_mix,
    );
    let load_ms = started.elapsed().as_secs_f64() * 1e3;
    let (wdt, hgt) = (gfx.config.width as f32, gfx.config.height as f32);
    let far = if p.far.is_empty() { 160.0 } else { FAR_SEE };
    let dir = p.player.look_dir();
    let eye = if p.zoom > 0.5 {
        pull_back(&p.world, p.player.eye(), dir, p.zoom)
    } else {
        p.player.eye()
    };
    let right = dir.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(dir);
    let vp = glam::Mat4::perspective_rh(crate::shot::STAND_FOV.to_radians(), wdt / hgt, 0.05, far)
        * glam::Mat4::look_to_rh(eye, dir, Vec3::Y);
    gfx.water
        .look_from(&p.world, eye.into(), p.hearth.unwrap_or_default());
    models::set_view(Some((eye, Vec3::from(sky_now.sun_dir))));
    let mut boxes = Vec::new();
    let mut lines = Vec::new();
    let light_at = |l: &Light, v: Vec3| {
        l.get(v.x.floor() as i32, v.y.floor() as i32, v.z.floor() as i32) as f32 / 15.0
    };
    for e in p.entities.values() {
        if let EntityKind::Item(s) = e.kind {
            let it = p.content.item(s.item);
            let size = if it.block.is_some() { 0.13 } else { 0.1 };
            render::push_oriented_box(
                &mut boxes,
                e.pos() + Vec3::Y * 0.2,
                [Vec3::X, Vec3::Y, Vec3::Z],
                Vec3::splat(size),
                it.color,
                false,
                light_at(&p.light, e.pos()),
            );
        }
    }
    if p.zoom > 0.5 {
        let mut look = models::Look::from_appearance(
            &p.my_look,
            p.class
                .and_then(|c| p.content.class(c))
                .map(|c| c.color)
                .unwrap_or([0.5, 0.5, 0.6]),
        );
        let keys: Vec<String> = p
            .equip
            .iter()
            .flatten()
            .map(|s| p.content.item_key(s.item))
            .collect();
        look.wear(keys.iter().map(String::as_str));
        look.weapon = models::weapon_look(&p.content, p.held().map(|s| s.item));
        if stand {
            look = crate::shot::stand_in_look(&p.content);
        }
        let at = p.player.pos + Vec3::Y;
        let l = p.light.get(
            at.x.floor() as i32,
            at.y.floor() as i32,
            at.z.floor() as i32,
        ) as f32
            / 15.0;
        models::push_person(
            &mut boxes,
            p.player.pos,
            p.player.yaw,
            p.player.pitch,
            &look,
            l,
            0.0,
            0.0,
        );
    }
    let before_creatures = boxes.len();
    let mut book = std::mem::take(&mut p.anims);
    push_creatures(&mut boxes, &p, &mut book, 1.3);
    p.anims = book;
    // ambient life, as play draws it (DYNO_LIFE=0 leaves it out)
    let life_quality = crate::life::quality(2);
    let figures = boxes.len();
    push_life(
        &mut p,
        gfx,
        &mut boxes,
        eye,
        dir,
        t,
        0.0,
        life_quality,
        true,
    );
    let (fx, fx_glow) = (gfx.fx.clone(), gfx.fx_glow.clone());
    for (id, r) in p.remotes.iter().filter(|(_, r)| !r.far) {
        let pose = r.current();
        let base = Vec3::from(pose.pos);
        models::push_avatar(
            &mut boxes,
            base,
            pose.yaw,
            pose.pitch,
            game::player_color(*id),
            light_at(&p.light, base + Vec3::Y),
            0.0,
            0.0,
        );
    }
    if p.zoom < 0.5 {
        let hand = eye + dir * 0.62 + right * 0.36 - up * 0.34;
        let (s1, c1) = 0.6f32.sin_cos();
        let ax = [right * c1 + dir * s1, up, dir * c1 - right * s1];
        models::push_held(
            &mut boxes,
            &p.content,
            p.held().map(|s| s.item),
            hand,
            ax,
            light_at(&p.light, eye),
        );
    }
    if let Some(tg) = &p.target {
        let b = Vec3::new(tg.block[0] as f32, tg.block[1] as f32, tg.block[2] as f32);
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
                    light: 1.0,
                    sky: 1.0,
                    tex: render::NO_TEX,
                });
            }
        }
    }
    let ctx = crate::theme::new_context();
    crate::theme::apply(&ctx);
    ctx.style_mut(|s| {
        s.visuals.selection.bg_fill = ACCENT.gamma_multiply(0.6);
        s.animation_time = 0.0;
    });
    let raw = || egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(wdt, hgt))),
        ..Default::default()
    };
    let mut host = None;
    let mut actions = Vec::new();
    // two passes so windows settle their size; the first pass loads the font atlas
    let first = ctx.run(raw(), |ctx| {
        if !stand {
            hud(ctx, &mut p, &mut host, Some(vp), &mut actions)
        }
    });
    gfx.fx = fx.clone();
    gfx.fx_glow = fx_glow.clone();
    gfx.frame(Some((vp, eye, far)), &sky_now, &boxes, &lines, &ctx, first);
    let second = ctx.run(raw(), |ctx| {
        if !stand {
            hud(ctx, &mut p, &mut host, Some(vp), &mut actions)
        }
    });
    gfx.screenshot = Some(out.clone());
    gfx.fx = fx;
    gfx.fx_glow = fx_glow;
    gfx.frame(Some((vp, eye, far)), &sky_now, &boxes, &lines, &ctx, second);
    // DYNO_SHOT_FRAMES=n: n more pictures a tenth of a second apart
    // (out-01.ppm ...), to see what moves
    let more: usize = std::env::var("DYNO_SHOT_FRAMES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    for i in 1..=more {
        std::thread::sleep(Duration::from_millis(100));
        // what moved meanwhile
        while let Ok(ev) = p.conn.rx.try_recv() {
            match ev {
                NetEvent::Msg(ServerMsg::EntityMoves(list)) => {
                    for (id, pos, yaw) in list {
                        if let Some(e) = p.entities.get_mut(&id) {
                            e.from = e.pos().into();
                            e.to = pos;
                            e.yaw = yaw;
                            e.at = Instant::now();
                        }
                    }
                }
                NetEvent::Msg(ServerMsg::Act {
                    actor: dyno_protocol::Actor::Entity(id),
                    act: dyno_protocol::Act::Emote(n),
                }) => {
                    if let Some(a) = entity_emote(&mut p, id, n) {
                        let now = p.anims.now();
                        p.anims.play_entity(id, now, a);
                    }
                }
                _ => {}
            }
        }
        let mut boxes = boxes[..before_creatures].to_vec();
        let mut book = std::mem::take(&mut p.anims);
        push_creatures(&mut boxes, &p, &mut book, 1.3);
        p.anims = book;
        push_life(
            &mut p,
            gfx,
            &mut boxes,
            eye,
            dir,
            t,
            0.1,
            life_quality,
            false,
        );
        let o = ctx.run(raw(), |_| {});
        let stem = out.with_extension("");
        gfx.screenshot = Some(format!("{}-{i:02}.ppm", stem.display()).into());
        gfx.frame(Some((vp, eye, far)), &sky_now, &boxes, &lines, &ctx, o);
    }
    if stats {
        // frame time on the headless path: the world, the figures and the
        // ambient life made anew each frame, GPU waited on
        gfx.wait_idle();
        let n = 30;
        let mut life_ms = 0.0;
        let t0 = Instant::now();
        for _ in 0..n {
            let o = ctx.run(raw(), |_| {});
            let mut boxes = boxes[..figures].to_vec();
            let t1 = Instant::now();
            push_life(
                &mut p,
                gfx,
                &mut boxes,
                eye,
                dir,
                t,
                0.016,
                life_quality,
                false,
            );
            life_ms += t1.elapsed().as_secs_f64() * 1e3;
            gfx.frame(Some((vp, eye, far)), &sky_now, &boxes, &lines, &ctx, o);
            gfx.wait_idle();
        }
        let frame_ms = t0.elapsed().as_secs_f64() * 1e3 / n as f64;
        let (vo, vt) = gfx.world_vertices();
        let c = p.life.counts;
        eprintln!(
            "stats: world complete after {load_ms:.0} ms, frame {frame_ms:.2} ms, world vertices {vo} opaque + {vt} translucent; life (quality {life_quality}): {:.3} ms a frame on the CPU (effects {:.3}, critters {:.3}), {} emitters, {} particles, {} critters",
            life_ms / n as f64,
            c.particles_ms,
            c.critters_ms,
            c.emitters,
            c.particles,
            c.critters
        );
        let (gv, levels) = gfx.ground_stats();
        eprintln!("ground: {gv} vertices; sections by voxels a metre {levels:?}");
        let (sections, ms) = crate::render::mesh_cost();
        eprintln!(
            "meshing: {sections} sections in {ms:.0} ms on the world thread's cores ({:.2} ms a section); vertex memory {:.0} MB",
            ms / sections.max(1) as f64,
            (vo + vt) as f64 * std::mem::size_of::<crate::render::Vertex>() as f64 / 1048576.0
        );
    }
    let mut creatures: Vec<String> = p
        .entities
        .values()
        .filter_map(|e| match e.kind {
            EntityKind::Creature(c) => p.content.creature(c).map(|c| {
                let v = e.pos() - p.player.pos;
                let d = v.length();
                let bearing = (-v.x).atan2(-v.z).to_degrees().rem_euclid(360.0);
                format!("{} {d:.0}m at {bearing:.0}°", c.name)
            }),
            _ => None,
        })
        .collect();
    creatures.sort();
    Ok(format!(
        "rendered {ui} at {:?}, {} items in pack, health {}, creatures [{}], to {}",
        p.player.pos,
        p.pack.iter().flatten().count(),
        p.health,
        creatures.join(", "),
        out.display()
    ))
}
