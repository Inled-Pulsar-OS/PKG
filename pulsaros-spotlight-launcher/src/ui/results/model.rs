use crate::core::root_search::{Row, RowKind};
use crate::apps::DesktopApp;

// ---------------------------------------------------------------------
// Sectioned result list
// ---------------------------------------------------------------------

/// What a file, app, web or clipboard row opens: its target url, mime type and desktop app.
#[derive(Clone, Debug)]
pub struct Payload {
    pub url: String,
    pub mime: String,
    pub app: Option<DesktopApp>,
}

/// Builds a backend row (file, web, clipboard, calculator hit) with its typed payload.
pub fn result_row(url: String, title: String, mime: String, snippet: String, app: Option<DesktopApp>) -> RowData {
    let (kind, id, icon) = if let Some(v) = url.strip_prefix("calc://") {
        (RowKind::Calc, format!("calc:{v}"), "accessories-calculator-symbolic")
    } else if url.starts_with("clipboard://") {
        (RowKind::Clipboard, url.clone(), "edit-paste-symbolic")
    } else {
        (RowKind::File, url.clone(), "text-x-generic")
    };
    let sub = if snippet.is_empty() { url.trim_start_matches("file://").to_string() } else { snippet };
    RowData { row: Row::simple(kind, &id, &title, &sub, icon), payload: Some(Payload { url, mime, app }), calc: None }
}

/// One displayed row: the core row plus the payload it opens
/// (files, clipboard, calculator) so activation can reuse existing logic.
#[derive(Clone, Debug)]
pub struct RowData {
    pub row: Row,
    pub payload: Option<Payload>,
    pub calc: Option<crate::calculator::CalcResult>,
}

#[derive(Clone, Debug)]
pub enum Item {
    Header(String),
    Row(RowData),
}

pub fn kind_label(r: &RowData) -> &'static str {
    match r.row.kind {
        RowKind::App => "Application",
        RowKind::Command => "Command",
        RowKind::Clipboard => "Clipboard",
        RowKind::Calc => "Calculator",
        RowKind::Fallback => "",
        RowKind::File => match &r.payload {
            Some(p) if p.url.starts_with("http") => "Web",
            Some(p) if p.mime == "inode/directory" || p.mime == "folder" => "Folder",
            _ => "File",
        },
    }
}
