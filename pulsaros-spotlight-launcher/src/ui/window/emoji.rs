//! Emoji picker: grid, selection, paste and copy.
use super::views::{ua, View};
use super::*;
use crate::core::emoji;

pub(super) const EMOJI_COLS: usize = 8;
const EMOJI_CHUNK: usize = 320;

impl SpotlightWindow {
    pub(super) fn setup_emoji(self: &Rc<Self>) {
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

    // ---- emoji -----------------------------------------------------------------------

    pub(super) fn fill_emoji(&self, query: &str) {
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

    pub(super) fn emoji_more(&self) {
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

    pub(super) fn emoji_selected(&self) -> Option<usize> {
        self.x.emoji_flow.selected_children().first().map(|c| c.index() as usize)
    }

    pub(super) fn emoji_select(&self, idx: usize) {
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

    pub(super) fn emoji_move(&self, delta: i32) {
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

    pub(super) fn emoji_paste_at(&self, idx: usize, copy_only: bool) {
        let ch = self.x.emoji_list.borrow().get(idx).map(|e| e.ch.clone());
        if let Some(ch) = ch {
            self.emoji_use(&ch, copy_only);
        }
    }

    pub(super) fn emoji_use(&self, ch: &str, copy_only: bool) {
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
