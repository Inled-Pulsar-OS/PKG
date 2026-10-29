//! Search and category filtering over catalog items.

use crate::core::model::{Item, SourceKind};
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

/// Lowercases and strips accents, so `editor` matches `Éditor`.
pub fn fold(s: &str) -> String {
    s.nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

fn matches(item: &Item, q: &str) -> bool {
    let hit = |s: &str| fold(s).contains(q);
    item.name.texts().into_iter().any(hit)
        || item.summary.texts().into_iter().any(hit)
        || item.description.texts().into_iter().any(hit)
        || hit(&item.id)
        || item.tags.iter().any(|t| hit(t))
}

/// Items whose name, summary, description, id, or tags contain `query`.
/// An empty query returns everything. Results keep catalog order.
pub fn search<'a>(items: &'a [Item], query: &str) -> Vec<&'a Item> {
    let q = fold(query.trim());
    if q.is_empty() {
        return items.iter().collect();
    }
    items.iter().filter(|i| matches(i, &q)).collect()
}

/// Filters by a sidebar category key: `all`/`discover`, `apps`, `extensions`,
/// `sayri`, `installed`, or a catalog category name (case-insensitive).
pub fn by_category<'a>(
    items: &'a [Item],
    category: &str,
    is_installed: &dyn Fn(&Item) -> bool,
) -> Vec<&'a Item> {
    let key = category.to_lowercase();
    items
        .iter()
        .filter(|i| match key.as_str() {
            "all" | "discover" => true,
            "apps" => matches!(i.kind, SourceKind::Flatpak | SourceKind::System),
            "extensions" => i.kind == SourceKind::GnomeExtension,
            "sayri" => matches!(i.kind, SourceKind::SayriSkill | SourceKind::SayriPlugin),
            "installed" => is_installed(i),
            other => i.category.to_lowercase() == other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<Item> {
        serde_json::from_str(
            r#"[
            {"id":"org.gimp.GIMP","type":"flatpak","name":"GIMP","summary":"Image editor","category":"Graphics","tags":["photo"]},
            {"id":"ext@x","type":"gnome_extension","name":"Éditor Bar","category":"Shell"},
            {"id":"sk","type":"sayri_skill","name":{"en":"Notes","es":"Notas"}}
        ]"#,
        )
        .unwrap()
    }

    #[test]
    fn empty_query_returns_all() {
        assert_eq!(search(&items(), "  ").len(), 3);
    }

    #[test]
    fn matches_fields_case_and_accent_insensitively() {
        let it = items();
        assert_eq!(search(&it, "GIMP").len(), 1);
        assert_eq!(search(&it, "photo").len(), 1);
        assert_eq!(search(&it, "editor").len(), 2); // "Image editor" and "Éditor Bar"
        assert_eq!(search(&it, "ORG.gimp").len(), 1);
    }

    #[test]
    fn matches_any_translation() {
        assert_eq!(search(&items(), "notas").len(), 1);
    }

    #[test]
    fn no_match_is_empty() {
        assert!(search(&items(), "zzzz").is_empty());
    }

    #[test]
    fn categories() {
        let it = items();
        let none = |_: &Item| false;
        assert_eq!(by_category(&it, "apps", &none).len(), 1);
        assert_eq!(by_category(&it, "extensions", &none).len(), 1);
        assert_eq!(by_category(&it, "sayri", &none).len(), 1);
        assert_eq!(by_category(&it, "graphics", &none).len(), 1);
        assert_eq!(
            by_category(&it, "installed", &|i: &Item| i.id == "sk").len(),
            1
        );
        assert_eq!(by_category(&it, "all", &none).len(), 3);
    }
}
