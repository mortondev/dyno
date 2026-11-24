//! `dyno drive`: the real client, without a window, played by a script.
//!
//! The script's mouse and keyboard lines become [`Hand`]s and go through
//! [`App::input`], the same path the window's events take; egui gets the
//! same pointer and key events egui-winit would give it; frames are drawn
//! (off screen) by [`App::frame`]. So a click in a script meets everything a
//! player's click meets: the HUD's areas, tips, windows, steering, the world
//! pick. After each step the script says what must now be true (`expect`),
//! and the run fails on the first thing that is not.
//!
//! One command a line; `#` starts a comment. Things on screen are named by
//! the marks the interface leaves (`probe.rs`): `bar.0`, `dialog.accept`,
//! `bag.3`, `header.screens`. See docs/design/ui.md for the list.

use super::*;
use crate::probe;

pub struct DriveOptions {
    pub addr: String,
    pub name: String,
    /// the class of the character to make (it is made if it does not exist)
    pub class: String,
    pub script: String,
    pub size: (u32, u32),
    /// where `shot NAME` lines save their pictures
    pub shots: Option<std::path::PathBuf>,
}

const HELP: &str = "\
say TEXT                      chat or a /command (set-up: sent directly)
wait SECONDS
look-at KIND NAME             set-up: turn the camera to an npc, creature or corpse
turn DEGREES                  set-up: turn the character and the camera
walk-to KIND NAME DIST        hold W toward it until within DIST metres
click left|right KIND NAME    click it in the world (it must be on screen and uncovered)
click left|right at MARK      click a marked part of the interface
hover at MARK                 rest the pointer there
drag at MARK to MARK | by DX DY
press at MARK · move at MARK · release   a drag in three lines (to `shot` it half way)
key NAME                      press and release (1-9, 0, Minus, Equal, B, C, P, K, L, M, Tab, Escape, Enter)
remember MARK                 note where it is, for `expect moved MARK`
shot NAME                     save the frame
expect [not] CONDITION        wait up to 8 s for it, else fail
until CONDITION ... end       repeat the lines between (up to 6 times) until it holds
conditions: panel dialog|log|vendor|none · quest KEY · done KEY · target creature|npc|none ·
  attacking · casting · gcd · loot open|closed · tab bags|character|spellbook|skills|none ·
  equipped ITEM · pack ITEM · bar N KEY|empty · mark NAME · tip ID|any|none · moved MARK ·
  hover creature|npc|corpse|none · map open|closed";

struct Driver {
    app: App,
    shots: Option<std::path::PathBuf>,
    remembered: HashMap<String, Rect>,
    log: Vec<String>,
    /// what the client held at each stage of the last clicks (shown when a
    /// step fails)
    trail: std::cell::RefCell<std::collections::VecDeque<String>>,
}

fn play(app: &App) -> Result<&Play, String> {
    match &app.screen {
        Screen::Playing(p) => Ok(p),
        _ => Err("the client is no longer in the world (the connection closed)".into()),
    }
}

fn play_mut(app: &mut App) -> Result<&mut Play, String> {
    match &mut app.screen {
        Screen::Playing(p) => Ok(p),
        _ => Err("the client is no longer in the world (the connection closed)".into()),
    }
}

fn key_code(name: &str) -> Option<KeyCode> {
    Some(match name.to_lowercase().as_str() {
        "1" => KeyCode::Digit1,
        "2" => KeyCode::Digit2,
        "3" => KeyCode::Digit3,
        "4" => KeyCode::Digit4,
        "5" => KeyCode::Digit5,
        "6" => KeyCode::Digit6,
        "7" => KeyCode::Digit7,
        "8" => KeyCode::Digit8,
        "9" => KeyCode::Digit9,
        "0" => KeyCode::Digit0,
        "minus" | "-" => KeyCode::Minus,
        "equal" | "=" => KeyCode::Equal,
        "b" => KeyCode::KeyB,
        "c" => KeyCode::KeyC,
        "p" => KeyCode::KeyP,
        "k" => KeyCode::KeyK,
        "l" => KeyCode::KeyL,
        "m" => KeyCode::KeyM,
        "w" => KeyCode::KeyW,
        "tab" => KeyCode::Tab,
        "escape" | "esc" => KeyCode::Escape,
        "enter" => KeyCode::Enter,
        _ => return None,
    })
}

impl Driver {
    /// Draw frames at about 60 a second.
    fn frames(&mut self, n: usize) -> Result<(), String> {
        for _ in 0..n {
            let t = Instant::now();
            probe::begin_frame();
            self.app.frame();
            play(&self.app)?;
            if let Some(rest) = Duration::from_millis(16).checked_sub(t.elapsed()) {
                std::thread::sleep(rest);
            }
        }
        Ok(())
    }

    fn secs(&mut self, s: f32) -> Result<(), String> {
        let until = Instant::now() + Duration::from_secs_f32(s);
        while Instant::now() < until {
            self.frames(1)?;
        }
        Ok(())
    }

    /// An entity by kind and (part of) its name or key: the nearest that
    /// matches, and the point on it to aim at.
    fn entity(&self, kind: &str, name: &str) -> Result<(u32, Vec3), String> {
        self.entities(kind, name)?
            .into_iter()
            .next()
            .ok_or_else(|| format!("no {kind} '{name}' is near"))
    }

    /// Every entity of a kind that matches a name, nearest first.
    fn entities(&self, kind: &str, name: &str) -> Result<Vec<(u32, Vec3)>, String> {
        let p = play(&self.app)?;
        let want = name.to_lowercase();
        let me = p.player.pos;
        let mut all: Vec<(u32, Vec3)> = p
            .entities
            .iter()
            .filter(|(_, e)| e.pos().distance(me) < 40.0)
            .filter_map(|(id, e)| {
                let (label, key, up) = match (kind, e.kind) {
                    ("npc", EntityKind::Npc(n)) => {
                        let n = p.content.npc(n)?;
                        (n.name.to_lowercase(), n.key.clone(), 1.0)
                    }
                    ("creature", EntityKind::Creature(c)) => {
                        let c = p.content.creature(c)?;
                        (c.name.to_lowercase(), c.key.clone(), 0.6)
                    }
                    // (one that still holds something)
                    (
                        "corpse",
                        EntityKind::Corpse {
                            creature,
                            loot: true,
                        },
                    ) => {
                        let c = p.content.creature(creature)?;
                        (c.name.to_lowercase(), c.key.clone(), 0.45)
                    }
                    _ => return None,
                };
                (want == "any" || key == want || label.contains(&want))
                    .then(|| (*id, e.pos() + Vec3::Y * up))
            })
            .collect();
        all.sort_by(|a, b| a.1.distance(me).total_cmp(&b.1.distance(me)));
        Ok(all)
    }

    /// Wait (up to 12 s) for an entity to stand still, and give the point
    /// on it to aim at.
    fn settled(&mut self, id: u32, kind: &str) -> Result<Vec3, String> {
        let up = match kind {
            "npc" => 1.0,
            "creature" => 0.6,
            _ => 0.45,
        };
        let pos = |d: &Driver| -> Result<Vec3, String> {
            play(&d.app)?
                .entities
                .get(&id)
                .map(|e| e.pos())
                .ok_or_else(|| "it has gone".to_string())
        };
        let until = Instant::now() + Duration::from_secs(12);
        loop {
            let before = pos(self)?;
            self.frames(4)?;
            let now = pos(self)?;
            if now.distance(before) < 0.03 || Instant::now() > until {
                return Ok(now + Vec3::Y * up);
            }
        }
    }

    /// Where a point of the world is on screen, in physical pixels.
    fn on_screen(&self, at: Vec3) -> Result<(f64, f64), String> {
        let p = play(&self.app)?;
        let vp = p.view_proj.ok_or("no frame has been drawn yet")?;
        let clip = vp * at.extend(1.0);
        if clip.w <= 0.1 {
            return Err("it is behind the camera".into());
        }
        let ndc = clip.truncate() / clip.w;
        if ndc.x.abs() > 0.98 || ndc.y.abs() > 0.98 {
            return Err("it is off the screen".into());
        }
        let (w, h) = p.screen_px;
        Ok((
            ((ndc.x * 0.5 + 0.5) * w) as f64,
            ((0.5 - ndc.y * 0.5) * h) as f64,
        ))
    }

    /// The middle of a marked part of the interface, in physical pixels.
    fn mark(&self, name: &str) -> Result<(f64, f64), String> {
        let r = probe::get(name).ok_or_else(|| {
            format!(
                "nothing marked '{name}' is on screen (marked: {})",
                probe::names().join(" ")
            )
        })?;
        let zoom = self.app.egui_ctx.zoom_factor();
        let c = r.center();
        Ok(((c.x * zoom) as f64, (c.y * zoom) as f64))
    }

    fn move_to(&mut self, at: (f64, f64)) -> Result<(), String> {
        self.app.input(Hand::Cursor(at.0, at.1));
        self.frames(2)
    }

    fn press(&mut self, button: MouseButton, at: (f64, f64)) -> Result<(), String> {
        if self.app.cursor != at {
            self.move_to(at)?;
        }
        self.trace("before press");
        self.app.input(Hand::Mouse(button, true));
        self.trace("pressed");
        self.frames(2)?;
        self.trace("held two frames");
        self.app.input(Hand::Mouse(button, false));
        self.trace("released");
        self.frames(3)
    }

    /// Note what the client holds at a stage of a click.
    fn trace(&self, at: &str) {
        if let Ok(p) = play(&self.app) {
            let kind = |id: Option<u32>| id.and_then(|id| p.entities.get(&id)).map(|e| e.kind);
            let mut trail = self.trail.borrow_mut();
            let sight = p.cursor_ray(self.app.cursor).map(|(o, d)| {
                let wall = game::raycast_solid(&p.world, o, d, 60.0).map(|t| {
                    (Vec3::new(
                        t.block[0] as f32 + 0.5,
                        t.block[1] as f32 + 0.5,
                        t.block[2] as f32 + 0.5,
                    ) - o)
                        .length()
                });
                // the nearest person: where, and how far the line passes from them
                let me = p.player.pos;
                let near = p
                    .entities
                    .iter()
                    .filter(|(_, e)| matches!(e.kind, EntityKind::Npc(_)))
                    .min_by(|a, b| a.1.pos().distance(me).total_cmp(&b.1.pos().distance(me)))
                    .map(|(id, e)| {
                        let c = e.pos() + Vec3::Y;
                        let off = (c - o).cross(d).length();
                        format!(
                            "; nearest person #{id} at {:.2} {:.2} {:.2}, {:.2} m off the line",
                            e.pos().x,
                            e.pos().y,
                            e.pos().z,
                            off
                        )
                    })
                    .unwrap_or_default();
                format!(
                    "camera {:.2} {:.2} {:.2}, first solid block {}{near}",
                    o.x,
                    o.y,
                    o.z,
                    wall.map_or("none".into(), |w| format!("{w:.2} m"))
                )
            });
            trail.push_back(format!(
                "{at}: pointer on {:?}, target {:?}, steering {}, buttons held {}{}, over the interface {}, the interface wants the pointer {}; cursor {:.0},{:.0}; facing {:.3} {:.3} orbit {:.3}; at {:.2} {:.2} {:.2}; {}",
                kind(p.hover),
                kind(p.focus),
                p.captured,
                if p.lmb { "L" } else { "-" },
                if p.rmb { "R" } else { "-" },
                self.app.egui_ctx.is_pointer_over_area(),
                self.app.egui_ctx.wants_pointer_input(),
                self.app.cursor.0,
                self.app.cursor.1,
                p.player.yaw,
                p.player.pitch,
                p.cam_orbit,
                p.player.pos.x,
                p.player.pos.y,
                p.player.pos.z,
                sight.unwrap_or_default()
            ));
            while trail.len() > 8 {
                trail.pop_front();
            }
        }
    }

    /// Turn the character and the camera toward a point (set-up).
    fn face(&mut self, at: Vec3) -> Result<(), String> {
        let p = play_mut(&mut self.app)?;
        let d = at - p.player.pos;
        p.player.yaw = (-d.x).atan2(-d.z);
        p.player.pitch = -0.32;
        p.cam_orbit = 0.0;
        Ok(())
    }

    fn cond(&self, w: &[&str]) -> Result<bool, String> {
        let p = play(&self.app)?;
        let arg = w.get(1).copied().unwrap_or("");
        let item = |key: &str| {
            p.content
                .item_by_key(key)
                .ok_or_else(|| format!("the pack has no item '{key}'"))
        };
        Ok(match w.first().copied().unwrap_or("") {
            "panel" => match arg {
                "dialog" => matches!(p.panel, Some(rpg_ui::Panel::Dialog { .. })),
                "log" => matches!(p.panel, Some(rpg_ui::Panel::Log { .. })),
                "vendor" => matches!(p.panel, Some(rpg_ui::Panel::Vendor { .. })),
                "none" => p.panel.is_none(),
                _ => return Err(format!("panel what? '{arg}'")),
            },
            "quest" => p.quest_log.iter().any(|q| {
                p.content
                    .quest(q.quest)
                    .is_some_and(|quest| quest.key == arg)
            }),
            "done" => p
                .quests_done
                .iter()
                .any(|q| p.content.quest(*q).is_some_and(|quest| quest.key == arg)),
            "target" => {
                let kind = p.focus.and_then(|id| p.entities.get(&id)).map(|e| e.kind);
                match arg {
                    "creature" => matches!(kind, Some(EntityKind::Creature(_))),
                    "npc" => matches!(kind, Some(EntityKind::Npc(_))),
                    "none" => kind.is_none(),
                    _ => return Err(format!("target what? '{arg}'")),
                }
            }
            "hover" => {
                let kind = p.hover.and_then(|id| p.entities.get(&id)).map(|e| e.kind);
                match arg {
                    "creature" => matches!(kind, Some(EntityKind::Creature(_))),
                    "npc" => matches!(kind, Some(EntityKind::Npc(_))),
                    "corpse" => matches!(kind, Some(EntityKind::Corpse { .. })),
                    "none" => kind.is_none(),
                    _ => return Err(format!("hover what? '{arg}'")),
                }
            }
            "attacking" => p.attacking.is_some(),
            "casting" => p.my_cast.is_some(),
            "gcd" => p.gcd.0 > Instant::now(),
            "loot" => (arg == "open") == p.loot.is_some(),
            "map" => (arg == "open") == p.ui.map_open,
            "tab" => match arg {
                "bags" => p.screen_tab == Some(screens::Tab::Bags),
                "character" => p.screen_tab == Some(screens::Tab::Character),
                "spellbook" => p.screen_tab == Some(screens::Tab::Spellbook),
                "skills" => p.screen_tab == Some(screens::Tab::Skills),
                "none" => p.screen_tab.is_none(),
                _ => return Err(format!("tab what? '{arg}'")),
            },
            "equipped" => {
                let it = item(arg)?;
                p.equip.iter().flatten().any(|s| s.item == it)
            }
            "pack" => {
                let it = item(arg)?;
                p.pack.iter().flatten().any(|s| s.item == it)
            }
            "bar" => {
                let i: usize = arg.parse().map_err(|_| "bar N KEY: N is 0 to 11")?;
                let want = w.get(2).copied().unwrap_or("empty");
                match (p.bar.get(i).copied().flatten(), want) {
                    (None, "empty") => true,
                    (Some(a), key) => p.content.ability(a).is_some_and(|a| a.key == key),
                    _ => false,
                }
            }
            "mark" => probe::get(arg).is_some(),
            "tip" => match (p.ui.tips.current(), arg) {
                (None, "none") => true,
                (Some(_), "any") => true,
                (Some(t), id) => t.id == id,
                _ => false,
            },
            "moved" => {
                let was = self
                    .remembered
                    .get(arg)
                    .ok_or_else(|| format!("'{arg}' was not remembered"))?;
                probe::get(arg).is_some_and(|now| (now.min - was.min).length() > 20.0)
            }
            other => return Err(format!("unknown condition '{other}'")),
        })
    }

    /// What the client shows now, for a failure's message.
    fn state(&self) -> String {
        let Ok(p) = play(&self.app) else {
            return "not in the world".into();
        };
        let kind = |id: Option<u32>| {
            id.and_then(|id| p.entities.get(&id))
                .map(|e| format!("{:?}", e.kind))
                .unwrap_or_else(|| "none".into())
        };
        let trail: Vec<String> = self.trail.borrow().iter().cloned().collect();
        format!(
            "panel {:?}; tab {:?}; target {}; hover {}; loot {}; tip {}; captured {}; over the interface {}; marks: {}\n  the last clicks:\n    {}",
            p.panel,
            p.screen_tab,
            kind(p.focus),
            kind(p.hover),
            p.loot.is_some(),
            p.ui.tips.current().map_or("none", |t| t.id.as_str()),
            p.captured,
            self.app.egui_ctx.is_pointer_over_area(),
            probe::names().join(" "),
            trail.join("\n    ")
        )
    }

    fn expect(&mut self, w: &[&str], secs: f32) -> Result<(), String> {
        let (not, w) = match w.first() {
            Some(&"not") => (true, &w[1..]),
            _ => (false, w),
        };
        let until = Instant::now() + Duration::from_secs_f32(secs);
        loop {
            if self.cond(w)? != not {
                return Ok(());
            }
            if Instant::now() > until {
                return Err(format!("it did not hold. Now: {}", self.state()));
            }
            self.frames(2)?;
        }
    }

    fn run_line(&mut self, w: &[&str]) -> Result<(), String> {
        let need = |i: usize| {
            w.get(i)
                .copied()
                .ok_or_else(|| format!("'{}' needs more words (see `dyno drive --help`)", w[0]))
        };
        match w[0] {
            "say" => {
                let text = w[1..].join(" ");
                let p = play(&self.app)?;
                let _ = p.conn.tx.send(ClientMsg::Chat { text });
                self.frames(6)
            }
            "wait" => {
                let s: f32 = need(1)?.parse().map_err(|_| "wait SECONDS")?;
                self.secs(s)
            }
            "turn" => {
                let by: f32 = need(1)?.parse().map_err(|_| "turn DEGREES")?;
                let p = play_mut(&mut self.app)?;
                p.player.yaw += by.to_radians();
                p.cam_orbit = 0.0;
                // (long enough for the server to hear which way I face)
                self.secs(0.4)
            }
            "look-at" => {
                let (_, at) = self.entity(need(1)?, need(2)?)?;
                self.face(at)?;
                self.frames(3)
            }
            "walk-to" => {
                let (kind, name) = (need(1)?, need(2)?);
                let dist: f32 = need(3)?.parse().map_err(|_| "walk-to KIND NAME DIST")?;
                let started = Instant::now();
                self.app.input(Hand::Key(KeyCode::KeyW, true, false));
                let done = loop {
                    let (_, at) = self.entity(kind, name)?;
                    let d = {
                        let p = play(&self.app)?;
                        Vec3::new(at.x - p.player.pos.x, 0.0, at.z - p.player.pos.z).length()
                    };
                    if d <= dist {
                        break Ok(());
                    }
                    if started.elapsed() > Duration::from_secs(25) {
                        break Err(format!("still {d:.1} m away after 25 s"));
                    }
                    self.face(at)?;
                    self.frames(1)?;
                };
                self.app.input(Hand::Key(KeyCode::KeyW, false, false));
                self.frames(2)?;
                done
            }
            "click" => {
                let button = match need(1)? {
                    "left" => MouseButton::Left,
                    "right" => MouseButton::Right,
                    b => return Err(format!("click left or right, not '{b}'")),
                };
                if need(2)? == "at" {
                    let at = self.mark(need(3)?)?;
                    return self.press(button, at);
                }
                // creatures wander: aim, look where the pointer landed, and
                // aim again (at the next nearest, too) until it is on one
                let (kind, name) = (need(2)?, need(3)?);
                let mut why = String::new();
                for attempt in 0..16 {
                    let all = self.entities(kind, name)?;
                    if all.is_empty() {
                        return Err(format!("no {kind} '{name}' is near"));
                    }
                    let (id, _) = all[(attempt / 2) % all.len()];
                    // a walker is clicked when it next stands still (a
                    // player waits for that too)
                    let at = self.settled(id, kind)?;
                    // (a little higher or lower each time: a different line
                    // of sight past whatever is in the way)
                    let feet = {
                        let p = play(&self.app)?;
                        p.entities.get(&id).map_or(at, |e| e.pos())
                    };
                    let at = feet + (at - feet) * [1.0, 1.3, 0.65, 1.15][attempt % 4];
                    let px = match self.on_screen(at) {
                        Ok(px) => px,
                        Err(e) => {
                            why = e;
                            self.face(at)?;
                            self.frames(3)?;
                            continue;
                        }
                    };
                    // (one frame, so a walker has not gone far by the time
                    // the game looks under the pointer)
                    self.app.input(Hand::Cursor(px.0, px.1));
                    self.frames(1)?;
                    // what a player sees before clicking: is the pointer on it?
                    if self.app.egui_ctx.is_pointer_over_area() {
                        return Err(format!("the interface covers it. {}", self.state()));
                    }
                    let hover = play(&self.app)?.hover;
                    if hover.is_some_and(|h| all.iter().any(|(id, _)| *id == h)) {
                        return self.press(button, px);
                    }
                    // why not: something else under the pointer, or a block
                    // in the line of sight?
                    let p = play(&self.app)?;
                    if let Some(other) = hover.and_then(|h| p.entities.get(&h)) {
                        why = format!("{:?} is in front of it, under the pointer", other.kind);
                        self.face(at)?;
                        self.frames(3)?;
                        continue;
                    }
                    why = match p.cursor_ray(self.app.cursor) {
                        Some((o, d)) => {
                            let wall = game::raycast_solid(&p.world, o, d, 60.0).map(|t| {
                                let c = Vec3::new(
                                    t.block[0] as f32 + 0.5,
                                    t.block[1] as f32 + 0.5,
                                    t.block[2] as f32 + 0.5,
                                );
                                (c - o).length()
                            });
                            let e = p.entities.get(&id);
                            format!(
                                "the pointer is on its place on screen but the game picks nothing there (it is {:.1} m from the camera; the first block on that line is {}; #{id} is {:?} at {:?}, aimed at {:?}, {:.2} m off the pointer's line; a pick now gives {:?}; cursor {:?} of {:?}; zoom {:.1})",
                                (at - o).length(),
                                wall.map_or("none".into(), |w| format!("{w:.1} m away")),
                                e.map(|e| e.kind),
                                e.map(|e| e.pos()),
                                at,
                                (at - o).cross(d).length(),
                                pick_entity_ray(p, o, d, 60.0),
                                self.app.cursor,
                                p.screen_px,
                                p.zoom
                            )
                        }
                        None => "the pointer is not on it".into(),
                    };
                    self.face(at)?;
                    self.frames(3)?;
                }
                Err(format!("{why}. {}", self.state()))
            }
            "hover" => {
                let at = self.mark(need(2)?)?;
                self.move_to(at)?;
                // long enough for a tooltip's delay
                self.secs(0.9)
            }
            "drag" => {
                let from = self.mark(need(2)?)?;
                let to = match need(3)? {
                    "to" => self.mark(need(4)?)?,
                    "by" => {
                        let dx: f64 = need(4)?.parse().map_err(|_| "drag ... by DX DY")?;
                        let dy: f64 = need(5)?.parse().map_err(|_| "drag ... by DX DY")?;
                        (from.0 + dx, from.1 + dy)
                    }
                    other => return Err(format!("drag at MARK to MARK | by DX DY, not '{other}'")),
                };
                self.move_to(from)?;
                self.app.input(Hand::Mouse(MouseButton::Left, true));
                self.frames(3)?;
                for k in 1..=8 {
                    let t = k as f64 / 8.0;
                    self.app.input(Hand::Cursor(
                        from.0 + (to.0 - from.0) * t,
                        from.1 + (to.1 - from.1) * t,
                    ));
                    self.frames(2)?;
                }
                self.frames(2)?;
                self.app.input(Hand::Mouse(MouseButton::Left, false));
                self.frames(4)
            }
            // a drag in three lines, to look at it half way
            "press" => {
                let at = self.mark(need(2)?)?;
                self.move_to(at)?;
                self.app.input(Hand::Mouse(MouseButton::Left, true));
                self.frames(3)
            }
            "move" => {
                let to = self.mark(need(2)?)?;
                let from = self.app.cursor;
                for k in 1..=8 {
                    let t = k as f64 / 8.0;
                    self.app.input(Hand::Cursor(
                        from.0 + (to.0 - from.0) * t,
                        from.1 + (to.1 - from.1) * t,
                    ));
                    self.frames(2)?;
                }
                Ok(())
            }
            "release" => {
                self.app.input(Hand::Mouse(MouseButton::Left, false));
                self.frames(4)
            }
            "key" => {
                let code = key_code(need(1)?).ok_or_else(|| format!("no key '{}'", w[1]))?;
                self.app.input(Hand::Key(code, true, false));
                self.frames(2)?;
                self.app.input(Hand::Key(code, false, false));
                self.frames(3)
            }
            "remember" => {
                let name = need(1)?;
                let r = probe::get(name).ok_or_else(|| format!("nothing marked '{name}'"))?;
                self.remembered.insert(name.to_string(), r);
                Ok(())
            }
            "shot" => {
                let name = need(1)?;
                let dir = self.shots.clone().ok_or("no --shots directory was given")?;
                let _ = std::fs::create_dir_all(&dir);
                if let Some(g) = self.app.gfx.as_mut() {
                    g.screenshot = Some(dir.join(format!("{name}.ppm")));
                }
                self.frames(2)
            }
            "expect" => self.expect(&w[1..], 8.0),
            other => Err(format!("unknown command '{other}'")),
        }
    }

    fn run(&mut self, script: &str) -> Result<usize, String> {
        let lines: Vec<(usize, Vec<&str>)> = script
            .lines()
            .enumerate()
            .map(|(n, l)| (n + 1, l.split('#').next().unwrap_or("").trim()))
            .filter(|(_, l)| !l.is_empty())
            .map(|(n, l)| (n, l.split_whitespace().collect()))
            .collect();
        let mut done = 0;
        let mut i = 0;
        // (the line an `until` began at, and how often its body has run)
        let mut block: Option<(usize, usize)> = None;
        while i < lines.len() {
            let (n, w) = &lines[i];
            let text = w.join(" ");
            let fail = |e: String| format!("line {n}: {text}\n  {e}");
            match w[0] {
                "until" => {
                    block = Some((i, 0));
                    i += 1;
                    continue;
                }
                "end" => {
                    let (start, tries) = block.ok_or_else(|| fail("no `until` is open".into()))?;
                    let cond = &lines[start].1[1..];
                    self.frames(4).map_err(&fail)?;
                    if self.cond(cond).map_err(&fail)? {
                        self.log.push(format!("ok   until {}", cond.join(" ")));
                        block = None;
                        done += 1;
                    } else if tries >= 5 {
                        return Err(fail(format!(
                            "'{}' never held in 6 tries. Now: {}",
                            cond.join(" "),
                            self.state()
                        )));
                    } else {
                        block = Some((start, tries + 1));
                        i = start;
                    }
                    i += 1;
                    continue;
                }
                _ => {}
            }
            self.run_line(w).map_err(&fail)?;
            if w[0] == "expect" || block.is_none() {
                self.log.push(format!("ok   {text}"));
            }
            done += 1;
            i += 1;
        }
        Ok(done)
    }
}

/// Join as a new character, play the script, and say how it went: `Ok` with
/// each step that held, `Err` with the first that did not.
pub fn drive(o: DriveOptions) -> Result<String, String> {
    if o.script.trim() == "help" {
        return Ok(HELP.into());
    }
    crate::assets::load_local_pack();
    let token = crate::settings::tool_token(&o.name);
    let join = |create: Option<proto::NewCharacter>| {
        net::connect_with(
            &o.addr,
            &o.name,
            net::JoinOptions {
                token,
                create,
                prepare_sections: true,
                ..Default::default()
            },
            Duration::from_secs(20),
        )
    };
    // the character, made now if this name has none yet
    let exists = net::realm(&o.addr, token, None, Duration::from_secs(8))
        .map(|(_, chars, _)| chars.iter().any(|c| c.name.eq_ignore_ascii_case(&o.name)))?;
    let j = join((!exists).then(|| proto::NewCharacter {
        class: o.class.clone(),
        look: Default::default(),
    }))?;
    let mut gfx = Gfx::new_headless(o.size.0, o.size.1)?;
    // as a join does: this world's textures, then its meshes as they are made
    let world = new_play(j);
    crate::textures::reload();
    gfx.begin_world(&world.world);
    probe::enable();
    // a fresh player: default settings, every tip still to come
    let settings = Settings::default();
    crate::options::set_interface(crate::options::Interface::of(&settings));
    let ctx = crate::theme::new_context();
    crate::theme::apply(&ctx);
    let zoom = crate::options::zoom_for(o.size.1 as f32, 1.0);
    ctx.options_mut(|opt| opt.zoom_factor = zoom);
    let app = App {
        front: Front::new(&[], ""),
        stage: None,
        stage_rx: None,
        stage_loaded: false,
        joining_new: false,
        menu: Menu {
            addr: String::new(),
            name: String::new(),
            error: None,
        },
        settings,
        gfx: Some(gfx),
        audio: Audio::silent(),
        egui_ctx: ctx,
        egui_state: None,
        screen: Screen::Playing(Box::new(world)),
        host: None,
        last_frame: Instant::now(),
        update_note: String::new(),
        fatal: None,
        autojoined: true,
        cursor: (0.0, 0.0),
        last_shot: Instant::now(),
        drive_events: Vec::new(),
        started: Instant::now(),
    };
    let mut d = Driver {
        app,
        shots: o.shots.clone(),
        remembered: HashMap::new(),
        log: Vec::new(),
        trail: Default::default(),
    };
    // the land round the character, before the first line
    let started = Instant::now();
    loop {
        d.frames(5)?;
        let p = play(&d.app)?;
        if !p.land_wait && started.elapsed() > Duration::from_secs(2) {
            break;
        }
        if started.elapsed() > Duration::from_secs(60) {
            return Err("the land round the character never arrived".into());
        }
    }
    let result = d.run(&o.script);
    // a picture of where it stopped, beside the others
    if let (Err(_), Some(dir)) = (&result, &d.shots) {
        let _ = std::fs::create_dir_all(dir);
        if let Some(g) = d.app.gfx.as_mut() {
            g.screenshot = Some(dir.join("failed.ppm"));
        }
        let _ = d.frames(2);
    }
    let log = d.log.join("\n");
    match result {
        Ok(n) => Ok(format!("{log}\nPASS: {n} steps")),
        Err(e) => Err(format!("{log}\nFAIL {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The interface's input test, as shipped in tools/drive.
    fn script() -> String {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/drive/first_steps.drive"
        );
        std::fs::read_to_string(path).unwrap()
    }

    #[test]
    fn the_shipped_script_uses_only_what_the_driver_knows() {
        let commands = [
            "say", "wait", "look-at", "walk-to", "click", "hover", "drag", "key", "remember",
            "shot", "expect", "until", "end", "press", "move", "release", "turn",
        ];
        let mut open = 0;
        let mut expects = 0;
        for (n, line) in script().lines().enumerate() {
            let line = line.split('#').next().unwrap().trim();
            let Some(first) = line.split_whitespace().next() else {
                continue;
            };
            assert!(commands.contains(&first), "line {}: '{first}'", n + 1);
            // every command the help lists, and no other
            assert!(HELP.contains(first), "'{first}' is not in the help");
            match first {
                "until" => open += 1,
                "end" => open -= 1,
                "expect" => expects += 1,
                "key" => {
                    let name = line.split_whitespace().nth(1).unwrap();
                    assert!(key_code(name).is_some(), "line {}: key '{name}'", n + 1);
                }
                _ => {}
            }
            assert!((0..=1).contains(&open), "line {}: until and end", n + 1);
        }
        assert_eq!(open, 0, "an `until` is left open");
        // it asserts what each step caused, not just that it ran
        assert!(expects >= 30, "{expects} expectations");
    }

    #[test]
    fn every_step_the_brief_names_is_in_the_script() {
        let s = script();
        for needed in [
            "click right npc", // right-click the quest giver
            "click left at dialog.accept",
            "click left creature",  // target a creature
            "key 1",                // press a key
            "click left at bar.",   // click a bar slot
            "click right corpse",   // loot
            "key B",                // open bags
            "expect equipped",      // equip
            "press at spell.",      // spellbook ability to the bar
            "expect mark tooltip.", // hover for a tooltip
            "click left at tip",    // dismiss a tip
            "drag at header.",      // drag a window
            "expect attacking",     // the world still takes clicks
        ] {
            assert!(s.contains(needed), "the script lacks: {needed}");
        }
    }
}
