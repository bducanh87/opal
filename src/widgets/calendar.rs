//! Month calendar with optional ICS subscription (Google/Outlook "secret
//! address in iCal format"). Designs: Lumen, Almanac, Blocks, Pop, Prism,
//! Grove.

use chrono::{Datelike, Duration as CDuration, Local, NaiveDate};
use egui::{Align2, Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Ui, pos2, vec2};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use super::{font, semibold, serif};
use crate::app::{App, glyph};
use crate::config::CalendarConfig;
use crate::glass::Look;
use crate::theme::rgb;
pub use crate::ics::{Event, add_months};
use crate::ics::fetch;

pub struct CalendarState {
    pub view: NaiveDate,
    pub selected: NaiveDate,
    today: NaiveDate,
    pub events: Vec<Event>,
    pub status: String,
    loaded_url: String,
    last_fetch: Option<Instant>,
    fetching: bool,
    req: Sender<String>,
    res: Receiver<(String, Result<Vec<Event>, String>)>,
}

impl CalendarState {
    pub fn new(ctx: &egui::Context, _cfg: &CalendarConfig) -> Self {
        let (req, jobs) = channel::<String>();
        let (tx, res) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            while let Ok(url) = jobs.recv() {
                let result = fetch(&url);
                if tx.send((url, result)).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        let today = Local::now().date_naive();
        CalendarState {
            view: today.with_day(1).unwrap(),
            selected: today,
            today,
            events: Vec::new(),
            status: String::new(),
            loaded_url: String::new(),
            last_fetch: None,
            fetching: false,
            req,
            res,
        }
    }

    pub fn refresh(&mut self) {
        self.last_fetch = None;
        self.loaded_url.clear();
    }

    pub fn pump(&mut self, cfg: &CalendarConfig) {
        // A desktop widget runs for days: follow "today" across midnight.
        let today = Local::now().date_naive();
        if today != self.today {
            if self.selected == self.today {
                self.selected = today;
            }
            if self.view == self.today.with_day(1).unwrap() {
                self.view = today.with_day(1).unwrap();
            }
            self.today = today;
            self.last_fetch = None; // the event window moves with the date
        }
        while let Ok((url, result)) = self.res.try_recv() {
            self.fetching = false;
            if url != cfg.ics_url.trim() {
                continue;
            }
            match result {
                Ok(events) => {
                    self.status = format!("{} events", events.len());
                    self.events = events;
                }
                Err(e) => self.status = e,
            }
        }
        let url = cfg.ics_url.trim().to_string();
        if url.is_empty() {
            if !self.events.is_empty() {
                self.events.clear();
            }
            self.loaded_url.clear();
            self.status.clear();
            return;
        }
        let stale = self.last_fetch.is_none_or(|t| t.elapsed() > Duration::from_secs(15 * 60));
        if !self.fetching && (url != self.loaded_url || stale) {
            self.fetching = true;
            self.loaded_url = url.clone();
            self.last_fetch = Some(Instant::now());
            self.status = "Syncing…".into();
            let _ = self.req.send(url);
        }
    }

    fn events_on(&self, day: NaiveDate) -> Vec<&Event> {
        let mut v: Vec<&Event> = self.events.iter().filter(|e| e.date <= day && day < e.end.max(e.date + CDuration::days(1))).collect();
        v.sort_by_key(|e| e.time);
        v
    }
}

// --------------------------------------------------------------- render --

struct Palette {
    today_fill: Color32,
    today_ink: Color32,
    ink: Color32,
    dim: Color32,
    faint: Color32,
    hover: Color32,
    dot: Color32,
}

fn month_grid(app: &mut App, ui: &mut Ui, rect: Rect, pal: &Palette, compact: bool) {
    let cfg_week = app.calendar.week_starts_on;
    let st = &mut app.cal;
    let p = ui.painter().clone();
    let today = Local::now().date_naive();
    let head_h = if compact { 18.0 } else { 22.0 };
    let names = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
    let start_idx = (cfg_week + 6) % 7; // 1 = Monday -> 0
    let cw = rect.width() / 7.0;
    for i in 0..7 {
        let name = names[(start_idx as usize + i) % 7];
        p.text(pos2(rect.min.x + cw * (i as f32 + 0.5), rect.min.y + head_h / 2.0), Align2::CENTER_CENTER, name, font(11.5), pal.faint);
    }
    let first = st.view;
    let offset = (first.weekday().num_days_from_monday() as i64 - start_idx as i64).rem_euclid(7);
    let grid_start = first - CDuration::days(offset);
    let ch = (rect.height() - head_h) / 6.0;
    let rad = (cw.min(ch) * 0.42).max(8.0);
    for k in 0..42 {
        let day = grid_start + CDuration::days(k);
        let cell = Rect::from_min_size(
            pos2(rect.min.x + (k % 7) as f32 * cw, rect.min.y + head_h + (k / 7) as f32 * ch),
            vec2(cw, ch),
        );
        let resp = ui.interact(cell, Id::new(("day", day)), Sense::click());
        let center = cell.center();
        let in_month = day.month() == first.month();
        if day == today {
            p.circle_filled(center, rad, pal.today_fill);
        } else if day == st.selected {
            p.circle_stroke(center, rad, Stroke::new(1.3_f32, pal.dim));
        } else if resp.hovered() {
            p.circle_filled(center, rad, pal.hover);
        }
        let color = if day == today { pal.today_ink } else if in_month { pal.ink } else { pal.faint };
        p.text(center, Align2::CENTER_CENTER, day.day().to_string(), font((rad * 0.85).clamp(10.0, 15.0)), color);
        if !st.events_on(day).is_empty() {
            p.circle_filled(pos2(center.x, center.y + rad * 0.72), 2.0, if day == today { pal.today_ink } else { pal.dot });
        }
        if resp.clicked() {
            st.selected = day;
            if !in_month {
                st.view = day.with_day(1).unwrap();
            }
        }
    }
}

fn nav(app: &mut App, ui: &mut Ui, title: &str, title_font: egui::FontId, ink: Color32, dim: Color32, look: &Look) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).font(title_font).color(ink));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut l = look.clone();
            l.ink.dim = dim;
            l.ink.ink = ink;
            if crate::app::icon_button(ui, glyph::RIGHT, 11.0, &l, "Next month").clicked() {
                app.cal.view = add_months(app.cal.view, 1).unwrap_or(app.cal.view);
            }
            let today = Local::now().date_naive();
            if app.cal.view != today.with_day(1).unwrap() || app.cal.selected != today {
                if ui.add(egui::Button::new(egui::RichText::new("Today").size(12.0).color(dim)).frame(false)).clicked() {
                    app.cal.view = today.with_day(1).unwrap();
                    app.cal.selected = today;
                }
            }
            if crate::app::icon_button(ui, glyph::LEFT, 11.0, &l, "Previous month").clicked() {
                app.cal.view = add_months(app.cal.view, -1).unwrap_or(app.cal.view);
            }
        });
    });
}


pub fn show(app: &mut App, ui: &mut Ui, look: &Look) {
    let design = app.state.settings.design("calendar");
    match design.as_str() {
        "almanac" => almanac(app, ui, look),
        "blocks" | "pop" | "prism" | "grove" => blocks(app, ui, look, &design),
        _ => lumen(app, ui, look),
    }
}

/// Lumen's default calendar: "‹ November 2026 ›  Today", spaced weekday
/// caps over a hairline, roomy day cells and a rounded tile for today.
/// The classic surface (Glass / Paper / Midnight) comes from calendar.json.
fn lumen(app: &mut App, ui: &mut Ui, look: &Look) {
    let r = ui.max_rect();
    let surface = app.calendar.theme.clone();
    let (ink, dim, faint, hair, hover, today_fill, today_edge) = match surface.as_str() {
        "paper" => {
            let plate = r.expand2(vec2(17.0, 15.0));
            ui.painter().rect_filled(plate, CornerRadius::same(33), rgb(0xeceef3));
            let ink = rgb(0x22252e);
            (ink, ink.gamma_multiply(0.7), ink.gamma_multiply(0.38), Color32::from_black_alpha(22), Color32::from_black_alpha(14), rgb(0xd9435a), rgb(0xd9435a))
        }
        "midnight" => {
            let plate = r.expand2(vec2(17.0, 15.0));
            ui.painter().rect_filled(plate, CornerRadius::same(33), rgb(0x191919));
            (Color32::WHITE, crate::theme::white(170), crate::theme::white(80), crate::theme::white(22), crate::theme::white(16), rgb(0xd9435a), rgb(0xd9435a))
        }
        _ => (
            look.ink.ink,
            look.ink.dim,
            look.ink.faint,
            look.ink.hair,
            look.ink.hover,
            crate::theme::css(70, 16, 52, 200),
            crate::theme::white(40),
        ),
    };
    let p = ui.painter().clone();
    let today = Local::now().date_naive();

    // Header.
    let head = Rect::from_min_size(r.min + vec2(0.0, 4.0), vec2(r.width(), 30.0));
    let arrow = |ui: &mut Ui, rect: Rect, left: bool, idn: &str| -> bool {
        let resp = ui.interact(rect, Id::new(idn), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(rect, CornerRadius::same(8), hover);
        }
        let c = rect.center();
        let (a1, a2, a3) = if left {
            (c + vec2(2.5, -5.0), c + vec2(-2.5, 0.0), c + vec2(2.5, 5.0))
        } else {
            (c + vec2(-2.5, -5.0), c + vec2(2.5, 0.0), c + vec2(-2.5, 5.0))
        };
        ui.painter().line_segment([a1, a2], Stroke::new(1.5_f32, dim));
        ui.painter().line_segment([a2, a3], Stroke::new(1.5_f32, dim));
        resp.clicked()
    };
    let prev = Rect::from_min_size(head.min, vec2(26.0, 30.0));
    if arrow(ui, prev, true, "cal-prev") {
        app.cal.view = add_months(app.cal.view, -1).unwrap_or(app.cal.view);
    }
    let month = app.cal.view.format("%B").to_string();
    let mg = ui.fonts(|f| f.layout_no_wrap(month.clone(), semibold(17.0), ink));
    let mx = prev.max.x + 8.0;
    let mw = mg.size().x;
    p.galley(pos2(mx, head.center().y - mg.size().y / 2.0), mg, ink);
    p.text(pos2(mx + mw + 6.0, head.center().y + 2.0), Align2::LEFT_CENTER, app.cal.view.format("%Y").to_string(), semibold(11.5), dim);
    let pill = Rect::from_min_size(pos2(head.max.x - 54.0, head.min.y + 3.0), vec2(54.0, 24.0));
    let next = Rect::from_min_size(pos2(pill.min.x - 34.0, head.min.y), vec2(26.0, 30.0));
    if arrow(ui, next, false, "cal-next") {
        app.cal.view = add_months(app.cal.view, 1).unwrap_or(app.cal.view);
    }
    let pr = ui.interact(pill, Id::new("cal-today"), Sense::click());
    p.rect_filled(pill, CornerRadius::same(12), if pr.hovered() { hover.gamma_multiply(1.0) } else { hover });
    p.rect_stroke(pill, CornerRadius::same(12), Stroke::new(1.0_f32, hair), egui::StrokeKind::Inside);
    p.text(pill.center(), Align2::CENTER_CENTER, "Today", semibold(11.0), ink);
    if pr.clicked() {
        app.cal.view = today.with_day(1).unwrap();
        app.cal.selected = today;
    }

    // Weekday caps + hairline.
    let names = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"];
    let start_idx = (app.calendar.week_starts_on + 6) % 7;
    let wk_y = head.max.y + 14.0;
    let cw = r.width() / 7.0;
    for i in 0..7 {
        let name = names[(start_idx as usize + i) % 7];
        p.text(pos2(r.min.x + cw * (i as f32 + 0.5), wk_y), Align2::CENTER_CENTER, name, semibold(10.0), dim);
    }
    p.line_segment([pos2(r.min.x + 4.0, wk_y + 12.0), pos2(r.max.x - 4.0, wk_y + 12.0)], Stroke::new(1.0_f32, hair));

    // Days.
    let first = app.cal.view;
    let offset = (first.weekday().num_days_from_monday() as i64 - start_idx as i64).rem_euclid(7);
    let grid_start = first - CDuration::days(offset);
    let top = wk_y + 18.0;
    let ch = ((r.max.y - top) / 6.0).max(18.0);
    for k in 0..42 {
        let day = grid_start + CDuration::days(k);
        let cell = Rect::from_min_size(pos2(r.min.x + (k % 7) as f32 * cw, top + (k / 7) as f32 * ch), vec2(cw, ch));
        let resp = ui.interact(cell, Id::new(("cal-day", day)), Sense::click());
        let in_month = day.month() == first.month();
        let tile = Rect::from_center_size(cell.center(), vec2((cw - 6.0).min(46.0), (ch - 4.0).min(62.0)));
        if day == today {
            p.rect_filled(tile, CornerRadius::same(12), today_fill);
            p.rect_stroke(tile, CornerRadius::same(12), Stroke::new(1.0_f32, today_edge), egui::StrokeKind::Inside);
        } else if day == app.cal.selected {
            p.rect_stroke(tile, CornerRadius::same(12), Stroke::new(1.0_f32, hair), egui::StrokeKind::Inside);
        } else if resp.hovered() {
            p.rect_filled(tile, CornerRadius::same(12), hover);
        }
        let color = if day == today {
            Color32::WHITE
        } else if in_month {
            ink
        } else {
            faint
        };
        let f = if day == today { semibold(13.5) } else { semibold(13.0) };
        p.text(cell.center(), Align2::CENTER_CENTER, day.day().to_string(), f, color);
        if !app.cal.events_on(day).is_empty() {
            p.circle_filled(pos2(cell.center().x, cell.center().y + 12.0), 2.0, if day == today { Color32::WHITE } else { look.ink.accent });
        }
        if resp.clicked() {
            app.cal.selected = day;
            if !in_month {
                app.cal.view = day.with_day(1).unwrap();
            }
        }
    }
}

fn almanac(app: &mut App, ui: &mut Ui, look: &Look) {
    let r = ui.max_rect();
    let p = ui.painter().clone();
    let ink = rgb(0x26231d);
    let red = rgb(0xc4453b);
    let sel = app.cal.selected;
    let left_w = (r.width() * 0.36).max(110.0);
    let left = Rect::from_min_size(r.min, vec2(left_w, r.height()));
    p.text(left.left_top() + vec2(0.0, 2.0), Align2::LEFT_TOP, sel.format("%A").to_string().to_uppercase(), semibold(12.0), red);
    let big = serif((left_w * 0.62).min(r.height() * 0.45));
    p.text(left.left_top() + vec2(-3.0, 18.0), Align2::LEFT_TOP, sel.day().to_string(), big.clone(), ink);
    p.text(left.left_top() + vec2(0.0, 22.0 + big.size * 1.05), Align2::LEFT_TOP, sel.format("%B %Y").to_string(), serif(15.0), ink.gamma_multiply(0.75));
    let ev = Rect::from_min_max(pos2(left.min.x, left.min.y + 52.0 + big.size * 1.05), left.max);
    let pal = Palette {
        today_fill: red,
        today_ink: Color32::WHITE,
        ink,
        dim: ink.gamma_multiply(0.7),
        faint: ink.gamma_multiply(0.38),
        hover: Color32::from_black_alpha(14),
        dot: red,
    };
    {
        let pe = p.with_clip_rect(ev);
        let mut y = ev.min.y;
        for e in app.cal.events_on(sel).iter().take(5) {
            let g = super::elided(ui, &e.title, font(12.0), ink, ev.width());
            pe.galley(pos2(ev.min.x, y), g, ink);
            y += 18.0;
        }
    }
    p.line_segment([pos2(left.max.x + 8.0, r.min.y), pos2(left.max.x + 8.0, r.max.y)], Stroke::new(1.0_f32, Color32::from_black_alpha(30)));
    let right = Rect::from_min_max(pos2(left.max.x + 20.0, r.min.y), r.max);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(right));
    let title = app.cal.view.format("%B").to_string();
    nav(app, &mut child, &title, serif(17.0), ink, ink.gamma_multiply(0.6), look);
    let g = child.available_rect_before_wrap();
    month_grid(app, &mut child, g, &pal, true);
}

fn blocks(app: &mut App, ui: &mut Ui, look: &Look, design: &str) {
    let (tile, tile_ink, accent) = match design {
        "pop" => (rgb(0x2f5bff), Color32::WHITE, rgb(0xffd23f)),
        "prism" => (rgb(0xff6fb5), Color32::WHITE, rgb(0xff9bd0)),
        "grove" => (rgb(0x1f3a2c), rgb(0xc8f560), rgb(0xc8f560)),
        _ => (rgb(0xff7a1a), Color32::WHITE, rgb(0xff7a1a)),
    };
    let r = ui.max_rect();
    let p = ui.painter().clone();
    let today = Local::now().date_naive();
    let tile_w = (r.width() * 0.34).clamp(100.0, 180.0);
    let tile_rect = Rect::from_min_size(r.min, vec2(tile_w, r.height()));
    p.rect_filled(tile_rect, CornerRadius::same(18), tile);
    if design == "prism" {
        p.rect_stroke(tile_rect, CornerRadius::same(18), Stroke::new(1.0_f32, crate::theme::white(120)), egui::StrokeKind::Inside);
    }
    let sel = app.cal.selected;
    p.text(tile_rect.left_top() + vec2(14.0, 12.0), Align2::LEFT_TOP, sel.format("%a").to_string().to_uppercase(), semibold(14.0), tile_ink.gamma_multiply(0.85));
    let big = semibold((tile_w * 0.55).min(r.height() * 0.42));
    p.text(pos2(tile_rect.min.x + 12.0, tile_rect.center().y), Align2::LEFT_CENTER, sel.day().to_string(), big, tile_ink);
    let n = app.cal.events_on(sel).len();
    let foot = if n == 0 { sel.format("%B").to_string() } else { format!("{n} event{}", if n == 1 { "" } else { "s" }) };
    p.text(tile_rect.left_bottom() + vec2(14.0, -12.0), Align2::LEFT_BOTTOM, foot, font(13.0), tile_ink.gamma_multiply(0.85));
    if design == "grove" {
        let days_in = days_in_month(today);
        let frac = today.day() as f32 / days_in as f32;
        let bar = Rect::from_min_size(tile_rect.left_bottom() + vec2(14.0, -34.0), vec2(tile_w - 28.0, 6.0));
        p.rect_filled(bar, CornerRadius::same(3), crate::theme::white(30));
        p.rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * frac, 6.0)), CornerRadius::same(3), accent);
    }
    let right = Rect::from_min_max(pos2(tile_rect.max.x + 14.0, r.min.y), r.max);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(right));
    let title = app.cal.view.format("%B %Y").to_string();
    nav(app, &mut child, &title, semibold(16.0), look.ink.ink, look.ink.dim, look);
    let g = child.available_rect_before_wrap();
    let pal = Palette {
        today_fill: accent,
        today_ink: if design == "pop" || design == "grove" { rgb(0x101010) } else { Color32::WHITE },
        ink: look.ink.ink,
        dim: look.ink.dim,
        faint: look.ink.faint,
        hover: look.ink.hover,
        dot: accent,
    };
    month_grid(app, &mut child, g, &pal, true);
    let _ = Pos2::ZERO;
}

fn days_in_month(d: NaiveDate) -> u32 {
    let next = add_months(d.with_day(1).unwrap(), 1).unwrap();
    (next - d.with_day(1).unwrap()).num_days() as u32
}

