use crate::apps::{DesktopApp, load_apps};
use crate::calculator::Calculator;
use crate::clipboard::ClipboardManager;
use rusqlite::{Connection as SqliteConnection, OpenFlags};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
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

    pub fn reload_apps(&self) {
        *self.apps.borrow_mut() = load_apps();
    }

    pub fn get_indexing_status(&self) -> (bool, String, f64) {
        let targets = &[
            ("org.freedesktop.LocalSearch3.Miner.Files", "/org/freedesktop/LocalSearch3/Miner/Files"),
            ("org.freedesktop.Tracker3.Miner.Files", "/org/freedesktop/Tracker3/Miner/Files"),
        ];

        if let Ok(conn) = ZbusConnection::session() {
            for &(bus_name, obj_path) in targets {
                if let Ok(status_reply) = conn.call_method(
                    Some(bus_name),
                    obj_path,
                    Some("org.freedesktop.Tracker3.Miner"),
                    "GetStatus",
                    &(),
                ) {
                    if let Ok((status,)) = status_reply.body().deserialize::<(String,)>() {
                        let status_lower = status.to_lowercase();
                        if !status_lower.contains("idle") && !status_lower.contains("inactivo") && !status_lower.contains("paused") {
                            if let Ok(progress_reply) = conn.call_method(
                                Some(bus_name),
                                obj_path,
                                Some("org.freedesktop.Tracker3.Miner"),
                                "GetProgress",
                                &(),
                            ) {
                                if let Ok((progress,)) = progress_reply.body().deserialize::<(f64,)>() {
                                    return (true, status, progress);
                                }
                            }
                            return (true, status, 0.0);
                        }
                    }
                }
            }
        }

        (false, "Idle".to_string(), 1.0)
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

    pub fn search_web(&self, query: &str) -> Vec<SearchResult> {
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

            results.extend(self.query_browser_history(q_strip, 5));
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

            results.extend(self.query_browser_history("", 6));
        }

        results
    }

    fn query_browser_history(&self, query: &str, limit: usize) -> Vec<SearchResult> {
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

    pub fn search_apps(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let q_lower = query.to_lowercase();
        let apps = self.apps.borrow().clone();
        let mut results = Vec::new();

        for app in apps {
            if q_lower.is_empty()
                || app.lower_name.contains(&q_lower)
                || app.lower_comment.contains(&q_lower)
            {
                results.push(SearchResult {
                    url: format!("app://{}", app.filename),
                    title: app.name.clone(),
                    mime: "application/x-desktop".to_string(),
                    snippet: app.comment.clone(),
                    app: Some(app),
                });

                if results.len() >= limit {
                    break;
                }
            }
        }

        results
    }

    pub fn search_sync(&self, query: &str, category: &str, limit: usize) -> Vec<SearchResult> {
        let mut results = self.search_instant(query, category, limit);
        if category == "apps" || category == "applications" || category == "clipboard" || category == "web" {
            return results;
        }

        let sparql = self.build_query(query, category, limit);
        results.extend(self.execute_sparql(&sparql));
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
        let sparql = self.build_query(&query, &category, limit);

        let (sender, receiver) = std::sync::mpsc::channel::<Vec<SearchResult>>();
        
        gtk4::glib::idle_add_local(move || {
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
            let results = execute_sparql_external(&sparql);
            let _ = sender.send(results);
        });
    }

    fn build_query(&self, query: &str, category: &str, limit: usize) -> String {
        let home_scope = r#"FILTER(STRSTARTS(?url, "file:///home/") || STRSTARTS(?url, "file:///media/") || STRSTARTS(?url, "file:///run/media/"))"#;
        let ignore_clutter = r#"FILTER(!CONTAINS(?url, "/node_modules/") && !CONTAINS(?url, "/.git/") && !CONTAINS(?url, "/.cache/"))"#;

        let filter_clause = if !query.trim().is_empty() {
            let q_clean = query.replace('"', "\\\"").to_lowercase();
            format!("{}\n    {}\n    FILTER(CONTAINS(LCASE(?url), \"{}\"))", home_scope, ignore_clutter, q_clean)
        } else {
            format!("{}\n    {}", home_scope, ignore_clutter)
        };

        let category_filters = [
            ("documents", r#"FILTER(REGEX(?url, "\\.(pdf|txt|md|doc|docx|odt|xls|xlsx|ods|ppt|pptx|odp|csv|rtf|epub|html|json|xml|yaml|yml|sh|py|c|cpp|h|rs|go|js|ts)$", "i"))"#),
            ("images", r#"FILTER(REGEX(?url, "\\.(png|jpg|jpeg|svg|webp|gif|avif|ico|bmp|tiff)$", "i"))"#),
            ("audio", r#"FILTER(REGEX(?url, "\\.(mp3|flac|wav|ogg|m4a|aac|opus|wma|oga)$", "i"))"#),
            ("video", r#"FILTER(REGEX(?url, "\\.(mp4|mkv|avi|mov|webm|flv|wmv|m4v|3gp)$", "i"))"#),
        ];

        let mut cat_filter = "";
        for &(cat_name, filter) in &category_filters {
            if cat_name == category {
                cat_filter = filter;
                break;
            }
        }

        if !cat_filter.is_empty() {
            format!(
                "SELECT DISTINCT ?url WHERE {{ ?u nie:url ?url . {} {} }} LIMIT {}",
                cat_filter, filter_clause, limit
            )
        } else {
            format!(
                "SELECT DISTINCT ?url WHERE {{ ?u nie:url ?url . {} }} LIMIT {}",
                filter_clause, limit
            )
        }
    }

    fn execute_sparql(&self, sparql: &str) -> Vec<SearchResult> {
        execute_sparql_external(sparql)
    }
}

fn execute_sparql_external(sparql: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();

    if let Ok(output) = Command::new("tinysparql")
        .args(&["query", "-b", "org.freedesktop.LocalSearch3", "-q", sparql])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut seen = std::collections::HashSet::new();

        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("Resultados:") {
                continue;
            }

            if trimmed.starts_with("file://") || trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                let url = trimmed.to_string();
                if seen.contains(&url) {
                    continue;
                }
                seen.insert(url.clone());

                let title = match urlencoding::decode(url.split('/').last().unwrap_or(&url)) {
                    Ok(decoded) => decoded.into_owned(),
                    Err(_) => url.clone(),
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

    results.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    results
}
