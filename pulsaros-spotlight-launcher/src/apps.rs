use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct DesktopApp {
    pub name: String,
    pub exec: String,
    pub icon: String,
    pub comment: String,
    pub categories: String,
    pub filename: String,
    pub lower_name: String,
    pub lower_comment: String,
    pub keywords: String,
    pub lower_keywords: String,
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
    dirs_list.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
    dirs_list.push(PathBuf::from("/var/lib/snapd/desktop/applications"));
    dirs_list
}

fn get_locale_keys() -> Vec<String> {
    let mut keys = Vec::new();
    let env_lang = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();

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
    let mut categories = None;

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
                } else if key == "Categories" {
                    categories = Some(value.to_string());
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

    let exec = exec?;
    if exec.is_empty() {
        return None;
    }

    let comment = pick_localized(&comments, &locales).unwrap_or_default();
    let keywords = pick_localized(&keywords_map, &locales).unwrap_or_default();

    Some(DesktopApp {
        lower_name: name.to_lowercase(),
        lower_comment: comment.to_lowercase(),
        lower_keywords: keywords.to_lowercase(),
        name,
        exec,
        icon: icon.unwrap_or_else(|| "application-x-executable".to_string()),
        comment,
        keywords,
        categories: categories.unwrap_or_default(),
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
                .filter(|p| p.extension().map_or(false, |ext| ext == "desktop"))
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

    apps
}
