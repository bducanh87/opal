//! Shell icons extracted on a COM worker thread and uploaded as textures.

use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, Sender, channel};

pub struct Icons {
    textures: HashMap<String, Option<TextureHandle>>,
    requested: HashSet<String>,
    to_worker: Sender<(String, u64)>,
    from_worker: Receiver<(String, u64, Option<ColorImage>)>,
    /// Bumped by `forget`; results from an older request are ignored.
    generation: HashMap<String, u64>,
}

impl Icons {
    pub fn new(ctx: &Context) -> Self {
        let (to_worker, jobs) = channel::<(String, u64)>();
        let (done, from_worker) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            crate::win::com_init();
            while let Ok((path, gen_)) = jobs.recv() {
                let image = crate::win::extract_icon(&path, 96).map(|(w, h, mut rgba)| {
                    crate::theme::avoid_key(&mut rgba);
                    ColorImage::from_rgba_unmultiplied([w, h], &rgba)
                });
                if done.send((path, gen_, image)).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Icons { textures: HashMap::new(), requested: HashSet::new(), to_worker, from_worker, generation: HashMap::new() }
    }

    /// Upload finished icons. Call once per frame.
    pub fn pump(&mut self, ctx: &Context) {
        while let Ok((path, gen_, image)) = self.from_worker.try_recv() {
            if self.generation.get(&path).copied().unwrap_or(0) != gen_ {
                continue;
            }
            let tex = image.map(|img| ctx.load_texture(format!("icon:{path}"), img, TextureOptions::LINEAR));
            self.textures.insert(path, tex);
        }
    }

    /// The texture once ready; queues extraction on first request.
    pub fn get(&mut self, path: &str) -> Option<TextureHandle> {
        if let Some(t) = self.textures.get(path) {
            return t.clone();
        }
        if self.requested.insert(path.to_string()) {
            let gen_ = self.generation.get(path).copied().unwrap_or(0);
            let _ = self.to_worker.send((path.to_string(), gen_));
        }
        None
    }

    pub fn forget(&mut self, path: &str) {
        self.textures.remove(path);
        self.requested.remove(path);
        *self.generation.entry(path.to_string()).or_insert(0) += 1;
    }
}
