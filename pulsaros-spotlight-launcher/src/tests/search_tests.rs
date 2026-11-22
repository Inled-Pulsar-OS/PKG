use super::*;

fn file_result(title: &str) -> SearchResult {
    SearchResult {
        url: format!("file:///tmp/{title}"),
        title: title.to_string(),
        mime: String::new(),
        snippet: String::new(),
        app: None,
    }
}

#[test]
fn order_file_results_puts_closest_title_first() {
    let mut results = vec![
        file_result("annual-report-draft-old.pdf"),
        file_result("report-2024.pdf"),
        file_result("report.pdf"),
    ];
    order_file_results(&mut results, "report");
    let titles: Vec<&str> = results.iter().map(|r| r.title.as_str()).collect();
    assert_eq!(titles, ["report.pdf", "report-2024.pdf", "annual-report-draft-old.pdf"]);
}

#[test]
fn order_file_results_keeps_input_order_for_equal_relevance() {
    let mut results = vec![file_result("zeta-report.txt"), file_result("alpha-report.txt")];
    order_file_results(&mut results, "report");
    assert_eq!(results[0].title, "zeta-report.txt");
}

#[test]
fn browser_history_plain_returns_empty_when_no_database() {
    let tmp = std::env::temp_dir().join("pulsar_spotlight_test_no_hist");
    let _ = std::fs::create_dir_all(&tmp);
    std::env::set_var("HOME", &tmp);
    let results = query_browser_history_plain("spotlight-test", 10);
    assert!(results.is_empty());
}

#[test]
fn matches_category_filters_correctly() {
    assert!(matches_category("document.pdf", "documents"));
    assert!(matches_category("notes.txt", "documents"));
    assert!(!matches_category("photo.jpg", "documents"));
    assert!(matches_category("photo.png", "images"));
    assert!(matches_category("song.mp3", "audio"));
    assert!(matches_category("clip.mp4", "video"));
    assert!(matches_category("anything.xyz", "all"));
    assert!(matches_category("anything.xyz", "files"));
    assert!(matches_category("noext", "files"));
}

#[test]
fn run_async_delivers_result_on_the_main_context_without_polling() {
    use std::cell::Cell;
    use std::rc::Rc;
    // As on the GTK main thread, this thread must own the default context so `invoke`
    // from the worker is queued here instead of running on the worker thread.
    let ctx = gtk4::glib::MainContext::default();
    let _owner = ctx.acquire().expect("default main context");
    let got: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
    let g = got.clone();
    run_async(
        || vec![SearchResult { url: "u".into(), title: "t".into(), mime: "m".into(), snippet: "s".into(), app: None }],
        move |r| g.set(Some(r.len())),
    );
    let start = std::time::Instant::now();
    while got.get().is_none() && start.elapsed() < std::time::Duration::from_secs(5) {
        ctx.iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(got.get(), Some(1));
}
