use gtk4::gio;
use gtk4::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static EXT_MAP: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
        let mut m = HashMap::new();
        m.insert("py", "file_python.svg");
        m.insert("pyw", "file_python.svg");
        m.insert("sh", "file_shell.svg");
        m.insert("bash", "file_shell.svg");
        m.insert("zsh", "file_shell.svg");
        m.insert("fish", "file_shell.svg");
        m.insert("js", "file_javascript.svg");
        m.insert("mjs", "file_javascript.svg");
        m.insert("cjs", "file_javascript.svg");
        m.insert("ts", "file_typescript.svg");
        m.insert("tsx", "file_typescript.svg");
        m.insert("jsx", "file_javascript.svg");
        m.insert("html", "file_html.svg");
        m.insert("htm", "file_html.svg");
        m.insert("css", "file_css.svg");
        m.insert("scss", "file_css.svg");
        m.insert("sass", "file_css.svg");
        m.insert("c", "file_c.svg");
        m.insert("h", "file_c.svg");
        m.insert("cpp", "file_cpp.svg");
        m.insert("hpp", "file_cpp.svg");
        m.insert("cc", "file_cpp.svg");
        m.insert("cs", "file_csharp.svg");
        m.insert("rs", "file_rust.svg");
        m.insert("go", "file_go.svg");
        m.insert("java", "file_java.svg");
        m.insert("jar", "file_package.svg");
        m.insert("php", "file_php.svg");
        m.insert("rb", "file_ruby.svg");
        m.insert("lua", "file_lua.svg");
        m.insert("sql", "file_sql.svg");
        m.insert("json", "file_json.svg");
        m.insert("xml", "file_xml.svg");
        m.insert("yaml", "file_yaml.svg");
        m.insert("yml", "file_yaml.svg");
        m.insert("md", "file_markdown.svg");
        m.insert("markdown", "file_markdown.svg");
        m.insert("txt", "file_text.svg");
        m.insert("pdf", "file_pdf.svg");
        m.insert("doc", "file_word.svg");
        m.insert("docx", "file_word.svg");
        m.insert("odt", "file_word.svg");
        m.insert("xls", "file_excel.svg");
        m.insert("xlsx", "file_excel.svg");
        m.insert("ods", "file_excel.svg");
        m.insert("csv", "file_excel.svg");
        m.insert("ppt", "file_powerpoint.svg");
        m.insert("pptx", "file_powerpoint.svg");
        m.insert("odp", "file_powerpoint.svg");
        m.insert("whl", "file_package.svg");
        m.insert("tar", "file_package.svg");
        m.insert("gz", "file_package.svg");
        m.insert("xz", "file_package.svg");
        m.insert("zst", "file_package.svg");
        m.insert("zip", "file_package.svg");
        m.insert("pkg", "file_package.svg");
        m.insert("deb", "file_package.svg");
        m
});

pub fn open_file(url: &str) -> bool {
    let clean_url = url.trim_end_matches('/');

    // Handle macOS .app bundles (e.g. app://CotEditor.app, file:///path/to/CotEditor.app, /path/to/CotEditor.app)
    if clean_url.ends_with(".app") {
        let path_str = if clean_url.starts_with("file://") {
            &clean_url[7..]
        } else if clean_url.starts_with("app://") {
            &clean_url[6..]
        } else {
            clean_url
        };

        let pbuf = PathBuf::from(path_str);
        let app_path = if pbuf.is_absolute() && pbuf.exists() {
            pbuf
        } else {
            let mut found = None;
            if let Ok(home) = std::env::var("HOME") {
                let h = PathBuf::from(home).join("Applications").join(path_str);
                if h.exists() {
                    found = Some(h);
                }
            }
            if found.is_none() {
                let a = PathBuf::from("/Applications").join(path_str);
                if a.exists() {
                    found = Some(a);
                }
            }
            found.unwrap_or(pbuf)
        };

        if app_path.exists() {
            return std::process::Command::new("/usr/bin/pulsar-appexec")
                .arg(&app_path)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .is_ok();
        }
        return false;
    }

    if let Some(desktop_filename) = url.strip_prefix("app://") {
        if let Some(app_info) = gio::DesktopAppInfo::new(desktop_filename) {
            return app_info.launch(&[], None::<&gio::AppLaunchContext>).is_ok();
        }
        // Fallback search
        let user_apps = dirs::data_dir().map(|d| d.join("applications"));
        for dir in std::iter::once(PathBuf::from("/usr/share/applications")).chain(user_apps) {
            let path = dir.join(desktop_filename);
            if path.exists() {
                if let Some(app_info) = gio::DesktopAppInfo::from_filename(path) {
                    return app_info.launch(&[], None::<&gio::AppLaunchContext>).is_ok();
                }
            }
        }
        return false;
    }

    gio::AppInfo::launch_default_for_uri(url, None::<&gio::AppLaunchContext>).is_ok()
}

pub fn get_icon_dir() -> PathBuf {
    PathBuf::from("/usr/share/pulsaros-spotlight/icons")
}

pub fn get_local_icon_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".local/share/pulsaros-spotlight/icons"))
        .unwrap_or_else(|| PathBuf::from("/usr/share/pulsaros-spotlight/icons"))
}

pub fn get_file_icon(url: &str, mime: Option<&str>, is_dir: bool) -> gtk4::Image {
    // Web results (search suggestions, browser history)
    if url.starts_with("http://") || url.starts_with("https://") {
        return gtk4::Image::from_icon_name("web-browser-symbolic");
    }

    if is_dir || mime == Some("inode/directory") || mime == Some("folder") {
        for base in &[get_local_icon_dir(), get_icon_dir()] {
            let f = base.join("folder.svg");
            if f.exists() {
                return gtk4::Image::from_file(f);
            }
        }
        return gtk4::Image::from_icon_name("folder");
    }

    let clean_path = url.trim_start_matches("file://");
    let ext = Path::new(clean_path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    if let Some(icon_file) = EXT_MAP.get(ext.as_str()) {
        for base in &[get_local_icon_dir(), get_icon_dir()] {
            let f = base.join(icon_file);
            if f.exists() {
                return gtk4::Image::from_file(f);
            }
        }
    }

    // Ask the system theme for an icon matching the file's content type
    let (content_type, _) = gtk4::gio::content_type_guess(Some(clean_path), &[]);
    let themed = gtk4::gio::content_type_get_symbolic_icon(&content_type);
    gtk4::Image::from_gicon(&themed)
}
