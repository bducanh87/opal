//! Music widget driven by the Windows media session (see `media.rs`).
//! Designs: Compact, Sleeve, Turntable.

use egui::{Align2, Color32, Context, CornerRadius, Id, Pos2, Rect, Sense, Shape, Stroke, Ui, pos2, vec2};

use super::{elided, font, semibold};
use crate::app::{App, glyph};
use crate::glass::Look;
use crate::media::Command;

fn fmt_time(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

fn cover(app: &App, p: &egui::Painter, rect: Rect, radius: u8, look: &Look) {
    if let Some(tex) = &app.media.cover {
        // Centre-crop to a square.
        let [w, h] = tex.size();
        let (w, h) = (w as f32, h as f32);
        let uv = if w > h {
            let m = (1.0 - h / w) / 2.0;
            Rect::from_min_max(pos2(m, 0.0), pos2(1.0 - m, 1.0))
        } else {
            let m = (1.0 - w / h) / 2.0;
            Rect::from_min_max(pos2(0.0, m), pos2(1.0, 1.0 - m))
        };
        p.add(Shape::Rect(
            egui::epaint::RectShape::filled(rect, CornerRadius::same(radius), Color32::WHITE).with_texture(tex.id(), uv),
        ));
    } else {
        p.rect_filled(rect, CornerRadius::same(radius), look.ink.hover);
        p.text(
            rect.center(),
            Align2::CENTER_CENTER,
            glyph::MUSIC.to_string(),
            egui::FontId::new(rect.height() * 0.36, egui::FontFamily::Name("icons".into())),
            look.ink.faint,
        );
    }
}

fn round_button(ui: &mut Ui, center: Pos2, r: f32, ch: char, filled: bool, look: &Look, id: &str) -> bool {
    let rect = Rect::from_center_size(center, vec2(r * 2.0, r * 2.0));
    let resp = ui.interact(rect, Id::new(("mbtn", id)), Sense::click());
    let p = ui.painter();
    if filled {
        p.circle_filled(center, r, if resp.hovered() { look.ink.ink } else { look.ink.ink.gamma_multiply(0.92) });
    } else if resp.hovered() {
        p.circle_filled(center, r, look.ink.hover);
    }
    let color = if filled { if look.light { Color32::WHITE } else { crate::theme::rgb(0x15131c) } } else { look.ink.ink };
    p.text(center, Align2::CENTER_CENTER, ch.to_string(), egui::FontId::new(r * 0.75, egui::FontFamily::Name("icons".into())), color);
    resp.clicked()
}

fn controls(app: &App, ui: &mut Ui, center: Pos2, size: f32, look: &Look) {
    let playing = app.media.now.as_ref().is_some_and(|n| n.playing);
    if round_button(ui, center - vec2(size * 2.4, 0.0), size * 0.8, glyph::PREV, false, look, "prev") {
        app.media.send(Command::Previous);
    }
    if round_button(ui, center, size, if playing { glyph::PAUSE } else { glyph::PLAY }, true, look, "play") {
        app.media.send(Command::PlayPause);
    }
    if round_button(ui, center + vec2(size * 2.4, 0.0), size * 0.8, glyph::NEXT, false, look, "next") {
        app.media.send(Command::Next);
    }
}

fn progress(app: &App, ui: &mut Ui, rect: Rect, look: &Look, show_times: bool) {
    let Some(now) = &app.media.now else { return };
    if now.duration <= 0.0 {
        return;
    }
    let pos = app.media.position();
    let frac = (pos / now.duration).clamp(0.0, 1.0) as f32;
    let bar = Rect::from_min_size(pos2(rect.min.x, rect.center().y - 2.0), vec2(rect.width(), 4.0));
    let resp = ui.interact(bar.expand2(vec2(0.0, 6.0)), Id::new("seek"), Sense::click_and_drag());
    let p = ui.painter();
    p.rect_filled(bar, CornerRadius::same(2), look.ink.hair);
    p.rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * frac, 4.0)), CornerRadius::same(2), look.ink.ink);
    if resp.hovered() || resp.dragged() {
        p.circle_filled(pos2(bar.min.x + bar.width() * frac, bar.center().y), 5.0, look.ink.ink);
    }
    if now.can_seek && (resp.clicked() || resp.drag_stopped()) {
        if let Some(pt) = resp.interact_pointer_pos() {
            let f = ((pt.x - bar.min.x) / bar.width()).clamp(0.0, 1.0) as f64;
            app.media.send(Command::Seek(f * now.duration));
        }
    }
    if show_times {
        p.text(pos2(bar.min.x, bar.max.y + 6.0), Align2::LEFT_TOP, fmt_time(pos), font(11.0), look.ink.faint);
        p.text(pos2(bar.max.x, bar.max.y + 6.0), Align2::RIGHT_TOP, fmt_time(now.duration), font(11.0), look.ink.faint);
    }
}

fn idle(app: &mut App, ui: &mut Ui, rect: Rect, look: &Look) {
    let p = ui.painter().clone();
    let tall = rect.height() > 120.0;
    let cy = rect.center().y - if tall { 18.0 } else { 26.0 };
    p.text(pos2(rect.center().x, cy), Align2::CENTER_CENTER, "Nothing playing", semibold(15.0), look.ink.dim);
    p.text(
        pos2(rect.center().x, cy + 20.0),
        Align2::CENTER_CENTER,
        "Play in Spotify, a browser or any player",
        font(11.5),
        look.ink.faint,
    );
    // Two ways to start music the widget can follow and control.
    let w = 150.0;
    let gap = 10.0;
    let left = rect.center().x - w - gap / 2.0;
    let players: [(&str, &str, fn() -> Result<(), String>); 2] = [
        ("YouTube Music", "open-ytm", crate::media::open_youtube_music),
        ("Media Player", "open-wmp", crate::media::open_media_player),
    ];
    for (i, (label, idn, open)) in players.into_iter().enumerate() {
        let btn = Rect::from_min_size(pos2(left + i as f32 * (w + gap), cy + 35.0), vec2(w, 30.0));
        let resp = ui.interact(btn, Id::new(idn), Sense::click());
        p.rect_filled(btn, CornerRadius::same(15), if resp.hovered() { look.ink.hover_strong } else { crate::theme::white(22) });
        p.rect_stroke(btn, CornerRadius::same(15), Stroke::new(1.0_f32, crate::theme::white(60)), egui::StrokeKind::Inside);
        p.text(btn.left_center() + vec2(16.0, 0.0), Align2::LEFT_CENTER, glyph::MUSIC.to_string(), egui::FontId::new(11.0, egui::FontFamily::Name("icons".into())), look.ink.ink);
        p.text(btn.left_center() + vec2(34.0, 0.0), Align2::LEFT_CENTER, label, semibold(12.0), look.ink.ink);
        if resp.clicked() {
            if let Err(e) = open() {
                app.notify(e);
            }
        }
    }
}

pub fn show(app: &mut App, ui: &mut Ui, _ctx: &Context, look: &Look) {
    let r = ui.max_rect();
    if app.media.now.is_none() {
        cover(app, ui.painter(), Rect::from_min_size(r.min, vec2(0.0, 0.0)), 0, look);
        return idle(app, ui, r, look);
    }
    match app.state.settings.design("music").as_str() {
        "coverflow" => coverflow(app, ui, r, look),
        "sleeve" => sleeve(app, ui, r, look),
        "turntable" => turntable(app, ui, r, look),
        _ => compact(app, ui, r, look),
    }
}

fn titles(app: &App, ui: &Ui, pos: Pos2, width: f32, align_center: bool, look: &Look, big: f32) -> f32 {
    let now = app.media.now.as_ref().unwrap();
    let p = ui.painter();
    let t = elided(ui, &now.title, semibold(big), look.ink.ink, width);
    let sub = if now.artist.is_empty() { now.app.clone() } else { now.artist.clone() };
    let a = elided(ui, &sub, font(big * 0.82), look.ink.dim, width);
    let (tx, ax) = if align_center { (pos.x - t.size().x / 2.0, pos.x - a.size().x / 2.0) } else { (pos.x, pos.x) };
    let th = t.size().y;
    let ah = a.size().y;
    p.galley(pos2(tx, pos.y), t, look.ink.ink);
    p.galley(pos2(ax, pos.y + th + 2.0), a, look.ink.dim);
    th + ah + 2.0
}

fn compact(app: &mut App, ui: &mut Ui, r: Rect, look: &Look) {
    let side = (r.height() - 4.0).min(r.width() * 0.34).max(48.0);
    let art = Rect::from_min_size(pos2(r.min.x, r.center().y - side / 2.0), vec2(side, side));
    cover(app, ui.painter(), art, 12, look);
    let x = art.max.x + 14.0;
    let w = r.max.x - x;
    titles(app, ui, pos2(x, art.min.y + 2.0), w, false, look, 15.0);
    let btn = (side * 0.16).clamp(12.0, 20.0);
    let cy = art.max.y - btn - 18.0;
    controls(app, ui, pos2(x + btn * 3.2, cy), btn, look);
    progress(app, ui, Rect::from_min_size(pos2(x, art.max.y - 10.0), vec2(w - 4.0, 8.0)), look, false);
    if let Some(n) = &app.media.now {
        if !n.app.is_empty() && w > 220.0 {
            let x_right = r.max.x - 4.0;
            ui.painter().text(pos2(x_right, cy), Align2::RIGHT_CENTER, &n.app, font(11.0), look.ink.faint);
        }
    }
}

/// Coverflow: the cover large in the middle with angled ghost covers on
/// both sides, and a floating playback deck holding the track card.
fn coverflow(app: &mut App, ui: &mut Ui, r: Rect, look: &Look) {
    let p = ui.painter().clone();
    p.text(r.left_top(), Align2::LEFT_TOP, "♪  NOW PLAYING", semibold(9.5), look.ink.dim);
    let deck_h = (r.height() * 0.46).clamp(70.0, 120.0);
    let stage = Rect::from_min_max(pos2(r.min.x, r.min.y + 16.0), pos2(r.max.x, r.max.y - deck_h - 8.0));
    let side = (stage.height() * 0.92).min(stage.width() * 0.34).max(24.0);
    let c = stage.center();
    // ghost covers
    for dx in [-1.0_f32, 1.0] {
        let g = Rect::from_center_size(c + vec2(dx * side * 0.62, 0.0), vec2(side * 0.72, side * 0.8));
        if let Some(tex) = &app.media.cover {
            p.add(Shape::Rect(
                egui::epaint::RectShape::filled(g, CornerRadius::same(10), crate::theme::white(110))
                    .with_texture(tex.id(), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))),
            ));
        } else {
            p.rect_filled(g, CornerRadius::same(10), crate::theme::white(25));
        }
        p.rect_stroke(g, CornerRadius::same(10), Stroke::new(1.0_f32, crate::theme::white(40)), egui::StrokeKind::Inside);
    }
    let main = Rect::from_center_size(c, vec2(side, side));
    p.rect_filled(main.translate(vec2(0.0, 4.0)), CornerRadius::same(12), Color32::from_black_alpha(70));
    cover(app, &p, main, 12, look);
    p.rect_stroke(main, CornerRadius::same(12), Stroke::new(1.0_f32, crate::theme::white(60)), egui::StrokeKind::Inside);
    // deck
    let deck = Rect::from_min_max(pos2(r.min.x, r.max.y - deck_h), r.max);
    p.rect_filled(deck, CornerRadius::same(16), crate::theme::white(16));
    p.rect_stroke(deck, CornerRadius::same(16), Stroke::new(1.0_f32, look.ink.hair), egui::StrokeKind::Inside);
    controls(app, ui, pos2(deck.min.x + 60.0, deck.min.y + 20.0), 11.0, look);
    let card = Rect::from_min_max(pos2(deck.min.x + 8.0, deck.min.y + 38.0), pos2(deck.max.x - 8.0, deck.max.y - 8.0));
    p.rect_filled(card, CornerRadius::same(10), Color32::from_black_alpha(70));
    let art = Rect::from_min_size(card.min + vec2(6.0, (card.height() - 30.0).max(0.0) / 2.0), vec2(30.0, 30.0));
    cover(app, &p, art, 6, look);
    titles(app, ui, pos2(art.max.x + 10.0, art.min.y), card.max.x - art.max.x - 18.0, false, look, 12.0);
    progress(app, ui, Rect::from_min_size(pos2(card.min.x + 6.0, card.max.y - 6.0), vec2(card.width() - 12.0, 4.0)), look, false);
}

fn sleeve(app: &mut App, ui: &mut Ui, r: Rect, look: &Look) {
    let footer = 104.0;
    let side = (r.height() - footer).min(r.width()).max(40.0);
    let art = Rect::from_min_size(pos2(r.center().x - side / 2.0, r.min.y), vec2(side, side));
    let p = ui.painter().clone();
    p.rect_filled(art.translate(vec2(0.0, 6.0)).shrink(6.0), CornerRadius::same(16), Color32::from_black_alpha(60));
    cover(app, &p, art, 16, look);
    let y = art.max.y + 10.0;
    titles(app, ui, pos2(r.center().x, y), r.width() - 20.0, true, look, 15.0);
    progress(app, ui, Rect::from_min_size(pos2(r.min.x + 6.0, y + 46.0), vec2(r.width() - 12.0, 8.0)), look, false);
    controls(app, ui, pos2(r.center().x, r.max.y - 18.0), 17.0, look);
}

/// Source label for the header chip ("YOUTUBE MUSIC", "SPOTIFY", …).
fn source_label(app: &App) -> String {
    let Some(n) = &app.media.now else { return "MUSIC".into() };
    let a = n.app.to_lowercase();
    if a.contains("chrome") || a.contains("edge") || a.is_empty() {
        "YOUTUBE MUSIC".into()
    } else {
        n.app.to_uppercase()
    }
}

/// Lumen's Turntable: header chip, a vinyl with the cover as its label and
/// a tonearm, "STEREO / SIDE A" with a big title, a timed progress bar and
/// a control deck with a red play key.
fn turntable(app: &mut App, ui: &mut Ui, r: Rect, look: &Look) {
    use crate::theme::{rgb, white};
    let p = ui.painter().clone();
    let red = rgb(0xf0284a);
    // header
    let chip = Rect::from_min_size(r.min, vec2(26.0, 26.0));
    p.rect_filled(chip, CornerRadius::same(8), red);
    p.text(chip.center(), Align2::CENTER_CENTER, glyph::MUSIC.to_string(), egui::FontId::new(11.0, egui::FontFamily::Name("icons".into())), Color32::WHITE);
    let label: String = source_label(app).chars().flat_map(|c| [c, '\u{200A}']).collect();
    p.text(pos2(chip.max.x + 10.0, chip.center().y), Align2::LEFT_CENTER, label, semibold(10.5), look.ink.ink);

    // record + titles
    let deck_h = 64.0;
    let bar_h = 34.0;
    let top = chip.max.y + 12.0;
    let area_h = (r.max.y - top - deck_h - bar_h - 8.0).max(60.0);
    let rad = (area_h / 2.0).min(r.width() * 0.24);
    let c = pos2(r.min.x + rad + 4.0, top + area_h / 2.0);
    p.circle_filled(c + vec2(0.0, 5.0), rad, Color32::from_black_alpha(80));
    p.circle_filled(c, rad, rgb(0x161616));
    for k in 1..10 {
        p.circle_stroke(c, rad * (0.42 + k as f32 * 0.058), Stroke::new(0.8_f32, white(16)));
    }
    let a = app.media.angle;
    let label_r = rad * 0.4;
    if let Some(tex) = &app.media.cover {
        let mut mesh = egui::epaint::Mesh::with_texture(tex.id());
        mesh.add_rect_with_uv(Rect::from_center_size(c, vec2(label_r * 2.0, label_r * 2.0)), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        mesh.rotate(egui::emath::Rot2::from_angle(a), c);
        p.add(Shape::mesh(mesh));
        // round the square label off with a ring in the record's colour
        p.circle_stroke(c, label_r * 1.25, Stroke::new(label_r * 0.5, rgb(0x161616)));
    } else {
        p.circle_filled(c, label_r, red);
    }
    p.circle_filled(c, rad * 0.045, rgb(0xdedede));
    // tonearm from the top-right of the record
    let pivot = c + vec2(rad * 0.95, -rad * 0.95);
    let playing = app.media.now.as_ref().is_some_and(|n| n.playing);
    let tip = if playing { c + vec2(rad * 0.45, -rad * 0.2) } else { c + vec2(rad * 0.95, rad * 0.05) };
    p.circle_filled(pivot, 6.0, rgb(0xd9d4cc));
    p.line_segment([pivot, tip], Stroke::new(3.0_f32, rgb(0xeae6df)));
    p.rect_filled(Rect::from_center_size(tip, vec2(8.0, 12.0)), CornerRadius::same(2), rgb(0x3a3733));

    let tx = c.x + rad + 22.0;
    let tw = (r.max.x - tx).max(40.0);
    let side: String = "STEREO / SIDE A".chars().flat_map(|ch| [ch, '\u{200A}']).collect();
    p.text(pos2(tx, c.y - rad * 0.55), Align2::LEFT_TOP, side, semibold(9.0), look.ink.dim);
    if let Some(n) = app.media.now.clone() {
        let title = n.title.to_uppercase();
        let mut job = egui::text::LayoutJob::single_section(title, egui::TextFormat::simple(super::serif(22.0), look.ink.ink));
        job.wrap = egui::text::TextWrapping { max_width: tw, max_rows: 2, break_anywhere: false, overflow_character: Some('…') };
        let g = ui.fonts(|f| f.layout_job(job));
        let gh = g.size().y;
        p.galley(pos2(tx, c.y - rad * 0.55 + 16.0), g, look.ink.ink);
        let artist = elided(ui, &n.artist, font(12.0), look.ink.dim, tw);
        p.galley(pos2(tx, c.y - rad * 0.55 + 20.0 + gh), artist, look.ink.dim);
    }

    // progress with times
    let bar_y = top + area_h + 10.0;
    progress(app, ui, Rect::from_min_size(pos2(r.min.x, bar_y), vec2(r.width(), 8.0)), look, true);

    // control deck
    let deck = Rect::from_min_max(pos2(r.min.x, r.max.y - deck_h), r.max);
    p.rect_filled(deck, CornerRadius::same(16), white(14));
    p.rect_stroke(deck, CornerRadius::same(16), Stroke::new(1.0_f32, look.ink.hair), egui::StrokeKind::Inside);
    let mid = deck.center();
    let key = Rect::from_center_size(mid, vec2(46.0, 40.0));
    let kr = ui.interact(key, Id::new("tt-play"), Sense::click());
    p.rect_filled(key.translate(vec2(0.0, 3.0)), CornerRadius::same(10), Color32::from_black_alpha(70));
    p.rect_filled(key, CornerRadius::same(10), if kr.hovered() { rgb(0xff4766) } else { red });
    let glyph_play = if playing { glyph::PAUSE } else { glyph::PLAY };
    p.text(key.center(), Align2::CENTER_CENTER, glyph_play.to_string(), egui::FontId::new(15.0, egui::FontFamily::Name("icons".into())), Color32::WHITE);
    if kr.clicked() {
        app.media.send(Command::PlayPause);
    }
    for (dx, ch, cmd, idn) in [(-deck.width() * 0.33, glyph::PREV, Command::Previous, "tt-prev"), (deck.width() * 0.33, glyph::NEXT, Command::Next, "tt-next")] {
        let b = Rect::from_center_size(mid + vec2(dx, 0.0), vec2(34.0, 34.0));
        let br = ui.interact(b, Id::new(idn), Sense::click());
        if br.hovered() {
            p.rect_filled(b, CornerRadius::same(10), look.ink.hover);
        }
        p.text(b.center(), Align2::CENTER_CENTER, ch.to_string(), egui::FontId::new(13.0, egui::FontFamily::Name("icons".into())), look.ink.dim);
        if br.clicked() {
            app.media.send(cmd);
        }
    }
}
