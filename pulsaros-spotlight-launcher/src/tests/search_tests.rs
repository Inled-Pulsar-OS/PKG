use super::*;

fn file_result(title: &str) -> RowData {
    result_row(format!("file:///tmp/{title}"), title.to_string(), String::new(), String::new(), None)
}

#[test]
fn order_file_results_puts_closest_title_first() {
    let mut results = vec![
        file_result("annual-report-draft-old.pdf"),
        file_result("report-2024.pdf"),
        file_result("report.pdf"),
    ];
    order_file_results(&mut results, "report");
    let titles: Vec<&str> = results.iter().map(|r| r.row.title.as_str()).collect();
    assert_eq!(titles, ["report.pdf", "report-2024.pdf", "annual-report-draft-old.pdf"]);
}

#[test]
fn order_file_results_keeps_input_order_for_equal_relevance() {
    let mut results = vec![file_result("zeta-report.txt"), file_result("alpha-report.txt")];
    order_file_results(&mut results, "report");
    assert_eq!(results[0].row.title, "zeta-report.txt");
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
    assert!(matches_category("document.pdf", Category::Documents));
    assert!(matches_category("notes.txt", Category::Documents));
    assert!(!matches_category("photo.jpg", Category::Documents));
    assert!(matches_category("photo.png", Category::Images));
    assert!(matches_category("song.mp3", Category::Audio));
    assert!(matches_category("clip.mp4", Category::Video));
    assert!(matches_category("anything.xyz", Category::All));
    assert!(matches_category("anything.xyz", Category::Files));
    assert!(matches_category("noext", Category::Files));
}

/// Tests that own the default main context run one at a time.
static CTX_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn run_async_delivers_result_on_the_main_context_without_polling() {
    use std::cell::Cell;
    use std::rc::Rc;
    // As on the GTK main thread, this thread must own the default context so `invoke`
    // from the worker is queued here instead of running on the worker thread.
    let _serial = CTX_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let ctx = gtk4::glib::MainContext::default();
    let _owner = ctx.acquire().expect("default main context");
    let got: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
    let g = got.clone();
    run_async(
        JobKind::Files,
        |_| vec![result_row("u".into(), "t".into(), "m".into(), "s".into(), None)],
        move |r| g.set(Some(r.len())),
    );
    let start = std::time::Instant::now();
    while got.get().is_none() && start.elapsed() < std::time::Duration::from_secs(5) {
        ctx.iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(got.get(), Some(1));
}

#[test]
fn newest_per_kind_keeps_only_the_last_job_of_each_kind() {
    use JobKind::{Files, History};
    assert_eq!(newest_per_kind(&[Files, Files, History, Files]), [false, false, true, true]);
    assert_eq!(newest_per_kind(&[History]), [true]);
    assert!(newest_per_kind(&[]).is_empty());
}

#[test]
fn superseded_job_never_runs_but_its_callback_still_fires() {
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{mpsc, Arc};
    let _serial = CTX_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let ctx = gtk4::glib::MainContext::default();
    let _owner = ctx.acquire().expect("default main context");

    // Block the worker so the next two jobs queue up behind it.
    let (release_tx, release_rx) = mpsc::channel::<()>();
    run_async(JobKind::History, move |_| { let _ = release_rx.recv(); Vec::new() }, |_| {});

    let ran = Arc::new(AtomicUsize::new(0));
    let answered = Rc::new(Cell::new(0));
    for _ in 0..2 {
        let (r, a) = (ran.clone(), answered.clone());
        run_async(JobKind::Files, move |_| { r.fetch_add(1, Ordering::SeqCst); Vec::new() }, move |_| a.set(a.get() + 1));
    }
    release_tx.send(()).unwrap();

    let start = std::time::Instant::now();
    while answered.get() < 2 && start.elapsed() < std::time::Duration::from_secs(5) {
        ctx.iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(answered.get(), 2);
    assert_eq!(ran.load(Ordering::SeqCst), 1);
}
