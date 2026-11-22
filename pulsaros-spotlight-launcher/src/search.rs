use crate::apps::{DesktopApp, load_apps};
use crate::category::Category;
use crate::clipboard::ClipboardManager;
use crate::core::debounce::{Debounce, RELOAD_DELAY_MS};
use crate::core::provider::Job as ProviderJob;
use crate::core::root_search::Row;
use crate::core::worker::{JobKind, WorkerCtx};
use crate::ui::results::{result_row, RowData};
use rusqlite::{Connection as SqliteConnection, OpenFlags};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::OnceLock;
use std::thread;

pub const DEFAULT_LIMIT: usize = 500;

#[derive(Clone)]
pub struct SearchBackend {
    clipboard_mgr: Option<ClipboardManager>,
    apps: Rc<RefCell<Vec<DesktopApp>>>,
    on_apps_updated: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    monitors: Rc<RefCell<Vec<gtk4::gio::FileMonitor>>>,
}

impl SearchBackend {
    pub fn new(clipboard_mgr: Option<ClipboardManager>) -> Self {
        let apps = Rc::new(RefCell::new(load_apps()));
        let on_apps_updated = Rc::new(RefCell::new(None));
        let monitors = Rc::new(RefCell::new(Vec::new()));

        let backend = Self {
            clipboard_mgr,
            apps,
            on_apps_updated,
            monitors,
        };

        backend.setup_app_monitors();
        backend
    }

    pub fn set_on_apps_updated<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        *self.on_apps_updated.borrow_mut() = Some(Box::new(callback));
    }

    fn setup_app_monitors(&self) {
        use gtk4::gio;
        use gtk4::prelude::*;

        let dirs = crate::apps::get_desktop_dirs();
        let mut monitors_borrow = self.monitors.borrow_mut();
        let debounce = Rc::new(RefCell::new(Debounce::default()));

        for dir in dirs {
            if dir.is_dir() {
                let gfile = gio::File::for_path(&dir);
                if let Ok(monitor) = gfile.monitor_directory(gio::FileMonitorFlags::NONE, None::<&gio::Cancellable>) {
                    let apps_c = self.apps.clone();
                    let cb_c = self.on_apps_updated.clone();
                    let debounce_c = debounce.clone();
                    monitor.connect_changed(move |_, _, _, event_type| {
                        if event_type == gio::FileMonitorEvent::Changed || event_type == gio::FileMonitorEvent::Created || event_type == gio::FileMonitorEvent::Deleted {
                            if !debounce_c.borrow_mut().trigger() {
                                return;
                            }
                            let apps_c = apps_c.clone();
                            let cb_c = cb_c.clone();
                            let debounce_c = debounce_c.clone();
                            gtk4::glib::timeout_add_local_once(
                                std::time::Duration::from_millis(RELOAD_DELAY_MS),
                                move || {
                                    debounce_c.borrow_mut().fire();
                                    gtk4::glib::spawn_future_local(async move {
                                        // load_apps() parses files: keep it off the GTK main thread.
                                        if let Ok(apps) = gio::spawn_blocking(load_apps).await {
                                            *apps_c.borrow_mut() = apps;
                                            if let Some(cb) = &*cb_c.borrow() {
                                                cb();
                                            }
                                        }
                                    });
                                },
                            );
                        }
                    });
                    monitors_borrow.push(monitor);
                }
            }
        }
    }

    pub fn clipboard(&self) -> Option<ClipboardManager> {
        self.clipboard_mgr.clone()
    }

    pub fn apps_snapshot(&self) -> Vec<DesktopApp> {
        self.apps.borrow().clone()
    }

}

thread_local! {
    /// Callbacks waiting for a worker result, keyed by job id. Touched only on the GTK main thread.
    static PENDING: RefCell<std::collections::HashMap<u64, Box<dyn FnOnce(Vec<RowData>)>>> =
        RefCell::new(std::collections::HashMap::new());
    static NEXT_JOB: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Queues a provider job on the search worker; `callback` gets its rows, with file payloads,
/// on the main thread.
pub fn run_job<F>(job: ProviderJob, callback: F)
where
    F: FnOnce(Vec<RowData>) + 'static,
{
    let run = job.run;
    run_async(job.kind, move |ctx| run(ctx).into_iter().map(file_data).collect(), callback);
}

/// A file or web row plus the payload that opens it.
pub fn file_data(row: Row) -> RowData {
    let mime = if row.id.starts_with("http") {
        "text/html"
    } else {
        mime_guess::from_path(&row.title).first_raw().unwrap_or("application/octet-stream")
    };
    result_row(row.id, row.title, mime.to_string(), row.subtitle, None)
}

struct Job {
    id: u64,
    kind: JobKind,
    run: Box<dyn FnOnce(&mut WorkerCtx) -> Vec<RowData> + Send>,
}

/// For queued jobs in arrival order, flags the ones to run: the last of each kind.
fn newest_per_kind(kinds: &[JobKind]) -> Vec<bool> {
    (0..kinds.len()).map(|i| !kinds[i + 1..].contains(&kinds[i])).collect()
}

fn worker_loop(rx: Receiver<Job>, main: gtk4::glib::MainContext) {
    let mut state = WorkerCtx::default();
    while let Ok(first) = rx.recv() {
        let mut batch = vec![first];
        while let Ok(job) = rx.try_recv() {
            batch.push(job);
        }
        let kinds: Vec<JobKind> = batch.iter().map(|j| j.kind).collect();
        for (job, keep) in batch.into_iter().zip(newest_per_kind(&kinds)) {
            // A superseded job still answers (empty) so the caller's callback runs exactly once.
            let results = if keep { (job.run)(&mut state) } else { Vec::new() };
            let id = job.id;
            main.invoke(move || {
                if let Some(cb) = PENDING.with(|p| p.borrow_mut().remove(&id)) {
                    cb(results);
                }
            });
        }
    }
}

static WORKER: OnceLock<Sender<Job>> = OnceLock::new();

/// Queues `job` on the single search worker and calls `callback` with its result on the calling
/// (main) thread. The worker wakes the main loop with `MainContext::invoke` when it finishes, so
/// no polling timer exists while a search is pending. A queued job superseded by a newer one of
/// the same kind is skipped and answered with an empty result. `callback` is always called
/// exactly once.
fn run_async<J, F>(kind: JobKind, job: J, callback: F)
where
    J: FnOnce(&mut WorkerCtx) -> Vec<RowData> + Send + 'static,
    F: FnOnce(Vec<RowData>) + 'static,
{
    let id = NEXT_JOB.with(|n| {
        let id = n.get();
        n.set(id + 1);
        id
    });
    PENDING.with(|p| p.borrow_mut().insert(id, Box::new(callback)));
    let tx = WORKER.get_or_init(|| {
        let (tx, rx) = channel();
        let main = gtk4::glib::MainContext::default();
        thread::spawn(move || worker_loop(rx, main));
        tx
    });
    let _ = tx.send(Job { id, kind, run: Box::new(job) });
}

pub fn matches_category(filename: &str, category: Category) -> bool {
    if matches!(category, Category::All | Category::Files) {
        return true;
    }
    let lower = filename.to_lowercase();
    let ext = lower.split('.').next_back().unwrap_or("");
    match category {
        Category::Documents => matches!(
            ext,
            "pdf" | "txt" | "md" | "doc" | "docx" | "odt" | "xls" | "xlsx" | "ods"
                | "ppt" | "pptx" | "odp" | "csv" | "rtf" | "epub" | "html" | "json"
                | "xml" | "yaml" | "yml" | "sh" | "py" | "c" | "cpp" | "h" | "rs"
                | "go" | "js" | "ts" | "toml" | "log" | "tex"
        ),
        Category::Images => matches!(
            ext,
            "png" | "jpg" | "jpeg" | "svg" | "webp" | "gif" | "avif" | "ico"
                | "bmp" | "tiff" | "heic" | "psd" | "raw"
        ),
        Category::Audio => matches!(
            ext,
            "mp3" | "flac" | "wav" | "ogg" | "m4a" | "aac" | "opus" | "wma"
                | "oga" | "mid" | "midi"
        ),
        Category::Video => matches!(
            ext,
            "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv" | "m4v"
                | "3gp" | "ts" | "mpg" | "mpeg"
        ),
        _ => true,
    }
}

pub(crate) fn search_nautilus(ctx: &mut WorkerCtx, query: &str, category: Category, limit: usize) -> Vec<RowData> {
    let mut results = Vec::new();
    let q_trimmed = query.trim();
    if q_trimmed.is_empty() {
        return results;
    }

    let terms: Vec<String> = q_trimmed.split_whitespace().map(|s| s.to_string()).collect();
    if terms.is_empty() {
        return results;
    }

    // Call Nautilus SearchProvider over D-Bus
    let nautilus_uris = (|| -> Option<Vec<String>> {
        let conn = ctx.session()?;
        let reply = conn.call_method(
            Some("org.gnome.Nautilus"),
            "/org/gnome/Nautilus/SearchProvider",
            Some("org.gnome.Shell.SearchProvider2"),
            "GetInitialResultSet",
            &(terms,),
        );
        let reply = match reply {
            Ok(r) => r,
            Err(zbus::Error::MethodError(..)) => return None,
            Err(_) => {
                // Transport failure: drop the connection so the next job reconnects.
                ctx.session = None;
                return None;
            }
        };
        let (uris,): (Vec<String>,) = reply.body().deserialize().ok()?;
        Some(uris)
    })();

    let uris = match nautilus_uris {
        Some(u) if !u.is_empty() => u,
        _ => fallback_file_search(q_trimmed, limit),
    };

    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for url in uris {
        if seen.contains(&url) {
            continue;
        }
        seen.insert(url.clone());

        if url.contains("/.cache/") || url.contains("/.git/") || url.contains("/node_modules/") {
            continue;
        }

        let clean_url = url.trim_end_matches('/');
        let filename_part = clean_url.split('/').next_back().unwrap_or(clean_url);
        let title = match urlencoding::decode(filename_part) {
            Ok(decoded) if !decoded.trim().is_empty() => decoded.into_owned(),
            _ => filename_part.to_string(),
        };

        if !matches_category(&title, category) {
            continue;
        }

        let mime = mime_guess::from_path(&title)
            .first_raw()
            .unwrap_or("application/octet-stream")
            .to_string();

        let path_display = if let Some(p) = clean_url.strip_prefix("file://") {
            urlencoding::decode(p).unwrap_or(std::borrow::Cow::Borrowed(p)).to_string()
        } else {
            clean_url.to_string()
        };

        results.push(result_row(url, title, mime, path_display, None));

        if results.len() >= limit {
            break;
        }
    }

    order_file_results(&mut results, q_trimmed);
    results
}

/// Stable sort by title relevance: exact name, prefix, whole word, substring, rest.
/// Equal relevance keeps input order (Nautilus rank, or fallback walk order).
fn order_file_results(results: &mut [RowData], query: &str) {
    let q = query.to_lowercase();
    let tier = |title: &str| -> u8 {
        let t = title.to_lowercase();
        let stem = t.rsplit_once('.').map_or(t.as_str(), |(s, _)| s);
        if t == q || stem == q {
            0
        } else if t.starts_with(&q) {
            1
        } else if t.split(|c: char| !c.is_alphanumeric()).any(|w| w == q) {
            2
        } else if t.contains(&q) {
            3
        } else {
            4
        }
    };
    results.sort_by_cached_key(|r| tier(&r.row.title));
}

fn fallback_file_search(query: &str, limit: usize) -> Vec<String> {
    let q_lower = query.to_lowercase();
    let mut results = Vec::new();
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    let search_roots = [
        dirs::document_dir().unwrap_or_else(|| home.join("Documents")),
        dirs::download_dir().unwrap_or_else(|| home.join("Downloads")),
        dirs::picture_dir().unwrap_or_else(|| home.join("Pictures")),
        dirs::audio_dir().unwrap_or_else(|| home.join("Music")),
        dirs::video_dir().unwrap_or_else(|| home.join("Videos")),
        home.join("Desktop"),
    ];

    for root in &search_roots {
        if !root.is_dir() {
            continue;
        }
        walk_shallow(root, &q_lower, &mut results, limit, 0, 3);
        if results.len() >= limit {
            break;
        }
    }
    results
}

fn walk_shallow(dir: &std::path::Path, query: &str, results: &mut Vec<String>, limit: usize, depth: usize, max_depth: usize) {
    if depth > max_depth || results.len() >= limit {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return; };
    for entry in entries.filter_map(Result::ok) {
        if results.len() >= limit {
            break;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "node_modules" || name == "target" {
            continue;
        }
        if name.to_lowercase().contains(query) {
            results.push(format!("file://{}", entry.path().display()));
        }
        let Ok(ft) = entry.file_type() else { continue; };
        if ft.is_dir() {
            walk_shallow(&entry.path(), query, results, limit, depth + 1, max_depth);
        }
    }
}

#[cfg(test)]
fn query_browser_history_plain(query: &str, limit: usize) -> Vec<RowData> {
    query_browser_history(&mut WorkerCtx::default(), query, limit)
}

pub(crate) fn query_browser_history(ctx: &mut WorkerCtx, query: &str, limit: usize) -> Vec<RowData> {
    let mut results = Vec::new();
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

    let mut paths = Vec::new();
    paths.push(home.join(".mozilla/seafari-profile/places.sqlite"));
    let ff_dir = home.join(".mozilla/firefox");
    if ff_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(ff_dir) {
            for entry in entries.filter_map(Result::ok) {
                let db_path = entry.path().join("places.sqlite");
                if db_path.is_file() {
                    paths.push(db_path);
                }
            }
        }
    }

    for db_path in paths {
        if !db_path.exists() {
            continue;
        }

        if !ctx.history_dbs.contains_key(&db_path) {
            if let Ok(conn) = SqliteConnection::open_with_flags(
                &db_path,
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
            ) {
                // The browser holds an exclusive lock while running; never
                // let the worker block waiting for it
                let _ = conn.busy_timeout(std::time::Duration::from_millis(100));
                ctx.history_dbs.insert(db_path.clone(), conn);
            }
        }
        if let Some(conn) = ctx.history_dbs.get(&db_path) {
            let sql = if query.is_empty() {
                format!("SELECT url, title FROM moz_places WHERE hidden = 0 AND title != '' ORDER BY frecency DESC LIMIT {}", limit)
            } else {
                format!("SELECT url, title FROM moz_places WHERE (title LIKE ?1 OR url LIKE ?2) AND hidden = 0 ORDER BY frecency DESC LIMIT {}", limit)
            };

            if let Ok(mut stmt) = conn.prepare(&sql) {
                let mut rows = Vec::new();
                if query.is_empty() {
                    if let Ok(mapped) = stmt.query_map([], |row| {
                        let url: String = row.get(0)?;
                        let title: Option<String> = row.get(1)?;
                        Ok((url, title))
                    }) {
                        rows.extend(mapped.filter_map(Result::ok));
                    }
                } else {
                    let param = format!("%{}%", query);
                    if let Ok(mapped) = stmt.query_map([&param, &param], |row| {
                        let url: String = row.get(0)?;
                        let title: Option<String> = row.get(1)?;
                        Ok((url, title))
                    }) {
                        rows.extend(mapped.filter_map(Result::ok));
                    }
                }

                for (url, title) in rows {
                    let title = title.unwrap_or_else(|| url.clone());
                    results.push(result_row(url.clone(), title, "text/html".to_string(), url, None));
                }
            }
        }
    }

    results
}

#[cfg(test)]
#[path = "tests/search_tests.rs"]
mod tests;
