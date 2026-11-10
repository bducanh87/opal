//! To-do list: add tasks, tick them off, delete, clear completed. Saved in
//! `config/opal-extras.json`.

use egui::{Align2, Color32, CornerRadius, Id, Key, Rect, RichText, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use super::{dot_header, elided, font, semibold};
use crate::app::{App, glyph};
use crate::config::Todo;
use crate::glass::Look;
use crate::theme::{rgb, white};

#[derive(Default)]
pub struct TodoUi {
    pub draft: String,
}

pub fn show(app: &mut App, ui: &mut Ui, look: &Look) {
    let r = ui.max_rect();
    let p = ui.painter().clone();
    let left = app.extras.todos.iter().filter(|t| !t.done).count();
    let body = dot_header(&p, r, "To-do", &format!("{left} left"), look);

    // Input pill.
    let input = Rect::from_min_size(body.min, vec2(body.width(), 34.0));
    p.rect_filled(input, CornerRadius::same(17), Color32::from_black_alpha(46));
    p.rect_stroke(input, CornerRadius::same(17), Stroke::new(1.0_f32, look.ink.hair), StrokeKind::Inside);
    let plus = Rect::from_center_size(pos2(input.max.x - 17.0, input.center().y), vec2(26.0, 26.0));
    let edit = Rect::from_min_max(pos2(input.min.x + 14.0, input.min.y + 8.0), pos2(plus.min.x - 6.0, input.max.y - 6.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(edit));
    let resp = child.add(
        egui::TextEdit::singleline(&mut app.todo_ui.draft)
            .frame(false)
            .hint_text(RichText::new("Add a task").color(look.ink.faint))
            .font(font(13.0))
            .text_color(look.ink.ink)
            .desired_width(edit.width()),
    );
    let pr = ui.interact(plus, Id::new("todo-add"), Sense::click());
    p.circle_filled(plus.center(), 13.0, if pr.hovered() { look.ink.hover_strong } else { white(22) });
    p.text(plus.center(), Align2::CENTER_CENTER, glyph::ADD.to_string(), egui::FontId::new(10.0, egui::FontFamily::Name("icons".into())), look.ink.ink);
    let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
    if (enter || pr.clicked()) && !app.todo_ui.draft.trim().is_empty() {
        let text = app.todo_ui.draft.trim().to_string();
        app.extras.todos.insert(0, Todo { id: crate::config::uid(), text, done: false, created_at: crate::config::now_ms() });
        app.todo_ui.draft.clear();
        app.mark_extras();
        if enter {
            resp.request_focus();
        }
    }

    // List: open tasks first, then done ones.
    let footer_h = 24.0;
    let list = Rect::from_min_max(pos2(body.min.x, input.max.y + 8.0), pos2(body.max.x, body.max.y - footer_h));
    let mut order: Vec<usize> = (0..app.extras.todos.len()).collect();
    order.sort_by_key(|&i| app.extras.todos[i].done);
    let mut toggle = None;
    let mut delete = None;
    let design = app.state.settings.design("todo");
    let row_h = match design.as_str() {
        "compact" => 24.0,
        "paper" => 30.0,
        _ => 32.0,
    };
    let mut list_ui = ui.new_child(egui::UiBuilder::new().max_rect(list));
    egui::ScrollArea::vertical().id_salt("todo-list").auto_shrink([false, false]).show(&mut list_ui, |ui| {
        if order.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| ui.label(RichText::new("Nothing to do — enjoy").color(look.ink.faint).size(12.0)));
        }
        for (n, &i) in order.iter().enumerate() {
            let t = &app.extras.todos[i];
            let (row, resp) = ui.allocate_exact_size(vec2(ui.available_width(), row_h), Sense::click());
            let p = ui.painter();
            let color = if t.done { look.ink.faint } else { look.ink.ink };
            let (text_x, text_font) = match design.as_str() {
                "paper" => {
                    // ruled line, red margin, square box
                    p.line_segment([pos2(row.min.x, row.max.y - 0.5), pos2(row.max.x, row.max.y - 0.5)], Stroke::new(1.0_f32, crate::theme::css(70, 110, 170, 70)));
                    p.line_segment([pos2(row.min.x + 30.0, row.min.y), pos2(row.min.x + 30.0, row.max.y)], Stroke::new(1.0_f32, crate::theme::css(210, 80, 80, 120)));
                    let b = Rect::from_center_size(pos2(row.min.x + 14.0, row.center().y), vec2(13.0, 13.0));
                    p.rect_stroke(b, CornerRadius::same(2), Stroke::new(1.3_f32, look.ink.dim), StrokeKind::Inside);
                    if t.done {
                        let c = b.center();
                        p.line_segment([c + vec2(-4.0, 0.0), c + vec2(-1.0, 3.5)], Stroke::new(1.8_f32, rgb(0xc8282e)));
                        p.line_segment([c + vec2(-1.0, 3.5), c + vec2(5.0, -5.0)], Stroke::new(1.8_f32, rgb(0xc8282e)));
                    }
                    (row.min.x + 40.0, super::serif(14.5))
                }
                "compact" => {
                    if resp.hovered() {
                        p.rect_filled(row, CornerRadius::same(6), look.ink.hover);
                    }
                    let num_color = if t.done { look.ink.faint } else { rgb(0x5fd6c0) };
                    p.text(pos2(row.min.x + 6.0, row.center().y), Align2::LEFT_CENTER, format!("{:02}", n + 1), semibold(11.0), num_color);
                    (row.min.x + 30.0, font(12.0))
                }
                _ => {
                    if resp.hovered() {
                        p.rect_filled(row, CornerRadius::same(10), look.ink.hover);
                    }
                    let c = pos2(row.min.x + 16.0, row.center().y);
                    if t.done {
                        p.circle_filled(c, 8.0, rgb(0x34d399));
                        p.line_segment([c + vec2(-3.5, 0.0), c + vec2(-1.0, 3.0)], Stroke::new(1.6_f32, Color32::WHITE));
                        p.line_segment([c + vec2(-1.0, 3.0), c + vec2(4.0, -3.0)], Stroke::new(1.6_f32, Color32::WHITE));
                    } else {
                        p.circle_stroke(c, 8.0, Stroke::new(1.4_f32, look.ink.dim));
                    }
                    (row.min.x + 34.0, font(13.0))
                }
            };
            let g = elided(ui, &t.text, text_font, color, row.max.x - text_x - 36.0);
            let tp = pos2(text_x, row.center().y - g.size().y / 2.0);
            let gw = g.size().x;
            ui.painter().galley(tp, g, color);
            if t.done {
                ui.painter().line_segment([pos2(tp.x, row.center().y), pos2(tp.x + gw, row.center().y)], Stroke::new(1.0_f32, look.ink.faint));
            }
            let del = Rect::from_center_size(pos2(row.max.x - 16.0, row.center().y), vec2(24.0, 24.0));
            let dr = ui.interact(del, Id::new(("todo-del", &t.id)), Sense::click());
            if resp.hovered() || dr.hovered() {
                ui.painter().text(del.center(), Align2::CENTER_CENTER, glyph::DELETE.to_string(), egui::FontId::new(11.0, egui::FontFamily::Name("icons".into())), if dr.hovered() { look.ink.ink } else { look.ink.dim });
            }
            if dr.clicked() {
                delete = Some(i);
            } else if resp.clicked() {
                toggle = Some(i);
            }
        }
    });
    if let Some(i) = toggle {
        app.extras.todos[i].done = !app.extras.todos[i].done;
        app.mark_extras();
    }
    if let Some(i) = delete {
        app.extras.todos.remove(i);
        app.mark_extras();
    }

    // Footer.
    let done = app.extras.todos.iter().filter(|t| t.done).count();
    if done > 0 {
        let btn = Rect::from_min_size(pos2(body.max.x - 116.0, body.max.y - 22.0), vec2(116.0, 22.0));
        let br = ui.interact(btn, Id::new("todo-clear"), Sense::click());
        p.rect_filled(btn, CornerRadius::same(11), if br.hovered() { look.ink.hover_strong } else { white(14) });
        p.text(btn.center(), Align2::CENTER_CENTER, format!("Clear {done} done"), semibold(10.5), look.ink.dim);
        if br.clicked() {
            app.extras.todos.retain(|t| !t.done);
            app.mark_extras();
        }
    }
}
