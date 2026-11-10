//! System monitor: CPU (with a 60-second sparkline), memory, system drive
//! and network throughput, sampled once a second while the widget is open.

use egui::{Align2, Color32, CornerRadius, Rect, Shape, Stroke, Ui, pos2, vec2};

use super::{dot_header, font, semibold};
use crate::app::App;
use crate::glass::Look;
use crate::theme::{rgb, white};
pub use crate::sysmon::SystemState;
use crate::sysmon::{gb, rate};

fn meter(p: &egui::Painter, rect: Rect, frac: f32, color: Color32) {
    let n = ((rect.width() / 11.0).floor() as usize).clamp(12, 40);
    let gap = 3.0;
    let dw = (rect.width() - gap * (n - 1) as f32) / n as f32;
    let lit = (frac.clamp(0.0, 1.0) * n as f32).round() as usize;
    for k in 0..n {
        let x = rect.min.x + k as f32 * (dw + gap);
        p.rect_filled(Rect::from_min_size(pos2(x, rect.min.y), vec2(dw, rect.height())), CornerRadius::same(2), if k < lit { color } else { white(22) });
    }
}

pub fn show(app: &mut App, ui: &mut Ui, look: &Look) {
    match app.state.settings.design("system").as_str() {
        "rings" => show_rings(app, ui, look),
        "terminal" => show_terminal(app, ui),
        _ => show_lumen(app, ui, look),
    }
}

fn uptime_text() -> String {
    let up = sysinfo::System::uptime();
    format!("up {}h {:02}m", up / 3600, (up % 3600) / 60)
}

fn fractions(s: &SystemState) -> [f32; 3] {
    let frac = |used: u64, total: u64| if total > 0 { used as f32 / total as f32 } else { 0.0 };
    [s.cpu / 100.0, frac(s.mem_used, s.mem_total), frac(s.disk_used, s.disk_total)]
}

/// Rings: three gauges for CPU, memory and the system drive.
fn show_rings(app: &mut App, ui: &mut Ui, look: &Look) {
    let s = &app.sys;
    let r = ui.max_rect();
    let p = ui.painter().clone();
    let body = dot_header(&p, r, "System", &uptime_text(), look);
    let net_h = if app.extras.system_show_network { 24.0 } else { 0.0 };
    let area = Rect::from_min_max(body.min, pos2(body.max.x, body.max.y - net_h));
    let f = fractions(s);
    let items = [
        ("CPU", f[0], rgb(0xa3e635), format!("{:.0}%", s.cpu)),
        ("Memory", f[1], rgb(0x34d399), format!("{:.0} GB", gb(s.mem_used))),
        ("Disk", f[2], rgb(0x60c3f6), format!("{:.0}%", f[2] * 100.0)),
    ];
    let cw = area.width() / 3.0;
    let rad = (cw * 0.36).min(area.height() * 0.34).max(16.0);
    for (i, (label, frac, color, value)) in items.iter().enumerate() {
        let c = pos2(area.min.x + cw * (i as f32 + 0.5), area.min.y + area.height() * 0.44);
        let fr = frac.clamp(0.0, 1.0);
        p.circle_stroke(c, rad, Stroke::new(rad * 0.16, white(22)));
        if fr > 0.005 {
            let steps = (fr * 90.0).ceil().max(2.0) as usize;
            let pts: Vec<_> = (0..=steps)
                .map(|k| {
                    let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * fr * k as f32 / steps as f32;
                    c + vec2(a.cos(), a.sin()) * rad
                })
                .collect();
            p.add(Shape::line(pts, Stroke::new(rad * 0.16, *color)));
        }
        p.text(c, Align2::CENTER_CENTER, value, semibold((rad * 0.36).clamp(10.0, 18.0)), look.ink.ink);
        p.text(pos2(c.x, c.y + rad + 14.0), Align2::CENTER_CENTER, *label, font(11.5), look.ink.dim);
    }
    if net_h > 0.0 {
        p.text(pos2(body.center().x, body.max.y - 8.0), Align2::CENTER_CENTER, format!("↓ {}    ↑ {}", rate(s.down), rate(s.up)), font(11.5), look.ink.dim);
    }
}

/// Terminal: a green monospace readout.
fn show_terminal(app: &mut App, ui: &mut Ui) {
    let s = &app.sys;
    let r = ui.max_rect();
    let p = ui.painter().clone();
    let green = rgb(0x4dff88);
    let dim = rgb(0x2a9c55);
    let size = (r.height() / 9.5).clamp(10.0, 15.0);
    let mono = egui::FontId::monospace(size);
    let f = fractions(s);
    let bar = |frac: f32| {
        let n = 14usize;
        let lit = (frac.clamp(0.0, 1.0) * n as f32).round() as usize;
        format!("[{}{}]", "|".repeat(lit), " ".repeat(n - lit))
    };
    let mut lines = vec![
        ("opal@desktop:~$ top".to_string(), dim),
        (format!("cpu  {} {:>3.0}%", bar(f[0]), s.cpu), green),
        (format!("mem  {} {:>3.0}%  {:.1}/{:.0}G", bar(f[1]), f[1] * 100.0, gb(s.mem_used), gb(s.mem_total)), green),
        (format!("disk {} {:>3.0}%  {}", bar(f[2]), f[2] * 100.0, s.disk_name), green),
    ];
    if app.extras.system_show_network {
        lines.push((format!("net  ↓ {}  ↑ {}", rate(s.down), rate(s.up)), green));
    }
    lines.push((format!("     {}", uptime_text()), dim));
    let mut y = r.min.y;
    for (text, color) in lines {
        p.text(pos2(r.min.x, y), Align2::LEFT_TOP, text, mono.clone(), color);
        y += size * 1.55;
    }
    if chrono::Local::now().timestamp() % 2 == 0 {
        let w = p.layout_no_wrap("opal@desktop:~$ ".into(), mono.clone(), dim).size().x;
        p.text(pos2(r.min.x, y), Align2::LEFT_TOP, "opal@desktop:~$", mono.clone(), dim);
        p.rect_filled(Rect::from_min_size(pos2(r.min.x + w, y + 1.0), vec2(size * 0.55, size)), CornerRadius::ZERO, green);
    }
}

fn show_lumen(app: &mut App, ui: &mut Ui, look: &Look) {
    let s = &app.sys;
    let r = ui.max_rect();
    let p = ui.painter().clone();
    let up_secs = sysinfo::System::uptime();
    let uptime = format!("up {}h {:02}m", up_secs / 3600, (up_secs % 3600) / 60);
    let body = dot_header(&p, r, "System", &uptime, look);

    // CPU headline + sparkline.
    let cpu_h = (body.height() * 0.34).clamp(44.0, 90.0);
    let cpu_area = Rect::from_min_size(body.min, vec2(body.width(), cpu_h));
    p.text(cpu_area.left_top(), Align2::LEFT_TOP, format!("{:.0}%", s.cpu), super::light((cpu_h * 0.6).clamp(24.0, 52.0)), look.ink.ink);
    p.text(pos2(cpu_area.min.x, cpu_area.max.y - 4.0), Align2::LEFT_BOTTOM, "CPU", semibold(10.5), look.ink.dim);
    let spark = Rect::from_min_max(pos2(cpu_area.min.x + cpu_area.width() * 0.38, cpu_area.min.y + 6.0), cpu_area.max - vec2(0.0, 4.0));
    if s.cpu_history.len() > 1 && app.extras.system_show_graph {
        let n = 60.0;
        let pts: Vec<_> = s
            .cpu_history
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let x = spark.max.x - (s.cpu_history.len() - 1 - i) as f32 / (n - 1.0) * spark.width();
                pos2(x, spark.max.y - (v / 100.0).clamp(0.0, 1.0) * spark.height())
            })
            .collect();
        let mut fill = pts.clone();
        fill.push(pos2(pts.last().unwrap().x, spark.max.y));
        fill.push(pos2(pts[0].x, spark.max.y));
        let mut mesh = egui::epaint::Mesh::default();
        for (i, w) in pts.windows(2).enumerate() {
            let base = mesh.vertices.len() as u32;
            let col_top = crate::theme::css(0xa3, 0xe6, 0x35, 70);
            let col_bot = crate::theme::css(0xa3, 0xe6, 0x35, 0);
            for (pos, col) in [(w[0], col_top), (w[1], col_top), (pos2(w[1].x, spark.max.y), col_bot), (pos2(w[0].x, spark.max.y), col_bot)] {
                mesh.vertices.push(egui::epaint::Vertex { pos, uv: egui::epaint::WHITE_UV, color: col });
            }
            mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            let _ = i;
        }
        p.add(Shape::mesh(mesh));
        p.add(Shape::line(pts, Stroke::new(1.6_f32, rgb(0xa3e635))));
    }

    // Meters.
    let rows = [
        (
            "Memory",
            format!("{:.1} / {:.0} GB", gb(s.mem_used), gb(s.mem_total)),
            if s.mem_total > 0 { s.mem_used as f32 / s.mem_total as f32 } else { 0.0 },
            rgb(0x34d399),
        ),
        (
            "Disk",
            format!("{} {:.0} / {:.0} GB", s.disk_name, gb(s.disk_used), gb(s.disk_total)),
            if s.disk_total > 0 { s.disk_used as f32 / s.disk_total as f32 } else { 0.0 },
            rgb(0x60c3f6),
        ),
    ];
    let mut y = cpu_area.max.y + 10.0;
    for (name, value, frac, color) in rows {
        p.text(pos2(body.min.x, y), Align2::LEFT_TOP, name, font(12.0), look.ink.dim);
        p.text(pos2(body.max.x, y), Align2::RIGHT_TOP, value, semibold(12.0), look.ink.ink);
        meter(&p, Rect::from_min_size(pos2(body.min.x, y + 20.0), vec2(body.width(), 6.0)), frac, color);
        y += 38.0;
    }
    if app.extras.system_show_network && y + 18.0 < body.max.y {
        p.text(pos2(body.min.x, y), Align2::LEFT_TOP, "Network", font(12.0), look.ink.dim);
        p.text(pos2(body.max.x, y), Align2::RIGHT_TOP, format!("↓ {}   ↑ {}", rate(s.down), rate(s.up)), semibold(12.0), look.ink.ink);
    }
}
