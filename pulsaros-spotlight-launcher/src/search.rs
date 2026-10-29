use crate::apps::{DesktopApp, load_apps};
use crate::calculator::Calculator;
use crate::clipboard::ClipboardManager;
use rusqlite::{Connection as SqliteConnection, OpenFlags};
use std::cell::RefCell;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;
use std::thread;
use zbus::blocking::Connection as ZbusConnection;

pub const DEFAULT_LIMIT: usize = 500;

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub url: String,
    pub title: String,
    pub mime: String,
    pub snippet: String,
    pub app: Option<DesktopApp>,
}

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

        for dir in dirs {
            if dir.is_dir() {
                let gfile = gio::File::for_path(&dir);
                if let Ok(monitor) = gfile.monitor_directory(gio::FileMonitorFlags::NONE, None::<&gio::Cancellable>) {
                    let apps_c = self.apps.clone();
                    let cb_c = self.on_apps_updated.clone();
                    monitor.connect_changed(move |_, _, _, event_type| {
                        if event_type == gio::FileMonitorEvent::Changed || event_type == gio::FileMonitorEvent::Created || event_type == gio::FileMonitorEvent::Deleted {
                            *apps_c.borrow_mut() = load_apps();
                            if let Some(cb) = &*cb_c.borrow() {
                                cb();
                            }
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

    #[allow(dead_code)]
    pub fn get_indexing_status(&self) -> (bool, String, f64) {
        get_indexing_status_plain()
    }

    pub fn search_instant(&self, query: &str, category: &str, limit: usize) -> Vec<SearchResult> {
        if category == "clipboard" {
            if let Some(mgr) = &self.clipboard_mgr {
                return mgr.search_history(query);
            }
            return Vec::new();
        }

        if category == "web" {
            return self.search_web(query);
        }

        let q_clean = query.trim();

        if category == "apps" || category == "applications" {
            return self.search_apps(q_clean, limit);
        }

        if q_clean.is_empty() && category == "all" {
            return self.search_apps("", limit.max(200));
        }

        if category == "all" {
            let mut results = Vec::new();

            // 1. Calculator
            let calc_eval = Calculator::evaluate(q_clean);
            if let Some((val_str, snippet)) = &calc_eval {
                results.push(SearchResult {
                    url: format!("calc://{}", val_str),
                    title: val_str.clone(),
                    mime: "application/x-calculator".to_string(),
                    snippet: snippet.clone(),
                    app: None,
                });
            }

            // 2. Clipboard (top 3)
            if let Some(mgr) = &self.clipboard_mgr {
                let mut clips = mgr.search_history(q_clean);
                clips.truncate(3);
                results.extend(clips);
            }

            // 3. Applications
            results.extend(self.search_apps(q_clean, limit));

            // 4. Web search (top 1)
            if !q_clean.is_empty() && calc_eval.is_none() {
                let mut web = self.search_web(q_clean);
                web.truncate(1);
                results.extend(web);
            }

            return results;
        }

        Vec::new()
    }

    pub fn search_browser_history_async<F>(&self, query: &str, limit: usize, callback: F)
    where
        F: Fn(Vec<SearchResult>) + 'static,
    {
        let query = query.to_string();
        let (sender, receiver) = std::sync::mpsc::channel::<Vec<SearchResult>>();

        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(1), move || {
            match receiver.try_recv() {
                Ok(results) => {
                    callback(results);
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

        thread::spawn(move || {
            let results = query_browser_history_plain(&query, limit);
            let _ = sender.send(results);
        });
    }

    pub fn search_web(&self, query: &str) -> Vec<SearchResult> {
        // Instant web result (Google new-tab link). Browser-history lookup is
        // intentionally NOT done here: it opens SQLite on the UI thread and can
        // block on a locked places.sqlite. It runs asynchronously via
        // search_browser_history_async() instead.
        let q_strip = query.trim();
        let mut results = Vec::new();

        if !q_strip.is_empty() {
            let encoded = urlencoding::encode(q_strip);
            results.push(SearchResult {
                url: format!("https://www.google.com/search?q={}", encoded),
                title: format!("Search '{}' on the Web", q_strip),
                mime: "text/html".to_string(),
                snippet: format!("Open Google search for '{}'", q_strip),
                app: None,
            });
        } else {
            results.push(SearchResult {
                url: "https://www.google.com".to_string(),
                title: "Google".to_string(),
                mime: "text/html".to_string(),
                snippet: "https://www.google.com".to_string(),
                app: None,
            });
            results.push(SearchResult {
                url: "https://github.com".to_string(),
                title: "GitHub".to_string(),
                mime: "text/html".to_string(),
                snippet: "https://github.com".to_string(),
                app: None,
            });
            results.push(SearchResult {
                url: "https://youtube.com".to_string(),
                title: "YouTube".to_string(),
                mime: "text/html".to_string(),
                snippet: "https://youtube.com".to_string(),
                app: None,
            });
            results.push(SearchResult {
                url: "https://wikipedia.org".to_string(),
                title: "Wikipedia".to_string(),
                mime: "text/html".to_string(),
                snippet: "https://wikipedia.org".to_string(),
                app: None,
            });
        }

        results
    }

    pub fn search_apps(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let q_lower = query.to_lowercase();
        let apps = self.apps.borrow();
        let mut results = Vec::new();

        for app in apps.iter() {
            if q_lower.is_empty()
                || app.lower_name.contains(&q_lower)
                || app.lower_comment.contains(&q_lower)
                || app.lower_keywords.contains(&q_lower)
                || app.filename.to_lowercase().contains(&q_lower)
            {
                results.push(SearchResult {
                    url: format!("app://{}", app.filename),
                    title: app.name.clone(),
                    mime: "application/x-desktop".to_string(),
                    snippet: if !app.comment.is_empty() {
                        app.comment.clone()
                    } else {
                        app.filename.clone()
                    },
                    app: Some(app.clone()),
                });

                if results.len() >= limit {
                    break;
                }
            }
        }

        results
    }

    #[allow(dead_code)]
    pub fn search_sync(&self, query: &str, category: &str, limit: usize) -> Vec<SearchResult> {
        let mut results = self.search_instant(query, category, limit);
        if category == "apps" || category == "applications" || category == "clipboard" || category == "web" {
            return results;
        }

        results.extend(search_nautilus(query, category, limit));
        results
    }

    pub fn search_async<F>(&self, query: &str, category: &str, limit: usize, callback: F)
    where
        F: Fn(Vec<SearchResult>) + 'static,
    {
        if category == "apps" || category == "applications" || category == "clipboard" || category == "web" {
            gtk4::glib::idle_add_local(move || {
                callback(Vec::new());
                gtk4::glib::ControlFlow::Break
            });
            return;
        }

        let query = query.to_string();
        let category = category.to_string();

        let (sender, receiver) = std::sync::mpsc::channel::<Vec<SearchResult>>();

        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(1), move || {
            match receiver.try_recv() {
                Ok(results) => {
                    callback(results);
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

        thread::spawn(move || {
            let results = search_nautilus(&query, &category, limit);
            let _ = sender.send(results);
        });
    }

    #[allow(dead_code)]
    fn execute_sparql(&self, sparql: &str) -> Vec<SearchResult> {
        execute_sparql_external(sparql)
    }
}

pub fn matches_category(filename: &str, category: &str) -> bool {
    if category.is_empty() || category == "all" || category == "files" {
        return true;
    }
    let lower = filename.to_lowercase();
    let ext = lower.split('.').last().unwrap_or("");
    match category {
        "documents" => matches!(
            ext,
            "pdf" | "txt" | "md" | "doc" | "docx" | "odt" | "xls" | "xlsx" | "ods"
                | "ppt" | "pptx" | "odp" | "csv" | "rtf" | "epub" | "html" | "json"
                | "xml" | "yaml" | "yml" | "sh" | "py" | "c" | "cpp" | "h" | "rs"
                | "go" | "js" | "ts" | "toml" | "log" | "tex"
        ),
        "images" => matches!(
            ext,
            "png" | "jpg" | "jpeg" | "svg" | "webp" | "gif" | "avif" | "ico"
                | "bmp" | "tiff" | "heic" | "psd" | "raw"
        ),
        "audio" => matches!(
            ext,
            "mp3" | "flac" | "wav" | "ogg" | "m4a" | "aac" | "opus" | "wma"
                | "oga" | "mid" | "midi"
        ),
        "video" => matches!(
            ext,
            "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv" | "m4v"
                | "3gp" | "ts" | "mpg" | "mpeg"
        ),
        _ => true,
    }
}

pub fn search_nautilus(query: &str, category: &str, limit: usize) -> Vec<SearchResult> {
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
        let conn = ZbusConnection::session().ok()?;
        let reply = conn.call_method(
            Some("org.gnome.Nautilus"),
            "/org/gnome/Nautilus/SearchProvider",
            Some("org.gnome.Shell.SearchProvider2"),
            "GetInitialResultSet",
            &(terms,),
        ).ok()?;
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
        let filename_part = clean_url.split('/').last().unwrap_or(clean_url);
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

        let path_display = if clean_url.starts_with("file://") {
            let p = &clean_url[7..];
            urlencoding::decode(p).unwrap_or(std::borrow::Cow::Borrowed(p)).to_string()
        } else {
            clean_url.to_string()
        };

        results.push(SearchResult {
            url,
            title,
            mime,
            snippet: path_display,
            app: None,
        });

        if results.len() >= limit {
            break;
        }
    }

    results.sort_unstable_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    results
}

fn fallback_file_search(query: &str, limit: usize) -> Vec<String> {
    let q_lower = query.to_lowercase();
    let mut results = Vec::new();
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/jaime"));
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

fn execute_sparql_external(sparql: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();

    let output = Command::new("tinysparql")
        .args(&["query", "-b", "org.freedesktop.LocalSearch3", "-q", sparql])
        .output()
        .or_else(|_| {
            Command::new("tracker3")
                .args(&["sparql", "-q", sparql])
                .output()
        });

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("Resultados:") || trimmed.starts_with("Results:") {
                continue;
            }

            if trimmed.starts_with("file://") || trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                let url = trimmed.to_string();
                if seen.contains(&url) {
                    continue;
                }
                seen.insert(url.clone());
                let clean_url = url.trim_end_matches('/');
                let filename_part = clean_url.split('/').last().unwrap_or(clean_url);
                let title = match urlencoding::decode(filename_part) {
                    Ok(decoded) if !decoded.trim().is_empty() => decoded.into_owned(),
                    _ => filename_part.to_string(),
                };

                let mime = mime_guess::from_path(&title)
                    .first_raw()
                    .unwrap_or("application/octet-stream")
                    .to_string();

                results.push(SearchResult {
                    url,
                    title,
                    mime,
                    snippet: String::new(),
                    app: None,
                });
            }
        }
    }

    results.sort_unstable_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    results
}

pub fn get_indexing_status_plain() -> (bool, String, f64) {
    // Localsearch / Tracker daemon is disabled in Pulsar OS to save CPU/battery.
    // Nautilus search engine handles file queries without background indexing.
    (false, "Idle".to_string(), 1.0)
}

fn query_browser_history_plain(query: &str, limit: usize) -> Vec<SearchResult> {
    let mut results = Vec::new();
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/jaime"));

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

        if let Ok(conn) = SqliteConnection::open_with_flags(
            &db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        ) {
            // The browser holds an exclusive lock while running; never
            // let the UI block waiting for it
            let _ = conn.busy_timeout(std::time::Duration::from_millis(100));
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
                    results.push(SearchResult {
                        title,
                        snippet: url.clone(),
                        url,
                        mime: "text/html".to_string(),
                        app: None,
                    });
                }
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_history_plain_returns_empty_when_no_database() {
        let tmp = std::env::temp_dir().join("pulsar_spotlight_test_no_hist");
        let _ = std::fs::create_dir_all(&tmp);
        std::env::set_var("HOME", &tmp);
        let results = query_browser_history_plain("spotlight-test", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn matches_category_filters_correctly() {
        assert!(matches_category("document.pdf", "documents"));
        assert!(matches_category("notes.txt", "documents"));
        assert!(!matches_category("photo.jpg", "documents"));
        assert!(matches_category("photo.png", "images"));
        assert!(matches_category("song.mp3", "audio"));
        assert!(matches_category("clip.mp4", "video"));
        assert!(matches_category("anything.xyz", "all"));
    }
}
