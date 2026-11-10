//! Weather from Open-Meteo (no API key). The city comes from settings; when
//! it is empty the location is looked up from the public IP address.

use egui::{Align2, Color32, Painter, Pos2, Rect, Stroke, Ui, pos2, vec2};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use super::{caps, dot_header, font, light, semibold};
use crate::app::App;
use crate::config::Extras;
use crate::glass::Look;
use crate::theme::rgb;
pub use crate::weather_api::{Weather, describe};
use crate::weather_api::fetch;

pub struct WeatherState {
    pub data: Option<Weather>,
    pub status: String,
    key: String,
    last: Option<Instant>,
    fetching: bool,
    tx: Sender<(String, bool)>,
    rx: Receiver<(String, Result<Weather, String>)>,
}

impl WeatherState {
    pub fn new(ctx: &egui::Context) -> Self {
        let (tx, jobs) = channel::<(String, bool)>();
        let (done, rx) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            while let Ok((city, fahrenheit)) = jobs.recv() {
                let key = format!("{city}|{fahrenheit}");
                let result = fetch(&city, fahrenheit);
                if done.send((key, result)).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        WeatherState { data: None, status: String::new(), key: String::new(), last: None, fetching: false, tx, rx }
    }

    pub fn refresh(&mut self) {
        self.last = None;
    }

    pub fn pump(&mut self, extras: &Extras, active: bool) {
        while let Ok((key, result)) = self.rx.try_recv() {
            self.fetching = false;
            if key != self.key {
                continue;
            }
            match result {
                Ok(w) => {
                    self.status.clear();
                    self.data = Some(w);
                }
                Err(e) => self.status = e,
            }
        }
        if !active || self.fetching {
            return;
        }
        let key = format!("{}|{}", extras.weather_city.trim(), extras.weather_fahrenheit);
        let stale = self.last.is_none_or(|t| t.elapsed() > Duration::from_secs(20 * 60));
        if key != self.key || stale {
            if key != self.key {
                self.data = None;
            }
            self.key = key;
            self.last = Some(Instant::now());
            self.fetching = true;
            self.status = "Updating…".into();
            let _ = self.tx.send((extras.weather_city.trim().to_string(), extras.weather_fahrenheit));
        }
    }
}

/// Simple vector weather icons: sun/moon, clouds, rain, snow, storm, fog.
pub fn icon(p: &Painter, c: Pos2, s: f32, code: i32, day: bool) {
    let sun = rgb(0xffc94a);
    let cloud = rgb(0xe9ecf4);
    let cloud_dark = rgb(0x9aa3b5);
    let draw_sun = |c: Pos2, r: f32| {
        if day {
            for k in 0..8 {
                let a = k as f32 / 8.0 * std::f32::consts::TAU;
                let d = vec2(a.cos(), a.sin());
                p.line_segment([c + d * r * 1.35, c + d * r * 1.75], Stroke::new((r * 0.18).max(1.2), sun));
            }
            p.circle_filled(c, r, sun);
        } else {
            p.circle_filled(c, r, rgb(0xdfe3f0));
            p.circle_filled(c + vec2(r * 0.45, -r * 0.3), r * 0.85, rgb(0x2a2740));
        }
    };
    let draw_cloud = |c: Pos2, w: f32, col: Color32| {
        p.circle_filled(c + vec2(-w * 0.28, w * 0.05), w * 0.26, col);
        p.circle_filled(c + vec2(w * 0.05, -w * 0.12), w * 0.34, col);
        p.circle_filled(c + vec2(w * 0.33, w * 0.08), w * 0.22, col);
        p.rect_filled(Rect::from_min_max(c + vec2(-w * 0.28, w * 0.05), c + vec2(w * 0.33, w * 0.31)), egui::CornerRadius::ZERO, col);
    };
    match code {
        0 => draw_sun(c, s * 0.32),
        1 | 2 => {
            draw_sun(c + vec2(-s * 0.15, -s * 0.15), s * 0.24);
            draw_cloud(c + vec2(s * 0.08, s * 0.1), s * 0.8, cloud);
        }
        3 => draw_cloud(c, s, cloud),
        45 | 48 => {
            for k in 0..3 {
                let y = c.y - s * 0.2 + k as f32 * s * 0.2;
                p.line_segment([pos2(c.x - s * 0.4, y), pos2(c.x + s * 0.4, y)], Stroke::new(s * 0.08, cloud_dark));
            }
        }
        71..=77 | 85 | 86 => {
            draw_cloud(c - vec2(0.0, s * 0.12), s * 0.9, cloud);
            for k in 0..3 {
                p.circle_filled(pos2(c.x - s * 0.25 + k as f32 * s * 0.25, c.y + s * 0.38), s * 0.06, Color32::WHITE);
            }
        }
        95..=99 => {
            draw_cloud(c - vec2(0.0, s * 0.12), s * 0.9, cloud_dark);
            let pts = vec![c + vec2(0.02 * s, 0.1 * s), c + vec2(-0.1 * s, 0.32 * s), c + vec2(0.04 * s, 0.32 * s), c + vec2(-0.08 * s, 0.52 * s)];
            p.add(egui::Shape::line(pts, Stroke::new(s * 0.06, sun)));
        }
        51..=67 | 80..=82 => {
            draw_cloud(c - vec2(0.0, s * 0.12), s * 0.9, cloud);
            for k in 0..3 {
                let x = c.x - s * 0.25 + k as f32 * s * 0.25;
                p.line_segment([pos2(x, c.y + s * 0.28), pos2(x - s * 0.06, c.y + s * 0.46)], Stroke::new(s * 0.05, rgb(0x7fb6ff)));
            }
        }
        _ => draw_cloud(c, s, cloud),
    }
}

pub fn show(app: &mut App, ui: &mut Ui, look: &Look) {
    match app.state.settings.design("weather").as_str() {
        "minimal" => show_minimal(app, ui, look),
        "forecast" => show_forecast(app, ui, look),
        _ => show_lumen(app, ui, look),
    }
}

/// Status line while there is no data yet. Returns the data when there is.
fn data_or_status(app: &App, ui: &Ui, look: &Look) -> Option<Weather> {
    if let Some(w) = app.weather.data.clone() {
        return Some(w);
    }
    let p = ui.painter();
    let body = dot_header(p, ui.max_rect(), "Weather", "", look);
    let msg = if app.weather.status.is_empty() { "Loading forecast…" } else { app.weather.status.as_str() };
    p.text(body.center(), Align2::CENTER_CENTER, msg, font(12.5), look.ink.dim);
    None
}

/// Minimal: the icon, one big temperature and a quiet line of detail.
fn show_minimal(app: &mut App, ui: &mut Ui, look: &Look) {
    let Some(w) = data_or_status(app, ui, look) else { return };
    let r = ui.max_rect();
    let p = ui.painter().clone();
    let big = light((r.height() * 0.42).clamp(36.0, 110.0));
    let c = r.center();
    let isz = (big.size * 0.5).max(26.0);
    icon(&p, pos2(c.x, r.min.y + isz * 0.55), isz, w.code, w.is_day);
    let temp_y = r.min.y + isz * 1.1 + big.size * 0.5;
    p.text(pos2(c.x, temp_y), Align2::CENTER_CENTER, format!("{:.0}°", w.temp), big.clone(), look.ink.ink);
    let line = format!("{} · {}", describe(w.code), w.place);
    p.text(pos2(c.x, temp_y + big.size * 0.62), Align2::CENTER_CENTER, line, semibold(12.5), look.ink.ink);
    p.text(
        pos2(c.x, temp_y + big.size * 0.62 + 20.0),
        Align2::CENTER_CENTER,
        format!("H {:.0}°   L {:.0}°   ·   {:.0}%   ·   {:.0} km/h", w.hi, w.lo, w.humidity, w.wind),
        font(11.5),
        look.ink.dim,
    );
}

/// Forecast: one row per day with a range bar scaled to the whole week.
fn show_forecast(app: &mut App, ui: &mut Ui, look: &Look) {
    let Some(w) = data_or_status(app, ui, look) else { return };
    let r = ui.max_rect();
    let p = ui.painter().clone();
    let now = format!("{:.0}° · {}", w.temp, describe(w.code));
    let body = dot_header(&p, r, &w.place, &now, look);
    let days = &w.daily[..w.daily.len().min(6)];
    if days.is_empty() {
        return;
    }
    let lo = days.iter().map(|d| d.2).fold(f32::INFINITY, f32::min);
    let hi = days.iter().map(|d| d.1).fold(f32::NEG_INFINITY, f32::max);
    let span = (hi - lo).max(1.0);
    let row_h = (body.height() / days.len() as f32).clamp(22.0, 44.0);
    let bar_x0 = body.min.x + 130.0;
    let bar_x1 = body.max.x - 40.0;
    for (i, d) in days.iter().enumerate() {
        let y = body.min.y + row_h * (i as f32 + 0.5);
        let label = if i == 0 { "Today".to_string() } else { d.0.clone() };
        p.text(pos2(body.min.x, y), Align2::LEFT_CENTER, label, semibold(12.0), look.ink.ink);
        icon(&p, pos2(body.min.x + 64.0, y), (row_h * 0.7).min(24.0), d.3, true);
        p.text(pos2(bar_x0 - 8.0, y), Align2::RIGHT_CENTER, format!("{:.0}°", d.2), font(11.5), look.ink.dim);
        p.text(pos2(body.max.x, y), Align2::RIGHT_CENTER, format!("{:.0}°", d.1), semibold(11.5), look.ink.ink);
        let track = Rect::from_min_max(pos2(bar_x0, y - 2.5), pos2(bar_x1, y + 2.5));
        p.rect_filled(track, egui::CornerRadius::same(3), crate::theme::white(22));
        let a = bar_x0 + (d.2 - lo) / span * (bar_x1 - bar_x0);
        let b = (bar_x0 + (d.1 - lo) / span * (bar_x1 - bar_x0)).max(a + 4.0);
        let mut mesh = egui::epaint::Mesh::default();
        let (c0, c1) = (rgb(0x60c3f6), rgb(0xffa24a));
        for (pos, col) in [(pos2(a, y - 2.5), c0), (pos2(b, y - 2.5), c1), (pos2(b, y + 2.5), c1), (pos2(a, y + 2.5), c0)] {
            mesh.vertices.push(egui::epaint::Vertex { pos, uv: egui::epaint::WHITE_UV, color: col });
        }
        mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
        p.add(egui::Shape::mesh(mesh));
        if i == 0 {
            let t = bar_x0 + (w.temp - lo).clamp(0.0, span) / span * (bar_x1 - bar_x0);
            p.circle_filled(pos2(t, y), 4.0, Color32::WHITE);
        }
    }
}

fn show_lumen(app: &mut App, ui: &mut Ui, look: &Look) {
    let r = ui.max_rect();
    let p = ui.painter().clone();
    let Some(w) = app.weather.data.clone() else {
        let body = dot_header(&p, r, "Weather", "", look);
        let msg = if app.weather.status.is_empty() { "Loading forecast…" } else { app.weather.status.as_str() };
        p.text(body.center(), Align2::CENTER_CENTER, msg, font(12.5), look.ink.dim);
        return;
    };
    let body = dot_header(&p, r, &w.place, &caps(describe(w.code)), look);
    let big = light((body.height() * 0.42).clamp(30.0, 72.0));
    let temp = format!("{:.0}°", w.temp);
    p.text(pos2(body.min.x - 2.0, body.min.y), Align2::LEFT_TOP, &temp, big.clone(), look.ink.ink);
    let line2 = format!("H {:.0}°  L {:.0}°   ·   {:.0}% humidity   ·   wind {:.0} km/h", w.hi, w.lo, w.humidity, w.wind);
    let g = super::elided(ui, &line2, font(11.5), look.ink.dim, body.width());
    p.galley(pos2(body.min.x, body.min.y + big.size * 1.08), g, look.ink.dim);
    let isz = (big.size * 1.2).min(body.width() * 0.3);
    icon(&p, pos2(body.max.x - isz * 0.55, body.min.y + big.size * 0.55), isz, w.code, w.is_day);

    // Forecast strip: hours when short, days when there is room.
    let strip_top = body.min.y + big.size * 1.08 + 26.0;
    let strip = Rect::from_min_max(pos2(body.min.x, strip_top), body.max);
    if strip.height() < 46.0 {
        return;
    }
    p.line_segment([strip.left_top(), strip.right_top()], Stroke::new(1.0_f32, look.ink.hair));
    let use_days = strip.height() > 110.0;
    let n = if use_days { w.daily.len().min(6) } else { w.hourly.len().min(6) };
    if n == 0 {
        return;
    }
    let cw = strip.width() / n as f32;
    for i in 0..n {
        let cx = strip.min.x + cw * (i as f32 + 0.5);
        let (label, temp_text, code, day) = if use_days {
            let d = &w.daily[i];
            (if i == 0 { "Today".to_string() } else { d.0.clone() }, format!("{:.0}° / {:.0}°", d.1, d.2), d.3, true)
        } else {
            let h = &w.hourly[i];
            let hour: u32 = h.0.parse().unwrap_or(12);
            (format!("{}:00", h.0), format!("{:.0}°", h.1), h.2, (6..18).contains(&hour))
        };
        p.text(pos2(cx, strip.min.y + 12.0), Align2::CENTER_CENTER, label, semibold(10.5), look.ink.dim);
        icon(&p, pos2(cx, strip.min.y + 36.0), 26.0, code, day);
        p.text(pos2(cx, strip.min.y + 62.0), Align2::CENTER_CENTER, temp_text, font(11.5), look.ink.ink);
    }
}
