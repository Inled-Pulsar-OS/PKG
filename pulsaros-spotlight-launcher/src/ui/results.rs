use crate::apps::DesktopApp;
use crate::search::SearchResult;
use crate::utils::{get_file_icon, open_file};
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::rc::Rc;
use std::time::Duration;

fn find_icon_file(name: &str) -> Option<String> {
    const EXTS: [&str; 3] = ["png", "svg", "xpm"];
    for root in ["/usr/share/icons", "/usr/local/share/icons"] {
        let themes = match std::fs::read_dir(root) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for theme in themes.filter_map(Result::ok) {
            let sizes = match std::fs::read_dir(theme.path()) {
                Ok(rd) => rd,
                Err(_) => continue,
            };
            for size in sizes.filter_map(Result::ok) {
                for ext in EXTS {
                    let candidate = size.path().join("apps").join(format!("{name}.{ext}"));
                    if candidate.is_file() {
                        return Some(candidate.to_string_lossy().into_owned());
                    }
                }
            }
        }
    }
    None
}

fn app_icon(icon: &str, desktop_file: &str) -> gtk4::Image {
    // 1. Absolute path in the .desktop Icon= field
    if icon.starts_with('/') && std::path::Path::new(icon).exists() {
        return gtk4::Image::from_file(icon);
    }

    let theme = gtk4::IconTheme::default();

    // 2. Themed lookup, as-is and stripped of any file extension
    let mut candidates = vec![icon.to_string()];
    if let Some(dot) = icon.rfind('.') {
        candidates.push(icon[..dot].to_string());
    }
    for candidate in &candidates {
        if !candidate.is_empty() && theme.has_icon(candidate) {
            return gtk4::Image::from_icon_name(candidate);
        }
    }

    // 3. Plain file lookup by Icon= name and by the .desktop basename.
    //    Some packages ship e.g. Icon=safari but install seafari.png
    let stem = std::path::Path::new(desktop_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    for name in [icon.to_string(), stem] {
        if name.is_empty() || name.starts_with('/') {
            continue;
        }
        for base in ["/usr/share/pixmaps", "/usr/local/share/pixmaps"] {
            for ext in ["png", "svg", "xpm", ""] {
                let p = if ext.is_empty() {
                    format!("{base}/{name}")
                } else {
                    format!("{base}/{name}.{ext}")
                };
                if std::path::Path::new(&p).is_file() {
                    return gtk4::Image::from_file(p);
                }
            }
        }
        if let Some(p) = find_icon_file(&name) {
            return gtk4::Image::from_file(p);
        }
    }

    gtk4::Image::from_icon_name("application-x-executable")
}

fn result_icon(result: &SearchResult) -> gtk4::Image {
    if let Some(app) = &result.app {
        return app_icon(&app.icon, &app.filename);
    }
    let is_dir = result.mime == "inode/directory" || result.mime == "folder";
    get_file_icon(&result.url, Some(&result.mime), is_dir)
}

pub struct ResultView {
    stack: gtk4::Stack,
    list_box: gtk4::ListBox,
    grid: gtk4::FlowBox,
    results: Rc<RefCell<Vec<SearchResult>>>,
    selected_index: Rc<RefCell<Option<usize>>>,
    on_activate: Rc<dyn Fn(SearchResult)>,
    on_uninstall_start: Rc<dyn Fn(String, String)>,
    on_uninstall_done: Rc<dyn Fn(bool, String, String)>,
    popover: gtk4::Popover,
    context_menu_index: Rc<RefCell<Option<usize>>>,
}

impl ResultView {
    pub fn new<F, U1, U2>(
        on_activate: F,
        on_uninstall_start: U1,
        on_uninstall_done: U2,
    ) -> Self
    where
        F: Fn(SearchResult) + 'static,
        U1: Fn(String, String) + 'static,
        U2: Fn(bool, String, String) + 'static,
    {
        let stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .build();

        let list_box = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::Single)
            .build();

        let grid = gtk4::FlowBox::builder()
            .valign(gtk4::Align::Start)
            .max_children_per_line(6)
            .selection_mode(gtk4::SelectionMode::Single)
            .build();

        stack.add_named(&list_box, Some("list"));
        stack.add_named(&grid, Some("grid"));

        let results = Rc::new(RefCell::new(Vec::new()));
        let selected_index = Rc::new(RefCell::new(None));
        let context_menu_index = Rc::new(RefCell::new(None));

        let popover = gtk4::Popover::builder()
            .has_arrow(false)
            .position(gtk4::PositionType::Bottom)
            .autohide(true)
            .build();
        popover.add_css_class("ctx-menu");

        let view = Self {
            stack,
            list_box,
            grid,
            results,
            selected_index,
            on_activate: Rc::new(on_activate),
            on_uninstall_start: Rc::new(on_uninstall_start),
            on_uninstall_done: Rc::new(on_uninstall_done),
            popover,
            context_menu_index,
        };

        view.setup_events();
        view.setup_context_menu();

        view
    }

    pub fn widget(&self) -> &gtk4::Stack {
        &self.stack
    }

    pub fn set_popover_parent<P: IsA<gtk4::Widget>>(&self, parent: &P) {
        self.popover.set_parent(parent);
    }

    pub fn set_results(&self, new_results: Vec<SearchResult>, as_grid: bool) {
        *self.results.borrow_mut() = new_results.clone();
        *self.selected_index.borrow_mut() = None;

        // Clear children
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        while let Some(child) = self.grid.first_child() {
            self.grid.remove(&child);
        }

        for result in new_results.iter().take(200) {
            // Build list row
            let list_row = self.build_list_row(result);
            self.list_box.append(&list_row);

            // Build grid child
            let grid_child = self.build_grid_child(result);
            self.grid.insert(&grid_child, -1);
        }

        self.stack.set_visible_child_name(if as_grid { "grid" } else { "list" });

        if let Some(adj) = self.viewport_vadjustment() {
            adj.set_value(0.0);
        }

        if !as_grid {
            if let Some(first_row) = self.list_box.row_at_index(0) {
                self.list_box.select_row(Some(&first_row));
                *self.selected_index.borrow_mut() = Some(0);
            }
        } else {
            if let Some(first_child) = self.grid.child_at_index(0) {
                self.grid.select_child(&first_child);
                *self.selected_index.borrow_mut() = Some(0);
            }
        }
    }

    fn build_list_row(&self, result: &SearchResult) -> gtk4::ListBoxRow {
        let row = gtk4::ListBoxRow::new();
        let box_widget = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        box_widget.add_css_class("result-item-list");

        let icon = result_icon(result);
        icon.set_pixel_size(32);
        icon.add_css_class("result-icon");

        let text_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);

        let title_label = gtk4::Label::builder()
            .label(&result.title)
            .xalign(0.0)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .max_width_chars(60)
            .build();
        title_label.add_css_class("result-title");
        text_box.append(&title_label);

        let mut sub_text = result.snippet.clone();
        if sub_text.is_empty() {
            if result.url.starts_with("file://") {
                sub_text = result.url.trim_start_matches("file://").to_string();
            } else if result.url.starts_with("http://") || result.url.starts_with("https://") {
                sub_text = result.url.clone();
            }
        }

        if !sub_text.is_empty() {
            let snippet_label = gtk4::Label::builder()
                .label(&sub_text)
                .xalign(0.0)
                .ellipsize(if result.url.starts_with("file://") {
                    gtk4::pango::EllipsizeMode::Middle
                } else {
                    gtk4::pango::EllipsizeMode::End
                })
                .max_width_chars(60)
                .build();
            snippet_label.add_css_class("result-snippet");
            text_box.append(&snippet_label);
        }

        box_widget.append(&icon);
        box_widget.append(&text_box);
        row.set_child(Some(&box_widget));

        row
    }

    fn build_grid_child(&self, result: &SearchResult) -> gtk4::FlowBoxChild {
        let child = gtk4::FlowBoxChild::new();
        let box_widget = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        box_widget.add_css_class("result-item-grid");
        box_widget.set_size_request(90, -1);

        let icon = result_icon(result);
        icon.set_pixel_size(48);
        icon.add_css_class("result-icon-grid");

        let title_label = gtk4::Label::builder()
            .label(&result.title)
            .wrap(true)
            .justify(gtk4::Justification::Center)
            .max_width_chars(12)
            .halign(gtk4::Align::Center)
            .build();
        title_label.add_css_class("result-title-grid");

        box_widget.append(&icon);
        box_widget.append(&title_label);
        child.set_child(Some(&box_widget));

        child
    }

    fn setup_events(&self) {
        let on_activate = self.on_activate.clone();
        let results = self.results.clone();
        let selected_index = self.selected_index.clone();

        // List row activated
        self.list_box.connect_row_activated(move |_, row| {
            let idx = row.index() as usize;
            if let Some(res) = results.borrow().get(idx) {
                (*on_activate)(res.clone());
            }
        });

        let results_c = self.results.clone();
        let on_activate_c = self.on_activate.clone();
        // Grid child activated
        self.grid.connect_child_activated(move |_, child| {
            let idx = child.index() as usize;
            if let Some(res) = results_c.borrow().get(idx) {
                (*on_activate_c)(res.clone());
            }
        });

        // Track selected index
        let sel_idx = selected_index.clone();
        self.list_box.connect_row_selected(move |_, row| {
            if let Some(r) = row {
                *sel_idx.borrow_mut() = Some(r.index() as usize);
            }
        });

        let sel_idx_c = selected_index.clone();
        self.grid.connect_selected_children_changed(move |fb| {
            let selected = fb.selected_children();
            if let Some(c) = selected.first() {
                *sel_idx_c.borrow_mut() = Some(c.index() as usize);
            }
        });

        // Gesture click right button for ListBox
        let list_click = gtk4::GestureClick::builder().button(3).build();
        let list_box_c = self.list_box.clone();
        let results_cc = self.results.clone();
        let popover_c = self.popover.clone();
        let ctx_menu_idx = self.context_menu_index.clone();
        list_click.connect_pressed(move |gesture, _, x, y| {
            gesture.set_state(gtk4::EventSequenceState::Claimed);
            let row = list_box_c.row_at_y(y as i32);
            if let Some(r) = row {
                list_box_c.select_row(Some(&r));
                let idx = r.index() as usize;
                if let Some(res) = results_cc.borrow().get(idx) {
                    *ctx_menu_idx.borrow_mut() = Some(idx);
                    if popover_c.parent().as_ref() != Some(r.upcast_ref()) {
                        popover_c.set_parent(&r);
                    }
                    let rect = gdk::Rectangle::new(x as i32, y as i32, 1, 1);
                    popover_c.set_pointing_to(Some(&rect));
                    popover_c.popup();
                }
            }
        });
        self.list_box.add_controller(list_click);

        // Gesture click right button for Grid
        let grid_click = gtk4::GestureClick::builder().button(3).build();
        let grid_c = self.grid.clone();
        let results_ccc = self.results.clone();
        let popover_cc = self.popover.clone();
        let ctx_menu_idx_c = self.context_menu_index.clone();
        grid_click.connect_pressed(move |gesture, _, x, y| {
            gesture.set_state(gtk4::EventSequenceState::Claimed);
            let child = grid_c.child_at_pos(x as i32, y as i32);
            if let Some(c) = child {
                grid_c.select_child(&c);
                let idx = c.index() as usize;
                if let Some(res) = results_ccc.borrow().get(idx) {
                    *ctx_menu_idx_c.borrow_mut() = Some(idx);
                    if popover_cc.parent().as_ref() != Some(c.upcast_ref()) {
                        popover_cc.set_parent(&c);
                    }
                    let rect = gdk::Rectangle::new(x as i32, y as i32, 1, 1);
                    popover_cc.set_pointing_to(Some(&rect));
                    popover_cc.popup();
                }
            }
        });
        self.grid.add_controller(grid_click);
    }

    fn setup_context_menu(&self) {
        let outer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        outer.add_css_class("ctx-menu-box");

        // 1. Open
        let btn_open = gtk4::Button::with_label("Open");
        btn_open.add_css_class("ctx-menu-btn");
        btn_open.set_halign(gtk4::Align::Fill);
        outer.append(&btn_open);

        // 2. Open Folder
        let btn_open_dir = gtk4::Button::with_label("Open containing folder");
        btn_open_dir.add_css_class("ctx-menu-btn");
        btn_open_dir.set_halign(gtk4::Align::Fill);
        outer.append(&btn_open_dir);

        let sep1 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        outer.append(&sep1);

        // 3. Pin to Dock
        let btn_pin = gtk4::Button::with_label("Pin to dock");
        btn_pin.add_css_class("ctx-menu-btn");
        btn_pin.set_halign(gtk4::Align::Fill);
        outer.append(&btn_pin);

        // 4. Uninstall
        let btn_uninstall = gtk4::Button::with_label("Uninstall");
        btn_uninstall.add_css_class("ctx-menu-btn");
        btn_uninstall.add_css_class("ctx-menu-btn-danger");
        btn_uninstall.set_halign(gtk4::Align::Fill);
        outer.append(&btn_uninstall);

        let sep2 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        outer.append(&sep2);

        // 5. Copy Name
        let btn_copy_name = gtk4::Button::with_label("Copy name");
        btn_copy_name.add_css_class("ctx-menu-btn");
        btn_copy_name.set_halign(gtk4::Align::Fill);
        outer.append(&btn_copy_name);

        // 6. Copy Path
        let btn_copy_path = gtk4::Button::with_label("Copy path");
        btn_copy_path.add_css_class("ctx-menu-btn");
        btn_copy_path.set_halign(gtk4::Align::Fill);
        outer.append(&btn_copy_path);

        self.popover.set_child(Some(&outer));

        // Connect popover opened to toggle buttons visibility
        let results_c = self.results.clone();
        let ctx_menu_idx = self.context_menu_index.clone();
        let btn_open_dir_c = btn_open_dir.clone();
        let btn_copy_path_c = btn_copy_path.clone();
        let btn_pin_c = btn_pin.clone();
        let btn_uninstall_c = btn_uninstall.clone();

        self.popover.connect_map(move |_| {
            if let Some(idx) = *ctx_menu_idx.borrow() {
                if let Some(res) = results_c.borrow().get(idx) {
                    let is_app = res.app.is_some();
                    btn_open_dir_c.set_visible(!is_app);
                    btn_copy_path_c.set_visible(!is_app);
                    btn_pin_c.set_visible(is_app);
                    btn_uninstall_c.set_visible(is_app);

                    if is_app {
                        let app = res.app.as_ref().unwrap();
                        let favs = get_favorites();
                        let is_pinned = favs.contains(&app.filename);
                        btn_pin_c.set_label(if is_pinned { "Unpin from dock" } else { "Pin to dock" });
                    }
                }
            }
        });

        // 1. Click Open
        let on_activate = self.on_activate.clone();
        let results_cc = self.results.clone();
        let ctx_menu_idx_c = self.context_menu_index.clone();
        let popover_c = self.popover.clone();
        btn_open.connect_clicked(move |_| {
            popover_c.popdown();
            if let Some(idx) = *ctx_menu_idx_c.borrow() {
                if let Some(res) = results_cc.borrow().get(idx) {
                    (*on_activate)(res.clone());
                }
            }
        });

        // 2. Click Open Folder
        let results_ccc = self.results.clone();
        let ctx_menu_idx_cc = self.context_menu_index.clone();
        let popover_cc = self.popover.clone();
        btn_open_dir.connect_clicked(move |_| {
            popover_cc.popdown();
            if let Some(idx) = *ctx_menu_idx_cc.borrow() {
                if let Some(res) = results_ccc.borrow().get(idx) {
                    let clean_path = res.url.trim_start_matches("file://");
                    if let Some(parent) = std::path::Path::new(clean_path).parent() {
                        let parent_url = format!("file://{}", parent.display());
                        open_file(&parent_url);
                    }
                }
            }
        });

        // 3. Click Pin to Dock
        let results_cccc = self.results.clone();
        let ctx_menu_idx_ccc = self.context_menu_index.clone();
        let popover_ccc = self.popover.clone();
        btn_pin.connect_clicked(move |_| {
            popover_ccc.popdown();
            if let Some(idx) = *ctx_menu_idx_ccc.borrow() {
                if let Some(res) = results_cccc.borrow().get(idx) {
                    if let Some(app) = &res.app {
                        let mut favs = get_favorites();
                        if favs.contains(&app.filename) {
                            favs.retain(|x| x != &app.filename);
                        } else {
                            favs.push(app.filename.clone());
                        }
                        set_favorites(&favs);
                    }
                }
            }
        });

        // 4. Click Uninstall
        let results_5 = self.results.clone();
        let ctx_menu_idx_5 = self.context_menu_index.clone();
        let popover_5 = self.popover.clone();
        let on_un_start = self.on_uninstall_start.clone();
        let on_un_done = self.on_uninstall_done.clone();
        btn_uninstall.connect_clicked(move |_| {
            popover_5.popdown();
            if let Some(idx) = *ctx_menu_idx_5.borrow() {
                if let Some(res) = results_5.borrow().get(idx) {
                    if let Some(app) = &res.app {
                        let desktop_id = app.filename.clone();
                        let app_name = app.name.clone();
                        (*on_un_start)(desktop_id.clone(), app_name.clone());

                        let (sender, receiver) = std::sync::mpsc::channel::<(bool, String)>();
                        let on_done = on_un_done.clone();
                        let app_name_c = app_name.clone();
                        gtk4::glib::idle_add_local(move || {
                            match receiver.try_recv() {
                                Ok((success, message)) => {
                                    (*on_done)(success, message, app_name_c.clone());
                                    gtk4::glib::ControlFlow::Break
                                }
                                Err(std::sync::mpsc::TryRecvError::Empty) => {
                                    gtk4::glib::ControlFlow::Continue
                                }
                                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                    gtk4::glib::ControlFlow::Break
                                }
                            }
                        });

                        std::thread::spawn(move || {
                            let mut cmd = Command::new("pkm");
                            cmd.args(&["--uninstall", &desktop_id]);
                            cmd.env("APPINSTALL_SUDO", "pkexec");
                            let output = cmd.output();
                            let success = output.as_ref().map_or(false, |o| o.status.success());
                            let message = output.map_or_else(
                                |e| e.to_string(),
                                |o| {
                                    let merged = [o.stdout, o.stderr].concat();
                                    String::from_utf8_lossy(&merged).into_owned()
                                },
                            );
                            let _ = sender.send((success, message));
                        });
                    }
                }
            }
        });

        // 5. Click Copy Name
        let results_6 = self.results.clone();
        let ctx_menu_idx_6 = self.context_menu_index.clone();
        let popover_6 = self.popover.clone();
        btn_copy_name.connect_clicked(move |_| {
            popover_6.popdown();
            if let Some(idx) = *ctx_menu_idx_6.borrow() {
                if let Some(res) = results_6.borrow().get(idx) {
                    if let Some(display) = gdk::Display::default() {
                        display.clipboard().set_text(&res.title);
                    }
                }
            }
        });

        // 6. Click Copy Path
        let results_7 = self.results.clone();
        let ctx_menu_idx_7 = self.context_menu_index.clone();
        let popover_7 = self.popover.clone();
        btn_copy_path.connect_clicked(move |_| {
            popover_7.popdown();
            if let Some(idx) = *ctx_menu_idx_7.borrow() {
                if let Some(res) = results_7.borrow().get(idx) {
                    if let Some(display) = gdk::Display::default() {
                        display.clipboard().set_text(&res.url);
                    }
                }
            }
        });
    }

    fn viewport_vadjustment(&self) -> Option<gtk4::Adjustment> {
        self.stack
            .ancestor(gtk4::ScrolledWindow::static_type())
            .and_then(|w| w.downcast::<gtk4::ScrolledWindow>().ok())
            .map(|sw| sw.vadjustment())
    }

    fn ensure_visible<W: IsA<gtk4::Widget>>(&self, widget: &W) {
        let Some(adj) = self.viewport_vadjustment() else {
            return;
        };
        let alloc = widget.allocation();
        let top = alloc.y() as f64;
        let bottom = top + alloc.height() as f64;
        let value = adj.value();
        let page = adj.page_size();
        if top < value {
            adj.set_value(top);
        } else if bottom > value + page {
            adj.set_value(bottom - page);
        }
    }

    pub fn move_selection_up(&self) {
        // Bind first: an `if let ... = *refcell.borrow()` scrutinee keeps the
        // borrow alive across the whole block, and select_child/select_row
        // emit signals whose handlers borrow again
        let current = *self.selected_index.borrow();
        if self.stack.visible_child_name().as_deref() == Some("list") {
            if let Some(idx) = current {
                if idx > 0 {
                    if let Some(row) = self.list_box.row_at_index((idx - 1) as i32) {
                        self.list_box.select_row(Some(&row));
                        self.ensure_visible(&row);
                    }
                }
            }
        } else if let Some(idx) = current {
            if idx >= 6 {
                if let Some(child) = self.grid.child_at_index((idx - 6) as i32) {
                    self.grid.select_child(&child);
                    self.ensure_visible(&child);
                }
            }
        }
    }

    pub fn move_selection_down(&self) {
        let max_len = self.results.borrow().len();
        let current = *self.selected_index.borrow();
        if self.stack.visible_child_name().as_deref() == Some("list") {
            let idx = current.unwrap_or(0);
            if idx + 1 < max_len {
                if let Some(row) = self.list_box.row_at_index((idx + 1) as i32) {
                    self.list_box.select_row(Some(&row));
                    self.ensure_visible(&row);
                }
            }
        } else if let Some(idx) = current {
            let next_idx = idx + 6;
            if next_idx < max_len {
                if let Some(child) = self.grid.child_at_index(next_idx as i32) {
                    self.grid.select_child(&child);
                    self.ensure_visible(&child);
                }
            }
        }
    }

    pub fn move_selection_left(&self) {
        if self.stack.visible_child_name().as_deref() == Some("grid") {
            let current = *self.selected_index.borrow();
            if let Some(idx) = current {
                if idx > 0 {
                    if let Some(child) = self.grid.child_at_index((idx - 1) as i32) {
                        self.grid.select_child(&child);
                        self.ensure_visible(&child);
                    }
                }
            }
        }
    }

    pub fn move_selection_right(&self) {
        if self.stack.visible_child_name().as_deref() == Some("grid") {
            let max_len = self.results.borrow().len();
            let current = *self.selected_index.borrow();
            if let Some(idx) = current {
                if idx + 1 < max_len {
                    if let Some(child) = self.grid.child_at_index((idx + 1) as i32) {
                        self.grid.select_child(&child);
                        self.ensure_visible(&child);
                    }
                }
            }
        }
    }

    pub fn activate_selected(&self) -> bool {
        // Clone the result and drop the borrows before invoking the
        // callback: activation may re-enter set_results()
        let res = {
            let results = self.results.borrow();
            (*self.selected_index.borrow())
                .and_then(|idx| results.get(idx).cloned())
        };
        if let Some(res) = res {
            (*self.on_activate)(res);
            return true;
        }
        false
    }
}

// Favorite apps helpers
fn get_favorites() -> Vec<String> {
    if let Ok(output) = Command::new("gsettings")
        .args(&["get", "org.gnome.shell", "favorite-apps"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let clean = if stdout.starts_with("@as ") { &stdout[4..] } else { &stdout };
        // Simple manual parsing of ['a', 'b'] string to Vec
        let mut list = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        for c in clean.chars() {
            if c == '\'' || c == '"' {
                in_quotes = !in_quotes;
                if !in_quotes && !current.is_empty() {
                    list.push(current.clone());
                    current.clear();
                }
            } else if in_quotes {
                current.push(c);
            }
        }
        return list;
    }
    Vec::new()
}

fn set_favorites(favs: &[String]) {
    let formatted = format!("[{}]", favs.iter().map(|s| format!("'{}'", s)).collect::<Vec<_>>().join(", "));
    let _ = Command::new("gsettings")
        .args(&["set", "org.gnome.shell", "favorite-apps", &formatted])
        .status();
}
