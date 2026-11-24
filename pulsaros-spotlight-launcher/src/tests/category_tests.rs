use super::*;

#[test]
fn table_over_all_variants() {
    // (variant, id, tab label, section title, file tab, file search)
    let t: [(Category, &str, &str, &str, bool, bool); 13] = [
        (Category::All, "all", "All", "Results", false, true),
        (Category::Applications, "applications", "Apps", "Files", false, false),
        (Category::Files, "files", "Files", "Files", true, true),
        (Category::Documents, "documents", "Docs", "Documents", true, true),
        (Category::Images, "images", "Images", "Images", true, true),
        (Category::Audio, "audio", "Music", "Music", true, true),
        (Category::Video, "video", "Videos", "Videos", true, true),
        (Category::Clipboard, "clipboard", "Clip", "Files", false, false),
        (Category::Actions, "actions", "Actions", "Files", false, false),
        (Category::Windows, "windows", "Windows", "Files", false, false),
        (Category::Snippets, "snippets", "Snippets", "Files", false, false),
        (Category::Calc, "calc", "Calc", "Files", false, false),
        (Category::Web, "web", "Web", "Files", false, false),
    ];
    assert_eq!(Category::ALL.len(), t.len());
    for (i, (c, id, label, title, file_tab, file_search)) in t.into_iter().enumerate() {
        assert_eq!(Category::ALL[i], c);
        assert_eq!(c.id(), id);
        assert_eq!(Category::from_id(id), Some(c));
        assert_eq!(c.label(), label);
        assert_eq!(c.section_title(), title);
        assert_eq!(c.is_file_tab(), file_tab);
        assert_eq!(c.needs_file_search(), file_search);
        assert_eq!(c.browse_dir().is_some(), file_tab);
    }
}

#[test]
fn browse_folders_per_category() {
    assert_eq!(Category::Documents.browse_dir(), dirs::document_dir().or_else(dirs::home_dir));
    assert_eq!(Category::Images.browse_dir(), dirs::picture_dir().or_else(dirs::home_dir));
    assert_eq!(Category::Audio.browse_dir(), dirs::audio_dir().or_else(dirs::home_dir));
    assert_eq!(Category::Video.browse_dir(), dirs::video_dir().or_else(dirs::home_dir));
    assert_eq!(Category::Files.browse_dir(), dirs::home_dir());
}

#[test]
fn from_id_aliases_and_unknown() {
    assert_eq!(Category::from_id("apps"), Some(Category::Applications));
    assert_eq!(Category::from_id("nope"), None);
}
