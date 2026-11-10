#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Opal — glass widgets that live on the Windows wallpaper.

mod app;
mod config;
mod fonts;
mod glass;
mod icons;
mod ics;
mod lunar;
mod media;
mod programs;
mod sysmon;
mod theme;
mod tray;
mod weather_api;
mod widgets;
mod win;

pub const HOST_TITLE: &str = "Opal Host";

fn main() -> eframe::Result<()> {
    // One edition at a time: the egui and Tauri builds share a lock.
    match win::claim_instance("Local\\OpalEgui") {
        win::Instance::First => {}
        win::Instance::SameRunning => {
            eprintln!("[opal] already running");
            return Ok(());
        }
        win::Instance::OtherRunning => {
            win::message_box(
                "Opal",
                "Opal bản Tauri đang chạy.\n\nMỗi lúc chỉ chạy một bản: thoát bản Tauri (chuột phải icon Opal ở khay hệ thống → Quit Opal) rồi mở lại bản này.",
            );
            return Ok(());
        }
    }
    // The host is a 1×1 transparent tool window; every widget is its own
    // viewport rendered from the host's update loop.
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(HOST_TITLE)
            .with_decorations(false)
            .with_transparent(true)
            .with_taskbar(false)
            .with_resizable(false)
            .with_mouse_passthrough(true)
            .with_position(egui::pos2(0.0, 0.0))
            .with_inner_size(egui::vec2(1.0, 1.0))
            .with_icon(fonts::app_icon()),
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native("Opal", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
