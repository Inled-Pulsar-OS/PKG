use crate::core::root_search::{Row, RowKind};
use crate::apps::DesktopApp;
use crate::utils::get_file_icon;
use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use gtk4::glib::BoxedAnyObject;
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

/// How long a thumbnail helper (`pdftoppm`, `ffmpeg`) may run before it is killed.
const THUMB_HELPER_TIMEOUT: Duration = Duration::from_secs(10);

/// Picks the directory that holds scratch roots: `$XDG_RUNTIME_DIR` when it is an existing
/// absolute directory, else the per-user cache dir. Never falls back to `/tmp`; with neither
/// available it returns `None` and the caller skips the thumbnail.
fn scratch_base(
    runtime_dir: Option<std::path::PathBuf>,
    cache_home: Option<std::path::PathBuf>,
    cache_dir: Option<std::path::PathBuf>,
    home: Option<std::path::PathBuf>,
) -> Option<std::path::PathBuf> {
    runtime_dir
        .filter(|p| p.is_absolute() && p.is_dir())
        .or_else(|| {
            cache_home
                .filter(|p| p.is_absolute())
                .or(cache_dir)
                .or_else(|| home.map(|h| h.join(".cache")))
                .map(|c| c.join("pulsaros-spotlight"))
        })
}

/// A private, exclusively created scratch directory for one thumbnail job. It lives under
/// `$XDG_RUNTIME_DIR` (or the cache dir), never in world-writable `/tmp`, so another local
/// user can't pre-create the output path as a symlink. Removed on drop.
struct ScratchDir(std::path::PathBuf);

impl ScratchDir {
    fn new() -> Option<Self> {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt};
        let base = scratch_base(
            std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from),
            std::env::var_os("XDG_CACHE_HOME").map(std::path::PathBuf::from),
            dirs::cache_dir(),
            dirs::home_dir(),
        )?;
        let root = base.join("pulsaros-spotlight-scratch");
        std::fs::DirBuilder::new().recursive(true).mode(0o700).create(&root).ok()?;
        let meta = std::fs::symlink_metadata(&root).ok()?;
        // SAFETY: getuid has no preconditions.
        let me = unsafe { libc::getuid() };
        if !meta.is_dir() || meta.uid() != me || meta.mode() & 0o077 != 0 {
            return None;
        }
        // mkdir fails if the name exists, so the job directory is ours alone.
        let dir = root.join(format!("job-{}-{}", std::process::id(), out_counter()));
        std::fs::DirBuilder::new().mode(0o700).create(&dir).ok()?;
        Some(ScratchDir(dir))
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Runs `cmd` and kills it if it outlives `limit`. Returns true when it exited by itself with
/// success; a spawn error, timeout or non-zero exit is false.
fn run_with_timeout(mut cmd: Command, limit: Duration) -> bool {
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;
    let Ok(mut child) = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
    else {
        return false;
    };
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(st)) => return st.success(),
            Ok(None) if start.elapsed() >= limit => {
                // SAFETY: signals the helper's own process group (pgid == pid via process_group(0)).
                unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL) };
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => return false,
        }
    }
}

// Runs on a worker thread: decodes + scales into a thread-safe Pixbuf.
fn generate_thumb_pixbuf(url: &str, size: i32) -> Option<gdk_pixbuf::Pixbuf> {
    let path = url.trim_start_matches("file://");

    if is_pdf_file(url) {
        let scratch = ScratchDir::new()?;
        let base = scratch.path().join("page");
        let mut cmd = Command::new("pdftoppm");
        cmd.args(["-png", "-singlefile", "-r", "72", "-l", "1", path]).arg(&base);
        if !run_with_timeout(cmd, THUMB_HELPER_TIMEOUT) {
            return None;
        }
        gdk_pixbuf::Pixbuf::from_file_at_scale(base.with_extension("png"), size, size, true).ok()
    } else if is_video_file(url) {
        let scratch = ScratchDir::new()?;
        let out = scratch.path().join("frame.jpg");
        let mut cmd = Command::new("ffmpeg");
        // -n: never overwrite; -nostdin: never read the terminal.
        cmd.args(["-nostdin", "-n", "-ss", "00:00:01", "-i", path, "-frames:v", "1", "-q:v", "5"]).arg(&out);
        if !run_with_timeout(cmd, THUMB_HELPER_TIMEOUT) {
            return None;
        }
        gdk_pixbuf::Pixbuf::from_file_at_scale(&out, size, size, true).ok()
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

pub fn result_icon(result: &Payload) -> gtk4::Image {
    if let Some(app) = &result.app {
        return app_icon(&app.icon, &app.filename);
    }
    let is_dir = result.mime == "inode/directory" || result.mime == "folder";
    get_file_icon(&result.url, Some(&result.mime), is_dir)
}


// ---------------------------------------------------------------------
// Sectioned result list
// ---------------------------------------------------------------------

/// What a file, app, web or clipboard row opens: its target url, mime type and desktop app.
#[derive(Clone, Debug)]
pub struct Payload {
    pub url: String,
    pub mime: String,
    pub app: Option<DesktopApp>,
}

/// Builds a backend row (file, web, clipboard, calculator hit) with its typed payload.
pub fn result_row(url: String, title: String, mime: String, snippet: String, app: Option<DesktopApp>) -> RowData {
    let (kind, id, icon) = if let Some(v) = url.strip_prefix("calc://") {
        (RowKind::Calc, format!("calc:{v}"), "accessories-calculator-symbolic")
    } else if url.starts_with("clipboard://") {
        (RowKind::Clipboard, url.clone(), "edit-paste-symbolic")
    } else {
        (RowKind::File, url.clone(), "text-x-generic")
    };
    let sub = if snippet.is_empty() { url.trim_start_matches("file://").to_string() } else { snippet };
    RowData { row: Row::simple(kind, &id, &title, &sub, icon), payload: Some(Payload { url, mime, app }), calc: None }
}

/// One displayed row: the core row plus the payload it opens
/// (files, clipboard, calculator) so activation can reuse existing logic.
#[derive(Clone, Debug)]
pub struct RowData {
    pub row: Row,
    pub payload: Option<Payload>,
    pub calc: Option<crate::calculator::CalcResult>,
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

/// A list slot: the item plus its quick-key ordinal among rows (headers carry 0).
struct Slot {
    item: Item,
    n: usize,
}

impl Slot {
    /// Same on screen. Rows past the quick-key range ignore a shifted ordinal.
    fn same(&self, o: &Slot) -> bool {
        let item = match (&self.item, &o.item) {
            (Item::Header(x), Item::Header(y)) => x == y,
            (Item::Row(x), Item::Row(y)) => x.row == y.row,
            _ => false,
        };
        item && (self.n == o.n || (self.n >= 9 && o.n >= 9))
    }
}

type Pictures = Rc<RefCell<HashMap<String, Vec<glib::WeakRef<gtk4::Stack>>>>>;

/// Thumbnail job queue plus the cells waiting on each result.
#[derive(Clone)]
struct Thumbs {
    tx: mpsc::Sender<ThumbJob>,
    pictures: Pictures,
}

pub struct ResultView {
    list: gtk4::ListView,
    store: gio::ListStore,
    selection: gtk4::SingleSelection,
    scroll: gtk4::ScrolledWindow,
    items: Rc<RefCell<Vec<Item>>>,
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
        let store = gio::ListStore::new::<BoxedAnyObject>();
        let selection = gtk4::SingleSelection::new(Some(store.clone()));
        selection.set_autoselect(false);
        selection.set_can_unselect(true);
        let (job_tx, result_rx) = spawn_thumbnail_pool();
        let thumbs = Thumbs { tx: job_tx, pictures: Rc::new(RefCell::new(HashMap::new())) };

        let on_context: Rc<dyn Fn()> = Rc::new(on_context);
        let last_pointer = Rc::new(Cell::new((-1.0, -1.0)));

        // Rows are recycled: `setup` makes an empty holder, `bind` fills it for one slot.
        let factory = gtk4::SignalListItemFactory::new();
        {
            let selection = selection.clone();
            factory.connect_setup(move |_, obj| {
                let li = obj.downcast_ref::<gtk4::ListItem>().expect("ListItem");
                let holder = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
                li.set_child(Some(&holder));

                // Right click selects the row, then opens its context menu.
                let click = gtk4::GestureClick::builder().button(3).build();
                let (weak, sel, ctx) = (li.downgrade(), selection.clone(), on_context.clone());
                click.connect_pressed(move |g, _, _, _| {
                    g.set_state(gtk4::EventSequenceState::Claimed);
                    if let Some(li) = weak.upgrade().filter(|li| li.is_selectable()) {
                        sel.set_selected(li.position());
                        ctx();
                    }
                });
                holder.add_controller(click);

                // Hover selects only after real pointer motion: `motion` is not
                // emitted when rows scroll or are rebuilt under a still pointer.
                let motion = gtk4::EventControllerMotion::new();
                let (weak, sel, last) = (li.downgrade(), selection.clone(), last_pointer.clone());
                let h = holder.clone();
                motion.connect_motion(move |_, x, y| {
                    let Some(li) = weak.upgrade().filter(|li| li.is_selectable()) else { return };
                    let Some(root) = h.ancestor(gtk4::ListView::static_type()) else { return };
                    let Some(p) = h.compute_point(&root, &gtk4::graphene::Point::new(x as f32, y as f32)) else { return };
                    let (x, y) = (p.x() as f64, p.y() as f64);
                    let (lx, ly) = last.get();
                    if (lx - x).abs() < 1.0 && (ly - y).abs() < 1.0 {
                        return;
                    }
                    last.set((x, y));
                    if sel.selected() != li.position() {
                        sel.set_selected(li.position());
                    }
                });
                holder.add_controller(motion);
            });
        }
        {
            let thumbs = thumbs.clone();
            factory.connect_bind(move |_, obj| {
                let li = obj.downcast_ref::<gtk4::ListItem>().expect("ListItem");
                let (Some(holder), Some(slot)) = (li.child().and_downcast::<gtk4::Box>(), li.item().and_downcast::<BoxedAnyObject>())
                else {
                    return;
                };
                let slot = slot.borrow::<Slot>();
                let row_widget = holder.parent();
                match &slot.item {
                    Item::Header(t) => {
                        li.set_selectable(false);
                        li.set_activatable(false);
                        if let Some(w) = &row_widget {
                            w.add_css_class("section-row");
                        }
                        let l = gtk4::Label::builder().label(t).xalign(0.0).build();
                        l.add_css_class("section-header");
                        holder.append(&l);
                    }
                    Item::Row(r) => {
                        li.set_selectable(true);
                        li.set_activatable(true);
                        if let Some(w) = &row_widget {
                            w.remove_css_class("section-row");
                        }
                        holder.append(&Self::build_row(&thumbs, r, slot.n));
                    }
                }
            });
        }
        factory.connect_unbind(|_, obj| {
            if let Some(holder) = obj.downcast_ref::<gtk4::ListItem>().and_then(|li| li.child()).and_downcast::<gtk4::Box>() {
                while let Some(c) = holder.first_child() {
                    holder.remove(&c);
                }
            }
        });

        let list = gtk4::ListView::new(Some(selection.clone()), Some(factory));
        list.set_single_click_activate(true);
        list.set_focusable(false);
        list.add_css_class("results-list");

        let scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .build();
        scroll.add_css_class("results-area");
        scroll.set_child(Some(&list));

        let view = Self {
            list,
            store,
            selection,
            scroll,
            items: Rc::new(RefCell::new(Vec::new())),
            thumb_poll: RefCell::new(None),
        };

        let items = view.items.clone();
        view.selection.connect_selected_notify(move |sel| {
            let data = match items.borrow().get(sel.selected() as usize) {
                Some(Item::Row(r)) => Some(r.clone()),
                _ => None,
            };
            on_select(data);
        });
        view.list.connect_activate(move |_, _| on_activate());

        view.start_thumbnail_poll(thumbs.pictures.clone(), result_rx);
        view
    }

    pub fn widget(&self) -> &gtk4::ScrolledWindow {
        &self.scroll
    }

    fn selected(&self) -> Option<usize> {
        Some(self.selection.selected() as usize).filter(|i| *i < self.items.borrow().len())
    }

    pub fn selected_row(&self) -> Option<RowData> {
        match self.items.borrow().get(self.selected()?) {
            Some(Item::Row(r)) => Some(r.clone()),
            _ => None,
        }
    }

    /// Replace the list. With `keep_selection` the previously selected row id
    /// stays selected when it is still present. Only the changed tail of the
    /// store is replaced, so rows already shown (and their thumbnails) stay.
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
        let mut n = 0;
        let slots: Vec<Slot> = items
            .iter()
            .map(|it| match it {
                Item::Header(_) => Slot { item: it.clone(), n: 0 },
                Item::Row(_) => {
                    n += 1;
                    Slot { item: it.clone(), n: n - 1 }
                }
            })
            .collect();

        let old_len = self.store.n_items() as usize;
        let same = |i: usize| {
            self.store
                .item(i as u32)
                .and_downcast::<BoxedAnyObject>()
                .is_some_and(|o| o.borrow::<Slot>().same(&slots[i]))
        };
        let keep = (0..old_len.min(slots.len())).take_while(|i| same(*i)).count();
        if keep == old_len && keep == slots.len() {
            return;
        }

        // Update the shared list first: the selection handler reads it during the splice.
        *self.items.borrow_mut() = items;
        let added: Vec<BoxedAnyObject> = slots.into_iter().skip(keep).map(BoxedAnyObject::new).collect();
        self.store.splice(keep as u32, (old_len - keep) as u32, &added);

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

    fn build_calc_card(c: &crate::calculator::CalcResult, n: usize) -> gtk4::Box {
        let b = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        b.add_css_class("result-row");
        b.add_css_class("calc-card");
        b.set_homogeneous(false);
        let pane = |text: &str, caption: &str| {
            let v = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
            v.set_hexpand(true);
            v.set_halign(gtk4::Align::Center);
            v.set_valign(gtk4::Align::Center);
            let big = gtk4::Label::builder().label(text).ellipsize(gtk4::pango::EllipsizeMode::End).build();
            big.add_css_class("calc-big");
            v.append(&big);
            if !caption.is_empty() {
                let cap = gtk4::Label::new(Some(caption));
                cap.add_css_class("calc-cap");
                v.append(&cap);
            }
            v
        };
        b.append(&pane(&c.question, &c.caption_question));
        let arrow = gtk4::Label::new(Some("\u{2192}"));
        arrow.add_css_class("calc-cap");
        b.append(&arrow);
        b.append(&pane(&c.answer, &c.caption_answer));
        if n < 9 {
            let acc = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
            acc.set_valign(gtk4::Align::Center);
            for k in crate::core::shortcuts::label(crate::core::shortcuts::Act::Quick(n)) {
                let l = gtk4::Label::new(Some(&k));
                l.add_css_class("keycap");
                l.add_css_class("badge");
                acc.append(&l);
            }
            b.append(&acc);
        }
        b
    }

    fn build_row(thumbs: &Thumbs, r: &RowData, n: usize) -> gtk4::Box {
        if let Some(c) = &r.calc {
            return Self::build_calc_card(c, n);
        }
        let row = &r.row;
        let b = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        b.add_css_class("result-row");

        // icon tile
        let tile = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        tile.add_css_class("icon-tile");
        tile.set_valign(gtk4::Align::Center);
        tile.set_halign(gtk4::Align::Start);
        tile.set_hexpand(false);
        tile.set_size_request(44, 44);
        let theme = gtk4::IconTheme::for_display(&gdk::Display::default().expect("display"));
        let themed = |name: &str, fallback: &str| {
            gtk4::Image::from_icon_name(if theme.has_icon(name) { name } else { fallback })
        };
        match row.kind {
            RowKind::App => {
                let desktop = row.id.strip_prefix("app:").unwrap_or(&row.id);
                let img = app_icon(&row.icon_name, desktop);
                img.set_pixel_size(40);
                img.add_css_class("result-icon");
                img.set_halign(gtk4::Align::Center);
                img.set_hexpand(true);
                tile.append(&img);
            }
            RowKind::File if r.payload.as_ref().map_or(false, |p| has_thumbnail_support(&p.url)) => {
                let url = &r.payload.as_ref().unwrap().url;
                tile.append(&thumbs.thumbnail_icon(url, 44));
            }
            RowKind::File => {
                let img = match &r.payload {
                    Some(p) => result_icon(p),
                    None => themed(&row.icon_name, "text-x-generic"),
                };
                img.set_pixel_size(36);
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
                img.set_pixel_size(22);
                img.set_halign(gtk4::Align::Center);
                img.set_hexpand(true);
                tile.append(&img);
            }
        }
        b.append(&tile);

        // title + subtitle
        let text = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
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
        if !kl.is_empty() && !matches!(row.kind, RowKind::App | RowKind::Command) {
            let l = gtk4::Label::new(Some(kl));
            l.add_css_class("result-kind");
            acc.append(&l);
        }
        // Primary-action hint, only visible on the selected row (see CSS).
        let hint = gtk4::Label::new(Some(
            crate::ui::window::primary_title(r).split_whitespace().next().unwrap_or("Open"),
        ));
        hint.add_css_class("row-hint");
        acc.append(&hint);
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
        self.selection.set_selected(idx as u32);
        // First row of a section: keep its header in view.
        let first_in_section = idx > 0 && matches!(self.items.borrow().get(idx - 1), Some(Item::Header(_)));
        self.list.scroll_to((idx - usize::from(first_in_section)) as u32, gtk4::ListScrollFlags::NONE, None);
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
        let cur = self.selected().and_then(|s| pos.iter().position(|p| *p == s)).unwrap_or(0) as i32;
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
        let cur = self.selected().unwrap_or(0);
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

    fn start_thumbnail_poll(&self, pictures: Pictures, result_rx: mpsc::Receiver<(String, Option<ThumbPixels>)>) {
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
                            for stack in stacks.iter().filter_map(|w| w.upgrade()) {
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

}

impl Thumbs {
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
        {
            let mut pictures = self.pictures.borrow_mut();
            let waiting = pictures.entry(cache_key.clone()).or_default();
            waiting.retain(|w| w.upgrade().is_some());
            waiting.push(stack.downgrade());
        }
        let _ = self.tx.send(ThumbJob { url: url.to_string(), cache_key, size: pixel_size, disk_png });
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
#[path = "../tests/results_tests.rs"]
mod tests;
