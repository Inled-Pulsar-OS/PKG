//! Modal confirm/prompt dialog.
use super::*;

pub(super) struct Dialog {
    pub(super) root: gtk4::Box,
    pub(super) title: gtk4::Label,
    pub(super) body: gtk4::Label,
    pub(super) entry: gtk4::Entry,
    pub(super) ok: gtk4::Button,
    pub(super) on_ok: RefCell<Option<Box<dyn Fn(String)>>>,
    pub(super) backdrop: gtk4::Box,
}

impl SpotlightWindow {
    pub(super) fn show_dialog(
        self: &Rc<Self>,
        title: &str,
        body: &str,
        entry: Option<String>,
        ok_label: &str,
        destructive: bool,
        on_ok: Box<dyn Fn(String)>,
    ) {
        self.close_panel();
        self.dialog_reset_form();
        self.dialog.title.set_label(title);
        self.dialog.body.set_label(body);
        self.dialog.ok.set_label(ok_label);
        if destructive {
            self.dialog.ok.add_css_class("destructive");
        } else {
            self.dialog.ok.remove_css_class("destructive");
        }
        match entry {
            Some(text) => {
                self.dialog.entry.set_text(&text);
                self.dialog.entry.set_visible(true);
            }
            None => self.dialog.entry.set_visible(false),
        }
        *self.dialog.on_ok.borrow_mut() = Some(on_ok);
        self.x.stack.set_visible(false);
        self.dialog.backdrop.set_visible(true);
        self.dialog.root.set_visible(true);
        if gtk4::prelude::WidgetExt::is_visible(&self.dialog.entry) {
            self.dialog.entry.grab_focus();
            self.dialog.entry.set_position(-1);
        } else {
            self.dialog.ok.grab_focus();
        }
    }

    pub(super) fn dialog_open(&self) -> bool {
        self.dialog.root.is_visible()
    }

    pub(super) fn close_dialog(&self) {
        if self.dialog.root.is_visible() {
            self.dialog.backdrop.set_visible(false);
            self.dialog.root.set_visible(false);
            self.x.stack.set_visible(true);
            self.dialog_reset_form();
            *self.dialog.on_ok.borrow_mut() = None;
            self.search_entry.grab_focus();
        }
    }

    pub(super) fn accept_dialog(self: &Rc<Self>) {
        if self.accept_form() {
            return;
        }
        let text = self.dialog.entry.text().to_string();
        let cb = self.dialog.on_ok.borrow_mut().take();
        self.close_dialog();
        if let Some(cb) = cb {
            cb(text);
        }
    }
}
