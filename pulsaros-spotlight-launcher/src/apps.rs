use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct DesktopApp {
    pub name: String,
    pub icon: String,
    pub comment: String,
    pub filename: String,
    pub keywords: String,
}

pub fn get_desktop_dirs() -> Vec<PathBuf> {
    let mut dirs_list = vec![PathBuf::from("/usr/share/applications")];
    if let Some(data_dir) = dirs::data_dir() {
        dirs_list.push(data_dir.join("applications"));
        dirs_list.push(data_dir.join("flatpak/exports/share/applications"));
    } else if let Ok(home) = std::env::var("HOME") {
        dirs_list.push(PathBuf::from(format!("{}/.local/share/applications", home)));
        dirs_list.push(PathBuf::from(format!("{}/.local/share/flatpak/exports/share/applications", home)));
    }
    if let Ok(home) = std::env::var("HOME") {
        dirs_list.push(PathBuf::from(format!("{}/Applications", home)));
    }
    dirs_list.push(PathBuf::from("/Applications"));
    dirs_list.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
    dirs_list.push(PathBuf::from("/var/lib/snapd/desktop/applications"));
    dirs_list
}

fn get_locale_keys() -> Vec<String> {
    let mut keys = Vec::new();
    let mut env_lang = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();

    // Check user and system locale configuration files as source of truth
    if let Ok(home) = std::env::var("HOME") {
        let user_loc = format!("{}/.config/locale.conf", home);
        if let Ok(content) = std::fs::read_to_string(&user_loc) {
            for line in content.lines() {
                if line.starts_with("LANG=") {
                    let val = line.trim_start_matches("LANG=").trim();
                    if !val.is_empty() {
                        env_lang = val.to_string();
                        break;
                    }
                }
            }
        }
    }
    if env_lang.is_empty() || env_lang == "C" || env_lang == "POSIX" {
        if let Ok(content) = std::fs::read_to_string("/etc/locale.conf") {
            for line in content.lines() {
                if line.starts_with("LANG=") {
                    let val = line.trim_start_matches("LANG=").trim();
                    if !val.is_empty() {
                        env_lang = val.to_string();
                        break;
                    }
                }
            }
        }
    }

    if !env_lang.is_empty() {
        let clean = env_lang.split('.').next().unwrap_or("").split('@').next().unwrap_or("");
        if !clean.is_empty() {
            keys.push(clean.to_string());
            if let Some(lang_only) = clean.split('_').next() {
                if lang_only != clean && !lang_only.is_empty() {
                    keys.push(lang_only.to_string());
                }
            }
        }
    }
    keys
}

fn pick_localized(map: &HashMap<String, String>, locales: &[String]) -> Option<String> {
    for loc in locales {
        if let Some(val) = map.get(loc) {
            if !val.trim().is_empty() {
                return Some(val.trim().to_string());
            }
        }
    }
    map.get("").map(|v| v.trim().to_string())
}

pub fn parse_desktop_file(path: &Path) -> Option<DesktopApp> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut in_desktop_entry = false;
    let mut app_type = None;
    let mut no_display = None;
    let mut hidden = None;
    let mut exec = None;
    let mut icon = None;

    let mut names: HashMap<String, String> = HashMap::new();
    let mut generic_names: HashMap<String, String> = HashMap::new();
    let mut comments: HashMap<String, String> = HashMap::new();
    let mut keywords_map: HashMap<String, String> = HashMap::new();

    for line_result in reader.lines() {
        let line = line_result.ok()?;
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = &trimmed[1..trimmed.len() - 1];
            in_desktop_entry = section == "Desktop Entry";
            continue;
        }

        if in_desktop_entry {
            if let Some(pos) = trimmed.find('=') {
                let key = trimmed[..pos].trim();
                let value = trimmed[pos + 1..].trim();

                if key == "Type" {
                    app_type = Some(value.to_string());
                } else if key == "NoDisplay" {
                    no_display = Some(value.to_lowercase());
                } else if key == "Hidden" {
                    hidden = Some(value.to_lowercase());
                } else if key == "Exec" {
                    exec = Some(value.to_string());
                } else if key == "Icon" {
                    icon = Some(value.to_string());
                } else if key == "Name" {
                    names.insert("".to_string(), value.to_string());
                } else if key.starts_with("Name[") && key.ends_with(']') {
                    let loc = &key[5..key.len() - 1];
                    names.insert(loc.to_string(), value.to_string());
                } else if key == "GenericName" {
                    generic_names.insert("".to_string(), value.to_string());
                } else if key.starts_with("GenericName[") && key.ends_with(']') {
                    let loc = &key[12..key.len() - 1];
                    generic_names.insert(loc.to_string(), value.to_string());
                } else if key == "Comment" {
                    comments.insert("".to_string(), value.to_string());
                } else if key.starts_with("Comment[") && key.ends_with(']') {
                    let loc = &key[8..key.len() - 1];
                    comments.insert(loc.to_string(), value.to_string());
                } else if key == "Keywords" {
                    keywords_map.insert("".to_string(), value.to_string());
                } else if key.starts_with("Keywords[") && key.ends_with(']') {
                    let loc = &key[9..key.len() - 1];
                    keywords_map.insert(loc.to_string(), value.to_string());
                }
            }
        }
    }

    if app_type.as_deref().unwrap_or("Application") != "Application" {
        return None;
    }
    if no_display.as_deref() == Some("true") || no_display.as_deref() == Some("1") {
        return None;
    }
    if hidden.as_deref() == Some("true") || hidden.as_deref() == Some("1") {
        return None;
    }

    let locales = get_locale_keys();
    let name = pick_localized(&names, &locales)
        .or_else(|| pick_localized(&generic_names, &locales))?;

    if name.is_empty() {
        return None;
    }

    exec.filter(|e| !e.is_empty())?;

    let comment = pick_localized(&comments, &locales).unwrap_or_default();
    let keywords = pick_localized(&keywords_map, &locales).unwrap_or_default();

    Some(DesktopApp {
        name,
        icon: icon.unwrap_or_else(|| "application-x-executable".to_string()),
        comment,
        keywords,
        filename: path.file_name()?.to_string_lossy().into_owned(),
    })
}

pub fn load_apps() -> Vec<DesktopApp> {
    let mut apps = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for dir in get_desktop_dirs() {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut paths: Vec<_> = entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|ext| ext == "desktop"))
                .collect();
            paths.sort();

            for path in paths {
                if let Some(app) = parse_desktop_file(&path) {
                    let app_id = app.filename.clone();
                    if !seen_ids.contains(&app_id) {
                        seen_ids.insert(app_id);
                        apps.push(app);
                    }
                }
            }
        }
    }

    // Scan ~/Applications and /Applications for native macOS .app bundles
    let mut mac_search_dirs = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        mac_search_dirs.push(PathBuf::from(format!("{}/Applications", home)));
    }
    mac_search_dirs.push(PathBuf::from("/Applications"));

    for dir in mac_search_dirs {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() && p.extension().map_or(false, |ext| ext == "app") {
                    let app_id = p.file_name().unwrap_or_default().to_string_lossy().into_owned();
                    if !seen_ids.contains(&app_id) {
                        if let Some(app) = parse_mac_app(&p) {
                            seen_ids.insert(app_id);
                            apps.push(app);
                        }
                    }
                }
            }
        }
    }

    apps
}

fn extract_png_from_icns(icns_path: &Path, out_png: &Path) -> bool {
    if let Ok(bytes) = std::fs::read(icns_path) {
        let png_header = b"\x89PNG\r\n\x1a\n";
        let iend_marker = b"IEND";
        let mut best_png: Option<&[u8]> = None;
        let mut pos = 0;

        while let Some(start_offset) = bytes[pos..].windows(8).position(|w| w == png_header) {
            let actual_start = pos + start_offset;
            if let Some(end_offset) = bytes[actual_start..].windows(4).position(|w| w == iend_marker) {
                let actual_end = actual_start + end_offset + 8;
                let slice = &bytes[actual_start..actual_end];
                if best_png.map_or(true, |b| slice.len() > b.len()) {
                    best_png = Some(slice);
                }
                pos = actual_end;
            } else {
                break;
            }
        }

        if let Some(png) = best_png {
            if let Some(parent) = out_png.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if std::fs::write(out_png, png).is_ok() {
                return true;
            }
        }
    }
    false
}

fn parse_mac_app(path: &Path) -> Option<DesktopApp> {
    if !path.is_dir() {
        return None;
    }
    let folder_name = path.file_name()?.to_string_lossy();
    if !folder_name.ends_with(".app") {
        return None;
    }
    let clean_name = folder_name.strip_suffix(".app").unwrap_or(&folder_name);

    let mut display_name = clean_name.to_string();
    let mut icon_path = String::new();

    let plist_path = path.join("Contents/Info.plist");
    if let Ok(content) = std::fs::read_to_string(&plist_path) {
        if let Some(pos) = content.find("<key>CFBundleDisplayName</key>") {
            if let Some(str_start) = content[pos..].find("<string>") {
                if let Some(str_end) = content[pos + str_start..].find("</string>") {
                    display_name = content[pos + str_start + 8..pos + str_start + str_end].trim().to_string();
                }
            }
        } else if let Some(pos) = content.find("<key>CFBundleName</key>") {
            if let Some(str_start) = content[pos..].find("<string>") {
                if let Some(str_end) = content[pos + str_start..].find("</string>") {
                    display_name = content[pos + str_start + 8..pos + str_start + str_end].trim().to_string();
                }
            }
        }

        let mut raw_icon = String::new();
        if let Some(pos) = content.find("<key>CFBundleIconFile</key>") {
            if let Some(str_start) = content[pos..].find("<string>") {
                if let Some(str_end) = content[pos + str_start..].find("</string>") {
                    raw_icon = content[pos + str_start + 8..pos + str_start + str_end].trim().to_string();
                }
            }
        }
        if !raw_icon.is_empty() && !raw_icon.ends_with(".icns") {
            raw_icon.push_str(".icns");
        }

        let mut icns_file = if !raw_icon.is_empty() {
            path.join("Contents/Resources").join(raw_icon)
        } else {
            path.join("Contents/Resources/AppIcon.icns")
        };

        if !icns_file.exists() {
            if let Ok(res_entries) = std::fs::read_dir(path.join("Contents/Resources")) {
                for r in res_entries.flatten() {
                    if r.path().extension().map_or(false, |ext| ext == "icns") {
                        icns_file = r.path();
                        break;
                    }
                }
            }
        }

        if icns_file.exists() {
            if let Ok(home) = std::env::var("HOME") {
                let cache_icon = PathBuf::from(format!("{}/.cache/pulsar-icons/{}.png", home, clean_name));
                if cache_icon.exists() || extract_png_from_icns(&icns_file, &cache_icon) {
                    icon_path = cache_icon.to_string_lossy().into_owned();
                }
            }
        }
    }

    if icon_path.is_empty() {
        icon_path = "application-x-executable".to_string();
    }

    let comment = "macOS Application".to_string();

    Some(DesktopApp {
        name: display_name,
        icon: icon_path,
        comment,
        keywords: format!("macos apple app {}", clean_name),
        filename: folder_name.to_string(),
    })
}

