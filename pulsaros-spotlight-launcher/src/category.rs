//! Search categories (the mode tabs). One place for ids, labels and per-category behavior.

use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    All,
    Applications,
    Files,
    Documents,
    Images,
    Audio,
    Video,
    Clipboard,
    Actions,
    Windows,
    Snippets,
    Calc,
    Web,
}

impl Category {
    /// Tab order.
    pub const ALL: [Category; 13] = [
        Category::All,
        Category::Applications,
        Category::Files,
        Category::Documents,
        Category::Images,
        Category::Audio,
        Category::Video,
        Category::Clipboard,
        Category::Actions,
        Category::Windows,
        Category::Snippets,
        Category::Calc,
        Category::Web,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Category::All => "all",
            Category::Applications => "applications",
            Category::Files => "files",
            Category::Documents => "documents",
            Category::Images => "images",
            Category::Audio => "audio",
            Category::Video => "video",
            Category::Clipboard => "clipboard",
            Category::Actions => "actions",
            Category::Windows => "windows",
            Category::Snippets => "snippets",
            Category::Calc => "calc",
            Category::Web => "web",
        }
    }

    pub fn from_id(id: &str) -> Option<Category> {
        if id == "apps" {
            return Some(Category::Applications);
        }
        Category::ALL.into_iter().find(|c| c.id() == id)
    }

    /// Short tab label.
    pub fn label(self) -> &'static str {
        match self {
            Category::All => "All",
            Category::Applications => "Apps",
            Category::Files => "Files",
            Category::Documents => "Docs",
            Category::Images => "Images",
            Category::Audio => "Music",
            Category::Video => "Videos",
            Category::Clipboard => "Clip",
            Category::Actions => "Actions",
            Category::Windows => "Windows",
            Category::Snippets => "Snippets",
            Category::Calc => "Calc",
            Category::Web => "Web",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            Category::All | Category::Calc | Category::Web => "spotlight",
            Category::Applications => "applications",
            Category::Files | Category::Documents | Category::Images | Category::Audio | Category::Video => "files",
            Category::Clipboard => "clipboard",
            Category::Actions | Category::Windows | Category::Snippets => "actions",
        }
    }

    pub fn placeholder(self) -> &'static str {
        match self {
            Category::All => "Spotlight Search...",
            Category::Applications => "Search Applications...",
            Category::Files => "Search Files...",
            Category::Documents => "Search Documents...",
            Category::Images => "Search Images...",
            Category::Audio => "Search Music...",
            Category::Video => "Search Videos...",
            Category::Clipboard => "Search Clipboard...",
            Category::Actions => "Search Commands & Settings...",
            Category::Windows => "Filter Open Windows...",
            Category::Snippets => "Search Snippets...",
            Category::Calc => "Calculate (e.g. 24*15, $50 in EUR)...",
            Category::Web => "Search the Web...",
        }
    }

    /// Title of the file-search results section.
    pub fn section_title(self) -> &'static str {
        match self {
            Category::Documents => "Documents",
            Category::Images => "Images",
            Category::Audio => "Music",
            Category::Video => "Videos",
            Category::All => "Results",
            _ => "Files",
        }
    }

    /// A file-type tab: browses a folder when the query is empty.
    pub fn is_file_tab(self) -> bool {
        matches!(
            self,
            Category::Files | Category::Documents | Category::Images | Category::Audio | Category::Video
        )
    }

    /// Whether Nautilus file search applies to this category.
    pub fn needs_file_search(self) -> bool {
        !matches!(
            self,
            Category::Applications
                | Category::Actions
                | Category::Clipboard
                | Category::Web
                | Category::Calc
                | Category::Windows
                | Category::Snippets
        )
    }

    /// Folder shown for an empty query. `None` for non-file tabs; `Files` and any missing
    /// XDG folder fall back to home.
    pub fn browse_dir(self) -> Option<PathBuf> {
        if !self.is_file_tab() {
            return None;
        }
        match self {
            Category::Documents => dirs::document_dir(),
            Category::Images => dirs::picture_dir(),
            Category::Audio => dirs::audio_dir(),
            Category::Video => dirs::video_dir(),
            _ => None,
        }
        .or_else(dirs::home_dir)
    }
}

#[cfg(test)]
#[path = "tests/category_tests.rs"]
mod tests;
