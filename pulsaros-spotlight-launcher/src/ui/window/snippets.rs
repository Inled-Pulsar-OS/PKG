//! Snippets, quicklinks and the form dialogs they share.
use super::*;
use crate::core::root_search::ActionDesc;
use crate::core::snippets::{self, Env, Snippet};

pub(super) enum FormField {
    Entry(gtk4::Entry),
    Text(gtk4::TextView),
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

pub(super) fn snippet_row(s: &Snippet) -> RowData {
    let first = s.text.lines().next().unwrap_or("").chars().take(80).collect::<String>();
    let sub = if s.keyword.is_empty() { first } else { format!("{}  \u{b7}  {first}", s.keyword) };
    let mut r = Row::simple(RowKind::Command, &format!("snippet:{}", s.id), &s.name, &sub, "insert-text-symbolic");
    r.breadcrumb = "Snippets".into();
    r.actions = vec![ActionDesc { id: "snippet_paste".into(), title: "Paste Snippet".into(), argv: None }];
    RowData { row: r, payload: None, calc: None }
}

impl SpotlightWindow {
    pub(super) fn clip_text(&self) -> String {
        self.backend_clipboard().and_then(|m| m.get_clip_by_index(0)).unwrap_or_default()
    }

    // ---- snippets --------------------------------------------------------------------

    pub(super) fn run_snippet(self: &Rc<Self>, id: &str, copy_only: bool) {
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

    pub(super) fn snippet_delete(self: &Rc<Self>, id: &str) {
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

    pub(super) fn form_reset(&self) {
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
    pub(super) fn show_list_dialog(self: &Rc<Self>, title: &str, empty: &str, items: Vec<(String, String)>, on_delete: Rc<dyn Fn(String)>) {
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
}
