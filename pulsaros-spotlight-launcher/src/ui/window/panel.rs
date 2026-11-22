//! Ctrl+K action panel and the "more" popover.
use super::*;

pub(super) struct Panel {
    pub(super) root: gtk4::Box,
    pub(super) list: gtk4::ListBox,
    pub(super) entry: gtk4::Entry,
    pub(super) all: RefCell<Vec<UiAction>>,
    pub(super) shown: RefCell<Vec<UiAction>>,
    pub(super) target: RefCell<Option<RowData>>,
}

/// Title of the primary action for the panel and the selected-row hint.
pub(crate) fn primary_title(r: &RowData) -> String {
    for (prefix, title) in [("dmenu:", "Select"), ("snippet:", "Paste Snippet"), ("window:", "Switch to Window"), ("script:", "Run Script")] {
        if r.row.id.starts_with(prefix) {
            return title.into();
        }
    }
    match r.row.kind {
        RowKind::App => "Open Application".into(),
        RowKind::Command => "Run".into(),
        RowKind::Clipboard => "Paste".into(),
        RowKind::Calc => "Copy Answer".into(),
        RowKind::File => match &r.payload {
            Some(p) if p.mime == "inode/directory" || p.mime == "folder" => "Open Folder".into(),
            Some(p) if p.url.starts_with("http") => "Open in Browser".into(),
            _ => "Open".into(),
        },
        RowKind::Fallback => {
            if r.row.id.starts_with("history:") {
                "Search Again".into()
            } else {
                r.row.actions.first().map(|a| a.title.clone()).unwrap_or_else(|| "Open".into())
            }
        }
    }
}

impl SpotlightWindow {
    pub(super) fn panel_open(&self) -> bool {
        self.panel.root.is_visible()
    }

    /// Context for Esc, Shift+Esc and the query-emptied event.
    pub(super) fn panel_ctx(&self) -> LayoutCtx {
        let nav = self.nav.borrow().clone();
        let empty = self.search_entry.text().is_empty();
        let sub_state = self.dialog_open()
            || self.panel_open()
            || self.x.view.get() != View::Root
            || !nav.is_root();
        LayoutCtx {
            mode_all: self.category.get() == Category::All,
            query_empty: empty,
            query_typed: nav.typed,
            at_root: !sub_state && empty,
            home_opened: nav.opened == Opened::Home,
            history_opened: nav.opened == Opened::History,
            ..self.layout_ctx()
        }
    }

    pub(super) fn open_more_menu(self: &Rc<Self>) {
        let menu_box = gtk4::Box::new(gtk4::Orientation::Vertical, 3);
        menu_box.add_css_class("popover-menu-box");

        // 1. Actions for currently selected item (if any)
        if let Some(rd) = self.result_view.selected_row() {
            let actions = self.actions_for(&rd);
            if !actions.is_empty() {
                for act in actions.into_iter().take(4) {
                    let btn = gtk4::Button::new();
                    btn.add_css_class("popover-item");
                    let row_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
                    let label = gtk4::Label::builder().label(&act.title).xalign(0.0).hexpand(true).build();
                    row_box.append(&label);
                    if !act.keys.is_empty() {
                        let k = gtk4::Label::builder().label(&act.keys.join("+")).build();
                        k.add_css_class("keycap");
                        row_box.append(&k);
                    }
                    btn.set_child(Some(&row_box));
                    let s = self.clone();
                    let a_id = act.id.clone();
                    let a_argv = act.argv.clone();
                    let rd_c = rd.clone();
                    btn.connect_clicked(move |_| {
                        s.more_popover.popdown();
                        s.perform(rd_c.clone(), &a_id, a_argv.clone());
                    });
                    menu_box.append(&btn);
                }
                let sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
                menu_box.append(&sep);
            }
        }

        // 2. Global tools & options
        let tools: &[(&str, &str, &str)] = &[
            ("ui:quicklink_create", "Create Quicklink", "list-add-symbolic"),
            ("ui:quicklink_manage", "Manage Quicklinks", "emblem-system-symbolic"),
            ("ui:snippet_create", "Create Snippet", "insert-text-symbolic"),
            ("ui:snippets", "Snippets", "edit-paste-symbolic"),
            ("ui:windows", "Open Windows", "preferences-system-windows-symbolic"),
            ("ui:emoji", "Emoji & Symbols", "face-smile-symbolic"),
            ("cmd:settings", "System Settings", "preferences-system-symbolic"),
        ];

        for &(id, label_text, icon_name) in tools {
            let btn = gtk4::Button::new();
            btn.add_css_class("popover-item");
            let row_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            let img = gtk4::Image::from_icon_name(icon_name);
            img.set_pixel_size(16);
            let label = gtk4::Label::builder().label(label_text).xalign(0.0).hexpand(true).build();
            row_box.append(&img);
            row_box.append(&label);
            btn.set_child(Some(&row_box));

            let s = self.clone();
            btn.connect_clicked(move |_| {
                s.more_popover.popdown();
                match id {
                    "ui:quicklink_create" => s.quicklink_form(),
                    "ui:quicklink_manage" => s.quicklink_manage(),
                    "ui:snippet_create" => s.snippet_form(),
                    "ui:snippets" => s.push_view(View::Snippets, "Search snippets\u{2026}"),
                    "ui:windows" => s.push_view(View::Windows, "Filter open windows\u{2026}"),
                    "ui:emoji" => s.open_emoji_panel(),
                    "cmd:settings" => {
                        let _ = std::process::Command::new("gnome-control-center").spawn();
                    }
                    _ => {}
                }
            });
            menu_box.append(&btn);
        }

        self.more_popover.set_child(Some(&menu_box));
        self.more_popover.popup();
    }

    pub(super) fn open_panel(self: &Rc<Self>) {
        if self.x.view.get() == View::Emoji {
            self.open_emoji_panel();
            return;
        }
        let Some(rd) = self.result_view.selected_row() else { return };
        if self.in_recent() {
            return;
        }
        *self.panel.all.borrow_mut() = self.actions_for(&rd);
        *self.panel.target.borrow_mut() = Some(rd);
        self.panel.entry.set_text("");
        self.fill_panel("");
        self.panel.root.set_visible(true);
        self.panel.entry.grab_focus();
    }

    pub(super) fn close_panel(&self) {
        if self.panel.root.is_visible() {
            self.panel.root.set_visible(false);
            self.search_entry.grab_focus();
        }
    }

    pub(super) fn fill_panel(&self, filter: &str) {
        let f = filter.trim().to_lowercase();
        while let Some(c) = self.panel.list.first_child() {
            self.panel.list.remove(&c);
        }
        let shown: Vec<UiAction> =
            self.panel.all.borrow().iter().filter(|a| f.is_empty() || a.title.to_lowercase().contains(&f)).cloned().collect();
        for a in &shown {
            let row = gtk4::ListBoxRow::new();
            let b = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
            b.add_css_class("action-item");
            if a.danger {
                b.add_css_class("danger");
            }
            let t = gtk4::Label::builder().label(&a.title).xalign(0.0).hexpand(true).build();
            t.add_css_class("action-title");
            b.append(&t);
            if !a.keys.is_empty() {
                b.append(&keycap_row(&a.keys));
            }
            row.set_child(Some(&b));
            self.panel.list.append(&row);
        }
        *self.panel.shown.borrow_mut() = shown;
        if let Some(first) = self.panel.list.row_at_index(0) {
            self.panel.list.select_row(Some(&first));
        }
    }

    pub(super) fn panel_step(&self, delta: i32) {
        let n = self.panel.shown.borrow().len() as i32;
        if n == 0 {
            return;
        }
        let cur = self.panel.list.selected_row().map(|r| r.index()).unwrap_or(0);
        let next = (cur + delta).rem_euclid(n);
        if let Some(r) = self.panel.list.row_at_index(next) {
            self.panel.list.select_row(Some(&r));
        }
    }

    pub(super) fn panel_activate(self: &Rc<Self>) {
        let idx = self.panel.list.selected_row().map(|r| r.index()).unwrap_or(-1);
        let act = self.panel.shown.borrow().get(idx.max(0) as usize).cloned();
        let target = self.panel.target.borrow().clone();
        if let (Some(a), Some(rd)) = (act, target) {
            self.close_panel();
            self.perform(rd, &a.id, a.argv);
        }
    }

    pub(super) fn setup_panel_events(self: &Rc<Self>) {
        let s = self.clone();
        self.panel.entry.connect_changed(move |e| s.fill_panel(&e.text()));
        let s = self.clone();
        self.panel.list.connect_row_activated(move |_, _| s.panel_activate());
    }
}
