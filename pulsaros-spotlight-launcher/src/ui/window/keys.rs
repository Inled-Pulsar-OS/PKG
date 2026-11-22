//! Keyboard and pointer handling, including bar-mode keys.
use super::*;

impl SpotlightWindow {
    pub(super) fn on_key_pressed(self: &Rc<Self>, keyval: gdk::Key, state: gdk::ModifierType) -> glib::Propagation {
        let name = keyval.to_lower().name().map(|n| n.to_lowercase()).unwrap_or_default();
        let mods = Mods {
            ctrl: state.contains(gdk::ModifierType::CONTROL_MASK),
            shift: state.contains(gdk::ModifierType::SHIFT_MASK),
            alt: state.contains(gdk::ModifierType::ALT_MASK),
        };
        let act = shortcuts::lookup(&name, mods);

        if self.dialog_open() {
            if act == Some(Act::Back) || keyval == gdk::Key::Escape {
                self.close_dialog();
                return glib::Propagation::Stop;
            }
            if (keyval == gdk::Key::Return || keyval == gdk::Key::KP_Enter) && !mods.ctrl && !mods.alt && !mods.shift {
                self.accept_dialog();
                return glib::Propagation::Stop;
            }
            return glib::Propagation::Proceed;
        }

        if self.panel_open() {
            match act {
                Some(Act::Up) => self.panel_step(-1),
                Some(Act::Down) => self.panel_step(1),
                Some(Act::Run(_)) => self.panel_activate(),
                Some(Act::Back) | Some(Act::ActionPanel) => self.close_panel(),
                _ => return glib::Propagation::Proceed,
            }
            return glib::Propagation::Stop;
        }

        if self.layout.get() != LayoutState::Panel {
            if let Some(p) = self.bar_key(keyval, mods, act) {
                return p;
            }
        }

        if let Some(p) = self.emoji_key(&name, mods) {
            return p;
        }
        if let Some(p) = self.arg_key(&name, mods) {
            return p;
        }

        // Enter on result: immediately run action index 0
        if (keyval == gdk::Key::Return || keyval == gdk::Key::KP_Enter || keyval == gdk::Key::ISO_Enter)
            && !mods.ctrl && !mods.alt && !mods.shift
        {
            if self.result_view.selected_row().is_some() {
                self.run_action_index(0);
                return glib::Propagation::Stop;
            }
        }

        // Down / Up arrows
        if keyval == gdk::Key::Down && !mods.ctrl && !mods.alt && !mods.shift {
            self.result_view.step(1);
            return glib::Propagation::Stop;
        }
        if keyval == gdk::Key::Up && !mods.ctrl && !mods.alt && !mods.shift {
            if !self.result_view.step(-1)
                && self.search_entry.text().is_empty()
                && self.nav.borrow().is_root()
            {
                self.show_history();
            }
            return glib::Propagation::Stop;
        }

        // Left / Right arrows when search entry is empty: cycle category tabs!
        if keyval == gdk::Key::Left && self.search_entry.text().is_empty() && !mods.ctrl && !mods.alt {
            self.cycle_category(-1);
            return glib::Propagation::Stop;
        }
        if keyval == gdk::Key::Right && self.search_entry.text().is_empty() && !mods.ctrl && !mods.alt {
            self.cycle_category(1);
            return glib::Propagation::Stop;
        }

        if let Some(act) = act {
            return self.handle_act(act);
        }

        // Backspace on empty: ascend directory
        if keyval == gdk::Key::BackSpace && self.search_entry.text().is_empty() {
            let before = self.nav.borrow().dir().map(str::to_string);
            self.nav_apply(NavEvent::ParentDir);
            let after = self.nav.borrow().dir().map(str::to_string);
            if let Some(parent) = after.filter(|p| Some(p) != before.as_ref()) {
                self.search_entry.set_placeholder_text(Some(&format!("Browsing: {}", parent)));
                self.do_search(false);
                return glib::Propagation::Stop;
            }
        }

        // Typing while a list row has focus: redirect into the search entry
        if !self.search_entry.has_focus() && !self.arg_has_focus() && !mods.ctrl && !mods.alt && !state.contains(gdk::ModifierType::SUPER_MASK) {
            if let Some(ch) = keyval.to_unicode() {
                if ch.is_alphanumeric() || ch.is_ascii_punctuation() || ch == ' ' {
                    self.search_entry.grab_focus();
                    let mut text = self.search_entry.text().to_string();
                    text.push(ch);
                    self.search_entry.set_text(&text);
                    self.search_entry.set_position(-1);
                    return glib::Propagation::Stop;
                }
            } else if keyval == gdk::Key::BackSpace {
                self.search_entry.grab_focus();
                let mut text = self.search_entry.text().to_string();
                text.pop();
                self.search_entry.set_text(&text);
                self.search_entry.set_position(-1);
                return glib::Propagation::Stop;
            }
        }

        glib::Propagation::Proceed
    }

    pub(super) fn layout_ctx(&self) -> LayoutCtx {
        LayoutCtx { revealed: self.revealed.get(), pointer_origin: self.pointer_origin.get(), ..LayoutCtx::default() }
    }

    pub(super) fn on_pointer(&self, x: f64, y: f64) {
        let o = layout::transition(self.layout.get(), &self.layout_ctx(), LayoutEvent::Pointer { x, y });
        if o.action == LayoutAction::RecordPointer {
            self.pointer_origin.set(Some((x, y)));
        }
        if o.state != self.layout.get() {
            self.apply_layout(o.state);
        }
    }

    /// Open the Panel in mode `n` (1-based, as Alt+n) with an empty query.
    pub(super) fn open_bar_mode(self: &Rc<Self>, n: usize) {
        let Some(cat) = shortcuts::mode_id(n).and_then(Category::from_id) else { return };
        self.apply_layout(LayoutState::Panel);
        self.set_category(cat);
        self.search_entry.grab_focus();
    }

    /// Keys in a bar state (Idle, IdleModes): expand to the Panel as the layout module says.
    /// Left and Right do nothing there. `None` leaves the key to the normal handling.
    fn bar_key(self: &Rc<Self>, keyval: gdk::Key, mods: Mods, act: Option<Act>) -> Option<glib::Propagation> {
        if matches!(keyval, gdk::Key::Left | gdk::Key::Right) && !mods.ctrl && !mods.alt {
            return Some(glib::Propagation::Stop);
        }
        if let Some(step @ (Act::CatNext | Act::CatPrev)) = act {
            let o = layout::transition(self.layout.get(), &self.layout_ctx(), LayoutEvent::Tab);
            if o.state != self.layout.get() {
                self.apply_layout(o.state);
            } else {
                self.apply_layout(LayoutState::Panel);
                self.cycle_category(if step == Act::CatNext { 1 } else { -1 });
            }
            return Some(glib::Propagation::Stop);
        }
        let event = bar_event(keyval, mods, act)?;
        match layout::transition(self.layout.get(), &self.layout_ctx(), event).action {
            LayoutAction::ShowHome => {
                self.apply_layout(LayoutState::Panel);
                self.nav_apply(NavEvent::PanelOpened(Opened::Home));
                self.do_search(false);
            }
            LayoutAction::ShowHistory => {
                self.apply_layout(LayoutState::Panel);
                self.nav_apply(NavEvent::PanelOpened(Opened::History));
                if self.settings.borrow().history().is_empty() {
                    self.do_search(false);
                } else {
                    self.show_history();
                }
            }
            LayoutAction::SwitchMode(_) => {
                let Some(Act::Mode(n)) = act else { return None };
                self.open_bar_mode(n);
            }
            _ => return None,
        }
        Some(glib::Propagation::Stop)
    }

    fn handle_act(self: &Rc<Self>, act: Act) -> glib::Propagation {
        let stop = glib::Propagation::Stop;
        match act {
            Act::ActionPanel => self.open_more_menu(),
            Act::Run(n) => {
                if self.result_view.selected_row().is_none() {
                    return glib::Propagation::Proceed;
                }
                self.run_action_index(n);
            }
            Act::Quick(n) => {
                if self.result_view.select_nth_row(n) {
                    self.run_action_index(0);
                }
            }
            Act::Up => {
                if !self.result_view.step(-1)
                    && self.search_entry.text().is_empty()
                    && self.nav.borrow().is_root()
                {
                    self.show_history();
                }
            }
            Act::Down => {
                self.result_view.step(1);
            }
            Act::PageUp => {
                self.result_view.step(-8);
            }
            Act::PageDown => {
                self.result_view.step(8);
            }
            Act::SectionPrev => self.result_view.jump_section(false),
            Act::SectionNext => self.result_view.jump_section(true),
            Act::Back => self.go_back(),
            Act::Root => {
                // Collapse and the old reset both land on the bar: reset_to_root follows the revealed flag.
                self.reset_to_root();
                self.search_entry.grab_focus();
            }
            Act::Favorite => self.perform_selected("favorite"),
            Act::Disable => self.perform_selected("disable"),
            Act::CopyFull => self.perform_selected("calc_full"),
            Act::DelItem => self.perform_selected("calc_del"),
            Act::DelAll => self.perform_selected("calc_delall"),
            Act::FavUp | Act::FavDown => {
                if let Some(rd) = self.result_view.selected_row() {
                    if rd.row.favorite {
                        self.settings.borrow_mut().move_favorite(&rd.row.id, if act == Act::FavUp { -1 } else { 1 });
                        self.save_settings();
                        self.do_search(true);
                    }
                }
            }
            Act::ZoomIn => self.set_zoom(self.config.borrow().zoom + 0.1),
            Act::ZoomOut => self.set_zoom(self.config.borrow().zoom - 0.1),
            Act::ZoomReset => self.set_zoom(1.0),
            Act::Quit => {
                if let Some(app) = self.window.application() {
                    app.quit();
                }
            }
            Act::CatNext => self.cycle_category(1),
            Act::CatPrev => self.cycle_category(-1),
            Act::Mode(n) => {
                if let Some(cat) = shortcuts::mode_id(n).and_then(Category::from_id) {
                    self.set_category(cat);
                }
            }
        }
        stop
    }
}

/// Layout event for a key pressed in a bar state: Down, Up, or Alt+1..4.
pub(super) fn bar_event(keyval: gdk::Key, mods: Mods, act: Option<Act>) -> Option<LayoutEvent> {
    if let Some(Act::Mode(n)) = act {
        let mode = [BarMode::Applications, BarMode::Files, BarMode::Actions, BarMode::Clipboard].get(n.checked_sub(1)?)?;
        return Some(LayoutEvent::Mode(*mode));
    }
    if mods.ctrl || mods.alt || mods.shift {
        return None;
    }
    match keyval {
        gdk::Key::Down => Some(LayoutEvent::Down),
        gdk::Key::Up => Some(LayoutEvent::Up),
        _ => None,
    }
}
