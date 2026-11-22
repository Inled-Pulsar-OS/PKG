//! Window constants, zoom CSS, keycap widgets and icon helpers.
use super::*;

pub(super) const DEBOUNCE_MS: u64 = 80;
pub(super) const BASE_W: i32 = 712;
pub(super) const BASE_H: i32 = 580;
/// Idle pill size (before zoom).
pub(super) const PILL_W: i32 = 740;
pub(super) const PILL_H: i32 = 60;
/// Mode button diameter and the gap between bar items (before zoom).
pub(super) const MODE_BTN: f64 = 60.0;
pub(super) const MODE_GAP: f64 = 12.0;
/// Bar buttons: (shortcut number, tooltip). Order matches `shortcuts::mode_id`.
pub(super) const BAR_BUTTONS: [(usize, &str); 4] = [(1, "Applications"), (2, "Files"), (3, "Actions"), (4, "Clipboard")];
pub(super) const ZOOM_MIN: f64 = 0.8;
pub(super) const ZOOM_MAX: f64 = 1.6;

pub(super) fn pulsar_liquid_glass_enabled() -> bool {
    let Some(src) = gtk4::gio::SettingsSchemaSource::default() else { return true };
    if src.lookup("org.gnome.shell", true).is_none() {
        return true;
    }
    let settings = gtk4::gio::Settings::new("org.gnome.shell");
    let exts = settings.strv("enabled-extensions");
    exts.iter().any(|uuid| {
        let u = uuid.as_str();
        u == "liquid-glass@thinkingcoding1231.gmail.com"
            || u == "blur-my-shell@aunetx"
            || u.contains("blur")
            || u.contains("glass")
    }) || true
}

/// Load `data/icons/<name>` from the install dirs, falling back to the source tree.
pub(super) fn icon_file(name: &str) -> Option<PathBuf> {
    [get_local_icon_dir(), get_icon_dir(), PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/data/icons"))]
        .into_iter()
        .map(|d| d.join(name))
        .find(|f| f.exists())
}

pub(super) fn set_mode_icon(img: &gtk4::Image, glyph: &str) {
    match icon_file(&format!("mode-{glyph}-symbolic.svg")) {
        Some(f) => img.set_from_gicon(&gtk4::gio::FileIcon::new(&gtk4::gio::File::for_path(f))),
        None => img.set_icon_name(Some(if glyph == "more" { "view-more-symbolic" } else { "system-search-symbolic" })),
    }
}

pub(super) fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub(super) fn keycap(text: &str) -> gtk4::Label {
    let l = gtk4::Label::new(Some(text));
    l.add_css_class("keycap");
    l
}

pub(super) fn keycap_row(keys: &[String]) -> gtk4::Box {
    let b = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    for k in keys {
        b.append(&keycap(k));
    }
    b
}

pub(super) fn zoom_css(z: f64) -> String {
    let p = |v: f64| (v * z).round();
    format!(
        "window.spotlight-window .spotlight-main {{ font-size: {}px; }}
         window.spotlight-window entry.search-input, window.spotlight-window entry.search-input > text {{ font-size: {}px; }}
         window.spotlight-window .result-row {{ min-height: {}px; }}
         window.spotlight-window .result-title {{ font-size: {}px; }}
         window.spotlight-window .result-subtitle, window.spotlight-window .result-kind {{ font-size: {}px; }}
         window.spotlight-window .section-header {{ font-size: {}px; }}
         window.spotlight-window .icon-tile {{ min-width: {}px; min-height: {}px; }}
         window.spotlight-window .result-icon {{ -gtk-icon-size: {}px; }}
         window.spotlight-window .search-header {{ min-height: {}px; }}
         window.spotlight-window.idle .search-header {{ min-height: {}px; }}
         window.spotlight-window.idle entry.search-input, window.spotlight-window.idle entry.search-input > text {{ font-size: {}px; }}
         window.spotlight-window button.mode-button {{ min-width: {}px; min-height: {}px; -gtk-icon-size: {}px; }}
         window.spotlight-window .mode-buttons {{ margin-left: {}px; }}",
        p(13.0),
        p(30.0),
        p(56.0),
        p(17.0),
        p(13.0),
        p(12.0),
        p(44.0),
        p(44.0),
        p(40.0),
        p(76.0),
        p(PILL_H as f64),
        p(24.0),
        p(MODE_BTN),
        p(MODE_BTN),
        p(26.0),
        p(MODE_GAP)
    )
}

impl SpotlightWindow {
    pub(super) fn set_zoom(&self, z: f64) {
        let z = ((z * 10.0).round() / 10.0).clamp(ZOOM_MIN, ZOOM_MAX);
        self.zoom_provider.load_from_string(&zoom_css(z));
        self.apply_size(z);
        let mut c = self.config.borrow_mut();
        c.zoom = z;
        c.save();
    }
}
