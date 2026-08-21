use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

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
}

pub fn get_desktop_dirs() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/usr/share/applications"),
        dirs::data_dir().unwrap_or_else(|| PathBuf::from("/home/jaime/.local/share")).join("applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
        dirs::data_dir().unwrap_or_else(|| PathBuf::from("/home/jaime/.local/share")).join("flatpak/exports/share/applications"),
        PathBuf::from("/var/lib/snapd/desktop/applications"),
    ]
}

pub fn parse_desktop_file(path: &Path) -> Option<DesktopApp> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut in_desktop_entry = false;
    let mut app_type = None;
    let mut no_display = None;
    let mut hidden = None;
    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut comment = None;
    let mut categories = None;

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

                match key {
                    "Type" => app_type = Some(value.to_string()),
                    "NoDisplay" => no_display = Some(value.to_lowercase()),
                    "Hidden" => hidden = Some(value.to_lowercase()),
                    "Name" => name = Some(value.to_string()),
                    "Exec" => exec = Some(value.to_string()),
                    "Icon" => icon = Some(value.to_string()),
                    "Comment" => comment = Some(value.to_string()),
                    "Categories" => categories = Some(value.to_string()),
                    _ => {}
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

    let name = name?;
    let exec = exec?;

    Some(DesktopApp {
        lower_name: name.to_lowercase(),
        lower_comment: comment.as_deref().unwrap_or("").to_lowercase(),
        name,
        exec,
        icon: icon.unwrap_or_else(|| "application-x-executable".to_string()),
        comment: comment.unwrap_or_default(),
        categories: categories.unwrap_or_default(),
        filename: path.file_name()?.to_string_lossy().into_owned(),
    })
}

pub fn load_apps() -> Vec<DesktopApp> {
    let mut apps = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

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
                    if !seen_names.contains(&app.name) {
                        seen_names.insert(app.name.clone());
                        apps.push(app);
                    }
                }
            }
        }
    }

    apps
}
