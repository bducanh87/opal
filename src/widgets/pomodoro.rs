//! Focus timer (Pomodoro): focus / short break / long break, a progress
//! ring, start-pause, reset and skip. A long break follows every fourth
//! focus session. Finishing a session beeps and shows a toast.

use egui::{Align2, Color32, CornerRadius, Id, Rect, Sense, Shape, Stroke, StrokeKind, Ui, pos2, vec2};
use std::f32::consts::TAU;
use std::time::{Duration, Instant};

use super::{caps, dot_header, font, light, semibold};
use crate::app::{App, glyph};
use crate::glass::Look;
use crate::theme::{rgb, white};

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Mode {
    #[default]
    Focus,
    Short,
    Long,
}

impl Mode {
    fn label(self) -> &'static str {
        match self {
            Mode::Focus => "Focus",
            Mode::Short => "Short break",
            Mode::Long => "Long break",
        }
    }
    fn color(self) -> Color32 {
        match self {
            Mode::Focus => rgb(0xf0284a),
            Mode::Short => rgb(0x34d399),
            Mode::Long => rgb(0x60c3f6),
        }
    }
}

#[derive(Default)]
pub struct PomodoroState {
    pub mode: Mode,
    /// Remaining time while paused.
    remaining: Option<Duration>,
    /// End time while running.
    ends: Option<Instant>,
    pub completed: u32,
}

impl PomodoroState {
    fn total(&self, e: &crate::config::Extras) -> Duration {
        let m = match self.mode {
            Mode::Focus => e.focus_minutes,
            Mode::Short => e.short_break_minutes,
            Mode::Long => e.long_break_minutes,
        };
        Duration::from_secs(m as u64 * 60)
    }

    pub fn running(&self) -> bool {
        self.ends.is_some()
    }

    fn left(&self, e: &crate::config::Extras) -> Duration {
        match (self.ends, self.remaining) {
            (Some(end), _) => end.saturating_duration_since(Instant::now()),
            (None, Some(r)) => r,
            (None, None) => self.total(e),
        }
    }

    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.ends = None;
        self.remaining = None;
    }

    /// Move to the next session. Only a finished focus session counts
    /// towards the long break; skipping one does not.
    fn advance(&mut self, finished: bool) {
        let next = match self.mode {
            Mode::Focus => {
                if finished {
                    self.completed += 1;
                }
                if self.completed % 4 == 0 { Mode::Long } else { Mode::Short }
            }
            _ => Mode::Focus,
        };
        self.set_mode(next);
    }

    /// Returns a message when a session just finished.
    pub fn pump(&mut self) -> Option<String> {
        let end = self.ends?;
        if Instant::now() < end {
            return None;
        }
        let done = self.mode;
        self.advance(true);
        crate::win::beep();
        Some(match done {
            Mode::Focus => "Focus session done — time for a break".into(),
            _ => "Break over — back to focus".into(),
        })
    }
}

fn pill(ui: &mut Ui, rect: Rect, label: &str, active: bool, color: Color32, look: &Look, id: &str) -> bool {
    let resp = ui.interact(rect, Id::new(("pomo", id)), Sense::click());
    let p = ui.painter();
    if active {
        p.rect_filled(rect, CornerRadius::same(11), color.gamma_multiply(0.85));
    } else if resp.hovered() {
        p.rect_filled(rect, CornerRadius::same(11), look.ink.hover);
    }
    p.text(rect.center(), Align2::CENTER_CENTER, label, semibold(10.5), if active { Color32::WHITE } else { look.ink.dim });
    resp.clicked()
}

pub fn show(app: &mut App, ui: &mut Ui, look: &Look) {
    match app.state.settings.design("pomodoro").as_str() {
        "minimal" => show_minimal(app, ui, look),
        "tomato" => show_tomato(app, ui, look),
        _ => show_lumen(app, ui, look),
    }
}

/// Reset · Start/Pause · Skip, centred on `c`.
fn controls(app: &mut App, ui: &mut Ui, p: &egui::Painter, c: egui::Pos2, look: &Look) {
    let total = app.pomodoro.total(&app.extras);
    let left = app.pomodoro.left(&app.extras);
    let mode = app.pomodoro.mode;
    let main = Rect::from_center_size(c, vec2(96.0, 34.0));
    let running = app.pomodoro.running();
    let mr = ui.interact(main, Id::new("pomo-main"), Sense::click());
    p.rect_filled(main, CornerRadius::same(17), if mr.hovered() { mode.color() } else { mode.color().gamma_multiply(0.85) });
    p.text(
        main.center(),
        Align2::CENTER_CENTER,
        if running { "Pause" } else if left < total { "Resume" } else { "Start" },
        semibold(12.5),
        Color32::WHITE,
    );
    if mr.clicked() {
        let st = &mut app.pomodoro;
        if running {
            st.remaining = Some(left);
            st.ends = None;
        } else {
            st.ends = Some(Instant::now() + left);
            st.remaining = None;
        }
    }
    for (dx, ch, tip, id) in [(-84.0, glyph::REFRESH, "Reset", "reset"), (84.0, glyph::NEXT, "Skip", "skip")] {
        let b = Rect::from_center_size(pos2(c.x + dx, c.y), vec2(34.0, 34.0));
        let br = ui.interact(b, Id::new(("pomo", id)), Sense::click()).on_hover_text(tip);
        p.circle_filled(b.center(), 17.0, if br.hovered() { look.ink.hover_strong } else { white(14) });
        p.circle_stroke(b.center(), 17.0, Stroke::new(1.0_f32, look.ink.hair));
        p.text(b.center(), Align2::CENTER_CENTER, ch.to_string(), egui::FontId::new(12.0, egui::FontFamily::Name("icons".into())), look.ink.ink);
        if br.clicked() {
            if id == "reset" {
                app.pomodoro.set_mode(mode);
            } else {
                app.pomodoro.advance(false);
            }
        }
    }
}

fn clock_text(app: &App) -> (String, f32) {
    let total = app.pomodoro.total(&app.extras);
    let left = app.pomodoro.left(&app.extras);
    let secs = left.as_secs() + if left.subsec_millis() > 0 && app.pomodoro.running() { 1 } else { 0 };
    let frac = 1.0 - left.as_secs_f32() / total.as_secs_f32().max(1.0);
    (format!("{:02}:{:02}", secs / 60, secs % 60), frac)
}

/// Minimal: the mode, large digits and a thin progress line.
fn show_minimal(app: &mut App, ui: &mut Ui, look: &Look) {
    let r = ui.max_rect();
    let p = ui.painter().clone();
    let mode = app.pomodoro.mode;
    let (text, frac) = clock_text(app);
    p.text(r.left_top(), Align2::LEFT_TOP, caps(mode.label()), semibold(10.5), mode.color());
    p.text(r.right_top() - vec2(super::HEADER_GEAR_GAP, 0.0), Align2::RIGHT_TOP, format!("{} done", app.pomodoro.completed), semibold(10.5), look.ink.dim);
    let big = light((r.height() * 0.36).min(r.width() * 0.3).clamp(28.0, 110.0));
    let cy = r.min.y + 20.0 + big.size * 0.6;
    p.text(pos2(r.center().x, cy), Align2::CENTER_CENTER, text, big.clone(), look.ink.ink);
    let line = Rect::from_min_size(pos2(r.min.x, cy + big.size * 0.62), vec2(r.width(), 3.0));
    p.rect_filled(line, CornerRadius::same(2), white(28));
    p.rect_filled(Rect::from_min_size(line.min, vec2(line.width() * frac.clamp(0.0, 1.0), 3.0)), CornerRadius::same(2), mode.color());
    // Mode switch as text links.
    let modes = [(Mode::Focus, "Focus"), (Mode::Short, "Short"), (Mode::Long, "Long")];
    let y = line.max.y + 18.0;
    for (i, (m, label)) in modes.iter().enumerate() {
        let x = r.min.x + r.width() * (i as f32 + 0.5) / 3.0;
        let rect = Rect::from_center_size(pos2(x, y), vec2(70.0, 20.0));
        let resp = ui.interact(rect, Id::new(("pomo-min", *label)), Sense::click());
        let active = app.pomodoro.mode == *m;
        p.text(rect.center(), Align2::CENTER_CENTER, *label, if active { semibold(12.0) } else { font(12.0) }, if active { m.color() } else if resp.hovered() { look.ink.ink } else { look.ink.dim });
        if resp.clicked() {
            app.pomodoro.set_mode(*m);
        }
    }
    controls(app, ui, &p, pos2(r.center().x, r.max.y - 20.0), look);
}

/// Tomato: a ripe tomato that fills from the bottom as the session runs.
fn show_tomato(app: &mut App, ui: &mut Ui, look: &Look) {
    let r = ui.max_rect();
    let p = ui.painter().clone();
    let mode = app.pomodoro.mode;
    let (text, frac) = clock_text(app);
    let body = dot_header(&p, r, mode.label(), &format!("{} done", app.pomodoro.completed), look);
    let area = Rect::from_min_max(body.min, pos2(body.max.x, body.max.y - 48.0));
    let rad = (area.width().min(area.height()) / 2.0 - 6.0).max(24.0);
    let c = area.center() + vec2(0.0, rad * 0.05);
    let (dark, bright) = match mode {
        Mode::Focus => (rgb(0x5a1218), rgb(0xe8352f)),
        Mode::Short => (rgb(0x12402c), rgb(0x34c77b)),
        Mode::Long => (rgb(0x123049), rgb(0x3aa0e8)),
    };
    // Squashed tomato body.
    let pts = |r_: f32| -> Vec<egui::Pos2> {
        (0..64).map(|k| {
            let a = k as f32 / 64.0 * std::f32::consts::TAU;
            c + vec2(a.cos() * r_ * 1.06, a.sin() * r_ * 0.94)
        }).collect()
    };
    p.add(egui::Shape::convex_polygon(pts(rad), dark, Stroke::NONE));
    // Fill level: the part of the ellipse below the water line.
    let level = c.y + rad * 0.94 - frac.clamp(0.0, 1.0) * rad * 1.88;
    let filled: Vec<egui::Pos2> = pts(rad).into_iter().map(|q| if q.y < level { pos2(q.x, level) } else { q }).collect();
    if frac > 0.0 {
        p.add(egui::Shape::convex_polygon(filled, bright, Stroke::NONE));
    }
    // Shine and leaves.
    p.circle_filled(c + vec2(-rad * 0.45, -rad * 0.4), rad * 0.12, white(60));
    let stem = c + vec2(0.0, -rad * 0.92);
    for k in 0..5 {
        let a = -std::f32::consts::FRAC_PI_2 + (k as f32 - 2.0) * 0.55;
        p.line_segment([stem, stem + vec2(a.cos(), a.sin() * 0.5 + 0.25) * rad * 0.32], Stroke::new(rad * 0.07, rgb(0x3fa34d)));
    }
    p.line_segment([stem, stem + vec2(rad * 0.04, -rad * 0.16)], Stroke::new(rad * 0.06, rgb(0x2f7d3a)));
    p.text(c + vec2(0.0, rad * 0.08), Align2::CENTER_CENTER, text, light(rad * 0.46), Color32::WHITE);
    controls(app, ui, &p, pos2(body.center().x, body.max.y - 20.0), look);
}

fn show_lumen(app: &mut App, ui: &mut Ui, look: &Look) {
    let r = ui.max_rect();
    let p = ui.painter().clone();
    let total = app.pomodoro.total(&app.extras);
    let left = app.pomodoro.left(&app.extras);
    let mode = app.pomodoro.mode;
    let header_right = format!("{} done", app.pomodoro.completed);
    let body = dot_header(&p, r, "Focus timer", &header_right, look);

    // Mode pills.
    let pills = [(Mode::Focus, "Focus"), (Mode::Short, "Short"), (Mode::Long, "Long")];
    let pw = (body.width() - 12.0) / 3.0;
    for (i, (m, label)) in pills.iter().enumerate() {
        let rect = Rect::from_min_size(pos2(body.min.x + i as f32 * (pw + 6.0), body.min.y), vec2(pw, 22.0));
        if pill(ui, rect, label, mode == *m, m.color(), look, label) {
            app.pomodoro.set_mode(*m);
        }
    }

    // Ring.
    let controls_h = 44.0;
    let ring_area = Rect::from_min_max(pos2(body.min.x, body.min.y + 30.0), pos2(body.max.x, body.max.y - controls_h));
    let rad = (ring_area.width().min(ring_area.height()) / 2.0 - 8.0).max(20.0);
    let c = ring_area.center();
    let frac = 1.0 - left.as_secs_f32() / total.as_secs_f32().max(1.0);
    p.circle_stroke(c, rad, Stroke::new(rad * 0.09, white(22)));
    if frac > 0.0 {
        let steps = (frac * 120.0).ceil().max(2.0) as usize;
        let pts: Vec<_> = (0..=steps)
            .map(|i| {
                let a = -TAU / 4.0 + TAU * frac * i as f32 / steps as f32;
                c + vec2(a.cos(), a.sin()) * rad
            })
            .collect();
        p.add(Shape::line(pts, Stroke::new(rad * 0.09, mode.color())));
    }
    let secs = left.as_secs() + if left.subsec_millis() > 0 && app.pomodoro.running() { 1 } else { 0 };
    let text = format!("{:02}:{:02}", secs / 60, secs % 60);
    p.text(c - vec2(0.0, rad * 0.08), Align2::CENTER_CENTER, text, light(rad * 0.5), look.ink.ink);
    p.text(c + vec2(0.0, rad * 0.36), Align2::CENTER_CENTER, caps(mode.label()), semibold(9.5), look.ink.dim);

    controls(app, ui, &p, pos2(c.x, body.max.y - controls_h / 2.0), look);
    let _ = (font(1.0), StrokeKind::Inside);
}
