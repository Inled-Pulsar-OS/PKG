//! Search and category filtering over catalog items.

use crate::core::model::{Item, SourceKind};
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

/// Lowercases and strips accents, so `editor` matches `Éditor`.
pub fn fold(s: &str) -> String {
    if s.is_ascii() {
        return s.to_ascii_lowercase();
    }
    s.nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

/// Separates fields inside one haystack so a query cannot match across two fields.
const FIELD_BREAK: char = '\u{1}';

fn haystack(item: &Item) -> String {
    let mut out = String::new();
    let fields = item
        .name
        .texts()
        .into_iter()
        .chain(item.summary.texts())
        .chain(item.description.texts())
        .chain(std::iter::once(item.id.as_str()))
        .chain(item.tags.iter().map(String::as_str));
    for field in fields {
        out.push_str(&fold(field));
        out.push(FIELD_BREAK);
    }
    out
}

/// Folded search text for every item, built once per catalog load so each
/// keystroke only scans strings instead of normalizing them again.
pub struct SearchIndex {
    haystacks: Vec<String>,
}

impl SearchIndex {
    pub fn build(items: &[Item]) -> Self {
        Self {
            haystacks: items.iter().map(haystack).collect(),
        }
    }

    /// Items whose name, summary, description, id, or tags contain `query`.
    /// An empty query returns everything. Results keep catalog order.
    /// `items` must be the slice the index was built from.
    pub fn search<'a>(&self, items: &'a [Item], query: &str) -> Vec<&'a Item> {
        let q = fold(query.trim());
        if q.is_empty() {
            return items.iter().collect();
        }
        items
            .iter()
            .zip(&self.haystacks)
            .filter(|(_, h)| h.contains(&q))
            .map(|(item, _)| item)
            .collect()
    }
}

/// One-off search without an index. Prefer `SearchIndex` when searching repeatedly.
pub fn search<'a>(items: &'a [Item], query: &str) -> Vec<&'a Item> {
    SearchIndex::build(items).search(items, query)
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
    fn index_matches_one_off_search_and_does_not_cross_fields() {
        let it = items();
        let index = SearchIndex::build(&it);
        for q in ["gimp", "editor", "notas", "zzzz", ""] {
            let a: Vec<_> = index.search(&it, q).iter().map(|i| &i.id).collect();
            let b: Vec<_> = search(&it, q).iter().map(|i| &i.id).collect();
            assert_eq!(a, b);
        }
        // "gimp" ends the name and "image" starts the summary; the pair must not match.
        assert!(index.search(&it, "gimpimage").is_empty());
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
