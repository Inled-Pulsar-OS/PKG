use crate::config::SpotlightConfig;
use crate::search::SearchResult;
use gtk4::gdk;
use gtk4::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClipboardItem {
    pub text: String,
    pub timestamp: f64,
}

#[derive(Clone)]
pub struct ClipboardManager {
    inner: Arc<Mutex<ClipboardManagerInner>>,
}

struct ClipboardManagerInner {
    items: Vec<ClipboardItem>,
    config: SpotlightConfig,
}

impl ClipboardManager {
    fn data_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("/home/jaime/.local/share"))
            .join("pulsaros-spotlight")
    }

    fn clipboard_file() -> PathBuf {
        Self::data_dir().join("clipboard_history.json")
    }

    pub fn new(config: SpotlightConfig) -> Self {
        let mut items = Vec::new();
        let file = Self::clipboard_file();
        if file.exists() {
            if let Ok(content) = fs::read_to_string(&file) {
                if let Ok(loaded) = serde_json::from_str::<Vec<ClipboardItem>>(&content) {
                    items = loaded;
                }
            }
        }

        let inner = Arc::new(Mutex::new(ClipboardManagerInner { items, config }));
        Self { inner }
    }

    pub fn save(&self) {
        let inner = self.inner.lock().unwrap();
        let dir = Self::data_dir();
        let _ = fs::create_dir_all(&dir);
        let file = Self::clipboard_file();
        if let Ok(content) = serde_json::to_string_pretty(&inner.items) {
            let _ = fs::write(&file, content);
        }
    }

    pub fn record_clip(&self, text: &str) {
        let clean = text.trim();
        if clean.is_empty() {
            return;
        }

        let mut inner = self.inner.lock().unwrap();
        // Avoid duplicate at top
        if let Some(first) = inner.items.first() {
            if first.text == clean {
                return;
            }
        }

        // Remove previous occurrences
        inner.items.retain(|item| item.text != clean);

        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);

        inner.items.insert(0, ClipboardItem {
            text: clean.to_string(),
            timestamp,
        });

        // Enforce max items
        let max_items = inner.config.clipboard_max_items;
        if inner.items.len() > max_items {
            inner.items.truncate(max_items);
        }

        // Save immediately
        drop(inner);
        self.save();
    }

    pub fn get_items(&self) -> Vec<ClipboardItem> {
        self.inner.lock().unwrap().items.clone()
    }

    pub fn get_clip_by_index(&self, index: usize) -> Option<String> {
        let inner = self.inner.lock().unwrap();
        inner.items.get(index).map(|item| item.text.clone())
    }

    pub fn search_history(&self, query: &str) -> Vec<SearchResult> {
        let items = self.get_items();
        let q_lower = query.trim().to_lowercase();
        let mut results = Vec::new();

        for (idx, item) in items.iter().enumerate() {
            if q_lower.is_empty() || item.text.to_lowercase().contains(&q_lower) {
                let lines: Vec<&str> = item.text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
                let mut first_line = lines.first().copied().unwrap_or(&item.text).to_string();
                if first_line.chars().count() > 60 {
                    first_line = first_line.chars().take(57).collect::<String>() + "...";
                }

                let mut preview = item.text.replace('\n', " ⏎ ");
                if preview.chars().count() > 100 {
                    preview = preview.chars().take(97).collect::<String>() + "...";
                }

                results.push(SearchResult {
                    url: format!("clipboard://{}", idx),
                    title: first_line,
                    mime: "text/plain-clipboard".to_string(),
                    snippet: preview,
                    app: None,
                });
            }
        }

        results
    }

    pub fn paste_clip(&self, text: &str) {
        if text.is_empty() {
            return;
        }

        // 1. Set to Gdk Clipboard
        if let Some(display) = gdk::Display::default() {
            display.clipboard().set_text(text);
        }

        // 2. Run wl-copy in background
        if let Ok(mut child) = Command::new("wl-copy")
            .stdin(Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(text.as_bytes());
            }
        }

        // 3. Simulate Paste (Ctrl+V) after window hides
        let auto_paste = self.inner.lock().unwrap().config.clipboard_auto_paste;
        if auto_paste {
            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                Self::simulate_paste();
                gtk4::glib::ControlFlow::Break
            });
        }
    }

    fn simulate_paste() {
        let which = |cmd: &str| -> Option<PathBuf> {
            std::env::var_os("PATH").and_then(|paths| {
                std::env::split_paths(&paths).filter_map(|dir| {
                    let full_path = dir.join(cmd);
                    if full_path.is_file() {
                        Some(full_path)
                    } else {
                        None
                    }
                }).next()
            })
        };

        // 1. Native Wayland uinput via ydotool
        if let Some(path) = which("ydotool") {
            let uid = unsafe { libc::getuid() };
            let sock = format!("/run/user/{}/.ydotool_socket", uid);
            let mut cmd = Command::new(path);
            cmd.args(&["key", "29:1", "47:1", "47:0", "29:0"]);
            if std::path::Path::new(&sock).exists() {
                cmd.env("YDOTOOL_SOCKET", sock);
            }
            if cmd.spawn().is_ok() {
                return;
            }
        }

        // 2. GNOME Shell Extension D-Bus key injection
        if let Ok(conn) = zbus::blocking::Connection::session() {
            let body = zbus::zvariant::Value::from(""); // empty or dummy
            // Make session call
            if conn.call_method(
                Some("org.gnome.Shell.Extensions.PulsarSpotlight"),
                "/org/gnome/Shell/Extensions/PulsarSpotlight",
                Some("org.gnome.Shell.Extensions.PulsarSpotlight"),
                "Paste",
                &body,
            ).is_ok() {
                return;
            }
        }

        // 3. Fallback: xdotool
        if let Some(path) = which("xdotool") {
            let _ = Command::new(path)
                .args(&["key", "ctrl+v"])
                .spawn();
        }
    }
}
