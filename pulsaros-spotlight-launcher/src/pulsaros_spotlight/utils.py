"""Utility helpers for PulsarOS Spotlight."""

from __future__ import annotations

from gi.repository import Gio


def open_file(url: str) -> bool:
    """Open a file or URI with the default application via GIO."""
    try:
        Gio.AppInfo.launch_default_for_uri(url, None)
        return True
    except Exception:
        return False


def icon_for_mime(mime: str) -> str:
    """Map a MIME type to a symbolic icon name."""
    if not mime:
        return "application-x-executable-symbolic"
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
    return "application-x-executable-symbolic"
