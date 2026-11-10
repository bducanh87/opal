pub mod calendar;
pub mod clock;
pub mod lunar_cal;
pub mod photos;
pub mod pomodoro;
pub mod system;
pub mod todo;
pub mod weather;
pub mod launcher;
pub mod music;
pub mod notes;
pub mod panel;
pub mod settings;
pub mod wsettings;

use egui::text::{LayoutJob, TextWrapping};
use egui::{Color32, FontId, Galley, Ui};
use std::sync::Arc;

/// Single-line text that ends in an ellipsis when it does not fit.
pub fn elided(ui: &Ui, text: &str, font: FontId, color: Color32, max_width: f32) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(text.to_string(), egui::TextFormat::simple(font, color));
    job.wrap = TextWrapping {
        max_width,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    ui.fonts(|f| f.layout_job(job))
}

pub fn font(size: f32) -> FontId {
    FontId::proportional(size)
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name("semibold".into()))
}

pub fn light(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name("light".into()))
}

pub fn serif(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name("serif".into()))
}

/// Space kept free at the right of a header for the hover gear.
pub const HEADER_GEAR_GAP: f32 = 30.0;

/// "• Label" header shared by the newer widgets; returns the body rect.
pub fn dot_header(p: &egui::Painter, r: egui::Rect, label: &str, right: &str, look: &crate::glass::Look) -> egui::Rect {
    use egui::{Align2, pos2};
    p.circle_filled(pos2(r.min.x + 3.0, r.min.y + 8.0), 2.5, look.ink.dim);
    p.text(pos2(r.min.x + 12.0, r.min.y + 8.0), Align2::LEFT_CENTER, label, semibold(12.0), look.ink.ink);
    if !right.is_empty() {
        // Leave room at the far right for the hover settings button.
        p.text(pos2(r.max.x - HEADER_GEAR_GAP, r.min.y + 8.0), Align2::RIGHT_CENTER, right, semibold(10.5), look.ink.dim);
    }
    egui::Rect::from_min_max(pos2(r.min.x, r.min.y + 26.0), r.max)
}

/// Spaced small caps, like Lumen's section labels.
pub fn caps(s: &str) -> String {
    s.to_uppercase().chars().flat_map(|c| [c, '\u{200A}']).collect()
}
