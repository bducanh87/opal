//! Sticky notes: Pinboard, Notebook and Index designs, colours, pinning,
//! delete with undo. Stored in Lumen's `notes.json` format.

use egui::{Align2, Color32, CornerRadius, Id, Rect, RichText, Sense, Stroke, Ui, pos2, vec2};

use super::{elided, font, semibold, serif};
use crate::app::{App, glyph, icon_text};
use crate::config::{NOTE_COLORS, Note};
use crate::glass::Look;
use crate::theme::{note_color, rgb};

#[derive(Default)]
pub struct NotesUi {
    pub query: String,
    pub trash: Vec<Note>,
    pub focus: Option<String>,
    pub selected: Option<String>,
}

const MAX_NOTES: usize = 400;

fn order(app: &App) -> Vec<usize> {
    let q = app.notes_ui.query.trim().to_lowercase();
    let mut idx: Vec<usize> = (0..app.notes.notes.len())
        .filter(|&i| q.is_empty() || app.notes.notes[i].text.to_lowercase().contains(&q))
        .collect();
    idx.sort_by(|&a, &b| {
        let (na, nb) = (&app.notes.notes[a], &app.notes.notes[b]);
        nb.pinned.cmp(&na.pinned).then(nb.created_at.cmp(&na.created_at))
    });
    idx
}

fn new_note(app: &mut App) {
    if app.notes.notes.len() >= MAX_NOTES {
        app.notify("Note limit reached");
        return;
    }
    // A new note is empty, so a search would hide it.
    app.notes_ui.query.clear();
    let color = NOTE_COLORS[app.notes.notes.len() % NOTE_COLORS.len()];
    let note = Note::new(color);
    app.notes_ui.focus = Some(note.id.clone());
    app.notes_ui.selected = Some(note.id.clone());
    app.notes.notes.push(note);
    app.mark_notes();
}

fn delete(app: &mut App, id: &str) {
    if let Some(pos) = app.notes.notes.iter().position(|n| n.id == id) {
        let note = app.notes.notes.remove(pos);
        app.notes_ui.trash.push(note);
        if app.notes_ui.trash.len() > 20 {
            app.notes_ui.trash.remove(0);
        }
        app.mark_notes();
    }
}

fn header(app: &mut App, ui: &mut Ui, look: &Look, _title_font: egui::FontId) {
    let width = ui.available_width();
    let (row, _) = ui.allocate_exact_size(vec2(width, 34.0), Sense::hover());
    let p = ui.painter().clone();
    // icon chip
    let chip = Rect::from_min_size(row.min, vec2(32.0, 32.0));
    p.rect_filled(chip, CornerRadius::same(10), crate::theme::white(20));
    p.rect_stroke(chip, CornerRadius::same(10), Stroke::new(1.0_f32, look.ink.hair), egui::StrokeKind::Inside);
    p.text(chip.center(), Align2::CENTER_CENTER, glyph::NOTE.to_string(), egui::FontId::new(14.0, egui::FontFamily::Name("icons".into())), look.ink.ink);
    let title_x = chip.max.x + 10.0;
    let title_w = if width > 380.0 {
        let g = ui.fonts(|f| f.layout_no_wrap("STICKY NOTES".into(), semibold(11.5), look.ink.ink));
        let w = g.size().x;
        p.galley(pos2(title_x, row.center().y - g.size().y / 2.0), g, look.ink.ink);
        w + 14.0
    } else {
        0.0
    };
    // round + and gear on the right
    let plus = Rect::from_center_size(pos2(row.max.x - 52.0, row.center().y), vec2(30.0, 30.0));
    let gear = Rect::from_center_size(pos2(row.max.x - 14.0, row.center().y), vec2(26.0, 26.0));
    let pr = ui.interact(plus, Id::new("notes-new"), Sense::click()).on_hover_text("New note");
    p.circle_filled(plus.center(), 15.0, if pr.hovered() { look.ink.hover_strong } else { crate::theme::white(22) });
    p.circle_stroke(plus.center(), 15.0, Stroke::new(1.0_f32, crate::theme::white(60)));
    p.text(plus.center(), Align2::CENTER_CENTER, glyph::ADD.to_string(), egui::FontId::new(12.0, egui::FontFamily::Name("icons".into())), look.ink.ink);
    if pr.clicked() {
        new_note(app);
    }
    let gr = ui.interact(gear, Id::new("notes-gear"), Sense::click()).on_hover_text("Sticky notes settings");
    if gr.hovered() {
        p.rect_filled(gear, CornerRadius::same(8), look.ink.hover);
    }
    p.text(gear.center(), Align2::CENTER_CENTER, glyph::SETTINGS.to_string(), egui::FontId::new(13.0, egui::FontFamily::Name("icons".into())), look.ink.dim);
    if gr.clicked() {
        app.widget_settings = if app.widget_settings == Some("notes") { None } else { Some("notes") };
    }
    // search pill fills the middle
    let search = Rect::from_min_max(pos2(title_x + title_w, row.min.y + 1.0), pos2(plus.min.x - 10.0, row.max.y - 1.0));
    if search.width() > 60.0 {
        p.rect_filled(search, CornerRadius::same(16), Color32::from_black_alpha(46));
        p.rect_stroke(search, CornerRadius::same(16), Stroke::new(1.0_f32, look.ink.hair), egui::StrokeKind::Inside);
        p.text(pos2(search.min.x + 14.0, search.center().y), Align2::LEFT_CENTER, glyph::SEARCH.to_string(), egui::FontId::new(12.0, egui::FontFamily::Name("icons".into())), look.ink.dim);
        let edit = Rect::from_min_max(pos2(search.min.x + 34.0, search.min.y + 7.0), pos2(search.max.x - 10.0, search.max.y - 5.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(edit));
        child.add(
            egui::TextEdit::singleline(&mut app.notes_ui.query)
                .frame(false)
                .hint_text(RichText::new("Search notes").color(look.ink.faint))
                .font(font(13.0))
                .text_color(look.ink.ink)
                .desired_width(edit.width()),
        );
    }
    if !app.notes_ui.trash.is_empty() {
        let undo = Rect::from_center_size(pos2(plus.min.x - 22.0, row.center().y), vec2(24.0, 24.0));
        if search.width() <= 60.0 || true {
            let ur = ui.interact(undo, Id::new("notes-undo"), Sense::click()).on_hover_text("Restore deleted note");
            if ur.hovered() {
                ui.painter().rect_filled(undo, CornerRadius::same(8), look.ink.hover);
            }
            ui.painter().text(undo.center(), Align2::CENTER_CENTER, glyph::UNDO.to_string(), egui::FontId::new(12.0, egui::FontFamily::Name("icons".into())), look.ink.dim);
            if ur.clicked() && app.notes.notes.len() >= MAX_NOTES {
                app.notify("Note limit reached");
            } else if ur.clicked() {
                if let Some(mut n) = app.notes_ui.trash.pop() {
                    n.updated_at = crate::config::now_ms();
                    app.notes.notes.push(n);
                    app.mark_notes();
                }
            }
        }
    }
}

/// "12 notes · Open folder" footer.
fn footer(app: &App, ui: &mut Ui, look: &Look) {
    let r = ui.max_rect();
    let btn = Rect::from_min_size(pos2(r.max.x - 92.0, r.max.y - 26.0), vec2(90.0, 24.0));
    let resp = ui.interact(btn, Id::new("notes-folder"), Sense::click());
    let p = ui.painter();
    p.rect_filled(btn, CornerRadius::same(12), if resp.hovered() { look.ink.hover_strong } else { crate::theme::white(14) });
    p.rect_stroke(btn, CornerRadius::same(12), Stroke::new(1.0_f32, look.ink.hair), egui::StrokeKind::Inside);
    p.text(btn.left_center() + vec2(12.0, 0.0), Align2::LEFT_CENTER, glyph::FOLDER.to_string(), egui::FontId::new(10.0, egui::FontFamily::Name("icons".into())), look.ink.dim);
    p.text(btn.left_center() + vec2(28.0, 0.0), Align2::LEFT_CENTER, "Open folder", font(11.0), look.ink.dim);
    let n = app.notes.notes.len();
    p.text(pos2(btn.min.x - 8.0, btn.center().y), Align2::RIGHT_CENTER, format!("{n} note{}", if n == 1 { "" } else { "s" }), font(11.0), look.ink.faint);
    if resp.clicked() {
        crate::win::open_folder(&crate::config::config_dir());
    }
}

/// Inline editor; returns true when the text changed.
fn editor(ui: &mut Ui, note: &mut Note, font: egui::FontId, color: Color32, focus: bool, width: f32) -> bool {
    let resp = ui.add(
        egui::TextEdit::multiline(&mut note.text)
            .id_salt(("note", &note.id))
            .frame(false)
            .font(font)
            .text_color(color)
            .hint_text(RichText::new("Write something…").color(color.gamma_multiply(0.45)))
            .desired_rows(2)
            .desired_width(width)
            .lock_focus(true),
    );
    if focus {
        resp.request_focus();
    }
    if resp.changed() {
        if note.text.len() > 200_000 {
            // Cut on a character boundary (Vietnamese / emoji are multi-byte).
            let mut cut = 200_000;
            while !note.text.is_char_boundary(cut) {
                cut -= 1;
            }
            note.text.truncate(cut);
        }
        note.updated_at = crate::config::now_ms();
        true
    } else {
        false
    }
}

enum Act {
    Delete(String),
    Pin(String),
    Color(String, &'static str),
}

fn tools(ui: &mut Ui, note: &Note, ink: Color32, act: &mut Option<Act>) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        for c in NOTE_COLORS {
            let (fill, edge) = note_color(c);
            let (r, resp) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::click());
            ui.painter().circle_filled(r.center(), 6.0, fill);
            ui.painter().circle_stroke(r.center(), 6.0, Stroke::new(if note.color == c { 2.0 } else { 1.0_f32 }, edge));
            if resp.on_hover_text(c).clicked() {
                *act = Some(Act::Color(note.id.clone(), c));
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let del = ui.add(egui::Label::new(icon_text(glyph::DELETE, 12.0, ink.gamma_multiply(0.7))).sense(Sense::click()));
            if del.on_hover_text("Delete").clicked() {
                *act = Some(Act::Delete(note.id.clone()));
            }
            let pin_glyph = if note.pinned { glyph::UNPIN } else { glyph::PIN };
            let pin = ui.add(egui::Label::new(icon_text(pin_glyph, 12.0, ink.gamma_multiply(0.7))).sense(Sense::click()));
            if pin.on_hover_text(if note.pinned { "Unpin" } else { "Pin to top" }).clicked() {
                *act = Some(Act::Pin(note.id.clone()));
            }
        });
    });
}

fn apply(app: &mut App, act: Option<Act>) {
    match act {
        Some(Act::Delete(id)) => delete(app, &id),
        Some(Act::Pin(id)) => {
            if let Some(n) = app.notes.notes.iter_mut().find(|n| n.id == id) {
                n.pinned = !n.pinned;
            }
            app.mark_notes();
        }
        Some(Act::Color(id, c)) => {
            if let Some(n) = app.notes.notes.iter_mut().find(|n| n.id == id) {
                n.color = c.into();
            }
            app.mark_notes();
        }
        None => {}
    }
}

pub fn show(app: &mut App, ui: &mut Ui, look: &Look) {
    match app.state.settings.design("notes").as_str() {
        "notebook" => notebook(app, ui, look),
        "index" => index(app, ui, look),
        _ => pinboard(app, ui, look),
    }
}

fn empty(app: &mut App, ui: &mut Ui, look: &Look) {
    let r = ui.available_rect_before_wrap();
    let c = pos2(r.center().x, r.min.y + (r.height() * 0.42).max(70.0));
    let chip = Rect::from_center_size(c - vec2(0.0, 52.0), vec2(64.0, 64.0));
    let p = ui.painter().clone();
    p.rect_filled(chip, CornerRadius::same(18), crate::theme::white(22));
    p.rect_stroke(chip, CornerRadius::same(18), Stroke::new(1.0_f32, crate::theme::white(55)), egui::StrokeKind::Inside);
    p.text(chip.center(), Align2::CENTER_CENTER, glyph::NOTE.to_string(), egui::FontId::new(22.0, egui::FontFamily::Name("icons".into())), look.ink.ink);
    let searching = !app.notes_ui.query.trim().is_empty();
    let title = if searching { "No matching notes" } else { "Your glass board is clear" };
    p.text(c + vec2(0.0, 0.0), Align2::CENTER_CENTER, title, semibold(15.0), look.ink.ink);
    let sub = ui.fonts(|f| {
        let mut job = egui::text::LayoutJob::simple(
            "Capture a thought before it slips away. Every note autosaves as you type.".into(),
            font(12.0),
            look.ink.dim,
            (r.width() - 60.0).min(280.0),
        );
        job.halign = egui::Align::Center;
        f.layout_job(job)
    });
    let sub_h = sub.size().y;
    p.galley(pos2(c.x, c.y + 14.0), sub, look.ink.dim);
    if !searching {
        let btn = Rect::from_center_size(pos2(c.x, c.y + 30.0 + sub_h + 14.0), vec2(110.0, 34.0));
        let resp = ui.interact(btn, Id::new("notes-empty-new"), Sense::click());
        p.rect_filled(btn, CornerRadius::same(17), if resp.hovered() { look.ink.hover_strong } else { crate::theme::white(22) });
        p.rect_stroke(btn, CornerRadius::same(17), Stroke::new(1.0_f32, crate::theme::white(60)), egui::StrokeKind::Inside);
        p.text(btn.left_center() + vec2(20.0, 0.0), Align2::LEFT_CENTER, glyph::ADD.to_string(), egui::FontId::new(10.0, egui::FontFamily::Name("icons".into())), look.ink.ink);
        p.text(btn.left_center() + vec2(38.0, 0.0), Align2::LEFT_CENTER, "New note", semibold(12.5), look.ink.ink);
        if resp.clicked() {
            new_note(app);
        }
    }
}

fn pinboard(app: &mut App, ui: &mut Ui, look: &Look) {
    header(app, ui, look, semibold(17.0));
    ui.add_space(6.0);
    if order(app).is_empty() {
        empty(app, ui, look);
        footer(app, ui, look);
        return;
    }
    let mut act = None;
    let mut changed = false;
    let focus = app.notes_ui.focus.take();
    let ids = order(app);
    let width = ui.available_width();
    let cols = if width > 520.0 { 3 } else if width > 330.0 { 2 } else { 1 };
    let gap = 10.0;
    let col_w = (width - gap * (cols - 1) as f32) / cols as f32;
    let area = ui.available_rect_before_wrap();
    let list = Rect::from_min_max(area.min, pos2(area.max.x, area.max.y - 32.0));
    let mut list_ui = ui.new_child(egui::UiBuilder::new().max_rect(list));
    egui::ScrollArea::vertical().id_salt("notes-board").auto_shrink([false, false]).show(&mut list_ui, |ui| {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for c in 0..cols {
                ui.vertical(|ui| {
                    ui.set_width(col_w);
                    for (k, &i) in ids.iter().enumerate() {
                        if k % cols != c {
                            continue;
                        }
                        let note = &mut app.notes.notes[i];
                        let (fill, edge) = note_color(&note.color);
                        let ink = rgb(0x2a2622);
                        let frame = egui::Frame::new()
                            .fill(fill)
                            .corner_radius(CornerRadius::same(14))
                            .stroke(Stroke::new(1.0_f32, edge.gamma_multiply(0.5)))
                            .inner_margin(egui::Margin { left: 12, right: 10, top: 10, bottom: 6 });
                        let r = frame.show(ui, |ui| {
                            ui.set_width(col_w - 22.0);
                            if note.pinned {
                                ui.label(icon_text(glyph::PIN, 10.0, edge));
                            }
                            changed |= editor(ui, note, font(14.0), ink, focus.as_deref() == Some(&note.id), col_w - 22.0);
                            tools(ui, note, ink, &mut act);
                        });
                        let _ = r;
                        ui.add_space(gap - 4.0);
                    }
                });
            }
        });
    });
    footer(app, ui, look);
    if changed {
        app.mark_notes();
    }
    apply(app, act);
}

fn notebook(app: &mut App, ui: &mut Ui, look: &Look) {
    header(app, ui, look, serif(19.0));
    ui.add_space(4.0);
    if order(app).is_empty() {
        empty(app, ui, look);
        footer(app, ui, look);
        return;
    }
    let mut act = None;
    let mut changed = false;
    let focus = app.notes_ui.focus.take();
    let ids = order(app);
    let ink = rgb(0x2b2622);
    let rule = crate::theme::css(70, 110, 170, 40);
    let margin = crate::theme::css(210, 80, 80, 90);
    egui::ScrollArea::vertical().id_salt("notes-book").auto_shrink([false, false]).show(ui, |ui| {
        for &i in &ids {
            let note = &mut app.notes.notes[i];
            let top = ui.cursor().min;
            let width = ui.available_width();
            ui.horizontal(|ui| {
                ui.add_space(30.0);
                ui.vertical(|ui| {
                    let date = chrono::DateTime::from_timestamp_millis(note.created_at)
                        .map(|d| d.with_timezone(&chrono::Local).format("%-d %b %Y").to_string())
                        .unwrap_or_default();
                    ui.label(RichText::new(date).font(serif(11.5)).italics().color(ink.gamma_multiply(0.5)));
                    changed |= editor(ui, note, serif(15.0), ink, focus.as_deref() == Some(&note.id), width - 40.0);
                    tools(ui, note, ink, &mut act);
                });
            });
            let bottom = ui.cursor().min.y;
            let p = ui.painter();
            let mut y = top.y + 24.0;
            while y < bottom {
                p.line_segment([pos2(top.x, y), pos2(top.x + width, y)], Stroke::new(1.0_f32, rule));
                y += 22.0;
            }
            p.line_segment([pos2(top.x + 22.0, top.y), pos2(top.x + 22.0, bottom)], Stroke::new(1.0_f32, margin));
            ui.add_space(10.0);
        }
    });
    if changed {
        app.mark_notes();
    }
    apply(app, act);
}

fn index(app: &mut App, ui: &mut Ui, look: &Look) {
    header(app, ui, look, semibold(17.0));
    ui.add_space(4.0);
    if order(app).is_empty() {
        empty(app, ui, look);
        footer(app, ui, look);
        return;
    }
    let mut act = None;
    let mut changed = false;
    let focus = app.notes_ui.focus.take();
    let ids = order(app);
    let accent = rgb(0x5fd6c0);
    egui::ScrollArea::vertical().id_salt("notes-index").auto_shrink([false, false]).show(ui, |ui| {
        for (n, &i) in ids.iter().enumerate() {
            let note = &mut app.notes.notes[i];
            let selected = app.notes_ui.selected.as_deref() == Some(note.id.as_str());
            let width = ui.available_width();
            if selected {
                let frame = egui::Frame::new()
                    .fill(crate::theme::white(10))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(egui::Margin::same(8));
                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{:02}", n + 1)).font(semibold(13.0)).color(accent));
                        ui.vertical(|ui| {
                            changed |= editor(ui, note, font(14.0), look.ink.ink, focus.as_deref() == Some(&note.id), width - 60.0);
                            tools(ui, note, look.ink.ink, &mut act);
                        });
                    });
                });
            } else {
                let (rect, resp) = ui.allocate_exact_size(vec2(width, 34.0), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(rect, CornerRadius::same(8), look.ink.hover);
                }
                let (_, edge) = note_color(&note.color);
                ui.painter().text(pos2(rect.min.x + 8.0, rect.center().y), Align2::LEFT_CENTER, format!("{:02}", n + 1), semibold(13.0), accent);
                ui.painter().circle_filled(pos2(rect.min.x + 36.0, rect.center().y), 3.5, edge);
                let first = note.text.lines().find(|l| !l.trim().is_empty()).unwrap_or("Untitled");
                let g = elided(ui, first, font(14.0), look.ink.ink, rect.width() - 110.0);
                ui.painter().galley(pos2(rect.min.x + 48.0, rect.center().y - g.size().y / 2.0), g, look.ink.ink);
                let date = chrono::DateTime::from_timestamp_millis(note.updated_at)
                    .map(|d| d.with_timezone(&chrono::Local).format("%-d %b").to_string())
                    .unwrap_or_default();
                ui.painter().text(pos2(rect.max.x - 6.0, rect.center().y), Align2::RIGHT_CENTER, date, font(11.5), look.ink.faint);
                if note.pinned {
                    ui.painter().text(pos2(rect.max.x - 52.0, rect.center().y), Align2::RIGHT_CENTER, glyph::PIN.to_string(), egui::FontId::new(10.0, egui::FontFamily::Name("icons".into())), look.ink.faint);
                }
                if resp.clicked() {
                    app.notes_ui.selected = Some(note.id.clone());
                    app.notes_ui.focus = Some(note.id.clone());
                }
                ui.painter().line_segment([pos2(rect.min.x, rect.max.y), pos2(rect.max.x, rect.max.y)], Stroke::new(1.0_f32, look.ink.hair));
            }
        }
    });
    if changed {
        app.mark_notes();
    }
    apply(app, act);
    let _ = (Id::NULL, Rect::NOTHING);
}
