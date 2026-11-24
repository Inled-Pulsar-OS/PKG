//! Argument pills shown beside the search entry for script commands and snippets.
use super::*;
use crate::core::script_commands::ArgType;
use crate::core::snippets::{self, Env};

pub(super) struct ArgField {
    pub(super) entry: gtk4::Entry,
    pub(super) required: bool,
}

pub(super) struct ArgSet {
    pub(super) id: String,
    pub(super) fields: Vec<ArgField>,
}

/// (name or placeholder, required, password, default)
type ArgSpec = (String, bool, bool, String);

impl SpotlightWindow {
    // ---- argument pills --------------------------------------------------------------

    pub(super) fn arg_specs_for(&self, rd: &RowData) -> Option<Vec<ArgSpec>> {
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

    pub(super) fn show_args(&self, id: &str, specs: Vec<ArgSpec>) {
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

    pub(super) fn clear_args(&self) {
        if self.x.args.borrow().is_none() {
            return;
        }
        while let Some(c) = self.x.arg_box.first_child() {
            self.x.arg_box.remove(&c);
        }
        self.x.arg_box.set_visible(false);
        *self.x.args.borrow_mut() = None;
    }

    pub(super) fn arg_values(&self, id: &str) -> Vec<String> {
        match self.x.args.borrow().as_ref() {
            Some(a) if a.id == id => a.fields.iter().map(|f| f.entry.text().to_string()).collect(),
            _ => Vec::new(),
        }
    }

    pub(super) fn arg_focus_index(&self) -> Option<usize> {
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
    pub(super) fn args_ready(&self, id: &str) -> bool {
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
}
