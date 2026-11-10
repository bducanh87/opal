//! Frosted glass. Like Lumen's `wallpaper.ts`, the desktop wallpaper is
//! loaded, blurred once and drawn behind each widget at the widget's screen
//! position, so the glass lines up with the real wallpaper underneath. A
//! theme veil, a frost haze, film grain and a hairline edge go on top.

use egui::epaint::RectShape;
use egui::{
    Color32, ColorImage, Context, CornerRadius, Painter, Pos2, Rect, Shape, StrokeKind,
    TextureHandle, TextureOptions, TextureWrapMode, Vec2, pos2, vec2,
};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant, SystemTime};

use crate::theme::{self, Ink};

struct Job {
    path: String,
    blur: u32,
}

pub struct Glass {
    /// Blurred wallpaper per blur size (widgets may differ).
    walls: HashMap<u32, TextureHandle>,
    wall_size: Vec2,
    source: Option<(String, Option<SystemTime>)>,
    pending: std::collections::HashSet<u32>,
    /// Blur sizes that failed for the current wallpaper; not retried until
    /// the wallpaper changes (an unreadable file would otherwise loop).
    failed: std::collections::HashSet<u32>,
    /// Sizes whose texture shows an older wallpaper.
    stale: std::collections::HashSet<u32>,
    checked: Instant,
    jobs: Sender<Job>,
    results: Receiver<(Job, Option<ColorImage>)>,
    themes: HashMap<String, TextureHandle>,
    grain: Option<TextureHandle>,
}

/// Everything needed to paint one widget surface.
#[derive(Clone)]
pub struct Look {
    pub theme: TextureHandle,
    pub frosted: bool,
    /// 0..1, Lumen's glass tint (dark veil).
    pub veil: f32,
    /// 0..1, how strongly the frosted wallpaper shows.
    pub frost: f32,
    /// Blur size in px; picks the blurred wallpaper texture.
    pub blur: u32,
    /// 0..1, opacity of the whole backing.
    pub opacity: f32,
    pub ink: Ink,
    pub light: bool,
    /// Window inner rect origin on screen, in points.
    pub window_origin: Pos2,
    pub ppp: f32,
}

impl Glass {
    pub fn new(ctx: &Context) -> Self {
        let (jobs, rx) = channel::<Job>();
        let (tx, results) = channel();
        let repaint = ctx.clone();
        std::thread::spawn(move || {
            // The decoded, downscaled wallpaper is kept so a new blur size
            // only costs a blur, not another decode of a 4K image.
            let mut base: Option<(String, Option<image::RgbaImage>)> = None;
            while let Ok(job) = rx.recv() {
                if base.as_ref().is_none_or(|b| b.0 != job.path) {
                    base = Some((job.path.clone(), load_base(&job.path)));
                }
                let image = base.as_ref().and_then(|b| b.1.as_ref()).map(|img| blur_image(img, job.blur));
                if tx.send((job, image)).is_err() {
                    break;
                }
                repaint.request_repaint();
            }
        });
        Glass {
            walls: HashMap::new(),
            wall_size: Vec2::ZERO,
            source: None,
            pending: Default::default(),
            failed: Default::default(),
            stale: Default::default(),
            checked: Instant::now() - Duration::from_secs(60),
            jobs,
            results,
            themes: HashMap::new(),
            grain: None,
        }
    }

    /// Watch the wallpaper file, and make sure a blurred copy exists for
    /// every blur size in use.
    pub fn pump(&mut self, ctx: &Context, blurs: &[u32]) {
        while let Ok((job, image)) = self.results.try_recv() {
            self.pending.remove(&job.blur);
            if self.source.as_ref().is_none_or(|s| s.0 != job.path) {
                continue;
            }
            match image {
                Some(img) => {
                    self.wall_size = vec2(img.size[0] as f32, img.size[1] as f32);
                    match self.walls.get_mut(&job.blur) {
                        Some(tex) => tex.set(img, TextureOptions::LINEAR),
                        None => {
                            let tex = ctx.load_texture(format!("wallpaper:{}", job.blur), img, TextureOptions::LINEAR);
                            self.walls.insert(job.blur, tex);
                        }
                    }
                }
                None => {
                    self.failed.insert(job.blur);
                }
            }
        }
        if self.checked.elapsed() >= Duration::from_secs(3) {
            self.checked = Instant::now();
            match crate::win::wallpaper_path() {
                Some(path) => {
                    let stamp = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
                    let now = Some((path, stamp));
                    if now != self.source {
                        // New wallpaper: keep the old textures on screen,
                        // but re-render every size (one at a time, below).
                        self.source = now;
                        self.failed.clear();
                        self.stale = self.walls.keys().copied().collect();
                    }
                }
                None => {
                    self.source = None;
                    self.walls.clear();
                }
            }
        }
        // One job in flight at a time: dragging the blur slider then costs
        // one blur per settled value instead of a queue of dozens.
        if self.pending.is_empty() {
            if let Some((path, _)) = &self.source {
                let next = blurs
                    .iter()
                    .copied()
                    .find(|b| (!self.walls.contains_key(b) || self.stale.contains(b)) && !self.failed.contains(b));
                if let Some(b) = next {
                    self.stale.remove(&b);
                    self.pending.insert(b);
                    let _ = self.jobs.send(Job { path: path.clone(), blur: b });
                }
            }
        }
        // Drop sizes nobody uses any more (keeps memory bounded).
        if self.walls.len() > blurs.len() + 2 {
            self.walls.retain(|b, _| blurs.contains(b));
        }
    }

    /// The blurred wallpaper closest to the requested blur.
    fn wall_for(&self, blur: u32) -> Option<TextureHandle> {
        if let Some(t) = self.walls.get(&blur) {
            return Some(t.clone());
        }
        self.walls
            .iter()
            .min_by_key(|(b, _)| (**b as i64 - blur as i64).abs())
            .map(|(_, t)| t.clone())
    }

    pub fn has_wallpaper(&self) -> bool {
        !self.walls.is_empty()
    }

    pub fn theme_texture(&mut self, ctx: &Context, key: &str, make: impl FnOnce() -> ColorImage) -> TextureHandle {
        self.themes
            .entry(key.to_string())
            .or_insert_with(|| ctx.load_texture(format!("theme:{key}"), make(), TextureOptions::LINEAR))
            .clone()
    }

    fn grain(&mut self, ctx: &Context) -> TextureHandle {
        self.grain
            .get_or_insert_with(|| {
                let n = 128;
                let mut seed: u32 = 0x9e37_79b9;
                let pixels = (0..n * n)
                    .map(|_| {
                        seed ^= seed << 13;
                        seed ^= seed >> 17;
                        seed ^= seed << 5;
                        let v = (seed & 0xff) as u8;
                        crate::theme::css(v, v, v, 255)
                    })
                    .collect();
                let options = TextureOptions {
                    wrap_mode: TextureWrapMode::Repeat,
                    ..TextureOptions::NEAREST
                };
                ctx.load_texture("grain", ColorImage { size: [n, n], pixels }, options)
            })
            .clone()
    }

    /// Build the look for a widget from settings.
    pub fn look(&mut self, ctx: &Context, settings: &crate::config::Settings, widget: &str) -> Look {
        let design = settings.design(widget);
        let (key, light, own_surface) = match theme::design_surface(widget, &design) {
            Some((_, _, light)) => (format!("design:{widget}:{design}"), light, true),
            None => {
                let t = theme::get(settings.theme_for(widget));
                (format!("theme:{}", t.id), t.light, false)
            }
        };
        let tex = self.theme_texture(ctx, &key, || {
            if let Some((base, radials, _)) = theme::design_surface(widget, &design) {
                theme::render_background(base, radials, 96, 96)
            } else {
                theme::render_theme(theme::get(settings.theme_for(widget)))
            }
        });
        let frosted = settings.frosted(widget) && settings.wallpaper_frost_for(widget) && self.has_wallpaper();
        // Designs with their own paper/colour surface stay mostly opaque.
        // Lumen's themeBlend(): the tint when frosted, else the full design.
        let mut veil = if settings.frosted(widget) { settings.veil_for(widget) as f32 / 100.0 } else { 1.0 };
        // Designs with their own surface (paper, film, terminal…) must read
        // as that surface; the wallpaper only shows through faintly.
        if own_surface {
            veil = veil.max(0.88);
        }
        let (origin, ppp) = ctx.input(|i| {
            let origin = i.viewport().inner_rect.map(|r| r.min).unwrap_or(Pos2::ZERO);
            (origin, i.pixels_per_point)
        });
        Look {
            theme: tex,
            frosted,
            veil,
            frost: settings.frost_for(widget) as f32 / 100.0,
            blur: settings.blur_for(widget),
            opacity: settings.opacity(widget) as f32 / 100.0,
            ink: Ink::for_light(light),
            light,
            window_origin: origin,
            ppp,
        }
    }

    /// Paint a glass plate exactly like Lumen's renderer component:
    ///
    /// ```text
    /// .glass        background rgba(12,10,20,.5)        (porcelain #f8f8ff73)
    ///  .glass-wall  wallpaper blur(blurSize) saturate(1.45) brightness(.84)
    ///               opacity .22 + frostStrength * .78
    ///  .glass-theme theme background, opacity = glass tint (veil)
    ///  .glass-sheen radial(120% 70% at 16% -18%, #fff 26% -> 0 55%)
    ///               + linear(#fff 10% -> 0 24%, 0 76% -> #fff 3.5%)
    ///  .glass-grain grain.png, overlay .55
    /// .plate:before 1px rim, 155deg: 62% -> 10% @30% -> 4% @64% -> 34%
    /// ```
    /// Everything sits under `opacity: var(--surface)` (background opacity).
    pub fn paint(&mut self, ctx: &Context, painter: &Painter, rect: Rect, radius: f32, look: &Look) {
        let radius = radius.min(rect.width().min(rect.height()) * 0.25);
        let cr = CornerRadius::same(radius.round().clamp(0.0, 255.0) as u8);
        let full = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        let surface = look.opacity;
        if surface <= 0.001 {
            return;
        }
        let blend = look.veil.clamp(0.0, 1.0);
        let dbg: u32 = std::env::var("OPAL_GLASS_DEBUG").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
        if dbg & 64 != 0 {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(crate::config::config_file("glass-debug.log")) {
                let _ = writeln!(f, "rect={rect:?} light={} veil={} frost={} blur={} opacity={} frosted={} wall={}", look.light, look.veil, look.frost, look.blur, look.opacity, look.frosted, self.walls.len());
            }
        }
        let wall = if look.frosted && blend < 1.0 { self.wall_for(look.blur) } else { None };
        let uv = self.wallpaper_uv(rect, look);

        // What shows through the transparent Electron window: the desktop.
        // Our window is opaque, so stand in with the wallpaper itself.
        if let Some(w) = &wall {
            if dbg & 1 == 0 {
                // Stand-in for the desktop seen through Electron's window,
                // calibrated against the original (it reads ~20% darker).
                // Use the unblurred wallpaper so detail shows through, as the
                // real desktop does behind Lumen's transparent window.
                let sharp = self.walls.get(&0).cloned().unwrap_or_else(|| w.clone());
                let v = (255.0 * 0.78 * surface) as u8;
                painter.add(Shape::Rect(RectShape::filled(rect, cr, Color32::from_rgba_premultiplied(v, v, v, (255.0 * surface) as u8)).with_texture(sharp.id(), uv)));
            }
        }
        // .glass background
        let base = if look.light {
            crate::theme::css(248, 248, 255, 115)
        } else {
            crate::theme::css(12, 10, 20, 128)
        };
        painter.rect_filled(rect, cr, base.gamma_multiply(surface));
        if let Some(w) = &wall {
            // .glass-wall: brightness(.84) (porcelain 1.02) at opacity k.
            let k = (0.22 + look.frost * 0.78).clamp(0.0, 1.0) * surface;
            let bright = if look.light { 1.0 } else { 0.72 };
            let v = (255.0 * bright * k).round() as u8;
            let tint = Color32::from_rgba_premultiplied(v, v, v, (255.0 * k).round() as u8);
            painter.add(Shape::Rect(RectShape::filled(rect, cr, tint).with_texture(w.id(), uv)));
        }
        // .glass-theme at the glass tint (fully opaque when not frosted).
        let theme_alpha = if wall.is_some() { blend } else { 1.0 };
        if dbg & 4 == 0 {
        painter.add(Shape::Rect(
            RectShape::filled(rect, cr, Color32::WHITE.gamma_multiply(theme_alpha * surface)).with_texture(look.theme.id(), full),
        ));
        }
        // .glass-sheen, drawn as a vertex-coloured mesh: translucent textures
        // come out several times too bright through egui's sRGB path.
        if dbg & 8 == 0 {
            sheen_mesh(painter, rect, radius, look.light, surface);
        }
        // .glass-grain (overlay .55 is close to a faint neutral noise)
        let grain = self.grain(ctx);
        let px = rect.size() * look.ppp / 160.0;
        let grain_alpha = if look.light { 0.008 } else { 0.012 };
        painter.add(Shape::Rect(
            RectShape::filled(rect, cr, Color32::WHITE.gamma_multiply(grain_alpha * surface))
                .with_texture(grain.id(), Rect::from_min_size(pos2(0.0, 0.0), px)),
        ));
        // box-shadow 0 0 0 1px rgba(0,0,0,.14) then the lit rim.
        painter.rect_stroke(rect, cr, egui::Stroke::new(1.0_f32, Color32::from_black_alpha((0.14 * 255.0 * surface) as u8)), StrokeKind::Outside);
        rim(painter, rect, radius, surface, look.light);
    }




    /// Map a window-local rect onto the wallpaper as Windows "Fill" lays it
    /// out on the primary monitor.
    fn wallpaper_uv(&self, rect: Rect, look: &Look) -> Rect {
        let (sw, sh) = crate::win::screen_size();
        let screen = vec2(sw.max(1) as f32, sh.max(1) as f32);
        let img = self.wall_size.max(vec2(1.0, 1.0));
        let scale = (screen.x / img.x).max(screen.y / img.y);
        let shown = img * scale;
        let offset = (screen - shown) / 2.0;
        let to_uv = |p: Pos2| {
            let phys = (look.window_origin.to_vec2() + p.to_vec2()) * look.ppp;
            pos2((phys.x - offset.x) / shown.x, (phys.y - offset.y) / shown.y)
        };
        Rect::from_min_max(to_uv(rect.min), to_uv(rect.max))
    }
}

/// `.glass-sheen`: radial-gradient(120% 70% at 16% -18%, #ffffff42, #0000 55%)
/// over linear-gradient(#ffffff1a, #0000 24% 76%, #ffffff09). Porcelain
/// uses #ffffffbf / #ffffff8c / #ffffff38 and a 58% radial stop.
/// CSS `.glass-sheen` alpha at a point (u, v in 0..1 of the plate).
fn sheen_alpha(light: bool, u: f32, v: f32) -> f32 {
    let (ra, stop, top, bottom) = if light { (0.75, 0.58, 0.55, 0.22) } else { (0.26, 0.55, 0.10, 0.035) };
    let d = (((u - 0.16) / 1.2).powi(2) + ((v + 0.18) / 0.7).powi(2)).sqrt();
    let a1 = ra * (1.0 - d / stop).clamp(0.0, 1.0);
    let a2 = if v < 0.24 {
        top * (1.0 - v / 0.24)
    } else if v > 0.76 {
        bottom * (v - 0.76) / 0.24
    } else {
        0.0
    };
    a1 + a2 * (1.0 - a1)
}

/// Sheen as a dense vertex-coloured grid clamped inside the rounded plate
/// (the plate is convex, so clamped triangles never leave it).
fn sheen_mesh(painter: &Painter, rect: Rect, radius: f32, light: bool, surface: f32) {
    use egui::epaint::{Mesh, Vertex, WHITE_UV};
    let (nx, ny) = (36usize, 30usize);
    let r = radius.max(0.0);
    let clamp_in = |p: Pos2| -> Pos2 {
        let cx = p.x.clamp(rect.min.x + r, rect.max.x - r);
        let cy = p.y.clamp(rect.min.y + r, rect.max.y - r);
        let c = pos2(cx, cy);
        let d = p - c;
        let len = d.length();
        if len > r && len > 0.0 { c + d * (r / len) } else { p }
    };
    let mut mesh = Mesh::default();
    for j in 0..=ny {
        for i in 0..=nx {
            let u = i as f32 / nx as f32;
            let v = j as f32 / ny as f32;
            let p = clamp_in(pos2(rect.min.x + u * rect.width(), rect.min.y + v * rect.height()));
            let a = (sheen_alpha(light, u, v) * surface * 255.0).round().clamp(0.0, 255.0) as u8;
            mesh.vertices.push(Vertex { pos: p, uv: WHITE_UV, color: crate::theme::white(a) });
        }
    }
    let w = (nx + 1) as u32;
    for j in 0..ny as u32 {
        for i in 0..nx as u32 {
            let a = j * w + i;
            mesh.indices.extend_from_slice(&[a, a + 1, a + w, a + 1, a + w + 1, a + w]);
        }
    }
    painter.add(Shape::mesh(mesh));
}

#[allow(dead_code)]
fn render_sheen(light: bool, _aspect: f32) -> ColorImage {
    let (ra, stop, top, bottom) = if light { (0.75, 0.58, 0.55, 0.22) } else { (0.26, 0.55, 0.10, 0.035) };
    let (w, h) = (96usize, 96usize);
    let mut pixels = Vec::with_capacity(w * h);
    const BAYER: [f32; 16] = [0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0];
    for y in 0..h {
        for x in 0..w {
            let u = (x as f32 + 0.5) / w as f32;
            let v = (y as f32 + 0.5) / h as f32;
            // CSS ellipse with explicit radii 120% x 70%.
            let d = (((u - 0.16) / 1.2).powi(2) + ((v + 0.18) / 0.7).powi(2)).sqrt();
            let a1 = ra * (1.0 - d / stop).clamp(0.0, 1.0);
            let a2 = if v < 0.24 {
                top * (1.0 - v / 0.24)
            } else if v > 0.76 {
                bottom * (v - 0.76) / 0.24
            } else {
                0.0
            };
            // Both layers are white: source-over composite of their alphas.
            let a = a1 + a2 * (1.0 - a1);
            let dither = BAYER[(y % 4) * 4 + (x % 4)] / 16.0 - 0.5;
            pixels.push(crate::theme::white((a * 255.0 + dither).round().clamp(0.0, 255.0) as u8));
        }
    }
    ColorImage { size: [w, h], pixels }
}

/// `.plate:before`: a 1px rim lit by a 155° gradient (bright top-left,
/// faint middle, soft bottom-right).
fn rim(painter: &Painter, rect: Rect, radius: f32, opacity: f32, light: bool) {
    use egui::epaint::{ColorMode, PathShape, PathStroke};
    let stops: [(f32, f32); 4] = if light {
        [(0.0, 0.98), (0.34, 0.50), (0.70, 0.62), (1.0, 0.95)]
    } else {
        [(0.0, 0.62), (0.30, 0.10), (0.64, 0.04), (1.0, 0.34)]
    };
    let r = rect.shrink(0.5);
    let mut pts = Vec::new();
    let rad = (radius - 0.5).max(0.0);
    let corners = [
        (pos2(r.max.x - rad, r.min.y + rad), -90.0_f32),
        (pos2(r.max.x - rad, r.max.y - rad), 0.0),
        (pos2(r.min.x + rad, r.max.y - rad), 90.0),
        (pos2(r.min.x + rad, r.min.y + rad), 180.0),
    ];
    for (c, start) in corners {
        for k in 0..=12 {
            let a = (start + 90.0 * k as f32 / 12.0).to_radians();
            pts.push(c + vec2(a.cos(), a.sin()) * rad);
        }
    }
    let th = 155.0_f32.to_radians();
    let dir = vec2(th.sin(), -th.cos());
    let color = move |bounds: Rect, p: Pos2| {
        let len = bounds.width() * dir.x.abs() + bounds.height() * dir.y.abs();
        let t = ((p - bounds.center()).dot(dir) / len.max(1.0) + 0.5).clamp(0.0, 1.0);
        let mut a = stops[3].1;
        for w in stops.windows(2) {
            if t <= w[1].0 {
                let f = (t - w[0].0) / (w[1].0 - w[0].0);
                a = w[0].1 + (w[1].1 - w[0].1) * f;
                break;
            }
        }
        crate::theme::white((a * opacity * 255.0) as u8)
    };
    let stroke = PathStroke {
        width: 1.0,
        color: ColorMode::UV(std::sync::Arc::new(color)),
        kind: StrokeKind::Middle,
    };
    painter.add(PathShape::closed_line(pts, stroke));
}

/// Decode and downscale the wallpaper once.
fn load_base(path: &str) -> Option<image::RgbaImage> {
    let reader = image::ImageReader::open(path).ok()?.with_guessed_format().ok()?;
    let img = reader.decode().ok()?;
    let target_w = 960u32;
    let img = if img.width() > target_w {
        let h = (img.height() as f64 * target_w as f64 / img.width() as f64).round() as u32;
        img.resize_exact(target_w, h.max(1), image::imageops::FilterType::Triangle)
    } else {
        img
    };
    Some(img.to_rgba8())
}

/// blur(px) saturate(1.45) like Lumen's `.glass-wall`. Blur 0 is the plain
/// "desktop behind the window" layer and stays unsaturated.
fn blur_image(base: &image::RgbaImage, blur: u32) -> ColorImage {
    let mut rgba = base.clone();
    if blur > 0 {
        for p in rgba.pixels_mut() {
            let [r, g, b, _] = p.0;
            let l = 0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32;
            for (i, c) in [r, g, b].into_iter().enumerate() {
                p.0[i] = (l + (c as f32 - l) * 1.45).clamp(0.0, 255.0) as u8;
            }
        }
    }
    let (sw, _) = crate::win::screen_size();
    let factor = rgba.width() as f32 / sw.max(1) as f32;
    let sigma = (blur as f32 * factor).max(0.0); // CSS blur(px) = sigma in px
    let mut blurred = if sigma > 0.3 { image::imageops::fast_blur(&rgba, sigma) } else { rgba };
    crate::theme::avoid_key(&mut blurred);
    let size = [blurred.width() as usize, blurred.height() as usize];
    ColorImage::from_rgba_unmultiplied(size, blurred.as_raw())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_sheen_matches_css() {
        let img = render_sheen(false, 1.0);
        let at = |u: f32, v: f32| img.pixels[(v * 96.0) as usize * 96 + (u * 96.0) as usize].a();
        eprintln!("topleft={} top-mid={} bottomleft={} mid={}", at(0.15, 0.15), at(0.5, 0.05), at(0.2, 0.85), at(0.5, 0.5));
        assert!(at(0.2, 0.85) < 8, "bottom should be nearly clear");
        assert!(at(0.15, 0.15) > 10, "top-left glow");
    }
}
