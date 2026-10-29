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

/// The security audit the catalog publishes for an item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityReport {
    pub score: Option<u32>,
    pub status: Option<String>,
    pub audited_by: Option<String>,
    pub summary: Option<String>,
}

impl Item {
    pub fn security_report(&self) -> Option<SecurityReport> {
        serde_json::from_value(self.extra.get("security_report")?.clone()).ok()
    }

    /// Embedded documentation: `raw_content`, else `skill_md`.
    pub fn inline_docs(&self) -> Option<&str> {
        ["raw_content", "skill_md"]
            .iter()
            .find_map(|k| self.extra.get(*k).and_then(Value::as_str))
            .filter(|s| !s.trim().is_empty())
    }

    /// Checksum the catalog declares for the download, if any.
    pub fn declared_sha256(&self) -> Option<&str> {
        self.extra
            .get("sha256")
            .or_else(|| self.metadata.extra.get("sha256"))
            .and_then(Value::as_str)
    }

    /// GNOME extension UUID: `metadata.uuid`, then `uuid`, then the item id.
    pub fn extension_uuid(&self) -> &str {
        self.metadata
            .extra
            .get("uuid")
            .or_else(|| self.extra.get("uuid"))
            .and_then(Value::as_str)
            .unwrap_or(&self.id)
    }

    /// Editorial: the catalog marks this item for the Discover page.
    pub fn is_featured(&self) -> bool {
        self.extra
            .get("featured")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    /// Editorial: a short story shown on the hero card instead of the summary.
    pub fn story(&self) -> Option<LocalizedText> {
        serde_json::from_value(self.extra.get("story")?.clone()).ok()
    }

    /// Editorial: the small label above a card or banner title, such as "Quick look".
    /// The UI falls back to the item's kind when the catalog gives none.
    pub fn tag(&self) -> Option<LocalizedText> {
        serde_json::from_value(self.extra.get("tag")?.clone()).ok()
    }

    /// Editorial: a wide image for the hero card.
    pub fn hero_image(&self) -> Option<&str> {
        self.extra
            .get("hero_image")
            .and_then(Value::as_str)
            .filter(|u| !u.is_empty())
    }

    /// Screenshot URLs: `demo_urls` plus `promo_url`, skipping blanks.
    pub fn screenshots(&self) -> Vec<&str> {
        let demos = self
            .extra
            .get("demo_urls")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str);
        let promo = self.extra.get("promo_url").and_then(Value::as_str);
        demos
            .chain(promo)
            .filter(|u| !u.trim().is_empty())
            .collect()
    }

    /// Native package name: `package_name`, else the item id.
    pub fn native_name(&self) -> &str {
        self.package_name.as_deref().unwrap_or(&self.id)
    }
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
    fn editorial_and_media_fields() {
        let i: Item = serde_json::from_str(
            r#"{"id":"x","type":"flatpak","featured":true,"hero_image":"https://x/h.png",
                "story":{"en":"A story"},"demo_urls":["https://x/1.png",""],"promo_url":"https://x/p.png"}"#,
        )
        .unwrap();
        assert!(i.is_featured());
        assert!(i.tag().is_none());
        assert_eq!(i.hero_image(), Some("https://x/h.png"));
        assert_eq!(i.story().unwrap().resolve(&["en"]), "A story");
        assert_eq!(i.screenshots(), ["https://x/1.png", "https://x/p.png"]);
        let plain: Item =
            serde_json::from_str(r#"{"id":"y","type":"flatpak","demo_urls":[],"promo_url":""}"#)
                .unwrap();
        assert!(!plain.is_featured() && plain.screenshots().is_empty() && plain.story().is_none());
    }

    #[test]
    fn security_report_and_docs() {
        let i: Item = serde_json::from_str(
            r##"{"id":"x","type":"sayri_skill","skill_md":"# Doc",
                "security_report":{"score":90,"status":"PASSED","audited_by":"Bot","summary":"ok","virustotal_detections":0}}"##,
        )
        .unwrap();
        let r = i.security_report().unwrap();
        assert_eq!((r.score, r.status.as_deref()), (Some(90), Some("PASSED")));
        assert_eq!(i.inline_docs(), Some("# Doc"));
        let none: Item =
            serde_json::from_str(r#"{"id":"y","type":"flatpak","raw_content":"  "}"#).unwrap();
        assert!(none.security_report().is_none() && none.inline_docs().is_none());
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
