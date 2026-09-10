use crate::search::SearchResult;
use crate::utils::{get_file_icon, open_file};
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use std::cell::RefCell;
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
    let mut map = cache.lock().unwrap();
    if !map.is_empty() {
        return;
    }

    for root in ["/usr/share/icons", "/usr/local/share/icons"] {
        let Ok(themes) = std::fs::read_dir(root) else { continue };
        for theme in themes.filter_map(Result::ok) {
            let Ok(sizes) = std::fs::read_dir(theme.path()) else { continue };
            for size in sizes.filter_map(Result::ok) {
                let apps_dir = size.path().join("apps");
                let Ok(entries) = std::fs::read_dir(&apps_dir) else { continue };
                for entry in entries.filter_map(Result::ok) {
                    let name_os = entry.file_name();
                    let name_str = name_os.to_string_lossy();
                    for ext in EXTS {
                        if let Some(stem) = name_str.strip_suffix(&format!(".{ext}")) {
                            if !map.contains_key(stem) {
                                map.insert(stem.to_string(), Some(entry.path().to_string_lossy().into_owned()));
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
}

fn find_icon_file_cached(name: &str) -> Option<String> {
    let cache = get_icon_cache();
    if let Some(hit) = cache.lock().unwrap().get(name) {
        return hit.clone();
    }
    let found = find_icon_file(name);
    cache.lock().unwrap().insert(name.to_string(), found.clone());
    found
}

fn find_icon_file(name: &str) -> Option<String> {
    const EXTS: [&str; 3] = ["png", "svg", "xpm"];
    for root in ["/usr/share/icons", "/usr/local/share/icons"] {
        let themes = match std::fs::read_dir(root) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for theme in themes.filter_map(Result::ok) {
            let sizes = match std::fs::read_dir(theme.path()) {
                Ok(rd) => rd,
                Err(_) => continue,
            };
            for size in sizes.filter_map(Result::ok) {
                for ext in EXTS {
                    let candidate = size.path().join("apps").join(format!("{name}.{ext}"));
                    if candidate.is_file() {
                        return Some(candidate.to_string_lossy().into_owned());
                    }
                }
            }
        }
    }
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

fn select_ctx_button(buttons: &[gtk4::Button], idx: Option<usize>) {
    let visible: Vec<&gtk4::Button> = buttons.iter().filter(|b| b.is_visible()).collect();
    if visible.is_empty() {
        return;
    }
    let idx = idx.unwrap_or(0).min(visible.len() - 1);
    for b in &visible {
        b.remove_css_class("ctx-menu-btn-selected");
        b.set_opacity(0.5);
    }
    visible[idx].add_css_class("ctx-menu-btn-selected");
    visible[idx].set_opacity(1.0);
}

fn select_ctx_button_to(buttons: &[gtk4::Button], target: &gtk4::Button) {
    let visible: Vec<&gtk4::Button> = buttons.iter().filter(|b| b.is_visible()).collect();
    if let Some(idx) = visible.iter().position(|b| std::ptr::eq(*b, target)) {
        select_ctx_button(buttons, Some(idx));
    }
}

fn app_icon(icon: &str, desktop_file: &str) -> gtk4::Image {
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

fn result_icon(result: &SearchResult) -> gtk4::Image {
    if let Some(app) = &result.app {
        return app_icon(&app.icon, &app.filename);
    }
    let is_dir = result.mime == "inode/directory" || result.mime == "folder";
    get_file_icon(&result.url, Some(&result.mime), is_dir)
}

pub struct ResultView {
    stack: gtk4::Stack,
    list_box: gtk4::ListBox,
    grid: gtk4::FlowBox,
    results: Rc<RefCell<Vec<SearchResult>>>,
    result_urls: Rc<RefCell<Vec<String>>>,
    selected_index: Rc<RefCell<Option<usize>>>,
    on_activate: Rc<dyn Fn(SearchResult)>,
    on_uninstall_start: Rc<dyn Fn(String, String)>,
    on_uninstall_done: Rc<dyn Fn(bool, String, String)>,
    popover: gtk4::Popover,
    context_menu_index: Rc<RefCell<Option<usize>>>,
    ctx_menu_open: Rc<RefCell<bool>>,
    ctx_menu_buttons: Vec<gtk4::Button>,
    ctx_selected: Rc<RefCell<Option<usize>>>,
    thumb_job_tx: RefCell<Option<mpsc::Sender<ThumbJob>>>,
    thumb_pictures: RefCell<HashMap<String, Vec<gtk4::Stack>>>,
    thumb_poll: RefCell<Option<glib::SourceId>>,
}

impl ResultView {
    pub fn new<F, U1, U2>(
        on_activate: F,
        on_uninstall_start: U1,
        on_uninstall_done: U2,
    ) -> Self
    where
        F: Fn(SearchResult) + 'static,
        U1: Fn(String, String) + 'static,
        U2: Fn(bool, String, String) + 'static,
    {
        let stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .build();

        let list_box = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::Single)
            .build();

        let grid = gtk4::FlowBox::builder()
            .valign(gtk4::Align::Start)
            .max_children_per_line(6)
            .selection_mode(gtk4::SelectionMode::Single)
            .build();

        stack.add_named(&list_box, Some("list"));
        stack.add_named(&grid, Some("grid"));

        let results = Rc::new(RefCell::new(Vec::new()));
        let result_urls = Rc::new(RefCell::new(Vec::new()));
        let selected_index = Rc::new(RefCell::new(None));
        let context_menu_index = Rc::new(RefCell::new(None));

        let popover = gtk4::Popover::builder()
            .has_arrow(false)
            .position(gtk4::PositionType::Bottom)
            .autohide(true)
            .build();
        popover.add_css_class("ctx-menu");

        let ctx_selected = Rc::new(RefCell::new(None));
        let ctx_menu_open = Rc::new(RefCell::new(false));

        let mut view = Self {
            stack,
            list_box,
            grid,
            results,
            result_urls,
            selected_index,
            on_activate: Rc::new(on_activate),
            on_uninstall_start: Rc::new(on_uninstall_start),
            on_uninstall_done: Rc::new(on_uninstall_done),
            popover,
            context_menu_index,
            ctx_menu_open,
            ctx_menu_buttons: Vec::new(),
            ctx_selected,
            thumb_job_tx: RefCell::new(None),
            thumb_pictures: RefCell::new(HashMap::new()),
            thumb_poll: RefCell::new(None),
        };

        view.setup_events();
        view.setup_context_menu();

        let open_c = view.ctx_menu_open.clone();
        view.popover.connect_closed(move |_| {
            *open_c.borrow_mut() = false;
        });

        let (job_tx, result_rx) = spawn_thumbnail_pool();
        *view.thumb_job_tx.borrow_mut() = Some(job_tx);
        view.start_thumbnail_poll(result_rx);

        view
    }

    pub fn widget(&self) -> &gtk4::Stack {
        &self.stack
    }

    pub fn set_popover_parent<P: IsA<gtk4::Widget>>(&self, parent: &P) {
        self.popover.set_parent(parent);
    }

    pub fn context_menu_is_open(&self) -> bool {
        *self.ctx_menu_open.borrow()
    }

    pub fn context_menu_step(&self, down: bool) {
        let visible: Vec<&gtk4::Button> = self
            .ctx_menu_buttons
            .iter()
            .filter(|b| b.is_visible())
            .collect();
        if visible.is_empty() {
            return;
        }
        let cnt = visible.len();
        let next = match *self.ctx_selected.borrow() {
            Some(i) if down => (i + 1) % cnt,
            Some(i) => (i + cnt - 1) % cnt,
            None => 0,
        };
        select_ctx_button(&self.ctx_menu_buttons, Some(next));
        *self.ctx_selected.borrow_mut() = Some(next);
        visible[next].grab_focus();
    }

    pub fn context_menu_activate(&self) {
        let visible: Vec<&gtk4::Button> = self
            .ctx_menu_buttons
            .iter()
            .filter(|b| b.is_visible())
            .collect();
        if let Some(i) = *self.ctx_selected.borrow() {
            if i < visible.len() {
                visible[i].emit_clicked();
            }
        }
    }

    pub fn context_menu_close(&self) {
        self.popover.popdown();
    }

    pub fn set_results(&self, new_results: Vec<SearchResult>, as_grid: bool) {
        let mut new_results = new_results;
        new_results.truncate(200);

        let new_urls: Vec<String> = new_results.iter().map(|r| r.url.clone()).collect();
        let old_urls = self.result_urls.borrow();

        // Skip rebuild if the result set is identical (common while typing)
        if *old_urls == new_urls {
            return;
        }
        drop(old_urls);

        // Rebind widgets to a fresh thumbnail map (in-flight loads are dropped)
        self.thumb_pictures.borrow_mut().clear();

        *self.selected_index.borrow_mut() = None;

        // Clear children
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        while let Some(child) = self.grid.first_child() {
            self.grid.remove(&child);
        }

        let visible = new_results.clone();
        *self.results.borrow_mut() = new_results;
        *self.result_urls.borrow_mut() = new_urls;

        for result in &visible {
            let list_row = self.build_list_row(result);
            self.list_box.append(&list_row);

            let grid_child = self.build_grid_child(result);
            self.grid.insert(&grid_child, -1);
        }

        self.stack.set_visible_child_name(if as_grid { "grid" } else { "list" });

        if let Some(adj) = self.viewport_vadjustment() {
            adj.set_value(0.0);
        }

        if !as_grid {
            if let Some(first_row) = self.list_box.row_at_index(0) {
                self.list_box.select_row(Some(&first_row));
                *self.selected_index.borrow_mut() = Some(0);
            }
        } else {
            if let Some(first_child) = self.grid.child_at_index(0) {
                self.grid.select_child(&first_child);
                *self.selected_index.borrow_mut() = Some(0);
            }
        }
    }

    fn start_thumbnail_poll(&self, result_rx: mpsc::Receiver<(String, Option<ThumbPixels>)>) {
        let pictures = self.thumb_pictures.clone();
        let source_id = glib::timeout_add_local(Duration::from_millis(5), move || {
            match result_rx.try_recv() {
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
            }
        });
        *self.thumb_poll.borrow_mut() = Some(source_id);
    }

    // Builds a sized cell that shows a spinner until the thumbnail is ready,
    // then swaps in the decoded texture. Always returns a widget (never blocks).
    fn thumbnail_icon(&self, url: &str, pixel_size: i32, grid: bool) -> gtk4::Stack {
        let path = url.trim_start_matches("file://").to_string();
        let css = if grid { "result-thumb-grid" } else { "result-thumb" };

        let stack = gtk4::Stack::new();
        stack.set_size_request(pixel_size, pixel_size);
        stack.set_hexpand(false);
        stack.set_halign(if grid { gtk4::Align::Center } else { gtk4::Align::Start });
        stack.set_valign(gtk4::Align::Center);
        stack.add_css_class(css);

        let spinner = gtk4::Spinner::new();
        spinner.set_halign(gtk4::Align::Center);
        spinner.set_valign(gtk4::Align::Center);
        spinner.set_size_request(24, 24);
        spinner.start();
        stack.add_named(&spinner, Some("loading"));

        let pic = gtk4::Picture::new();
        pic.set_size_request(pixel_size, pixel_size);
        pic.set_can_shrink(true);
        pic.add_css_class(css);
        pic.set_halign(gtk4::Align::Center);
        pic.set_valign(gtk4::Align::Center);
        stack.add_named(&pic, Some("thumb"));

        let cache_key = format!("{}@{}", path, pixel_size);
        let disk_png = thumb_cache_dir().join(format!("{}.png", fnv1a64(&cache_key)));

        // Already decoded this exact size this session: show it straight away
        let cache = THUMBNAIL_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Some(Some(texture)) = cache.lock().unwrap().get(&cache_key) {
            pic.set_paintable(Some(texture));
            stack.set_visible_child_name("thumb");
            return stack;
        }

        stack.set_visible_child_name("loading");
        self.thumb_pictures
            .borrow_mut()
            .entry(cache_key.clone())
            .or_default()
            .push(stack.clone());

        if let Some(job_tx) = self.thumb_job_tx.borrow().as_ref() {
            let _ = job_tx.send(ThumbJob {
                url: url.to_string(),
                cache_key,
                size: pixel_size,
                disk_png,
            });
        }
        stack
    }

    fn build_list_row(&self, result: &SearchResult) -> gtk4::ListBoxRow {
        let row = gtk4::ListBoxRow::new();
        let box_widget = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        box_widget.add_css_class("result-item-list");

        let is_file = result.app.is_none() && result.url.starts_with("file://");
        if is_file && has_thumbnail_support(&result.url) {
            box_widget.append(&self.thumbnail_icon(&result.url, 32, false));
        } else {
            let icon = result_icon(result);
            icon.set_pixel_size(32);
            icon.add_css_class("result-icon");
            box_widget.append(&icon);
        }

        let text_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);

        let title_label = gtk4::Label::builder()
            .label(&result.title)
            .xalign(0.0)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .max_width_chars(60)
            .build();
        title_label.add_css_class("result-title");
        text_box.append(&title_label);

        let mut sub_text = result.snippet.clone();
        if sub_text.is_empty() {
            if result.url.starts_with("file://") {
                sub_text = result.url.trim_start_matches("file://").to_string();
            } else if result.url.starts_with("http://") || result.url.starts_with("https://") {
                sub_text = result.url.clone();
            }
        }

        if !sub_text.is_empty() {
            let snippet_label = gtk4::Label::builder()
                .label(&sub_text)
                .xalign(0.0)
                .ellipsize(if result.url.starts_with("file://") {
                    gtk4::pango::EllipsizeMode::Middle
                } else {
                    gtk4::pango::EllipsizeMode::End
                })
                .max_width_chars(60)
                .build();
            snippet_label.add_css_class("result-snippet");
            text_box.append(&snippet_label);
        }

        box_widget.append(&text_box);
        row.set_child(Some(&box_widget));

        row
    }

    fn build_grid_child(&self, result: &SearchResult) -> gtk4::FlowBoxChild {
        let child = gtk4::FlowBoxChild::new();
        let box_widget = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        box_widget.add_css_class("result-item-grid");
        box_widget.set_size_request(90, -1);

        let is_file = result.app.is_none() && result.url.starts_with("file://");
        if is_file && has_thumbnail_support(&result.url) {
            box_widget.append(&self.thumbnail_icon(&result.url, 48, true));
        } else {
            let icon = result_icon(result);
            icon.set_pixel_size(48);
            icon.add_css_class("result-icon-grid");
            box_widget.append(&icon);
        }

        let title_label = gtk4::Label::builder()
            .label(&result.title)
            .wrap(true)
            .justify(gtk4::Justification::Center)
            .max_width_chars(12)
            .halign(gtk4::Align::Center)
            .build();
        title_label.add_css_class("result-title-grid");

        box_widget.append(&title_label);
        child.set_child(Some(&box_widget));

        child
    }

    fn setup_events(&self) {
        let on_activate = self.on_activate.clone();
        let results = self.results.clone();
        let selected_index = self.selected_index.clone();

        // List row activated
        self.list_box.connect_row_activated(move |_, row| {
            let idx = row.index() as usize;
            if let Some(res) = results.borrow().get(idx) {
                (*on_activate)(res.clone());
            }
        });

        let results_c = self.results.clone();
        let on_activate_c = self.on_activate.clone();
        // Grid child activated
        self.grid.connect_child_activated(move |_, child| {
            let idx = child.index() as usize;
            if let Some(res) = results_c.borrow().get(idx) {
                (*on_activate_c)(res.clone());
            }
        });

        // Track selected index
        let sel_idx = selected_index.clone();
        self.list_box.connect_row_selected(move |_, row| {
            if let Some(r) = row {
                *sel_idx.borrow_mut() = Some(r.index() as usize);
            }
        });

        let sel_idx_c = selected_index.clone();
        self.grid.connect_selected_children_changed(move |fb| {
            let selected = fb.selected_children();
            if let Some(c) = selected.first() {
                *sel_idx_c.borrow_mut() = Some(c.index() as usize);
            }
        });

        // Gesture click right button for ListBox
        let list_click = gtk4::GestureClick::builder().button(3).build();
        let list_box_c = self.list_box.clone();
        let results_cc = self.results.clone();
        let popover_c = self.popover.clone();
        let ctx_menu_idx = self.context_menu_index.clone();
        list_click.connect_pressed(move |gesture, _, x, y| {
            gesture.set_state(gtk4::EventSequenceState::Claimed);
            let row = list_box_c.row_at_y(y as i32);
            if let Some(r) = row {
                list_box_c.select_row(Some(&r));
                let idx = r.index() as usize;
                if results_cc.borrow().get(idx).is_some() {
                    *ctx_menu_idx.borrow_mut() = Some(idx);
                    if popover_c.parent().as_ref() != Some(r.upcast_ref()) {
                        popover_c.set_parent(&r);
                    }
                    let rect = gdk::Rectangle::new(x as i32, y as i32, 1, 1);
                    popover_c.set_pointing_to(Some(&rect));
                    popover_c.popup();
                }
            }
        });
        self.list_box.add_controller(list_click);

        // Gesture click right button for Grid
        let grid_click = gtk4::GestureClick::builder().button(3).build();
        let grid_c = self.grid.clone();
        let results_ccc = self.results.clone();
        let popover_cc = self.popover.clone();
        let ctx_menu_idx_c = self.context_menu_index.clone();
        grid_click.connect_pressed(move |gesture, _, x, y| {
            gesture.set_state(gtk4::EventSequenceState::Claimed);
            let child = grid_c.child_at_pos(x as i32, y as i32);
            if let Some(c) = child {
                grid_c.select_child(&c);
                let idx = c.index() as usize;
                if results_ccc.borrow().get(idx).is_some() {
                    *ctx_menu_idx_c.borrow_mut() = Some(idx);
                    if popover_cc.parent().as_ref() != Some(c.upcast_ref()) {
                        popover_cc.set_parent(&c);
                    }
                    let rect = gdk::Rectangle::new(x as i32, y as i32, 1, 1);
                    popover_cc.set_pointing_to(Some(&rect));
                    popover_cc.popup();
                }
            }
        });
        self.grid.add_controller(grid_click);
    }

    fn setup_context_menu(&mut self) {
        let outer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        outer.add_css_class("ctx-menu-box");

        // 1. Open
        let btn_open = gtk4::Button::with_label("Open");
        btn_open.add_css_class("ctx-menu-btn");
        btn_open.set_halign(gtk4::Align::Fill);
        outer.append(&btn_open);

        // 2. Open Folder
        let btn_open_dir = gtk4::Button::with_label("Open containing folder");
        btn_open_dir.add_css_class("ctx-menu-btn");
        btn_open_dir.set_halign(gtk4::Align::Fill);
        outer.append(&btn_open_dir);

        let sep1 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        outer.append(&sep1);

        // 3. Pin to Dock
        let btn_pin = gtk4::Button::with_label("Pin to dock");
        btn_pin.add_css_class("ctx-menu-btn");
        btn_pin.set_halign(gtk4::Align::Fill);
        outer.append(&btn_pin);

        // 4. Uninstall
        let btn_uninstall = gtk4::Button::with_label("Uninstall");
        btn_uninstall.add_css_class("ctx-menu-btn");
        btn_uninstall.add_css_class("ctx-menu-btn-danger");
        btn_uninstall.set_halign(gtk4::Align::Fill);
        outer.append(&btn_uninstall);

        let sep2 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        outer.append(&sep2);

        // 5. Copy Name
        let btn_copy_name = gtk4::Button::with_label("Copy name");
        btn_copy_name.add_css_class("ctx-menu-btn");
        btn_copy_name.set_halign(gtk4::Align::Fill);
        outer.append(&btn_copy_name);

        // 6. Copy Path
        let btn_copy_path = gtk4::Button::with_label("Copy path");
        btn_copy_path.add_css_class("ctx-menu-btn");
        btn_copy_path.set_halign(gtk4::Align::Fill);
        outer.append(&btn_copy_path);

        self.popover.set_child(Some(&outer));

        // Arrow key navigation within the context menu
        let ctx_menu_buttons: Vec<gtk4::Button> = vec![
            btn_open.clone(),
            btn_open_dir.clone(),
            btn_pin.clone(),
            btn_uninstall.clone(),
            btn_copy_name.clone(),
            btn_copy_path.clone(),
        ];
        self.ctx_menu_buttons = ctx_menu_buttons.clone();
        *self.ctx_selected.borrow_mut() = None;

        // Mouse hover also moves the selection highlight
        for b in &ctx_menu_buttons {
            let b_c = b.clone();
            let btns_c = ctx_menu_buttons.clone();
            let motion = gtk4::EventControllerMotion::new();
            motion.connect_enter(move |_, _, _| {
                select_ctx_button_to(&btns_c, &b_c);
            });
            b.add_controller(motion);
        }

        let popover_key = gtk4::EventControllerKey::new();
        let pop_c = self.popover.clone();
        let btns_key = ctx_menu_buttons.clone();
        let sel_key = self.ctx_selected.clone();
        popover_key.connect_key_pressed(move |_, keyval, _, _| {
            let visible_btns: Vec<&gtk4::Button> = btns_key
                .iter()
                .filter(|b| b.is_visible())
                .collect();
            if visible_btns.is_empty() {
                return gtk4::glib::Propagation::Proceed;
            }

            match keyval {
                gdk::Key::Down => {
                    let cnt = visible_btns.len();
                    let next = match *sel_key.borrow() {
                        Some(i) => (i + 1) % cnt,
                        None => 0,
                    };
                    select_ctx_button(&btns_key, Some(next));
                    *sel_key.borrow_mut() = Some(next);
                    visible_btns[next].grab_focus();
                    gtk4::glib::Propagation::Stop
                }
                gdk::Key::Up => {
                    let cnt = visible_btns.len();
                    let prev = match *sel_key.borrow() {
                        Some(0) | None => cnt - 1,
                        Some(i) => i - 1,
                    };
                    select_ctx_button(&btns_key, Some(prev));
                    *sel_key.borrow_mut() = Some(prev);
                    visible_btns[prev].grab_focus();
                    gtk4::glib::Propagation::Stop
                }
                gdk::Key::Escape => {
                    pop_c.popdown();
                    gtk4::glib::Propagation::Stop
                }
                _ => gtk4::glib::Propagation::Proceed,
            }
        });
        self.popover.add_controller(popover_key);

        // Connect popover opened to toggle buttons visibility
        let results_c = self.results.clone();
        let ctx_menu_idx = self.context_menu_index.clone();
        let btn_open_c = btn_open.clone();
        let btn_open_dir_c = btn_open_dir.clone();
        let btn_copy_path_c = btn_copy_path.clone();
        let btn_pin_c = btn_pin.clone();
        let btn_uninstall_c = btn_uninstall.clone();
        let btns_map = ctx_menu_buttons.clone();
        let sel_map = self.ctx_selected.clone();
        let map_open = self.ctx_menu_open.clone();

        self.popover.connect_map(move |_| {
            *map_open.borrow_mut() = true;
            if let Some(idx) = *ctx_menu_idx.borrow() {
                if let Some(res) = results_c.borrow().get(idx) {
                    let is_app = res.app.is_some();
                    btn_open_dir_c.set_visible(!is_app);
                    btn_copy_path_c.set_visible(!is_app);
                    btn_pin_c.set_visible(is_app);
                    btn_uninstall_c.set_visible(is_app);

                    if is_app {
                        let app = res.app.as_ref().unwrap();
                        let favs = get_favorites();
                        let is_pinned = favs.contains(&app.filename);
                        btn_pin_c.set_label(if is_pinned { "Unpin from dock" } else { "Pin to dock" });
                    }

                    select_ctx_button(&btns_map, Some(0));
                    *sel_map.borrow_mut() = Some(0);
                    btn_open_c.grab_focus();
                }
            }
        });

        // 1. Click Open
        let on_activate = self.on_activate.clone();
        let results_cc = self.results.clone();
        let ctx_menu_idx_c = self.context_menu_index.clone();
        let popover_c = self.popover.clone();
        btn_open.connect_clicked(move |_| {
            popover_c.popdown();
            if let Some(idx) = *ctx_menu_idx_c.borrow() {
                if let Some(res) = results_cc.borrow().get(idx) {
                    (*on_activate)(res.clone());
                }
            }
        });

        // 2. Click Open Folder
        let results_ccc = self.results.clone();
        let ctx_menu_idx_cc = self.context_menu_index.clone();
        let popover_cc = self.popover.clone();
        btn_open_dir.connect_clicked(move |_| {
            popover_cc.popdown();
            if let Some(idx) = *ctx_menu_idx_cc.borrow() {
                if let Some(res) = results_ccc.borrow().get(idx) {
                    let clean_path = res.url.trim_start_matches("file://");
                    if let Some(parent) = std::path::Path::new(clean_path).parent() {
                        let parent_url = format!("file://{}", parent.display());
                        open_file(&parent_url);
                    }
                }
            }
        });

        // 3. Click Pin to Dock
        let results_cccc = self.results.clone();
        let ctx_menu_idx_ccc = self.context_menu_index.clone();
        let popover_ccc = self.popover.clone();
        btn_pin.connect_clicked(move |_| {
            popover_ccc.popdown();
            if let Some(idx) = *ctx_menu_idx_ccc.borrow() {
                if let Some(res) = results_cccc.borrow().get(idx) {
                    if let Some(app) = &res.app {
                        let mut favs = get_favorites();
                        if favs.contains(&app.filename) {
                            favs.retain(|x| x != &app.filename);
                        } else {
                            favs.push(app.filename.clone());
                        }
                        set_favorites(&favs);
                    }
                }
            }
        });

        // 4. Click Uninstall
        let results_5 = self.results.clone();
        let ctx_menu_idx_5 = self.context_menu_index.clone();
        let popover_5 = self.popover.clone();
        let on_un_start = self.on_uninstall_start.clone();
        let on_un_done = self.on_uninstall_done.clone();
        btn_uninstall.connect_clicked(move |_| {
            popover_5.popdown();
            if let Some(idx) = *ctx_menu_idx_5.borrow() {
                if let Some(res) = results_5.borrow().get(idx) {
                    if let Some(app) = &res.app {
                        let desktop_id = app.filename.clone();
                        let app_name = app.name.clone();
                        (*on_un_start)(desktop_id.clone(), app_name.clone());

                        let on_done = on_un_done.clone();
                        let app_name_c = app_name.clone();

                        let (sender, receiver) = std::sync::mpsc::channel::<(bool, String)>();
                        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(1), move || {
                            match receiver.try_recv() {
                                Ok((success, message)) => {
                                    (*on_done)(success, message, app_name_c.clone());
                                    gtk4::glib::ControlFlow::Break
                                }
                                Err(std::sync::mpsc::TryRecvError::Empty) => {
                                    gtk4::glib::ControlFlow::Continue
                                }
                                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                    gtk4::glib::ControlFlow::Break
                                }
                            }
                        });

                        std::thread::spawn(move || {
                            let mut cmd = Command::new("pkm");
                            cmd.args(&["--uninstall", &desktop_id]);
                            cmd.env("APPINSTALL_SUDO", "pkexec");
                            let output = cmd.output();
                            let success = output.as_ref().map_or(false, |o| o.status.success());
                            let message = output.map_or_else(
                                |e| e.to_string(),
                                |o| {
                                    let merged = [o.stdout, o.stderr].concat();
                                    String::from_utf8_lossy(&merged).into_owned()
                                },
                            );
                            let _ = sender.send((success, message));
                        });
                    }
                }
            }
        });

        // 5. Click Copy Name
        let results_6 = self.results.clone();
        let ctx_menu_idx_6 = self.context_menu_index.clone();
        let popover_6 = self.popover.clone();
        btn_copy_name.connect_clicked(move |_| {
            popover_6.popdown();
            if let Some(idx) = *ctx_menu_idx_6.borrow() {
                if let Some(res) = results_6.borrow().get(idx) {
                    if let Some(display) = gdk::Display::default() {
                        display.clipboard().set_text(&res.title);
                    }
                }
            }
        });

        // 6. Click Copy Path
        let results_7 = self.results.clone();
        let ctx_menu_idx_7 = self.context_menu_index.clone();
        let popover_7 = self.popover.clone();
        btn_copy_path.connect_clicked(move |_| {
            popover_7.popdown();
            if let Some(idx) = *ctx_menu_idx_7.borrow() {
                if let Some(res) = results_7.borrow().get(idx) {
                    if let Some(display) = gdk::Display::default() {
                        display.clipboard().set_text(&res.url);
                    }
                }
            }
        });
    }

    fn viewport_vadjustment(&self) -> Option<gtk4::Adjustment> {
        self.stack
            .ancestor(gtk4::ScrolledWindow::static_type())
            .and_then(|w| w.downcast::<gtk4::ScrolledWindow>().ok())
            .map(|sw| sw.vadjustment())
    }

    fn ensure_visible<W: IsA<gtk4::Widget>>(&self, widget: &W) {
        let Some(adj) = self.viewport_vadjustment() else {
            return;
        };
        let alloc = widget.allocation();
        let top = alloc.y() as f64;
        let bottom = top + alloc.height() as f64;
        let value = adj.value();
        let page = adj.page_size();
        if top < value {
            adj.set_value(top);
        } else if bottom > value + page {
            adj.set_value(bottom - page);
        }
    }

    pub fn move_selection_up(&self) {
        // Bind first: an `if let ... = *refcell.borrow()` scrutinee keeps the
        // borrow alive across the whole block, and select_child/select_row
        // emit signals whose handlers borrow again
        let current = *self.selected_index.borrow();
        if self.stack.visible_child_name().as_deref() == Some("list") {
            if let Some(idx) = current {
                if idx > 0 {
                    if let Some(row) = self.list_box.row_at_index((idx - 1) as i32) {
                        self.list_box.select_row(Some(&row));
                        self.ensure_visible(&row);
                    }
                }
            }
        } else if let Some(idx) = current {
            if idx >= 6 {
                if let Some(child) = self.grid.child_at_index((idx - 6) as i32) {
                    self.grid.select_child(&child);
                    self.ensure_visible(&child);
                }
            }
        }
    }

    pub fn move_selection_down(&self) {
        let max_len = self.results.borrow().len();
        let current = *self.selected_index.borrow();
        if self.stack.visible_child_name().as_deref() == Some("list") {
            let idx = current.unwrap_or(0);
            if idx + 1 < max_len {
                if let Some(row) = self.list_box.row_at_index((idx + 1) as i32) {
                    self.list_box.select_row(Some(&row));
                    self.ensure_visible(&row);
                }
            }
        } else if let Some(idx) = current {
            let next_idx = idx + 6;
            if next_idx < max_len {
                if let Some(child) = self.grid.child_at_index(next_idx as i32) {
                    self.grid.select_child(&child);
                    self.ensure_visible(&child);
                }
            }
        }
    }

    pub fn move_selection_left(&self) {
        if self.stack.visible_child_name().as_deref() == Some("grid") {
            let current = *self.selected_index.borrow();
            if let Some(idx) = current {
                if idx > 0 {
                    if let Some(child) = self.grid.child_at_index((idx - 1) as i32) {
                        self.grid.select_child(&child);
                        self.ensure_visible(&child);
                    }
                }
            }
        }
    }

    pub fn move_selection_right(&self) {
        if self.stack.visible_child_name().as_deref() == Some("grid") {
            let max_len = self.results.borrow().len();
            let current = *self.selected_index.borrow();
            if let Some(idx) = current {
                if idx + 1 < max_len {
                    if let Some(child) = self.grid.child_at_index((idx + 1) as i32) {
                        self.grid.select_child(&child);
                        self.ensure_visible(&child);
                    }
                }
            }
        }
    }

    pub fn show_context_menu_for_selected(&self) {
        let idx = match *self.selected_index.borrow() {
            Some(i) => i,
            None => return,
        };
        if self.results.borrow().get(idx).is_none() {
            return;
        }

        *self.context_menu_index.borrow_mut() = Some(idx);
        *self.ctx_menu_open.borrow_mut() = true;

        let is_list = self.stack.visible_child_name().as_deref() == Some("list");

        if is_list {
            if let Some(row) = self.list_box.row_at_index(idx as i32) {
                if self.popover.parent().as_ref() != Some(row.upcast_ref()) {
                    self.popover.set_parent(&row);
                }
                let alloc = row.allocation();
                let rect = gdk::Rectangle::new(
                    alloc.width() / 2,
                    alloc.height() / 2,
                    1,
                    1,
                );
                self.popover.set_pointing_to(Some(&rect));
                self.popover.popup();
            }
        } else if let Some(child) = self.grid.child_at_index(idx as i32) {
            if self.popover.parent().as_ref() != Some(child.upcast_ref()) {
                self.popover.set_parent(&child);
            }
            let alloc = child.allocation();
            let rect = gdk::Rectangle::new(
                alloc.width() / 2,
                alloc.height() / 2,
                1,
                1,
            );
            self.popover.set_pointing_to(Some(&rect));
            self.popover.popup();
        }
    }

    pub fn activate_selected(&self) -> bool {
        // Clone the result and drop the borrows before invoking the
        // callback: activation may re-enter set_results()
        let res = {
            let results = self.results.borrow();
            (*self.selected_index.borrow())
                .and_then(|idx| results.get(idx).cloned())
        };
        if let Some(res) = res {
            (*self.on_activate)(res);
            return true;
        }
        false
    }
}

// Favorite apps helpers via D-Bus
fn get_favorites() -> Vec<String> {
    let conn = match zbus::blocking::Connection::session() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let reply = conn.call_method(
        Some("ca.desrt.dconf"),
        "/ca/desrt/dconf/Writer/user",
        Some("ca.desrt.dconf.Writer"),
        "Read",
        &"/org/gnome/shell/favorite-apps",
    );
    let Ok(repl) = reply else { return Vec::new() };

    let Ok((variant,)): Result<(zbus::zvariant::OwnedValue,), _> = repl.body().deserialize() else {
        return Vec::new();
    };

    let Ok(str_val): Result<&str, _> = variant.downcast_ref() else {
        return Vec::new();
    };

    let clean = str_val.trim();
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

fn set_favorites(favs: &[String]) {
    let formatted = format!("[{}]", favs.iter().map(|s| format!("'{}'", s)).collect::<Vec<_>>().join(", "));

    let conn = match zbus::blocking::Connection::session() {
        Ok(c) => c,
        Err(_) => return,
    };

    let _ = conn.call_method(
        Some("ca.desrt.dconf"),
        "/ca/desrt/dconf/Writer/user",
        Some("ca.desrt.dconf.Writer"),
        "Write",
        &("/org/gnome/shell/favorite-apps", zbus::zvariant::Value::Str(formatted.into())),
    );
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
