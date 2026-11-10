//! Âm lịch widget: today's solar date beside the lunar date, with the
//! can-chi of the day, month and year, the solar term and auspicious hours.

use chrono::{Datelike, Local};
use egui::{Align2, Rect, Stroke, Ui, pos2};

use super::{caps, dot_header, font, light, semibold, serif};
use crate::app::App;
use crate::glass::Look;
use crate::lunar;
use crate::theme::rgb;

const THU: [&str; 7] = ["Thứ Hai", "Thứ Ba", "Thứ Tư", "Thứ Năm", "Thứ Sáu", "Thứ Bảy", "Chủ Nhật"];

pub fn show(app: &mut App, ui: &mut Ui, look: &Look) {
    match app.state.settings.design("lunar").as_str() {
        "toloch" => show_to_lich(app, ui, look),
        "minimal" => show_minimal(app, ui, look),
        _ => show_lumen(app, ui, look),
    }
}

fn today_lunar() -> (chrono::NaiveDate, lunar::LunarDate) {
    let today = Local::now().date_naive();
    let l = lunar::solar_to_lunar(today.day() as i64, today.month() as i64, today.year() as i64);
    (today, l)
}

/// Tờ lịch: a Vietnamese tear-off calendar sheet. Red header with the
/// month, a huge red solar day, the weekday, then the lunar date block.
fn show_to_lich(app: &mut App, ui: &mut Ui, _look: &Look) {
    let r = ui.max_rect();
    let p = ui.painter().clone();
    let (today, l) = today_lunar();
    let red = rgb(0xc8282e);
    let ink = rgb(0x2b2622);
    let sunday = today.weekday() == chrono::Weekday::Sun;
    let day_color = if sunday { red } else { rgb(0x1f3f8a) };
    // Red header band (bleeds to the plate edge).
    let band = Rect::from_min_max(r.min - egui::vec2(18.0, 16.0), egui::pos2(r.max.x + 18.0, r.min.y + 26.0));
    p.rect_filled(band, egui::CornerRadius { nw: 33, ne: 33, sw: 0, se: 0 }, red);
    p.text(egui::pos2(r.center().x, r.min.y + 9.0), Align2::CENTER_CENTER, caps(&format!("Tháng {} · {}", today.month(), today.year())), semibold(12.0), egui::Color32::WHITE);
    // Binding holes.
    for dx in [-0.3_f32, 0.3] {
        p.circle_filled(egui::pos2(r.center().x + r.width() * dx, r.min.y - 6.0), 3.5, rgb(0x7a1418));
    }
    let lunar_h = if app.extras.lunar_show_hours { 72.0 } else { 56.0 };
    let top = r.min.y + 30.0;
    let big = serif(((r.max.y - lunar_h - top) * 0.78).clamp(30.0, 120.0));
    p.text(egui::pos2(r.center().x, top + big.size * 0.5), Align2::CENTER_CENTER, today.day().to_string(), big.clone(), day_color);
    let wd = THU[today.weekday().num_days_from_monday() as usize];
    p.text(egui::pos2(r.center().x, top + big.size * 1.14), Align2::CENTER_CENTER, caps(wd), semibold(12.0), day_color);
    // Lunar block.
    let y0 = r.max.y - lunar_h;
    let dash = |y: f32| {
        let mut x = r.min.x;
        while x < r.max.x {
            p.line_segment([egui::pos2(x, y), egui::pos2((x + 5.0).min(r.max.x), y)], Stroke::new(1.0_f32, ink.gamma_multiply(0.35)));
            x += 9.0;
        }
    };
    dash(y0);
    p.text(egui::pos2(r.min.x, y0 + 8.0), Align2::LEFT_TOP, l.day.to_string(), serif(30.0), red);
    let month = format!("Tháng {}{}", lunar::month_name(l.month), if l.leap { " nhuận" } else { "" });
    p.text(egui::pos2(r.min.x + 46.0, y0 + 10.0), Align2::LEFT_TOP, month, semibold(12.0), ink);
    p.text(egui::pos2(r.min.x + 46.0, y0 + 27.0), Align2::LEFT_TOP, format!("Năm {}", lunar::year_can_chi(l.year)), font(11.5), ink.gamma_multiply(0.75));
    p.text(egui::pos2(r.max.x, y0 + 10.0), Align2::RIGHT_TOP, format!("Ngày {}", lunar::day_can_chi(l.jd)), font(11.5), ink.gamma_multiply(0.75));
    if app.extras.lunar_show_term {
        p.text(egui::pos2(r.max.x, y0 + 27.0), Align2::RIGHT_TOP, lunar::tiet_khi(l.jd), font(11.5), ink.gamma_multiply(0.75));
    }
    if app.extras.lunar_show_hours {
        let hours = format!("Giờ tốt: {}", lunar::gio_hoang_dao(l.jd).join(", "));
        let g = super::elided(ui, &hours, font(10.5), ink.gamma_multiply(0.6), r.width());
        p.galley(egui::pos2(r.min.x, y0 + 50.0), g, ink.gamma_multiply(0.6));
    }
}

/// Minimal: the lunar date in large serif type with one line of context.
fn show_minimal(app: &mut App, ui: &mut Ui, look: &Look) {
    let r = ui.max_rect();
    let p = ui.painter().clone();
    let (today, l) = today_lunar();
    let gold = rgb(0xf0b44a);
    let big = serif((r.height() * 0.48).clamp(32.0, 120.0));
    let c = r.center();
    p.text(egui::pos2(c.x, r.min.y + 4.0), Align2::CENTER_TOP, caps("Âm lịch"), semibold(10.0), gold);
    p.text(egui::pos2(c.x, c.y - big.size * 0.08), Align2::CENTER_CENTER, l.day.to_string(), big.clone(), gold);
    let month = format!("tháng {}{} · năm {}", lunar::month_name(l.month), if l.leap { " nhuận" } else { "" }, lunar::year_can_chi(l.year));
    p.text(egui::pos2(c.x, c.y + big.size * 0.5), Align2::CENTER_CENTER, month, semibold(12.5), look.ink.ink);
    let solar = format!("{}, {}/{}/{}", THU[today.weekday().num_days_from_monday() as usize], today.day(), today.month(), today.year());
    p.text(egui::pos2(c.x, c.y + big.size * 0.5 + 20.0), Align2::CENTER_CENTER, solar, font(11.5), look.ink.dim);
    let _ = app;
}

fn show_lumen(app: &mut App, ui: &mut Ui, look: &Look) {
    let r = ui.max_rect();
    let p = ui.painter().clone();
    let today = Local::now().date_naive();
    let l = lunar::solar_to_lunar(today.day() as i64, today.month() as i64, today.year() as i64);
    let year_cc = lunar::year_can_chi(l.year);
    let body = dot_header(&p, r, "Âm lịch", &caps(&format!("Năm {year_cc}")), look);

    let accent = rgb(0xf0b44a);
    let half = body.width() / 2.0;
    let left = Rect::from_min_max(body.min, pos2(body.min.x + half - 8.0, body.max.y));
    let right = Rect::from_min_max(pos2(body.min.x + half + 8.0, body.min.y), body.max);
    let big = (body.height() * 0.42).clamp(30.0, 84.0);

    // Solar side.
    let wd = THU[today.weekday().num_days_from_monday() as usize];
    p.text(left.left_top(), Align2::LEFT_TOP, caps(wd), semibold(10.0), look.ink.dim);
    p.text(pos2(left.min.x - 2.0, left.min.y + 14.0), Align2::LEFT_TOP, today.day().to_string(), light(big), look.ink.ink);
    p.text(pos2(left.min.x, left.min.y + 18.0 + big * 1.05), Align2::LEFT_TOP, format!("Tháng {} · {}", today.month(), today.year()), font(12.0), look.ink.dim);

    // Divider.
    p.line_segment([pos2(body.center().x, body.min.y + 4.0), pos2(body.center().x, body.min.y + 18.0 + big * 1.2)], Stroke::new(1.0_f32, look.ink.hair));

    // Lunar side.
    p.text(right.left_top(), Align2::LEFT_TOP, caps("Âm lịch"), semibold(10.0), accent);
    p.text(pos2(right.min.x - 2.0, right.min.y + 14.0), Align2::LEFT_TOP, l.day.to_string(), serif(big * 0.95), accent);
    let month = format!("Tháng {}{}", lunar::month_name(l.month), if l.leap { " nhuận" } else { "" });
    p.text(pos2(right.min.x, right.min.y + 18.0 + big * 1.05), Align2::LEFT_TOP, month, font(12.0), look.ink.dim);

    // Details.
    let mut y = body.min.y + 40.0 + big * 1.05;
    let mut rows = vec![("Ngày", lunar::day_can_chi(l.jd)), ("Tháng", lunar::month_can_chi(&l))];
    if app.extras.lunar_show_term {
        rows.push(("Tiết khí", lunar::tiet_khi(l.jd).to_string()));
    }
    if app.extras.lunar_show_hours {
        rows.push(("Giờ hoàng đạo", lunar::gio_hoang_dao(l.jd).join(", ")));
    }
    for (k, v) in rows {
        if y + 16.0 > body.max.y {
            break;
        }
        p.text(pos2(body.min.x, y), Align2::LEFT_TOP, k, font(11.5), look.ink.dim);
        let g = super::elided(ui, &v, semibold(11.5), look.ink.ink, body.width() - 100.0);
        p.galley(pos2(body.max.x - g.size().x, y), g, look.ink.ink);
        y += 20.0;
    }
}
