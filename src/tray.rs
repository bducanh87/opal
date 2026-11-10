//! System tray icon and menu (quit from the tray).

use std::cell::RefCell;
use std::sync::mpsc::{Receiver, channel};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub enum Action {
    Settings,
    Toggle(&'static str),
    ConfigFolder,
    Quit,
}

enum Event {
    Menu(MenuEvent),
    Tray(TrayIconEvent),
}

pub struct Tray {
    _icon: TrayIcon,
    events: Receiver<Event>,
    settings: MenuItem,
    folder: MenuItem,
    quit: MenuItem,
    toggles: Vec<(&'static str, CheckMenuItem)>,
    last: RefCell<Vec<bool>>,
}

impl Tray {
    pub fn new(ctx: &egui::Context) -> Option<Self> {
        let menu = Menu::new();
        let settings = MenuItem::new("Settings", true, None);
        let folder = MenuItem::new("Open config folder", true, None);
        let quit = MenuItem::new("Quit Opal", true, None);
        let toggles: Vec<(&'static str, CheckMenuItem)> = crate::app::WIDGETS
            .iter()
            .map(|(id, name)| (*id, CheckMenuItem::new(*name, true, true, None)))
            .collect();
        menu.append(&settings).ok()?;
        menu.append(&PredefinedMenuItem::separator()).ok()?;
        for (_, item) in &toggles {
            menu.append(item).ok()?;
        }
        menu.append(&PredefinedMenuItem::separator()).ok()?;
        menu.append(&folder).ok()?;
        menu.append(&quit).ok()?;

        let rgba = crate::fonts::icon_rgba(32);
        let icon = tray_icon::Icon::from_rgba(rgba, 32, 32).ok()?;
        let tray = TrayIconBuilder::new()
            .with_tooltip("Opal")
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()
            .map_err(|e| eprintln!("[opal] tray: {e}"))
            .ok()?;

        let (tx, events) = channel();
        let tx2 = tx.clone();
        let c1 = ctx.clone();
        let c2 = ctx.clone();
        MenuEvent::set_event_handler(Some(move |e| {
            let _ = tx.send(Event::Menu(e));
            c1.request_repaint();
        }));
        TrayIconEvent::set_event_handler(Some(move |e| {
            let _ = tx2.send(Event::Tray(e));
            c2.request_repaint();
        }));
        Some(Tray { _icon: tray, events, settings, folder, quit, toggles, last: RefCell::new(Vec::new()) })
    }

    pub fn poll(&self) -> Vec<Action> {
        let mut out = Vec::new();
        while let Ok(event) = self.events.try_recv() {
            match event {
                Event::Menu(e) => {
                    if e.id == *self.settings.id() {
                        out.push(Action::Settings);
                    } else if e.id == *self.folder.id() {
                        out.push(Action::ConfigFolder);
                    } else if e.id == *self.quit.id() {
                        out.push(Action::Quit);
                    } else if let Some((id, _)) = self.toggles.iter().find(|(_, item)| e.id == *item.id()) {
                        out.push(Action::Toggle(id));
                        self.last.borrow_mut().clear();
                    }
                }
                Event::Tray(TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }) => out.push(Action::Settings),
                Event::Tray(_) => {}
            }
        }
        out
    }

    /// Keep the check marks in line with which widgets are open.
    pub fn sync(&self, open: &[(&str, bool)]) {
        let now: Vec<bool> = open.iter().map(|o| o.1).collect();
        if *self.last.borrow() == now {
            return;
        }
        for ((_, item), (_, is_open)) in self.toggles.iter().zip(open) {
            item.set_checked(*is_open);
        }
        *self.last.borrow_mut() = now;
    }
}
