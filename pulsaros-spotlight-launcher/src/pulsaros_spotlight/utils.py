"""Utility helpers for PulsarOS Spotlight."""

from __future__ import annotations

import os
from pathlib import Path
from gi.repository import Gio, Gdk, Gtk


_ICON_DIR = Path("/usr/share/pulsaros-spotlight/icons")
_LOCAL_ICON_DIR = Path(__file__).resolve().parent.parent.parent / "data" / "icons"


def open_file(url: str) -> bool:
    """Open a file or URI with the default application via GIO."""
    try:
        Gio.AppInfo.launch_default_for_uri(url, None)
        return True
    except Exception:
        return False


def get_file_icon(url: str, mime: str | None = None, is_dir: bool = False) -> Gtk.Image:
    """Return a Gtk.Image with the best icon (bundled vector SVG, thumbnail, or theme icon)."""
    # 1. Directory
    if is_dir or mime in ("inode/directory", "folder"):
        for base in (_LOCAL_ICON_DIR, _ICON_DIR):
            f = base / "folder.svg"
            if f.exists():
                return Gtk.Image.new_from_file(str(f))
        return Gtk.Image.new_from_icon_name("folder")

    # 2. Check bundled programming language & format vector icons by file extension
    clean_path = url.removeprefix("file://")
    ext = Path(clean_path).suffix.lower().lstrip(".")

    _EXT_MAP = {
        "py": "file_python.svg",
        "pyw": "file_python.svg",
        "sh": "file_shell.svg",
        "bash": "file_shell.svg",
        "zsh": "file_shell.svg",
        "fish": "file_shell.svg",
        "js": "file_javascript.svg",
        "mjs": "file_javascript.svg",
        "cjs": "file_javascript.svg",
        "ts": "file_typescript.svg",
        "tsx": "file_typescript.svg",
        "jsx": "file_javascript.svg",
        "html": "file_html.svg",
        "htm": "file_html.svg",
        "css": "file_css.svg",
        "scss": "file_css.svg",
        "sass": "file_css.svg",
        "c": "file_c.svg",
        "h": "file_c.svg",
        "cpp": "file_cpp.svg",
        "hpp": "file_cpp.svg",
        "cc": "file_cpp.svg",
        "cs": "file_csharp.svg",
        "rs": "file_rust.svg",
        "go": "file_go.svg",
        "java": "file_java.svg",
        "jar": "file_package.svg",
        "php": "file_php.svg",
        "rb": "file_ruby.svg",
        "lua": "file_lua.svg",
        "sql": "file_sql.svg",
        "json": "file_json.svg",
        "xml": "file_xml.svg",
        "yaml": "file_yaml.svg",
        "yml": "file_yaml.svg",
        "md": "file_markdown.svg",
        "markdown": "file_markdown.svg",
        "txt": "file_text.svg",
        "pdf": "file_pdf.svg",
        "doc": "file_word.svg",
        "docx": "file_word.svg",
        "odt": "file_word.svg",
        "xls": "file_excel.svg",
        "xlsx": "file_excel.svg",
        "ods": "file_excel.svg",
        "csv": "file_excel.svg",
        "ppt": "file_powerpoint.svg",
        "pptx": "file_powerpoint.svg",
        "odp": "file_powerpoint.svg",
        "whl": "file_package.svg",
        "tar": "file_package.svg",
        "gz": "file_package.svg",
        "xz": "file_package.svg",
        "zst": "file_package.svg",
        "zip": "file_package.svg",
        "pkg": "file_package.svg",
        "deb": "file_package.svg",
    }

    if ext in _EXT_MAP:
        icon_file = _EXT_MAP[ext]
        for base in (_LOCAL_ICON_DIR, _ICON_DIR):
            f = base / icon_file
            if f.exists():
                return Gtk.Image.new_from_file(str(f))

    # 3. Mime-type symbolic fallback
    fallback_name = icon_for_mime(mime or "")
    return Gtk.Image.new_from_icon_name(fallback_name)


def icon_for_mime(mime: str) -> str:
    """Map a MIME type to a symbolic icon name."""
    if not mime:
        return "text-x-generic-symbolic"
    if mime == "text/plain-clipboard":
        return "edit-paste-symbolic"
    if mime.startswith("image/"):
        return "image-x-generic-symbolic"
    if mime.startswith("audio/"):
        return "audio-x-generic-symbolic"
    if mime.startswith("video/"):
        return "video-x-generic-symbolic"
    if mime == "application/pdf":
        return "application-pdf-symbolic"
    if mime.startswith("text/"):
        return "text-x-generic-symbolic"
    if "document" in mime or "wordprocessing" in mime:
        return "x-office-document-symbolic"
    if "spreadsheet" in mime:
        return "x-office-spreadsheet-symbolic"
    if "presentation" in mime:
        return "x-office-presentation-symbolic"
    if "archive" in mime or "compressed" in mime or "zip" in mime or "tar" in mime:
        return "package-x-generic-symbolic"
    return "text-x-generic-symbolic"
