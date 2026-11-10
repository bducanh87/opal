//! The application: one tiny invisible host window plus one borderless,
//! transparent viewport per widget. Widgets are tool windows owned by the
//! desktop and kept just above the wallpaper, as in Lumen's `widgets.ts`.

use egui::{
    Align2, Color32, Context, CornerRadius, FontFamily, FontId, Frame, Id, Pos2, Rect, RichText,
    Sense, Ui, ViewportBuilder, ViewportCommand, ViewportId, pos2, vec2,
};
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use crate::config::{CalendarConfig, Layout, NotesFile, Placement, State};
use crate::glass::{Glass, Look};
use crate::icons::Icons;
use crate::programs::Program;
use crate::win::Hwnd;
use crate::{media, theme, tray, widgets};

pub const WIDGETS: [(&str, &str); 11] = [
    ("launcher", "Launcher"),
    ("clock", "Clock"),
    ("notes", "Notes"),
    ("calendar", "Calendar"),
    ("music", "Music"),
    ("weather", "Weather"),
    ("system", "System"),
    ("pomodoro", "Focus timer"),
    ("photos", "Photos"),
    ("lunar", "Âm lịch"),
    ("todo", "To-do"),
];

/// Widgets added by this rewrite start hidden.
pub fn starts_open(id: &str) -> bool {
    matches!(id, "launcher" | "clock" | "notes" | "calendar" | "music")
}

/// Append a line to `config/opal-egui.log` (kept small).
pub fn log_line(msg: &str) {
    use std::io::Write;
    let path = crate::config::config_file("opal-egui.log");
    if std::fs::metadata(&path).map(|m| m.len() > 256 * 1024).unwrap_or(false) {
        let _ = std::fs::remove_file(&path);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{} {msg}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
    }
}

pub fn widget_title(id: &str) -> String {
    let name = WIDGETS.iter().find(|w| w.0 == id).map(|w| w.1).unwrap_or(id);
    format!("Opal · {name}")
}

const SETTINGS_TITLE: &str = "Opal Settings";
const PANEL_TITLE: &str = "Opal Widgets";
const WS_TITLE: &str = "Opal Widget Settings";

pub mod glyph {
    pub const SETTINGS: char = '\u{E713}';
    pub const ADD: char = '\u{E710}';
    pub const SEARCH: char = '\u{E721}';
    pub const DELETE: char = '\u{E74D}';
    pub const PIN: char = '\u{E718}';
    pub const UNPIN: char = '\u{E77A}';
    pub const PLAY: char = '\u{E768}';
    pub const PAUSE: char = '\u{E769}';
    pub const NEXT: char = '\u{E893}';
    pub const PREV: char = '\u{E892}';
    pub const LEFT: char = '\u{E76B}';
    pub const RIGHT: char = '\u{E76C}';
    pub const CLOSE: char = '\u{E711}';
    pub const UNDO: char = '\u{E7A7}';
    pub const MUSIC: char = '\u{E8D6}';
    pub const APPS: char = '\u{E8A9}';
    pub const FOLDER: char = '\u{E838}';
    pub const REFRESH: char = '\u{E72C}';
    pub const NOTE: char = '\u{E8A5}';
    pub const CALENDAR: char = '\u{E787}';
    pub const CLOCK: char = '\u{E823}';
    pub const DOCK: char = '\u{E8A9}';
}

pub fn icon_text(ch: char, size: f32, color: Color32) -> RichText {
    RichText::new(ch.to_string())
        .font(FontId::new(size, FontFamily::Name("icons".into())))
        .color(color)
}

/// Borderless icon button that tints on hover.
pub fn icon_button(ui: &mut Ui, ch: char, size: f32, look: &Look, tip: &str) -> egui::Response {
    let side = size + 12.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(side, side), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(8), look.ink.hover);
    }
    let color = if resp.hovered() { look.ink.ink } else { look.ink.dim };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        ch.to_string(),
        FontId::new(size, FontFamily::Name("icons".into())),
        color,
    );
    if tip.is_empty() { resp } else { resp.on_hover_text(tip) }
}

pub struct Toast {
    pub text: String,
    pub until: Instant,
    /// Widget window that shows it (the one the user was using).
    pub target: Option<&'static str>,
}

pub struct App {
    pub state: State,
    pub notes: NotesFile,
    pub calendar: CalendarConfig,
    pub layout: Layout,
    pub extras: crate::config::Extras,
    pub weather: widgets::weather::WeatherState,
    pub sys: widgets::system::SystemState,
    pub pomodoro: widgets::pomodoro::PomodoroState,
    pub photos: widgets::photos::PhotosState,
    pub todo_ui: widgets::todo::TodoUi,
    extras_dirty: bool,

    pub glass: Glass,
    pub icons: Icons,
    pub programs: Vec<Program>,
    programs_rx: Option<Receiver<Vec<Program>>>,
    pub media: media::Media,
    pub cal: widgets::calendar::CalendarState,
    pub launcher_ui: widgets::launcher::LauncherUi,
    pub notes_ui: widgets::notes::NotesUi,
    pub settings_ui: widgets::settings::SettingsUi,

    pub settings_open: bool,
    pub widgets_panel_open: bool,
    pub widget_settings: Option<&'static str>,
    pub ws_ui: widgets::wsettings::WidgetSettingsUi,
    pub toast: Option<Toast>,

    state_dirty: bool,
    notes_dirty: Option<Instant>,
    calendar_dirty: bool,
    layout_dirty: Option<Instant>,

    hwnds: HashMap<String, Hwnd>,
    focused: HashMap<String, bool>,
    spawn: HashMap<String, Placement>,
    last_pin: Instant,
    work_area: Option<Rect>,
    tray: Option<tray::Tray>,
    quitting: bool,
    /// Set by reset_positions(): skip one frame so windows are re-created.
    reopen_all: bool,
    settings_hwnd: Option<Hwnd>,
    /// Widget most recently focused, where toasts appear.
    last_widget: Option<&'static str>,
    initialized: bool,
    work_area_checked: Instant,
    panel_hwnd: Option<Hwnd>,
    ws_hwnd: Option<Hwnd>,
    ws_for: Option<&'static str>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = &cc.egui_ctx;
        crate::fonts::install(ctx);
        let mut style = (*ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        // Menus and popups: Lumen's --menu-bg (dark violet glass).
        style.visuals.window_fill = crate::theme::css(28, 25, 40, 245);
        style.visuals.window_stroke = egui::Stroke::new(1.0, crate::theme::white(30));
        style.visuals.window_corner_radius = CornerRadius::same(14);
        style.visuals.menu_corner_radius = CornerRadius::same(14);
        style.visuals.popup_shadow = egui::epaint::Shadow { offset: [0, 12], blur: 30, spread: 0, color: Color32::from_black_alpha(120) };
        style.visuals.panel_fill = Color32::TRANSPARENT;
        style.spacing.item_spacing = vec2(8.0, 6.0);
        style.spacing.button_padding = vec2(12.0, 5.0);
        // Glass-style controls instead of egui's solid grey boxes.
        for (w, fill) in [
            (&mut style.visuals.widgets.inactive, 18u8),
            (&mut style.visuals.widgets.hovered, 30),
            (&mut style.visuals.widgets.active, 42),
            (&mut style.visuals.widgets.open, 30),
        ] {
            w.bg_fill = crate::theme::white(fill);
            w.weak_bg_fill = crate::theme::white(fill);
            w.bg_stroke = egui::Stroke::new(1.0, crate::theme::white(28));
            w.corner_radius = CornerRadius::same(9);
        }
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, crate::theme::white(22));
        style.visuals.selection.bg_fill = Color32::from_rgb(0x6d, 0x7d, 0xf2);
        style.visuals.extreme_bg_color = Color32::from_black_alpha(70);
        style.interaction.tooltip_delay = 0.4;
        // Always dark: egui would otherwise follow the Windows light theme.
        ctx.set_theme(egui::ThemePreference::Dark);
        ctx.set_style_of(egui::Theme::Dark, style);

        let state = State::load();
        let notes = NotesFile::load();
        let calendar = CalendarConfig::load();
        let layout = Layout::load();
        let extras = crate::config::Extras::load();
        let tray = tray::Tray::new(ctx);
        let cal = widgets::calendar::CalendarState::new(ctx, &calendar);
        App {
            glass: Glass::new(ctx),
            icons: Icons::new(ctx),
            programs: Vec::new(),
            programs_rx: Some(crate::programs::load_async()),
            media: media::Media::start(ctx),
            cal,
            launcher_ui: Default::default(),
            notes_ui: Default::default(),
            settings_ui: Default::default(),
            settings_open: std::env::args().any(|a| a == "--settings"),
            widgets_panel_open: std::env::args().any(|a| a == "--widgets"),
            widget_settings: std::env::args().skip_while(|a| a != "--widget-settings").nth(1).and_then(|w| WIDGETS.iter().find(|x| x.0 == w).map(|x| x.0)),
            ws_ui: Default::default(),
            toast: None,
            state,
            notes,
            calendar,
            layout,
            extras,
            weather: widgets::weather::WeatherState::new(ctx),
            sys: widgets::system::SystemState::new(),
            pomodoro: Default::default(),
            photos: widgets::photos::PhotosState::new(ctx),
            todo_ui: Default::default(),
            extras_dirty: false,
            state_dirty: false,
            notes_dirty: None,
            calendar_dirty: false,
            layout_dirty: None,
            hwnds: HashMap::new(),
            focused: HashMap::new(),
            spawn: HashMap::new(),
            last_pin: Instant::now(),
            work_area: None,
            tray,
            quitting: false,
            reopen_all: false,
            settings_hwnd: None,
            last_widget: None,
            initialized: false,
            work_area_checked: Instant::now(),
            panel_hwnd: None,
            ws_hwnd: None,
            ws_for: None,
        }
    }

    pub fn mark_state(&mut self) {
        self.state_dirty = true;
    }
    pub fn mark_notes(&mut self) {
        self.notes_dirty.get_or_insert_with(Instant::now);
    }
    pub fn mark_extras(&mut self) {
        self.extras_dirty = true;
    }
    pub fn mark_calendar(&mut self) {
        self.calendar_dirty = true;
    }

    pub fn notify(&mut self, text: impl Into<String>) {
        self.toast = Some(Toast { text: text.into(), until: Instant::now() + Duration::from_secs(4), target: self.last_widget });
    }

    /// Save everything now (on exit / quit).
    pub fn flush(&mut self) {
        self.save_due(true);
    }

    fn maybe_flush(&mut self) {
        self.save_due(false);
    }

    /// Write dirty files. Notes and layout are debounced unless `force`.
    /// A dirty flag is cleared only when its file really reached the disk,
    /// so a locked or failing file is retried instead of silently dropped.
    fn save_due(&mut self, force: bool) {
        let notes_due = self.notes_dirty.is_some_and(|t| force || t.elapsed() > Duration::from_millis(600));
        let layout_due = self.layout_dirty.is_some_and(|t| force || t.elapsed() > Duration::from_millis(500));
        if self.state_dirty && self.state.save() {
            self.state_dirty = false;
        }
        if self.calendar_dirty && self.calendar.save() {
            self.calendar_dirty = false;
        }
        if self.extras_dirty && self.extras.save() {
            self.extras_dirty = false;
        }
        if notes_due {
            self.notes_dirty = if self.notes.save() { None } else { Some(Instant::now()) };
        }
        if layout_due {
            self.layout_dirty = if self.layout.save() { None } else { Some(Instant::now()) };
        }
    }

    pub fn is_open(&self, id: &str) -> bool {
        self.layout.widgets.get(id).map(|p| p.open).unwrap_or_else(|| starts_open(id))
    }

    #[track_caller]
    pub fn set_open(&mut self, id: &str, open: bool) {
        // Record who hides a widget, to track down unexpected closes.
        if !open && self.is_open(id) {
            let at = std::panic::Location::caller();
            log_line(&format!("hide {id} from {}:{}", at.file(), at.line()));
        }
        let p = self.placement(id);
        self.layout.widgets.insert(id.to_string(), Placement { open, ..p });
        self.layout_dirty = Some(Instant::now());
        if !open {
            self.hwnds.remove(id);
            self.spawn.remove(id);
            if self.widget_settings == Some(id) {
                self.widget_settings = None;
            }
        }
    }

    pub fn reset_positions(&mut self) {
        for (id, _) in WIDGETS {
            let open = self.is_open(id);
            self.layout.widgets.remove(id);
            let mut p = self.placement(id);
            p.open = open;
            self.spawn.remove(id);
            self.hwnds.remove(id);
            self.layout.widgets.insert(id.to_string(), p);
        }
        self.layout_dirty = Some(Instant::now());
        // Re-create the windows at their default spots next frame.
        self.reopen_all = true;
    }

    fn default_placement(&self, id: &str) -> Placement {
        let wa = self.work_area.unwrap_or(Rect::from_min_size(pos2(0.0, 0.0), vec2(1600.0, 860.0)));
        let m = 28.0;
        let (w, h) = match id {
            "launcher" => (400.0, 290.0),
            "clock" => (360.0, 240.0),
            "notes" => (560.0, 420.0),
            "calendar" => (340.0, 340.0),
            "weather" => (360.0, 250.0),
            "system" => (340.0, 280.0),
            "pomodoro" => (300.0, 340.0),
            "photos" => (420.0, 300.0),
            "lunar" => (360.0, 260.0),
            "todo" => (340.0, 380.0),
            _ => (380.0, 150.0),
        };
        let right = wa.max.x - m;
        let (x, y) = match id {
            "launcher" => (wa.min.x + m, wa.min.y + m),
            "clock" => (right - w, wa.min.y + m),
            "calendar" => (right - w, wa.min.y + m + 240.0 + 20.0),
            "notes" => (right - 360.0 - 20.0 - w, wa.min.y + m),
            "weather" => (wa.min.x + m + 420.0, wa.min.y + m),
            "system" => (wa.min.x + m + 420.0, wa.min.y + m + 270.0),
            "pomodoro" => (wa.center().x - w / 2.0, wa.center().y - h / 2.0),
            "photos" => (wa.min.x + m, wa.min.y + m + 310.0),
            "lunar" => (wa.center().x - w / 2.0, wa.min.y + m),
            "todo" => (wa.center().x + 200.0, wa.center().y - h / 2.0),
            _ => (wa.min.x + m, wa.max.y - m - h),
        };
        Placement { open: starts_open(id), x, y, w, h }
    }

    fn placement(&self, id: &str) -> Placement {
        self.layout.widgets.get(id).copied().unwrap_or_else(|| self.default_placement(id))
    }

    pub fn quit(&mut self, ctx: &Context) {
        self.quitting = true;
        self.flush();
        ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Close);
    }

    // ------------------------------------------------------------ frames --

    fn widget_viewport(&mut self, ctx: &Context, id: &'static str) {
        let p = match self.spawn.get(id) {
            Some(p) => *p,
            None => {
                let p = self.placement(id);
                self.spawn.insert(id.to_string(), p);
                p
            }
        };
        let builder = ViewportBuilder::default()
            .with_title(widget_title(id))
            .with_decorations(false)
            .with_transparent(true)
            .with_taskbar(false)
            .with_resizable(true)
            .with_position(pos2(p.x, p.y))
            .with_inner_size(vec2(p.w, p.h))
            .with_min_inner_size(vec2(200.0, 110.0))
            .with_icon(crate::fonts::app_icon());
        let vid = ViewportId::from_hash_of(("widget", id));
        ctx.show_viewport_immediate(vid, builder, |ctx, _class| self.render_widget(ctx, id));
    }

    fn render_widget(&mut self, ctx: &Context, id: &'static str) {
        if !self.hwnds.contains_key(id) {
            if let Some(h) = crate::win::find_own_window(&widget_title(id)) {
                crate::win::prepare_widget_window(h);
                crate::win::pin_to_desktop(h);
                self.hwnds.insert(id.to_string(), h);
            }
        }
        let (close, focused, rect) = ctx.input(|i| {
            let v = i.viewport();
            (v.close_requested(), v.focused.unwrap_or(false), v.inner_rect)
        });
        if close {
            self.set_open(id, false);
            return;
        }
        self.focused.insert(id.to_string(), focused);
        if focused {
            self.last_widget = Some(id);
        }
        if let Some(r) = rect {
            let saved = self.placement(id);
            let now = Placement { open: true, x: r.min.x, y: r.min.y, w: r.width(), h: r.height() };
            let moved = (saved.x - now.x).abs() > 0.5
                || (saved.y - now.y).abs() > 0.5
                || (saved.w - now.w).abs() > 0.5
                || (saved.h - now.h).abs() > 0.5;
            if moved && r.width() > 50.0 {
                self.layout.widgets.insert(id.to_string(), now);
                self.layout_dirty = Some(Instant::now());
            }
        }

        let look = self.glass.look(ctx, &self.state.settings, id);
        egui::CentralPanel::default().frame(Frame::NONE).show(ctx, |ui| {
            let full = ui.max_rect();
            let painter = ui.painter().clone();
            self.glass.paint(ctx, &painter, full.shrink(1.0), 34.0, &look);

            // The glass itself drags the window; right click opens widget options.
            let bg = ui.interact(full, Id::new(("bg", id)), Sense::click_and_drag());
            if bg.drag_started() {
                ctx.send_viewport_cmd(ViewportCommand::StartDrag);
            }
            bg.context_menu(|ui| self.widget_menu(ui, ctx, id));

            let inner = full.shrink2(vec2(18.0, 16.0));
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
            child.set_clip_rect(full.shrink(2.0));
            child.visuals_mut().override_text_color = Some(look.ink.ink);
            match id {
                "launcher" => widgets::launcher::show(self, &mut child, ctx, &look),
                "clock" => widgets::clock::show(self, &mut child, &look),
                "notes" => widgets::notes::show(self, &mut child, &look),
                "calendar" => widgets::calendar::show(self, &mut child, &look),
                "music" => widgets::music::show(self, &mut child, ctx, &look),
                "weather" => widgets::weather::show(self, &mut child, &look),
                "system" => widgets::system::show(self, &mut child, &look),
                "pomodoro" => widgets::pomodoro::show(self, &mut child, &look),
                "photos" => widgets::photos::show(self, &mut child, &look),
                "lunar" => widgets::lunar_cal::show(self, &mut child, &look),
                "todo" => widgets::todo::show(self, &mut child, &look),
                _ => {}
            }

            // Widgets without a gear of their own show one on hover. The
            // older ones keep it at the lower right; the newer widgets have a
            // header row, so theirs sits at the top right over the header.
            if !matches!(id, "launcher" | "notes") {
                let hovering = ctx.input(|i| i.pointer.hover_pos()).is_some_and(|p| full.contains(p));
                if hovering || self.widget_settings == Some(id) {
                    let at = if matches!(id, "clock" | "music" | "calendar") {
                        pos2(full.max.x - 22.0, full.max.y - 46.0)
                    } else {
                        pos2(full.max.x - 26.0, full.min.y + 24.0)
                    };
                    let gear = Rect::from_center_size(at, vec2(26.0, 26.0));
                    let resp = ui.interact(gear, Id::new(("gear", id)), Sense::click()).on_hover_text("Widget settings");
                    let p = ui.painter();
                    p.circle_filled(gear.center(), 13.0, if resp.hovered() { look.ink.hover_strong } else { Color32::from_black_alpha(60) });
                    p.circle_stroke(gear.center(), 13.0, egui::Stroke::new(1.0_f32, look.ink.hair));
                    p.text(gear.center(), Align2::CENTER_CENTER, glyph::SETTINGS.to_string(), FontId::new(12.0, FontFamily::Name("icons".into())), look.ink.ink);
                    if resp.clicked() {
                        self.widget_settings = if self.widget_settings == Some(id) { None } else { Some(id) };
                    }
                }
            }
            // Toast: in the widget the user was last using, else the
            // launcher, else the first open widget.
            let shown_in = self
                .toast
                .as_ref()
                .and_then(|t| t.target)
                .filter(|w| self.is_open(w))
                .or_else(|| self.is_open("launcher").then_some("launcher"))
                .or_else(|| WIDGETS.iter().map(|w| w.0).find(|w| self.is_open(w)));
            if let (Some(t), Some(true)) = (&self.toast, shown_in.map(|w| w == id)) {
                let g = widgets::elided(ui, &t.text, FontId::proportional(12.5), Color32::WHITE, full.width() - 48.0);
                let pill = Rect::from_center_size(pos2(full.center().x, full.max.y - 26.0), g.size() + vec2(24.0, 12.0));
                ui.painter().rect_filled(pill, CornerRadius::same(14), Color32::from_black_alpha(190));
                ui.painter().galley(pill.center() - g.size() / 2.0, g, Color32::WHITE);
            }
            resize_grip(ui, ctx, full, &look, id);
        });
    }

    fn widget_menu(&mut self, ui: &mut Ui, ctx: &Context, id: &'static str) {
        ui.set_min_width(180.0);
        ui.menu_button("Design", |ui| {
            let current = self.state.settings.design(id);
            for d in theme::designs(id) {
                if ui.selectable_label(current == d.id, d.name).on_hover_text(d.description).clicked() {
                    self.state.settings.set_design(id, d.id);
                    self.mark_state();
                    ui.close_menu();
                }
            }
        });
        ui.menu_button("Theme", |ui| {
            let current = self.state.settings.widget_theme.get(id).cloned();
            if ui.selectable_label(current.is_none(), "Same as Opal").clicked() {
                self.state.settings.widget_theme.remove(id);
                self.mark_state();
                ui.close_menu();
            }
            for t in theme::THEMES {
                if ui.selectable_label(current.as_deref() == Some(t.id), t.name).clicked() {
                    self.state.settings.widget_theme.insert(id.into(), t.id.into());
                    self.mark_state();
                    ui.close_menu();
                }
            }
        });
        let mut frosted = self.state.settings.frosted(id);
        if ui.checkbox(&mut frosted, "Frosted glass").changed() {
            if id == "clock" {
                self.state.settings.clock.frosted_glass = frosted;
            }
            self.state.settings.widget_frosted.insert(id.into(), frosted);
            self.mark_state();
        }
        ui.separator();
        if ui.button("Widget settings…").clicked() {
            self.widget_settings = Some(id);
            ui.close_menu();
        }
        if ui.button("Opal settings…").clicked() {
            self.settings_ui.tab = widgets::settings::Tab::for_widget(id);
            self.settings_open = true;
            ui.close_menu();
        }
        if ui.button("Hide widget").clicked() {
            self.set_open(id, false);
            ui.close_menu();
        }
        let _ = ctx;
    }

    /// "<Widget> settings", opened beside its widget.
    fn widget_settings_viewport(&mut self, ctx: &Context, id: &'static str) {
        let anchor = self.placement(id);
        let size = vec2(500.0, 620.0);
        let wa = self.work_area.unwrap_or(Rect::from_min_size(Pos2::ZERO, vec2(1600.0, 860.0)));
        let mut pos = pos2(anchor.x + anchor.w + 14.0, anchor.y);
        if pos.x + size.x > wa.max.x {
            pos.x = (anchor.x - size.x - 14.0).max(wa.min.x);
        }
        pos.y = pos.y.min(wa.max.y - size.y).max(wa.min.y);
        // One title per widget, so the lookup never finds the previous
        // widget's settings window while it is being replaced.
        let title_owned = format!("{WS_TITLE} · {id}");
        let title = title_owned.as_str();
        let builder = ViewportBuilder::default()
            .with_title(title)
            .with_decorations(false)
            .with_transparent(true)
            .with_taskbar(false)
            .with_resizable(true)
            .with_position(pos)
            .with_inner_size(size)
            .with_min_inner_size(vec2(420.0, 420.0))
            .with_icon(crate::fonts::app_icon());
        ctx.show_viewport_immediate(ViewportId::from_hash_of(("widget-settings", id)), builder, |ctx, _| {
            if ctx.input(|i| i.viewport().close_requested() || i.key_pressed(egui::Key::Escape)) {
                self.widget_settings = None;
                return;
            }
            if self.ws_hwnd.is_none() {
                self.ws_hwnd = crate::win::find_own_window(title);
                if let Some(h) = self.ws_hwnd {
                    crate::win::prepare_widget_window(h);
                }
            }
            let look = self.glass.look(ctx, &self.state.settings, id);
            egui::CentralPanel::default().frame(Frame::NONE).show(ctx, |ui| {
                let full = ui.max_rect();
                let painter = ui.painter().clone();
                let mut solid = look.clone();
                solid.opacity = 1.0;
                self.glass.paint(ctx, &painter, full.shrink(1.0), 30.0, &solid);
                let bg = ui.interact(full, Id::new("ws-bg"), Sense::click_and_drag());
                if bg.drag_started() {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(full.shrink2(vec2(18.0, 16.0))));
                child.visuals_mut().override_text_color = Some(solid.ink.ink);
                widgets::wsettings::show(self, &mut child, ctx, id, &solid);
            });
        });
        if self.widget_settings.is_none() {
            self.ws_hwnd = None;
        }
    }

    /// The Widgets manager, opened beside the launcher.
    fn panel_viewport(&mut self, ctx: &Context) {
        let dock = self.placement("launcher");
        let wa = self.work_area.unwrap_or(Rect::from_min_size(Pos2::ZERO, vec2(1600.0, 860.0)));
        // Tall enough for every row, but never taller than the screen.
        let size = vec2(460.0, 820.0_f32.min(wa.height() - 16.0).max(420.0));
        let mut pos = pos2(dock.x + dock.w + 14.0, dock.y);
        if pos.x + size.x > wa.max.x {
            pos.x = (dock.x - size.x - 14.0).max(wa.min.x);
        }
        // Keep it on screen even when the dock sits near the bottom.
        pos.y = pos.y.min(wa.max.y - size.y).max(wa.min.y);
        let builder = ViewportBuilder::default()
            .with_title(PANEL_TITLE)
            .with_decorations(false)
            .with_transparent(true)
            .with_taskbar(false)
            .with_resizable(true)
            .with_position(pos)
            .with_inner_size(size)
            .with_min_inner_size(vec2(380.0, 420.0))
            .with_icon(crate::fonts::app_icon());
        ctx.show_viewport_immediate(ViewportId::from_hash_of("widgets-panel"), builder, |ctx, _| {
            if ctx.input(|i| i.viewport().close_requested() || i.key_pressed(egui::Key::Escape)) {
                self.widgets_panel_open = false;
                return;
            }
            if self.panel_hwnd.is_none() {
                self.panel_hwnd = crate::win::find_own_window(PANEL_TITLE);
                if let Some(h) = self.panel_hwnd {
                    crate::win::prepare_widget_window(h);
                }
            }
            let look = self.glass.look(ctx, &self.state.settings, "launcher");
            egui::CentralPanel::default().frame(Frame::NONE).show(ctx, |ui| {
                let full = ui.max_rect();
                let painter = ui.painter().clone();
                self.glass.paint(ctx, &painter, full.shrink(1.0), 34.0, &look);
                let bg = ui.interact(full, Id::new("panel-bg"), Sense::click_and_drag());
                if bg.drag_started() {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(full.shrink2(vec2(20.0, 18.0))));
                child.visuals_mut().override_text_color = Some(look.ink.ink);
                widgets::panel::show(self, &mut child, &look);
            });
        });
    }

    fn settings_viewport(&mut self, ctx: &Context) {
        let wa = self.work_area.unwrap_or(Rect::from_min_size(Pos2::ZERO, vec2(1600.0, 860.0)));
        let size = vec2(720.0, 540.0);
        let builder = ViewportBuilder::default()
            .with_title(SETTINGS_TITLE)
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(true)
            .with_position(wa.center() - size / 2.0)
            .with_inner_size(size)
            .with_min_inner_size(vec2(560.0, 400.0))
            .with_icon(crate::fonts::app_icon());
        ctx.show_viewport_immediate(ViewportId::from_hash_of("settings"), builder, |ctx, _| {
            if ctx.input(|i| i.viewport().close_requested() || i.key_pressed(egui::Key::Escape)) {
                self.settings_open = false;
                return;
            }
            if self.settings_hwnd.is_none() {
                self.settings_hwnd = crate::win::find_own_window(SETTINGS_TITLE);
                if let Some(h) = self.settings_hwnd {
                    crate::win::prepare_glass_window(h);
                }
            }
            let look = self.glass.look(ctx, &self.state.settings, "settings");
            egui::CentralPanel::default().frame(Frame::NONE).show(ctx, |ui| {
                let full = ui.max_rect();
                let painter = ui.painter().clone();
                let mut solid = look.clone();
                solid.opacity = 1.0;
                solid.veil = solid.veil.max(0.82);
                self.glass.paint(ctx, &painter, full.shrink(1.0), 28.0, &solid);
                let bar = Rect::from_min_size(full.min, vec2(full.width(), 44.0));
                let drag = ui.interact(bar, Id::new("settings-drag"), Sense::click_and_drag());
                if drag.drag_started() {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(full.shrink2(vec2(18.0, 12.0))));
                child.visuals_mut().override_text_color = Some(solid.ink.ink);
                widgets::settings::show(self, &mut child, ctx, &solid);
                resize_grip(ui, ctx, full, &solid, "settings");
            });
        });
    }

    fn handle_tray(&mut self, ctx: &Context) {
        let Some(tray) = &self.tray else { return };
        for action in tray.poll() {
            match action {
                tray::Action::Settings => {
                    self.settings_open = true;
                    ctx.send_viewport_cmd_to(ViewportId::from_hash_of("settings"), ViewportCommand::Focus);
                }
                tray::Action::Toggle(id) => {
                    let open = self.is_open(id);
                    self.set_open(id, !open);
                }
                tray::Action::ConfigFolder => crate::win::open_folder(&crate::config::config_dir()),
                tray::Action::Quit => self.quit(ctx),
            }
        }
        if let Some(tray) = &self.tray {
            let open: Vec<(&str, bool)> = WIDGETS.iter().map(|(id, _)| (*id, self.is_open(id))).collect();
            tray.sync(&open);
        }
    }

    fn keep_on_desktop(&mut self) {
        if self.last_pin.elapsed() < Duration::from_millis(1500) {
            return;
        }
        self.last_pin = Instant::now();
        for (id, h) in &self.hwnds {
            if !self.focused.get(id).copied().unwrap_or(false) {
                crate::win::pin_to_desktop(*h);
            }
        }
    }

    fn next_repaint(&self) -> Duration {
        let now = chrono::Local::now();
        let ms = 1000 - now.timestamp_subsec_millis().min(999) as u64;
        let mut wait = Duration::from_millis(ms + 5);
        if self.photos.animating() {
            wait = wait.min(Duration::from_millis(16));
        }
        if self.media.wants_animation(&self.state.settings) {
            wait = wait.min(Duration::from_millis(33));
        }
        wait
    }
}

fn resize_grip(ui: &mut Ui, ctx: &Context, full: Rect, look: &Look, id: &str) {
    let grip = Rect::from_min_max(full.max - vec2(26.0, 26.0), full.max);
    let resp = ui.interact(grip, Id::new(("grip", id)), Sense::drag());
    let active = resp.hovered() || resp.dragged();
    if active {
        ctx.set_cursor_icon(egui::CursorIcon::ResizeNwSe);
    }
    // An arc that follows the rounded corner, like Lumen's resize handle.
    let radius = 34.0_f32.min(full.width().min(full.height()) * 0.25);
    let c = full.max - vec2(radius, radius) - vec2(1.0, 1.0);
    let r = radius - 6.0;
    let pts: Vec<Pos2> = (0..=16)
        .map(|k| {
            let a = (10.0 + 70.0 * k as f32 / 16.0).to_radians();
            c + vec2(a.cos(), a.sin()) * r
        })
        .collect();
    let color = if active { look.ink.dim } else { look.ink.faint.gamma_multiply(0.7) };
    ui.painter().add(egui::Shape::line(pts, egui::Stroke::new(2.0_f32, color)));
    if resp.drag_started() {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(egui::viewport::ResizeDirection::SouthEast));
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        crate::win::KEY_CLEAR
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // The work area follows resolution, scaling and taskbar changes.
        if self.work_area.is_none() || self.work_area_checked.elapsed() > Duration::from_secs(5) {
            self.work_area_checked = Instant::now();
            let ppp = ctx.pixels_per_point();
            if let Some((x, y, w, h)) = crate::win::work_area() {
                self.work_area = Some(Rect::from_min_size(
                    pos2(x as f32 / ppp, y as f32 / ppp),
                    vec2(w as f32 / ppp, h as f32 / ppp),
                ));
            }
        }
        if !self.initialized {
            self.initialized = true;
            if let Some(h) = crate::win::find_own_window(crate::HOST_TITLE) {
                crate::win::prepare_widget_window(h);
            }
            if !self.state.settings.seeded {
                widgets::launcher::seed_defaults(self);
            }
            // `--show weather,system,…` turns widgets on at start.
            if let Some(list) = std::env::args().skip_while(|a| a != "--show").nth(1) {
                for id in list.split(',') {
                    if let Some((w, _)) = WIDGETS.iter().find(|w| w.0 == id.trim()) {
                        self.set_open(w, true);
                    }
                }
            }
        }
        if let Some(rx) = &self.programs_rx {
            while let Ok(list) = rx.try_recv() {
                self.programs = list;
            }
        }
        self.icons.pump(ctx);
        // 0 = the sharp wallpaper used as the 'desktop behind the window'.
        let mut blurs: Vec<u32> = vec![0, self.state.settings.blur_size];
        for (id, _) in WIDGETS {
            let b = self.state.settings.blur_for(id);
            if !blurs.contains(&b) {
                blurs.push(b);
            }
        }
        self.glass.pump(ctx, &blurs);
        self.media.pump(ctx);
        self.cal.pump(&self.calendar);
        let weather_open = self.is_open("weather");
        self.weather.pump(&self.extras, weather_open);
        let system_open = self.is_open("system");
        self.sys.pump(system_open);
        if let Some(msg) = self.pomodoro.pump() {
            self.notify(msg);
        }
        let photos_open = self.is_open("photos");
        self.photos.pump(ctx, &self.extras, photos_open);
        self.handle_tray(ctx);
        if self.quitting {
            return;
        }

        // The host window itself paints nothing.
        egui::CentralPanel::default().frame(Frame::NONE).show(ctx, |_| {});

        if self.reopen_all {
            self.reopen_all = false;
            ctx.request_repaint();
        } else {
            for (id, _) in WIDGETS {
                if self.is_open(id) {
                    self.widget_viewport(ctx, id);
                }
            }
        }
        if let Some(id) = self.widget_settings {
            if self.ws_for != Some(id) {
                // A different widget opens a new window: prepare it again
                // (transparency, desktop owner) and start with the gallery closed.
                self.ws_for = Some(id);
                self.ws_ui = Default::default();
                self.ws_hwnd = None;
            }
            self.widget_settings_viewport(ctx, id);
        } else {
            self.ws_hwnd = None;
            self.ws_for = None;
        }
        if self.widgets_panel_open {
            self.panel_viewport(ctx);
        } else {
            self.panel_hwnd = None;
        }
        if self.settings_open {
            self.settings_viewport(ctx);
        } else {
            self.settings_hwnd = None;
        }
        self.keep_on_desktop();
        self.maybe_flush();
        if self.toast.as_ref().is_some_and(|t| Instant::now() > t.until) {
            self.toast = None;
        }
        ctx.request_repaint_after(self.next_repaint());
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.flush();
    }
}

