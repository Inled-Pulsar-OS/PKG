use crate::core::root_search::RowKind;
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

mod favorites;
mod icons;
mod model;
mod row;
mod thumbs;

pub use favorites::{get_favorites, set_favorites};
pub use icons::{app_icon, result_icon, warm_icon_cache};
pub use model::{kind_label, result_row, Item, Payload, RowData};
#[cfg(test)]
use thumbs::*;
use thumbs::{fnv1a64, has_thumbnail_support, spawn_thumbnail_pool, thumb_cache_dir, ThumbJob, ThumbPixels, THUMBNAIL_CACHE};

pub struct ResultView {
    list_box: gtk4::ListBox,
    scroll: gtk4::ScrolledWindow,
    items: Rc<RefCell<Vec<Item>>>,
    selected: Rc<Cell<Option<usize>>>,
    last_pointer: Rc<Cell<(f64, f64)>>,
    thumb_job_tx: RefCell<Option<mpsc::Sender<ThumbJob>>>,
    thumb_pictures: Rc<RefCell<HashMap<String, Vec<gtk4::Stack>>>>,
    thumb_poll: RefCell<Option<glib::SourceId>>,
}

impl ResultView {
    /// `on_activate` runs when a row is clicked; `on_context` on right click
    /// (the row is selected first).
    pub fn new<F, G, S>(on_activate: F, on_context: G, on_select: S) -> Self
    where
        F: Fn() + 'static,
        G: Fn() + 'static,
        S: Fn(Option<RowData>) + 'static,
    {
        let list_box = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::Single)
            .activate_on_single_click(true)
            .build();
        list_box.add_css_class("results-list");

        let scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .build();
        scroll.add_css_class("results-area");
        scroll.set_child(Some(&list_box));

        let view = Self {
            list_box,
            scroll,
            items: Rc::new(RefCell::new(Vec::new())),
            selected: Rc::new(Cell::new(None)),
            last_pointer: Rc::new(Cell::new((-1.0, -1.0))),
            thumb_job_tx: RefCell::new(None),
            thumb_pictures: Rc::new(RefCell::new(HashMap::new())),
            thumb_poll: RefCell::new(None),
        };

        let sel = view.selected.clone();
        let items = view.items.clone();
        let lb_clone = view.list_box.clone();
        view.list_box.connect_row_selected(move |_, row| {
            let mut child = lb_clone.first_child();
            while let Some(c) = child {
                c.remove_css_class("active-selection");
                if let Some(r) = c.downcast_ref::<gtk4::ListBoxRow>() {
                    if let Some(inner) = r.child() {
                        inner.remove_css_class("active-selection");
                    }
                }
                child = c.next_sibling();
            }
            if let Some(r) = row {
                r.add_css_class("active-selection");
                if let Some(inner) = r.child() {
                    inner.add_css_class("active-selection");
                }
            }
            let idx = row.map(|r| r.index() as usize);
            sel.set(idx);
            let data = idx.and_then(|i| match items.borrow().get(i) {
                Some(Item::Row(r)) => Some(r.clone()),
                _ => None,
            });
            on_select(data);
        });

        view.list_box.connect_row_activated(move |_, _| on_activate());

        let click = gtk4::GestureClick::builder().button(3).build();
        let lb = view.list_box.clone();
        click.connect_pressed(move |g, _, _, y| {
            g.set_state(gtk4::EventSequenceState::Claimed);
            if let Some(r) = lb.row_at_y(y as i32) {
                if r.is_selectable() {
                    lb.select_row(Some(&r));
                    on_context();
                }
            }
        });
        view.list_box.add_controller(click);

        // Hover selects only after real pointer motion: `motion` is not
        // emitted when rows scroll or are rebuilt under a still pointer.
        let motion = gtk4::EventControllerMotion::new();
        let lb = view.list_box.clone();
        let last = view.last_pointer.clone();
        motion.connect_motion(move |_, x, y| {
            let (lx, ly) = last.get();
            if (lx - x).abs() < 1.0 && (ly - y).abs() < 1.0 {
                return;
            }
            last.set((x, y));
            if let Some(r) = lb.row_at_y(y as i32) {
                if r.is_selectable() && !r.is_selected() {
                    lb.select_row(Some(&r));
                }
            }
        });
        view.list_box.add_controller(motion);

        let (job_tx, result_rx) = spawn_thumbnail_pool();
        *view.thumb_job_tx.borrow_mut() = Some(job_tx);
        view.start_thumbnail_poll(result_rx);
        view
    }

    pub fn widget(&self) -> &gtk4::ScrolledWindow {
        &self.scroll
    }

    pub fn selected_row(&self) -> Option<RowData> {
        let idx = self.selected.get()?;
        match self.items.borrow().get(idx) {
            Some(Item::Row(r)) => Some(r.clone()),
            _ => None,
        }
    }

    /// Replace the list. With `keep_selection` the previously selected row id
    /// stays selected when it is still present.
    pub fn set_sections(&self, sections: Vec<(String, Vec<RowData>)>, keep_selection: bool) {
        let prev_id = if keep_selection { self.selected_row().map(|r| r.row.id) } else { None };

        let mut items = Vec::new();
        for (title, rows) in sections {
            if rows.is_empty() {
                continue;
            }
            items.push(Item::Header(title));
            items.extend(rows.into_iter().take(300).map(Item::Row));
        }

        let same = {
            let old = self.items.borrow();
            old.len() == items.len()
                && old.iter().zip(items.iter()).all(|(a, b)| match (a, b) {
                    (Item::Header(x), Item::Header(y)) => x == y,
                    (Item::Row(x), Item::Row(y)) => x.row == y.row,
                    _ => false,
                })
        };
        if same {
            return;
        }

        self.thumb_pictures.borrow_mut().clear();
        self.selected.set(None);
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        let mut n = 0usize;
        for it in &items {
            match it {
                Item::Header(t) => {
                    let row = gtk4::ListBoxRow::new();
                    row.set_selectable(false);
                    row.set_activatable(false);
                    row.add_css_class("section-row");
                    let l = gtk4::Label::builder().label(t).xalign(0.0).build();
                    l.add_css_class("section-header");
                    row.set_child(Some(&l));
                    self.list_box.append(&row);
                }
                Item::Row(r) => {
                    let row = gtk4::ListBoxRow::new();
                    row.set_child(Some(&self.build_row(r, n)));
                    self.list_box.append(&row);
                    n += 1;
                }
            }
        }
        *self.items.borrow_mut() = items;

        let target = prev_id
            .and_then(|id| self.items.borrow().iter().position(|i| matches!(i, Item::Row(r) if r.row.id == id)))
            .or_else(|| self.items.borrow().iter().position(|i| matches!(i, Item::Row(_))));
        match target {
            Some(i) => self.select_index(i),
            None => self.scroll.vadjustment().set_value(0.0),
        }
        if !keep_selection {
            self.scroll.vadjustment().set_value(0.0);
        }
    }

    fn update_active_selection(&self, selected_row: &gtk4::ListBoxRow) {
        let mut child = self.list_box.first_child();
        while let Some(c) = child {
            c.remove_css_class("active-selection");
            if let Some(r) = c.downcast_ref::<gtk4::ListBoxRow>() {
                if let Some(inner) = r.child() {
                    inner.remove_css_class("active-selection");
                }
            }
            child = c.next_sibling();
        }
        selected_row.add_css_class("active-selection");
        if let Some(inner) = selected_row.child() {
            inner.add_css_class("active-selection");
        }
    }

    fn select_index(&self, idx: usize) {
        if let Some(row) = self.list_box.row_at_index(idx as i32) {
            self.list_box.select_row(Some(&row));
            self.update_active_selection(&row);
            self.ensure_visible(&row, idx);
        }
    }

    fn ensure_visible(&self, row: &gtk4::ListBoxRow, idx: usize) {
        let adj = self.scroll.vadjustment();
        // First row of a section: keep its header in view.
        let first_in_section = idx > 0 && matches!(self.items.borrow().get(idx - 1), Some(Item::Header(_)));
        // The row's bounds in list coordinates (`allocation` is deprecated since GTK 4.12).
        let Some(bounds) = row.compute_bounds(&self.list_box) else { return };
        let mut top = bounds.y() as f64;
        let bottom = top + bounds.height() as f64;
        if first_in_section {
            top = (top - 30.0).max(0.0);
        }
        let (value, page) = (adj.value(), adj.page_size());
        if top < value {
            adj.set_value(top);
        } else if bottom > value + page {
            adj.set_value(bottom - page);
        }
    }

    fn row_positions(&self) -> Vec<usize> {
        self.items
            .borrow()
            .iter()
            .enumerate()
            .filter(|(_, i)| matches!(i, Item::Row(_)))
            .map(|(p, _)| p)
            .collect()
    }

    /// Move the selection by `delta` rows (clamped). Returns false when it did not move.
    pub fn step(&self, delta: i32) -> bool {
        let pos = self.row_positions();
        if pos.is_empty() {
            return false;
        }
        let cur = self.selected.get().and_then(|s| pos.iter().position(|p| *p == s)).unwrap_or(0) as i32;
        let next = (cur + delta).clamp(0, pos.len() as i32 - 1) as usize;
        if next as i32 == cur {
            return false;
        }
        self.select_index(pos[next]);
        true
    }

    /// Jump to the first row of the previous / next section.
    pub fn jump_section(&self, forward: bool) {
        let items = self.items.borrow();
        let starts: Vec<usize> = items
            .iter()
            .enumerate()
            .filter(|(i, it)| matches!(it, Item::Row(_)) && *i > 0 && matches!(items[*i - 1], Item::Header(_)))
            .map(|(i, _)| i)
            .collect();
        drop(items);
        if starts.is_empty() {
            return;
        }
        let cur = self.selected.get().unwrap_or(0);
        let target = if forward {
            starts.iter().find(|s| **s > cur).copied()
        } else {
            starts.iter().rev().find(|s| **s < cur).copied()
        };
        if let Some(t) = target {
            self.select_index(t);
        }
    }

    pub fn select_nth_row(&self, n: usize) -> bool {
        match self.row_positions().get(n) {
            Some(p) => {
                self.select_index(*p);
                true
            }
            None => false,
        }
    }

    fn start_thumbnail_poll(&self, result_rx: mpsc::Receiver<(String, Option<ThumbPixels>)>) {
        let pictures = self.thumb_pictures.clone();
        let source_id = glib::timeout_add_local(Duration::from_millis(5), move || match result_rx.try_recv() {
            Ok((cache_key, pixels)) => {
                let cache = THUMBNAIL_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
                match pixels {
                    Some(px) => {
                        let pb = gdk_pixbuf::Pixbuf::from_mut_slice(
                            px.bytes.to_vec(),
                            gdk_pixbuf::Colorspace::Rgb,
                            px.has_alpha,
                            px.bits_per_sample,
                            px.width,
                            px.height,
                            px.rowstride,
                        );
                        let texture = gdk::Texture::for_pixbuf(&pb);
                        cache.lock().unwrap().insert(cache_key.clone(), Some(texture.clone()));
                        if let Some(stacks) = pictures.borrow().get(&cache_key) {
                            for stack in stacks {
                                stack.set_visible_child_name("thumb");
                                if let Some(widget) = stack.child_by_name("thumb") {
                                    if let Ok(pic) = widget.downcast::<gtk4::Picture>() {
                                        pic.set_paintable(Some(&texture));
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        cache.lock().unwrap().insert(cache_key, None);
                    }
                }
                glib::ControlFlow::Continue
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
        });
        *self.thumb_poll.borrow_mut() = Some(source_id);
    }

    // Sized cell: spinner until the thumbnail is ready, then the decoded texture.
    fn thumbnail_icon(&self, url: &str, pixel_size: i32) -> gtk4::Stack {
        let path = url.trim_start_matches("file://").to_string();
        let stack = gtk4::Stack::new();
        stack.set_size_request(pixel_size, pixel_size);
        stack.set_halign(gtk4::Align::Center);
        stack.set_valign(gtk4::Align::Center);
        stack.add_css_class("result-thumb");

        let spinner = gtk4::Spinner::new();
        spinner.set_size_request(16, 16);
        spinner.start();
        stack.add_named(&spinner, Some("loading"));

        let pic = gtk4::Picture::new();
        pic.set_size_request(pixel_size, pixel_size);
        pic.set_can_shrink(true);
        pic.add_css_class("result-thumb");
        stack.add_named(&pic, Some("thumb"));

        let cache_key = format!("{}@{}", path, pixel_size);
        let disk_png = thumb_cache_dir().join(format!("{}.png", fnv1a64(&cache_key)));

        let cache = THUMBNAIL_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Some(Some(texture)) = cache.lock().unwrap().get(&cache_key) {
            pic.set_paintable(Some(texture));
            stack.set_visible_child_name("thumb");
            return stack;
        }

        stack.set_visible_child_name("loading");
        self.thumb_pictures.borrow_mut().entry(cache_key.clone()).or_default().push(stack.clone());
        if let Some(job_tx) = self.thumb_job_tx.borrow().as_ref() {
            let _ = job_tx.send(ThumbJob { url: url.to_string(), cache_key, size: pixel_size, disk_png });
        }
        stack
    }
}

#[cfg(test)]
#[path = "../../tests/results_tests.rs"]
mod tests;

