//! Loads the design-system CSS and switches theme files with light/dark.
//! The CSS is compiled in; set `PULSAR_STORE_CSS_DIR` to load files from disk while developing.

use gtk4 as gtk;
use std::path::PathBuf;

const BASE: &str = include_str!("../../assets/css/base.css");
const GLASS: &str = include_str!("../../assets/css/glass.css");
const LIGHT: &str = include_str!("../../assets/css/light.css");
const DARK: &str = include_str!("../../assets/css/dark.css");

fn read(name: &str, embedded: &'static str) -> String {
    std::env::var_os("PULSAR_STORE_CSS_DIR")
        .map(PathBuf::from)
        .and_then(|dir| std::fs::read_to_string(dir.join(name)).ok())
        .unwrap_or_else(|| embedded.to_string())
}

/// The full stylesheet for one theme: colors first, then components.
pub fn stylesheet(dark: bool) -> String {
    let theme = if dark {
        read("dark.css", DARK)
    } else {
        read("light.css", LIGHT)
    };
    [theme, read("base.css", BASE), read("glass.css", GLASS)].join("\n")
}

/// Installs the stylesheet on the default display and keeps it in step with the system theme.
pub fn install() {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let provider = gtk::CssProvider::new();
    let manager = adw::StyleManager::default();
    let apply = {
        let provider = provider.clone();
        move |m: &adw::StyleManager| provider.load_from_string(&stylesheet(m.is_dark()))
    };
    apply(&manager);
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    manager.connect_dark_notify(apply);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stylesheets_differ_by_theme_and_share_components() {
        let (light, dark) = (stylesheet(false), stylesheet(true));
        assert_ne!(light, dark);
        for css in [&light, &dark] {
            assert!(css.contains(".store-tile"));
            assert!(css.contains(".store-glass"));
        }
    }
}
