//! Clock widget with Lumen's nine faces.

use chrono::{Datelike, NaiveDateTime, Timelike};
use egui::{Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Shape, Stroke, Ui, pos2, vec2};
use std::f32::consts::{PI, TAU};

use super::{font, light, semibold};
use crate::app::App;
use crate::glass::Look;
use crate::theme::rgb;

pub fn now_in(zone: &str) -> NaiveDateTime {
    if zone != "local" {
        if let Ok(tz) = zone.parse::<chrono_tz::Tz>() {
            return chrono::Utc::now().with_timezone(&tz).naive_local();
        }
    }
    chrono::Local::now().naive_local()
}

struct T {
    h: u32,
    m: u32,
    s: u32,
    hour12: bool,
    seconds: bool,
    now: NaiveDateTime,
}

impl T {
    fn hh(&self) -> String {
        if self.hour12 {
            let h = self.h % 12;
            format!("{}", if h == 0 { 12 } else { h })
        } else {
            format!("{:02}", self.h)
        }
    }
    fn mm(&self) -> String {
        format!("{:02}", self.m)
    }
    fn ss(&self) -> String {
        format!("{:02}", self.s)
    }
    fn ampm(&self) -> &'static str {
        if self.h < 12 { "AM" } else { "PM" }
    }
    fn hm(&self) -> String {
        format!("{}:{}", self.hh(), self.mm())
    }
    fn date_long(&self) -> String {
        self.now.format("%A, %-d %B").to_string()
    }
    fn day_fraction(&self) -> f32 {
        (self.h * 3600 + self.m * 60 + self.s) as f32 / 86400.0
    }
}

pub fn show(app: &mut App, ui: &mut Ui, look: &Look) {
    let c = &app.state.settings.clock;
    let now = now_in(&c.time_zone);
    let t = T { h: now.hour(), m: now.minute(), s: now.second(), hour12: c.hour12, seconds: c.show_seconds, now };
    let rect = ui.max_rect();
    let p = ui.painter().clone();
    match c.theme.as_str() {
        "sunset" => afterglow(&p, rect, &t, look),
        "paper" => paper(&p, rect, &t),
        "neon" => nightfall(&p, rect, &t, look),
        "modular" => colour_block(&p, rect, &t),
        "ocean" => {
            let body = zone_header(&p, rect, &t, &c.time_zone, look);
            split_flap(&p, body, &t, look)
        }
        "rose" => {
            let body = zone_header(&p, rect, &t, &c.time_zone, look);
            daybook(&p, body, &t, look)
        }
        "aurora" => timeline(&p, rect, &t, look),
        "terminal" => terminal(&p, rect, &t),
        _ => lumen(&p, rect, &t, look),
    }
    if c.time_zone != "local" && !matches!(c.theme.as_str(), "ocean" | "rose") {
        p.text(rect.right_top(), Align2::RIGHT_TOP, &c.time_zone, font(11.0), look.ink.faint);
    }
}

fn text_w(p: &Painter, s: &str, f: &FontId) -> f32 {
    p.layout_no_wrap(s.to_string(), f.clone(), Color32::WHITE).size().x
}

/// Biggest font size (up to `max`) so `s` fits in `width`.
fn fit(p: &Painter, s: &str, font: impl Fn(f32) -> FontId, max: f32, width: f32) -> FontId {
    let probe = font(100.0);
    let w = text_w(p, s, &probe).max(1.0);
    font((width / w * 100.0).min(max).max(8.0))
}

fn lumen(p: &Painter, r: Rect, t: &T, look: &Look) {
    let show_dial = r.width() > r.height() * 1.25 && r.height() > 120.0;
    let dial_r = (r.height() * 0.36).min(r.width() * 0.2);
    let text_area = if show_dial { Rect::from_min_max(r.min, pos2(r.max.x - dial_r * 2.0 - 24.0, r.max.y)) } else { r };
    let main = t.hm();
    let f = fit(p, &main, light, r.height() * 0.42, text_area.width() * if t.seconds { 0.78 } else { 0.95 });
    let size = p.layout_no_wrap(main.clone(), f.clone(), look.ink.ink).size();
    let y = text_area.center().y - size.y * 0.15;
    let x = text_area.min.x + 4.0;
    p.text(pos2(x, y), Align2::LEFT_CENTER, &main, f.clone(), look.ink.ink);
    let mut after = x + size.x + 6.0;
    if t.seconds {
        let sf = light(f.size * 0.38);
        p.text(pos2(after, y - size.y * 0.2), Align2::LEFT_CENTER, t.ss(), sf.clone(), look.ink.dim);
        after += text_w(p, "00", &sf);
    }
    if t.hour12 {
        p.text(pos2(after - if t.seconds { text_w(p, "00", &light(f.size * 0.38)) } else { 0.0 }, y + size.y * 0.2), Align2::LEFT_CENTER, t.ampm(), semibold(f.size * 0.2), look.ink.dim);
    }
    p.text(pos2(x + 2.0, y + size.y * 0.55), Align2::LEFT_TOP, t.date_long(), font((f.size * 0.2).clamp(12.0, 22.0)), look.ink.dim);
    if show_dial {
        let c = pos2(r.max.x - dial_r - 6.0, r.center().y);
        p.circle_filled(c, dial_r, look.ink.hover);
        p.circle_stroke(c, dial_r, Stroke::new(1.0_f32, look.ink.hair));
        dial_hands(p, c, dial_r, t, look.ink.ink, look.ink.accent, false);
    }
}

fn hand(p: &Painter, c: Pos2, angle: f32, len: f32, back: f32, width: f32, color: Color32) {
    let d = vec2(angle.sin(), -angle.cos());
    p.line_segment([c - d * back, c + d * len], Stroke::new(width, color));
}

fn dial_hands(p: &Painter, c: Pos2, r: f32, t: &T, ink: Color32, accent: Color32, numbers: bool) {
    for i in 0..60 {
        let a = i as f32 / 60.0 * TAU;
        let d = vec2(a.sin(), -a.cos());
        let major = i % 5 == 0;
        let (inner, w) = if major { (r * 0.84, 2.0) } else { (r * 0.91, 1.0_f32) };
        if numbers || major {
            p.line_segment([c + d * inner, c + d * (r * 0.96)], Stroke::new(w, ink.gamma_multiply(if major { 0.9 } else { 0.35 })));
        }
    }
    if numbers {
        for (n, label) in [(0, "12"), (3, "3"), (6, "6"), (9, "9")] {
            let a = n as f32 / 12.0 * TAU;
            let d = vec2(a.sin(), -a.cos());
            p.text(c + d * r * 0.68, Align2::CENTER_CENTER, label, super::serif(r * 0.2), ink);
        }
    }
    let s = t.s as f32;
    let m = t.m as f32 + s / 60.0;
    let h = (t.h % 12) as f32 + m / 60.0;
    hand(p, c, h / 12.0 * TAU, r * 0.5, r * 0.08, (r * 0.06).max(2.5), ink);
    hand(p, c, m / 60.0 * TAU, r * 0.78, r * 0.08, (r * 0.04).max(2.0), ink);
    if t.seconds {
        hand(p, c, s / 60.0 * TAU, r * 0.86, r * 0.18, 1.3, accent);
    }
    p.circle_filled(c, (r * 0.05).max(3.0), accent);
}

fn afterglow(p: &Painter, r: Rect, t: &T, _look: &Look) {
    let sun_c = pos2(r.center().x, r.max.y + r.height() * 0.18);
    let sun_r = r.width().min(r.height() * 1.4) * 0.42;
    for k in 0..6 {
        let rr = sun_r * (1.0 + k as f32 * 0.16);
        p.circle_filled(sun_c, rr, crate::theme::css(255, 190, 120, (34 - k * 5) as u8));
    }
    p.circle_filled(sun_c, sun_r, crate::theme::css(255, 200, 140, 120));
    let main = t.hm();
    let f = fit(p, &main, light, r.height() * 0.4, r.width() * 0.8);
    let y = r.center().y - r.height() * 0.1;
    p.text(pos2(r.center().x, y), Align2::CENTER_CENTER, &main, f.clone(), Color32::WHITE);
    let sub = if t.seconds { format!("{} · {}", t.date_long(), t.ss()) } else { t.date_long() };
    p.text(pos2(r.center().x, y + f.size * 0.62), Align2::CENTER_CENTER, sub, font((f.size * 0.18).clamp(12.0, 20.0)), crate::theme::white(210));
}

fn paper(p: &Painter, r: Rect, t: &T) {
    let ink = rgb(0x2b2622);
    let rad = r.width().min(r.height()) * 0.44;
    let c = r.center();
    p.circle_filled(c + vec2(0.0, 3.0), rad, Color32::from_black_alpha(18));
    p.circle_filled(c, rad, rgb(0xfbf8f1));
    p.circle_stroke(c, rad, Stroke::new(1.0_f32, Color32::from_black_alpha(30)));
    dial_hands(p, c, rad * 0.94, t, ink, rgb(0xd0543c), true);
    if r.height() > 220.0 {
        let day = t.now.format("%a %-d").to_string().to_uppercase();
        p.text(c + vec2(0.0, rad * 0.36), Align2::CENTER_CENTER, day, semibold(rad * 0.09), ink.gamma_multiply(0.6));
    }
}

fn arc(p: &Painter, c: Pos2, r: f32, from: f32, to: f32, stroke: Stroke) {
    let steps = ((to - from).abs() / TAU * 120.0).ceil().max(2.0) as usize;
    let pts: Vec<Pos2> = (0..=steps)
        .map(|i| {
            let a = from + (to - from) * i as f32 / steps as f32;
            c + vec2(a.sin(), -a.cos()) * r
        })
        .collect();
    p.add(Shape::line(pts, stroke));
}

fn nightfall(p: &Painter, r: Rect, t: &T, look: &Look) {
    let c = r.center();
    let rad = r.width().min(r.height()) * 0.42;
    let glow = rgb(0x8be9ff);
    let violet = rgb(0xc59bff);
    p.circle_stroke(c, rad, Stroke::new(rad * 0.06, look.ink.hover));
    let frac = t.day_fraction();
    let end = frac * TAU;
    for k in 0..3 {
        arc(p, c, rad, 0.0, end, Stroke::new(rad * 0.06 + k as f32 * 4.0, glow.gamma_multiply(0.18 - k as f32 * 0.05)));
    }
    arc(p, c, rad, 0.0, end, Stroke::new(rad * 0.05, glow));
    let tip = c + vec2(end.sin(), -end.cos()) * rad;
    p.circle_filled(tip, rad * 0.07, violet);
    p.circle_filled(tip, rad * 0.04, Color32::WHITE);
    for (h, label) in [(0, "0"), (6, "6"), (12, "12"), (18, "18")] {
        let a = h as f32 / 24.0 * TAU;
        p.text(c + vec2(a.sin(), -a.cos()) * rad * 0.8, Align2::CENTER_CENTER, label, font(rad * 0.1), look.ink.faint);
    }
    let main = t.hm();
    let f = fit(p, &main, light, rad * 0.55, rad * 1.1);
    p.text(c, Align2::CENTER_CENTER, &main, f.clone(), look.ink.ink);
    if t.seconds {
        p.text(c + vec2(0.0, f.size * 0.6), Align2::CENTER_CENTER, t.ss(), font(rad * 0.12), glow);
    }
}

fn colour_block(p: &Painter, r: Rect, t: &T) {
    let gap = 8.0;
    let w = (r.width() - gap) / 2.0;
    let h = (r.height() - gap) / 2.0;
    let cells = [
        (rgb(0xff8a3d), t.hh(), if t.hour12 { t.ampm().to_string() } else { "HOUR".into() }),
        (rgb(0x4f7cff), t.mm(), if t.seconds { format!("SEC {}", t.ss()) } else { "MIN".into() }),
        (rgb(0x2fbf71), t.now.format("%a").to_string().to_uppercase(), t.now.format("%B").to_string().to_uppercase()),
        (rgb(0xf05b8c), format!("{}", t.now.day()), format!("WEEK {}", t.now.iso_week().week())),
    ];
    for (i, (color, big, small)) in cells.iter().enumerate() {
        let x = r.min.x + (i % 2) as f32 * (w + gap);
        let y = r.min.y + (i / 2) as f32 * (h + gap);
        let cell = Rect::from_min_size(pos2(x, y), vec2(w, h));
        p.rect_filled(cell, CornerRadius::same(16), *color);
        let f = fit(p, big, semibold, h * 0.55, w * 0.8);
        p.text(cell.left_top() + vec2(12.0, 8.0), Align2::LEFT_TOP, big, f, Color32::WHITE);
        p.text(cell.left_bottom() + vec2(12.0, -8.0), Align2::LEFT_BOTTOM, small, semibold((h * 0.13).clamp(9.0, 14.0)), crate::theme::white(220));
    }
}

/// "• Local time" (or the zone) header used by Lumen's clock faces.
/// Returns the rect left for the face.
fn zone_header(p: &Painter, r: Rect, t: &T, zone: &str, look: &Look) -> Rect {
    let _ = t;
    let label = if zone == "local" { "Local time".to_string() } else { zone.replace('_', " ") };
    p.circle_filled(pos2(r.min.x + 3.0, r.min.y + 8.0), 2.5, look.ink.dim);
    p.text(pos2(r.min.x + 12.0, r.min.y + 8.0), Align2::LEFT_CENTER, label, semibold(12.0), look.ink.ink);
    Rect::from_min_max(pos2(r.min.x, r.min.y + 26.0), r.max)
}

/// Lumen's Split flap: two dark flip cards with cream digits, a hinge
/// line, captions, the date and a dashed seconds meter.
fn split_flap(p: &Painter, r: Rect, t: &T, look: &Look) {
    let cream = rgb(0xf6ecd2);
    let caps = |s: &str| -> String { s.chars().flat_map(|c| [c, '\u{200A}']).collect() };
    p.text(r.left_top(), Align2::LEFT_TOP, caps(&t.now.format("%a / %-d").to_string().to_uppercase()), semibold(10.5), look.ink.ink);
    p.text(r.right_top(), Align2::RIGHT_TOP, if t.hour12 { t.ampm() } else { "24H" }, semibold(10.0), look.ink.dim);

    let meter_h = if t.seconds { 34.0 } else { 0.0 };
    let date_h = 22.0;
    let top = r.min.y + 20.0;
    let card_h = (r.max.y - top - date_h - meter_h - 8.0).max(40.0);
    let gap = 10.0;
    let card_w = (r.width() - gap) / 2.0;
    for (i, (text, cap)) in [(t.hh(), "HOURS"), (t.mm(), "MINUTES")].iter().enumerate() {
        let card = Rect::from_min_size(pos2(r.min.x + i as f32 * (card_w + gap), top), vec2(card_w, card_h));
        p.rect_filled(card.translate(vec2(0.0, 3.0)), CornerRadius::same(12), Color32::from_black_alpha(80));
        p.rect_filled(card, CornerRadius::same(12), rgb(0x1b1d23));
        let upper = Rect::from_min_max(card.min, pos2(card.max.x, card.center().y));
        p.rect_filled(upper, CornerRadius { nw: 12, ne: 12, sw: 0, se: 0 }, rgb(0x22252c));
        p.rect_stroke(card, CornerRadius::same(12), Stroke::new(1.0_f32, crate::theme::white(18)), egui::StrokeKind::Inside);
        let f = fit(p, text, semibold, card_h * 0.62, card_w * 0.78);
        p.text(card.center() - vec2(0.0, card_h * 0.06), Align2::CENTER_CENTER, text, f, cream);
        p.line_segment([pos2(card.min.x, card.center().y), pos2(card.max.x, card.center().y)], Stroke::new(1.5_f32, Color32::from_black_alpha(190)));
        p.text(pos2(card.center().x, card.max.y - 10.0), Align2::CENTER_BOTTOM, caps(cap), semibold(8.0), crate::theme::white(110));
    }
    let date = t.now.format("%B %-d, %Y").to_string();
    p.text(pos2(r.center().x, top + card_h + 12.0), Align2::CENTER_CENTER, date, font(12.0), look.ink.dim);
    if t.seconds {
        let y = r.max.y - meter_h + 6.0;
        p.text(pos2(r.min.x, y), Align2::LEFT_TOP, "Seconds", font(12.0), look.ink.dim);
        p.text(pos2(r.max.x, y), Align2::RIGHT_TOP, t.ss(), semibold(12.0), look.ink.ink);
        let n = 30usize;
        let gap = 3.0;
        let dw = (r.width() - gap * (n - 1) as f32) / n as f32;
        let lit = ((t.s as f32 / 60.0) * n as f32).round() as usize;
        for k in 0..n {
            let x = r.min.x + k as f32 * (dw + gap);
            let c = if k < lit { cream } else { crate::theme::white(30) };
            p.rect_filled(Rect::from_min_size(pos2(x, y + 20.0), vec2(dw, 3.0)), CornerRadius::same(1), c);
        }
    }
}

/// Lumen's Daybook: "DAY IN MOTION", big time, and three dotted meters.
fn daybook(p: &Painter, r: Rect, t: &T, look: &Look) {
    let title: String = "DAY IN MOTION".chars().flat_map(|c| [c, '\u{2009}']).collect();
    p.text(r.left_top(), Align2::LEFT_TOP, title, semibold(10.5), look.ink.dim);
    p.text(r.right_top(), Align2::RIGHT_TOP, t.now.format("%a %-d").to_string(), semibold(11.5), look.ink.dim);

    let main = if t.hour12 { format!("{} {}", t.hm(), t.ampm()) } else { t.hm() };
    let f = fit(p, &main, light, r.height() * 0.3, r.width() * 0.8);
    let time_top = r.min.y + 18.0;
    p.text(pos2(r.min.x - 2.0, time_top), Align2::LEFT_TOP, &main, f.clone(), look.ink.ink);

    let rows = [
        ("Day elapsed", t.day_fraction(), format!("{:.0}%", t.day_fraction() * 100.0), rgb(0xa3e635)),
        ("This hour", t.m as f32 / 60.0, format!("{} / 60 min", t.m), rgb(0x34d399)),
        ("This minute", t.s as f32 / 60.0, format!("{} / 60 sec", t.s), rgb(0x60c3f6)),
    ];
    let top = time_top + f.size * 1.08;
    let row_h = ((r.max.y - top) / 3.0).max(26.0);
    for (i, (name, frac, value, color)) in rows.iter().enumerate() {
        let y = top + i as f32 * row_h;
        p.text(pos2(r.min.x, y), Align2::LEFT_TOP, *name, font(12.0), look.ink.dim);
        p.text(pos2(r.max.x, y), Align2::RIGHT_TOP, value, semibold(12.0), look.ink.ink);
        // Segmented meter: small rounded dots lit up to `frac`.
        let n = ((r.width() / 11.0).floor() as usize).clamp(12, 40);
        let gap = 3.0;
        let dw = (r.width() - gap * (n - 1) as f32) / n as f32;
        let lit = (frac.clamp(0.0, 1.0) * n as f32).round() as usize;
        let dy = y + 20.0;
        for k in 0..n {
            let x = r.min.x + k as f32 * (dw + gap);
            let dot = Rect::from_min_size(pos2(x, dy), vec2(dw, 6.0));
            let c = if k < lit { *color } else { crate::theme::white(22) };
            p.rect_filled(dot, CornerRadius::same(2), c);
        }
    }
}

fn timeline(p: &Painter, r: Rect, t: &T, look: &Look) {
    let ruler_x = r.min.x + r.width() * 0.62;
    let center_y = r.center().y;
    let px_per_hour = (r.height() / 3.2).max(40.0);
    let now_h = t.h as f32 + t.m as f32 / 60.0 + t.s as f32 / 3600.0;
    let clip = p.with_clip_rect(r);
    for dh in -3..=3 {
        let hour = now_h.floor() as i32 + dh;
        for q in 0..4 {
            let v = hour as f32 + q as f32 * 0.25;
            let y = center_y + (v - now_h) * px_per_hour;
            if y < r.min.y - 10.0 || y > r.max.y + 10.0 {
                continue;
            }
            let len = if q == 0 { 26.0 } else { 12.0 };
            clip.line_segment([pos2(ruler_x, y), pos2(ruler_x + len, y)], Stroke::new(if q == 0 { 1.6 } else { 1.0_f32 }, look.ink.faint));
            if q == 0 {
                let label = ((hour % 24) + 24) % 24;
                let label = if t.hour12 { format!("{} {}", if label % 12 == 0 { 12 } else { label % 12 }, if label < 12 { "AM" } else { "PM" }) } else { format!("{label:02}:00") };
                clip.text(pos2(ruler_x + 34.0, y), Align2::LEFT_CENTER, label, font(12.0), look.ink.dim);
            }
        }
    }
    clip.line_segment([pos2(ruler_x, r.min.y), pos2(ruler_x, r.max.y)], Stroke::new(1.0_f32, look.ink.hair));
    let accent = rgb(0x7ee0c8);
    p.line_segment([pos2(r.min.x, center_y), pos2(r.max.x, center_y)], Stroke::new(1.5_f32, accent));
    p.circle_filled(pos2(ruler_x, center_y), 5.0, accent);
    let main = if t.seconds { format!("{}:{}", t.hm(), t.ss()) } else { t.hm() };
    let f = fit(p, &main, light, r.height() * 0.24, ruler_x - r.min.x - 16.0);
    p.text(pos2(r.min.x, center_y - 6.0), Align2::LEFT_BOTTOM, &main, f, look.ink.ink);
    p.text(pos2(r.min.x, center_y + 8.0), Align2::LEFT_TOP, t.date_long(), font(13.0), look.ink.dim);
}

fn terminal(p: &Painter, r: Rect, t: &T) {
    let green = rgb(0x4dff88);
    let dim = rgb(0x2a9c55);
    let mono = |s: f32| FontId::monospace(s);
    let line = (r.height() * 0.1).clamp(11.0, 16.0);
    p.text(r.left_top(), Align2::LEFT_TOP, "opal@desktop:~$ date", mono(line), dim);
    let main = if t.seconds { format!("{}:{}", t.hm(), t.ss()) } else { t.hm() };
    let main = if t.hour12 { format!("{main} {}", t.ampm()) } else { main };
    let f = fit(p, &main, |s| FontId::monospace(s), r.height() * 0.4, r.width() * 0.95);
    let y = r.min.y + line * 1.8;
    p.text(pos2(r.min.x, y), Align2::LEFT_TOP, &main, f.clone(), green);
    let date = t.now.format("%a %Y-%m-%d").to_string();
    p.text(pos2(r.min.x, y + f.size * 1.1), Align2::LEFT_TOP, &date, mono(line), green.gamma_multiply(0.8));
    if t.s % 2 == 0 {
        let cy = y + f.size * 1.1 + line * 1.6;
        p.text(pos2(r.min.x, cy), Align2::LEFT_TOP, "opal@desktop:~$", mono(line), dim);
        let w = text_w(p, "opal@desktop:~$ ", &mono(line));
        p.rect_filled(Rect::from_min_size(pos2(r.min.x + w, cy + 1.0), vec2(line * 0.55, line)), CornerRadius::ZERO, green);
    }
    let _ = PI;
}
