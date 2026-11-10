//! Per-widget settings panel ("Clock settings", "Sticky notes settings"…),
//! modelled on Lumen's: Design with a gallery, Background (frosted glass,
//! glass tint, frost strength, blur size, background opacity), a Surface
//! palette, then widget-specific options.

use egui::{Align2, Color32, CornerRadius, Id, Rect, RichText, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use super::{elided, font, semibold};
use crate::app::{App, glyph};
use crate::glass::Look;
use crate::theme;

#[derive(Default)]
pub struct WidgetSettingsUi {
    pub gallery_open: bool,
    pub palette_open: bool,
    /// Text being typed; applied on Enter or when the field loses focus so
    /// every keystroke doesn't start a network request.
    pub city_draft: Option<String>,
    pub ics_draft: Option<String>,
}

/// A text field that edits a draft and commits it on Enter / focus loss.
/// Returns the committed value when it changes.
pub fn commit_field(ui: &mut Ui, draft: &mut Option<String>, current: &str, hint: &str) -> Option<String> {
    let text = draft.get_or_insert_with(|| current.to_string());
    let resp = ui.add(egui::TextEdit::singleline(text).hint_text(hint).desired_width(f32::INFINITY));
    if resp.lost_focus() {
        let value = draft.take().unwrap_or_default();
        return (value.trim() != current.trim()).then(|| value.trim().to_string());
    }
    if !resp.has_focus() {
        // Not editing: follow the stored value.
        *draft = None;
    }
    None
}

pub fn widget_name(id: &str) -> &'static str {
    match id {
        "launcher" => "App Dock",
        "clock" => "Clock",
        "notes" => "Sticky notes",
        "calendar" => "Calendar",
        "music" => "Music",
        "weather" => "Weather",
        "system" => "System",
        "pomodoro" => "Focus timer",
        "photos" => "Photos",
        "lunar" => "Âm lịch",
        "todo" => "To-do",
        _ => "Widget",
    }
}

/// Spaced small caps section label, like Lumen's `DESIGN`, `BACKGROUND`.
fn section(ui: &mut Ui, text: &str, look: &Look) {
    ui.add_space(10.0);
    let spaced: String = text.to_uppercase().chars().flat_map(|c| [c, '\u{200A}']).collect();
    ui.label(RichText::new(spaced).font(semibold(10.0)).color(look.ink.faint));
    ui.add_space(2.0);
}

/// Rounded translucent card that groups rows.
fn card<R>(ui: &mut Ui, look: &Look, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .fill(crate::theme::white(9))
        .stroke(Stroke::new(1.0_f32, look.ink.hair))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// Lumen's slider row: label, faint value top-right, thin track, white knob,
/// bold value at the end.
pub fn glass_slider(ui: &mut Ui, label: &str, value: &mut u32, max: u32, suffix: &str, look: &Look) -> bool {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, 46.0), Sense::hover());
    let p = ui.painter().clone();
    p.text(rect.left_top() + vec2(0.0, 4.0), Align2::LEFT_TOP, label, font(12.5), look.ink.ink);
    p.text(rect.right_top() + vec2(0.0, 4.0), Align2::RIGHT_TOP, format!("{value}{suffix}"), font(10.0), look.ink.faint);
    let track = Rect::from_min_max(pos2(rect.min.x, rect.min.y + 30.0), pos2(rect.max.x - 40.0, rect.min.y + 34.0));
    let hit = track.expand2(vec2(8.0, 9.0));
    let resp = ui.interact(hit, Id::new(("slider", label, ui.id())), Sense::click_and_drag());
    let mut changed = false;
    if let Some(pt) = resp.interact_pointer_pos() {
        if resp.dragged() || resp.clicked() || resp.is_pointer_button_down_on() {
            let f = ((pt.x - track.min.x) / track.width()).clamp(0.0, 1.0);
            let v = (f * max as f32).round() as u32;
            if v != *value {
                *value = v;
                changed = true;
            }
        }
    }
    let f = *value as f32 / max.max(1) as f32;
    p.rect_filled(track, CornerRadius::same(2), crate::theme::white(30));
    p.rect_filled(Rect::from_min_size(track.min, vec2(track.width() * f, track.height())), CornerRadius::same(2), crate::theme::white(210));
    let knob = pos2(track.min.x + track.width() * f, track.center().y);
    p.circle_filled(knob + vec2(0.0, 1.0), 7.5, Color32::from_black_alpha(60));
    p.circle_filled(knob, 7.0, Color32::WHITE);
    if resp.hovered() || resp.dragged() {
        p.circle_stroke(knob, 10.0, Stroke::new(3.0_f32, crate::theme::white(30)));
    }
    p.text(pos2(rect.max.x, track.center().y), Align2::RIGHT_CENTER, value.to_string(), semibold(13.0), look.ink.ink);
    changed
}

fn switch_row(ui: &mut Ui, title: &str, hint: &str, on: &mut bool, look: &Look) -> bool {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
    let p = ui.painter();
    p.text(rect.left_top(), Align2::LEFT_TOP, title, font(13.0), look.ink.ink);
    if !hint.is_empty() {
        p.text(rect.left_top() + vec2(0.0, 18.0), Align2::LEFT_TOP, hint, font(10.5), look.ink.faint);
    }
    let sw = Rect::from_center_size(pos2(rect.max.x - 20.0, rect.center().y), vec2(40.0, 22.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(sw));
    if super::panel::switch(&mut child, *on, Id::new(("switch", title))) {
        *on = !*on;
        return true;
    }
    false
}

/// Title + hint on the left and a "− n +" stepper pill on the right.
fn stepper_row(ui: &mut Ui, title: &str, hint: &str, value: &mut u32, min: u32, max: u32, look: &Look) -> bool {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
    let p = ui.painter().clone();
    p.text(rect.left_top(), Align2::LEFT_TOP, title, font(13.0), look.ink.ink);
    if !hint.is_empty() {
        p.text(rect.left_top() + vec2(0.0, 18.0), Align2::LEFT_TOP, hint, font(10.5), look.ink.faint);
    }
    let pill = Rect::from_center_size(pos2(rect.max.x - 34.0, rect.center().y), vec2(66.0, 26.0));
    p.rect_filled(pill, CornerRadius::same(13), crate::theme::white(14));
    p.rect_stroke(pill, CornerRadius::same(13), Stroke::new(1.0_f32, look.ink.hair), StrokeKind::Inside);
    p.text(pill.center(), Align2::CENTER_CENTER, value.to_string(), semibold(12.0), look.ink.ink);
    let mut changed = false;
    for (dx, sign, label) in [(-22.0, -1i32, "–"), (22.0, 1, "+")] {
        let b = Rect::from_center_size(pill.center() + vec2(dx, 0.0), vec2(20.0, 22.0));
        let resp = ui.interact(b, Id::new(("stepper", title, sign)), Sense::click());
        p.text(b.center(), Align2::CENTER_CENTER, label, font(13.0), if resp.hovered() { look.ink.ink } else { look.ink.dim });
        if resp.clicked() {
            let next = (*value as i32 + sign).clamp(min as i32, max as i32) as u32;
            if next != *value {
                *value = next;
                changed = true;
            }
        }
    }
    changed
}

/// Title + hint on the left and a small pill button on the right.
fn button_row(ui: &mut Ui, title: &str, hint: &str, label: &str, look: &Look) -> bool {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
    let p = ui.painter().clone();
    p.text(rect.left_top(), Align2::LEFT_TOP, title, font(13.0), look.ink.ink);
    p.text(rect.left_top() + vec2(0.0, 18.0), Align2::LEFT_TOP, hint, font(10.5), look.ink.faint);
    let g = ui.fonts(|f| f.layout_no_wrap(label.to_string(), font(11.5), look.ink.ink));
    let btn = Rect::from_center_size(pos2(rect.max.x - g.size().x / 2.0 - 14.0, rect.center().y), vec2(g.size().x + 24.0, 26.0));
    let resp = ui.interact(btn, Id::new(("btnrow", title)), Sense::click());
    p.rect_filled(btn, CornerRadius::same(13), if resp.hovered() { look.ink.hover_strong } else { crate::theme::white(14) });
    p.rect_stroke(btn, CornerRadius::same(13), Stroke::new(1.0_f32, look.ink.hair), StrokeKind::Inside);
    p.galley(btn.center() - g.size() / 2.0, g, look.ink.ink);
    resp.clicked()
}

fn row_separator(ui: &mut Ui, look: &Look) {
    let r = ui.available_rect_before_wrap();
    ui.painter().line_segment([pos2(r.min.x - 12.0, r.min.y + 3.0), pos2(r.max.x + 12.0, r.min.y + 3.0)], Stroke::new(1.0_f32, look.ink.hair));
    ui.add_space(7.0);
}

pub fn show(app: &mut App, ui: &mut Ui, ctx: &egui::Context, id: &'static str, look: &Look) {
    let full = ui.max_rect();
    // Header: back, title, close.
    let back = Rect::from_min_size(full.min, vec2(30.0, 30.0));
    {
        let resp = ui.interact(back, Id::new("ws-back"), Sense::click()).on_hover_text("Back to widgets");
        let p = ui.painter();
        p.circle_filled(back.center(), 14.0, if resp.hovered() { look.ink.hover_strong } else { look.ink.hover });
        p.circle_stroke(back.center(), 14.0, Stroke::new(1.0_f32, look.ink.hair));
        let c = back.center();
        p.line_segment([c + vec2(2.5, -5.0), c + vec2(-2.5, 0.0)], Stroke::new(1.6_f32, look.ink.ink));
        p.line_segment([c + vec2(-2.5, 0.0), c + vec2(2.5, 5.0)], Stroke::new(1.6_f32, look.ink.ink));
        if resp.clicked() {
            app.widget_settings = None;
            app.widgets_panel_open = true;
        }
    }
    let name = widget_name(id);
    ui.painter().text(pos2(back.max.x + 10.0, back.center().y), Align2::LEFT_CENTER, format!("{name} settings"), semibold(14.5), look.ink.ink);
    let close = Rect::from_center_size(pos2(full.max.x - 12.0, back.center().y), vec2(24.0, 24.0));
    let cr = ui.interact(close, Id::new("ws-close"), Sense::click()).on_hover_text("Close");
    if cr.hovered() {
        ui.painter().rect_filled(close, CornerRadius::same(8), look.ink.hover);
    }
    ui.painter().text(close.center(), Align2::CENTER_CENTER, glyph::CLOSE.to_string(), egui::FontId::new(10.0, egui::FontFamily::Name("icons".into())), look.ink.dim);
    if cr.clicked() {
        app.widget_settings = None;
    }

    let body = Rect::from_min_max(pos2(full.min.x, back.max.y + 10.0), full.max);
    let mut c = ui.new_child(egui::UiBuilder::new().max_rect(body));
    egui::ScrollArea::vertical().id_salt(("ws-scroll", id)).auto_shrink([false, false]).show(&mut c, |ui| {
        ui.set_width(body.width() - 14.0);
        ui.label(RichText::new(format!("Changes here apply only to {name}.")).size(12.0).color(look.ink.dim));
        let before = app.state.settings.clone();
        let cal_before = app.calendar.clone();

        design(app, ui, ctx, id, look);
        background(app, ui, ctx, id, look);
        extras(app, ui, id, look);

        if app.state.settings != before {
            app.state.settings.sanitize();
            app.mark_state();
        }
        if app.calendar != cal_before {
            app.mark_calendar();
        }
        ui.add_space(10.0);
    });
}

fn design(app: &mut App, ui: &mut Ui, ctx: &egui::Context, id: &'static str, look: &Look) {
    section(ui, "Design", look);
    let current = app.state.settings.design(id);
    let designs = theme::designs(id);
    let current_name = designs.iter().find(|d| d.id == current).map(|d| d.name).unwrap_or(designs[0].name);
    card(ui, look, |ui| {
        let width = ui.available_width();
        let (rect, resp) = ui.allocate_exact_size(vec2(width, 22.0), Sense::click());
        let p = ui.painter();
        p.text(rect.left_center(), Align2::LEFT_CENTER, "Design", font(12.5), look.ink.dim);
        p.text(rect.left_center() + vec2(48.0, 0.0), Align2::LEFT_CENTER, current_name, semibold(12.5), look.ink.ink);
        let arrow = if app.ws_ui.gallery_open { "▴" } else { "▾" };
        p.text(rect.right_center(), Align2::RIGHT_CENTER, format!("Change design {arrow}"), font(11.5), if resp.hovered() { look.ink.ink } else { look.ink.dim });
        if resp.clicked() && designs.len() > 1 {
            app.ws_ui.gallery_open = !app.ws_ui.gallery_open;
        }
        if app.ws_ui.gallery_open && designs.len() > 1 {
            ui.add_space(8.0);
            let cols = 3;
            let gap = 8.0;
            let w = (ui.available_width() - gap * (cols - 1) as f32) / cols as f32;
            let h = w * 0.62 + 44.0;
            for chunk in designs.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for d in chunk {
                        let (r, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
                        let active = current == d.id;
                        let p = ui.painter();
                        p.rect_filled(r, CornerRadius::same(12), if resp.hovered() { look.ink.hover } else { Color32::from_black_alpha(30) });
                        p.rect_stroke(r, CornerRadius::same(12), Stroke::new(if active { 1.5_f32 } else { 1.0 }, if active { Color32::WHITE } else { look.ink.hair }), StrokeKind::Inside);
                        // Preview tile: the design's own surface, or a dark card.
                        let prev = Rect::from_min_size(r.min + vec2(8.0, 8.0), vec2(w - 16.0, w * 0.62 - 8.0));
                        let tex = app.glass.theme_texture(ctx, &format!("preview:{id}:{}", d.id), || match theme::design_surface(id, d.id) {
                            Some((base, radials, _)) => theme::render_background(base, radials, 64, 40),
                            None => theme::render_background(theme::Base::Solid(0x161a24), &[], 8, 8),
                        });
                        p.add(egui::Shape::Rect(
                            egui::epaint::RectShape::filled(prev, CornerRadius::same(8), Color32::WHITE)
                                .with_texture(tex.id(), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))),
                        ));
                        let light = theme::design_surface(id, d.id).is_some_and(|s| s.2);
                        preview_art(p, prev, id, d.id, light);
                        p.text(pos2(r.min.x + 10.0, prev.max.y + 8.0), Align2::LEFT_TOP, d.name, semibold(12.0), look.ink.ink);
                        let g = elided(ui, d.description, font(10.0), look.ink.dim, w - 18.0);
                        ui.painter().galley(pos2(r.min.x + 10.0, prev.max.y + 24.0), g, look.ink.dim);
                        if resp.clicked() {
                            app.state.settings.set_design(id, d.id);
                        }
                    }
                });
                ui.add_space(gap);
            }
        }
    });
}

/// Small illustration of a design, like Lumen's gallery previews.
fn preview_art(p: &egui::Painter, r: Rect, widget: &str, design: &str, light: bool) {
    use crate::theme::{rgb, white};
    let ink = if light { rgb(0x1d2233) } else { Color32::WHITE };
    let c = r.center();
    let h = r.height();
    let rr = |p: &egui::Painter, rect: Rect, rad: u8, col: Color32| p.rect_filled(rect, CornerRadius::same(rad), col);
    match (widget, design) {
        ("music", "coverflow") => {
            let s = h * 0.62;
            rr(p, Rect::from_center_size(c - vec2(s * 0.55, 0.0), vec2(s * 0.8, s * 0.8)), 4, rgb(0xb7a9d8).gamma_multiply(0.7));
            rr(p, Rect::from_center_size(c + vec2(s * 0.55, 0.0), vec2(s * 0.8, s * 0.8)), 4, rgb(0xb7a9d8).gamma_multiply(0.7));
            rr(p, Rect::from_center_size(c, vec2(s, s)), 5, rgb(0x6c5a9e));
            p.rect_stroke(Rect::from_center_size(c, vec2(s, s)), CornerRadius::same(5), Stroke::new(1.0_f32, white(90)), StrokeKind::Inside);
        }
        ("music", "lumen") => {
            let s = h * 0.62;
            let sq = Rect::from_min_size(pos2(r.min.x + h * 0.18, c.y - s / 2.0), vec2(s, s));
            rr(p, sq, 4, rgb(0xc98bb6));
            for k in 0..3 {
                let y = c.y - s * 0.32 + k as f32 * s * 0.32;
                rr(p, Rect::from_min_size(pos2(sq.max.x + 8.0, y - 3.0), vec2(r.max.x - sq.max.x - 16.0, 6.0)), 3, rgb(0xa9c4ef));
            }
        }
        ("music", "sleeve") => {
            let s = h * 0.5;
            rr(p, Rect::from_center_size(c - vec2(0.0, h * 0.08), vec2(s, s)), 3, rgb(0xd9a2a8));
            rr(p, Rect::from_center_size(pos2(c.x, c.y + h * 0.3), vec2(s * 1.1, 3.0)), 1, rgb(0xa9c4ef));
        }
        ("music", "turntable") => {
            let d = h * 0.7;
            let cc = pos2(r.min.x + h * 0.2 + d / 2.0, c.y);
            p.circle_filled(cc, d / 2.0, rgb(0x1a1a1a));
            p.circle_filled(cc, d * 0.16, rgb(0xe0703a));
            p.line_segment([pos2(r.max.x - h * 0.25, r.min.y + h * 0.15), pos2(r.max.x - h * 0.45, r.max.y - h * 0.18)], Stroke::new(2.0_f32, white(220)));
        }
        ("notes", "lumen") => {
            let cols = [rgb(0xfff0bf), rgb(0xffd6dd), rgb(0xd4ebff), rgb(0xd3f5e3)];
            let w = (r.width() - 30.0) / 4.0;
            for (k, col) in cols.iter().enumerate() {
                let x = r.min.x + 8.0 + k as f32 * (w + 4.0);
                let tall = if k % 2 == 0 { h * 0.62 } else { h * 0.48 };
                rr(p, Rect::from_min_size(pos2(x, r.min.y + h * 0.18), vec2(w, tall)), 3, *col);
            }
        }
        ("notes", "notebook") => {
            for k in 0..5 {
                let y = r.min.y + h * 0.22 + k as f32 * h * 0.15;
                p.line_segment([pos2(r.min.x + 8.0, y), pos2(r.max.x - 8.0, y)], Stroke::new(1.0_f32, crate::theme::css(70, 110, 170, 90)));
            }
            p.line_segment([pos2(r.min.x + 22.0, r.min.y + 4.0), pos2(r.min.x + 22.0, r.max.y - 4.0)], Stroke::new(1.0_f32, crate::theme::css(210, 80, 80, 140)));
        }
        ("notes", "index") => {
            for k in 0..3 {
                let y = r.min.y + h * 0.25 + k as f32 * h * 0.25;
                p.text(pos2(r.min.x + 10.0, y), Align2::LEFT_CENTER, format!("0{}", k + 1), semibold(10.0), rgb(0x5fd6c0));
                rr(p, Rect::from_min_size(pos2(r.min.x + 30.0, y - 3.0), vec2(r.width() * 0.6, 6.0)), 3, white(60));
            }
        }
        ("launcher", _) => {
            let s = h * 0.26;
            for k in 0..8 {
                let x = r.min.x + 10.0 + (k % 4) as f32 * (s + 8.0);
                let y = r.min.y + h * 0.16 + (k / 4) as f32 * (s + 8.0);
                rr(p, Rect::from_min_size(pos2(x, y), vec2(s, s)), 4, crate::theme::accent_for(&k.to_string()));
            }
        }
        ("calendar", "lumen") => {
            // Lumen: rows of short blue bars over a ruled grid.
            for row in 0..4 {
                let y = r.min.y + h * 0.18 + row as f32 * h * 0.2;
                p.line_segment([pos2(r.min.x + 6.0, y + 6.0), pos2(r.max.x - 6.0, y + 6.0)], Stroke::new(1.0_f32, white(30)));
                if row == 0 {
                    for k in 0..4 {
                        let x = r.min.x + 10.0 + k as f32 * (r.width() - 20.0) / 4.0;
                        rr(p, Rect::from_min_size(pos2(x, y - 2.0), vec2((r.width() - 20.0) / 4.0 - 8.0, 4.0)), 2, rgb(0x7fa7ff));
                    }
                }
            }
        }
        ("calendar", "almanac") => {
            p.text(pos2(r.min.x + 10.0, c.y), Align2::LEFT_CENTER, "12", super::serif(h * 0.55), rgb(0xc4453b));
            for k in 0..5 {
                let y = r.min.y + h * 0.25 + k as f32 * h * 0.12;
                p.line_segment([pos2(c.x, y), pos2(r.max.x - 8.0, y)], Stroke::new(1.0_f32, rgb(0x5a5248)));
            }
        }
        ("calendar", "blocks") => {
            let w = (r.width() - 24.0) / 2.0;
            let hh = (h - 20.0) / 2.0;
            for (k, col) in [rgb(0x4f7cff), rgb(0xff8a3d), rgb(0xf05b5b), rgb(0xffd84f)].iter().enumerate() {
                let x = r.min.x + 8.0 + (k % 2) as f32 * (w + 8.0);
                let y = r.min.y + 6.0 + (k / 2) as f32 * (hh + 8.0);
                rr(p, Rect::from_min_size(pos2(x, y), vec2(w, hh)), 4, *col);
            }
        }
        ("calendar", d) => {
            // Pop / Prism / Grove: a coloured "26" tile beside ruled lines.
            let tile_col = match d {
                "pop" => rgb(0x2f5bff),
                "prism" => rgb(0xff6fb5),
                _ => rgb(0x2f6b3c),
            };
            let tile = Rect::from_min_size(pos2(r.min.x + 8.0, r.min.y + h * 0.12), vec2(r.width() * 0.4, h * 0.76));
            rr(p, tile, 6, tile_col);
            p.text(tile.center(), Align2::CENTER_CENTER, "26", super::semibold(h * 0.4), Color32::WHITE);
            for k in 0..6 {
                let y = r.min.y + h * 0.2 + k as f32 * h * 0.12;
                p.line_segment([pos2(tile.max.x + 8.0, y), pos2(r.max.x - 8.0, y)], Stroke::new(1.5_f32, white(200)));
            }
        }
        ("weather", "lumen") => {
            p.text(pos2(r.min.x + 8.0, c.y - h * 0.08), Align2::LEFT_CENTER, "28°", super::light(h * 0.42), ink);
            crate::widgets::weather::icon(p, pos2(r.max.x - h * 0.42, c.y - h * 0.1), h * 0.5, 2, true);
            for k in 0..4 {
                p.circle_filled(pos2(r.min.x + 14.0 + k as f32 * (r.width() - 28.0) / 3.0, r.max.y - h * 0.16), 3.0, white(150));
            }
        }
        ("weather", "minimal") => {
            crate::widgets::weather::icon(p, pos2(c.x, r.min.y + h * 0.24), h * 0.3, 0, true);
            p.text(pos2(c.x, c.y + h * 0.14), Align2::CENTER_CENTER, "28°", super::light(h * 0.42), ink);
        }
        ("weather", "forecast") => {
            for k in 0..4 {
                let y = r.min.y + h * 0.2 + k as f32 * h * 0.2;
                rr(p, Rect::from_min_size(pos2(r.min.x + 8.0, y - 2.0), vec2(16.0, 4.0)), 2, white(120));
                let a = r.min.x + 34.0 + (k as f32 * 7.0) % 20.0;
                rr(p, Rect::from_min_size(pos2(a, y - 2.0), vec2(r.width() * 0.45, 4.0)), 2, rgb(0xffa24a));
            }
        }
        ("system", "lumen") => {
            let pts: Vec<egui::Pos2> = (0..12).map(|k| pos2(r.min.x + 8.0 + k as f32 * (r.width() - 16.0) / 11.0, c.y - ((k * 37 % 11) as f32 / 11.0) * h * 0.3)).collect();
            p.add(egui::Shape::line(pts, Stroke::new(1.6_f32, rgb(0xa3e635))));
            for k in 0..2 {
                rr(p, Rect::from_min_size(pos2(r.min.x + 8.0, r.max.y - h * 0.3 + k as f32 * 9.0), vec2(r.width() * (0.6 - k as f32 * 0.2), 4.0)), 2, if k == 0 { rgb(0x34d399) } else { rgb(0x60c3f6) });
            }
        }
        ("system", "rings") => {
            for (k, col) in [rgb(0xa3e635), rgb(0x34d399), rgb(0x60c3f6)].iter().enumerate() {
                let cc = pos2(r.min.x + r.width() * (k as f32 + 0.5) / 3.0, c.y);
                p.circle_stroke(cc, h * 0.24, Stroke::new(3.0_f32, white(40)));
                let pts: Vec<egui::Pos2> = (0..=20).map(|i| {
                    let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * (0.3 + k as f32 * 0.2) * i as f32 / 20.0;
                    cc + vec2(a.cos(), a.sin()) * h * 0.24
                }).collect();
                p.add(egui::Shape::line(pts, Stroke::new(3.0_f32, *col)));
            }
        }
        ("system", "terminal") => {
            for k in 0..4 {
                p.text(pos2(r.min.x + 8.0, r.min.y + 8.0 + k as f32 * h * 0.2), Align2::LEFT_TOP, ["$ top", "cpu [||||  ]", "mem [||||||]", "net ↓ ↑"][k], egui::FontId::monospace(h * 0.13), rgb(0x4dff88));
            }
        }
        ("pomodoro", "lumen") => {
            p.circle_stroke(c, h * 0.34, Stroke::new(3.0_f32, white(40)));
            let pts: Vec<egui::Pos2> = (0..=24).map(|i| {
                let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * 0.6 * i as f32 / 24.0;
                c + vec2(a.cos(), a.sin()) * h * 0.34
            }).collect();
            p.add(egui::Shape::line(pts, Stroke::new(3.0_f32, rgb(0xf0284a))));
            p.text(c, Align2::CENTER_CENTER, "25:00", super::light(h * 0.16), ink);
        }
        ("pomodoro", "minimal") => {
            p.text(pos2(c.x, c.y - h * 0.06), Align2::CENTER_CENTER, "25:00", super::light(h * 0.36), ink);
            rr(p, Rect::from_min_size(pos2(r.min.x + 10.0, c.y + h * 0.22), vec2(r.width() - 20.0, 3.0)), 1, white(50));
            rr(p, Rect::from_min_size(pos2(r.min.x + 10.0, c.y + h * 0.22), vec2((r.width() - 20.0) * 0.6, 3.0)), 1, rgb(0xf0284a));
        }
        ("pomodoro", "tomato") => {
            p.circle_filled(c + vec2(0.0, h * 0.04), h * 0.34, rgb(0xe8352f));
            p.line_segment([c + vec2(-h * 0.12, -h * 0.3), c + vec2(h * 0.12, -h * 0.3)], Stroke::new(3.0_f32, rgb(0x3fa34d)));
            p.text(c + vec2(0.0, h * 0.04), Align2::CENTER_CENTER, "25:00", super::light(h * 0.16), Color32::WHITE);
        }
        ("photos", "lumen") => {
            rr(p, r.shrink(4.0), 6, rgb(0x5b7fa8));
            p.circle_filled(pos2(r.max.x - h * 0.3, r.min.y + h * 0.3), h * 0.1, rgb(0xffd56b));
            p.add(egui::Shape::convex_polygon(vec![pos2(r.min.x + 4.0, r.max.y - 4.0), pos2(c.x - 6.0, c.y - h * 0.05), pos2(c.x + h * 0.3, r.max.y - 4.0)], rgb(0x3d5a3c), Stroke::NONE));
        }
        ("photos", "polaroid") => {
            let pic = Rect::from_min_max(r.min + vec2(14.0, 6.0), pos2(r.max.x - 14.0, r.max.y - h * 0.28));
            rr(p, pic, 1, rgb(0x5b7fa8));
            p.circle_filled(pos2(pic.max.x - h * 0.18, pic.min.y + h * 0.16), h * 0.07, rgb(0xffd56b));
            rr(p, Rect::from_min_size(pos2(r.min.x + 16.0, r.max.y - h * 0.18), vec2(r.width() * 0.4, 3.0)), 1, rgb(0x6b6255));
        }
        ("photos", "film") => {
            for row in [r.min.y + 3.0, r.max.y - 8.0] {
                for k in 0..((r.width() - 12.0) / 12.0) as usize {
                    rr(p, Rect::from_min_size(pos2(r.min.x + 6.0 + k as f32 * 12.0, row), vec2(6.0, 5.0)), 1, rgb(0x3a3a3a));
                }
            }
            rr(p, Rect::from_min_max(pos2(r.min.x + 6.0, r.min.y + 12.0), pos2(r.max.x - 6.0, r.max.y - 12.0)), 2, rgb(0x5b7fa8));
        }
        ("lunar", "lumen") => {
            p.text(pos2(r.min.x + 10.0, c.y), Align2::LEFT_CENTER, "9", super::light(h * 0.5), ink);
            p.line_segment([pos2(c.x, r.min.y + 8.0), pos2(c.x, r.max.y - 8.0)], Stroke::new(1.0_f32, white(60)));
            p.text(pos2(c.x + 10.0, c.y), Align2::LEFT_CENTER, "1", super::serif(h * 0.5), rgb(0xf0b44a));
        }
        ("lunar", "toloch") => {
            rr(p, Rect::from_min_size(r.min, vec2(r.width(), h * 0.2)), 4, rgb(0xc8282e));
            p.text(pos2(c.x, c.y + h * 0.06), Align2::CENTER_CENTER, "9", super::serif(h * 0.5), rgb(0x1f3f8a));
            p.text(pos2(r.min.x + 8.0, r.max.y - 6.0), Align2::LEFT_BOTTOM, "1/10", super::serif(h * 0.14), rgb(0xc8282e));
        }
        ("lunar", "minimal") => {
            p.text(c, Align2::CENTER_CENTER, "1", super::serif(h * 0.6), rgb(0xf0b44a));
        }
        ("todo", "lumen") => {
            for k in 0..3 {
                let y = r.min.y + h * 0.24 + k as f32 * h * 0.26;
                if k == 0 {
                    p.circle_filled(pos2(r.min.x + 12.0, y), 5.0, rgb(0x34d399));
                } else {
                    p.circle_stroke(pos2(r.min.x + 12.0, y), 5.0, Stroke::new(1.2_f32, white(150)));
                }
                rr(p, Rect::from_min_size(pos2(r.min.x + 24.0, y - 2.0), vec2(r.width() * 0.55, 4.0)), 2, white(if k == 0 { 70 } else { 170 }));
            }
        }
        ("todo", "paper") => {
            p.line_segment([pos2(r.min.x + 18.0, r.min.y + 2.0), pos2(r.min.x + 18.0, r.max.y - 2.0)], Stroke::new(1.0_f32, crate::theme::css(210, 80, 80, 140)));
            for k in 0..4 {
                let y = r.min.y + h * 0.22 + k as f32 * h * 0.2;
                p.line_segment([pos2(r.min.x + 4.0, y + 5.0), pos2(r.max.x - 4.0, y + 5.0)], Stroke::new(1.0_f32, crate::theme::css(70, 110, 170, 90)));
                p.rect_stroke(Rect::from_center_size(pos2(r.min.x + 9.0, y), vec2(7.0, 7.0)), CornerRadius::same(1), Stroke::new(1.0_f32, rgb(0x6b6255)), StrokeKind::Inside);
                rr(p, Rect::from_min_size(pos2(r.min.x + 24.0, y - 2.0), vec2(r.width() * 0.5, 3.0)), 1, rgb(0x6b6255));
            }
        }
        ("todo", "compact") => {
            for k in 0..5 {
                let y = r.min.y + h * 0.16 + k as f32 * h * 0.17;
                p.text(pos2(r.min.x + 6.0, y), Align2::LEFT_CENTER, format!("0{}", k + 1), semibold(h * 0.11), rgb(0x5fd6c0));
                rr(p, Rect::from_min_size(pos2(r.min.x + 24.0, y - 1.5), vec2(r.width() * 0.6, 3.0)), 1, white(160));
            }
        }
        ("clock", "paper") => {
            let rad = h * 0.34;
            p.circle_stroke(c, rad, Stroke::new(2.0_f32, ink));
            p.line_segment([c, c + vec2(0.0, -rad * 0.7)], Stroke::new(2.0_f32, ink));
            p.line_segment([c, c + vec2(rad * 0.6, 0.0)], Stroke::new(2.0_f32, ink));
        }
        ("clock", "modular") => {
            let w = (r.width() - 24.0) / 2.0;
            let hh = (h - 20.0) / 2.0;
            for (k, col) in [rgb(0x4f7cff), rgb(0xff8a3d), rgb(0xf05b5b), rgb(0xffd84f)].iter().enumerate() {
                let x = r.min.x + 8.0 + (k % 2) as f32 * (w + 8.0);
                let y = r.min.y + 6.0 + (k / 2) as f32 * (hh + 8.0);
                rr(p, Rect::from_min_size(pos2(x, y), vec2(w, hh)), 4, *col);
            }
        }
        _ => {
            p.text(r.left_center() + vec2(12.0, 0.0), Align2::LEFT_CENTER, "12", super::light(h * 0.45), ink);
        }
    }
}

fn background(app: &mut App, ui: &mut Ui, ctx: &egui::Context, id: &'static str, look: &Look) {
    section(ui, "Background", look);
    card(ui, look, |ui| {
        let mut frosted = app.state.settings.frosted(id);
        if switch_row(ui, "Frosted glass", "Blend this widget's design with the wallpaper", &mut frosted, look) {
            if id == "clock" {
                app.state.settings.clock.frosted_glass = frosted;
            }
            app.state.settings.widget_frosted.insert(id.into(), frosted);
        }
        row_separator(ui, look);
        let s = &mut app.state.settings;
        let mut veil = s.veil_for(id);
        if glass_slider(ui, "Glass tint", &mut veil, 100, "%", look) {
            s.appearance_mut(id).veil = Some(veil);
        }
        let mut frost = s.frost_for(id);
        if glass_slider(ui, "Frost strength", &mut frost, 100, "%", look) {
            s.appearance_mut(id).frost_strength = Some(frost);
        }
        let mut blur = s.blur_for(id);
        if glass_slider(ui, "Blur size", &mut blur, 80, "px", look) {
            s.appearance_mut(id).blur_size = Some(blur);
        }
        let mut opacity = s.opacity(id);
        if glass_slider(ui, "Background opacity", &mut opacity, 100, "%", look) {
            s.widget_opacity.insert(id.into(), opacity);
        }
        ui.label(
            RichText::new("Background opacity controls the widget panel. Glass tint: 0% frosted wallpaper · 100% design background. Turn off Frosted glass for the full design.")
                .size(10.0)
                .color(look.ink.faint),
        );
        row_separator(ui, look);
        let arrow = if app.ws_ui.palette_open { "▾" } else { "▸" };
        let resp = ui.add(egui::Label::new(RichText::new(format!("{arrow}  Surface palette")).size(12.0).color(look.ink.dim)).sense(Sense::click()));
        if resp.clicked() {
            app.ws_ui.palette_open = !app.ws_ui.palette_open;
        }
        if app.ws_ui.palette_open {
            ui.add_space(6.0);
            palette(app, ui, ctx, id, look);
        }
    });
}

fn palette(app: &mut App, ui: &mut Ui, ctx: &egui::Context, id: &'static str, look: &Look) {
    let current = app.state.settings.theme_for(id).to_string();
    let cols = 3;
    let gap = 8.0;
    let w = (ui.available_width() - gap * (cols - 1) as f32) / cols as f32;
    for chunk in theme::THEMES.chunks(cols) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for t in chunk {
                let (r, resp) = ui.allocate_exact_size(vec2(w, 72.0), Sense::click());
                let active = current == t.id;
                let p = ui.painter();
                p.rect_filled(r, CornerRadius::same(12), if resp.hovered() || active { look.ink.hover } else { Color32::from_black_alpha(25) });
                p.rect_stroke(r, CornerRadius::same(12), Stroke::new(if active { 1.5_f32 } else { 1.0 }, if active { Color32::WHITE } else { look.ink.hair }), StrokeKind::Inside);
                let sw = Rect::from_min_size(r.min + vec2(8.0, 8.0), vec2(w - 16.0, 36.0));
                let tex = app.glass.theme_texture(ctx, &format!("theme:{}", t.id), || theme::render_theme(t));
                p.add(egui::Shape::Rect(
                    egui::epaint::RectShape::filled(sw, CornerRadius::same(8), Color32::WHITE)
                        .with_texture(tex.id(), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))),
                ));
                p.text(pos2(r.min.x + 10.0, sw.max.y + 6.0), Align2::LEFT_TOP, t.name, semibold(11.5), look.ink.ink);
                if resp.clicked() {
                    app.state.settings.widget_theme.insert(id.into(), t.id.into());
                }
            }
        });
        ui.add_space(gap);
    }
}

fn extras(app: &mut App, ui: &mut Ui, id: &'static str, look: &Look) {
    match id {
        "calendar" => {
            section(ui, "Classic calendar surface", look);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let w = (ui.available_width() - 16.0) / 3.0;
                for (key, name, bg, ink) in [
                    ("glass", "Glass", crate::theme::white(18), Color32::WHITE),
                    ("paper", "Paper", theme::rgb(0xeceef3), theme::rgb(0x2a2d36)),
                    ("midnight", "Midnight", theme::rgb(0x191919), Color32::WHITE),
                ] {
                    let (r, resp) = ui.allocate_exact_size(vec2(w, 92.0), Sense::click());
                    let active = app.calendar.theme == key;
                    let p = ui.painter();
                    p.rect_filled(r, CornerRadius::same(12), if resp.hovered() { look.ink.hover } else { Color32::from_black_alpha(25) });
                    p.rect_stroke(r, CornerRadius::same(12), Stroke::new(if active { 1.5_f32 } else { 1.0 }, if active { Color32::WHITE } else { look.ink.hair }), StrokeKind::Inside);
                    let sw = Rect::from_min_size(r.min + vec2(8.0, 8.0), vec2(w - 16.0, 50.0));
                    p.rect_filled(sw, CornerRadius::same(8), bg);
                    p.text(sw.left_center() + vec2(12.0, 0.0), Align2::LEFT_CENTER, "Aa", font(20.0), ink);
                    p.circle_filled(sw.right_center() - vec2(22.0, 0.0), 11.0, theme::rgb(0xf04848));
                    p.text(sw.right_center() - vec2(22.0, 0.0), Align2::CENTER_CENTER, "26", semibold(10.0), Color32::WHITE);
                    p.text(pos2(r.min.x + 10.0, sw.max.y + 8.0), Align2::LEFT_TOP, name, font(12.0), look.ink.ink);
                    if resp.clicked() {
                        app.calendar.theme = key.into();
                    }
                }
            });
            section(ui, "Display", look);
            card(ui, look, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Week starts").size(13.0).color(look.ink.ink));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let label = match app.calendar.week_starts_on {
                            0 => "Sunday",
                            6 => "Saturday",
                            _ => "Monday",
                        };
                        egui::ComboBox::from_id_salt("week-starts").selected_text(label).width(110.0).show_ui(ui, |ui| {
                            ui.selectable_value(&mut app.calendar.week_starts_on, 1, "Monday");
                            ui.selectable_value(&mut app.calendar.week_starts_on, 0, "Sunday");
                            ui.selectable_value(&mut app.calendar.week_starts_on, 6, "Saturday");
                        });
                    });
                });
            });
            section(ui, "Calendar feed", look);
            card(ui, look, |ui| {
                ui.label(RichText::new("Private iCal (ICS) address from Google or Outlook").size(11.0).color(look.ink.dim));
                let current = app.calendar.ics_url.clone();
                if let Some(v) = commit_field(ui, &mut app.ws_ui.ics_draft, &current, "https://…/basic.ics — press Enter") {
                    app.calendar.ics_url = v;
                }
                ui.horizontal(|ui| {
                    if ui.button("Sync now").clicked() {
                        app.cal.refresh();
                    }
                    if !app.cal.status.is_empty() {
                        ui.label(RichText::new(&app.cal.status).size(11.0).color(look.ink.dim));
                    }
                });
            });
        }
        "clock" => {
            section(ui, "Display", look);
            card(ui, look, |ui| {
                let c = &mut app.state.settings.clock;
                switch_row(ui, "12-hour clock", "Show AM / PM", &mut c.hour12, look);
                row_separator(ui, look);
                switch_row(ui, "Show seconds", "", &mut c.show_seconds, look);
                row_separator(ui, look);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Time zone").size(13.0).color(look.ink.ink));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        egui::ComboBox::from_id_salt("ws-zone")
                            .selected_text(if c.time_zone == "local" { "Local time".to_string() } else { c.time_zone.clone() })
                            .width(180.0)
                            .show_ui(ui, |ui| {
                                for z in crate::widgets::settings::ZONES {
                                    let label = if *z == "local" { "Local time" } else { z };
                                    ui.selectable_value(&mut c.time_zone, z.to_string(), label);
                                }
                            });
                    });
                });
            });
        }
        "launcher" => {
            section(ui, "App layout", look);
            card(ui, look, |ui| {
                let s = &mut app.state.settings;
                stepper_row(ui, "Columns", "Used by the app grid", &mut s.columns, 3, 12, look);
                row_separator(ui, look);
                glass_slider(ui, "Icon size", &mut s.icon_size, 60, "px", look);
                s.icon_size = s.icon_size.max(36);
                row_separator(ui, look);
                switch_row(ui, "Show labels", "", &mut s.show_labels, look);
                row_separator(ui, look);
                switch_row(ui, "Show search", "", &mut s.show_search, look);
                row_separator(ui, look);
                if button_row(ui, "App icons", "Re-extract dock app icons", "Refresh icons", look) {
                    let paths: Vec<String> = app.state.items.iter().map(|i| i.icon_path()).collect();
                    for p in paths {
                        app.icons.forget(&p);
                    }
                }
            });
        }
        "notes" => {
            section(ui, "Storage", look);
            card(ui, look, |ui| {
                ui.label(RichText::new("Notes save as you type to config\\notes.json beside the app.").size(11.5).color(look.ink.dim));
                if ui.button("Open folder").clicked() {
                    crate::win::open_folder(&crate::config::config_dir());
                }
            });
        }
        "weather" => {
            section(ui, "Location", look);
            card(ui, look, |ui| {
                ui.label(RichText::new("City (leave empty to use your IP location)").size(11.5).color(look.ink.dim));
                let current = app.extras.weather_city.clone();
                if let Some(v) = commit_field(ui, &mut app.ws_ui.city_draft, &current, "e.g. Hà Nội, Đà Nẵng, Tokyo — press Enter") {
                    app.extras.weather_city = v;
                    app.mark_extras();
                }
                row_separator(ui, look);
                let mut f = app.extras.weather_fahrenheit;
                if switch_row(ui, "Fahrenheit", "Show °F instead of °C", &mut f, look) {
                    app.extras.weather_fahrenheit = f;
                    app.mark_extras();
                }
                row_separator(ui, look);
                if button_row(ui, "Forecast", if app.weather.status.is_empty() { "Updates every 20 minutes" } else { app.weather.status.as_str() }, "Refresh", look) {
                    app.weather.refresh();
                }
            });
        }
        "photos" => {
            section(ui, "Slideshow", look);
            card(ui, look, |ui| {
                let folder = if app.extras.photos_folder.is_empty() { "No folder".to_string() } else { app.extras.photos_folder.clone() };
                if button_row(ui, "Folder", &folder, "Choose…", look) {
                    if let Some(dir) = rfd::FileDialog::new().set_title("Photo folder").pick_folder() {
                        app.extras.photos_folder = dir.to_string_lossy().to_string();
                        app.mark_extras();
                    }
                }
                row_separator(ui, look);
                let mut secs = app.extras.photos_interval.min(600);
                if glass_slider(ui, "Change every", &mut secs, 600, "s", look) {
                    app.extras.photos_interval = secs.max(5);
                    app.mark_extras();
                }
            });
        }
        "pomodoro" => {
            section(ui, "Durations", look);
            card(ui, look, |ui| {
                let e = &mut app.extras;
                let mut changed = false;
                changed |= stepper_row(ui, "Focus", "Minutes", &mut e.focus_minutes, 1, 180, look);
                row_separator(ui, look);
                changed |= stepper_row(ui, "Short break", "Minutes", &mut e.short_break_minutes, 1, 60, look);
                row_separator(ui, look);
                changed |= stepper_row(ui, "Long break", "Minutes, after every fourth focus", &mut e.long_break_minutes, 1, 90, look);
                if changed {
                    app.mark_extras();
                }
            });
        }
        "system" => {
            section(ui, "Display", look);
            card(ui, look, |ui| {
                let e = &mut app.extras;
                let mut changed = switch_row(ui, "CPU graph", "Last 60 seconds of CPU use", &mut e.system_show_graph, look);
                row_separator(ui, look);
                changed |= switch_row(ui, "Network", "Download and upload speed", &mut e.system_show_network, look);
                if changed {
                    app.mark_extras();
                }
            });
        }
        "lunar" => {
            section(ui, "Hiển thị", look);
            card(ui, look, |ui| {
                let e = &mut app.extras;
                let mut changed = switch_row(ui, "Tiết khí", "Tiết khí của ngày", &mut e.lunar_show_term, look);
                row_separator(ui, look);
                changed |= switch_row(ui, "Giờ hoàng đạo", "Các giờ tốt trong ngày", &mut e.lunar_show_hours, look);
                if changed {
                    app.mark_extras();
                }
            });
        }
        "todo" => {
            section(ui, "Tasks", look);
            card(ui, look, |ui| {
                let done = app.extras.todos.iter().filter(|t| t.done).count();
                if button_row(ui, "Completed", &format!("{done} finished tasks"), "Clear", look) {
                    app.extras.todos.retain(|t| !t.done);
                    app.mark_extras();
                }
            });
        }
        "music" => {
            section(ui, "Player", look);
            card(ui, look, |ui| {
                if button_row(ui, "YouTube Music", "Opens as an app window in Chrome or Edge", "Open", look) {
                    if let Err(e) = crate::media::open_youtube_music() {
                        app.notify(e);
                    }
                }
                row_separator(ui, look);
                if button_row(ui, "Media Player", "Windows' own music player", "Open", look) {
                    if let Err(e) = crate::media::open_media_player() {
                        app.notify(e);
                    }
                }
            });
            section(ui, "Source", look);
            card(ui, look, |ui| {
                ui.label(
                    RichText::new("Follows whatever Windows is playing: Spotify, YouTube Music or any site in your browser, Media Player, VLC and more.")
                        .size(11.5)
                        .color(look.ink.dim),
                );
            });
        }
        _ => {}
    }
}
