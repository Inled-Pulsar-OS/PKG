use crate::config::SpotlightConfig;
use crate::search::SearchResult;
use gtk4::gdk;
use gtk4::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// Quiet period after a copy before history is written; copies inside it share one write.
const SAVE_DELAY: Duration = Duration::from_millis(1500);
/// Largest clipboard entry kept in history; longer text is cut on a char boundary.
pub const MAX_ITEM_BYTES: usize = 64 * 1024;
const SECRET_HINT_MIME: &str = "x-kde-passwordManagerHint";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClipboardItem {
    pub text: String,
    pub timestamp: f64,
}

#[derive(Clone)]
pub struct ClipboardManager {
    inner: Arc<Mutex<ClipboardManagerInner>>,
    /// Serializes disk writes so the worker thread and a shutdown flush never share the temp file.
    write_lock: Arc<Mutex<()>>,
    #[cfg(test)]
    writes: Arc<std::sync::atomic::AtomicUsize>,
}

struct ClipboardManagerInner {
    items: Vec<ClipboardItem>,
    config: SpotlightConfig,
    /// Resolved data directory for this session; `None` means the platform
    /// data-directory lookup failed, so history stays in-memory only.
    data_dir: Option<PathBuf>,
    /// History changed since the last write.
    dirty: bool,
    /// A delayed save thread is already waiting.
    save_pending: bool,
}

impl ClipboardManager {
    fn data_dir() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join("pulsaros-spotlight"))
    }

    fn clipboard_file_in(dir: &PathBuf) -> PathBuf {
        dir.join("clipboard_history.json")
    }

    pub fn new(config: SpotlightConfig) -> Self {
        Self::new_internal(config, Self::data_dir())
    }

    /// Test-only constructor that pins the data directory instead of resolving
    /// the real platform one, so tests never touch the user's actual files.
    #[cfg(test)]
    fn new_for_test(config: SpotlightConfig, dir: PathBuf) -> Self {
        Self::new_internal(config, Some(dir))
    }

    fn new_internal(config: SpotlightConfig, dir: Option<PathBuf>) -> Self {
        let mut items = Vec::new();
        match &dir {
            Some(dir) => {
                let file = Self::clipboard_file_in(dir);
                if file.exists() {
                    if let Ok(content) = fs::read_to_string(&file) {
                        if let Ok(loaded) = serde_json::from_str::<Vec<ClipboardItem>>(&content) {
                            items = loaded;
                        }
                    }
                }
            }
            None => {
                eprintln!(
                    "pulsaros-spotlight: could not resolve data directory; clipboard history will not persist this session"
                );
            }
        }

        let inner = Arc::new(Mutex::new(ClipboardManagerInner {
            items,
            config,
            data_dir: dir,
            dirty: false,
            save_pending: false,
        }));
        Self {
            inner,
            write_lock: Arc::new(Mutex::new(())),
            #[cfg(test)]
            writes: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    /// Writes the history now, whether or not it is dirty.
    pub fn save(&self) {
        let _guard = self.write_lock.lock().unwrap();
        let (dir, content) = {
            let mut inner = self.inner.lock().unwrap();
            let Some(dir) = inner.data_dir.clone() else { return };
            let Ok(content) = serde_json::to_string(&inner.items) else { return };
            inner.dirty = false;
            (dir, content)
        };
        if Self::ensure_private_dir(&dir).is_err() {
            return;
        }
        #[cfg(test)]
        self.writes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let _ = Self::write_private(&Self::clipboard_file_in(&dir), content.as_bytes());
    }

    /// Writes the history only if it changed since the last write. Call on shutdown.
    pub fn flush(&self) {
        if self.inner.lock().unwrap().dirty {
            self.save();
        }
    }

    /// Marks history dirty and, unless a worker is already waiting, starts one that
    /// writes it after `delay`.
    fn schedule_save(&self, delay: Duration) {
        {
            let mut inner = self.inner.lock().unwrap();
            inner.dirty = true;
            if inner.save_pending {
                return;
            }
            inner.save_pending = true;
        }
        let mgr = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(delay);
            mgr.inner.lock().unwrap().save_pending = false;
            mgr.flush();
        });
    }

    /// Creates `dir` (and parents) and locks it to 0700, so other local accounts
    /// can't list or traverse it, whatever the umask or a previous version left.
    fn ensure_private_dir(dir: &std::path::Path) -> std::io::Result<()> {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        fs::DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
    }

    /// Writes `bytes` to a 0600 temp file created with that mode (never readable by
    /// others, even briefly) and renames it over `path`, which also tightens a
    /// pre-existing looser file.
    fn write_private(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let tmp = path.with_extension("json.tmp");
        let _ = fs::remove_file(&tmp);
        let mut f = fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&tmp)?;
        f.write_all(bytes)?;
        drop(f);
        fs::rename(&tmp, path)
    }

    /// True when the clipboard offer marks its content as secret (password managers set
    /// `x-kde-passwordManagerHint`), so history must not record it.
    pub fn has_secret_hint<S: AsRef<str>>(mime_types: &[S]) -> bool {
        mime_types.iter().any(|m| m.as_ref() == SECRET_HINT_MIME)
    }

    /// Records `text` unless the offer's mime types flag it as a secret.
    pub fn record_clip_with_formats<S: AsRef<str>>(&self, text: &str, mime_types: &[S]) {
        if Self::has_secret_hint(mime_types) {
            return;
        }
        self.record_clip(text);
    }

    pub fn record_clip(&self, text: &str) {
        let mut clean = text.trim();
        if clean.is_empty() {
            return;
        }
        if clean.len() > MAX_ITEM_BYTES {
            let mut end = MAX_ITEM_BYTES;
            while !clean.is_char_boundary(end) {
                end -= 1;
            }
            clean = &clean[..end];
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

        drop(inner);
        self.schedule_save(SAVE_DELAY);
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

#[cfg(test)]
#[path = "tests/clipboard_tests.rs"]
mod tests;
