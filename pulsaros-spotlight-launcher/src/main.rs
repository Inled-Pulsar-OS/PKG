mod apps;
mod calculator;
mod clipboard;
mod config;
mod search;
mod ui;
mod utils;

use crate::clipboard::ClipboardManager;
use crate::config::SpotlightConfig;
use crate::search::SearchBackend;
use crate::ui::window::SpotlightWindow;
use crate::utils::{get_icon_dir, get_local_icon_dir};
use gtk4::gdk;
use gtk4::gio;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

const APP_ID: &str = "es.inled.pulsaros-spotlight";

fn load_css() {
    let css_paths = &[
        PathBuf::from("/usr/share/pulsaros-spotlight/index.css"),
        get_local_icon_dir().parent().unwrap().join("index.css"),
    ];

    let mut css_file = None;
    for path in css_paths {
        if path.exists() {
            css_file = Some(path.clone());
            break;
        }
    }

    let css_file = match css_file {
        Some(f) => f,
        None => {
            eprintln!("Warning: spotlight index.css not found — results area may be invisible");
            return;
        }
    };

    let provider = gtk4::CssProvider::new();
    provider.load_from_path(css_file);

    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn main() -> gtk4::glib::ExitCode {
    // Initialize libadwaita
    adw::init().expect("Failed to initialize Libadwaita");

    let app = gtk4::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::FLAGS_NONE)
        .build();

    let window_cell: Rc<RefCell<Option<Rc<SpotlightWindow>>>> = Rc::new(RefCell::new(None));
    let hold_guard_cell: Rc<RefCell<Option<gio::ApplicationHoldGuard>>> = Rc::new(RefCell::new(None));

    let window_cell_c = window_cell.clone();
    let hold_guard_cell_c = hold_guard_cell.clone();
    app.connect_activate(move |app| {
        let is_hidden = std::env::args().any(|arg| arg == "--hidden");

        let mut win_borrow = window_cell_c.borrow_mut();
        if win_borrow.is_none() {
            // Apply CSS
            load_css();

            let config = SpotlightConfig::load();
            let clipboard_mgr = ClipboardManager::new(config.clone());
            let backend = SearchBackend::new(Some(clipboard_mgr.clone()));

            // Clipboard change listener
            let cb_c = clipboard_mgr.clone();
            if let Some(display) = gdk::Display::default() {
                let clipboard = display.clipboard();
                clipboard.connect_changed(move |cb| {
                    let cb_c2 = cb_c.clone();
                    cb.read_text_async(None::<&gio::Cancellable>, move |result| {
                        if let Ok(Some(text)) = result {
                            cb_c2.record_clip(&text);
                        }
                    });
                });
            }

            let spotlight_win = SpotlightWindow::new(
                app,
                config,
                backend,
                Some(clipboard_mgr),
            );

            *win_borrow = Some(spotlight_win);
        }

        if !is_hidden {
            if let Some(win) = win_borrow.as_ref() {
                win.present_with_focus();
            }
        } else {
            // Under GTK/gio, to prevent app from exiting immediately when hidden is passed,
            // we hold the application.
            let guard = app.hold();
            *hold_guard_cell_c.borrow_mut() = Some(guard);
        }
    });

    let clean_args: Vec<String> = std::env::args()
        .filter(|arg| arg != "--hidden")
        .collect();

    app.run_with_args(&clean_args)
}
