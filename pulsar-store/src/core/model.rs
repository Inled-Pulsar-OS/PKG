//! Domain types shared by every layer. No I/O.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Text that is either a plain string or a map of BCP 47 tag to string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LocalizedText {
    Plain(String),
    Map(BTreeMap<String, String>),
}

impl Default for LocalizedText {
    fn default() -> Self {
        Self::Plain(String::new())
    }
}

impl LocalizedText {
    /// Every translation, for matching searches across languages.
    pub fn texts(&self) -> Vec<&str> {
        match self {
            Self::Plain(s) => vec![s.as_str()],
            Self::Map(m) => m.values().map(String::as_str).collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Plain(s) => s.is_empty(),
            Self::Map(m) => m.values().all(String::is_empty),
        }
    }

    /// Picks text for the first matching locale in `locales` (BCP 47 tags such
    /// as `es-DO`), trying the tag, then its language, then `en`, then any
    /// entry. Returns an empty string when there is no text.
    pub fn resolve(&self, locales: &[&str]) -> &str {
        match self {
            Self::Plain(s) => s,
            Self::Map(m) => {
                for tag in locales {
                    if let Some(s) = m.get(*tag) {
                        return s;
                    }
                    let lang = tag.split(['-', '_']).next().unwrap_or(tag);
                    if let Some(s) = m.get(lang) {
                        return s;
                    }
                }
                m.get("en")
                    .or_else(|| m.values().next())
                    .map_or("", String::as_str)
            }
        }
    }
}

/// Where an item is installed from. Catalog aliases map onto these kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    #[serde(alias = "app", alias = "desktop_app")]
    Flatpak,
    System,
    #[serde(alias = "extension")]
    GnomeExtension,
    SayriSkill,
    SayriPlugin,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EditionKey {
    Debian,
    Arch,
    Flatpak,
}

/// An installable form of an item. Display labels belong to the UI layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edition {
    pub key: EditionKey,
    pub url: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Formats {
    pub deb: Option<String>,
    pub arch: Option<String>,
    pub pacman: Option<String>,
    pub flatpak: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    pub flatpakref_url: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// One catalog entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: SourceKind,
    #[serde(default)]
    pub name: LocalizedText,
    #[serde(default)]
    pub summary: LocalizedText,
    #[serde(default)]
    pub description: LocalizedText,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub version: Option<String>,
    pub package_name: Option<String>,
    #[serde(default)]
    pub download_url: String,
    #[serde(default)]
    pub formats: Formats,
    #[serde(default)]
    pub metadata: Metadata,
    /// Fields this version does not know about; kept so data round-trips.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catalog {
    #[serde(default, alias = "items")]
    pub packages: Vec<Item>,
    pub announcement: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallState {
    NotInstalled,
    Installed {
        version: Option<String>,
    },
    UpdateAvailable {
        installed: String,
        available: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Preparing,
    Downloading,
    Installing,
    Finalizing,
}

/// A progress report. `fraction` is `None` when the amount is unknown.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Progress {
    pub phase: Phase,
    pub fraction: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: String,
    pub edition: Option<EditionKey>,
    pub package_name: Option<String>,
    pub version: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> LocalizedText {
        LocalizedText::Map(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
    }

    #[test]
    fn plain_text_ignores_locale() {
        let t = LocalizedText::Plain("Hello".into());
        assert_eq!(t.resolve(&["es"]), "Hello");
    }

    #[test]
    fn resolve_prefers_exact_then_language_then_en_then_any() {
        let t = map(&[("en", "Hello"), ("es", "Hola"), ("es-DO", "Qué lo qué")]);
        assert_eq!(t.resolve(&["es-DO"]), "Qué lo qué");
        assert_eq!(t.resolve(&["es-MX"]), "Hola");
        assert_eq!(t.resolve(&["fr"]), "Hello");
        assert_eq!(map(&[("de", "Hallo")]).resolve(&["fr"]), "Hallo");
        assert_eq!(map(&[]).resolve(&["fr"]), "");
    }

    #[test]
    fn item_parses_python_catalog_shape() {
        let json = r#"{
            "packages": [{
                "id": "org.gimp.GIMP", "type": "desktop_app", "name": "GIMP",
                "summary": {"en": "Image editor", "es": "Editor de imágenes"},
                "category": "Graphics", "tags": ["image"],
                "formats": {"deb": "https://x/gimp.deb"},
                "metadata": {"flatpakref_url": "https://x/gimp.flatpakref", "stars": 5},
                "future_field": 1
            }]
        }"#;
        let c: Catalog = serde_json::from_str(json).unwrap();
        let i = &c.packages[0];
        assert_eq!(i.kind, SourceKind::Flatpak);
        assert_eq!(i.summary.resolve(&["es"]), "Editor de imágenes");
        assert_eq!(i.formats.deb.as_deref(), Some("https://x/gimp.deb"));
        assert_eq!(i.metadata.extra["stars"], 5);
        assert!(i.extra.contains_key("future_field"));
    }

    #[test]
    fn unknown_type_does_not_fail_parsing() {
        let i: Item = serde_json::from_str(r#"{"id":"x","type":"hologram"}"#).unwrap();
        assert_eq!(i.kind, SourceKind::Unknown);
    }

    #[test]
    fn items_key_is_accepted() {
        let c: Catalog = serde_json::from_str(r#"{"items":[{"id":"x","type":"system"}]}"#).unwrap();
        assert_eq!(c.packages.len(), 1);
    }

    #[test]
    fn item_round_trips() {
        let src = r#"{"id":"x","type":"gnome_extension","extra_thing":[1,2]}"#;
        let item: Item = serde_json::from_str(src).unwrap();
        let again: Item = serde_json::from_str(&serde_json::to_string(&item).unwrap()).unwrap();
        assert_eq!(item, again);
    }
}
