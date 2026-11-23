//! The front end a player meets before the world: title, realm list,
//! character select and character creation (class, appearance, name).
//! The 3D scene behind these screens (Hearthstead at dusk, the character on
//! the dais) is drawn by the app; this module draws the interface and keeps
//! the state. Design: the "New-player flow" mockups.

use crate::models::Look;
use crate::theme;
use dyno_protocol::{Appearance, CharacterSummary, ClassInfo, NewCharacter, RealmInfo};
use egui::{Align2, Color32, Pos2, Rect, RichText, Vec2};
use std::sync::mpsc::Receiver;

type RealmAnswer = Result<(RealmInfo, Vec<CharacterSummary>, u32), String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Title,
    Realms,
    Select,
    Class,
    Appearance,
    Name,
}

/// A realm on the list: its address and what it last told us.
pub struct RealmRow {
    pub addr: String,
    pub info: Option<RealmInfo>,
    pub chars: Vec<CharacterSummary>,
    pub ping: Option<u32>,
    pub error: Option<String>,
    pending: Option<Receiver<RealmAnswer>>,
}

pub struct Front {
    pub page: Page,
    /// a local world is running (Options can stop it)
    pub hosting: bool,
    pub realms: Vec<RealmRow>,
    pub realm: usize,
    pub add_addr: String,
    pub chosen: usize,
    /// creation
    pub class: usize,
    pub look: Appearance,
    pub name: String,
    /// turntable angle (radians)
    pub turn: f32,
    pub delete_confirm: Option<String>,
    pub error: Option<String>,
}

/// What the player asked for.
pub enum FrontCmd {
    /// Ask a realm (index) for its status and my characters.
    Query(usize),
    /// Delete a character on the current realm.
    Delete(usize, String),
    /// Join the current realm as this character (creating it with `create`).
    Enter {
        addr: String,
        name: String,
        create: Option<NewCharacter>,
    },
    Host,
    StopHost,
    /// remember this world address (Options)
    SetWorld(String),
    /// the graphics options changed (options.rs)
    Graphics,
    Quit,
}

/// The world's address, set when the game is built for release. Players
/// never choose a server: there is one world, sharded behind this address.
pub const DEFAULT_SERVER: &str = match option_env!("DYNO_DEFAULT_SERVER") {
    Some(s) => s,
    None => "",
};

impl Front {
    pub fn new(recent: &[String], last_char: &str) -> Front {
        // the world: an address override from Options, else the built-in one
        let world: Vec<String> = recent
            .first()
            .cloned()
            .or_else(|| (!DEFAULT_SERVER.is_empty()).then(|| DEFAULT_SERVER.to_string()))
            .into_iter()
            .collect();
        let realms = world
            .iter()
            .take(1)
            .map(|a| RealmRow {
                addr: a.clone(),
                info: None,
                chars: Vec::new(),
                ping: None,
                error: None,
                pending: None,
            })
            .collect();
        let _ = last_char;
        Front {
            page: Page::Title,
            hosting: false,
            realms,
            realm: 0,
            add_addr: String::new(),
            chosen: 0,
            class: 0,
            look: Appearance {
                hair_style: 1,
                hair_color: 3,
                skin: 1,
                eyes: 0,
                beard: false,
                build: 1,
                mood: 0,
                ears: 1,
            },
            name: String::new(),
            turn: 0.0,
            delete_confirm: None,
            error: None,
        }
    }

    pub fn current(&self) -> Option<&RealmRow> {
        self.realms.get(self.realm)
    }

    /// Start (or restart) a background query of a realm.
    pub fn query(&mut self, i: usize, token: u64, delete: Option<String>) {
        let Some(r) = self.realms.get_mut(i) else {
            return;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let addr = r.addr.clone();
        std::thread::spawn(move || {
            let _ = tx.send(crate::net::realm(
                &addr,
                token,
                delete.as_deref(),
                std::time::Duration::from_secs(6),
            ));
        });
        r.pending = Some(rx);
    }

    /// Collect finished queries.
    pub fn poll(&mut self) {
        for r in &mut self.realms {
            let Some(rx) = &r.pending else { continue };
            if let Ok(ans) = rx.try_recv() {
                r.pending = None;
                match ans {
                    Ok((info, chars, ping)) => {
                        r.info = Some(info);
                        r.chars = chars;
                        r.ping = Some(ping);
                        r.error = None;
                    }
                    Err(e) => r.error = Some(e),
                }
            }
        }
    }

    pub fn add_realm(&mut self, addr: &str) -> usize {
        let addr = addr.trim().to_string();
        if let Some(i) = self.realms.iter().position(|r| r.addr == addr) {
            return i;
        }
        self.realms.insert(
            0,
            RealmRow {
                addr,
                info: None,
                chars: Vec::new(),
                ping: None,
                error: None,
                pending: None,
            },
        );
        0
    }

    /// The class being chosen in creation.
    pub fn class_info(&self) -> Option<&ClassInfo> {
        self.current()?.info.as_ref()?.classes.get(self.class)
    }

    /// The character to show in 3D: in creation, the one being made; in
    /// character select, the chosen one. Returns its look.
    pub fn preview(&self) -> Option<Look> {
        match self.page {
            Page::Class | Page::Appearance | Page::Name => {
                let color = self
                    .class_info()
                    .map(|c| c.color)
                    .unwrap_or([0.5, 0.5, 0.6]);
                Some(Look::from_appearance(&self.look, color))
            }
            Page::Select => {
                let c = self.current()?.chars.get(self.chosen)?;
                Some(Look::from_appearance(&c.look, c.color))
            }
            _ => None,
        }
    }

    /// How far right (in clip space, -1 to 1) to draw the 3D preview, so the
    /// character stands in the space the panels leave free.
    pub fn preview_offset(&self) -> f32 {
        match self.page {
            Page::Class => 0.27,
            Page::Appearance | Page::Name => 0.34,
            _ => 0.0,
        }
    }

    pub fn randomise(&mut self, seed: u32) {
        let mut h = seed.wrapping_mul(2_654_435_761) | 1;
        let mut next = |n: u32| {
            h ^= h << 13;
            h ^= h >> 17;
            h ^= h << 5;
            (h % n) as u8
        };
        self.look = Appearance {
            skin: next(8),
            eyes: next(6),
            hair_style: next(5),
            hair_color: next(10),
            beard: next(3) == 0,
            build: next(6),
            mood: next(3),
            ears: next(3),
        };
    }
}

/// A fantasy name from syllables.
pub fn random_name(seed: u32) -> String {
    const A: [&str; 16] = [
        "Ael", "Bran", "Cor", "Dra", "El", "Fen", "Gar", "Hal", "Isa", "Jor", "Kel", "Lyr", "Mor",
        "Ny", "Ro", "Sel",
    ];
    const B: [&str; 12] = [
        "wen", "dric", "ion", "ra", "eth", "mir", "nor", "is", "ald", "ena", "ric", "wyn",
    ];
    let h = seed.wrapping_mul(0x9E37_79B9);
    format!("{}{}", A[(h % 16) as usize], B[((h >> 8) % 12) as usize])
}

/// Why a name cannot be used, if it cannot.
pub fn name_problem(n: &str) -> Option<&'static str> {
    let n = n.trim();
    if n.chars().count() < 3 {
        Some("At least 3 letters")
    } else if n.chars().count() > 16 {
        Some("At most 16 letters")
    } else if !n.chars().all(|c| c.is_alphabetic()) {
        Some("Letters only")
    } else {
        None
    }
}

fn gold_button(ui: &mut egui::Ui, text: &str, w: f32, h: f32) -> egui::Response {
    let b = egui::Button::new(
        RichText::new(text)
            .font(theme::title_font(h * 0.42))
            .color(theme::INK),
    )
    .fill(theme::GOLD)
    .stroke(egui::Stroke::new(2.0_f32, theme::GOLD_BRIGHT))
    .min_size(Vec2::new(w, h));
    ui.add(b)
}

fn dark_button(ui: &mut egui::Ui, text: &str, w: f32) -> egui::Response {
    let b = egui::Button::new(
        RichText::new(text)
            .font(theme::title_font(14.0))
            .color(theme::GOLD),
    )
    .fill(theme::WOOD_DARK)
    .stroke(egui::Stroke::new(1.0_f32, theme::GOLD_DARK))
    .min_size(Vec2::new(w, 36.0));
    ui.add(b)
}

fn panel<R>(ctx: &egui::Context, id: &str, rect: Rect, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::new((id, "bg")),
    ));
    theme::panel_frame(&painter, rect);
    egui::Area::new(egui::Id::new(id))
        .fixed_pos(rect.min + Vec2::splat(16.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.set_width(rect.width() - 32.0);
            ui.set_max_height(rect.height() - 32.0);
            add(ui)
        })
        .inner
}

/// Draw the front end. `last` is the last character played ("name@realm" index 0).
pub fn ui(
    ctx: &egui::Context,
    f: &mut Front,
    version: &str,
    update_note: &str,
    hosting: bool,
    out: &mut Vec<FrontCmd>,
) {
    f.hosting = hosting;
    f.poll();
    let screen = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Background,
        egui::Id::new("front"),
    ));
    // a soft vignette so text reads over the 3D scene
    painter.rect_filled(
        Rect::from_min_max(
            screen.min,
            Pos2::new(screen.left() + 420.0, screen.bottom()),
        ),
        0.0,
        Color32::from_black_alpha(70),
    );
    match f.page {
        Page::Title => title(ctx, f, version, update_note, hosting, out),
        Page::Realms => realms(ctx, f, out),
        Page::Select => select(ctx, f, out),
        Page::Class | Page::Appearance | Page::Name => create(ctx, f, out),
    }
}

fn title(
    ctx: &egui::Context,
    f: &mut Front,
    version: &str,
    update_note: &str,
    _hosting: bool,
    out: &mut Vec<FrontCmd>,
) {
    let screen = ctx.content_rect();
    let p = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::new("title"),
    ));
    let x = screen.left() + 240.0;
    theme::title_text(
        &p,
        Pos2::new(x, screen.top() + 120.0),
        "HEARTHBLOCK",
        46.0,
        theme::GOLD_BRIGHT,
    );
    theme::subtitle_text(
        &p,
        Pos2::new(x, screen.top() + 168.0),
        "O N L I N E",
        18.0,
        theme::GOLD,
    );
    theme::divider(
        &p,
        Pos2::new(x, screen.top() + 190.0),
        260.0,
        theme::GOLD_DARK,
    );
    let last = f.realms.first().and_then(|r| {
        r.chars
            .first()
            .map(|c| (c.name.clone(), r.info.as_ref().map(|i| i.name.clone())))
    });
    egui::Area::new(egui::Id::new("title-menu"))
        .fixed_pos(Pos2::new(x - 130.0, screen.top() + 230.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.set_width(260.0);
            ui.vertical_centered(|ui| {
                if gold_button(ui, "Play", 260.0, 58.0).clicked() {
                    // straight to your characters; Options only if no world is set
                    if f.realms.is_empty() {
                        f.page = Page::Realms;
                    } else {
                        f.realm = 0;
                        out.push(FrontCmd::Query(0));
                        f.page = Page::Select;
                    }
                }
                let caption = match &last {
                    Some((n, _)) => format!("Continue as {n}"),
                    None => "Begin your adventure".to_string(),
                };
                ui.label(
                    RichText::new(caption)
                        .font(theme::body_font(14.0))
                        .color(theme::TEXT_DIM),
                );
                ui.add_space(18.0);
                if dark_button(ui, "Options", 220.0).clicked() {
                    f.add_addr = f.realms.first().map(|r| r.addr.clone()).unwrap_or_default();
                    f.page = Page::Realms;
                }
                ui.add_space(6.0);
                if dark_button(ui, "Quit", 220.0).clicked() {
                    out.push(FrontCmd::Quit);
                }
            });
        });
    theme::shadow_text(
        &p,
        Pos2::new(screen.left() + 16.0, screen.bottom() - 14.0),
        Align2::LEFT_BOTTOM,
        &format!("Version {version} · {update_note}"),
        theme::body_font(13.0),
        theme::TEXT_DIM,
    );
    // news, from the pack of the last world joined (none before the first join)
    let news = crate::app::pack_lines("news", "lines");
    let r = Rect::from_min_size(
        Pos2::new(screen.right() - 360.0, screen.bottom() - 200.0),
        Vec2::new(330.0, 170.0),
    );
    if !news.is_empty() {
        panel(ctx, "news", r, |ui| {
            ui.label(
                RichText::new(crate::app::pack_text("news", "title"))
                    .font(theme::title_font(18.0))
                    .color(theme::GOLD),
            );
            ui.label(
                RichText::new("What's new")
                    .font(theme::body_font(12.0))
                    .color(theme::TEXT_DIM),
            );
            ui.add_space(4.0);
            for l in &news {
                ui.label(
                    RichText::new(format!("• {l}"))
                        .font(theme::body_font(14.0))
                        .color(theme::TEXT),
                );
            }
        });
    }
    let _ = f;
}

/// Options: the world's address (for development and private test worlds)
/// and hosting a local world. Players normally never come here.
fn realms(ctx: &egui::Context, f: &mut Front, out: &mut Vec<FrontCmd>) {
    let screen = ctx.content_rect();
    let r = Rect::from_center_size(screen.center(), Vec2::new(620.0, 560.0));
    let hosting = f.hosting;
    panel(ctx, "options", r, |ui| {
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new("Options")
                    .font(theme::title_font(26.0))
                    .color(theme::GOLD_BRIGHT),
            );
        });
        theme::ui_divider(ui, theme::GOLD_DARK);
        ui.label(
            RichText::new("World address")
                .font(theme::title_font(14.0))
                .color(theme::GOLD),
        );
        ui.label(
            RichText::new(if DEFAULT_SERVER.is_empty() {
                "Paste the address of the world to play in.".to_string()
            } else {
                format!("Leave empty for the world ({DEFAULT_SERVER}).")
            })
            .font(theme::body_font(13.0))
            .color(theme::TEXT_DIM),
        );
        ui.add(
            egui::TextEdit::singleline(&mut f.add_addr)
                .hint_text("https://… or 192.168.1.20")
                .desired_width(560.0),
        );
        if let Some(row) = f.realms.first() {
            let status = match (&row.info, &row.error) {
                (Some(i), _) => format!("{} · {} online · version {}", i.name, i.online, i.version),
                (None, Some(e)) => format!(
                    "Cannot reach it: {}",
                    e.chars().take(70).collect::<String>()
                ),
                _ => "Checking…".to_string(),
            };
            ui.label(
                RichText::new(status)
                    .font(theme::body_font(13.0))
                    .color(theme::TEXT),
            );
        }
        ui.add_space(8.0);
        if crate::options::panel(ui) {
            out.push(FrontCmd::Graphics);
        }
        // hosting a local world is for the dev build (`dyno`), not the player client
        if cfg!(feature = "host") {
            ui.add_space(12.0);
            ui.label(
                RichText::new("Developers: run a world on this computer")
                    .font(theme::title_font(14.0))
                    .color(theme::GOLD),
            );
        }
        ui.horizontal(|ui| {
            if !cfg!(feature = "host") {
                return;
            }
            if dark_button(
                ui,
                if hosting {
                    "Play in my world"
                } else {
                    "Host a local world"
                },
                200.0,
            )
            .clicked()
            {
                out.push(FrontCmd::Host);
            }
            if hosting && dark_button(ui, "Stop hosting", 140.0).clicked() {
                out.push(FrontCmd::StopHost);
            }
        });
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            if dark_button(ui, "Back", 110.0).clicked() {
                f.page = Page::Title;
            }
            ui.add_space(ui.available_width() - 160.0);
            if gold_button(ui, "Save", 150.0, 40.0).clicked() {
                let addr = f.add_addr.trim().to_string();
                let addr = if addr.is_empty() {
                    DEFAULT_SERVER.to_string()
                } else {
                    addr
                };
                f.realms.clear();
                if !addr.is_empty() {
                    f.add_realm(&addr);
                    f.realm = 0;
                    out.push(FrontCmd::Query(0));
                    out.push(FrontCmd::SetWorld(addr));
                }
                f.page = Page::Title;
            }
        });
    });
}

fn select(ctx: &egui::Context, f: &mut Front, out: &mut Vec<FrontCmd>) {
    let screen = ctx.content_rect();
    let p = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::new("select"),
    ));
    let Some(row) = f.realms.get(f.realm) else {
        f.page = Page::Realms;
        return;
    };
    let realm_name = row
        .info
        .as_ref()
        .map(|i| i.name.clone())
        .unwrap_or_else(|| row.addr.clone());
    theme::shadow_text(
        &p,
        screen.left_top() + Vec2::new(18.0, 16.0),
        Align2::LEFT_TOP,
        &format!(
            "{realm_name} · {} online{}",
            row.info.as_ref().map(|i| i.online).unwrap_or(0),
            row.ping.map(|v| format!(" · {v} ms")).unwrap_or_default()
        ),
        theme::body_font(15.0),
        theme::TEXT,
    );
    let chars = row.chars.clone();
    let loading = row.pending.is_some() && row.info.is_none();
    let error = row.error.clone();
    let addr = row.addr.clone();
    let has_info = row.info.is_some();
    if f.chosen >= chars.len() {
        f.chosen = 0;
    }
    // the chosen character's name over the dais
    if let Some(c) = chars.get(f.chosen) {
        theme::title_text(
            &p,
            Pos2::new(screen.center().x, screen.top() + 70.0),
            &c.name,
            40.0,
            theme::GOLD_BRIGHT,
        );
        theme::subtitle_text(
            &p,
            Pos2::new(screen.center().x, screen.top() + 108.0),
            &format!("Level {} {} · {}", c.level, c.class, c.zone),
            17.0,
            theme::TEXT,
        );
    }
    // character list
    let r = Rect::from_min_size(
        Pos2::new(screen.right() - 330.0, screen.top() + 40.0),
        Vec2::new(310.0, screen.height() - 80.0),
    );
    panel(ctx, "chars", r, |ui| {
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new("Characters")
                    .font(theme::title_font(20.0))
                    .color(theme::GOLD),
            );
        });
        theme::ui_divider(ui, theme::GOLD_DARK);
        if loading {
            ui.spinner();
        }
        if let Some(e) = &error {
            ui.label(RichText::new(e).color(Color32::from_rgb(230, 120, 110)));
        }
        for (i, c) in chars.iter().enumerate() {
            let col = Color32::from_rgb(
                (c.color[0] * 255.0) as u8,
                (c.color[1] * 255.0) as u8,
                (c.color[2] * 255.0) as u8,
            );
            let resp = ui.selectable_label(
                i == f.chosen,
                RichText::new(format!(
                    "{}\nLevel {} {} · {}",
                    c.name, c.level, c.class, c.zone
                ))
                .font(theme::body_font(15.0))
                .color(col),
            );
            if resp.clicked() {
                f.chosen = i;
                f.delete_confirm = None;
            }
            if resp.double_clicked() {
                out.push(FrontCmd::Enter {
                    addr: addr.clone(),
                    name: c.name.clone(),
                    create: None,
                });
            }
            ui.add_space(4.0);
        }
        for _ in chars.len()..4 {
            ui.label(
                RichText::new("+  Empty slot")
                    .font(theme::body_font(14.0))
                    .color(theme::TEXT_DIM),
            );
            ui.add_space(8.0);
        }
        ui.add_space(8.0);
        if dark_button(ui, "Create New Character", ui.available_width()).clicked() && has_info {
            f.page = Page::Class;
            f.name.clear();
            f.randomise(chars.len() as u32 * 7919 + 17);
        }
        if let Some(c) = chars.get(f.chosen) {
            ui.add_space(6.0);
            match &mut f.delete_confirm {
                None => {
                    if ui
                        .button(RichText::new("Delete").color(Color32::from_rgb(220, 100, 90)))
                        .clicked()
                    {
                        f.delete_confirm = Some(String::new());
                    }
                }
                Some(typed) => {
                    ui.label(
                        RichText::new(format!("Type {} to delete for ever", c.name))
                            .small()
                            .color(Color32::from_rgb(220, 100, 90)),
                    );
                    ui.text_edit_singleline(typed);
                    if typed.eq_ignore_ascii_case(&c.name) && ui.button("Delete for ever").clicked()
                    {
                        out.push(FrontCmd::Delete(f.realm, c.name.clone()));
                        f.delete_confirm = None;
                    }
                }
            }
        }
    });
    egui::Area::new(egui::Id::new("select-bottom"))
        .anchor(Align2::CENTER_BOTTOM, Vec2::new(-170.0, -30.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            if let Some(c) = chars.get(f.chosen)
                && gold_button(ui, "Enter World", 280.0, 56.0).clicked()
            {
                out.push(FrontCmd::Enter {
                    addr: addr.clone(),
                    name: c.name.clone(),
                    create: None,
                });
            }
            if chars.is_empty()
                && !loading
                && has_info
                && gold_button(ui, "Create Your First Character", 340.0, 56.0).clicked()
            {
                f.page = Page::Class;
                f.randomise(17);
            }
        });
    egui::Area::new(egui::Id::new("select-back"))
        .anchor(Align2::LEFT_BOTTOM, Vec2::new(20.0, -24.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            if dark_button(ui, "Back", 110.0).clicked() {
                f.page = Page::Title;
            }
        });
    // drag to turn the character
    let drag = ctx.input(|i| {
        if i.pointer.primary_down() && !ctx.is_pointer_over_area() {
            i.pointer.delta().x
        } else {
            0.0
        }
    });
    f.turn += drag * 0.01;
}

fn stepper(ui: &mut egui::Ui, label: &str, value: &mut u8, n: u8, names: Option<&[&str]>) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(label)
                .font(theme::title_font(13.0))
                .color(theme::GOLD),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(">").clicked() {
                *value = (*value + 1) % n;
            }
            let shown = match names {
                Some(v) => v[*value as usize % v.len()].to_string(),
                None => format!("{} of {n}", *value + 1),
            };
            ui.label(
                RichText::new(shown)
                    .font(theme::body_font(15.0))
                    .color(theme::TEXT),
            );
            if ui.button("<").clicked() {
                *value = (*value + n - 1) % n;
            }
        });
    });
}

fn swatches(ui: &mut egui::Ui, label: &str, value: &mut u8, colors: &[[f32; 3]]) {
    ui.label(
        RichText::new(label)
            .font(theme::title_font(13.0))
            .color(theme::GOLD),
    );
    ui.horizontal_wrapped(|ui| {
        for (i, c) in colors.iter().enumerate() {
            let (r, resp) = ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::click());
            let col = Color32::from_rgb(
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
            );
            ui.painter().rect_filled(r, 4.0, col);
            let on = *value as usize == i;
            ui.painter().rect_stroke(
                r,
                4.0,
                egui::Stroke::new(
                    if on { 2.5_f32 } else { 1.0 },
                    if on {
                        theme::GOLD_BRIGHT
                    } else {
                        theme::OUTLINE
                    },
                ),
                egui::StrokeKind::Outside,
            );
            if resp.clicked() {
                *value = i as u8;
            }
        }
    });
    ui.add_space(4.0);
}

fn create(ctx: &egui::Context, f: &mut Front, out: &mut Vec<FrontCmd>) {
    let screen = ctx.content_rect();
    let p = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::new("create"),
    ));
    theme::shadow_text(
        &p,
        screen.left_top() + Vec2::new(20.0, 16.0),
        Align2::LEFT_TOP,
        "Create a Character",
        theme::title_font(26.0),
        theme::GOLD_BRIGHT,
    );
    // steps
    let steps = [
        (Page::Class, "1 · Class"),
        (Page::Appearance, "2 · Appearance"),
        (Page::Name, "3 · Name"),
    ];
    for (k, (pg, label)) in steps.iter().enumerate() {
        let col = if *pg == f.page {
            theme::GOLD_BRIGHT
        } else {
            theme::TEXT_DIM
        };
        theme::shadow_text(
            &p,
            Pos2::new(
                screen.center().x - 200.0 + k as f32 * 200.0,
                screen.top() + 24.0,
            ),
            Align2::CENTER_TOP,
            label,
            theme::title_font(15.0),
            col,
        );
    }
    let classes: Vec<ClassInfo> = f
        .current()
        .and_then(|r| r.info.as_ref())
        .map(|i| i.classes.clone())
        .unwrap_or_default();
    let left = Rect::from_min_size(
        Pos2::new(screen.left() + 20.0, screen.top() + 70.0),
        Vec2::new(420.0, screen.height() - 150.0),
    );
    match f.page {
        Page::Class => {
            class_cards(ctx, f, &classes, screen);
        }
        Page::Appearance => {
            panel(ctx, "looks", left, |ui| {
                ui.label(
                    RichText::new("Appearance")
                        .font(theme::title_font(18.0))
                        .color(theme::GOLD),
                );
                ui.label(
                    RichText::new("Drag the character to turn them.")
                        .font(theme::body_font(13.0))
                        .color(theme::TEXT_DIM),
                );
                theme::ui_divider(ui, theme::GOLD_DARK);
                swatches(ui, "Skin", &mut f.look.skin, &Look::options("skin"));
                swatches(ui, "Eyes", &mut f.look.eyes, &Look::options("eyes"));
                hair_thumbs(ui, &mut f.look);
                swatches(
                    ui,
                    "Hair colour",
                    &mut f.look.hair_color,
                    &Look::options("hair"),
                );
                // the pack says which features the creator offers, and
                // how they pack into the appearance's fields
                for st in Look::steppers() {
                    let mut beard = f.look.beard as u8;
                    let raw = match st.field.as_str() {
                        "beard" => &mut beard,
                        "build" => &mut f.look.build,
                        "mood" => &mut f.look.mood,
                        "ears" => &mut f.look.ears,
                        "hair_style" => &mut f.look.hair_style,
                        _ => continue,
                    };
                    let mut v = st.get(*raw);
                    let names: Vec<&str> = st.names.iter().map(String::as_str).collect();
                    stepper(ui, &st.label, &mut v, st.count(), Some(&names));
                    *raw = st.put(*raw, v);
                    if st.field == "beard" {
                        f.look.beard = beard == 1;
                    }
                }
                ui.add_space(8.0);
                if dark_button(ui, "Randomise", 160.0).clicked() {
                    let seed = (ctx.input(|i| i.time) * 1000.0) as u32;
                    f.randomise(seed);
                }
            });
        }
        Page::Name => {
            panel(ctx, "naming", left, |ui| {
                ui.label(
                    RichText::new("Name your character")
                        .font(theme::title_font(18.0))
                        .color(theme::GOLD),
                );
                theme::ui_divider(ui, theme::GOLD_DARK);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut f.name)
                            .font(theme::title_font(20.0))
                            .desired_width(280.0)
                            .char_limit(16),
                    );
                    if ui
                        .button(RichText::new("?").size(20.0))
                        .on_hover_text("A random name")
                        .clicked()
                    {
                        let seed = (ctx.input(|i| i.time) * 977.0) as u32;
                        f.name = random_name(seed);
                    }
                });
                match name_problem(&f.name) {
                    Some(why) => {
                        ui.label(RichText::new(why).color(Color32::from_rgb(230, 150, 90)))
                    }
                    None => ui.label(RichText::new("A fine name").color(theme::GOOD)),
                };
                ui.add_space(10.0);
                if let Some(c) = classes.get(f.class) {
                    ui.label(
                        RichText::new(format!("{} · {} armour · {}", c.name, c.armor, c.resource))
                            .font(theme::body_font(15.0))
                            .color(theme::TEXT),
                    );
                }
                if let Some(e) = &f.error {
                    ui.label(RichText::new(e).color(Color32::from_rgb(230, 120, 110)));
                }
            });
        }
        _ => {}
    }
    // back and next
    egui::Area::new(egui::Id::new("create-nav"))
        .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-24.0, -24.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if dark_button(ui, "Back", 110.0).clicked() {
                    f.page = match f.page {
                        Page::Class => Page::Select,
                        Page::Appearance => Page::Class,
                        _ => Page::Appearance,
                    };
                }
                ui.add_space(10.0);
                let (label, ok) = match f.page {
                    Page::Class => ("Next: Appearance", !classes.is_empty()),
                    Page::Appearance => ("Next: Name", true),
                    _ => ("Create Character", name_problem(&f.name).is_none()),
                };
                ui.add_enabled_ui(ok, |ui| {
                    if gold_button(ui, label, 240.0, 50.0).clicked() {
                        match f.page {
                            Page::Class => f.page = Page::Appearance,
                            Page::Appearance => {
                                if f.name.is_empty() {
                                    f.name = random_name((ctx.input(|i| i.time) * 313.0) as u32);
                                }
                                f.page = Page::Name;
                            }
                            _ => {
                                if let (Some(row), Some(c)) = (f.current(), classes.get(f.class)) {
                                    out.push(FrontCmd::Enter {
                                        addr: row.addr.clone(),
                                        name: f.name.trim().to_string(),
                                        create: Some(NewCharacter {
                                            class: c.key.clone(),
                                            look: f.look,
                                        }),
                                    });
                                }
                            }
                        }
                    }
                });
            });
        });
    let drag = ctx.input(|i| {
        if i.pointer.primary_down() && !ctx.is_pointer_over_area() {
            i.pointer.delta().x
        } else {
            0.0
        }
    });
    f.turn += drag * 0.01;
}

fn col32(c: [f32; 3]) -> Color32 {
    Color32::from_rgb(
        (c[0].clamp(0.0, 1.0) * 255.0) as u8,
        (c[1].clamp(0.0, 1.0) * 255.0) as u8,
        (c[2].clamp(0.0, 1.0) * 255.0) as u8,
    )
}

/// Roles and how hard a class is to play well (1 to 3), by class key.
fn class_meta(key: &str) -> (&'static [&'static str], u8) {
    match key {
        "warrior" => (&["Tank", "Damage"], 1),
        "paladin" => (&["Healer", "Damage"], 2),
        "hunter" => (&["Damage"], 1),
        "rogue" => (&["Damage"], 3),
        "priest" => (&["Healer"], 2),
        "mage" => (&["Damage"], 2),
        _ => (&["Damage"], 2),
    }
}

fn difficulty_text(d: u8) -> &'static str {
    match d {
        1 => "Easy",
        2 => "Moderate",
        _ => "Hard",
    }
}

/// Small diamonds: filled up to `d` of 3.
fn diamonds(p: &egui::Painter, right_center: Pos2, d: u8) {
    for k in 0..3 {
        let c = right_center + Vec2::new(-(2 - k) as f32 * 11.0, 0.0);
        let pts = vec![
            c + Vec2::new(0.0, -4.5),
            c + Vec2::new(4.5, 0.0),
            c + Vec2::new(0.0, 4.5),
            c + Vec2::new(-4.5, 0.0),
        ];
        let fill = if (k as u8) < d {
            theme::GOLD_BRIGHT
        } else {
            Color32::TRANSPARENT
        };
        p.add(egui::Shape::convex_polygon(
            pts,
            fill,
            egui::Stroke::new(1.0_f32, theme::GOLD_DARK),
        ));
    }
}

/// Step 1: the six classes as cards, and the chosen one in detail.
fn class_cards(ctx: &egui::Context, f: &mut Front, classes: &[ClassInfo], screen: Rect) {
    let top = screen.top() + 64.0;
    let bottom = screen.bottom() - 90.0;
    let rows = classes.len().div_ceil(2).max(1);
    let gap = 12.0;
    let cw = ((screen.width() * 0.5 - 40.0) / 2.0).clamp(220.0, 320.0);
    let ch = ((bottom - top - gap * (rows as f32 - 1.0)) / rows as f32).min(170.0);
    egui::Area::new(egui::Id::new("class-cards"))
        .fixed_pos(Pos2::new(screen.left() + 20.0, top))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            for (i, c) in classes.iter().enumerate() {
                let (row, colm) = (i / 2, i % 2);
                let r = Rect::from_min_size(
                    Pos2::new(
                        screen.left() + 20.0 + colm as f32 * (cw + gap),
                        top + row as f32 * (ch + gap),
                    ),
                    Vec2::new(cw, ch),
                );
                let resp = ui.interact(r, egui::Id::new(("class-card", i)), egui::Sense::click());
                if resp.clicked() {
                    f.class = i;
                }
                let sel = i == f.class;
                let p = ui.painter();
                let tint = col32(c.color);
                p.rect_filled(r, 6.0, Color32::from_rgba_unmultiplied(28, 20, 14, 235));
                if resp.hovered() && !sel {
                    p.rect_filled(r, 6.0, Color32::from_white_alpha(10));
                }
                p.rect_stroke(
                    r,
                    6.0,
                    egui::Stroke::new(
                        if sel { 2.5_f32 } else { 1.0 },
                        if sel {
                            theme::GOLD_BRIGHT
                        } else {
                            theme::GOLD_DARK
                        },
                    ),
                    egui::StrokeKind::Inside,
                );
                // crest: a shield in the class colour with its initial
                let crest =
                    Rect::from_min_size(r.min + Vec2::new(12.0, 12.0), Vec2::new(30.0, 34.0));
                let sh = vec![
                    crest.left_top(),
                    crest.right_top(),
                    Pos2::new(crest.right(), crest.top() + 20.0),
                    Pos2::new(crest.center().x, crest.bottom()),
                    Pos2::new(crest.left(), crest.top() + 20.0),
                ];
                p.add(egui::Shape::convex_polygon(
                    sh,
                    tint.gamma_multiply(0.85),
                    egui::Stroke::new(1.5_f32, theme::GOLD),
                ));
                p.text(
                    crest.center() - Vec2::new(0.0, 3.0),
                    Align2::CENTER_CENTER,
                    c.name.chars().next().unwrap_or('?').to_string(),
                    theme::title_font(16.0),
                    Color32::from_rgb(250, 240, 220),
                );
                p.text(
                    Pos2::new(crest.right() + 10.0, r.top() + 14.0),
                    Align2::LEFT_TOP,
                    &c.name,
                    theme::title_font(19.0),
                    tint,
                );
                let (roles, diff) = class_meta(&c.key);
                p.text(
                    Pos2::new(r.right() - 12.0, r.top() + 12.0),
                    Align2::RIGHT_TOP,
                    "DIFFICULTY",
                    theme::body_font(9.0),
                    theme::TEXT_DIM,
                );
                diamonds(p, Pos2::new(r.right() - 16.0, r.top() + 30.0), diff);
                p.text(
                    Pos2::new(crest.right() + 10.0, r.top() + 38.0),
                    Align2::LEFT_TOP,
                    format!("{} · {} · {}", roles.join(" / "), c.armor, c.resource),
                    theme::body_font(12.5),
                    theme::TEXT_DIM,
                );
                // the blurb, wrapped
                let galley = p.layout(
                    c.blurb.clone(),
                    theme::body_font(13.5),
                    theme::TEXT,
                    cw - 24.0,
                );
                let blurb_h = galley.size().y;
                p.galley(
                    Pos2::new(r.left() + 12.0, r.top() + 58.0),
                    galley,
                    theme::TEXT,
                );
                // signature abilities
                let ay = (r.top() + 64.0 + blurb_h).min(r.bottom() - 26.0);
                let mut x = r.left() + 12.0;
                for a in c.abilities.iter().take(3) {
                    let g = p.layout_no_wrap(a.clone(), theme::body_font(11.5), theme::GOLD);
                    let w = g.size().x + 14.0;
                    if x + w > r.right() - 8.0 {
                        break;
                    }
                    let pill = Rect::from_min_size(Pos2::new(x, ay), Vec2::new(w, 18.0));
                    p.rect_filled(pill, 4.0, Color32::from_black_alpha(110));
                    p.rect_stroke(
                        pill,
                        4.0,
                        egui::Stroke::new(1.0_f32, theme::GOLD_DARK),
                        egui::StrokeKind::Inside,
                    );
                    p.galley(pill.min + Vec2::new(7.0, 2.0), g, theme::GOLD);
                    x += w + 6.0;
                }
                if sel {
                    let tag = Rect::from_min_size(
                        Pos2::new(r.right() - 74.0, r.top() - 8.0),
                        Vec2::new(64.0, 15.0),
                    );
                    p.rect_filled(tag, 3.0, theme::GOLD);
                    p.text(
                        tag.center(),
                        Align2::CENTER_CENTER,
                        "SELECTED",
                        theme::body_font(9.5),
                        theme::INK,
                    );
                }
            }
        });
    // the chosen class in detail, on the right
    let Some(c) = classes.get(f.class) else {
        return;
    };
    let (roles, diff) = class_meta(&c.key);
    let r = Rect::from_min_size(
        Pos2::new(screen.right() - 300.0, top),
        Vec2::new(280.0, (bottom - top).min(470.0)),
    );
    panel(ctx, "class-detail", r, |ui| {
        ui.label(
            RichText::new(&c.name)
                .font(theme::title_font(24.0))
                .color(col32(c.color)),
        );
        ui.label(
            RichText::new(roles.join(" · "))
                .font(theme::body_font(14.0))
                .color(theme::TEXT_DIM),
        );
        theme::ui_divider(ui, theme::GOLD_DARK);
        ui.label(
            RichText::new(&c.blurb)
                .font(theme::body_font(15.0))
                .color(theme::TEXT),
        );
        ui.add_space(8.0);
        egui::Grid::new("class-facts")
            .num_columns(2)
            .spacing([14.0, 4.0])
            .show(ui, |ui| {
                for (k, v) in [
                    ("ARMOUR", c.armor.clone()),
                    ("RESOURCE", c.resource.clone()),
                    ("DIFFICULTY", difficulty_text(diff).to_string()),
                ] {
                    ui.label(
                        RichText::new(k)
                            .font(theme::body_font(11.0))
                            .color(theme::TEXT_DIM),
                    );
                    ui.label(
                        RichText::new(v)
                            .font(theme::body_font(14.0))
                            .color(theme::TEXT),
                    );
                    ui.end_row();
                }
            });
        ui.add_space(8.0);
        ui.label(
            RichText::new("SIGNATURE ABILITIES")
                .font(theme::body_font(11.0))
                .color(theme::TEXT_DIM),
        );
        for a in &c.abilities {
            ui.label(
                RichText::new(format!("•  {a}"))
                    .font(theme::body_font(14.5))
                    .color(theme::GOLD),
            );
        }
    });
}

/// Hair styles as little portraits in the chosen skin and hair colours.
fn hair_thumbs(ui: &mut egui::Ui, look: &mut Appearance) {
    const NAMES: [&str; 5] = ["Bald", "Short", "Long", "Hood", "Bun"];
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Hair style")
                .font(theme::title_font(13.0))
                .color(theme::GOLD),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(NAMES[look.hair_style as usize % 5])
                    .font(theme::body_font(14.0))
                    .color(theme::TEXT),
            );
        });
    });
    let pick = |name: &str, i: u8| {
        let o = Look::options(name);
        col32(
            o.get(i as usize % o.len().max(1))
                .copied()
                .unwrap_or([0.5; 3]),
        )
    };
    let skin = pick("skin", look.skin);
    let hair = pick("hair", look.hair_color);
    let eyes = pick("eyes", look.eyes);
    let hood = Color32::from_rgb(92, 70, 52);
    ui.horizontal(|ui| {
        for style in 0..5u8 {
            let (r, resp) = ui.allocate_exact_size(Vec2::new(64.0, 70.0), egui::Sense::click());
            if resp.clicked() {
                look.hair_style = style;
            }
            let on = look.hair_style == style;
            let p = ui.painter();
            p.rect_filled(
                r,
                4.0,
                Color32::from_black_alpha(if resp.hovered() { 90 } else { 130 }),
            );
            p.rect_stroke(
                r,
                4.0,
                egui::Stroke::new(
                    if on { 2.5_f32 } else { 1.0 },
                    if on {
                        theme::GOLD_BRIGHT
                    } else {
                        theme::OUTLINE
                    },
                ),
                egui::StrokeKind::Inside,
            );
            // a head 8 "pixels" wide, each pixel 4 points
            let u = 4.0;
            let o = r.center() - Vec2::new(4.0 * u, 4.5 * u);
            let px = |x: f32, y: f32, w: f32, h: f32, c: Color32| {
                p.rect_filled(
                    Rect::from_min_size(o + Vec2::new(x * u, y * u), Vec2::new(w * u, h * u)),
                    0.0,
                    c,
                );
            };
            // behind the head
            match style {
                2 => px(-0.5, 1.0, 9.0, 8.5, hair),
                3 => px(-1.0, -1.0, 10.0, 11.0, hood),
                _ => {}
            }
            if style == 4 {
                px(2.5, -2.5, 3.0, 2.5, hair);
            }
            px(0.0, 0.0, 8.0, 8.0, skin);
            // eyes and mouth
            px(1.5, 4.0, 1.0, 1.0, eyes);
            px(5.5, 4.0, 1.0, 1.0, eyes);
            px(3.0, 6.0, 2.0, 0.6, Color32::from_rgb(120, 60, 50));
            if look.beard {
                px(0.0, 6.0, 8.0, 2.0, hair);
                px(1.0, 8.0, 6.0, 1.0, hair);
                px(3.0, 6.3, 2.0, 0.6, Color32::from_rgb(120, 60, 50));
            }
            match style {
                1 => {
                    px(0.0, 0.0, 8.0, 2.0, hair);
                    px(0.0, 2.0, 1.0, 1.5, hair);
                    px(7.0, 2.0, 1.0, 1.5, hair);
                }
                2 => {
                    px(0.0, 0.0, 8.0, 2.5, hair);
                    px(0.0, 2.5, 1.0, 6.0, hair);
                    px(7.0, 2.5, 1.0, 6.0, hair);
                }
                3 => {
                    px(-1.0, -1.0, 10.0, 2.0, hood);
                    px(-1.0, 1.0, 1.5, 9.0, hood);
                    px(7.5, 1.0, 1.5, 9.0, hood);
                    px(0.0, 1.0, 8.0, 1.0, hair);
                }
                4 => {
                    px(0.0, 0.0, 8.0, 2.0, hair);
                    px(0.0, 2.0, 1.0, 1.0, hair);
                    px(7.0, 2.0, 1.0, 1.0, hair);
                }
                _ => {}
            }
            p.text(
                Pos2::new(r.center().x, r.bottom() - 3.0),
                Align2::CENTER_BOTTOM,
                NAMES[style as usize],
                theme::body_font(10.5),
                if on {
                    theme::GOLD_BRIGHT
                } else {
                    theme::TEXT_DIM
                },
            );
            ui.add_space(-4.0);
        }
    });
    ui.add_space(4.0);
}
