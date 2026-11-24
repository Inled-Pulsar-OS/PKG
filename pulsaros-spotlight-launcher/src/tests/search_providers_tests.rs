use super::*;

fn job(p: &dyn Provider, q: &str) -> Option<(JobKind, &'static str)> {
    p.background(&Query::new(q)).map(|j| (j.kind, j.title))
}

#[test]
fn files_job_titled_by_category_and_skipped_when_not_applicable() {
    assert_eq!(job(&FilesProvider::new(Category::All), "doc"), Some((JobKind::Files, "Results")));
    assert_eq!(job(&FilesProvider::new(Category::Images), "cat"), Some((JobKind::Files, "Images")));
    assert_eq!(job(&FilesProvider::new(Category::All), "  "), None);
    // A file tab runs even with an empty query; Web and Clipboard never do.
    assert!(job(&FilesProvider::new(Category::Documents), "").is_some());
    assert_eq!(job(&FilesProvider::new(Category::Web), "x"), None);
    assert_eq!(job(&FilesProvider::new(Category::Clipboard), "x"), None);
    assert!(FilesProvider::new(Category::All).instant(&Query::new("x")).is_empty());
}

#[test]
fn web_rows_are_a_search_link_or_quick_links_in_web_mode_only() {
    let rows = WebProvider::new(Category::Web).instant(&Query::new(" a b "));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "https://www.google.com/search?q=a%20b");
    assert_eq!(rows[0].title, "Search 'a b' on the Web");
    assert_eq!(rows[0].kind, RowKind::File);
    let quick = WebProvider::new(Category::Web).instant(&Query::new(""));
    assert_eq!(quick.iter().map(|r| r.title.as_str()).collect::<Vec<_>>(), ["Google", "GitHub", "YouTube", "Wikipedia"]);
    assert!(WebProvider::new(Category::All).instant(&Query::new("a")).is_empty());
}

#[test]
fn history_job_only_in_web_mode_or_a_typed_all_query() {
    assert_eq!(job(&HistoryProvider::new(Category::Web), ""), Some((JobKind::History, "Web")));
    assert_eq!(job(&HistoryProvider::new(Category::All), "rust"), Some((JobKind::History, "Results")));
    assert_eq!(job(&HistoryProvider::new(Category::All), ""), None);
    assert_eq!(job(&HistoryProvider::new(Category::Images), "rust"), None);
    assert!(HistoryProvider::new(Category::Web).instant(&Query::new("x")).is_empty());
}

#[test]
fn registry_merges_providers_in_order() {
    let mut r = crate::core::provider::Registry::default();
    r.register(Box::new(WebProvider::new(Category::Web)));
    r.register(Box::new(FilesProvider::new(Category::Web)));
    r.register(Box::new(HistoryProvider::new(Category::Web)));
    let q = Query::new("x");
    assert_eq!(r.instant(&q).len(), 1);
    assert_eq!(r.background(&q).len(), 1);
}
