use super::*;

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
}
