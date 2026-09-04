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
from pulsaros_spotlight.calculator import Calculator

if TYPE_CHECKING:
    from pulsaros_spotlight.clipboard import ClipboardManager

logger = logging.getLogger(__name__)

_BUS_NAME = "org.freedesktop.LocalSearch3"

# -- SPARQL query templates ---------------------------------------------------

_SEARCH_ALL = """\
SELECT DISTINCT ?url WHERE {{
    ?u nie:url ?url .
    {filter}
}}
"""

_SEARCH_APPS = """\
SELECT ?url ?name WHERE {{
    ?u a nfo:Software ;
       nie:url ?url ;
       nie:title ?name .
    FILTER(CONTAINS(LCASE(?name), LCASE("{query}")))
}} LIMIT {limit}
"""

_CATEGORY_FILTERS: dict[str, str] = {
    "documents": 'FILTER(REGEX(?url, "\\\\.(pdf|txt|md|doc|docx|odt|xls|xlsx|ods|ppt|pptx|odp|csv|rtf|epub|html|json|xml|yaml|yml|sh|py|c|cpp|h|rs|go|js|ts)$", "i"))',
    "images": 'FILTER(REGEX(?url, "\\\\.(png|jpg|jpeg|svg|webp|gif|avif|ico|bmp|tiff)$", "i"))',
    "audio": 'FILTER(REGEX(?url, "\\\\.(mp3|flac|wav|ogg|m4a|aac|opus|wma|oga)$", "i"))',
    "video": 'FILTER(REGEX(?url, "\\\\.(mp4|mkv|avi|mov|webm|flv|wmv|m4v|3gp)$", "i"))',
}

_SEARCH_CATEGORY = """\
SELECT DISTINCT ?url WHERE {{
    ?u nie:url ?url .
    {category_filter}
    {filter}
}}
"""

DEFAULT_LIMIT = 500


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
        self.connect()

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

    def close(self) -> None:
        """Cancel file monitors and release backend connections."""
        for monitor in self._monitors:
            try:
                monitor.cancel()
            except Exception:
                pass
        self._monitors.clear()
        self._conn = None

    def connect(self) -> bool:
        """Connect to the Tracker SPARQL endpoint over D-Bus."""
        if self._conn is not None:
            return True
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

    def get_indexing_status(self) -> tuple[bool, str, float]:
        """Check if Tracker/LocalSearch miner is currently indexing.
        Returns (is_indexing, status_text, progress_float).
        """
        for bus_name, obj_path in [
            ("org.freedesktop.LocalSearch3.Miner.Files", "/org/freedesktop/LocalSearch3/Miner/Files"),
            ("org.freedesktop.Tracker3.Miner.Files", "/org/freedesktop/Tracker3/Miner/Files"),
        ]:
            try:
                bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
                proxy = Gio.DBusProxy.new_sync(
                    bus,
                    Gio.DBusProxyFlags.NONE,
                    None,
                    bus_name,
                    obj_path,
                    "org.freedesktop.Tracker3.Miner",
                    None,
                )
                status_var = proxy.call_sync("GetStatus", None, Gio.DBusCallFlags.NONE, 200, None)
                status = status_var.unpack()[0] if status_var else "Idle"
                if status and status.lower() not in ("idle", "inactivo", "paused"):
                    progress_var = proxy.call_sync("GetProgress", None, Gio.DBusCallFlags.NONE, 200, None)
                    progress = float(progress_var.unpack()[0]) if progress_var else 0.0
                    return True, status, progress
            except Exception:
                pass
        return False, "Idle", 1.0

    def _load_apps(self) -> None:
        """Load desktop applications for app search."""
        try:
            self._apps = load_apps()
        except Exception:
            logger.warning("Failed to load desktop applications", exc_info=True)
            self._apps = []

    @property
    def is_ready(self) -> bool:
        if self._conn is None:
            self.connect()
        return self._conn is not None

    def search_instant(
        self,
        query: str,
        category: str = "all",
        limit: int = DEFAULT_LIMIT,
    ) -> list[SearchResult]:
        """Instant in-memory search for apps, clipboard, and calculator without blocking UI."""
        if category == "clipboard":
            if self._clipboard_mgr:
                return self._clipboard_mgr.search_history(query)
            return []

        if category == "web":
            return self._search_web(query)

        query = query.strip()

        if category in ("apps", "applications"):
            return self._search_apps(query, limit)

        if not query and category == "all":
            return self._search_apps("", max(limit, 200))

        if category == "all":
            calc_eval = Calculator.evaluate(query)
            calc_list = []
            if calc_eval:
                val_str, snippet = calc_eval
                calc_list = [
                    SearchResult(
                        url=f"calc://{val_str}",
                        title=val_str,
                        mime="application/x-calculator",
                        snippet=snippet,
                        app=None,
                    )
                ]
            app_results = self._search_apps(query, limit)
            clip_results = []
            if self._clipboard_mgr:
                clip_results = self._clipboard_mgr.search_history(query)[:3]
            web_results = self._search_web(query)[:1] if query and not calc_eval else []
            return calc_list + clip_results + app_results + web_results

        return []

    def _search_web(self, query: str) -> list[SearchResult]:
        """Provide instant web search, bookmarks, and default web sites."""
        import urllib.parse
        results: list[SearchResult] = []
        q_strip = query.strip()
        if q_strip:
            encoded = urllib.parse.quote_plus(q_strip)
            results.append(
                SearchResult(
                    url=f"https://www.google.com/search?q={encoded}",
                    title=f"Search '{q_strip}' on the Web",
                    mime="text/html",
                    snippet=f"Open Google search for '{q_strip}'",
                    app=None,
                )
            )
            import sqlite3
            from pathlib import Path
            home = Path.home()
            for db_path in home.glob(".mozilla/firefox/*/places.sqlite"):
                try:
                    conn = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
                    cursor = conn.cursor()
                    cursor.execute(
                        "SELECT url, title FROM moz_places WHERE (title LIKE ? OR url LIKE ?) AND hidden = 0 ORDER BY frecency DESC LIMIT 5",
                        (f"%{q_strip}%", f"%{q_strip}%"),
                    )
                    for u, t in cursor.fetchall():
                        results.append(SearchResult(url=u, title=t or u, mime="text/html", snippet=u, app=None))
                    conn.close()
                except Exception:
                    pass
        else:
            results.extend([
                SearchResult(url="https://www.google.com", title="Google", mime="text/html", snippet="https://www.google.com", app=None),
                SearchResult(url="https://github.com", title="GitHub", mime="text/html", snippet="https://github.com", app=None),
                SearchResult(url="https://youtube.com", title="YouTube", mime="text/html", snippet="https://youtube.com", app=None),
                SearchResult(url="https://wikipedia.org", title="Wikipedia", mime="text/html", snippet="https://wikipedia.org", app=None),
            ])
            import sqlite3
            from pathlib import Path
            home = Path.home()
            for db_path in home.glob(".mozilla/firefox/*/places.sqlite"):
                try:
                    conn = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
                    cursor = conn.cursor()
                    cursor.execute(
                        "SELECT url, title FROM moz_places WHERE hidden = 0 AND title != '' ORDER BY frecency DESC LIMIT 6"
                    )
                    for u, t in cursor.fetchall():
                        results.append(SearchResult(url=u, title=t or u, mime="text/html", snippet=u, app=None))
                    conn.close()
                except Exception:
                    pass
        return results

    def search(
        self,
        query: str,
        category: str = "all",
        limit: int = DEFAULT_LIMIT,
    ) -> list[SearchResult]:
        """Synchronous search combining instant apps and Tracker files."""
        instant = self.search_instant(query, category, limit)
        if category in ("apps", "applications", "clipboard", "web") or not self.is_ready:
            return instant

        sparql = self._build_query(query.strip(), category, limit)
        files = self._execute(sparql)
        return instant + files

    def search_async(
        self,
        query: str,
        category: str = "all",
        limit: int | None = None,
        callback: Callable[[list[SearchResult]], None] | None = None,
    ) -> None:
        """Run Tracker SPARQL query in background thread and return results via callback."""
        if not self.is_ready or category in ("apps", "applications", "clipboard", "web"):
            if callback:
                GLib.idle_add(callback, [])
            return

        import mimetypes
        import threading
        import urllib.parse

        def _worker():
            try:
                gi.require_version("Tsparql", "3.0")
                from gi.repository import Tsparql

                conn = Tsparql.SparqlConnection.bus_new(_BUS_NAME, None, None)
                sparql = self._build_query(query.strip(), category, limit)
                cursor = conn.query(sparql, None)
                results: list[SearchResult] = []
                seen: set[str] = set()
                while cursor.next():
                    raw_url = cursor.get_string(0)
                    url = raw_url[0] if isinstance(raw_url, tuple) else raw_url
                    url = url or ""
                    if not url or url in seen:
                        continue
                    seen.add(url)
                    title = urllib.parse.unquote(url.rsplit("/", 1)[-1])
                    mime = mimetypes.guess_type(url)[0] or "application/octet-stream"
                    results.append(
                        SearchResult(url=url, title=title, mime=mime, snippet="", app=None)
                    )
                results.sort(key=lambda r: r.title.lower())
                if callback:
                    GLib.idle_add(callback, results)
            except Exception as e:
                logger.debug("Async search worker error: %s", e)
                if callback:
                    GLib.idle_add(callback, [])

        threading.Thread(target=_worker, daemon=True).start()

    def _search_apps(self, query: str, limit: int = 500) -> list[SearchResult]:
        """Search desktop applications by name, comment, keywords, and filename."""
        query_lower = query.lower()
        results: list[SearchResult] = []

        for app in self._apps:
            if (
                not query_lower
                or query_lower in app.lower_name
                or (app.lower_comment and query_lower in app.lower_comment)
                or (app.lower_keywords and query_lower in app.lower_keywords)
                or query_lower in app.filename.lower()
            ):
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

    def _build_query(self, query: str, category: str, limit: int | None = None) -> str:
        home_scope = 'FILTER(STRSTARTS(?url, "file:///home/") || STRSTARTS(?url, "file:///media/") || STRSTARTS(?url, "file:///run/media/"))'
        ignore_clutter = 'FILTER(!CONTAINS(?url, "/node_modules/") && !CONTAINS(?url, "/.git/") && !CONTAINS(?url, "/.cache/"))'
        if query:
            q_clean = query.replace('"', '\\"').lower()
            filter_clause = (
                f'{home_scope}\n    {ignore_clutter}\n    FILTER(CONTAINS(LCASE(?url), "{q_clean}"))'
            )
        else:
            filter_clause = f"{home_scope}\n    {ignore_clutter}"

        limit_val = limit if limit is not None else 100
        limit_clause = f" LIMIT {limit_val}"

        if category in ("apps", "applications"):
            return _SEARCH_APPS.format(query=query, limit=limit_val)
        if category in _CATEGORY_FILTERS:
            return _SEARCH_CATEGORY.format(
                category_filter=_CATEGORY_FILTERS[category],
                filter=filter_clause,
            ) + limit_clause
        return _SEARCH_ALL.format(query=query, filter=filter_clause) + limit_clause

    def _execute(self, sparql: str) -> list[SearchResult]:
        if not self._conn:
            return []
        try:
            cursor = self._conn.query(sparql, None)
        except Exception:
            logger.warning("SPARQL query failed", exc_info=True)
            return []

        import mimetypes
        import urllib.parse
        results: list[SearchResult] = []
        seen: set[str] = set()
        while cursor.next():
            raw_url = cursor.get_string(0)
            url = raw_url[0] if isinstance(raw_url, tuple) else raw_url
            url = url or ""
            if not url or url in seen:
                continue
            seen.add(url)
            title = urllib.parse.unquote(url.rsplit("/", 1)[-1])
            mime = mimetypes.guess_type(url)[0] or "application/octet-stream"
            results.append(
                SearchResult(url=url, title=title, mime=mime, snippet="", app=None)
            )
        results.sort(key=lambda r: r.title.lower())
        return results
