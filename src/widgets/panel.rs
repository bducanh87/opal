//! The "Widgets" manager panel opened from the launcher's grid button:
//! one row per widget with its status, settings and an on/off switch.

use egui::{Align2, Color32, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use super::{elided, font, semibold};
use crate::app::{App, WIDGETS, glyph};
use crate::glass::Look;
use crate::theme::rgb;

struct Row {
    id: &'static str,
    name: &'static str,
    glyph: char,
    color: Color32,
    description: &'static str,
}

const ROWS: [Row; 11] = [
    Row { id: "clock", name: "Clock", glyph: glyph::CLOCK, color: rgb(0xf5a35c), description: "Live time, nine expressive faces and optional frosted glass." },
    Row { id: "launcher", name: "App Dock", glyph: glyph::DOCK, color: rgb(0x8ea4ff), description: "The glass dock, app grid and widget manager." },
    Row { id: "notes", name: "Sticky notes", glyph: glyph::NOTE, color: rgb(0xf2c94c), description: "A glass board for quick notes that saves right beside the app." },
    Row { id: "calendar", name: "Calendar", glyph: glyph::CALENDAR, color: rgb(0x5fd38d), description: "A month calendar with expressive date cards on the desktop." },
    Row { id: "music", name: "Music", glyph: glyph::MUSIC, color: rgb(0xff6b81), description: "A compact card that follows and controls whatever is playing." },
    Row { id: "weather", name: "Weather", glyph: '\u{E706}', color: rgb(0xffc94a), description: "Current conditions and the next hours or days, from Open-Meteo." },
    Row { id: "system", name: "System", glyph: '\u{E7F4}', color: rgb(0xa3e635), description: "CPU with a live graph, memory, system drive and network speed." },
    Row { id: "pomodoro", name: "Focus timer", glyph: '\u{E916}', color: rgb(0xf0284a), description: "Pomodoro focus sessions with short and long breaks." },
    Row { id: "photos", name: "Photos", glyph: '\u{E91B}', color: rgb(0x60c3f6), description: "A slideshow of a photo folder, cross-faded in the glass." },
    Row { id: "lunar", name: "Âm lịch", glyph: glyph::CALENDAR, color: rgb(0xf0b44a), description: "Ngày âm lịch, can chi, tiết khí và giờ hoàng đạo." },
    Row { id: "todo", name: "To-do", glyph: '\u{E73E}', color: rgb(0x34d399), description: "A simple task list that saves as you go." },
];

/// iOS-style switch; returns true when clicked.
pub fn switch(ui: &mut Ui, on: bool, id: Id) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(40.0, 22.0), Sense::click());
    let rect = Rect::from_center_size(rect.center(), vec2(40.0, 22.0));
    let resp = ui.interact(rect, id, Sense::click()).union(resp);
    let t = ui.ctx().animate_bool(id, on);
    let track = if on {
        rgb(0x4a2257).gamma_multiply(0.8 + 0.2 * t)
    } else {
        crate::theme::white(28)
    };
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(11), track);
    p.rect_stroke(rect, CornerRadius::same(11), Stroke::new(1.0_f32, crate::theme::white(30)), StrokeKind::Inside);
    let x = egui::lerp(rect.min.x + 11.0..=rect.max.x - 11.0, t);
    p.circle_filled(pos2(x, rect.center().y + 0.5), 8.5, Color32::from_black_alpha(40));
    p.circle_filled(pos2(x, rect.center().y), 8.0, if on { Color32::WHITE } else { crate::theme::white(200) });
    resp.clicked()
}

fn small_icon_button(ui: &mut Ui, rect: Rect, ch: char, look: &Look, id: Id, tip: &str) -> bool {
    let resp = ui.interact(rect, id, Sense::click()).on_hover_text(tip);
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(8), look.ink.hover);
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        ch.to_string(),
        egui::FontId::new(13.0, egui::FontFamily::Name("icons".into())),
        if resp.hovered() { look.ink.ink } else { look.ink.dim },
    );
    resp.clicked()
}

pub fn show(app: &mut App, ui: &mut Ui, look: &Look) {
    let full = ui.max_rect();
    // Header: back, title, close.
    let head = Rect::from_min_size(full.min, vec2(full.width(), 34.0));
    let back = Rect::from_min_size(head.min, vec2(30.0, 30.0));
    {
        let resp = ui.interact(back, Id::new("panel-back"), Sense::click()).on_hover_text("Back");
        let p = ui.painter();
        p.circle_filled(back.center(), 14.0, if resp.hovered() { look.ink.hover_strong } else { look.ink.hover });
        p.circle_stroke(back.center(), 14.0, Stroke::new(1.0_f32, look.ink.hair));
        let c = back.center();
        p.line_segment([c + vec2(2.5, -5.0), c + vec2(-2.5, 0.0)], Stroke::new(1.6_f32, look.ink.ink));
        p.line_segment([c + vec2(-2.5, 0.0), c + vec2(2.5, 5.0)], Stroke::new(1.6_f32, look.ink.ink));
        if resp.clicked() {
            app.widgets_panel_open = false;
        }
    }
    ui.painter().text(pos2(back.max.x + 10.0, back.center().y), Align2::LEFT_CENTER, "Widgets", semibold(15.0), look.ink.ink);
    let close = Rect::from_center_size(pos2(head.max.x - 14.0, back.center().y), vec2(26.0, 26.0));
    if small_icon_button(ui, close, glyph::CLOSE, look, Id::new("panel-close"), "Close") {
        app.widgets_panel_open = false;
    }

    let visible = WIDGETS.iter().filter(|(id, _)| app.is_open(id)).count();
    let intro = format!(
        "One launcher, every surface. Turn widgets on or off, open each widget's settings to customize its UI. {visible} of {} visible.",
        WIDGETS.len()
    );
    let intro_g = ui.fonts(|f| f.layout(intro, font(12.5), look.ink.dim, full.width() - 4.0));
    let intro_pos = pos2(full.min.x + 2.0, head.max.y + 8.0);
    let intro_h = intro_g.size().y;
    ui.painter().galley(intro_pos, intro_g, look.ink.dim);

    let row_h = 52.0;
    let gap = 7.0;
    // Rows scroll, so all widgets stay reachable on small or scaled screens.
    let list_rect = Rect::from_min_max(pos2(full.min.x, intro_pos.y + intro_h + 12.0), full.max);
    let mut list_ui = ui.new_child(egui::UiBuilder::new().max_rect(list_rect));
    egui::ScrollArea::vertical().id_salt("panel-rows").auto_shrink([false, false]).show(&mut list_ui, |ui| {
    let total = (ROWS.len() + 1) as f32 * (row_h + gap);
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width() - 6.0, total), Sense::hover());
    let full = area;
    let mut y = area.min.y;
    for row in &ROWS {
        let r = Rect::from_min_size(pos2(full.min.x, y), vec2(full.width(), row_h));
        y += row_h + gap;
        let open = row.id == "launcher" || app.is_open(row.id);
        let card = ui.interact(r, Id::new(("panel-row", row.id)), Sense::hover());
        let p = ui.painter();
        p.rect_filled(r, CornerRadius::same(16), if card.hovered() { look.ink.hover } else { crate::theme::white(11) });
        p.rect_stroke(r, CornerRadius::same(16), Stroke::new(1.0_f32, look.ink.hair), StrokeKind::Inside);
        // icon chip
        let chip = Rect::from_min_size(pos2(r.min.x + 12.0, r.center().y - 18.0), vec2(36.0, 36.0));
        p.rect_filled(chip, CornerRadius::same(11), Color32::from_black_alpha(60));
        p.rect_filled(chip, CornerRadius::same(11), row.color.gamma_multiply(0.22));
        p.rect_stroke(chip, CornerRadius::same(11), Stroke::new(1.0_f32, row.color.gamma_multiply(0.55)), StrokeKind::Inside);
        p.text(chip.center(), Align2::CENTER_CENTER, row.glyph.to_string(), egui::FontId::new(16.0, egui::FontFamily::Name("icons".into())), row.color);
        // title + status
        let tx = chip.max.x + 14.0;
        let title_g = ui.fonts(|f| f.layout_no_wrap(row.name.to_string(), semibold(14.0), look.ink.ink));
        let title_w = title_g.size().x;
        p.galley(pos2(tx, r.min.y + 9.0), title_g, look.ink.ink);
        let (status, scolor) = match (row.id, open) {
            ("launcher", _) => ("HOST", rgb(0x7bd88f)),
            (_, true) => ("ON DESKTOP", rgb(0x7bd88f)),
            (_, false) => ("HIDDEN", look.ink.faint),
        };
        let sx = tx + title_w + 10.0;
        p.circle_filled(pos2(sx, r.min.y + 18.0), 2.5, scolor);
        p.text(pos2(sx + 6.0, r.min.y + 18.0), Align2::LEFT_CENTER, status, semibold(9.5), if open { look.ink.dim } else { look.ink.faint });
        let right_w = 100.0;
        let desc = elided(ui, row.description, font(11.5), look.ink.dim, r.max.x - right_w - tx);
        ui.painter().galley(pos2(tx, r.min.y + 31.0), desc, look.ink.dim);
        // gear + switch
        let gear = Rect::from_center_size(pos2(r.max.x - 82.0, r.center().y), vec2(26.0, 26.0));
        if small_icon_button(ui, gear, glyph::SETTINGS, look, Id::new(("panel-gear", row.id)), "Widget settings") {
            app.widget_settings = Some(row.id);
            app.widgets_panel_open = false;
        }
        if row.id == "launcher" {
            ui.painter().text(pos2(r.max.x - 14.0, r.center().y), Align2::RIGHT_CENTER, "Always on", font(11.5), look.ink.dim);
        } else {
            let sw = Rect::from_center_size(pos2(r.max.x - 34.0, r.center().y), vec2(40.0, 22.0));
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(sw));
            if switch(&mut child, open, Id::new(("panel-switch", row.id))) {
                app.set_open(row.id, !open);
            }
        }
    }

    // "More widgets" placeholder row with a dashed outline.
    let r = Rect::from_min_size(pos2(full.min.x, y), vec2(full.width(), row_h));
    if r.max.y <= full.max.y + 4.0 {
        let pts = super::launcher::rounded_points(r.shrink(0.75), 16.0);
        ui.painter().extend(egui::Shape::dashed_line(&pts, Stroke::new(1.0_f32, crate::theme::white(40)), 5.0, 4.0));
        let chip = Rect::from_min_size(pos2(r.min.x + 12.0, r.center().y - 18.0), vec2(36.0, 36.0));
        ui.painter().rect_filled(chip, CornerRadius::same(11), crate::theme::white(10));
        ui.painter().text(chip.center(), Align2::CENTER_CENTER, glyph::ADD.to_string(), egui::FontId::new(14.0, egui::FontFamily::Name("icons".into())), look.ink.dim);
        let tx = chip.max.x + 14.0;
        ui.painter().text(pos2(tx, r.min.y + 18.0), Align2::LEFT_CENTER, "More widgets", semibold(14.0), look.ink.dim);
        let desc = elided(ui, "New widgets register themselves with this launcher and show up here automatically.", font(11.5), look.ink.faint, r.max.x - tx - 12.0);
        ui.painter().galley(pos2(tx, r.min.y + 31.0), desc, look.ink.faint);
    }
    });
}
