//! GTK wiring for script commands, quicklinks, snippets, emoji and the window switcher.
//! The logic lives in `crate::core`; this file only builds widgets, pushed views and worker threads.
use super::*;
use crate::core::emoji::{self, Emoji};
use crate::core::quicklinks::{self, Quicklinks};
use crate::core::root_search::ActionDesc;
use crate::core::script_commands::{self, ArgType, Outcome, Script, SystemRunner};
use crate::core::snippets::{self, Env, Snippet, Snippets};
use crate::core::windows::{self as wincore, WindowEntry};
use std::sync::mpsc;

const EMOJI_COLS: usize = 8;
const EMOJI_CHUNK: usize = 320;

/// Pushed views on top of the root list (Esc pops).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum View {
    Root,
    Snippets,
    Windows,
    Emoji,
    Output,
}

enum FormField {
    Entry(gtk4::Entry),
    Text(gtk4::TextView),
}

struct ArgField {
    entry: gtk4::Entry,
    required: bool,
}

struct ArgSet {
    id: String,
    fields: Vec<ArgField>,
}

/// (name or placeholder, required, password, default)
type ArgSpec = (String, bool, bool, String);

pub(super) struct Extras {
    pub view: Cell<View>,
    pub stack: gtk4::Stack,
    pub arg_box: gtk4::Box,
    pub form: gtk4::Box,
    pub base: Vec<Command>,
    args: RefCell<Option<ArgSet>>,
    fields: RefCell<Vec<FormField>>,
    form_cb: RefCell<Option<Box<dyn Fn(Vec<String>)>>>,
    emoji_flow: gtk4::FlowBox,
    emoji_scroll: gtk4::ScrolledWindow,
    emoji_name: gtk4::Label,
    emoji_list: RefCell<Vec<&'static Emoji>>,
    emoji_shown: Cell<usize>,
    output: gtk4::TextView,
    scripts: RefCell<Vec<Script>>,
    quicklinks: RefCell<Quicklinks>,
    snippets: RefCell<Snippets>,
    windows: RefCell<Vec<WindowEntry>>,
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
fn spawn_poll<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static, done: impl FnOnce(T) + 'static) {
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

fn utc_offset_secs() -> i64 {
    // SAFETY: localtime_r only writes into the zeroed tm we hand it.
    unsafe {
        let t: libc::time_t = now_secs() as libc::time_t;
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        tm.tm_gmtoff as i64
    }
}

fn ua(id: &str, title: &str, keys: Vec<String>, danger: bool) -> UiAction {
    UiAction { id: id.into(), title: title.into(), keys, danger, argv: None }
}

fn snippet_row(s: &Snippet) -> RowData {
    let first = s.text.lines().next().unwrap_or("").chars().take(80).collect::<String>();
    let sub = if s.keyword.is_empty() { first } else { format!("{}  \u{b7}  {first}", s.keyword) };
    let mut r = Row::simple(RowKind::Command, &format!("snippet:{}", s.id), &s.name, &sub, "insert-text-symbolic");
    r.breadcrumb = "Snippets".into();
    r.actions = vec![ActionDesc { id: "snippet_paste".into(), title: "Paste Snippet".into(), argv: None }];
    RowData { row: r, payload: None, calc: None }
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
        let s = self.clone();
        self.x.emoji_flow.connect_child_activated(move |_, c| s.emoji_paste_at(c.index() as usize, false));
        let s = self.clone();
        self.x.emoji_flow.connect_selected_children_changed(move |f| {
            if let Some(c) = f.selected_children().first() {
                let name = s.x.emoji_list.borrow().get(c.index() as usize).map(|e| format!("{}  {}", e.ch, e.name));
                s.x.emoji_name.set_label(&name.unwrap_or_default());
            }
        });
        let s = self.clone();
        self.x.emoji_scroll.connect_edge_reached(move |_, pos| {
            if pos == gtk4::PositionType::Bottom {
                s.emoji_more();
            }
        });
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

    fn clip_text(&self) -> String {
        self.backend_clipboard().and_then(|m| m.get_clip_by_index(0)).unwrap_or_default()
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

    // ---- script commands -------------------------------------------------------------

    fn run_script(self: &Rc<Self>, id: &str) {
        let Some(script) = self.x.scripts.borrow().iter().find(|s| format!("script:{}", s.rel) == id).cloned() else {
            self.toast("Script not found", None);
            return;
        };
        let mut values = Vec::new();
        if !script.header.arguments.is_empty() {
            if !self.args_ready(id) {
                return;
            }
            values = self.arg_values(id);
        }
        self.frecency.borrow_mut().record_launch(id, "", now_secs());
        self.save_frecency();
        self.loading_begin();
        let (s, title) = (self.clone(), script.header.title.clone());
        spawn_poll(
            move || script_commands::run(&script, &values, &SystemRunner),
            move |outcome| {
                s.loading_end();
                s.script_outcome(&title, outcome);
            },
        );
    }

    fn script_outcome(self: &Rc<Self>, title: &str, outcome: Outcome) {
        let shown = self.window.is_visible();
        match outcome {
            Outcome::Silent => self.window.set_visible(false),
            Outcome::Compact(l) | Outcome::Inline(l) => {
                let msg = if l.is_empty() { format!("{title}: done") } else { l };
                self.toast(&msg, None);
            }
            Outcome::FullOutput(text) => {
                self.x.output.buffer().set_text(&text);
                if !shown {
                    self.window.present();
                }
                self.push_view(View::Output, title);
            }
            Outcome::Failed(m) => {
                let msg = format!("{title} failed: {m}");
                if shown {
                    self.toast(&msg, None);
                } else {
                    let _ = std::process::Command::new("notify-send").arg("Spotlight").arg(&msg).spawn();
                }
            }
        }
    }

    // ---- argument pills --------------------------------------------------------------

    fn arg_specs_for(&self, rd: &RowData) -> Option<Vec<ArgSpec>> {
        let id = rd.row.id.as_str();
        if id.starts_with("script:") {
            let scripts = self.x.scripts.borrow();
            let s = scripts.iter().find(|s| format!("script:{}", s.rel) == id)?;
            let v: Vec<ArgSpec> = s
                .header
                .arguments
                .iter()
                .map(|a| {
                    let ph = if a.placeholder.is_empty() { "Argument".to_string() } else { a.placeholder.clone() };
                    let def = if a.kind == ArgType::Dropdown { a.data.first().map(|d| d.value.clone()).unwrap_or_default() } else { String::new() };
                    (ph, !a.optional, a.kind == ArgType::Password, def)
                })
                .collect();
            return (!v.is_empty()).then_some(v);
        }
        if let Some(sid) = id.strip_prefix("snippet:") {
            let snippets = self.x.snippets.borrow();
            let sn = snippets.items.iter().find(|s| s.id == sid)?;
            if !sn.text.contains("{argument") {
                return None;
            }
            let r = snippets::render(&sn.text, &Env::system(String::new(), 0));
            let v: Vec<ArgSpec> =
                r.arguments.into_iter().map(|a| (a.name, a.default.is_none(), false, a.default.unwrap_or_default())).collect();
            return (!v.is_empty()).then_some(v);
        }
        None
    }

    pub(super) fn on_row_selected(self: &Rc<Self>, rd: Option<RowData>) {
        match rd {
            Some(rd) => match self.arg_specs_for(&rd) {
                Some(specs) => self.show_args(&rd.row.id, specs),
                None => self.clear_args(),
            },
            None => {
                // The list rebuilds through a transient "no selection"; only hide when it stays empty.
                let s = self.clone();
                glib::idle_add_local_once(move || {
                    if s.result_view.selected_row().is_none() {
                        s.clear_args();
                    }
                });
            }
        }
    }

    fn show_args(&self, id: &str, specs: Vec<ArgSpec>) {
        if self.x.args.borrow().as_ref().is_some_and(|a| a.id == id) {
            return;
        }
        self.clear_args();
        let mut fields = Vec::new();
        for (name, required, password, default) in specs {
            let e = gtk4::Entry::new();
            e.add_css_class("arg-pill");
            e.set_width_chars(10);
            e.set_valign(gtk4::Align::Center);
            e.set_placeholder_text(Some(&if required { name.clone() } else { format!("{name} (optional)") }));
            e.set_text(&default);
            if password {
                e.set_visibility(false);
            }
            self.x.arg_box.append(&e);
            fields.push(ArgField { entry: e, required });
        }
        self.x.arg_box.set_visible(true);
        *self.x.args.borrow_mut() = Some(ArgSet { id: id.to_string(), fields });
    }

    fn clear_args(&self) {
        if self.x.args.borrow().is_none() {
            return;
        }
        while let Some(c) = self.x.arg_box.first_child() {
            self.x.arg_box.remove(&c);
        }
        self.x.arg_box.set_visible(false);
        *self.x.args.borrow_mut() = None;
    }

    fn arg_values(&self, id: &str) -> Vec<String> {
        match self.x.args.borrow().as_ref() {
            Some(a) if a.id == id => a.fields.iter().map(|f| f.entry.text().to_string()).collect(),
            _ => Vec::new(),
        }
    }

    fn arg_focus_index(&self) -> Option<usize> {
        let f = gtk4::prelude::RootExt::focus(&self.window)?;
        let args = self.x.args.borrow();
        args.as_ref()?.fields.iter().position(|a| {
            let w: &gtk4::Widget = a.entry.upcast_ref();
            f == *w || f.is_ancestor(w)
        })
    }

    pub(super) fn arg_has_focus(&self) -> bool {
        self.arg_focus_index().is_some()
    }

    /// True when the selected command may run; otherwise focuses the first required empty pill.
    fn args_ready(&self, id: &str) -> bool {
        let args = self.x.args.borrow();
        let Some(set) = args.as_ref().filter(|a| a.id == id) else { return true };
        match set.fields.iter().find(|f| f.required && f.entry.text().is_empty()) {
            Some(f) => {
                f.entry.grab_focus();
                false
            }
            None => true,
        }
    }

    /// Tab into the pills, Shift+Tab / Esc out. `None` lets the key through.
    pub(super) fn arg_key(&self, name: &str, mods: Mods) -> Option<glib::Propagation> {
        let args = self.x.args.borrow();
        let set = args.as_ref()?;
        let focus = self.arg_focus_index();
        let n = set.fields.len();
        let shift_tab = (name == "iso_left_tab") || (name == "tab" && mods.shift);
        let plain_tab = name == "tab" && !mods.shift && !mods.ctrl && !mods.alt;
        if plain_tab {
            let next = focus.map_or(0, |i| (i + 1).min(n - 1));
            set.fields[next].entry.grab_focus();
            return Some(glib::Propagation::Stop);
        }
        match (shift_tab, name, focus) {
            (true, _, Some(0)) | (_, "escape", Some(_)) => self.search_entry.grab_focus(),
            (true, _, Some(i)) => set.fields[i - 1].entry.grab_focus(),
            _ => return None,
        };
        Some(glib::Propagation::Stop)
    }

    // ---- snippets --------------------------------------------------------------------

    fn run_snippet(self: &Rc<Self>, id: &str, copy_only: bool) {
        let sid = id.strip_prefix("snippet:").unwrap_or(id);
        let Some(sn) = self.x.snippets.borrow().items.iter().find(|s| s.id == sid).cloned() else { return };
        let mut env = Env::system(self.clip_text(), utc_offset_secs());
        let probe = snippets::render(&sn.text, &env);
        if !probe.arguments.is_empty() {
            if !self.args_ready(id) {
                return;
            }
            for (spec, v) in probe.arguments.iter().zip(self.arg_values(id)) {
                let v = if v.is_empty() { spec.default.clone().unwrap_or_default() } else { v };
                env.args.insert(spec.name.clone(), v);
            }
        }
        // shortcut: `Rendered::cursor_offset` is ignored, the paste helper only sends Ctrl+V and cannot press Left.
        let text = snippets::render(&sn.text, &env).text;
        self.frecency.borrow_mut().record_launch(id, "", now_secs());
        self.save_frecency();
        self.window.set_visible(false);
        match (copy_only, self.backend_clipboard()) {
            (false, Some(mgr)) => mgr.paste_clip(&text),
            _ => self.copy(&text),
        }
    }

    fn snippet_delete(self: &Rc<Self>, id: &str) {
        let sid = id.strip_prefix("snippet:").unwrap_or(id);
        let removed = self.x.snippets.borrow_mut().remove(sid);
        if removed {
            if let Err(e) = self.x.snippets.borrow().save(&self.store) {
                self.toast(&format!("Could not save snippets: {e}"), None);
                return;
            }
            self.toast("Snippet deleted", None);
            self.do_search(true);
        }
    }

    pub(super) fn snippet_form(self: &Rc<Self>) {
        let s = self.clone();
        self.show_form(
            "Create Snippet",
            "Placeholders: {clipboard} {date} {time} {uuid} {cursor} {argument name=\"x\"}",
            &[("Name", "", false), ("Keyword (optional)", "", false), ("Text", "", true)],
            "Save",
            Box::new(move |v| {
                let res = s.x.snippets.borrow_mut().add(&v[0], &v[1], &v[2]);
                match res {
                    Ok(_) => match s.x.snippets.borrow().save(&s.store) {
                        Ok(()) => s.toast("Snippet saved", None),
                        Err(e) => s.toast(&format!("Could not save: {e}"), None),
                    },
                    Err(e) => s.toast(&e, None),
                }
                s.do_search(true);
            }),
        );
    }

    // ---- quicklinks ------------------------------------------------------------------

    pub(super) fn quicklink_form(self: &Rc<Self>) {
        let s = self.clone();
        self.show_form(
            "Create Quicklink",
            "Use {query} (and optionally {clipboard}) in the URL. Type the alias, then the query.",
            &[("Alias, e.g. gh", "", false), ("Name, e.g. GitHub search", "", false), ("URL, e.g. https://github.com/search?q={query}", "", false)],
            "Save",
            Box::new(move |v| {
                let res = s.x.quicklinks.borrow_mut().add(&v[0], &v[1], &v[2]);
                match res {
                    Ok(()) => match s.x.quicklinks.borrow().save(&s.store) {
                        Ok(()) => s.toast("Quicklink saved", None),
                        Err(e) => s.toast(&format!("Could not save: {e}"), None),
                    },
                    Err(e) => s.toast(&e, None),
                }
            }),
        );
    }

    pub(super) fn quicklink_manage(self: &Rc<Self>) {
        let items: Vec<(String, String)> = self
            .x
            .quicklinks
            .borrow()
            .items
            .iter()
            .map(|q| (q.alias.clone(), format!("{}  \u{b7}  {}", q.alias, if q.name.is_empty() { &q.url } else { &q.name })))
            .collect();
        let s = self.clone();
        self.show_list_dialog("Manage Quicklinks", "No quicklinks yet. Use Create Quicklink.", items, Rc::new(move |alias| {
            s.x.quicklinks.borrow_mut().remove(&alias);
            let _ = s.x.quicklinks.borrow().save(&s.store);
            s.quicklink_manage();
        }));
    }

    // ---- dialog forms ----------------------------------------------------------------

    fn form_reset(&self) {
        while let Some(c) = self.x.form.first_child() {
            self.x.form.remove(&c);
        }
        self.x.fields.borrow_mut().clear();
        *self.x.form_cb.borrow_mut() = None;
        self.x.form.set_visible(false);
    }

    /// Multi-field form in the dialog card. `fields` = (placeholder, initial, multiline).
    pub(super) fn show_form(self: &Rc<Self>, title: &str, body: &str, fields: &[(&str, &str, bool)], ok_label: &str, on_ok: Box<dyn Fn(Vec<String>)>) {
        self.show_dialog(title, body, None, ok_label, false, Box::new(|_| {}));
        self.form_reset();
        let n = fields.len();
        for (i, (ph, init, multi)) in fields.iter().enumerate() {
            if *multi {
                let tv = gtk4::TextView::builder().wrap_mode(gtk4::WrapMode::WordChar).build();
                tv.add_css_class("form-text");
                tv.buffer().set_text(init);
                let sw = gtk4::ScrolledWindow::builder().min_content_height(96).child(&tv).build();
                sw.add_css_class("form-text-wrap");
                self.x.form.append(&sw);
                self.x.fields.borrow_mut().push(FormField::Text(tv));
            } else {
                let e = gtk4::Entry::builder().placeholder_text(*ph).text(*init).build();
                let s = self.clone();
                e.connect_activate(move |_| {
                    if i + 1 == n {
                        s.accept_dialog();
                    } else if let Some(FormField::Entry(next)) = s.x.fields.borrow().get(i + 1) {
                        next.grab_focus();
                    } else if let Some(FormField::Text(next)) = s.x.fields.borrow().get(i + 1) {
                        next.grab_focus();
                    }
                });
                self.x.form.append(&e);
                self.x.fields.borrow_mut().push(FormField::Entry(e));
            }
        }
        *self.x.form_cb.borrow_mut() = Some(on_ok);
        self.x.form.set_visible(true);
        if let Some(FormField::Entry(e)) = self.x.fields.borrow().first() {
            e.grab_focus();
        }
    }

    /// Simple list with a delete button per row (reuses the dialog card). `items` = (key, label).
    fn show_list_dialog(self: &Rc<Self>, title: &str, empty: &str, items: Vec<(String, String)>, on_delete: Rc<dyn Fn(String)>) {
        self.show_dialog(title, if items.is_empty() { empty } else { "" }, None, "Done", false, Box::new(|_| {}));
        self.form_reset();
        for (key, label) in items {
            let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            let l = gtk4::Label::builder().label(&label).xalign(0.0).hexpand(true).ellipsize(gtk4::pango::EllipsizeMode::End).max_width_chars(44).build();
            let b = gtk4::Button::with_label("Delete");
            b.add_css_class("destructive");
            let cb = on_delete.clone();
            b.connect_clicked(move |_| cb(key.clone()));
            row.append(&l);
            row.append(&b);
            self.x.form.append(&row);
        }
        *self.x.form_cb.borrow_mut() = Some(Box::new(|_| {}));
        self.x.form.set_visible(true);
    }

    /// Called by `accept_dialog`: true when a form consumed the accept.
    pub(super) fn accept_form(self: &Rc<Self>) -> bool {
        let cb = self.x.form_cb.borrow_mut().take();
        let Some(cb) = cb else { return false };
        let values: Vec<String> = self
            .x
            .fields
            .borrow()
            .iter()
            .map(|f| match f {
                FormField::Entry(e) => e.text().trim().to_string(),
                FormField::Text(t) => {
                    let b = t.buffer();
                    b.text(&b.start_iter(), &b.end_iter(), false).to_string()
                }
            })
            .collect();
        self.close_dialog();
        cb(values);
        true
    }

    pub(super) fn dialog_reset_form(&self) {
        self.form_reset();
    }

    // ---- emoji -----------------------------------------------------------------------

    fn fill_emoji(&self, query: &str) {
        let list = emoji::search(query, &self.frecency.borrow(), now_secs());
        while let Some(c) = self.x.emoji_flow.first_child() {
            self.x.emoji_flow.remove(&c);
        }
        *self.x.emoji_list.borrow_mut() = list;
        self.x.emoji_shown.set(0);
        self.x.emoji_scroll.vadjustment().set_value(0.0);
        self.emoji_more();
        self.emoji_select(0);
    }

    fn emoji_more(&self) {
        let list = self.x.emoji_list.borrow();
        let from = self.x.emoji_shown.get();
        let to = (from + EMOJI_CHUNK).min(list.len());
        for e in &list[from..to] {
            let l = gtk4::Label::new(Some(&e.ch));
            l.add_css_class("emoji-cell");
            self.x.emoji_flow.insert(&l, -1);
        }
        self.x.emoji_shown.set(to);
    }

    fn emoji_selected(&self) -> Option<usize> {
        self.x.emoji_flow.selected_children().first().map(|c| c.index() as usize)
    }

    fn emoji_select(&self, idx: usize) {
        let Some(child) = self.x.emoji_flow.child_at_index(idx as i32) else {
            self.x.emoji_name.set_label("");
            return;
        };
        self.x.emoji_flow.select_child(&child);
        if let Some(b) = child.compute_bounds(&self.x.emoji_flow) {
            let adj = self.x.emoji_scroll.vadjustment();
            let (top, bottom) = (b.y() as f64, (b.y() + b.height()) as f64);
            if top < adj.value() {
                adj.set_value(top);
            } else if bottom > adj.value() + adj.page_size() {
                adj.set_value(bottom - adj.page_size());
            }
        }
    }

    fn emoji_move(&self, delta: i32) {
        let total = self.x.emoji_list.borrow().len() as i32;
        if total == 0 {
            return;
        }
        let cur = self.emoji_selected().unwrap_or(0) as i32;
        let next = (cur + delta).clamp(0, total - 1);
        while next as usize >= self.x.emoji_shown.get() && self.x.emoji_shown.get() < total as usize {
            self.emoji_more();
        }
        self.emoji_select(next as usize);
    }

    fn emoji_paste_at(&self, idx: usize, copy_only: bool) {
        let ch = self.x.emoji_list.borrow().get(idx).map(|e| e.ch.clone());
        if let Some(ch) = ch {
            self.emoji_use(&ch, copy_only);
        }
    }

    fn emoji_use(&self, ch: &str, copy_only: bool) {
        let q = self.search_entry.text().to_string();
        emoji::record_use(&mut self.frecency.borrow_mut(), ch, &q, now_secs());
        self.save_frecency();
        self.window.set_visible(false);
        match (copy_only, self.backend_clipboard()) {
            (false, Some(mgr)) => mgr.paste_clip(ch),
            _ => self.copy(ch),
        }
    }

    pub(super) fn open_emoji_panel(self: &Rc<Self>) {
        let Some(idx) = self.emoji_selected() else { return };
        let Some(e) = self.x.emoji_list.borrow().get(idx).copied() else { return };
        let mut acts = vec![
            ua(&format!("emoji_paste:{}", e.ch), &format!("Paste {}", e.ch), shortcuts::label(Act::Run(0)), false),
            ua(&format!("emoji_copy:{}", e.ch), &format!("Copy {}", e.ch), Vec::new(), false),
        ];
        for v in &e.variants {
            acts.push(ua(&format!("emoji_paste:{v}"), &format!("Paste {v}  skin tone"), Vec::new(), false));
        }
        *self.panel.all.borrow_mut() = acts;
        let row = Row::simple(RowKind::Command, &format!("emoji:{}", e.ch), &e.name, "", "face-smile-symbolic");
        *self.panel.target.borrow_mut() = Some(RowData { row, payload: None, calc: None });
        self.panel.entry.set_text("");
        self.fill_panel("");
        self.panel.root.set_visible(true);
        self.panel.entry.grab_focus();
    }

    /// Grid navigation and Enter while the emoji view is up. `None` lets the key through.
    pub(super) fn emoji_key(&self, name: &str, mods: Mods) -> Option<glib::Propagation> {
        if self.x.view.get() != View::Emoji || mods.ctrl || mods.alt {
            return None;
        }
        match name {
            "left" => self.emoji_move(-1),
            "right" => self.emoji_move(1),
            "up" => self.emoji_move(-(EMOJI_COLS as i32)),
            "down" => self.emoji_move(EMOJI_COLS as i32),
            "return" | "kp_enter" => {
                if let Some(i) = self.emoji_selected() {
                    self.emoji_paste_at(i, false);
                }
            }
            _ => return None,
        }
        Some(glib::Propagation::Stop)
    }
}
