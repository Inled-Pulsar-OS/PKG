//! Loading bar and toast pill.
use super::*;

impl SpotlightWindow {
    pub(super) fn loading_begin(self: &Rc<Self>) {
        let n = self.loading_count.get() + 1;
        self.loading_count.set(n);
        if n == 1 {
            self.loading_bar.set_opacity(1.0);
            let bar = self.loading_bar.clone();
            let id = glib::timeout_add_local(Duration::from_millis(50), move || {
                bar.pulse();
                glib::ControlFlow::Continue
            });
            *self.loading_timer.borrow_mut() = Some(id);
        }
    }

    pub(super) fn loading_end(&self) {
        let n = self.loading_count.get().saturating_sub(1);
        self.loading_count.set(n);
        if n == 0 {
            if let Some(id) = self.loading_timer.borrow_mut().take() {
                id.remove();
            }
            self.loading_bar.set_opacity(0.0);
            self.loading_bar.set_fraction(0.0);
        }
    }

    pub(super) fn toast(self: &Rc<Self>, msg: &str, undo: Option<Rc<dyn Fn()>>) {
        self.footer_status.set_label(msg);
        self.toast_pill.set_visible(true);
        self.toast_undo.set_visible(undo.is_some());
        *self.toast_cb.borrow_mut() = undo;
        if let Some(id) = self.toast_timer.borrow_mut().take() {
            id.remove();
        }
        let s = self.clone();
        let id = glib::timeout_add_local(Duration::from_millis(4000), move || {
            *s.toast_timer.borrow_mut() = None;
            s.reset_toast();
            glib::ControlFlow::Break
        });
        *self.toast_timer.borrow_mut() = Some(id);
    }

    pub(super) fn reset_toast(&self) {
        self.toast_pill.set_visible(false);
        self.toast_undo.set_visible(false);
        *self.toast_cb.borrow_mut() = None;
    }
}
