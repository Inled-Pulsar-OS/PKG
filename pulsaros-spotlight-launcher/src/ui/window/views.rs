//! Pushed views (snippets, windows, emoji, script output), the row dispatch and the state the extras share.
//! The logic lives in `crate::core`; the feature files next to this one build the widgets.
use super::*;
use super::args::ArgSet;
use super::emoji::EMOJI_COLS;
use super::snippets::{snippet_row, FormField};
use crate::core::emoji::Emoji;
use crate::core::quicklinks::{self, Quicklinks};
use crate::core::root_search::ActionDesc;
use crate::core::script_commands::{self, Script};
use crate::core::snippets::Snippets;
use crate::core::windows::{self as wincore, WindowEntry};
use std::sync::mpsc;

/// Pushed views on top of the root list (Esc pops).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum View {
    Root,
    Snippets,
    Windows,
    Emoji,
    Output,
}

pub(super) struct Extras {
    pub(super) view: Cell<View>,
    pub(super) stack: gtk4::Stack,
    pub(super) arg_box: gtk4::Box,
    pub(super) form: gtk4::Box,
    pub(super) base: Vec<Command>,
    pub(super) args: RefCell<Option<ArgSet>>,
    pub(super) fields: RefCell<Vec<FormField>>,
    pub(super) form_cb: RefCell<Option<Box<dyn Fn(Vec<String>)>>>,
    pub(super) emoji_flow: gtk4::FlowBox,
    pub(super) emoji_scroll: gtk4::ScrolledWindow,
    pub(super) emoji_name: gtk4::Label,
    pub(super) emoji_list: RefCell<Vec<&'static Emoji>>,
    pub(super) emoji_shown: Cell<usize>,
    pub(super) output: gtk4::TextView,
    pub(super) scripts: RefCell<Vec<Script>>,
    pub(super) quicklinks: RefCell<Quicklinks>,
    pub(super) snippets: RefCell<Snippets>,
    pub(super) windows: RefCell<Vec<WindowEntry>>,
}

fn ui_cmd(id: &str, title: &str, breadcrumb: &str, keywords: &[&str], icon: &str) -> Command {
    Command {
        id: id.into(),
        title: title.into(),
        breadcrumb: breadcrumb.into(),
        keywords: keywords.iter().map(|k| k.to_string()).collect(),
        icon: icon.into(),
        argv: Vec::new(),
        confirm: false,
        state: None,
    }
}

fn ui_commands() -> Vec<Command> {
    vec![
        ui_cmd("ui:snippets", "Snippets", "Snippets", &["text", "expand", "paste"], "insert-text-symbolic"),
        ui_cmd("ui:snippet_create", "Create Snippet", "Snippets", &["new", "add", "text"], "list-add-symbolic"),
        ui_cmd("ui:emoji", "Emoji & Symbols", "Emoji", &["emoji", "symbols", "smiley", "unicode"], "face-smile-symbolic"),
        ui_cmd("ui:quicklink_create", "Create Quicklink", "Quicklinks", &["new", "add", "bookmark", "url"], "list-add-symbolic"),
        ui_cmd("ui:quicklink_manage", "Manage Quicklinks", "Quicklinks", &["delete", "bookmark", "url"], "emblem-system-symbolic"),
    ]
}

/// Run `work` on a thread and call `done` on the UI thread with its result.
pub(super) fn spawn_poll<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static, done: impl FnOnce(T) + 'static) {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(work());
    });
    let mut done = Some(done);
    glib::timeout_add_local(Duration::from_millis(40), move || match rx.try_recv() {
        Ok(v) => {
            if let Some(d) = done.take() {
                d(v);
            }
            glib::ControlFlow::Break
        }
        Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
        Err(_) => glib::ControlFlow::Break,
    });
}

pub(super) fn ua(id: &str, title: &str, keys: Vec<String>, danger: bool) -> UiAction {
    UiAction { id: id.into(), title: title.into(), keys, danger, argv: None }
}

fn only(rows: Vec<RowData>, title: &str) -> SectionList {
    vec![(title.to_string(), rows)]
}

impl Extras {
    pub fn new(store: &Store, base: Vec<Command>) -> Self {
        let stack = gtk4::Stack::new();
        stack.set_vexpand(true);

        let arg_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        arg_box.set_valign(gtk4::Align::Center);
        arg_box.set_visible(false);

        let form = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        form.set_visible(false);

        let emoji_flow = gtk4::FlowBox::builder()
            .min_children_per_line(EMOJI_COLS as u32)
            .max_children_per_line(EMOJI_COLS as u32)
            .homogeneous(true)
            .selection_mode(gtk4::SelectionMode::Single)
            .activate_on_single_click(true)
            .valign(gtk4::Align::Start)
            .build();
        emoji_flow.add_css_class("emoji-grid");
        let emoji_scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&emoji_flow)
            .build();
        emoji_scroll.add_css_class("results-area");
        let emoji_name = gtk4::Label::builder().xalign(0.0).ellipsize(gtk4::pango::EllipsizeMode::End).build();
        emoji_name.add_css_class("emoji-name");
        let emoji_page = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        emoji_page.append(&emoji_scroll);
        emoji_page.append(&emoji_name);

        let output = gtk4::TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .monospace(true)
            .wrap_mode(gtk4::WrapMode::WordChar)
            .build();
        output.add_css_class("output-view");
        let output_scroll = gtk4::ScrolledWindow::builder().vexpand(true).child(&output).build();
        output_scroll.add_css_class("results-area");

        stack.add_named(&emoji_page, Some("emoji"));
        stack.add_named(&output_scroll, Some("output"));

        Extras {
            view: Cell::new(View::Root),
            stack,
            arg_box,
            form,
            base,
            args: RefCell::new(None),
            fields: RefCell::new(Vec::new()),
            form_cb: RefCell::new(None),
            emoji_flow,
            emoji_scroll,
            emoji_name,
            emoji_list: RefCell::new(Vec::new()),
            emoji_shown: Cell::new(0),
            output,
            scripts: RefCell::new(Vec::new()),
            quicklinks: RefCell::new(Quicklinks::load(store)),
            snippets: RefCell::new(Snippets::load(store)),
            windows: RefCell::new(Vec::new()),
        }
    }
}

impl SpotlightWindow {
    pub(super) fn setup_extras(self: &Rc<Self>) {
        self.rebuild_commands();
        self.setup_emoji();
    }

    pub(super) fn rebuild_commands(&self) {
        let mut v = self.x.base.clone();
        v.extend(ui_commands());
        if !self.x.windows.borrow().is_empty() {
            v.push(ui_cmd("ui:windows", "Switch Window", "Windows", &["window", "switch", "focus"], "preferences-system-windows-symbolic"));
        }
        v.extend(self.x.scripts.borrow().iter().map(script_commands::to_command));
        *self.commands.borrow_mut() = v;
    }

    /// Discover script commands and windows off the UI thread (called on every present).
    pub(super) fn refresh_extras(self: &Rc<Self>) {
        let s = self.clone();
        spawn_poll(
            || script_commands::discover(&[script_commands::default_dir()]),
            move |d| {
                for m in &d.skipped {
                    eprintln!("script command skipped: {m}");
                }
                if d.scripts != *s.x.scripts.borrow() {
                    *s.x.scripts.borrow_mut() = d.scripts;
                    s.rebuild_commands();
                    if s.window.is_visible() {
                        s.do_search(true);
                    }
                }
            },
        );
        let s = self.clone();
        spawn_poll(crate::windows_client::list_windows, move |w| {
            if w != *s.x.windows.borrow() {
                *s.x.windows.borrow_mut() = w;
                s.rebuild_commands();
                if s.window.is_visible() {
                    s.do_search(true);
                }
            }
        });
    }

    // ---- views ---------------------------------------------------------------------

    /// Back to the root list without searching (callers search afterwards).
    pub(super) fn leave_view(&self) {
        self.x.view.set(View::Root);
        self.x.stack.set_visible_child_name("list");
        self.clear_args();
    }

    pub(super) fn push_view(self: &Rc<Self>, v: View, placeholder: &str) {
        self.close_panel();
        self.clear_args();
        self.x.view.set(v);
        self.x.stack.set_visible_child_name(match v {
            View::Emoji => "emoji",
            View::Output => "output",
            _ => "list",
        });
        self.search_entry.set_placeholder_text(Some(placeholder));
        self.search_entry.set_text("");
        self.search_entry.grab_focus();
        if v != View::Output {
            self.do_search(false);
        }
    }

    pub(super) fn pop_view(self: &Rc<Self>) {
        self.leave_view();
        self.search_entry.set_placeholder_text(Some(self.mode_name()));
        self.search_entry.set_text("");
        self.search_entry.grab_focus();
        self.do_search(false);
    }

    /// Search inside a pushed view. Returns true when the view handled it.
    pub(super) fn search_view(self: &Rc<Self>, query: &str, keep: bool) -> bool {
        let now = now_secs();
        match self.x.view.get() {
            View::Root => return false,
            View::Output => {}
            View::Emoji => self.fill_emoji(query),
            View::Snippets => {
                let mut rows: Vec<RowData> = self.x.snippets.borrow().search(query).into_iter().map(snippet_row).collect();
                let mut c = Row::simple(RowKind::Command, "ui:snippet_create", "Create Snippet", "Snippets", "list-add-symbolic");
                c.actions = vec![ActionDesc { id: "run".into(), title: "Create Snippet".into(), argv: None }];
                rows.push(RowData { row: c, payload: None, calc: None });
                *self.current.borrow_mut() = only(rows, "Snippets");
                self.render(keep);
            }
            View::Windows => {
                let rows = wincore::rows(query, &self.x.windows.borrow(), &self.frecency.borrow(), now)
                    .into_iter()
                    .map(|row| RowData { row, payload: None, calc: None })
                    .collect();
                *self.current.borrow_mut() = only(rows, "Windows");
                self.render(keep);
            }
        }
        true
    }

    /// Quicklink, snippet-keyword and window rows for the root results of a typed query.
    pub(super) fn add_extra_rows(&self, query: &str, secs: &mut SectionList) {
        let mut top: Vec<RowData> = Vec::new();
        let q = query.trim();
        let uses_clip = self.x.quicklinks.borrow().items.iter().any(|l| l.url.contains("{clipboard}"));
        let clip = if uses_clip { self.clip_text() } else { String::new() };
        if let Some(r) = self.x.quicklinks.borrow().resolve_with_clipboard(q, &clip) {
            let mut row = Row::simple(RowKind::Fallback, "quicklink:open", &r.title, &r.url, "web-browser-symbolic");
            row.actions = vec![ActionDesc { id: "open_url".into(), title: "Open Link".into(), argv: Some(quicklinks::open_argv(&r.url)) }];
            top.push(RowData { row, payload: None, calc: None });
        }
        top.extend(
            self.x.snippets.borrow().search(q).into_iter().filter(|s| !s.keyword.is_empty() && s.keyword.eq_ignore_ascii_case(q)).map(snippet_row),
        );
        let wins: Vec<RowData> = wincore::rows(q, &self.x.windows.borrow(), &self.frecency.borrow(), now_secs())
            .into_iter()
            .take(5)
            .map(|row| RowData { row, payload: None, calc: None })
            .collect();
        if top.is_empty() && wins.is_empty() {
            return;
        }
        let pos = match secs.iter().position(|(t, _)| t == "Results") {
            Some(p) => p,
            None => {
                secs.insert(0, ("Results".to_string(), Vec::new()));
                0
            }
        };
        let rows = &mut secs[pos].1;
        rows.splice(0..0, top);
        rows.extend(wins);
    }

    // ---- actions ---------------------------------------------------------------------

    pub(super) fn extra_actions(&self, rd: &RowData) -> Option<Vec<UiAction>> {
        let id = rd.row.id.as_str();
        if id.starts_with("snippet:") {
            return Some(vec![
                ua("snippet_paste", "Paste Snippet", shortcuts::label(Act::Run(0)), false),
                ua("snippet_copy", "Copy Snippet", shortcuts::label(Act::Run(1)), false),
                ua("snippet_delete", "Delete Snippet", Vec::new(), true),
            ]);
        }
        if id.starts_with("window:") {
            return Some(vec![
                ua("window_activate", "Switch to Window", shortcuts::label(Act::Run(0)), false),
                ua("window_close", "Close Window", shortcuts::label(Act::Run(1)), true),
            ]);
        }
        None
    }

    /// Returns true when the action was handled here.
    pub(super) fn perform_extra(self: &Rc<Self>, rd: &RowData, action: &str) -> bool {
        let id = rd.row.id.as_str();
        if let Some(ch) = action.strip_prefix("emoji_paste:") {
            self.emoji_use(ch, false);
            return true;
        }
        if let Some(ch) = action.strip_prefix("emoji_copy:") {
            self.emoji_use(ch, true);
            return true;
        }
        let primary = matches!(action, "open" | "run" | "open_url");
        if id.starts_with("ui:") && primary {
            match id {
                "ui:snippets" => self.push_view(View::Snippets, "Search snippets\u{2026}"),
                "ui:emoji" => self.push_view(View::Emoji, "Search emoji\u{2026}"),
                "ui:windows" => self.push_view(View::Windows, "Switch window\u{2026}"),
                "ui:snippet_create" => self.snippet_form(),
                "ui:quicklink_create" => self.quicklink_form(),
                "ui:quicklink_manage" => self.quicklink_manage(),
                _ => return false,
            }
            return true;
        }
        if id.starts_with("script:") && primary {
            self.run_script(id);
            return true;
        }
        if id.starts_with("snippet:") {
            match action {
                "snippet_paste" | "open" | "run" => self.run_snippet(id, false),
                "snippet_copy" => self.run_snippet(id, true),
                "snippet_delete" => self.snippet_delete(id),
                _ => return false,
            }
            return true;
        }
        if let Some(wid) = WindowEntry::parse_row_id(id) {
            match action {
                "window_activate" | "open" | "run" => {
                    self.frecency.borrow_mut().record_launch(id, "", now_secs());
                    self.save_frecency();
                    self.window.set_visible(false);
                    std::thread::spawn(move || {
                        let _ = crate::windows_client::activate(wid);
                    });
                }
                "window_close" => {
                    let s = self.clone();
                    spawn_poll(
                        move || {
                            let _ = crate::windows_client::close(wid);
                            std::thread::sleep(Duration::from_millis(250));
                        },
                        move |_| s.refresh_extras(),
                    );
                    self.toast("Closing window", None);
                }
                _ => return false,
            }
            return true;
        }
        false
    }

}
