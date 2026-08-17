"""Tracker/TinySPARQL search backend for PulsarOS Spotlight."""

from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import TYPE_CHECKING

import gi
gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib
from typing import Callable

from pulsaros_spotlight.apps import DESKTOP_DIRS, DesktopApp, load_apps

if TYPE_CHECKING:
    from pulsaros_spotlight.clipboard import ClipboardManager

logger = logging.getLogger(__name__)

_BUS_NAME = "org.freedesktop.LocalSearch3"

# -- SPARQL query templates ---------------------------------------------------

_SEARCH_ALL = """\
SELECT DISTINCT ?url ?title ?mime WHERE {{
    ?u a nfo:FileDataObject ;
       nie:url ?url ;
       nfo:fileName ?title .
    OPTIONAL {{ ?u nie:interpretedAs ?ie . ?ie nie:mimeType ?mime . }}
    {filter}
}} ORDER BY ?title LIMIT {limit}
"""

_SEARCH_APPS = """\
SELECT ?url ?name WHERE {{
    ?u a nfo:Software ;
       nie:url ?url ;
       nie:title ?name .
    FILTER(CONTAINS(LCASE(?name), LCASE("{query}")))
}} ORDER BY ?name LIMIT {limit}
"""

_SEARCH_CATEGORY = """\
SELECT DISTINCT ?url ?title ?mime WHERE {{
    ?f a nfo:FileDataObject ;
       nie:url ?url ;
       nfo:fileName ?title ;
       nie:interpretedAs ?m .
    ?m a {rdf_type} .
    OPTIONAL {{ ?m nie:mimeType ?mime . }}
    {excludes}
    {filter}
}} ORDER BY ?title LIMIT {limit}
"""

# Tracker 3.x stores the MIME-derived type (Image/Video/Audio/…) on the
# resource linked via nie:interpretedAs, not on the nfo:FileDataObject itself.
_CATEGORY_RDF_TYPES: dict[str, str] = {
    "documents": "nie:InformationElement",
    "images": "nfo:Image",
    "audio": "nfo:Audio",
    "video": "nfo:Video",
}

# "documents" = any file that is not an image, video, audio or folder.
_CATEGORY_EXCLUDES: dict[str, str] = {
    "documents": (
        "FILTER NOT EXISTS { ?m a nfo:Image }\n"
        "    FILTER NOT EXISTS { ?m a nfo:Video }\n"
        "    FILTER NOT EXISTS { ?m a nfo:Audio }\n"
        "    FILTER NOT EXISTS { ?m a nfo:Folder }"
    ),
}

DEFAULT_LIMIT = 50


@dataclass(frozen=True)
class SearchResult:
    """Single search result from Tracker or internal providers."""

    url: str
    title: str
    mime: str = ""
    snippet: str = ""
    app: DesktopApp | None = None


class SearchBackend:
    """SPARQL query layer over Tracker/TinySPARQL and local providers."""

    def __init__(self, clipboard_mgr: ClipboardManager | None = None) -> None:
        self._conn = None
        self._clipboard_mgr = clipboard_mgr
        self._apps: list[DesktopApp] = []
        self._on_apps_updated: Callable[[], None] | None = None
        self._monitors: list[Gio.FileMonitor] = []
        self._load_apps()
        self._setup_app_monitors()

    def set_on_apps_updated(self, callback: Callable[[], None]) -> None:
        """Register a callback to be notified when the installed apps list changes."""
        self._on_apps_updated = callback

    def _setup_app_monitors(self) -> None:
        """Watch application directories for new/removed/updated .desktop files."""
        for dir_path in DESKTOP_DIRS:
            try:
                gfile = Gio.File.new_for_path(str(dir_path))
                if dir_path.is_dir():
                    monitor = gfile.monitor_directory(Gio.FileMonitorFlags.NONE, None)
                    monitor.connect("changed", self._on_apps_dir_changed)
                    self._monitors.append(monitor)
            except Exception:
                logger.debug("Failed to set up directory monitor for %s", dir_path, exc_info=True)

    def _on_apps_dir_changed(self, monitor, file, other_file, event_type) -> None:
        """Handler for file changes in desktop directories."""
        self.reload_apps()
        if self._on_apps_updated:
            GLib.idle_add(self._on_apps_updated)

    def reload_apps(self) -> None:
        """Reload desktop applications from disk."""
        self._load_apps()

    def connect(self) -> bool:
        """Connect to the Tracker SPARQL endpoint over D-Bus."""
        try:
            gi.require_version("Tsparql", "3.0")
            from gi.repository import Tsparql

            self._conn = Tsparql.SparqlConnection.bus_new(_BUS_NAME, None, None)
            logger.info("Connected to %s", _BUS_NAME)
            return True
        except Exception:
            logger.warning("Failed to connect to Tracker (%s)", _BUS_NAME)
            self._conn = None
            return False

    def _load_apps(self) -> None:
        """Load desktop applications for app search."""
        try:
            self._apps = load_apps()
        except Exception:
            logger.warning("Failed to load desktop applications", exc_info=True)
            self._apps = []

    @property
    def is_ready(self) -> bool:
        return self._conn is not None

    def search(
        self,
        query: str,
        category: str = "all",
        limit: int = DEFAULT_LIMIT,
    ) -> list[SearchResult]:
        """Run a full-text search and return results."""
        if category == "clipboard":
            if self._clipboard_mgr:
                return self._clipboard_mgr.search_history(query)
            return []

        query = query.strip()

        if category in ("apps", "applications"):
            return self._search_apps(query, limit)

        # Empty query: show the full application list, like macOS Spotlight on open
        if not query and category == "all":
            return self._search_apps("", max(limit, 200))

        if category == "all":
            app_results = self._search_apps(query, limit)
            clip_results = []
            if self._clipboard_mgr:
                clip_results = self._clipboard_mgr.search_history(query)[:3]

            if self.is_ready:
                sparql = self._build_query(query, category, limit)
                file_results = self._execute(sparql)
                return clip_results + app_results + file_results
            return clip_results + app_results

        if not self.is_ready:
            return []

        sparql = self._build_query(query, category, limit)
        return self._execute(sparql)

    def _search_apps(self, query: str, limit: int) -> list[SearchResult]:
        """Search desktop applications by name and comment."""
        query_lower = query.lower()
        results: list[SearchResult] = []

        for app in self._apps:
            if query_lower in app.name.lower() or (app.comment and query_lower in app.comment.lower()):
                # Use exec as the URL so open_file can launch it directly
                app_url = f"app://{app.filename}"
                results.append(
                    SearchResult(
                        url=app_url,
                        title=app.name,
                        mime="application/x-desktop",
                        snippet=app.comment or "",
                        app=app,
                    )
                )
                if len(results) >= limit:
                    break

        return results

    # -- internal -------------------------------------------------------------

    def _build_query(self, query: str, category: str, limit: int) -> str:
        filter_clause = (
            f'FILTER(CONTAINS(LCASE(?title), LCASE("{query}")))'
            if query
            else ""
        )
        if category in ("apps", "applications"):
            return _SEARCH_APPS.format(query=query, limit=limit)
        if category in _CATEGORY_RDF_TYPES:
            return _SEARCH_CATEGORY.format(
                query=query,
                rdf_type=_CATEGORY_RDF_TYPES[category],
                excludes=_CATEGORY_EXCLUDES.get(category, ""),
                filter=filter_clause,
                limit=limit,
            )
        return _SEARCH_ALL.format(query=query, filter=filter_clause, limit=limit)

    def _execute(self, sparql: str) -> list[SearchResult]:
        try:
            cursor = self._conn.query(sparql, None)
        except Exception:
            logger.warning("SPARQL query failed", exc_info=True)
            return []

        results: list[SearchResult] = []
        seen: set[str] = set()
        while cursor.next():
            raw_url = cursor.get_string(0)
            url = raw_url[0] if isinstance(raw_url, tuple) else raw_url
            url = url or ""
            raw_title = cursor.get_string(1)
            title = raw_title[0] if isinstance(raw_title, tuple) else raw_title
            title = title or url.rsplit("/", 1)[-1]
            raw_mime = cursor.get_string(2)
            mime = raw_mime[0] if isinstance(raw_mime, tuple) else raw_mime
            mime = mime or ""
            if url in seen:
                continue
            seen.add(url)
            results.append(
                SearchResult(url=url, title=title, mime=mime, snippet="", app=None)
            )
        return results
