//! Internationalization (ADR 0006). Every user-facing string goes through
//! `tr!`, `ntr!`, or `ptr!`. Source strings are English.
//!
//! Set `PULSAR_STORE_PSEUDO=1` to render every string in a pseudo-locale
//! (accented, padded, bracketed) to spot untranslated text and truncation.
//! Set `PULSAR_STORE_RTL=1` to force right-to-left layout.

use gettextrs::{bindtextdomain, setlocale, textdomain, LocaleCategory};
use std::path::PathBuf;
use std::sync::OnceLock;

pub const DOMAIN: &str = "pulsar-store";

/// Binds the text domain and selects the locale from the environment.
/// Call once, before building any UI.
pub fn init() {
    // SAFETY: called once at startup, before any other thread reads the locale.
    unsafe {
        setlocale(LocaleCategory::LcAll, "");
    }
    // Failure leaves strings untranslated, which is a fine fallback.
    let _ = bindtextdomain(DOMAIN, locale_dir());
    let _ = textdomain(DOMAIN);
}

fn locale_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("PULSAR_STORE_LOCALE_DIR") {
        return dir.into();
    }
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/locale");
    if cfg!(debug_assertions) && dev.is_dir() {
        return dev;
    }
    "/usr/share/locale".into()
}

fn pseudo_enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("PULSAR_STORE_PSEUDO").is_some_and(|v| v != "0"))
}

pub fn gettext(msgid: &str) -> String {
    finish(gettextrs::gettext(msgid))
}

pub fn ngettext(singular: &str, plural: &str, n: u32) -> String {
    finish(gettextrs::ngettext(singular, plural, n))
}

pub fn pgettext(context: &str, msgid: &str) -> String {
    finish(gettextrs::pgettext(context, msgid))
}

fn finish(s: String) -> String {
    if pseudo_enabled() {
        pseudo(&s)
    } else {
        s
    }
}

/// Replaces `{name}` placeholders. Translators keep placeholders unchanged.
pub fn format(template: &str, args: &[(&str, &str)]) -> String {
    args.iter().fold(template.to_string(), |s, (k, v)| {
        s.replace(&format!("{{{k}}}"), v)
    })
}

/// Pseudo-locale: accents letters, pads by about 30%, and brackets the text.
/// Placeholders such as `{name}` are left intact.
pub fn pseudo(s: &str) -> String {
    let mut out = String::from("[");
    let mut in_placeholder = false;
    for c in s.chars() {
        match c {
            '{' => in_placeholder = true,
            '}' => in_placeholder = false,
            _ => {}
        }
        out.push(if in_placeholder { c } else { accent(c) });
    }
    let pad = s.chars().count() * 3 / 10;
    out.extend(std::iter::repeat('~').take(pad));
    out.push(']');
    out
}

fn accent(c: char) -> char {
    match c {
        'a' => 'á',
        'e' => 'é',
        'i' => 'í',
        'o' => 'ó',
        'u' => 'ú',
        'A' => 'Á',
        'E' => 'É',
        'I' => 'Í',
        'O' => 'Ó',
        'U' => 'Ú',
        'c' => 'ç',
        'n' => 'ñ',
        _ => c,
    }
}

/// True for languages written right to left. `lang` is a locale such as `ar_EG.UTF-8` or `he`.
pub fn is_rtl(lang: &str) -> bool {
    let code = lang.split(['_', '-', '.']).next().unwrap_or("");
    matches!(code, "ar" | "he" | "fa" | "ur" | "ps" | "sd" | "yi" | "ckb")
}

/// Whether the UI should lay out right to left, from `PULSAR_STORE_RTL` or the locale.
pub fn wants_rtl() -> bool {
    if let Some(v) = std::env::var_os("PULSAR_STORE_RTL") {
        return v != "0";
    }
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
        .is_some_and(|v| is_rtl(&v))
}

/// BCP 47 tags for a POSIX locale string: `es_DO.UTF-8@euro` gives `["es-DO", "es"]`.
/// `C` and `POSIX` give none.
pub fn parse_locale(value: &str) -> Vec<String> {
    let base = value.split(['.', '@']).next().unwrap_or("");
    if base.is_empty() || base == "C" || base == "POSIX" {
        return Vec::new();
    }
    let tag = base.replace('_', "-");
    let lang = tag.split('-').next().unwrap_or("").to_string();
    if lang == tag {
        vec![tag]
    } else {
        vec![tag, lang]
    }
}

/// The user's preferred locales, most preferred first: `LANGUAGE`, then
/// `LC_ALL`, `LC_MESSAGES`, `LANG`.
pub fn locales() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |tags: Vec<String>| {
        for t in tags {
            if !out.contains(&t) {
                out.push(t);
            }
        }
    };
    if let Ok(list) = std::env::var("LANGUAGE") {
        list.split(':').for_each(|l| push(parse_locale(l)));
    }
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(key) {
            push(parse_locale(&v));
        }
    }
    out
}

/// Translates a string literal.
#[macro_export]
macro_rules! tr {
    ($msgid:literal) => {
        $crate::core::i18n::gettext($msgid)
    };
}

/// Translates with plural forms: `ntr!("{n} update", "{n} updates", n)`.
#[macro_export]
macro_rules! ntr {
    ($singular:literal, $plural:literal, $n:expr) => {
        $crate::core::i18n::ngettext($singular, $plural, $n)
    };
}

/// Translates with a disambiguating context: `ptr!("button", "Open")`.
#[macro_export]
macro_rules! ptr {
    ($context:literal, $msgid:literal) => {
        $crate::core::i18n::pgettext($context, $msgid)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_replaces_named_placeholders() {
        assert_eq!(
            format("Install {name} now", &[("name", "GIMP")]),
            "Install GIMP now"
        );
    }

    #[test]
    fn pseudo_keeps_placeholders_and_brackets() {
        let p = pseudo("Install {name}");
        assert!(p.starts_with('[') && p.ends_with(']'));
        assert!(p.contains("{name}"));
        assert!(p.contains("Íñstáll"));
    }

    #[test]
    fn pseudo_is_longer_than_source() {
        assert!(
            pseudo("Updates available").chars().count() > "Updates available".chars().count() + 2
        );
    }

    #[test]
    fn parses_posix_locales() {
        assert_eq!(parse_locale("es_DO.UTF-8@euro"), ["es-DO", "es"]);
        assert_eq!(parse_locale("en"), ["en"]);
        assert!(parse_locale("C").is_empty());
        assert!(parse_locale("POSIX").is_empty());
        assert!(parse_locale("").is_empty());
    }

    #[test]
    fn rtl_detection() {
        assert!(is_rtl("ar_EG.UTF-8"));
        assert!(is_rtl("he"));
        assert!(!is_rtl("es_DO.UTF-8"));
        assert!(!is_rtl("C"));
    }

    #[test]
    fn macros_pass_through_without_catalog() {
        assert_eq!(crate::tr!("Open"), "Open");
        assert_eq!(crate::ntr!("{n} update", "{n} updates", 2), "{n} updates");
        assert_eq!(crate::ptr!("button", "Open"), "Open");
    }
}
