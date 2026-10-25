use crate::core::root_search::{Row, RowKind};
use crate::search::SearchResult;
use crate::utils::get_file_icon;
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::Duration;

static ICON_CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
static THUMBNAIL_CACHE: OnceLock<Mutex<HashMap<String, Option<gtk4::gdk::Texture>>>> = OnceLock::new();

struct ThumbPixels {
    bytes: glib::Bytes,
    width: i32,
    height: i32,
    rowstride: i32,
    has_alpha: bool,
    bits_per_sample: i32,
}

struct ThumbJob {
    url: String,
    cache_key: String,
    size: i32,
    disk_png: std::path::PathBuf,
}

fn fnv1a64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

// Creates the worker pool and returns (job sender, result receiver).
fn spawn_thumbnail_pool() -> (
    mpsc::Sender<ThumbJob>,
    mpsc::Receiver<(String, Option<ThumbPixels>)>,
) {
    const WORKERS: usize = 4;
    let (job_tx, job_rx) = mpsc::channel::<ThumbJob>();
    let (result_tx, result_rx) = mpsc::channel::<(String, Option<ThumbPixels>)>();

    let worker_txs: Vec<mpsc::Sender<ThumbJob>> = (0..WORKERS)
        .map(|_| {
            let (wtx, wrx) = mpsc::channel::<ThumbJob>();
            let rt = result_tx.clone();
            std::thread::spawn(move || {
                while let Ok(job) = wrx.recv() {
                    let pixels = generate_thumb_pixels(&job);
                    let _ = rt.send((job.cache_key, pixels));
                }
            });
            wtx
        })
        .collect();

    std::thread::spawn(move || {
        let mut i = 0usize;
        while let Ok(job) = job_rx.recv() {
            let _ = worker_txs[i % WORKERS].send(job);
            i += 1;
        }
    });

    (job_tx, result_rx)
}

fn get_icon_cache() -> &'static Mutex<HashMap<String, Option<String>>> {
    ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn warm_icon_cache() {
    const EXTS: [&str; 3] = ["png", "svg", "xpm"];
    let cache = get_icon_cache();
    {
        let map = cache.lock().unwrap();
        if !map.is_empty() {
            return;
        }
    }

    // Build map locally on the background thread without holding the Mutex lock
    let mut local_map = HashMap::new();

    for root in ["/usr/share/icons", "/usr/local/share/icons", "/usr/share/pixmaps", "/usr/local/share/pixmaps"] {
        let Ok(entries) = std::fs::read_dir(root) else { continue };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                let Ok(subdirs) = std::fs::read_dir(&path) else { continue };
                for subdir in subdirs.filter_map(Result::ok) {
                    let subpath = subdir.path();
                    if subpath.is_dir() {
                        let apps_dir = subpath.join("apps");
                        if let Ok(app_entries) = std::fs::read_dir(&apps_dir) {
                            for app_entry in app_entries.filter_map(Result::ok) {
                                let name_os = app_entry.file_name();
                                let name_str = name_os.to_string_lossy();
                                for ext in EXTS {
                                    if let Some(stem) = name_str.strip_suffix(&format!(".{ext}")) {
                                        local_map.entry(stem.to_string()).or_insert_with(|| Some(app_entry.path().to_string_lossy().into_owned()));
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            } else if path.is_file() {
                let name_os = entry.file_name();
                let name_str = name_os.to_string_lossy();
                for ext in EXTS {
                    if let Some(stem) = name_str.strip_suffix(&format!(".{ext}")) {
                        local_map.entry(stem.to_string()).or_insert_with(|| Some(path.to_string_lossy().into_owned()));
                        break;
                    }
                }
            }
        }
    }

    let mut global_map = cache.lock().unwrap();
    for (k, v) in local_map {
        global_map.entry(k).or_insert(v);
    }
}

fn find_icon_file_cached(name: &str) -> Option<String> {
    let cache = get_icon_cache();
    if let Some(hit) = cache.lock().unwrap().get(name) {
        return hit.clone();
    }
    // Never do synchronous directory crawling on the main GTK UI thread
    None
}

fn is_image_file(url: &str) -> bool {
    let path = url.trim_start_matches("file://");
    let ext = Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "avif" | "tiff" | "ico")
}

fn is_pdf_file(url: &str) -> bool {
    url.trim_start_matches("file://").ends_with(".pdf")
}

fn is_video_file(url: &str) -> bool {
    let ext = Path::new(url.trim_start_matches("file://"))
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(ext.as_str(), "mp4" | "mkv" | "webm" | "avi" | "mov" | "flv" | "wmv" | "m4v" | "3gp")
}

fn has_thumbnail_support(url: &str) -> bool {
    is_image_file(url) || is_pdf_file(url) || is_video_file(url)
}

// Runs on a worker thread: decodes + scales into a thread-safe Pixbuf.
fn generate_thumb_pixbuf(url: &str, size: i32) -> Option<gdk_pixbuf::Pixbuf> {
    let path = url.trim_start_matches("file://");

    if is_pdf_file(url) {
        let tmp_base = format!("/tmp/pulsar_thumb_{}_{}", std::process::id(), out_counter());
        let _ = Command::new("pdftoppm")
            .args(&["-png", "-singlefile", "-r", "72", "-l", "1", path, &tmp_base])
            .output();
        let tmp_png = format!("{}.png", tmp_base);
        let pb = gdk_pixbuf::Pixbuf::from_file_at_scale(&tmp_png, size, size, true).ok();
        let _ = std::fs::remove_file(&tmp_png);
        pb
    } else if is_video_file(url) {
        let tmp_jpg = format!("/tmp/pulsar_thumb_{}_{}.jpg", std::process::id(), out_counter());
        let _ = Command::new("ffmpeg")
            .args(&["-y", "-ss", "00:00:01", "-i", path, "-frames:v", "1", "-q:v", "5", &tmp_jpg])
            .output();
        let pb = gdk_pixbuf::Pixbuf::from_file_at_scale(&tmp_jpg, size, size, true).ok();
        let _ = std::fs::remove_file(&tmp_jpg);
        pb
    } else if is_image_file(url) {
        gdk_pixbuf::Pixbuf::from_file_at_scale(path, size, size, true).ok()
    } else {
        None
    }
}

fn generate_thumb_pixels(job: &ThumbJob) -> Option<ThumbPixels> {
    let pb = if job.disk_png.exists() {
        gdk_pixbuf::Pixbuf::from_file_at_scale(&job.disk_png, job.size, job.size, true).ok()
    } else {
        let pb = generate_thumb_pixbuf(&job.url, job.size);
        if let Some(p) = &pb {
            if let Some(parent) = job.disk_png.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = p.savev(&job.disk_png, "png", &[]);
        }
        pb
    };
    pb.map(|p| ThumbPixels {
        bytes: p.read_pixel_bytes(),
        width: p.width(),
        height: p.height(),
        rowstride: p.rowstride(),
        has_alpha: p.has_alpha(),
        bits_per_sample: p.bits_per_sample(),
    })
}

fn thumb_cache_dir() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(dirs::cache_dir)
        .or_else(|| dirs::home_dir().map(|h| h.join(".cache")))
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
    base.join("pulsaros-spotlight").join("thumbnails")
}

fn out_counter() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}


pub fn app_icon(icon: &str, desktop_file: &str) -> gtk4::Image {
    // 1. Absolute path in the .desktop Icon= field
    if icon.starts_with('/') && std::path::Path::new(icon).exists() {
        return gtk4::Image::from_file(icon);
    }

    let theme = gtk4::IconTheme::default();

    // 2. Themed lookup, as-is and stripped of any file extension
    let mut candidates = vec![icon.to_string()];
    if let Some(dot) = icon.rfind('.') {
        candidates.push(icon[..dot].to_string());
    }
    for candidate in &candidates {
        if !candidate.is_empty() && theme.has_icon(candidate) {
            return gtk4::Image::from_icon_name(candidate);
        }
    }

    // 3. Plain file lookup by Icon= name and by the .desktop basename.
    //    Some packages ship e.g. Icon=safari but install seafari.png
    let stem = std::path::Path::new(desktop_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    for name in [icon.to_string(), stem] {
        if name.is_empty() || name.starts_with('/') {
            continue;
        }
        for base in ["/usr/share/pixmaps", "/usr/local/share/pixmaps"] {
            for ext in ["png", "svg", "xpm", ""] {
                let p = if ext.is_empty() {
                    format!("{base}/{name}")
                } else {
                    format!("{base}/{name}.{ext}")
                };
                if std::path::Path::new(&p).is_file() {
                    return gtk4::Image::from_file(p);
                }
            }
        }
        if let Some(p) = find_icon_file_cached(&name) {
            return gtk4::Image::from_file(p);
        }
    }

    gtk4::Image::from_icon_name("application-x-executable")
}

pub fn result_icon(result: &SearchResult) -> gtk4::Image {
    if let Some(app) = &result.app {
        return app_icon(&app.icon, &app.filename);
    }
    let is_dir = result.mime == "inode/directory" || result.mime == "folder";
    get_file_icon(&result.url, Some(&result.mime), is_dir)
}


// ---------------------------------------------------------------------
// Sectioned result list
// ---------------------------------------------------------------------

/// One displayed row: the core row plus the backend result it came from
/// (files, clipboard, calculator) so activation can reuse existing logic.
#[derive(Clone, Debug)]
pub struct RowData {
    pub row: Row,
    pub payload: Option<SearchResult>,
}

#[derive(Clone, Debug)]
pub enum Item {
    Header(String),
    Row(RowData),
}

pub fn kind_label(r: &RowData) -> &'static str {
    match r.row.kind {
        RowKind::App => "Application",
        RowKind::Command => "Command",
        RowKind::Clipboard => "Clipboard",
        RowKind::Calc => "Calculator",
        RowKind::Fallback => "",
        RowKind::File => match &r.payload {
            Some(p) if p.url.starts_with("http") => "Web",
            Some(p) if p.mime == "inode/directory" || p.mime == "folder" => "Folder",
            _ => "File",
        },
    }
}

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
        view.list_box.connect_row_selected(move |_, row| {
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

    fn build_row(&self, r: &RowData, n: usize) -> gtk4::Box {
        let row = &r.row;
        let b = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        b.add_css_class("result-row");

        // icon tile
        let tile = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        tile.add_css_class("icon-tile");
        tile.set_valign(gtk4::Align::Center);
        tile.set_halign(gtk4::Align::Start);
        tile.set_hexpand(false);
        tile.set_size_request(28, 28);
        let theme = gtk4::IconTheme::for_display(&gdk::Display::default().expect("display"));
        let themed = |name: &str, fallback: &str| {
            gtk4::Image::from_icon_name(if theme.has_icon(name) { name } else { fallback })
        };
        match row.kind {
            RowKind::App => {
                let desktop = row.id.strip_prefix("app:").unwrap_or(&row.id);
                let img = app_icon(&row.icon_name, desktop);
                img.set_pixel_size(26);
                img.add_css_class("result-icon");
                img.set_halign(gtk4::Align::Center);
                img.set_hexpand(true);
                tile.append(&img);
            }
            RowKind::File if r.payload.as_ref().map_or(false, |p| has_thumbnail_support(&p.url)) => {
                let url = &r.payload.as_ref().unwrap().url;
                tile.append(&self.thumbnail_icon(url, 28));
            }
            RowKind::File => {
                let img = match &r.payload {
                    Some(p) => result_icon(p),
                    None => themed(&row.icon_name, "text-x-generic"),
                };
                img.set_pixel_size(24);
                img.set_halign(gtk4::Align::Center);
                img.set_hexpand(true);
                tile.append(&img);
            }
            kind => {
                tile.add_css_class("tinted");
                let (name, fb) = match kind {
                    RowKind::Calc => ("accessories-calculator-symbolic", "accessories-calculator-symbolic"),
                    RowKind::Clipboard => ("edit-paste-symbolic", "edit-copy-symbolic"),
                    RowKind::Fallback if row.id.starts_with("history:") => ("document-open-recent-symbolic", "view-refresh-symbolic"),
                    RowKind::Fallback => ("web-browser-symbolic", "web-browser-symbolic"),
                    _ => (row.icon_name.as_str(), "system-run-symbolic"),
                };
                let img = themed(name, fb);
                img.set_pixel_size(18);
                img.set_halign(gtk4::Align::Center);
                img.set_hexpand(true);
                tile.append(&img);
            }
        }
        b.append(&tile);

        // title + subtitle
        let text = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        text.set_hexpand(true);
        text.set_valign(gtk4::Align::Center);
        let title = gtk4::Label::builder()
            .label(&row.title)
            .xalign(0.0)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();
        title.add_css_class("result-title");
        text.append(&title);
        let sub = if row.kind == RowKind::Fallback { String::new() } else { row.subtitle.clone() };
        if !sub.is_empty() && sub != row.title {
            let l = gtk4::Label::builder()
                .label(&sub)
                .xalign(0.0)
                .hexpand(true)
                .ellipsize(if row.kind == RowKind::File {
                    gtk4::pango::EllipsizeMode::Middle
                } else {
                    gtk4::pango::EllipsizeMode::End
                })
                .build();
            l.add_css_class("result-subtitle");
            text.append(&l);
        }
        b.append(&text);

        // accessories
        let acc = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        acc.set_valign(gtk4::Align::Center);
        if row.favorite {
            let l = gtk4::Label::new(Some("\u{2605}"));
            l.add_css_class("result-star");
            acc.append(&l);
        }
        if let Some(a) = &row.alias {
            let l = gtk4::Label::new(Some(a));
            l.add_css_class("alias-chip");
            acc.append(&l);
        }
        if row.checked == Some(true) {
            let l = gtk4::Label::new(Some("\u{2713}"));
            l.add_css_class("result-check");
            acc.append(&l);
        }
        let kl = kind_label(r);
        if !kl.is_empty() {
            let l = gtk4::Label::new(Some(kl));
            l.add_css_class("result-kind");
            acc.append(&l);
        }
        if n < 9 {
            let keys = crate::core::shortcuts::label(crate::core::shortcuts::Act::Quick(n));
            for k in keys {
                let l = gtk4::Label::new(Some(&k));
                l.add_css_class("keycap");
                l.add_css_class("badge");
                acc.append(&l);
            }
        }
        b.append(&acc);
        b
    }

    fn select_index(&self, idx: usize) {
        if let Some(row) = self.list_box.row_at_index(idx as i32) {
            self.list_box.select_row(Some(&row));
            self.ensure_visible(&row, idx);
        }
    }

    fn ensure_visible(&self, row: &gtk4::ListBoxRow, idx: usize) {
        let adj = self.scroll.vadjustment();
        // First row of a section: keep its header in view.
        let first_in_section = idx > 0 && matches!(self.items.borrow().get(idx - 1), Some(Item::Header(_)));
        let alloc = row.allocation();
        let mut top = alloc.y() as f64;
        let bottom = top + alloc.height() as f64;
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

// Favorite apps helpers.
// English: Reading/writing /org/gnome/shell/favorite-apps through the dconf
//          D-Bus "ca.desrt.dconf.Writer" interface (Read/Write methods) is used
//          by Pulsar OS's own distro builds, but newer dconf releases (e.g.
//          Arch's) dropped those two methods, so pin/unpin silently did nothing.
//          Shelling out to the `gsettings` CLI works on every GNOME/dconf.
// Español: Leer/escribir /org/gnome/shell/favorite-apps vía el D-Bus
//          "ca.desrt.dconf.Writer" (métodos Read/Write) funcionaba en los ISO,
//          pero las versiones nuevas de dconf (p.ej. la de Arch) eliminaron esos
//          métodos y el pin/unpin no hacía nada. Usar `gsettings` funciona en
//          cualquier GNOME/dconf.
pub fn get_favorites() -> Vec<String> {
    let out = match std::process::Command::new("gsettings")
        .args(["get", "org.gnome.shell", "favorite-apps"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    if !out.status.success() {
        return Vec::new();
    }
    let Ok(text) = String::from_utf8(out.stdout) else {
        return Vec::new();
    };

    let clean = text.trim();
    let clean = clean.strip_prefix("@as ").unwrap_or(clean);
    let clean = clean.strip_prefix('[').unwrap_or(clean);
    let clean = clean.strip_suffix(']').unwrap_or(clean);

    let mut list = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for c in clean.chars() {
        if c == '\'' || c == '"' {
            in_quotes = !in_quotes;
            if !in_quotes && !current.is_empty() {
                list.push(std::mem::take(&mut current));
            }
        } else if in_quotes {
            current.push(c);
        }
    }
    list
}

pub fn set_favorites(favs: &[String]) {
    let formatted = format!("[{}]", favs.iter().map(|s| format!("'{}'", s)).collect::<Vec<_>>().join(", "));

    let _ = std::process::Command::new("gsettings")
        .args(["set", "org.gnome.shell", "favorite-apps", &formatted])
        .output();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumb_generates_scaled_from_real_image() {
        let url = "file:///usr/share/backgrounds/pulsar-os-tahoe.png";
        let pb = generate_thumb_pixbuf(url, 48);
        assert!(pb.is_some(), "el pixbuf de la imagen debería generarse");
        let p = pb.unwrap();
        assert!(p.width() > 0 && p.height() > 0);
        assert!(p.width() <= 48 && p.height() <= 48, "debe caber en 48px: {}x{}", p.width(), p.height());
    }

    #[test]
    fn thumb_generates_from_real_pdf() {
        let url = "file:///usr/share/doc/glm/manual.pdf";
        if std::path::Path::new(url.trim_start_matches("file://")).exists() {
            let pb = generate_thumb_pixbuf(url, 48);
            assert!(pb.is_some(), "el pixbuf del PDF debería generarse");
        }
    }

    #[test]
    fn thumb_rejects_unsupported_files() {
        let pb = generate_thumb_pixbuf("file:///etc/hostname", 48);
        assert!(pb.is_none());
    }

    #[test]
    fn thumb_detects_extensions() {
        assert!(is_image_file("file:///tmp/a.PNG"));
        assert!(is_pdf_file("file:///tmp/a.pdf"));
        assert!(is_video_file("file:///tmp/a.Mp4"));
        assert!(!is_image_file("file:///tmp/a.txt"));
    }
}
