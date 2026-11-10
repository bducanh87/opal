//! Settings window: appearance, widgets and per-widget options.

use egui::{Align2, Color32, Context, CornerRadius, Id, Rect, RichText, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use super::{font, semibold};
use crate::app::{App, WIDGETS, glyph, icon_button};
use crate::glass::Look;
use crate::theme;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Tab {
    #[default]
    General,
    Appearance,
    Widgets,
    Launcher,
    Clock,
    Calendar,
    Notes,
    Music,
    About,
}

impl Tab {
    const ALL: [(Tab, &'static str); 9] = [
        (Tab::General, "General"),
        (Tab::Appearance, "Appearance"),
        (Tab::Widgets, "Widgets"),
        (Tab::Launcher, "Launcher"),
        (Tab::Clock, "Clock"),
        (Tab::Calendar, "Calendar"),
        (Tab::Notes, "Notes"),
        (Tab::Music, "Music"),
        (Tab::About, "About"),
    ];

    pub fn for_widget(id: &str) -> Tab {
        match id {
            "launcher" => Tab::Launcher,
            "clock" => Tab::Clock,
            "calendar" => Tab::Calendar,
            "notes" => Tab::Notes,
            "music" => Tab::Music,
            _ => Tab::General,
        }
    }
}

#[derive(Default)]
pub struct SettingsUi {
    pub tab: Tab,
    pub custom_zone: String,
}

pub const ZONES: &[&str] = &[
    "local",
    "Asia/Ho_Chi_Minh",
    "Asia/Bangkok",
    "Asia/Singapore",
    "Asia/Shanghai",
    "Asia/Tokyo",
    "Asia/Seoul",
    "Asia/Kolkata",
    "Asia/Dubai",
    "Europe/London",
    "Europe/Paris",
    "Europe/Berlin",
    "Europe/Moscow",
    "America/New_York",
    "America/Chicago",
    "America/Denver",
    "America/Los_Angeles",
    "America/Sao_Paulo",
    "Australia/Sydney",
    "Pacific/Auckland",
    "UTC",
];

fn section(ui: &mut Ui, title: &str, look: &Look) {
    ui.add_space(10.0);
    ui.label(RichText::new(title).font(semibold(13.0)).color(look.ink.dim));
    ui.add_space(2.0);
}

fn hint(ui: &mut Ui, text: &str, look: &Look) {
    ui.label(RichText::new(text).size(12.0).color(look.ink.faint));
}

fn slider(ui: &mut Ui, label: &str, value: &mut u32, range: std::ops::RangeInclusive<u32>, suffix: &str) -> bool {
    ui.horizontal(|ui| {
        ui.add_sized(vec2(130.0, 20.0), egui::Label::new(label));
        ui.add(egui::Slider::new(value, range).suffix(suffix)).changed()
    })
    .inner
}

pub fn show(app: &mut App, ui: &mut Ui, ctx: &Context, look: &Look) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Opal").font(semibold(18.0)).color(look.ink.ink));
        ui.label(RichText::new("Settings").font(font(18.0)).color(look.ink.dim));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icon_button(ui, glyph::CLOSE, 12.0, look, "Close").clicked() {
                app.settings_open = false;
            }
        });
    });
    ui.add_space(10.0);
    let body = ui.available_rect_before_wrap();
    let nav = Rect::from_min_size(body.min, vec2(150.0, body.height()));
    let content = Rect::from_min_max(pos2(nav.max.x + 16.0, body.min.y), body.max);

    let mut nav_ui = ui.new_child(egui::UiBuilder::new().max_rect(nav));
    for (tab, name) in Tab::ALL {
        let (r, resp) = nav_ui.allocate_exact_size(vec2(nav.width(), 32.0), Sense::click());
        let active = app.settings_ui.tab == tab;
        if active || resp.hovered() {
            nav_ui.painter().rect_filled(r, CornerRadius::same(9), if active { look.ink.hover_strong } else { look.ink.hover });
        }
        nav_ui.painter().text(
            pos2(r.min.x + 12.0, r.center().y),
            Align2::LEFT_CENTER,
            name,
            if active { semibold(14.0) } else { font(14.0) },
            if active { look.ink.ink } else { look.ink.dim },
        );
        if resp.clicked() {
            app.settings_ui.tab = tab;
        }
    }
    ui.painter().line_segment(
        [pos2(nav.max.x + 7.0, nav.min.y), pos2(nav.max.x + 7.0, nav.max.y)],
        Stroke::new(1.0_f32, look.ink.hair),
    );

    let mut c = ui.new_child(egui::UiBuilder::new().max_rect(content));
    egui::ScrollArea::vertical().id_salt("settings-scroll").auto_shrink([false, false]).show(&mut c, |ui| {
        ui.set_width(content.width() - 12.0);
        let before = app.state.settings.clone();
        match app.settings_ui.tab {
            Tab::General => general(app, ui, ctx, look),
            Tab::Appearance => appearance(app, ui, ctx, look),
            Tab::Widgets => widgets_tab(app, ui, look),
            Tab::Launcher => launcher(app, ui, look),
            Tab::Clock => clock(app, ui, ctx, look),
            Tab::Calendar => calendar(app, ui, ctx, look),
            Tab::Notes => {
                design_picker(app, ui, ctx, "notes", look);
                section(ui, "Storage", look);
                hint(ui, "Notes are saved as you type to config\\notes.json.", look);
                if ui.button("Open config folder").clicked() {
                    crate::win::open_folder(&crate::config::config_dir());
                }
            }
            Tab::Music => {
                design_picker(app, ui, ctx, "music", look);
                section(ui, "Source", look);
                hint(ui, "The music widget follows the media session Windows shows in its volume flyout: Spotify, YouTube Music or any site in your browser, Media Player, VLC and more. Start playback there, then use the widget to pause, skip and seek.", look);
            }
            Tab::About => about(app, ui, look),
        }
        if app.state.settings != before {
            app.state.settings.sanitize();
            app.mark_state();
        }
    });
}

fn general(app: &mut App, ui: &mut Ui, ctx: &Context, look: &Look) {
    section(ui, "Startup", look);
    let mut on = app.state.settings.start_with_windows;
    if ui.checkbox(&mut on, "Start Opal with Windows").changed() {
        match crate::win::set_start_with_windows(on) {
            Ok(()) => app.state.settings.start_with_windows = on,
            Err(e) => app.notify(e),
        }
    }
    section(ui, "Theme", look);
    theme_grid(app, ui, ctx, look, None);
    section(ui, "Layout", look);
    ui.horizontal(|ui| {
        if ui.button("Reset widget positions").clicked() {
            app.reset_positions();
        }
        if ui.button("Open config folder").clicked() {
            crate::win::open_folder(&crate::config::config_dir());
        }
    });
    section(ui, "Opal", look);
    if ui.button(RichText::new("Quit Opal").color(Color32::from_rgb(0xff, 0x8a, 0x8a))).clicked() {
        app.quit(ctx);
    }
}

fn theme_grid(app: &mut App, ui: &mut Ui, ctx: &Context, look: &Look, widget: Option<&str>) {
    let current = match widget {
        Some(w) => app.state.settings.widget_theme.get(w).cloned(),
        None => Some(app.state.settings.theme.clone()),
    };
    let w = 96.0;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
        if widget.is_some() {
            let (r, resp) = ui.allocate_exact_size(vec2(w, 64.0), Sense::click());
            ui.painter().rect_stroke(r, CornerRadius::same(12), Stroke::new(if current.is_none() { 2.0 } else { 1.0_f32 }, if current.is_none() { look.ink.accent } else { look.ink.hair }), StrokeKind::Inside);
            ui.painter().text(r.center(), Align2::CENTER_CENTER, "Same as Opal", font(12.0), look.ink.dim);
            if resp.clicked() {
                app.state.settings.widget_theme.remove(widget.unwrap());
            }
        }
        for t in theme::THEMES {
            let (r, resp) = ui.allocate_exact_size(vec2(w, 64.0), Sense::click());
            let tex = app.glass.theme_texture(ctx, &format!("theme:{}", t.id), || theme::render_theme(t));
            ui.painter().add(egui::Shape::Rect(
                egui::epaint::RectShape::filled(r, CornerRadius::same(12), Color32::WHITE)
                    .with_texture(tex.id(), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))),
            ));
            let selected = current.as_deref() == Some(t.id);
            ui.painter().rect_stroke(
                r,
                CornerRadius::same(12),
                Stroke::new(if selected { 2.5 } else { 1.0_f32 }, if selected { Color32::WHITE } else { crate::theme::white(40) }),
                StrokeKind::Inside,
            );
            let ink = if t.light { theme::rgb(0x1d2233) } else { Color32::WHITE };
            ui.painter().text(r.left_bottom() + vec2(10.0, -8.0), Align2::LEFT_BOTTOM, t.name, semibold(12.5), ink);
            if resp.clicked() {
                match widget {
                    Some(wid) => {
                        app.state.settings.widget_theme.insert(wid.into(), t.id.into());
                    }
                    None => app.state.settings.theme = t.id.into(),
                }
            }
        }
    });
}

fn appearance(app: &mut App, ui: &mut Ui, ctx: &Context, look: &Look) {
    let s = &mut app.state.settings;
    section(ui, "Glass", look);
    ui.checkbox(&mut s.wallpaper_frost, "Frosted wallpaper behind widgets");
    hint(ui, "Widgets show a blurred copy of your wallpaper, lined up with the desktop beneath them.", look);
    ui.add_enabled_ui(s.wallpaper_frost, |ui| {
        slider(ui, "Blur", &mut s.blur_size, 0..=80, " px");
        slider(ui, "Frost", &mut s.frost_strength, 0..=100, "%");
        slider(ui, "Theme tint", &mut s.veil, 0..=100, "%");
    });
    slider(ui, "Background opacity", &mut s.background_opacity, 0..=100, "%");
    if !app.glass.has_wallpaper() && app.state.settings.wallpaper_frost {
        hint(ui, "No wallpaper image found (solid colour or slideshow); widgets use the theme only.", look);
    }
    section(ui, "Per-widget opacity", look);
    for (id, name) in WIDGETS {
        let mut v = app.state.settings.opacity(id);
        if slider(ui, name, &mut v, 0..=100, "%") {
            app.state.settings.widget_opacity.insert(id.into(), v);
        }
    }
    let _ = ctx;
}

fn widgets_tab(app: &mut App, ui: &mut Ui, look: &Look) {
    section(ui, "Show on desktop", look);
    for (id, name) in WIDGETS {
        let mut open = app.is_open(id);
        if ui.checkbox(&mut open, name).changed() {
            app.set_open(id, open);
        }
    }
    hint(ui, "Drag a widget by its glass to move it; drag the bottom-right corner to resize. Right-click a widget for its design and theme.", look);
}

fn design_picker(app: &mut App, ui: &mut Ui, ctx: &Context, widget: &str, look: &Look) {
    section(ui, "Design", look);
    let current = app.state.settings.design(widget);
    for d in theme::designs(widget) {
        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
        let active = current == d.id;
        if active || resp.hovered() {
            ui.painter().rect_filled(r, CornerRadius::same(10), look.ink.hover);
        }
        if active {
            ui.painter().rect_stroke(r, CornerRadius::same(10), Stroke::new(1.5_f32, look.ink.accent), StrokeKind::Inside);
        }
        ui.painter().text(pos2(r.min.x + 12.0, r.min.y + 8.0), Align2::LEFT_TOP, d.name, semibold(13.5), look.ink.ink);
        ui.painter().text(pos2(r.min.x + 12.0, r.max.y - 8.0), Align2::LEFT_BOTTOM, d.description, font(12.0), look.ink.dim);
        if resp.clicked() {
            app.state.settings.set_design(widget, d.id);
        }
    }
    section(ui, "Theme", look);
    theme_grid(app, ui, ctx, look, Some(widget));
    let mut on = app.is_open(widget);
    ui.add_space(6.0);
    if ui.checkbox(&mut on, "Show this widget").changed() {
        app.set_open(widget, on);
    }
    let _ = Id::NULL;
}

fn launcher(app: &mut App, ui: &mut Ui, look: &Look) {
    let s = &mut app.state.settings;
    section(ui, "Grid", look);
    slider(ui, "Columns", &mut s.columns, 3..=12, "");
    slider(ui, "Icon size", &mut s.icon_size, 36..=60, " px");
    ui.checkbox(&mut s.show_labels, "Show labels");
    ui.checkbox(&mut s.show_search, "Show search");
    section(ui, "Apps", look);
    hint(ui, "Search to find any installed app and pin it, drop files or folders onto the launcher, or use + to browse. Drag icons to reorder; right-click for more.", look);
    ui.label(RichText::new(format!("{} pinned · {} programs indexed", app.state.items.len(), app.programs.len())).size(12.0).color(look.ink.dim));
}

fn clock(app: &mut App, ui: &mut Ui, ctx: &Context, look: &Look) {
    {
        let c = &mut app.state.settings.clock;
        section(ui, "Time", look);
        ui.checkbox(&mut c.hour12, "12-hour clock");
        ui.checkbox(&mut c.show_seconds, "Show seconds");
        ui.horizontal(|ui| {
            ui.add_sized(vec2(130.0, 20.0), egui::Label::new("Time zone"));
            egui::ComboBox::from_id_salt("zone")
                .selected_text(if c.time_zone == "local" { "Local time".to_string() } else { c.time_zone.clone() })
                .width(220.0)
                .show_ui(ui, |ui| {
                    for z in ZONES {
                        let label = if *z == "local" { "Local time" } else { z };
                        ui.selectable_value(&mut c.time_zone, z.to_string(), label);
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.add_sized(vec2(130.0, 20.0), egui::Label::new("Other zone"));
            ui.add(egui::TextEdit::singleline(&mut app.settings_ui.custom_zone).hint_text("e.g. Europe/Rome").desired_width(170.0));
            if ui.button("Use").clicked() {
                let z = app.settings_ui.custom_zone.trim().to_string();
                if z.parse::<chrono_tz::Tz>().is_ok() {
                    app.state.settings.clock.time_zone = z;
                } else {
                    app.notify("Unknown time zone");
                }
            }
        });
    }
    design_picker(app, ui, ctx, "clock", look);
}

fn calendar(app: &mut App, ui: &mut Ui, ctx: &Context, look: &Look) {
    section(ui, "Calendar feed", look);
    hint(ui, "Paste a private iCal (ICS) address. Google Calendar: Settings → your calendar → \"Secret address in iCal format\". Outlook: Settings → Shared calendars → Publish → ICS.", look);
    let before = app.calendar.clone();
    let current = app.calendar.ics_url.clone();
    if let Some(v) = crate::widgets::wsettings::commit_field(ui, &mut app.ws_ui.ics_draft, &current, "https://…/basic.ics — press Enter") {
        app.calendar.ics_url = v;
    }
    ui.horizontal(|ui| {
        if ui.button("Sync now").clicked() {
            app.cal.refresh();
        }
        if !app.cal.status.is_empty() {
            ui.label(RichText::new(&app.cal.status).size(12.0).color(look.ink.dim));
        }
    });
    section(ui, "Week", look);
    ui.horizontal(|ui| {
        ui.add_sized(vec2(130.0, 20.0), egui::Label::new("Week starts on"));
        for (v, name) in [(1, "Monday"), (0, "Sunday"), (6, "Saturday")] {
            ui.radio_value(&mut app.calendar.week_starts_on, v, name);
        }
    });
    if app.calendar != before {
        app.mark_calendar();
    }
    design_picker(app, ui, ctx, "calendar", look);
}

fn about(app: &mut App, ui: &mut Ui, look: &Look) {
    section(ui, "Opal for Windows", look);
    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
    hint(ui, "Opal, a Rust rewrite of Lumen: glass widgets that live on your wallpaper. Built with egui; no Electron, Node.js or Python needed.", look);
    section(ui, "Data", look);
    ui.label(RichText::new(crate::config::config_dir().display().to_string()).size(12.0).color(look.ink.dim));
    ui.horizontal(|ui| {
        if ui.button("Open config folder").clicked() {
            crate::win::open_folder(&crate::config::config_dir());
        }
        if ui.button("Import from Lumen…").on_hover_text("Pick the original Lumen folder (or its config folder) to copy apps, settings, notes and calendar").clicked() {
            if let Some(dir) = rfd::FileDialog::new().set_title("Choose the Lumen folder").pick_folder() {
                import_lumen(app, &dir);
            }
        }
    });
}

fn import_lumen(app: &mut App, dir: &std::path::Path) {
    let src = if dir.join("config").join("state.json").exists() || dir.join("config").join("notes.json").exists() {
        dir.join("config")
    } else {
        dir.to_path_buf()
    };
    let mut copied = Vec::new();
    for name in ["state.json", "notes.json", "calendar.json"] {
        let from = src.join(name);
        if from.exists() {
            let to = crate::config::config_file(name);
            if to.exists() {
                let _ = std::fs::copy(&to, to.with_extension("json.bak"));
            }
            if std::fs::copy(&from, &to).is_ok() {
                copied.push(name);
            }
        }
    }
    if copied.is_empty() {
        app.notify("No Lumen data found in that folder");
        return;
    }
    app.state = crate::config::State::load();
    app.notes = crate::config::NotesFile::load();
    app.calendar = crate::config::CalendarConfig::load();
    app.cal.refresh();
    app.notify(format!("Imported {}", copied.join(", ")));
}
