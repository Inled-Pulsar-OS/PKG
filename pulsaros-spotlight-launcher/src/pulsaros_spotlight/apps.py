"""Desktop file parser for PulsarOS Spotlight app search."""

from __future__ import annotations

import configparser
import logging
from dataclasses import dataclass
from pathlib import Path

logger = logging.getLogger(__name__)

DESKTOP_DIRS = [
    Path("/usr/share/applications"),
    Path.home() / ".local/share/applications",
    Path("/var/lib/flatpak/exports/share/applications"),
    Path.home() / ".local/share/flatpak/exports/share/applications",
    Path("/var/lib/snapd/desktop/applications"),
]


@dataclass(frozen=True)
class DesktopApp:
    """Parsed .desktop file entry."""

    name: str
    exec: str
    icon: str
    comment: str
    categories: str
    filename: str
    lower_name: str = ""
    lower_comment: str = ""


def _parse_desktop_file(path: Path) -> DesktopApp | None:
    """Parse a single .desktop file. Returns None on failure."""
    try:
        cp = configparser.ConfigParser(interpolation=None)
        cp.read(path, encoding="utf-8")

        if not cp.has_section("Desktop Entry"):
            return None

        entry_type = cp.get("Desktop Entry", "Type", fallback="Application")
        if entry_type != "Application":
            return None

        no_display = cp.get("Desktop Entry", "NoDisplay", fallback="false")
        if no_display.lower() in ("true", "1", "yes"):
            return None

        hidden = cp.get("Desktop Entry", "Hidden", fallback="false")
        if hidden.lower() in ("true", "1", "yes"):
            return None

        name = cp.get("Desktop Entry", "Name", fallback="")
        if not name:
            return None

        exec_cmd = cp.get("Desktop Entry", "Exec", fallback="")
        if not exec_cmd:
            return None

        comment = cp.get("Desktop Entry", "Comment", fallback="")
        return DesktopApp(
            name=name,
            exec=exec_cmd,
            icon=cp.get("Desktop Entry", "Icon", fallback="application-x-executable"),
            comment=comment,
            categories=cp.get("Desktop Entry", "Categories", fallback=""),
            filename=path.name,
            lower_name=name.lower(),
            lower_comment=comment.lower(),
        )
    except Exception:
        logger.debug("Failed to parse %s", path, exc_info=True)
        return None


def load_apps() -> list[DesktopApp]:
    """Load all .desktop files from standard directories."""
    apps: list[DesktopApp] = []
    seen_names: set[str] = set()

    for dir_path in DESKTOP_DIRS:
        if not dir_path.is_dir():
            continue
        for desktop_file in sorted(dir_path.glob("*.desktop")):
            app = _parse_desktop_file(desktop_file)
            if app and app.name not in seen_names:
                apps.append(app)
                seen_names.add(app.name)

    logger.info("Loaded %d desktop applications", len(apps))
    return apps
