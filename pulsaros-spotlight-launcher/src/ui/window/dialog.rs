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

impl Dialog {
    /// The confirm/prompt dialog card and its backdrop, hidden until shown.
    /// Also returns the Cancel button and the backdrop click gesture for the caller to wire.
    pub(super) fn build(form: &gtk4::Box) -> (Dialog, gtk4::Button, gtk4::GestureClick) {
            let dialog_root = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
            dialog_root.add_css_class("dialog-card");
            dialog_root.set_halign(gtk4::Align::Center);
            dialog_root.set_valign(gtk4::Align::Center);
            dialog_root.set_visible(false);
            let dialog_title = gtk4::Label::builder().xalign(0.0).wrap(true).build();
            dialog_title.add_css_class("dialog-title");
            let dialog_body = gtk4::Label::builder().xalign(0.0).wrap(true).build();
            dialog_body.add_css_class("dialog-body");
            let dialog_entry = gtk4::Entry::new();
            let dialog_buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            dialog_buttons.set_halign(gtk4::Align::End);
            let dialog_cancel = gtk4::Button::with_label("Cancel");
            dialog_cancel.add_css_class("dialog-cancel-btn");
            let dialog_ok = gtk4::Button::with_label("OK");
            dialog_ok.add_css_class("dialog-ok-btn");
            dialog_ok.add_css_class("suggested-action");
            dialog_buttons.append(&dialog_cancel);
            dialog_buttons.append(&dialog_ok);
            dialog_root.append(&dialog_title);
            dialog_root.append(&dialog_body);
            dialog_root.append(form);
            dialog_root.append(&dialog_entry);
            dialog_root.append(&dialog_buttons);

            let dialog_backdrop = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
            dialog_backdrop.add_css_class("dialog-backdrop");
            dialog_backdrop.set_hexpand(true);
            dialog_backdrop.set_vexpand(true);
            dialog_backdrop.set_visible(false);

            let click_backdrop = gtk4::GestureClick::new();
            dialog_backdrop.add_controller(click_backdrop.clone());
        let dialog = Dialog {
            root: dialog_root,
            title: dialog_title,
            body: dialog_body,
            entry: dialog_entry,
            ok: dialog_ok,
            on_ok: RefCell::new(None),
            backdrop: dialog_backdrop,
        };
        (dialog, dialog_cancel, click_backdrop)
    }
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
