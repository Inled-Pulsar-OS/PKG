use crate::utils::get_file_icon;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;
use super::model::Payload;

pub(super) static ICON_CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();

pub(super) fn get_icon_cache() -> &'static Mutex<HashMap<String, Option<String>>> {
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

pub(super) fn find_icon_file_cached(name: &str) -> Option<String> {
    let cache = get_icon_cache();
    if let Some(hit) = cache.lock().unwrap().get(name) {
        return hit.clone();
    }
    // Never do synchronous directory crawling on the main GTK UI thread
    None
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


