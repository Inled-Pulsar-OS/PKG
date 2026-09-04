"""Desktop file parser for PulsarOS Spotlight app search."""

from __future__ import annotations

import logging
import os
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
    keywords: str = ""
    lower_keywords: str = ""


def _get_locale_keys() -> list[str]:
    keys: list[str] = []
    env_lang = (
        os.environ.get("LC_ALL")
        or os.environ.get("LC_MESSAGES")
        or os.environ.get("LANG")
        or ""
    )
    if env_lang:
        clean = env_lang.split(".")[0].split("@")[0].strip()
        if clean:
            keys.append(clean)
            if "_" in clean:
                lang_only = clean.split("_")[0].strip()
                if lang_only and lang_only != clean:
                    keys.append(lang_only)
    return keys


def _pick_localized(field_map: dict[str, str], locales: list[str]) -> str | None:
    for loc in locales:
        if loc in field_map and field_map[loc].strip():
            return field_map[loc].strip()
    if "" in field_map and field_map[""].strip():
        return field_map[""].strip()
    return None


def _parse_desktop_file(path: Path) -> DesktopApp | None:
    """Parse a single .desktop file. Returns None on failure."""
    try:
        in_desktop_entry = False
        app_type = None
        no_display = None
        hidden = None
        exec_cmd = None
        icon = "application-x-executable"
        categories = ""

        names: dict[str, str] = {}
        generic_names: dict[str, str] = {}
        comments: dict[str, str] = {}
        keywords_map: dict[str, str] = {}

        with path.open("r", encoding="utf-8", errors="replace") as f:
            for raw_line in f:
                line = raw_line.strip()
                if not line or line.startswith("#"):
                    continue

                if line.startswith("[") and line.endswith("]"):
                    in_desktop_entry = (line == "[Desktop Entry]")
                    continue

                if not in_desktop_entry:
                    continue

                if "=" not in line:
                    continue

                key, value = line.split("=", 1)
                key = key.strip()
                value = value.strip()

                if key == "Type":
                    app_type = value
                elif key == "NoDisplay":
                    no_display = value.lower()
                elif key == "Hidden":
                    hidden = value.lower()
                elif key == "Exec":
                    exec_cmd = value
                elif key == "Icon":
                    icon = value or "application-x-executable"
                elif key == "Categories":
                    categories = value
                elif key == "Name":
                    names[""] = value
                elif key.startswith("Name[") and key.endswith("]"):
                    loc = key[5:-1]
                    names[loc] = value
                elif key == "GenericName":
                    generic_names[""] = value
                elif key.startswith("GenericName[") and key.endswith("]"):
                    loc = key[12:-1]
                    generic_names[loc] = value
                elif key == "Comment":
                    comments[""] = value
                elif key.startswith("Comment[") and key.endswith("]"):
                    loc = key[8:-1]
                    comments[loc] = value
                elif key == "Keywords":
                    keywords_map[""] = value
                elif key.startswith("Keywords[") and key.endswith("]"):
                    loc = key[9:-1]
                    keywords_map[loc] = value

        if app_type != "Application" and app_type is not None:
            return None
        if no_display in ("true", "1", "yes"):
            return None
        if hidden in ("true", "1", "yes"):
            return None
        if not exec_cmd:
            return None

        locales = _get_locale_keys()
        name = _pick_localized(names, locales) or _pick_localized(generic_names, locales)
        if not name:
            return None

        comment = _pick_localized(comments, locales) or ""
        keywords = _pick_localized(keywords_map, locales) or ""

        return DesktopApp(
            name=name,
            exec=exec_cmd,
            icon=icon,
            comment=comment,
            categories=categories,
            filename=path.name,
            lower_name=name.lower(),
            lower_comment=comment.lower(),
            keywords=keywords,
            lower_keywords=keywords.lower(),
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
