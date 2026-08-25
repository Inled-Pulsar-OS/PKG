use crate::config::SpotlightConfig;
use crate::search::{SearchBackend, SearchResult, DEFAULT_LIMIT};
use crate::clipboard::ClipboardManager;
use crate::ui::results::ResultView;
use crate::utils::{get_icon_dir, get_local_icon_dir, open_file};
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

const CATEGORIES: &[(&str, &str)] = &[
    ("all", "All"),
    ("applications", "Apps"),
    ("documents", "Docs"),
    ("images", "Images"),
    ("audio", "Music"),
    ("video", "Video"),
    ("clipboard", "Clip"),
    ("web", "Web"),
];

const DEBOUNCE_MS: u64 = 150;

pub struct SpotlightWindow {
    window: gtk4::ApplicationWindow,
    search_entry: gtk4::Entry,
    view_toggle: gtk4::Button,
    result_view: Rc<ResultView>,
    config: Rc<RefCell<SpotlightConfig>>,
    backend: SearchBackend,
    category: Rc<RefCell<String>>,
    current_dir: Rc<RefCell<Option<String>>>,
    debounce_id: Rc<RefCell<Option<glib::SourceId>>>,
    search_seq: Rc<RefCell<u64>>,
    category_buttons: HashMap<String, gtk4::ToggleButton>,

    indexing_revealer: gtk4::Revealer,
    indexing_label: gtk4::Label,
    indexing_pbar: gtk4::ProgressBar,
    indexing_timer_id: Rc<RefCell<Option<glib::SourceId>>>,

    progress_revealer: gtk4::Revealer,
    progress_spinner: gtk4::Spinner,
    progress_label: gtk4::Label,
}

impl SpotlightWindow {
    pub fn new(
        app: &gtk4::Application,
        config: SpotlightConfig,
        backend: SearchBackend,
        clipboard_mgr: Option<ClipboardManager>,
    ) -> Rc<Self> {
        let config_rc = Rc::new(RefCell::new(config));

        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("Spotlight")
            .default_width(680)
            .default_height(520)
            .resizable(false)
            .decorated(false)
            .build();

        window.add_css_class("spotlight-window");

        let style_manager = adw::StyleManager::default();
        let window_c = window.clone();
        style_manager.connect_notify_local(Some("dark"), move |sm, _| {
            let dark = sm.is_dark();
            if dark {
                window_c.remove_css_class("light");
                window_c.add_css_class("dark");
            } else {
                window_c.remove_css_class("dark");
                window_c.add_css_class("light");
            }
        });

        // Initialize theme class
        if style_manager.is_dark() {
            window.add_css_class("dark");
        } else {
            window.add_css_class("light");
        }

        let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        main_box.add_css_class("spotlight-main");
        window.set_child(Some(&main_box));

        // -- Search Header --
        let search_container = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        search_container.add_css_class("search-header");

        let search_icon = {
            let mut img = None;
            for base in &[get_local_icon_dir(), get_icon_dir()] {
                let f = base.join("spotlight-symbolic.svg");
                if f.exists() {
                    img = Some(gtk4::Image::from_file(f));
                    break;
                }
            }
            img.unwrap_or_else(|| gtk4::Image::from_icon_name("system-search-symbolic"))
        };
        search_icon.add_css_class("search-icon");

        let search_entry = gtk4::Entry::builder()
            .placeholder_text("Search applications, files, or clipboard...")
            .hexpand(true)
            .build();
        search_entry.add_css_class("search-input");

        let view_toggle = gtk4::Button::new();
        view_toggle.add_css_class("view-toggle");
        let is_grid = config_rc.borrow().is_grid_view;
        view_toggle.set_icon_name(if is_grid { "view-list-symbolic" } else { "view-grid-symbolic" });

        search_container.append(&search_icon);
        search_container.append(&search_entry);
        search_container.append(&view_toggle);
        main_box.append(&search_container);

        // -- Category Bar --
        let category_bar = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        category_bar.add_css_class("category-bar");
        let mut category_buttons = HashMap::new();
        let current_category = Rc::new(RefCell::new("all".to_string()));

        for &(cat_id, cat_label) in CATEGORIES {
            let btn = gtk4::ToggleButton::builder()
                .label(cat_label)
                .active(cat_id == "all")
                .build();
            btn.add_css_class("category-btn");
            category_bar.append(&btn);
            category_buttons.insert(cat_id.to_string(), btn);
        }
        main_box.append(&category_bar);

        // -- Uninstall Progress Bar --
        let progress_revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideDown)
            .transition_duration(200)
            .reveal_child(false)
            .build();

        let progress_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
        progress_box.add_css_class("uninstall-progress");

        let progress_spinner = gtk4::Spinner::new();
        let progress_label = gtk4::Label::builder()
            .hexpand(true)
            .xalign(0.0)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .max_width_chars(65)
            .build();

        progress_box.append(&progress_spinner);
        progress_box.append(&progress_label);
        progress_revealer.set_child(Some(&progress_box));

        // -- Indexing Progress Bar --
        let indexing_revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideDown)
            .transition_duration(200)
            .reveal_child(false)
            .build();

        let indexing_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        indexing_box.add_css_class("indexing-status-bar");
        indexing_box.set_margin_start(16);
        indexing_box.set_margin_end(16);
        indexing_box.set_margin_top(4);
        indexing_box.set_margin_bottom(6);

        let indexing_spinner = gtk4::Spinner::new();
        indexing_spinner.start();

        let indexing_label = gtk4::Label::builder()
            .label("Indexing files in background...")
            .xalign(0.0)
            .hexpand(true)
            .build();
        indexing_label.add_css_class("dim-label");

        let indexing_pbar = gtk4::ProgressBar::builder()
            .valign(gtk4::Align::Center)
            .width_request(100)
            .height_request(4)
            .build();

        indexing_box.append(&indexing_spinner);
        indexing_box.append(&indexing_label);
        indexing_box.append(&indexing_pbar);
        indexing_revealer.set_child(Some(&indexing_box));

        // -- Result View Setup --
        let window_c = window.clone();
        let current_dir = Rc::new(RefCell::new(None));
        let current_dir_c = current_dir.clone();
        let search_entry_c = search_entry.clone();
        let clipboard_mgr_c = clipboard_mgr.clone();
        let window_cc = window.clone();

        let on_activate = move |res: SearchResult| {
            if res.url.starts_with("calc://") {
                let val = res.url.trim_start_matches("calc://");
                if let Some(display) = gdk::Display::default() {
                    display.clipboard().set_text(val);
                }
                window_cc.set_visible(false);
                return;
            }

            if res.url.starts_with("clipboard://") {
                if let Some(mgr) = &clipboard_mgr_c {
                    if let Some(idx_str) = res.url.split("://").nth(1) {
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            if let Some(text) = mgr.get_clip_by_index(idx) {
                                window_cc.set_visible(false);
                                mgr.paste_clip(&text);
                                return;
                            }
                        }
                    }
                }
                window_cc.set_visible(false);
                return;
            }

            let clean_path = res.url.trim_start_matches("file://");
            if std::path::Path::new(clean_path).is_dir() && (res.mime == "inode/directory" || res.mime == "folder") {
                *current_dir_c.borrow_mut() = Some(clean_path.to_string());
                search_entry_c.set_text("");
                search_entry_c.set_placeholder_text(Some(&format!("Browsing: {}", clean_path)));
                // set_text only emits 'changed' when the text differs; emit it
                // manually so browsing always refreshes even with an empty entry
                search_entry_c.emit_by_name::<()>("changed", &[]);
                return;
            }

            open_file(&res.url);
            window_cc.set_visible(false);
        };

        let progress_spinner_c = progress_spinner.clone();
        let progress_label_c = progress_label.clone();
        let progress_revealer_c = progress_revealer.clone();
        let on_un_start = move |_: String, app_name: String| {
            progress_label_c.set_label(&format!("Uninstalling {}...", app_name));
            progress_spinner_c.start();
            progress_revealer_c.set_reveal_child(true);
        };

        let progress_spinner_cc = progress_spinner.clone();
        let progress_label_cc = progress_label.clone();
        let progress_revealer_cc = progress_revealer.clone();
        let on_un_done = move |success: bool, message: String, app_name: String| {
            progress_spinner_cc.stop();
            if success {
                progress_label_cc.set_label(&format!("✓ {} uninstalled successfully", app_name));
                let pr_rev = progress_revealer_cc.clone();
                glib::timeout_add_local(Duration::from_millis(1800), move || {
                    pr_rev.set_reveal_child(false);
                    glib::ControlFlow::Break
                });
            } else {
                let mut clean_msg = message.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ");
                if clean_msg.to_lowercase().contains("dependencias") || clean_msg.to_lowercase().contains("dependencies") {
                    clean_msg = format!("Cannot remove {}: required by other system packages", app_name);
                } else if let Some(err_part) = clean_msg.split("error:").nth(1) {
                    clean_msg = err_part.trim().to_string();
                }
                progress_label_cc.set_label(&format!("✗ {}", clean_msg));
                progress_label_cc.set_tooltip_text(Some(&message));
                let pr_rev = progress_revealer_cc.clone();
                glib::timeout_add_local(Duration::from_millis(4000), move || {
                    pr_rev.set_reveal_child(false);
                    glib::ControlFlow::Break
                });
            }
        };

        let result_view = Rc::new(ResultView::new(
            on_activate,
            on_un_start,
            on_un_done,
        ));

        let scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .min_content_height(300)
            .max_content_height(480)
            .build();
        scroll.add_css_class("results-area");
        scroll.set_child(Some(result_view.widget()));
        main_box.append(&scroll);

        main_box.append(&progress_revealer);
        main_box.append(&indexing_revealer);

        result_view.set_popover_parent(&window);

        let spotlight = Rc::new(Self {
            window,
            search_entry,
            view_toggle,
            result_view,
            config: config_rc,
            backend,
            category: current_category,
            current_dir,
            debounce_id: Rc::new(RefCell::new(None)),
            search_seq: Rc::new(RefCell::new(0)),
            category_buttons,
            indexing_revealer,
            indexing_label,
            indexing_pbar,
            indexing_timer_id: Rc::new(RefCell::new(None)),
            progress_revealer,
            progress_spinner,
            progress_label,
        });

        spotlight.setup_ui_interactions();

        spotlight
    }

    pub fn present_with_focus(self: &Rc<Self>) {
        *self.current_dir.borrow_mut() = None;
        self.search_entry.set_placeholder_text(Some("Search applications, files, or clipboard..."));
        self.search_entry.set_text("");
        self.backend.reload_apps();
        self.do_search();

        self.window.present();
        self.search_entry.grab_focus();
        self.start_indexing_timer();
    }

    fn start_indexing_timer(self: &Rc<Self>) {
        if self.indexing_timer_id.borrow().is_none() && self.window.is_visible() {
            let self_c = self.clone();
            let id = glib::timeout_add_seconds_local(3, move || {
                if !self_c.window.is_visible() {
                    // Drop the handle and stop ticking; removing the source
                    // from within its own callback is not safe
                    *self_c.indexing_timer_id.borrow_mut() = None;
                    return glib::ControlFlow::Break;
                }
                self_c.check_indexing_status();
                glib::ControlFlow::Continue
            });
            *self.indexing_timer_id.borrow_mut() = Some(id);
        }
    }

    fn stop_indexing_timer(&self) {
        if let Some(id) = self.indexing_timer_id.borrow_mut().take() {
            id.remove();
        }
    }

    fn check_indexing_status(&self) {
        let (is_indexing, status, progress) = self.backend.get_indexing_status();
        if is_indexing {
            let percent = (progress * 100.0) as i32;
            let label_text = if percent > 0 {
                format!("Indexing files in background... ({}%)", percent)
            } else {
                "Indexing files in background...".to_string()
            };
            self.indexing_label.set_text(&label_text);
            self.indexing_pbar.set_fraction(progress.max(0.0).min(1.0));
            self.indexing_revealer.set_reveal_child(true);
        } else {
            self.indexing_revealer.set_reveal_child(false);
        }
    }

    fn setup_ui_interactions(self: &Rc<Self>) {
        let self_c = self.clone();
        self.search_entry.connect_changed(move |_| {
            self_c.on_search_changed();
        });

        // View toggle (list/grid)
        let self_c2 = self.clone();
        self.view_toggle.connect_clicked(move |_| {
            // Scope the mutable borrow: do_search() reads config again and
            // borrowing across it would panic (RefCell already mutably borrowed)
            let new_grid = {
                let mut conf = self_c2.config.borrow_mut();
                conf.is_grid_view = !conf.is_grid_view;
                conf.save();
                conf.is_grid_view
            };
            self_c2.view_toggle.set_icon_name(if new_grid { "view-list-symbolic" } else { "view-grid-symbolic" });
            self_c2.do_search();
        });

        // Category button clicks
        for (cat_id, btn) in &self.category_buttons {
            let self_c3 = self.clone();
            let cat_id_c = cat_id.clone();
            let btn_c = btn.clone();
            btn.connect_toggled(move |_| {
                if btn_c.is_active() {
                    *self_c3.category.borrow_mut() = cat_id_c.clone();
                    *self_c3.current_dir.borrow_mut() = None;
                    self_c3.search_entry.set_placeholder_text(Some("Search applications, files, or clipboard..."));

                    // Deactivate others
                    for (cid, b) in &self_c3.category_buttons {
                        if cid != &cat_id_c && b.is_active() {
                            b.set_active(false);
                        }
                    }
                    self_c3.do_search();
                }
            });
        }

        // Window focus out -> hide
        let self_c4 = self.clone();
        let focus_ctrl = gtk4::EventControllerFocus::new();
        focus_ctrl.connect_leave(move |_| {
            let s = self_c4.clone();
            glib::timeout_add_local(Duration::from_millis(100), move || {
                if s.window.is_visible() && !s.window.is_active() {
                    s.window.set_visible(false);
                    s.stop_indexing_timer();
                }
                glib::ControlFlow::Break
            });
        });
        self.window.add_controller(focus_ctrl);

        // Key Press Events
        let self_c5 = self.clone();
        let key_ctrl = gtk4::EventControllerKey::new();
        // Capture phase: see keys before the focused widget (search entry
        // cursor, FlowBox internal navigation) can consume them
        key_ctrl.set_propagation_phase(gtk4::PropagationPhase::Capture);
        key_ctrl.connect_key_pressed(move |_, keyval, _, state| {
            self_c5.on_key_pressed(keyval, state)
        });
        self.window.add_controller(key_ctrl);

        // Apps changed monitor reload
        let self_c6 = self.clone();
        self.backend.set_on_apps_updated(move || {
            let s = self_c6.clone();
            glib::idle_add_local(move || {
                if s.window.is_visible() {
                    s.do_search();
                }
                glib::ControlFlow::Break
            });
        });

        // Window close-request
        self.window.connect_close_request(move |win| {
            win.set_visible(false);
            glib::Propagation::Stop
        });
    }

    fn on_search_changed(self: &Rc<Self>) {
        if let Some(id) = self.debounce_id.borrow_mut().take() {
            id.remove();
        }

        let self_c = self.clone();
        let id = glib::timeout_add_local(Duration::from_millis(DEBOUNCE_MS), move || {
            // The source has fired: drop the stale handle so a later
            // on_search_changed never removes an already-destroyed source
            *self_c.debounce_id.borrow_mut() = None;
            self_c.do_search();
            glib::ControlFlow::Break
        });
        *self.debounce_id.borrow_mut() = Some(id);
    }

    fn do_search(&self) {
        let mut seq = self.search_seq.borrow_mut();
        *seq += 1;
        let current_seq = *seq;

        let query = self.search_entry.text().to_string();
        let category = self.category.borrow().clone();

        // If in documents and query empty and not browsing, start browsing $HOME
        if category == "documents" && query.trim().is_empty() && self.current_dir.borrow().is_none() {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/jaime"));
            *self.current_dir.borrow_mut() = Some(home.display().to_string());
        }

        // If browsing directory
        let browsing_dir = self.current_dir.borrow().clone();
        if let Some(dir_path) = browsing_dir.as_ref() {
            let as_grid = self.config.borrow().is_grid_view;
            let results = self.browse_directory(dir_path, query.trim());
            self.result_view.set_results(results, as_grid);
            return;
        }

        // 1. Instant results
        let instant_results = self.backend.search_instant(&query, &category, DEFAULT_LIMIT);
        let as_grid = self.config.borrow().is_grid_view;
        self.result_view.set_results(instant_results.clone(), as_grid);

        // 2. Async file search
        let is_empty = query.trim().is_empty();
        let needs_files = (!is_empty || ["documents", "images", "audio", "video"].contains(&category.as_str()))
            && !["apps", "applications", "clipboard", "web"].contains(&category.as_str());

        if needs_files {
            let result_view_c = self.result_view.clone();
            let is_grid = self.config.borrow().is_grid_view;
            let seq_c = self.search_seq.clone();
            let seq_val = current_seq;

            self.backend.search_async(&query, &category, DEFAULT_LIMIT, move |file_results| {
                if *seq_c.borrow() == seq_val {
                    let mut combined = instant_results.clone();
                    combined.extend(file_results);
                    result_view_c.set_results(combined, is_grid);
                }
            });
        }
    }

    fn browse_directory(&self, path_str: &str, filter: &str) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let p = Path::new(path_str);
        if !p.is_dir() {
            return results;
        }

        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/jaime"));
        // Parent navigation
        if p != home && p.parent().is_some() {
            results.push(SearchResult {
                url: format!("file://{}", p.parent().unwrap().display()),
                title: ".. (Parent directory)".to_string(),
                mime: "inode/directory".to_string(),
                snippet: p.parent().unwrap().display().to_string(),
                app: None,
            });
        }

        if let Ok(entries) = std::fs::read_dir(p) {
            let mut list: Vec<_> = entries.filter_map(Result::ok).collect();
            list.sort_by_key(|e| (e.file_type().map(|t| !t.is_dir()).unwrap_or(true), e.file_name().to_string_lossy().to_lowercase()));

            for entry in list {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') {
                    continue;
                }
                if !filter.is_empty() && !name.to_lowercase().contains(&filter.to_lowercase()) {
                    continue;
                }

                let is_dir = entry.file_type().map_or(false, |t| t.is_dir());
                let url = format!("file://{}", entry.path().display());
                let mime = if is_dir { "inode/directory".to_string() } else { "application/octet-stream".to_string() };

                results.push(SearchResult {
                    url,
                    title: name,
                    mime,
                    snippet: entry.path().display().to_string(),
                    app: None,
                });
            }
        }

        results
    }

    fn cycle_category(&self, step: i32) {
        let mut current_idx = 0;
        for (idx, &(cat_id, _)) in CATEGORIES.iter().enumerate() {
            if cat_id == *self.category.borrow() {
                current_idx = idx;
                break;
            }
        }

        let next_idx = (current_idx as i32 + step).rem_euclid(CATEGORIES.len() as i32) as usize;
        let next_cat = CATEGORIES[next_idx].0;
        if let Some(btn) = self.category_buttons.get(next_cat) {
            btn.set_active(true);
        }
    }

    fn on_key_pressed(&self, keyval: gdk::Key, state: gdk::ModifierType) -> glib::Propagation {
        if keyval == gdk::Key::Escape {
            if self.current_dir.borrow().is_some() {
                *self.current_dir.borrow_mut() = None;
                self.search_entry.set_text("");
                self.search_entry.set_placeholder_text(Some("Search applications, files, or clipboard..."));
                self.do_search();
                return glib::Propagation::Stop;
            }
            self.window.set_visible(false);
            return glib::Propagation::Stop;
        }

        if state.contains(gdk::ModifierType::CONTROL_MASK) && keyval == gdk::Key::q {
            self.window.application().unwrap().quit();
            return glib::Propagation::Stop;
        }

        // Tab navigation cycles categories
        if keyval == gdk::Key::Tab {
            if state.contains(gdk::ModifierType::SHIFT_MASK) {
                self.cycle_category(-1);
            } else {
                self.cycle_category(1);
            }
            return glib::Propagation::Stop;
        }
        if keyval == gdk::Key::ISO_Left_Tab {
            self.cycle_category(-1);
            return glib::Propagation::Stop;
        }

        // Alt/Ctrl + Left/Right cycles categories
        if state.contains(gdk::ModifierType::ALT_MASK) || state.contains(gdk::ModifierType::CONTROL_MASK) {
            if keyval == gdk::Key::Left {
                self.cycle_category(-1);
                return glib::Propagation::Stop;
            }
            if keyval == gdk::Key::Right {
                self.cycle_category(1);
                return glib::Propagation::Stop;
            }
        }

        // Arrows select in results. Left/Right fall through to the entry's
        // text cursor while there is a query to edit
        let editing = self.search_entry.has_focus() && !self.search_entry.text().is_empty();
        if keyval == gdk::Key::Up {
            self.result_view.move_selection_up();
            return glib::Propagation::Stop;
        }
        if keyval == gdk::Key::Down {
            self.result_view.move_selection_down();
            return glib::Propagation::Stop;
        }
        if keyval == gdk::Key::Left {
            if editing {
                return glib::Propagation::Proceed;
            }
            self.result_view.move_selection_left();
            return glib::Propagation::Stop;
        }
        if keyval == gdk::Key::Right {
            if editing {
                return glib::Propagation::Proceed;
            }
            self.result_view.move_selection_right();
            return glib::Propagation::Stop;
        }

        // Enter activates
        if keyval == gdk::Key::Return || keyval == gdk::Key::KP_Enter {
            if self.result_view.activate_selected() {
                return glib::Propagation::Stop;
            }
        }

        // Backspace on empty: ascend directory
        if keyval == gdk::Key::BackSpace && self.search_entry.text().is_empty() {
            if let Some(dir_path) = self.current_dir.borrow().as_ref() {
                if let Some(parent) = Path::new(dir_path).parent() {
                    let parent_str = parent.display().to_string();
                    if &parent_str != dir_path {
                        *self.current_dir.borrow_mut() = Some(parent_str.clone());
                        self.search_entry.set_placeholder_text(Some(&format!("Browsing: {}", parent_str)));
                        let results = self.browse_directory(&parent_str, "");
                        self.result_view.set_results(results, self.config.borrow().is_grid_view);
                        return glib::Propagation::Stop;
                    }
                }
            }
        }

        // Grab focus to entry when user types alphanumeric characters
        if !self.search_entry.has_focus()
            && !state.contains(gdk::ModifierType::CONTROL_MASK)
            && !state.contains(gdk::ModifierType::ALT_MASK)
            && !state.contains(gdk::ModifierType::SUPER_MASK)
        {
            if let Some(ch) = keyval.to_unicode() {
                if ch.is_alphanumeric() || ch.is_ascii_punctuation() {
                    self.search_entry.grab_focus();
                    let current_text = self.search_entry.text();
                    let new_text = format!("{}{}", current_text, ch);
                    self.search_entry.set_text(&new_text);
                    self.search_entry.set_position(-1);
                    return glib::Propagation::Stop;
                }
            } else if keyval == gdk::Key::BackSpace {
                self.search_entry.grab_focus();
                let current_text = self.search_entry.text();
                if !current_text.is_empty() {
                    let mut chars = current_text.chars();
                    chars.next_back();
                    self.search_entry.set_text(chars.as_str());
                    self.search_entry.set_position(-1);
                }
                return glib::Propagation::Stop;
            }
        }

        glib::Propagation::Proceed
    }
}
