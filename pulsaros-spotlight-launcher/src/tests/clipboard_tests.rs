use super::*;
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "pulsaros-clipboard-test-{}-{}-{}",
        tag,
        std::process::id(),
        n
    ));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn manager_with_max(max_items: usize) -> ClipboardManager {
    let mut config = SpotlightConfig::default();
    config.clipboard_max_items = max_items;
    ClipboardManager::new_for_test(config, temp_dir("mgr"))
}

#[test]
fn record_clip_dedups_at_top() {
    let mgr = manager_with_max(50);
    mgr.record_clip("a");
    mgr.record_clip("b");
    mgr.record_clip("a");
    let items: Vec<String> = mgr.get_items().into_iter().map(|i| i.text).collect();
    assert_eq!(items, vec!["a", "b"]);
}

#[test]
fn record_clip_truncates_at_configured_max_items() {
    let mgr = manager_with_max(2);
    mgr.record_clip("a");
    mgr.record_clip("b");
    mgr.record_clip("c");
    let items: Vec<String> = mgr.get_items().into_iter().map(|i| i.text).collect();
    assert_eq!(items, vec!["c", "b"]);
}

#[test]
fn unresolvable_data_dir_disables_persistence_without_panicking() {
    // Mirrors `data_dir()` returning `None` (platform lookup failed):
    // history must stay in-memory only, and `save` must be a safe no-op
    // instead of writing to a hardcoded fallback path.
    let mgr = ClipboardManager::new_internal(SpotlightConfig::default(), None);
    mgr.record_clip("in-memory-only");

    let items: Vec<String> = mgr.get_items().into_iter().map(|i| i.text).collect();
    assert_eq!(items, vec!["in-memory-only"]);
    assert!(mgr.inner.lock().unwrap().data_dir.is_none());

    // save() is called from record_clip already; call it again directly
    // to confirm it doesn't panic when there's no data dir to write to.
    mgr.save();
}

#[test]
fn save_writes_file_with_owner_only_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let mgr = manager_with_max(50);
    mgr.record_clip("secret-token");

    let dir = mgr.inner.lock().unwrap().data_dir.clone().unwrap();
    let file = ClipboardManager::clipboard_file_in(&dir);
    let mode = fs::metadata(&file).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}
