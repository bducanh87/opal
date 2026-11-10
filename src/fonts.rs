//! Windows system fonts (Segoe UI family, Consolas, Georgia, Segoe icons)
//! loaded at runtime, plus the procedurally drawn app icon.

use egui::{Context, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

fn load(name: &str) -> Option<Vec<u8>> {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    std::fs::read(format!("{windir}\\Fonts\\{name}")).ok()
}

pub fn install(ctx: &Context) {
    let mut defs = FontDefinitions::default();
    let mut add = |key: &str, files: &[&str]| -> bool {
        for f in files {
            if let Some(bytes) = load(f) {
                defs.font_data.insert(key.to_string(), Arc::new(FontData::from_owned(bytes)));
                return true;
            }
        }
        false
    };
    let ui = add("segoe", &["segoeui.ttf"]);
    let semibold = add("segoe-sb", &["seguisb.ttf", "segoeuib.ttf"]);
    let light = add("segoe-light", &["segoeuil.ttf", "segoeuisl.ttf"]);
    let mono = add("consolas", &["consola.ttf", "cour.ttf"]);
    let serif = add("georgia", &["georgia.ttf", "times.ttf"]);
    let icons = add("segoe-icons", &["SegoeIcons.ttf", "segmdl2.ttf"]);
    let emoji = add("segoe-emoji", &["seguiemj.ttf"]);

    let base: Vec<String> = defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let with = |first: Option<&str>| -> Vec<String> {
        let mut v = Vec::new();
        if let Some(f) = first {
            v.push(f.to_string());
        }
        if ui {
            v.push("segoe".into());
        }
        v.extend(base.iter().cloned());
        if emoji {
            v.push("segoe-emoji".into());
        }
        if icons {
            v.push("segoe-icons".into());
        }
        v
    };
    defs.families.insert(FontFamily::Proportional, with(None));
    defs.families.insert(FontFamily::Name("semibold".into()), with(semibold.then_some("segoe-sb")));
    defs.families.insert(FontFamily::Name("light".into()), with(light.then_some("segoe-light")));
    defs.families.insert(FontFamily::Name("serif".into()), with(serif.then_some("georgia")));
    let mut mono_list = Vec::new();
    if mono {
        mono_list.push("consolas".to_string());
    }
    mono_list.extend(defs.families.get(&FontFamily::Monospace).cloned().unwrap_or_default());
    defs.families.insert(FontFamily::Monospace, mono_list);
    let mut icon_list = Vec::new();
    if icons {
        icon_list.push("segoe-icons".to_string());
    }
    icon_list.extend(base.iter().cloned());
    defs.families.insert(FontFamily::Name("icons".into()), icon_list);
    ctx.set_fonts(defs);
}

/// n×n RGBA of the Opal icon (an opal cabochon on a dark glass tile,
/// `assets/icon.png`, drawn by opal-tauri/icons/make-icon.ps1).
pub fn icon_rgba(n: usize) -> Vec<u8> {
    let png = include_bytes!("../assets/icon.png");
    match image::load_from_memory(png) {
        Ok(img) => img.resize_exact(n as u32, n as u32, image::imageops::FilterType::Lanczos3).to_rgba8().into_raw(),
        Err(_) => vec![0u8; n * n * 4],
    }
}

pub fn app_icon() -> Arc<egui::IconData> {
    static ICON: std::sync::OnceLock<Arc<egui::IconData>> = std::sync::OnceLock::new();
    ICON.get_or_init(|| Arc::new(egui::IconData { rgba: icon_rgba(64), width: 64, height: 64 }))
        .clone()
}
