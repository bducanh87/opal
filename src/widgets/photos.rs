//! Photo frame: a slideshow of the images in a folder (Pictures by default),
//! cover-cropped into the glass with a soft cross-fade.

use egui::{Align2, Color32, ColorImage, CornerRadius, Id, Rect, Sense, Shape, Stroke, TextureHandle, TextureOptions, Ui, pos2, vec2};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use super::{font, semibold};
use crate::app::{App, glyph};
use crate::config::Extras;
use crate::glass::Look;
use crate::theme::white;

pub struct PhotosState {
    folder: String,
    files: Vec<PathBuf>,
    index: usize,
    current: Option<(TextureHandle, String)>,
    previous: Option<TextureHandle>,
    fade_start: Option<Instant>,
    shown_at: Instant,
    loading: bool,
    tx: Sender<PathBuf>,
    rx: Receiver<(PathBuf, Option<ColorImage>)>,
    scan_rx: Option<Receiver<(String, Vec<PathBuf>)>>,
    pub status: String,
    /// Unreadable files in a row; stops a folder of bad files from looping.
    failures: usize,
}

fn is_image(p: &Path) -> bool {
    matches!(
        p.extension().map(|e| e.to_string_lossy().to_lowercase()).as_deref(),
        Some("jpg" | "jpeg" | "png" | "webp" | "bmp")
    )
}

fn scan(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 3 || out.len() > 5000 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            scan(&p, out, depth + 1);
        } else if is_image(&p) {
            out.push(p);
        }
    }
}

/// Deterministic shuffle so the order varies but stays stable per folder.
fn shuffle(files: &mut [PathBuf]) {
    let mut seed: u64 = 0x2545_f491_4f6c_dd1d ^ files.len() as u64;
    for i in (1..files.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        files.swap(i, (seed % (i as u64 + 1)) as usize);
    }
}

impl PhotosState {
    pub fn new(ctx: &egui::Context) -> Self {
        let (tx, jobs) = channel::<PathBuf>();
        let (done, rx) = channel();
        let repaint = ctx.clone();
        std::thread::spawn(move || {
            while let Ok(path) = jobs.recv() {
                let img = image::ImageReader::open(&path)
                    .ok()
                    .and_then(|r| r.with_guessed_format().ok())
                    .and_then(|r| r.decode().ok())
                    .map(|img| {
                        let img = if img.width() > 1400 || img.height() > 1400 { img.thumbnail(1400, 1400) } else { img };
                        let mut rgba = img.to_rgba8();
                        crate::theme::avoid_key(&mut rgba);
                        ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw())
                    });
                if done.send((path, img)).is_err() {
                    break;
                }
                repaint.request_repaint();
            }
        });
        PhotosState {
            folder: String::new(),
            files: Vec::new(),
            index: 0,
            current: None,
            previous: None,
            fade_start: None,
            shown_at: Instant::now(),
            loading: false,
            tx,
            rx,
            scan_rx: None,
            status: String::new(),
            failures: 0,
        }
    }

    fn request(&mut self) {
        if let Some(path) = self.files.get(self.index).cloned() {
            self.loading = true;
            let _ = self.tx.send(path);
        }
    }

    pub fn step(&mut self, delta: i64) {
        if self.files.is_empty() {
            return;
        }
        let n = self.files.len() as i64;
        self.index = ((self.index as i64 + delta).rem_euclid(n)) as usize;
        self.request();
    }

    /// Animating while a cross-fade runs.
    pub fn animating(&self) -> bool {
        self.fade_start.is_some_and(|t| t.elapsed() < Duration::from_millis(700))
    }

    pub fn pump(&mut self, ctx: &egui::Context, extras: &Extras, active: bool) {
        if !active {
            return;
        }
        if extras.photos_folder != self.folder {
            self.folder = extras.photos_folder.clone();
            self.files.clear();
            self.current = None;
            self.status = "Looking for photos…".into();
            let (tx, rx) = channel();
            let folder = self.folder.clone();
            let repaint = ctx.clone();
            std::thread::spawn(move || {
                let mut files = Vec::new();
                scan(Path::new(&folder), &mut files, 0);
                shuffle(&mut files);
                let _ = tx.send((folder, files));
                repaint.request_repaint();
            });
            self.scan_rx = Some(rx);
        }
        if let Some(rx) = &self.scan_rx {
            if let Ok((folder, files)) = rx.try_recv() {
                self.scan_rx = None;
                if folder == self.folder {
                    self.status = if files.is_empty() { "No photos in this folder".into() } else { String::new() };
                    self.files = files;
                    self.index = 0;
                    self.request();
                }
            }
        }
        while let Ok((path, img)) = self.rx.try_recv() {
            self.loading = false;
            // A result for a folder we already left: ignore it.
            if !self.files.contains(&path) {
                continue;
            }
            match img {
                Some(img) => {
                    let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    let tex = ctx.load_texture(format!("photo:{}", path.display()), img, TextureOptions::LINEAR);
                    self.previous = self.current.take().map(|c| c.0);
                    self.current = Some((tex, name));
                    self.fade_start = Some(Instant::now());
                    self.shown_at = Instant::now();
                    self.failures = 0;
                }
                None => {
                    // Drop the unreadable file and try the next one, but give
                    // up once every file has failed.
                    self.failures += 1;
                    if let Some(pos) = self.files.iter().position(|f| *f == path) {
                        self.files.remove(pos);
                        if self.index >= self.files.len() {
                            self.index = 0;
                        }
                    }
                    if self.files.is_empty() {
                        self.status = "No readable photos in this folder".into();
                    } else if self.failures <= self.files.len() + 1 {
                        self.request();
                    }
                }
            }
        }
        let interval = Duration::from_secs(extras.photos_interval as u64);
        if !self.loading && !self.files.is_empty() && self.current.is_some() && self.shown_at.elapsed() >= interval {
            self.step(1);
        }
        if self.animating() {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }
}

fn cover_uv(tex: &TextureHandle, rect: Rect) -> Rect {
    let [w, h] = tex.size();
    let img = w as f32 / h.max(1) as f32;
    let target = rect.width() / rect.height().max(1.0);
    if img > target {
        let m = (1.0 - target / img) / 2.0;
        Rect::from_min_max(pos2(m, 0.0), pos2(1.0 - m, 1.0))
    } else {
        let m = (1.0 - img / target) / 2.0;
        Rect::from_min_max(pos2(0.0, m), pos2(1.0, 1.0 - m))
    }
}

fn draw(p: &egui::Painter, tex: &TextureHandle, rect: Rect, alpha: f32, radius: u8) {
    let uv = cover_uv(tex, rect);
    p.add(Shape::Rect(
        egui::epaint::RectShape::filled(rect, CornerRadius::same(radius), Color32::WHITE.gamma_multiply(alpha)).with_texture(tex.id(), uv),
    ));
}

/// The current photo (cross-faded) in `r`. Returns false when there is none.
fn photo(app: &App, p: &egui::Painter, r: Rect, radius: u8) -> bool {
    let st = &app.photos;
    let Some((tex, _)) = &st.current else { return false };
    let t = st.fade_start.map(|t| (t.elapsed().as_secs_f32() / 0.7).min(1.0)).unwrap_or(1.0);
    if let (Some(prev), true) = (&st.previous, t < 1.0) {
        draw(p, prev, r, 1.0, radius);
    }
    draw(p, tex, r, t, radius);
    true
}

/// Small previous / next buttons at `at` (centre of the pair).
fn nav(app: &mut App, ui: &mut Ui, p: &egui::Painter, at: egui::Pos2, light_bg: bool) {
    let mut step = 0;
    for (dx, ch, d) in [(-16.0, glyph::LEFT, -1), (16.0, glyph::RIGHT, 1)] {
        let b = Rect::from_center_size(pos2(at.x + dx, at.y), vec2(26.0, 26.0));
        let resp = ui.interact(b, Id::new(("photo-step", d)), Sense::click());
        let (bg, fg) = if light_bg { (Color32::from_black_alpha(if resp.hovered() { 40 } else { 18 }), crate::theme::rgb(0x2b2622)) } else { (white(if resp.hovered() { 70 } else { 35 }), Color32::WHITE) };
        p.circle_filled(b.center(), 12.0, bg);
        p.text(b.center(), Align2::CENTER_CENTER, ch.to_string(), egui::FontId::new(9.0, egui::FontFamily::Name("icons".into())), fg);
        if resp.clicked() {
            step = d;
        }
    }
    if step != 0 {
        app.photos.step(step);
    }
}

fn hovering(ui: &Ui, r: Rect) -> bool {
    ui.ctx().input(|i| i.pointer.hover_pos()).is_some_and(|pt| r.contains(pt))
}

/// Polaroid: a white instant-photo frame with the file name as caption.
fn show_polaroid(app: &mut App, ui: &mut Ui, look: &Look) {
    let r = ui.max_rect();
    let p = ui.painter().clone();
    let pic = Rect::from_min_max(r.min + vec2(4.0, 4.0), pos2(r.max.x - 4.0, r.max.y - 46.0));
    p.rect_filled(pic, CornerRadius::same(2), crate::theme::rgb(0xd9d4c8));
    if !photo(app, &p, pic, 2) {
        return empty(app, ui, look);
    }
    p.rect_stroke(pic, CornerRadius::same(2), Stroke::new(1.0_f32, Color32::from_black_alpha(30)), egui::StrokeKind::Inside);
    let name = app.photos.current.as_ref().map(|c| c.1.clone()).unwrap_or_default();
    let ink = crate::theme::rgb(0x2b2622);
    let g = super::elided(ui, &name, super::serif(15.0), ink, r.width() - 90.0);
    p.galley(pos2(r.min.x + 8.0, r.max.y - 30.0), g, ink);
    if hovering(ui, r) {
        nav(app, ui, &p, pos2(r.max.x - 30.0, r.max.y - 20.0), true);
    }
}

/// Film: the photo on a dark strip between rows of sprocket holes.
fn show_film(app: &mut App, ui: &mut Ui, look: &Look) {
    let r = ui.max_rect().expand2(vec2(8.0, 4.0));
    let p = ui.painter().clone();
    let hole_h = 9.0;
    for row in [r.min.y + 4.0, r.max.y - 4.0 - hole_h] {
        let n = (r.width() / 22.0) as usize;
        for k in 0..n {
            let x = r.min.x + 8.0 + k as f32 * 22.0;
            p.rect_filled(Rect::from_min_size(pos2(x, row), vec2(12.0, hole_h)), CornerRadius::same(2), crate::theme::rgb(0x2a2a2a));
        }
    }
    let pic = Rect::from_min_max(pos2(r.min.x + 6.0, r.min.y + hole_h + 10.0), pos2(r.max.x - 6.0, r.max.y - hole_h - 10.0));
    if !photo(app, &p, pic, 3) {
        return empty(app, ui, look);
    }
    let orange = crate::theme::rgb(0xf0a030);
    let frame = format!("{}A", app.photos.index + 1);
    p.text(pos2(pic.min.x + 4.0, pic.max.y + 2.0), Align2::LEFT_TOP, frame, egui::FontId::monospace(9.0), orange);
    p.text(pos2(pic.max.x - 4.0, pic.max.y + 2.0), Align2::RIGHT_TOP, "OPAL 400", egui::FontId::monospace(9.0), orange);
    if hovering(ui, r) {
        nav(app, ui, &p, pos2(pic.max.x - 34.0, pic.max.y - 22.0), false);
    }
}

fn empty(app: &mut App, ui: &mut Ui, look: &Look) {
    let p = ui.painter().clone();
    let body = super::dot_header(&p, ui.max_rect(), "Photos", "", look);
    let msg = if app.photos.status.is_empty() { "Loading…".to_string() } else { app.photos.status.clone() };
    p.text(body.center() - vec2(0.0, 16.0), Align2::CENTER_CENTER, msg, font(12.5), look.ink.dim);
    let btn = Rect::from_center_size(body.center() + vec2(0.0, 18.0), vec2(150.0, 30.0));
    let resp = ui.interact(btn, Id::new("photos-folder"), Sense::click());
    p.rect_filled(btn, CornerRadius::same(15), if resp.hovered() { look.ink.hover_strong } else { white(22) });
    p.rect_stroke(btn, CornerRadius::same(15), Stroke::new(1.0_f32, white(60)), egui::StrokeKind::Inside);
    p.text(btn.center(), Align2::CENTER_CENTER, "Choose a folder…", semibold(12.0), look.ink.ink);
    if resp.clicked() {
        if let Some(dir) = rfd::FileDialog::new().set_title("Photo folder").pick_folder() {
            app.extras.photos_folder = dir.to_string_lossy().to_string();
            app.mark_extras();
        }
    }
}

pub fn show(app: &mut App, ui: &mut Ui, look: &Look) {
    match app.state.settings.design("photos").as_str() {
        "polaroid" => show_polaroid(app, ui, look),
        "film" => show_film(app, ui, look),
        _ => show_lumen(app, ui, look),
    }
}

fn show_lumen(app: &mut App, ui: &mut Ui, look: &Look) {
    // The photo fills the plate, inset a little inside the rim.
    let r = ui.max_rect().expand2(vec2(10.0, 8.0));
    let p = ui.painter().clone();
    let st = &app.photos;
    if let Some((tex, name)) = &st.current {
        let t = st.fade_start.map(|t| (t.elapsed().as_secs_f32() / 0.7).min(1.0)).unwrap_or(1.0);
        if let (Some(prev), true) = (&st.previous, t < 1.0) {
            draw(&p, prev, r, 1.0, 24);
        }
        draw(&p, tex, r, t, 24);
        let hovering = ui.ctx().input(|i| i.pointer.hover_pos()).is_some_and(|pt| r.contains(pt));
        if hovering {
            // caption band + arrows
            let band = Rect::from_min_max(pos2(r.min.x, r.max.y - 44.0), r.max);
            p.rect_filled(band, CornerRadius { nw: 0, ne: 0, sw: 24, se: 24 }, Color32::from_black_alpha(110));
            let caption = super::elided(ui, name, semibold(12.0), Color32::WHITE, band.width() - 120.0);
            p.galley(pos2(band.min.x + 16.0, band.center().y - caption.size().y / 2.0), caption, Color32::WHITE);
            p.text(pos2(band.max.x - 82.0, band.center().y), Align2::RIGHT_CENTER, format!("{} / {}", st.index + 1, st.files.len()), font(11.0), white(200));
            let mut step = 0;
            for (dx, ch, d) in [(-56.0, glyph::LEFT, -1), (-24.0, glyph::RIGHT, 1)] {
                let b = Rect::from_center_size(pos2(band.max.x + dx, band.center().y), vec2(28.0, 28.0));
                let resp = ui.interact(b, Id::new(("photo-step", d)), Sense::click());
                p.circle_filled(b.center(), 13.0, if resp.hovered() { white(70) } else { white(35) });
                p.text(b.center(), Align2::CENTER_CENTER, ch.to_string(), egui::FontId::new(10.0, egui::FontFamily::Name("icons".into())), Color32::WHITE);
                if resp.clicked() {
                    step = d;
                }
            }
            if step != 0 {
                app.photos.step(step);
            }
        }
        return;
    }
    // Empty state.
    let body = super::dot_header(&p, ui.max_rect(), "Photos", "", look);
    let msg = if app.photos.status.is_empty() { "Loading…".to_string() } else { app.photos.status.clone() };
    p.text(body.center() - vec2(0.0, 16.0), Align2::CENTER_CENTER, msg, font(12.5), look.ink.dim);
    let btn = Rect::from_center_size(body.center() + vec2(0.0, 18.0), vec2(150.0, 30.0));
    let resp = ui.interact(btn, Id::new("photos-folder"), Sense::click());
    p.rect_filled(btn, CornerRadius::same(15), if resp.hovered() { look.ink.hover_strong } else { white(22) });
    p.rect_stroke(btn, CornerRadius::same(15), Stroke::new(1.0_f32, white(60)), egui::StrokeKind::Inside);
    p.text(btn.center(), Align2::CENTER_CENTER, "Choose a folder…", semibold(12.0), look.ink.ink);
    if resp.clicked() {
        if let Some(dir) = rfd::FileDialog::new().set_title("Photo folder").pick_folder() {
            app.extras.photos_folder = dir.to_string_lossy().to_string();
            app.mark_extras();
        }
    }
}
