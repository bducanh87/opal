//! The app launcher: a floating icon grid with Start-menu search, pinning,
//! drag-to-reorder, drop-to-add and per-item actions (Lumen's dock).

use egui::{RichText, Align2, Color32, Context, CornerRadius, Id, Key, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use std::path::Path;

use super::{elided, font, semibold};
use crate::app::{App, glyph};
use crate::config::Item;
use crate::glass::Look;

#[derive(Default)]
pub struct LauncherUi {
    pub query: String,
    pub renaming: Option<(String, String)>,
    pub focus_search: bool,
    /// Tile that was just clicked, for the launch glow.
    pub launching: Option<(String, std::time::Instant)>,
}

/// First run: a few familiar shortcuts, like Lumen's seeded dock.
pub fn seed_defaults(app: &mut App) {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let candidates = [
        (r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe".to_string(), "Edge", "app"),
        (format!("{windir}\\explorer.exe"), "File Explorer", "app"),
        ("ms-settings:".to_string(), "Settings", "url"),
        (format!("{windir}\\System32\\notepad.exe"), "Notepad", "app"),
        (format!("{windir}\\System32\\calc.exe"), "Calculator", "app"),
    ];
    if app.state.items.is_empty() {
        for (path, name, kind) in candidates {
            if kind == "url" || Path::new(&path).exists() {
                app.state.items.push(Item::new(name.into(), path, kind));
            }
        }
    }
    app.state.settings.seeded = true;
    app.mark_state();
}

fn read_url_shortcut(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines()
        .find_map(|l| l.trim().strip_prefix("URL=").map(|u| u.trim().to_string()))
}

pub fn add_path(app: &mut App, path: &Path) {
    let target = path.to_string_lossy().to_string();
    if app.state.items.iter().any(|i| i.target.eq_ignore_ascii_case(&target)) {
        app.notify(format!("{} is already pinned", crate::programs::pretty_name(&target)));
        return;
    }
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let item = if path.is_dir() {
        Item::new(crate::programs::pretty_name(&target), target, "folder")
    } else if ext == "url" {
        match read_url_shortcut(path) {
            Some(url) => {
                let mut i = Item::new(stem, url, "url");
                i.icon_source = Some(target);
                i
            }
            None => Item::new(stem, target, "file"),
        }
    } else if ext == "lnk" {
        Item::new(stem, target, "file")
    } else if ext == "exe" {
        Item::new(crate::programs::pretty_name(&target), target, "app")
    } else {
        Item::new(stem, target, "file")
    };
    app.state.items.push(item);
    app.mark_state();
}

pub fn launch_item(app: &mut App, index: usize) {
    let Some(item) = app.state.items.get(index).cloned() else { return };
    let result = if item.kind == "aumid" {
        crate::win::shell_open(&format!("shell:AppsFolder\\{}", item.target), None, None)
    } else if item.kind == "shell" {
        // Shell places (Recycle Bin, This PC…) open through Explorer.
        crate::win::shell_open("explorer.exe", Some(&item.target), None)
    } else {
        crate::win::shell_open(&item.target, item.args.as_deref(), item.cwd.as_deref())
    };
    match result {
        Ok(()) => {
            app.launcher_ui.launching = Some((app.state.items[index].id.clone(), std::time::Instant::now()));
            let it = &mut app.state.items[index];
            it.launch_count += 1;
            it.last_used = Some(crate::config::now_ms());
            app.mark_state();
        }
        Err(e) => app.notify(e),
    }
}

fn launch_program(app: &mut App, p: &crate::programs::Program) {
    let target = if p.kind == "aumid" { format!("shell:AppsFolder\\{}", p.target) } else { p.target.clone() };
    if let Err(e) = crate::win::shell_open(&target, None, None) {
        app.notify(e);
    }
}

fn pin_program(app: &mut App, p: &crate::programs::Program) {
    if app.state.items.iter().any(|i| i.target.eq_ignore_ascii_case(&p.target)) {
        app.notify(format!("{} is already pinned", p.name));
        return;
    }
    let mut item = Item::new(p.name.clone(), p.target.clone(), p.kind);
    item.icon_source = Some(p.icon_path());
    app.state.items.push(item);
    app.mark_state();
    app.notify(format!("Pinned {}", p.name));
}

/// Inbox apps such as Calculator or Notepad also exist as Store apps with
/// proper colour logos; prefer that logo over the old exe icon.
fn store_icon(app: &App, item: &Item) -> Option<String> {
    if item.kind != "app" || item.icon_source.is_some() {
        return None;
    }
    app.programs
        .iter()
        .find(|p| p.kind == "aumid" && p.name.eq_ignore_ascii_case(&item.name))
        .map(|p| p.icon_path())
}

/// A menu entry: icon glyph + label, full-width hover row.
fn menu_row(ui: &mut Ui, ch: char, label: &str, look: &Look) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width().max(200.0), 30.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(8), look.ink.hover_strong);
    }
    ui.painter().text(
        pos2(rect.min.x + 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        ch.to_string(),
        egui::FontId::new(12.0, egui::FontFamily::Name("icons".into())),
        look.ink.dim,
    );
    ui.painter().text(pos2(rect.min.x + 36.0, rect.center().y), Align2::LEFT_CENTER, label, font(13.0), look.ink.ink);
    resp.clicked()
}

/// Icon image, or a letter tile in the item's accent colour.
pub fn draw_icon(app: &mut App, ui: &Ui, rect: Rect, icon_path: &str, name: &str) {
    if let Some(tex) = app.icons.get(icon_path) {
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        ui.painter().image(tex.id(), rect, uv, Color32::WHITE);
    } else {
        let accent = crate::theme::accent_for(name);
        let cr = CornerRadius::same((rect.width() * 0.26) as u8);
        ui.painter().rect_filled(rect, cr, accent.gamma_multiply(0.9));
        let letter = name.chars().next().unwrap_or('?').to_uppercase().to_string();
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, letter, semibold(rect.height() * 0.5), Color32::WHITE);
    }
}

pub fn show(app: &mut App, ui: &mut Ui, ctx: &Context, look: &Look) {
    // Files dropped from Explorer are pinned.
    let dropped: Vec<_> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
    for path in dropped {
        add_path(app, &path);
    }

    header(app, ui, look);
    ui.add_space(10.0);

    if let Some((id, mut name)) = app.launcher_ui.renaming.take() {
        let mut keep = true;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Rename").color(look.ink.dim));
            let r = ui.add(egui::TextEdit::singleline(&mut name).desired_width(ui.available_width() - 80.0));
            r.request_focus();
            let enter = ui.input(|i| i.key_pressed(Key::Enter));
            if ui.button("Save").clicked() || enter {
                if let Some(item) = app.state.items.iter_mut().find(|i| i.id == id) {
                    if !name.trim().is_empty() {
                        item.name = name.trim().to_string();
                    }
                }
                app.mark_state();
                keep = false;
            }
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                keep = false;
            }
        });
        if keep {
            app.launcher_ui.renaming = Some((id, name));
        }
        ui.add_space(6.0);
    }

    if !app.launcher_ui.query.trim().is_empty() {
        results(app, ui, look);
    } else {
        grid(app, ui, look);
    }

}

fn pill_button(ui: &mut Ui, ch: char, active: bool, look: &Look, tip: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(10), look.ink.hover);
    }
    let color = if resp.hovered() || active { look.ink.ink } else { look.ink.dim };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        ch.to_string(),
        egui::FontId::new(14.0, egui::FontFamily::Name("icons".into())),
        color,
    );
    resp.on_hover_text(tip).clicked()
}

/// Lumen's search pill: icon, input, divider and the widget shortcuts.
fn header(app: &mut App, ui: &mut Ui, look: &Look) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, 46.0), Sense::hover());
    let cr = CornerRadius::same(23);
    let fill = if look.light {
        crate::theme::css(255, 255, 255, 107)
    } else {
        crate::theme::white(22)
    };
    let p = ui.painter();
    p.rect_filled(rect, cr, fill);
    p.rect_stroke(rect, cr, Stroke::new(1.0_f32, look.ink.hair), StrokeKind::Inside);
    p.line_segment(
        [pos2(rect.min.x + 20.0, rect.min.y + 1.5), pos2(rect.max.x - 20.0, rect.min.y + 1.5)],
        Stroke::new(1.0_f32, crate::theme::white(if look.light { 120 } else { 22 })),
    );
    p.text(
        pos2(rect.min.x + 18.0, rect.center().y),
        Align2::LEFT_CENTER,
        glyph::SEARCH.to_string(),
        egui::FontId::new(14.0, egui::FontFamily::Name("icons".into())),
        look.ink.dim,
    );
    let buttons_w = 5.0 * 32.0 + 12.0;
    let edit = Rect::from_min_max(
        pos2(rect.min.x + 42.0, rect.min.y + 13.0),
        pos2(rect.max.x - buttons_w - 10.0, rect.max.y - 10.0),
    );
    if app.state.settings.show_search {
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(edit));
        let resp = child.add(
            egui::TextEdit::singleline(&mut app.launcher_ui.query)
                .frame(false)
                .hint_text(egui::RichText::new("Search apps...").color(look.ink.faint))
                .text_color(look.ink.ink)
                .font(font(14.5))
                .desired_width(edit.width()),
        );
        if app.launcher_ui.focus_search {
            resp.request_focus();
            app.launcher_ui.focus_search = false;
        }
        if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            let first = crate::programs::search(&app.programs, &app.launcher_ui.query, 1).first().map(|p| (*p).clone());
            if let Some(p) = first {
                launch_program(app, &p);
                app.launcher_ui.query.clear();
            }
        }
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            app.launcher_ui.query.clear();
        }
    }
    let div_x = rect.max.x - buttons_w;
    ui.painter().line_segment(
        [pos2(div_x, rect.center().y - 9.0), pos2(div_x, rect.center().y + 9.0)],
        Stroke::new(1.0_f32, look.ink.hair),
    );
    let row = Rect::from_min_max(pos2(div_x + 6.0, rect.min.y + 8.0), pos2(rect.max.x - 6.0, rect.max.y - 8.0));
    let mut bar = ui.new_child(
        egui::UiBuilder::new().max_rect(row).layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    bar.spacing_mut().item_spacing.x = 2.0;
    if pill_button(&mut bar, glyph::APPS, app.widgets_panel_open, look, "Widgets") {
        app.widgets_panel_open = !app.widgets_panel_open;
    }
    for (id, ch, tip) in [("music", glyph::MUSIC, "Music"), ("notes", glyph::NOTE, "Notes"), ("calendar", glyph::CALENDAR, "Calendar")] {
        let open = app.is_open(id);
        if pill_button(&mut bar, ch, open, look, tip) {
            app.set_open(id, !open);
        }
    }
    if pill_button(&mut bar, glyph::SETTINGS, app.widget_settings == Some("launcher"), look, "Dock settings") {
        app.widget_settings = if app.widget_settings == Some("launcher") { None } else { Some("launcher") };
    }
}

fn results(app: &mut App, ui: &mut Ui, look: &Look) {
    let hits: Vec<crate::programs::Program> = crate::programs::search(&app.programs, &app.launcher_ui.query, 24)
        .into_iter()
        .cloned()
        .collect();
    if hits.is_empty() {
        let msg = if app.programs.is_empty() { "Loading programs…" } else { "No matching apps" };
        ui.add_space(12.0);
        ui.label(egui::RichText::new(msg).color(look.ink.dim));
        return;
    }
    egui::ScrollArea::vertical().id_salt("launcher-results").auto_shrink([false, false]).show(ui, |ui| {
        for (n, p) in hits.iter().enumerate() {
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
            let resp = resp.on_hover_text(&p.target);
            if resp.hovered() || n == 0 {
                ui.painter().rect_filled(rect, CornerRadius::same(10), if resp.hovered() { look.ink.hover } else { look.ink.hover.gamma_multiply(0.5) });
            }
            let icon = Rect::from_min_size(pos2(rect.min.x + 8.0, rect.center().y - 13.0), vec2(26.0, 26.0));
            draw_icon(app, ui, icon, &p.icon_path(), &p.name);
            let g = elided(ui, &p.name, font(14.0), look.ink.ink, rect.width() - 100.0);
            ui.painter().galley(pos2(icon.max.x + 12.0, rect.center().y - g.size().y / 2.0), g, look.ink.ink);
            let pin_rect = Rect::from_center_size(pos2(rect.max.x - 22.0, rect.center().y), vec2(30.0, 30.0));
            let pin = ui.interact(pin_rect, Id::new(("pin", &p.target)), Sense::click()).on_hover_text("Pin to launcher");
            if resp.hovered() || pin.hovered() {
                ui.painter().text(
                    pin_rect.center(),
                    Align2::CENTER_CENTER,
                    glyph::PIN.to_string(),
                    egui::FontId::new(14.0, egui::FontFamily::Name("icons".into())),
                    if pin.hovered() { look.ink.ink } else { look.ink.dim },
                );
            }
            if pin.clicked() {
                pin_program(app, p);
            } else if resp.clicked() {
                launch_program(app, p);
                app.launcher_ui.query.clear();
            }
        }
    });
}

fn grid(app: &mut App, ui: &mut Ui, look: &Look) {
    let s = &app.state.settings;
    let cols = s.columns.max(1) as usize;
    let icon = s.icon_size as f32;
    let labels = s.show_labels;
    let width = ui.available_width();
    let cell_w = width / cols as f32;
    let cell_h = icon + if labels { 40.0 } else { 18.0 };
    let mut action: Option<(usize, &'static str)> = None;
    let mut reorder: Option<(usize, usize)> = None;
    let mut add_clicked = false;

    egui::ScrollArea::vertical().id_salt("launcher-grid").auto_shrink([false, false]).show(ui, |ui| {
        let rows = (app.state.items.len() + 1).div_ceil(cols);
        let (area, _) = ui.allocate_exact_size(vec2(width, rows as f32 * cell_h), Sense::hover());
        for index in 0..app.state.items.len() {
            let (row, col) = (index / cols, index % cols);
            let cell = Rect::from_min_size(
                pos2(area.min.x + col as f32 * cell_w, area.min.y + row as f32 * cell_h),
                vec2(cell_w, cell_h),
            );
            let item = app.state.items[index].clone();
            let resp = ui.interact(cell.shrink(2.0), Id::new(("tile", &item.id)), Sense::click_and_drag());
            if resp.dragged() {
                resp.dnd_set_drag_payload(index);
            }
            let dropping = resp.dnd_hover_payload::<usize>().is_some_and(|from| *from != index);
            if let Some(from) = resp.dnd_release_payload::<usize>() {
                reorder = Some((*from, index));
            }
            let hovered = resp.hovered() && !resp.dragged();
            let tile = Rect::from_min_max(pos2(cell.min.x + 2.0, cell.min.y + 2.0), pos2(cell.max.x - 2.0, cell.max.y - 2.0));
            // Launch feedback (Lumen's .tile.is-launching): a glow that fades.
            let launch_t = app
                .launcher_ui
                .launching
                .as_ref()
                .filter(|(lid, _)| *lid == item.id)
                .map(|(_, t)| t.elapsed().as_secs_f32() / 0.9)
                .filter(|t| *t < 1.0);
            if let Some(t) = launch_t {
                let a = 1.0 - t;
                let glow = look.ink.accent;
                for k in 1..=4 {
                    ui.painter().rect_stroke(
                        tile.expand(k as f32 * 1.5),
                        CornerRadius::same(18 + k * 2),
                        Stroke::new(1.5_f32, glow.gamma_multiply(a * (0.5 - k as f32 * 0.1))),
                        StrokeKind::Outside,
                    );
                }
                ui.painter().rect_filled(tile, CornerRadius::same(18), look.ink.hover_strong.gamma_multiply(0.5 + a * 0.5));
                ui.painter().rect_stroke(tile, CornerRadius::same(18), Stroke::new(1.0_f32, glow.gamma_multiply(a)), StrokeKind::Inside);
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
            } else if hovered || dropping {
                tile_hover(ui, tile, look, dropping);
            }
            let pressed = resp.is_pointer_button_down_on() && !resp.dragged();
            let (lift, scale) = if pressed {
                (0.0, 0.97)
            } else if let Some(t) = launch_t {
                // a small bounce
                (-4.0 * (t * std::f32::consts::PI).sin() * (1.0 - t), 1.0 + 0.06 * (1.0 - t))
            } else if hovered {
                (-1.5, 1.055)
            } else {
                (0.0, 1.0)
            };
            let icon_rect = Rect::from_center_size(
                pos2(cell.center().x, cell.min.y + 10.0 + icon / 2.0 + lift),
                vec2(icon, icon) * scale,
            );
            let icon_path = store_icon(app, &item).unwrap_or_else(|| item.icon_path());
            draw_icon_shadowed(app, ui, icon_rect, &icon_path, &item.name);
            if resp.dragged() {
                ui.painter().rect_filled(icon_rect, CornerRadius::same(12), Color32::from_black_alpha(90));
            }
            if labels {
                tile_label(ui, &item.name, cell, cell.min.y + 16.0 + icon, look, false);
            }
            let resp = resp.on_hover_text(&item.name);
            if resp.clicked() {
                action = Some((index, "open"));
            }
            resp.context_menu(|ui| {
                ui.set_min_width(170.0);
                if ui.button("Open").clicked() {
                    action = Some((index, "open"));
                    ui.close_menu();
                }
                if ui.button("Rename").clicked() {
                    action = Some((index, "rename"));
                    ui.close_menu();
                }
                if Path::new(&item.target).exists() && ui.button("Show in Explorer").clicked() {
                    action = Some((index, "reveal"));
                    ui.close_menu();
                }
                if ui.button("Refresh icon").clicked() {
                    action = Some((index, "icon"));
                    ui.close_menu();
                }
                ui.separator();
                if index > 0 && ui.button("Move left").clicked() {
                    action = Some((index, "left"));
                    ui.close_menu();
                }
                if index + 1 < app.state.items.len() && ui.button("Move right").clicked() {
                    action = Some((index, "right"));
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Remove from Opal").clicked() {
                    action = Some((index, "remove"));
                    ui.close_menu();
                }
            });
        }
        // Lumen's dashed "Add" tile closes the grid.
        let index = app.state.items.len();
        let (row, col) = (index / cols, index % cols);
        let cell = Rect::from_min_size(
            pos2(area.min.x + col as f32 * cell_w, area.min.y + row as f32 * cell_h),
            vec2(cell_w, cell_h),
        );
        let resp = ui.interact(cell.shrink(2.0), Id::new("tile-add"), Sense::click()).on_hover_text("Add apps, files or folders");
        let hovered = resp.hovered();
        let scale = if hovered { 1.04 } else { 1.0 };
        let icon_rect = Rect::from_center_size(pos2(cell.center().x, cell.min.y + 10.0 + icon / 2.0), vec2(icon, icon) * scale);
        let cr = CornerRadius::same(13);
        ui.painter().rect_filled(icon_rect, cr, if hovered { look.ink.hover_strong } else { crate::theme::white(13) });
        let dash = crate::theme::white(if hovered { 128 } else { 77 });
        let pts = rounded_points(icon_rect.shrink(0.75), 13.0);
        ui.painter().extend(egui::Shape::dashed_line(&pts, Stroke::new(1.5_f32, dash), 4.0, 3.5));
        ui.painter().text(
            icon_rect.center(),
            Align2::CENTER_CENTER,
            glyph::ADD.to_string(),
            egui::FontId::new(icon * 0.32, egui::FontFamily::Name("icons".into())),
            if hovered { look.ink.ink } else { crate::theme::white(173) },
        );
        if labels {
            tile_label(ui, "Add", cell, cell.min.y + 16.0 + icon, look, true);
        }
        let popup_id = Id::new("add-anything");
        if resp.clicked() {
            ui.memory_mut(|m| m.toggle_popup(popup_id));
        }
        egui::popup_above_or_below_widget(ui, popup_id, &resp, egui::AboveOrBelow::Above, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
            ui.set_min_width(210.0);
            ui.label(RichText::new("Add anything").font(semibold(13.0)).color(look.ink.ink));
            let caps: String = "FILES · SHORTCUTS · PLACES".chars().flat_map(|c| [c, '\u{200A}']).collect();
            ui.label(RichText::new(caps).font(semibold(8.5)).color(look.ink.faint));
            ui.add_space(4.0);
            if menu_row(ui, glyph::FOLDER, "Browse files & apps…", look) {
                add_clicked = true;
                ui.memory_mut(|m| m.close_popup());
            }
            ui.separator();
            for (glyph_ch, name, target) in [
                (glyph::DELETE, "Recycle Bin", "::{645FF040-5081-101B-9F08-00AA002F954E}"),
                ('\u{E7F8}', "This PC", "::{20D04FE0-3AEA-1069-A2D8-08002B30309D}"),
            ] {
                if menu_row(ui, glyph_ch, name, look) {
                    if app.state.items.iter().any(|i| i.target == target) {
                        app.notify(format!("{name} is already pinned"));
                    } else {
                        let mut item = Item::new(name.into(), target.into(), "shell");
                        item.icon_source = Some(target.into());
                        app.state.items.push(item);
                        app.mark_state();
                    }
                    ui.memory_mut(|m| m.close_popup());
                }
            }
        });
        if false {
            add_clicked = true;
        }
    });

    if add_clicked {
        if let Some(files) = rfd::FileDialog::new().set_title("Add to Opal").pick_files() {
            for f in files {
                add_path(app, &f);
            }
        }
    }
    if let Some((from, to)) = reorder {
        if from < app.state.items.len() && to < app.state.items.len() {
            let item = app.state.items.remove(from);
            app.state.items.insert(to, item);
            app.mark_state();
        }
    }
    if let Some((index, what)) = action {
        match what {
            "open" => launch_item(app, index),
            "rename" => {
                let it = &app.state.items[index];
                app.launcher_ui.renaming = Some((it.id.clone(), it.name.clone()));
            }
            "reveal" => crate::win::reveal(&app.state.items[index].target),
            "icon" => {
                let path = app.state.items[index].icon_path();
                app.icons.forget(&path);
            }
            "left" => {
                app.state.items.swap(index, index - 1);
                app.mark_state();
            }
            "right" => {
                app.state.items.swap(index, index + 1);
                app.mark_state();
            }
            "remove" => {
                let removed = app.state.items.remove(index);
                app.mark_state();
                app.notify(format!("Removed {}", removed.name));
            }
            _ => {}
        }
    }
}

fn tile_hover(ui: &Ui, tile: Rect, look: &Look, target: bool) {
    let cr = CornerRadius::same(18);
    let p = ui.painter();
    p.rect_filled(tile, cr, if target { look.ink.accent.gamma_multiply(0.18) } else { look.ink.hover });
    p.rect_stroke(tile, cr, Stroke::new(1.0_f32, if target { look.ink.accent } else { look.ink.hair }), StrokeKind::Inside);
    p.line_segment(
        [pos2(tile.min.x + 14.0, tile.min.y + 1.5), pos2(tile.max.x - 14.0, tile.min.y + 1.5)],
        Stroke::new(1.0_f32, crate::theme::white(if look.light { 140 } else { 34 })),
    );
}

fn tile_label(ui: &Ui, text: &str, cell: Rect, y: f32, look: &Look, faint: bool) {
    let color = if faint { look.ink.faint } else { look.ink.ink.gamma_multiply(0.92) };
    let g = elided(ui, text, semibold(11.5), color, cell.width() - 8.0);
    let pos = pos2(cell.center().x - g.size().x / 2.0, y);
    let shadow = if look.light { crate::theme::white(120) } else { Color32::from_black_alpha(110) };
    let gs = elided(ui, text, semibold(11.5), shadow, cell.width() - 8.0);
    ui.painter().galley(pos + vec2(0.0, 1.0), gs, shadow);
    ui.painter().galley(pos, g, color);
}

/// Icon with Lumen's `drop-shadow(0 6px 10px #0006)`, approximated.
fn draw_icon_shadowed(app: &mut App, ui: &Ui, rect: Rect, icon_path: &str, name: &str) {
    if let Some(tex) = app.icons.get(icon_path) {
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        for (dy, a) in [(6.0, 26u8), (4.0, 22), (2.0, 18)] {
            ui.painter().image(tex.id(), rect.translate(vec2(0.0, dy)).expand(1.0), uv, Color32::from_black_alpha(a));
        }
        ui.painter().image(tex.id(), rect, uv, Color32::WHITE);
    } else {
        draw_icon(app, ui, rect, icon_path, name);
    }
}

pub fn rounded_points(r: Rect, rad: f32) -> Vec<egui::Pos2> {
    let rad = rad.min(r.width() / 2.0).min(r.height() / 2.0);
    let mut pts = Vec::new();
    let corners = [
        (pos2(r.max.x - rad, r.min.y + rad), -90.0_f32),
        (pos2(r.max.x - rad, r.max.y - rad), 0.0),
        (pos2(r.min.x + rad, r.max.y - rad), 90.0),
        (pos2(r.min.x + rad, r.min.y + rad), 180.0),
    ];
    for (c, start) in corners {
        for k in 0..=8 {
            let a = (start + 90.0 * k as f32 / 8.0).to_radians();
            pts.push(c + vec2(a.cos(), a.sin()) * rad);
        }
    }
    pts.push(pts[0]);
    pts
}
