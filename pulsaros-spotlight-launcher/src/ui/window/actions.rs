//! Row actions: the Ctrl+K action list and its execution.
use super::*;

/// One entry of the Ctrl+K panel.
#[derive(Clone, Debug)]
pub(super) struct UiAction {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) keys: Vec<String>,
    pub(super) danger: bool,
    pub(super) argv: Option<Vec<String>>,
}

impl SpotlightWindow {
    pub(super) fn actions_for(&self, rd: &RowData) -> Vec<UiAction> {
        let mk = |id: &str, title: &str, keys: Vec<String>, danger: bool, argv: Option<Vec<String>>| UiAction {
            id: id.into(),
            title: title.into(),
            keys,
            danger,
            argv,
        };
        let mut out = Vec::new();
        if let Some(c) = &rd.calc {
            let hist = rd.row.id.starts_with("calchist:");
            let _ = c;
            out.push(mk("calc_copy", "Copy Answer", shortcuts::label(Act::Run(0)), false, None));
            out.push(mk("calc_paste", "Paste Answer", shortcuts::label(Act::Run(1)), false, None));
            out.push(mk("calc_full", "Copy Question = Answer", shortcuts::label(Act::CopyFull), false, None));
            if hist {
                out.push(mk("calc_del", "Delete Entry", shortcuts::label(Act::DelItem), true, None));
                out.push(mk("calc_delall", "Delete All History", shortcuts::label(Act::DelAll), true, None));
            } else {
                out.push(mk("calc_history", "Calculator History", Vec::new(), false, None));
            }
            return out;
        }
        if let Some(a) = self.extra_actions(rd) {
            return a;
        }
        let primary = rd.row.actions.first().cloned();
        let primary_id = primary.as_ref().map(|a| a.id.clone()).unwrap_or_else(|| "open".into());
        out.push(mk(
            &primary_id,
            &primary_title(rd),
            shortcuts::label(Act::Run(0)),
            false,
            primary.and_then(|a| a.argv),
        ));
        match rd.row.kind {
            RowKind::App | RowKind::Command => {
                out.push(mk("copy_name", "Copy Name", shortcuts::label(Act::Run(1)), false, None));
            }
            RowKind::File => {
                out.push(mk("copy_path", "Copy Path", shortcuts::label(Act::Run(1)), false, None));
                if rd.payload.as_ref().map_or(false, |p| p.url.starts_with("file://")) {
                    out.push(mk("open_folder", "Open Containing Folder", shortcuts::label(Act::Run(2)), false, None));
                }
            }
            RowKind::Clipboard => {
                out.push(mk("copy_name", "Copy Text", shortcuts::label(Act::Run(1)), false, None));
            }
            _ => {}
        }
        for a in rd.row.actions.iter().skip(1) {
            let keys = match a.id.as_str() {
                "favorite" => shortcuts::label(Act::Favorite),
                "disable" => shortcuts::label(Act::Disable),
                _ => Vec::new(),
            };
            out.push(mk(&a.id, &a.title, keys, false, a.argv.clone()));
        }
        if rd.row.kind == RowKind::App {
            let desktop = rd.row.id.strip_prefix("app:").unwrap_or(&rd.row.id);
            let pinned = get_favorites().iter().any(|f| f == desktop);
            out.push(mk("pin", if pinned { "Unpin from Dock" } else { "Pin to Dock" }, Vec::new(), false, None));
            out.push(mk("uninstall", "Uninstall", Vec::new(), true, None));
        }
        out
    }

    pub(super) fn run_action_index(self: &Rc<Self>, n: usize) {
        if self.in_recent() {
            if let Some(rd) = self.result_view.selected_row() {
                self.perform(rd, "open", None);
            }
            return;
        }
        let Some(rd) = self.result_view.selected_row() else { return };
        if let Some(a) = self.actions_for(&rd).get(n).cloned() {
            self.perform(rd, &a.id, a.argv);
        }
    }

    pub(super) fn perform(self: &Rc<Self>, rd: RowData, action: &str, argv: Option<Vec<String>>) {
        let id = rd.row.id.clone();
        if id == "calc:history" && matches!(action, "open" | "run") {
            self.enter_calc_history();
            return;
        }
        if self.perform_extra(&rd, action) {
            return;
        }
        match action {
            "calc_copy" | "calc_paste" | "calc_full" => {
                let Some(c) = rd.calc.clone() else { return };
                if !self.in_calc_history() {
                    self.calc_history.borrow_mut().add(&c);
                }
                match action {
                    "calc_copy" => self.copy(&c.answer),
                    "calc_full" => self.copy(&format!("{} = {}", c.question, c.answer)),
                    _ => {
                        self.window.set_visible(false);
                        if let Some(mgr) = self.backend_clipboard() {
                            mgr.paste_clip(&c.answer);
                        } else {
                            self.copy(&c.answer);
                        }
                        return;
                    }
                }
                self.window.set_visible(false);
            }
            "calc_history" => self.enter_calc_history(),
            "calc_del" => {
                if let Some(n) = id.strip_prefix("calchist:").and_then(|n| n.parse::<u64>().ok()) {
                    self.calc_history.borrow_mut().delete(n);
                    self.toast("Deleted from history", None);
                    self.do_search(true);
                }
            }
            "calc_delall" => {
                let s = self.clone();
                self.show_dialog(
                    "Delete all calculator history?",
                    "This cannot be undone.",
                    None,
                    "Delete All",
                    true,
                    Box::new(move |_| {
                        s.calc_history.borrow_mut().delete_all();
                        s.toast("History cleared", None);
                        s.do_search(true);
                    }),
                );
            }
            "open" | "run" | "open_url" => {
                if let Some(q) = id.strip_prefix("history:") {
                    self.nav_apply(NavEvent::QueryChanged);
                    self.search_entry.set_text(q);
                    self.search_entry.set_position(-1);
                    return;
                }
                if rd.row.confirm {
                    let s = self.clone();
                    let title = rd.row.title.clone();
                    self.show_dialog(
                        &format!("{title}?"),
                        "This action can interrupt your session. Continue?",
                        None,
                        "Run",
                        true,
                        Box::new(move |_| s.launch(rd.clone(), argv.clone())),
                    );
                } else {
                    self.launch(rd, argv);
                }
            }
            "favorite" => {
                let added = self.settings.borrow_mut().toggle_favorite(&id);
                self.save_settings();
                let s = self.clone();
                let id2 = id.clone();
                self.toast(
                    &format!("{} {}", if added { "Added to Favorites:" } else { "Removed from Favorites:" }, rd.row.title),
                    Some(Rc::new(move || {
                        s.settings.borrow_mut().toggle_favorite(&id2);
                        s.save_settings();
                        s.do_search(true);
                    })),
                );
                self.do_search(true);
            }
            "alias" => {
                let current = self.settings.borrow().alias(&id).unwrap_or("").to_string();
                let s = self.clone();
                self.show_dialog(
                    &format!("Alias for {}", rd.row.title),
                    "Type the alias to put this item first. Leave empty to remove it.",
                    Some(current),
                    "Save",
                    false,
                    Box::new(move |text| {
                        let res = s.settings.borrow_mut().set_alias(&id, &text);
                        match res {
                            Ok(()) => {
                                s.save_settings();
                                s.toast("Alias saved", None);
                                s.do_search(true);
                            }
                            Err(e) => s.toast(&e, None),
                        }
                    }),
                );
            }
            "disable" => {
                self.settings.borrow_mut().disable(&id);
                self.save_settings();
                let s = self.clone();
                let id2 = id.clone();
                self.toast(
                    &format!("Disabled {}", rd.row.title),
                    Some(Rc::new(move || {
                        s.settings.borrow_mut().enable(&id2);
                        s.save_settings();
                        s.do_search(true);
                    })),
                );
                self.do_search(true);
            }
            "enable" => {
                self.settings.borrow_mut().enable(&id);
                self.save_settings();
                self.do_search(true);
            }
            "copy_name" => {
                self.copy(&rd.row.title);
                self.toast("Copied", None);
            }
            "copy_path" => {
                let text = rd.payload.as_ref().map(|p| p.url.trim_start_matches("file://").to_string()).unwrap_or_default();
                self.copy(&text);
                self.toast("Copied path", None);
            }
            "open_folder" => {
                if let Some(p) = &rd.payload {
                    let clean = p.url.trim_start_matches("file://");
                    if let Some(parent) = Path::new(clean).parent() {
                        open_file(&format!("file://{}", parent.display()));
                    }
                }
                self.window.set_visible(false);
            }
            "pin" => {
                let desktop = id.strip_prefix("app:").unwrap_or(&id).to_string();
                let mut favs = get_favorites();
                let msg = if favs.contains(&desktop) {
                    favs.retain(|x| x != &desktop);
                    "Unpinned from dock"
                } else {
                    favs.push(desktop);
                    "Pinned to dock"
                };
                set_favorites(&favs);
                self.toast(msg, None);
            }
            "uninstall" => {
                let s = self.clone();
                let name = rd.row.title.clone();
                self.show_dialog(
                    &format!("Uninstall {name}?"),
                    "The package will be removed from the system.",
                    None,
                    "Uninstall",
                    true,
                    Box::new(move |_| s.uninstall(id.strip_prefix("app:").unwrap_or(&id).to_string(), name.clone())),
                );
            }
            _ => {}
        }
    }

    pub(super) fn copy(&self, text: &str) {
        if let Some(display) = gdk::Display::default() {
            display.clipboard().set_text(text);
        }
    }

    /// Run the primary action: launch / open / paste / browse.
    pub(super) fn launch(self: &Rc<Self>, rd: RowData, argv: Option<Vec<String>>) {
        let query = self.search_entry.text().to_string();
        let mut hide = true;
        let mut ok = true;
        match rd.row.kind {
            RowKind::App => {
                let desktop = rd.row.id.strip_prefix("app:").unwrap_or(&rd.row.id);
                ok = open_file(&format!("app://{desktop}"));
            }
            RowKind::Command | RowKind::Fallback => {
                if let Some(argv) = argv.as_ref().filter(|a| !a.is_empty()) {
                    match std::process::Command::new(&argv[0]).args(&argv[1..]).spawn() {
                        Ok(mut child) => {
                            std::thread::spawn(move || {
                                let _ = child.wait();
                            });
                        }
                        Err(e) => {
                            self.toast(&format!("Could not run {}: {e}", argv[0]), None);
                            return;
                        }
                    }
                } else {
                    ok = false;
                }
            }
            RowKind::Calc | RowKind::Clipboard | RowKind::File => {
                if let Some(res) = rd.payload.clone() {
                    hide = self.open_result(&res);
                }
            }
        }
        if !ok {
            self.toast(&format!("Could not open {}", rd.row.title), None);
            return;
        }
        if matches!(rd.row.kind, RowKind::App | RowKind::Command) {
            self.frecency.borrow_mut().record_launch(&rd.row.id, &query, now_secs());
            self.save_frecency();
        }
        if !query.trim().is_empty() && hide {
            self.settings.borrow_mut().push_history(&query);
            self.save_settings();
        }
        if hide {
            self.window.set_visible(false);
        }
    }

    /// Returns true when the window should hide afterwards.
    pub(super) fn open_result(self: &Rc<Self>, res: &Payload) -> bool {
        if let Some(val) = res.url.strip_prefix("calc://") {
            self.copy(val);
            return true;
        }
        if let Some(idx_str) = res.url.strip_prefix("clipboard://") {
            if let (Some(mgr), Ok(idx)) = (self.backend_clipboard(), idx_str.parse::<usize>()) {
                if let Some(text) = mgr.get_clip_by_index(idx) {
                    self.window.set_visible(false);
                    mgr.paste_clip(&text);
                }
            }
            return true;
        }
        let clean_path = res.url.trim_start_matches("file://");
        if Path::new(clean_path).is_dir() && (res.mime == "inode/directory" || res.mime == "folder") {
            self.nav_apply(NavEvent::OpenDir(clean_path.to_string()));
            self.search_entry.set_text("");
            self.search_entry.set_placeholder_text(Some(&format!("Browsing: {}", clean_path)));
            // set_text only emits 'changed' when the text differs
            self.search_entry.emit_by_name::<()>("changed", &[]);
            return false;
        }
        open_file(&res.url);
        true
    }

    pub(super) fn backend_clipboard(&self) -> Option<ClipboardManager> {
        self.backend.clipboard()
    }

    pub(super) fn uninstall(self: &Rc<Self>, desktop_id: String, app_name: String) {
        self.progress_label.set_label(&format!("Uninstalling {}...", app_name));
        self.progress_spinner.start();
        self.progress_revealer.set_reveal_child(true);

        let (sender, receiver) = std::sync::mpsc::channel::<(bool, String)>();
        let s = self.clone();
        glib::timeout_add_local(Duration::from_millis(50), move || match receiver.try_recv() {
            Ok((success, message)) => {
                s.progress_spinner.stop();
                let (text, delay) = if success {
                    (format!("\u{2713} {} uninstalled successfully", app_name), 1800)
                } else {
                    s.progress_label.set_tooltip_text(Some(&message));
                    (format!("\u{2717} {}", clean_uninstall_message(&message, &app_name)), 4000)
                };
                s.progress_label.set_label(&text);
                let rev = s.progress_revealer.clone();
                glib::timeout_add_local(Duration::from_millis(delay), move || {
                    rev.set_reveal_child(false);
                    glib::ControlFlow::Break
                });
                glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(_) => glib::ControlFlow::Break,
        });

        std::thread::spawn(move || {
            let mut cmd = std::process::Command::new("pkm");
            cmd.args(["--uninstall", &desktop_id]);
            cmd.env("APPINSTALL_SUDO", "pkexec");
            let output = cmd.output();
            let success = output.as_ref().map_or(false, |o| o.status.success());
            let message = output.map_or_else(
                |e| e.to_string(),
                |o| String::from_utf8_lossy(&[o.stdout, o.stderr].concat()).into_owned(),
            );
            let _ = sender.send((success, message));
        });
    }
}
